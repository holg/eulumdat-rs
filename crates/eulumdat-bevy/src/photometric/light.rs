//! PhotometricLight component and bundle for Bevy.
//!
//! This module provides the core components for spawning photometric lights
//! in a Bevy application.

use super::PhotometricData;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Component for photometric light data.
///
/// Attach this component to an entity to create a photometric light.
/// The `PhotometricPlugin` will automatically spawn Bevy lights,
/// luminaire models, and photometric solids based on the settings.
///
/// # Type Parameters
/// * `T` - The photometric data type (must implement [`PhotometricData`])
///
/// # Example
/// ```ignore
/// commands.spawn((
///     PhotometricLight::new(ldt_data),
///     Transform::from_xyz(0.0, 3.0, 0.0),
/// ));
/// ```
#[derive(Component, Clone, Debug)]
pub struct PhotometricLight<T: PhotometricData> {
    /// The photometric data
    pub data: T,
    /// Intensity scale factor (default: 1.0)
    pub intensity_scale: f32,
    /// Whether to render the photometric solid mesh
    pub show_solid: bool,
    /// Whether to render the luminaire geometry model
    pub show_model: bool,
    /// Whether to enable shadows
    pub shadows_enabled: bool,
}

impl<T: PhotometricData> PhotometricLight<T> {
    /// Create a new PhotometricLight with default settings.
    pub fn new(data: T) -> Self {
        Self {
            data,
            intensity_scale: 1.0,
            show_solid: false,
            show_model: true,
            shadows_enabled: false,
        }
    }

    /// Set the intensity scale factor.
    pub fn with_intensity_scale(mut self, scale: f32) -> Self {
        self.intensity_scale = scale;
        self
    }

    /// Enable or disable the photometric solid visualization.
    pub fn with_solid(mut self, show: bool) -> Self {
        self.show_solid = show;
        self
    }

    /// Enable or disable the luminaire model.
    pub fn with_model(mut self, show: bool) -> Self {
        self.show_model = show;
        self
    }

    /// Enable or disable shadows.
    pub fn with_shadows(mut self, enabled: bool) -> Self {
        self.shadows_enabled = enabled;
        self
    }
}

/// Bundle for spawning a photometric light with transform.
///
/// This is the recommended way to spawn a photometric light.
///
/// # Example
/// ```ignore
/// commands.spawn(
///     PhotometricLightBundle::new(ldt_data)
///         .with_transform(Transform::from_xyz(0.0, 3.0, 0.0))
/// );
/// ```
#[derive(Bundle, Clone)]
pub struct PhotometricLightBundle<T: PhotometricData> {
    /// The photometric light component
    pub light: PhotometricLight<T>,
    /// Transform for positioning
    pub transform: Transform,
    /// Global transform (computed automatically)
    pub global_transform: GlobalTransform,
}

impl<T: PhotometricData> PhotometricLightBundle<T> {
    /// Create a new bundle with default transform at origin.
    pub fn new(data: T) -> Self {
        Self {
            light: PhotometricLight::new(data),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
        }
    }

    /// Set the transform.
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Set the intensity scale.
    pub fn with_intensity_scale(mut self, scale: f32) -> Self {
        self.light = self.light.with_intensity_scale(scale);
        self
    }

    /// Enable or disable the photometric solid.
    pub fn with_solid(mut self, show: bool) -> Self {
        self.light = self.light.with_solid(show);
        self
    }

    /// Enable or disable the luminaire model.
    pub fn with_model(mut self, show: bool) -> Self {
        self.light = self.light.with_model(show);
        self
    }

    /// Enable or disable shadows.
    pub fn with_shadows(mut self, enabled: bool) -> Self {
        self.light = self.light.with_shadows(enabled);
        self
    }
}

/// Marker component for Bevy lights spawned by PhotometricPlugin.
///
/// Used to track and update lights when the PhotometricLight component changes.
#[derive(Component)]
pub struct BevyLightMarker<T: PhotometricData> {
    /// The parent entity with PhotometricLight
    pub parent: Entity,
    _phantom: PhantomData<T>,
}

impl<T: PhotometricData> BevyLightMarker<T> {
    /// Create a new marker pointing to the parent entity.
    pub fn new(parent: Entity) -> Self {
        Self {
            parent,
            _phantom: PhantomData,
        }
    }
}

/// Marker component for photometric solid mesh entities.
#[derive(Component)]
pub struct PhotometricSolid<T: PhotometricData> {
    /// The parent entity with PhotometricLight
    pub parent: Entity,
    _phantom: PhantomData<T>,
}

impl<T: PhotometricData> PhotometricSolid<T> {
    /// Create a new marker pointing to the parent entity.
    pub fn new(parent: Entity) -> Self {
        Self {
            parent,
            _phantom: PhantomData,
        }
    }
}

/// Marker component for luminaire model entities.
#[derive(Component)]
pub struct LuminaireModel<T: PhotometricData> {
    /// The parent entity with PhotometricLight
    pub parent: Entity,
    _phantom: PhantomData<T>,
}

impl<T: PhotometricData> LuminaireModel<T> {
    /// Create a new marker pointing to the parent entity.
    pub fn new(parent: Entity) -> Self {
        Self {
            parent,
            _phantom: PhantomData,
        }
    }
}

/// Resource to track whether the plugin has been initialized.
#[derive(Resource)]
pub struct PhotometricPluginState<T: PhotometricData> {
    _phantom: PhantomData<T>,
}

impl<T: PhotometricData> Default for PhotometricPluginState<T> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}
