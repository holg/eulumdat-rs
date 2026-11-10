//! GPU-side structs and the bind group contract of `trace.wgsl`.
//!
//! Group 0:
//! 0 `Params` uniform · 1 prims (storage, read) · 2 lamps (storage, read) ·
//! 3 IES table `texture_2d<f32>` R32Float, column = C (1°), row = γ (0.5°) ·
//! 4 radiance accumulation `array<vec4<f32>>` (storage, read_write) ·
//! 5 illuminance accumulation `array<vec4<f32>>` (storage, read_write) ·
//! 6 output `texture_storage_2d<rgba16float, write>` · 7 probe positions
//! (`vec4` pos, `vec4` normal pairs, storage read) · 8 probe values
//! `array<vec4<f32>>` (storage, read_write, x = lux).

use glam::{Mat4, UVec4, Vec4};

/// Photometry samples per pixel before the picture is considered done.
pub const MAX_SAMPLES: u32 = 768;
/// Illuminance samples per measurement point per probe dispatch.
pub const PROBE_PASS_SAMPLES: u32 = 16;
/// Probe samples per point before the points are considered done.
pub const PROBE_MAX_SAMPLES: u32 = 1024;

/// Uniform block, mirrors `Params` in `trace.wgsl` (all 16-byte members, so
/// `repr(C)` matches the WGSL layout).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub inv_view_proj: Mat4,
    /// xyz eye.
    pub cam_pos: Vec4,
    /// rgb linear tint, w = luma of the tint.
    pub tint: Vec4,
    /// xyz housing half extents, w = round (1) / rect (0).
    pub housing_half: Vec4,
    /// half_x, half_z of the luminous face, IES scale (dimming), unused.
    pub lum: Vec4,
    /// IES c size, IES g size, lamp count, box count.
    pub counts: UVec4,
    /// ground (0 none, 1 paving, 2 street, 3 parking), unused, max bounces, mode.
    pub flags: UVec4,
    /// frame index, probe count, probe samples per pass, isolines.
    pub frame: UVec4,
    /// blend, probe blend, road half width, outdoor.
    pub f0: Vec4,
    /// sky scale, ambient, haze, glare.
    pub f1: Vec4,
    /// firefly clamp, exposure, lux max, iso step.
    pub f2: Vec4,
    /// width, height, unused, unused.
    pub size: UVec4,
    /// xyz unit vector toward the moon, w = its illuminance normal to the
    /// beam (lx; 0 = no moon).
    pub moon: Vec4,
    /// rgb moonlight colour (luma = the tint's luma), w = radiance of the
    /// moonlit sky (cd/m², uniform).
    pub moon_sky: Vec4,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            inv_view_proj: Mat4::IDENTITY,
            cam_pos: Vec4::ZERO,
            tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
            housing_half: Vec4::new(0.1, 0.04, 0.1, 1.0),
            lum: Vec4::new(0.08, 0.08, 1.0, 0.0),
            counts: UVec4::new(1, 1, 0, 0),
            flags: UVec4::new(1, 0, 3, 0),
            frame: UVec4::ZERO,
            f0: Vec4::new(1.0, 1.0, 3.5, 1.0),
            f1: Vec4::new(0.25, 0.01, 0.0, 0.0),
            f2: Vec4::new(100.0, 1.0, 100.0, 10.0),
            size: UVec4::new(1, 1, 0, 0),
            moon: Vec4::ZERO,
            moon_sky: Vec4::ZERO,
        }
    }
}

/// `Prim` in `trace.wgsl`. `kind`: 0 oriented box (`p0` centre, `p1`
/// half extents, `r0..r2` rotation columns), 1 sphere (`p0` centre + radius),
/// 2 capsule and 3 capped cylinder (`p0` = a + radius, `p1` = b). Bit 0 of
/// `flags` casts shadow, bit 1 hidden from primary rays (cutaway).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPrim {
    pub kind: u32,
    pub material: u32,
    pub flags: u32,
    /// Reflectance overriding the material's brightness, 0 = as is.
    pub reflectance: f32,
    pub p0: [f32; 4],
    pub p1: [f32; 4],
    pub r0: [f32; 4],
    pub r1: [f32; 4],
    pub r2: [f32; 4],
}

pub const PRIM_OBB: u32 = 0;
pub const PRIM_SPHERE: u32 = 1;
pub const PRIM_CAPSULE: u32 = 2;
pub const PRIM_CYLINDER: u32 = 3;

/// `Lamp` in `trace.wgsl`: position (w = illuminance at the eye for the
/// glare term) and the local→world rotation as three columns.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuLamp {
    pub pos: [f32; 4],
    pub r0: [f32; 4],
    pub r1: [f32; 4],
    pub r2: [f32; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_sizes_match_wgsl() {
        // mat4 + 13 vec4 (the last two: moon, moon_sky)
        assert_eq!(std::mem::size_of::<Params>(), 64 + 13 * 16);
        assert_eq!(std::mem::size_of::<GpuPrim>(), 96);
        assert_eq!(std::mem::size_of::<GpuLamp>(), 64);
    }
}
