//! eulumdat-rt — GPU ray tracing engine for photometric simulation.
//!
//! Pure wgpu compute pipeline. No Bevy dependency in Phase 1.
//! Validates against the CPU reference (eulumdat-goniosim) via CIE 171:2006.
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat_rt::GpuTracer;
//!
//! let tracer = pollster::block_on(GpuTracer::new()).unwrap();
//! let result = pollster::block_on(tracer.trace_isotropic(1_000_000, 15.0, 5.0));
//! println!("Detected energy: {}", result.total_energy());
//! ```

pub mod camera;
mod pipeline;

pub use camera::{CameraConfig, CameraImage, GpuCamera};
pub use pipeline::{
    GpuDetectorResult, GpuMaterial, GpuPrimitive, GpuTracer, GpuTracerConfig, SourceType,
};
