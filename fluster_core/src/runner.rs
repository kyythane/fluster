#![deny(clippy::all)]
//TODO: break into smaller files... again... somehow

use super::actions::{
    Action, ActionList, EntityDefinition, EntityUpdateDefinition, PartDefinition,
    PartUpdateDefinition, RectPoints, ScaleRotationTranslation,
};
use super::rendering::{Bitmap, Renderer, Shape};
use super::tween::{Easing, Tween};
use pathfinder_color::{ColorF, ColorU};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::collections::HashMap;
use std::mem;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use streaming_iterator::StreamingIterator;
use uuid::Uuid;

enum DisplayLibraryItem {
    Vector(Shape),
    Bitmap(Bitmap),
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

impl Part {
    fn item_id(&self) -> &Uuid {
        match self {
            Part::Vector { item_id, .. } => item_id,
            Part::Bitmap { item_id, .. } => item_id,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct LerpTransform {
    pub scale: Vector2F,
    //https://gamedev.stackexchange.com/questions/72348/how-do-i-lerp-between-values-that-loop-such-as-hue-or-rotation
    pub angle: Vector2F,
    pub translation: Vector2F,
}

impl LerpTransform {
    pub fn from_transform(transform: &Transform2F) -> LerpTransform {
        let theta = transform.rotation();
        LerpTransform {
            scale: Vector2F::new(
                Vector2F::new(transform.m11(), transform.m21()).length(),
                Vector2F::new(transform.m21(), transform.m22()).length(),
            ),
            angle: Vector2F::new(theta.cos(), theta.sin()),
            translation: transform.translation(),
        }
    }

    pub fn from_scale_rotation_translation(transform: &ScaleRotationTranslation) -> LerpTransform {
        LerpTransform {
            scale: transform.scale,
            angle: Vector2F::new(transform.theta.cos(), transform.theta.sin()),
            translation: transform.translation,
        }
    }
}

#[derive(Clone, Debug)]
struct PropertyTween {
    data: PropertyTweenData,
    elapsed_seconds: f32,
}

#[derive(Clone, Debug)]
enum PropertyTweenData {
    Color {
        start: ColorF,
        end: ColorF,
        duration_seconds: f32,
        easing: Easing,
    },
    Transform {
        start: LerpTransform,
        end: LerpTransform,
        duration_seconds: f32,
        easing: Easing,
    },
    ViewRect {
        start: RectPoints,
        end: RectPoints,
        duration_seconds: f32,
        easing: Easing,
    },
}

impl PropertyTween {
    pub fn new_color(
        start: ColorU,
        end: ColorU,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Color {
                start: start.to_f32(),
                end: end.to_f32(),
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }

    pub fn new_transform(
        start: LerpTransform,
        end: LerpTransform,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::Transform {
                start,
                end,
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }

    pub fn new_view_rect(
        start: RectPoints,
        end: RectPoints,
        duration_seconds: f32,
        easing: Easing,
    ) -> PropertyTween {
        PropertyTween {
            data: PropertyTweenData::ViewRect {
                start,
                end,
                duration_seconds,
                easing,
            },
            elapsed_seconds: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
enum PropertyTweenUpdate {
    Color(ColorU),
    Transform(Transform2F),
    ViewRect(RectF),
}

impl Tween for PropertyTween {
    type Item = PropertyTweenUpdate;

    fn update(&mut self, delta_time: f32) -> Self::Item {
        self.elapsed_seconds += delta_time;
        match &self.data {
            PropertyTweenData::Color {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::Color(start.lerp(*end, value).to_u8())
            }
            PropertyTweenData::Transform {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                let angle_vector = start.angle.lerp(end.angle, value);
                PropertyTweenUpdate::Transform(Transform2F::from_scale_rotation_translation(
                    start.scale.lerp(end.scale, value),
                    f32::atan2(angle_vector.y(), angle_vector.x()),
                    start.translation.lerp(end.translation, value),
                ))
            }
            PropertyTweenData::ViewRect {
                start,
                end,
                duration_seconds,
                easing,
            } => {
                let value = easing.ease(self.elapsed_seconds / duration_seconds);
                PropertyTweenUpdate::ViewRect(RectF::from_points(
                    start.origin.lerp(end.origin, value),
                    start.lower_right.lerp(end.lower_right, value),
                ))
            }
        }
    }
    fn is_complete(&self) -> bool {
        match self.data {
            PropertyTweenData::Color {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
            PropertyTweenData::Transform {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
            PropertyTweenData::ViewRect {
                duration_seconds, ..
            } => self.elapsed_seconds >= duration_seconds,
        }
    }
    fn easing(&self) -> Easing {
        match self.data {
            PropertyTweenData::Color { easing, .. } => easing,
            PropertyTweenData::Transform { easing, .. } => easing,
            PropertyTweenData::ViewRect { easing, .. } => easing,
        }
    }
}

//TODO: Bounding boxes and hit tests for mouse interactions
#[derive(Clone, Debug)]
struct Entity {
    active: bool,
    children: Vec<Uuid>,
    depth: u32,
    id: Uuid,
    name: String,
    parent: Uuid,
    parts: Vec<Part>,
    transform: Transform2F,
    tweens: HashMap<Uuid, Vec<PropertyTween>>,
}

impl PartialEq for Entity {
    //Tweens are ignored for the purpose of equality
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.children == other.children
            && self.depth == other.depth
            && self.name == other.name
            && self.parent == other.parent
            && self.parts == other.parts
            && self.transform == other.transform
    }
}

impl Entity {
    fn new(
        id: Uuid,
        depth: u32,
        name: String,
        parent: Uuid,
        parts: Vec<Part>,
        transform: Transform2F,
    ) -> Entity {
        Entity {
            active: true,
            children: vec![],
            depth,
            id,
            name,
            parent,
            parts,
            transform,
            tweens: HashMap::new(),
        }
    }

    fn add_child(&mut self, child: Uuid) {
        self.children.push(child);
    }
}

pub struct State {
    seconds_per_frame: f32,
    frame: u32,
    delta_time: f32,
    frame_end_time: f64,
    root_entity_id: Uuid,
    background_color: ColorU,
    running: bool,
    stage_size: Vector2F,
}

impl State {
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
    seconds_per_frame: f32,
    stage_size: Vector2F,
) -> Result<(), String> {
    let mut display_list: HashMap<Uuid, Entity> = HashMap::new();
    let mut library: HashMap<Uuid, DisplayLibraryItem> = HashMap::new();
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
            } else if state.frame > *start + *count {
                return Err("Attempting to play incorrect frame. Frame counter and action list have gotten desynced".to_string());
            } else {
                for frame in 0..*count {
                    state.frame = *start + frame;
                    //TODO: skip updates/paints to catch up to frame rate if we are lagging
                    //TODO: handle input
                    //TODO: scripts
                    update_tweens(state.delta_time, &mut display_list);
                    paint(renderer, &state, &display_list, &library)?;
                    state = on_frame_complete(state);
                    if !state.running {
                        break;
                    }
                    let frame_end_time = time_seconds();
                    let frame_time_left =
                        state.seconds_per_frame - (frame_end_time - state.frame_end_time) as f32;
                    let frame_end_time = if frame_time_left > 0.0 {
                        thread::sleep(Duration::from_secs_f32(frame_time_left));
                        time_seconds()
                    } else {
                        frame_end_time
                    };
                    state.delta_time = (frame_end_time - state.frame_end_time) as f32;
                    state.frame_end_time = frame_end_time;
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
        if entity.tweens.is_empty() {
            continue;
        }
        for (key, tweens) in entity.tweens.iter_mut() {
            if key == &entity.id {
                for update in tweens.iter_mut().map(|tween| tween.update(elapsed)) {
                    if let PropertyTweenUpdate::Transform(transform) = update {
                        entity.transform = transform;
                    }
                }
                tweens.retain(|tween| !tween.is_complete());
            } else if let Some(part) = entity.parts.iter_mut().find(|p| p.item_id() == key) {
                let (new_transform, new_color, new_view_rect) = tweens
                    .iter_mut()
                    .map(|tween| {
                        let update = tween.update(elapsed);
                        match update {
                            PropertyTweenUpdate::Transform(transform) => {
                                (Some(transform), None, None)
                            }
                            PropertyTweenUpdate::Color(color) => (None, Some(color), None),
                            PropertyTweenUpdate::ViewRect(view_rect) => {
                                (None, None, Some(view_rect))
                            }
                        }
                    })
                    .fold((None, None, None), |acc, x| {
                        let t = if x.0.is_some() { x.0 } else { acc.0 };
                        let c = if x.1.is_some() { x.1 } else { acc.1 };
                        let v = if x.2.is_some() { x.2 } else { acc.2 };
                        (t, c, v)
                    });
                let new_part = match part {
                    Part::Vector {
                        item_id,
                        transform,
                        color,
                    } => Part::Vector {
                        item_id: *item_id,
                        transform: if let Some(new_transform) = new_transform {
                            new_transform
                        } else {
                            *transform
                        },
                        color: if new_color.is_some() {
                            new_color
                        } else {
                            *color
                        },
                    },
                    Part::Bitmap {
                        item_id,
                        transform,
                        view_rect,
                        tint,
                    } => Part::Bitmap {
                        item_id: *item_id,
                        transform: if let Some(new_transform) = new_transform {
                            new_transform
                        } else {
                            *transform
                        },
                        tint: if new_color.is_some() {
                            new_color
                        } else {
                            *tint
                        },
                        view_rect: if let Some(new_view_rect) = new_view_rect {
                            new_view_rect
                        } else {
                            *view_rect
                        },
                    },
                };
                mem::replace(part, new_part);
                tweens.retain(|tween| !tween.is_complete());
            }
        }
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
        let item = DisplayLibraryItem::Bitmap(bitmap.release_contents());
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
                let root = Entity::new(
                    *id,
                    0,
                    String::from("root"),
                    *id,
                    vec![],
                    Transform2F::default(),
                );
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
        Ok(State {
            frame: 0,
            delta_time: 0.0,
            frame_end_time: time_seconds(),
            root_entity_id,
            background_color,
            running: true,
            seconds_per_frame,
            stage_size,
        })
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
                    for c in old.children {
                        display_list.remove(&c);
                    }
                    let parent = display_list.get_mut(&old.parent).unwrap();
                    parent.children.retain(|elem| elem != id);
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
    let (id, name, transform, depth, parts, parent) = match entity_definition {
        EntityDefinition {
            id,
            name,
            transform,
            depth,
            parts,
            parent,
        } => (*id, name, transform, *depth, parts, *parent),
    };

    let parent = match parent {
        Some(id) => id,
        None => state.root_entity_id,
    };
    match display_list.get_mut(&parent) {
        Some(parent_entity) => {
            let transform = Transform2F::from_scale_rotation_translation(
                transform.scale,
                transform.theta,
                transform.translation,
            );
            let parts = {
                let constructed = parts
                    .iter()
                    .map(|x| match x {
                        PartDefinition::Vector { item_id, transform } => {
                            match library.get(&item_id) {
                                Some(DisplayLibraryItem::Vector { .. }) => Some(Part::Vector {
                                    item_id: *item_id,
                                    transform: Transform2F::from_scale_rotation_translation(
                                        transform.scale,
                                        transform.theta,
                                        transform.translation,
                                    ),
                                    color: None,
                                }),
                                _ => None,
                            }
                        }
                        PartDefinition::Bitmap {
                            item_id,
                            transform,
                            view_rect,
                        } => match library.get(&item_id) {
                            Some(DisplayLibraryItem::Bitmap { .. }) => Some(Part::Bitmap {
                                item_id: *item_id,
                                transform: Transform2F::from_scale_rotation_translation(
                                    transform.scale,
                                    transform.theta,
                                    transform.translation,
                                ),
                                view_rect: RectF::from_points(
                                    view_rect.origin,
                                    view_rect.lower_right,
                                ),
                                tint: None,
                            }),
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
            let entity = Entity::new(id, depth, name.clone(), parent, parts, transform);
            parent_entity.add_child(id);
            if let Some(old) = display_list.insert(id, entity) {
                // If we replace this entitiy, clear the old children out of the display list.
                for c in old.children {
                    display_list.remove(&c);
                }
                if old.parent != parent {
                    let parent = display_list.get_mut(&old.parent).unwrap();
                    parent.children.retain(|elem| elem != &id);
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
        if let Some(end_transform) = &entity_update_definition.transform {
            let tween = if let Some(easing) = entity_update_definition.easing {
                PropertyTween::new_transform(
                    LerpTransform::from_transform(&entity.transform),
                    LerpTransform::from_scale_rotation_translation(end_transform),
                    duration_seconds,
                    easing,
                )
            } else {
                return Err(format!(
                    "Attempted to define tween for {} with no easing",
                    entity.id
                ));
            };
            match entity.tweens.get_mut(&entity.id) {
                Some(tweens) => tweens.push(tween),
                None => {
                    entity.tweens.insert(entity.id, vec![tween]);
                }
            };
        }
        for part_update in &entity_update_definition.part_updates {
            let update_item_id = part_update.item_id();
            if let Some(part) = entity.parts.iter().find(|p| p.item_id() == update_item_id) {
                if let Some(library_item) = library.get(update_item_id) {
                    let tweens =
                        create_part_tween(part, library_item, part_update, duration_seconds)?;
                    match entity.tweens.get_mut(part.item_id()) {
                        Some(existing_tweens) => existing_tweens.extend(tweens),
                        None => {
                            entity.tweens.insert(entity.id, tweens);
                        }
                    };
                }
            }
        }
    }
    Ok(()) //Updating a removed entity or part is a no-op, since a script or event could remove an entity in a way the editor can't account for.
}

fn create_part_tween(
    part: &Part,
    library_item: &DisplayLibraryItem,
    part_update: &PartUpdateDefinition,
    duration_seconds: f32,
) -> Result<Vec<PropertyTween>, String> {
    let mut tweens: Vec<PropertyTween> = vec![];
    match part_update {
        PartUpdateDefinition::Bitmap {
            tint: end_tint,
            easing,
            transform: end_transform,
            view_rect: end_view_rect,
            ..
        } => {
            if let Part::Bitmap {
                transform: start_transform,
                tint: start_tint,
                view_rect: start_view_rect,
                ..
            } = part
            {
                if let Some(end_tint) = end_tint {
                    if let Some(start_tint) = start_tint {
                        tweens.push(PropertyTween::new_color(
                            *start_tint,
                            *end_tint,
                            duration_seconds,
                            *easing,
                        ));
                    } else {
                        tweens.push(PropertyTween::new_color(
                            ColorU::white(),
                            *end_tint,
                            duration_seconds,
                            *easing,
                        ));
                    }
                }
                if let Some(end_transform) = end_transform {
                    tweens.push(PropertyTween::new_transform(
                        LerpTransform::from_transform(start_transform),
                        LerpTransform::from_scale_rotation_translation(end_transform),
                        duration_seconds,
                        *easing,
                    ));
                }
                if let Some(end_view_rect) = end_view_rect {
                    tweens.push(PropertyTween::new_view_rect(
                        RectPoints::from_rect(start_view_rect),
                        *end_view_rect,
                        duration_seconds,
                        *easing,
                    ));
                }
            } else {
                return Err(format!(
                    "Tried to apply Bitmap update to a Bector part {}",
                    part.item_id()
                ));
            }
        }
        PartUpdateDefinition::Vector {
            color: end_color,
            easing,
            transform: end_transform,
            ..
        } => {
            if let Part::Vector {
                transform: start_transform,
                color: start_color,
                ..
            } = part
            {
                if let Some(end_color) = end_color {
                    if let Some(start_color) = start_color {
                        tweens.push(PropertyTween::new_color(
                            *start_color,
                            *end_color,
                            duration_seconds,
                            *easing,
                        ));
                    } else if let DisplayLibraryItem::Vector(shape) = library_item {
                        if let Some(start_color) = shape.color() {
                            tweens.push(PropertyTween::new_color(
                                start_color,
                                *end_color,
                                duration_seconds,
                                *easing,
                            ));
                        }
                    } else {
                        return Err(format!(
                            "Vector part {} references a Bitmap object",
                            part.item_id()
                        ));
                    }
                }
                if let Some(end_transform) = end_transform {
                    tweens.push(PropertyTween::new_transform(
                        LerpTransform::from_transform(start_transform),
                        LerpTransform::from_scale_rotation_translation(end_transform),
                        duration_seconds,
                        *easing,
                    ));
                }
            } else {
                return Err(format!(
                    "Tried to apply Vector update to a Bitmap part {}",
                    part.item_id()
                ));
            }
        }
    }
    Ok(tweens)
}

fn paint(
    renderer: &mut impl Renderer,
    state: &State,
    display_list: &HashMap<Uuid, Entity>,
    library: &HashMap<Uuid, DisplayLibraryItem>,
) -> Result<(), String> {
    use std::collections::BTreeMap;
    use std::collections::VecDeque;
    let mut depth_list: BTreeMap<u64, &Entity> = BTreeMap::new();
    let mut world_space_transforms: HashMap<Uuid, Transform2F> = HashMap::new();
    let root = display_list.get(&state.root_entity_id);
    if root.is_none() {
        return Err("Root Entity unloaded.".to_string());
    }
    let root = root.unwrap();
    world_space_transforms.insert(root.id, root.transform);
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
            if let Some(parent_transform) = world_space_transforms.get(&node.parent) {
                let parent_transform = *parent_transform;
                world_space_transforms.insert(node.id, parent_transform * node.transform);
            } else {
                return Err(format!(
                    "Could not find parent {} of entity {} in world_space_transforms",
                    node.parent, node.id
                ));
            }
        }
    }
    renderer.start_frame(state.stage_size);
    renderer.set_background(state.background_color);
    //Render from back to front (TODO: Does Pathfinder work better front to back or back to front?)
    for entity in depth_list.values() {
        let world_space_transform = world_space_transforms.get(&entity.id).unwrap();
        for part in &entity.parts {
            match part {
                Part::Vector {
                    item_id,
                    transform,
                    color,
                } => {
                    if let Some(&DisplayLibraryItem::Vector(ref shape)) = library.get(&item_id) {
                        renderer.draw_shape(shape, *world_space_transform * *transform, *color);
                    }
                }
                Part::Bitmap {
                    item_id,
                    transform,
                    view_rect,
                    tint,
                } => {
                    if let Some(&DisplayLibraryItem::Bitmap(ref bitmap)) = library.get(&item_id) {
                        renderer.draw_bitmap(
                            bitmap,
                            *view_rect,
                            *world_space_transform * *transform,
                            *tint,
                        );
                    }
                }
            }
        }
    }
    renderer.end_frame();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ScaleRotationTranslation;
    use mockall::predicate::*;
    use mockall::*;
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
                shape: Shape::FillPath {
                    points: vec![
                        Vector2F::new(-15.0, -15.0),
                        Vector2F::new(15.0, -15.0),
                        Vector2F::new(15.0, 15.0),
                        Vector2F::new(-15.0, 15.0),
                    ],
                    color: ColorU::new(0, 255, 0, 255),
                },
            },
            Action::AddEntity(EntityDefinition {
                id: entity_id,
                name: String::from("first"),
                transform: scale_rotation_translation,
                depth: 2,
                parts: vec![PartDefinition::Vector {
                    item_id: shape_id,
                    transform: scale_rotation_translation,
                }],
                parent: None,
            }),
            Action::AddEntity(EntityDefinition {
                id: entity2_id,
                name: String::from("second"),
                transform: scale_rotation_translation,
                depth: 3,
                parts: vec![PartDefinition::Vector {
                    item_id: shape_id,
                    transform: scale_rotation_translation,
                }],
                parent: Some(entity_id),
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
                String::from("root"),
                root_id,
                vec![],
                Transform2F::default(),
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

    #[test]
    fn it_updates_tweens() {
        const FRAME_TIME: f32 = 1.0 / 60.0;
        let root_id = Uuid::parse_str("cfc4e1a4-5623-485a-bd79-88dc82e3e26f").unwrap();
        let entity_id = Uuid::parse_str("b06f8577-aa30-4000-9967-9ba336e9248c").unwrap();
        let shape_id = Uuid::parse_str("1c3ad65b-ebbf-4d5e-8943-28b94df19361").unwrap();

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
                tweens: HashMap::new(),
            },
        );
        let mut tweens: HashMap<Uuid, Vec<PropertyTween>> = HashMap::new();
        tweens.insert(
            entity_id,
            vec![PropertyTween::new_transform(
                LerpTransform::from_transform(&Transform2F::default()),
                LerpTransform::from_transform(&Transform2F::from_rotation(FRAC_PI_2)),
                FRAME_TIME * 5.0,
                Easing::CubicIn,
            )],
        );
        tweens.insert(
            shape_id,
            vec![PropertyTween::new_transform(
                LerpTransform::from_transform(&Transform2F::default()),
                LerpTransform::from_transform(&Transform2F::from_scale(Vector2F::new(6.0, 15.0))),
                FRAME_TIME * 5.0,
                Easing::Linear,
            )],
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
                tweens,
            },
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
        assert!((entity.unwrap().transform.rotation() - FRAC_PI_2).abs() < std::f32::EPSILON);
        let part_transform = match entity.unwrap().parts[0] {
            Part::Bitmap { transform, .. } => transform,
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
            fn draw_shape(&mut self, shape: &Shape, transform: Transform2F, color_override: Option<ColorU>);
            fn draw_bitmap(&mut self, bitmap: &Bitmap, view_rect: RectF, transform: Transform2F, tint: Option<ColorU>);
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
            DisplayLibraryItem::Vector(Shape::FillPath {
                points: vec![
                    Vector2F::new(-15.0, -15.0),
                    Vector2F::new(15.0, -15.0),
                    Vector2F::new(15.0, 15.0),
                    Vector2F::new(-15.0, 15.0),
                ],
                color: ColorU::new(0, 255, 0, 255),
            }),
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
                tweens: HashMap::new(),
            },
        );
        display_list.insert(
            entity_id,
            Entity::new(
                entity_id,
                1,
                String::from("entity"),
                root_id,
                vec![Part::Vector {
                    item_id: shape_id,
                    transform: Transform2F::default(),
                    color: None,
                }],
                Transform2F::default(),
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
            .withf(|drawn_shape, transform, color_override| {
                let model_shape = Shape::FillPath {
                    points: vec![
                        Vector2F::new(-15.0, -15.0),
                        Vector2F::new(15.0, -15.0),
                        Vector2F::new(15.0, 15.0),
                        Vector2F::new(-15.0, 15.0),
                    ],
                    color: ColorU::new(0, 255, 0, 255),
                };
                drawn_shape == &model_shape
                    && *transform == Transform2F::default()
                    && *color_override == None
            })
            .return_const(())
            .in_sequence(&mut seq);
        mock_renderer
            .expect_end_frame()
            .times(1)
            .return_const(())
            .in_sequence(&mut seq);
        paint(&mut mock_renderer, &state, &display_list, &library).unwrap();
    }
}
