//! Presence / motion sensors (GLDF `sensor/sensldt`, `sensor/sensxml`).
//!
//! ```python
//! import eulumdat
//! f = eulumdat.SensorFile.from_file("sensor.ldt", "sensor/sensldt")
//! for a in f.areas(2.0):
//!     print(a.motion, a.max_range_m, a.area_m2)
//! open("plan.svg", "w").write(f.plan_svg(2.0))
//! ```

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use ::eulumdat as core;
use core::sensor::{Motion, Quality};

use crate::diagram::SvgTheme;

fn motion_name(m: Motion) -> &'static str {
    match m {
        Motion::Tangential => "tangential",
        Motion::Radial => "radial",
        Motion::Presence => "presence",
    }
}

fn quality_name(q: Quality) -> &'static str {
    match q {
        Quality::Measured => "measured",
        Quality::Interpolated => "interpolated",
        Quality::Extrapolated => "extrapolated",
    }
}

/// Detection area of one motion type at one height.
#[pyclass(frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct DetectionArea {
    /// "tangential", "radial" or "presence" (IEC 63180: major motion
    /// across / towards the detector, minor motion).
    #[pyo3(get)]
    pub motion: String,
    /// Something is detected at this height.
    #[pyo3(get)]
    pub detected: bool,
    /// Largest distance from the point below the sensor (m).
    #[pyo3(get)]
    pub max_range_m: f64,
    /// Area (m²).
    #[pyo3(get)]
    pub area_m2: f64,
    /// Diameter of the circle with the same area (m).
    #[pyo3(get)]
    pub diameter_m: f64,
    /// SensXML: "measured", "interpolated" or "extrapolated"; None for a sensor LDT.
    #[pyo3(get)]
    pub quality: Option<String>,
    /// Sensor LDT: depth (m below the sensor) where this motion's volume ends.
    #[pyo3(get)]
    pub depth_limit_m: Option<f64>,
    /// Outline as (x, y) points in m, sensor at the origin, x along C0.
    #[pyo3(get)]
    pub outline: Vec<(f64, f64)>,
}

#[pymethods]
impl DetectionArea {
    fn __repr__(&self) -> String {
        if self.detected {
            format!(
                "DetectionArea({}, max_range_m={:.2}, area_m2={:.1})",
                self.motion, self.max_range_m, self.area_m2
            )
        } else {
            format!("DetectionArea({}, not detected)", self.motion)
        }
    }
}

impl From<core::sensor::DetectionArea> for DetectionArea {
    fn from(a: core::sensor::DetectionArea) -> Self {
        Self {
            motion: motion_name(a.motion).to_string(),
            detected: a.detected(),
            max_range_m: a.max_range(),
            area_m2: a.area_m2(),
            diameter_m: a.diameter(),
            quality: a.quality.map(|q| quality_name(q).to_string()),
            depth_limit_m: a.depth_limit,
            outline: a.footprint.outline.clone(),
        }
    }
}

/// A sensor file: sensor LDT (detection volume in metres) or SensXML
/// (detection areas measured per mounting height). Never open a luminaire
/// LDT with this: the caller has to know the file is a sensor (GLDF
/// content type).
#[pyclass]
pub struct SensorFile {
    inner: core::sensor::SensorFile,
}

#[pymethods]
impl SensorFile {
    /// Parse bytes; `content_type` = GLDF content type ("sensor/sensldt",
    /// "sensor/sensxml") or "" to sniff SensXML.
    #[staticmethod]
    #[pyo3(signature = (data, content_type = ""))]
    fn from_bytes(data: &[u8], content_type: &str) -> PyResult<Self> {
        core::sensor::SensorFile::from_bytes(data, content_type)
            .map(|inner| Self { inner })
            .map_err(PyValueError::new_err)
    }

    /// Read a file; `content_type` as for `from_bytes` (a `.xml` file is
    /// sniffed for `<SensXML`).
    #[staticmethod]
    #[pyo3(signature = (path, content_type = ""))]
    fn from_file(path: &str, content_type: &str) -> PyResult<Self> {
        let data = std::fs::read(path).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Self::from_bytes(&data, content_type)
    }

    /// "sensor LDT" or "SensXML".
    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.kind_label()
    }

    #[getter]
    fn is_sensxml(&self) -> bool {
        self.inner.is_sensxml()
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name()
    }

    #[getter]
    fn manufacturer(&self) -> String {
        self.inner.manufacturer()
    }

    /// SensXML: e.g. "IEC 63180:2020 (automated)".
    #[getter]
    fn measurement_type(&self) -> Option<String> {
        self.inner.measurement_type()
    }

    /// Motion types the file has data for.
    #[getter]
    fn motions(&self) -> Vec<&'static str> {
        self.inner.motions().into_iter().map(motion_name).collect()
    }

    /// SensXML: measured heights (m); empty for a sensor LDT.
    #[getter]
    fn measured_heights(&self) -> Vec<f64> {
        self.inner.measured_heights()
    }

    /// 2.0 m (reference height) for a sensor LDT, lowest measured height for SensXML.
    #[getter]
    fn default_height(&self) -> f64 {
        self.inner.default_height()
    }

    /// Plausibility problems and reader notes.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.warnings().iter().map(|w| w.message()).collect()
    }

    /// Detection areas at `height` (sensor LDT: height of the sensor above
    /// the detection plane; SensXML: mounting height). `planning` uses
    /// SensXML's idealised planning shapes.
    #[pyo3(signature = (height, planning = false))]
    fn areas(&self, height: f64, planning: bool) -> Vec<DetectionArea> {
        self.inner
            .areas(height, planning)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Plan view of the detection areas at `height` as SVG.
    #[pyo3(signature = (height, planning = false, size = 480.0, theme = SvgTheme::Light))]
    fn plan_svg(&self, height: f64, planning: bool, size: f64, theme: SvgTheme) -> String {
        self.inner
            .plan_svg(height, planning, size, &theme.to_core())
    }

    fn __repr__(&self) -> String {
        format!(
            "SensorFile({}, {:?})",
            self.inner.kind_label(),
            self.inner.name()
        )
    }
}
