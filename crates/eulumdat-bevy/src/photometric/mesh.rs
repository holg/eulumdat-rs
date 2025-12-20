//! Mesh generation for photometric visualization.
//!
//! This module provides functions to generate:
//! - Photometric solid meshes (3D representation of light distribution)
//! - Luminaire geometry meshes (physical shape of the light fixture)

use super::{heatmap_color, PhotometricData};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

/// Resolution settings for photometric solid mesh generation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PhotometricMeshResolution {
    /// Low resolution: 20° C-step, 10° gamma-step (~324 vertices)
    Low,
    /// Medium resolution: 10° C-step, 5° gamma-step (~1296 vertices)
    #[default]
    Medium,
    /// High resolution: 5° C-step, 2.5° gamma-step (~5184 vertices)
    High,
    /// Custom resolution with specified step sizes
    Custom {
        /// C-plane angle step in degrees
        c_step: f64,
        /// Gamma angle step in degrees
        g_step: f64,
    },
}

impl PhotometricMeshResolution {
    /// Get the step sizes in degrees
    pub fn steps(&self) -> (f64, f64) {
        match self {
            Self::Low => (20.0, 10.0),
            Self::Medium => (10.0, 5.0),
            Self::High => (5.0, 2.5),
            Self::Custom { c_step, g_step } => (*c_step, *g_step),
        }
    }
}

/// Generate a photometric solid mesh from photometric data.
///
/// The mesh represents the 3D light distribution as a surface where
/// the distance from the origin at any direction equals the intensity
/// in that direction.
///
/// # Arguments
/// * `data` - Photometric data source implementing [`PhotometricData`]
/// * `resolution` - Mesh resolution (affects vertex count and detail)
/// * `scale` - Scale factor for the mesh size (default: 0.3)
///
/// # Returns
/// A Bevy Mesh with position, normal, and color attributes
///
/// # Example
/// ```ignore
/// let mesh = photometric_solid_mesh(&ldt, PhotometricMeshResolution::Medium, 0.3);
/// commands.spawn(Mesh3dBundle {
///     mesh: meshes.add(mesh),
///     material: materials.add(StandardMaterial {
///         base_color: Color::WHITE,
///         alpha_mode: AlphaMode::Blend,
///         ..default()
///     }),
///     ..default()
/// });
/// ```
pub fn photometric_solid_mesh<T: PhotometricData>(
    data: &T,
    resolution: PhotometricMeshResolution,
    scale: f32,
) -> Mesh {
    let (c_step, g_step) = resolution.steps();
    let num_c = (360.0 / c_step) as usize;
    let num_g = (180.0 / g_step) as usize + 1;

    let max_intensity = data.max_intensity();
    if max_intensity <= 0.0 {
        // Return empty mesh if no intensity data
        return Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
    }

    let mut positions = Vec::with_capacity(num_c * num_g);
    let mut normals = Vec::with_capacity(num_c * num_g);
    let mut colors = Vec::with_capacity(num_c * num_g);
    let mut indices = Vec::with_capacity(num_c * (num_g - 1) * 6);

    // Generate vertices
    for ci in 0..num_c {
        let c_angle = ci as f64 * c_step;
        let c_rad = c_angle.to_radians() as f32;

        for gi in 0..num_g {
            let g_angle = gi as f64 * g_step;
            let normalized = data.sample(c_angle, g_angle) / max_intensity;
            let r = normalized as f32 * scale;
            let g_rad = g_angle.to_radians() as f32;

            // Spherical to Cartesian (Y-down for gamma=0, i.e., nadir)
            let x = r * g_rad.sin() * c_rad.cos();
            let z = r * g_rad.sin() * c_rad.sin();
            let y = -r * g_rad.cos();

            positions.push([x, y, z]);

            // Approximate normals (pointing outward)
            let len = (x * x + y * y + z * z).sqrt().max(0.001);
            normals.push([x / len, y / len, z / len]);

            // Heatmap color based on intensity
            let (cr, cg, cb) = heatmap_color(normalized);
            colors.push([cr, cg, cb, 0.7]); // Semi-transparent
        }
    }

    // Generate triangle indices
    for c in 0..num_c {
        let next_c = (c + 1) % num_c;
        for g in 0..(num_g - 1) {
            let v0 = (c * num_g + g) as u32;
            let v1 = (next_c * num_g + g) as u32;
            let v2 = (next_c * num_g + (g + 1)) as u32;
            let v3 = (c * num_g + (g + 1)) as u32;

            // Two triangles per quad
            indices.push(v0);
            indices.push(v1);
            indices.push(v2);

            indices.push(v0);
            indices.push(v2);
            indices.push(v3);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Generate a luminaire geometry mesh based on dimensions.
///
/// Creates either a box or cylinder mesh depending on the luminaire type:
/// - Cylindrical: width ≈ 0, length = diameter
/// - Rectangular: width × length × height box
///
/// # Arguments
/// * `data` - Photometric data source implementing [`PhotometricData`]
///
/// # Returns
/// A Bevy Mesh representing the luminaire geometry
pub fn luminaire_mesh<T: PhotometricData>(data: &T) -> Mesh {
    let (width, length, height) = data.dimensions();

    if data.is_cylindrical() {
        // Cylindrical luminaire: length is diameter
        let radius = length.max(0.1) / 2.0;
        Cylinder::new(radius, height).into()
    } else {
        // Rectangular luminaire
        Cuboid::new(width.max(0.1), height, length.max(0.1)).into()
    }
}

/// Create a material for the luminaire model.
///
/// Returns a semi-emissive metallic material that glows with the light color.
///
/// # Arguments
/// * `light_color` - The color of the light
///
/// # Returns
/// StandardMaterial configured for luminaire visualization
pub fn luminaire_material(light_color: Color) -> StandardMaterial {
    let linear = light_color.to_linear();
    StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.3),
        emissive: LinearRgba::new(linear.red * 2.0, linear.green * 2.0, linear.blue * 2.0, 1.0),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    }
}

/// Create a material for the photometric solid.
///
/// Returns a transparent material suitable for the photometric solid mesh.
///
/// # Returns
/// StandardMaterial configured for photometric solid visualization
pub fn photometric_solid_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}
