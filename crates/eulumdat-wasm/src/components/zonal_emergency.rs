//! Emergency lighting (EN 1838) in the designers: emergency luminaires
//! along escape routes drawn on the plan and over open areas (or the
//! room's own luminaires in emergency mode), points of emphasis, the check
//! with `eulumdat::emergency`, the plan overlay, results and the report
//! section.
//!
//! Interior first. The state is not tied to rooms ([`EmergencyState::outdoor`]
//! is reserved for exterior escape routes in the outdoor designer, a later
//! step): routes, points and luminaires are plan coordinates in metres.

use super::plan_drag::Moves;
use super::zonal_sensors::fill;
use crate::i18n::use_locale;
use eulumdat::emergency::{
    self, Check, EmergencyLuminaire, EmergencyReport, EmphasisKind, EmphasisPoint, EscapeRoute,
    FloorGrid, Limits, PlaceCategory, Settings, SpacingRow,
};
use eulumdat::Eulumdat;
use leptos::prelude::*;
use std::sync::{Arc, OnceLock};

const DEMO_ROUTE: &str = include_str!("../../demo/demo_em_route.ldt");
const DEMO_OPEN: &str = include_str!("../../demo/demo_em_open.ldt");

fn demo(which: EmLuminaire) -> Option<Arc<Eulumdat>> {
    static ROUTE: OnceLock<Option<Arc<Eulumdat>>> = OnceLock::new();
    static OPEN: OnceLock<Option<Arc<Eulumdat>>> = OnceLock::new();
    let (cell, text) = match which {
        EmLuminaire::DemoRoute => (&ROUTE, DEMO_ROUTE),
        EmLuminaire::DemoOpen => (&OPEN, DEMO_OPEN),
        _ => return None,
    };
    cell.get_or_init(|| Eulumdat::parse(text).ok().map(Arc::new))
        .clone()
}

/// Which luminaire lights in emergency mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmLuminaire {
    /// Synthetic demo: escape-route optic.
    DemoRoute,
    /// Synthetic demo: anti-panic optic.
    DemoOpen,
    /// The luminaire loaded in the editor (e.g. an emergency LDT).
    Loaded,
    /// The room's luminaires in emergency mode (a share of their flux).
    Room,
}

impl EmLuminaire {
    pub const ALL: [Self; 4] = [Self::DemoRoute, Self::DemoOpen, Self::Loaded, Self::Room];
    pub fn key(self) -> &'static str {
        match self {
            Self::DemoRoute => "route",
            Self::DemoOpen => "open",
            Self::Loaded => "loaded",
            Self::Room => "room",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.key() == k)
    }
}

/// What a click on the plan does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmTool {
    Off,
    /// Add a point to the route being drawn.
    Route,
    /// Place a point of emphasis.
    Point(EmphasisKind),
}

/// An escape route as drawn (centre line in metres).
#[derive(Clone, Debug, PartialEq)]
pub struct RouteDraft {
    pub path: Vec<(f64, f64)>,
    pub width: f64,
    /// What the route is (China: corridor 3 lx, stairs / refuge 10 lx).
    pub place: PlaceCategory,
}

/// Label of a place category in a standard (its own words when the
/// locale has them, e.g. Brazil's "level changes").
pub fn place_label_in(s: &eulumdat_i18n::ZonalEmergency, std_id: &str, p: PlaceCategory) -> String {
    s.std_place(std_id, p.key())
        .unwrap_or_else(|| place_label(s, p))
}

/// Label of a place category.
pub fn place_label(s: &eulumdat_i18n::ZonalEmergency, p: PlaceCategory) -> String {
    match p {
        PlaceCategory::EscapeCorridor => s.place_corridor.clone(),
        PlaceCategory::StairsOrRefuge => s.place_stairs.clone(),
        PlaceCategory::CrowdedPlace => s.place_crowded.clone(),
        PlaceCategory::OtherPlace => s.place_other.clone(),
    }
}

/// A point of emphasis as placed (facing into the room, degrees).
#[derive(Clone, Debug, PartialEq)]
pub struct PointDraft {
    pub kind: EmphasisKind,
    pub x: f64,
    pub y: f64,
    pub facing: f64,
}

pub const POINT_KINDS: [EmphasisKind; 8] = [
    EmphasisKind::FireEquipment,
    EmphasisKind::FirstAid,
    EmphasisKind::CallPoint,
    EmphasisKind::ExternallyLitSign,
    EmphasisKind::DoorRelease,
    EmphasisKind::EscapePlan,
    EmphasisKind::EvacuationAid,
    EmphasisKind::Exit,
];

fn kind_key(k: EmphasisKind) -> &'static str {
    match k {
        EmphasisKind::FireEquipment => "fe",
        EmphasisKind::FirstAid => "fa",
        EmphasisKind::CallPoint => "cp",
        EmphasisKind::ExternallyLitSign => "sg",
        EmphasisKind::Exit => "ex",
        EmphasisKind::Stairs => "st",
        EmphasisKind::LevelChange => "lc",
        EmphasisKind::Intersection => "in",
        EmphasisKind::DoorRelease => "dr",
        EmphasisKind::EscapePlan => "ep",
        EmphasisKind::EvacuationAid => "ea",
    }
}

fn kind_from_key(k: &str) -> Option<EmphasisKind> {
    [
        EmphasisKind::FireEquipment,
        EmphasisKind::FirstAid,
        EmphasisKind::CallPoint,
        EmphasisKind::ExternallyLitSign,
        EmphasisKind::Exit,
        EmphasisKind::Stairs,
        EmphasisKind::LevelChange,
        EmphasisKind::Intersection,
        EmphasisKind::DoorRelease,
        EmphasisKind::EscapePlan,
        EmphasisKind::EvacuationAid,
    ]
    .into_iter()
    .find(|v| kind_key(*v) == k)
}

pub fn kind_icon(k: EmphasisKind) -> &'static str {
    match k {
        EmphasisKind::FireEquipment => "🧯",
        EmphasisKind::FirstAid => "⛑",
        EmphasisKind::CallPoint => "🔔",
        EmphasisKind::ExternallyLitSign => "🪧",
        EmphasisKind::Exit => "🚪",
        EmphasisKind::DoorRelease => "🔓",
        EmphasisKind::EscapePlan => "🗺",
        EmphasisKind::EvacuationAid => "♿",
        _ => "•",
    }
}

pub fn kind_label(s: &eulumdat_i18n::ZonalEmergency, k: EmphasisKind) -> String {
    match k {
        EmphasisKind::FireEquipment => s.kind_fire.clone(),
        EmphasisKind::FirstAid => s.kind_first_aid.clone(),
        EmphasisKind::CallPoint => s.kind_call.clone(),
        EmphasisKind::ExternallyLitSign => s.kind_sign.clone(),
        EmphasisKind::Exit => s.kind_exit.clone(),
        EmphasisKind::DoorRelease => s.kind_door_release.clone(),
        EmphasisKind::EscapePlan => s.kind_escape_plan.clone(),
        EmphasisKind::EvacuationAid => s.kind_evac_aid.clone(),
        other => format!("{other:?}"),
    }
}

/// UI state of the emergency section (all signals, `Copy`).
#[derive(Clone, Copy)]
pub struct EmergencyState {
    pub enabled: RwSignal<bool>,
    pub luminaire: RwSignal<EmLuminaire>,
    /// Emergency flux (lm); `None` = the file's own flux.
    pub flux: RwSignal<Option<f64>>,
    /// The room's luminaires: emergency flux in % of the normal flux.
    pub pct: RwSignal<f64>,
    /// Mounting height (m); `None` = the room height.
    pub height: RwSignal<Option<f64>>,
    pub mf: RwSignal<f64>,
    pub routes: RwSignal<Vec<RouteDraft>>,
    /// Width of new routes (m).
    pub width: RwSignal<f64>,
    /// The room is an open (anti-panic) area.
    pub open_area: RwSignal<bool>,
    pub points: RwSignal<Vec<PointDraft>>,
    pub tool: RwSignal<EmTool>,
    /// The route being drawn.
    pub wip: RwSignal<Vec<(f64, f64)>>,
    /// Emergency luminaires moved by hand.
    pub moves: RwSignal<Moves>,
    /// Exterior planning (outdoor designer, later): no walls, no room area.
    pub outdoor: bool,
    /// The plan shows emergency conditions (normal lighting off, the
    /// floor in the emergency light); the Light scene opens that way.
    pub show: RwSignal<bool>,
    /// The standard checked against (a [`Limits`] id).
    pub standard: RwSignal<String>,
    /// What the room is when its floor is checked (standards with a
    /// minimum per kind of place).
    pub open_place: RwSignal<PlaceCategory>,
    /// Output at the end of the rated duration (% of the emergency flux;
    /// standards with end-of-duration values).
    pub end_pct: RwSignal<f64>,
}

/// The standard a plan starts with when its link names none: from the UI
/// language, else the browser's region (en-US → NFPA 101 …), else EN
/// 1838:2024 (the current edition; links name `emstd=en1838` for 2013).
pub fn default_standard(ui_code: &str) -> String {
    let browser = web_sys::window()
        .and_then(|w| w.navigator().language())
        .unwrap_or_default();
    standard_for_language(ui_code, &browser).to_string()
}

/// [`default_standard`] for a UI language code and a browser language tag.
pub fn standard_for_language(ui_code: &str, browser: &str) -> &'static str {
    let b = browser.to_ascii_lowercase().replace('_', "-");
    let region = b.split('-').nth(1).unwrap_or("");
    match ui_code {
        c if c.starts_with("zh") => return "gb55037",
        "pt-BR" => return "nbr10898",
        "ru" => return "sp52",
        _ => {}
    }
    match (b.split('-').next().unwrap_or(""), region) {
        ("ja", _) => "jp-bsl",
        (_, "us") => "nfpa101",
        (_, "ca") => "nbcc",
        (_, "gb") => "bs5266",
        (_, "au") => "as2293",
        (_, "nz") => "nz-f6",
        (_, "in") => "nbc-in",
        (_, "br") => "nbr10898",
        ("es", "es") => "cte-sua4",
        ("fr", "fr") => "fr-erp",
        _ => "en1838-2024",
    }
}

fn parse_pairs(s: &str) -> Vec<(f64, f64)> {
    s.split(';')
        .filter_map(|p| {
            let (x, y) = p.split_once(',')?;
            Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
        })
        .collect()
}

impl EmergencyState {
    /// State from the share link (`get` reads a key of the `#interior?` hash).
    pub fn from_link(get: impl Fn(&str) -> Option<String>) -> Self {
        let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
        let routes = get("emr")
            .map(|v| {
                v.split('_')
                    .filter_map(|r| {
                        // centre line ~ width [~ place]
                        let mut parts = r.split('~');
                        let path = parse_pairs(parts.next()?);
                        let w = parts.next().unwrap_or("2");
                        let place = parts
                            .next()
                            .and_then(PlaceCategory::from_key)
                            .unwrap_or_default();
                        (path.len() >= 2).then(|| RouteDraft {
                            path,
                            width: w.parse().unwrap_or(2.0f64).clamp(0.5, 20.0),
                            place,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let points = get("emp")
            .map(|v| {
                v.split('_')
                    .filter_map(|p| {
                        let (k, rest) = p.split_once(':')?;
                        let n: Vec<f64> = rest.split(',').filter_map(|x| x.parse().ok()).collect();
                        Some(PointDraft {
                            kind: kind_from_key(k)?,
                            x: *n.first()?,
                            y: *n.get(1)?,
                            facing: n.get(2).copied().unwrap_or(90.0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            enabled: RwSignal::new(get("em").is_some_and(|v| v == "1")),
            luminaire: RwSignal::new(
                get("eml")
                    .and_then(|v| EmLuminaire::from_key(&v))
                    .unwrap_or(EmLuminaire::DemoRoute),
            ),
            flux: RwSignal::new(num("emflux").filter(|v| *v > 0.0)),
            pct: RwSignal::new(num("empct").unwrap_or(10.0).clamp(1.0, 100.0)),
            height: RwSignal::new(num("emh").filter(|v| *v > 0.5)),
            mf: RwSignal::new(num("emmf").unwrap_or(0.8).clamp(0.3, 1.0)),
            routes: RwSignal::new(routes),
            width: RwSignal::new(num("emw").unwrap_or(2.0).clamp(0.5, 20.0)),
            open_area: RwSignal::new(get("emo").map(|v| v != "0").unwrap_or(true)),
            points: RwSignal::new(points),
            tool: RwSignal::new(EmTool::Off),
            wip: RwSignal::new(Vec::new()),
            moves: RwSignal::new(get("emmove").map(|v| Moves::decode(&v)).unwrap_or_default()),
            outdoor: false,
            show: RwSignal::new(get("emview").is_some_and(|v| v == "1")),
            // a link names the standard; else the Chinese UI checks against
            // the Chinese one
            standard: RwSignal::new(
                get("emstd")
                    .and_then(|v| Limits::by_id(&v))
                    .map(|l| l.id)
                    .unwrap_or_else(|| default_standard(&use_locale().get_untracked().meta.code)),
            ),
            open_place: RwSignal::new(
                get("emoplace")
                    .and_then(|v| PlaceCategory::from_key(&v))
                    .unwrap_or(PlaceCategory::CrowdedPlace),
            ),
            end_pct: RwSignal::new(num("emend").unwrap_or(100.0).clamp(1.0, 100.0)),
        }
    }

    /// Switch the standard; places it does not offer become its first
    /// choice.
    pub fn set_standard(&self, id: &str) {
        let Some(l) = Limits::by_id(id) else {
            return;
        };
        if let Some(p) = &l.places {
            if let Some(&first) = p.route_places.first() {
                self.routes.update(|v| {
                    for r in v.iter_mut().filter(|r| !p.route_places.contains(&r.place)) {
                        r.place = first;
                    }
                });
            }
            if let Some(&first) = p.area_places.first() {
                if !p.area_places.contains(&self.open_place.get_untracked()) {
                    self.open_place.set(first);
                }
            }
        }
        self.standard.set(l.id);
    }

    /// Link keys (tracked), `None` when emergency planning is off.
    pub fn link_query(&self) -> Option<String> {
        if !self.enabled.get() {
            return None;
        }
        let mut parts = vec![
            "em=1".to_string(),
            format!("eml={}", self.luminaire.get().key()),
            format!("emstd={}", self.standard.get()),
        ];
        if self.open_place.get() != PlaceCategory::CrowdedPlace {
            parts.push(format!("emoplace={}", self.open_place.get().key()));
        }
        if let Some(f) = self.flux.get() {
            parts.push(format!("emflux={f:.0}"));
        }
        if self.luminaire.get() == EmLuminaire::Room {
            parts.push(format!("empct={:.0}", self.pct.get()));
        }
        if let Some(h) = self.height.get() {
            parts.push(format!("emh={h:.2}"));
        }
        if (self.mf.get() - 0.8).abs() > 1e-9 {
            parts.push(format!("emmf={:.2}", self.mf.get()));
        }
        let routes = self.routes.get();
        if !routes.is_empty() {
            let r: Vec<String> = routes
                .iter()
                .map(|r| {
                    let pts: Vec<String> = r
                        .path
                        .iter()
                        .map(|(x, y)| format!("{x:.2},{y:.2}"))
                        .collect();
                    if r.place == PlaceCategory::EscapeCorridor {
                        format!("{}~{:.2}", pts.join(";"), r.width)
                    } else {
                        format!("{}~{:.2}~{}", pts.join(";"), r.width, r.place.key())
                    }
                })
                .collect();
            parts.push(format!("emr={}", r.join("_")));
        }
        if !self.open_area.get() {
            parts.push("emo=0".into());
        }
        let points = self.points.get();
        if !points.is_empty() {
            let p: Vec<String> = points
                .iter()
                .map(|p| format!("{}:{:.2},{:.2},{:.0}", kind_key(p.kind), p.x, p.y, p.facing))
                .collect();
            parts.push(format!("emp={}", p.join("_")));
        }
        if let Some(m) = self.moves.with(|m| m.encode()) {
            parts.push(format!("emmove={m}"));
        }
        if self.show.get() {
            parts.push("emview=1".into());
        }
        if self.end_pct.get() < 100.0 {
            parts.push(format!("emend={:.0}", self.end_pct.get()));
        }
        Some(parts.join("&"))
    }

    /// The limits of the chosen standard (tracked).
    pub fn limits(&self) -> Limits {
        Limits::by_id(&self.standard.get()).unwrap_or_else(Limits::en1838)
    }

    /// End the route being drawn (kept when it has two points or more).
    pub fn finish_route(&self) {
        let wip = self.wip.get_untracked();
        if wip.len() >= 2 {
            let width = self.width.get_untracked();
            self.routes.update(|r| {
                r.push(RouteDraft {
                    path: wip,
                    width,
                    place: PlaceCategory::default(),
                })
            });
        }
        self.wip.set(Vec::new());
        self.tool.set(EmTool::Off);
    }
}

/// Direction a wall-mounted item faces: into the room, away from the
/// nearest edge of the outline (degrees counter-clockwise from +x).
pub fn facing_from_walls(poly: &[(f64, f64)], x: f64, y: f64) -> f64 {
    let mut best = (f64::INFINITY, 90.0);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        if len2 <= 0.0 {
            continue;
        }
        let t = (((x - a.0) * dx + (y - a.1) * dy) / len2).clamp(0.0, 1.0);
        let (qx, qy) = (a.0 + t * dx, a.1 + t * dy);
        let d = (x - qx).hypot(y - qy);
        if d < best.0 {
            // from the wall towards the point; on the wall: the edge's normal
            // towards the room's centre
            let (cx, cy) = poly.iter().fold((0.0, 0.0), |s, p| (s.0 + p.0, s.1 + p.1));
            let (cx, cy) = (cx / poly.len() as f64, cy / poly.len() as f64);
            let mut n = (-dy, dx);
            if n.0 * (cx - qx) + n.1 * (cy - qy) < 0.0 {
                n = (dy, -dx);
            }
            best = (d, n.1.atan2(n.0).to_degrees());
        }
    }
    best.1
}

/// Colour rendering from an LDT's lamp data (group or number).
fn ra_of(l: &Eulumdat) -> Option<f64> {
    let g = l
        .lamp_sets
        .first()?
        .color_rendering_group
        .trim()
        .to_ascii_uppercase();
    match g.as_str() {
        "1A" => Some(90.0),
        "1B" => Some(80.0),
        "2A" => Some(70.0),
        "2B" => Some(60.0),
        "3" => Some(40.0),
        "4" => Some(20.0),
        other => other
            .parse::<f64>()
            .ok()
            .filter(|v| (0.0..=100.0).contains(v)),
    }
}

/// The emergency luminaire and what it achieves at the mounting height.
#[derive(Clone)]
pub struct EmSource {
    pub ldt: Arc<Eulumdat>,
    pub which: EmLuminaire,
    pub name: String,
    /// Flux of the file (lm).
    pub own_flux: f64,
    /// Emergency flux used (lm).
    pub flux: f64,
    pub height: f64,
    pub mf: f64,
    /// Spacing table row (not for the room's luminaires).
    pub spacing: Option<SpacingRow>,
}

impl PartialEq for EmSource {
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.ldt, &o.ldt)
            && self.which == o.which
            && self.flux == o.flux
            && self.height == o.height
            && self.mf == o.mf
            && self.spacing == o.spacing
    }
}

/// The emergency luminaire (tracked; the spacing search runs here, apart
/// from the route edits).
pub fn source(st: &EmergencyState, loaded: &Eulumdat, room_height: f64) -> Option<EmSource> {
    if !st.enabled.get() {
        return None;
    }
    let which = st.luminaire.get();
    let ldt = match which {
        EmLuminaire::DemoRoute | EmLuminaire::DemoOpen => demo(which)?,
        _ => Arc::new(loaded.clone()),
    };
    let own_flux = emergency::luminaire_flux_lm(&ldt);
    let flux = match which {
        EmLuminaire::Room => own_flux * st.pct.get() / 100.0,
        _ => st.flux.get().unwrap_or(own_flux),
    };
    if flux.is_nan() || flux <= 0.0 {
        return None;
    }
    let height = st.height.get().unwrap_or(room_height).max(0.5);
    let mf = st.mf.get();
    let width = st
        .routes
        .with(|r| r.iter().map(|r| r.width).fold(0.0f64, f64::max));
    let width = if width > 0.0 {
        width.min(2.0)
    } else {
        st.width.get().min(2.0)
    };
    let settings = Settings {
        maintenance_factor: mf,
        end_output_share: st.end_pct.get() / 100.0,
        ..Default::default()
    };
    let spacing = (which != EmLuminaire::Room).then(|| {
        emergency::spacing_table(&ldt, flux, &[height], width, &st.limits(), &settings)
            .into_iter()
            .next()
    });
    let name = if ldt.luminaire_name.trim().is_empty() {
        ldt.file_name.clone()
    } else {
        ldt.luminaire_name.clone()
    };
    Some(EmSource {
        ldt,
        which,
        name,
        own_flux,
        flux,
        height,
        mf,
        spacing: spacing.flatten(),
    })
}

/// The check of a plan.
#[derive(Clone, PartialEq)]
pub struct EmResult {
    pub report: EmergencyReport,
    /// The standard checked against.
    pub limits: Limits,
    /// Luminaires: x, y, C0 direction (deg).
    pub positions: Vec<(f64, f64, f64)>,
    pub source: EmSource,
    pub move_key: String,
    pub moved: usize,
    pub room_area: f64,
    /// Facing of each point of emphasis (deg), in the report's order.
    pub facings: Vec<f64>,
}

impl EmResult {
    /// Anything to check besides the luminaires' glare.
    pub fn has_checks(&self) -> bool {
        !self.report.routes.is_empty()
            || !self.report.open_areas.is_empty()
            || !self.report.points.is_empty()
    }
}

/// Check the plan (tracked).
pub fn compute(
    st: &EmergencyState,
    src: &EmSource,
    poly: &[(f64, f64)],
    room_positions: &[(f64, f64)],
) -> EmResult {
    let routes: Vec<EscapeRoute> = st
        .routes
        .get()
        .iter()
        .enumerate()
        .map(|(i, r)| EscapeRoute {
            name: format!("{}", i + 1),
            path: r.path.clone(),
            width: r.width,
            place: r.place,
        })
        .collect();
    let open = st.open_area.get();
    let limits = st.limits();
    let settings = Settings {
        maintenance_factor: src.mf,
        end_output_share: st.end_pct.get() / 100.0,
        ..Default::default()
    };
    let room_area = if poly.len() >= 3 {
        eulumdat::area::AreaPolygon::new(poly.to_vec()).area()
    } else {
        0.0
    };
    let points: Vec<EmphasisPoint> = st.points.with(|p| {
        p.iter()
            .map(|p| EmphasisPoint {
                kind: p.kind,
                x: p.x,
                y: p.y,
                z: None,
                facing: Some(p.facing),
            })
            .collect()
    });
    let fallback = SpacingRow {
        height_m: src.height,
        e_nadir_lx: 0.0,
        route_spacing_c0_m: 0.0,
        route_spacing_c90_m: 0.0,
        route_width_m: 2.0,
        open_area_grid_m: 0.0,
        glare_i_max_cd: 0.0,
        glare_limit_cd: 0.0,
    };
    let input = emergency::AutoInput {
        ldt: &src.ldt,
        flux_lm: src.flux,
        height: src.height,
        ra: ra_of(&src.ldt),
        spacing: src.spacing.as_ref().unwrap_or(&fallback),
        routes: &routes,
        room: (poly.len() >= 3).then_some(poly),
        open_area: (open && poly.len() >= 3).then_some(poly),
        open_area_place: st.open_place.get(),
        points: &points,
    };
    let mut pos: Vec<(f64, f64, f64)> = match src.which {
        EmLuminaire::Room => room_positions.iter().map(|&(x, y)| (x, y, 0.0)).collect(),
        _ if src.spacing.is_some() => emergency::auto_layout(&input, &limits, &settings),
        _ => Vec::new(),
    };
    let move_key = format!("{}:{}", src.which.key(), pos.len());
    let moves = st.moves.with(|m| m.for_key(&move_key).to_vec());
    for &(i, x, y) in &moves {
        if let Some(p) = pos.get_mut(i) {
            (p.0, p.1) = (x, y);
        }
    }
    let report = emergency::evaluate(&input.plan(&pos), &limits, &settings);
    EmResult {
        report,
        limits,
        positions: pos,
        source: src.clone(),
        move_key,
        moved: moves.len(),
        room_area,
        facings: points.iter().map(|p| p.facing.unwrap_or(90.0)).collect(),
    }
}

// ─── emergency conditions on the plan ────────────────────────────────

/// Floor illuminance of the emergency lighting over the room (0.25 m
/// cells; coarser in large rooms).
pub fn floor(res: &EmResult, poly: &[(f64, f64)]) -> FloorGrid {
    let lums: Vec<EmergencyLuminaire> = res
        .positions
        .iter()
        .map(|&(x, y, rotation)| EmergencyLuminaire {
            ldt: &res.source.ldt,
            flux_lm: res.source.flux,
            x,
            y,
            height: res.source.height,
            rotation,
            ra: None,
        })
        .collect();
    emergency::floor_grid(&lums, poly, 0.25, res.source.mf)
}

/// Band edges of the floor map (lx), with the standard's own values among
/// them, so the colours tell where a minimum is met. EN 1838: the
/// open-area minimum (0.5), the route minimum (1), then steps up to the
/// top of the Light scene's false colours (5); China: its minimums (1, 3,
/// 10) and steps between.
pub fn floor_bands(limits: &Limits) -> [f64; 6] {
    match limits.id.as_str() {
        "gb55037" | "nbr10898" => [1.0, 2.0, 3.0, 5.0, 7.5, 10.0],
        "nfpa101" => [1.1, 2.5, 5.0, 10.8, 15.0, 20.0],
        "nbcc" | "nbc-in" => [1.0, 2.0, 5.0, 10.0, 15.0, 20.0],
        "as2293" | "nz-f6" => [0.2, 0.5, 1.0, 2.0, 3.0, 5.0],
        "jp-bsl" => [0.5, 1.0, 2.0, 3.0, 5.0, 10.0],
        _ => [0.5, 1.0, 2.0, 3.0, 4.0, 5.0],
    }
}

/// Colour of each band: viridis at the band's middle on a 0–5 lx scale,
/// the same ramp as the Light scene's false colours.
const FLOOR_COLOURS: [&str; 7] = [
    "#46085c", "#46327e", "#365c8d", "#21918c", "#44bf70", "#bddf26", "#fde725",
];

fn floor_band(bands: &[f64; 6], e: f64) -> usize {
    bands.iter().filter(|&&b| e >= b).count()
}

/// The floor map as SVG for a plan at `px = ox + x·s`, `py = oy + y·s`:
/// one rectangle per run of equal band in each row.
pub fn floor_svg(g: &FloorGrid, bands: &[f64; 6], ox: f64, oy: f64, s: f64) -> String {
    let mut out = String::from(r#"<g shape-rendering="crispEdges">"#);
    let h = g.step * s;
    for j in 0..g.ny {
        let mut i = 0;
        while i < g.nx {
            let Some(e) = g.at(i, j) else {
                i += 1;
                continue;
            };
            let band = floor_band(bands, e);
            let start = i;
            while i + 1 < g.nx && g.at(i + 1, j).is_some_and(|v| floor_band(bands, v) == band) {
                i += 1;
            }
            out.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
                ox + (g.x0 + start as f64 * g.step) * s,
                oy + (g.y0 + j as f64 * g.step) * s,
                (i + 1 - start) as f64 * h + 0.4,
                h + 0.4,
                FLOOR_COLOURS[band]
            ));
            i += 1;
        }
    }
    out.push_str("</g>");
    out
}

/// Band labels of the floor map's legend.
fn band_labels(b: &[f64; 6]) -> [String; 7] {
    let f = |v: f64| {
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            format!("{v}")
        }
    };
    [
        format!("< {}", f(b[0])),
        format!("{}–{}", f(b[0]), f(b[1])),
        format!("{}–{}", f(b[1]), f(b[2])),
        format!("{}–{}", f(b[2]), f(b[3])),
        format!("{}–{}", f(b[3]), f(b[4])),
        format!("{}–{}", f(b[4]), f(b[5])),
        format!("≥ {} lx", f(b[5])),
    ]
}

/// Legend under the plan in emergency conditions.
#[component]
pub fn FloorLegend(grid: Memo<Option<FloorGrid>>, result: Memo<Option<EmResult>>) -> impl IntoView {
    let t = Memo::new(move |_| use_locale().get().zonal_designer.emergency.clone());
    move || {
        let (lo, hi) = grid.with(|g| g.as_ref().and_then(|g| g.min_max()))?;
        let (mf, room, bands) = result.with(|r| {
            r.as_ref().map(|r| {
                (
                    r.source.mf,
                    r.source.which == EmLuminaire::Room,
                    floor_bands(&r.limits),
                )
            })
        })?;
        let tt = t.get();
        Some(view! {
            <div class="zonal-em-legend">
                <div class="zonal-em-legend-bands">
                    {band_labels(&bands).into_iter().zip(FLOOR_COLOURS).enumerate().map(|(k, (label, colour))| {
                        let light = k >= 4;
                        view! {
                            <span style=format!("background: {colour}; color: {};", if light { "#111" } else { "#fff" })>{label}</span>
                        }
                    }).collect_view()}
                </div>
                <p class="zonal-sensors-hint">
                    {fill(&tt.floor_legend, &[("mf", format!("{mf:.2}")), ("min", format!("{lo:.2}")), ("max", format!("{hi:.1}"))])}
                    {(!room).then(|| format!(" {}", tt.lum_off))}
                </p>
            </div>
        })
    }
}

// ─── plan overlay ────────────────────────────────────────────────────

/// Plain-text symbol of a point of emphasis (fonts without emoji, PDF).
fn kind_letter(k: EmphasisKind) -> &'static str {
    match k {
        EmphasisKind::FireEquipment => "F",
        EmphasisKind::FirstAid => "+",
        EmphasisKind::CallPoint => "!",
        EmphasisKind::ExternallyLitSign => "S",
        EmphasisKind::Exit => "X",
        _ => "•",
    }
}

/// SVG of routes, failing points, emergency luminaires and points of
/// emphasis, for a plan drawn at `px = ox + x·s`, `py = oy + y·s`
/// (`emoji: false` for the PDF, whose font has none).
/// Safety green (ISO 3864-4 / ISO 7010 escape signs).
const SAFETY_GREEN: &str = "#00924a";

/// A polyline moved sideways by `off` (left of its direction for
/// `off > 0`), with mitred corners (limited to 3 × `off`).
fn offset_path(path: &[(f64, f64)], off: f64) -> Vec<(f64, f64)> {
    let normal = |a: (f64, f64), b: (f64, f64)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        (len > 1e-9).then(|| (-dy / len, dx / len))
    };
    // drop repeated points first
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(path.len());
    for &p in path {
        if pts
            .last()
            .is_none_or(|q: &(f64, f64)| (q.0 - p.0).hypot(q.1 - p.1) > 1e-6)
        {
            pts.push(p);
        }
    }
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    (0..n)
        .map(|i| {
            let before = (i > 0).then(|| normal(pts[i - 1], pts[i])).flatten();
            let after = (i + 1 < n).then(|| normal(pts[i], pts[i + 1])).flatten();
            let (nx, ny) = match (before, after) {
                (Some(a), Some(b)) => {
                    let (sx, sy) = (a.0 + b.0, a.1 + b.1);
                    let l = sx.hypot(sy);
                    if l < 1e-6 {
                        a
                    } else {
                        // mitre: along the bisector, 1 / cos(half the turn)
                        let (mx, my) = (sx / l, sy / l);
                        let k = (1.0 / (mx * a.0 + my * a.1).max(1.0 / 3.0)).min(3.0);
                        (mx * k, my * k)
                    }
                }
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => (0.0, 0.0),
            };
            (pts[i].0 + nx * off, pts[i].1 + ny * off)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn overlay(
    res: Option<&EmResult>,
    routes: &[RouteDraft],
    wip: &[(f64, f64)],
    ox: f64,
    oy: f64,
    s: f64,
    emoji: bool,
) -> String {
    let px = |x: f64| ox + x * s;
    let py = |y: f64| oy + y * s;
    let mut out = String::new();
    // escape routes: a safety-green band of the route's width with a
    // dashed centre line
    for r in routes {
        let left = offset_path(&r.path, 0.5 * r.width);
        let right = offset_path(&r.path, -0.5 * r.width);
        if left.len() < 2 {
            continue;
        }
        let ring: Vec<String> = left
            .iter()
            .chain(right.iter().rev())
            .map(|&(x, y)| format!("{:.1},{:.1}", px(x), py(y)))
            .collect();
        let centre: Vec<String> = r
            .path
            .iter()
            .map(|&(x, y)| format!("{:.1},{:.1}", px(x), py(y)))
            .collect();
        out.push_str(&format!(
            r##"<polygon points="{}" fill="{SAFETY_GREEN}" fill-opacity="0.3" stroke="{SAFETY_GREEN}" stroke-opacity="0.85" stroke-width="1" stroke-linejoin="miter"/><polyline points="{}" fill="none" stroke="#14532d" stroke-opacity="0.7" stroke-width="1.2" stroke-dasharray="5,4"/>"##,
            ring.join(" "),
            centre.join(" ")
        ));
    }
    if let Some(res) = res {
        // where a centre line misses the requirement: red over the band
        for rr in &res.report.routes {
            for w in rr.samples.windows(2) {
                let need = rr.point_min_lx();
                let ok = w[0].2 >= need && w[1].2 >= need;
                if ok {
                    continue;
                }
                out.push_str(&format!(
                    r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#dc2626" stroke-width="3" stroke-linecap="round"/>"##,
                    px(w[0].0),
                    py(w[0].1),
                    px(w[1].0),
                    py(w[1].1),
                ));
            }
            if !rr.pass_centre || !rr.pass_band || !rr.pass_end {
                let (x, y) = if rr.pass_centre {
                    rr.band.at_min
                } else {
                    rr.centre.at_min
                };
                out.push_str(&format!(
                    r##"<circle cx="{:.1}" cy="{:.1}" r="7" fill="none" stroke="#dc2626" stroke-width="2"/>"##,
                    px(x),
                    py(y)
                ));
            }
        }
        // open area points below the minimum
        for a in &res.report.open_areas {
            if !a.required {
                continue;
            }
            for &(x, y, e) in &a.samples {
                if e < a.required_lx {
                    out.push_str(&format!(
                        r##"<circle cx="{:.1}" cy="{:.1}" r="1.6" fill="#dc2626" fill-opacity="0.7"/>"##,
                        px(x),
                        py(y)
                    ));
                }
            }
        }
        // emergency luminaires (C0 direction marked)
        if res.source.which != EmLuminaire::Room {
            for &(x, y, r) in &res.positions {
                let (cx, cy) = (px(x), py(y));
                let a = r.to_radians();
                out.push_str(&format!(
                    r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#15803d" stroke-width="2"/><rect x="{:.1}" y="{:.1}" width="13" height="13" rx="2" fill="#fff" stroke="#15803d" stroke-width="2"/><text x="{:.1}" y="{:.1}" font-size="9" font-weight="700" text-anchor="middle" fill="#15803d">E</text>"##,
                    cx - a.cos() * 12.0,
                    cy - a.sin() * 12.0,
                    cx + a.cos() * 12.0,
                    cy + a.sin() * 12.0,
                    cx - 6.5,
                    cy - 6.5,
                    cx,
                    cy + 3.2
                ));
            }
        } else {
            for &(x, y, _) in &res.positions {
                out.push_str(&format!(
                    r##"<circle cx="{:.1}" cy="{:.1}" r="9" fill="none" stroke="#16a34a" stroke-width="2"/>"##,
                    px(x),
                    py(y)
                ));
            }
        }
        // points of emphasis
        for (k, p) in res.report.points.iter().enumerate() {
            let colour = match p.pass {
                Some(true) => "#16a34a",
                Some(false) => "#dc2626",
                None => "#64748b",
            };
            out.push_str(&format!(
                r##"<circle cx="{:.1}" cy="{:.1}" r="9" fill="#fff" stroke="{colour}" stroke-width="2"/><text x="{:.1}" y="{:.1}" font-size="11" text-anchor="middle">{}</text>"##,
                px(p.x),
                py(p.y),
                px(p.x),
                py(p.y) + 4.0,
                if emoji { kind_icon(p.kind) } else { kind_letter(p.kind) }
            ));
            if let Some(e) = p.vertical_lx {
                // into the room, along the way the equipment faces
                let f = res.facings.get(k).copied().unwrap_or(90.0).to_radians();
                let (lx, ly) = (px(p.x) + f.cos() * 24.0, py(p.y) + f.sin() * 20.0 + 4.0);
                let anchor = if f.cos() > 0.5 {
                    "start"
                } else if f.cos() < -0.5 {
                    "end"
                } else {
                    "middle"
                };
                out.push_str(&format!(
                    r##"<text x="{lx:.1}" y="{ly:.1}" font-size="10" font-weight="600" text-anchor="{anchor}" fill="{colour}" stroke="#fff" stroke-width="3" stroke-linejoin="round" paint-order="stroke">{e:.1} lx</text>"##
                ));
            }
        }
    }
    // the route being drawn
    if !wip.is_empty() {
        let pts: Vec<String> = wip
            .iter()
            .map(|&(x, y)| format!("{:.1},{:.1}", px(x), py(y)))
            .collect();
        out.push_str(&format!(
            r##"<polyline points="{}" fill="none" stroke="#2563eb" stroke-width="3" stroke-dasharray="6,4"/>"##,
            pts.join(" ")
        ));
        for &(x, y) in wip {
            out.push_str(&format!(
                r##"<circle cx="{:.1}" cy="{:.1}" r="4" fill="#2563eb"/>"##,
                px(x),
                py(y)
            ));
        }
    }
    out
}

// ─── sidebar ─────────────────────────────────────────────────────────

#[component]
pub fn EmergencySidebar(
    state: EmergencyState,
    source: Memo<Option<EmSource>>,
    /// Room floor area (m²) and the loaded luminaire's name.
    room_area: Signal<f64>,
    loaded_name: Signal<String>,
) -> impl IntoView {
    let locale = use_locale();
    let num_row = move |label: Signal<String>, value: Signal<String>, on: Callback<f64>| {
        view! {
            <div class="zonal-prop-row">
                <span class="zonal-prop-label">{move || label.get()}</span>
                <input type="number" step="any" style="width:6em"
                    prop:value=move || value.get()
                    on:change=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            on.run(v);
                        }
                    }
                />
            </div>
        }
    };
    view! {
        <div class="zonal-sidebar-section zonal-emergency">
            <h4>{move || format!("🚨 {}", locale.get().zonal_designer.emergency.title)}</h4>
            <p class="zonal-sensors-hint">{move || locale.get().zonal_designer.emergency.intro.clone()}</p>
            <label class="zonal-toggle">
                <input type="checkbox"
                    prop:checked=move || state.enabled.get()
                    on:change=move |ev| state.enabled.set(event_target_checked(&ev))
                />
                {move || format!(" {}", locale.get().zonal_designer.emergency.enable)}
            </label>
            {move || state.enabled.get().then(|| {
                let e = move || locale.get().zonal_designer.emergency.clone();
                // a minimum per kind of place (China …): a place per route
                // and for the room
                let per_place = move || state.limits().places.is_some();
                view! {
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || e().standard}</span>
                        {move || {
                            let s = e();
                            let cur = state.standard.get();
                            view! {
                                <select on:change=move |ev| state.set_standard(&event_target_value(&ev))>
                                    {[("europe", s.group_europe.clone()), ("americas", s.group_americas.clone()), ("asia", s.group_asia.clone())].into_iter().map(|(g, label)| {
                                        let cur = cur.clone();
                                        let s = s.clone();
                                        view! {
                                            <optgroup label=label>
                                                {Limits::all().into_iter().filter(|l| l.group() == g).map(|l| view! {
                                                    <option value=l.id.clone() selected=cur == l.id>{s.std_label(&l.id)}</option>
                                                }).collect_view()}
                                            </optgroup>
                                        }
                                    }).collect_view()}
                                </select>
                            }
                        }}
                    </div>
                    <p class="zonal-sensors-hint">{move || e().std_note(&state.standard.get())}</p>
                    {move || state.limits().end_of_duration.map(|end| {
                        let name = state.limits().name;
                        view! {
                            {num_row(
                                Signal::derive(move || e().end_output),
                                Signal::derive(move || format!("{:.0}", state.end_pct.get())),
                                Callback::new(move |v: f64| state.end_pct.set(v.clamp(1.0, 100.0))),
                            )}
                            <p class="zonal-sensors-hint">{move || fill(&e().end_output_hint, &[
                                ("std", name.clone()),
                                ("h", format!("{}", end.after_h)),
                                ("avg", format!("{}", end.avg_min_lx)),
                                ("min", format!("{}", end.min_lx)),
                            ])}</p>
                        }
                    })}
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || e().luminaire}</span>
                        {move || {
                            let s = e();
                            let cur = state.luminaire.get();
                            let name = loaded_name.get();
                            view! {
                                <select on:change=move |ev| {
                                    if let Some(v) = EmLuminaire::from_key(&event_target_value(&ev)) {
                                        state.luminaire.set(v);
                                        state.flux.set(None);
                                    }
                                }>
                                    {EmLuminaire::ALL.into_iter().map(|v| {
                                        let label = match v {
                                            EmLuminaire::DemoRoute => s.lum_demo_route.clone(),
                                            EmLuminaire::DemoOpen => s.lum_demo_open.clone(),
                                            EmLuminaire::Loaded => fill(&s.lum_loaded, &[("name", name.clone())]),
                                            EmLuminaire::Room => s.lum_room.clone(),
                                        };
                                        view! { <option value={v.key()} selected=cur == v>{label}</option> }
                                    }).collect_view()}
                                </select>
                            }
                        }}
                    </div>
                    {move || if state.luminaire.get() == EmLuminaire::Room {
                        num_row(
                            Signal::derive(move || e().pct),
                            Signal::derive(move || format!("{:.0}", state.pct.get())),
                            Callback::new(move |v: f64| state.pct.set(v.clamp(1.0, 100.0))),
                        ).into_any()
                    } else {
                        view! {
                            {num_row(
                                Signal::derive(move || e().flux),
                                Signal::derive(move || source.with(|s| s.as_ref().map(|s| format!("{:.0}", s.flux)).unwrap_or_default())),
                                Callback::new(move |v: f64| state.flux.set((v > 0.0).then_some(v))),
                            )}
                            {move || source.with(|s| s.as_ref().map(|s| s.own_flux)).map(|lm| view! {
                                <p class="zonal-sensors-hint">{fill(&e().flux_auto, &[("lm", format!("{lm:.0}"))])}</p>
                            })}
                        }.into_any()
                    }}
                    {num_row(
                        Signal::derive(move || e().height),
                        Signal::derive(move || source.with(|s| s.as_ref().map(|s| format!("{:.2}", s.height)).unwrap_or_default())),
                        Callback::new(move |v: f64| state.height.set((v > 0.5).then_some(v))),
                    )}
                    {num_row(
                        Signal::derive(move || e().mf),
                        Signal::derive(move || format!("{:.2}", state.mf.get())),
                        Callback::new(move |v: f64| state.mf.set(v.clamp(0.3, 1.0))),
                    )}
                    {move || source.with(|s| s.as_ref().and_then(|s| s.spacing.clone().map(|sp| (s.height, sp)))).map(|(h, sp)| view! {
                        <p class="zonal-sensors-hint">{fill(&e().spacing_note, &[
                            ("h", format!("{h:.1}")),
                            ("r", format!("{:.1}", sp.route_spacing_c0_m.max(sp.route_spacing_c90_m))),
                            ("g", format!("{:.1}", sp.open_area_grid_m)),
                        ])}</p>
                    })}

                    <h5 class="zonal-emergency-h">{move || e().routes}</h5>
                    {move || {
                        let s = e();
                        state.routes.get().into_iter().enumerate().map(|(i, r)| {
                            let len: f64 = r.path.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
                            let place = r.place;
                            let limits = state.limits();
                            view! {
                                <div class="zonal-emergency-item">
                                    <span>{fill(&s.route_item, &[
                                        ("n", (i + 1).to_string()),
                                        ("len", format!("{len:.1}")),
                                        ("w", format!("{:.1}", r.width)),
                                    ])}</span>
                                    <button class="zonal-move-btn" title=s.remove.clone()
                                        on:click=move |_| state.routes.update(|v| { if i < v.len() { v.remove(i); } })
                                    >"✕"</button>
                                </div>
                                // per place (China …): what the route is decides its minimum
                                {limits.places.clone().map(|pl| view! {
                                    <select class="zonal-emergency-place" on:change=move |ev| {
                                        if let Some(c) = PlaceCategory::from_key(&event_target_value(&ev)) {
                                            state.routes.update(|v| { if let Some(r) = v.get_mut(i) { r.place = c; } });
                                        }
                                    }>
                                        {pl.route_places.clone().into_iter().map(|c| view! {
                                            <option value=c.key() selected=c == place>{format!("{} · {} lx", place_label_in(&s, &limits.id, c), pl.required(c))}</option>
                                        }).collect_view()}
                                    </select>
                                })}
                            }
                        }).collect_view()
                    }}
                    {num_row(
                        Signal::derive(move || e().width),
                        Signal::derive(move || format!("{:.1}", state.width.get())),
                        Callback::new(move |v: f64| state.width.set(v.clamp(0.5, 20.0))),
                    )}
                    {move || if state.tool.get() == EmTool::Route {
                        view! {
                            <p class="zonal-sensors-hint">{move || e().route_hint}</p>
                            <div class="zonal-emergency-item">
                                <button class="zonal-move-btn active" on:click=move |_| state.finish_route()>{move || e().finish}</button>
                                <button class="zonal-move-btn" on:click=move |_| { state.wip.set(Vec::new()); state.tool.set(EmTool::Off); }>{move || e().cancel}</button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <button class="zonal-share-btn" on:click=move |_| { state.wip.set(Vec::new()); state.tool.set(EmTool::Route); }>{move || e().draw_route}</button>
                        }.into_any()
                    }}

                    <label class="zonal-toggle">
                        <input type="checkbox"
                            prop:checked=move || state.open_area.get()
                            on:change=move |ev| state.open_area.set(event_target_checked(&ev))
                        />
                        {move || format!(" {}", e().open_area)}
                    </label>
                    {move || if per_place() {
                        let s = e();
                        let cur = state.open_place.get();
                        let pl = state.limits().places;
                        view! {
                            <div class="zonal-prop-row">
                                <span class="zonal-prop-label">{s.room_place.clone()}</span>
                                <select on:change=move |ev| {
                                    if let Some(c) = PlaceCategory::from_key(&event_target_value(&ev)) {
                                        state.open_place.set(c);
                                    }
                                }>
                                    {pl.as_ref().map(|p| p.area_places.clone()).unwrap_or_default().into_iter().map(|c| {
                                        let lx = pl.as_ref().map(|p| p.required(c)).unwrap_or(0.0);
                                        view! { <option value=c.key() selected=c == cur>{format!("{} · {} lx", place_label_in(&s, &state.standard.get_untracked(), c), lx)}</option> }
                                    }).collect_view()}
                                </select>
                            </div>
                        }.into_any()
                    } else {
                        view! { <p class="zonal-sensors-hint">{move || {
                            let above = state.limits().anti_panic.applies_above_m2;
                            let a = format!("{:.1}", room_area.get());
                            if above > 0.0 {
                                fill(&e().open_area_hint, &[("a", a), ("limit", format!("{above}"))])
                            } else {
                                fill(&e().room_floor_hint, &[("a", a)])
                            }
                        }}</p> }.into_any()
                    }}

                    <h5 class="zonal-emergency-h">{move || e().points}</h5>
                    <div class="zonal-emergency-kinds">
                        <span class="zonal-prop-label">{move || e().place}</span>
                        {POINT_KINDS.into_iter().map(|k| view! {
                            <button class="zonal-move-btn"
                                class:active=move || state.tool.get() == EmTool::Point(k)
                                title=move || kind_label(&e(), k)
                                on:click=move |_| state.tool.set(if state.tool.get_untracked() == EmTool::Point(k) { EmTool::Off } else { EmTool::Point(k) })
                            >{kind_icon(k)}</button>
                        }).collect_view()}
                    </div>
                    {move || matches!(state.tool.get(), EmTool::Point(_)).then(|| view! {
                        <p class="zonal-sensors-hint">{move || e().point_hint}
                            " "<button class="zonal-move-btn" on:click=move |_| state.tool.set(EmTool::Off)>{move || e().done}</button>
                        </p>
                    })}
                    {move || {
                        let s = e();
                        state.points.get().into_iter().enumerate().map(|(i, p)| view! {
                            <div class="zonal-emergency-item">
                                <span>{format!("{} {}", kind_icon(p.kind), fill(&s.point_item, &[
                                    ("kind", kind_label(&s, p.kind)),
                                    ("x", format!("{:.1}", p.x)),
                                    ("y", format!("{:.1}", p.y)),
                                ]))}</span>
                                <button class="zonal-move-btn" title=s.remove.clone()
                                    on:click=move |_| state.points.update(|v| { if i < v.len() { v.remove(i); } })
                                >"✕"</button>
                            </div>
                        }).collect_view()
                    }}
                }
            })}
        </div>
    }
}

// ─── results ─────────────────────────────────────────────────────────

fn mark(ok: bool) -> &'static str {
    if ok {
        "✓"
    } else {
        "✗"
    }
}

fn ratio(r: f64) -> String {
    if r.is_finite() {
        format!("{r:.1}")
    } else {
        "∞".into()
    }
}

/// The localized words for a band of `share` of the route width.
fn band_label(s: &eulumdat_i18n::ZonalEmergency, share: f64) -> String {
    if (share - 0.5).abs() < 1e-9 {
        s.band_half.clone()
    } else if share >= 1.0 - 1e-9 {
        s.band_full.clone()
    } else {
        fill(&s.band_pct, &[("pct", format!("{:.0}", share * 100.0))])
    }
}

/// One localized row of the results table: what, value, required, pass.
pub fn check_row(
    s: &eulumdat_i18n::ZonalEmergency,
    rep: &EmergencyReport,
    limits: &Limits,
    c: &Check,
) -> (String, String, String, Option<bool>) {
    use eulumdat::emergency::{CheckKind as K, Subject};
    let num = |v: f64| format!("{}", (v * 1000.0).round() / 1000.0);
    let band = band_label(s, limits.escape_route.band_share);
    let per_place = limits.places.is_some();
    // the subject's name as the labels put it in
    let route = |i: usize| {
        rep.routes.get(i).map_or(String::new(), |r| {
            let name = fill(&s.route_name, &[("n", r.name.clone())]);
            if per_place {
                format!("{name} ({})", place_label_in(s, &limits.id, r.place))
            } else {
                name
            }
        })
    };
    let area = |i: usize| {
        rep.open_areas.get(i).map_or(String::new(), |a| {
            if per_place {
                fill(
                    &s.room_place_name,
                    &[("place", place_label_in(s, &limits.id, a.place))],
                )
            } else if limits.anti_panic.applies_above_m2 > 0.0 {
                s.open_name.clone()
            } else {
                s.room_name.clone()
            }
        })
    };
    let subject = match c.subject {
        Subject::Route(i) => route(i),
        Subject::Area(i) => area(i),
        Subject::HighRisk(i) => rep
            .high_risk
            .get(i)
            .map_or(String::new(), |a| a.name.clone()),
        Subject::Point(i) => rep.points.get(i).map_or(String::new(), |p| {
            format!("{} {}", kind_icon(p.kind), kind_label(s, p.kind))
        }),
        Subject::Luminaires => String::new(),
    };
    let after = limits.end_of_duration.as_ref().map_or(0.0, |e| e.after_h);
    let end = |t: &str| {
        fill(
            t,
            &[
                ("name", subject.clone()),
                ("h", num(after)),
                (
                    "pct",
                    format!("{:.0}", rep.settings.end_output_share * 100.0),
                ),
            ],
        )
    };
    let named = |t: &str| fill(t, &[("name", subject.clone()), ("band", band.clone())]);
    let what = match c.kind {
        K::CentreMin => named(&s.route_centre),
        // EN 1838's "central band"; other profiles name their share
        K::BandMin if !per_place && limits.escape_route.centre_min_lx > 0.0 => named(&s.route_band),
        K::BandMin => named(&s.route_band_min),
        K::RouteAverage => named(&s.route_avg),
        K::RouteRatio if limits.escape_route.ratio_over_band => named(&s.route_ratio_band),
        K::RouteRatio => named(&s.route_ratio),
        K::RouteWidth => named(&s.route_width),
        K::AreaMin => named(&s.area_min_of),
        K::AreaAverage => named(&s.area_avg_of),
        K::AreaRatio | K::HighRiskRatio => named(&s.area_ratio_of),
        K::HighRiskMin => named(&s.area_min_of),
        K::EndAverage => end(&s.end_avg),
        K::EndMin => end(&s.end_min),
        K::Vertical if limits.emphasis_horizontal => {
            fill(&s.point_horizontal, &[("kind", subject.clone())])
        }
        K::Vertical => fill(&s.point_vertical, &[("kind", subject.clone())]),
        K::Glare => s.glare.clone(),
        K::Ra => s.ra.clone(),
        K::ExitFloor => named(&s.exit_floor),
        K::LuminaireFlux => s.lum_flux_min.clone(),
        K::RouteCount | K::AreaCount => named(&s.count_of),
        K::RouteSpacing => named(&s.route_gap),
        K::AreaFlux => named(&s.area_flux),
        K::AreaSpacing => named(&s.area_spacing),
    };
    let value = match (c.kind, c.value) {
        (_, None) => s.ra_unknown.clone(),
        (K::RouteRatio | K::AreaRatio | K::HighRiskRatio, Some(v)) => ratio(v),
        (K::Glare, Some(v)) => format!("{v:.1} cd"),
        (K::Ra, Some(v)) => format!("{v:.0}"),
        (K::RouteWidth | K::RouteSpacing, Some(v)) => format!("{v:.1} m"),
        (K::RouteCount | K::AreaCount, Some(v)) => format!("{v:.0}"),
        (K::LuminaireFlux, Some(v)) => format!("{v:.0} lm"),
        (K::AreaFlux, Some(v)) => format!("{v:.1} lm/m²"),
        (K::AreaSpacing, Some(v)) => format!("{v:.1}"),
        (_, Some(v)) => format!("{v:.2} lx"),
    };
    let required = match c.kind {
        _ if c.pass.is_none() && c.limit == 0.0 && c.kind != K::Vertical => {
            s.for_information.clone()
        }
        K::AreaMin if c.pass.is_none() => fill(
            &s.not_required_above,
            &[("a", num(limits.anti_panic.applies_above_m2))],
        ),
        K::Vertical if c.pass.is_none() => s.no_requirement.clone(),
        K::RouteWidth => fill(&s.route_width_req, &[("w", num(c.limit))]),
        K::RouteRatio | K::AreaRatio | K::HighRiskRatio => format!("≤ {}", num(c.limit)),
        K::Glare => format!("≤ {:.0} cd", c.limit),
        K::Ra | K::RouteCount | K::AreaCount => format!("≥ {}", num(c.limit)),
        K::RouteSpacing => format!("≤ {} m", num(c.limit)),
        K::LuminaireFlux => format!("≥ {} lm", num(c.limit)),
        K::AreaFlux => format!("≥ {} lm/m²", num(c.limit)),
        K::AreaSpacing => format!("≤ {}", num(c.limit)),
        _ => format!("≥ {} lx", num(c.limit)),
    };
    (what, value, required, c.pass)
}

#[component]
pub fn EmergencyResults(result: Memo<Option<EmResult>>) -> impl IntoView {
    let locale = use_locale();
    move || {
        let r = result.get()?;
        let l = locale.get();
        let s = l.zonal_designer.emergency.clone();
        let limits = &r.limits;
        let rep = &r.report;
        let rows: Vec<(String, String, String, Option<bool>)> = rep
            .checks
            .iter()
            .map(|c| check_row(&s, rep, limits, c))
            .collect();
        let pass = rep.pass();
        let summary = fill(
            &s.count,
            &[
                ("n", r.positions.len().to_string()),
                ("lm", format!("{:.0}", r.source.flux)),
                ("h", format!("{:.2}", r.source.height)),
            ],
        );
        Some(view! {
            <h3 class="zonal-sensors-h">{format!("🚨 {} · {}", s.results_title, limits.name)}</h3>
            <p class=if pass && r.has_checks() { "zonal-emergency-pass" } else { "zonal-emergency-fail" }>
                {if r.has_checks() { if pass { s.pass.clone() } else { s.fail.clone() } } else { s.nothing.clone() }}
            </p>
            <p class="zonal-sensors-hint">{format!("{} · {}", r.source.name, summary)}</p>
            <table class="zonal-stats-table zonal-emergency-table">
                <tr><th>{s.col_what.clone()}</th><th>{s.col_value.clone()}</th><th>{s.col_required.clone()}</th><th></th></tr>
                {rows.into_iter().map(|(what, value, req, ok)| view! {
                    <tr>
                        <td>{what}</td>
                        <td class="val">{value}</td>
                        <td>{req}</td>
                        <td class=match ok { Some(true) => "zonal-spacing-ok", Some(false) => "zonal-spacing-warn", None => "" }>
                            {ok.map(mark).unwrap_or("")}
                        </td>
                    </tr>
                }).collect_view()}
            </table>
            <p class="zonal-sensors-hint">{s.std_note(&limits.id)}</p>
            <p class="zonal-sensors-hint">{fill(&s.method, &[("mf", format!("{:.2}", r.source.mf))])}</p>
        })
    }
}

// ─── report ──────────────────────────────────────────────────────────

fn typ_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('$', "\\$")
        .replace('@', "\\@")
        .replace('<', "\\<")
}

/// A PDF of the emergency lighting alone (Typst source): title with the
/// standard, the room, then `section` ([`typst_section`], its page break
/// dropped).
pub fn report_typst(r: &EmResult, room: &eulumdat::zonal::Room, section: &str) -> String {
    let mut t = format!(
        "#set page(paper: \"a4\", margin: 1.5cm, footer: align(center)[#text(size: 8pt, fill: gray)[Candelabra · emergency lighting · {}]])\n#set text(font: \"Helvetica\", size: 10pt)\n\n= Emergency Lighting Report\n=== {}\n\n",
        typ_text(&r.limits.name),
        typ_text(&r.limits.name)
    );
    t.push_str(&format!(
        "*Room:* {:.1} × {:.1} m, {:.1} m high, {:.1} m² floor  |  *Region:* {}\n\n",
        room.length,
        room.width,
        room.height,
        room.area(),
        typ_text(&r.limits.region)
    ));
    t.push_str(
        section
            .trim_start()
            .trim_start_matches("#pagebreak()")
            .trim_start(),
    );
    t
}

/// `"Emergency Lighting (<standard>)"` section of the interior PDF (English
/// like the rest of that report).
pub fn typst_section(
    r: &EmResult,
    plan_svg: &str,
    floor_svg: Option<(&str, &FloorGrid)>,
) -> String {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let limits = &r.limits;
    let rep = &r.report;
    let mut t = format!(
        "#pagebreak()\n\n== Emergency Lighting ({})\n\n",
        limits.name
    );
    t.push_str(&format!(
        "*Standard:* {} ({}). {}\n\n",
        typ_text(&limits.name),
        typ_text(&limits.region),
        typ_text(&limits.clauses)
    ));
    t.push_str(&format!(
        "*Emergency luminaire:* {}  |  *Luminaires:* {}  |  *Emergency flux:* {:.0} lm  |  *Mounting height:* {:.2} m  |  *Maintenance factor:* {:.2}\n\n",
        typ_text(&r.source.name),
        r.positions.len(),
        r.source.flux,
        r.source.height,
        r.source.mf
    ));
    if let Some(sp) = &r.source.spacing {
        t.push_str(&format!(
            "From the luminaire's photometry at {:.2} m: escape-route spacing up to {:.1} m (along C0), open-area grid up to {:.1} m.\n\n",
            r.source.height, sp.route_spacing_c0_m, sp.open_area_grid_m
        ));
    }
    t.push_str(&format!(
        "#align(center)[#image(bytes(\"{}\"), width: 90%)]\n\n",
        escape(plan_svg)
    ));
    t.push_str("#table(\n  columns: 4,\n  align: (left, right, right, center),\n  [*Check*], [*Value*], [*Required*], [],\n");
    let row = |t: &mut String, what: String, value: String, req: String, ok: Option<bool>| {
        t.push_str(&format!(
            "  [{}], [{}], [{}], [{}],\n",
            typ_text(&what),
            typ_text(&value),
            typ_text(&req),
            match ok {
                Some(true) => "✓",
                Some(false) => "✗",
                None => "",
            }
        ));
    };
    for c in &rep.checks {
        let [what, value, req] = rep.describe(c, limits);
        row(&mut t, what, value, req, c.pass);
    }
    t.push_str(")\n\n");
    t.push_str(&format!(
        "*Result:* {}\n\n",
        if rep.pass() {
            "all requirements met"
        } else {
            "not all requirements met"
        }
    ));
    let route_rule = if limits.escape_route.centre_min_lx > 0.0 {
        format!(
            "the centre line and the {}",
            emergency::band_words(limits.escape_route.band_share)
        )
    } else {
        format!(
            "the {}",
            emergency::band_words(limits.escape_route.band_share)
        )
    };
    let mut notes = limits.notes.join(" ");
    if limits.end_of_duration.is_some() {
        notes.push_str(&format!(
            " Output at the end of the rated duration: {:.0} % of the flux above.",
            rep.settings.end_output_share * 100.0
        ));
    }
    t.push_str(&format!(
        "_Method: direct light only, emergency flux × maintenance factor {:.2}; points every {:.1} m along routes over {}, area grid {:.2} m with a {:.1} m border; equipment at {:.1} m facing into the room. {} Not calculated: {}. Colour rendering of the emergency light: {}._\n\n",
        r.source.mf,
        rep.settings.route_step_m,
        route_rule,
        rep.settings.grid_step_m,
        limits.anti_panic.border_m,
        rep.settings.emphasis_height_m,
        typ_text(&notes),
        typ_text(&limits.not_calculated()),
        match rep.ra_unknown.is_empty() {
            _ if !rep.ra_below_limit.is_empty() => format!("below Ra {}", limits.min_ra),
            true if !r.positions.is_empty() && limits.checks_ra() => format!("Ra ≥ {}", limits.min_ra),
            _ => "not checked or unknown".to_string(),
        }
    ));
    if let Some((svg, g)) = floor_svg {
        let (lo, hi) = g.min_max().unwrap_or((0.0, 0.0));
        let bands: Vec<String> = band_labels(&floor_bands(&r.limits))
            .into_iter()
            .zip(FLOOR_COLOURS)
            .map(|(l, c)| {
                format!(
                    "#box(fill: rgb(\"{c}\"), inset: 3pt)[#text(fill: {}, size: 8pt)[{}]]",
                    if c == "#bddf26" || c == "#fde725" || c == "#44bf70" {
                        "black"
                    } else {
                        "white"
                    },
                    typ_text(&l)
                )
            })
            .collect();
        t.push_str(&format!(
            "=== Floor in emergency mode\n\nNormal lighting off; direct light of the emergency luminaires on the floor, maintenance factor {:.2}: min {:.2} lx, max {:.1} lx.\n\n#align(center)[#image(bytes(\"{}\"), width: 90%)]\n\n#align(center)[{}]\n\n",
            r.source.mf,
            lo,
            hi,
            escape(svg),
            bands.join(" ")
        ));
    }
    t
}
