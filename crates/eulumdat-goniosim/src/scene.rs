//! Scene description and builder for luminaire simulation.

use crate::catalog;
use crate::geometry::{Primitive, SceneObject};
use crate::material::{Material, MaterialParams};
use crate::ray::{HitRecord, Ray};
use crate::source::Source;
use crate::MaterialId;
use nalgebra::{Point3, Unit, Vector3};

/// A complete scene ready for tracing.
#[derive(Debug, Clone)]
pub struct Scene {
    pub sources: Vec<Source>,
    pub objects: Vec<SceneObject>,
    /// Internal physics materials, indexed by MaterialId.
    materials: Vec<Material>,
    /// User-facing material params, parallel to `materials`.
    material_params: Vec<MaterialParams>,
}

impl Scene {
    /// Create an empty scene.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            objects: Vec::new(),
            materials: Vec::new(),
            material_params: Vec::new(),
        }
    }

    /// Add a light source.
    pub fn add_source(&mut self, source: Source) {
        self.sources.push(source);
    }

    /// Add a material from user-facing params. Returns the material ID.
    pub fn add_material(&mut self, params: MaterialParams) -> MaterialId {
        let id = self.materials.len();
        self.materials.push(params.to_material());
        self.material_params.push(params);
        id
    }

    /// Add a scene object with a primitive, material, and label.
    pub fn add_object(
        &mut self,
        primitive: Primitive,
        material: MaterialId,
        label: &str,
    ) -> usize {
        let idx = self.objects.len();
        self.objects.push(SceneObject {
            primitive,
            material,
            label: label.to_string(),
        });
        idx
    }

    /// Get the internal physics material by ID.
    pub fn material(&self, id: MaterialId) -> &Material {
        &self.materials[id]
    }

    /// Get the user-facing material params by ID.
    pub fn material_params(&self, id: MaterialId) -> &MaterialParams {
        &self.material_params[id]
    }

    /// Total source flux in lumens.
    pub fn total_source_flux(&self) -> f64 {
        self.sources.iter().map(|s| s.flux_lm()).sum()
    }

    /// Find the nearest intersection of a ray with any scene object.
    pub fn intersect(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let mut closest: Option<HitRecord> = None;
        let mut closest_t = t_max;

        for obj in &self.objects {
            if let Some(hit) = obj.primitive.intersect(ray, t_min, closest_t, obj.material) {
                closest_t = hit.t;
                closest = Some(hit);
            }
        }

        closest
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Scene Builder
// ---------------------------------------------------------------------------

/// Which side of the source to place a reflector.
#[derive(Debug, Clone, Copy)]
pub enum ReflectorSide {
    Left,
    Right,
    Back,
    /// Cylindrical reflector surrounding the source.
    Surround,
}

/// Placement of a transmissive cover relative to the light source.
#[derive(Debug, Clone)]
pub struct CoverPlacement {
    /// Abstand zur Lichtquelle \[mm\] — along emission axis.
    pub distance_mm: f64,
    /// Cover width \[mm\].
    pub width_mm: f64,
    /// Cover height \[mm\].
    pub height_mm: f64,
}

/// Placement of a reflector surface relative to the light source.
#[derive(Debug, Clone)]
pub struct ReflectorPlacement {
    /// Abstand zur Lichtquelle \[mm\] — perpendicular to emission axis.
    pub distance_mm: f64,
    /// Reflector length along emission axis \[mm\].
    pub length_mm: f64,
    /// Which side of the source.
    pub side: ReflectorSide,
}

/// High-level builder for common luminaire configurations.
///
/// Positions objects relative to the light source so the user
/// specifies distances in mm, not 3D coordinates.
pub struct SceneBuilder {
    scene: Scene,
    source_position: Point3<f64>,
    source_direction: Unit<Vector3<f64>>,
}

impl SceneBuilder {
    /// Start building a scene. Source at origin, emitting downward (-Z).
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            source_position: Point3::origin(),
            source_direction: Unit::new_unchecked(Vector3::new(0.0, 0.0, -1.0)),
        }
    }

    /// Set the light source.
    pub fn source(mut self, source: Source) -> Self {
        self.scene.add_source(source);
        self
    }

    /// Add a reflector/housing surface at a given distance from the source.
    pub fn reflector(mut self, material: MaterialParams, placement: ReflectorPlacement) -> Self {
        let mat_name = material.name.clone();
        let mat_id = self.scene.add_material(material);
        let d = placement.distance_mm / 1000.0; // mm → m
        let l = placement.length_mm / 1000.0;

        match placement.side {
            ReflectorSide::Surround => {
                // Cylindrical reflector around the source
                let prim = Primitive::Cylinder {
                    center: self.source_position + self.source_direction.as_ref() * (l / 2.0),
                    axis: self.source_direction,
                    radius: d,
                    half_height: l / 2.0,
                    capped: false,
                };
                self.scene
                    .add_object(prim, mat_id, &format!("{mat_name} housing"));
            }
            ReflectorSide::Back => {
                let normal = Unit::new_unchecked(-self.source_direction.into_inner());
                let u_axis = perpendicular_axis(&self.source_direction);
                let center = self.source_position - self.source_direction.as_ref() * d;
                let prim = Primitive::Sheet {
                    center,
                    normal,
                    u_axis,
                    half_width: l / 2.0,
                    half_height: l / 2.0,
                    thickness: 0.001,
                };
                self.scene
                    .add_object(prim, mat_id, &format!("{mat_name} back"));
            }
            ReflectorSide::Left => {
                let u_axis = perpendicular_axis(&self.source_direction);
                let normal = Unit::new_normalize(u_axis.into_inner());
                let center = self.source_position - u_axis.as_ref() * d
                    + self.source_direction.as_ref() * (l / 2.0);
                let prim = Primitive::Sheet {
                    center,
                    normal,
                    u_axis: self.source_direction,
                    half_width: l / 2.0,
                    half_height: l / 2.0,
                    thickness: 0.001,
                };
                self.scene
                    .add_object(prim, mat_id, &format!("{mat_name} left"));
            }
            ReflectorSide::Right => {
                let u_axis = perpendicular_axis(&self.source_direction);
                let normal = Unit::new_normalize(-u_axis.into_inner());
                let center = self.source_position + u_axis.as_ref() * d
                    + self.source_direction.as_ref() * (l / 2.0);
                let prim = Primitive::Sheet {
                    center,
                    normal,
                    u_axis: self.source_direction,
                    half_width: l / 2.0,
                    half_height: l / 2.0,
                    thickness: 0.001,
                };
                self.scene
                    .add_object(prim, mat_id, &format!("{mat_name} right"));
            }
        }

        self
    }

    /// Add a transmissive cover (PMMA, glass) at a given distance from the source.
    pub fn cover(mut self, material: MaterialParams, placement: CoverPlacement) -> Self {
        let mat_name = material.name.clone();
        let mat_id = self.scene.add_material(material);
        let d = placement.distance_mm / 1000.0;
        let w = placement.width_mm / 1000.0;
        let h = placement.height_mm / 1000.0;

        // Place the cover sheet perpendicular to the emission direction,
        // at the given distance along the emission axis.
        let center = self.source_position + self.source_direction.as_ref() * d;
        let normal = Unit::new_unchecked(-self.source_direction.into_inner());
        let u_axis = perpendicular_axis(&self.source_direction);

        let thickness = self.scene.material_params[mat_id].thickness_mm / 1000.0;

        let prim = Primitive::Sheet {
            center,
            normal,
            u_axis,
            half_width: w / 2.0,
            half_height: h / 2.0,
            thickness,
        };
        self.scene
            .add_object(prim, mat_id, &format!("{mat_name} cover"));

        self
    }

    /// Build the final scene.
    pub fn build(self) -> Scene {
        self.scene
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Find a vector perpendicular to the given axis.
fn perpendicular_axis(axis: &Unit<Vector3<f64>>) -> Unit<Vector3<f64>> {
    let a = if axis.x.abs() > 0.9 {
        Vector3::y_axis()
    } else {
        Vector3::x_axis()
    };
    Unit::new_normalize(axis.cross(a.as_ref()))
}

// ---------------------------------------------------------------------------
// Preset scenes
// ---------------------------------------------------------------------------

/// Bare Lambertian emitter in free space.
/// Expected result: cosine LVK.
pub fn bare_lambertian(flux_lm: f64) -> Scene {
    let mut scene = Scene::new();
    scene.add_source(Source::Lambertian {
        position: Point3::origin(),
        normal: Unit::new_unchecked(Vector3::new(0.0, 0.0, -1.0)),
        flux_lm,
    });
    scene
}

/// Bare isotropic point source.
/// Expected result: constant cd in all directions.
pub fn bare_isotropic(flux_lm: f64) -> Scene {
    let mut scene = Scene::new();
    scene.add_source(Source::Isotropic {
        position: Point3::origin(),
        flux_lm,
    });
    scene
}

/// LED with reflector housing.
pub fn led_with_housing(flux_lm: f64, beam_angle_deg: f64) -> Scene {
    SceneBuilder::new()
        .source(Source::Led {
            position: Point3::origin(),
            direction: Unit::new_unchecked(Vector3::new(0.0, 0.0, -1.0)),
            half_angle_deg: beam_angle_deg / 2.0,
            flux_lm,
        })
        .reflector(
            catalog::white_paint(),
            ReflectorPlacement {
                distance_mm: 25.0,
                length_mm: 50.0,
                side: ReflectorSide::Surround,
            },
        )
        .build()
}

/// LED + housing + cover with configurable material and distance.
pub fn led_housing_with_cover(
    flux_lm: f64,
    beam_angle_deg: f64,
    cover_material: MaterialParams,
    cover_distance_mm: f64,
) -> Scene {
    SceneBuilder::new()
        .source(Source::Led {
            position: Point3::origin(),
            direction: Unit::new_unchecked(Vector3::new(0.0, 0.0, -1.0)),
            half_angle_deg: beam_angle_deg / 2.0,
            flux_lm,
        })
        .reflector(
            catalog::white_paint(),
            ReflectorPlacement {
                distance_mm: 25.0,
                length_mm: cover_distance_mm + 10.0,
                side: ReflectorSide::Surround,
            },
        )
        .cover(
            cover_material,
            CoverPlacement {
                distance_mm: cover_distance_mm,
                width_mm: 60.0,
                height_mm: 60.0,
            },
        )
        .build()
}

/// Round-trip validation: trace an existing LDT through empty space.
pub fn roundtrip_validation(ldt: &eulumdat::Eulumdat) -> Scene {
    let flux = ldt.total_luminous_flux();
    let mut scene = Scene::new();
    scene.add_source(Source::from_lvk(
        Point3::origin(),
        nalgebra::Rotation3::identity(),
        ldt.clone(),
        flux,
    ));
    scene
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_builder_creates_objects() {
        let scene = SceneBuilder::new()
            .source(Source::Led {
                position: Point3::origin(),
                direction: Unit::new_unchecked(Vector3::new(0.0, 0.0, -1.0)),
                half_angle_deg: 60.0,
                flux_lm: 1000.0,
            })
            .reflector(
                catalog::white_paint(),
                ReflectorPlacement {
                    distance_mm: 25.0,
                    length_mm: 50.0,
                    side: ReflectorSide::Surround,
                },
            )
            .cover(
                catalog::opal_pmma_3mm(),
                CoverPlacement {
                    distance_mm: 40.0,
                    width_mm: 60.0,
                    height_mm: 60.0,
                },
            )
            .build();

        assert_eq!(scene.sources.len(), 1);
        assert_eq!(scene.objects.len(), 2); // housing + cover
        assert!((scene.total_source_flux() - 1000.0).abs() < 0.01);
    }

    #[test]
    fn bare_lambertian_has_no_objects() {
        let scene = bare_lambertian(1000.0);
        assert_eq!(scene.sources.len(), 1);
        assert!(scene.objects.is_empty());
    }
}
