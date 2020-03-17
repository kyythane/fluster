#![deny(clippy::all)]

use super::actions::{Action, ActionList, PartDefinition};
use super::rendering::{Bitmap, Renderer, Shape};
use pathfinder_color::ColorU;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use std::collections::HashMap;
use streaming_iterator::StreamingIterator;
use uuid::Uuid;

enum DisplayLibraryItem {
    Vector { shape: Shape },
    Bitmap { bitmap: Bitmap },
}

#[derive(Clone, PartialEq, Debug)]
enum Part {
    Vector {
        item_id: Uuid,
        transform: Transform2F,
        color: Option<ColorU>,
    },
    Bitmap {
        item_id: Uuid,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    },
}

#[derive(Clone, PartialEq, Debug)]
struct Entity {
    id: Uuid,
    name: String,
    transform: Transform2F,
    depth: u32,
    active: bool,
    parts: Vec<Part>,
    parent: Uuid,
    children: Vec<Uuid>,
}

impl Entity {
    fn add_child(&mut self, child: Uuid) {
        self.children.push(child);
    }
}

pub struct State {
    frame: usize,
    root_entity_id: Uuid,
    background_color: ColorU,
    running: bool,
}

pub fn play(
    renderer: &impl Renderer,
    actions: &mut ActionList,
    on_frame_complete: &dyn Fn(&State) -> (),
) {
    let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
    let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
    let mut state = initialize(actions, &mut display_list, &mut library);
    while let Some(_) = actions.get() {
        if !state.running {
            break;
        }
        state = execute_actions(state, actions, &mut display_list, &mut library);
        if let Some(Action::PresentFrame(count)) = actions.get() {
            if *count == -1 {
                loop {
                    //TODO: handle input
                    //TODO: scripts
                    paint(renderer, &state, &display_list, &library);
                    on_frame_complete(&state);
                }
            } else if *count == 0 {
                panic!("Cannot display a frame 0 times! That's just silly");
            } else {
                for _ in 0..*count {
                    //TODO: handle input
                    //TODO: scripts
                    paint(renderer, &state, &display_list, &library);
                    on_frame_complete(&state);
                }
                state = State {
                    frame: state.frame + *count as usize,
                    ..state
                };
            }
        }
        actions.advance();
    }
}

fn define_shape(id: &Uuid, shape: &Shape, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
    if !library.contains_key(id) {
        let item = DisplayLibraryItem::Vector {
            shape: shape.clone(),
        };
        library.insert(*id, item);
    }
}

// Note: this is destructive to the source bitmap. Bitmaps can be very large, and library loads are idempotent
fn load_bitmap(id: &Uuid, bitmap: &mut Bitmap, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
    if !library.contains_key(id) {
        let item = DisplayLibraryItem::Bitmap {
            bitmap: bitmap.release_contents(),
        };
        library.insert(*id, item);
    }
}

fn initialize(
    actions: &mut ActionList,
    display_list: &mut HashMap<Uuid, Entity>,
    library: &mut HashMap<Uuid, DisplayLibraryItem>,
) -> State {
    let mut root_entity_id: Option<Uuid> = None;
    let mut background_color = ColorU::white();
    while let Some(action) = actions.get_mut() {
        match action {
            Action::CreateRoot { id } => {
                root_entity_id = Some(*id);
                if !display_list.is_empty() {
                    panic!("Attempted to create root in non-empty display list");
                }
                let root = Entity {
                    id: *id,
                    name: String::from("root"),
                    transform: Transform2F::default(),
                    depth: 0,
                    active: true,
                    parent: *id,
                    parts: vec![],
                    children: vec![],
                };
                display_list.insert(*id, root);
            }
            Action::DefineShape { id, shape } => {
                define_shape(id, shape, library);
            }
            Action::LoadBitmap { id, ref mut bitmap } => {
                load_bitmap(id, bitmap, library);
            }
            Action::SetBackground { color } => background_color = *color,
            Action::EndInitialization => break,
            _ => panic!("Unexpected action in initialization"),
        }
        actions.advance();
    }

    State {
        frame: 0,
        root_entity_id: root_entity_id.expect("Action list did not define a root element"),
        background_color,
        running: true,
    }
}

fn execute_actions(
    state: State,
    actions: &mut ActionList,
    display_list: &mut HashMap<Uuid, Entity>,
    library: &mut HashMap<Uuid, DisplayLibraryItem>,
) -> State {
    let mut state = state;
    while let Some(action) = actions.get_mut() {
        match action {
            Action::DefineShape { id, shape } => {
                define_shape(id, shape, library);
            }
            Action::LoadBitmap { id, ref mut bitmap } => {
                load_bitmap(id, bitmap, library);
            }
            Action::AddEntity {
                id,
                name,
                transform,
                depth,
                parts,
                parent,
            } => {
                let parent = match parent {
                    Some(id) => *id,
                    None => state.root_entity_id,
                };
                match display_list.get_mut(&parent) {
                    Some(parent_entity) => {
                        let entity = Entity {
                            id: *id,
                            name: name.clone(),
                            active: true,
                            transform: Transform2F::from_scale_rotation_translation(transform.scale, transform.theta, transform.translation),
                            depth: *depth,
                            parts:
                                parts.iter().map(|x| {
                                    match x {
                                        PartDefinition::Vector { item_id, transform} => {
                                            match library.get(&item_id) {
                                                Some(DisplayLibraryItem::Vector { .. }) => Part::Vector {
                                                    item_id: *item_id,
                                                    transform: Transform2F::from_scale_rotation_translation(transform.scale, transform.theta, transform.translation),
                                                    color: None
                                                },
                                                None => panic!(format!("Attempting to add entity {} with missing part {}", *id, item_id)),
                                                _ => panic!(format!("Attempting part {} on entity {} mislabeled as Vector when it is a Bitmap", item_id, *id)),
                                            }
                                        },
                                        PartDefinition::Bitmap { item_id, transform, view_rect } => {
                                            match library.get(&item_id) {
                                                Some(DisplayLibraryItem::Bitmap { .. }) => Part::Bitmap {
                                                    item_id: *item_id,
                                                    transform: Transform2F::from_scale_rotation_translation(transform.scale, transform.theta, transform.translation),
                                                    view_rect: RectF::from_points(view_rect.origin, view_rect.lower_right),
                                                    tint: None
                                                    },
                                                None => panic!(format!("Attempting to add entity {} with missing part {}", *id, item_id)),
                                                _ => panic!(format!("Attempting part {} on entity {} mislabeled as Bitmap when it is a Vector", item_id, *id)),
                                            }
                                        },
                                    }
                                }).collect(),
                            parent,
                            children: vec!()
                        };
                        parent_entity.add_child(*id);
                        if let Some(e) = display_list.insert(*id, entity) {
                            // If we replace this entitiy, clear the old children out of the display list.
                            for c in e.children {
                                display_list.remove(&c);
                            }
                        }
                    }
                    None => panic!(format!(
                        "Attempted to attach child {} to non-existant parent {}",
                        *id, parent
                    )),
                }
            }
            Action::SetBackground { color } => state.background_color = *color,
            Action::PresentFrame(_) => break,
            Action::CreateRoot { .. } => panic!("Attempting to define an additional Root"),
            Action::Label(_) => (),
            Action::EndInitialization => (),
        }
        actions.advance();
    }
    state
}

fn paint(
    renderer: &impl Renderer,
    state: &State,
    display_list: &HashMap<Uuid, Entity>,
    library: &HashMap<Uuid, DisplayLibraryItem>,
) {
    use std::collections::BTreeMap;
    use std::collections::VecDeque;
    let mut depth_list: BTreeMap<u64, &Entity> = BTreeMap::new();
    let root = display_list
        .get(&state.root_entity_id)
        .expect("Root Entity unloaded.");
    let mut nodes = VecDeque::new();
    nodes.push_back(root);
    while !nodes.is_empty() {
        if let Some(node) = nodes.pop_front() {
            for child_id in &node.children {
                if let Some(child) = display_list.get(child_id) {
                    nodes.push_back(child);
                }
            }
            let mut depth = (node.depth as u64) << 32;
            while depth_list.contains_key(&depth) {
                depth += 1;
            }
            depth_list.insert(depth, node);
        }
    }
    renderer.set_background(state.background_color);
    //Render from back to front
    for (_, entity) in depth_list {
        for part in &entity.parts {
            match part {
                Part::Vector {
                    item_id,
                    transform,
                    color,
                } => {
                    if let Some(&DisplayLibraryItem::Vector { ref shape }) = library.get(&item_id) {
                        renderer.draw_shape(shape, entity.transform * *transform, *color)
                    }
                }
                Part::Bitmap {
                    item_id,
                    transform,
                    view_rect,
                    tint,
                } => {
                    if let Some(&DisplayLibraryItem::Bitmap { ref bitmap }) = library.get(&item_id)
                    {
                        renderer.draw_bitmap(
                            bitmap,
                            *view_rect,
                            entity.transform * *transform,
                            *tint,
                        )
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ScaleRotationTranslation;
    use mockall::predicate::*;
    use mockall::*;
    use pathfinder_geometry::vector::Vector2F;

    #[test]
    fn it_initializes_state() {
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let actions = vec![
            Action::SetBackground {
                color: ColorU::black(),
            },
            Action::CreateRoot { id: root_id },
            Action::EndInitialization,
        ];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        let state = initialize(&mut action_list, &mut display_list, &mut library);
        assert_eq!(state.background_color, ColorU::black());
        assert_eq!(state.root_entity_id, root_id);
        assert_eq!(action_list.current_index(), 2);
        assert_eq!(action_list.get(), Some(&Action::EndInitialization));
        assert_eq!(display_list.len(), 1);
        let entity1 = display_list
            .get(&root_id)
            .expect("Failed to get expected entity");
        assert_eq!(entity1.name, String::from("root"));
        assert_eq!(entity1.id, root_id);
        assert_eq!(entity1.parent, root_id);
        assert_eq!(entity1.active, true);
    }

    #[test]
    fn it_executes_actions() {
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let entity2_id = Uuid::parse_str("3ec76e6a-7758-47bf-bcb5-7cf5bc309aad").unwrap();
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();
        let scale_rotation_translation = ScaleRotationTranslation {
            scale: Vector2F::splat(1.0),
            theta: 0.0,
            translation: Vector2F::splat(0.0),
        };
        let actions = vec![
            Action::SetBackground {
                color: ColorU::black(),
            },
            Action::DefineShape {
                id: shape_id,
                shape: Shape::FillRect {
                    dimensions: Vector2F::new(5.0, 5.0),
                    color: ColorU::white(),
                },
            },
            Action::AddEntity {
                id: entity_id,
                name: String::from("first"),
                transform: scale_rotation_translation,
                depth: 2,
                parts: vec![PartDefinition::Vector {
                    item_id: shape_id,
                    transform: scale_rotation_translation,
                }],
                parent: None,
            },
            Action::AddEntity {
                id: entity2_id,
                name: String::from("second"),
                transform: scale_rotation_translation,
                depth: 3,
                parts: vec![PartDefinition::Vector {
                    item_id: shape_id,
                    transform: scale_rotation_translation,
                }],
                parent: Some(entity_id),
            },
            Action::PresentFrame(1),
            Action::SetBackground {
                color: ColorU::white(),
            }, // This action will not get run
        ];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        display_list.insert(
            root_id,
            Entity {
                id: root_id,
                name: String::from("root"),
                transform: Transform2F::default(),
                depth: 0,
                active: true,
                parent: root_id,
                parts: vec![],
                children: vec![],
            },
        );
        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        let mut state = State {
            frame: 0,
            root_entity_id: root_id,
            background_color: ColorU::white(),
            running: true,
        };
        state = execute_actions(state, &mut action_list, &mut display_list, &mut library);
        assert_eq!(state.background_color, ColorU::black());
        assert_eq!(action_list.current_index(), 4);
        assert_eq!(action_list.get(), Some(&Action::PresentFrame(1)));
        assert_eq!(library.len(), 1);
        assert_eq!(display_list.len(), 3);
        let entity1 = display_list
            .get(&entity_id)
            .expect("Failed to get expected entity");
        assert_eq!(entity1.name, String::from("first"));
        assert_eq!(entity1.id, entity_id);
        assert_eq!(entity1.parent, root_id);
        assert_eq!(entity1.active, true);
        let entity2 = display_list
            .get(&entity2_id)
            .expect("Failed to get expected entity");
        assert_eq!(entity2.name, String::from("second"));
        assert_eq!(entity2.id, entity2_id);
        assert_eq!(entity2.parent, entity_id);
        assert_eq!(entity2.active, true);
    }

    mock! {
        pub Renderer { }
        trait Renderer {
            fn set_background(&self, color: ColorU);
            fn draw_shape(&self, shape: &Shape, transform: Transform2F, color_override: Option<ColorU>);
            fn draw_bitmap(&self, bitmap: &Bitmap, view_rect: RectF, transform: Transform2F, tint: Option<ColorU>);
        }
    }

    #[test]
    fn it_paints() {
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();

        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        library.insert(
            shape_id,
            DisplayLibraryItem::Vector {
                shape: Shape::FillRect {
                    dimensions: Vector2F::new(15.0, 15.0),
                    color: ColorU::new(0, 255, 0, 255),
                },
            },
        );
        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        display_list.insert(
            root_id,
            Entity {
                id: root_id,
                name: String::from("root"),
                transform: Transform2F::default(),
                depth: 0,
                active: true,
                parent: root_id,
                parts: vec![],
                children: vec![entity_id],
            },
        );
        display_list.insert(
            entity_id,
            Entity {
                id: entity_id,
                name: String::from("entity"),
                transform: Transform2F::default(),
                depth: 1,
                active: true,
                parent: root_id,
                parts: vec![Part::Vector {
                    item_id: shape_id,
                    transform: Transform2F::default(),
                    color: None,
                }],
                children: vec![],
            },
        );
        let state = State {
            frame: 0,
            root_entity_id: root_id,
            background_color: ColorU::white(),
            running: true,
        };
        let mut seq = Sequence::new();
        let mut mock_renderer = MockRenderer::new();
        mock_renderer
            .expect_set_background()
            .times(1)
            .with(eq(ColorU::white()))
            .return_const(())
            .in_sequence(&mut seq);
        mock_renderer
            .expect_draw_shape()
            .times(1)
            .withf(|drawn_shape, transform, color_override| {
                let model_shape = Shape::FillRect {
                    dimensions: Vector2F::new(15.0, 15.0),
                    color: ColorU::new(0, 255, 0, 255),
                };
                drawn_shape == &model_shape
                    && *transform == Transform2F::default()
                    && *color_override == None
            })
            .return_const(())
            .in_sequence(&mut seq);
        paint(&mock_renderer, &state, &display_list, &library)
    }
}
