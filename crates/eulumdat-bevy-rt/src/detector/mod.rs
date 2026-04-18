//! Photometric detector plugin — goniophotometer mode for CIE 171:2006 validation.

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;

pub struct PhotometricDetectorPlugin;

impl Plugin for PhotometricDetectorPlugin {
    fn build(&self, _app: &mut App) {
        // TODO: embedded_asset for trace_detector.wgsl
    }

    fn finish(&self, _app: &mut App) {
        // TODO: register render systems
        //   - init_detector_pipelines (RenderStartup)
        //   - extract_detector (ExtractSchedule)
        //   - prepare_detector_buffers (PrepareResources)
        //   - photometric_detector dispatch (Core3d)
        //   - detector_readback (async buffer map)
    }
}

/// Attach to an entity to enable goniophotometer detector mode.
///
/// Traces photons from the scene's luminaire and accumulates
/// on a virtual spherical detector, matching CIE 171:2006.
#[derive(Component, Clone)]
pub struct PhotometricDetector {
    /// C-plane angular resolution in degrees.
    pub c_res_deg: f32,
    /// Gamma angular resolution in degrees.
    pub g_res_deg: f32,
    /// Number of photons to trace.
    pub num_photons: u32,
    /// Maximum bounces per photon.
    pub max_bounces: u32,
    /// Russian roulette energy threshold.
    pub rr_threshold: f32,
}

impl Default for PhotometricDetector {
    fn default() -> Self {
        Self {
            c_res_deg: 15.0,
            g_res_deg: 5.0,
            num_photons: 1_000_000,
            max_bounces: 50,
            rr_threshold: 0.01,
        }
    }
}
