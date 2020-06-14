#![deny(clippy::all)]

use super::actions::Action;
use bincode::Error as BinError;
use bincode::ErrorKind as BinErrorKind;
use circular::Buffer;
use nom::error::ErrorKind as NomErrorKind;
use nom::number::streaming::{le_i32, le_u32, le_u8};
use nom::{Err, IResult, Needed};
use pathfinder_geometry::vector::Vector2I;
use std::io::{Read, Write};

const FILE_VERSION: u8 = 1;
const ACTION_VERSION: u8 = 1;
const STARTING_BUFFER_SIZE: usize = 1000;
const MAX_BUFFER_SIZE: usize = 4_096_000; // 1000 *(2 ^ 12) bytes ~ 4MB

#[derive(Clone, Debug, PartialEq, Eq)]
struct Header {
    version: u8,
    fps: u8,
    stage_size: Vector2I,
}

named!(
    header<Header>,
    do_parse!(
        tag!("FSR")
            >> version: le_u8
            >> fps: le_u8
            >> stage_width: le_i32
            >> stage_height: le_i32
            >> (Header {
                version: version,
                fps: fps,
                stage_size: Vector2I::new(stage_width, stage_height),
            })
    )
);

macro_rules! extract_action(
    ($i:expr, $version:expr, $size:expr) => (
        extract_action($version, $size)($i)
    );
);

named!(
    action<Action>,
    do_parse!(
        version: le_u8
            >> data_size: le_u32
            >> action: extract_action!(version, data_size as usize)
            >> (action)
    )
);

fn extract_action(version: u8, size: usize) -> impl Fn(&[u8]) -> IResult<&[u8], Action> {
    move |input: &[u8]| map!(input, |i| parse_action(i, version, size), |v| v)
}

fn parse_action(input: &[u8], version: u8, size: usize) -> IResult<&[u8], Action> {
    if input.len() < size {
        return Err(Err::Incomplete(Needed::Size(size)));
    }
    if size < 1 {
        return Err(Err::Incomplete(Needed::Size(1)));
    }

    match deserialize_action(&input[..size], version) {
        Ok(action) => Ok((&input[size..], action)),
        Err(e) => {
            println!("{:?}", e); //Write the bincode error so we get it when debugging
            Err(Err::Failure((input, NomErrorKind::Verify)))
        }
    }
}

pub struct DeserializationIterator<T: Read> {
    stream: T,
    buffer: Buffer,
}

//TODO: handle error cases correctly!
impl<T: Read> Iterator for DeserializationIterator<T> {
    type Item = Action;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match action(self.buffer.data()) {
                Ok((remaining, action)) => {
                    let offset = self.buffer.available_data() - remaining.len();
                    self.buffer.consume(offset);
                    return Some(action);
                }
                Err(error) => match error {
                    Err::Incomplete(needed) => {
                        if let nom::Needed::Size(bytes_needed) = needed {
                            if bytes_needed > self.buffer.capacity() {
                                if self.buffer.capacity() * 2 > MAX_BUFFER_SIZE {
                                    return None;
                                }
                                self.buffer.grow(self.buffer.capacity() * 2);
                            }
                        } else {
                            return None;
                        }
                        let bytes = self.stream.read(self.buffer.space()).unwrap();
                        if bytes == 0 {
                            return None;
                        }
                        self.buffer.fill(bytes);
                    }
                    _ => return None,
                },
            }
        }
    }
}

pub fn deserialize_stream<T: Read>(
    mut stream: T,
) -> Result<(Vector2I, u8, DeserializationIterator<T>), BinError> {
    let mut buffer = Buffer::with_capacity(STARTING_BUFFER_SIZE);

    let bytes = stream.read(buffer.space())?;
    buffer.fill(bytes);
    let (remaining, header) = match header(buffer.data()) {
        Ok(h) => h,
        Err(_) => {
            return Err(BinError::from(BinErrorKind::Custom(
                "Could not parse header".to_string(),
            )));
        }
    };
    let offset = buffer.available_data() - remaining.len();
    buffer.consume(offset);

    match header.version {
        1 => {
            let iter = DeserializationIterator { stream, buffer };
            Ok((header.stage_size, header.fps, iter))
        }
        _ => Err(BinError::from(BinErrorKind::Custom(format!(
            "Unsupported verion: {}, maximum supported version: {}",
            header.version, FILE_VERSION
        )))),
    }
}

pub fn serialize_stream(
    actions: &[Action],
    stage_size: Vector2I,
    frames_per_second: u8,
    out: &mut impl Write,
) -> Result<(), BinError> {
    out.write_all(&"FSR".bytes().collect::<Vec<u8>>()[..])?;
    out.write_all(&[FILE_VERSION, frames_per_second])?;
    out.write_all(&stage_size.x().to_le_bytes())?;
    out.write_all(&stage_size.y().to_le_bytes())?;
    for action in actions {
        out.write_all(&[ACTION_VERSION])?;
        let serialized = serialize_action(action, ACTION_VERSION)?;
        out.write_all(&(serialized.len() as u32).to_le_bytes())?;
        out.write_all(&serialized[..])?;
    }
    Ok(())
}

//Version is unused for now, but I hate unversioned APIs
pub fn serialize_action(action: &Action, _version: u8) -> Result<Vec<u8>, BinError> {
    bincode::serialize(action)
}

//Version is unused for now, but I hate unversioned APIs
pub fn deserialize_action(bytes: &[u8], _version: u8) -> Result<Action, BinError> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{EntityDefinition, PartDefinition};
    use crate::types::{
        basic::ScaleRotationTranslation,
        shapes::{Edge, Shape},
    };
    use pathfinder_color::ColorU;
    use pathfinder_geometry::transform2d::Transform2F;
    use pathfinder_geometry::vector::Vector2F;
    use uuid::Uuid;

    #[test]
    fn it_serializes_and_deserializes_actions() {
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();
        let part_id = Uuid::parse_str("b06f8577-aa30-4000-9943-28b94df19361").unwrap();

        let action = Action::AddEntity(EntityDefinition {
            id: entity_id,
            name: String::from("first"),
            transform: Transform2F::default(),
            depth: 2,
            parts: vec![PartDefinition::new(
                part_id,
                shape_id,
                ScaleRotationTranslation::default(),
                vec![],
            )],
            parent: None,
            morph_index: 0.0,
        });

        let serialized = serialize_action(&action, 1).unwrap();
        println!("AddEntity serilized as {} bytes.", serialized.len());
        let deserialized = deserialize_action(&serialized, 1).unwrap();

        assert_eq!(action, deserialized);

        let action = Action::DefineShape {
            id: shape_id,
            shape: Shape::Fill {
                edges: vec![
                    Edge::Line(Vector2F::new(1.0, 5.0)),
                    Edge::Line(Vector2F::new(5.0, 1.0)),
                    Edge::Line(Vector2F::new(1.0, 1.0)),
                    Edge::Line(Vector2F::new(5.0, 5.0)),
                ],
                color: ColorU::white(),
            },
        };

        let serialized = serialize_action(&action, 1).unwrap();
        println!("DefineShape serilized as {} bytes.", serialized.len());
        let deserialized = deserialize_action(&serialized, 1).unwrap();

        assert_eq!(action, deserialized);
    }

    #[test]
    fn it_serializes_and_deserializes_streams() {
        use iobuffer::IoBuffer;
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();
        let part_id = Uuid::parse_str("b06f8577-aa30-4000-9943-28b94df19361").unwrap();
        let actions = vec![
            Action::DefineShape {
                id: shape_id,
                shape: Shape::Fill {
                    edges: vec![
                        Edge::Line(Vector2F::new(1.0, 5.0)),
                        Edge::Line(Vector2F::new(5.0, 1.0)),
                        Edge::Line(Vector2F::new(1.0, 1.0)),
                        Edge::Line(Vector2F::new(5.0, 5.0)),
                    ],
                    color: ColorU::white(),
                },
            },
            Action::AddEntity(EntityDefinition {
                id: entity_id,
                name: String::from("first"),
                transform: Transform2F::default(),
                depth: 2,
                parts: vec![PartDefinition::new(
                    part_id,
                    shape_id,
                    ScaleRotationTranslation::default(),
                    vec![],
                )],
                parent: None,
                morph_index: 0.0,
            }),
            Action::Label("a label".to_string()),
            Action::PresentFrame(1, 2),
            Action::Quit,
        ];
        let mut buffer = IoBuffer::new();
        serialize_stream(&actions, Vector2I::new(960, 480), 60, &mut buffer).unwrap();
        match deserialize_stream(buffer) {
            Ok((size, frames_per_second, iterator)) => {
                assert_eq!(size, Vector2I::new(960, 480));
                assert_eq!(frames_per_second, 60);
                let read: Vec<Action> = iterator.collect();
                assert_eq!(read.len(), actions.len());
                for i in 0..actions.len() {
                    assert_eq!(read[i], actions[i]);
                }
            }
            Err(err) => {
                panic!(err);
            }
        }
    }
}
