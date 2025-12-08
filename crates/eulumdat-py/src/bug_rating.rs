//! BUG rating types for Python bindings

use pyo3::prelude::*;

/// BUG (Backlight-Uplight-Glare) rating per IESNA TM-15-11.
#[pyclass]
#[derive(Clone)]
pub struct BugRating {
    /// Backlight rating (0-5).
    #[pyo3(get)]
    pub b: u8,
    /// Uplight rating (0-5).
    #[pyo3(get)]
    pub u: u8,
    /// Glare rating (0-5).
    #[pyo3(get)]
    pub g: u8,
}

#[pymethods]
impl BugRating {
    fn __repr__(&self) -> String {
        format!("B{} U{} G{}", self.b, self.u, self.g)
    }

    fn __str__(&self) -> String {
        format!("B{} U{} G{}", self.b, self.u, self.g)
    }
}

/// Zone lumens breakdown for BUG rating calculation.
#[pyclass]
#[derive(Clone)]
pub struct ZoneLumens {
    /// Backlight Low: 0-30°
    #[pyo3(get)]
    pub bl: f64,
    /// Backlight Mid: 30-60°
    #[pyo3(get)]
    pub bm: f64,
    /// Backlight High: 60-80°
    #[pyo3(get)]
    pub bh: f64,
    /// Backlight Very High: 80-90°
    #[pyo3(get)]
    pub bvh: f64,
    /// Forward Low: 0-30°
    #[pyo3(get)]
    pub fl: f64,
    /// Forward Mid: 30-60°
    #[pyo3(get)]
    pub fm: f64,
    /// Forward High: 60-80°
    #[pyo3(get)]
    pub fh: f64,
    /// Forward Very High: 80-90°
    #[pyo3(get)]
    pub fvh: f64,
    /// Uplight Low: 90-100°
    #[pyo3(get)]
    pub ul: f64,
    /// Uplight High: 100-180°
    #[pyo3(get)]
    pub uh: f64,
}

#[pymethods]
impl ZoneLumens {
    fn __repr__(&self) -> String {
        format!(
            "ZoneLumens(BL={:.1}, BM={:.1}, BH={:.1}, BVH={:.1}, FL={:.1}, FM={:.1}, FH={:.1}, FVH={:.1}, UL={:.1}, UH={:.1})",
            self.bl, self.bm, self.bh, self.bvh, self.fl, self.fm, self.fh, self.fvh, self.ul, self.uh
        )
    }
}
