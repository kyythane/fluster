use specs::Entity;
use std::collections::HashMap;
use std::time::Duration;

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

    pub fn add_entity(&mut self, parent: &Entity, entity: &Entity) {}

    pub fn remove_entity(&mut self, entity: &Entity) {
        let parent = self.parents.remove(entity);
        if let Some(children) = self.tree.remove(entity) {
            let parent = parent.unwrap();
            for child in children {
                self.parents.insert(child, parent);
            }
            self.tree
                .entry(parent)
                .and_modify(|existing_children| existing_children.extend(children));
        }
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
            self.current = next;
            Some(self.current)
        } else {
            None
        }
    }
}
