//! First-person camera controller for the viewer.
//!
//! Supports both desktop (mouse + keyboard) and touch devices (iPad/iPhone):
//! - Desktop: Right-click + drag to look, WASD/Arrows to move, Q/E up/down, scroll wheel to zoom
//! - Touch: Single finger drag to look, two finger pinch to zoom, two finger drag to pan

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::prelude::*;

/// Plugin for first-person camera controls.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchState>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    camera_look,
                    camera_move,
                    camera_zoom,
                    camera_reset,
                    touch_camera_control,
                ),
            );
    }
}

/// Track touch state for gesture recognition.
#[derive(Resource, Default)]
struct TouchState {
    /// Active touch points (id -> position)
    touches: Vec<(u64, Vec2)>,
    /// Previous frame touch positions for delta calculation
    prev_touches: Vec<(u64, Vec2)>,
    /// Previous distance between two fingers (for pinch zoom)
    prev_pinch_distance: Option<f32>,
    /// Previous center of two fingers (for two-finger pan)
    prev_two_finger_center: Option<Vec2>,
}

/// Default camera position and look target.
/// Position: standing on sidewalk looking at the lamp/scene center
const DEFAULT_CAM_POS: Vec3 = Vec3::new(1.0, 1.7, 5.0); // Sidewalk, eye height ~1.7m
const DEFAULT_LOOK_AT: Vec3 = Vec3::new(5.0, 4.0, 15.0); // Looking at lamp area

/// First-person camera component.
#[derive(Component)]
pub struct FirstPersonCamera {
    /// Movement speed in meters per second
    pub speed: f32,
    /// Mouse look sensitivity
    pub sensitivity: f32,
    /// Current pitch angle (up/down)
    pub pitch: f32,
    /// Current yaw angle (left/right)
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

/// Calculate yaw and pitch from a direction vector.
fn direction_to_yaw_pitch(dir: Vec3) -> (f32, f32) {
    let yaw = dir.z.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
    let pitch = (-dir.y).asin();
    (yaw, pitch)
}

/// Get default yaw/pitch for looking from DEFAULT_CAM_POS to DEFAULT_LOOK_AT.
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

/// Reset camera with R or Home key.
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

/// Mouse look system (right-click + drag).
fn camera_look(
    mut query: Query<(&mut Transform, &mut FirstPersonCamera)>,
    mut mouse_motion: MessageReader<MouseMotion>,
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

/// Keyboard movement system (WASD/Arrows, Q/E for up/down).
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

/// Mouse scroll wheel zoom (move forward/backward along view direction).
fn camera_zoom(
    mut query: Query<(&mut Transform, &FirstPersonCamera)>,
    mut scroll_events: MessageReader<MouseWheel>,
) {
    let mut scroll_delta: f32 = 0.0;
    for event in scroll_events.read() {
        scroll_delta += event.y;
    }

    if scroll_delta.abs() > 0.0 {
        for (mut transform, camera) in query.iter_mut() {
            let forward = transform.forward();
            let zoom_speed = camera.speed * 0.5; // Slightly slower than movement
            transform.translation += forward * scroll_delta * zoom_speed;
        }
    }
}

/// Touch camera control system for iPad/iPhone.
///
/// Gestures:
/// - Single finger drag: Look around (rotate camera)
/// - Two finger pinch: Zoom (move forward/backward)
/// - Two finger drag: Pan (strafe left/right, up/down)
fn touch_camera_control(
    mut query: Query<(&mut Transform, &mut FirstPersonCamera)>,
    mut touch_events: MessageReader<TouchInput>,
    mut touch_state: ResMut<TouchState>,
) {
    // Update touch state from events
    for event in touch_events.read() {
        match event.phase {
            TouchPhase::Started => {
                // Add new touch
                touch_state.touches.push((event.id, event.position));
            }
            TouchPhase::Moved => {
                // Update existing touch position
                if let Some(touch) = touch_state
                    .touches
                    .iter_mut()
                    .find(|(id, _)| *id == event.id)
                {
                    touch.1 = event.position;
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                // Remove touch
                touch_state.touches.retain(|(id, _)| *id != event.id);
                // Reset gesture state when touches change
                touch_state.prev_pinch_distance = None;
                touch_state.prev_two_finger_center = None;
            }
        }
    }

    // Process gestures based on number of active touches
    let num_touches = touch_state.touches.len();

    for (mut transform, mut camera) in query.iter_mut() {
        if num_touches == 1 {
            // Single finger: Look around
            let current_pos = touch_state.touches[0].1;

            // Find previous position for this touch
            if let Some((_, prev_pos)) = touch_state
                .prev_touches
                .iter()
                .find(|(id, _)| *id == touch_state.touches[0].0)
            {
                let delta = current_pos - *prev_pos;

                // Apply rotation (scaled for touch - typically needs higher sensitivity)
                let touch_sensitivity = camera.sensitivity * 0.5;
                camera.yaw -= delta.x * touch_sensitivity;
                camera.pitch -= delta.y * touch_sensitivity;
                camera.pitch = camera.pitch.clamp(-1.5, 1.5);

                transform.rotation = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
            }

            // Reset two-finger gesture state
            touch_state.prev_pinch_distance = None;
            touch_state.prev_two_finger_center = None;
        } else if num_touches == 2 {
            // Two fingers: Pinch zoom and pan
            let pos1 = touch_state.touches[0].1;
            let pos2 = touch_state.touches[1].1;

            let current_distance = pos1.distance(pos2);
            let current_center = (pos1 + pos2) / 2.0;

            // Pinch zoom (move forward/backward)
            if let Some(prev_distance) = touch_state.prev_pinch_distance {
                let zoom_delta = current_distance - prev_distance;
                let zoom_speed = 0.01; // Adjust for feel

                let forward = transform.forward();
                transform.translation += forward * zoom_delta * zoom_speed;
            }

            // Two-finger pan (strafe)
            if let Some(prev_center) = touch_state.prev_two_finger_center {
                let pan_delta = current_center - prev_center;
                let pan_speed = 0.005 * camera.speed; // Adjust for feel

                let right = transform.right();
                let up = Vec3::Y;

                // Pan horizontally and vertically
                transform.translation -= right * pan_delta.x * pan_speed;
                transform.translation += up * pan_delta.y * pan_speed;
            }

            touch_state.prev_pinch_distance = Some(current_distance);
            touch_state.prev_two_finger_center = Some(current_center);
        } else {
            // No touches or 3+ fingers - reset gesture state
            touch_state.prev_pinch_distance = None;
            touch_state.prev_two_finger_center = None;
        }
    }

    // Save current touches as previous for next frame
    touch_state.prev_touches = touch_state.touches.clone();
}
