//! Scene geometry generation

use crate::SceneSettings;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // Initialize default SceneSettings if not already present
        app.init_resource::<SceneSettings>();
        app.add_systems(
            Startup,
            setup_scene.run_if(resource_exists::<SceneSettings>),
        )
        .add_systems(
            Update,
            rebuild_scene_on_change.run_if(resource_exists::<SceneSettings>),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneType {
    #[default]
    Room,
    Road,
    Parking,
    Outdoor,
}

impl SceneType {
    pub fn default_dimensions(&self) -> (f32, f32, f32, f32) {
        // (width, length, height, mount_height)
        match self {
            SceneType::Room => (4.0, 5.0, 2.8, 2.5),
            SceneType::Road => (10.0, 30.0, 0.0, 8.0),
            SceneType::Parking => (20.0, 30.0, 0.0, 6.0),
            SceneType::Outdoor => (10.0, 15.0, 0.0, 3.0),
        }
    }
}

#[derive(Component)]
pub struct SceneGeometry;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<SceneSettings>,
) {
    build_scene(&mut commands, &mut meshes, &mut materials, &settings);
}

fn rebuild_scene_on_change(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<SceneSettings>,
    query: Query<Entity, With<SceneGeometry>>,
) {
    if !settings.is_changed() {
        return;
    }

    // Remove old scene geometry
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Build new scene
    build_scene(&mut commands, &mut meshes, &mut materials, &settings);
}

fn build_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &SceneSettings,
) {
    match settings.scene_type {
        SceneType::Room => build_room(commands, meshes, materials, settings),
        SceneType::Road => build_road(commands, meshes, materials, settings),
        SceneType::Parking => build_parking(commands, meshes, materials, settings),
        SceneType::Outdoor => build_outdoor(commands, meshes, materials, settings),
    }

    // Add ambient light - keep low so luminaire effect is visible
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0),
        brightness: 50.0, // Low ambient to see lighting differences
    });
}

fn build_room(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &SceneSettings,
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
}

fn build_road(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &SceneSettings,
) {
    let w = settings.room_width;
    let l = settings.room_length;

    // Asphalt road
    let road_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(w, l))),
        MeshMaterial3d(road_material),
        Transform::from_xyz(w / 2.0, 0.0, l / 2.0),
        SceneGeometry,
    ));

    // Road markings (center line)
    let marking_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(0.3, 0.3, 0.3, 1.0),
        ..default()
    });

    let mut z = 2.0;
    while z < l - 2.0 {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.15, 0.02, 2.0))),
            MeshMaterial3d(marking_material.clone()),
            Transform::from_xyz(w / 2.0, 0.01, z),
            SceneGeometry,
        ));
        z += 4.0;
    }

    // Sidewalks
    let sidewalk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.6),
        perceptual_roughness: 0.8,
        ..default()
    });

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.1, l))),
        MeshMaterial3d(sidewalk_material.clone()),
        Transform::from_xyz(0.5, 0.05, l / 2.0),
        SceneGeometry,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.1, l))),
        MeshMaterial3d(sidewalk_material),
        Transform::from_xyz(w - 0.5, 0.05, l / 2.0),
        SceneGeometry,
    ));

    // Light pole - on the right sidewalk, not on the road!
    spawn_pole(
        commands,
        meshes,
        materials,
        Vec3::new(w - 0.7, 0.0, l / 2.0),
        settings.mounting_height,
    );
}

fn build_parking(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &SceneSettings,
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
    settings: &SceneSettings,
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
