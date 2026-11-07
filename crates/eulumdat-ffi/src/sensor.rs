//! Presence / motion sensors (GLDF `sensor/sensldt` and `sensor/sensxml`).
//!
//! A sensor file is not a luminaire: a sensor LDT's table is the detection
//! volume in metres, SensXML holds detection areas measured per mounting
//! height. Open it with [`SensorFile::from_bytes`] and the GLDF content
//! type (the caller has to know it is a sensor), then ask for the detection
//! areas at a height or the plan view.

use std::sync::Arc;

use eulumdat::sensor::{self as core, Motion, Quality};

use crate::diagram::SvgThemeType;
use crate::error::EulumdatError;

/// Motion type (IEC 63180: tangential / radial walking = major motion,
/// presence = minor motion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SensorMotion {
    /// Walking across the detector's view.
    Tangential,
    /// Walking towards the detector.
    Radial,
    /// Seated presence (minor motion).
    Presence,
}

impl From<Motion> for SensorMotion {
    fn from(m: Motion) -> Self {
        match m {
            Motion::Tangential => SensorMotion::Tangential,
            Motion::Radial => SensorMotion::Radial,
            Motion::Presence => SensorMotion::Presence,
        }
    }
}

/// How a SensXML area was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SensorDataQuality {
    Measured,
    Interpolated,
    Extrapolated,
}

impl From<Quality> for SensorDataQuality {
    fn from(q: Quality) -> Self {
        match q {
            Quality::Measured => SensorDataQuality::Measured,
            Quality::Interpolated => SensorDataQuality::Interpolated,
            Quality::Extrapolated => SensorDataQuality::Extrapolated,
        }
    }
}

/// A point of a detection-area outline (m; x along C0, y along C90, the
/// sensor at the origin).
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct SensorPoint {
    pub x: f64,
    pub y: f64,
}

/// Detection area of one motion type at one height.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SensorDetectionArea {
    pub motion: SensorMotion,
    /// Something is detected at this height.
    pub detected: bool,
    /// Largest distance from the point below the sensor (m).
    pub max_range_m: f64,
    /// Area (m²).
    pub area_m2: f64,
    /// Diameter of the circle with the same area (m).
    pub diameter_m: f64,
    /// SensXML only: measured / interpolated / extrapolated.
    pub quality: Option<SensorDataQuality>,
    /// Sensor LDT only: depth (m below the sensor) where this motion's
    /// volume ends.
    pub depth_limit_m: Option<f64>,
    /// Outline of the area, for drawing it natively.
    pub outline: Vec<SensorPoint>,
}

impl From<core::DetectionArea> for SensorDetectionArea {
    fn from(a: core::DetectionArea) -> Self {
        Self {
            motion: a.motion.into(),
            detected: a.detected(),
            max_range_m: a.max_range(),
            area_m2: a.area_m2(),
            diameter_m: a.diameter(),
            quality: a.quality.map(Into::into),
            depth_limit_m: a.depth_limit,
            outline: a
                .footprint
                .outline
                .iter()
                .map(|&(x, y)| SensorPoint { x, y })
                .collect(),
        }
    }
}

/// What a sensor file is.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SensorSummary {
    /// "sensor LDT" or "SensXML".
    pub kind: String,
    pub is_sensxml: bool,
    pub name: String,
    pub manufacturer: String,
    /// SensXML: e.g. "IEC 63180:2020 (automated)".
    pub measurement_type: Option<String>,
    pub motions: Vec<SensorMotion>,
    /// SensXML: measured heights (m); empty for a sensor LDT.
    pub measured_heights: Vec<f64>,
    /// 2.0 m (reference height) for a sensor LDT, lowest measured height for SensXML.
    pub default_height: f64,
    /// Plausibility problems and reader notes (English).
    pub warnings: Vec<String>,
}

/// A parsed sensor file.
#[derive(Debug, uniffi::Object)]
pub struct SensorFile {
    inner: core::SensorFile,
}

#[uniffi::export]
impl SensorFile {
    /// Parse a sensor file. `content_type`: the GLDF content type
    /// (`sensor/sensldt`, `sensor/sensxml`), or empty to sniff SensXML.
    #[uniffi::constructor]
    pub fn from_bytes(bytes: Vec<u8>, content_type: String) -> Result<Arc<Self>, EulumdatError> {
        core::SensorFile::from_bytes(&bytes, &content_type)
            .map(|inner| Arc::new(Self { inner }))
            .map_err(EulumdatError::ParseError)
    }

    /// Kind, names, motion types, heights and warnings.
    pub fn summary(&self) -> SensorSummary {
        let f = &self.inner;
        SensorSummary {
            kind: f.kind_label().to_string(),
            is_sensxml: f.is_sensxml(),
            name: f.name(),
            manufacturer: f.manufacturer(),
            measurement_type: f.measurement_type(),
            motions: f.motions().into_iter().map(Into::into).collect(),
            measured_heights: f.measured_heights(),
            default_height: f.default_height(),
            warnings: f.warnings().iter().map(|w| w.message()).collect(),
        }
    }

    /// Detection areas at `height`: sensor LDT = height of the sensor above
    /// the detection plane, SensXML = mounting height. `planning` uses
    /// SensXML's idealised planning shapes.
    pub fn areas(&self, height: f64, planning: bool) -> Vec<SensorDetectionArea> {
        self.inner
            .areas(height, planning)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Plan view of the detection areas at `height` as SVG.
    pub fn plan_svg(&self, height: f64, planning: bool, size: f64, theme: SvgThemeType) -> String {
        self.inner
            .plan_svg(height, planning, size, &theme.to_core())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LDT: &str = include_str!("../../eulumdat/tests/fixtures/sensors/demo_ceiling_360.ldt");
    const XML: &[u8] =
        include_bytes!("../../eulumdat/tests/fixtures/sensors/demo_highbay_sensxml.xml");

    #[test]
    fn sensor_ldt_through_the_ffi() {
        let f = SensorFile::from_bytes(LDT.as_bytes().to_vec(), "sensor/sensldt".into()).unwrap();
        let s = f.summary();
        assert_eq!(s.kind, "sensor LDT");
        assert!(!s.is_sensxml && s.warnings.is_empty());
        assert_eq!(s.motions.len(), 3);
        let areas = f.areas(s.default_height, false);
        let t = areas
            .iter()
            .find(|a| a.motion == SensorMotion::Tangential)
            .unwrap();
        assert!(t.detected && (t.max_range_m - 8.0).abs() < 0.1);
        assert!(t.outline.len() > 10 && t.depth_limit_m.is_some());
        assert!(f
            .plan_svg(2.0, false, 400.0, SvgThemeType::Dark)
            .starts_with("<svg"));
    }

    #[test]
    fn sensxml_through_the_ffi() {
        let f = SensorFile::from_bytes(XML.to_vec(), String::new()).unwrap();
        let s = f.summary();
        assert!(s.is_sensxml);
        assert_eq!(s.measured_heights, vec![6.0, 10.0, 14.0]);
        let q = f
            .areas(8.0, false)
            .into_iter()
            .find(|a| a.motion == SensorMotion::Radial)
            .unwrap()
            .quality;
        assert_eq!(q, Some(SensorDataQuality::Interpolated));
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(SensorFile::from_bytes(b"not a sensor".to_vec(), "sensor/sensldt".into()).is_err());
    }
}
