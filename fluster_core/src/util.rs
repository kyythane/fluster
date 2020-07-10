#![deny(clippy::all)]
use pathfinder_geometry::{rect::RectF, vector::Vector2F};
use std::mem;

pub fn clamp_0_1(n: f32) -> f32 {
    n.clamp(0.0, 1.0)
}

pub fn lerp(s: f32, e: f32, p: f32) -> f32 {
    (e - s) * p + s
}

pub fn distance_from_abb(point: Vector2F, aabb: RectF) -> f32 {
    (point - closest_point_on_aabb(point, aabb)).length()
}

pub fn closest_point_on_aabb(point: Vector2F, aabb: RectF) -> Vector2F {
    Vector2F::new(
        point.x().min(aabb.min_x()).max(aabb.max_x()),
        point.y().min(aabb.min_y()).max(aabb.max_y()),
    )
}

pub fn ray_aabb_intersect(ray_origin: Vector2F, ray_direction: Vector2F, aabb: RectF) -> bool {
    // ref: Page 180-181 of Real Time Colision Detection (Ch. 5.3.3)
    let mut t_min = std::f32::MIN;
    let mut t_max = std::f32::MAX;
    // test x-slab
    if ray_direction.x().abs() < std::f32::EPSILON {
        return ray_origin.x() >= aabb.min_x() && ray_origin.x() <= aabb.max_x();
    } else {
        // compute intersection with near and far planes of slab
        let ood = 1.0 / ray_direction.x();
        let mut t1 = (aabb.min_x() - ray_origin.x()) * ood;
        let mut t2 = (aabb.max_x() - ray_origin.x()) * ood;

        // ensure t1 is intersection w/ near plane and t2 w/ far plane
        if t1 > t2 {
            mem::swap(&mut t1, &mut t2);
        }

        // compute intersetion of slab with interval
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        // return if we are not in overlap
        if t_min > t_max {
            return false;
        }
    }
    // test y-slab
    if ray_direction.y().abs() < std::f32::EPSILON {
        return ray_origin.y() >= aabb.min_y() && ray_origin.y() <= aabb.max_y();
    } else {
        // compute intersection with near and far planes of slab
        let ood = 1.0 / ray_direction.y();
        let mut t1 = (aabb.min_y() - ray_origin.y()) * ood;
        let mut t2 = (aabb.max_y() - ray_origin.y()) * ood;

        // ensure t1 is intersection w/ near plane and t2 w/ far plane
        if t1 > t2 {
            mem::swap(&mut t1, &mut t2);
        }

        // compute intersetion of slab with interval
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        // return if we are not in overlap
        if t_min > t_max {
            return false;
        }
    }
    true
}
