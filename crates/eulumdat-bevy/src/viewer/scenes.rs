//! Scene geometry generation for the viewer.
//!
//! Provides pre-built demo scenes: Room, Road, Parking, Outdoor.

use super::ViewerSettings;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

/// Road lighting arrangement types per EN 13201.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RoadArrangement {
    /// Single side - poles on one side only (narrow roads)
    SingleSide,
    /// Staggered - alternating sides (medium roads)
    Staggered,
    /// Opposite - poles on both sides, aligned (wide roads)
    Opposite,
}

/// Calculate the optimal arrangement based on road width and mounting height.
/// EN 13201 guidelines:
/// - W/H < 1.0: Single side
/// - 1.0 <= W/H < 1.5: Staggered
/// - W/H >= 1.5: Opposite (both sides)
///
/// Note: Outer-side arrangements are preferred over central median because:
/// - They illuminate sidewalks as well as the road
/// - Allow different luminaire types for road vs pedestrian areas
/// - More practical for maintenance
fn determine_road_arrangement(settings: &ViewerSettings) -> RoadArrangement {
    let road_width = settings.num_lanes as f32 * settings.lane_width;
    let ratio = road_width / settings.mounting_height;

    if ratio < 1.0 {
        RoadArrangement::SingleSide
    } else if ratio < 1.5 {
        RoadArrangement::Staggered
    } else {
        // For wider roads, opposite arrangement on both sides
        // This illuminates both sidewalks and provides good road coverage
        RoadArrangement::Opposite
    }
}

/// Plugin for scene geometry.
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewerSettings>();
        app.add_systems(
            Startup,
            setup_scene.run_if(resource_exists::<ViewerSettings>),
        )
        .add_systems(
            Update,
            rebuild_scene_on_change.run_if(resource_exists::<ViewerSettings>),
        );
    }
}

/// Scene type for demo scenes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneType {
    /// Indoor room scene (4×5×2.8m)
    #[default]
    Room,
    /// Street lighting scene (10×30m road)
    Road,
    /// Parking lot scene (20×30m lot)
    Parking,
    /// Outdoor/garden scene (10×15m)
    Outdoor,
    /// Exterior designer scene (from Area Lighting Designer)
    DesignerExterior,
    /// Interior designer scene (from Zonal Cavity Designer)
    DesignerInterior,
}

impl SceneType {
    /// Get default dimensions for this scene type.
    /// Returns (width, length, height, mount_height)
    /// Note: For Road scene, width is calculated from lane_width * num_lanes + 2*sidewalk_width
    pub fn default_dimensions(&self) -> (f32, f32, f32, f32) {
        match self {
            SceneType::Room => (4.0, 5.0, 2.8, 2.5),
            // Road: 2 lanes × 3.5m + 2 sidewalks × 2m = 11m wide, 100m long
            SceneType::Road => (11.0, 100.0, 0.0, 8.0),
            SceneType::Parking => (20.0, 30.0, 0.0, 6.0),
            SceneType::Outdoor => (10.0, 15.0, 0.0, 3.0),
            SceneType::DesignerExterior => (20.0, 20.0, 0.0, 8.0),
            SceneType::DesignerInterior => (4.0, 5.0, 2.8, 2.5),
        }
    }
}

/// Marker component for scene geometry entities.
#[derive(Component)]
pub struct SceneGeometry;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<ViewerSettings>,
) {
    build_scene(&mut commands, &mut meshes, &mut materials, &mut images, &settings);
}

fn rebuild_scene_on_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    settings: Res<ViewerSettings>,
    query: Query<Entity, With<SceneGeometry>>,
) {
    if !settings.is_changed() {
        return;
    }

    // Remove old scene geometry
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    // Build new scene
    build_scene(&mut commands, &mut meshes, &mut materials, &mut images, &settings);
}

fn build_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    settings: &ViewerSettings,
) {
    match settings.scene_type {
        SceneType::Room => build_room(commands, meshes, materials, settings),
        SceneType::Road => build_road(commands, meshes, materials, settings),
        SceneType::Parking => build_parking(commands, meshes, materials, settings),
        SceneType::Outdoor => build_outdoor(commands, meshes, materials, settings),
        SceneType::DesignerExterior => {
            super::designer_scenes::build_designer_exterior(commands, meshes, materials, images, settings);
        }
        SceneType::DesignerInterior => {
            super::designer_scenes::build_designer_interior(commands, meshes, materials, images, settings);
        }
    }

    // Add ambient light - keep low so luminaire effect is visible
    // In Bevy 0.18, AmbientLight is now a component, use GlobalAmbientLight as resource
    commands.insert_resource(bevy::light::GlobalAmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0),
        brightness: 50.0, // Low ambient to see lighting differences
        affects_lightmapped_meshes: true,
    });
}

fn build_room(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &ViewerSettings,
) {
    let w = settings.room_width;
    let l = settings.room_length;
    let h = settings.room_height;

    // Floor
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.85),
        perceptual_roughness: 0.8,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, l))),
        MeshMaterial3d(floor_material.clone()),
        Transform::from_xyz(w / 2.0, 0.0, l / 2.0),
        SceneGeometry,
    ));

    // Ceiling
    let ceiling_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, l))),
        MeshMaterial3d(ceiling_material),
        Transform::from_xyz(w / 2.0, h, l / 2.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
        SceneGeometry,
    ));

    // Walls
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Back wall (z=0)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, h))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(w / 2.0, h / 2.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Front wall (z=l)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, h))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(w / 2.0, h / 2.0, l)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Left wall (x=0)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(l, h))),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_xyz(0.0, h / 2.0, l / 2.0)
            .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Right wall (x=w)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(l, h))),
        MeshMaterial3d(wall_material),
        Transform::from_xyz(w, h / 2.0, l / 2.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Pendulum/suspension cable (if pendulum_length > 0)
    spawn_pendulum_cable(commands, meshes, materials, settings, w / 2.0, l / 2.0);
}

fn build_road(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &ViewerSettings,
) {
    // Calculate dimensions from settings
    let lane_w = settings.lane_width;
    let num_lanes = settings.num_lanes;
    let sidewalk_w = settings.sidewalk_width;
    let road_width = num_lanes as f32 * lane_w; // Just the lanes
    let total_width = road_width + 2.0 * sidewalk_w; // Including sidewalks
    let road_length = settings.room_length;
    let pole_spacing = settings.effective_pole_spacing();

    // Determine arrangement based on road/height ratio
    let arrangement = determine_road_arrangement(settings);

    // Materials
    let road_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    let sidewalk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.6),
        perceptual_roughness: 0.8,
        ..default()
    });

    let marking_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(0.3, 0.3, 0.3, 1.0),
        ..default()
    });

    let yellow_marking = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.0),
        emissive: LinearRgba::new(0.3, 0.25, 0.0, 1.0),
        ..default()
    });

    // Asphalt road surface
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(road_width, road_length))),
        MeshMaterial3d(road_material),
        Transform::from_xyz(sidewalk_w + road_width / 2.0, 0.0, road_length / 2.0),
        SceneGeometry,
    ));

    // Left sidewalk
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(sidewalk_w, 0.15, road_length))),
        MeshMaterial3d(sidewalk_material.clone()),
        Transform::from_xyz(sidewalk_w / 2.0, 0.075, road_length / 2.0),
        SceneGeometry,
    ));

    // Right sidewalk
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(sidewalk_w, 0.15, road_length))),
        MeshMaterial3d(sidewalk_material.clone()),
        Transform::from_xyz(total_width - sidewalk_w / 2.0, 0.075, road_length / 2.0),
        SceneGeometry,
    ));

    // Center line (yellow, double line for two-way traffic)
    let center_x = sidewalk_w + road_width / 2.0;
    let mut z = 1.0;
    while z < road_length - 1.0 {
        // Double yellow line
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.12, 0.02, 3.0))),
            MeshMaterial3d(yellow_marking.clone()),
            Transform::from_xyz(center_x - 0.15, 0.01, z + 1.5),
            SceneGeometry,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.12, 0.02, 3.0))),
            MeshMaterial3d(yellow_marking.clone()),
            Transform::from_xyz(center_x + 0.15, 0.01, z + 1.5),
            SceneGeometry,
        ));
        z += 4.0;
    }

    // Lane edge lines (white dashed)
    for lane_idx in 0..num_lanes {
        if lane_idx == 0 {
            // Left edge - solid white line
            let edge_x = sidewalk_w + 0.15;
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.15, 0.02, road_length - 2.0))),
                MeshMaterial3d(marking_material.clone()),
                Transform::from_xyz(edge_x, 0.01, road_length / 2.0),
                SceneGeometry,
            ));
        }
        if lane_idx == num_lanes - 1 {
            // Right edge - solid white line
            let edge_x = sidewalk_w + road_width - 0.15;
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.15, 0.02, road_length - 2.0))),
                MeshMaterial3d(marking_material.clone()),
                Transform::from_xyz(edge_x, 0.01, road_length / 2.0),
                SceneGeometry,
            ));
        }
    }

    // Spawn poles based on arrangement - always on outer sides (sidewalks)
    // This provides illumination for both road and pedestrian areas
    let num_poles = ((road_length / pole_spacing).floor() as i32).max(1);
    let actual_spacing = road_length / (num_poles as f32 + 1.0);

    // Middle pole spacing for center illumination on wide roads
    let middle_pole_spacing = 50.0;

    match arrangement {
        RoadArrangement::SingleSide => {
            // Poles on right side only - arm extends toward road
            for i in 1..=num_poles {
                let z = i as f32 * actual_spacing;
                spawn_pole(
                    commands,
                    meshes,
                    materials,
                    Vec3::new(total_width - sidewalk_w / 2.0, 0.0, z),
                    settings.mounting_height,
                );
            }
        }
        RoadArrangement::Staggered => {
            // Alternating sides - better uniformity for medium roads
            for i in 1..=num_poles {
                let z = i as f32 * actual_spacing;
                let x = if i % 2 == 0 {
                    sidewalk_w / 2.0 // Left sidewalk
                } else {
                    total_width - sidewalk_w / 2.0 // Right sidewalk
                };
                spawn_pole(
                    commands,
                    meshes,
                    materials,
                    Vec3::new(x, 0.0, z),
                    settings.mounting_height,
                );
            }
        }
        RoadArrangement::Opposite => {
            // Both sides, aligned - best for wide roads
            // Each luminaire illuminates its adjacent sidewalk + half the road
            for i in 1..=num_poles {
                let z = i as f32 * actual_spacing;
                spawn_pole(
                    commands,
                    meshes,
                    materials,
                    Vec3::new(sidewalk_w / 2.0, 0.0, z),
                    settings.mounting_height,
                );
                spawn_pole(
                    commands,
                    meshes,
                    materials,
                    Vec3::new(total_width - sidewalk_w / 2.0, 0.0, z),
                    settings.mounting_height,
                );
            }

            // Add middle poles every 50m for better center illumination on wide roads
            if road_width > 6.0 {
                let num_middle_poles = ((road_length / middle_pole_spacing).floor() as i32).max(0);
                for i in 1..=num_middle_poles {
                    let z = i as f32 * middle_pole_spacing;
                    spawn_dual_arm_pole(
                        commands,
                        meshes,
                        materials,
                        Vec3::new(center_x, 0.0, z),
                        settings.mounting_height,
                    );
                }
            }
        }
    }
}

/// Spawn a dual-arm pole for center median (used for middle poles on wide roads).
fn spawn_dual_arm_pole(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    base_position: Vec3,
    height: f32,
) {
    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });

    // Vertical pole
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.1, height))),
        MeshMaterial3d(pole_material.clone()),
        Transform::from_xyz(base_position.x, height / 2.0, base_position.z),
        SceneGeometry,
    ));

    // Left arm
    let arm_length = 2.0;
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.05, arm_length))),
        MeshMaterial3d(pole_material.clone()),
        Transform::from_xyz(
            base_position.x - arm_length / 2.0,
            height - 0.25,
            base_position.z,
        )
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));

    // Right arm
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.05, arm_length))),
        MeshMaterial3d(pole_material),
        Transform::from_xyz(
            base_position.x + arm_length / 2.0,
            height - 0.25,
            base_position.z,
        )
        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
    ));
}

fn build_parking(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &ViewerSettings,
) {
    let w = settings.room_width;
    let l = settings.room_length;

    // Parking lot surface
    let lot_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        perceptual_roughness: 0.85,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, l))),
        MeshMaterial3d(lot_material),
        Transform::from_xyz(w / 2.0, 0.0, l / 2.0),
        SceneGeometry,
    ));

    // Parking lines
    let line_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(0.2, 0.2, 0.2, 1.0),
        ..default()
    });

    let space_width = 2.5;
    let space_length = 5.0;

    let mut row = 3.0;
    while row < l - 3.0 {
        let mut col = space_width;
        while col < w - 1.0 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.1, 0.02, space_length))),
                MeshMaterial3d(line_material.clone()),
                Transform::from_xyz(col, 0.01, row),
                SceneGeometry,
            ));
            col += space_width;
        }
        row += space_length + 1.0;
    }

    // Light pole
    spawn_pole(
        commands,
        meshes,
        materials,
        Vec3::new(w / 2.0, 0.0, l / 2.0),
        settings.mounting_height,
    );
}

fn build_outdoor(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &ViewerSettings,
) {
    let w = settings.room_width;
    let l = settings.room_length;

    // Grass
    let grass_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.3, 0.1),
        perceptual_roughness: 0.95,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, l))),
        MeshMaterial3d(grass_material),
        Transform::from_xyz(w / 2.0, 0.0, l / 2.0),
        SceneGeometry,
    ));

    // Garden path
    let path_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        perceptual_roughness: 0.8,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.2, 0.02, l - 2.0))),
        MeshMaterial3d(path_material),
        Transform::from_xyz(w / 2.0, 0.01, l / 2.0),
        SceneGeometry,
    ));

    // Bushes
    let bush_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.25, 0.05),
        perceptual_roughness: 0.95,
        ..default()
    });

    for (x, y, z) in [
        (2.0, 0.4, 3.0),
        (w - 2.0, 0.3, l - 4.0),
        (1.5, 0.35, l - 2.0),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(y))),
            MeshMaterial3d(bush_material.clone()),
            Transform::from_xyz(x, y, z),
            SceneGeometry,
        ));
    }

    // Light pole
    spawn_pole(
        commands,
        meshes,
        materials,
        Vec3::new(w / 2.0, 0.0, l / 2.0),
        settings.mounting_height,
    );
}

fn spawn_pole(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    height: f32,
) {
    let pole_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.4),
        metallic: 0.6,
        perceptual_roughness: 0.4,
        ..default()
    });

    // Pole - don't cast shadows to avoid blocking the lamp
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.08, height - 0.3))),
        MeshMaterial3d(pole_material.clone()),
        Transform::from_xyz(position.x, height / 2.0, position.z),
        SceneGeometry,
        NotShadowCaster,
    ));

    // Arm - short stub, luminaire hangs separately below
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.05, 0.3))),
        MeshMaterial3d(pole_material),
        Transform::from_xyz(position.x - 0.05, height - 0.2, position.z)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        SceneGeometry,
        NotShadowCaster,
    ));
}

/// Spawn a pendulum/suspension cable for ceiling-mounted luminaires.
/// Only spawns if pendulum_length > 0.
fn spawn_pendulum_cable(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &ViewerSettings,
    x: f32,
    z: f32,
) {
    if settings.pendulum_length <= 0.0 {
        return;
    }

    let cable_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        metallic: 0.3,
        perceptual_roughness: 0.6,
        ..default()
    });

    // Cable hangs from ceiling (room_height) down by pendulum_length
    let cable_top = settings.room_height;
    let cable_bottom = settings.room_height - settings.pendulum_length;
    let cable_center_y = (cable_top + cable_bottom) / 2.0;

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.01, settings.pendulum_length))),
        MeshMaterial3d(cable_material),
        Transform::from_xyz(x, cable_center_y, z),
        SceneGeometry,
        NotShadowCaster,
    ));
}
