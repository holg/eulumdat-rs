//! Minimal light stress test: raw Bevy lights at Bistro positions.
//! No photometric plugin, no sky, no material fixes.
//! If this flickers, it's a Bevy 0.19 issue. If not, it's our PhotometricPlugin.
//!
//! Run:
//!   cargo run --example light_stress -p eulumdat-bevy --features post-process --release

use bevy::camera::Hdr;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;

#[derive(Component)]
struct SimpleCam;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (simple_cam_move, simple_cam_look).chain())
        .run();
}

fn simple_cam_look(
    mut query: Query<&mut Transform, With<SimpleCam>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    if !mouse_button.pressed(MouseButton::Right) {
        mouse_motion.clear();
        return;
    }
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    for mut tf in query.iter_mut() {
        let (mut yaw, mut pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
        yaw -= delta.x * 0.003;
        pitch -= delta.y * 0.003;
        pitch = pitch.clamp(-1.5, 1.5);
        tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
}

fn simple_cam_move(
    mut query: Query<&mut Transform, With<SimpleCam>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for mut tf in query.iter_mut() {
        let speed = 5.0 * time.delta_secs();
        let fwd = tf.forward();
        let right = tf.right();
        if keys.pressed(KeyCode::KeyW) { tf.translation += fwd * speed; }
        if keys.pressed(KeyCode::KeyS) { tf.translation -= fwd * speed; }
        if keys.pressed(KeyCode::KeyA) { tf.translation -= right * speed; }
        if keys.pressed(KeyCode::KeyD) { tf.translation += right * speed; }
        if keys.pressed(KeyCode::KeyQ) { tf.translation += Vec3::Y * speed; }
        if keys.pressed(KeyCode::KeyE) { tf.translation -= Vec3::Y * speed; }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut cluster_settings: ResMut<bevy::light::cluster::GlobalClusterSettings>,
) {
    // Pre-allocate cluster buffers
    if let Some(ref mut gpu) = cluster_settings.gpu_clustering {
        gpu.initial_z_slice_list_capacity = 4096;
        gpu.initial_index_list_capacity = 524288;
    }

    commands.insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)));
    commands.insert_resource(bevy::light::GlobalAmbientLight {
        color: Color::srgb(0.7, 0.7, 0.9),
        brightness: 8.0,
        affects_lightmapped_meshes: true,
    });

    // Moonlight
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.7, 0.75, 0.95),
            illuminance: 15000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::PI * 0.35,
            -std::f32::consts::PI * 0.13,
            0.0,
        )),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Tonemapping::AgX,
        Bloom { intensity: 0.08, ..default() },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 600.0,
            ..default()
        },
        Transform::from_translation(Vec3::new(-10.5, 1.7, -1.0))
            .looking_at(Vec3::new(0.0, 3.5, 0.0), Vec3::Y),
        SimpleCam,
        NoIndirectDrawing,
        bevy::light::cluster::ClusterConfig::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: default(),
            dynamic_resizing: true,
        },
    ));

    // Load Bistro
    #[cfg(not(target_arch = "wasm32"))]
    let bistro_glb = "BistroExterior_gpu.glb#Scene0";
    #[cfg(target_arch = "wasm32")]
    let bistro_glb = "BistroExterior_web.glb#Scene0";

    commands.spawn(SceneRoot(asset_server.load(bistro_glb)));

    // --- Spawn raw Bevy lights at the same Bistro positions as obscura_demo ---
    // Same light color as road_luminaire.ldt would produce
    let warm_white = Color::srgb(1.0, 0.85, 0.6);
    let intensity = 50000.0;

    // 12 street lamps — 1 PointLight + 3 SpotLights each (same as photometric system)
    let street_positions: &[(f32, f32, f32)] = &[
        (-6.93, 6.92, -6.70), (5.98, 6.93, -34.01), (78.92, 6.93, 54.97),
        (56.16, 6.93, 29.31), (-33.31, 6.93, -29.18), (-15.44, 6.93, 3.34),
        (-3.34, 6.93, 7.44), (-2.81, 6.93, 16.06), (12.50, 6.93, 13.85),
        (34.53, 6.93, 30.11), (53.29, 6.93, 38.56), (62.08, 6.93, 54.31),
    ];

    // 16 wall lights
    let wall_positions: &[(f32, f32, f32)] = &[
        (-21.44, 6.33, 1.94), (-27.61, 6.33, -6.06), (28.84, 7.04, -66.51),
        (28.25, 7.04, -55.05), (14.37, 7.15, -32.13), (48.01, 6.35, 21.60),
        (39.14, 7.24, 37.07), (33.22, 6.35, 20.41), (-32.27, 6.33, -11.50),
        (-39.34, 6.31, -18.89), (-45.62, 5.14, -27.51), (-43.03, 5.14, -33.70),
        (-28.37, 6.33, -23.91), (-19.10, 6.33, -15.39), (-13.49, 6.37, -18.26),
        (-3.39, 6.63, -29.25),
    ];

    // 5 spotlights
    let spot_positions: &[(f32, f32, f32)] = &[
        (-26.19, 11.98, -22.00), (15.72, 5.55, -33.45), (-20.12, 13.68, 3.93),
        (-26.09, 13.68, -3.01), (-29.69, 13.67, -9.11),
    ];

    // 5 lanterns
    let lantern_positions: &[(f32, f32, f32)] = &[
        (7.33, 4.13, 5.27), (2.91, 4.14, 3.52), (-1.79, 4.13, -7.54),
        (-0.07, 4.08, -11.55), (11.71, 4.09, 6.69),
    ];

    let all_positions: Vec<Vec3> = street_positions.iter()
        .chain(wall_positions)
        .chain(spot_positions)
        .chain(lantern_positions)
        .map(|&(x, y, z)| Vec3::new(x, y, z))
        .collect();

    println!("Spawning {} luminaire positions, each with 1 point + 3 spots = {} total Bevy lights",
        all_positions.len(), all_positions.len() * 4);

    for pos in &all_positions {
        let h = pos.y;

        // 1) Ambient point light
        commands.spawn((
            PointLight {
                color: warm_white,
                intensity: intensity * 0.3,
                radius: 0.05,
                range: 50.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(*pos),
        ));

        // 2) Main downward spot
        let target = Vec3::new(pos.x, 0.0, pos.z);
        commands.spawn((
            SpotLight {
                color: warm_white,
                intensity: intensity * 0.3,
                range: h * 3.0,
                radius: 0.05,
                inner_angle: 0.15,
                outer_angle: 0.45,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(*pos).looking_at(target, Vec3::Z),
        ));

        // 3) Side spot +Z
        let target_side = Vec3::new(pos.x, 0.0, pos.z + 8.0);
        commands.spawn((
            SpotLight {
                color: warm_white,
                intensity: intensity * 0.35,
                range: h * 4.0,
                radius: 0.05,
                inner_angle: 0.3,
                outer_angle: 0.8,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(*pos).looking_at(target_side, Vec3::X),
        ));

        // 4) Side spot -Z
        let target_neg = Vec3::new(pos.x, 0.0, pos.z - 4.0);
        commands.spawn((
            SpotLight {
                color: warm_white,
                intensity: intensity * 0.175,
                range: h * 3.0,
                radius: 0.05,
                inner_angle: 0.2,
                outer_angle: 0.6,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(*pos).looking_at(target_neg, Vec3::X),
        ));
    }
}
