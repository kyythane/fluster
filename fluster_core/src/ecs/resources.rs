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
}
