//! One sensor file of either kind, as it comes out of a GLDF container or a
//! file drop: a sensor LDT (detection volume) or a SensXML file (measured
//! detection areas). Shared by the web apps, the CLI and the bindings.
//!
//! ```
//! use eulumdat::sensor::SensorFile;
//! let ldt = include_str!("../../tests/fixtures/sensors/demo_ceiling_360.ldt");
//! let f = SensorFile::from_bytes(ldt.as_bytes(), "sensor/sensldt").unwrap();
//! for a in f.areas(2.0, false) {
//!     println!("{}: {:.1} m, {:.1} m²", a.motion.label(), a.max_range(), a.area_m2());
//! }
//! ```

#[cfg(feature = "xml")]
use super::SensXml;
use super::{
    diagrams::sensor_theme, DetectionVolume, Footprint, Motion, Quality, SensorPlanSvg,
    SensorPlanTrace, REFERENCE_DEPTH, TRACE_COLORS,
};
use crate::diagram::SvgTheme;
use crate::Eulumdat;

/// A parsed sensor file.
#[derive(Debug, Clone)]
pub enum SensorFile {
    /// Sensor LDT (GLDF `sensor/sensldt`): the detection volume in metres.
    Ldt(DetectionVolume),
    /// SensXML (GLDF `sensor/sensxml`): detection areas per mounting height.
    #[cfg(feature = "xml")]
    Xml(SensXml),
}

/// The detection area of one motion type at one height.
#[derive(Debug, Clone)]
pub struct DetectionArea {
    pub motion: Motion,
    pub footprint: Footprint,
    /// SensXML: measured / interpolated / extrapolated; `None` for an LDT.
    pub quality: Option<Quality>,
    /// Sensor LDT: depth (m below the sensor) where this motion's volume
    /// ends; below it nothing is detected.
    pub depth_limit: Option<f64>,
}

/// Something odd about a sensor file.
#[derive(Debug, Clone, PartialEq)]
pub enum SensorWarning {
    /// A sensor LDT whose values cannot be ranges in metres (largest
    /// value `max`): usually a luminaire LDT stored as a sensor file.
    ImplausibleRange { max: f64 },
    /// A sensor LDT whose lines 33–42 do not hold sensor parameters (no
    /// mounting code 1–3 in field 8, or values rising like a luminaire's
    /// direct ratios): probably a luminaire LDT.
    NoSensorParameters,
    /// Something the SensXML reader noticed (English text).
    Reader(String),
}

/// Largest plausible detection range (m); PIR detectors reach about 20 m
/// radius even on high bays, so 60 m leaves a wide margin.
pub const MAX_PLAUSIBLE_RANGE_M: f64 = 60.0;

impl SensorWarning {
    /// English text.
    pub fn message(&self) -> String {
        match self {
            SensorWarning::ImplausibleRange { max } => format!(
                "Values up to {max:.0} cannot be detection ranges in metres; this looks like a luminaire LDT stored as a sensor file."
            ),
            SensorWarning::NoSensorParameters => "Lines 33–42 hold no sensor parameters (mounting code 1–3 in field 8); this looks like a luminaire LDT, the motion types and scales may be wrong.".to_string(),
            SensorWarning::Reader(s) => s.clone(),
        }
    }
}

impl DetectionArea {
    /// Something is detected at this height.
    pub fn detected(&self) -> bool {
        !self.footprint.is_empty()
    }
    /// Largest distance from the point below the sensor (m).
    pub fn max_range(&self) -> f64 {
        self.footprint.max_range
    }
    /// Area (m²).
    pub fn area_m2(&self) -> f64 {
        self.footprint.area_m2
    }
    /// Diameter of the circle with the same area (m).
    pub fn diameter(&self) -> f64 {
        self.footprint.equivalent_diameter()
    }
}

/// `<SensXML` within the first bytes.
fn looks_like_sensxml(bytes: &[u8]) -> bool {
    bytes.windows(8).take(1024).any(|w| w == b"<SensXML")
}

impl SensorFile {
    /// Parse a sensor file. `content_type` is the GLDF content type
    /// (`sensor/sensldt`, `sensor/sensxml`) and decides the kind; when it is
    /// empty or unknown the bytes are sniffed for `<SensXML`. Never call
    /// this for a luminaire LDT: nothing in a sensor LDT says "sensor", the
    /// caller has to know.
    pub fn from_bytes(bytes: &[u8], content_type: &str) -> Result<Self, String> {
        let ct = content_type.trim().to_ascii_lowercase();
        let xml =
            ct.ends_with("sensxml") || (!ct.ends_with("sensldt") && looks_like_sensxml(bytes));
        if xml {
            #[cfg(feature = "xml")]
            {
                return SensXml::parse(bytes).map(SensorFile::Xml);
            }
            #[cfg(not(feature = "xml"))]
            {
                return Err("SensXML needs the `xml` feature".to_string());
            }
        }
        // EULUMDAT files are often Latin-1.
        let text = match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| b as char).collect(),
        };
        Eulumdat::parse(&text)
            .map(|ldt| SensorFile::Ldt(DetectionVolume::new(ldt)))
            .map_err(|e| e.to_string())
    }

    /// The file is SensXML.
    pub fn is_sensxml(&self) -> bool {
        !matches!(self, SensorFile::Ldt(_))
    }

    /// "sensor LDT" or "SensXML".
    pub fn kind_label(&self) -> &'static str {
        match self {
            SensorFile::Ldt(_) => "sensor LDT",
            #[cfg(feature = "xml")]
            SensorFile::Xml(_) => "SensXML",
        }
    }

    /// Product name from the file (may be empty).
    pub fn name(&self) -> String {
        match self {
            SensorFile::Ldt(v) => {
                let l = v.ldt();
                if l.luminaire_name.trim().is_empty() {
                    l.identification.trim().to_string()
                } else {
                    l.luminaire_name.trim().to_string()
                }
            }
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => x
                .header
                .product_name
                .clone()
                .or_else(|| x.header.product_number.clone())
                .unwrap_or_default(),
        }
    }

    /// Manufacturer from the file (may be empty).
    pub fn manufacturer(&self) -> String {
        match self {
            SensorFile::Ldt(v) => v.ldt().identification.trim().to_string(),
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => x.header.manufacturer.clone().unwrap_or_default(),
        }
    }

    /// SensXML: how it was measured (e.g. "IEC 63180:2020 (automated)").
    pub fn measurement_type(&self) -> Option<String> {
        match self {
            SensorFile::Ldt(_) => None,
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => x.sensor.measurement_type.clone(),
        }
    }

    /// Plausibility problems and reader notes.
    pub fn warnings(&self) -> Vec<SensorWarning> {
        match self {
            SensorFile::Ldt(v) => {
                let max = v
                    .ldt()
                    .intensities
                    .iter()
                    .flatten()
                    .cloned()
                    .fold(0.0, f64::max);
                if max > MAX_PLAUSIBLE_RANGE_M {
                    vec![SensorWarning::ImplausibleRange { max }]
                } else if !super::has_sensor_parameters(v.ldt()) {
                    vec![SensorWarning::NoSensorParameters]
                } else {
                    Vec::new()
                }
            }
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => x
                .warnings
                .iter()
                .cloned()
                .map(SensorWarning::Reader)
                .collect(),
        }
    }

    /// Motion types the file has data for.
    pub fn motions(&self) -> Vec<Motion> {
        Motion::ALL
            .into_iter()
            .filter(|m| match self {
                SensorFile::Ldt(v) => v.motion_footprint(*m, REFERENCE_DEPTH, 1.0).is_some(),
                #[cfg(feature = "xml")]
                SensorFile::Xml(x) => x.source(*m).is_some(),
            })
            .collect()
    }

    /// SensXML: all measured heights (m, ascending); empty for an LDT.
    pub fn measured_heights(&self) -> Vec<f64> {
        match self {
            SensorFile::Ldt(_) => Vec::new(),
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => {
                let mut h: Vec<f64> = Motion::ALL.iter().flat_map(|m| x.heights(*m)).collect();
                h.sort_by(f64::total_cmp);
                h.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
                h
            }
        }
    }

    /// A sensible first height: the reference height 2.0 m for an LDT, the
    /// lowest measured height for SensXML.
    pub fn default_height(&self) -> f64 {
        self.measured_heights()
            .first()
            .copied()
            .unwrap_or(REFERENCE_DEPTH)
    }

    /// Detection areas of all motion types at `height`: for a sensor LDT
    /// the height of the sensor above the detection plane, for SensXML the
    /// mounting height of the measurement. `planning` uses SensXML's
    /// idealised planning shapes instead of the measured data.
    pub fn areas(&self, height: f64, planning: bool) -> Vec<DetectionArea> {
        let _ = planning;
        Motion::ALL
            .into_iter()
            .filter_map(|m| match self {
                SensorFile::Ldt(v) => {
                    v.motion_footprint(m, height, 1.0)
                        .map(|footprint| DetectionArea {
                            motion: m,
                            footprint,
                            quality: None,
                            depth_limit: v.motion_depth_limit(m),
                        })
                }
                #[cfg(feature = "xml")]
                SensorFile::Xml(x) => {
                    x.footprint(m, height, planning)
                        .map(|(footprint, q)| DetectionArea {
                            motion: m,
                            footprint,
                            quality: Some(q),
                            depth_limit: None,
                        })
                }
            })
            .collect()
    }

    /// Footprint of one motion type for a sensor mounted `mount` metres
    /// above the floor, evaluated on the plane `plane` metres above the
    /// floor (walking: floor; seated presence: workplane). A sensor LDT is
    /// cut at `mount − plane`; SensXML areas are keyed by the mounting
    /// height (the measurement fixes the plane), so `plane` is not used.
    pub fn area_at(
        &self,
        motion: Motion,
        mount: f64,
        plane: f64,
        planning: bool,
    ) -> Option<DetectionArea> {
        match self {
            SensorFile::Ldt(v) => {
                let _ = planning;
                let depth = mount - plane;
                if depth <= 0.0 {
                    return None;
                }
                v.motion_footprint(motion, depth, 1.0)
                    .map(|footprint| DetectionArea {
                        motion,
                        footprint,
                        quality: None,
                        depth_limit: v.motion_depth_limit(motion),
                    })
            }
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => {
                let _ = plane;
                x.footprint(motion, mount, planning)
                    .map(|(footprint, q)| DetectionArea {
                        motion,
                        footprint,
                        quality: Some(q),
                        depth_limit: None,
                    })
            }
        }
    }

    /// Plan view of the detection areas at `height` (English labels).
    pub fn plan_svg(&self, height: f64, planning: bool, size: f64, theme: &SvgTheme) -> String {
        self.plan_svg_with_labels(height, planning, size, theme, &|m| m.label().to_string())
    }

    /// Plan view with the motion names from `label` (translations).
    pub fn plan_svg_with_labels(
        &self,
        height: f64,
        planning: bool,
        size: f64,
        theme: &SvgTheme,
        label: &dyn Fn(Motion) -> String,
    ) -> String {
        let traces: Vec<SensorPlanTrace> = self
            .areas(height, planning)
            .into_iter()
            .filter(|a| a.detected())
            .map(|a| {
                let i = Motion::ALL.iter().position(|m| *m == a.motion).unwrap_or(0);
                SensorPlanTrace {
                    label: format!("{} · {height:.2} m", label(a.motion)),
                    color: TRACE_COLORS[i].to_string(),
                    footprint: a.footprint,
                }
            })
            .collect();
        SensorPlanSvg::render(&traces, size, &sensor_theme(theme))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LDT: &str = include_str!("../../tests/fixtures/sensors/demo_ceiling_360.ldt");

    #[test]
    fn sensor_ldt_areas_and_plan() {
        let f = SensorFile::from_bytes(LDT.as_bytes(), "sensor/sensldt").unwrap();
        assert!(!f.is_sensxml());
        assert_eq!(f.kind_label(), "sensor LDT");
        assert_eq!(f.default_height(), REFERENCE_DEPTH);
        let areas = f.areas(2.0, false);
        assert_eq!(areas.len(), 3, "tangential, radial, presence");
        let t = &areas[0];
        assert_eq!(t.motion, Motion::Tangential);
        assert!(
            t.detected() && (t.max_range() - 8.0).abs() < 0.1,
            "{}",
            t.max_range()
        );
        assert!(t.depth_limit.is_some() && t.quality.is_none());
        let svg = f.plan_svg(2.0, false, 400.0, &SvgTheme::light());
        assert!(svg.starts_with("<svg") && svg.contains("tangential walking"));
        // Far below the end of the volume nothing is detected.
        assert!(f.areas(50.0, false).iter().all(|a| !a.detected()));
    }

    #[cfg(feature = "xml")]
    #[test]
    fn sensxml_is_sniffed_and_has_heights() {
        let xml = include_bytes!("../../tests/fixtures/sensors/demo_highbay_sensxml.xml");
        let f = SensorFile::from_bytes(xml, "").unwrap();
        assert!(f.is_sensxml());
        assert_eq!(f.measured_heights(), vec![6.0, 10.0, 14.0]);
        assert_eq!(f.default_height(), 6.0);
        // Walking is measured at 6 / 10 / 14 m, presence only at 6 m.
        let q = |h: f64, m: Motion| {
            f.areas(h, false)
                .into_iter()
                .find(|a| a.motion == m)
                .unwrap()
                .quality
        };
        assert_eq!(q(10.0, Motion::Tangential), Some(Quality::Measured));
        assert_eq!(q(8.0, Motion::Radial), Some(Quality::Interpolated));
        assert_eq!(q(10.0, Motion::Presence), Some(Quality::Extrapolated));
        assert!(!f.motions().is_empty());
    }

    #[test]
    fn luminaire_ldt_as_sensor_is_flagged() {
        let lum = include_str!("../../../eulumdat-wasm/templates/fluorescent_luminaire.ldt");
        let f = SensorFile::from_bytes(lum.as_bytes(), "sensor/sensldt").unwrap();
        assert!(matches!(
            f.warnings()[..],
            [SensorWarning::ImplausibleRange { .. }]
        ));
        let ok = SensorFile::from_bytes(LDT.as_bytes(), "sensor/sensldt").unwrap();
        assert!(ok.warnings().is_empty());
    }

    #[test]
    fn content_type_wins_over_sniffing() {
        // A sensor LDT never sniffs as SensXML, and "sensldt" forces the LDT path.
        assert!(SensorFile::from_bytes(b"<SensXML>", "sensor/sensldt").is_err());
    }
}
