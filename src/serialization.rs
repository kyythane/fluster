#![deny(clippy::all)]

use super::actions::Action;
use bincode::Error;
use nom::error::ErrorKind;
use nom::number::streaming::{le_i32, le_u32, le_u8};
use nom::{Err, IResult, Needed};
use pathfinder_geometry::vector::Vector2I;
use std::io::{Read, Write};

const FILE_VERSION: u8 = 1;
const ACTION_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Header {
    version: u8,
    stage_size: Vector2I,
}

named!(
    header<Header>,
    do_parse!(
        tag!("FSR")
            >> version: le_u8
            >> stage_height: le_i32
            >> stage_width: le_i32
            >> (Header {
                version: version,
                stage_size: Vector2I::new(stage_width, stage_height)
            })
    )
);

#[derive(Clone, Debug, PartialEq)]
struct ActionData {
    version: u8,
    data_size: u32,
    action: Action,
}

macro_rules! extract_action(
    ($i:expr, $version:expr, $size:expr) => (
        extract_action($version, $size)($i)
    );
);

named!(
    full_action<Action>,
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
            Err(Err::Failure((input, ErrorKind::Verify)))
        }
    }
}

pub fn deserialize_stream(stream: impl Read) {}

pub fn serialize_stream(
    actions: &[Action],
    stage_size: Vector2I,
    out: &mut impl Write,
) -> Result<(), Error> {
    out.write_all(&"FSR".bytes().collect::<Vec<u8>>()[..])?;
    out.write_all(&[FILE_VERSION, 1])?;
    out.write_all(&stage_size.x().to_le_bytes())?;
    out.write_all(&stage_size.y().to_le_bytes())?;
    for action in actions {
        out.write_all(&[ACTION_VERSION, 1])?;
        let serialized = serialize_action(action, ACTION_VERSION)?;
        out.write_all(&(serialized.len() as u32).to_le_bytes())?;
        out.write_all(&serialized[..])?;
    }
    Ok(())
}

//Version is unused for now, but I hate unversioned APIs
pub fn serialize_action(action: &Action, _version: u8) -> Result<Vec<u8>, Error> {
    bincode::serialize(action)
}

//Version is unused for now, but I hate unversioned APIs
pub fn deserialize_action(bytes: &[u8], _version: u8) -> Result<Action, Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{PartDefinition, ScaleRotationTranslation};
    use crate::rendering::Shape;
    use pathfinder_color::ColorU;
    use pathfinder_geometry::vector::Vector2F;
    use uuid::Uuid;

    #[test]
    fn it_serializes_actions() {
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();
        let scale_rotation_translation = ScaleRotationTranslation {
            scale: Vector2F::splat(1.0),
            theta: 0.0,
            translation: Vector2F::splat(0.0),
        };

        let action = Action::AddEntity {
            id: entity_id,
            name: String::from("first"),
            transform: scale_rotation_translation,
            depth: 2,
            parts: vec![PartDefinition::Vector {
                item_id: shape_id,
                transform: scale_rotation_translation,
            }],
            parent: None,
        };

        let serialized = serialize_action(&action, 1).unwrap();
        println!("AddEntity serilized as {} bytes.", serialized.len());
        let deserialized = deserialize_action(&serialized, 1).unwrap();

        assert_eq!(action, deserialized);

        let action = Action::DefineShape {
            id: shape_id,
            shape: Shape::FillPath {
                points: vec![
                    Vector2F::new(1.0, 5.0),
                    Vector2F::new(5.0, 1.0),
                    Vector2F::new(1.0, 1.0),
                    Vector2F::new(5.0, 5.0),
                ],
                color: ColorU::white(),
            },
        };

        let serialized = serialize_action(&action, 1).unwrap();
        println!("DefineShape serilized as {} bytes.", serialized.len());
        let deserialized = deserialize_action(&serialized, 1).unwrap();

        assert_eq!(action, deserialized);
    }
}
