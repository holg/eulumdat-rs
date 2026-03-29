//! Virtual Goniophotometer 3D scene visualization.
//!
//! Shows the simulation setup: source, cover sheet, and detector sphere
//! with heatmap coloring from the traced photon distribution.

use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;

/// Plugin for the goniophotometer visualization scene.
pub struct GonioSimScenePlugin;

impl Plugin for GonioSimScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GonioSimState>()
            .add_systems(Startup, setup_goniosim_scene)
            .add_systems(Update, update_detector_heatmap);
    }
}

/// State of the goniophotometer simulation for visualization.
#[derive(Resource, Default)]
pub struct GonioSimState {
    /// Detector candela bins [c_index][g_index] — updated from CPU/GPU tracer.
    pub detector_bins: Vec<Vec<f64>>,
    /// Number of C-angle bins.
    pub num_c: usize,
    /// Number of gamma bins.
    pub num_g: usize,
    /// Maximum candela value (for normalization).
    pub max_cd: f64,
    /// Whether the detector data has changed since last render.
    pub dirty: bool,
    /// Whether a cover is present.
    pub has_cover: bool,
    /// Cover distance from source (meters).
    pub cover_distance: f32,
}

/// Marker component for the detector sphere mesh.
#[derive(Component)]
pub struct DetectorSphere;

/// Marker component for the cover sheet mesh.
#[derive(Component)]
pub struct CoverSheet;

/// Marker component for the source indicator.
#[derive(Component)]
pub struct SourceIndicator;

/// Set up the 3D scene: source point, cover sheet, detector sphere.
fn setup_goniosim_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Source indicator (small glowing sphere at origin)
    commands.spawn((
        SourceIndicator,
        Mesh3d(meshes.add(Sphere::new(0.02).mesh().ico(2).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.3),
            emissive: LinearRgba::new(5.0, 4.5, 1.5, 1.0),
            ..default()
        })),
        Transform::from_translation(Vec3::ZERO),
    ));

    // Cover sheet (semi-transparent, below source)
    commands.spawn((
        CoverSheet,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.6, 0.8, 1.0, 0.3),
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, -0.04, 0.0)),
        Visibility::Hidden,
    ));

    // Detector sphere (wireframe, large)
    let sphere_mesh = create_detector_sphere_mesh(1.0, 36, 18);
    commands.spawn((
        DetectorSphere,
        Mesh3d(meshes.add(sphere_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.2, 0.3, 0.5, 0.15),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(Vec3::ZERO),
    ));

    // Ambient light
    commands.spawn((
        AmbientLight {
            color: Color::WHITE,
            brightness: 50.0,
        },
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.5, 1.0, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Update detector sphere vertex colors from the simulation state.
fn update_detector_heatmap(
    state: Res<GonioSimState>,
    query: Query<&Mesh3d, With<DetectorSphere>>,
    mut meshes: ResMut<Assets<Mesh>>,
    cover_query: Query<Entity, With<CoverSheet>>,
    mut visibility_query: Query<&mut Visibility>,
) {
    if !state.dirty {
        return;
    }

    // Show/hide cover
    for entity in cover_query.iter() {
        if let Ok(mut vis) = visibility_query.get_mut(entity) {
            *vis = if state.has_cover {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    // Update sphere vertex colors based on detector bins
    if state.detector_bins.is_empty() || state.max_cd <= 0.0 {
        return;
    }

    for mesh_handle in query.iter() {
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            update_sphere_colors(mesh, &state);
        }
    }
}

/// Create a UV sphere mesh with vertex colors for heatmap display.
fn create_detector_sphere_mesh(radius: f32, segments: u32, rings: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32;
        let y = radius * phi.cos();
        let r = radius * phi.sin();

        for seg in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let x = r * theta.cos();
            let z = r * theta.sin();

            positions.push([x, y, z]);
            let n = Vec3::new(x, y, z).normalize();
            normals.push([n.x, n.y, n.z]);
            colors.push([0.1f32, 0.15, 0.25, 0.15]); // default dark blue, transparent
        }
    }

    for ring in 0..rings {
        for seg in 0..segments {
            let a = ring * (segments + 1) + seg;
            let b = a + 1;
            let c = a + segments + 1;
            let d = c + 1;

            indices.push(a);
            indices.push(c);
            indices.push(b);

            indices.push(b);
            indices.push(c);
            indices.push(d);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// Update sphere vertex colors from detector state.
fn update_sphere_colors(mesh: &mut Mesh, state: &GonioSimState) {
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::render::mesh::VertexAttributeValues::Float32x3(p)) => p.clone(),
        _ => return,
    };

    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(positions.len());

    for pos in &positions {
        // Convert vertex position to (C, gamma) angles
        let dir = Vec3::new(pos[0], pos[1], pos[2]).normalize();

        // gamma: angle from -Y (nadir in Bevy's Y-up coordinate system)
        let gamma_rad = (-dir.y).acos();
        let gamma_deg = gamma_rad.to_degrees();

        // C: azimuth from +X towards +Z
        let c_deg = dir.z.atan2(dir.x).to_degrees();
        let c_deg = if c_deg < 0.0 { c_deg + 360.0 } else { c_deg };

        // Look up detector bin
        let ci = ((c_deg / (360.0 / state.num_c as f64)) as usize).min(state.num_c.saturating_sub(1));
        let gi = ((gamma_deg as f64 / (180.0 / state.num_g as f64)) as usize).min(state.num_g.saturating_sub(1));

        let cd = if ci < state.detector_bins.len() && gi < state.detector_bins[ci].len() {
            state.detector_bins[ci][gi]
        } else {
            0.0
        };

        // Normalize to 0..1 and map to heatmap color
        let normalized = (cd / state.max_cd).clamp(0.0, 1.0) as f32;
        let color = heatmap_color(normalized);
        colors.push(color);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
}

/// Heatmap color: black → blue → cyan → green → yellow → red → white
fn heatmap_color(t: f32) -> [f32; 4] {
    let alpha = 0.15 + t * 0.6; // more opaque where brighter
    if t < 0.0001 {
        return [0.05, 0.08, 0.15, 0.1];
    }
    let (r, g, b) = if t < 0.2 {
        let s = t / 0.2;
        (0.0, 0.0, s)
    } else if t < 0.4 {
        let s = (t - 0.2) / 0.2;
        (0.0, s, 1.0)
    } else if t < 0.6 {
        let s = (t - 0.4) / 0.2;
        (s, 1.0, 1.0 - s)
    } else if t < 0.8 {
        let s = (t - 0.6) / 0.2;
        (1.0, 1.0 - s, 0.0)
    } else {
        let s = (t - 0.8) / 0.2;
        (1.0, s, s)
    };
    [r, g, b, alpha]
}
