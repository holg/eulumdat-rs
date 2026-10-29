//! Orbit camera for the path-traced viewer: yaw / pitch / distance around a
//! target, with a short ease-out when a scene sets a new home view.

use super::scene::{CameraLimits, CameraView};
use glam::{Mat4, Vec3};

#[derive(Clone, Debug)]
pub struct OrbitCamera {
    pub view: CameraView,
    pub limits: CameraLimits,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    /// Keep the eye above the floor.
    pub clamp_ground: bool,
    anim: Option<(CameraView, CameraView, f32)>,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            view: CameraView {
                target: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.4,
                distance: 10.0,
            },
            limits: CameraLimits::default(),
            fov_y: 50.0_f32.to_radians(),
            clamp_ground: true,
            anim: None,
        }
    }
}

const ANIM_SECONDS: f32 = 0.6;

impl OrbitCamera {
    pub fn eye(&self) -> Vec3 {
        eye_of(&self.view)
    }

    pub fn set_view(&mut self, view: CameraView, animate: bool) {
        let view = self.clamped(view);
        if animate {
            self.anim = Some((self.view, view, 0.0));
        } else {
            self.anim = None;
            self.view = view;
        }
    }

    fn clamped(&self, mut v: CameraView) -> CameraView {
        v.pitch = v.pitch.clamp(self.limits.min_pitch, self.limits.max_pitch);
        v.distance = v
            .distance
            .clamp(self.limits.min_distance, self.limits.max_distance);
        if self.clamp_ground {
            // Pitch low enough that the eye dips under the floor is lifted.
            let eye = eye_of(&v);
            if eye.y < 0.15 {
                let want = ((0.15 - v.target.y) / v.distance).clamp(-1.0, 1.0).asin();
                v.pitch = v.pitch.max(want).min(self.limits.max_pitch);
            }
        }
        v
    }

    /// Mouse drag in CSS pixels.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.anim = None;
        let mut v = self.view;
        v.yaw -= dx * 0.006;
        v.pitch += dy * 0.006;
        self.view = self.clamped(v);
    }

    /// Right-drag in CSS pixels; `css_height` scales the pan to the view.
    pub fn pan(&mut self, dx: f32, dy: f32, css_height: f32) {
        self.anim = None;
        let per_px = 2.0 * (self.fov_y / 2.0).tan() * self.view.distance / css_height.max(1.0);
        let (right, up) = self.right_up();
        let mut v = self.view;
        v.target -= right * dx * per_px;
        v.target += up * dy * per_px;
        self.view = self.clamped(v);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.anim = None;
        let mut v = self.view;
        v.distance *= factor;
        self.view = self.clamped(v);
    }

    /// Advance the ease-out; returns true when the view changed.
    pub fn update(&mut self, dt: f32) -> bool {
        let Some((from, to, t)) = self.anim.as_mut() else {
            return false;
        };
        *t += dt / ANIM_SECONDS;
        let s = if *t >= 1.0 {
            1.0
        } else {
            1.0 - (1.0 - *t).powi(3)
        };
        let lerp = |a: f32, b: f32| a + (b - a) * s;
        self.view = CameraView {
            target: from.target.lerp(to.target, s),
            yaw: lerp(from.yaw, to.yaw),
            pitch: lerp(from.pitch, to.pitch),
            distance: lerp(from.distance, to.distance),
        };
        if *t >= 1.0 {
            self.anim = None;
        }
        true
    }

    fn right_up(&self) -> (Vec3, Vec3) {
        let forward = (self.view.target - self.eye()).normalize_or(Vec3::NEG_Z);
        let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Y);
        (right, up)
    }

    /// Clip-from-world and its inverse (wgpu depth 0..1).
    pub fn matrices(&self, aspect: f32) -> (Mat4, Mat4) {
        let eye = self.eye();
        let near = (self.view.distance * 0.01).clamp(0.02, 0.5);
        let view = Mat4::look_at_rh(eye, self.view.target, Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, aspect.max(0.01), near, 2000.0);
        let vp = proj * view;
        (vp, vp.inverse())
    }

    /// Project a world point to NDC; `None` behind the camera.
    pub fn project(vp: &Mat4, p: Vec3) -> Option<(f32, f32, f32)> {
        let c = *vp * p.extend(1.0);
        if c.w <= 0.01 {
            return None;
        }
        Some((c.x / c.w, c.y / c.w, c.w))
    }
}

/// Eye position for an orbit view: `yaw = 0` is on +Z looking at −Z.
pub fn eye_of(v: &CameraView) -> Vec3 {
    let cp = v.pitch.cos();
    v.target + v.distance * Vec3::new(cp * v.yaw.sin(), v.pitch.sin(), cp * v.yaw.cos())
}
