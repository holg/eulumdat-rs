//! Bevy render plugin for photometric raytracing.
//!
//! Compute-shader based, WASM/WebGPU compatible — no hardware RT required.
//! Reuses physics from `eulumdat-rt` (Monte Carlo photon tracing, CIE 171:2006).

pub mod detector;
pub mod scene;
pub mod tracer;

use bevy_app::{App, Plugin, PluginGroup, PluginGroupBuilder};

/// Plugin group for photometric raytracing in Bevy.
///
/// Includes scene management and camera rendering by default.
/// Add [`PhotometricDetectorPlugin`](detector::PhotometricDetectorPlugin) separately
/// for goniophotometer validation mode.
pub struct PhotometricRtPlugins;

impl PluginGroup for PhotometricRtPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(scene::PhotometricScenePlugin)
            .add(tracer::PhotometricTracerPlugin)
    }
}

/// Standalone plugin that bundles everything including detector mode.
pub struct PhotometricRtAllPlugins;

impl PluginGroup for PhotometricRtAllPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(scene::PhotometricScenePlugin)
            .add(tracer::PhotometricTracerPlugin)
            .add(detector::PhotometricDetectorPlugin)
    }
}
