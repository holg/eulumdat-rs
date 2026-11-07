//! SensXML reader (Relux / sensNORM sensor format, GLDF `sensor/sensxml`).
//!
//! There is no public schema. This reader follows what real files contain
//! (22 files from Signify, Osram, Steinel, Kopp, ESYLUX and Relux, versions
//! 3.9 – 4.5, see `docs/sensors-plan.md` §3.1a). Meanings marked "comment"
//! are documented by XML comments inside manufacturer files:
//!
//! * `DetectionSource/DetectionType`: 1 tangential, 2 radial, 3 presence (comment)
//! * `Sensor/MountingType`: 1 wall, 2 ceiling, 3 wall/ceiling (comment)
//! * `PolarData` / `GridData` `DistanceFromCenter`: mounting height in m
//!   (comments "2.5m", "3m")
//! * `PolarData/RangeData angle="…"`: detection radius in m on the floor at
//!   that azimuth
//! * `GridData/CellData x y`: 1 = detected, cell centre in m; a single 0
//!   cell is a placeholder ("dummy cell data", comment) when only the
//!   planning shape is given
//! * `PlanningData/Shape`: 2 circle (`Radius`, comment); 1 and 4 rectangle
//!   (`Length` along C0 × `Width`); 3 polygon (`Vertex` = "x, y" in m)
//!
//! Relux's plan view of a SensXML file is the planning shapes at a 2.0 m
//! mounting height, scaled linearly below the lowest measured height:
//! `footprint(m, 2.0, true)` reproduces its renders (KOOP PD-360-12m
//! 4.80 / 2.40 / 1.60 m vs 4.81 / 2.42 / 1.61 m; Osram OCI 3.50 / 2.00 /
//! 2.40 m vs 3.48 / 1.99 / 2.39 m).
//!
//! The reader never fails on unknown content: every field is optional,
//! unknown elements are kept in [`SensXml::extras`], oddities are listed in
//! [`SensXml::warnings`].

use super::{DetectionVolume, Footprint, Motion};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Element tree node.
#[derive(Debug, Clone, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<Node>,
}

impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }
    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
    fn text_of(&self, name: &str) -> Option<String> {
        self.child(name)
            .map(|c| c.text.trim().to_string())
            .filter(|s| !s.is_empty())
    }
    fn num_of(&self, name: &str) -> Option<f64> {
        self.text_of(name).and_then(|s| num(&s))
    }
    fn int_of(&self, name: &str) -> Option<i32> {
        self.num_of(name).map(|v| v.round() as i32)
    }
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn num(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Bytes → text. Files declare ISO-8859-1 or UTF-8; some declare Latin-1
/// but contain UTF-8 (seen: "PrÃ¤senz"), so valid non-ASCII UTF-8 wins.
fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if let Ok(s) = core::str::from_utf8(bytes) {
        return s.to_string();
    }
    // Latin-1 / Windows-1252 (treated as Latin-1): every byte is one char.
    bytes.iter().map(|&b| b as char).collect()
}

fn parse_tree(text: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Node> = vec![Node::default()];
    let mut buf = Vec::new();
    let open = |e: &quick_xml::events::BytesStart| Node {
        name: String::from_utf8_lossy(e.local_name().as_ref()).to_string(),
        attrs: e
            .attributes()
            .flatten()
            .map(|a| {
                (
                    String::from_utf8_lossy(a.key.local_name().as_ref()).to_string(),
                    a.unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).to_string()),
                )
            })
            .collect(),
        ..Node::default()
    };
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => stack.push(open(&e)),
            Ok(Event::Empty(e)) => {
                let n = open(&e);
                if let Some(top) = stack.last_mut() {
                    top.children.push(n);
                }
            }
            Ok(Event::Text(t)) => {
                let s = t
                    .unescape()
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&t).to_string());
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&s);
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() > 1 {
                    let n = stack.pop().unwrap();
                    stack.last_mut().unwrap().children.push(n);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(format!("XML error at {}: {e}", reader.buffer_position())),
        }
        buf.clear();
    }
    // close anything left open (truncated files)
    while stack.len() > 1 {
        let n = stack.pop().unwrap();
        stack.last_mut().unwrap().children.push(n);
    }
    stack
        .pop()
        .and_then(|root| root.children.into_iter().next())
        .ok_or_else(|| "empty document".to_string())
}

/// `Header` block.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SensXmlHeader {
    pub manufacturer: Option<String>,
    pub product_name: Option<String>,
    pub product_group: Option<String>,
    pub product_number: Option<String>,
    pub product_type: Option<String>,
    pub product_technology: Option<String>,
    pub description: Option<String>,
    pub laboratory: Option<String>,
    pub report_number: Option<String>,
    pub report_date: Option<String>,
    pub unique_identifier: Option<String>,
}

/// `Sensor` block.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SensXmlSensor {
    /// W
    pub total_system_power: Option<f64>,
    /// 1 wall, 2 ceiling, 3 wall/ceiling
    pub mounting_type: Option<i32>,
    /// Number of detection sources
    pub quantity: Option<i32>,
    /// Housing length / width / height as written (files say "mm" but hold m)
    pub dimensions: [Option<f64>; 3],
    /// Detection centre offset from the housing centre (length, width, height)
    pub center_offset: [Option<f64>; 3],
    /// `Measurement/Type`, e.g. "IEC 63180:2020 (automated)", "sensNorm"
    pub measurement_type: Option<String>,
}

/// Idealised planning shape (`PlanningData`).
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Planning {
    pub shape: Option<i32>,
    pub radius: Option<f64>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    /// % of triggers inside the shape (sensNORM uses 85)
    pub detection_rate: Option<f64>,
    pub polygon: Vec<(f64, f64)>,
}

impl Planning {
    /// Radius of the shape at azimuth `deg` (C0 = +x), if the shape is known.
    pub fn radius_at(&self, deg: f64) -> Option<f64> {
        let a = deg.to_radians();
        match self.shape {
            Some(3) if self.polygon.len() >= 3 => Some(ray_polygon(&self.polygon, a)),
            Some(1) | Some(4) => {
                let (l, w) = (self.length? / 2.0, self.width? / 2.0);
                if l <= 0.0 || w <= 0.0 {
                    return None;
                }
                let (c, s) = (a.cos().abs(), a.sin().abs());
                Some(
                    (if c > 1e-12 { l / c } else { f64::INFINITY }).min(if s > 1e-12 {
                        w / s
                    } else {
                        f64::INFINITY
                    }),
                )
            }
            _ => {
                let r = self.radius.filter(|r| *r > 0.0)?;
                let (s, e) = (
                    self.start_angle.unwrap_or(0.0),
                    self.end_angle.unwrap_or(0.0),
                );
                let full = (e - s).abs() < 1e-9 || (e - s).abs() >= 360.0 - 1e-9;
                let inside = full || {
                    let d = (deg - s).rem_euclid(360.0);
                    d <= (e - s).rem_euclid(360.0)
                };
                Some(if inside { r } else { 0.0 })
            }
        }
    }
}

fn ray_polygon(poly: &[(f64, f64)], a: f64) -> f64 {
    let (dx, dy) = (a.cos(), a.sin());
    let mut best = 0.0f64;
    for i in 0..poly.len() {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % poly.len()];
        let (ex, ey) = (x2 - x1, y2 - y1);
        let den = dx * ey - dy * ex;
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (x1 * ey - y1 * ex) / den;
        let u = (x1 * dy - y1 * dx) / den;
        if t >= 0.0 && (-1e-9..=1.0 + 1e-9).contains(&u) {
            best = best.max(t);
        }
    }
    best
}

/// One measured height of a detection source.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Sample {
    /// `PolarData`: radius (m) per azimuth (deg).
    Polar {
        mounting_height: f64,
        ranges: Vec<(f64, f64)>,
        planning: Option<Planning>,
    },
    /// `GridData`: detected cell centres (m) and the cell size.
    Grid {
        mounting_height: f64,
        cell: (f64, f64),
        cells: Vec<(f64, f64)>,
        planning: Option<Planning>,
    },
}

impl Sample {
    pub fn mounting_height(&self) -> f64 {
        match self {
            Sample::Polar {
                mounting_height, ..
            }
            | Sample::Grid {
                mounting_height, ..
            } => *mounting_height,
        }
    }
    pub fn planning(&self) -> Option<&Planning> {
        match self {
            Sample::Polar { planning, .. } | Sample::Grid { planning, .. } => planning.as_ref(),
        }
    }
    fn has_measured(&self) -> bool {
        match self {
            Sample::Polar { ranges, .. } => !ranges.is_empty(),
            Sample::Grid { cells, .. } => !cells.is_empty(),
        }
    }
    /// Radius per azimuth on a 1° grid (360 values), measured or planning.
    fn radius_profile(&self, use_planning: bool, rotation: f64) -> Option<Vec<f64>> {
        let mut out = vec![0.0; 360];
        if use_planning || !self.has_measured() {
            let p = self.planning()?;
            for (i, r) in out.iter_mut().enumerate() {
                *r = p.radius_at(i as f64 - rotation)?.min(1e3);
            }
            return Some(out);
        }
        match self {
            Sample::Polar { ranges, .. } => {
                let mut pts: Vec<(f64, f64)> = ranges
                    .iter()
                    .map(|&(a, r)| ((a + rotation).rem_euclid(360.0), r))
                    .collect();
                pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
                pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
                if pts.is_empty() {
                    return None;
                }
                for (i, r) in out.iter_mut().enumerate() {
                    *r = interp_circular(&pts, i as f64);
                }
            }
            Sample::Grid { cell, cells, .. } => {
                let half = 0.5 * cell.0.hypot(cell.1);
                for &(x, y) in cells {
                    let dist = x.hypot(y);
                    // far edge of the cell along the ray through its centre
                    let d = if dist > 1e-9 {
                        dist + 0.5 * (cell.0 * (x / dist).abs() + cell.1 * (y / dist).abs())
                    } else {
                        half
                    };
                    let a = (y.atan2(x).to_degrees() + rotation).rem_euclid(360.0);
                    // spread the cell over the azimuths it covers
                    let span = if x.hypot(y) > 1e-9 {
                        (half / x.hypot(y)).atan().to_degrees().ceil() as i32
                    } else {
                        180
                    };
                    for k in -span..=span {
                        let i = (a.round() as i32 + k).rem_euclid(360) as usize;
                        out[i] = out[i].max(d);
                    }
                }
            }
        }
        Some(out)
    }
}

fn interp_circular(pts: &[(f64, f64)], a: f64) -> f64 {
    if pts.len() == 1 {
        return pts[0].1;
    }
    let n = pts.len();
    let i = pts.iter().rposition(|p| p.0 <= a);
    let (lo, hi) = match i {
        Some(i) => (
            pts[i],
            if i + 1 < n {
                pts[i + 1]
            } else {
                (pts[0].0 + 360.0, pts[0].1)
            },
        ),
        None => ((pts[n - 1].0 - 360.0, pts[n - 1].1), pts[0]),
    };
    let span = hi.0 - lo.0;
    if span <= 1e-12 {
        return lo.1;
    }
    lo.1 + (hi.1 - lo.1) * (a - lo.0) / span
}

/// `DetectionSource`: one motion type at one or more mounting heights.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DetectionSource {
    pub detection_type: Option<i32>,
    /// Rotation of the pattern (deg), added to the azimuths.
    pub rotation: f64,
    pub show_raw_data: Option<bool>,
    pub samples: Vec<Sample>,
}

impl DetectionSource {
    pub fn motion(&self) -> Option<Motion> {
        match self.detection_type? {
            1 => Some(Motion::Tangential),
            2 => Some(Motion::Radial),
            3 => Some(Motion::Presence),
            _ => None,
        }
    }
}

/// How a footprint was obtained (defined in `params`, shared with
/// [`super::SensorFile`]).
pub use super::params::Quality;

/// A parsed SensXML file.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SensXml {
    pub version: Option<String>,
    pub header: SensXmlHeader,
    pub sensor: SensXmlSensor,
    pub sources: Vec<DetectionSource>,
    /// Elements this reader does not map (path, text).
    pub extras: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

const KNOWN_TOP: &[&str] = &["Version", "Header", "Sensor", "DetectionSource"];

impl SensXml {
    /// Parse SensXML bytes (any of the observed encodings).
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let text = decode(bytes);
        let root = parse_tree(&text)?;
        if root.name != "SensXML" {
            return Err(format!("root element is <{}>, not <SensXML>", root.name));
        }
        let mut out = SensXml {
            version: root.text_of("Version"),
            ..Default::default()
        };
        if let Some(h) = root.child("Header") {
            out.header = SensXmlHeader {
                manufacturer: h.text_of("Manufacturer"),
                product_name: h.text_of("ProductName"),
                product_group: h.text_of("ProductGroup"),
                product_number: h.text_of("ProductNumber"),
                product_type: h.text_of("ProductType"),
                product_technology: h.text_of("ProductTechnology"),
                description: h.text_of("Description"),
                laboratory: h.text_of("Laboratory"),
                report_number: h.text_of("ReportNumber"),
                report_date: h.text_of("ReportDate"),
                unique_identifier: h.text_of("UniqueIdentifier"),
            };
        }
        if let Some(s) = root.child("Sensor") {
            let dim = s.child("Dimensions");
            let cen = s.child("SensorCenter");
            out.sensor = SensXmlSensor {
                total_system_power: s.num_of("TotalSystemPower"),
                mounting_type: s.int_of("MountingType"),
                quantity: s.int_of("Quantity"),
                dimensions: [
                    dim.and_then(|d| d.num_of("Length")),
                    dim.and_then(|d| d.num_of("Width")),
                    dim.and_then(|d| d.num_of("Height")),
                ],
                center_offset: [
                    cen.and_then(|d| d.num_of("LengthOffset")),
                    cen.and_then(|d| d.num_of("WidthOffset")),
                    cen.and_then(|d| d.num_of("HeightOffset")),
                ],
                measurement_type: s.child("Measurement").and_then(|m| m.text_of("Type")),
            };
        }
        for ds in root.all("DetectionSource") {
            out.sources.push(read_source(ds, &mut out.warnings));
        }
        for c in &root.children {
            if !KNOWN_TOP.contains(&c.name.as_str()) {
                out.extras
                    .push((format!("/SensXML/{}", c.name), c.text.trim().to_string()));
            }
        }
        if let (Some(q), n) = (out.sensor.quantity, out.sources.len()) {
            if q as usize != n {
                out.warnings
                    .push(format!("Quantity says {q} detection sources, file has {n}"));
            }
        }
        Ok(out)
    }

    /// The detection source for a motion type.
    pub fn source(&self, motion: Motion) -> Option<&DetectionSource> {
        self.sources.iter().find(|s| s.motion() == Some(motion))
    }

    /// Mounting heights with data for a motion type, ascending.
    pub fn heights(&self, motion: Motion) -> Vec<f64> {
        let mut h: Vec<f64> = self
            .source(motion)
            .map(|s| s.samples.iter().map(Sample::mounting_height).collect())
            .unwrap_or_default();
        h.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        h.dedup();
        h
    }

    /// Detection volume of one motion type rebuilt from the footprints, so
    /// the angular diagrams (polar, cartesian, heatmap, 3D) apply to SensXML
    /// as they do to a sensor LDT.
    ///
    /// A footprint at mounting height `H` is the volume cut at depth `H`, so
    /// the measured heights are horizontal slices of one volume. Between them
    /// the footprints are interpolated linearly per azimuth, above the lowest
    /// the volume narrows to the sensor (radius ∝ depth), and it ends at the
    /// deepest measured height (nothing is known below). Cutting the result
    /// at a measured height gives that footprint back. The returned LDT has
    /// C-planes every 5°, γ every 1°, and parameter block "scale 1" (each
    /// motion type is its own volume).
    pub fn to_detection_volume(
        &self,
        motion: Motion,
        use_planning: bool,
    ) -> Option<DetectionVolume> {
        let deepest = self.heights(motion).last().copied()?;
        const LEVELS: usize = 400;
        // reach[k][c]: footprint radius at depth z_k, azimuth c (1° steps)
        let depths: Vec<f64> = (1..=LEVELS)
            .map(|k| deepest * k as f64 / LEVELS as f64)
            .collect();
        let reach: Vec<Vec<f64>> = depths
            .iter()
            .map(|&z| {
                self.footprint(motion, z, use_planning)
                    .map(|(fp, _)| fp.reach)
                    .unwrap_or_else(|| vec![0.0; 360])
            })
            .collect();
        let c_angles: Vec<f64> = (0..72).map(|i| i as f64 * 5.0).collect();
        let g_angles: Vec<f64> = (0..=90).map(|g| g as f64).collect();
        let intensities: Vec<Vec<f64>> = c_angles
            .iter()
            .map(|&c| {
                let ci = (c.round() as usize) % 360;
                g_angles
                    .iter()
                    .map(|&g| {
                        let t = g.to_radians().tan();
                        // farthest depth whose slice still contains the ray
                        let z = depths
                            .iter()
                            .zip(&reach)
                            .filter(|(z, r)| r[ci] > 0.0 && **z * t <= r[ci] + 1e-9)
                            .map(|(z, _)| *z)
                            .fold(0.0, f64::max);
                        if z > 0.0 {
                            z / g.to_radians().cos()
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect();
        let mut ldt = crate::Eulumdat::new();
        ldt.identification = self.header.manufacturer.clone().unwrap_or_default();
        ldt.luminaire_name = format!(
            "{} ({} volume derived from SensXML)",
            self.header.product_name.clone().unwrap_or_default(),
            motion.label()
        );
        ldt.luminaire_number = self.header.product_number.clone().unwrap_or_default();
        ldt.symmetry = crate::Symmetry::None;
        ldt.num_c_planes = c_angles.len();
        ldt.c_plane_distance = 5.0;
        ldt.num_g_planes = g_angles.len();
        ldt.g_plane_distance = 1.0;
        ldt.c_angles = c_angles;
        ldt.g_angles = g_angles;
        ldt.intensities = intensities;
        let mounting = match self.sensor.mounting_type {
            Some(1) => 1.0,
            Some(3) => 3.0,
            _ => 2.0,
        };
        ldt.direct_ratios = [1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, mounting, 0.0, 0.0];
        Some(DetectionVolume::new(ldt))
    }

    /// Footprint on the floor for a motion type at a mounting height.
    /// `use_planning` picks the idealised planning shape instead of the
    /// measured data (falls back to whichever exists).
    pub fn footprint(
        &self,
        motion: Motion,
        mounting_height: f64,
        use_planning: bool,
    ) -> Option<(Footprint, Quality)> {
        let src = self.source(motion)?;
        let mut profiles: Vec<(f64, Vec<f64>)> = src
            .samples
            .iter()
            .filter_map(|s| {
                s.radius_profile(use_planning, src.rotation)
                    .map(|p| (s.mounting_height(), p))
            })
            .collect();
        if profiles.is_empty() {
            return None;
        }
        profiles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
        let h = mounting_height;
        let (lo, hi) = (profiles.first().unwrap(), profiles.last().unwrap());
        let (reach, quality): (Vec<f64>, Quality) =
            if let Some(p) = profiles.iter().find(|p| (p.0 - h).abs() < 1e-6) {
                (p.1.clone(), Quality::Measured)
            } else if h < lo.0 {
                let k = if lo.0 > 0.0 { h / lo.0 } else { 1.0 };
                (lo.1.iter().map(|r| r * k).collect(), Quality::Extrapolated)
            } else if h > hi.0 {
                (hi.1.clone(), Quality::Extrapolated)
            } else {
                let j = profiles.iter().position(|p| p.0 > h).unwrap();
                let (a, b) = (&profiles[j - 1], &profiles[j]);
                let t = (h - a.0) / (b.0 - a.0);
                (
                    a.1.iter().zip(&b.1).map(|(x, y)| x + (y - x) * t).collect(),
                    Quality::Interpolated,
                )
            };
        Some((footprint_from_reach(reach, h), quality))
    }
}

fn footprint_from_reach(reach: Vec<f64>, depth: f64) -> Footprint {
    let n = reach.len();
    let step = 360.0 / n as f64;
    let outline = reach
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let a = (i as f64 * step).to_radians();
            (r * a.cos(), r * a.sin())
        })
        .collect();
    let s = step.to_radians().sin();
    let area_m2 = (0..n)
        .map(|i| 0.5 * reach[i] * reach[(i + 1) % n] * s)
        .sum();
    let max_range = reach.iter().copied().fold(0.0, f64::max);
    Footprint {
        depth,
        c_step_deg: step,
        reach,
        outline,
        area_m2,
        max_range,
    }
}

fn read_planning(n: &Node) -> Option<Planning> {
    let p = n.child("PlanningData")?;
    Some(Planning {
        shape: p.int_of("Shape"),
        radius: p.num_of("Radius"),
        length: p.num_of("Length"),
        width: p.num_of("Width"),
        start_angle: p.num_of("StartAngle"),
        end_angle: p.num_of("EndAngle"),
        detection_rate: p.num_of("DetectionRate"),
        polygon: p
            .child("Polygon")
            .map(|pg| {
                pg.all("Vertex")
                    .filter_map(|v| {
                        let mut it = v.text.split([',', ';', ' ']).filter(|s| !s.is_empty());
                        Some((num(it.next()?)?, num(it.next()?)?))
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn read_source(ds: &Node, warnings: &mut Vec<String>) -> DetectionSource {
    let mut samples = Vec::new();
    for pd in ds.all("PolarData") {
        let Some(h) = pd.num_of("DistanceFromCenter") else {
            warnings.push("PolarData without DistanceFromCenter skipped".into());
            continue;
        };
        let ranges: Vec<(f64, f64)> = pd
            .all("RangeData")
            .filter_map(|r| Some((num(r.attr("angle")?)?, num(&r.text)?)))
            .collect();
        if let Some(nv) = pd.int_of("NumberValues") {
            if nv as usize != ranges.len() {
                warnings.push(format!(
                    "PolarData at {h} m: NumberValues {nv}, found {}",
                    ranges.len()
                ));
            }
        }
        samples.push(Sample::Polar {
            mounting_height: h,
            ranges,
            planning: read_planning(pd),
        });
    }
    for gd in ds.all("GridData") {
        let Some(h) = gd.num_of("DistanceFromCenter") else {
            warnings.push("GridData without DistanceFromCenter skipped".into());
            continue;
        };
        let cell = (
            gd.num_of("DimCellX").unwrap_or(0.5),
            gd.num_of("DimCellY").unwrap_or(0.5),
        );
        let cells: Vec<(f64, f64)> = gd
            .all("CellData")
            .filter(|c| num(&c.text).is_some_and(|v| v > 0.0))
            .filter_map(|c| Some((num(c.attr("x")?)?, num(c.attr("y")?)?)))
            .collect();
        samples.push(Sample::Grid {
            mounting_height: h,
            cell,
            cells,
            planning: read_planning(gd),
        });
    }
    DetectionSource {
        detection_type: ds.int_of("DetectionType"),
        rotation: ds.num_of("RotationAngle").unwrap_or(0.0),
        show_raw_data: ds.int_of("ShowRawData").map(|v| v != 0),
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic file in the observed shape: Latin-1 umlaut, padded
    /// attributes, a gap in the polar data, a count mismatch, comments,
    /// a presence grid and a planning-only rectangle.
    fn sample() -> Vec<u8> {
        let mut s = String::from(
            r#"<?xml version="1.0" encoding="iso-8859-1" standalone="yes"?>
<SensXML xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="./sensXML.xsd">
<Version>4.5</Version>  <!-- version of the sensxml format -->
<Header><Manufacturer>Test GmbH</Manufacturer><ProductName>Pr"#,
        );
        s.push('\u{E4}'); // 'ä' as one Latin-1 byte below
        s.push_str(
            r#"senz 360</ProductName><Laboratory>Lab</Laboratory></Header>
<Sensor><TotalSystemPower>0.4</TotalSystemPower><MountingType>2</MountingType><Quantity>4</Quantity>
<Measurement><Type>sensNorm</Type></Measurement></Sensor>
<DetectionSource n="1"><DetectionType>1</DetectionType><RotationAngle>0</RotationAngle>
<PolarData n="1"><NumberValues>36</NumberValues><AngularIncrement>10</AngularIncrement><DistanceFromCenter>2.5</DistanceFromCenter>
"#,
        );
        for a in (0..360).step_by(10) {
            if (60..120).contains(&a) {
                continue; // gap
            }
            s.push_str(&format!("<RangeData angle=\"{a:>7.3}\">8,0</RangeData>\n"));
        }
        s.push_str(
            r#"<PlanningData><Shape>2</Shape><Radius>7</Radius><StartAngle>0</StartAngle><EndAngle>360</EndAngle></PlanningData>
</PolarData>
<PolarData n="2"><DistanceFromCenter>4</DistanceFromCenter>
"#,
        );
        for a in (0..360).step_by(10) {
            s.push_str(&format!("<RangeData angle=\"{a}\">12</RangeData>\n"));
        }
        s.push_str(
            r#"</PolarData></DetectionSource>
<DetectionSource n="2"><DetectionType>3</DetectionType>
<GridData n="1"><NumCellsX>3</NumCellsX><NumCellsY>1</NumCellsY><DimCellX>1</DimCellX><DimCellY>1</DimCellY><DistanceFromCenter>2.5</DistanceFromCenter>
<CellData x="-1" y="0">1</CellData><CellData x=" 0.000" y="0">1</CellData><CellData x="1" y="0">1</CellData><CellData x="2" y="0">0</CellData>
</GridData></DetectionSource>
<DetectionSource n="3"><DetectionType>2</DetectionType>
<GridData n="1"><NumCellsX>1</NumCellsX><NumCellsY>1</NumCellsY><DistanceFromCenter>3</DistanceFromCenter>
<CellData x="0" y="0">0</CellData> <!-- dummy cell data -->
<PlanningData><Shape>1</Shape><Length>6</Length><Width>2</Width><Radius>0</Radius></PlanningData>
</GridData></DetectionSource>
<Unknown>kept</Unknown>
</SensXML>"#,
        );
        // encode as Latin-1
        s.chars().map(|c| c as u32 as u8).collect()
    }

    #[test]
    fn parses_observed_shape() {
        let x = SensXml::parse(&sample()).unwrap();
        assert_eq!(x.version.as_deref(), Some("4.5"));
        assert_eq!(x.header.product_name.as_deref(), Some("Präsenz 360"));
        assert_eq!(x.sensor.mounting_type, Some(2));
        assert_eq!(x.sources.len(), 3);
        assert_eq!(x.sources[0].motion(), Some(Motion::Tangential));
        assert_eq!(x.heights(Motion::Tangential), vec![2.5, 4.0]);
        assert_eq!(x.extras, vec![("/SensXML/Unknown".into(), "kept".into())]);
        assert!(x.warnings.iter().any(|w| w.contains("NumberValues 36")));
        assert!(x.warnings.iter().any(|w| w.contains("Quantity says 4")));
    }

    #[test]
    fn measured_interpolated_extrapolated() {
        let x = SensXml::parse(&sample()).unwrap();
        let (fp, q) = x.footprint(Motion::Tangential, 2.5, false).unwrap();
        assert_eq!(q, Quality::Measured);
        assert!(
            (fp.reach[90] - 8.0).abs() < 1e-9,
            "gap filled: {}",
            fp.reach[90]
        );
        let (fp, q) = x.footprint(Motion::Tangential, 3.25, false).unwrap();
        assert_eq!(q, Quality::Interpolated);
        assert!((fp.max_range - 10.0).abs() < 1e-9);
        let (fp, q) = x.footprint(Motion::Tangential, 2.0, false).unwrap();
        assert_eq!(q, Quality::Extrapolated);
        assert!((fp.max_range - 6.4).abs() < 1e-9);
        // planning circle
        let (fp, _) = x.footprint(Motion::Tangential, 2.5, true).unwrap();
        assert!((fp.max_range - 7.0).abs() < 1e-9);
    }

    #[test]
    fn grid_and_planning_rectangle() {
        let x = SensXml::parse(&sample()).unwrap();
        let (fp, _) = x.footprint(Motion::Presence, 2.5, false).unwrap();
        // farthest cell centre 1 m + half the 1 m cell along the ray
        assert!((fp.max_range - 1.5).abs() < 1e-9, "{}", fp.max_range);
        // radial: only a planning rectangle 6 × 2 m
        let (fp, _) = x.footprint(Motion::Radial, 3.0, false).unwrap();
        assert!((fp.reach[0] - 3.0).abs() < 1e-9);
        assert!((fp.reach[90] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_planning() {
        let p = Planning {
            shape: Some(3),
            polygon: vec![(2.0, 0.0), (0.0, 2.0), (-2.0, 0.0), (0.0, -2.0)],
            ..Default::default()
        };
        assert!((p.radius_at(0.0).unwrap() - 2.0).abs() < 1e-9);
        assert!((p.radius_at(45.0).unwrap() - 2f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn derived_volume_gives_the_footprints_back() {
        let x = SensXml::parse(&sample()).unwrap();
        let v = x.to_detection_volume(Motion::Tangential, false).unwrap();
        for h in x.heights(Motion::Tangential) {
            let (want, _) = x.footprint(Motion::Tangential, h, false).unwrap();
            let got = v.footprint(h, 5.0);
            for (i, r) in got.reach.iter().enumerate() {
                let w = want.reach[i * 5];
                assert!(
                    (r - w).abs() < 0.05 * w.max(1.0),
                    "h {h} c {} got {r} want {w}",
                    i * 5
                );
            }
        }
        // between the heights it follows the interpolated footprint
        let (mid, _) = x.footprint(Motion::Tangential, 3.25, false).unwrap();
        assert!((v.footprint(3.25, 5.0).max_range - mid.max_range).abs() < 0.1);
        // nothing below the deepest measured height
        assert!(v.footprint(4.2, 5.0).is_empty());
    }

    #[test]
    fn rejects_other_roots() {
        assert!(SensXml::parse(b"<Luminaire/>").is_err());
    }

    /// Real files are not in the repo (no licence). Point `SENSXML_CORPUS` at
    /// a folder to check every `.xml` below it.
    #[test]
    fn corpus() {
        let Ok(dir) = std::env::var("SENSXML_CORPUS") else {
            return;
        };
        let mut files = Vec::new();
        fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if let Ok(rd) = std::fs::read_dir(p) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        walk(&p, out);
                    } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) {
                        out.push(p);
                    }
                }
            }
        }
        walk(std::path::Path::new(&dir), &mut files);
        let mut ok = 0;
        for f in &files {
            let bytes = std::fs::read(f).unwrap();
            let Ok(x) = SensXml::parse(&bytes) else {
                continue; // not a SensXML file
            };
            assert!(
                !x.sources.is_empty(),
                "{}: no detection source",
                f.display()
            );
            for m in Motion::ALL {
                for h in x.heights(m) {
                    assert!((1.0..=20.0).contains(&h), "{}: height {h}", f.display());
                    if let Some((fp, _)) = x.footprint(m, h, false) {
                        assert!(fp.max_range < 60.0, "{}: {}", f.display(), fp.max_range);
                    }
                }
            }
            ok += 1;
        }
        assert!(ok > 0, "no SensXML files found under {dir}");
        eprintln!("SensXML corpus: {ok} files parsed");
    }
}
