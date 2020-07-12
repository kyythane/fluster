use crate::{
    actions::{ContainerCreationDefintition, ContainerUpdateDefintition},
    types::shapes::Shape,
};
use aabb_quadtree_pathfinder::{QuadTree, RectF};
use pathfinder_content::pattern::Pattern;
use serde::{Deserialize, Serialize};
use specs::Entity;
use std::collections::{hash_map::RandomState, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Default, Debug)]
pub struct ContainerCreationQueue {
    container_data: VecDeque<ContainerCreationDefintition>,
}

impl ContainerCreationQueue {
    pub fn enqueue(&mut self, definition: ContainerCreationDefintition) {
        self.container_data.push_back(definition);
    }

    pub fn dequeue(&mut self) -> Option<ContainerCreationDefintition> {
        self.container_data.pop_front()
    }
}

#[derive(Default, Debug)]
pub struct ContainerUpdateQueue {
    container_data: VecDeque<ContainerUpdateDefintition>,
}

impl ContainerUpdateQueue {
    pub fn enqueue(&mut self, definition: ContainerUpdateDefintition) {
        self.container_data.push_back(definition);
    }

    pub fn dequeue(&mut self) -> Option<ContainerUpdateDefintition> {
        self.container_data.pop_front()
    }
}

#[derive(Default, Debug)]
pub struct Library {
    shapes: HashMap<Uuid, Arc<Shape>>,
    textures: HashMap<Uuid, Arc<Pattern>>,
}

impl Library {
    pub fn add_shape(&mut self, uuid: Uuid, shape: Shape) {
        self.shapes.insert(uuid, Arc::new(shape));
    }

    pub fn add_texture(&mut self, uuid: Uuid, pattern: Pattern) {
        self.textures.insert(uuid, Arc::new(pattern));
    }

    pub fn get_shape(&self, uuid: &Uuid) -> Option<Arc<Shape>> {
        self.shapes.get(uuid).cloned()
    }

    pub fn get_texture(&self, uuid: &Uuid) -> Option<Arc<Pattern>> {
        self.textures.get(uuid).cloned()
    }

    pub fn remove_shape(&mut self, uuid: &Uuid) {
        self.shapes.remove(uuid);
    }

    pub fn remove_texture(&mut self, uuid: &Uuid) {
        self.textures.remove(uuid);
    }

    pub fn contains_shape(&self, uuid: &Uuid) -> bool {
        self.shapes.contains_key(uuid)
    }

    pub fn contains_texture(&self, uuid: &Uuid) -> bool {
        self.textures.contains_key(uuid)
    }
}

#[derive(Default, Debug)]
pub struct ContainerMapping {
    container_to_entity: HashMap<Uuid, Entity>,
    entity_to_container: HashMap<Entity, Uuid>,
}

impl ContainerMapping {
    pub fn add_container(&mut self, container_id: Uuid, entity: Entity) {
        self.container_to_entity.insert(container_id, entity);
        self.entity_to_container.insert(entity, container_id);
    }

    pub fn remove_container(&mut self, container_id: &Uuid) {
        self.container_to_entity
            .remove(container_id)
            .and_then(|removed_entity| self.entity_to_container.remove(&removed_entity));
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        self.entity_to_container
            .remove(entity)
            .and_then(|removed_container| self.container_to_entity.remove(&removed_container));
    }

    pub fn get_container(&self, entity: &Entity) -> Option<&Uuid> {
        self.entity_to_container.get(entity)
    }

    pub fn get_entity(&self, container_id: &Uuid) -> Option<&Entity> {
        self.container_to_entity.get(container_id)
    }

    pub fn contains_container(&self, container_id: &Uuid) -> bool {
        self.container_to_entity.contains_key(container_id)
    }

    pub fn contains_entity(&self, entity: &Entity) -> bool {
        self.entity_to_container.contains_key(entity)
    }
}

pub type QuadTreeLayer = u32;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuadTreeLayerOptions {
    dilation: f32,
}

impl QuadTreeLayerOptions {
    pub fn new(dilation: f32) -> Self {
        Self { dilation }
    }

    pub fn dilation(&self) -> f32 {
        self.dilation
    }
}

#[derive(Default, Debug)]
pub struct QuadTrees(HashMap<QuadTreeLayer, (QuadTree<Entity, RandomState>, QuadTreeLayerOptions)>);

impl QuadTrees {
    pub fn create_quad_tree(
        &mut self,
        layer: QuadTreeLayer,
        bounds: RectF,
        options: QuadTreeLayerOptions,
    ) {
        self.0.insert(
            layer,
            (QuadTree::default(bounds, RandomState::new()), options),
        );
    }

    pub fn remove_quad_tree(&mut self, layer: &QuadTreeLayer) {
        self.0.remove(layer);
    }

    pub fn update(&mut self, entity: Entity, layer: QuadTreeLayer, aabb: RectF) {
        self.0.get_mut(&layer).and_then(|(tree, options)| {
            let dialated = aabb.dilate(options.dilation());
            tree.remove(&entity);
            tree.insert(entity, dialated);
            Some(())
        });
    }

    pub fn remove_from_layer(&mut self, layer: &QuadTreeLayer, entity: Entity) {
        self.0
            .get_mut(&layer)
            .and_then(|(tree, _)| tree.remove(&entity));
    }

    pub fn remove_all_layers(&mut self, entity: Entity) {
        self.0.iter_mut().for_each(|(_, (tree, _))| {
            tree.remove(&entity);
        });
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct FrameTime {
    pub delta_time: Duration,
    pub delta_frame: u32,
    // Other frame time data will *eventually* live here
}

#[derive(Debug)]
pub struct SceneGraph {
    root: Entity,
    tree: HashMap<Entity, Vec<Entity>>,
    parents: HashMap<Entity, Entity>,
}

impl SceneGraph {
    pub fn new(root: Entity) -> Self {
        let mut tree = HashMap::new();
        tree.insert(root, vec![]);
        let mut parents = HashMap::new();
        parents.insert(root, root);
        Self {
            root,
            tree,
            parents,
        }
    }

    pub fn add_entity(&mut self, parent: &Entity, entity: &Entity) {
        self.tree
            .entry(*parent)
            .and_modify(|children| children.push(*entity));
        self.parents.insert(*entity, *parent);
        self.tree.insert(*entity, vec![]);
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        let parent = self.parents.remove(entity);
        if let Some(children) = self.tree.remove(entity) {
            let parent = parent.unwrap();
            for child in children.iter() {
                self.parents.insert(*child, parent);
            }
            self.tree
                .entry(parent)
                .and_modify(|existing_children| existing_children.extend(children));
        }
    }

    pub fn reparent(&mut self, new_parent: &Entity, entity: Entity) {
        if let Some(old_parent) = self.parents.get(&entity) {
            self.tree
                .entry(*old_parent)
                .and_modify(|children| children.retain(|child| child != &entity));
        }
        self.parents.insert(entity, *new_parent);
        self.tree.entry(*new_parent).or_default().push(entity);
    }

    pub fn remove_entity_and_children(&mut self, entity: &Entity) -> Vec<Entity> {
        let mut queue = VecDeque::new();
        let mut removed = vec![];
        queue.push_back(*entity);
        removed.push(*entity);
        while let Some(next) = queue.pop_front() {
            self.tree
                .remove(&next)
                .unwrap_or_default()
                .into_iter()
                .for_each(|entity| {
                    removed.push(entity);
                    queue.push_back(entity);
                });
            self.parents.remove(&next);
        }
        removed
    }

    pub fn get_parent(&self, entity: &Entity) -> Option<&Entity> {
        self.parents.get(entity)
    }

    pub fn get_children(&self, entity: &Entity) -> Option<&Vec<Entity>> {
        self.tree.get(entity)
    }

    pub fn root(&self) -> &Entity {
        &self.root
    }

    pub fn get_parent_iter<'a>(&'a self, entity: &'a Entity) -> ParentIterator<'a> {
        ParentIterator {
            graph: self,
            current: entity,
        }
    }
}

pub struct ParentIterator<'a> {
    graph: &'a SceneGraph,
    current: &'a Entity,
}

impl<'a> Iterator for ParentIterator<'a> {
    type Item = &'a Entity;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.graph.parents.get(self.current);
        if let Some(next) = next {
            if next != self.graph.root() {
                self.current = next;
                Some(self.current)
            } else {
                None
            }
        } else {
            None
        }
    }
}
