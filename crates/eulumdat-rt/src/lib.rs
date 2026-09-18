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

/// True when `info` describes a CPU / software adapter (DX12 WARP
/// "Microsoft Basic Render Driver", Mesa llvmpipe, SwiftShader, …).
/// Such adapters exist on headless CI runners but cannot be trusted to run
/// the compute pipelines (WARP fails the readback with `BufferAsyncError`),
/// so GPU tests skip on them.
pub fn is_software_adapter(info: &wgpu::AdapterInfo) -> bool {
    let name = info.name.to_ascii_lowercase();
    matches!(info.device_type, wgpu::DeviceType::Cpu)
        || name.contains("basic render")
        || name.contains("warp")
        || name.contains("llvmpipe")
        || name.contains("swiftshader")
        || name.contains("software")
}
