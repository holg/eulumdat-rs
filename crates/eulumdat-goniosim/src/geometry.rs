//! Scene geometry primitives with exact ray intersection.

use crate::ray::{HitRecord, Ray};
use crate::MaterialId;
use nalgebra::{Point3, Unit, Vector3};

/// Geometric primitives for luminaire scenes.
#[derive(Debug, Clone)]
pub enum Primitive {
    /// Infinite plane (one-sided).
    Plane {
        point: Point3<f64>,
        normal: Unit<Vector3<f64>>,
    },

    /// Axis-aligned box.
    AaBox { min: Point3<f64>, max: Point3<f64> },

    /// Cylinder (capped or open).
    Cylinder {
        center: Point3<f64>,
        axis: Unit<Vector3<f64>>,
        radius: f64,
        half_height: f64,
        capped: bool,
    },

    /// Flat rectangular sheet (finite plane, for covers/panels).
    Sheet {
        center: Point3<f64>,
        normal: Unit<Vector3<f64>>,
        u_axis: Unit<Vector3<f64>>,
        half_width: f64,
        half_height: f64,
        thickness: f64,
    },
}

/// A scene object: a primitive with an assigned material.
#[derive(Debug, Clone)]
pub struct SceneObject {
    pub primitive: Primitive,
    pub material: MaterialId,
    pub label: String,
}

impl Primitive {
    /// Test ray intersection, returning a hit record if the ray hits
    /// within the range \[t_min, t_max\].
    pub fn intersect(
        &self,
        ray: &Ray,
        t_min: f64,
        t_max: f64,
        material_id: MaterialId,
    ) -> Option<HitRecord> {
        match self {
            Primitive::Plane { point, normal } => {
                intersect_plane(ray, point, normal, t_min, t_max, material_id)
            }
            Primitive::AaBox { min, max } => {
                intersect_aa_box(ray, min, max, t_min, t_max, material_id)
            }
            Primitive::Cylinder {
                center,
                axis,
                radius,
                half_height,
                capped,
            } => intersect_cylinder(
                ray,
                center,
                axis,
                *radius,
                *half_height,
                *capped,
                t_min,
                t_max,
                material_id,
            ),
            Primitive::Sheet {
                center,
                normal,
                u_axis,
                half_width,
                half_height,
                ..
            } => intersect_sheet(
                ray,
                center,
                normal,
                u_axis,
                *half_width,
                *half_height,
                t_min,
                t_max,
                material_id,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Plane intersection
// ---------------------------------------------------------------------------

fn intersect_plane(
    ray: &Ray,
    point: &Point3<f64>,
    normal: &Unit<Vector3<f64>>,
    t_min: f64,
    t_max: f64,
    material_id: MaterialId,
) -> Option<HitRecord> {
    let denom = ray.direction.dot(normal.as_ref());
    if denom.abs() < 1e-10 {
        return None; // parallel
    }
    let t = (point - ray.origin).dot(normal.as_ref()) / denom;
    if t < t_min || t > t_max {
        return None;
    }
    let mut hit = HitRecord {
        t,
        point: ray.at(t),
        normal: *normal,
        front_face: true,
        material: material_id,
    };
    hit.set_face_normal(ray, *normal);
    Some(hit)
}

// ---------------------------------------------------------------------------
// Axis-aligned box intersection (slab method)
// ---------------------------------------------------------------------------

fn intersect_aa_box(
    ray: &Ray,
    min: &Point3<f64>,
    max: &Point3<f64>,
    t_min: f64,
    t_max: f64,
    material_id: MaterialId,
) -> Option<HitRecord> {
    let mut tmin = t_min;
    let mut tmax = t_max;
    let mut hit_axis = 0usize;

    for i in 0..3 {
        let inv_d = 1.0 / ray.direction[i];
        let mut t0 = (min[i] - ray.origin[i]) * inv_d;
        let mut t1 = (max[i] - ray.origin[i]) * inv_d;
        if inv_d < 0.0 {
            std::mem::swap(&mut t0, &mut t1);
        }
        if t0 > tmin {
            tmin = t0;
            hit_axis = i;
        }
        if t1 < tmax {
            tmax = t1;
        }
        if tmax < tmin {
            return None;
        }
    }

    let t = tmin;
    let point = ray.at(t);
    let mut normal = Vector3::zeros();
    normal[hit_axis] = if ray.direction[hit_axis] < 0.0 {
        1.0
    } else {
        -1.0
    };
    let outward_normal = Unit::new_normalize(normal);

    let mut hit = HitRecord {
        t,
        point,
        normal: outward_normal,
        front_face: true,
        material: material_id,
    };
    hit.set_face_normal(ray, outward_normal);
    Some(hit)
}

// ---------------------------------------------------------------------------
// Cylinder intersection
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn intersect_cylinder(
    ray: &Ray,
    center: &Point3<f64>,
    axis: &Unit<Vector3<f64>>,
    radius: f64,
    half_height: f64,
    capped: bool,
    t_min: f64,
    t_max: f64,
    material_id: MaterialId,
) -> Option<HitRecord> {
    let oc = ray.origin - center;
    let d = ray.direction.as_ref();
    let a_vec = axis.as_ref();

    // Project onto plane perpendicular to axis
    let d_perp = d - d.dot(a_vec) * a_vec;
    let oc_perp = oc - oc.dot(a_vec) * a_vec;

    let a = d_perp.dot(&d_perp);
    let b = 2.0 * d_perp.dot(&oc_perp);
    let c = oc_perp.dot(&oc_perp) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_d = discriminant.sqrt();
    let mut best_t = None;
    let mut best_normal = Vector3::zeros();

    // Check both roots for the cylinder body
    for sign in [-1.0, 1.0] {
        let t = (-b + sign * sqrt_d) / (2.0 * a);
        if t < t_min || t > t_max {
            continue;
        }
        let p = ray.at(t);
        let h = (p - center).dot(a_vec);
        if h.abs() <= half_height && (best_t.is_none() || t < best_t.unwrap()) {
            let n = (p - center) - h * a_vec;
            best_t = Some(t);
            best_normal = n / radius;
        }
    }

    // Check caps
    if capped {
        for sign in [-1.0f64, 1.0] {
            let cap_center = center + sign * half_height * a_vec;
            let cap_normal = sign * a_vec;
            let denom = d.dot(&cap_normal);
            if denom.abs() < 1e-10 {
                continue;
            }
            let t = (cap_center - ray.origin).dot(&cap_normal) / denom;
            if t < t_min || t > t_max {
                continue;
            }
            let p = ray.at(t);
            let diff = p - cap_center;
            let dist2 = diff.dot(&diff) - diff.dot(&cap_normal).powi(2);
            if dist2 <= radius * radius && (best_t.is_none() || t < best_t.unwrap()) {
                best_t = Some(t);
                best_normal = cap_normal;
            }
        }
    }

    let t = best_t?;
    let outward_normal = Unit::new_normalize(best_normal);
    let mut hit = HitRecord {
        t,
        point: ray.at(t),
        normal: outward_normal,
        front_face: true,
        material: material_id,
    };
    hit.set_face_normal(ray, outward_normal);
    Some(hit)
}

// ---------------------------------------------------------------------------
// Sheet (finite plane) intersection
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn intersect_sheet(
    ray: &Ray,
    center: &Point3<f64>,
    normal: &Unit<Vector3<f64>>,
    u_axis: &Unit<Vector3<f64>>,
    half_width: f64,
    half_height: f64,
    t_min: f64,
    t_max: f64,
    material_id: MaterialId,
) -> Option<HitRecord> {
    let denom = ray.direction.dot(normal.as_ref());
    if denom.abs() < 1e-10 {
        return None;
    }
    let t = (center - ray.origin).dot(normal.as_ref()) / denom;
    if t < t_min || t > t_max {
        return None;
    }

    let point = ray.at(t);
    let local = point - center;
    let v_axis = normal.cross(u_axis.as_ref());

    let u = local.dot(u_axis.as_ref());
    let v = local.dot(&v_axis);

    if u.abs() > half_width || v.abs() > half_height {
        return None;
    }

    let mut hit = HitRecord {
        t,
        point,
        normal: *normal,
        front_face: true,
        material: material_id,
    };
    hit.set_face_normal(ray, *normal);
    Some(hit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_plane() {
        let ray = Ray::new(
            Point3::new(0.0, 0.0, 1.0),
            Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
        );
        let hit = intersect_plane(
            &ray,
            &Point3::origin(),
            &Vector3::z_axis(),
            0.001,
            f64::INFINITY,
            0,
        );
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.t - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ray_misses_plane_parallel() {
        let ray = Ray::new(
            Point3::new(0.0, 0.0, 1.0),
            Unit::new_normalize(Vector3::new(1.0, 0.0, 0.0)),
        );
        let hit = intersect_plane(
            &ray,
            &Point3::origin(),
            &Vector3::z_axis(),
            0.001,
            f64::INFINITY,
            0,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn ray_hits_sheet() {
        let ray = Ray::new(
            Point3::new(0.0, 0.0, 1.0),
            Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
        );
        let hit = intersect_sheet(
            &ray,
            &Point3::origin(),
            &Vector3::z_axis(),
            &Vector3::x_axis(),
            0.5,
            0.5,
            0.001,
            f64::INFINITY,
            0,
        );
        assert!(hit.is_some());
    }

    #[test]
    fn ray_misses_sheet_outside_bounds() {
        let ray = Ray::new(
            Point3::new(2.0, 0.0, 1.0),
            Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
        );
        let hit = intersect_sheet(
            &ray,
            &Point3::origin(),
            &Vector3::z_axis(),
            &Vector3::x_axis(),
            0.5,
            0.5,
            0.001,
            f64::INFINITY,
            0,
        );
        assert!(hit.is_none());
    }

    #[test]
    fn ray_hits_aa_box() {
        let ray = Ray::new(
            Point3::new(0.0, 0.0, 2.0),
            Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
        );
        let hit = intersect_aa_box(
            &ray,
            &Point3::new(-1.0, -1.0, -1.0),
            &Point3::new(1.0, 1.0, 1.0),
            0.001,
            f64::INFINITY,
            0,
        );
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.t - 1.0).abs() < 1e-6);
    }
}
