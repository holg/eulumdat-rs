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
    E: Display,
{
    match pollster::block_on(fut) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("SKIP: no GPU adapter available ({e})");
            None
        }
    }
}
