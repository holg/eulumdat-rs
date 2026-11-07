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
    self, EmergencyLuminaire, EmergencyReport, EmphasisKind, EmphasisPoint, EscapeRoute, FloorGrid,
    Limits, Settings, SpacingRow,
};
use eulumdat::Eulumdat;
use leptos::prelude::*;
use std::sync::{Arc, OnceLock};

const DEMO_ROUTE: &str =
    include_str!("../../../eulumdat/tests/fixtures/emergency/demo_em_route.ldt");
const DEMO_OPEN: &str = include_str!("../../../eulumdat/tests/fixtures/emergency/demo_em_open.ldt");

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
}

/// A point of emphasis as placed (facing into the room, degrees).
#[derive(Clone, Debug, PartialEq)]
pub struct PointDraft {
    pub kind: EmphasisKind,
    pub x: f64,
    pub y: f64,
    pub facing: f64,
}

pub const POINT_KINDS: [EmphasisKind; 5] = [
    EmphasisKind::FireEquipment,
    EmphasisKind::FirstAid,
    EmphasisKind::CallPoint,
    EmphasisKind::ExternallyLitSign,
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
                        let (pts, w) = r.split_once('~').unwrap_or((r, "2"));
                        let path = parse_pairs(pts);
                        (path.len() >= 2).then(|| RouteDraft {
                            path,
                            width: w.parse().unwrap_or(2.0f64).clamp(0.5, 20.0),
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
        }
    }

    /// Link keys (tracked), `None` when emergency planning is off.
    pub fn link_query(&self) -> Option<String> {
        if !self.enabled.get() {
            return None;
        }
        let mut parts = vec![
            "em=1".to_string(),
            format!("eml={}", self.luminaire.get().key()),
        ];
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
                    format!("{}~{:.2}", pts.join(";"), r.width)
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
        Some(parts.join("&"))
    }

    /// End the route being drawn (kept when it has two points or more).
    pub fn finish_route(&self) {
        let wip = self.wip.get_untracked();
        if wip.len() >= 2 {
            let width = self.width.get_untracked();
            self.routes
                .update(|r| r.push(RouteDraft { path: wip, width }));
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
        ..Default::default()
    };
    let spacing = (which != EmLuminaire::Room).then(|| {
        emergency::spacing_table(&ldt, flux, &[height], width, &Limits::en1838(), &settings)
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
        })
        .collect();
    let open = st.open_area.get();
    let limits = Limits::en1838();
    let settings = Settings {
        maintenance_factor: src.mf,
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

/// Band edges of the floor map (lx): the open-area minimum (0.5), the
/// route minimum (1), then steps up to the top of the Light scene's false
/// colours (5).
pub const FLOOR_BANDS: [f64; 6] = [0.5, 1.0, 2.0, 3.0, 4.0, 5.0];

/// Colour of each band: viridis at the band's middle on a 0–5 lx scale,
/// the same ramp as the Light scene's false colours.
const FLOOR_COLOURS: [&str; 7] = [
    "#46085c", "#46327e", "#365c8d", "#21918c", "#44bf70", "#bddf26", "#fde725",
];

fn floor_band(e: f64) -> usize {
    FLOOR_BANDS.iter().filter(|&&b| e >= b).count()
}

/// The floor map as SVG for a plan at `px = ox + x·s`, `py = oy + y·s`:
/// one rectangle per run of equal band in each row.
pub fn floor_svg(g: &FloorGrid, ox: f64, oy: f64, s: f64) -> String {
    let mut out = String::from(r#"<g shape-rendering="crispEdges">"#);
    let h = g.step * s;
    for j in 0..g.ny {
        let mut i = 0;
        while i < g.nx {
            let Some(e) = g.at(i, j) else {
                i += 1;
                continue;
            };
            let band = floor_band(e);
            let start = i;
            while i + 1 < g.nx && g.at(i + 1, j).is_some_and(|v| floor_band(v) == band) {
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
fn band_labels() -> [String; 7] {
    let f = |v: f64| {
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            format!("{v}")
        }
    };
    let b = FLOOR_BANDS;
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
        let (mf, room) = result.with(|r| {
            r.as_ref()
                .map(|r| (r.source.mf, r.source.which == EmLuminaire::Room))
        })?;
        let tt = t.get();
        Some(view! {
            <div class="zonal-em-legend">
                <div class="zonal-em-legend-bands">
                    {band_labels().into_iter().zip(FLOOR_COLOURS).enumerate().map(|(k, (label, colour))| {
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
    let limits = Limits::en1838();
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
                let ok = w[0].2 >= limits.escape_route.centre_min_lx
                    && w[1].2 >= limits.escape_route.centre_min_lx;
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
            if !rr.pass_centre {
                let (x, y) = rr.centre.at_min;
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
                view! {
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
                    <p class="zonal-sensors-hint">{move || fill(&e().open_area_hint, &[("a", format!("{:.1}", room_area.get()))])}</p>

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

#[component]
pub fn EmergencyResults(result: Memo<Option<EmResult>>) -> impl IntoView {
    let locale = use_locale();
    move || {
        let r = result.get()?;
        let l = locale.get();
        let s = l.zonal_designer.emergency.clone();
        let limits = Limits::en1838();
        let rep = &r.report;
        let mut rows: Vec<(String, String, String, Option<bool>)> = Vec::new();
        for x in &rep.routes {
            rows.push((
                fill(&s.route_centre, &[("name", x.name.clone())]),
                format!("{:.2} lx", x.centre.min_lx),
                format!("≥ {} lx", limits.escape_route.centre_min_lx),
                Some(x.pass_centre),
            ));
            rows.push((
                fill(&s.route_band, &[("name", x.name.clone())]),
                format!("{:.2} lx", x.band.min_lx),
                format!("≥ {} lx", limits.escape_route.band_min_lx),
                Some(x.pass_band),
            ));
            rows.push((
                fill(&s.route_ratio, &[("name", x.name.clone())]),
                ratio(x.centre.ratio),
                format!("≤ {}", limits.escape_route.max_ratio),
                Some(x.pass_ratio),
            ));
        }
        for a in &rep.open_areas {
            if a.required {
                rows.push((
                    s.open_min.clone(),
                    format!("{:.2} lx", a.stats.min_lx),
                    format!("≥ {} lx", a.required_lx),
                    Some(a.pass_min),
                ));
                rows.push((
                    s.open_ratio.clone(),
                    ratio(a.stats.ratio),
                    format!("≤ {}", a.max_ratio),
                    Some(a.pass_ratio),
                ));
            } else {
                rows.push((
                    s.open_min.clone(),
                    format!("{:.2} lx", a.stats.min_lx),
                    s.not_required.clone(),
                    None,
                ));
            }
        }
        for p in &rep.points {
            if let (Some(e), Some(ok)) = (p.vertical_lx, p.pass) {
                rows.push((
                    format!(
                        "{} {}",
                        kind_icon(p.kind),
                        fill(&s.point_vertical, &[("kind", kind_label(&s, p.kind))])
                    ),
                    format!("{e:.2} lx"),
                    format!("≥ {} lx", limits.emphasis_min_vertical_lx),
                    Some(ok),
                ));
            }
        }
        if let Some(g) = rep
            .glare
            .iter()
            .max_by(|a, b| (a.i_max_cd / a.limit_cd).total_cmp(&(b.i_max_cd / b.limit_cd)))
        {
            rows.push((
                s.glare.clone(),
                format!("{:.1} cd", g.i_max_cd),
                format!("≤ {:.0} cd", g.limit_cd),
                Some(rep.glare.iter().all(|g| g.pass)),
            ));
        }
        let ra = if !rep.ra_below_limit.is_empty() {
            Some(false)
        } else if rep.ra_unknown.is_empty() && !rep.glare.is_empty() {
            Some(true)
        } else {
            None
        };
        rows.push((
            s.ra.clone(),
            if ra.is_none() {
                s.ra_unknown.clone()
            } else {
                String::new()
            },
            format!("≥ {}", limits.min_ra),
            ra,
        ));
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
            <h3 class="zonal-sensors-h">{format!("🚨 {}", s.results_title)}</h3>
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

/// "Emergency Lighting (EN 1838)" section of the interior PDF (English like
/// the rest of that report).
pub fn typst_section(
    r: &EmResult,
    plan_svg: &str,
    floor_svg: Option<(&str, &FloorGrid)>,
) -> String {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let limits = Limits::en1838();
    let rep = &r.report;
    let mut t = String::from("#pagebreak()\n\n== Emergency Lighting (EN 1838)\n\n");
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
    for x in &rep.routes {
        row(
            &mut t,
            format!("Route {}: centre line min", x.name),
            format!("{:.2} lx", x.centre.min_lx),
            format!("≥ {} lx", limits.escape_route.centre_min_lx),
            Some(x.pass_centre),
        );
        row(
            &mut t,
            format!("Route {}: central band min", x.name),
            format!("{:.2} lx", x.band.min_lx),
            format!("≥ {} lx", limits.escape_route.band_min_lx),
            Some(x.pass_band),
        );
        row(
            &mut t,
            format!("Route {}: Emax / Emin", x.name),
            ratio(x.centre.ratio),
            format!("≤ {}", limits.escape_route.max_ratio),
            Some(x.pass_ratio),
        );
    }
    for a in &rep.open_areas {
        if a.required {
            row(
                &mut t,
                format!("Open area ({:.1} m²) min", a.area_m2),
                format!("{:.2} lx", a.stats.min_lx),
                format!("≥ {} lx", a.required_lx),
                Some(a.pass_min),
            );
            row(
                &mut t,
                "Open area Emax / Emin".into(),
                ratio(a.stats.ratio),
                format!("≤ {}", a.max_ratio),
                Some(a.pass_ratio),
            );
        } else {
            row(
                &mut t,
                format!("Open area ({:.1} m²)", a.area_m2),
                format!("{:.2} lx", a.stats.min_lx),
                "not required (≤ 60 m²)".into(),
                None,
            );
        }
    }
    for p in &rep.points {
        if let (Some(e), Some(ok)) = (p.vertical_lx, p.pass) {
            row(
                &mut t,
                format!("{:?} at ({:.1}, {:.1}): vertical", p.kind, p.x, p.y),
                format!("{e:.2} lx"),
                format!("≥ {} lx", limits.emphasis_min_vertical_lx),
                Some(ok),
            );
        }
    }
    if let Some(g) = rep
        .glare
        .iter()
        .max_by(|a, b| (a.i_max_cd / a.limit_cd).total_cmp(&(b.i_max_cd / b.limit_cd)))
    {
        row(
            &mut t,
            "Glare, I max 60°–90°".into(),
            format!("{:.1} cd", g.i_max_cd),
            format!("≤ {:.0} cd", g.limit_cd),
            Some(rep.glare.iter().all(|g| g.pass)),
        );
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
    t.push_str(&format!(
        "_Method: direct light only (EN 1838), emergency flux at the end of the rated duration × maintenance factor {:.2}; points every {:.1} m along routes, open-area grid {:.2} m with a {:.1} m border; equipment at {:.1} m facing into the room. Response times ({} s / {} s) and the rated duration are not calculated. Colour rendering of the emergency light: {}._\n\n",
        r.source.mf,
        rep.settings.route_step_m,
        rep.settings.grid_step_m,
        limits.anti_panic.border_m,
        rep.settings.emphasis_height_m,
        limits.timing.half_s,
        limits.timing.full_s,
        if !rep.ra_below_limit.is_empty() {
            "below Ra 40".to_string()
        } else if rep.ra_unknown.is_empty() {
            format!("Ra ≥ {}", limits.min_ra)
        } else {
            "unknown".to_string()
        }
    ));
    if let Some((svg, g)) = floor_svg {
        let (lo, hi) = g.min_max().unwrap_or((0.0, 0.0));
        let bands: Vec<String> = band_labels()
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
