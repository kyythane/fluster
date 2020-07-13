#![deny(missing_docs)]
#![deny(clippy::all)]

use crate::util::{distance_from_abb, ray_aabb_intersect};
use fmt::Debug;
use pathfinder_geometry::{rect::RectF, vector::Vector2F};
use std::{
    collections::HashMap,
    fmt,
    hash::{BuildHasher, Hash},
    mem,
};
#[derive(Debug, Clone)]
struct QuadTreeConfig {
    allow_duplicates: bool,
    max_children: usize,
    min_children: usize,
    max_depth: usize,
    epsilon: f32,
}

#[derive(Clone)]
pub struct QuadTree<T: Eq + PartialEq + Hash + Clone + Copy + Debug, S: BuildHasher> {
    root: QuadNode<T>,
    config: QuadTreeConfig,
    rect_cache: HashMap<T, RectF, S>,
}

impl<T: Eq + PartialEq + Hash + Clone + Copy + Copy + Debug, S: BuildHasher> Debug
    for QuadTree<T, S>
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_struct("QuadTree")
            .field("root", &self.root)
            .field("config", &self.config)
            .finish()
    }
}
impl<T: Eq + PartialEq + Hash + Clone + Copy + Debug, S: BuildHasher> Iterator for QuadTree<T, S> {
    type Item = (T, RectF);
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[derive(Clone, Debug)]
struct QuadNode<T: Eq + PartialEq + Clone + Copy + Debug> {
    aabb: RectF,
    elements: Vec<(T, RectF)>,
    sub_tree_count: usize,
    depth: usize,
    children: Option<[Box<QuadNode<T>>; 4]>,
}

impl<T: Eq + PartialEq + Hash + Clone + Copy + Debug, S: BuildHasher> QuadTree<T, S> {
    pub fn new(
        size: RectF,
        allow_duplicates: bool,
        min_children: usize,
        max_children: usize,
        max_depth: usize,
        hasher: S,
    ) -> QuadTree<T, S> {
        QuadTree {
            root: QuadNode {
                aabb: size,
                elements: Vec::with_capacity(max_children),
                children: None,
                depth: 0,
                sub_tree_count: 0,
            },
            config: QuadTreeConfig {
                allow_duplicates,
                max_children,
                min_children,
                max_depth,
                epsilon: 0.0001,
            },
            rect_cache: HashMap::with_hasher(hasher),
        }
    }

    /// Constructs a new QuadTree with default options
    ///
    /// * `size`: the enclosing space for the quad-tree.
    /// ### Defauts
    /// * `allow_duplicates`: true
    /// * `min_children`: 4
    /// * `max_children`: 16
    /// * `max_depth`: 8
    pub fn default(size: RectF, hasher: S) -> QuadTree<T, S> {
        QuadTree::new(size, true, 4, 16, 8, hasher)
    }

    /// Inserts an element with the provided bounding box. Area must be non-zero.
    pub fn insert(&mut self, item: T, aabb: RectF) -> Option<()> {
        debug_assert!(self.bounding_box().contains_rect(aabb));

        let &mut QuadTree {
            ref mut root,
            ref config,
            ref mut rect_cache,
        } = self;

        if root.insert(item, aabb, config) {
            rect_cache.insert(item, aabb);
            return Some(());
        } else {
            return None;
        }
    }

    /// Returns a vector of (element, bounding-box, id) for each element
    /// whose bounding box intersects with `bounding_box`.
    pub fn query_rect(&self, bounding_box: &RectF) -> Vec<(T, RectF)> {
        let mut nodes = vec![];
        self.root.query_rect(bounding_box, &self.config, &mut nodes);
        nodes
    }

    /// Returns a vector of (element, bounding-box, id) for each element
    /// whose bounding box which is within `query_radius` distance from `query_point`
    pub fn query_disk(&self, query_point: &Vector2F, query_radius: f32) -> Vec<(T, RectF)> {
        let mut nodes = vec![];
        self.root
            .query_disk(query_point, query_radius, &self.config, &mut nodes);
        nodes
    }

    /// Returns a vector of (element, bounding-box, id) for each element
    /// whose bounding box which contains `query_point`.
    pub fn query_point(&self, query_point: &Vector2F) -> Vec<(T, RectF)> {
        let mut nodes = vec![];
        self.root.query_point(query_point, &self.config, &mut nodes);
        nodes
    }

    /// Returns a vector of (element, bounding-box, id) for each element
    /// whose bounding box which intersects the ray defined by `origin` and `direction`.
    pub fn query_ray(&self, origin: &Vector2F, direction: &Vector2F) -> Vec<(T, RectF)> {
        let mut nodes = vec![];
        self.root
            .query_ray(origin, direction, &self.config, &mut nodes);
        nodes
    }

    /// Attempts to remove the item from the tree.  If that
    /// item was present, it returns the bounding-box of the removed item
    pub fn remove(&mut self, item: &T) -> Option<RectF> {
        if let Some(aabb) = self.rect_cache.get(item) {
            self.root.remove(item, aabb, &self.config);
            self.rect_cache.remove(item)
        } else {
            None
        }
    }

    /// Returns the enclosing bounding-box for the entire tree.
    pub fn bounding_box(&self) -> RectF {
        self.root.bounding_box()
    }
}

impl<T: Eq + PartialEq + Clone + Copy + Debug> QuadNode<T> {
    fn bounding_box(&self) -> RectF {
        self.aabb
    }

    fn new_leaf(aabb: RectF, depth: usize, config: &QuadTreeConfig) -> QuadNode<T> {
        QuadNode {
            aabb,
            elements: Vec::with_capacity(config.max_children / 2),
            children: None,
            depth,
            sub_tree_count: 0,
        }
    }

    fn insert(&mut self, item: T, item_aabb: RectF, config: &QuadTreeConfig) -> bool {
        // Assert that this insert is valid.
        assert!(item_aabb.width() <= self.aabb.width() && item_aabb.height() <= self.aabb.height());

        let inserted = if item_aabb.contains_point(self.aabb.center())
            || self.depth == config.max_depth
            || self.elements.len() < config.max_children - 1
        {
            self.attempt_insert_self(item, item_aabb, config)
        } else {
            if self.children.is_none() {
                let split = split_quad(self.aabb);
                self.children = Some([
                    Box::new(QuadNode::new_leaf(split[0], self.depth + 1, config)),
                    Box::new(QuadNode::new_leaf(split[1], self.depth + 1, config)),
                    Box::new(QuadNode::new_leaf(split[2], self.depth + 1, config)),
                    Box::new(QuadNode::new_leaf(split[3], self.depth + 1, config)),
                ]);
                // Try to push elements down into children
                let mut elements = mem::replace(&mut self.elements, vec![]);
                elements.retain(|(elem_index, elem_aabb)| {
                    !self.attempt_insert_children(*elem_index, *elem_aabb, config)
                });
                self.elements = elements;
            }
            // Try to fit this item into a child. If it doesn't fit, put it in
            self.attempt_insert_children(item, item_aabb, config)
                || self.attempt_insert_self(item, item_aabb, config)
        };
        if inserted {
            self.sub_tree_count += 1;
        }
        inserted
    }

    fn attempt_insert_self(&mut self, item: T, item_aabb: RectF, config: &QuadTreeConfig) -> bool {
        if config.allow_duplicates
            || self
                .elements
                .iter()
                .any(|&(_, e_bb)| close_to_rect(e_bb, item_aabb, config.epsilon))
        {
            self.elements.push((item, item_aabb));
            true
        } else {
            false
        }
    }

    fn attempt_insert_children(
        &mut self,
        item: T,
        item_aabb: RectF,
        config: &QuadTreeConfig,
    ) -> bool {
        if let Some(ref mut children) = self.children {
            for ref mut child in children.iter_mut() {
                if child.aabb.contains_rect(item_aabb) {
                    child.insert(item, item_aabb, config);
                    return true;
                }
            }
        }
        false
    }

    fn remove(&mut self, item: &T, item_aabb: &RectF, config: &QuadTreeConfig) -> bool {
        let mut removed = false;
        if let Some(ref mut children) = self.children {
            for ref mut child in children.iter_mut() {
                if child.aabb.contains_rect(*item_aabb) {
                    removed = child.remove(item, item_aabb, config);
                    break;
                }
            }
        }
        if !removed {
            let len_before = self.elements.len();
            self.elements.retain(|(id, _)| id != item);
            removed = len_before != self.elements.len();
        }
        if removed {
            self.sub_tree_count -= 1;
            if self.sub_tree_count < config.min_children && self.children.is_some() {
                let mut children = self.children.take().unwrap();
                self.elements.extend(
                    children
                        .iter_mut()
                        .map(|c| mem::take(&mut c.elements))
                        .flatten(),
                );
            }
        }
        removed
    }

    fn query_rect(&self, query_aabb: &RectF, config: &QuadTreeConfig, out: &mut Vec<(T, RectF)>) {
        if let Some(ref children) = self.children {
            for child in children.iter() {
                if query_aabb.intersects(child.aabb) {
                    child.query_rect(query_aabb, config, out);
                }
            }
        }
        out.extend(
            self.elements
                .iter()
                .filter(|(_, elem_aab)| query_aabb.intersects(*elem_aab)),
        );
    }

    fn query_disk(
        &self,
        query_point: &Vector2F,
        query_radius: f32,
        config: &QuadTreeConfig,
        out: &mut Vec<(T, RectF)>,
    ) {
        if let Some(ref children) = self.children {
            for child in children.iter() {
                if child.aabb.contains_point(*query_point)
                    || distance_from_abb(*query_point, child.aabb) < query_radius
                {
                    child.query_disk(query_point, query_radius, config, out);
                }
            }
        }
        out.extend(self.elements.iter().filter(|(_, elem_aab)| {
            elem_aab.contains_point(*query_point)
                || distance_from_abb(*query_point, *elem_aab) < query_radius
        }));
    }

    fn query_point(
        &self,
        query_point: &Vector2F,
        config: &QuadTreeConfig,
        out: &mut Vec<(T, RectF)>,
    ) {
        if let Some(ref children) = self.children {
            for child in children.iter() {
                if child.aabb.contains_point(*query_point) {
                    child.query_point(query_point, config, out);
                    break; // A point can only be contained by 1 child.
                }
            }
        }
        out.extend(
            self.elements
                .iter()
                .filter(|(_, elem_aab)| elem_aab.contains_point(*query_point)),
        );
    }

    fn query_ray(
        &self,
        origin: &Vector2F,
        direction: &Vector2F,
        config: &QuadTreeConfig,
        out: &mut Vec<(T, RectF)>,
    ) {
        if let Some(ref children) = self.children {
            for child in children.iter() {
                if ray_aabb_intersect(*origin, *direction, child.aabb) {
                    child.query_ray(origin, direction, config, out);
                    break; // A point can only be contained by 1 child.
                }
            }
        }
        out.extend(
            self.elements
                .iter()
                .filter(|(_, elem_aab)| ray_aabb_intersect(*origin, *direction, *elem_aab)),
        );
    }
}

fn split_quad(rect: RectF) -> [RectF; 4] {
    let origin = rect.origin();
    let half = rect.size() / 2.0;

    [
        RectF::new(origin, half),
        RectF::new(origin + Vector2F::new(half.x(), 0.0), half),
        RectF::new(origin + Vector2F::new(0.0, half.y()), half),
        RectF::new(origin + Vector2F::new(half.x(), half.y()), half),
    ]
}

fn close_to_point(a: Vector2F, b: Vector2F, epsilon: f32) -> bool {
    (a.x() - b.x()).abs() < epsilon && (a.y() - b.y()).abs() < epsilon
}

fn close_to_rect(a: RectF, b: RectF, epsilon: f32) -> bool {
    close_to_point(a.origin(), b.origin(), epsilon)
        && close_to_point(a.lower_right(), b.lower_right(), epsilon)
}
