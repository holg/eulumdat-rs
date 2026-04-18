//! Area Lighting Designer — Phases 1-5: Core computation, interactive plan view,
//! spacing optimizer, export, undo/redo, wall-mounted support.

use eulumdat::area::{
    compute_area_illuminance, compute_area_illuminance_mixed, compute_area_illuminance_polygon,
    optimize_spacing, AreaPolygon, AreaSvg, ArrangementType, ContourOverlay, GridPreset,
    LuminairePlace, OptimizationCriteria, OptimizationRow, PoleConfig,
};
use eulumdat::diagram::SvgTheme;
use eulumdat::scene3d::{build_exterior_scene, fit_scale, render_scene_svg, CameraPreset};
use eulumdat::Eulumdat;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::use_locale;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = compileTypstToPdf, catch)]
    async fn compile_typst_to_pdf_js(source: &str) -> Result<JsValue, JsValue>;
}

async fn compile_typst_to_pdf(typst_source: &str) -> Result<Vec<u8>, String> {
    match compile_typst_to_pdf_js(typst_source).await {
        Ok(val) => {
            let arr = js_sys::Uint8Array::new(&val);
            Ok(arr.to_vec())
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

/// A single pole with editable properties.
#[derive(Debug, Clone, PartialEq)]
struct PoleState {
    id: usize,
    x: f64,
    y: f64,
    /// Per-pole height override (None = use global)
    height_override: Option<f64>,
    /// Per-pole tilt override
    tilt: f64,
    /// Per-pole rotation (C0 direction)
    rotation: f64,
    /// Index into extra_ldts (0 = primary LDT, 1+ = loaded extras)
    ldt_index: usize,
}

/// An additional LDT loaded for mixed luminaire types.
#[derive(Debug, Clone)]
struct ExtraLdt {
    name: String,
    ldt: Eulumdat,
}

// ── URL state serialization ──────────────────────────────────────────────────

/// All shareable designer parameters.
struct DesignerParams {
    grid_preset: GridPreset,
    arrangement: ArrangementType,
    mounting_height: f64,
    arm_length: f64,
    arm_droop: f64,
    area_width: f64,
    area_depth: f64,
    proration: f64,
    grid_resolution: usize,
    base_rotation: f64,
    bay_view: bool,
    poles: Vec<PoleState>,
    polygon: Option<AreaPolygon>,
}

fn grid_preset_to_str(g: &GridPreset) -> String {
    match g {
        GridPreset::Single => "single".to_string(),
        GridPreset::Pair => "pair".to_string(),
        GridPreset::Row3 => "row3".to_string(),
        GridPreset::Grid2x2 => "grid2x2".to_string(),
        GridPreset::Grid2x3 => "grid2x3".to_string(),
        GridPreset::Grid3x3 => "grid3x3".to_string(),
        GridPreset::Perimeter(n) => format!("perim{n}"),
    }
}

fn str_to_grid_preset(s: &str) -> GridPreset {
    if let Some(rest) = s.strip_prefix("perim") {
        let n = rest.parse::<usize>().unwrap_or(8);
        return GridPreset::Perimeter(n);
    }
    match s {
        "single" => GridPreset::Single,
        "pair" => GridPreset::Pair,
        "row3" => GridPreset::Row3,
        "grid2x2" => GridPreset::Grid2x2,
        "grid2x3" => GridPreset::Grid2x3,
        "grid3x3" => GridPreset::Grid3x3,
        _ => GridPreset::Grid2x3,
    }
}

fn arrangement_to_str(a: &ArrangementType) -> &'static str {
    match a {
        ArrangementType::Single => "single",
        ArrangementType::BackToBack => "b2b",
        ArrangementType::TwinArm => "twin",
        ArrangementType::Quad => "quad",
        ArrangementType::WallMounted => "wall",
    }
}

fn str_to_arrangement(s: &str) -> ArrangementType {
    match s {
        "single" => ArrangementType::Single,
        "b2b" => ArrangementType::BackToBack,
        "twin" => ArrangementType::TwinArm,
        "quad" => ArrangementType::Quad,
        "wall" => ArrangementType::WallMounted,
        _ => ArrangementType::Single,
    }
}

/// Encode poles as compact string: "x,y,h,t,r;x,y,h,t,r;..."
/// h is empty if no height override.
fn encode_poles(poles: &[PoleState]) -> String {
    poles
        .iter()
        .map(|p| {
            let h = match p.height_override {
                Some(v) => format!("{v:.2}"),
                None => String::new(),
            };
            if p.ldt_index > 0 {
                format!(
                    "{:.2},{:.2},{},{:.0},{:.0},{}",
                    p.x, p.y, h, p.tilt, p.rotation, p.ldt_index
                )
            } else {
                format!(
                    "{:.2},{:.2},{},{:.0},{:.0}",
                    p.x, p.y, h, p.tilt, p.rotation
                )
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Decode poles from compact string.
fn decode_poles(s: &str) -> Vec<PoleState> {
    s.split(';')
        .enumerate()
        .filter_map(|(i, part)| {
            let fields: Vec<&str> = part.split(',').collect();
            if fields.len() < 5 {
                return None;
            }
            Some(PoleState {
                id: i,
                x: fields[0].parse().ok()?,
                y: fields[1].parse().ok()?,
                height_override: if fields[2].is_empty() {
                    None
                } else {
                    Some(fields[2].parse().ok()?)
                },
                tilt: fields[3].parse().ok()?,
                rotation: fields[4].parse().ok()?,
                ldt_index: fields.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            })
        })
        .collect()
}

/// Build a URL hash string from designer params.
fn params_to_hash(p: &DesignerParams) -> String {
    let mut parts = vec![
        format!("layout={}", grid_preset_to_str(&p.grid_preset)),
        format!("arr={}", arrangement_to_str(&p.arrangement)),
        format!("h={:.2}", p.mounting_height),
        format!("arm={:.2}", p.arm_length),
        format!("droop={:.1}", p.arm_droop),
        format!("w={:.1}", p.area_width),
        format!("d={:.1}", p.area_depth),
        format!("rot={:.0}", p.base_rotation),
    ];
    // Only include non-default values to keep URL short
    if (p.proration - 1.0).abs() > 0.01 {
        parts.push(format!("pf={:.2}", p.proration));
    }
    if p.grid_resolution != 40 {
        parts.push(format!("res={}", p.grid_resolution));
    }
    if p.bay_view {
        parts.push("bay=1".to_string());
    }
    if !p.poles.is_empty() {
        parts.push(format!("poles={}", encode_poles(&p.poles)));
    }
    if let Some(ref poly) = p.polygon {
        let s: String = poly
            .vertices
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(";");
        parts.push(format!("poly={s}"));
    }
    format!("#designer?{}", parts.join("&"))
}

/// Parse designer params from URL hash fragment.
/// Returns None if hash doesn't start with #designer.
fn parse_hash_params() -> Option<std::collections::HashMap<String, String>> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    if !hash.starts_with("#designer?") {
        return None;
    }
    let query = &hash["#designer?".len()..];
    let mut map = std::collections::HashMap::new();
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            // URL-decode the value
            let decoded = js_sys::decode_uri_component(v)
                .ok()
                .map(|s| s.as_string().unwrap_or_default())
                .unwrap_or_else(|| v.to_string());
            map.insert(k.to_string(), decoded);
        }
    }
    Some(map)
}

/// Update the URL hash without triggering a page reload.
fn set_url_hash(hash: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(history) = window.history() {
            let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(hash));
        }
    }
}

/// Copy current URL to clipboard.
fn copy_url_to_clipboard() {
    if let Some(window) = web_sys::window() {
        if let Ok(href) = window.location().href() {
            let _ = window.navigator().clipboard().write_text(&href);
        }
    }
}

/// Compute sensible defaults from LDT photometric data.
struct SmartDefaults {
    mounting_height: f64,
    area_width: f64,
    area_depth: f64,
    opt_height_min: f64,
    opt_height_max: f64,
}

fn compute_smart_defaults(ldt: &Eulumdat) -> SmartDefaults {
    let flux = ldt.total_luminous_flux();
    let max_gamma = ldt.g_angles.last().copied().unwrap_or(90.0);
    let dff = ldt.downward_flux_fraction; // 0-100, percentage of light going down

    // Estimate mounting height from luminous flux, beam spread, and DFF.
    // Low DFF (< 30%) = mostly uplight → lower mounting height
    let h: f64 = if dff > 0.0 && dff < 30.0 {
        // Uplight or indirect — typically low-mounted
        if flux < 5000.0 {
            2.5
        } else {
            3.0
        }
    } else if max_gamma <= 90.0 && flux < 3000.0 {
        // Small downlight / indoor spot
        3.0
    } else if flux < 5000.0 {
        // Small indoor / decorative
        3.5
    } else if flux < 15000.0 {
        // Medium indoor / small outdoor
        5.0
    } else if flux < 30000.0 {
        // Large indoor / medium outdoor
        8.0
    } else if flux < 80000.0 {
        // High-power outdoor / road
        10.0
    } else {
        // Stadium / large area floodlight
        15.0
    };

    // Area: spacing ≈ 3× height is typical for general lighting.
    // For a 2×3 grid: width = 3 spacings, depth = 2 spacings.
    let spacing: f64 = h * 3.0;
    let aw: f64 = (spacing * 3.0).max(10.0).round();
    let ad: f64 = (spacing * 2.0).max(8.0).round();

    // Optimizer height range: explore around the default
    let opt_min: f64 = (h * 0.5).max(2.5).round();
    let opt_max: f64 = (h * 2.0).min(20.0).round();

    SmartDefaults {
        mounting_height: h,
        area_width: aw.clamp(5.0, 200.0),
        area_depth: ad.clamp(5.0, 200.0),
        opt_height_min: opt_min,
        opt_height_max: opt_max,
    }
}

/// Use mixed computation if any pole has a non-zero LDT index, else single LDT.
#[allow(clippy::too_many_arguments)]
fn compute_mixed_or_single(
    primary: &Eulumdat,
    extras: &[ExtraLdt],
    placements: &[LuminairePlace],
    ldt_indices: &[usize],
    aw: f64,
    ad: f64,
    gr: usize,
    pf: f64,
) -> eulumdat::area::AreaResult {
    let has_mixed = ldt_indices.iter().any(|&i| i > 0);
    if !has_mixed || extras.is_empty() {
        compute_area_illuminance(primary, placements, aw, ad, gr, pf)
    } else {
        let mut all_ldts: Vec<&Eulumdat> = vec![primary];
        for e in extras {
            all_ldts.push(&e.ldt);
        }
        compute_area_illuminance_mixed(&all_ldts, placements, ldt_indices, aw, ad, gr, pf)
    }
}

/// Area Lighting Designer component.
#[component]
pub fn AreaDesigner(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let units = super::app::use_unit_system();

    // --- Compute smart defaults from the loaded LDT ---
    let defaults = compute_smart_defaults(&ldt.get());

    // --- Parse URL hash for initial values ---
    let url_params = parse_hash_params();
    let url_get = |key: &str| -> Option<String> { url_params.as_ref()?.get(key).cloned() };
    let url_f64 = {
        let url_params_ref = url_params.as_ref();
        move |key: &str, default: f64| -> f64 {
            url_params_ref
                .and_then(|m| m.get(key))
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(default)
        }
    };

    // --- Global parameters (URL overrides smart defaults) ---
    let (grid_preset, set_grid_preset) = signal(
        url_get("layout")
            .map(|s| str_to_grid_preset(&s))
            .unwrap_or(GridPreset::Grid2x3),
    );
    let (arrangement, set_arrangement) = signal(
        url_get("arr")
            .map(|s| str_to_arrangement(&s))
            .unwrap_or(ArrangementType::Single),
    );
    let (mounting_height, set_mounting_height) = signal(url_f64("h", defaults.mounting_height));
    let (arm_length, set_arm_length) = signal(url_f64("arm", 0.0));
    let (arm_droop, set_arm_droop) = signal(url_f64("droop", 0.0));
    let (area_width, set_area_width) = signal(url_f64("w", defaults.area_width));
    let (area_depth, set_area_depth) = signal(url_f64("d", defaults.area_depth));
    let (proration, set_proration) = signal(url_f64("pf", 1.0));
    let (grid_resolution, set_grid_resolution) = signal(
        url_get("res")
            .and_then(|v| v.parse().ok())
            .unwrap_or(40_usize),
    );
    let (base_rotation, set_base_rotation) = signal(url_f64("rot", 0.0));
    let (bay_view, set_bay_view) = signal(url_get("bay").map(|v| v == "1").unwrap_or(false));

    // --- Custom polygon area ---
    let url_poly = url_get("poly").and_then(|s| {
        let verts: Vec<(f64, f64)> = s
            .split(';')
            .filter_map(|p| {
                let mut it = p.split(',');
                Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
            })
            .collect();
        if verts.len() >= 3 {
            Some(AreaPolygon::new(verts))
        } else {
            None
        }
    });
    let (custom_polygon, set_custom_polygon) = signal(url_poly.clone());
    let (polygon_drawing, set_polygon_drawing) = signal(false);
    let (polygon_wip, set_polygon_wip) = signal(Vec::<(f64, f64)>::new());

    // --- Pole state ---
    let url_poles = url_get("poles").map(|s| decode_poles(&s));
    let (poles, set_poles) = signal(Vec::<PoleState>::new());
    let (selected_pole_id, set_selected_pole_id) = signal(Option::<usize>::None);
    let (dragging, set_dragging) = signal(false);

    // --- Undo/redo history ---
    let (undo_stack, set_undo_stack) = signal(Vec::<Vec<PoleState>>::new());
    let (redo_stack, set_redo_stack) = signal(Vec::<Vec<PoleState>>::new());
    const MAX_HISTORY: usize = 50;

    let push_undo = move || {
        let snapshot = poles.get();
        set_undo_stack.update(|stack| {
            stack.push(snapshot);
            if stack.len() > MAX_HISTORY {
                stack.remove(0);
            }
        });
        set_redo_stack.update(|stack| stack.clear());
    };

    let do_undo = move || {
        let stack = undo_stack.get();
        if let Some(prev) = stack.last().cloned() {
            let current = poles.get();
            set_redo_stack.update(|rs| rs.push(current));
            set_undo_stack.update(|us| {
                us.pop();
            });
            set_poles.set(prev);
        }
    };

    let do_redo = move || {
        let stack = redo_stack.get();
        if let Some(next) = stack.last().cloned() {
            let current = poles.get();
            set_undo_stack.update(|us| us.push(current));
            set_redo_stack.update(|rs| {
                rs.pop();
            });
            set_poles.set(next);
        }
    };

    // --- Extra LDT files for mixed luminaire types ---
    let (extra_ldts, set_extra_ldts) = signal(Vec::<ExtraLdt>::new());
    let extra_ldts_ref = NodeRef::<leptos::html::Input>::new();

    // Generate initial pole positions from grid preset
    let regenerate_poles = move || {
        push_undo();
        let preset = grid_preset.get();
        let aw = area_width.get();
        let ad = area_depth.get();
        let poly = custom_polygon.get();
        let positions = if let GridPreset::Perimeter(n) = preset {
            if let Some(ref polygon) = poly {
                eulumdat::area::layout::generate_perimeter_positions_polygon(n, polygon)
            } else {
                eulumdat::area::layout::generate_perimeter_positions(n, aw, ad)
            }
        } else {
            let (rows, cols) = preset.rows_cols();
            if let Some(ref polygon) = poly {
                eulumdat::area::layout::generate_pole_positions_in_polygon(rows, cols, polygon)
            } else {
                eulumdat::area::layout::generate_pole_positions(rows, cols, aw, ad)
            }
        };
        let new_poles: Vec<PoleState> = positions
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| PoleState {
                id: i,
                x,
                y,
                height_override: None,
                tilt: 0.0,
                rotation: 0.0,
                ldt_index: 0,
            })
            .collect();
        set_poles.set(new_poles);
        set_selected_pole_id.set(None);
    };

    // Initialize poles — from URL if available, otherwise generate from preset
    if let Some(url_pole_list) = url_poles {
        if !url_pole_list.is_empty() {
            set_poles.set(url_pole_list);
        } else {
            regenerate_poles();
        }
    } else {
        regenerate_poles();
    }

    // --- URL sync: update hash whenever parameters change ---
    let (link_copied, set_link_copied) = signal(false);
    let (pdf_exporting, set_pdf_exporting) = signal(false);
    let (show_heatmap_values, set_show_heatmap_values) = signal(false);
    // Unified view tab: "heatmap", "room", "3d"
    let (area_view_tab, set_area_view_tab) = signal("3d".to_string());
    let (_scene_camera_preset, set_scene_camera_preset) = signal(CameraPreset::FrontRight);
    let (show_light_cones, set_show_light_cones) = signal(true);

    // Interactive 3D camera
    let (area_cam_az, set_area_cam_az) = signal(30.0_f64);
    let (area_cam_el, set_area_cam_el) = signal(30.0_f64);
    let (area_cam_zoom, set_area_cam_zoom) = signal(1.0_f64);
    let (area_dragging, set_area_dragging) = signal(false);
    let (area_drag_start, set_area_drag_start) = signal((0.0_f64, 0.0_f64));
    let (area_drag_az0, set_area_drag_az0) = signal(30.0_f64);
    let (area_drag_el0, set_area_drag_el0) = signal(30.0_f64);

    let update_url_hash = move || {
        let hash = params_to_hash(&DesignerParams {
            grid_preset: grid_preset.get(),
            arrangement: arrangement.get(),
            mounting_height: mounting_height.get(),
            arm_length: arm_length.get(),
            arm_droop: arm_droop.get(),
            area_width: area_width.get(),
            area_depth: area_depth.get(),
            proration: proration.get(),
            grid_resolution: grid_resolution.get(),
            base_rotation: base_rotation.get(),
            bay_view: bay_view.get(),
            poles: poles.get(),
            polygon: custom_polygon.get(),
        });
        set_url_hash(&hash);
    };

    // Reactively sync all params to URL hash (reads all signals → triggers on any change)
    Effect::new(move |_| {
        update_url_hash();
    });

    // Convert poles to LuminairePlaces + per-placement LDT indices
    let make_placements = move || -> (Vec<LuminairePlace>, Vec<usize>) {
        let pole_list = poles.get();
        let arr = arrangement.get();
        let h = mounting_height.get();
        let al = arm_length.get();
        let droop = arm_droop.get();
        let brot = base_rotation.get();

        let pole_positions: Vec<(f64, f64)> = pole_list.iter().map(|p| (p.x, p.y)).collect();
        let pole_cfg = PoleConfig {
            arrangement: arr,
            arm_length: al,
            arm_droop: droop,
        };
        let mut placements =
            eulumdat::area::layout::generate_placements(&pole_positions, h, &pole_cfg, brot);

        // Build per-placement LDT indices and apply per-pole overrides
        let lpp = arr.luminaires_per_pole();
        let mut ldt_indices = Vec::with_capacity(placements.len());
        for (pi, pole) in pole_list.iter().enumerate() {
            let start = pi * lpp;
            let end = (start + lpp).min(placements.len());
            for lum in &mut placements[start..end] {
                if let Some(ho) = pole.height_override {
                    lum.mounting_height = ho;
                }
                lum.tilt_angle += pole.tilt;
                lum.rotation = (lum.rotation + pole.rotation) % 360.0;
                ldt_indices.push(pole.ldt_index);
            }
        }

        (placements, ldt_indices)
    };

    // Plan view SVG
    let plan_svg = move || {
        let (placements, _ldt_idx) = make_placements();
        let pole_list = poles.get();
        let aw = area_width.get();
        let ad = area_depth.get();
        let sel = selected_pole_id.get();
        let lpp = arrangement.get().luminaires_per_pole();
        let svg_theme = SvgTheme::css_variables_with_locale(&locale.get());

        let pole_positions: Vec<(usize, f64, f64)> =
            pole_list.iter().map(|p| (p.id, p.x, p.y)).collect();

        let mut svg = if let Some(ref poly) = custom_polygon.get() {
            AreaSvg::plan_view_with_polygon(
                &pole_positions,
                &placements,
                poly,
                500.0,
                400.0,
                &svg_theme,
                sel,
                lpp,
                units.get(),
            )
        } else {
            AreaSvg::plan_view_with_poles(
                &pole_positions,
                &placements,
                aw,
                ad,
                500.0,
                400.0,
                &svg_theme,
                sel,
                lpp,
                units.get(),
            )
        };

        // Overlay work-in-progress polygon vertices
        let wip = polygon_wip.get();
        if !wip.is_empty() {
            // Remove closing </svg> to append WIP overlay
            if svg.ends_with("</svg>") {
                svg.truncate(svg.len() - 6);
            }
            let margin = 40.0;
            let plot_w = 500.0 - 2.0 * margin;
            let plot_h = 400.0 - 2.0 * margin;
            let sx = plot_w / aw;
            let sy = plot_h / ad;

            // Draw WIP polyline
            let pts: String = wip
                .iter()
                .map(|&(x, y)| format!("{:.1},{:.1}", margin + x * sx, margin + y * sy))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r#"<polyline points="{pts}" fill="none" stroke="red" stroke-width="1.5" stroke-dasharray="4,3"/>"#
            ));

            // Draw vertex dots
            for (i, &(x, y)) in wip.iter().enumerate() {
                let cx = margin + x * sx;
                let cy = margin + y * sy;
                let r = if i == 0 { 5.0 } else { 3.5 };
                svg.push_str(&format!(
                    r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="rgba(220,50,50,0.7)" stroke="red" stroke-width="1"/>"#
                ));
            }
            svg.push_str("</svg>");
        }

        svg
    };

    // --- Mouse event handlers for SVG drag ---
    let plan_ref = NodeRef::<leptos::html::Div>::new();
    let svg_margin = 40.0;
    let svg_w = 500.0;
    let svg_h = 400.0;

    // Convert mouse screen coords to world coords.
    // Accounts for SVG preserveAspectRatio="xMidYMid meet" (default):
    // the SVG content is uniformly scaled and centered within the element.
    let screen_to_world_from_event = move |ev: &web_sys::MouseEvent| -> Option<(f64, f64)> {
        let container = plan_ref.get()?;
        let svg_el = container.query_selector("svg").ok().flatten()?;
        let rect = svg_el.get_bounding_client_rect();

        let el_w = rect.width();
        let el_h = rect.height();
        if el_w < 1.0 || el_h < 1.0 {
            return None;
        }

        // The SVG viewBox is svg_w × svg_h. With "meet", it scales uniformly
        // to fit inside the element, then centers the result.
        let scale = (el_w / svg_w).min(el_h / svg_h);
        let rendered_w = svg_w * scale;
        let rendered_h = svg_h * scale;
        // Offset due to centering (xMidYMid)
        let offset_x = (el_w - rendered_w) / 2.0;
        let offset_y = (el_h - rendered_h) / 2.0;

        // Mouse position relative to SVG element
        let mx = ev.client_x() as f64 - rect.left();
        let my = ev.client_y() as f64 - rect.top();

        // Convert to SVG viewBox coordinates
        let svg_x = (mx - offset_x) / scale;
        let svg_y = (my - offset_y) / scale;

        let aw = area_width.get();
        let ad = area_depth.get();
        let plot_w = svg_w - 2.0 * svg_margin;
        let plot_h = svg_h - 2.0 * svg_margin;

        let wx = (svg_x - svg_margin) / plot_w * aw;
        let wy = (svg_y - svg_margin) / plot_h * ad;
        Some((wx.clamp(0.0, aw), wy.clamp(0.0, ad)))
    };

    let on_plan_mousedown = move |ev: web_sys::MouseEvent| {
        let Some((wx, wy)) = screen_to_world_from_event(&ev) else {
            return;
        };

        // Polygon drawing mode: clicks add vertices
        if polygon_drawing.get() {
            let mut wip = polygon_wip.get();
            // Close polygon if clicking near the first vertex (and >= 3 vertices)
            if wip.len() >= 3 {
                let (fx, fy) = wip[0];
                let close_dist = ((wx - fx).powi(2) + (wy - fy).powi(2)).sqrt();
                let threshold = area_width.get().max(area_depth.get()) * 0.03;
                if close_dist < threshold {
                    // Finalize polygon and regenerate poles to fit inside
                    set_custom_polygon.set(Some(AreaPolygon::new(wip)));
                    set_polygon_wip.set(Vec::new());
                    set_polygon_drawing.set(false);
                    regenerate_poles();
                    ev.prevent_default();
                    return;
                }
            }
            wip.push((wx, wy));
            set_polygon_wip.set(wip);
            ev.prevent_default();
            return;
        }

        // Find closest pole
        let pole_list = poles.get();
        let mut closest: Option<(usize, f64)> = None;
        for p in &pole_list {
            let dist = ((p.x - wx).powi(2) + (p.y - wy).powi(2)).sqrt();
            if let Some((_, best_dist)) = closest {
                if dist < best_dist {
                    closest = Some((p.id, dist));
                }
            } else {
                closest = Some((p.id, dist));
            }
        }

        // Selection threshold in meters (scale with area size)
        let threshold = area_width.get().max(area_depth.get()) * 0.05;
        if let Some((id, dist)) = closest {
            if dist < threshold {
                push_undo(); // snapshot before drag
                set_selected_pole_id.set(Some(id));
                set_dragging.set(true);
                ev.prevent_default();
            } else {
                set_selected_pole_id.set(None);
            }
        }
    };

    let on_plan_mousemove = move |ev: web_sys::MouseEvent| {
        if !dragging.get() {
            return;
        }
        if let Some(sel_id) = selected_pole_id.get() {
            let Some((wx, wy)) = screen_to_world_from_event(&ev) else {
                return;
            };
            set_poles.update(|ps| {
                if let Some(p) = ps.iter_mut().find(|p| p.id == sel_id) {
                    p.x = wx;
                    p.y = wy;
                }
            });
        }
    };

    let on_plan_mouseup = move |_ev: web_sys::MouseEvent| {
        set_dragging.set(false);
    };

    let on_plan_mouseleave = move |_ev: web_sys::MouseEvent| {
        set_dragging.set(false);
    };

    // --- Quick position presets ---
    let move_selected_to = move |x_frac: Option<f64>, y_frac: Option<f64>| {
        if let Some(sel_id) = selected_pole_id.get() {
            push_undo();
            let aw = area_width.get();
            let ad = area_depth.get();
            set_poles.update(|ps| {
                if let Some(p) = ps.iter_mut().find(|p| p.id == sel_id) {
                    if let Some(xf) = x_frac {
                        p.x = aw * xf;
                    }
                    if let Some(yf) = y_frac {
                        p.y = ad * yf;
                    }
                }
            });
        }
    };

    // Selected pole info for left panel
    let selected_pole_info = move || -> Option<PoleState> {
        let sel = selected_pole_id.get()?;
        poles.get().iter().find(|p| p.id == sel).cloned()
    };

    // --- Optimizer state ---
    let (opt_target_lux, set_opt_target_lux) = signal(20.0_f64);
    let (opt_target_u0, set_opt_target_u0) = signal(0.0_f64); // 0 = disabled
    let (opt_height_min, set_opt_height_min) = signal(defaults.opt_height_min);
    let (opt_height_max, set_opt_height_max) = signal(defaults.opt_height_max);
    let (opt_height_step, set_opt_height_step) = signal(2.0_f64);
    let (opt_results, set_opt_results) = signal(Vec::<OptimizationRow>::new());
    let (opt_running, set_opt_running) = signal(false);
    let (selected_opt_idx, set_selected_opt_idx) = signal(Option::<usize>::None);
    let (overlay_indices, set_overlay_indices) = signal(Vec::<usize>::new());

    let run_optimizer = move || {
        set_opt_running.set(true);
        let ldt_val = ldt.get();
        let arr = arrangement.get();
        let al = arm_length.get();
        let droop = arm_droop.get();
        let aw = area_width.get();
        let ad = area_depth.get();
        let pf = proration.get();
        let target = opt_target_lux.get();
        let u0 = opt_target_u0.get();
        let h_min = opt_height_min.get();
        let h_max = opt_height_max.get();
        let h_step = opt_height_step.get();

        let criteria = OptimizationCriteria {
            target_min_lux: target,
            target_uniformity: if u0 > 0.01 { Some(u0) } else { None },
            height_range: (h_min, h_max),
            height_step: h_step,
            spacing_range: (10.0, aw.min(ad)),
            arrangement: arr,
        };
        let pole_cfg = PoleConfig {
            arrangement: arr,
            arm_length: al,
            arm_droop: droop,
        };

        let results = optimize_spacing(&ldt_val, &criteria, &pole_cfg, aw, ad, 20, pf);
        set_selected_opt_idx.set(None);
        set_overlay_indices.set(Vec::new());
        set_opt_results.set(results);
        set_opt_running.set(false);
    };

    // Apply optimizer result: regenerate poles with computed spacing
    let apply_opt_result = move |row: OptimizationRow| {
        push_undo();
        let aw = area_width.get();
        let ad = area_depth.get();
        let poly = custom_polygon.get();
        let spacing = row.optimal_spacing;

        set_mounting_height.set(row.mounting_height);

        let cols = (aw / spacing).ceil() as usize;
        let rows = (ad / spacing).ceil() as usize;

        let positions = if let Some(ref polygon) = poly {
            eulumdat::area::layout::generate_pole_positions_in_polygon(rows, cols, polygon)
        } else {
            eulumdat::area::layout::generate_pole_positions(rows, cols, aw, ad)
        };
        let new_poles: Vec<PoleState> = positions
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| PoleState {
                id: i,
                x,
                y,
                height_override: None,
                tilt: 0.0,
                rotation: 0.0,
                ldt_index: 0,
            })
            .collect();
        set_poles.set(new_poles);
        set_selected_pole_id.set(None);
    };

    // Overlay colors for up to 8 heights
    const OVERLAY_COLORS: &[&str] = &[
        "rgba(255,80,80,0.85)",   // red
        "rgba(80,200,255,0.85)",  // cyan
        "rgba(255,200,60,0.85)",  // gold
        "rgba(120,255,120,0.85)", // green
        "rgba(200,120,255,0.85)", // purple
        "rgba(255,150,80,0.85)",  // orange
        "rgba(80,255,200,0.85)",  // teal
        "rgba(255,80,200,0.85)",  // pink
    ];

    // ISO view SVG + stats (must be after optimizer signals for overlay access)
    let area_data = move || {
        let ldt_val = ldt.get();
        let aw = area_width.get();
        let ad = area_depth.get();
        let pf = proration.get();
        let gr = grid_resolution.get();
        let is_bay = bay_view.get();

        let (result, n_placements) = if is_bay {
            // Bay view: compute a single repeating bay with 3×3 neighbor contributions
            let pole_list = poles.get();
            let n_poles = pole_list.len();
            if n_poles < 2 {
                // Need at least 2 poles to determine spacing
                let (placements, ldt_idx) = make_placements();
                let np = placements.len();
                let result = compute_mixed_or_single(
                    &ldt_val,
                    &extra_ldts.get(),
                    &placements,
                    &ldt_idx,
                    aw,
                    ad,
                    gr,
                    pf,
                );
                (result, np)
            } else {
                // Estimate bay spacing from pole positions
                let mut xs: Vec<f64> = pole_list.iter().map(|p| p.x).collect();
                let mut ys: Vec<f64> = pole_list.iter().map(|p| p.y).collect();
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
                xs.dedup_by(|a, b| (*a - *b).abs() < 0.1);
                ys.dedup_by(|a, b| (*a - *b).abs() < 0.1);

                let bay_w = if xs.len() > 1 { xs[1] - xs[0] } else { aw };
                let bay_d = if ys.len() > 1 { ys[1] - ys[0] } else { ad };

                // Create 3×3 grid of one bay (center bay = the actual computation)
                let h = mounting_height.get();
                let arr = arrangement.get();
                let pole_cfg = PoleConfig {
                    arrangement: arr,
                    arm_length: arm_length.get(),
                    arm_droop: arm_droop.get(),
                };
                let bay_poles =
                    eulumdat::area::layout::generate_pole_positions(3, 3, bay_w * 3.0, bay_d * 3.0);
                let bay_placements = eulumdat::area::layout::generate_placements(
                    &bay_poles,
                    h,
                    &pole_cfg,
                    base_rotation.get(),
                );
                let np = bay_placements.len();
                let full = compute_area_illuminance(
                    &ldt_val,
                    &bay_placements,
                    bay_w * 3.0,
                    bay_d * 3.0,
                    gr * 3,
                    pf,
                );

                // Extract center bay
                let n = gr;
                let mut lux_grid = vec![vec![0.0_f64; n]; n];
                let mut min_lux = f64::MAX;
                let mut max_lux: f64 = 0.0;
                let mut sum_lux: f64 = 0.0;
                #[allow(clippy::needless_range_loop)]
                for row in 0..n {
                    for col in 0..n {
                        let lux = full.lux_grid[n + row][n + col];
                        lux_grid[row][col] = lux;
                        min_lux = min_lux.min(lux);
                        max_lux = max_lux.max(lux);
                        sum_lux += lux;
                    }
                }
                if min_lux == f64::MAX {
                    min_lux = 0.0;
                }
                let count = (n * n) as f64;
                let avg_lux = if count > 0.0 { sum_lux / count } else { 0.0 };

                let bay_result = eulumdat::area::AreaResult {
                    lux_grid,
                    min_lux,
                    avg_lux,
                    max_lux,
                    uniformity_min_avg: if avg_lux > 0.0 {
                        min_lux / avg_lux
                    } else {
                        0.0
                    },
                    uniformity_avg_min: if min_lux > 0.0 {
                        avg_lux / min_lux
                    } else {
                        f64::INFINITY
                    },
                    uniformity_min_max: if max_lux > 0.0 {
                        min_lux / max_lux
                    } else {
                        0.0
                    },
                    area_width: bay_w,
                    area_depth: bay_d,
                    grid_resolution: n,
                    mask: None,
                };
                (bay_result, np)
            }
        } else {
            let (placements, ldt_idx) = make_placements();
            let np = placements.len();
            let result = if let Some(ref poly) = custom_polygon.get() {
                // Polygon area: use polygon computation
                compute_area_illuminance_polygon(&ldt_val, &placements, poly, gr, pf)
            } else {
                compute_mixed_or_single(
                    &ldt_val,
                    &extra_ldts.get(),
                    &placements,
                    &ldt_idx,
                    aw,
                    ad,
                    gr,
                    pf,
                )
            };
            (result, np)
        };
        let svg_theme = SvgTheme::css_variables_with_locale(&locale.get());
        let u = units.get();

        // Build overlay contour sets from checked height comparison cards
        let ovl_idx = overlay_indices.get();
        let opt_res = opt_results.get();
        let overlays: Vec<ContourOverlay> = ovl_idx
            .iter()
            .filter_map(|&idx| {
                let opt_row = opt_res.get(idx)?;
                let h = opt_row.mounting_height;
                let spacing = opt_row.optimal_spacing;

                // Generate a grid of poles at this height/spacing
                let cols = (aw / spacing).ceil() as usize;
                let rows = (ad / spacing).ceil() as usize;
                let pole_positions =
                    eulumdat::area::layout::generate_pole_positions(rows, cols, aw, ad);
                let pole_cfg = PoleConfig {
                    arrangement: arrangement.get(),
                    arm_length: arm_length.get(),
                    arm_droop: arm_droop.get(),
                };
                let ol_placements = eulumdat::area::layout::generate_placements(
                    &pole_positions,
                    h,
                    &pole_cfg,
                    base_rotation.get(),
                );
                let ol_result = compute_area_illuminance(&ldt_val, &ol_placements, aw, ad, 20, pf);

                let color_idx = idx % OVERLAY_COLORS.len();
                Some(ContourOverlay {
                    result: ol_result,
                    label: format!("{:.0} {}", u.convert_meters(h), u.distance_label()),
                    color: OVERLAY_COLORS[color_idx].to_string(),
                })
            })
            .collect();

        let iso_svg = if overlays.is_empty() {
            AreaSvg::iso_view(&result, 600.0, 450.0, &svg_theme, u)
        } else {
            AreaSvg::iso_view_with_overlays(&result, &overlays, 600.0, 450.0, &svg_theme, u)
        };

        // Sync designer data to localStorage for Bevy 3D viewer
        let (sync_placements, _) = make_placements();
        super::bevy_scene::save_designer_exterior_to_storage(&result, &sync_placements);

        (result, iso_svg, n_placements)
    };

    view! {
        <div class="area-designer">
            // Toolbar
            <div class="area-toolbar">
                <div class="area-toolbar-row">
                    <label>{move || locale.get().area_designer.layout.label.clone()}
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                let preset = match val.as_str() {
                                    "single" => GridPreset::Single,
                                    "pair" => GridPreset::Pair,
                                    "row3" => GridPreset::Row3,
                                    "grid2x2" => GridPreset::Grid2x2,
                                    "grid2x3" => GridPreset::Grid2x3,
                                    "grid3x3" => GridPreset::Grid3x3,
                                    "perimeter" => GridPreset::Perimeter(8),
                                    _ => GridPreset::Grid2x3,
                                };
                                set_grid_preset.set(preset);
                                regenerate_poles();
                            }
                        >
                            <option value="single">{move || locale.get().area_designer.layout.single.clone()}</option>
                            <option value="pair">{move || locale.get().area_designer.layout.pair.clone()}</option>
                            <option value="row3">{move || locale.get().area_designer.layout.row_of_3.clone()}</option>
                            <option value="grid2x2">{move || locale.get().area_designer.layout.grid_2x2.clone()}</option>
                            <option value="grid2x3" selected>{move || locale.get().area_designer.layout.grid_2x3.clone()}</option>
                            <option value="grid3x3">{move || locale.get().area_designer.layout.grid_3x3.clone()}</option>
                            <option value="perimeter">{move || locale.get().area_designer.layout.perimeter.clone()}</option>
                        </select>
                    </label>
                    {move || {
                        if grid_preset.get().is_perimeter() {
                            let current_n = match grid_preset.get() {
                                GridPreset::Perimeter(n) => n,
                                _ => 8,
                            };
                            view! {
                                <label>{move || locale.get().area_designer.params.poles.clone()}
                                    <input type="number" min="3" max="30" step="1"
                                        class="area-prop-input"
                                        style="width: 45px;"
                                        prop:value=move || current_n.to_string()
                                        on:change=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                                                set_grid_preset.set(GridPreset::Perimeter(v.clamp(3, 30)));
                                                regenerate_poles();
                                            }
                                        }
                                    />
                                </label>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}

                    <label>{move || locale.get().area_designer.arrangement.label.clone()}
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                let arr = match val.as_str() {
                                    "single" => ArrangementType::Single,
                                    "b2b" => ArrangementType::BackToBack,
                                    "twin" => ArrangementType::TwinArm,
                                    "quad" => ArrangementType::Quad,
                                    "wall" => ArrangementType::WallMounted,
                                    _ => ArrangementType::Single,
                                };
                                set_arrangement.set(arr);
                            }
                        >
                            <option value="single" selected>{move || locale.get().area_designer.arrangement.single.clone()}</option>
                            <option value="b2b">{move || locale.get().area_designer.arrangement.back_to_back.clone()}</option>
                            <option value="twin">{move || locale.get().area_designer.arrangement.twin_arm.clone()}</option>
                            <option value="quad">{move || locale.get().area_designer.arrangement.quad.clone()}</option>
                            <option value="wall">{move || locale.get().area_designer.arrangement.wall_mounted.clone()}</option>
                        </select>
                    </label>

                    <label>{move || locale.get().designer.height.clone()}
                        <input type="range" min="2" max="20" step="0.5"
                            prop:value=move || mounting_height.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_mounting_height.set(v);
                                }
                            }
                        />
                        <span>{move || {
                            let u = units.get();
                            format!("{:.1} {}", u.convert_meters(mounting_height.get()), u.distance_label())
                        }}</span>
                    </label>

                    <label>{move || locale.get().area_designer.params.arm.clone()}
                        <input type="range" min="0" max="3" step="0.1"
                            prop:value=move || arm_length.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_arm_length.set(v);
                                }
                            }
                        />
                        <span>{move || {
                            let u = units.get();
                            format!("{:.1} {}", u.convert_meters(arm_length.get()), u.distance_label())
                        }}</span>
                    </label>

                    <label>{move || locale.get().area_designer.params.droop.clone()}
                        <input type="range" min="0" max="30" step="1"
                            prop:value=move || arm_droop.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_arm_droop.set(v);
                                }
                            }
                        />
                        <span>{move || format!("{:.0}°", arm_droop.get())}</span>
                    </label>
                </div>
                <div class="area-toolbar-row">
                    <label>{move || locale.get().area_designer.params.area_w.clone()}
                        <input type="range" min="5" max="200" step="1"
                            prop:value=move || area_width.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_area_width.set(v);
                                }
                            }
                        />
                        <span>{move || {
                            let u = units.get();
                            format!("{:.0} {}", u.convert_meters(area_width.get()), u.distance_label())
                        }}</span>
                    </label>

                    <label>{move || locale.get().area_designer.params.area_d.clone()}
                        <input type="range" min="5" max="200" step="1"
                            prop:value=move || area_depth.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_area_depth.set(v);
                                }
                            }
                        />
                        <span>{move || {
                            let u = units.get();
                            format!("{:.0} {}", u.convert_meters(area_depth.get()), u.distance_label())
                        }}</span>
                    </label>

                    <label>{move || locale.get().designer.rotation.clone()}
                        <input type="range" min="0" max="350" step="10"
                            prop:value=move || base_rotation.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_base_rotation.set(v);
                                }
                            }
                        />
                        <span>{move || format!("{:.0}°", base_rotation.get())}</span>
                    </label>

                    <label>{move || locale.get().area_designer.params.proration.clone()}
                        <input type="range" min="0.3" max="1.0" step="0.05"
                            prop:value=move || proration.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    set_proration.set(v);
                                }
                            }
                        />
                        <span>{move || format!("{:.2}", proration.get())}</span>
                    </label>

                    <label>{move || locale.get().designer.resolution.clone()}
                        <select on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                                set_grid_resolution.set(v);
                            }
                        }>
                            <option value="20">"20 (fast)"</option>
                            <option value="40" selected>"40 (balanced)"</option>
                            <option value="80">"80 (detailed)"</option>
                        </select>
                    </label>

                    <label class="area-bay-toggle"
                        title="Single-bay mode: shows one repeating cell with neighbor contributions. Useful for evaluating uniform spacing — assumes the pattern repeats infinitely."
                    >
                        <input type="checkbox"
                            prop:checked=move || bay_view.get()
                            on:change=move |ev| {
                                let el: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
                                set_bay_view.set(el.checked());
                            }
                        />
                        " Single Bay"
                    </label>

                    {move || {
                        if polygon_drawing.get() {
                            let n = polygon_wip.get().len();
                            let cancel_label = locale.get().area_designer.polygon.cancel.clone();
                            view! {
                                <button class="area-export-btn"
                                    style="color: var(--error-color); border-color: var(--error-color);"
                                    on:click=move |_| {
                                        set_polygon_drawing.set(false);
                                        set_polygon_wip.set(Vec::new());
                                    }
                                >{format!("{cancel_label} ({n} pts)")}</button>
                            }.into_any()
                        } else if custom_polygon.get().is_some() {
                            view! {
                                <button class="area-export-btn"
                                    title="Clear custom polygon, revert to rectangle"
                                    on:click=move |_| {
                                        set_custom_polygon.set(None);
                                        regenerate_poles();
                                    }
                                >{move || locale.get().area_designer.polygon.clear.clone()}</button>
                            }.into_any()
                        } else {
                            view! {
                                <button class="area-export-btn"
                                    title="Draw custom area polygon"
                                    on:click=move |_| {
                                        set_polygon_drawing.set(true);
                                        set_polygon_wip.set(Vec::new());
                                    }
                                >{move || locale.get().area_designer.polygon.draw.clone()}</button>
                            }.into_any()
                        }
                    }}

                    <button
                        class="area-share-btn"
                        title="Copy shareable link to clipboard"
                        on:click=move |_| {
                            copy_url_to_clipboard();
                            set_link_copied.set(true);
                            // Reset after 2 seconds
                            let cb = wasm_bindgen::closure::Closure::once(move || {
                                set_link_copied.set(false);
                            });
                            if let Some(w) = web_sys::window() {
                                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                                    cb.as_ref().unchecked_ref(),
                                    2000,
                                );
                            }
                            cb.forget();
                        }
                    >
                        {move || if link_copied.get() { locale.get().designer.copied.clone() } else { locale.get().designer.share_link.clone() }}
                    </button>

                    <button
                        class="area-export-btn"
                        title="Save current configuration to browser storage"
                        on:click=move |_| {
                            if let Some(window) = web_sys::window() {
                                if let Ok(Some(storage)) = window.local_storage() {
                                    let hash = params_to_hash(&DesignerParams {
                                        grid_preset: grid_preset.get(),
                                        arrangement: arrangement.get(),
                                        mounting_height: mounting_height.get(),
                                        arm_length: arm_length.get(),
                                        arm_droop: arm_droop.get(),
                                        area_width: area_width.get(),
                                        area_depth: area_depth.get(),
                                        proration: proration.get(),
                                        grid_resolution: grid_resolution.get(),
                                        base_rotation: base_rotation.get(),
                                        bay_view: bay_view.get(),
                                        poles: poles.get(),
                                        polygon: custom_polygon.get(),
                                    });
                                    let _ = storage.set_item("eulumdat_designer_config", &hash);
                                }
                            }
                        }
                    >{move || locale.get().designer.save.clone()}</button>

                    <button
                        class="area-export-btn"
                        title="Load saved configuration from browser storage"
                        on:click=move |_| {
                            if let Some(window) = web_sys::window() {
                                if let Ok(Some(storage)) = window.local_storage() {
                                    if let Ok(Some(hash)) = storage.get_item("eulumdat_designer_config") {
                                        push_undo();
                                        // Parse the hash and apply all values
                                        let query = hash.strip_prefix("#designer?").unwrap_or(&hash);
                                        let mut map = std::collections::HashMap::new();
                                        for pair in query.split('&') {
                                            if let Some((k, v)) = pair.split_once('=') {
                                                let decoded = js_sys::decode_uri_component(v)
                                                    .ok()
                                                    .map(|s| s.as_string().unwrap_or_default())
                                                    .unwrap_or_else(|| v.to_string());
                                                map.insert(k.to_string(), decoded);
                                            }
                                        }
                                        let get_f = |key: &str, def: f64| -> f64 {
                                            map.get(key).and_then(|v| v.parse().ok()).unwrap_or(def)
                                        };
                                        if let Some(layout) = map.get("layout") {
                                            set_grid_preset.set(str_to_grid_preset(layout));
                                        }
                                        if let Some(arr) = map.get("arr") {
                                            set_arrangement.set(str_to_arrangement(arr));
                                        }
                                        set_mounting_height.set(get_f("h", mounting_height.get()));
                                        set_arm_length.set(get_f("arm", arm_length.get()));
                                        set_arm_droop.set(get_f("droop", arm_droop.get()));
                                        set_area_width.set(get_f("w", area_width.get()));
                                        set_area_depth.set(get_f("d", area_depth.get()));
                                        set_base_rotation.set(get_f("rot", base_rotation.get()));
                                        if let Some(pf) = map.get("pf") {
                                            if let Ok(v) = pf.parse() { set_proration.set(v); }
                                        }
                                        if let Some(res) = map.get("res") {
                                            if let Ok(v) = res.parse() { set_grid_resolution.set(v); }
                                        }
                                        set_bay_view.set(map.get("bay").map(|v| v == "1").unwrap_or(false));
                                        // Load polygon if present
                                        if let Some(poly_str) = map.get("poly") {
                                            let verts: Vec<(f64, f64)> = poly_str.split(';')
                                                .filter_map(|p| {
                                                    let mut it = p.split(',');
                                                    Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
                                                })
                                                .collect();
                                            if verts.len() >= 3 {
                                                set_custom_polygon.set(Some(AreaPolygon::new(verts)));
                                            } else {
                                                set_custom_polygon.set(None);
                                            }
                                        } else {
                                            set_custom_polygon.set(None);
                                        }
                                        if let Some(pole_str) = map.get("poles") {
                                            let loaded_poles = decode_poles(pole_str);
                                            if !loaded_poles.is_empty() {
                                                set_poles.set(loaded_poles);
                                            } else {
                                                regenerate_poles();
                                            }
                                        } else {
                                            regenerate_poles();
                                        }
                                        set_selected_pole_id.set(None);
                                    }
                                }
                            }
                        }
                    >{move || locale.get().designer.load.clone()}</button>

                    <button
                        class="area-export-btn"
                        title="Undo last pole change (Ctrl+Z)"
                        disabled=move || undo_stack.get().is_empty()
                        on:click=move |_| do_undo()
                    >{move || locale.get().designer.undo.clone()}</button>

                    <button
                        class="area-export-btn"
                        title="Redo (Ctrl+Shift+Z)"
                        disabled=move || redo_stack.get().is_empty()
                        on:click=move |_| do_redo()
                    >{move || locale.get().designer.redo.clone()}</button>
                </div>
            </div>

            {move || {
                if arrangement.get() == ArrangementType::WallMounted {
                    view! {
                        <div class="area-wall-note">
                            {move || locale.get().area_designer.wall_mounted_hint.clone()}
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Main content: left panel + center views
            <div class="area-layout">
                // Left sidebar
                <div class="area-sidebar">
                    // Position presets
                    <div class="area-sidebar-section">
                        <h4>{move || locale.get().area_designer.presets.title.clone()}</h4>
                        <div class="area-preset-grid">
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.top_left.clone()
                                on:click=move |_| move_selected_to(Some(0.2), Some(0.2))>"↖"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.top.clone()
                                on:click=move |_| move_selected_to(None, Some(0.2))>"↑"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.top_right.clone()
                                on:click=move |_| move_selected_to(Some(0.8), Some(0.2))>"↗"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.left.clone()
                                on:click=move |_| move_selected_to(Some(0.2), None)>"←"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.center.clone()
                                on:click=move |_| move_selected_to(Some(0.5), Some(0.5))>"●"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.right.clone()
                                on:click=move |_| move_selected_to(Some(0.8), None)>"→"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.bottom_left.clone()
                                on:click=move |_| move_selected_to(Some(0.2), Some(0.8))>"↙"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.bottom.clone()
                                on:click=move |_| move_selected_to(None, Some(0.8))>"↓"</button>
                            <button class="area-preset-btn" title=move || locale.get().area_designer.presets.bottom_right.clone()
                                on:click=move |_| move_selected_to(Some(0.8), Some(0.8))>"↘"</button>
                        </div>
                    </div>

                    // Mixed luminaire types
                    <div class="area-sidebar-section">
                        <h4>{move || locale.get().area_designer.mixed.title.clone()}</h4>
                        <p class="area-hint">{move || locale.get().area_designer.mixed.description.clone()}</p>
                        <input type="file"
                            accept=".ldt,.ies"
                            node_ref=extra_ldts_ref
                            style="display: none;"
                            on:change=move |ev| {
                                let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
                                let files = match input.files() {
                                    Some(f) => f,
                                    None => return,
                                };
                                for i in 0..files.length() {
                                    let file = match files.get(i) {
                                        Some(f) => f,
                                        None => continue,
                                    };
                                    let name = file.name();
                                    let reader = web_sys::FileReader::new().unwrap();
                                    let reader_clone = reader.clone();
                                    let name_clone = name.clone();
                                    let cb = wasm_bindgen::closure::Closure::once(move || {
                                        let text = reader_clone.result().ok()
                                            .and_then(|v| v.as_string())
                                            .unwrap_or_default();
                                        let parsed = if name_clone.ends_with(".ies") {
                                            eulumdat::IesParser::parse(&text).ok()
                                        } else {
                                            Eulumdat::parse(&text).ok()
                                        };
                                        if let Some(ldt_parsed) = parsed {
                                            let display_name = name_clone
                                                .strip_suffix(".ldt").or_else(|| name_clone.strip_suffix(".ies"))
                                                .unwrap_or(&name_clone).to_string();
                                            set_extra_ldts.update(|v| v.push(ExtraLdt {
                                                name: display_name,
                                                ldt: ldt_parsed,
                                            }));
                                        }
                                    });
                                    reader.set_onload(Some(cb.as_ref().unchecked_ref()));
                                    let _ = reader.read_as_text(&file);
                                    cb.forget();
                                }
                                input.set_value(""); // allow re-loading same file
                            }
                        />
                        <button class="area-export-btn"
                            on:click=move |_| {
                                if let Some(input) = extra_ldts_ref.get() {
                                    input.click();
                                }
                            }
                        >"+ Add LDT/IES"</button>
                        {move || {
                            let extras = extra_ldts.get();
                            if extras.is_empty() {
                                return view! { <span></span> }.into_any();
                            }
                            view! {
                                <div class="area-extra-ldts">
                                    {extras.iter().enumerate().map(|(i, e)| {
                                        let name = e.name.clone();
                                        view! {
                                            <div class="area-extra-ldt-row">
                                                <span class="area-extra-ldt-idx">{format!("#{}", i + 1)}</span>
                                                <span class="area-extra-ldt-name">{name}</span>
                                                <button class="area-extra-ldt-rm"
                                                    title=move || locale.get().designer.remove.clone()
                                                    on:click=move |_| {
                                                        push_undo();
                                                        set_extra_ldts.update(|v| {
                                                            v.remove(i);
                                                        });
                                                        // Reset any pole references to removed index
                                                        set_poles.update(|ps| {
                                                            for p in ps.iter_mut() {
                                                                if p.ldt_index == i + 1 {
                                                                    p.ldt_index = 0;
                                                                } else if p.ldt_index > i + 1 {
                                                                    p.ldt_index -= 1;
                                                                }
                                                            }
                                                        });
                                                    }
                                                >"×"</button>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </div>

                    // Selected pole properties
                    <div class="area-sidebar-section">
                        <h4>{move || locale.get().area_designer.pole.selected.clone()}</h4>
                        {move || {
                            if let Some(pole) = selected_pole_info() {
                                let pole_id = pole.id;
                                view! {
                                    <div class="area-pole-props">
                                        <div class="area-prop-row">
                                            <span class="area-prop-label">{move || locale.get().area_designer.pole.pole_n.clone()}</span>
                                            <span class="area-prop-value">{pole.id + 1}</span>
                                        </div>
                                        <div class="area-prop-row">
                                            <span class="area-prop-label">{move || locale.get().area_designer.pole.x.clone()}</span>
                                            <input type="number" step="0.5"
                                                class="area-prop-input"
                                                prop:value=move || {
                                                    let u = units.get();
                                                    poles.get().iter()
                                                        .find(|p| p.id == pole_id)
                                                        .map(|p| format!("{:.1}", u.convert_meters(p.x)))
                                                        .unwrap_or_default()
                                                }
                                                on:change=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                        let m = units.get().to_meters(v);
                                                        set_poles.update(|ps| {
                                                            if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                                p.x = m.clamp(0.0, area_width.get());
                                                            }
                                                        });
                                                    }
                                                }
                                            />
                                            <span class="area-prop-unit">{move || units.get().distance_label()}</span>
                                        </div>
                                        <div class="area-prop-row">
                                            <span class="area-prop-label">{move || locale.get().area_designer.pole.y.clone()}</span>
                                            <input type="number" step="0.5"
                                                class="area-prop-input"
                                                prop:value=move || {
                                                    let u = units.get();
                                                    poles.get().iter()
                                                        .find(|p| p.id == pole_id)
                                                        .map(|p| format!("{:.1}", u.convert_meters(p.y)))
                                                        .unwrap_or_default()
                                                }
                                                on:change=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                        let m = units.get().to_meters(v);
                                                        set_poles.update(|ps| {
                                                            if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                                p.y = m.clamp(0.0, area_depth.get());
                                                            }
                                                        });
                                                    }
                                                }
                                            />
                                            <span class="area-prop-unit">{move || units.get().distance_label()}</span>
                                        </div>
                                        <div class="area-prop-row">
                                            <span class="area-prop-label">{move || locale.get().designer.height.clone()}</span>
                                            <input type="number" step="0.5" min="2" max="20"
                                                class="area-prop-input"
                                                prop:value=move || {
                                                    let u = units.get();
                                                    poles.get().iter()
                                                        .find(|p| p.id == pole_id)
                                                        .map(|p| format!("{:.1}", u.convert_meters(p.height_override.unwrap_or(mounting_height.get()))))
                                                        .unwrap_or_default()
                                                }
                                                on:change=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                        let m = units.get().to_meters(v);
                                                        set_poles.update(|ps| {
                                                            if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                                p.height_override = Some(m.clamp(2.0, 20.0));
                                                            }
                                                        });
                                                    }
                                                }
                                            />
                                            <span class="area-prop-unit">{move || units.get().distance_label()}</span>
                                        </div>
                                        <div class="area-prop-row">
                                            <span class="area-prop-label">{move || locale.get().designer.tilt.clone()}</span>
                                            <input type="number" step="1" min="-30" max="90"
                                                class="area-prop-input"
                                                prop:value=move || {
                                                    poles.get().iter()
                                                        .find(|p| p.id == pole_id)
                                                        .map(|p| format!("{:.0}", p.tilt))
                                                        .unwrap_or_default()
                                                }
                                                on:change=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                        set_poles.update(|ps| {
                                                            if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                                p.tilt = v;
                                                            }
                                                        });
                                                    }
                                                }
                                            />
                                            <span class="area-prop-unit">"°"</span>
                                        </div>
                                        <div class="area-prop-row">
                                            <span class="area-prop-label">{move || locale.get().designer.rotation.clone()}</span>
                                            <input type="number" step="10" min="0" max="350"
                                                class="area-prop-input"
                                                prop:value=move || {
                                                    poles.get().iter()
                                                        .find(|p| p.id == pole_id)
                                                        .map(|p| format!("{:.0}", p.rotation))
                                                        .unwrap_or_default()
                                                }
                                                on:change=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                        set_poles.update(|ps| {
                                                            if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                                p.rotation = v % 360.0;
                                                            }
                                                        });
                                                    }
                                                }
                                            />
                                            <span class="area-prop-unit">"°"</span>
                                        </div>
                                        {move || {
                                            let extras = extra_ldts.get();
                                            if extras.is_empty() {
                                                view! { <span></span> }.into_any()
                                            } else {
                                                let current_idx = poles.get().iter()
                                                    .find(|p| p.id == pole_id)
                                                    .map(|p| p.ldt_index)
                                                    .unwrap_or(0);
                                                view! {
                                                    <div class="area-prop-row">
                                                        <span class="area-prop-label">{move || locale.get().area_designer.pole.ldt.clone()}</span>
                                                        <select class="area-prop-input"
                                                            on:change=move |ev| {
                                                                if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                                    push_undo();
                                                                    set_poles.update(|ps| {
                                                                        if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                                            p.ldt_index = idx;
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                        >
                                                            <option value="0" selected=move || current_idx == 0>{move || locale.get().area_designer.pole.primary.clone()}</option>
                                                            {extras.iter().enumerate().map(|(i, e)| {
                                                                let idx = i + 1;
                                                                let name = e.name.clone();
                                                                view! {
                                                                    <option value=idx.to_string() selected=move || current_idx == idx>
                                                                        {name}
                                                                    </option>
                                                                }
                                                            }).collect_view()}
                                                        </select>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                        <button class="area-reset-btn"
                                            on:click=move |_| {
                                                push_undo();
                                                set_poles.update(|ps| {
                                                    if let Some(p) = ps.iter_mut().find(|p| p.id == pole_id) {
                                                        p.height_override = None;
                                                        p.tilt = 0.0;
                                                        p.rotation = 0.0;
                                                        p.ldt_index = 0;
                                                    }
                                                });
                                            }
                                        >{move || locale.get().area_designer.pole.reset_overrides.clone()}</button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <p class="area-no-selection">{move || locale.get().area_designer.pole.click_to_select.clone()}</p>
                                }.into_any()
                            }
                        }}
                    </div>

                    // Optimizer panel
                    <div class="area-sidebar-section">
                        <h4>{move || locale.get().area_designer.optimizer.title.clone()}</h4>
                        <div class="area-opt-inputs">
                            <div class="area-prop-row">
                                <span class="area-prop-label">{move || locale.get().area_designer.optimizer.target.clone()}</span>
                                <input type="number" step="1" min="1" max="500"
                                    class="area-prop-input"
                                    prop:value=move || format!("{:.0}", opt_target_lux.get())
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_opt_target_lux.set(v.max(1.0));
                                        }
                                    }
                                />
                                <span class="area-prop-unit">{move || format!("{} min", units.get().illuminance_label())}</span>
                            </div>
                            <div class="area-prop-row">
                                <span class="area-prop-label">{move || locale.get().area_designer.optimizer.uniformity.clone()}</span>
                                <input type="number" step="0.05" min="0" max="1"
                                    class="area-prop-input"
                                    prop:value=move || format!("{:.2}", opt_target_u0.get())
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_opt_target_u0.set(v.clamp(0.0, 1.0));
                                        }
                                    }
                                />
                                <span class="area-prop-unit">"(0=off)"</span>
                            </div>
                            <div class="area-prop-row">
                                <span class="area-prop-label">{move || locale.get().area_designer.optimizer.heights.clone()}</span>
                                <input type="number" step="1" min="2" max="20"
                                    class="area-prop-input area-prop-input-sm"
                                    prop:value=move || format!("{:.0}", opt_height_min.get())
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_opt_height_min.set(v);
                                        }
                                    }
                                />
                                <span class="area-prop-unit">"–"</span>
                                <input type="number" step="1" min="2" max="20"
                                    class="area-prop-input area-prop-input-sm"
                                    prop:value=move || format!("{:.0}", opt_height_max.get())
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_opt_height_max.set(v);
                                        }
                                    }
                                />
                                <span class="area-prop-unit">{move || units.get().distance_label()}</span>
                            </div>
                            <div class="area-prop-row">
                                <span class="area-prop-label">{move || locale.get().area_designer.optimizer.step.clone()}</span>
                                <input type="number" step="0.5" min="0.5" max="5"
                                    class="area-prop-input"
                                    prop:value=move || format!("{:.1}", opt_height_step.get())
                                    on:change=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                            set_opt_height_step.set(v.max(0.5));
                                        }
                                    }
                                />
                                <span class="area-prop-unit">{move || units.get().distance_label()}</span>
                            </div>
                            <button class="area-opt-run-btn"
                                disabled=move || opt_running.get()
                                on:click=move |_| run_optimizer()
                            >
                                {move || if opt_running.get() { locale.get().area_designer.optimizer.running.clone() } else { format!("▶ {}", locale.get().area_designer.optimizer.optimize) }}
                            </button>
                        </div>

                        // Results table
                        {move || {
                            let results = opt_results.get();
                            if results.is_empty() {
                                return view! { <div></div> }.into_any();
                            }
                            view! {
                                <div class="area-opt-results">
                                    <table class="area-opt-table">
                                        <thead>
                                            <tr>
                                                <th>{move || locale.get().area_designer.optimizer.ht.clone()}</th>
                                                <th>{move || locale.get().area_designer.optimizer.spc.clone()}</th>
                                                <th>{move || locale.get().designer.min.clone()}</th>
                                                <th>{move || locale.get().designer.avg.clone()}</th>
                                                <th>{move || locale.get().area_designer.stats.u0.clone()}</th>
                                                <th>{move || locale.get().area_designer.optimizer.poles_col.clone()}</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {results.iter().enumerate().map(|(idx, row)| {
                                                let r = row.clone();
                                                let r2 = row.clone();
                                                let cls = if row.meets_criteria { "area-opt-row area-opt-pass" } else { "area-opt-row" };
                                                let u = units.get();
                                                view! {
                                                    <tr class=cls
                                                        on:click=move |_| {
                                                            set_selected_opt_idx.set(Some(idx));
                                                            apply_opt_result(r.clone());
                                                        }
                                                        title="Click to apply"
                                                        style="cursor: pointer;"
                                                    >
                                                        <td>{format!("{:.0}", u.convert_meters(r2.mounting_height))}</td>
                                                        <td>{format!("{:.0}", u.convert_meters(r2.optimal_spacing))}</td>
                                                        <td>{format!("{:.0}", u.convert_lux(r2.min_lux))}</td>
                                                        <td>{format!("{:.0}", u.convert_lux(r2.avg_lux))}</td>
                                                        <td>{format!("{:.2}", r2.uniformity_min_avg)}</td>
                                                        <td>{r2.poles_needed}</td>
                                                        <td>{if r2.meets_criteria { "✓" } else { "" }}</td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any()
                        }}
                    </div>

                    // Height comparison cards
                    <div class="area-sidebar-section">
                        <h4>{move || locale.get().area_designer.height_compare.title.clone()}</h4>
                        {move || {
                            let results = opt_results.get();
                            let sel = selected_opt_idx.get();
                            let u = units.get();
                            if results.is_empty() {
                                return view! {
                                    <p class="area-no-selection">{move || locale.get().area_designer.height_compare.run_hint.clone()}</p>
                                }.into_any();
                            }
                            let ovl = overlay_indices.get();
                            view! {
                                <div class="area-opt-cards">
                                    {results.iter().enumerate().map(|(idx, row)| {
                                        let r = row.clone();
                                        let is_sel = sel == Some(idx);
                                        let is_overlay = ovl.contains(&idx);
                                        let cls = if is_sel && row.meets_criteria {
                                            "area-opt-card area-opt-card-selected area-opt-card-pass"
                                        } else if is_sel {
                                            "area-opt-card area-opt-card-selected"
                                        } else if row.meets_criteria {
                                            "area-opt-card area-opt-card-pass"
                                        } else {
                                            "area-opt-card"
                                        };
                                        view! {
                                            <div class=cls
                                                on:click=move |_| {
                                                    set_selected_opt_idx.set(Some(idx));
                                                    apply_opt_result(r.clone());
                                                }
                                            >
                                                <div class="area-opt-card-header">
                                                    <input type="checkbox"
                                                        class="area-overlay-cb"
                                                        title="Overlay contours on ISO view"
                                                        prop:checked=is_overlay
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                            set_overlay_indices.update(|v| {
                                                                if let Some(pos) = v.iter().position(|&i| i == idx) {
                                                                    v.remove(pos);
                                                                } else {
                                                                    v.push(idx);
                                                                }
                                                            });
                                                        }
                                                    />
                                                    <span class="area-opt-card-height">
                                                        {format!("{:.0} {}", u.convert_meters(row.mounting_height), u.distance_label())}
                                                    </span>
                                                    {if row.meets_criteria {
                                                        view! { <span class="area-opt-card-badge">"PASS"</span> }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }}
                                                </div>
                                                <div class="area-opt-card-stats">
                                                    <div><span class="area-opt-card-lbl">{move || locale.get().designer.min.clone()}</span><span>{u.format_lux(row.min_lux)}</span></div>
                                                    <div><span class="area-opt-card-lbl">{move || locale.get().designer.avg.clone()}</span><span>{u.format_lux(row.avg_lux)}</span></div>
                                                    <div><span class="area-opt-card-lbl">{move || locale.get().designer.max.clone()}</span><span>{u.format_lux(row.max_lux)}</span></div>
                                                    <div><span class="area-opt-card-lbl">{move || locale.get().area_designer.stats.u0.clone()}</span><span>{format!("{:.2}", row.uniformity_min_avg)}</span></div>
                                                </div>
                                                <div class="area-opt-card-footer">
                                                    {format!("Spacing: {:.0} {} | {} poles", u.convert_meters(row.optimal_spacing), u.distance_label(), row.poles_needed)}
                                                </div>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>

                // Center: views
                <div class="area-views">
                    // Plan view
                    <div class="area-panel">
                        <h3>{move || locale.get().designer.plan_view.clone()}
                            <span class="area-subtitle">" — click to select, drag to move"</span>
                        </h3>
                        <div class="area-plan-svg"
                            node_ref=plan_ref
                            inner_html=plan_svg
                            on:mousedown=on_plan_mousedown
                            on:mousemove=on_plan_mousemove
                            on:mouseup=on_plan_mouseup
                            on:mouseleave=on_plan_mouseleave
                            style="cursor: crosshair;"
                        ></div>
                        <div class="area-placement-info">
                            {move || {
                                let n_poles = poles.get().len();
                                let lpp = arrangement.get().luminaires_per_pole();
                                let total = n_poles * lpp;
                                format!("{n_poles} poles × {lpp} luminaires/pole = {total} total")
                            }}
                        </div>
                    </div>

                    // ISO view + stats + export
                    <div class="area-panel">
                        <div style="display: flex; align-items: center; gap: 12px;">
                            <h3 style="margin: 0;">{move || locale.get().area_designer.combined.clone()}</h3>
                            <label style="font-size: 0.8rem; display: flex; align-items: center; gap: 4px; cursor: pointer; user-select: none;">
                                <input type="checkbox"
                                    prop:checked=show_heatmap_values
                                    on:change=move |ev| {
                                        let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
                                        set_show_heatmap_values.set(input.checked());
                                    }
                                />
                                "Show values"
                            </label>
                        </div>
                        {move || {
                            let (result, iso_svg, _n) = area_data();
                            let result_for_csv = result.clone();
                            let u = units.get();
                            let svg = AreaSvg::iso_view_opts(&result, 600.0, 450.0, &SvgTheme::css_variables_with_locale(&locale.get()), u, show_heatmap_values.get());
                            view! {
                                <div class="area-iso-svg" inner_html=svg></div>
                                <div class="area-stats">
                                    <table class="area-stats-table">
                                        <tr>
                                            <td>{move || locale.get().designer.min.clone()}</td>
                                            <td class="val">{u.format_lux(result.min_lux)}</td>
                                            <td>{move || locale.get().designer.avg.clone()}</td>
                                            <td class="val">{u.format_lux(result.avg_lux)}</td>
                                            <td>{move || locale.get().designer.max.clone()}</td>
                                            <td class="val">{u.format_lux(result.max_lux)}</td>
                                        </tr>
                                        <tr>
                                            <td>{move || locale.get().area_designer.stats.u0.clone()}</td>
                                            <td class="val">{format!("{:.3}", result.uniformity_min_avg)}</td>
                                            <td>{move || locale.get().area_designer.stats.ud.clone()}</td>
                                            <td class="val">{format!("{:.3}", result.uniformity_min_max)}</td>
                                            <td>{move || locale.get().area_designer.stats.avg_min.clone()}</td>
                                            <td class="val">{
                                                if result.uniformity_avg_min.is_finite() {
                                                    format!("{:.1}", result.uniformity_avg_min)
                                                } else {
                                                    "∞".to_string()
                                                }
                                            }</td>
                                        </tr>
                                    </table>
                                </div>
                                <div class="area-export-buttons">
                                    <button class="area-export-btn"
                                        on:click=move |_| {
                                            let r = result_for_csv.clone();
                                            let u = units.get();
                                            let dl = u.distance_label();
                                            let il = u.illuminance_label();
                                            let n = r.grid_resolution;
                                            let dx = r.area_width / n as f64;
                                            let dy = r.area_depth / n as f64;
                                            let mut csv = format!("Y({dl})\\X({dl})");
                                            for c in 0..n {
                                                csv.push_str(&format!(",{:.2}", u.convert_meters((c as f64 + 0.5) * dx)));
                                            }
                                            csv.push('\n');
                                            for row in 0..n {
                                                let y = u.convert_meters((row as f64 + 0.5) * dy);
                                                csv.push_str(&format!("{y:.2}"));
                                                for col in 0..n {
                                                    csv.push_str(&format!(",{:.1}", u.convert_lux(r.lux_grid[row][col])));
                                                }
                                                csv.push('\n');
                                            }
                                            csv.push_str(&format!("\nMin ({il}),{:.1}\n", u.convert_lux(r.min_lux)));
                                            csv.push_str(&format!("Avg ({il}),{:.1}\n", u.convert_lux(r.avg_lux)));
                                            csv.push_str(&format!("Max ({il}),{:.1}\n", u.convert_lux(r.max_lux)));
                                            csv.push_str(&format!("U0 (min/avg),{:.3}\n", r.uniformity_min_avg));
                                            csv.push_str(&format!("Ud (min/max),{:.3}\n", r.uniformity_min_max));
                                            super::file_handler::download_file("illuminance_grid.csv", &csv, "text/csv");
                                        }
                                    >{move || locale.get().designer.export_csv.clone()}</button>
                                    <button class="area-export-btn"
                                        on:click=move |_| {
                                            let svg = iso_svg.clone();
                                            super::file_handler::download_svg("illuminance_iso.svg", &svg);
                                        }
                                    >{move || locale.get().area_designer.export_iso_svg.clone()}</button>
                                    <button class="area-export-btn"
                                        on:click=move |_| {
                                            let svg = plan_svg();
                                            super::file_handler::download_svg("area_plan.svg", &svg);
                                        }
                                    >{move || locale.get().area_designer.export_plan_svg.clone()}</button>
                                    <button class="area-export-btn"
                                        on:click=move |_| {
                                            let results = opt_results.get();
                                            if results.is_empty() { return; }
                                            let u = units.get();
                                            let dl = u.distance_label();
                                            let il = u.illuminance_label();
                                            let mut csv = format!("Height ({dl}),Spacing ({dl}),Min ({il}),Avg ({il}),Max ({il}),U0,Ud,Poles,Pass\n");
                                            for row in &results {
                                                csv.push_str(&format!(
                                                    "{:.1},{:.1},{:.1},{:.1},{:.1},{:.3},{:.3},{},{}\n",
                                                    u.convert_meters(row.mounting_height),
                                                    u.convert_meters(row.optimal_spacing),
                                                    u.convert_lux(row.min_lux),
                                                    u.convert_lux(row.avg_lux),
                                                    u.convert_lux(row.max_lux),
                                                    row.uniformity_min_avg,
                                                    row.uniformity_min_max,
                                                    row.poles_needed,
                                                    if row.meets_criteria { "Yes" } else { "No" },
                                                ));
                                            }
                                            super::file_handler::download_file("optimizer_results.csv", &csv, "text/csv");
                                        }
                                    >{move || locale.get().area_designer.export_optimizer.clone()}</button>
                                    <button class="area-export-btn"
                                        disabled=move || pdf_exporting.get()
                                        on:click=move |_| {
                                            let ldt_val = ldt.get();
                                            let u = units.get();
                                            let dl = u.distance_label();
                                            let il = u.illuminance_label();
                                            let (r, _, _) = area_data();
                                            let pf = proration.get();
                                            let h = mounting_height.get();
                                            let (placements, _) = make_placements();

                                            // Build Typst source for PDF generation
                                            let plan = plan_svg();
                                            let iso = AreaSvg::iso_view(
                                                &r, 600.0, 450.0,
                                                &SvgTheme::light(),
                                                u,
                                            );
                                            let room = AreaSvg::room_view(
                                                &r, &placements, &ldt_val,
                                                h, pf,
                                                600.0, 420.0,
                                                &SvgTheme::light(), u,
                                            );

                                            let name = if ldt_val.luminaire_name.is_empty() {
                                                "Luminaire"
                                            } else {
                                                &ldt_val.luminaire_name
                                            };

                                            let mut typ = String::from(
                                                "#set page(paper: \"a4\", margin: 1.5cm)\n\
                                                 #set text(font: \"Helvetica\", size: 10pt)\n\n\
                                                 = Area Lighting Design Report\n\n"
                                            );
                                            typ.push_str(&format!("*Luminaire:* {name}\n\n"));
                                            typ.push_str(&format!(
                                                "*Area:* {:.0} {dl} × {:.0} {dl}\n\n",
                                                u.convert_meters(r.area_width),
                                                u.convert_meters(r.area_depth),
                                            ));

                                            // Stats table
                                            typ.push_str("== Illuminance Results\n\n");
                                            typ.push_str("#table(\n  columns: 6,\n  align: center,\n");
                                            typ.push_str(&format!(
                                                "  [Min ({il})], [*{:.1}*], [Avg ({il})], [*{:.1}*], [Max ({il})], [*{:.1}*],\n",
                                                u.convert_lux(r.min_lux),
                                                u.convert_lux(r.avg_lux),
                                                u.convert_lux(r.max_lux),
                                            ));
                                            typ.push_str(&format!(
                                                "  [U#sub[0] (min/avg)], [*{:.3}*], [U#sub[d] (min/max)], [*{:.3}*], [Avg/Min], [*{}*],\n)\n\n",
                                                r.uniformity_min_avg,
                                                r.uniformity_min_max,
                                                if r.uniformity_avg_min.is_finite() { format!("{:.1}", r.uniformity_avg_min) } else { "∞".to_string() },
                                            ));

                                            // Embed SVGs inline (escape for Typst string)
                                            let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");

                                            typ.push_str("== Plan View\n\n");
                                            typ.push_str(&format!(
                                                "#align(center)[#image(bytes(\"{}\"), width: 95%)]\n\n", escape(&plan),
                                            ));
                                            typ.push_str("== Illuminance Heatmap\n\n");
                                            typ.push_str(&format!(
                                                "#align(center)[#image(bytes(\"{}\"), width: 95%)]\n\n", escape(&iso),
                                            ));
                                            typ.push_str("#pagebreak()\n\n");
                                            typ.push_str("== Room View\n\n");
                                            typ.push_str(&format!(
                                                "#align(center)[#image(bytes(\"{}\"), width: 95%)]\n\n", escape(&room),
                                            ));

                                            // Optimizer results if available
                                            let opt_res = opt_results.get();
                                            if !opt_res.is_empty() {
                                                typ.push_str("== Optimizer Results\n\n");
                                                typ.push_str(&format!(
                                                    "#table(\n  columns: 8,\n  align: center,\n  \
                                                    [Height ({dl})], [Spacing ({dl})], [Min ({il})], [Avg ({il})], [Max ({il})], [U#sub[0]], [Poles], [Pass],\n"
                                                ));
                                                for row in &opt_res {
                                                    typ.push_str(&format!(
                                                        "  [{:.0}], [{:.0}], [{:.0}], [{:.0}], [{:.0}], [{:.2}], [{}], [{}],\n",
                                                        u.convert_meters(row.mounting_height),
                                                        u.convert_meters(row.optimal_spacing),
                                                        u.convert_lux(row.min_lux),
                                                        u.convert_lux(row.avg_lux),
                                                        u.convert_lux(row.max_lux),
                                                        row.uniformity_min_avg,
                                                        row.poles_needed,
                                                        if row.meets_criteria { "Yes" } else { "No" },
                                                    ));
                                                }
                                                typ.push_str(")\n");
                                            }

                                            set_pdf_exporting.set(true);
                                            wasm_bindgen_futures::spawn_local(async move {
                                                match compile_typst_to_pdf(&typ).await {
                                                    Ok(pdf_bytes) => {
                                                        super::file_handler::download_bytes(
                                                            "area_design_report.pdf", &pdf_bytes, "application/pdf",
                                                        );
                                                    }
                                                    Err(e) => {
                                                        web_sys::console::error_1(
                                                            &format!("PDF export failed: {e}").into(),
                                                        );
                                                        // Fallback: download .typ source
                                                        web_sys::window().unwrap()
                                                            .alert_with_message(
                                                                &format!("PDF export failed: {e}. Downloading .typ file instead."),
                                                            ).ok();
                                                        super::file_handler::download_file(
                                                            "area_design_report.typ", &typ, "text/plain",
                                                        );
                                                    }
                                                }
                                                set_pdf_exporting.set(false);
                                            });
                                        }
                                    >{move || if pdf_exporting.get() { locale.get().designer.exporting_pdf.clone() } else { locale.get().designer.export_pdf.clone() }}</button>
                                </div>
                            }
                        }}
                    </div>

                    // View tab bar
                    <div class="area-view-tabs">
                        <button
                            class=move || if area_view_tab.get() == "room" { "area-view-tab active" } else { "area-view-tab" }
                            on:click=move |_| set_area_view_tab.set("room".to_string())
                        >{move || locale.get().designer.room_view.clone()}</button>
                        <button
                            class=move || if area_view_tab.get() == "3d" { "area-view-tab active" } else { "area-view-tab" }
                            on:click=move |_| set_area_view_tab.set("3d".to_string())
                        >"3D Scene"</button>

                        // 3D options (inline when 3D tab active)
                        {move || (area_view_tab.get() == "3d").then(|| view! {
                            <label style="display:flex; align-items:center; gap:4px; font-size:12px; margin-left:8px;">
                                <input type="checkbox"
                                    prop:checked=move || show_light_cones.get()
                                    on:change=move |ev| set_show_light_cones.set(event_target_checked(&ev))
                                />" Light cones"
                            </label>
                            <select style="font-size:12px; padding:2px 4px; margin-left:4px;"
                                on:change=move |ev| {
                                    let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                                    let p = CameraPreset::all()[idx];
                                    set_scene_camera_preset.set(p);
                                    let cam = p.to_camera(1.0, 1.0, 1.0);
                                    set_area_cam_az.set(cam.azimuth);
                                    set_area_cam_el.set(cam.elevation);
                                    set_area_cam_zoom.set(1.0);
                                }
                            >
                                {CameraPreset::all().iter().enumerate().map(|(i, p)| {
                                    view! { <option value={i.to_string()} selected=i==0>{p.label()}</option> }
                                }).collect::<Vec<_>>()}
                            </select>
                        })}
                    </div>

                    // Tab content
                    {move || {
                        let tab = area_view_tab.get();
                        match tab.as_str() {
                            "room" => {
                                let ldt_val = ldt.get();
                                let (placements, _ldt_idx) = make_placements();
                                let (result, _, _) = area_data();
                                let h = mounting_height.get();
                                let pf = proration.get();
                                let svg_theme = SvgTheme::css_variables_with_locale(&locale.get());
                                let u = units.get();
                                let room_svg = AreaSvg::room_view(
                                    &result, &placements, &ldt_val,
                                    h, pf,
                                    600.0, 420.0,
                                    &svg_theme, u,
                                );
                                view! {
                                    <div class="area-panel">
                                        <h3>{move || locale.get().designer.room_view.clone()}</h3>
                                        <div class="area-iso-svg" inner_html=room_svg.clone()></div>
                                        <div class="area-export-buttons">
                                            <button class="area-export-btn"
                                                on:click=move |_| {
                                                    let svg = room_svg.clone();
                                                    super::file_handler::download_svg("area_room.svg", &svg);
                                                }
                                            >{move || locale.get().area_designer.export_room_svg.clone()}</button>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            _ => {
                                // "3d" (default) — interactive
                                let (placements, _ldt_idx) = make_placements();
                                let (result, _, _) = area_data();
                                let h = mounting_height.get();

                                let faces = build_exterior_scene(&result, &placements, show_light_cones.get());

                                let svg_w = 600.0;
                                let svg_h = 450.0;
                                let az = area_cam_az.get();
                                let el = area_cam_el.get();
                                let zoom = area_cam_zoom.get();
                                let cam = eulumdat::scene3d::SceneCamera {
                                    azimuth: az,
                                    elevation: el,
                                    scale: 1.0,
                                    center: (svg_w / 2.0, svg_h / 2.0),
                                };
                                let base_scale = fit_scale(result.area_width, result.area_depth, h, svg_w, svg_h, &cam);
                                let cam = eulumdat::scene3d::SceneCamera {
                                    scale: base_scale * zoom,
                                    ..cam
                                };

                                let scene_svg = render_scene_svg(&faces, &cam, svg_w, svg_h, "#f8f9fa");
                                view! {
                                    <div class="area-panel">
                                        <h3>"3D Scene"
                                            <span style="font-size:10px; font-weight:normal; opacity:0.5; margin-left:8px;">
                                                "drag to rotate, scroll to zoom"
                                            </span>
                                        </h3>
                                        <div class="area-3d-interactive"
                                            inner_html=scene_svg.clone()
                                            style="cursor:grab; user-select:none;"
                                            on:mousedown=move |ev| {
                                                ev.prevent_default();
                                                set_area_dragging.set(true);
                                                set_area_drag_start.set((ev.client_x() as f64, ev.client_y() as f64));
                                                set_area_drag_az0.set(area_cam_az.get());
                                                set_area_drag_el0.set(area_cam_el.get());
                                            }
                                            on:mousemove=move |ev| {
                                                if !area_dragging.get() { return; }
                                                ev.prevent_default();
                                                let (sx, sy) = area_drag_start.get();
                                                let dx = ev.client_x() as f64 - sx;
                                                let dy = ev.client_y() as f64 - sy;
                                                set_area_cam_az.set(area_drag_az0.get() - dx * 0.5);
                                                set_area_cam_el.set((area_drag_el0.get() + dy * 0.5).clamp(0.0, 89.0));
                                            }
                                            on:mouseup=move |_| set_area_dragging.set(false)
                                            on:mouseleave=move |_| set_area_dragging.set(false)
                                            on:wheel=move |ev: web_sys::WheelEvent| {
                                                ev.prevent_default();
                                                let delta = ev.delta_y();
                                                let factor = if delta > 0.0 { 0.9 } else { 1.1 };
                                                set_area_cam_zoom.set((area_cam_zoom.get() * factor).clamp(0.3, 5.0));
                                            }
                                        ></div>
                                        <div class="area-export-buttons">
                                            <button class="area-export-btn"
                                                on:click=move |_| {
                                                    let svg = scene_svg.clone();
                                                    super::file_handler::download_svg("area_3d_scene.svg", &svg);
                                                }
                                            >{move || locale.get().area_designer.export_3d_svg.clone()}</button>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
