//! Extract photometric scene data from the main world to the render world.

use super::types::{PhotometricScene, RtLuminaire, RtMaterial, RtPrimitive};
use bevy_ecs::prelude::*;
use bevy_render::Extract;
use bevy_transform::components::GlobalTransform;

/// Extracted scene data ready for GPU upload.
#[derive(Resource, Default)]
pub struct ExtractedPhotometricScene {
    pub primitives: Vec<eulumdat_rt::GpuPrimitive>,
    pub materials: Vec<eulumdat_rt::GpuMaterial>,
    pub luminaires: Vec<ExtractedLuminaire>,
    pub changed: bool,
}

pub struct ExtractedLuminaire {
    pub source_type: u32,
    pub flux: f32,
    pub half_width: f32,
    pub half_height: f32,
    pub transform: GlobalTransform,
    pub profile: Option<super::types::LightProfile>,
}

pub fn extract_photometric_scene(
    mut extracted: ResMut<ExtractedPhotometricScene>,
    scenes: Extract<Query<&Children, With<PhotometricScene>>>,
    primitives: Extract<Query<(&RtPrimitive, &GlobalTransform)>>,
    materials: Extract<Query<&RtMaterial>>,
    luminaires: Extract<Query<(&RtLuminaire, &GlobalTransform)>>,
) {
    extracted.primitives.clear();
    extracted.materials.clear();
    extracted.luminaires.clear();

    for children in scenes.iter() {
        for child in children.iter() {
            if let Ok((prim, _transform)) = primitives.get(child) {
                extracted.primitives.push(prim.primitive);
            }
            if let Ok(mat) = materials.get(child) {
                extracted.materials.push(mat.material);
            }
            if let Ok((lum, transform)) = luminaires.get(child) {
                extracted.luminaires.push(ExtractedLuminaire {
                    source_type: lum.source_type.to_gpu_id(),
                    flux: lum.flux,
                    half_width: lum.half_width,
                    half_height: lum.half_height,
                    transform: *transform,
                    profile: lum.profile.clone(),
                });
            }
        }
    }

    extracted.changed = true;
}
