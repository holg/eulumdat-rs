//! Light scene for the path-traced luminaire viewer, engine-free.
//!
//! Puts an LDT/IES into an urban street, a parking lot, in front of a facade
//! or into an office. The scene is a ground plane and axis-aligned boxes,
//! lit by the real intensity table through next-event estimation in a
//! progressive path tracer (`TRACE_WGSL`, three compute entry points on one
//! bind group). Three views share one accumulation: a tonemapped photo,
//! false-colour illuminance with isolux lines, and measurement points on
//! EN 12464-1 / EN 13201-3 grids with Em, Emin and Uo per grid.
//!
//! This crate holds everything that does not touch a GPU API: the scenes,
//! the orbit camera, the intensity table, the grids, the GPU-side structs
//! (`gpu`), the editor sync types (`sync`) and the [`viewer::Viewer`] state
//! machine that turns settings and input into one `FrameInput` per frame.
//! `eulumdat-rt` renders it with plain wgpu; `eulumdat-bevy` with Bevy;
//! [`cpu::CpuTracer`] is the same transport in Rust, for machines without
//! WebGPU and as the reference for the grid values.
//!
//! Frame: Y up, C0 = +X, C90 = +Z, nadir = −Y. The road, the bays and the
//! facade lie on −Z, where `scene::auto_rotation` aims an asymmetric throw.
//!
//! The idea of dropping a file into a lit scene with a progressive tracer
//! follows the MIT-licensed Taman photometric-viewer
//! (<https://github.com/Taman-Skira/photometric-viewer>); scenes,
//! materials, ramp and overlay are our own.

pub use glam;

pub mod camera;
pub mod cpu;
pub mod doc;
pub mod gpu;
#[cfg(feature = "l3d")]
pub mod l3d;
pub mod measure;
pub mod mesh;
pub mod moon;
pub mod photometry;
pub mod scene;
pub mod sensing;
pub mod sync;
pub mod viewer;

/// The tracer: entry points `trace` (8×8 per pixel), `probe` (64 per
/// measurement point) and `present` (8×8 per pixel), one bind group, see
/// [`gpu`] for the layout.
pub const TRACE_WGSL: &str = include_str!("trace.wgsl");
