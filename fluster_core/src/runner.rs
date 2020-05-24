#![deny(clippy::all)]

use super::actions::{
    Action, ActionList, EntityDefinition, EntityUpdateDefinition, PartDefinition,
};
use super::rendering::{compute_render_data, paint, Renderer};
use super::types::{
    basic::Bitmap,
    model::{DisplayLibraryItem, Entity, Part},
    shapes::Shape,
};
use aabb_quadtree_pathfinder::QuadTree;
use pathfinder_color::ColorU;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::collections::{HashMap, HashSet, VecDeque};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use streaming_iterator::StreamingIterator;
use uuid::Uuid;

pub struct State {
    seconds_per_frame: f32,
    frame: u32,
    delta_time: f32,
    frame_end_time: f64,
    root_entity_id: Uuid,
    background_color: ColorU,
    running: bool,
    stage_size: Vector2F,
    //TODO: pause
}

impl State {
    pub fn new(
        root_entity_id: Uuid,
        background_color: ColorU,
        seconds_per_frame: f32,
        stage_size: Vector2F,
        current_time: f64,
    ) -> State {
        State {
            frame: 0,
            delta_time: 0.0,
            frame_end_time: current_time,
            root_entity_id,
            background_color,
            running: true,
            seconds_per_frame,
            stage_size,
        }
    }

    pub fn get_running(&self) -> bool {
        self.running
    }

    pub fn set_running(&mut self, is_running: bool) {
        self.running = is_running;
    }
}

pub struct SceneData {
    quad_tree: QuadTree<Uuid>,
    world_space_transforms: HashMap<Uuid, Transform2F>,
}

impl SceneData {
    pub fn new(size: Vector2F) -> Self {
        SceneData {
            quad_tree: QuadTree::new(
                RectF::from_points(Vector2F::zero(), size),
                true,
                //TODO: experiment with parameters!
                3,
                10,
                8,
            ),
            world_space_transforms: HashMap::new(),
        }
    }

    pub fn recompute(
        &mut self,
        state: &State,
        display_list: &mut HashMap<Uuid, Entity>,
        library: &HashMap<Uuid, DisplayLibraryItem>,
    ) {
        // First pass algorithm. O(m log n), where m is # dirty nodes and n is # total nodes.
        let mut dirty_roots = display_list
            .iter()
            .filter(|(_, entity)| entity.dirty())
            .map(|(id, entity)| {
                let mut entity = entity;
                let mut maximal_id = id;
                let mut query_id = id;
                while query_id != &state.root_entity_id {
                    query_id = entity.parent();
                    entity = display_list.get(query_id).unwrap();
                    if entity.dirty() {
                        maximal_id = query_id;
                    }
                }
                *maximal_id
            })
            .collect::<HashSet<Uuid>>();
        let mut queue = VecDeque::with_capacity(dirty_roots.len());
        for dirty_root in dirty_roots {
            queue.push_back(dirty_root);
            while let Some(next_node) = queue.pop_front() {
                if let Some(next_entity) = display_list.get_mut(&next_node) {
                    for child_id in next_entity.children() {
                        queue.push_back(*child_id);
                    }
                    if next_node == state.root_entity_id {
                        self.world_space_transforms
                            .insert(*next_entity.id(), *next_entity.transform());
                    } else {
                        // Since we are starting from the highest dirty nodes in our tree, we can always trust that the parent transform is valid
                        let parent_transform = *self
                            .world_space_transforms
                            .get(next_entity.parent())
                            .unwrap();
                        self.world_space_transforms.insert(
                            *next_entity.id(),
                            parent_transform * *next_entity.transform(),
                        );
                    }
                    let next_world_space_transform =
                        self.world_space_transforms.get(next_entity.id()).unwrap();
                    let new_bounds =
                        next_entity.recompute_bounds(next_world_space_transform, library);
                }
            }
        }
    }
}

pub fn play(
    renderer: &mut impl Renderer,
    actions: &mut ActionList,
    on_frame_complete: &mut dyn FnMut(State) -> State,
    seconds_per_frame: f32,
    stage_size: Vector2F,
) -> Result<(), String> {
    let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
    let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
    let mut scene_data = SceneData::new(stage_size);
    let mut state = initialize(
        actions,
        &mut display_list,
        &mut library,
        seconds_per_frame,
        stage_size,
    )?;
    while let Some(_) = actions.get() {
        if !state.running {
            break;
        }
        state = execute_actions(state, actions, &mut display_list, &mut library)?;
        if let Some(Action::PresentFrame(start, count)) = actions.get() {
            if *count == 0 {
                continue; //Treat PresentFrame(_, 0) as a no-op
            } else if state.frame > start + count {
                return Err("Attempting to play incorrect frame. Frame counter and action list have gotten desynced".to_string());
            } else {
                let start = state.frame - *start;
                for frame in 0..(*count - start) {
                    //TODO: skip updates/paints to catch up to frame rate if we are lagging
                    //TODO: handle input
                    //TODO: scripts
                    //TODO: tweens should update consistently w/ frame index instead of via timer
                    update_tweens(state.delta_time, &mut display_list);
                    scene_data.recompute(&state, &mut display_list, &library);
                    draw_frame(renderer, &state, &display_list, &library)?;
                    state = on_frame_complete(state);
                    if !state.running {
                        break;
                    }
                    let frame_end_time = time_seconds();
                    let frame_time_left =
                        state.seconds_per_frame - (frame_end_time - state.frame_end_time) as f32;
                    println!(
                        "frame {:?} time {:?}% of target ",
                        state.frame,
                        (frame_end_time - state.frame_end_time) as f32 / state.seconds_per_frame
                            * 100.0
                    );
                    let frame_end_time = if frame_time_left > 0.0 {
                        thread::sleep(Duration::from_secs_f32(frame_time_left));
                        time_seconds()
                    } else {
                        frame_end_time
                    };
                    state.delta_time = (frame_end_time - state.frame_end_time) as f32;
                    state.frame_end_time = frame_end_time;
                    state.frame = start + frame + 1;
                }
            }
        }
        actions.advance();
    }
    Ok(())
}

#[inline]
fn time_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn update_tweens(elapsed: f32, display_list: &mut HashMap<Uuid, Entity>) {
    for (_, entity) in display_list.iter_mut() {
        entity.update_tweens(elapsed);
    }
}

fn define_shape(id: &Uuid, shape: &Shape, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
    if !library.contains_key(id) {
        let item = DisplayLibraryItem::Vector(shape.clone());
        library.insert(*id, item);
    }
}

// Note: this is destructive to the source bitmap. Bitmaps can be very large, and library loads are idempotent
fn load_bitmap(id: &Uuid, bitmap: &mut Bitmap, library: &mut HashMap<Uuid, DisplayLibraryItem>) {
    if !library.contains_key(id) {
        let item = DisplayLibraryItem::Raster(bitmap.release_contents());
        library.insert(*id, item);
    }
}

fn initialize(
    actions: &mut ActionList,
    display_list: &mut HashMap<Uuid, Entity>,
    library: &mut HashMap<Uuid, DisplayLibraryItem>,
    seconds_per_frame: f32,
    stage_size: Vector2F,
) -> Result<State, String> {
    let mut root_entity_id: Option<Uuid> = None;
    let mut background_color = ColorU::white();
    while let Some(action) = actions.get_mut() {
        match action {
            Action::CreateRoot(id) => {
                root_entity_id = Some(*id);
                if !display_list.is_empty() {
                    return Err("Attempted to create root in non-empty display list".to_string());
                }
                let root = Entity::new(*id, 0, "root", *id, vec![], Transform2F::default(), 0.0);
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
            _ => return Err("Unexpected action in initialization".to_string()),
        }
        actions.advance();
    }

    if let Some(root_entity_id) = root_entity_id {
        Ok(State::new(
            root_entity_id,
            background_color,
            seconds_per_frame,
            stage_size,
            time_seconds(),
        ))
    } else {
        Err("Action list did not define a root element".to_string())
    }
}

fn execute_actions(
    state: State,
    actions: &mut ActionList,
    display_list: &mut HashMap<Uuid, Entity>,
    library: &mut HashMap<Uuid, DisplayLibraryItem>,
) -> Result<State, String> {
    let mut state = state;
    while let Some(action) = actions.get_mut() {
        match action {
            Action::DefineShape { id, shape } => {
                define_shape(id, shape, library);
            }
            Action::LoadBitmap { id, ref mut bitmap } => {
                load_bitmap(id, bitmap, library);
            }
            Action::AddEntity(entity_definition) => {
                add_entity(&state, entity_definition, display_list, library)?;
            }
            Action::UpdateEntity(entity_update_definition) => {
                add_tweens(&state, entity_update_definition, display_list, library)?;
            }
            Action::RemoveEntity(id) => {
                //Removing an entity also removes it's children
                if let Some(old) = display_list.remove(id) {
                    for c in old.children() {
                        display_list.remove(&c);
                    }
                    let parent = display_list.get_mut(old.parent()).unwrap();
                    parent.remove_child(id);
                }
            }
            Action::SetBackground { color } => state.background_color = *color,
            Action::PresentFrame(_, _) => break,
            Action::CreateRoot { .. } => {
                return Err("Attempting to define an additional Root".to_string())
            }
            Action::Label(_) => (),
            Action::EndInitialization => (),
            Action::Quit => {
                state.running = false;
                break;
            }
        }
        actions.advance();
    }
    Ok(state)
}

fn add_entity(
    state: &State,
    entity_definition: &EntityDefinition,
    display_list: &mut HashMap<Uuid, Entity>,
    library: &HashMap<Uuid, DisplayLibraryItem>,
) -> Result<(), String> {
    let (id, name, transform, depth, morph_index, parts, parent) = match entity_definition {
        EntityDefinition {
            id,
            name,
            transform,
            depth,
            morph_index,
            parts,
            parent,
        } => (*id, name, transform, *depth, *morph_index, parts, *parent),
    };

    let parent = match parent {
        Some(id) => id,
        None => state.root_entity_id,
    };
    match display_list.get_mut(&parent) {
        Some(parent_entity) => {
            let parts = {
                let constructed = parts
                    .iter()
                    .map(|x| match x {
                        PartDefinition::Vector { item_id, transform } => {
                            let item = library.get(&item_id);
                            match item {
                                Some(DisplayLibraryItem::Vector { .. }) => {
                                    Some(Part::new_vector(*item_id, *transform, None))
                                }
                                _ => None,
                            }
                        }
                        PartDefinition::Raster {
                            item_id,
                            transform,
                            view_rect,
                        } => match library.get(&item_id) {
                            Some(DisplayLibraryItem::Raster { .. }) => Some(Part::new_raster(
                                *item_id,
                                RectF::from_points(view_rect.origin, view_rect.lower_right),
                                *transform,
                                None,
                            )),
                            _ => None,
                        },
                    })
                    .filter(|e| e.is_some())
                    .map(|e| e.unwrap())
                    .collect::<Vec<Part>>();
                if constructed.len() < parts.len() {
                    return Err(format!("Some parts on {} could not be processed", id));
                }
                constructed
            };
            let entity = Entity::new(id, depth, &name, parent, parts, *transform, morph_index);
            parent_entity.add_child(id);
            if let Some(old) = display_list.insert(id, entity) {
                // If we replace this entitiy, clear the old children out of the display list.
                for c in old.children() {
                    display_list.remove(&c);
                }
                if old.parent() != &parent {
                    let parent = display_list.get_mut(old.parent()).unwrap();
                    parent.remove_child(&id);
                }
            }
            Ok(())
        }
        None => Err(format!(
            "Attempted to attach child {} to non-existant parent {}",
            id, parent
        )),
    }
}

fn add_tweens(
    state: &State,
    entity_update_definition: &EntityUpdateDefinition,
    display_list: &mut HashMap<Uuid, Entity>,
    library: &HashMap<Uuid, DisplayLibraryItem>,
) -> Result<(), String> {
    let duration_seconds =
        state.seconds_per_frame * (entity_update_definition.duration_frames as f32);
    if let Some(entity) = display_list.get_mut(&entity_update_definition.id) {
        entity.add_tweens(entity_update_definition, duration_seconds, library)?;
    }
    Ok(()) //Updating a removed entity or part is a no-op, since a script or event could remove an entity in a way the editor can't account for.
}

fn draw_frame(
    renderer: &mut impl Renderer,
    state: &State,
    display_list: &HashMap<Uuid, Entity>,
    library: &HashMap<Uuid, DisplayLibraryItem>,
) -> Result<(), String> {
    renderer.start_frame(state.stage_size);
    renderer.set_background(state.background_color);
    let render_data = compute_render_data(&state.root_entity_id, display_list)?;
    paint(renderer, render_data, library);
    renderer.end_frame();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tween::Easing;
    use crate::tween::PropertyTween;
    use crate::types::basic::ScaleRotationTranslation;
    use crate::types::shapes::{Coloring, Edge};
    use mockall::predicate::*;
    use mockall::*;
    use pathfinder_content::pattern::Pattern;
    use pathfinder_geometry::transform2d::Transform2F;
    use pathfinder_geometry::vector::Vector2F;
    use std::f32::consts::FRAC_PI_2;
    use std::{thread, time};

    #[test]
    fn it_initializes_state() {
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let actions = vec![
            Action::SetBackground {
                color: ColorU::black(),
            },
            Action::CreateRoot(root_id),
            Action::EndInitialization,
        ];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        let state = initialize(
            &mut action_list,
            &mut display_list,
            &mut library,
            0.016,
            Vector2F::new(800.0, 600.0),
        )
        .unwrap();
        assert_eq!(state.background_color, ColorU::black());
        assert_eq!(state.root_entity_id, root_id);
        assert!((state.seconds_per_frame - 0.016).abs() < std::f32::EPSILON);
        assert_eq!(state.stage_size, Vector2F::new(800.0, 600.0));
        assert_eq!(action_list.current_index(), 2);
        assert_eq!(action_list.get(), Some(&Action::EndInitialization));
        assert_eq!(display_list.len(), 1);
        let entity1 = display_list
            .get(&root_id)
            .expect("Failed to get expected entity");
        assert_eq!(entity1.name(), "root");
        assert_eq!(*entity1.id(), root_id);
        assert_eq!(*entity1.parent(), root_id);
        assert_eq!(entity1.active(), true);
    }

    #[test]
    fn it_executes_actions() {
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let entity2_id = Uuid::parse_str("3ec76e6a-7758-47bf-bcb5-7cf5bc309aad").unwrap();
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();
        let actions = vec![
            Action::SetBackground {
                color: ColorU::black(),
            },
            Action::DefineShape {
                id: shape_id,
                shape: Shape::Fill {
                    edges: vec![
                        Edge::Line(Vector2F::new(-15.0, -15.0)),
                        Edge::Line(Vector2F::new(15.0, -15.0)),
                        Edge::Line(Vector2F::new(15.0, 15.0)),
                        Edge::Line(Vector2F::new(-15.0, 15.0)),
                    ],
                    color: ColorU::new(0, 255, 0, 255),
                },
            },
            Action::AddEntity(EntityDefinition {
                id: entity_id,
                name: String::from("first"),
                transform: Transform2F::default(),
                depth: 2,
                parts: vec![PartDefinition::Vector {
                    item_id: shape_id,
                    transform: Transform2F::default(),
                }],
                parent: None,
                morph_index: 0.0,
            }),
            Action::AddEntity(EntityDefinition {
                id: entity2_id,
                name: String::from("second"),
                transform: Transform2F::default(),
                depth: 3,
                parts: vec![PartDefinition::Vector {
                    item_id: shape_id,
                    transform: Transform2F::default(),
                }],
                parent: Some(entity_id),
                morph_index: 0.0,
            }),
            Action::PresentFrame(1, 1),
            Action::SetBackground {
                color: ColorU::white(),
            }, // This action will not get run
        ];
        let mut action_list = ActionList::new(Box::new(|| None), Some(&actions));
        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        display_list.insert(
            root_id,
            Entity::new(
                root_id,
                0,
                "root",
                root_id,
                vec![],
                Transform2F::default(),
                0.0,
            ),
        );
        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        let mut state = State {
            frame: 0,
            delta_time: 0.0,
            frame_end_time: time_seconds(),
            root_entity_id: root_id,
            background_color: ColorU::white(),
            running: true,
            seconds_per_frame: 0.016,
            stage_size: Vector2F::new(800.0, 600.0),
        };
        state = execute_actions(state, &mut action_list, &mut display_list, &mut library).unwrap();
        assert_eq!(state.background_color, ColorU::black());
        assert_eq!(action_list.current_index(), 4);
        assert_eq!(action_list.get(), Some(&Action::PresentFrame(1, 1)));
        assert_eq!(library.len(), 1);
        assert_eq!(display_list.len(), 3);
        let entity1 = display_list
            .get(&entity_id)
            .expect("Failed to get expected entity");
        assert_eq!(entity1.name(), "first");
        assert_eq!(*entity1.id(), entity_id);
        assert_eq!(*entity1.parent(), root_id);
        assert_eq!(entity1.active(), true);
        let entity2 = display_list
            .get(&entity2_id)
            .expect("Failed to get expected entity");
        assert_eq!(entity2.name(), "second");
        assert_eq!(*entity2.id(), entity2_id);
        assert_eq!(*entity2.parent(), entity_id);
        assert_eq!(entity2.active(), true);
    }

    #[test]
    fn it_updates_tweens() {
        const FRAME_TIME: f32 = 1.0 / 60.0;
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();

        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        let mut root = Entity::new(
            root_id,
            0,
            "root",
            root_id,
            vec![],
            Transform2F::default(),
            0.0,
        );
        root.add_child(entity_id);
        display_list.insert(root_id, root);
        let mut tweens: HashMap<Uuid, Vec<PropertyTween>> = HashMap::new();
        tweens.insert(
            entity_id,
            vec![PropertyTween::new_transform(
                ScaleRotationTranslation::from_transform(&Transform2F::default()),
                ScaleRotationTranslation::from_transform(&Transform2F::from_rotation(FRAC_PI_2)),
                FRAME_TIME * 5.0,
                Easing::CubicIn,
            )],
        );
        tweens.insert(
            shape_id,
            vec![PropertyTween::new_transform(
                ScaleRotationTranslation::from_transform(&Transform2F::default()),
                ScaleRotationTranslation::from_transform(&Transform2F::from_scale(Vector2F::new(
                    6.0, 15.0,
                ))),
                FRAME_TIME * 5.0,
                Easing::Linear,
            )],
        );
        display_list.insert(
            entity_id,
            Entity::new(
                entity_id,
                1,
                "entity",
                root_id,
                vec![Part::new_vector(shape_id, Transform2F::default(), None)],
                Transform2F::default(),
                0.0,
            ),
        );

        let mut delta_time = 0.0;
        let mut frame_end_time = time_seconds();

        let sleep_duration = time::Duration::from_secs_f32(FRAME_TIME);
        for _ in 0..6 {
            update_tweens(delta_time, &mut display_list);
            thread::sleep(sleep_duration);
            let new_frame_end_time = time_seconds();
            delta_time = (new_frame_end_time - frame_end_time) as f32;
            frame_end_time = new_frame_end_time;
        }
        let entity = display_list.get(&entity_id);
        assert!((entity.unwrap().transform().rotation() - FRAC_PI_2).abs() < std::f32::EPSILON);
        let part_transform = match (entity.unwrap().parts())[0] {
            Part::Raster { transform, .. } => transform,
            Part::Vector { transform, .. } => transform,
        };
        assert_eq!(
            Vector2F::new(part_transform.m11(), part_transform.m22()),
            Vector2F::new(6.0, 15.0)
        );
    }

    mock! {
        pub Renderer { }
        trait Renderer {
            fn start_frame(&mut self, stage_size: Vector2F);
            fn set_background(&mut self, color: ColorU);
            fn draw_shape(&mut self, shape: &Shape, transform: Transform2F, color_override:  &Option<Coloring>, morph_index: f32);
            fn draw_raster(&mut self, bitmap: &Pattern, view_rect: RectF, transform: Transform2F, tint: Option<ColorU>);
            fn end_frame(&mut self);
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
            DisplayLibraryItem::Vector(Shape::Fill {
                edges: vec![
                    Edge::Line(Vector2F::new(-15.0, -15.0)),
                    Edge::Line(Vector2F::new(15.0, -15.0)),
                    Edge::Line(Vector2F::new(15.0, 15.0)),
                    Edge::Line(Vector2F::new(-15.0, 15.0)),
                ],
                color: ColorU::new(0, 255, 0, 255),
            }),
        );
        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        let mut root = Entity::new(
            root_id,
            0,
            "root",
            root_id,
            vec![],
            Transform2F::default(),
            0.0,
        );
        root.add_child(entity_id);
        display_list.insert(root_id, root);
        display_list.insert(
            entity_id,
            Entity::new(
                entity_id,
                1,
                "entity",
                root_id,
                vec![Part::new_vector(shape_id, Transform2F::default(), None)],
                Transform2F::default(),
                0.0,
            ),
        );
        let state = State {
            frame: 0,
            delta_time: 0.0,
            frame_end_time: time_seconds(),
            root_entity_id: root_id,
            background_color: ColorU::white(),
            running: true,
            seconds_per_frame: 0.016,
            stage_size: Vector2F::new(800.0, 600.0),
        };
        let mut seq = Sequence::new();
        let mut mock_renderer = MockRenderer::new();
        mock_renderer
            .expect_start_frame()
            .times(1)
            .return_const(())
            .in_sequence(&mut seq);
        mock_renderer
            .expect_set_background()
            .times(1)
            .with(eq(ColorU::white()))
            .return_const(())
            .in_sequence(&mut seq);
        mock_renderer
            .expect_draw_shape()
            .times(1)
            .withf(|drawn_shape, transform, color_override, morph_index| {
                let model_shape = Shape::Fill {
                    edges: vec![
                        Edge::Line(Vector2F::new(-15.0, -15.0)),
                        Edge::Line(Vector2F::new(15.0, -15.0)),
                        Edge::Line(Vector2F::new(15.0, 15.0)),
                        Edge::Line(Vector2F::new(-15.0, 15.0)),
                    ],
                    color: ColorU::new(0, 255, 0, 255),
                };
                drawn_shape == &model_shape
                    && *transform == Transform2F::default()
                    && *color_override == None
                    && morph_index.abs() < std::f32::EPSILON
            })
            .return_const(())
            .in_sequence(&mut seq);
        mock_renderer
            .expect_end_frame()
            .times(1)
            .return_const(())
            .in_sequence(&mut seq);
        draw_frame(&mut mock_renderer, &state, &display_list, &library).unwrap();
    }
}
