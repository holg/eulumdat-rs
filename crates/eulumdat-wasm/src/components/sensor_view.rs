//! Sensors tab: presence / motion detectors.
//!
//! Sources: the sensor files of an opened GLDF (`GldfExtract::sensors`), a
//! dropped SensXML file, or the loaded LDT viewed as a sensor LDT. A sensor
//! LDT is a detection volume in metres, so it gets the plan view plus the
//! shape diagrams that stay meaningful (polar, cartesian, heatmap, 3D), all
//! through `eulumdat::sensor::diagrams` with range wording. SensXML holds
//! detection areas per mounting height, so it gets the plan view only.

use crate::i18n::use_locale;
use eulumdat::diagram::{PolarDiagram, SvgTheme};
use eulumdat::sensor::{
    diagrams, DetectionVolume, Motion, MountingHint, Quality, SensXml, SensorDiagram,
    SensorPlanSvg, SensorPlanTrace, TRACE_COLORS,
};
use eulumdat::Eulumdat;
use leptos::prelude::*;
use std::sync::Arc;

/// Parsed sensor data.
#[derive(Clone)]
pub enum SensorData {
    Ldt(Arc<DetectionVolume>),
    Xml(Arc<SensXml>),
}

impl PartialEq for SensorData {
    /// Same parse (pointer equality): a new parse is new data.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SensorData::Ldt(a), SensorData::Ldt(b)) => Arc::ptr_eq(a, b),
            (SensorData::Xml(a), SensorData::Xml(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl SensorData {
    /// The shared core type (coverage planning in the interior designer).
    pub fn to_core(&self) -> eulumdat::sensor::SensorFile {
        match self {
            SensorData::Ldt(v) => eulumdat::sensor::SensorFile::Ldt((**v).clone()),
            SensorData::Xml(x) => eulumdat::sensor::SensorFile::Xml((**x).clone()),
        }
    }
}

/// One sensor the tab can show.
#[derive(Clone, PartialEq)]
pub struct SensorFile {
    pub name: String,
    pub data: SensorData,
}

impl SensorFile {
    /// Parse a sensor file; `content_type` is the GLDF content type
    /// (`sensor/sensldt` / `sensor/sensxml`) or empty.
    pub fn from_bytes(name: &str, content_type: &str, bytes: &[u8]) -> Option<Self> {
        let xml = content_type.ends_with("sensxml")
            || bytes.windows(8).take(800).any(|w| w == b"<SensXML");
        let data = if xml {
            SensorData::Xml(Arc::new(SensXml::parse(bytes).ok()?))
        } else {
            let text = String::from_utf8_lossy(bytes);
            SensorData::Ldt(Arc::new(DetectionVolume::new(Eulumdat::parse(&text).ok()?)))
        };
        Some(Self {
            name: name.to_string(),
            data,
        })
    }
}

/// Synthetic demo sensors (our own numbers, `make_demo.py`), so a link can
/// show the tab without anyone's product data.
const DEMO_CEILING: &str = include_str!("../../demo/demo_ceiling_360.ldt");
const DEMO_CORRIDOR: &str = include_str!("../../demo/demo_corridor.ldt");
const DEMO_SENSXML: &[u8] = include_bytes!("../../demo/demo_ceiling_sensxml.xml");
const DEMO_SENSXML_WALL: &[u8] = include_bytes!("../../demo/demo_wall_180_sensxml.xml");
const DEMO_SENSXML_HIGHBAY: &[u8] = include_bytes!("../../demo/demo_highbay_sensxml.xml");
const DEMO_SENSXML_CORRIDOR: &[u8] = include_bytes!("../../demo/demo_corridor_sensxml.xml");

/// Names, content types and bytes of the demo sensors.
fn demo_sensor_list() -> [(&'static str, &'static str, &'static [u8]); 6] {
    [
        (
            "Demo · ceiling presence detector (sensor LDT)",
            "sensor/sensldt",
            DEMO_CEILING.as_bytes(),
        ),
        (
            "Demo · corridor detector (sensor LDT)",
            "sensor/sensldt",
            DEMO_CORRIDOR.as_bytes(),
        ),
        (
            "Demo · ceiling detector (SensXML)",
            "sensor/sensxml",
            DEMO_SENSXML,
        ),
        (
            "Demo · wall motion detector 180° (SensXML)",
            "sensor/sensxml",
            DEMO_SENSXML_WALL,
        ),
        (
            "Demo · high-bay detector 6–14 m (SensXML)",
            "sensor/sensxml",
            DEMO_SENSXML_HIGHBAY,
        ),
        (
            "Demo · corridor detector (SensXML)",
            "sensor/sensxml",
            DEMO_SENSXML_CORRIDOR,
        ),
    ]
}

/// Names and bytes of the demo sensors (to add them to a project).
pub fn demo_sensor_sources() -> Vec<(&'static str, &'static [u8])> {
    demo_sensor_list()
        .iter()
        .map(|(n, _, b)| (*n, *b))
        .collect()
}

/// The demo sensors, newest first in the list.
pub fn demo_sensors() -> Vec<SensorFile> {
    demo_sensor_list()
        .iter()
        .filter_map(|(n, ct, b)| SensorFile::from_bytes(n, ct, b))
        .collect()
}

/// Add the demo sensors to `files` unless they are already there.
pub fn add_demo(files: RwSignal<Vec<SensorFile>>) {
    // With projects: the Demo project has them, a user project gets them
    // as items (the sensor list follows the project).
    if let Some(ctx) = super::projects_ui::ctx() {
        ctx.add_demo_sensors();
        return;
    }
    files.update(|v| {
        for d in demo_sensors() {
            if !v.iter().any(|e| e.name == d.name) {
                v.push(d);
            }
        }
    });
}

/// An opened `.ldt` that was recognised as a sensor LDT (`is_sensor_ldt`)
/// and went to the Sensors tab (context): the tab shows a note with "open
/// as luminaire instead", which hands the file back to the app through
/// `open_as_luminaire`.
#[derive(Clone, Copy)]
pub struct RecognisedSensorLdt {
    /// (file name, content) of the last recognised file.
    pub notice: RwSignal<Option<(String, String)>>,
    /// Set by the tab's button; the app loads it as a luminaire.
    pub open_as_luminaire: RwSignal<Option<(String, String)>>,
}

/// Sensors known to the app (context), filled by the GLDF / SensXML loaders.
#[derive(Clone, Copy)]
pub struct SensorFiles(pub RwSignal<Vec<SensorFile>>);

fn motion_label(l: &eulumdat_i18n::UiSensor, m: Motion) -> String {
    match m {
        Motion::Tangential => l.tangential.clone(),
        Motion::Radial => l.radial.clone(),
        Motion::Presence => l.presence.clone(),
    }
}

fn quality_label(l: &eulumdat_i18n::UiSensor, q: Quality) -> String {
    match q {
        Quality::Measured => l.measured.clone(),
        Quality::Interpolated => l.interpolated.clone(),
        Quality::Extrapolated => l.extrapolated.clone(),
    }
}

fn view_label(l: &eulumdat_i18n::UiSensor, k: SensorDiagram) -> String {
    match k {
        SensorDiagram::Plan => l.view_plan.clone(),
        SensorDiagram::Polar => l.view_polar.clone(),
        SensorDiagram::Cartesian => l.view_cartesian.clone(),
        SensorDiagram::Heatmap => l.view_heatmap.clone(),
        SensorDiagram::Volume3d => l.view_3d.clone(),
    }
}

/// One row of the per-motion table.
struct Row {
    motion: Motion,
    max_range: f64,
    area: f64,
    diameter: f64,
    quality: Option<Quality>,
    /// Sensor LDT: depth (m below the sensor) where this volume ends.
    depth_limit: Option<f64>,
}

fn rows_and_traces(
    data: &SensorData,
    height: f64,
    planning: bool,
    l: &eulumdat_i18n::UiSensor,
) -> (Vec<Row>, Vec<SensorPlanTrace>) {
    let mut rows = Vec::new();
    let mut traces = Vec::new();
    for (i, m) in Motion::ALL.iter().enumerate() {
        let fp = match data {
            SensorData::Ldt(v) => v.motion_footprint(*m, height, 1.0).map(|f| (f, None)),
            SensorData::Xml(x) => x.footprint(*m, height, planning).map(|(f, q)| (f, Some(q))),
        };
        let Some((fp, quality)) = fp else { continue };
        let depth_limit = match data {
            SensorData::Ldt(v) => v.motion_depth_limit(*m),
            SensorData::Xml(_) => None,
        };
        rows.push(Row {
            motion: *m,
            max_range: fp.max_range,
            area: fp.area_m2,
            diameter: fp.equivalent_diameter(),
            quality,
            depth_limit,
        });
        if fp.is_empty() {
            continue;
        }
        traces.push(SensorPlanTrace {
            label: format!("{} · {height:.2} m", motion_label(l, *m)),
            color: TRACE_COLORS[i].to_string(),
            footprint: fp,
        });
    }
    (rows, traces)
}

/// State of the Sensors tab, app-wide: the tab keeps it across tab
/// switches, and the PDF report (dashboard, File menu) uses the sensor the
/// user picked.
#[derive(Clone, Copy)]
pub struct SensorUi {
    pub files: RwSignal<Vec<SensorFile>>,
    pub selected: RwSignal<usize>,
    pub use_loaded: RwSignal<bool>,
    pub kind: RwSignal<SensorDiagram>,
    pub height: RwSignal<f64>,
    pub planning: RwSignal<bool>,
    pub motion: RwSignal<Motion>,
    /// C-plane of the polar / cartesian views and the cut on the plan
    /// (`None` = the default C0–C180 / C90–C270 overview).
    pub c_plane: RwSignal<Option<f64>>,
    /// A real luminaire is loaded (not the default "New Luminaire").
    pub luminaire_loaded: RwSignal<bool>,
}

impl SensorUi {
    pub fn new(files: RwSignal<Vec<SensorFile>>) -> Self {
        Self {
            files,
            selected: RwSignal::new(0),
            use_loaded: RwSignal::new(false),
            kind: RwSignal::new(SensorDiagram::Plan),
            height: RwSignal::new(eulumdat::sensor::REFERENCE_DEPTH),
            planning: RwSignal::new(false),
            motion: RwSignal::new(Motion::Tangential),
            c_plane: RwSignal::new(None),
            luminaire_loaded: RwSignal::new(false),
        }
    }

    /// Deep link, applied once at startup: `sensor=ldt` views the loaded
    /// LDT as a sensor, `sensor=demo` adds the demo sensors,
    /// `sensorview=plan|polar|cartesian|heatmap|3d`, `height=2.5`.
    pub fn apply_link(&self, params: &[(String, String)]) {
        for (k, v) in params {
            match (k.as_str(), v.as_str()) {
                ("sensor", "ldt" | "loaded") => self.use_loaded.set(true),
                ("sensor", "demo") => add_demo(self.files),
                ("sensorview", view) => {
                    let kind = match view {
                        "polar" => Some(SensorDiagram::Polar),
                        "cartesian" => Some(SensorDiagram::Cartesian),
                        "heatmap" => Some(SensorDiagram::Heatmap),
                        "3d" | "volume" => Some(SensorDiagram::Volume3d),
                        "plan" => Some(SensorDiagram::Plan),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        self.kind.set(kind);
                    }
                }
                ("height", h) => {
                    if let Ok(h) = h.parse::<f64>() {
                        self.height.set(h.clamp(1.0, 20.0));
                    }
                }
                ("cplane", c) => {
                    if let Ok(c) = c.parse::<f64>() {
                        self.c_plane.set(Some(c.rem_euclid(360.0)));
                    }
                }
                _ => {}
            }
        }
    }

    /// The loaded LDT as a sensor: the as-loaded file, not the ATLA round
    /// trip (that recomputes the direct ratios = the sensor parameters).
    fn loaded_entry(ldc: &Eulumdat, label: &str) -> SensorFile {
        let ldt = super::app::lossless_ldc(ldc).unwrap_or_else(|| ldc.clone());
        SensorFile {
            name: label.to_string(),
            data: SensorData::Ldt(Arc::new(DetectionVolume::new(ldt))),
        }
    }

    /// The sensor selected on the tab, for a report (untracked):
    /// file, report options, and whether it is the loaded LDT itself.
    pub fn report_choice(
        &self,
        ldc: &Eulumdat,
        loaded_label: &str,
    ) -> Option<(SensorFile, eulumdat_typst::SensorReportOptions, bool)> {
        let files = self.files.get_untracked();
        let use_loaded = self.use_loaded.get_untracked();
        let n = files.len() + usize::from(use_loaded);
        if n == 0 {
            return None;
        }
        let i = self.selected.get_untracked().min(n - 1);
        let (file, is_loaded) = if i < files.len() {
            (files[i].clone(), false)
        } else {
            (Self::loaded_entry(ldc, loaded_label), true)
        };
        let opt = eulumdat_typst::SensorReportOptions {
            name: file.name.clone(),
            height: self.height.get_untracked(),
            planning: self.planning.get_untracked(),
            date: eulumdat_typst::sensor_report::today(),
            demo: file.name.starts_with("Demo ·"),
        };
        Some((file, opt, is_loaded))
    }
}

/// `"Demo · high-bay (SensXML)"` → `"Demo_high-bay_SensXML"`.
fn file_stem(name: &str) -> String {
    let raw: String = name
        .rsplit('/')
        .next()
        .unwrap_or("sensor")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut stem = raw.trim_matches('_').to_string();
    while stem.contains("__") {
        stem = stem.replace("__", "_");
    }
    if stem.is_empty() {
        "sensor".into()
    } else {
        stem
    }
}

/// Typst source and file name of the PDF report.
///
/// - no sensor selected: the photometric report of `photometric_ldt`;
/// - a sensor selected and a luminaire loaded: the photometric report with a
///   "Sensors" chapter;
/// - a sensor selected but no luminaire loaded, or the loaded LDT is the
///   sensor: the sensor report alone.
pub fn report_typst(
    ui: &SensorUi,
    photometric_ldt: &Eulumdat,
    current_ldc: &Eulumdat,
    loaded_label: &str,
    default_name: String,
    spectrum: Option<(&eulumdat::atla::SpectralDistribution, &str)>,
) -> (String, String) {
    // Luminaire report; with a measured spectrum it ends with the colour
    // quality chapter (TM-30 card, CRI, hue bins).
    let photometric = || {
        let report = eulumdat_typst::generate_typst_with_files(
            photometric_ldt,
            &eulumdat_typst::ReportOptions::default().sections,
        )
        .0;
        match spectrum {
            Some((spd, name)) => eulumdat_typst::append_colour_quality_chapter(&report, spd, name),
            None => report,
        }
    };
    let Some((file, opt, is_loaded)) = ui.report_choice(current_ldc, loaded_label) else {
        return (photometric(), default_name);
    };
    let src = match &file.data {
        SensorData::Ldt(v) => eulumdat_typst::SensorSource::Ldt(v),
        SensorData::Xml(x) => eulumdat_typst::SensorSource::Xml(x),
    };
    if is_loaded || !ui.luminaire_loaded.get_untracked() {
        (
            eulumdat_typst::generate_sensor_report(&src, &opt),
            format!("{}_sensor_report.pdf", file_stem(&file.name)),
        )
    } else {
        (
            eulumdat_typst::append_sensor_chapter(&photometric(), &[(src, opt)]),
            default_name,
        )
    }
}

#[component]
pub fn SensorView(ldc: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let ui = use_context::<SensorUi>().unwrap_or_else(|| {
        SensorUi::new(
            use_context::<SensorFiles>()
                .map(|f| f.0)
                .unwrap_or_else(|| RwSignal::new(Vec::new())),
        )
    });
    let files = ui.files;
    let use_loaded = ui.use_loaded;
    let selected = ui.selected;
    let kind = ui.kind;
    let height = ui.height;
    let planning = ui.planning;
    let motion = ui.motion;
    let c_plane = ui.c_plane;
    let (pdf_busy, set_pdf_busy) = signal(false);
    let report_spectrum = use_context::<super::app::ReportSpectrum>();

    // GLDF / SensXML sensors first, then the loaded LDT if wanted.
    let entries = Memo::new(move |_| {
        let mut v = files.get();
        if use_loaded.get() {
            v.push(SensorUi::loaded_entry(
                &ldc.get(),
                &locale.get().ui.sensor.loaded_ldt,
            ));
        }
        v
    });
    let current = Memo::new(move |_| {
        let e = entries.get();
        let i = selected.get().min(e.len().saturating_sub(1));
        e.get(i).map(|f| f.data.clone())
    });
    // PDF report: the same report as the dashboard / File menu export.
    let on_report = move |_| {
        let ldc_now = ldc.get_untracked();
        let label = locale.get_untracked().ui.sensor.loaded_ldt.clone();
        let spectrum = report_spectrum.and_then(|s| s.0.get_untracked());
        let (source, pdf_name) = report_typst(
            &ui,
            &ldc_now,
            &ldc_now,
            &label,
            "photometric_report.pdf".to_string(),
            spectrum.as_ref().map(|(spd, n)| (spd, n.as_str())),
        );
        set_pdf_busy.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match super::app::compile_typst_to_pdf(&source).await {
                Ok(pdf) => super::file_handler::download_bytes(&pdf_name, &pdf, "application/pdf"),
                Err(e) => {
                    web_sys::console::error_1(&format!("sensor PDF failed: {e}").into());
                    // the Typst source still gives a report (compile it locally)
                    super::file_handler::download_file(
                        &pdf_name.replace(".pdf", ".typ"),
                        &source,
                        "text/plain",
                    );
                }
            }
            set_pdf_busy.set(false);
        });
    };

    // SensXML holds footprints only: for the angular views, rebuild the
    // detection volume of the chosen motion type from the measured heights.
    let derived = Memo::new(move |_| match current.get() {
        Some(SensorData::Xml(x)) => x
            .to_detection_volume(motion.get(), planning.get())
            .map(|v| SensorData::Ldt(Arc::new(v))),
        _ => None,
    });

    let recognised = use_context::<RecognisedSensorLdt>();

    // C-planes of the volume shown (a SensXML file: the rebuilt volume);
    // none for a rotationally symmetric sensor, like the luminaire views.
    let c_planes = Memo::new(move |_| {
        let planes = |d: &SensorData| match d {
            SensorData::Ldt(v) => PolarDiagram::available_c_planes(v.ldt()),
            SensorData::Xml(_) => Vec::new(),
        };
        match current.get() {
            Some(d @ SensorData::Ldt(_)) => planes(&d),
            Some(SensorData::Xml(_)) => derived.get().map(|d| planes(&d)).unwrap_or_default(),
            None => Vec::new(),
        }
    });
    let c_plane_on = Memo::new(move |_| c_plane.get().is_some());
    // The slider lives in its own block: the diagram block is rebuilt on
    // every change, which would end a drag.
    let c_plane_control = move || {
        let planes = c_planes.get();
        let k = kind.get();
        let applies = matches!(
            k,
            SensorDiagram::Plan | SensorDiagram::Polar | SensorDiagram::Cartesian
        );
        if planes.is_empty() || !applies || current.with(|c| c.is_none()) {
            return None;
        }
        let max_idx = planes.len() - 1;
        let planes_for_idx = planes.clone();
        let idx = move || {
            let c = c_plane.get().unwrap_or(0.0);
            planes_for_idx
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    let d = |x: f64| ((x - c + 540.0).rem_euclid(360.0) - 180.0).abs();
                    d(*a.1).total_cmp(&d(*b.1))
                })
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        Some(view! {
            <div class="isolux-controls">
                <div class="control-group">
                    <span>{move || locale.get().diagram.axis.c_plane.clone()}</span>
                    {move || if c_plane_on.get() {
                        let planes = planes.clone();
                        let idx = idx.clone();
                        view! {
                            <input type="range" min="0" prop:max=max_idx.to_string()
                                prop:value=move || idx().to_string()
                                on:input=move |ev| {
                                    if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                                        if let Some(&a) = planes.get(i) {
                                            c_plane.set(Some(a));
                                        }
                                    }
                                }
                            />
                            <span class="control-value">
                                {move || format!("C {:.0}°", c_plane.get().unwrap_or(0.0))}
                            </span>
                            <button class="btn btn-sm btn-outline" on:click=move |_| c_plane.set(None)>"✕"</button>
                        }.into_any()
                    } else {
                        let first = planes.first().copied().unwrap_or(0.0);
                        view! {
                            <button class="btn btn-sm btn-outline" on:click=move |_| c_plane.set(Some(first))>
                                {move || locale.get().ui.butterfly.c_plane_selector.clone()}
                            </button>
                        }.into_any()
                    }}
                </div>
            </div>
        })
    };

    view! {
        <div class="sensor-view">
            <div class="diagram-header">
                <span class="diagram-title">{move || locale.get().ui.sensor.title.clone()}</span>
            </div>
            <p class="area-hint">{move || locale.get().ui.sensor.intro.clone()}</p>
            {move || recognised.and_then(|r| r.notice.get().map(|(name, content)| {
                let l = locale.get().ui.sensor.clone();
                view! {
                    <div class="sensor-recognised">
                        <span>{l.recognised.replace("{name}", &name)}</span>
                        <button class="btn btn-sm btn-outline" on:click=move |_| {
                            r.notice.set(None);
                            r.open_as_luminaire.set(Some((name.clone(), content.clone())));
                        }>{l.open_as_luminaire.clone()}</button>
                        <button class="btn btn-sm" title="×" on:click=move |_| r.notice.set(None)>"×"</button>
                    </div>
                }
            }))}

            <div class="isolux-controls">
                <div class="control-group">
                    <span>{move || locale.get().ui.sensor.source.clone()}</span>
                    <select on:change=move |ev| {
                        if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                            selected.set(i);
                        }
                    }>
                        {move || entries.get().into_iter().enumerate().map(|(i, f)| view! {
                            <option value=i.to_string() selected=move || selected.get() == i>{f.name}</option>
                        }).collect_view()}
                    </select>
                </div>
                <button class="btn btn-sm btn-outline" on:click=move |_| add_demo(files)>
                    {move || locale.get().ui.sensor.demo.clone()}
                </button>
                <button class="btn btn-sm btn-primary"
                    prop:disabled=move || pdf_busy.get() || current.get().is_none()
                    on:click=on_report
                >
                    {move || if pdf_busy.get() { "…".to_string() } else { format!("📄 {}", locale.get().ui.sensor.report) }}
                </button>
                <label class="control-group">
                    <input type="checkbox"
                        prop:checked=move || use_loaded.get()
                        on:change=move |ev| use_loaded.set(event_target_checked(&ev))
                    />
                    <span>{move || locale.get().ui.sensor.use_loaded.clone()}</span>
                </label>
            </div>

            {move || {
                let l = locale.get().ui.sensor.clone();
                let Some(data) = current.get() else {
                    return view! { <p class="area-hint">{l.none.clone()}</p> }.into_any();
                };
                let is_xml = matches!(data, SensorData::Xml(_));
                let kinds: Vec<SensorDiagram> = SensorDiagram::ALL.to_vec();
                let k = kind.get();
                let h = height.get();
                let (rows, traces) = rows_and_traces(&data, h, planning.get(), &l);
                let theme = diagrams::sensor_theme(&SvgTheme::css_variables_with_locale(&locale.get()));
                let nothing = k == SensorDiagram::Plan && traces.is_empty();
                // Only this closure reads the C-plane, so moving the slider
                // redraws the diagram without rebuilding the controls.
                let svg = {
                    let (data, traces, theme) = (data.clone(), traces.clone(), theme.clone());
                    move || {
                        let cp = c_plane.get().filter(|_| !c_planes.with(|p| p.is_empty()));
                        match (&data, k) {
                            (_, SensorDiagram::Plan) if nothing => String::new(),
                            (_, SensorDiagram::Plan) => {
                                SensorPlanSvg::render_with_cut(&traces, 520.0, &theme, cp)
                            }
                            (SensorData::Ldt(v), other) => {
                                diagrams::render_at_plane(v, other, 560.0, 480.0, h, &theme, cp)
                            }
                            (SensorData::Xml(_), other) => match derived.get() {
                                Some(SensorData::Ldt(v)) => diagrams::render_at_plane(
                                    &v, other, 560.0, 480.0, h, &theme, cp,
                                ),
                                _ => String::new(),
                            },
                        }
                    }
                };
                // SensXML angular views: which motion type, and from which heights
                let xml_angular = is_xml && k != SensorDiagram::Plan;
                let derived_note = match &data {
                    SensorData::Xml(x) if xml_angular => {
                        let hs: Vec<String> = x.heights(motion.get()).iter().map(|h| format!("{h} m")).collect();
                        Some(l.derived.replace("{h}", &hs.join(", ")))
                    }
                    _ => None,
                };
                // Slider range: the deepest measured height (SensXML) or the
                // depth where the volume ends (sensor LDT), at least 8 m.
                let slider_max = match &data {
                    SensorData::Xml(x) => Motion::ALL
                        .iter()
                        .filter_map(|m| x.heights(*m).last().copied())
                        .fold(0.0, f64::max)
                        + 2.0,
                    SensorData::Ldt(v) => v.motion_depth_limit(Motion::Tangential).unwrap_or(0.0).ceil() + 1.0,
                }
                .clamp(8.0, 20.0);
                let info = match &data {
                    SensorData::Ldt(v) => {
                        let p = v.params();
                        let mount = match p.mounting_hint() {
                            MountingHint::Wall => l.wall.clone(),
                            MountingHint::Ceiling => l.ceiling.clone(),
                            MountingHint::WallOrCeiling => l.wall_ceiling.clone(),
                            MountingHint::Other(n) => n.to_string(),
                        };
                        let raw: Vec<String> = p.raw.iter().map(|x| format!("{x}")).collect();
                        let mut lines = vec![
                            format!("{}: {mount}", l.mounting),
                            format!("{}: [{}]", l.parameters, raw.join(", ")),
                        ];
                        let warnings: Vec<String> =
                            data.to_core().warnings().iter().map(|w| w.message()).collect();
                        if !warnings.is_empty() {
                            lines.push(format!("{}: {}", l.warnings, warnings.join("; ")));
                        }
                        lines
                    }
                    SensorData::Xml(x) => {
                        let mut v = vec![format!(
                            "{} {} · SensXML {} · {} · {}",
                            x.header.manufacturer.clone().unwrap_or_default(),
                            x.header.product_name.clone().unwrap_or_default(),
                            x.version.clone().unwrap_or_default(),
                            x.header.laboratory.clone().unwrap_or_default(),
                            x.sensor.measurement_type.clone().unwrap_or_default(),
                        )];
                        if let Some(mt) = x.sensor.mounting_type {
                            let mount = match mt {
                                1 => l.wall.clone(),
                                2 => l.ceiling.clone(),
                                3 => l.wall_ceiling.clone(),
                                n => n.to_string(),
                            };
                            v.push(format!("{}: {mount}", l.mounting));
                        }
                        for m in Motion::ALL {
                            let hs = x.heights(m);
                            if !hs.is_empty() {
                                let hs: Vec<String> = hs.iter().map(|h| format!("{h} m")).collect();
                                v.push(format!("{}: {}", motion_label(&l, m), hs.join(", ")));
                            }
                        }
                        if !x.warnings.is_empty() {
                            v.push(format!("{}: {}", l.warnings, x.warnings.join("; ")));
                        }
                        v
                    }
                };
                let height_label = if is_xml { l.height_xml.clone() } else { l.height_ldt.clone() };
                let show_quality = is_xml;
                view! {
                    <div class="isolux-controls">
                        <div class="control-group">
                            <span>{l.view.clone()}</span>
                            {kinds.into_iter().map(|kk| {
                                let label = view_label(&l, kk);
                                view! {
                                    <button
                                        class=move || format!("btn btn-sm {}", if kind.get() == kk { "btn-primary" } else { "btn-outline" })
                                        on:click=move |_| kind.set(kk)
                                    >{label}</button>
                                }
                            }).collect_view()}
                        </div>
                        <div class="control-group">
                            <span>{height_label}</span>
                            <input type="range" min="1.0" prop:max=slider_max.to_string() step="0.1"
                                prop:value=move || height.get().to_string()
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        height.set(v);
                                    }
                                }
                            />
                            <span class="control-value">{format!("{h:.1} m")}</span>
                            <span style="color: var(--text-muted); font-size: 0.8rem;">{l.height_hint.clone()}</span>
                        </div>
                        {is_xml.then(|| view! {
                            <label class="control-group">
                                <input type="checkbox"
                                    prop:checked=move || planning.get()
                                    on:change=move |ev| planning.set(event_target_checked(&ev))
                                />
                                <span>{l.planning.clone()}</span>
                            </label>
                        })}
                    </div>
                    {xml_angular.then(|| view! {
                        <div class="isolux-controls">
                            <div class="control-group">
                                <span>{l.motion.clone()}</span>
                                {Motion::ALL.iter().map(|m| {
                                    let m = *m;
                                    let label = motion_label(&l, m);
                                    view! {
                                        <button
                                            class=move || format!("btn btn-sm {}", if motion.get() == m { "btn-primary" } else { "btn-outline" })
                                            on:click=move |_| motion.set(m)
                                        >{label}</button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    })}
                    {c_plane_control}
                    {derived_note.map(|n| view! { <p class="area-hint">{n}</p> })}
                    {if nothing {
                        view! { <p class="area-hint">{l.nothing_detected.clone()}</p> }.into_any()
                    } else {
                        view! { <div class="sensor-diagram" inner_html=svg /> }.into_any()
                    }}
                    <table class="area-stats-table">
                        <thead><tr>
                            <th>{l.motion.clone()}</th>
                            <th>{l.max_range.clone()}</th>
                            <th>{l.area.clone()}</th>
                            <th>{l.diameter.clone()}</th>
                            {show_quality.then(|| view! { <th>{l.quality.clone()}</th> })}
                        </tr></thead>
                        <tbody>
                            {Motion::ALL.iter().map(|m| {
                                let name = motion_label(&l, *m);
                                match rows.iter().find(|r| r.motion == *m) {
                                    Some(r) if r.max_range <= 0.0 => {
                                        let mut t = l.not_detected.clone();
                                        if let Some(d) = r.depth_limit {
                                            t = format!("{t} · {}", l.volume_ends.replace("{d}", &format!("{d:.2}")));
                                        }
                                        view! { <tr><td>{name}</td><td colspan="4">{t}</td></tr> }.into_any()
                                    }
                                    Some(r) => view! {
                                        <tr>
                                            <td>{name}</td>
                                            <td>{format!("{:.2} m", r.max_range)}</td>
                                            <td>{format!("{:.1} m²", r.area)}</td>
                                            <td>{format!("{:.2} m", r.diameter)}</td>
                                            {show_quality.then(|| view! {
                                                <td>{r.quality.map(|q| quality_label(&l, q)).unwrap_or_default()}</td>
                                            })}
                                        </tr>
                                    }.into_any(),
                                    None => view! {
                                        <tr><td>{name}</td><td colspan="4">{l.no_data.clone()}</td></tr>
                                    }.into_any(),
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                    <ul class="area-hint">
                        {info.into_iter().map(|s| view! { <li>{s}</li> }).collect_view()}
                    </ul>
                    <p class="area-hint">{l.not_applicable.clone()}</p>
                    {entries.get().iter().any(|e| e.name.starts_with("Demo ·")).then(|| view! {
                        <p class="area-hint">{l.demo_note.clone()}</p>
                    })}
                }.into_any()
            }}
        </div>
    }
}
