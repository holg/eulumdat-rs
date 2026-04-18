//! Dashboard view — parameter grid with expandable detail panels.
//!
//! Supports switchable layout templates:
//! - **Standard** — compact grid with Polar + Cartesian + Isolux + BUG detail diagrams.
//! - **AEC** — isometric 3D ISO view + Polar + BUG, with sidebar diagram modes.
//! - **Alternative** — wider grid with download badge columns and sidebar-driven
//!   diagram modes (ISO curve, beam angles, beam intensities, UGR table).
//!
//! **Selection model:**
//! - Click a row → select it (primary selection, blue highlight).
//! - Ctrl/Cmd-click a different row → compare target (green highlight).
//! - Click the already-selected row → toggle its detail panel open/closed.
//!
//! **Sidebar diagram mode:**
//! Each sidebar icon selects a *diagram mode* for the expanded detail area.
//! The magnifier button opens a **zoom overlay** of the currently active diagram.
//!
//! Template preference is persisted to `localStorage`.

use super::beam_angle_diagram::BeamAngleDiagram;
use super::bug_rating::BugRating;
use super::cartesian_diagram::CartesianDiagram;
use super::cone_diagram::ConeDiagramView;
use super::isocandela_diagram::IsocandelaDiagramView;
use super::isolux_aec::IsoluxAec;
use super::isolux_footprint::IsoluxFootprint;
use super::isolux_isometric::IsoluxIsometric;
use super::polar_diagram::PolarDiagram;
use super::templates::{TemplateFormat, ALL_TEMPLATES};
use crate::i18n::use_locale;
use eulumdat::{Eulumdat, IesParser};
use leptos::prelude::*;

// ── Dashboard Template Config ─────────────────────────────────────

const TEMPLATE_STORAGE_KEY: &str = "eulumdat_dashboard_template";

/// Which dashboard layout to use.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DashboardTemplate {
    #[default]
    Default,
    Aec,
    Alternative,
}

impl DashboardTemplate {
    fn label(self) -> &'static str {
        match self {
            Self::Default => "Standard",
            Self::Aec => "AEC",
            Self::Alternative => "Alternative",
        }
    }

    fn storage_key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Aec => "aec",
            Self::Alternative => "alternative",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "aec" => Self::Aec,
            "alternative" => Self::Alternative,
            _ => Self::Default,
        }
    }

    fn all() -> &'static [DashboardTemplate] {
        &[Self::Default, Self::Aec, Self::Alternative]
    }
}

fn load_dashboard_template() -> DashboardTemplate {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                if let Some(val) = params.get("template").or_else(|| params.get("dashboard")) {
                    return DashboardTemplate::from_str(&val.to_lowercase());
                }
            }
        }
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(val)) = storage.get_item(TEMPLATE_STORAGE_KEY) {
                return DashboardTemplate::from_str(&val);
            }
        }
    }
    DashboardTemplate::default()
}

fn save_dashboard_template(template: DashboardTemplate) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(TEMPLATE_STORAGE_KEY, template.storage_key());
        }
    }
}

// ── Column definitions ────────────────────────────────────────────

#[derive(Clone)]
struct ColumnDef {
    label: String,
    min_width: &'static str,
    extract: fn(&Eulumdat) -> String,
}

impl PartialEq for ColumnDef {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label && self.min_width == other.min_width
    }
}

// ── Diagram mode ──────────────────────────────────────────────────

/// Which diagram set is displayed in the detail panel.
/// The sidebar icons switch between these modes.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum DiagramMode {
    /// 4 diagrams: ISO curve + Polar + Isolux + BUG (AEC default, slide 2)
    #[default]
    Overview,
    /// Beam angle diagrams side-by-side with C-plane sliders (slide 5)
    BeamAngles,
    /// Beam intensities / cone diagram (slide 6)
    BeamIntensities,
    /// UGR table (slide 7)
    UgrTable,
}

impl DiagramMode {
    fn icon(self) -> &'static str {
        match self {
            Self::Overview => "\u{1F4CA}",        // bar chart
            Self::BeamAngles => "\u{1F4D0}",      // triangular ruler
            Self::BeamIntensities => "\u{1F4A1}", // light bulb
            Self::UgrTable => "\u{1F4CB}",        // clipboard
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview (ISO + Polar + Isolux + BUG)",
            Self::BeamAngles => "Beam Angles",
            Self::BeamIntensities => "Beam Intensities / Cone",
            Self::UgrTable => "UGR Table",
        }
    }

    fn all() -> &'static [DiagramMode] {
        &[
            Self::Overview,
            Self::BeamAngles,
            Self::BeamIntensities,
            Self::UgrTable,
        ]
    }
}

/// Which single diagram is in the expanded detail panel (for zoom targeting).
#[derive(Clone, Copy, PartialEq, Eq)]
enum DiagramSlot {
    Polar,
    Cartesian,
    Isolux,
    IsoluxAec,
    IsoluxIso,
    BugRating,
    Isocandela,
    Cone,
    BeamAngle,
}

#[derive(Clone, PartialEq)]
struct DashboardConfig {
    columns: Vec<ColumnDef>,
    /// Whether sidebar icons switch diagram modes (AEC) or are fixed (Default).
    has_diagram_modes: bool,
    /// Default diagram slots when `has_diagram_modes` is false.
    default_diagrams: Vec<DiagramSlot>,
}

fn default_config(cols: &eulumdat_i18n::DashboardColumns) -> DashboardConfig {
    DashboardConfig {
        columns: vec![
            ColumnDef {
                label: cols.name.clone(),
                min_width: "150px",
                extract: extract_name,
            },
            ColumnDef {
                label: cols.cct.clone(),
                min_width: "70px",
                extract: extract_cct,
            },
            ColumnDef {
                label: cols.power.clone(),
                min_width: "60px",
                extract: extract_power,
            },
            ColumnDef {
                label: cols.lumens.clone(),
                min_width: "80px",
                extract: extract_lumens,
            },
            ColumnDef {
                label: cols.lor.clone(),
                min_width: "55px",
                extract: extract_lor,
            },
            ColumnDef {
                label: cols.bug.clone(),
                min_width: "80px",
                extract: extract_bug,
            },
            ColumnDef {
                label: cols.cri.clone(),
                min_width: "50px",
                extract: extract_cri,
            },
            ColumnDef {
                label: cols.beam.clone(),
                min_width: "60px",
                extract: extract_beam,
            },
        ],
        has_diagram_modes: false,
        default_diagrams: vec![
            DiagramSlot::Polar,
            DiagramSlot::Cartesian,
            DiagramSlot::IsoluxIso,
            DiagramSlot::Isolux,
            DiagramSlot::BugRating,
        ],
    }
}

fn aec_config(cols: &eulumdat_i18n::DashboardColumns) -> DashboardConfig {
    DashboardConfig {
        columns: vec![
            ColumnDef {
                label: cols.light_dist.clone(),
                min_width: "150px",
                extract: extract_name,
            },
            ColumnDef {
                label: cols.optics.clone(),
                min_width: "60px",
                extract: extract_optics,
            },
            ColumnDef {
                label: cols.cct.clone(),
                min_width: "55px",
                extract: extract_cct,
            },
            ColumnDef {
                label: cols.power.clone(),
                min_width: "55px",
                extract: extract_power,
            },
            ColumnDef {
                label: cols.lumens.clone(),
                min_width: "70px",
                extract: extract_lumens,
            },
            ColumnDef {
                label: cols.bug.clone(),
                min_width: "75px",
                extract: extract_bug,
            },
            ColumnDef {
                label: cols.cri.clone(),
                min_width: "40px",
                extract: extract_cri,
            },
        ],
        has_diagram_modes: true,
        default_diagrams: vec![
            DiagramSlot::IsoluxIso,
            DiagramSlot::Polar,
            DiagramSlot::IsoluxAec,
            DiagramSlot::BugRating,
        ],
    }
}

fn alternative_config(cols: &eulumdat_i18n::DashboardColumns) -> DashboardConfig {
    DashboardConfig {
        columns: vec![
            ColumnDef {
                label: cols.light_dist.clone(),
                min_width: "150px",
                extract: extract_name,
            },
            ColumnDef {
                label: cols.optics.clone(),
                min_width: "60px",
                extract: extract_optics,
            },
            ColumnDef {
                label: cols.modules.clone(),
                min_width: "55px",
                extract: extract_modules,
            },
            ColumnDef {
                label: cols.cct.clone(),
                min_width: "55px",
                extract: extract_cct,
            },
            ColumnDef {
                label: cols.current.clone(),
                min_width: "60px",
                extract: extract_current,
            },
            ColumnDef {
                label: cols.power.clone(),
                min_width: "55px",
                extract: extract_power,
            },
            ColumnDef {
                label: cols.lumens.clone(),
                min_width: "70px",
                extract: extract_lumens,
            },
            ColumnDef {
                label: cols.bug.clone(),
                min_width: "75px",
                extract: extract_bug,
            },
            ColumnDef {
                label: cols.cri.clone(),
                min_width: "40px",
                extract: extract_cri,
            },
            // Icon-style download columns (values shown as badges)
            ColumnDef {
                label: cols.spec.clone(),
                min_width: "35px",
                extract: extract_spec_badge,
            },
            ColumnDef {
                label: cols.ies.clone(),
                min_width: "35px",
                extract: extract_ies_badge,
            },
            ColumnDef {
                label: cols.cad.clone(),
                min_width: "35px",
                extract: extract_cad_badge,
            },
            ColumnDef {
                label: cols.bim.clone(),
                min_width: "35px",
                extract: extract_bim_badge,
            },
        ],
        has_diagram_modes: true,
        default_diagrams: vec![
            DiagramSlot::Isocandela,
            DiagramSlot::Polar,
            DiagramSlot::Isolux,
            DiagramSlot::BugRating,
        ],
    }
}

/// Get diagrams for a given mode (AEC sidebar-driven).
fn diagrams_for_mode(mode: DiagramMode) -> Vec<DiagramSlot> {
    match mode {
        DiagramMode::Overview => vec![
            DiagramSlot::Isocandela,
            DiagramSlot::Polar,
            DiagramSlot::Isolux,
            DiagramSlot::BugRating,
        ],
        DiagramMode::BeamAngles => vec![DiagramSlot::BeamAngle, DiagramSlot::BeamAngle],
        DiagramMode::BeamIntensities => vec![DiagramSlot::Cone],
        DiagramMode::UgrTable => vec![DiagramSlot::BugRating],
    }
}

// ── Extractors ────────────────────────────────────────────────────

fn extract_name(ldt: &Eulumdat) -> String {
    if !ldt.luminaire_name.is_empty() {
        ldt.luminaire_name.clone()
    } else if !ldt.identification.is_empty() {
        ldt.identification.clone()
    } else if !ldt.luminaire_number.is_empty() {
        ldt.luminaire_number.clone()
    } else {
        String::new()
    }
}

fn extract_cct(ldt: &Eulumdat) -> String {
    ldt.lamp_sets
        .first()
        .map(|ls| ls.color_appearance.clone())
        .unwrap_or_default()
}

fn extract_power(ldt: &Eulumdat) -> String {
    format!("{:.0}W", ldt.total_wattage())
}

fn extract_lumens(ldt: &Eulumdat) -> String {
    format!("{:.0} lm", ldt.total_luminous_flux())
}

fn extract_lor(ldt: &Eulumdat) -> String {
    format!("{:.0}%", ldt.light_output_ratio)
}

fn extract_bug(ldt: &Eulumdat) -> String {
    let r = eulumdat::bug_rating::BugRating::from_eulumdat(ldt);
    format!("B{} U{} G{}", r.b, r.u, r.g)
}

fn extract_cri(ldt: &Eulumdat) -> String {
    ldt.lamp_sets
        .first()
        .map(|ls| ls.color_rendering_group.clone())
        .unwrap_or_default()
}

fn extract_beam(ldt: &Eulumdat) -> String {
    let s = eulumdat::PhotometricSummary::from_eulumdat(ldt);
    format!("{:.0}\u{00b0}", s.beam_angle)
}

fn extract_optics(ldt: &Eulumdat) -> String {
    let s = eulumdat::PhotometricSummary::from_eulumdat(ldt);
    if s.beam_angle < 30.0 {
        "Narrow".into()
    } else if s.beam_angle < 60.0 {
        "Medium".into()
    } else if s.beam_angle < 100.0 {
        "Wide".into()
    } else {
        "V. Wide".into()
    }
}

fn extract_modules(ldt: &Eulumdat) -> String {
    let n: i32 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.num_lamps.unsigned_abs() as i32)
        .sum();
    if n > 0 {
        format!("{}", n)
    } else {
        "-".into()
    }
}

fn extract_current(ldt: &Eulumdat) -> String {
    let w = ldt.total_wattage();
    if w > 0.0 {
        format!("{:.0}mA", (w / 48.0) * 1000.0)
    } else {
        "-".into()
    }
}

// AEC badge columns — show colored dot to indicate availability
fn extract_spec_badge(_ldt: &Eulumdat) -> String {
    "\u{1F7E1}".into()
} // yellow circle
fn extract_ies_badge(_ldt: &Eulumdat) -> String {
    "\u{1F7E2}".into()
} // green circle
fn extract_cad_badge(_ldt: &Eulumdat) -> String {
    "\u{1F535}".into()
} // blue circle
fn extract_bim_badge(_ldt: &Eulumdat) -> String {
    "\u{1F7E0}".into()
} // orange circle

// ── Grid CSS helpers ──────────────────────────────────────────────

fn grid_template_columns(config: &DashboardConfig) -> String {
    let mut cols = String::from("40px ");
    for col in &config.columns {
        cols.push_str(&format!("minmax({}, 1fr) ", col.min_width));
    }
    cols.push_str("40px");
    cols
}

// ── Parsed entry ──────────────────────────────────────────────────

#[derive(Clone)]
struct LuminaireEntry {
    name: String,
    ldt: Eulumdat,
}

type RowId = usize;
const USER_FILE_ROW: RowId = usize::MAX;

// ── Dashboard Component ───────────────────────────────────────────

#[component]
pub fn Dashboard(
    ldt: ReadSignal<Eulumdat>,
    on_select: Callback<(Eulumdat, String)>,
    on_compare: Callback<(Eulumdat, String)>,
    on_edit: Callback<()>,
    on_compare_view: Callback<()>,
    on_export_pdf: Callback<Eulumdat>,
    on_designer: Callback<()>,
) -> impl IntoView {
    let locale = use_locale();

    // ── Template selection ──────────────────────────────────────
    let (template, set_template) = signal(load_dashboard_template());
    Effect::new(move |_| save_dashboard_template(template.get()));

    let config = Memo::new(move |_| {
        let cols = locale.get().dashboard.columns;
        match template.get() {
            DashboardTemplate::Default => default_config(&cols),
            DashboardTemplate::Aec => aec_config(&cols),
            DashboardTemplate::Alternative => alternative_config(&cols),
        }
    });

    let grid_style = Memo::new(move |_| {
        format!(
            "grid-template-columns: {}",
            grid_template_columns(&config.get())
        )
    });

    // ── Diagram mode (sidebar-driven for AEC) ───────────────────
    let (diagram_mode, set_diagram_mode) = signal(DiagramMode::default());

    // ── Zoom overlay ────────────────────────────────────────────
    let (zoom_open, set_zoom_open) = signal(false);
    let (zoom_ldt, set_zoom_ldt) = signal(None::<Eulumdat>);
    let (zoom_slot, set_zoom_slot) = signal(DiagramSlot::Polar);

    // ── Row state ───────────────────────────────────────────────
    let (expanded_idx, set_expanded_idx) = signal(None::<RowId>);
    let (selected_idx, set_selected_idx) = signal(None::<RowId>);
    let (compare_idx, set_compare_idx) = signal(None::<RowId>);

    let (entries, set_entries) = signal(Vec::<LuminaireEntry>::new());
    let (loading, set_loading) = signal(true);

    // Load templates (embedded directly in WASM via include_str!)
    Effect::new(move |_| {
        set_loading.set(true);
        let mut loaded = Vec::new();
        for tpl in ALL_TEMPLATES.iter() {
            match tpl.format {
                TemplateFormat::Ldt | TemplateFormat::IesLm63 => {}
                _ => continue,
            }
            if let Some(content) = eulumdat_wasm_templates::get_template_content(tpl.id) {
                let parsed = match tpl.format {
                    TemplateFormat::Ldt => Eulumdat::parse(&content).ok(),
                    TemplateFormat::IesLm63 => IesParser::parse(&content).ok(),
                    _ => None,
                };
                if let Some(ldt) = parsed {
                    loaded.push(LuminaireEntry {
                        name: tpl.name.to_string(),
                        ldt,
                    });
                }
            }
        }
        set_entries.set(loaded);
        set_loading.set(false);
    });

    // ── Row click handler ───────────────────────────────────────
    let make_row_handler = move |row_id: RowId, entry_ldt: Eulumdat, label: String| {
        Callback::new(move |ctrl: bool| {
            if ctrl {
                let current = compare_idx.get_untracked();
                if current == Some(row_id) {
                    set_compare_idx.set(None);
                } else {
                    set_compare_idx.set(Some(row_id));
                    on_compare.run((entry_ldt.clone(), label.clone()));
                }
            } else {
                let current = selected_idx.get_untracked();
                if current == Some(row_id) {
                    set_expanded_idx.update(|v| {
                        *v = if *v == Some(row_id) {
                            None
                        } else {
                            Some(row_id)
                        };
                    });
                } else {
                    set_selected_idx.set(Some(row_id));
                    set_expanded_idx.set(Some(row_id));
                    on_select.run((entry_ldt.clone(), label.clone()));
                }
            }
        })
    };

    // ── Sidebar derived state ───────────────────────────────────
    let (has_selection, set_has_selection) = signal(false);
    let (has_compare_pair, set_has_compare_pair) = signal(false);

    Effect::new(move |_| {
        set_has_selection.set(selected_idx.get().is_some());
        set_has_compare_pair.set(selected_idx.get().is_some() && compare_idx.get().is_some());
    });

    // Zoom: open overlay with the first diagram of the active mode for the selected row
    let on_zoom = move || {
        if let Some(row_id) = selected_idx.get_untracked() {
            let sel_ldt = if row_id == USER_FILE_ROW {
                Some(ldt.get_untracked())
            } else {
                entries.get_untracked().get(row_id).map(|e| e.ldt.clone())
            };
            if let Some(l) = sel_ldt {
                let cfg = config.get_untracked();
                let diagrams = if cfg.has_diagram_modes {
                    diagrams_for_mode(diagram_mode.get_untracked())
                } else {
                    cfg.default_diagrams.clone()
                };
                let slot = diagrams.first().copied().unwrap_or(DiagramSlot::Polar);
                set_zoom_ldt.set(Some(l));
                set_zoom_slot.set(slot);
                set_zoom_open.set(true);
            }
        }
    };

    let on_compare_btn = move || {
        if selected_idx.get_untracked().is_some() && compare_idx.get_untracked().is_some() {
            on_compare_view.run(());
        }
    };

    let on_print = move || {
        if let Some(row_id) = selected_idx.get_untracked() {
            let sel_ldt = if row_id == USER_FILE_ROW {
                Some(ldt.get_untracked())
            } else {
                entries.get_untracked().get(row_id).map(|e| e.ldt.clone())
            };
            if let Some(l) = sel_ldt {
                on_export_pdf.run(l);
            }
        }
    };

    // Helper: get current diagrams to show
    let active_diagrams = move || {
        let cfg = config.get();
        if cfg.has_diagram_modes {
            let mode = diagram_mode.get();
            if mode == DiagramMode::Overview {
                // Overview uses the template's default_diagrams
                cfg.default_diagrams.clone()
            } else {
                diagrams_for_mode(mode)
            }
        } else {
            cfg.default_diagrams.clone()
        }
    };

    // ── View ────────────────────────────────────────────────────
    view! {
        <div class="dashboard-layout">
            <DashboardSidebar
                on_edit=on_edit
                has_selection=has_selection.into()
                has_compare_pair=has_compare_pair.into()
                has_diagram_modes=Memo::new(move |_| config.get().has_diagram_modes)
                diagram_mode=diagram_mode
                set_diagram_mode=set_diagram_mode
                on_zoom=Callback::new(move |_| on_zoom())
                on_compare_view=Callback::new(move |_| on_compare_btn())
                on_print=Callback::new(move |_| on_print())
                on_designer=on_designer
            />

            <div class="dashboard-main">
                // Template switcher
                <div class="dashboard-toolbar">
                    <select
                        class="template-select"
                        on:change=move |ev| {
                            let val = leptos::prelude::event_target_value(&ev);
                            set_template.set(DashboardTemplate::from_str(&val));
                        }
                    >
                        {DashboardTemplate::all().iter().map(|&t| {
                            let label = move || {
                                let d = locale.get().dashboard;
                                match t {
                                    DashboardTemplate::Default => d.templates.standard.clone(),
                                    DashboardTemplate::Aec => d.templates.aec.clone(),
                                    DashboardTemplate::Alternative => d.templates.alternative.clone(),
                                }
                            };
                            view! {
                                <option
                                    value=t.storage_key()
                                    selected=move || template.get() == t
                                >
                                    {label}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                // Column headers
                <div class="params-header" style=move || grid_style.get()>
                    <div class="params-col params-col-expand"></div>
                    {move || config.get().columns.into_iter().map(|col| {
                        let lbl = col.label;
                        view! { <div class="params-col">{lbl}</div> }
                    }).collect_view()}
                    <div class="params-col params-col-actions"></div>
                </div>

                // User-loaded file row
                {move || {
                    let l = ldt.get();
                    let is_default = l.luminaire_name.is_empty()
                        && l.identification.is_empty()
                        && l.intensities.is_empty();
                    if is_default {
                        None
                    } else {
                        let is_expanded = expanded_idx.get() == Some(USER_FILE_ROW);
                        let is_selected = selected_idx.get() == Some(USER_FILE_ROW);
                        let is_compare = compare_idx.get() == Some(USER_FILE_ROW);
                        let user_ldt = ldt.get();
                        let label = locale.get().dashboard.loaded_file.clone();
                        let cfg = config.get();
                        let mut cells: Vec<String> = cfg.columns.iter()
                            .map(|c| (c.extract)(&user_ldt))
                            .collect();
                        if !cells.is_empty() && cells[0].is_empty() {
                            cells[0] = label.clone();
                        }
                        let diagrams = active_diagrams();
                        let gs = grid_style.get();
                        Some(view! {
                            <LuminaireRowStatic
                                cells=cells
                                label=label.clone()
                                grid_style=gs.clone()
                                expanded=is_expanded
                                selected=is_selected
                                compare=is_compare
                                on_toggle=make_row_handler(USER_FILE_ROW, user_ldt, label)
                            />
                            {is_expanded.then(|| view! {
                                <LuminaireDetailStatic ldt=ldt.get() diagrams=diagrams />
                            })}
                        })
                    }
                }}

                // Loading
                {move || loading.get().then(|| view! {
                    <div class="dashboard-loading">
                        <span class="loading-text">{move || locale.get().dashboard.loading_templates.clone()}</span>
                    </div>
                })}

                // Template rows
                {move || {
                    let items = entries.get();
                    let current_expanded = expanded_idx.get();
                    let current_selected = selected_idx.get();
                    let current_compare = compare_idx.get();
                    let cfg = config.get();
                    let gs = grid_style.get();
                    let diagrams_template = active_diagrams();
                    items.into_iter().enumerate().map(|(idx, entry)| {
                        let is_expanded = current_expanded == Some(idx);
                        let is_selected = current_selected == Some(idx);
                        let is_compare = current_compare == Some(idx);
                        let mut cells: Vec<String> = cfg.columns.iter()
                            .map(|c| (c.extract)(&entry.ldt))
                            .collect();
                        if !cells.is_empty() && cells[0].is_empty() {
                            cells[0] = entry.name.clone();
                        }
                        let diagrams = diagrams_template.clone();
                        let entry_ldt = entry.ldt.clone();
                        let detail_ldt = entry.ldt.clone();
                        view! {
                            <LuminaireRowStatic
                                cells=cells
                                label=entry.name.clone()
                                grid_style=gs.clone()
                                expanded=is_expanded
                                selected=is_selected
                                compare=is_compare
                                on_toggle=make_row_handler(idx, entry_ldt, entry.name.clone())
                            />
                            {is_expanded.then(|| {
                                let ldt = detail_ldt.clone();
                                view! { <LuminaireDetailStatic ldt=ldt diagrams=diagrams /> }
                            })}
                        }
                    }).collect_view()
                }}

                // Footer
                {move || {
                    if !loading.get() {
                        let count = entries.get().len();
                        let luminaires_text = locale.get().dashboard.luminaires_count
                            .replace("{}", &count.to_string());
                        let click_text = locale.get().dashboard.click_to_select.clone();
                        Some(view! {
                            <div class="dashboard-footer">
                                {luminaires_text}
                                <span class="dashboard-hint">
                                    " \u{2022} "{click_text}" \u{2022} "
                                    <kbd>"Ctrl"</kbd>" / "<kbd>"\u{2318}"</kbd>" + click to compare"
                                </span>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
            </div>

            // Zoom overlay
            {move || {
                if zoom_open.get() {
                    if let Some(z_ldt) = zoom_ldt.get() {
                        let active_slot = zoom_slot.get();
                        Some(view! {
                            <ZoomOverlay
                                ldt=z_ldt
                                diagram=active_slot
                                on_close=Callback::new(move |_| set_zoom_open.set(false))
                            />
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }}
        </div>
    }
}

// ── Sidebar ───────────────────────────────────────────────────────

#[component]
fn DashboardSidebar(
    on_edit: Callback<()>,
    has_selection: ReadSignal<bool>,
    has_compare_pair: ReadSignal<bool>,
    has_diagram_modes: Memo<bool>,
    diagram_mode: ReadSignal<DiagramMode>,
    set_diagram_mode: WriteSignal<DiagramMode>,
    on_zoom: Callback<()>,
    on_compare_view: Callback<()>,
    on_print: Callback<()>,
    on_designer: Callback<()>,
) -> impl IntoView {
    let locale = use_locale();

    view! {
        <aside class="dashboard-sidebar">
            // Edit
            <button
                class="sidebar-icon-btn"
                title=move || locale.get().dashboard.edit_data.clone()
                on:click=move |_| on_edit.run(())
            >
                "\u{270F}"
            </button>

            // Zoom overlay
            <button
                class=move || format!("sidebar-icon-btn{}", if has_selection.get() { "" } else { " disabled" })
                title=move || locale.get().dashboard.zoom_diagram.clone()
                disabled=move || !has_selection.get()
                on:click=move |_| on_zoom.run(())
            >
                "\u{1F50D}"
            </button>

            // Diagram mode buttons (AEC only)
            {move || {
                if has_diagram_modes.get() {
                    Some(view! {
                        <div class="sidebar-divider"></div>
                        {DiagramMode::all().iter().map(|&mode| {
                            let is_active = move || diagram_mode.get() == mode;
                            let mode_title = move || {
                                let m = locale.get().dashboard.modes;
                                match mode {
                                    DiagramMode::Overview => m.overview.clone(),
                                    DiagramMode::BeamAngles => m.beam_angles.clone(),
                                    DiagramMode::BeamIntensities => m.beam_intensities.clone(),
                                    DiagramMode::UgrTable => m.ugr_table.clone(),
                                }
                            };
                            view! {
                                <button
                                    class=move || format!("sidebar-icon-btn{}", if is_active() { " active" } else { "" })
                                    title=mode_title
                                    on:click=move |_| set_diagram_mode.set(mode)
                                >
                                    {mode.icon()}
                                </button>
                            }
                        }).collect_view()}
                        <div class="sidebar-divider"></div>
                    })
                } else {
                    None
                }
            }}

            // Compare
            <button
                class=move || format!("sidebar-icon-btn{}", if has_compare_pair.get() { "" } else { " disabled" })
                title=move || locale.get().dashboard.open_compare.clone()
                disabled=move || !has_compare_pair.get()
                on:click=move |_| on_compare_view.run(())
            >
                "\u{2194}"
            </button>

            // Area Designer
            <button
                class="sidebar-icon-btn"
                title=move || locale.get().dashboard.area_designer.clone()
                on:click=move |_| on_designer.run(())
            >
                "\u{1F4D0}"  // triangular ruler 📐
            </button>

            // Print / Export PDF
            <button
                class=move || format!("sidebar-icon-btn{}", if has_selection.get() { "" } else { " disabled" })
                title=move || locale.get().dashboard.export_pdf.clone()
                disabled=move || !has_selection.get()
                on:click=move |_| on_print.run(())
            >
                "\u{1F5A8}"
            </button>
        </aside>
    }
}

// ── Zoom Overlay ──────────────────────────────────────────────────

/// Full-screen overlay showing a single diagram at large size.
#[component]
fn ZoomOverlay(ldt: Eulumdat, diagram: DiagramSlot, on_close: Callback<()>) -> impl IntoView {
    let (ldt_sig, _) = signal(ldt);

    // Close on Escape key
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            on_close.run(());
        }
    };

    view! {
        <div
            class="zoom-overlay"
            on:click=move |_| on_close.run(())
            on:keydown=on_keydown
            tabindex="-1"
        >
            <div class="zoom-overlay-content" on:click=move |ev: leptos::ev::MouseEvent| {
                ev.stop_propagation(); // don't close when clicking the diagram
            }>
                <button class="zoom-overlay-close" on:click=move |_| on_close.run(())>
                    "\u{2715}"
                </button>
                <div class="zoom-overlay-diagram">
                    {match diagram {
                        DiagramSlot::Polar => view! { <PolarDiagram ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::Cartesian => view! { <CartesianDiagram ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::Isolux => view! { <IsoluxFootprint ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::IsoluxAec => view! { <IsoluxAec ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::IsoluxIso => view! { <IsoluxIsometric ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::BugRating => view! { <BugRating ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::Isocandela => view! { <IsocandelaDiagramView ldt=ldt_sig /> }.into_any(),
                        DiagramSlot::Cone => {
                            let (mh, _) = signal(3.0_f64);
                            let (cp, _) = signal(None::<f64>);
                            view! { <ConeDiagramView ldt=ldt_sig mounting_height=mh c_plane=cp /> }.into_any()
                        },
                        DiagramSlot::BeamAngle => view! { <BeamAngleDiagram ldt=ldt_sig /> }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Dynamic Row ───────────────────────────────────────────────────

#[component]
fn LuminaireRowStatic(
    cells: Vec<String>,
    label: String,
    grid_style: String,
    expanded: bool,
    #[prop(optional)] selected: bool,
    #[prop(optional)] compare: bool,
    on_toggle: Callback<bool>,
) -> impl IntoView {
    let mut classes = String::from("luminaire-row");
    if expanded {
        classes.push_str(" expanded");
    }
    if selected {
        classes.push_str(" selected");
    }
    if compare {
        classes.push_str(" compare-target");
    }

    let arrow_class = if expanded {
        "expand-arrow open"
    } else {
        "expand-arrow"
    };

    view! {
        <div class=classes style=grid_style on:click=move |ev: leptos::ev::MouseEvent| {
            let ctrl = ev.ctrl_key() || ev.meta_key();
            on_toggle.run(ctrl);
        }>
            <div class="params-col params-col-expand">
                <span class=arrow_class>"\u{25B6}"</span>
            </div>
            {cells.into_iter().enumerate().map(|(i, val)| {
                let class = if i == 0 { "params-col luminaire-name" } else { "params-col" };
                let title = if i == 0 { label.clone() } else { String::new() };
                view! { <div class=class title=title>{val}</div> }
            }).collect_view()}
            <div class="params-col params-col-actions"></div>
        </div>
    }
}

// ── Dynamic Detail Panel ──────────────────────────────────────────

#[component]
fn LuminaireDetailStatic(ldt: Eulumdat, diagrams: Vec<DiagramSlot>) -> impl IntoView {
    let (ldt_sig, _) = signal(ldt);
    let locale = use_locale();

    view! {
        <div class="luminaire-detail">
            {diagrams.into_iter().map(|slot| {
                match slot {
                    DiagramSlot::Polar => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.polar_title.clone()}</h4>
                            <PolarDiagram ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::Cartesian => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.cartesian_title.clone()}</h4>
                            <CartesianDiagram ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::Isolux => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.iso_lux_title.clone()}</h4>
                            <IsoluxFootprint ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::IsoluxAec => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.iso_lux_title.clone()}</h4>
                            <IsoluxAec ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::IsoluxIso => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.iso_view_title.clone()}</h4>
                            <IsoluxIsometric ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::BugRating => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.bug_title.clone()}</h4>
                            <BugRating ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::Isocandela => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.iso_curve_title.clone()}</h4>
                            <IsocandelaDiagramView ldt=ldt_sig />
                        </div>
                    }.into_any(),
                    DiagramSlot::Cone => {
                        let (mh, _) = signal(3.0_f64);
                        let (cp, _) = signal(None::<f64>);
                        view! {
                            <div class="detail-diagram">
                                <h4>{move || locale.get().dashboard.beam_intensities_title.clone()}</h4>
                                <ConeDiagramView ldt=ldt_sig mounting_height=mh c_plane=cp />
                            </div>
                        }.into_any()
                    },
                    DiagramSlot::BeamAngle => view! {
                        <div class="detail-diagram">
                            <h4>{move || locale.get().dashboard.beam_angle_title.clone()}</h4>
                            <BeamAngleDiagram ldt=ldt_sig />
                        </div>
                    }.into_any(),
                }
            }).collect_view()}
        </div>
    }
}
