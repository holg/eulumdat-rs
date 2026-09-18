//! Shared test helpers for eulumdat-rt integration tests.

use std::fmt::Display;
use std::future::Future;

/// Block on a GPU constructor; return `Some(v)` on success.
/// On failure (e.g. headless CI with no adapter), print a SKIP notice and return `None`.
/// Tests should early-return when this yields `None`:
///
/// ```ignore
/// let Some(tracer) = gpu_or_skip(eulumdat_rt::GpuTracer::new()) else { return; };
/// ```
pub fn gpu_or_skip<F, T, E>(fut: F) -> Option<T>
where
    F: Future<Output = Result<T, E>>,
    T: GpuHandle,
    E: Display,
{
    match pollster::block_on(fut) {
        Ok(v) if v.is_software() => {
            eprintln!(
                "SKIP: software adapter ({}) — GPU tests need real hardware",
                v.adapter_name()
            );
            None
        }
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("SKIP: no GPU adapter available ({e})");
            None
        }
    }
}

/// Anything constructed on a wgpu adapter that can tell us what it is.
pub trait GpuHandle {
    fn is_software(&self) -> bool;
    fn adapter_name(&self) -> String;
}

impl GpuHandle for eulumdat_rt::GpuTracer {
    fn is_software(&self) -> bool {
        self.is_software_adapter()
    }
    fn adapter_name(&self) -> String {
        self.adapter_info().name.clone()
    }
}

impl GpuHandle for eulumdat_rt::GpuCamera {
    fn is_software(&self) -> bool {
        self.is_software_adapter()
    }
    fn adapter_name(&self) -> String {
        self.adapter_info().name.clone()
    }
}
