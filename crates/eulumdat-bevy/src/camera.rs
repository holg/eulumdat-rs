//! First-person camera controller

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (camera_look, camera_move, camera_reset));
    }
}

/// Default camera position and look target
/// Position: standing on sidewalk looking at the lamp/scene center
const DEFAULT_CAM_POS: Vec3 = Vec3::new(1.0, 1.7, 5.0); // Sidewalk, eye height ~1.7m
const DEFAULT_LOOK_AT: Vec3 = Vec3::new(5.0, 4.0, 15.0); // Looking at lamp area

#[derive(Component)]
pub struct FirstPersonCamera {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for FirstPersonCamera {
    fn default() -> Self {
        Self {
            speed: 3.0,
            sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.8, // ~45 degrees, looking into the room
        }
    }
}

/// Calculate yaw and pitch from a direction vector
fn direction_to_yaw_pitch(dir: Vec3) -> (f32, f32) {
    let yaw = dir.z.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
    let pitch = (-dir.y).asin();
    (yaw, pitch)
}

/// Get default yaw/pitch for looking from DEFAULT_CAM_POS to DEFAULT_LOOK_AT
fn default_yaw_pitch() -> (f32, f32) {
    let dir = (DEFAULT_LOOK_AT - DEFAULT_CAM_POS).normalize();
    direction_to_yaw_pitch(dir)
}

fn spawn_camera(mut commands: Commands) {
    let (yaw, pitch) = default_yaw_pitch();
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(DEFAULT_CAM_POS).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            yaw,
            pitch,
            0.0,
        )),
        FirstPersonCamera {
            yaw,
            pitch,
            ..default()
        },
    ));
}

/// Reset camera with R or Home key
fn camera_reset(
    mut query: Query<(&mut Transform, &mut FirstPersonCamera)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) || keyboard.just_pressed(KeyCode::Home) {
        let (yaw, pitch) = default_yaw_pitch();
        for (mut transform, mut camera) in query.iter_mut() {
            transform.translation = DEFAULT_CAM_POS;
            camera.yaw = yaw;
            camera.pitch = pitch;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
        }
    }
}

fn camera_look(
    mut query: Query<(&mut Transform, &mut FirstPersonCamera)>,
    mut mouse_motion: EventReader<MouseMotion>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    // Only look when right mouse button is held
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

    for (mut transform, mut camera) in query.iter_mut() {
        camera.yaw -= delta.x * camera.sensitivity;
        camera.pitch -= delta.y * camera.sensitivity;
        camera.pitch = camera.pitch.clamp(-1.5, 1.5);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
    }
}

fn camera_move(
    mut query: Query<(&mut Transform, &FirstPersonCamera)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut transform, camera) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        // Get forward/right vectors (ignore Y for movement)
        let forward = transform.forward();
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = transform.right();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        // WASD movement
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction += forward_flat;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction -= forward_flat;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction -= right_flat;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction += right_flat;
        }

        // Q/E for up/down
        if keyboard.pressed(KeyCode::KeyQ) {
            direction += Vec3::Y;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            direction -= Vec3::Y;
        }

        // Apply movement
        if direction != Vec3::ZERO {
            direction = direction.normalize();
            transform.translation += direction * camera.speed * time.delta_secs();
        }
    }
}
