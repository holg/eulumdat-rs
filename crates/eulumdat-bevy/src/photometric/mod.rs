//! Photometric lighting module for Bevy.
//!
//! This module provides generic photometric lighting support for any Bevy application.
//! It is designed to be extraction-ready for a standalone `bevy_photometry` crate.
//!
//! # Features
//!
//! - [`PhotometricData`] trait for abstracting photometric data sources
//! - [`PhotometricLight`] component for spawning photometric lights
//! - [`PhotometricPlugin`] for automatic light synchronization
//! - Color utilities (Kelvin to RGB, CRI adjustment)
//! - Photometric solid mesh generation
//!
//! # Example
//!
//! ```ignore
//! use bevy::prelude::*;
//! use eulumdat_bevy::photometric::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(PhotometricPlugin::<MyLightData>::default())
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     let data = MyLightData::load("light.ldt");
//!     commands.spawn(PhotometricLightBundle::new(data)
//!         .with_transform(Transform::from_xyz(0.0, 3.0, 0.0)));
//! }
//! ```

mod color;
mod data;
mod light;
mod mesh;
mod plugin;
mod systems;

// Re-export public API
pub use color::{
    apply_cri_adjustment, heatmap_color, kelvin_to_color, parse_color_temperature, parse_cri,
};
pub use data::PhotometricData;
pub use light::{
    BevyLightMarker, LuminaireModel, PhotometricLight, PhotometricLightBundle, PhotometricSolid,
};
pub use mesh::{
    luminaire_material, luminaire_mesh, photometric_solid_material, photometric_solid_mesh,
    PhotometricMeshResolution,
};
pub use plugin::PhotometricPlugin;
