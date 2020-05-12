#![deny(clippy::all)]
use super::types::{
    model::{DisplayLibraryItem, Entity, Part},
    shapes::{Coloring, Shape},
};
use pathfinder_color::ColorU;
use pathfinder_content::pattern::Pattern;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::BuildHasher;
use uuid::Uuid;

pub trait Renderer {
    fn start_frame(&mut self, stage_size: Vector2F);
    fn set_background(&mut self, color: ColorU);
    fn draw_shape(
        &mut self,
        shape: &Shape,
        transform: Transform2F,
        color_override: &Option<Coloring>,
        morph_index: f32,
    );
    fn draw_raster(
        &mut self,
        pattern: &Pattern,
        view_rect: RectF,
        transform: Transform2F,
        tint: Option<ColorU>,
    ); //TODO: filters?
    fn end_frame(&mut self);
}
#[derive(Debug)]
pub struct PaintData<'a> {
    depth_list: BTreeMap<u64, &'a Entity>,
    world_space_transforms: HashMap<Uuid, Transform2F>,
}

impl<'a> PaintData<'a> {
    pub fn new(
        depth_list: BTreeMap<u64, &'a Entity>,
        world_space_transforms: HashMap<Uuid, Transform2F>,
    ) -> PaintData<'a> {
        PaintData {
            depth_list,
            world_space_transforms,
        }
    }
}

pub fn adjust_depth(depth: u32, depth_list: &BTreeMap<u64, &Entity>) -> u64 {
    let mut depth = (depth as u64) << 32;
    while depth_list.contains_key(&depth) {
        depth += 1;
    }
    depth
}

//TODO: add a concept of "dirty/clean" entities. Ideally we would structure our renderer to do partial rerenders
pub fn compute_render_data<'a, S: BuildHasher>(
    root_entity_id: &Uuid,
    display_list: &'a HashMap<Uuid, Entity, S>,
) -> Result<PaintData<'a>, String> {
    use std::collections::VecDeque;
    let mut depth_list: BTreeMap<u64, &'a Entity> = BTreeMap::new();
    let mut world_space_transforms: HashMap<Uuid, Transform2F> = HashMap::new();
    let root = display_list.get(root_entity_id);
    if root.is_none() {
        return Err("Root Entity unloaded.".to_string());
    }
    let root = root.unwrap();
    world_space_transforms.insert(*root.id(), *root.transform());
    let mut nodes = VecDeque::new();
    nodes.push_back(root);
    while let Some(node) = nodes.pop_front() {
        for child_id in node.children() {
            if let Some(child) = display_list.get(child_id) {
                nodes.push_back(child);
            }
        }
        let depth = adjust_depth(node.depth(), &depth_list);
        depth_list.insert(depth, node);
        if let Some(parent_transform) = world_space_transforms.get(node.parent()) {
            let parent_transform = *parent_transform;
            world_space_transforms.insert(*node.id(), parent_transform * *node.transform());
        } else {
            return Err(format!(
                "Could not find parent {} of entity {} in world_space_transforms",
                node.parent(),
                node.id()
            ));
        }
    }
    Ok(PaintData {
        depth_list,
        world_space_transforms,
    })
}

pub fn paint<S: BuildHasher>(
    renderer: &mut impl Renderer,
    paint_data: PaintData,
    library: &HashMap<Uuid, DisplayLibraryItem, S>,
) {
    //Render from back to front (TODO: Does Pathfinder work better front to back or back to front?)
    for entity in paint_data.depth_list.values() {
        let world_space_transform = paint_data.world_space_transforms.get(entity.id()).unwrap();
        for part in entity.parts() {
            match part {
                Part::Vector {
                    item_id,
                    transform,
                    color,
                } => {
                    if let Some(&DisplayLibraryItem::Vector(ref shape)) = library.get(&item_id) {
                        renderer.draw_shape(
                            shape,
                            *world_space_transform * *transform,
                            color,
                            entity.morph_index(),
                        );
                    }
                }
                Part::Raster {
                    item_id,
                    transform,
                    view_rect,
                    tint,
                } => {
                    if let Some(&DisplayLibraryItem::Raster(ref bitmap)) = library.get(&item_id) {
                        renderer.draw_raster(
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
}
