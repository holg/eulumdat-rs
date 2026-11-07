//! Presence sensors in the interior designer (and motion detectors in the
//! outdoor area designer): place detectors, show their detection areas and
//! the gaps on the plan, and (indoors) estimate the energy saved with
//! presence control (EN 15193-1 Annex E). Sensors can be dragged on the
//! plan ([`SensorPlanState::moves`]).
//!
//! The numbers come from `eulumdat::sensor::coverage` and
//! `eulumdat::sensor::en15193`; this module holds the UI state (also the
//! share-link keys `sensor`, `splace`, `snx`, `sny`, `smargin`, `smotion`,
//! `splan`, `sctl`, `sspace`, `shold`, `shours`, `smove`), the sidebar
//! section, the result tables and the PDF section.

use std::sync::Arc;

use eulumdat::sensor::coverage::{
    auto_grid_oriented_in, coverage_in, grid_positions, overlay_svg, rotated, square_grid,
    Coverage, CoverageRoom, PlaneAreas, SensorPosition, DEFAULT_WALL_MARGIN,
};
use eulumdat::sensor::en15193::{
    annual_kwh, assess, FoAssessment, FoInput, OccupancyControl, SpaceType, MAX_GROUP_M2,
    MIN_COVERAGE,
};
use eulumdat::sensor::{Motion, TRACE_COLORS};
use eulumdat::zonal::RoomPreset;
use leptos::prelude::*;

use super::plan_drag::Moves;
use super::sensor_view::{add_demo, demo_sensors, SensorData, SensorFile, SensorFiles, SensorUi};
use crate::i18n::use_locale;

/// Most sensors the automatic placement tries.
const MAX_AUTO_SENSORS: usize = 100;
/// Default annual operating hours (an assumption the planner can change).
const DEFAULT_HOURS: f64 = 2500.0;

/// How the sensors are placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Fewest sensors on a regular grid that detect ≥ 95 %.
    Auto,
    /// `nx` × `ny` grid.
    Grid,
    /// One sensor at every luminaire (integrated sensors).
    Luminaires,
}

impl Placement {
    const ALL: [Placement; 3] = [Placement::Auto, Placement::Grid, Placement::Luminaires];

    fn key(self) -> &'static str {
        match self {
            Placement::Auto => "auto",
            Placement::Grid => "grid",
            Placement::Luminaires => "lum",
        }
    }

    fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.key() == k)
    }
}

fn motion_key(m: Motion) -> &'static str {
    match m {
        Motion::Tangential => "tangential",
        Motion::Radial => "radial",
        Motion::Presence => "presence",
    }
}

fn motion_from_key(k: &str) -> Option<Motion> {
    Motion::ALL.into_iter().find(|m| motion_key(*m) == k)
}

pub fn motion_label(l: &eulumdat_i18n::UiSensor, m: Motion) -> String {
    match m {
        Motion::Tangential => l.tangential.clone(),
        Motion::Radial => l.radial.clone(),
        Motion::Presence => l.presence.clone(),
    }
}

/// The EN 15193-1 space type that fits a room preset.
pub fn space_for_preset(p: RoomPreset) -> SpaceType {
    match p {
        RoomPreset::OpenOffice => SpaceType::OpenPlan,
        RoomPreset::PrivateOffice => SpaceType::CellularOffice1,
        RoomPreset::Classroom => SpaceType::Classroom,
        RoomPreset::Corridor => SpaceType::OfficeCorridor,
        RoomPreset::ConferenceRoom => SpaceType::ConferenceRoom,
        RoomPreset::Restroom => SpaceType::Toilet,
        RoomPreset::ParkingGarage => SpaceType::CarParkPublic,
        RoomPreset::Warehouse | RoomPreset::Retail | RoomPreset::Workshop => {
            SpaceType::OfficeOverall
        }
    }
}

/// The motion type to plan for in a room preset: walking where people pass
/// through, seated presence where they sit or stand still.
pub fn motion_for_preset(p: RoomPreset) -> Motion {
    match p {
        RoomPreset::Corridor
        | RoomPreset::Warehouse
        | RoomPreset::Retail
        | RoomPreset::ParkingGarage => Motion::Tangential,
        _ => Motion::Presence,
    }
}

/// Fill `{key}` placeholders.
pub fn fill(s: &str, vals: &[(&str, String)]) -> String {
    let mut out = s.to_string();
    for (k, v) in vals {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Requests to the designer shown (Interior or outdoor area; context,
/// provided by `App`): use this sensor (name in the app's sensor list),
/// e.g. picked in the header's project menu.
#[derive(Clone, Copy)]
pub struct InteriorRequests {
    pub pick: RwSignal<Option<String>>,
}

/// UI state of the sensor section (all signals, `Copy`).
#[derive(Clone, Copy)]
pub struct SensorPlanState {
    pub files: RwSignal<Vec<SensorFile>>,
    /// Name of the chosen sensor file (`None`: no sensors).
    pub selected: RwSignal<Option<String>>,
    /// Until the user picks a sensor here (or a link names one), the
    /// designer uses the sensor shown on the Sensors tab, else the newest
    /// ([`Self::current`]).
    pub follow: RwSignal<bool>,
    /// The Sensors tab's state (which sensor it shows).
    pub ui: Option<SensorUi>,
    pub placement: RwSignal<Placement>,
    pub nx: RwSignal<usize>,
    pub ny: RwSignal<usize>,
    pub margin: RwSignal<f64>,
    /// Motion type the placement and the gap shading are for.
    pub motion: RwSignal<Motion>,
    /// SensXML: use the idealised planning shapes.
    pub planning: RwSignal<bool>,
    /// Draw the footprints of every motion type, not only the planned one.
    pub show_all: RwSignal<bool>,
    /// Sensor C0 direction for grid / luminaire placement: 0° = along the
    /// room length, 90° = along the width (automatic placement tries both).
    pub rotation: RwSignal<f64>,
    pub control: RwSignal<OccupancyControl>,
    pub space: RwSignal<SpaceType>,
    pub hold_ok: RwSignal<bool>,
    pub hours: RwSignal<f64>,
    /// Sensors moved by hand on the plan (for the layout `Placed::key`).
    pub moves: RwSignal<Moves>,
    /// Outdoor area designer: detection on the ground, no walls, no
    /// energy estimate.
    pub outdoor: bool,
}

impl SensorPlanState {
    /// State from the share link (`get` reads a key of the `#interior?` hash).
    pub fn from_link(get: impl Fn(&str) -> Option<String>, preset: RoomPreset) -> Self {
        Self::build(
            get,
            false,
            motion_for_preset(preset),
            space_for_preset(preset),
        )
    }

    /// State for the outdoor area designer (link keys of its own hash):
    /// one sensor per pole, planned for walking across, no edge margin.
    pub fn outdoor_from_link(get: impl Fn(&str) -> Option<String>) -> Self {
        Self::build(get, true, Motion::Tangential, SpaceType::ALL[0])
    }

    fn build(
        get: impl Fn(&str) -> Option<String>,
        outdoor: bool,
        motion: Motion,
        space: SpaceType,
    ) -> Self {
        let files = use_context::<SensorFiles>()
            .map(|f| f.0)
            .unwrap_or_else(|| RwSignal::new(Vec::new()));
        // Only the demo sensors can travel in a link.
        let selected = get("sensor")
            .and_then(|v| v.strip_prefix("demo")?.parse::<usize>().ok())
            .and_then(|i| demo_sensors().get(i).map(|d| d.name.clone()));
        if selected.is_some() {
            add_demo(files);
        }
        let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
        // `sensor=none`: no sensors (e.g. a link about something else)
        let none = get("sensor").is_some_and(|v| v == "none");
        let follow = RwSignal::new(selected.is_none() && !none);
        let state = Self {
            files,
            follow,
            ui: use_context::<SensorUi>(),
            selected: RwSignal::new(selected),
            placement: RwSignal::new(
                get("splace")
                    .and_then(|v| Placement::from_key(&v))
                    .unwrap_or(if outdoor {
                        Placement::Luminaires
                    } else {
                        Placement::Auto
                    }),
            ),
            nx: RwSignal::new(num("snx").map(|v| v as usize).unwrap_or(2).clamp(1, 30)),
            ny: RwSignal::new(num("sny").map(|v| v as usize).unwrap_or(2).clamp(1, 30)),
            margin: RwSignal::new(if outdoor {
                0.0
            } else {
                num("smargin")
                    .unwrap_or(DEFAULT_WALL_MARGIN)
                    .clamp(0.0, 5.0)
            }),
            motion: RwSignal::new(
                get("smotion")
                    .and_then(|v| motion_from_key(&v))
                    .unwrap_or(motion),
            ),
            planning: RwSignal::new(get("splan").is_some_and(|v| v == "1")),
            show_all: RwSignal::new(get("sall").is_some_and(|v| v == "1")),
            rotation: RwSignal::new(if get("srot").is_some_and(|v| v == "90") {
                90.0
            } else {
                0.0
            }),
            control: RwSignal::new(
                get("sctl")
                    .and_then(|v| OccupancyControl::from_key(&v))
                    .unwrap_or(OccupancyControl::AutoOnAutoOff),
            ),
            space: RwSignal::new(
                get("sspace")
                    .and_then(|v| SpaceType::from_key(&v))
                    .unwrap_or(space),
            ),
            hold_ok: RwSignal::new(get("shold").map(|v| v != "0").unwrap_or(true)),
            hours: RwSignal::new(num("shours").unwrap_or(DEFAULT_HOURS).clamp(0.0, 8760.0)),
            moves: RwSignal::new(get("smove").map(|v| Moves::decode(&v)).unwrap_or_default()),
            outdoor,
        };
        if let Some(req) = use_context::<InteriorRequests>() {
            Effect::new(move |_| {
                if let Some(name) = req.pick.get() {
                    req.pick.set(None);
                    state.follow.set(false);
                    state.selected.set(Some(name));
                }
            });
        }
        state
    }

    /// The sensor the designer uses (tracked): the user's pick, or while
    /// following, the one shown on the Sensors tab, else the newest. A
    /// sensor opened anywhere in the app shows up on the plan at once.
    pub fn current(&self) -> Option<String> {
        if !self.follow.get() {
            return self.selected.get();
        }
        self.files.with(|v| {
            let shown = self
                .ui
                .filter(|u| !u.use_loaded.get())
                .and_then(|u| v.get(u.selected.get()));
            shown.or(v.first()).map(|f| f.name.clone())
        })
    }

    /// Link keys (tracked), `None` when no sensor is chosen (`sensor=none`
    /// when the user chose none, so the link keeps the choice).
    pub fn link_query(&self) -> Option<String> {
        let Some(name) = self.current() else {
            return (!self.follow.get()).then(|| "sensor=none".to_string());
        };
        let mut parts = Vec::new();
        if let Some(i) = demo_sensors().iter().position(|d| d.name == name) {
            parts.push(format!("sensor=demo{i}"));
        }
        parts.push(format!("splace={}", self.placement.get().key()));
        if self.placement.get() == Placement::Grid {
            parts.push(format!("snx={}&sny={}", self.nx.get(), self.ny.get()));
        }
        if self.placement.get() != Placement::Auto && self.rotation.get() != 0.0 {
            parts.push("srot=90".into());
        }
        if !self.outdoor {
            parts.push(format!("smargin={:.2}", self.margin.get()));
        }
        parts.push(format!("smotion={}", motion_key(self.motion.get())));
        if self.planning.get() {
            parts.push("splan=1".into());
        }
        if self.show_all.get() {
            parts.push("sall=1".into());
        }
        if !self.outdoor {
            parts.push(format!("sctl={}", self.control.get().key()));
            parts.push(format!("sspace={}", self.space.get().key()));
            if !self.hold_ok.get() {
                parts.push("shold=0".into());
            }
            parts.push(format!("shours={:.0}", self.hours.get()));
        }
        if let Some(m) = self.moves.with(|m| m.encode()) {
            parts.push(format!("smove={m}"));
        }
        Some(parts.join("&"))
    }
}

/// The chosen sensor as the core type, compared by pointer.
#[derive(Clone)]
pub struct CoreSensor(pub Arc<eulumdat::sensor::SensorFile>);

impl PartialEq for CoreSensor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Detection areas cut for the room's planes, compared by pointer.
#[derive(Clone)]
pub struct PlaneCut(pub Arc<PlaneAreas>);

impl PartialEq for PlaneCut {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Cut the chosen sensor at the mounting height for the floor and the
/// workplane (the slow step for a sensor LDT; it does not depend on the
/// room size, the margin or the layout).
pub fn cut(state: &SensorPlanState, sensor: &CoreSensor, mount: f64, workplane: f64) -> PlaneCut {
    let room = CoverageRoom {
        length: 1.0,
        width: 1.0,
        mount_height: mount,
        walk_plane: 0.0,
        presence_plane: workplane,
        margin: 0.0,
    };
    PlaneCut(Arc::new(PlaneAreas::new(
        sensor.0.as_ref(),
        &room,
        state.planning.get(),
    )))
}

/// What the automatic placement found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoOutcome {
    /// `nx` × `ny` sensors turned by the rotation (degrees).
    Found(usize, usize, f64),
    /// No grid up to the limit reaches the target; this many are shown.
    Failed(usize),
}

/// A computed sensor layout.
#[derive(Clone)]
pub struct SensorPlan {
    pub name: String,
    pub cov: Coverage,
    /// Motion type planned for (has data in the file).
    pub motion: Motion,
    /// Motion types in the file.
    pub motions: Vec<Motion>,
    pub auto: Option<AutoOutcome>,
    pub is_sensxml: bool,
    pub warnings: Vec<String>,
    /// Layout key of the hand moves ([`Moves`]) and how many are applied.
    pub move_key: String,
    pub moved: usize,
}

impl PartialEq for SensorPlan {
    /// Recomputed means changed (the memo inputs decide).
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl SensorPlan {
    /// Coverage of the planned motion type (0–1).
    pub fn planned_coverage(&self) -> f64 {
        self.cov
            .get(self.motion)
            .map(|m| m.covered_fraction)
            .unwrap_or(0.0)
    }

    /// Motion types drawn on the plan.
    pub fn shown(&self, all: bool) -> Vec<Motion> {
        if all {
            self.motions.clone()
        } else {
            vec![self.motion]
        }
    }

    /// SVG elements for `ZonalSvg::room_plan_with_overlay`.
    pub fn overlay(&self, ox: f64, oy: f64, scale: f64, all: bool) -> String {
        overlay_svg(
            &self.cov,
            ox,
            oy,
            scale,
            &self.shown(all),
            Some(self.motion),
        )
    }
}

/// EN 15193-1 estimate for a plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorEnergy {
    pub fo: FoAssessment,
    pub power_w: f64,
    pub hours: f64,
    pub kwh_manual: f64,
    pub kwh: f64,
    pub m2_per_sensor: f64,
    pub coverage: f64,
    /// The chosen control uses presence detection (conditions apply).
    pub uses_detection: bool,
}

impl SensorEnergy {
    pub fn saving_pct(&self) -> f64 {
        if self.kwh_manual > 0.0 {
            (1.0 - self.kwh / self.kwh_manual) * 100.0
        } else {
            0.0
        }
    }
}

/// Where the sensors go before any was moved by hand.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub positions: Vec<SensorPosition>,
    /// Motion type planned for (has data in the file).
    pub motion: Motion,
    /// Motion types in the file.
    pub motions: Vec<Motion>,
    pub auto: Option<AutoOutcome>,
    /// The layout the hand moves belong to ([`Moves::key`]).
    pub key: String,
}

/// Place the sensors (`areas` from [`cut`]). `inside`: the area's outline
/// (always true for a room); `luminaires`: positions for "one per
/// luminaire" (only read for that placement).
pub fn place(
    state: &SensorPlanState,
    areas: &PlaneAreas,
    room: &CoverageRoom,
    inside: &dyn Fn(f64, f64) -> bool,
    luminaires: impl FnOnce() -> Vec<(f64, f64)>,
) -> Placed {
    let motions = areas.motions();
    let wanted = state.motion.get();
    let motion = if motions.contains(&wanted) {
        wanted
    } else {
        motions.first().copied().unwrap_or(Motion::Tangential)
    };
    let placement = state.placement.get();
    let grid = |nx, ny| grid_positions(room.length, room.width, nx, ny);
    let (positions, auto): (Vec<SensorPosition>, Option<AutoOutcome>) = match placement {
        Placement::Grid => (
            rotated(grid(state.nx.get(), state.ny.get()), state.rotation.get()),
            None,
        ),
        Placement::Luminaires => (
            luminaires()
                .into_iter()
                .map(|(x, y)| SensorPosition {
                    x,
                    y,
                    rotation_deg: state.rotation.get(),
                })
                .collect(),
            None,
        ),
        Placement::Auto => {
            let detected = areas.get(motion).is_some();
            match auto_grid_oriented_in(
                areas,
                room,
                inside,
                motion,
                MIN_COVERAGE,
                MAX_AUTO_SENSORS,
                &[0.0, 90.0],
            ) {
                Some((nx, ny, rot)) => (
                    rotated(grid(nx, ny), rot),
                    Some(AutoOutcome::Found(nx, ny, rot)),
                ),
                // Nothing detected at this height: one sensor shows why.
                None if !detected => (grid(1, 1), None),
                None => {
                    let (nx, ny) = square_grid(room.length, room.width, MAX_AUTO_SENSORS);
                    (grid(nx, ny), Some(AutoOutcome::Failed(nx * ny)))
                }
            }
        }
    };
    // A grid over the bounds of a shaped room / area: sensors off the
    // floor see nothing useful.
    let positions = if placement == Placement::Luminaires {
        positions
    } else {
        let inside: Vec<SensorPosition> = positions
            .iter()
            .copied()
            .filter(|s| inside(s.x, s.y))
            .collect();
        if inside.is_empty() {
            positions
        } else {
            inside
        }
    };
    let key = format!("{}:{}", placement.key(), positions.len());
    Placed {
        positions,
        motion,
        motions,
        auto,
        key,
    }
}

/// The placed sensors with the hand moves applied, and their coverage.
pub fn finish(
    state: &SensorPlanState,
    placed: &Placed,
    sensor: &CoreSensor,
    areas: &PlaneAreas,
    room: &CoverageRoom,
    inside: &dyn Fn(f64, f64) -> bool,
    name: String,
) -> SensorPlan {
    let f = sensor.0.as_ref();
    let mut positions = placed.positions.clone();
    let moved = state.moves.with(|m| {
        let list = m.for_key(&placed.key);
        for &(i, x, y) in list {
            if let Some(p) = positions.get_mut(i) {
                (p.x, p.y) = (x, y);
            }
        }
        list.iter().filter(|e| e.0 < positions.len()).count()
    });
    let cov = coverage_in(areas, room, &positions, inside);
    SensorPlan {
        name,
        cov,
        motion: placed.motion,
        motions: placed.motions.clone(),
        auto: placed.auto,
        is_sensxml: f.is_sensxml(),
        warnings: f.warnings().iter().map(|w| w.message()).collect(),
        move_key: placed.key.clone(),
        moved,
    }
}

/// The EN 15193-1 estimate (one control group per sensor).
pub fn compute_energy(state: &SensorPlanState, plan: &SensorPlan, power_w: f64) -> SensorEnergy {
    let m2 = plan.cov.m2_per_sensor();
    let coverage = plan.planned_coverage();
    let fo = assess(&FoInput {
        control: state.control.get(),
        space: state.space.get(),
        hold_time_ok: state.hold_ok.get(),
        group_m2: m2,
        m2_per_sensor: m2,
        coverage,
    });
    let hours = state.hours.get();
    SensorEnergy {
        fo,
        power_w,
        hours,
        kwh_manual: annual_kwh(power_w, hours, fo.f_o_manual),
        kwh: annual_kwh(power_w, hours, fo.f_o),
        m2_per_sensor: m2,
        coverage,
        uses_detection: state.control.get().uses_detection(),
    }
}

/// Sidebar section: sensor, placement, planes, energy inputs.
/// `height`: outdoors the sensor height is an input (indoors the sensors
/// sit in the ceiling).
#[component]
pub fn SensorSidebar(
    state: SensorPlanState,
    plan: Memo<Option<SensorPlan>>,
    #[prop(optional)] height: Option<RwSignal<f64>>,
) -> impl IntoView {
    let locale = use_locale();
    let files = state.files;
    let outdoor = state.outdoor;
    let num_input = |sig: RwSignal<usize>| {
        view! {
            <input type="number" min="1" max="30" step="1" style="width:4em"
                prop:value=move || sig.get().to_string()
                on:change=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                        sig.set(v.clamp(1, 30));
                    }
                }
            />
        }
    };

    view! {
        <div class="zonal-sidebar-section zonal-sensors">
            <h4>{move || locale.get().zonal_designer.sensors.title.clone()}</h4>
            <p class="zonal-sensors-hint">{move || {
                let s = &locale.get().zonal_designer.sensors;
                if outdoor { s.outdoor_intro.clone() } else { s.intro.clone() }
            }}</p>
            <div class="zonal-prop-row">
                <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.sensor.clone()}</span>
                // Rebuilt whole with the choice preselected: options patched
                // in place can show a stale entry when the list changes.
                {move || {
                    let names: Vec<String> = files.with(|f| f.iter().map(|f| f.name.clone()).collect());
                    let current = state.current();
                    let none = locale.get().zonal_designer.sensors.none.clone();
                    view! {
                        <select class="zonal-sensors-select" on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let name = v.parse::<usize>().ok().and_then(|i| files.with(|f| f.get(i).map(|f| f.name.clone())));
                            state.follow.set(false);
                            state.selected.set(name);
                        }>
                            <option value="none" selected=current.is_none()>{none}</option>
                            {names.into_iter().enumerate().map(|(i, name)| {
                                let sel = current.as_deref() == Some(name.as_str());
                                view! { <option value={i.to_string()} selected=sel>{name}</option> }
                            }).collect_view()}
                        </select>
                    }
                }}
            </div>
            {move || files.with(|f| f.is_empty()).then(|| view! {
                <p class="zonal-sensors-hint">{move || locale.get().zonal_designer.sensors.no_files.clone()}</p>
            })}
            {move || {
                let has_demo = files.with(|f| demo_sensors().iter().all(|d| f.iter().any(|e| e.name == d.name)));
                (!has_demo).then(|| view! {
                    <button class="zonal-share-btn" on:click=move |_| {
                        let first_time = untrack(|| state.current()).is_none();
                        add_demo(files);
                        if first_time {
                            state.follow.set(false);
                            state.selected.set(demo_sensors().first().map(|d| d.name.clone()));
                        }
                    }>{move || locale.get().zonal_designer.sensors.load_demo.clone()}</button>
                })
            }}

            {move || state.current().is_some().then(|| view! {
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.placement.clone()}</span>
                    <select on:change=move |ev| {
                        if let Some(p) = Placement::from_key(&event_target_value(&ev)) {
                            state.placement.set(p);
                        }
                    }>
                        {Placement::ALL.into_iter().map(|p| view! {
                            <option value={p.key()} selected=move || state.placement.get() == p>
                                {move || {
                                    let l = locale.get();
                                    let s = &l.zonal_designer.sensors;
                                    match p {
                                        Placement::Auto => s.place_auto.clone(),
                                        Placement::Grid => s.place_grid.clone(),
                                        Placement::Luminaires if outdoor => s.on_poles.clone(),
                                        Placement::Luminaires => s.place_luminaires.clone(),
                                    }
                                }}
                            </option>
                        }).collect_view()}
                    </select>
                </div>
                {move || (state.placement.get() == Placement::Grid).then(|| view! {
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.along_length.clone()}</span>
                        {num_input(state.nx)}
                    </div>
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.along_width.clone()}</span>
                        {num_input(state.ny)}
                    </div>
                })}
                {move || (state.placement.get() != Placement::Auto).then(|| view! {
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.orientation.clone()}</span>
                        <select on:change=move |ev| {
                            state.rotation.set(if event_target_value(&ev) == "90" { 90.0 } else { 0.0 });
                        }>
                            <option value="0" selected=move || state.rotation.get() == 0.0>
                                {move || format!("0° · {}", locale.get().zonal_designer.sensors.along_length)}
                            </option>
                            <option value="90" selected=move || state.rotation.get() != 0.0>
                                {move || format!("90° · {}", locale.get().zonal_designer.sensors.along_width)}
                            </option>
                        </select>
                    </div>
                })}
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.plan_for.clone()}</span>
                    <select on:change=move |ev| {
                        if let Some(m) = motion_from_key(&event_target_value(&ev)) {
                            state.motion.set(m);
                        }
                    }>
                        {move || {
                            let motions = plan.with(|p| p.as_ref().map(|p| p.motions.clone()).unwrap_or_default());
                            let current = plan.with(|p| p.as_ref().map(|p| p.motion));
                            motions.into_iter().map(|m| {
                                let sel = current == Some(m);
                                view! {
                                    <option value={motion_key(m)} selected=sel>
                                        {move || motion_label(&locale.get().ui.sensor, m)}
                                    </option>
                                }
                            }).collect_view()
                        }}
                    </select>
                </div>
                <label class="zonal-toggle">
                    <input type="checkbox"
                        prop:checked=move || state.show_all.get()
                        on:change=move |ev| state.show_all.set(event_target_checked(&ev))
                    />{" "}{move || locale.get().zonal_designer.sensors.show_all.clone()}
                </label>
                {move || plan.with(|p| p.as_ref().is_some_and(|p| p.is_sensxml)).then(|| view! {
                    <label class="zonal-toggle">
                        <input type="checkbox"
                            prop:checked=move || state.planning.get()
                            on:change=move |ev| state.planning.set(event_target_checked(&ev))
                        />{" "}{move || locale.get().ui.sensor.planning.clone()}
                    </label>
                })}
                {(!outdoor).then(|| view! {
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.margin.clone()}</span>
                        <input type="number" min="0" max="5" step="0.1" style="width:5em"
                            prop:value=move || format!("{:.1}", state.margin.get())
                            on:change=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    state.margin.set(v.clamp(0.0, 5.0));
                                }
                            }
                        />
                        <span class="zonal-toolbar-val">"m"</span>
                    </div>
                })}
                {match height {
                    Some(h) => view! {
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.sensor_height.clone()}</span>
                            <input type="number" min="1" max="30" step="0.5" style="width:5em"
                                prop:value=move || format!("{:.1}", h.get())
                                on:change=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        h.set(v.clamp(1.0, 30.0));
                                    }
                                }
                            />
                            <span class="zonal-toolbar-val">"m"</span>
                        </div>
                    }.into_any(),
                    None => view! {
                        <p class="zonal-sensors-hint">{move || {
                            let h = plan.with(|p| p.as_ref().map(|p| p.cov.room.mount_height)).unwrap_or(0.0);
                            fill(&locale.get().zonal_designer.sensors.mounting, &[("h", format!("{h:.2}"))])
                        }}</p>
                    }.into_any(),
                }}
                {move || {
                    let s = locale.get().zonal_designer.sensors.clone();
                    let pct = format!("{:.0}", MIN_COVERAGE * 100.0);
                    plan.with(|p| match p.as_ref().and_then(|p| p.auto) {
                        Some(AutoOutcome::Found(nx, ny, rot)) => Some(view! {
                            <p class="zonal-sensors-ok">{fill(&s.auto_result, &[
                                ("n", p.as_ref().map_or(nx * ny, |p| p.cov.positions.len()).to_string()), ("nx", nx.to_string()), ("ny", ny.to_string()), ("pct", pct.clone()),
                            ])}{if rot != 0.0 { format!(" · {} {rot:.0}°", s.orientation) } else { String::new() }}</p>
                        }.into_any()),
                        Some(AutoOutcome::Failed(n)) => Some(view! {
                            <p class="zonal-sensors-warn">{fill(&s.auto_failed, &[("n", n.to_string()), ("pct", pct.clone())])}</p>
                        }.into_any()),
                        None => None,
                    })
                }}
                {move || plan.with(|p| p.as_ref().map(|p| p.warnings.clone()).unwrap_or_default())
                    .into_iter().map(|w| view! { <p class="zonal-sensors-warn">{format!("⚠ {w}")}</p> }).collect_view()}

                {(!outdoor).then(|| view! {
                    <h5 class="zonal-sensors-sub">{move || locale.get().zonal_designer.sensors.energy_title.clone()}</h5>
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.control.clone()}</span>
                        <select class="zonal-sensors-select" on:change=move |ev| {
                            if let Some(c) = OccupancyControl::from_key(&event_target_value(&ev)) {
                                state.control.set(c);
                            }
                        }>
                            {OccupancyControl::ALL.into_iter().enumerate().map(|(i, c)| view! {
                                <option value={c.key()} selected=move || state.control.get() == c>
                                    {move || {
                                        let l = locale.get();
                                        let label = l.zonal_designer.sensors.controls.get(i).cloned().unwrap_or_else(|| c.label().to_string());
                                        format!("{label} (F_OC {:.2})", c.f_oc())
                                    }}
                                </option>
                            }).collect_view()}
                        </select>
                    </div>
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.space.clone()}</span>
                        <select class="zonal-sensors-select" on:change=move |ev| {
                            if let Some(s) = SpaceType::from_key(&event_target_value(&ev)) {
                                state.space.set(s);
                            }
                        }>
                            {SpaceType::ALL.into_iter().enumerate().map(|(i, s)| view! {
                                <option value={s.key()} selected=move || state.space.get() == s>
                                    {move || locale.get().zonal_designer.sensors.spaces.get(i).cloned().unwrap_or_else(|| s.label().to_string())}
                                </option>
                            }).collect_view()}
                        </select>
                    </div>
                    <label class="zonal-toggle">
                        <input type="checkbox"
                            prop:checked=move || state.hold_ok.get()
                            on:change=move |ev| state.hold_ok.set(event_target_checked(&ev))
                        />{" "}{move || locale.get().zonal_designer.sensors.hold_time.clone()}
                    </label>
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{move || locale.get().zonal_designer.sensors.hours.clone()}</span>
                        <input type="number" min="0" max="8760" step="50" style="width:6em"
                            prop:value=move || format!("{:.0}", state.hours.get())
                            on:change=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    state.hours.set(v.clamp(0.0, 8760.0));
                                }
                            }
                        />
                        <span class="zonal-toolbar-val">"h/a"</span>
                    </div>
                    <p class="zonal-sensors-hint">{move || locale.get().zonal_designer.sensors.hours_note.clone()}</p>
                })}
            })}
        </div>
    }
}

/// Coverage table, legend and energy table under the plan.
/// `outdoor`: every plane is the ground, no energy estimate.
#[component]
pub fn SensorResults(
    plan: Memo<Option<SensorPlan>>,
    energy: Memo<Option<SensorEnergy>>,
    show_all: RwSignal<bool>,
    #[prop(optional)] outdoor: bool,
) -> impl IntoView {
    let locale = use_locale();
    move || {
        let p = plan.get()?;
        let l = locale.get();
        let s = l.zonal_designer.sensors.clone();
        let ls = l.ui.sensor.clone();
        let n = p.cov.positions.len();
        let rows = p
            .cov
            .motions
            .iter()
            .map(|mc| {
                let idx = Motion::ALL.iter().position(|m| *m == mc.motion).unwrap_or(0);
                let plane = if outdoor {
                    &s.plane_ground
                } else if mc.motion == Motion::Presence {
                    &s.plane_workplane
                } else {
                    &s.plane_floor
                };
                let planned = mc.motion == p.motion;
                let detected = mc.area.is_some();
                let cls = if planned { "zonal-sensors-planned" } else { "" };
                let swatch = format!("background:{}", TRACE_COLORS[idx]);
                view! {
                    <tr class=cls>
                        <td>
                            <span class="zonal-sensors-swatch" style=swatch></span>
                            {motion_label(&ls, mc.motion)}{if planned { " ●" } else { "" }}
                        </td>
                        <td>{format!("{} {:.2} m", plane, mc.plane_height)}</td>
                        <td class="val">{if detected { format!("{:.1} %", mc.covered_fraction * 100.0) } else { s.not_detected.clone() }}</td>
                        <td>{if detected { format!("{:.1} %", mc.overlap_fraction * 100.0) } else { "–".into() }}</td>
                        <td>{mc.area.as_ref().map(|a| format!("{:.1} m", a.diameter())).unwrap_or_else(|| "–".into())}</td>
                    </tr>
                }
            })
            .collect_view();
        let summary = fill(
            &s.summary,
            &[
                ("n", n.to_string()),
                ("a", format!("{:.1}", p.cov.m2_per_sensor())),
            ],
        );
        let energy_view = energy.get().map(|e| {
            let check = |ok: bool| if ok { "✓" } else { "✗" };
            let min = format!("{:.0}", MIN_COVERAGE * 100.0);
            view! {
                <h3 class="zonal-sensors-h">{s.energy_title.clone()}</h3>
                <table class="zonal-stats-table">
                    <tr>
                        <td></td><td>"F_O"</td><td>{format!("{} (kWh/a)", s.energy)}</td>
                    </tr>
                    <tr>
                        <td>{s.manual.clone()}</td>
                        <td>{format!("{:.2}", e.fo.f_o_manual)}</td>
                        <td>{format!("{:.0}", e.kwh_manual)}</td>
                    </tr>
                    <tr>
                        <td>{s.with_sensors.clone()}</td>
                        <td class="val">{format!("{:.2}", e.fo.f_o)}</td>
                        <td class="val">{format!("{:.0}", e.kwh)}</td>
                    </tr>
                    <tr>
                        <td>{s.saving.clone()}</td>
                        <td></td>
                        <td class="val">{format!("{:.0} %  ({:.0} kWh/a)", e.saving_pct(), e.kwh_manual - e.kwh)}</td>
                    </tr>
                </table>
                <p class="zonal-sensors-hint">{format!(
                    "{} {:.0} W × {:.0} h/a · F_OC {:.2} · F_A {:.2}",
                    s.power, e.power_w, e.hours, e.fo.f_oc, e.fo.f_a
                )}</p>
                {e.uses_detection.then(|| view! {
                    <ul class="zonal-sensors-conditions">
                        <li>{format!("{} {}", check(e.fo.hold_time_ok), s.cond_hold)}</li>
                        <li>{format!("{} {}", check(e.fo.group_ok), fill(&s.cond_group, &[("a", format!("{:.1}", e.m2_per_sensor))]))}</li>
                        <li>{format!("{} {}", check(e.fo.coverage_ok), fill(&s.cond_coverage, &[("min", min.clone()), ("pct", format!("{:.1}", e.coverage * 100.0))]))}</li>
                    </ul>
                })}
                {(!e.fo.credited).then(|| view! { <p class="zonal-sensors-warn">{s.not_credited.clone()}</p> })}
            }
        });
        Some(view! {
            <div class="zonal-sensors-results">
                <div class="zonal-sensors-legend">
                    {p.shown(show_all.get()).iter().map(|m| {
                        let idx = Motion::ALL.iter().position(|x| x == m).unwrap_or(0);
                        let swatch = format!("background:{}", TRACE_COLORS[idx]);
                        view! {
                            <span><span class="zonal-sensors-swatch" style=swatch></span>{motion_label(&ls, *m)}</span>
                        }
                    }).collect_view()}
                    <span><span class="zonal-sensors-swatch" style="background:#ef4444;opacity:0.6"></span>{format!("{} ({})", s.legend_gap, motion_label(&ls, p.motion))}</span>
                    <span><span class="zonal-sensors-swatch zonal-sensors-dashed"></span>{s.legend_area.clone()}</span>
                    <span><span class="zonal-sensors-swatch zonal-sensors-mark"></span>{s.legend_sensor.clone()}</span>
                </div>
                <h3 class="zonal-sensors-h">{format!("{} · {}", s.title, p.name)}</h3>
                <table class="zonal-stats-table">
                    <tr>
                        <td>{s.col_motion.clone()}</td><td>{s.col_plane.clone()}</td><td>{s.col_covered.clone()}</td><td>{s.col_overlap.clone()}</td><td>{s.col_diameter.clone()}</td>
                    </tr>
                    {rows}
                </table>
                <p class="zonal-sensors-hint">{summary}</p>
                {energy_view}
                <p class="zonal-sensors-method">{if outdoor { s.outdoor_method.clone() } else { s.method.clone() }}</p>
            </div>
        })
    }
}

/// Typst escape for text in content blocks.
fn typ_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '#' | '*' | '_' | '[' | ']' | '$' | '@' | '<' | '>' | '`' | '~' | '/'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// "Presence sensors" section for the interior PDF (English, like the
/// rest of that report). `plan_svg` is the plan with the sensor overlay.
pub fn typst_section(plan: &SensorPlan, energy: Option<&SensorEnergy>, plan_svg: &str) -> String {
    let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let name = |m: Motion| match m {
        Motion::Tangential => "Walking across (tangential)",
        Motion::Radial => "Walking towards (radial)",
        Motion::Presence => "Seated presence",
    };
    let mut t = String::from("#pagebreak()\n\n== Presence Sensors\n\n");
    let n = plan.cov.positions.len();
    t.push_str(&format!(
        "*Sensor:* {}  |  *Sensors:* {n}  |  *Mounting height:* {:.2} m  |  *Wall margin:* {:.2} m  |  *Floor area per sensor:* {:.1} m#super[2]\n\n",
        typ_text(&plan.name),
        plan.cov.room.mount_height,
        plan.cov.room.margin,
        plan.cov.m2_per_sensor()
    ));
    let rot = plan
        .cov
        .positions
        .first()
        .map(|p| p.rotation_deg)
        .unwrap_or(0.0);
    if rot != 0.0 {
        t.push_str(&format!(
            "Sensors turned by {rot:.0}° (C0 along the room width).\n\n"
        ));
    }
    match plan.auto {
        Some(AutoOutcome::Found(nx, ny, _)) => t.push_str(&format!(
            "Automatic placement: {nx} × {ny} is the smallest grid that detects ≥ {:.0} % of the evaluated area ({}).\n\n",
            MIN_COVERAGE * 100.0,
            name(plan.motion).to_lowercase()
        )),
        Some(AutoOutcome::Failed(c)) => t.push_str(&format!(
            "Automatic placement: no grid of up to {c} sensors reaches {:.0} %.\n\n",
            MIN_COVERAGE * 100.0
        )),
        None => {}
    }
    for w in &plan.warnings {
        t.push_str(&format!("_Warning: {}_\n\n", typ_text(w)));
    }
    t.push_str("#table(\n  columns: 5,\n  align: (left, center, center, center, center),\n");
    t.push_str("  [*Motion*], [*Plane*], [*Covered*], [*Overlap*], [*Ø per sensor*],\n");
    for mc in &plan.cov.motions {
        let plane = if mc.motion == Motion::Presence {
            "workplane"
        } else {
            "floor"
        };
        let mark = if mc.motion == plan.motion {
            " (planned)"
        } else {
            ""
        };
        match &mc.area {
            Some(a) => t.push_str(&format!(
                "  [{}{mark}], [{plane} {:.2} m], [{:.1} %], [{:.1} %], [{:.1} m],\n",
                name(mc.motion),
                mc.plane_height,
                mc.covered_fraction * 100.0,
                mc.overlap_fraction * 100.0,
                a.diameter()
            )),
            None => t.push_str(&format!(
                "  [{}{mark}], [{plane} {:.2} m], [not detected], [–], [–],\n",
                name(mc.motion),
                mc.plane_height
            )),
        }
    }
    t.push_str(")\n\n");
    t.push_str(&format!(
        "#align(center)[#image(bytes(\"{}\"), width: 85%)]\n\n",
        escape(plan_svg)
    ));
    t.push_str("#text(size: 8pt)[Red: not detected (planned motion type). Dashed: evaluated area. Purple rings: sensors.]\n\n");
    if let Some(e) = energy {
        t.push_str("=== Energy Estimate (EN 15193-1)\n\n");
        t.push_str("#table(\n  columns: 3,\n  align: (left, center, center),\n");
        t.push_str("  [], [*F#sub[O]*], [*Energy (kWh/a)*],\n");
        t.push_str(&format!(
            "  [Manual switching], [{:.2}], [{:.0}],\n",
            e.fo.f_o_manual, e.kwh_manual
        ));
        t.push_str(&format!(
            "  [With presence control], [*{:.2}*], [*{:.0}*],\n",
            e.fo.f_o, e.kwh
        ));
        t.push_str(&format!(
            "  [Saving], [], [*{:.0} %* ({:.0} kWh/a)],\n)\n\n",
            e.saving_pct(),
            e.kwh_manual - e.kwh
        ));
        t.push_str(&format!(
            "Installed power {:.0} W × {:.0} h/a (assumed operating hours) · F#sub[OC] {:.2} · F#sub[A] {:.2}\n\n",
            e.power_w, e.hours, e.fo.f_oc, e.fo.f_a
        ));
        let ok = |b: bool| if b { "met" } else { "*not met*" };
        t.push_str(&format!(
            "- Hold time ≤ 15 min: {}\n- Control group ≤ {:.0} m#super[2] ({:.1} m#super[2] per sensor): {}\n- Detection covers ≥ {:.0} % of the area ({:.1} %): {}\n\n",
            ok(e.fo.hold_time_ok),
            MAX_GROUP_M2,
            e.m2_per_sensor,
            ok(e.fo.group_ok),
            MIN_COVERAGE * 100.0,
            e.coverage * 100.0,
            ok(e.fo.coverage_ok)
        ));
        if !e.fo.credited {
            t.push_str("_Not all conditions are met: counted as manual switching._\n\n");
        }
    }
    t.push_str("#text(size: 8pt)[Coverage: the detection area of each sensor at the mounting height, cut at the floor (walking) and at the workplane (seated presence); wall margin excluded, walls stop detection. F#sub[O] from EN 15193-1 Annex E (F#sub[OC] by control type, F#sub[A] by space type; open plan interpolated between 10 and 30 m#super[2] per sensor), one control group per sensor, without daylight or constant-illuminance factors.]\n\n");
    t
}

/// Sensor data of the chosen name.
pub fn chosen_data(state: &SensorPlanState) -> Option<(String, SensorData)> {
    let name = state.current()?;
    state.files.with(|v| {
        v.iter()
            .find(|f| f.name == name)
            .map(|f| (name.clone(), f.data.clone()))
    })
}
