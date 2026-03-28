//! Ray, hit record, and photon types for Monte Carlo tracing.

use nalgebra::{Point3, Unit, Vector3};

/// A ray with origin and direction.
#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Point3<f64>,
    pub direction: Unit<Vector3<f64>>,
}

impl Ray {
    /// Create a new ray.
    pub fn new(origin: Point3<f64>, direction: Unit<Vector3<f64>>) -> Self {
        Self { origin, direction }
    }

    /// Point along the ray at parameter t.
    pub fn at(&self, t: f64) -> Point3<f64> {
        self.origin + self.direction.as_ref() * t
    }
}

/// Record of a ray-surface intersection.
#[derive(Debug, Clone)]
pub struct HitRecord {
    /// Distance along the ray to the hit point.
    pub t: f64,
    /// World-space hit position.
    pub point: Point3<f64>,
    /// Outward-facing surface normal at the hit point.
    pub normal: Unit<Vector3<f64>>,
    /// True if the ray hit the outside of the surface (front face).
    pub front_face: bool,
    /// Index into the scene's material list.
    pub material: crate::MaterialId,
}

impl HitRecord {
    /// Set the face normal based on ray direction.
    /// Ensures the normal always points against the ray.
    pub fn set_face_normal(&mut self, ray: &Ray, outward_normal: Unit<Vector3<f64>>) {
        self.front_face = ray.direction.dot(outward_normal.as_ref()) < 0.0;
        self.normal = if self.front_face {
            outward_normal
        } else {
            Unit::new_unchecked(-outward_normal.into_inner())
        };
    }
}

/// A photon being traced through the scene.
#[derive(Debug, Clone)]
pub struct Photon {
    /// Current ray (position + direction).
    pub ray: Ray,
    /// Relative energy, starts at 1.0, reduced by absorption at each interaction.
    pub energy: f64,
    /// Wavelength in nm (default 555nm, for future spectral rendering).
    pub wavelength: f64,
    /// Number of bounces so far.
    pub bounces: u32,
}

impl Photon {
    /// Create a new photon from a ray with full energy.
    pub fn new(ray: Ray) -> Self {
        Self {
            ray,
            energy: 1.0,
            wavelength: 555.0,
            bounces: 0,
        }
    }
}
