//! EN 1838 emergency lighting: escape routes, open (anti-panic) areas,
//! high-risk task areas, points of emphasis, disability glare and colour
//! rendering.
//!
//! EN 1838 counts direct light only (no inter-reflections), from the
//! luminaires' emergency flux at the end of the rated duration, times a
//! maintenance factor. The limit values are data
//! (`data/en1838_en17037_limits.json`, [`Limits::en1838`]); a national
//! profile can pass its own table. The illuminance kernel is the one of the
//! area / interior designers (same rotation convention: `rotation` = C0
//! direction, degrees counter-clockwise from +x).
//!
//! Plan coordinates are metres on the floor (x, y), heights above the floor.

use crate::area::{compute_illuminance_at_point, AreaPolygon, LuminairePlace};
use crate::{Eulumdat, PhotometricCalculations};

// ─── limit values ────────────────────────────────────────────────────

/// Escape routes up to 2 m wide.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteLimits {
    /// Minimum on the centre line (lx).
    pub centre_min_lx: f64,
    /// Minimum over the central band, half the route width (lx).
    pub band_min_lx: f64,
    /// Maximum Emax / Emin along the centre line.
    pub max_ratio: f64,
}

/// Open (anti-panic) areas.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AreaLimits {
    /// Minimum on the floor of the core area (lx).
    pub min_lx: f64,
    /// Border left out along the walls (m).
    pub border_m: f64,
    /// Maximum Emax / Emin.
    pub max_ratio: f64,
    /// Areas larger than this need anti-panic lighting (m²).
    pub applies_above_m2: f64,
}

/// High-risk task areas.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HighRiskLimits {
    /// Percentage of the normal maintained illuminance of the task.
    pub pct_of_normal: f64,
    /// Absolute minimum (lx).
    pub abs_min_lx: f64,
    /// Maximum Emax / Emin.
    pub max_ratio: f64,
}

/// Luminous intensity limit between 60° and 90° from the downward vertical
/// for mounting heights below `below_height_m`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlareStep {
    pub below_height_m: f64,
    /// Escape routes and open areas (cd).
    pub escape_cd: f64,
    /// High-risk task areas (cd).
    pub high_risk_cd: f64,
}

/// Response times (reported, not computed).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timing {
    /// 50 % of the illuminance within (s).
    pub half_s: f64,
    /// 100 % within (s).
    pub full_s: f64,
    /// High-risk areas: 100 % within (s).
    pub high_risk_full_s: f64,
}

/// One set of EN 1838 limit values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Limits {
    pub escape_route: RouteLimits,
    pub anti_panic: AreaLimits,
    pub high_risk: HighRiskLimits,
    /// Vertical illuminance at safety equipment and externally lit signs (lx).
    pub emphasis_min_vertical_lx: f64,
    /// Ascending by height.
    pub glare: Vec<GlareStep>,
    /// Minimum colour rendering index Ra of the emergency light.
    pub min_ra: f64,
    pub timing: Timing,
}

impl Limits {
    /// EN 1838 (the values of `data/en1838_en17037_limits.json`; a test
    /// keeps both in step).
    pub fn en1838() -> Self {
        let step = |h: f64, e: f64, r: f64| GlareStep {
            below_height_m: h,
            escape_cd: e,
            high_risk_cd: r,
        };
        Limits {
            escape_route: RouteLimits {
                centre_min_lx: 1.0,
                band_min_lx: 0.5,
                max_ratio: 40.0,
            },
            anti_panic: AreaLimits {
                min_lx: 0.5,
                border_m: 0.5,
                max_ratio: 40.0,
                applies_above_m2: 60.0,
            },
            high_risk: HighRiskLimits {
                pct_of_normal: 10.0,
                abs_min_lx: 15.0,
                max_ratio: 10.0,
            },
            emphasis_min_vertical_lx: 5.0,
            glare: vec![
                step(2.5, 500.0, 1000.0),
                step(3.0, 900.0, 1800.0),
                step(3.5, 1600.0, 3200.0),
                step(4.0, 2500.0, 5000.0),
                step(4.5, 3500.0, 7000.0),
                step(f64::INFINITY, 5000.0, 10000.0),
            ],
            min_ra: 40.0,
            timing: Timing {
                half_s: 5.0,
                full_s: 60.0,
                high_risk_full_s: 0.5,
            },
        }
    }

    /// Glare limit (cd) for a mounting height.
    pub fn glare_limit(&self, height: f64, high_risk: bool) -> f64 {
        let step = self
            .glare
            .iter()
            .find(|s| height < s.below_height_m)
            .or(self.glare.last());
        step.map_or(f64::INFINITY, |s| {
            if high_risk {
                s.high_risk_cd
            } else {
                s.escape_cd
            }
        })
    }

    /// Required minimum in a high-risk area with the given normal
    /// maintained illuminance.
    pub fn high_risk_min_lx(&self, normal_maintained_lx: f64) -> f64 {
        (normal_maintained_lx * self.high_risk.pct_of_normal / 100.0).max(self.high_risk.abs_min_lx)
    }

    /// Limits from the JSON layout of `data/en1838_en17037_limits.json`
    /// (section `EN_1838`); missing values keep the EN 1838 defaults.
    #[cfg(feature = "json")]
    pub fn from_json(text: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let en = v.get("EN_1838").ok_or("no EN_1838 section")?;
        let mut l = Limits::en1838();
        let num = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_f64());
        if let Some(r) = en.get("escape_route") {
            l.escape_route.centre_min_lx =
                num(r, "min_illuminance_center_lx").unwrap_or(l.escape_route.centre_min_lx);
            l.escape_route.band_min_lx =
                num(r, "min_illuminance_band_lx").unwrap_or(l.escape_route.band_min_lx);
            l.escape_route.max_ratio =
                num(r, "max_uniformity_ratio").unwrap_or(l.escape_route.max_ratio);
            l.timing.half_s = num(r, "response_time_50pct_sec").unwrap_or(l.timing.half_s);
            l.timing.full_s = num(r, "response_time_100pct_sec").unwrap_or(l.timing.full_s);
            l.min_ra = num(r, "min_color_rendering_Ra").unwrap_or(l.min_ra);
        }
        if let Some(a) = en.get("anti_panic") {
            l.anti_panic.min_lx = num(a, "min_illuminance_lx").unwrap_or(l.anti_panic.min_lx);
            l.anti_panic.border_m = num(a, "edge_exclusion_m").unwrap_or(l.anti_panic.border_m);
            l.anti_panic.max_ratio =
                num(a, "max_uniformity_ratio").unwrap_or(l.anti_panic.max_ratio);
        }
        if let Some(h) = en.get("high_risk_task") {
            l.high_risk.pct_of_normal =
                num(h, "min_illuminance_pct_of_normal").unwrap_or(l.high_risk.pct_of_normal);
            l.high_risk.abs_min_lx =
                num(h, "absolute_min_illuminance_lx").unwrap_or(l.high_risk.abs_min_lx);
            l.high_risk.max_ratio = num(h, "max_uniformity_ratio").unwrap_or(l.high_risk.max_ratio);
            l.timing.high_risk_full_s =
                num(h, "response_time_100pct_sec").unwrap_or(l.timing.high_risk_full_s);
        }
        if let Some(p) = en.get("points_of_emphasis") {
            l.emphasis_min_vertical_lx =
                num(p, "min_vertical_illuminance_lx").unwrap_or(l.emphasis_min_vertical_lx);
        }
        if let Some(steps) = en.get("glare_limits_cd").and_then(|g| g.as_array()) {
            let mut glare: Vec<GlareStep> = steps
                .iter()
                .filter_map(|s| {
                    let h = num(s, "max_height_m")?;
                    Some(GlareStep {
                        // a last step with a huge height stands for "and above"
                        below_height_m: if h >= 100.0 { f64::INFINITY } else { h },
                        escape_cd: num(s, "escape_route_cd")?,
                        high_risk_cd: num(s, "high_risk_cd")?,
                    })
                })
                .collect();
            glare.sort_by(|a, b| a.below_height_m.total_cmp(&b.below_height_m));
            if !glare.is_empty() {
                l.glare = glare;
            }
        }
        Ok(l)
    }
}

/// The limit table shipped with the crate (`EN_1838` and `EN_17037`).
pub const LIMITS_JSON: &str = include_str!("../data/en1838_en17037_limits.json");

// ─── plan ────────────────────────────────────────────────────────────

/// A luminaire in emergency mode.
#[derive(Debug, Clone)]
pub struct EmergencyLuminaire<'a> {
    pub ldt: &'a Eulumdat,
    /// Luminous flux of the luminaire in emergency mode at the end of the
    /// rated duration (lm, luminaire output).
    pub flux_lm: f64,
    pub x: f64,
    pub y: f64,
    /// Mounting height above the floor (m).
    pub height: f64,
    /// C0 direction, degrees counter-clockwise from +x.
    pub rotation: f64,
    /// Colour rendering index of the emergency light, when known.
    pub ra: Option<f64>,
}

/// Flux of a luminaire per 1000 lm of lamp flux, from its intensity table.
fn table_flux_per_klm(ldt: &Eulumdat) -> f64 {
    let cf = if ldt.conversion_factor > 0.0 {
        ldt.conversion_factor
    } else {
        1.0
    };
    PhotometricCalculations::calculated_luminous_flux(ldt) / cf
}

/// Luminous flux of the luminaire described by an LDT (lm): the flux of
/// its intensity table for its lamp flux (absolute photometry: the lamp
/// flux is the luminaire flux).
pub fn luminaire_flux_lm(ldt: &Eulumdat) -> f64 {
    table_flux_per_klm(ldt) * ldt.luminous_flux() / 1000.0
}

/// A copy of a luminaire whose lamp flux is scaled so that the luminaire
/// gives `flux_lm` (e.g. its emergency mode from a GLDF emitter).
pub fn at_flux(ldt: &Eulumdat, flux_lm: f64) -> Eulumdat {
    let mut out = ldt.clone();
    let now = luminaire_flux_lm(ldt);
    if now > 0.0 && flux_lm > 0.0 {
        let k = flux_lm / now;
        for ls in &mut out.lamp_sets {
            ls.total_luminous_flux *= k;
        }
    }
    out
}

impl EmergencyLuminaire<'_> {
    /// Factor from the LDT's cd/klm to candela at the emergency flux.
    pub fn candela_scale(&self) -> f64 {
        let per_klm = table_flux_per_klm(self.ldt);
        if per_klm > 0.0 {
            self.flux_lm / per_klm
        } else {
            self.flux_lm / 1000.0
        }
    }

    fn place(&self) -> LuminairePlace {
        let mut p = LuminairePlace::simple(0, self.x, self.y, self.height);
        p.rotation = self.rotation;
        p
    }

    /// Luminous intensity (cd) towards plan direction `azimuth` (degrees
    /// counter-clockwise from +x) at `gamma` from the downward vertical.
    pub fn intensity_cd(&self, azimuth: f64, gamma: f64) -> f64 {
        let c = (azimuth - self.rotation).rem_euclid(360.0);
        self.ldt.sample(c, gamma) * self.candela_scale()
    }
}

/// An escape route: centre line and width.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscapeRoute {
    pub name: String,
    /// Centre line (m), at least two points.
    pub path: Vec<(f64, f64)>,
    /// Width (m).
    pub width: f64,
}

/// An open (anti-panic) area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenArea {
    pub name: String,
    pub polygon: Vec<(f64, f64)>,
}

/// A high-risk task area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HighRiskArea {
    pub name: String,
    pub polygon: Vec<(f64, f64)>,
    /// Height of the task plane above the floor (m).
    pub task_height: f64,
    /// Normal maintained illuminance of the task (lx).
    pub normal_maintained_lx: f64,
}

/// What sits at a point of emphasis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum EmphasisKind {
    /// Fire alarm call point (vertical illuminance required).
    CallPoint,
    /// First-aid equipment (vertical illuminance required).
    FirstAid,
    /// Fire-fighting equipment (vertical illuminance required).
    FireEquipment,
    /// Externally lit escape sign (vertical illuminance required).
    ExternallyLitSign,
    /// Exit door (a luminaire nearby; reported).
    Exit,
    /// Stairs (reported).
    Stairs,
    /// Change of floor level (reported).
    LevelChange,
    /// Change of direction / intersection of routes (reported).
    Intersection,
}

impl EmphasisKind {
    /// Kinds with a vertical illuminance requirement.
    pub fn needs_vertical_illuminance(self) -> bool {
        matches!(
            self,
            Self::CallPoint | Self::FirstAid | Self::FireEquipment | Self::ExternallyLitSign
        )
    }
}

/// A point of emphasis.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmphasisPoint {
    pub kind: EmphasisKind,
    pub x: f64,
    pub y: f64,
    /// Height of the equipment / sign above the floor (m); default from the settings.
    pub z: Option<f64>,
    /// Direction the equipment faces (degrees counter-clockwise from +x);
    /// without it the best of eight directions is taken and the result says so.
    pub facing: Option<f64>,
}

/// Everything EN 1838 looks at.
#[derive(Debug, Clone, Default)]
pub struct EmergencyPlan<'a> {
    pub luminaires: Vec<EmergencyLuminaire<'a>>,
    pub routes: Vec<EscapeRoute>,
    pub open_areas: Vec<OpenArea>,
    pub high_risk: Vec<HighRiskArea>,
    pub points: Vec<EmphasisPoint>,
}

/// Calculation settings (not in the limit table; stated in reports).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Settings {
    /// Maintenance factor applied to the emergency flux.
    pub maintenance_factor: f64,
    /// Point spacing along routes (m).
    pub route_step_m: f64,
    /// Grid spacing in areas (m).
    pub grid_step_m: f64,
    /// Height of points of emphasis without their own height (m).
    pub emphasis_height_m: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            maintenance_factor: 0.8,
            route_step_m: 0.1,
            grid_step_m: 0.25,
            emphasis_height_m: 1.2,
        }
    }
}

// ─── results ─────────────────────────────────────────────────────────

/// Minimum, maximum, mean and Emax / Emin of a set of points.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stats {
    pub min_lx: f64,
    pub max_lx: f64,
    pub avg_lx: f64,
    /// Emax / Emin (infinite when Emin is 0).
    pub ratio: f64,
    /// Where the minimum is (m).
    pub at_min: (f64, f64),
    pub points: usize,
}

impl Stats {
    fn of(values: &[(f64, f64, f64)]) -> Self {
        let mut s = Stats {
            min_lx: f64::INFINITY,
            max_lx: 0.0,
            avg_lx: 0.0,
            ratio: f64::INFINITY,
            at_min: (0.0, 0.0),
            points: values.len(),
        };
        if values.is_empty() {
            s.min_lx = 0.0;
            return s;
        }
        let mut sum = 0.0;
        for &(x, y, e) in values {
            if e < s.min_lx {
                s.min_lx = e;
                s.at_min = (x, y);
            }
            s.max_lx = s.max_lx.max(e);
            sum += e;
        }
        s.avg_lx = sum / values.len() as f64;
        s.ratio = if s.min_lx > 0.0 {
            s.max_lx / s.min_lx
        } else {
            f64::INFINITY
        };
        s
    }
}

/// Result for one escape route.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteResult {
    pub name: String,
    pub length_m: f64,
    /// Centre line.
    pub centre: Stats,
    /// Central band (half the width).
    pub band: Stats,
    pub pass_centre: bool,
    pub pass_band: bool,
    pub pass_ratio: bool,
    /// Wider than 2 m: EN 1838 treats it as several strips or as an open area.
    pub too_wide: bool,
    /// Centre-line samples (x, y, lx) for drawing.
    pub samples: Vec<(f64, f64, f64)>,
}

impl RouteResult {
    pub fn pass(&self) -> bool {
        self.pass_centre && self.pass_band && self.pass_ratio
    }
}

/// Result for an open or high-risk area.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AreaResult {
    pub name: String,
    pub area_m2: f64,
    pub stats: Stats,
    pub required_lx: f64,
    pub max_ratio: f64,
    pub pass_min: bool,
    pub pass_ratio: bool,
    /// Anti-panic: the area is large enough to need it (> 60 m²).
    pub required: bool,
    /// Grid samples (x, y, lx).
    pub samples: Vec<(f64, f64, f64)>,
}

impl AreaResult {
    pub fn pass(&self) -> bool {
        self.pass_min && self.pass_ratio
    }
}

/// Result for a point of emphasis.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointResult {
    pub kind: EmphasisKind,
    pub x: f64,
    pub y: f64,
    /// Vertical illuminance at the point (lx), for kinds that need it.
    pub vertical_lx: Option<f64>,
    /// The facing was not given: best of eight directions.
    pub assumed_facing: bool,
    /// Horizontal distance to the nearest emergency luminaire (m).
    pub nearest_luminaire_m: Option<f64>,
    /// `None` for kinds without a limit.
    pub pass: Option<bool>,
}

/// Disability glare of one luminaire.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlareResult {
    pub luminaire: usize,
    /// Highest intensity between 60° and 90° in the checked directions (cd).
    pub i_max_cd: f64,
    /// Where: plan azimuth and γ (degrees).
    pub at: (f64, f64),
    pub limit_cd: f64,
    /// Checked against the high-risk column.
    pub high_risk: bool,
    /// Checked only in the directions of travel of a route.
    pub route_directions: bool,
    pub pass: bool,
}

/// The EN 1838 check of a plan.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmergencyReport {
    pub routes: Vec<RouteResult>,
    pub open_areas: Vec<AreaResult>,
    pub high_risk: Vec<AreaResult>,
    pub points: Vec<PointResult>,
    pub glare: Vec<GlareResult>,
    /// Luminaires whose Ra is known and below the limit.
    pub ra_below_limit: Vec<usize>,
    /// Luminaires without a known Ra.
    pub ra_unknown: Vec<usize>,
    pub settings: Settings,
}

impl EmergencyReport {
    /// Everything with a limit passes (anti-panic areas only when required).
    pub fn pass(&self) -> bool {
        self.routes.iter().all(RouteResult::pass)
            && self.open_areas.iter().all(|a| !a.required || a.pass())
            && self.high_risk.iter().all(AreaResult::pass)
            && self.points.iter().all(|p| p.pass != Some(false))
            && self.glare.iter().all(|g| g.pass)
            && self.ra_below_limit.is_empty()
    }
}

// ─── evaluation ──────────────────────────────────────────────────────

/// A luminaire ready for many illuminance calls (placement and candela
/// scale computed once: the scale integrates the intensity table).
struct Prepared<'a> {
    ldt: &'a Eulumdat,
    place: LuminairePlace,
    scale: f64,
}

fn prepare<'a>(luminaires: &[EmergencyLuminaire<'a>]) -> Vec<Prepared<'a>> {
    let mut cache: Vec<(*const Eulumdat, f64)> = Vec::new();
    luminaires
        .iter()
        .map(|l| {
            let key = l.ldt as *const Eulumdat;
            let per_klm = match cache.iter().find(|(k, _)| *k == key) {
                Some((_, v)) => *v,
                None => {
                    let v = table_flux_per_klm(l.ldt);
                    cache.push((key, v));
                    v
                }
            };
            Prepared {
                ldt: l.ldt,
                place: l.place(),
                scale: if per_klm > 0.0 {
                    l.flux_lm / per_klm
                } else {
                    l.flux_lm / 1000.0
                },
            }
        })
        .collect()
}

fn prepared_illuminance(
    lums: &[Prepared],
    point: (f64, f64, f64),
    normal: (f64, f64, f64),
    maintenance_factor: f64,
) -> f64 {
    lums.iter()
        .map(|l| compute_illuminance_at_point(l.ldt, &l.place, point, normal, l.scale))
        .sum::<f64>()
        * maintenance_factor
}

/// Illuminance (lx) at `point` on a surface with `normal`, from all
/// luminaires, maintenance factor included.
pub fn illuminance(
    luminaires: &[EmergencyLuminaire],
    point: (f64, f64, f64),
    normal: (f64, f64, f64),
    maintenance_factor: f64,
) -> f64 {
    prepared_illuminance(&prepare(luminaires), point, normal, maintenance_factor)
}

const UP: (f64, f64, f64) = (0.0, 0.0, 1.0);

/// Points along a polyline every `step` (first and last point included),
/// shifted sideways by `offset` (left of the direction of travel).
fn walk(path: &[(f64, f64)], step: f64, offset: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for w in path.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        if len <= 0.0 {
            continue;
        }
        let (ux, uy) = ((x1 - x0) / len, (y1 - y0) / len);
        let (nx, ny) = (-uy * offset, ux * offset);
        let n = (len / step.max(1e-3)).ceil().max(1.0) as usize;
        let start = if out.is_empty() { 0 } else { 1 };
        for k in start..=n {
            let t = len * k as f64 / n as f64;
            out.push((x0 + ux * t + nx, y0 + uy * t + ny));
        }
    }
    out
}

fn path_length(path: &[(f64, f64)]) -> f64 {
    path.windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .sum()
}

/// Distance from a point to a segment, and the segment's direction (deg).
fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a.0 + t * dx, a.1 + t * dy);
    (
        ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt(),
        dy.atan2(dx).to_degrees(),
    )
}

/// Distance from a point to the polygon's outline.
fn edge_distance(p: (f64, f64), poly: &[(f64, f64)]) -> f64 {
    (0..poly.len())
        .map(|i| segment_distance(p, poly[i], poly[(i + 1) % poly.len()]).0)
        .fold(f64::INFINITY, f64::min)
}

/// Grid points inside a polygon, at least `border` from its outline.
fn area_points(poly: &[(f64, f64)], step: f64, border: f64) -> Vec<(f64, f64)> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let area = AreaPolygon::new(poly.to_vec());
    let (x0, y0, x1, y1) = area.bounding_box();
    let step = step.max(0.01);
    let nx = ((x1 - x0) / step).floor() as usize;
    let ny = ((y1 - y0) / step).floor() as usize;
    let mut out = Vec::new();
    for j in 0..=ny {
        for i in 0..=nx {
            let p = (x0 + (i as f64 + 0.5) * step, y0 + (j as f64 + 0.5) * step);
            if p.0 > x1 || p.1 > y1 {
                continue;
            }
            if area.contains(p.0, p.1) && edge_distance(p, poly) >= border {
                out.push(p);
            }
        }
    }
    out
}

/// Horizontal illuminance on the floor in emergency mode, for a false-colour
/// plan: cell centres of a `step` grid over the outline's bounding box, row
/// by row from the lowest y; cells whose centre is outside are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct FloorGrid {
    pub x0: f64,
    pub y0: f64,
    pub step: f64,
    pub nx: usize,
    pub ny: usize,
    pub values: Vec<Option<f64>>,
}

impl FloorGrid {
    /// Value of cell (`i`, `j`).
    pub fn at(&self, i: usize, j: usize) -> Option<f64> {
        self.values.get(j * self.nx + i).copied().flatten()
    }

    /// Lowest and highest value inside the outline.
    pub fn min_max(&self) -> Option<(f64, f64)> {
        self.values
            .iter()
            .flatten()
            .fold(None, |acc, &v| match acc {
                None => Some((v, v)),
                Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
            })
    }
}

/// [`FloorGrid`] of `luminaires` over `outline` (maintenance factor
/// included, direct light as EN 1838 counts it). At most 200 × 200 cells:
/// a coarser step is taken for large rooms.
pub fn floor_grid(
    luminaires: &[EmergencyLuminaire],
    outline: &[(f64, f64)],
    step: f64,
    maintenance_factor: f64,
) -> FloorGrid {
    let empty = FloorGrid {
        x0: 0.0,
        y0: 0.0,
        step,
        nx: 0,
        ny: 0,
        values: Vec::new(),
    };
    if outline.len() < 3 {
        return empty;
    }
    let area = AreaPolygon::new(outline.to_vec());
    let (x0, y0, x1, y1) = area.bounding_box();
    let step = step.max((x1 - x0).max(y1 - y0) / 200.0).max(0.01);
    let nx = ((x1 - x0) / step).ceil().max(1.0) as usize;
    let ny = ((y1 - y0) / step).ceil().max(1.0) as usize;
    let lums = prepare(luminaires);
    let mut values = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let (x, y) = (x0 + (i as f64 + 0.5) * step, y0 + (j as f64 + 0.5) * step);
            values.push(
                area.contains(x, y)
                    .then(|| prepared_illuminance(&lums, (x, y, 0.0), UP, maintenance_factor)),
            );
        }
    }
    FloorGrid {
        x0,
        y0,
        step,
        nx,
        ny,
        values,
    }
}

fn polygon_area(poly: &[(f64, f64)]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    AreaPolygon::new(poly.to_vec()).area()
}

/// Highest intensity between 60° and 90° (1° steps) over the given plan
/// azimuths.
fn glare_max(l: &Prepared, azimuths: &[f64]) -> (f64, (f64, f64)) {
    let mut best = (0.0, (0.0, 60.0));
    for &az in azimuths {
        let c = (az - l.place.rotation).rem_euclid(360.0);
        for g in 60..=90 {
            let i = l.ldt.sample(c, g as f64) * l.scale;
            if i > best.0 {
                best = (i, (az.rem_euclid(360.0), g as f64));
            }
        }
    }
    best
}

/// Check a plan against EN 1838.
pub fn evaluate(plan: &EmergencyPlan, limits: &Limits, settings: &Settings) -> EmergencyReport {
    let lums = &plan.luminaires;
    let prepared = prepare(lums);
    let mf = settings.maintenance_factor;
    let e_floor = |p: (f64, f64)| prepared_illuminance(&prepared, (p.0, p.1, 0.0), UP, mf);

    let routes = plan
        .routes
        .iter()
        .map(|r| {
            let centre: Vec<(f64, f64, f64)> = walk(&r.path, settings.route_step_m, 0.0)
                .into_iter()
                .map(|p| (p.0, p.1, e_floor(p)))
                .collect();
            // the central band is half the width: its edges are a quarter
            // of the width off the centre line
            let q = r.width / 4.0;
            let band: Vec<(f64, f64, f64)> = [q, -q]
                .iter()
                .flat_map(|&o| walk(&r.path, settings.route_step_m, o))
                .chain(centre.iter().map(|&(x, y, _)| (x, y)))
                .map(|p| (p.0, p.1, e_floor(p)))
                .collect();
            let (c, b) = (Stats::of(&centre), Stats::of(&band));
            RouteResult {
                name: r.name.clone(),
                length_m: path_length(&r.path),
                pass_centre: c.min_lx >= limits.escape_route.centre_min_lx,
                pass_band: b.min_lx >= limits.escape_route.band_min_lx,
                pass_ratio: c.ratio <= limits.escape_route.max_ratio,
                too_wide: r.width > 2.0 + 1e-9,
                centre: c,
                band: b,
                samples: centre,
            }
        })
        .collect();

    let open_areas = plan
        .open_areas
        .iter()
        .map(|a| {
            let samples: Vec<(f64, f64, f64)> =
                area_points(&a.polygon, settings.grid_step_m, limits.anti_panic.border_m)
                    .into_iter()
                    .map(|p| (p.0, p.1, e_floor(p)))
                    .collect();
            let stats = Stats::of(&samples);
            let area_m2 = polygon_area(&a.polygon);
            AreaResult {
                name: a.name.clone(),
                area_m2,
                pass_min: stats.points > 0 && stats.min_lx >= limits.anti_panic.min_lx,
                pass_ratio: stats.ratio <= limits.anti_panic.max_ratio,
                required: area_m2 > limits.anti_panic.applies_above_m2,
                required_lx: limits.anti_panic.min_lx,
                max_ratio: limits.anti_panic.max_ratio,
                stats,
                samples,
            }
        })
        .collect();

    let high_risk = plan
        .high_risk
        .iter()
        .map(|a| {
            let samples: Vec<(f64, f64, f64)> = area_points(&a.polygon, settings.grid_step_m, 0.0)
                .into_iter()
                .map(|p| {
                    (
                        p.0,
                        p.1,
                        prepared_illuminance(&prepared, (p.0, p.1, a.task_height), UP, mf),
                    )
                })
                .collect();
            let stats = Stats::of(&samples);
            let required = limits.high_risk_min_lx(a.normal_maintained_lx);
            AreaResult {
                name: a.name.clone(),
                area_m2: polygon_area(&a.polygon),
                pass_min: stats.points > 0 && stats.min_lx >= required,
                pass_ratio: stats.ratio <= limits.high_risk.max_ratio,
                required: true,
                required_lx: required,
                max_ratio: limits.high_risk.max_ratio,
                stats,
                samples,
            }
        })
        .collect();

    let points = plan
        .points
        .iter()
        .map(|p| {
            let z = p.z.unwrap_or(settings.emphasis_height_m);
            let nearest = lums
                .iter()
                .map(|l| ((l.x - p.x).powi(2) + (l.y - p.y).powi(2)).sqrt())
                .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))));
            let vertical = p.kind.needs_vertical_illuminance().then(|| {
                let e_dir = |deg: f64| {
                    let a = deg.to_radians();
                    prepared_illuminance(&prepared, (p.x, p.y, z), (a.cos(), a.sin(), 0.0), mf)
                };
                match p.facing {
                    Some(f) => e_dir(f),
                    None => (0..8).map(|k| e_dir(k as f64 * 45.0)).fold(0.0, f64::max),
                }
            });
            PointResult {
                kind: p.kind,
                x: p.x,
                y: p.y,
                pass: vertical.map(|e| e >= limits.emphasis_min_vertical_lx),
                vertical_lx: vertical,
                assumed_facing: vertical.is_some() && p.facing.is_none(),
                nearest_luminaire_m: nearest,
            }
        })
        .collect();

    let glare = lums
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let high = plan.high_risk.iter().any(|a| {
                a.polygon.len() >= 3 && AreaPolygon::new(a.polygon.clone()).contains(l.x, l.y)
            });
            // a luminaire over a route: the directions of travel there
            let on_route = plan
                .routes
                .iter()
                .flat_map(|r| {
                    r.path
                        .windows(2)
                        .map(move |w| (segment_distance((l.x, l.y), w[0], w[1]), r.width))
                })
                .filter(|((d, _), w)| *d <= w / 2.0 + 0.5)
                .min_by(|a, b| a.0 .0.total_cmp(&b.0 .0))
                .map(|((_, dir), _)| dir);
            let azimuths: Vec<f64> = match (high, on_route) {
                (false, Some(dir)) => vec![dir, dir + 180.0],
                _ => (0..360).map(|a| a as f64).collect(),
            };
            let (i_max, at) = glare_max(&prepared[i], &azimuths);
            let limit = limits.glare_limit(l.height, high);
            GlareResult {
                luminaire: i,
                i_max_cd: i_max,
                at,
                limit_cd: limit,
                high_risk: high,
                route_directions: !high && on_route.is_some(),
                pass: i_max <= limit,
            }
        })
        .collect();

    EmergencyReport {
        routes,
        open_areas,
        high_risk,
        points,
        glare,
        ra_below_limit: lums
            .iter()
            .enumerate()
            .filter(|(_, l)| l.ra.is_some_and(|ra| ra < limits.min_ra))
            .map(|(i, _)| i)
            .collect(),
        ra_unknown: lums
            .iter()
            .enumerate()
            .filter(|(_, l)| l.ra.is_none())
            .map(|(i, _)| i)
            .collect(),
        settings: settings.clone(),
    }
}

// ─── spacing table ───────────────────────────────────────────────────

/// What one emergency luminaire achieves at one mounting height: the
/// spacing tables manufacturers publish, from the limit values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpacingRow {
    pub height_m: f64,
    /// Illuminance below the luminaire, alone (lx).
    pub e_nadir_lx: f64,
    /// Largest spacing in a row along an escape route of `route_width_m`,
    /// row along C0 / C90 (m; 0 when even close spacing fails).
    pub route_spacing_c0_m: f64,
    pub route_spacing_c90_m: f64,
    pub route_width_m: f64,
    /// Largest square grid for an open area (m).
    pub open_area_grid_m: f64,
    /// Highest intensity between 60° and 90°, all directions (cd).
    pub glare_i_max_cd: f64,
    pub glare_limit_cd: f64,
}

/// Spacing table of a luminaire at its emergency flux.
pub fn spacing_table(
    ldt: &Eulumdat,
    flux_lm: f64,
    heights: &[f64],
    route_width_m: f64,
    limits: &Limits,
    settings: &Settings,
) -> Vec<SpacingRow> {
    heights
        .iter()
        .map(|&h| {
            let lum = |x: f64, y: f64| EmergencyLuminaire {
                ldt,
                flux_lm,
                x,
                y,
                height: h,
                rotation: 0.0,
                ra: None,
            };
            let mf = settings.maintenance_factor;
            // a row of luminaires along x (C0) or y (C90), spacing s
            let route_ok = |s: f64, along_y: bool| -> bool {
                let row = prepare(
                    &(-6..=6)
                        .map(|k| {
                            let t = k as f64 * s;
                            if along_y {
                                lum(0.0, t)
                            } else {
                                lum(t, 0.0)
                            }
                        })
                        .collect::<Vec<_>>(),
                );
                let q = route_width_m / 4.0;
                let (mut cmin, mut cmax, mut bmin) = (f64::INFINITY, 0.0f64, f64::INFINITY);
                for k in 0..=40 {
                    let t = s * k as f64 / 40.0;
                    for off in [0.0, q, -q] {
                        let p = if along_y {
                            (off, t, 0.0)
                        } else {
                            (t, off, 0.0)
                        };
                        let e = prepared_illuminance(&row, p, UP, mf);
                        if off == 0.0 {
                            cmin = cmin.min(e);
                            cmax = cmax.max(e);
                        }
                        bmin = bmin.min(e);
                    }
                }
                cmin >= limits.escape_route.centre_min_lx
                    && bmin >= limits.escape_route.band_min_lx
                    && cmax / cmin <= limits.escape_route.max_ratio
            };
            let open_ok = |s: f64| -> bool {
                let grid = prepare(
                    &(-4..=4)
                        .flat_map(|i| (-4..=4).map(move |j| (i, j)))
                        .map(|(i, j)| lum(i as f64 * s, j as f64 * s))
                        .collect::<Vec<_>>(),
                );
                let (mut mn, mut mx) = (f64::INFINITY, 0.0f64);
                for i in 0..=8 {
                    for j in 0..=8 {
                        let p = (s * i as f64 / 8.0, s * j as f64 / 8.0, 0.0);
                        let e = prepared_illuminance(&grid, p, UP, mf);
                        mn = mn.min(e);
                        mx = mx.max(e);
                    }
                }
                mn >= limits.anti_panic.min_lx && mx / mn <= limits.anti_panic.max_ratio
            };
            // in 0.1 m steps from 0.5 m; the limits get harder with the
            // spacing, so the search stops 1 m after the first failure
            let largest = |ok: &dyn Fn(f64) -> bool| -> f64 {
                let mut best = 0.0;
                let mut first_fail: Option<f64> = None;
                let mut k = 5;
                while k <= 400 {
                    let s = k as f64 / 10.0;
                    if ok(s) {
                        best = s;
                    } else if first_fail.is_none() {
                        first_fail = Some(s);
                    }
                    if first_fail.is_some_and(|f| s >= f + 1.0) {
                        break;
                    }
                    k += 1;
                }
                best
            };
            let one = prepare(&[lum(0.0, 0.0)]);
            let (i_max, _) = glare_max(&one[0], &(0..360).map(|a| a as f64).collect::<Vec<_>>());
            SpacingRow {
                height_m: h,
                e_nadir_lx: prepared_illuminance(&one, (0.0, 0.0, 0.0), UP, mf),
                route_spacing_c0_m: largest(&|s| route_ok(s, false)),
                route_spacing_c90_m: largest(&|s| route_ok(s, true)),
                route_width_m,
                open_area_grid_m: largest(&open_ok),
                glare_i_max_cd: i_max,
                glare_limit_cd: limits.glare_limit(h, false),
            }
        })
        .collect()
}

// ─── automatic layout ────────────────────────────────────────────────

/// What [`auto_layout`] places luminaires for.
#[derive(Debug, Clone)]
pub struct AutoInput<'a> {
    pub ldt: &'a Eulumdat,
    /// Emergency flux (lm).
    pub flux_lm: f64,
    /// Mounting height (m).
    pub height: f64,
    pub ra: Option<f64>,
    /// The luminaire's spacing at this height ([`spacing_table`]).
    pub spacing: &'a SpacingRow,
    pub routes: &'a [EscapeRoute],
    /// The room's outline (luminaires are only added inside it).
    pub room: Option<&'a [(f64, f64)]>,
    /// The open (anti-panic) area, if the room is one.
    pub open_area: Option<&'a [(f64, f64)]>,
    pub points: &'a [EmphasisPoint],
}

impl<'a> AutoInput<'a> {
    /// The plan for luminaires at `positions` (x, y, C0 direction).
    pub fn plan(&self, positions: &[(f64, f64, f64)]) -> EmergencyPlan<'a> {
        EmergencyPlan {
            luminaires: positions
                .iter()
                .map(|&(x, y, r)| EmergencyLuminaire {
                    ldt: self.ldt,
                    flux_lm: self.flux_lm,
                    x,
                    y,
                    height: self.height,
                    rotation: r,
                    ra: self.ra,
                })
                .collect(),
            routes: self.routes.to_vec(),
            open_areas: self
                .open_area
                .filter(|p| p.len() >= 3)
                .map(|p| {
                    vec![OpenArea {
                        name: "room".into(),
                        polygon: p.to_vec(),
                    }]
                })
                .unwrap_or_default(),
            high_risk: Vec::new(),
            points: self.points.to_vec(),
        }
    }
}

/// Point and direction (deg) at arc length `t` of a polyline.
fn along(p: &[(f64, f64)], mut t: f64) -> (f64, f64, f64) {
    let n = p.len().saturating_sub(1);
    for i in 0..n {
        let (a, b) = (p[i], p[i + 1]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if t <= len + 1e-9 || i + 1 == n {
            let k = if len > 0.0 {
                (t / len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dir = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
            return (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k, dir);
        }
        t -= len;
    }
    let last = p.last().copied().unwrap_or((0.0, 0.0));
    (last.0, last.1, 0.0)
}

/// Route luminaires (both ends and evenly between, C0 along the route) and
/// an open-area grid of `grid_m`, away from the route luminaires.
fn layout_with_grid(input: &AutoInput, grid_m: Option<f64>) -> Vec<(f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    let s = input
        .spacing
        .route_spacing_c0_m
        .max(input.spacing.route_spacing_c90_m)
        .max(0.5);
    for r in input.routes {
        let len = path_length(&r.path);
        if len <= 0.0 {
            continue;
        }
        let n = (len / s).ceil().max(1.0) as usize;
        for k in 0..=n {
            let p = along(&r.path, len * k as f64 / n as f64);
            if out.iter().all(|q| (q.0 - p.0).hypot(q.1 - p.1) >= 0.5) {
                out.push(p);
            }
        }
    }
    if let (Some(poly), Some(g)) = (input.open_area.filter(|p| p.len() >= 3), grid_m) {
        let area = AreaPolygon::new(poly.to_vec());
        let (x0, y0, x1, y1) = area.bounding_box();
        let g = g.max(0.5);
        let nx = ((x1 - x0) / g).ceil().max(1.0) as usize;
        let ny = ((y1 - y0) / g).ceil().max(1.0) as usize;
        let on_routes = out.len();
        for j in 0..ny {
            for i in 0..nx {
                let x = x0 + (i as f64 + 0.5) * (x1 - x0) / nx as f64;
                let y = y0 + (j as f64 + 0.5) * (y1 - y0) / ny as f64;
                if area.contains(x, y)
                    && out[..on_routes]
                        .iter()
                        .all(|q| (q.0 - x).hypot(q.1 - y) >= g * 0.5)
                {
                    out.push((x, y, 0.0));
                }
            }
        }
    }
    out
}

/// Automatic layout: luminaires at both ends of every route and evenly
/// between (C0 along the route); when the open area is larger than the
/// anti-panic limit, a grid over it, tightened (× 1.0 … 0.5 of the
/// spacing) until the area passes (the spacing holds for an endless grid,
/// along walls a luminaire has fewer neighbours); then, for points of
/// emphasis still short of their vertical illuminance, a luminaire in front
/// of each (C0 towards it, inside the room) at the distance (0.5–4 m) that
/// lights it best: a wide optic close by, a directional one from where its
/// beam meets the wall.
pub fn auto_layout(
    input: &AutoInput,
    limits: &Limits,
    settings: &Settings,
) -> Vec<(f64, f64, f64)> {
    let area_required = input
        .open_area
        .filter(|p| p.len() >= 3)
        .is_some_and(|p| polygon_area(p) > limits.anti_panic.applies_above_m2);
    let mut pos = Vec::new();
    for f in [1.0, 0.9, 0.8, 0.7, 0.6, 0.5] {
        let grid = area_required.then_some(input.spacing.open_area_grid_m * f);
        pos = layout_with_grid(input, grid);
        if !area_required
            || evaluate(&input.plan(&pos), limits, settings)
                .open_areas
                .iter()
                .all(AreaResult::pass)
        {
            break;
        }
    }
    let rep = evaluate(&input.plan(&pos), limits, settings);
    for (p, r) in input.points.iter().zip(&rep.points) {
        if r.pass != Some(false) {
            continue;
        }
        let face = p.facing.unwrap_or(90.0);
        let (fx, fy) = (face.to_radians().cos(), face.to_radians().sin());
        let z = p.z.unwrap_or(settings.emphasis_height_m);
        let vertical = |cand: (f64, f64, f64)| -> f64 {
            let mut with = pos.clone();
            with.push(cand);
            illuminance(
                &input.plan(&with).luminaires,
                (p.x, p.y, z),
                (fx, fy, 0.0),
                settings.maintenance_factor,
            )
        };
        let room = input
            .room
            .filter(|r| r.len() >= 3)
            .map(|r| AreaPolygon::new(r.to_vec()));
        let best = [0.5, 0.7, 0.9, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0, 3.5, 4.0]
            .into_iter()
            .map(|d| (p.x + fx * d, p.y + fy * d, face + 180.0))
            .filter(|c| room.as_ref().is_none_or(|r| r.contains(c.0, c.1)))
            .max_by(|a, b| vertical(*a).total_cmp(&vertical(*b)));
        if let Some(c) = best {
            pos.push(c);
        }
    }
    pos
}
