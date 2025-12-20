//! Eulumdat 3D Scene Viewer Library
//!
//! This crate provides Bevy-based photometric lighting visualization.
//!
//! # Architecture
//!
//! The crate is organized into two main modules:
//!
//! - [`photometric`] - Generic photometric lighting for any Bevy application
//! - [`viewer`] - Demo application with pre-built scenes and controls
//!
//! # Feature Flags
//!
//! - `photometric` - Generic photometric lighting (minimal dependencies)
//! - `viewer` - Full demo application with scenes, camera, controls (implies `photometric`)
//! - `wasm-sync` - localStorage polling for WASM hot-reload (implies `viewer`)
//! - `standalone` - Enable standalone binary (implies `wasm-sync`)
//!
//! # Usage as a Generic Photometric Plugin
//!
//! For embedding photometric lights in your own Bevy application:
//!
//! ```ignore
//! use bevy::prelude::*;
//! use eulumdat_bevy::photometric::*;
//! use eulumdat_bevy::{EulumdatLight, EulumdatLightBundle};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(PhotometricPlugin::<eulumdat::Eulumdat>::default())
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     // Your own camera
//!     commands.spawn(Camera3dBundle { ... });
//!
//!     // Your own scene geometry
//!     commands.spawn(PbrBundle { ... });
//!
//!     // Spawn a photometric light
//!     let ldt = eulumdat::Eulumdat::from_file("light.ldt").unwrap();
//!     commands.spawn(EulumdatLightBundle::new(ldt)
//!         .with_transform(Transform::from_xyz(0.0, 3.0, 0.0)));
//! }
//! ```
//!
//! # Usage as a Demo Viewer
//!
//! For the full demo experience with pre-built scenes:
//!
//! ```ignore
//! use bevy::prelude::*;
//! use eulumdat_bevy::viewer::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EulumdatViewerPlugin::default())
//!         .run();
//! }
//! ```
//!
//! # Implementing PhotometricData for Custom Types
//!
//! To use photometric lighting with your own data format:
//!
//! ```ignore
//! use eulumdat_bevy::photometric::PhotometricData;
//!
//! impl PhotometricData for MyLightData {
//!     fn sample(&self, c_angle: f64, g_angle: f64) -> f64 { ... }
//!     fn max_intensity(&self) -> f64 { ... }
//!     // ... implement other required methods
//! }
//!
//! // Then use PhotometricPlugin with your type:
//! app.add_plugins(PhotometricPlugin::<MyLightData>::default());
//! ```

// Generic photometric module (always available)
pub mod photometric;

// Eulumdat-specific implementation (always available)
mod eulumdat_impl;
pub use eulumdat_impl::{EulumdatLight, EulumdatLightBundle};

// Viewer module (only with "viewer" feature)
#[cfg(feature = "viewer")]
pub mod viewer;

// Re-export commonly used types at crate root for convenience
pub use photometric::{
    PhotometricData, PhotometricLight, PhotometricLightBundle, PhotometricPlugin,
};

// Re-export viewer types at crate root when available
#[cfg(feature = "viewer")]
pub use viewer::{EulumdatViewerPlugin, SceneType, ViewerSettings};

// Legacy compatibility: re-export old names
#[cfg(feature = "viewer")]
pub use viewer::ViewerSettings as SceneSettings;

// ============================================================================
// Standalone app functions (for running as a separate binary)
// ============================================================================

/// Run the 3D viewer on a specific canvas element (WASM).
///
/// # Arguments
/// * `canvas_selector` - CSS selector for the canvas element (e.g., "#bevy-canvas")
#[cfg(all(target_arch = "wasm32", feature = "standalone"))]
pub fn run_on_canvas(canvas_selector: &str) {
    use bevy::prelude::*;

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Eulumdat 3D Viewer".to_string(),
            canvas: Some(canvas_selector.to_string()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(viewer::EulumdatViewerPlugin::default());

    app.run();
}

/// Run the 3D viewer as a native window (desktop).
#[cfg(all(not(target_arch = "wasm32"), feature = "viewer"))]
pub fn run_native() {
    use bevy::prelude::*;

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Eulumdat 3D Viewer".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(viewer::EulumdatViewerPlugin::default())
        .run();
}

#[cfg(all(target_arch = "wasm32", feature = "standalone"))]
pub fn run_native() {
    // On WASM, run_native falls back to a default canvas
    run_on_canvas("#bevy-canvas");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "standalone"))]
pub fn run_on_canvas(_canvas_selector: &str) {
    run_native();
}
