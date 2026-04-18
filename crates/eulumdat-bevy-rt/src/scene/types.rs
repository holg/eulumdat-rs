//! ECS components for photometric raytracing scenes.

use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use eulumdat_rt::{GpuMaterial, GpuPrimitive};

/// Pre-computed light distribution data for GPU emission sampling.
///
/// Built from an LDT/IES file via `LightProfile::from_eulumdat()`.
#[derive(Clone)]
pub struct LightProfile {
    /// Flattened CDF data: marginal_g (g_steps) + conditional_c (g_steps * c_steps).
    pub cdf_data: Vec<f32>,
    pub cdf_g_steps: u32,
    pub cdf_c_steps: u32,
    pub cdf_g_max: f32,
    /// Flattened intensity lookup table [c0g0, c0g1, ..., c1g0, ...] for camera rendering.
    pub lvk_data: Vec<f32>,
    pub lvk_max_intensity: f32,
    /// Total luminous flux (lm).
    pub flux: f32,
}

impl LightProfile {
    /// Build from a parsed Eulumdat (LDT) file.
    pub fn from_eulumdat(ldt: &eulumdat::Eulumdat) -> Self {
        use eulumdat_goniosim::source::LvkCdf;

        let cdf = LvkCdf::build(ldt);
        let g_steps = cdf.g_steps;
        let c_steps = cdf.c_steps;

        // Flatten CDF: marginal_g + conditional_c
        let mut cdf_data = Vec::with_capacity(g_steps + g_steps * c_steps);
        for v in &cdf.marginal_g {
            cdf_data.push(*v as f32);
        }
        for row in &cdf.conditional_c {
            for v in row {
                cdf_data.push(*v as f32);
            }
        }

        // Build LVK intensity lookup table for camera rendering
        let mut lvk_data = Vec::with_capacity(c_steps * g_steps);
        let mut max_intensity: f64 = 0.0;
        let g_max = cdf.g_max;
        for ci in 0..c_steps {
            let c = ci as f64 * (360.0 / c_steps as f64);
            for gi in 0..g_steps {
                let g = (gi as f64 * (g_max / (g_steps - 1).max(1) as f64)).min(g_max);
                let intensity = ldt.sample(c, g);
                lvk_data.push(intensity as f32);
                if intensity > max_intensity {
                    max_intensity = intensity;
                }
            }
        }

        let flux = ldt.total_luminous_flux() as f32;

        Self {
            cdf_data,
            cdf_g_steps: g_steps as u32,
            cdf_c_steps: c_steps as u32,
            cdf_g_max: g_max as f32,
            lvk_data,
            lvk_max_intensity: max_intensity as f32,
            flux,
        }
    }
}

/// Marker component: children of this entity define a photometric scene.
#[derive(Component, Default, Clone)]
pub struct PhotometricScene;

/// An optical element (sheet/reflector/lens) in the raytracing scene.
#[derive(Component, Clone)]
pub struct RtPrimitive {
    pub primitive: GpuPrimitive,
}

/// Material for optical elements.
#[derive(Component, Clone)]
pub struct RtMaterial {
    pub material: GpuMaterial,
}

/// Source type for photon emission.
#[derive(Clone, Copy, Debug, Default, Reflect)]
pub enum RtSourceType {
    #[default]
    Isotropic,
    Lambertian,
    FromLvk,
    Area,
}

impl RtSourceType {
    pub fn to_gpu_id(&self) -> u32 {
        match self {
            RtSourceType::Isotropic => 0,
            RtSourceType::Lambertian => 1,
            RtSourceType::FromLvk => 2,
            RtSourceType::Area => 3,
        }
    }
}

/// A photometric light source with emission profile.
#[derive(Component, Clone)]
pub struct RtLuminaire {
    pub source_type: RtSourceType,
    pub flux: f32,
    pub half_width: f32,
    pub half_height: f32,
    /// Light distribution profile from LDT/IES file.
    pub profile: Option<LightProfile>,
}

impl Default for RtLuminaire {
    fn default() -> Self {
        Self {
            source_type: RtSourceType::Isotropic,
            flux: 1000.0,
            half_width: 0.0,
            half_height: 0.0,
            profile: None,
        }
    }
}

impl RtLuminaire {
    /// Create a luminaire from a parsed LDT file.
    pub fn from_eulumdat(ldt: &eulumdat::Eulumdat) -> Self {
        let profile = LightProfile::from_eulumdat(ldt);
        Self {
            source_type: RtSourceType::FromLvk,
            flux: profile.flux,
            half_width: 0.0,
            half_height: 0.0,
            profile: Some(profile),
        }
    }
}

/// BVH node for software ray traversal.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BvhNode {
    pub aabb_min: [f32; 3],
    pub left_or_prim: u32,
    pub aabb_max: [f32; 3],
    pub right_or_count: u32,
}
