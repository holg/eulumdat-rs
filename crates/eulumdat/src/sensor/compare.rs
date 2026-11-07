//! Two sensors under the same conditions: the same mounting height, the
//! same detection planes (walking on the floor, seated presence on the
//! workplane), the same C-plane. Sensor LDT and SensXML can be mixed: both
//! go through [`SensorFile::area_at`], exactly as in the room planner.
//!
//! Pieces: [`SensorComparison`] (per-motion rows with Δ %), plan / polar /
//! cartesian overlays, the range over the mounting height
//! ([`height_sweep`], [`height_chart_svg`]) and the room check
//! ([`room_check`]: sensors needed for one room).

use super::coverage::{
    auto_grid_oriented, coverage_with, grid_positions, rotated, CoverageRoom, PlaneAreas,
};
use super::en15193::{assess, FoAssessment, FoInput, OccupancyControl, SpaceType, MIN_COVERAGE};
use super::{diagrams, Motion, Quality, SensorFile, SensorPlanSvg, SensorPlanTrace, TRACE_COLORS};
use crate::compare::Significance;
use crate::diagram::{CartesianDiagram, PolarDiagram, SvgTheme};
use core::fmt::Write;

/// The conditions both sensors are compared under.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompareConditions {
    /// Height of the sensors above the floor (m).
    pub mount: f64,
    /// Plane for walking (tangential / radial), usually the floor (m).
    pub walk_plane: f64,
    /// Plane for seated presence, usually the workplane (m).
    pub presence_plane: f64,
    /// SensXML: idealised planning shapes instead of the measured areas.
    pub planning: bool,
}

impl Default for CompareConditions {
    fn default() -> Self {
        Self {
            mount: 3.0,
            walk_plane: 0.0,
            presence_plane: 0.8,
            planning: false,
        }
    }
}

impl CompareConditions {
    /// Detection plane of a motion type.
    pub fn plane(&self, motion: Motion) -> f64 {
        match motion {
            Motion::Presence => self.presence_plane,
            Motion::Tangential | Motion::Radial => self.walk_plane,
        }
    }
}

/// One sensor's detection area of one motion type under the conditions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SideValues {
    /// Something is detected on the plane.
    pub detected: bool,
    pub max_range: f64,
    pub area_m2: f64,
    pub diameter: f64,
    /// SensXML: measured / interpolated / extrapolated.
    pub quality: Option<Quality>,
    /// Sensor LDT: depth below the sensor where the volume ends (m).
    pub depth_limit: Option<f64>,
}

/// One motion type, both sensors (`None`: the file has no data for it).
#[derive(Debug, Clone, PartialEq)]
pub struct MotionRow {
    pub motion: Motion,
    /// Detection plane above the floor (m).
    pub plane: f64,
    pub a: Option<SideValues>,
    pub b: Option<SideValues>,
}

impl MotionRow {
    /// (B − A) / A in %, when both detect.
    pub fn delta_pct(&self, value: impl Fn(&SideValues) -> f64) -> Option<f64> {
        let (a, b) = (self.a?, self.b?);
        (a.detected && b.detected && value(&a) > 0.0)
            .then(|| (value(&b) - value(&a)) / value(&a) * 100.0)
    }

    /// Significance of the area difference (the luminaire comparison's
    /// thresholds: < 5, 15, 30 %).
    pub fn area_significance(&self) -> Option<Significance> {
        self.delta_pct(|s| s.area_m2)
            .map(Significance::from_delta_percent)
    }
}

/// Two sensors side by side.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorComparison {
    pub conditions: CompareConditions,
    pub label_a: String,
    pub label_b: String,
    /// Motion types either file has, in [`Motion::ALL`] order.
    pub rows: Vec<MotionRow>,
}

fn side(file: &SensorFile, motion: Motion, cond: &CompareConditions) -> Option<SideValues> {
    if !file.motions().contains(&motion) {
        return None;
    }
    let area = file.area_at(motion, cond.mount, cond.plane(motion), cond.planning);
    Some(match area {
        Some(a) => SideValues {
            detected: a.detected(),
            max_range: a.max_range(),
            area_m2: a.area_m2(),
            diameter: a.diameter(),
            quality: a.quality,
            depth_limit: a.depth_limit,
        },
        None => SideValues {
            detected: false,
            max_range: 0.0,
            area_m2: 0.0,
            diameter: 0.0,
            quality: None,
            depth_limit: None,
        },
    })
}

impl SensorComparison {
    pub fn new(
        a: &SensorFile,
        b: &SensorFile,
        conditions: &CompareConditions,
        label_a: &str,
        label_b: &str,
    ) -> Self {
        let rows = Motion::ALL
            .into_iter()
            .filter_map(|m| {
                let (sa, sb) = (side(a, m, conditions), side(b, m, conditions));
                (sa.is_some() || sb.is_some()).then_some(MotionRow {
                    motion: m,
                    plane: conditions.plane(m),
                    a: sa,
                    b: sb,
                })
            })
            .collect();
        Self {
            conditions: *conditions,
            label_a: label_a.to_string(),
            label_b: label_b.to_string(),
            rows,
        }
    }

    /// Plain-text table (CLI).
    pub fn to_text(&self) -> String {
        let c = &self.conditions;
        let mut s = String::new();
        let _ = writeln!(s, "A: {}", self.label_a);
        let _ = writeln!(s, "B: {}", self.label_b);
        let _ = writeln!(
            s,
            "Mounting height {:.2} m; walking on {:.2} m, presence on {:.2} m{}",
            c.mount,
            c.walk_plane,
            c.presence_plane,
            if c.planning {
                "; SensXML planning shapes"
            } else {
                ""
            }
        );
        let _ = writeln!(
            s,
            "{:<20} {:>10} {:>10} {:>8}   {:>10} {:>10} {:>8}",
            "motion", "A range", "B range", "Δ %", "A area", "B area", "Δ %"
        );
        let cell = |v: Option<SideValues>, f: fn(&SideValues) -> f64, unit: &str| match v {
            Some(x) if x.detected => format!("{:.2} {unit}", f(&x)),
            Some(_) => "not det.".to_string(),
            None => "–".to_string(),
        };
        let pct = |d: Option<f64>| d.map(|d| format!("{d:+.0}")).unwrap_or_else(|| "–".into());
        for r in &self.rows {
            let _ = writeln!(
                s,
                "{:<20} {:>10} {:>10} {:>8}   {:>10} {:>10} {:>8}",
                r.motion.label(),
                cell(r.a, |x| x.max_range, "m"),
                cell(r.b, |x| x.max_range, "m"),
                pct(r.delta_pct(|x| x.max_range)),
                cell(r.a, |x| x.area_m2, "m²"),
                cell(r.b, |x| x.area_m2, "m²"),
                pct(r.delta_pct(|x| x.area_m2)),
            );
        }
        s
    }
}

impl SensorFile {
    /// Detection areas of one motion type for many mounting heights on one
    /// plane: [`SensorFile::area_at`] per height, but a sensor LDT's volume
    /// is sampled only once ([`super::DetectionVolume::depth_profile`]).
    pub fn area_sweep(
        &self,
        motion: Motion,
        mounts: &[f64],
        plane: f64,
        planning: bool,
    ) -> Vec<Option<super::DetectionArea>> {
        match self {
            SensorFile::Ldt(v) => {
                let Some(k) = v.params().scale(motion) else {
                    return vec![None; mounts.len()];
                };
                let profile = v.depth_profile(1.0, k);
                let depth_limit = v.motion_depth_limit(motion);
                mounts
                    .iter()
                    .map(|&h| {
                        let depth = h - plane;
                        (depth > 0.0).then(|| super::DetectionArea {
                            motion,
                            footprint: profile.footprint(depth),
                            quality: None,
                            depth_limit,
                        })
                    })
                    .collect()
            }
            #[cfg(feature = "xml")]
            SensorFile::Xml(_) => mounts
                .iter()
                .map(|&h| self.area_at(motion, h, plane, planning))
                .collect(),
        }
    }

    /// The detection volume of one motion type: a sensor LDT's volume
    /// scaled by the motion's factor, or the volume rebuilt from SensXML's
    /// measured heights. For the angular views of a comparison.
    pub fn motion_volume(&self, motion: Motion, planning: bool) -> Option<super::DetectionVolume> {
        match self {
            SensorFile::Ldt(v) => {
                // the volume of a sensor LDT has no planning variant
                let _ = planning;
                let k = v.params().scale(motion)?;
                let mut ldt = v.ldt().clone();
                for row in &mut ldt.intensities {
                    for x in row {
                        *x *= k;
                    }
                }
                Some(super::DetectionVolume::new(ldt))
            }
            #[cfg(feature = "xml")]
            SensorFile::Xml(x) => x.to_detection_volume(motion, planning),
        }
    }
}

/// Plan view of both sensors: A filled, B dashed, one colour per motion
/// type, optional C-plane cut with the reach of both marked.
#[allow(clippy::too_many_arguments)]
pub fn plan_overlay_svg(
    a: &SensorFile,
    b: &SensorFile,
    cond: &CompareConditions,
    motions: &[Motion],
    size: f64,
    theme: &SvgTheme,
    label: &dyn Fn(Motion) -> String,
    c_plane: Option<f64>,
) -> String {
    let mut traces = Vec::new();
    let mut dashed = Vec::new();
    for (file, tag, dash) in [(a, "A", false), (b, "B", true)] {
        for (i, m) in Motion::ALL.iter().enumerate() {
            if !motions.contains(m) {
                continue;
            }
            let Some(area) = file.area_at(*m, cond.mount, cond.plane(*m), cond.planning) else {
                continue;
            };
            traces.push(SensorPlanTrace {
                label: format!("{tag} · {}", label(*m)),
                color: TRACE_COLORS[i].to_string(),
                footprint: area.footprint,
            });
            dashed.push(dash);
        }
    }
    let theme = diagrams::sensor_theme(theme);
    SensorPlanSvg::render_styled(&traces, &dashed, size, &theme, c_plane)
}

/// Polar overlay of one motion type's volumes (ranges in m) at a C-plane.
#[allow(clippy::too_many_arguments)]
pub fn polar_overlay_svg(
    a: &SensorFile,
    b: &SensorFile,
    motion: Motion,
    planning: bool,
    c_plane: f64,
    size: f64,
    theme: &SvgTheme,
    label_a: &str,
    label_b: &str,
) -> Option<String> {
    let va = a.motion_volume(motion, planning)?;
    let vb = b.motion_volume(motion, planning)?;
    let pa = PolarDiagram::from_eulumdat_for_plane(va.ldt(), c_plane);
    let pb = PolarDiagram::from_eulumdat_for_plane(vb.ldt(), c_plane);
    let theme = diagrams::sensor_theme(theme);
    Some(PolarDiagram::to_overlay_svg(
        &pa, &pb, size, size, &theme, label_a, label_b,
    ))
}

/// Cartesian overlay of one motion type's volumes (ranges in m) at a C-plane.
#[allow(clippy::too_many_arguments)]
pub fn cartesian_overlay_svg(
    a: &SensorFile,
    b: &SensorFile,
    motion: Motion,
    planning: bool,
    c_plane: f64,
    width: f64,
    height: f64,
    theme: &SvgTheme,
    label_a: &str,
    label_b: &str,
) -> Option<String> {
    let va = a.motion_volume(motion, planning)?;
    let vb = b.motion_volume(motion, planning)?;
    let ca = CartesianDiagram::from_eulumdat_for_plane(va.ldt(), c_plane, width, height);
    let cb = CartesianDiagram::from_eulumdat_for_plane(vb.ldt(), c_plane, width, height);
    let theme = diagrams::sensor_theme(theme);
    Some(CartesianDiagram::to_overlay_svg(
        &ca, &cb, width, height, &theme, label_a, label_b,
    ))
}

/// One point of the range over the mounting height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeightPoint {
    /// Mounting height (m).
    pub height: f64,
    /// Equivalent diameter of the detection area (m), 0 = not detected.
    pub diameter: f64,
    pub max_range: f64,
    pub quality: Option<Quality>,
}

/// Detection area of one motion type for each mounting height (planes as
/// in `cond`, its `mount` is ignored). `None` if the file has no data for
/// the motion type.
pub fn height_sweep(
    file: &SensorFile,
    motion: Motion,
    cond: &CompareConditions,
    heights: &[f64],
) -> Option<Vec<HeightPoint>> {
    if !file.motions().contains(&motion) {
        return None;
    }
    let areas = file.area_sweep(motion, heights, cond.plane(motion), cond.planning);
    Some(
        heights
            .iter()
            .zip(areas)
            .map(|(&h, area)| {
                let area = area.filter(|a| a.detected());
                HeightPoint {
                    height: h,
                    diameter: area.as_ref().map(|a| a.diameter()).unwrap_or(0.0),
                    max_range: area.as_ref().map(|a| a.max_range()).unwrap_or(0.0),
                    quality: area.and_then(|a| a.quality),
                }
            })
            .collect(),
    )
}

/// Axis and legend texts of [`height_chart_svg`].
#[derive(Debug, Clone)]
pub struct HeightChartLabels {
    pub x_axis: String,
    pub y_axis: String,
    pub measured: String,
}

impl Default for HeightChartLabels {
    fn default() -> Self {
        Self {
            x_axis: "Mounting height (m)".into(),
            y_axis: "Ø detection area (m)".into(),
            measured: "measured height".into(),
        }
    }
}

fn nice_step(max: f64) -> f64 {
    let raw = (max / 5.0).max(1e-9);
    let p = 10f64.powf(raw.log10().floor());
    let m = raw / p;
    let n = if m <= 1.0 {
        1.0
    } else if m <= 2.0 {
        2.0
    } else if m <= 5.0 {
        5.0
    } else {
        10.0
    };
    n * p
}

/// Ø of the detection area over the mounting height `h_min`…`h_max` for A
/// (solid) and B (dashed), one colour per motion type; measured heights of a
/// SensXML file are marked, a gap means "not detected at this height".
#[allow(clippy::too_many_arguments)]
pub fn height_chart_svg(
    a: &SensorFile,
    b: &SensorFile,
    cond: &CompareConditions,
    motions: &[Motion],
    (h_min, h_max): (f64, f64),
    width: f64,
    height: f64,
    theme: &SvgTheme,
    labels: &HeightChartLabels,
    motion_label: &dyn Fn(Motion) -> String,
    label_a: &str,
    label_b: &str,
) -> String {
    let n = 120usize;
    let heights: Vec<f64> = (0..=n)
        .map(|i| h_min + (h_max - h_min) * i as f64 / n as f64)
        .collect();
    let mut series = Vec::new(); // (motion index, dashed, points, measured heights)
    for (file, dash) in [(a, false), (b, true)] {
        for (i, m) in Motion::ALL.iter().enumerate() {
            if !motions.contains(m) {
                continue;
            }
            if let Some(pts) = height_sweep(file, *m, cond, &heights) {
                let measured: Vec<f64> = file
                    .measured_heights()
                    .into_iter()
                    .filter(|h| *h >= h_min && *h <= h_max)
                    .collect();
                series.push((i, dash, pts, measured));
            }
        }
    }
    let y_max = series
        .iter()
        .flat_map(|s| s.2.iter().map(|p| p.diameter))
        .fold(1.0f64, f64::max);
    let y_step = nice_step(y_max);
    let y_top = (y_max / y_step).ceil() * y_step;
    let x_step = nice_step(h_max - h_min).max(1.0);
    let (ml, mr, mt, mb) = (56.0, 16.0, 16.0, 46.0 + 18.0 * 2.0);
    let (pw, ph) = (width - ml - mr, height - mt - mb);
    let px = |h: f64| ml + (h - h_min) / (h_max - h_min).max(1e-9) * pw;
    let py = |d: f64| mt + ph - d / y_top.max(1e-9) * ph;

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="{}"><rect width="{width}" height="{height}" fill="{}"/>"#,
        theme.font_family, theme.background
    );
    // grid + ticks
    let mut y = 0.0;
    while y <= y_top + 1e-9 {
        let _ = write!(
            s,
            r#"<line x1="{ml}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="0.7"/><text x="{:.1}" y="{:.1}" font-size="10" fill="{}" text-anchor="end">{}</text>"#,
            py(y),
            ml + pw,
            py(y),
            theme.grid,
            ml - 6.0,
            py(y) + 3.5,
            theme.text_secondary,
            (y * 100.0).round() / 100.0
        );
        y += y_step;
    }
    let mut x = (h_min / x_step).ceil() * x_step;
    while x <= h_max + 1e-9 {
        let _ = write!(
            s,
            r#"<line x1="{:.1}" y1="{mt}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="0.7"/><text x="{:.1}" y="{:.1}" font-size="10" fill="{}" text-anchor="middle">{}</text>"#,
            px(x),
            px(x),
            mt + ph,
            theme.grid,
            px(x),
            mt + ph + 14.0,
            theme.text_secondary,
            (x * 100.0).round() / 100.0
        );
        x += x_step;
    }
    let _ = write!(
        s,
        r#"<rect x="{ml}" y="{mt}" width="{pw:.1}" height="{ph:.1}" fill="none" stroke="{}"/><text x="{:.1}" y="{:.1}" font-size="11" fill="{}" text-anchor="middle">{}</text><text transform="translate(14,{:.1}) rotate(-90)" font-size="11" fill="{}" text-anchor="middle">{}</text>"#,
        theme.axis,
        ml + pw / 2.0,
        mt + ph + 30.0,
        theme.text,
        esc(&labels.x_axis),
        mt + ph / 2.0,
        theme.text,
        esc(&labels.y_axis)
    );
    // curves: a new path segment after every gap (not detected)
    for (i, dash, pts, measured) in &series {
        let color = TRACE_COLORS[*i];
        let mut d = String::new();
        let mut pen = false;
        for p in pts {
            if p.diameter > 0.0 {
                let _ = write!(
                    d,
                    "{}{:.1},{:.1}",
                    if pen { "L" } else { "M" },
                    px(p.height),
                    py(p.diameter)
                );
                pen = true;
            } else {
                pen = false;
            }
        }
        let dash_attr = if *dash {
            r#" stroke-dasharray="7,4""#
        } else {
            ""
        };
        let _ = write!(
            s,
            r#"<path d="{d}" fill="none" stroke="{color}" stroke-width="2"{dash_attr}/>"#
        );
        for h in measured {
            if let Some(p) = pts
                .iter()
                .min_by(|a, b| (a.height - h).abs().total_cmp(&(b.height - h).abs()))
                .filter(|p| p.diameter > 0.0)
            {
                let (fill, stroke) = if *dash {
                    (theme.background.as_str(), color)
                } else {
                    (color, theme.background.as_str())
                };
                let _ = write!(
                    s,
                    r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{fill}" stroke="{stroke}" stroke-width="1.5"><title>{}</title></circle>"#,
                    px(*h),
                    py(p.diameter),
                    esc(&labels.measured)
                );
            }
        }
    }
    // legend: line style per sensor, colour per motion type
    let ly = height - 34.0;
    let _ = write!(
        s,
        r#"<line x1="{ml}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="2"/><text x="{:.1}" y="{:.1}" font-size="11" fill="{}">A · {}</text>"#,
        ly,
        ml + 22.0,
        ly,
        theme.text,
        ml + 28.0,
        ly + 4.0,
        theme.text,
        esc(label_a)
    );
    let _ = write!(
        s,
        r#"<line x1="{ml}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="2" stroke-dasharray="7,4"/><text x="{:.1}" y="{:.1}" font-size="11" fill="{}">B · {}</text>"#,
        ly + 16.0,
        ml + 22.0,
        ly + 16.0,
        theme.text,
        ml + 28.0,
        ly + 20.0,
        theme.text,
        esc(label_b)
    );
    let mut lx = ml + pw * 0.62;
    for (i, m) in Motion::ALL.iter().enumerate() {
        if !motions.contains(m) {
            continue;
        }
        let _ = write!(
            s,
            r#"<rect x="{lx:.1}" y="{:.1}" width="10" height="10" fill="{}"/><text x="{:.1}" y="{:.1}" font-size="10" fill="{}">{}</text>"#,
            ly - 4.0 + 16.0 * (i as f64 / 2.0).floor(),
            TRACE_COLORS[i],
            lx + 14.0,
            ly + 5.0 + 16.0 * (i as f64 / 2.0).floor(),
            theme.text,
            esc(&motion_label(*m))
        );
        if i % 2 == 0 {
            lx += pw * 0.2;
        } else {
            lx = ml + pw * 0.62;
        }
    }
    s.push_str("</svg>");
    s
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// What one sensor needs in one room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomCheck {
    pub nx: usize,
    pub ny: usize,
    /// Sensor rotation (C0 from the room length, degrees).
    pub rotation: f64,
    /// The automatic grid reached the target.
    pub reached: bool,
    pub count: usize,
    pub covered: f64,
    pub overlap: f64,
    pub m2_per_sensor: f64,
    /// EN 15193-1 assessment for the control / space given.
    pub fo: Option<FoAssessment>,
}

/// Sensors one file needs for `room` (automatic grid, ≥ 95 % of the
/// evaluated area for `motion`, both orientations, at most `max_count`).
/// `None` if the motion type is not detected at the room's mounting height.
pub fn room_check(
    file: &SensorFile,
    room: &CoverageRoom,
    planning: bool,
    motion: Motion,
    max_count: usize,
    control: Option<(OccupancyControl, SpaceType, bool)>,
) -> Option<RoomCheck> {
    let areas = PlaneAreas::for_motions(file, room, planning, &[motion]);
    areas.get(motion)?;
    let (nx, ny, rotation, reached) =
        match auto_grid_oriented(&areas, room, motion, MIN_COVERAGE, max_count, &[0.0, 90.0]) {
            Some((nx, ny, r)) => (nx, ny, r, true),
            None => {
                let (nx, ny) = super::coverage::square_grid(room.length, room.width, max_count);
                (nx, ny, 0.0, false)
            }
        };
    let pos = rotated(grid_positions(room.length, room.width, nx, ny), rotation);
    let cov = coverage_with(&areas, room, &pos);
    let mc = cov.get(motion)?;
    let m2 = cov.m2_per_sensor();
    let fo = control.map(|(control, space, hold)| {
        assess(&FoInput {
            control,
            space,
            hold_time_ok: hold,
            group_m2: m2,
            m2_per_sensor: m2,
            coverage: mc.covered_fraction,
        })
    });
    Some(RoomCheck {
        nx,
        ny,
        rotation,
        reached,
        count: nx * ny,
        covered: mc.covered_fraction,
        overlap: mc.overlap_fraction,
        m2_per_sensor: m2,
        fo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::coverage::DEFAULT_WALL_MARGIN;

    fn ldt(s: &str) -> SensorFile {
        SensorFile::from_bytes(s.as_bytes(), "sensor/sensldt").unwrap()
    }
    fn ceiling() -> SensorFile {
        ldt(include_str!(
            "../../tests/fixtures/sensors/demo_ceiling_360.ldt"
        ))
    }
    fn corridor() -> SensorFile {
        ldt(include_str!(
            "../../tests/fixtures/sensors/demo_corridor.ldt"
        ))
    }
    #[cfg(feature = "xml")]
    fn highbay() -> SensorFile {
        SensorFile::from_bytes(
            include_bytes!("../../tests/fixtures/sensors/demo_highbay_sensxml.xml"),
            "",
        )
        .unwrap()
    }

    #[test]
    fn same_sensor_has_no_difference() {
        let c = CompareConditions::default();
        let cmp = SensorComparison::new(&ceiling(), &ceiling(), &c, "A", "B");
        assert_eq!(cmp.rows.len(), 3);
        for r in &cmp.rows {
            assert_eq!(r.delta_pct(|s| s.area_m2), Some(0.0));
            assert_eq!(r.area_significance(), Some(Significance::Negligible));
        }
        assert!(cmp.to_text().contains("Δ %"));
    }

    #[test]
    fn rows_follow_the_room_planes() {
        let c = CompareConditions::default();
        let cmp = SensorComparison::new(&ceiling(), &corridor(), &c, "ceiling", "corridor");
        let pres = cmp
            .rows
            .iter()
            .find(|r| r.motion == Motion::Presence)
            .unwrap();
        assert_eq!(pres.plane, 0.8);
        // the corridor demo has no presence volume
        assert!(pres.a.is_some() && pres.b.is_none());
        // A's presence area equals the room planner's at 3.0 − 0.8 m
        let direct = ceiling()
            .area_at(Motion::Presence, 3.0, 0.8, false)
            .unwrap()
            .area_m2();
        assert!((pres.a.unwrap().area_m2 - direct).abs() < 1e-9);
    }

    #[cfg(feature = "xml")]
    #[test]
    fn ldt_and_sensxml_mix() {
        let c = CompareConditions {
            mount: 8.0,
            ..Default::default()
        };
        let cmp = SensorComparison::new(&ceiling(), &highbay(), &c, "LDT", "SensXML");
        let t = cmp
            .rows
            .iter()
            .find(|r| r.motion == Motion::Tangential)
            .unwrap();
        let b = t.b.unwrap();
        assert!(b.detected && b.quality.is_some());
        let svg = plan_overlay_svg(
            &ceiling(),
            &highbay(),
            &c,
            &Motion::ALL,
            400.0,
            &SvgTheme::light(),
            &|m| m.label().to_string(),
            Some(45.0),
        );
        assert!(svg.contains(r#"stroke-dasharray="7,4""#) && svg.contains("B · "));
        let polar = polar_overlay_svg(
            &ceiling(),
            &highbay(),
            Motion::Tangential,
            false,
            90.0,
            400.0,
            &SvgTheme::light(),
            "A",
            "B",
        )
        .unwrap();
        assert!(polar.starts_with("<svg"));
        let chart = height_chart_svg(
            &ceiling(),
            &highbay(),
            &c,
            &Motion::ALL,
            (2.0, 15.0),
            600.0,
            360.0,
            &SvgTheme::light(),
            &HeightChartLabels::default(),
            &|m| m.label().to_string(),
            "A",
            "B",
        );
        // measured heights of the SensXML file are marked
        assert!(chart.matches("<circle").count() >= 3, "{chart}");
    }

    #[test]
    fn motion_volume_scales_the_ldt() {
        let f = ceiling();
        let t = f.motion_volume(Motion::Tangential, false).unwrap();
        let p = f.motion_volume(Motion::Presence, false).unwrap();
        let max = |v: &crate::sensor::DetectionVolume| {
            v.ldt()
                .intensities
                .iter()
                .flatten()
                .copied()
                .fold(0.0, f64::max)
        };
        assert!(max(&p) < max(&t));
    }

    #[test]
    fn area_sweep_equals_area_at() {
        let f = corridor();
        let hs = [1.0, 2.5, 3.0, 4.2];
        let sweep = f.area_sweep(Motion::Tangential, &hs, 0.0, false);
        for (h, a) in hs.iter().zip(sweep) {
            let direct = f.area_at(Motion::Tangential, *h, 0.0, false);
            assert_eq!(
                a.map(|a| a.footprint.reach),
                direct.map(|a| a.footprint.reach),
                "{h}"
            );
        }
    }

    #[test]
    fn height_sweep_ends_where_the_volume_ends() {
        let c = CompareConditions::default();
        let heights: Vec<f64> = (4..=40).map(|i| i as f64 * 0.5).collect();
        let pts = height_sweep(&ceiling(), Motion::Presence, &c, &heights).unwrap();
        assert!(pts.first().unwrap().diameter > 0.0);
        assert_eq!(pts.last().unwrap().diameter, 0.0);
        assert!(height_sweep(&corridor(), Motion::Presence, &c, &heights).is_none());
    }

    #[test]
    fn room_check_counts_sensors() {
        let room = CoverageRoom {
            length: 10.0,
            width: 8.0,
            mount_height: 3.0,
            walk_plane: 0.0,
            presence_plane: 0.8,
            margin: DEFAULT_WALL_MARGIN,
        };
        let ctl = Some((OccupancyControl::AutoOnAutoOff, SpaceType::Classroom, true));
        let r = room_check(&ceiling(), &room, false, Motion::Presence, 100, ctl).unwrap();
        // the landing page's classroom: 3 × 3, 98 %, credited
        assert_eq!((r.nx, r.ny, r.count), (3, 3, 9));
        assert!(r.reached && r.covered >= 0.95 && r.fo.unwrap().credited);
        // the corridor demo sees no seated presence at all
        assert!(room_check(&corridor(), &room, false, Motion::Presence, 100, ctl).is_none());
        let walk = room_check(&corridor(), &room, false, Motion::Tangential, 100, ctl).unwrap();
        assert!(walk.reached);
    }
}
