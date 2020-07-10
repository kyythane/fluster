#![deny(clippy::all)]

use super::actions::{
    Action, ActionList, EntityDefinition, EntityUpdateDefinition, PartDefinition,
    PartDefinitionPayload,
};
use super::rendering::{compute_render_data, paint, Renderer};
use super::types::model::{Entity, Part};
use crate::{
    ecs::resources::{FrameTime, Library},
    engine::Engine,
};
use pathfinder_color::ColorU;
use pathfinder_geometry::{rect::RectF, vector::Vector2F};
use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};
use streaming_iterator::StreamingIterator;
use uuid::Uuid;

pub struct State {
    frame_duration: Duration,
    delta_time: Duration,
    frame: u32,
    frame_end_time: Instant,
    background_color: ColorU,
    running: bool,
    stage_size: Vector2F,
    //TODO: pause
}

impl State {
    pub fn new(background_color: ColorU, frame_duration: Duration, stage_size: Vector2F) -> State {
        State {
            frame: 0,
            frame_end_time: Instant::now(),
            delta_time: Duration::from_millis(0),
            frame_duration,
            background_color,
            running: true,
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

pub fn play(
    renderer: &mut impl Renderer,
    actions: &mut ActionList,
    on_frame_complete: &mut dyn FnMut(State) -> State,
    frame_duration: Duration,
    stage_size: Vector2F,
) -> Result<(), String> {
    let (root_container_id, mut state, library) = initialize(actions, frame_duration, stage_size)?;
    let mut engine = Engine::new(root_container_id, library);

    while let Some(_) = actions.get() {
        if !state.running {
            break;
        }
        state = execute_actions(state, actions, &mut engine)?;
        if let Some(Action::PresentFrame(start, count)) = actions.get() {
            if *count == 0 {
                continue; //Treat PresentFrame(_, 0) as a no-op
            } else if state.frame > start + count {
                return Err("Attempting to play incorrect frame. Frame counter and action list have gotten desynced".to_string());
            } else {
                let start = state.frame - *start;
                for frame in 0..(*count - start) {
                    //TODO: skip updates/paints to catch up to frame rate if we are lagging
                    let frame_time = FrameTime {
                        delta_frame: 1,
                        delta_time: state.delta_time,
                    };
                    engine.update(frame_time);
                    draw_frame(renderer, &state, &display_list, &library, &scene_data)?;
                    state = on_frame_complete(state);
                    if !state.running {
                        break;
                    }
                    handle_frame_delta(&mut state);
                }
            }
        }
        actions.advance();
    }
    Ok(())
}

fn handle_frame_delta(state: &mut State) {
    let frame_end_time = Instant::now();
    let frame_time_left = state.frame_duration - (frame_end_time - state.frame_end_time);
    println!(
        "frame {:?} time {:?}, {:?}% of target ",
        state.frame,
        (frame_end_time - state.frame_end_time),
        (frame_end_time - state.frame_end_time).div_duration_f32(state.frame_duration) * 100.0
    );
    let frame_end_time = if frame_time_left.as_millis() > 0 {
        thread::sleep(frame_time_left);
        Instant::now()
    } else {
        frame_end_time
    };
    state.delta_time = frame_end_time - state.frame_end_time;
    state.frame_end_time = frame_end_time;
    state.frame = state.frame + 1;
}

fn update_tweens(elapsed: f32, display_list: &mut HashMap<Uuid, Entity>) -> Result<(), String> {
    for (_, entity) in display_list.iter_mut() {
        entity.update_tweens(elapsed)?;
    }
    Ok(())
}

fn initialize(
    actions: &mut ActionList,
    frame_duration: Duration,
    stage_size: Vector2F,
) -> Result<(Uuid, State, Library), String> {
    let mut library: Library = Library::default();
    let mut root_entity_id: Option<Uuid> = None;
    let mut background_color = ColorU::white();
    while let Some(action) = actions.get_mut() {
        match action {
            Action::CreateRoot(id) => {
                root_entity_id = Some(*id);
            }
            Action::DefineShape { id, shape } => {
                if !library.contains_shape(id) {
                    library.add_shape(*id, shape.clone());
                }
            }
            Action::LoadBitmap { id, ref mut bitmap } => {
                // Note: this is destructive to the source bitmap. Bitmaps can be very large, and library loads are idempotent
                if !library.contains_texture(id) {
                    library.add_texture(*id, bitmap.release_contents());
                }
            }
            Action::SetBackground { color } => background_color = *color,
            Action::EndInitialization => break,
            _ => return Err("Unexpected action in initialization".to_string()),
        }
        actions.advance();
    }

    if let Some(root_entity_id) = root_entity_id {
        Ok((
            root_entity_id,
            State::new(background_color, frame_duration, stage_size),
            library,
        ))
    } else {
        Err("Action list did not define a root element".to_string())
    }
}

fn execute_actions(
    state: State,
    actions: &mut ActionList,
    engine: &mut Engine,
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
            Action::RemoveContainer(id, recursive) => {
                if *recursive {
                    engine.remove_container(id)
                } else {
                    engine.remove_container_and_children(id)
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

fn build_part(
    definition: &PartDefinition,
    library: &HashMap<Uuid, DisplayLibraryItem>,
) -> Option<(Uuid, Part)> {
    let mut color = None;
    let mut tint = None;
    let mut view_rect = None;
    for definition_payload in definition.payload() {
        match definition_payload {
            PartDefinitionPayload::ViewRect(rect) => {
                view_rect = Some(*rect);
            }
            PartDefinitionPayload::Coloring(coloring) => {
                color = Some(coloring.clone());
            }
            PartDefinitionPayload::Tint(new_tint) => {
                tint = Some(*new_tint);
            }
        }
    }
    match library.get(definition.item_id()) {
        Some(DisplayLibraryItem::Vector(..)) => Some((
            *definition.part_id(),
            Part::new_vector(*definition.item_id(), definition.transform(), color),
        )),
        Some(DisplayLibraryItem::Raster(pattern)) => {
            let view_rect = if let Some(points) = view_rect {
                RectF::from_points(points.origin, points.lower_right)
            } else {
                RectF::from_points(Vector2F::zero(), pattern.size().to_f32())
            };
            Some((
                *definition.part_id(),
                Part::new_raster(
                    *definition.item_id(),
                    view_rect,
                    definition.transform(),
                    tint,
                ),
            ))
        }
        None => None,
    }
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
                    .map(|definition| build_part(definition, library))
                    .filter(|e| e.is_some())
                    .map(|e| e.unwrap())
                    .collect::<HashMap<Uuid, Part>>();
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
        state.seconds_per_frame * (entity_update_definition.duration_frames() as f32);
    if let Some(entity) = display_list.get_mut(&entity_update_definition.id()) {
        entity.add_tweens(entity_update_definition, duration_seconds, library)?;
    }
    Ok(()) //Updating a removed entity or part is a no-op, since a script or event could remove an entity in a way the editor can't account for.
}

fn draw_frame(
    renderer: &mut impl Renderer,
    state: &State,
    display_list: &HashMap<Uuid, Entity>,
    library: &HashMap<Uuid, DisplayLibraryItem>,
    scene_data: &SceneData,
) -> Result<(), String> {
    renderer.start_frame(state.stage_size);
    renderer.set_background(state.background_color);
    let render_data = compute_render_data(&state.root_entity_id, display_list)?;
    paint(renderer, library, render_data, scene_data);
    renderer.end_frame();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{
        EntityUpdatePayload, PartDefinition, PartUpdateDefinition, PartUpdatePayload,
    };
    use crate::tween::Easing;
    use crate::types::basic::ScaleRotationTranslation;
    use crate::types::coloring::Coloring;
    use crate::types::shapes::Edge;
    use mockall::predicate::*;
    use mockall::*;
    use pathfinder_content::{pattern::Pattern, stroke::StrokeStyle};
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
        let part_id = Uuid::parse_str("1c3ad65b-ebbf-485a-bd79-9ba336e9248c").unwrap();
        let part_id_2 = Uuid::parse_str("3ec76e6a-7758-47bf-bd79-9ba336e9248c").unwrap();
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
                parts: vec![PartDefinition::new(
                    part_id,
                    shape_id,
                    ScaleRotationTranslation::default(),
                    vec![],
                )],
                parent: None,
                morph_index: 0.0,
            }),
            Action::AddEntity(EntityDefinition {
                id: entity2_id,
                name: String::from("second"),
                transform: Transform2F::default(),
                depth: 3,
                parts: vec![PartDefinition::new(
                    part_id_2,
                    shape_id,
                    ScaleRotationTranslation::default(),
                    vec![],
                )],
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
        display_list.insert(root_id, Entity::create_root(root_id));
        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        let mut state = State {
            frame: 0,
            delta_time: Duration::from_millis(16.0),
            frame_end_time: time_seconds(),
            root_entity_id: root_id,
            background_color: ColorU::white(),
            running: true,
            seconds_per_frame: 0.016,
            stage_size: Vector2F::new(800.0, 600.0),
        };
        let mut scene_data = SceneData::new();
        state = execute_actions(
            state,
            &mut action_list,
            &mut display_list,
            &mut library,
            &mut scene_data,
        )
        .unwrap();
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
        let part_id = Uuid::parse_str("cfc4e1a4-5623-485a-8943-28b94df19361").unwrap();

        let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
        let mut root = Entity::create_root(root_id);
        root.add_child(entity_id);
        display_list.insert(root_id, root);
        let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
        library.insert(
            shape_id,
            DisplayLibraryItem::Vector(Shape::Path {
                edges: vec![],
                color: ColorU::white(),
                stroke_style: StrokeStyle::default(),
            }),
        );
        let mut parts = HashMap::new();
        parts.insert(
            part_id,
            Part::new_vector(shape_id, Transform2F::default(), None),
        );
        let mut entity = Entity::new(
            entity_id,
            1,
            "entity",
            root_id,
            parts,
            Transform2F::default(),
            0.0,
        );
        // easing: Some(Easing::CubicIn),
        entity
            .add_tweens(
                &EntityUpdateDefinition::new(
                    entity_id,
                    5,
                    vec![PartUpdateDefinition::new(
                        part_id,
                        Easing::Linear,
                        PartUpdatePayload::from_transform(&Transform2F::from_scale(Vector2F::new(
                            6.0, 15.0,
                        ))),
                    )],
                    vec![EntityUpdatePayload::from_transform(
                        &Transform2F::from_rotation(FRAC_PI_2),
                        Easing::CubicIn,
                    )],
                ),
                FRAME_TIME * 5.0,
                &library,
            )
            .unwrap();
        display_list.insert(entity_id, entity);

        let mut delta_time = 0.0;
        let mut frame_end_time = time_seconds();

        let sleep_duration = time::Duration::from_secs_f32(FRAME_TIME);
        for _ in 0..6 {
            update_tweens(delta_time, &mut display_list).unwrap();
            thread::sleep(sleep_duration);
            let new_frame_end_time = time_seconds();
            delta_time = (new_frame_end_time - frame_end_time) as f32;
            frame_end_time = new_frame_end_time;
        }
        let entity = display_list.get(&entity_id);
        assert!((entity.unwrap().transform().rotation() - FRAC_PI_2).abs() < std::f32::EPSILON);
        let part_transform = entity.unwrap().get_part(&part_id).unwrap().transform();
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
        let part_id = Uuid::parse_str("b06f8577-aa30-4000-bd79-88dc82e3e26f").unwrap();

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
        let mut root = Entity::create_root(root_id);
        root.add_child(entity_id);
        display_list.insert(root_id, root);
        let mut parts = HashMap::new();
        parts.insert(
            part_id,
            Part::new_vector(shape_id, Transform2F::default(), None),
        );
        display_list.insert(
            entity_id,
            Entity::new(
                entity_id,
                1,
                "entity",
                root_id,
                parts,
                Transform2F::from_translation(Vector2F::splat(200.0)),
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
                    && *transform == Transform2F::from_translation(Vector2F::splat(200.0))
                    && *color_override == None
                    && morph_index.abs() < std::f32::EPSILON
            })
            .return_const(())
            .in_sequence(&mut seq);
        let mut scene_data = SceneData::new();
        scene_data.recompute(&state.root_entity_id, &mut display_list, &library);
        mock_renderer
            .expect_end_frame()
            .times(1)
            .return_const(())
            .in_sequence(&mut seq);
        draw_frame(
            &mut mock_renderer,
            &state,
            &display_list,
            &library,
            &scene_data,
        )
        .unwrap();
    }
}
