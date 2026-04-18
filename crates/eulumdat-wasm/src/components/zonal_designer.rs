//! Zonal Cavity Interior Lighting Designer — Leptos WASM component.

use eulumdat::diagram::SvgTheme;
use eulumdat::scene3d::{build_interior_scene, fit_scale, render_scene_svg, CameraPreset};
use eulumdat::zonal::{
    compute_cavity_ratios, compute_ppb_overlay, compute_zonal, LightLossFactor, LlfPreset,
    ReflectancePreset, Reflectances, Room, RoomPreset, SolveMode, ZonalSvg,
};
use eulumdat::CuTable;
use eulumdat::Eulumdat;
use eulumdat::PhotometricCalculations;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::use_locale;

// ── URL state serialization ──────────────────────────────────────────────────

/// Build a URL hash string from zonal designer params.
#[allow(clippy::too_many_arguments)]
fn zonal_params_to_hash(
    preset: usize,
    l: f64,
    w: f64,
    h: f64,
    wp: f64,
    sus: f64,
    target: f64,
    refl: usize,
    rc: f64,
    rw: f64,
    rf: f64,
    llf: usize,
    mode: &SolveMode,
    count: usize,
    lpd: f64,
    view: &str,
) -> String {
    let mut parts = vec![
        format!("preset={preset}"),
        format!("l={l:.1}"),
        format!("w={w:.1}"),
        format!("h={h:.1}"),
        format!("wp={wp:.2}"),
        format!("sus={sus:.2}"),
        format!("target={target:.0}"),
        format!("refl={refl}"),
    ];
    // Only include custom reflectances if preset is non-standard (could be edited)
    parts.push(format!("rc={:.0}", rc * 100.0));
    parts.push(format!("rw={:.0}", rw * 100.0));
    parts.push(format!("rf={:.0}", rf * 100.0));
    parts.push(format!("llf={llf}"));
    let mode_str = match mode {
        SolveMode::TargetToCount => "target",
        SolveMode::CountToIlluminance => "count",
        SolveMode::TargetToLpd => "lpd",
    };
    parts.push(format!("mode={mode_str}"));
    if matches!(mode, SolveMode::CountToIlluminance) {
        parts.push(format!("count={count}"));
    }
    if matches!(mode, SolveMode::TargetToLpd) {
        parts.push(format!("lpd={lpd:.1}"));
    }
    parts.push(format!("view={view}"));
    format!("#interior?{}", parts.join("&"))
}

/// Parse zonal designer params from URL hash fragment.
fn parse_interior_hash_params() -> Option<std::collections::HashMap<String, String>> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    if !hash.starts_with("#interior?") {
        return None;
    }
    let query = &hash["#interior?".len()..];
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
    Some(map)
}

/// Update the URL hash without triggering a page reload.
fn set_interior_url_hash(hash: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(history) = window.history() {
            let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(hash));
        }
    }
}

/// Copy current URL to clipboard.
fn copy_interior_url_to_clipboard() {
    if let Some(window) = web_sys::window() {
        if let Ok(href) = window.location().href() {
            let _ = window.navigator().clipboard().write_text(&href);
        }
    }
}

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

/// Zonal Cavity Interior Lighting Designer component.
#[component]
pub fn ZonalDesigner(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let units = super::app::use_unit_system();

    // ─── Parse URL hash for initial values ───────────────────────────────
    let url_params = parse_interior_hash_params();
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

    // ─── Room preset ────────────────────────────────────────────────────
    let url_preset_idx = url_get("preset").and_then(|v| v.parse::<usize>().ok());
    let init_room_preset = url_preset_idx
        .and_then(|i| RoomPreset::all().get(i).copied())
        .unwrap_or(RoomPreset::Classroom);
    let default_room = init_room_preset.to_room();

    let (room_preset, set_room_preset) = signal(init_room_preset);
    let (room_length, set_room_length) = signal(url_f64("l", default_room.length));
    let (room_width, set_room_width) = signal(url_f64("w", default_room.width));
    let (room_height, set_room_height) = signal(url_f64("h", default_room.height));
    let (workplane_height, set_workplane_height) =
        signal(url_f64("wp", default_room.workplane_height));
    let (suspension_length, set_suspension_length) =
        signal(url_f64("sus", default_room.suspension_length));
    let (target_illuminance, set_target_illuminance) =
        signal(url_f64("target", init_room_preset.target_lux()));

    // ─── Reflectances ───────────────────────────────────────────────────
    let url_refl_idx = url_get("refl").and_then(|v| v.parse::<usize>().ok());
    let init_refl_preset = url_refl_idx
        .and_then(|i| ReflectancePreset::all().get(i).copied())
        .unwrap_or(ReflectancePreset::Standard);
    let default_refl = init_refl_preset.to_reflectances();

    let (refl_preset, set_refl_preset) = signal(init_refl_preset);
    let (rho_ceiling, set_rho_ceiling) =
        signal(url_f64("rc", default_refl.ceiling * 100.0) / 100.0);
    let (rho_wall, set_rho_wall) = signal(url_f64("rw", default_refl.wall * 100.0) / 100.0);
    let (rho_floor, set_rho_floor) = signal(url_f64("rf", default_refl.floor * 100.0) / 100.0);

    // ─── LLF ────────────────────────────────────────────────────────────
    let url_llf_idx = url_get("llf").and_then(|v| v.parse::<usize>().ok());
    let init_llf_preset = url_llf_idx
        .and_then(|i| LlfPreset::all().get(i).copied())
        .unwrap_or(LlfPreset::Led);
    let default_llf = init_llf_preset.to_llf();

    let (llf_preset, set_llf_preset) = signal(init_llf_preset);
    let (lld, set_lld) = signal(default_llf.lld);
    let (ldd, set_ldd) = signal(default_llf.ldd);
    let (bf, set_bf) = signal(default_llf.ballast_factor);
    let (rsdd, set_rsdd) = signal(default_llf.rsdd);

    // ─── Solve mode ─────────────────────────────────────────────────────
    let init_solve_mode = match url_get("mode").as_deref() {
        Some("count") => SolveMode::CountToIlluminance,
        Some("lpd") => SolveMode::TargetToLpd,
        _ => SolveMode::TargetToCount,
    };
    let (solve_mode, set_solve_mode) = signal(init_solve_mode);
    let (fixed_count, set_fixed_count) = signal(
        url_get("count")
            .and_then(|v| v.parse().ok())
            .unwrap_or(12_usize),
    );
    let (target_lpd, set_target_lpd) = signal(url_f64("lpd", 10.0));

    // ─── View tab ───────────────────────────────────────────────────────
    // Unified view toggle: "heatmap", "3d", "section", "cu"
    let init_view = if url_get("3d").map(|v| v != "0").unwrap_or(true) {
        "3d"
    } else if url_get("ppb").map(|v| v == "1").unwrap_or(false) {
        "heatmap"
    } else {
        "3d"
    };
    let (view_tab, set_view_tab) = signal(init_view.to_string());

    // Derived booleans for backwards compatibility with computation logic
    let show_ppb = Memo::new(move |_| {
        let tab = view_tab.get();
        tab == "heatmap" || tab == "table" || tab == "3d"
    });
    let show_3d = Memo::new(move |_| view_tab.get() == "3d");

    let (show_heatmap_values, set_show_heatmap_values) = signal(false);
    let (show_cavities_3d, set_show_cavities_3d) = signal(false);
    let (show_light_cones, set_show_light_cones) = signal(true);
    let (camera_preset, set_camera_preset) = signal(CameraPreset::FrontRight);

    // Interactive camera: azimuth/elevation overrides from mouse drag
    let (cam_azimuth, set_cam_azimuth) = signal(30.0_f64);
    let (cam_elevation, set_cam_elevation) = signal(30.0_f64);
    let (cam_zoom, set_cam_zoom) = signal(1.0_f64);
    let (dragging_3d, set_dragging_3d) = signal(false);
    let (drag_start, set_drag_start) = signal((0.0_f64, 0.0_f64));
    let (drag_az_start, set_drag_az_start) = signal(30.0_f64);
    let (drag_el_start, set_drag_el_start) = signal(30.0_f64);

    // ─── PDF export / share ──────────────────────────────────────────────
    let (pdf_exporting, set_pdf_exporting) = signal(false);
    let (link_copied, set_link_copied) = signal(false);

    // ─── URL sync: update hash whenever parameters change ────────────────
    Effect::new(move |_| {
        let preset_idx = RoomPreset::all()
            .iter()
            .position(|p| *p == room_preset.get())
            .unwrap_or(0);
        let refl_idx = ReflectancePreset::all()
            .iter()
            .position(|p| *p == refl_preset.get())
            .unwrap_or(0);
        let llf_idx = LlfPreset::all()
            .iter()
            .position(|p| *p == llf_preset.get())
            .unwrap_or(0);
        let hash = zonal_params_to_hash(
            preset_idx,
            room_length.get(),
            room_width.get(),
            room_height.get(),
            workplane_height.get(),
            suspension_length.get(),
            target_illuminance.get(),
            refl_idx,
            rho_ceiling.get(),
            rho_wall.get(),
            rho_floor.get(),
            llf_idx,
            &solve_mode.get(),
            fixed_count.get(),
            target_lpd.get(),
            &view_tab.get(),
        );
        set_interior_url_hash(&hash);
    });

    // ─── Apply room preset ──────────────────────────────────────────────
    let apply_room_preset = move |preset: RoomPreset| {
        set_room_preset.set(preset);
        let room = preset.to_room();
        set_room_length.set(room.length);
        set_room_width.set(room.width);
        set_room_height.set(room.height);
        set_workplane_height.set(room.workplane_height);
        set_suspension_length.set(room.suspension_length);
        set_target_illuminance.set(preset.target_lux());
    };

    // ─── Apply reflectance preset ───────────────────────────────────────
    let apply_refl_preset = move |preset: ReflectancePreset| {
        set_refl_preset.set(preset);
        let r = preset.to_reflectances();
        set_rho_ceiling.set(r.ceiling);
        set_rho_wall.set(r.wall);
        set_rho_floor.set(r.floor);
    };

    // ─── Apply LLF preset ───────────────────────────────────────────────
    let apply_llf_preset = move |preset: LlfPreset| {
        set_llf_preset.set(preset);
        let l = preset.to_llf();
        set_lld.set(l.lld);
        set_ldd.set(l.ldd);
        set_bf.set(l.ballast_factor);
        set_rsdd.set(l.rsdd);
    };

    // ─── CU table (Memo — computed once per LDT change) ────────────────
    let cu_table = Memo::new(move |_| CuTable::calculate(&ldt.get()));

    // ─── Main computation (reactive) ────────────────────────────────────
    let result = Memo::new(move |_| {
        let room = Room::new(
            room_length.get(),
            room_width.get(),
            room_height.get(),
            workplane_height.get(),
            suspension_length.get(),
        );
        let reflectances = Reflectances::new(rho_ceiling.get(), rho_wall.get(), rho_floor.get());
        let llf = LightLossFactor::new(lld.get(), ldd.get(), bf.get(), rsdd.get());
        let ct = cu_table.get();

        let mut zr = compute_zonal(
            &ldt.get(),
            &room,
            &reflectances,
            &llf,
            target_illuminance.get(),
            &ct,
            solve_mode.get(),
            Some(fixed_count.get()),
            Some(target_lpd.get()),
        );

        // PPB overlay (for heatmap view or 3D scene lighting)
        if (show_ppb.get() || show_3d.get()) && zr.layout.count > 0 {
            let ppb = compute_ppb_overlay(
                &ldt.get(),
                &zr.layout,
                &room,
                30, // grid resolution
                zr.llf_total,
                zr.cu,
                zr.achieved_illuminance,
            );
            zr.ppb = Some(ppb);
        }

        // Sync designer data to localStorage for Bevy 3D viewer
        super::bevy_scene::save_designer_interior_to_storage(
            &room,
            &zr.layout,
            &reflectances,
            &zr.cavity,
            zr.ppb.as_ref(),
        );

        zr
    });

    // ─── Luminaire info ─────────────────────────────────────────────────
    let luminaire_info = Memo::new(move |_| {
        let l = ldt.get();
        let flux: f64 = l
            .lamp_sets
            .iter()
            .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
            .sum();
        let watts: f64 = l
            .lamp_sets
            .iter()
            .map(|ls| ls.wattage_with_ballast * ls.num_lamps.unsigned_abs() as f64)
            .sum();
        let name = l.luminaire_name.clone();
        let dff = l.downward_flux_fraction;
        (name, flux, watts, dff)
    });

    // ─── Zonal lumen summary ────────────────────────────────────────────
    let zonal_lumens = Memo::new(move |_| PhotometricCalculations::zonal_lumens_30deg(&ldt.get()));

    // ─── Render ─────────────────────────────────────────────────────────
    view! {
        <div class="zonal-designer">
            // ─── Toolbar ──────────────────────────────────────────────
            <div class="zonal-toolbar">
                <div class="zonal-toolbar-row">
                    // Room preset
                    <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.room.clone()}</span>
                    <select class="zonal-toolbar-select" on:change=move |ev| {
                        let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                        apply_room_preset(RoomPreset::all()[idx]);
                    }>
                        {RoomPreset::all().iter().enumerate().map(|(i, p)| {
                            view! {
                                <option value={i.to_string()} selected=move || room_preset.get() == *p>
                                    {p.label()}
                                </option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>

                    // Length
                    <span class="zonal-toolbar-label">{move || locale.get().designer.length.clone()}</span>
                    <input type="range" min="2" max="50" step="0.5" class="zonal-toolbar-range"
                        prop:value=move || format!("{:.1}", room_length.get())
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                set_room_length.set(v.max(2.0));
                            }
                        }
                    />
                    <span class="zonal-toolbar-val">{move || format!("{:.1}m", room_length.get())}</span>

                    // Width
                    <span class="zonal-toolbar-label">{move || locale.get().designer.width.clone()}</span>
                    <input type="range" min="2" max="50" step="0.5" class="zonal-toolbar-range"
                        prop:value=move || format!("{:.1}", room_width.get())
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                set_room_width.set(v.max(2.0));
                            }
                        }
                    />
                    <span class="zonal-toolbar-val">{move || format!("{:.1}m", room_width.get())}</span>

                    // Height
                    <span class="zonal-toolbar-label">{move || locale.get().designer.height.clone()}</span>
                    <input type="range" min="2" max="12" step="0.1" class="zonal-toolbar-range"
                        prop:value=move || format!("{:.1}", room_height.get())
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                set_room_height.set(v.max(2.0));
                            }
                        }
                    />
                    <span class="zonal-toolbar-val">{move || format!("{:.1}m", room_height.get())}</span>
                </div>
                <div class="zonal-toolbar-row">
                    // Workplane
                    <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.workplane.clone()}</span>
                    <input type="range" min="0" max="1.5" step="0.05" class="zonal-toolbar-range"
                        prop:value=move || format!("{:.2}", workplane_height.get())
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                set_workplane_height.set(v.max(0.0));
                            }
                        }
                    />
                    <span class="zonal-toolbar-val">{move || format!("{:.2}m", workplane_height.get())}</span>

                    // Suspension
                    <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.suspension.clone()}</span>
                    <input type="range" min="0" max="1.5" step="0.05" class="zonal-toolbar-range"
                        prop:value=move || format!("{:.2}", suspension_length.get())
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                set_suspension_length.set(v.max(0.0));
                            }
                        }
                    />
                    <span class="zonal-toolbar-val">{move || format!("{:.2}m", suspension_length.get())}</span>

                    // Solve mode
                    <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.mode.label.clone()}</span>
                    <select class="zonal-toolbar-select" on:change=move |ev| {
                        let v = event_target_value(&ev);
                        set_solve_mode.set(match v.as_str() {
                            "count" => SolveMode::CountToIlluminance,
                            "lpd" => SolveMode::TargetToLpd,
                            _ => SolveMode::TargetToCount,
                        });
                    }>
                        <option value="target" selected=move || solve_mode.get() == SolveMode::TargetToCount>{move || locale.get().zonal_designer.mode.target_to_count.clone()}</option>
                        <option value="count" selected=move || solve_mode.get() == SolveMode::CountToIlluminance>{move || locale.get().zonal_designer.mode.count_to_illuminance.clone()}</option>
                        <option value="lpd" selected=move || solve_mode.get() == SolveMode::TargetToLpd>{move || locale.get().zonal_designer.mode.target_lpd.clone()}</option>
                    </select>

                    // Mode-specific input
                    {move || match solve_mode.get() {
                        SolveMode::TargetToCount => view! {
                            <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.target.clone()}</span>
                            <input type="number" step="25" min="10" max="5000" class="zonal-toolbar-num"
                                prop:value=move || format!("{:.0}", target_illuminance.get())
                                on:change=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_target_illuminance.set(v.max(10.0));
                                    }
                                }
                            />
                            <span class="zonal-toolbar-val">"lux"</span>
                        }.into_any(),
                        SolveMode::CountToIlluminance => view! {
                            <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.count.clone()}</span>
                            <input type="number" step="1" min="1" max="500" class="zonal-toolbar-num"
                                prop:value=move || format!("{}", fixed_count.get())
                                on:change=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                                        set_fixed_count.set(v.max(1));
                                    }
                                }
                            />
                        }.into_any(),
                        SolveMode::TargetToLpd => view! {
                            <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.lpd.clone()}</span>
                            <input type="number" step="0.5" min="1" max="50" class="zonal-toolbar-num"
                                prop:value=move || format!("{:.1}", target_lpd.get())
                                on:change=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_target_lpd.set(v.max(1.0));
                                    }
                                }
                            />
                            <span class="zonal-toolbar-val">{"\u{0057}/m\u{00B2}"}</span>
                        }.into_any(),
                    }}

                    // Reflectance preset
                    <span class="zonal-toolbar-label">{move || locale.get().zonal_designer.reflectances.title.clone()}</span>
                    <select class="zonal-toolbar-select" on:change=move |ev| {
                        let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                        apply_refl_preset(ReflectancePreset::all()[idx]);
                    }>
                        {ReflectancePreset::all().iter().enumerate().map(|(i, p)| {
                            view! {
                                <option value={i.to_string()} selected=move || refl_preset.get() == *p>
                                    {p.label()}
                                </option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>

                    // View tab toggle
                    <div class="zonal-view-tabs">
                        <button
                            class=move || if view_tab.get() == "heatmap" { "zonal-view-tab active" } else { "zonal-view-tab" }
                            on:click=move |_| set_view_tab.set("heatmap".to_string())
                        >{move || locale.get().zonal_designer.views.heatmap.clone()}</button>
                        <button
                            class=move || if view_tab.get() == "table" { "zonal-view-tab active" } else { "zonal-view-tab" }
                            on:click=move |_| set_view_tab.set("table".to_string())
                        >"Schedule"</button>
                        <button
                            class=move || if view_tab.get() == "3d" { "zonal-view-tab active" } else { "zonal-view-tab" }
                            on:click=move |_| set_view_tab.set("3d".to_string())
                        >{move || locale.get().zonal_designer.views.room_3d.clone()}</button>
                        <button
                            class=move || if view_tab.get() == "section" { "zonal-view-tab active" } else { "zonal-view-tab" }
                            on:click=move |_| set_view_tab.set("section".to_string())
                        >{move || locale.get().zonal_designer.views.section.clone()}</button>
                        <button
                            class=move || if view_tab.get() == "cu" { "zonal-view-tab active" } else { "zonal-view-tab" }
                            on:click=move |_| set_view_tab.set("cu".to_string())
                        >{move || locale.get().zonal_designer.views.cu_table.clone()}</button>
                    </div>
                </div>
            </div>

            // ─── Layout: sidebar + views ──────────────────────────────
            <div class="zonal-layout">
                // ─── Sidebar ──────────────────────────────────────────
                <div class="zonal-sidebar">
                    // Luminaire info
                    <div class="zonal-sidebar-section">
                        <h4>{move || locale.get().designer.luminaire.clone()}</h4>
                        <div class="zonal-info-grid">
                            <span class="label">{move || locale.get().zonal_designer.info.name.clone()}</span>
                            <span>{move || luminaire_info.get().0}</span>
                            <span class="label">{move || locale.get().zonal_designer.info.lumens.clone()}</span>
                            <span>{move || format!("{:.0} lm", luminaire_info.get().1)}</span>
                            <span class="label">{move || locale.get().zonal_designer.info.power.clone()}</span>
                            <span>{move || format!("{:.1} W", luminaire_info.get().2)}</span>
                            <span class="label">{move || locale.get().zonal_designer.info.dff.clone()}</span>
                            <span>{move || format!("{:.1}%", luminaire_info.get().3)}</span>
                            <span class="label">{move || locale.get().zonal_designer.info.smh.clone()}</span>
                            <span>{move || format!("{:.2}", result.get().spacing_criterion)}</span>
                        </div>
                        // Zonal lumen summary
                        <div class="zonal-flux-summary">
                            <div class="zonal-flux-row">
                                <span class="zonal-flux-label">{"\u{2193} Down (0-90\u{00B0}):"}</span>
                                <span class="zonal-flux-val">{move || format!("{:.1}%", zonal_lumens.get().downward_total())}</span>
                            </div>
                            <div class="zonal-flux-bar-row">
                                <span class="zonal-flux-zone">"0-30\u{00B0}"</span>
                                <div class="zonal-flux-bar">
                                    <div class="zonal-flux-bar-fill zonal-flux-down"
                                        style=move || format!("width:{}%", zonal_lumens.get().zone_0_30.min(100.0))>
                                    </div>
                                </div>
                                <span class="zonal-flux-pct">{move || format!("{:.1}%", zonal_lumens.get().zone_0_30)}</span>
                            </div>
                            <div class="zonal-flux-bar-row">
                                <span class="zonal-flux-zone">"30-60\u{00B0}"</span>
                                <div class="zonal-flux-bar">
                                    <div class="zonal-flux-bar-fill zonal-flux-down"
                                        style=move || format!("width:{}%", zonal_lumens.get().zone_30_60.min(100.0))>
                                    </div>
                                </div>
                                <span class="zonal-flux-pct">{move || format!("{:.1}%", zonal_lumens.get().zone_30_60)}</span>
                            </div>
                            <div class="zonal-flux-bar-row">
                                <span class="zonal-flux-zone">"60-90\u{00B0}"</span>
                                <div class="zonal-flux-bar">
                                    <div class="zonal-flux-bar-fill zonal-flux-down"
                                        style=move || format!("width:{}%", zonal_lumens.get().zone_60_90.min(100.0))>
                                    </div>
                                </div>
                                <span class="zonal-flux-pct">{move || format!("{:.1}%", zonal_lumens.get().zone_60_90)}</span>
                            </div>
                            <div class="zonal-flux-row">
                                <span class="zonal-flux-label">{"\u{2191} Up (90-180\u{00B0}):"}</span>
                                <span class="zonal-flux-val">{move || format!("{:.1}%", zonal_lumens.get().upward_total())}</span>
                            </div>
                            <div class="zonal-flux-bar-row">
                                <span class="zonal-flux-zone">"90-120\u{00B0}"</span>
                                <div class="zonal-flux-bar">
                                    <div class="zonal-flux-bar-fill zonal-flux-up"
                                        style=move || format!("width:{}%", zonal_lumens.get().zone_90_120.min(100.0))>
                                    </div>
                                </div>
                                <span class="zonal-flux-pct">{move || format!("{:.1}%", zonal_lumens.get().zone_90_120)}</span>
                            </div>
                            <div class="zonal-flux-bar-row">
                                <span class="zonal-flux-zone">"120-150\u{00B0}"</span>
                                <div class="zonal-flux-bar">
                                    <div class="zonal-flux-bar-fill zonal-flux-up"
                                        style=move || format!("width:{}%", zonal_lumens.get().zone_120_150.min(100.0))>
                                    </div>
                                </div>
                                <span class="zonal-flux-pct">{move || format!("{:.1}%", zonal_lumens.get().zone_120_150)}</span>
                            </div>
                            <div class="zonal-flux-bar-row">
                                <span class="zonal-flux-zone">"150-180\u{00B0}"</span>
                                <div class="zonal-flux-bar">
                                    <div class="zonal-flux-bar-fill zonal-flux-up"
                                        style=move || format!("width:{}%", zonal_lumens.get().zone_150_180.min(100.0))>
                                    </div>
                                </div>
                                <span class="zonal-flux-pct">{move || format!("{:.1}%", zonal_lumens.get().zone_150_180)}</span>
                            </div>
                        </div>

                        // IESNA Classification + BUG Rating
                        <div class="zonal-info-grid" style="margin-top: 8px; font-size: 0.8rem;">
                            {move || {
                                let l = ldt.get();
                                let cls = eulumdat::iesna_classify(&l);
                                let bug = eulumdat::BugRating::from_eulumdat(&l);
                                let zone = bug.most_restrictive_zone()
                                    .map(|z| z.to_string())
                                    .unwrap_or_else(|| "None".to_string());
                                view! {
                                    <>
                                    <span class="label">"IES Type"</span>
                                    <span title=format!("{}", cls.applicability)>
                                        {format!("{}", cls.lateral_type)}
                                        {if cls.applicability != eulumdat::IesnaApplicability::Applicable {
                                            " *"
                                        } else { "" }}
                                    </span>
                                    <span class="label">"Throw"</span>
                                    <span>{format!("{}", cls.longitudinal)}</span>
                                    <span class="label">"Cutoff"</span>
                                    <span>{format!("{}", cls.cutoff)}</span>
                                    <span class="label">"BUG"</span>
                                    <span>{format!("{}", bug)}</span>
                                    <span class="label">"LZ min"</span>
                                    <span>{zone}</span>
                                    </>
                                }
                            }}
                        </div>
                    </div>

                    // Reflectances
                    <div class="zonal-sidebar-section">
                        <h4>{move || locale.get().zonal_designer.reflectances.title.clone()}</h4>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || { let l = locale.get(); format!("{} {:.0}%", l.zonal_designer.reflectances.ceiling, rho_ceiling.get() * 100.0) }}</span>
                            <input type="range" min="0" max="100" step="5"
                                prop:value=move || format!("{}", (rho_ceiling.get() * 100.0).round() as i32)
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_rho_ceiling.set(v / 100.0);
                                    }
                                }
                            />
                        </div>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || { let l = locale.get(); format!("{} {:.0}%", l.zonal_designer.reflectances.wall, rho_wall.get() * 100.0) }}</span>
                            <input type="range" min="0" max="100" step="5"
                                prop:value=move || format!("{}", (rho_wall.get() * 100.0).round() as i32)
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_rho_wall.set(v / 100.0);
                                    }
                                }
                            />
                        </div>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || { let l = locale.get(); format!("{} {:.0}%", l.zonal_designer.reflectances.floor, rho_floor.get() * 100.0) }}</span>
                            <input type="range" min="0" max="100" step="5"
                                prop:value=move || format!("{}", (rho_floor.get() * 100.0).round() as i32)
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_rho_floor.set(v / 100.0);
                                    }
                                }
                            />
                        </div>
                    </div>

                    // LLF
                    <div class="zonal-sidebar-section">
                        <h4>{move || locale.get().zonal_designer.llf.title.clone()}</h4>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || locale.get().zonal_designer.llf.preset.clone()}</span>
                            <select on:change=move |ev| {
                                let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                                apply_llf_preset(LlfPreset::all()[idx]);
                            }>
                                {LlfPreset::all().iter().enumerate().map(|(i, p)| {
                                    view! {
                                        <option value={i.to_string()} selected=move || llf_preset.get() == *p>
                                            {p.label()}
                                        </option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || { let l = locale.get(); format!("{} {:.0}%", l.zonal_designer.llf.lld, lld.get() * 100.0) }}</span>
                            <input type="range" min="50" max="100" step="1"
                                prop:value=move || format!("{}", (lld.get() * 100.0).round() as i32)
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_lld.set(v / 100.0);
                                    }
                                }
                            />
                        </div>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || { let l = locale.get(); format!("{} {:.0}%", l.zonal_designer.llf.ldd, ldd.get() * 100.0) }}</span>
                            <input type="range" min="50" max="100" step="1"
                                prop:value=move || format!("{}", (ldd.get() * 100.0).round() as i32)
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_ldd.set(v / 100.0);
                                    }
                                }
                            />
                        </div>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label">{move || { let l = locale.get(); format!("{} {:.0}%", l.zonal_designer.llf.bf, bf.get() * 100.0) }}</span>
                            <input type="range" min="50" max="100" step="1"
                                prop:value=move || format!("{}", (bf.get() * 100.0).round() as i32)
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                        set_bf.set(v / 100.0);
                                    }
                                }
                            />
                        </div>
                        <div class="zonal-prop-row">
                            <span class="zonal-prop-label" style="font-weight:bold">{move || { let l = locale.get(); format!("{} {:.3}", l.zonal_designer.llf.total, lld.get() * ldd.get() * bf.get() * rsdd.get()) }}</span>
                        </div>
                    </div>

                    // 3D view options (visible when 3D tab is active)
                    {move || (view_tab.get() == "3d").then(|| view! {
                        <div class="zonal-sidebar-section">
                            <h4>{move || locale.get().zonal_designer.options_3d.clone()}</h4>
                            <label class="zonal-toggle">
                                <input type="checkbox"
                                    prop:checked=move || show_cavities_3d.get()
                                    on:change=move |ev| set_show_cavities_3d.set(event_target_checked(&ev))
                                />{" "}{move || locale.get().zonal_designer.cavity_lines.clone()}
                            </label>
                            <label class="zonal-toggle">
                                <input type="checkbox"
                                    prop:checked=move || show_light_cones.get()
                                    on:change=move |ev| set_show_light_cones.set(event_target_checked(&ev))
                                />{" "}{move || locale.get().zonal_designer.light_cones.clone()}
                            </label>
                            <div class="zonal-prop-row">
                                <span class="zonal-prop-label">{move || locale.get().zonal_designer.camera.clone()}</span>
                                <select on:change=move |ev| {
                                    let idx: usize = event_target_value(&ev).parse().unwrap_or(0);
                                    let p = CameraPreset::all()[idx];
                                    set_camera_preset.set(p);
                                    let cam = p.to_camera(1.0, 1.0, 1.0);
                                    set_cam_azimuth.set(cam.azimuth);
                                    set_cam_elevation.set(cam.elevation);
                                    set_cam_zoom.set(1.0);
                                }>
                                    {CameraPreset::all().iter().enumerate().map(|(i, p)| {
                                        view! {
                                            <option value={i.to_string()} selected=move || camera_preset.get() == *p>
                                                {p.label()}
                                            </option>
                                        }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </div>
                            <div class="zonal-prop-row" style="font-size:10px; opacity:0.6;">
                                {move || format!("Az: {:.0}°  El: {:.0}°  Zoom: {:.0}%",
                                    cam_azimuth.get(), cam_elevation.get(), cam_zoom.get() * 100.0)}
                            </div>
                        </div>
                    })}

                    // Export
                    <div class="zonal-sidebar-section">
                        <h4>{move || locale.get().ui.tabs.export.clone()}</h4>
                        <button class="zonal-share-btn"
                            on:click=move |_| {
                                copy_interior_url_to_clipboard();
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
                        >{move || if link_copied.get() { locale.get().designer.copied.clone() } else { locale.get().designer.share_link.clone() }}</button>
                        <button class="zonal-export-btn"
                            disabled=move || pdf_exporting.get()
                            on:click=move |_| {
                                let ldt_val = ldt.get();
                                let u = units.get();
                                let r = result.get();
                                let ct = cu_table.get();

                                let room = Room::new(
                                    room_length.get(), room_width.get(), room_height.get(),
                                    workplane_height.get(), suspension_length.get(),
                                );

                                let theme = SvgTheme::light();

                                let plan_svg = ZonalSvg::room_plan(&room, &r.layout, &theme, u);
                                let section_svg = ZonalSvg::section_view(&room, &r.cavity, &theme, u);
                                let cu_svg = ZonalSvg::cu_table_svg(&ct, r.cavity.rcr, r.cavity.rho_cc_eff, rho_wall.get(), &theme);

                                let name = if ldt_val.luminaire_name.is_empty() {
                                    "Luminaire"
                                } else {
                                    &ldt_val.luminaire_name
                                };

                                let escape = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");

                                let mut typ = String::from(
                                    "#set page(paper: \"a4\", margin: 1.5cm)\n\
                                     #set text(font: \"Helvetica\", size: 10pt)\n\n\
                                     = Interior Lighting Design Report\n\
                                     === Zonal Cavity Method\n\n"
                                );

                                let info = luminaire_info.get();
                                typ.push_str(&format!("*Luminaire:* {name}\n\n"));
                                typ.push_str(&format!("*Lumens:* {:.0} lm  |  *Power:* {:.1} W  |  *DFF:* {:.1}%  |  *S/MH:* {:.2}\n\n", info.1, info.2, info.3, r.spacing_criterion));

                                typ.push_str("== Room Parameters\n\n");
                                typ.push_str("#table(\n  columns: 6,\n  align: center,\n");
                                typ.push_str(&format!(
                                    "  [Length], [*{:.1} m*], [Width], [*{:.1} m*], [Height], [*{:.1} m*],\n",
                                    room.length, room.width, room.height
                                ));
                                typ.push_str(&format!(
                                    "  [Workplane], [*{:.2} m*], [Suspension], [*{:.2} m*], [Area], [*{:.1} m#super[2]*],\n)\n\n",
                                    room.workplane_height, room.suspension_length, room.area()
                                ));

                                typ.push_str("#table(\n  columns: 6,\n  align: center,\n");
                                typ.push_str(&format!(
                                    "  [Ceiling], [*{:.0}%*], [Wall], [*{:.0}%*], [Floor], [*{:.0}%*],\n",
                                    rho_ceiling.get() * 100.0, rho_wall.get() * 100.0, rho_floor.get() * 100.0
                                ));
                                typ.push_str(&format!(
                                    "  [LLD], [*{:.0}%*], [LDD], [*{:.0}%*], [Total LLF], [*{:.3}*],\n)\n\n",
                                    lld.get() * 100.0, ldd.get() * 100.0, r.llf_total
                                ));

                                typ.push_str("== Cavity Analysis\n\n");
                                typ.push_str("#table(\n  columns: 6,\n  align: center,\n");
                                typ.push_str(&format!(
                                    "  [RCR], [*{:.2}*], [CCR], [*{:.2}*], [FCR], [*{:.2}*],\n",
                                    r.cavity.rcr, r.cavity.ccr, r.cavity.fcr
                                ));
                                typ.push_str(&format!(
                                    "  [ρ#sub[cc,eff]], [*{:.0}%*], [ρ#sub[fc,eff]], [*{:.0}%*], [CU], [*{:.1}%*],\n)\n\n",
                                    r.cavity.rho_cc_eff * 100.0, r.cavity.rho_fc_eff * 100.0, r.cu * 100.0
                                ));

                                typ.push_str("== Results\n\n");
                                typ.push_str("#table(\n  columns: 6,\n  align: center,\n");
                                typ.push_str(&format!(
                                    "  [Luminaires], [*{} ({}×{})*], [Target], [*{:.0} lux*], [Achieved], [*{:.0} lux*],\n",
                                    r.layout.count, r.layout.rows, r.layout.cols, r.target_illuminance, r.achieved_illuminance
                                ));
                                let spacing_status = if r.layout.spacing_ok { "OK" } else { "EXCEEDS" };
                                typ.push_str(&format!(
                                    "  [LPD], [*{:.1} W/m#super[2]*], [S/MH], [*{:.2} × {:.2}*], [Spacing], [*{}*],\n)\n\n",
                                    r.lpd, r.layout.s_mh_x, r.layout.s_mh_y, spacing_status
                                ));

                                typ.push_str("== Room Plan\n\n");
                                typ.push_str(&format!(
                                    "#align(center)[#image(bytes(\"{}\"), width: 90%)]\n\n",
                                    escape(&plan_svg)
                                ));

                                typ.push_str("== Cavity Section\n\n");
                                typ.push_str(&format!(
                                    "#align(center)[#image(bytes(\"{}\"), width: 80%)]\n\n",
                                    escape(&section_svg)
                                ));

                                if let Some(ppb) = &r.ppb {
                                    let heatmap_svg = ZonalSvg::illuminance_view(ppb, &room, &theme, u);
                                    typ.push_str("#pagebreak()\n\n");
                                    typ.push_str("== Illuminance Heatmap (Point-by-Point)\n\n");
                                    typ.push_str("#table(\n  columns: 6,\n  align: center,\n");
                                    typ.push_str(&format!(
                                        "  [Min], [*{:.0} lux*], [Avg], [*{:.0} lux*], [Max], [*{:.0} lux*],\n",
                                        ppb.min_lux, ppb.avg_lux, ppb.max_lux
                                    ));
                                    typ.push_str(&format!(
                                        "  [U#sub[0] (min/avg)], [*{:.3}*], [U#sub[d] (min/max)], [*{:.3}*], [], [],\n)\n\n",
                                        ppb.uniformity_min_avg, ppb.uniformity_min_max
                                    ));
                                    typ.push_str(&format!(
                                        "#align(center)[#image(bytes(\"{}\"), width: 95%)]\n\n",
                                        escape(&heatmap_svg)
                                    ));
                                }

                                typ.push_str("#pagebreak()\n\n");
                                typ.push_str("== Coefficient of Utilization Table\n\n");
                                typ.push_str(&format!(
                                    "#align(center)[#image(bytes(\"{}\"), width: 100%)]\n\n",
                                    escape(&cu_svg)
                                ));

                                set_pdf_exporting.set(true);
                                wasm_bindgen_futures::spawn_local(async move {
                                    match compile_typst_to_pdf(&typ).await {
                                        Ok(pdf_bytes) => {
                                            super::file_handler::download_bytes(
                                                "zonal_design_report.pdf", &pdf_bytes, "application/pdf",
                                            );
                                        }
                                        Err(e) => {
                                            web_sys::console::error_1(
                                                &format!("PDF export failed: {e}").into(),
                                            );
                                            web_sys::window().unwrap()
                                                .alert_with_message(
                                                    &format!("PDF export failed: {e}. Downloading .typ file instead."),
                                                ).ok();
                                            super::file_handler::download_file(
                                                "zonal_design_report.typ", &typ, "text/plain",
                                            );
                                        }
                                    }
                                    set_pdf_exporting.set(false);
                                });
                            }
                        >{move || if pdf_exporting.get() { locale.get().designer.exporting_pdf.clone() } else { locale.get().zonal_designer.export_pdf.clone() }}</button>
                    </div>
                </div>

                // ─── Views ────────────────────────────────────────────
                <div class="zonal-views">
                    // Results summary + plan view
                    <div class="zonal-panel">
                        <h3>{move || locale.get().designer.plan_view.clone()}</h3>
                        // Room plan SVG
                        <div class="zonal-plan-svg" inner_html=move || {
                            let r = result.get();
                            let room = Room::new(
                                room_length.get(), room_width.get(), room_height.get(),
                                workplane_height.get(), suspension_length.get(),
                            );
                            ZonalSvg::room_plan(&room, &r.layout, &SvgTheme::light(), units.get())
                        }></div>
                        // Results summary table
                        <table class="zonal-stats-table">
                            <tr>
                                <td>{move || locale.get().zonal_designer.results.luminaires.clone()}</td>
                                <td class="val">{move || {
                                    let r = result.get();
                                    format!("{} ({}×{})", r.layout.count, r.layout.rows, r.layout.cols)
                                }}</td>
                                <td>{move || locale.get().zonal_designer.results.achieved.clone()}</td>
                                <td class="val">{move || format!("{:.0} lux", result.get().achieved_illuminance)}</td>
                                <td>{move || locale.get().zonal_designer.results.cu.clone()}</td>
                                <td class="val">{move || format!("{:.1}%", result.get().cu * 100.0)}</td>
                            </tr>
                            <tr>
                                <td>{move || locale.get().zonal_designer.results.rcr.clone()}</td>
                                <td class="val">{move || format!("{:.2}", result.get().cavity.rcr)}</td>
                                <td>{move || locale.get().zonal_designer.results.lpd.clone()}</td>
                                <td class="val">{move || format!("{:.1} W/m\u{00B2}", result.get().lpd)}</td>
                                <td>{move || locale.get().zonal_designer.results.spacing.clone()}</td>
                                <td class=move || if result.get().layout.spacing_ok { "val zonal-spacing-ok" } else { "val zonal-spacing-warn" }>
                                    {move || {
                                        let r = result.get();
                                        let icon = if r.layout.spacing_ok { "\u{2713}" } else { "\u{26A0}" };
                                        format!("{icon} {:.2}×{:.2}", r.layout.s_mh_x, r.layout.s_mh_y)
                                    }}
                                </td>
                            </tr>
                        </table>
                        // PPB stats row
                        {move || result.get().ppb.as_ref().map(|ppb| view! {
                            <table class="zonal-stats-table">
                                <tr>
                                    <td>{move || locale.get().designer.min.clone()}</td>
                                    <td class="val">{format!("{:.0} lux", ppb.min_lux)}</td>
                                    <td>{move || locale.get().designer.avg.clone()}</td>
                                    <td class="val">{format!("{:.0} lux", ppb.avg_lux)}</td>
                                    <td>{move || locale.get().designer.max.clone()}</td>
                                    <td class="val">{format!("{:.0} lux", ppb.max_lux)}</td>
                                </tr>
                                <tr>
                                    <td>{"\u{0055}\u{2080} (min/avg)"}</td>
                                    <td class="val">{format!("{:.3}", ppb.uniformity_min_avg)}</td>
                                    <td>{"\u{0055}\u{0064} (min/max)"}</td>
                                    <td class="val">{format!("{:.3}", ppb.uniformity_min_max)}</td>
                                    <td></td><td></td>
                                </tr>
                            </table>
                        })}
                    </div>

                    // Tab-driven view panel
                    {move || {
                        let tab = view_tab.get();
                        let r = result.get();
                        let room = Room::new(
                            room_length.get(), room_width.get(), room_height.get(),
                            workplane_height.get(), suspension_length.get(),
                        );

                        match tab.as_str() {
                            "heatmap" => {
                                if let Some(ppb) = r.ppb.as_ref() {
                                    let ppb = ppb.clone();
                                    let room = room.clone();
                                    view! {
                                        <div class="zonal-panel">
                                            <div style="display: flex; align-items: center; gap: 12px;">
                                                <h3 style="margin: 0;">{move || locale.get().zonal_designer.heatmap_title.clone()}</h3>
                                                <label style="font-size: 0.8rem; display: flex; align-items: center; gap: 4px; cursor: pointer; user-select: none;">
                                                    <input type="checkbox"
                                                        prop:checked=show_heatmap_values
                                                        on:change=move |ev| {
                                                            use wasm_bindgen::JsCast;
                                                            let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
                                                            set_show_heatmap_values.set(input.checked());
                                                        }
                                                    />
                                                    "Show values"
                                                </label>
                                            </div>
                                            <div class="zonal-iso-svg" inner_html=move || {
                                                ZonalSvg::illuminance_view_opts(&ppb, &room, &SvgTheme::light(), units.get(), show_heatmap_values.get())
                                            }></div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div class="zonal-panel"><p>{move || locale.get().zonal_designer.computing_heatmap.clone()}</p></div> }.into_any()
                                }
                            }
                            "table" => {
                                if let Some(ppb) = r.ppb.as_ref() {
                                    let svg = ZonalSvg::illuminance_table(ppb, &room, &SvgTheme::light(), units.get());
                                    view! {
                                        <div class="zonal-panel">
                                            <h3>"Illuminance Schedule"</h3>
                                            <div class="zonal-iso-svg" inner_html=svg></div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div class="zonal-panel"><p>"Computing..."</p></div> }.into_any()
                                }
                            }
                            "section" => {
                                let svg = ZonalSvg::section_view(&room, &r.cavity, &SvgTheme::light(), units.get());
                                view! {
                                    <div class="zonal-panel">
                                        <h3>{move || locale.get().zonal_designer.cavity_section.clone()}</h3>
                                        <div class="zonal-iso-svg" inner_html=svg></div>
                                    </div>
                                }.into_any()
                            }
                            "cu" => {
                                let ct = cu_table.get();
                                let svg = ZonalSvg::cu_table_svg(
                                    &ct, r.cavity.rcr, r.cavity.rho_cc_eff,
                                    rho_wall.get(), &SvgTheme::light(),
                                );
                                view! {
                                    <div class="zonal-panel">
                                        <h3>{move || locale.get().zonal_designer.cu_table_title.clone()}</h3>
                                        <div class="zonal-cu-table-scroll" inner_html=svg></div>
                                    </div>
                                }.into_any()
                            }
                            _ => {
                                // "3d" (default) — interactive
                                let reflectances = Reflectances::new(rho_ceiling.get(), rho_wall.get(), rho_floor.get());
                                let cavity = compute_cavity_ratios(&room, &reflectances);

                                let faces = build_interior_scene(
                                    &room, &r.layout, &reflectances, &cavity,
                                    r.ppb.as_ref(), show_cavities_3d.get(), show_light_cones.get(),
                                );

                                let svg_w = 600.0;
                                let svg_h = 450.0;
                                let az = cam_azimuth.get();
                                let el = cam_elevation.get();
                                let zoom = cam_zoom.get();
                                let cam = eulumdat::scene3d::SceneCamera {
                                    azimuth: az,
                                    elevation: el,
                                    scale: 1.0,
                                    center: (svg_w / 2.0, svg_h / 2.0),
                                };
                                let base_scale = fit_scale(room.length, room.width, room.height, svg_w, svg_h, &cam);
                                let cam = eulumdat::scene3d::SceneCamera {
                                    scale: base_scale * zoom,
                                    ..cam
                                };

                                let scene_svg = render_scene_svg(&faces, &cam, svg_w, svg_h, "#f8f9fa");
                                view! {
                                    <div class="zonal-panel">
                                        <h3>{move || locale.get().zonal_designer.room_view_3d.clone()}
                                            <span style="font-size:10px; font-weight:normal; opacity:0.5; margin-left:8px;">
                                                {move || locale.get().zonal_designer.room_view_hint.clone()}
                                            </span>
                                        </h3>
                                        <div class="zonal-3d-interactive"
                                            inner_html=scene_svg.clone()
                                            style="cursor:grab; user-select:none;"
                                            on:mousedown=move |ev| {
                                                ev.prevent_default();
                                                set_dragging_3d.set(true);
                                                set_drag_start.set((ev.client_x() as f64, ev.client_y() as f64));
                                                set_drag_az_start.set(cam_azimuth.get());
                                                set_drag_el_start.set(cam_elevation.get());
                                            }
                                            on:mousemove=move |ev| {
                                                if !dragging_3d.get() { return; }
                                                ev.prevent_default();
                                                let (sx, sy) = drag_start.get();
                                                let dx = ev.client_x() as f64 - sx;
                                                let dy = ev.client_y() as f64 - sy;
                                                set_cam_azimuth.set(drag_az_start.get() - dx * 0.5);
                                                set_cam_elevation.set((drag_el_start.get() + dy * 0.5).clamp(0.0, 89.0));
                                            }
                                            on:mouseup=move |_| set_dragging_3d.set(false)
                                            on:mouseleave=move |_| set_dragging_3d.set(false)
                                            on:wheel=move |ev: web_sys::WheelEvent| {
                                                ev.prevent_default();
                                                let delta = ev.delta_y();
                                                let factor = if delta > 0.0 { 0.9 } else { 1.1 };
                                                set_cam_zoom.set((cam_zoom.get() * factor).clamp(0.3, 5.0));
                                            }
                                        ></div>
                                        <div class="zonal-export-buttons">
                                            <button class="zonal-export-btn-sm"
                                                on:click=move |_| {
                                                    let svg = scene_svg.clone();
                                                    super::file_handler::download_svg("zonal_3d_room.svg", &svg);
                                                }
                                            >{move || locale.get().zonal_designer.export_3d_svg.clone()}</button>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    }}
                </div>
            </div>
        </div>

        // ─── Inline CSS ─────────────────────────────────────────────
        <style>
        {r#"
.zonal-designer {
    display: flex;
    flex-direction: column;
    gap: 0;
    min-height: 500px;
}
/* ── Toolbar ────────────────────────────────────── */
.zonal-toolbar {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-elevated);
}
.zonal-toolbar-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 12px;
}
.zonal-toolbar-label {
    font-size: 11px;
    opacity: 0.7;
    white-space: nowrap;
}
.zonal-toolbar-range {
    width: 80px;
    flex-shrink: 0;
}
.zonal-toolbar-val {
    font-size: 11px;
    min-width: 36px;
    font-variant-numeric: tabular-nums;
}
.zonal-toolbar-select {
    font-size: 11px;
    padding: 2px 4px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--surface);
    color: var(--text-primary);
}
.zonal-toolbar-num {
    width: 60px;
    font-size: 11px;
    padding: 2px 4px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--surface);
    color: var(--text-primary);
}
.zonal-toolbar-toggle {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
}
.zonal-view-tabs {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
    margin-left: 4px;
}
.zonal-view-tab {
    padding: 2px 10px;
    font-size: 11px;
    border: none;
    border-right: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
}
.zonal-view-tab:last-child { border-right: none; }
.zonal-view-tab:hover { background: var(--surface-elevated); }
.zonal-view-tab.active {
    background: var(--primary-color);
    color: #fff;
    font-weight: 600;
}
/* ── Layout ─────────────────────────────────────── */
.zonal-layout {
    display: flex;
    gap: 0;
    flex: 1;
    min-height: 0;
}
/* ── Sidebar ────────────────────────────────────── */
.zonal-sidebar {
    width: 220px;
    min-width: 200px;
    flex-shrink: 0;
    overflow-y: auto;
    max-height: 75vh;
    padding: 6px;
    border-right: 1px solid var(--border);
}
.zonal-sidebar-section {
    margin-bottom: 10px;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: 5px;
}
.zonal-sidebar-section h4 {
    margin: 0 0 4px 0;
    font-size: 10px;
    text-transform: uppercase;
    opacity: 0.6;
}
.zonal-prop-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 3px;
    font-size: 11px;
}
.zonal-prop-row select {
    flex: 1;
    font-size: 11px;
    padding: 2px 4px;
    border: 1px solid var(--border);
    border-radius: 3px;
}
.zonal-prop-row input[type="range"] {
    flex: 1;
}
.zonal-prop-label {
    min-width: 72px;
    flex-shrink: 0;
    font-size: 11px;
}
.zonal-info-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 1px 6px;
    font-size: 11px;
}
.zonal-info-grid .label {
    opacity: 0.6;
}
.zonal-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    margin-bottom: 3px;
    cursor: pointer;
}
/* ── Views ──────────────────────────────────────── */
.zonal-views {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    max-height: 80vh;
    padding: 8px;
}
.zonal-panel {
    margin-bottom: 12px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
}
.zonal-panel h3 {
    margin: 0 0 6px 0;
    font-size: 12px;
    text-transform: uppercase;
    opacity: 0.6;
}
.zonal-plan-svg svg, .zonal-iso-svg svg {
    width: 100%;
    height: auto;
    max-height: 420px;
}
.zonal-cu-table-scroll {
    overflow-x: auto;
}
.zonal-cu-table-scroll svg {
    min-width: 600px;
}
/* ── Stats table ────────────────────────────────── */
.zonal-stats-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
    margin-top: 6px;
}
.zonal-stats-table td {
    padding: 2px 6px;
    border: 1px solid var(--border);
}
.zonal-stats-table .val {
    font-weight: bold;
    text-align: center;
}
.zonal-spacing-ok {
    color: #22c55e;
}
.zonal-spacing-warn {
    color: #ef4444;
}
/* ── Export buttons ──────────────────────────────── */
.zonal-export-btn {
    width: 100%;
    padding: 6px 12px;
    font-size: 11px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--primary-color);
    color: white;
    cursor: pointer;
}
.zonal-export-btn:hover:not(:disabled) { opacity: 0.9; }
.zonal-export-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.zonal-export-buttons {
    margin-top: 6px;
    display: flex;
    gap: 6px;
}
.zonal-export-btn-sm {
    padding: 3px 10px;
    font-size: 11px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--surface);
    color: var(--text-primary);
    cursor: pointer;
}
.zonal-export-btn-sm:hover { background: var(--surface-elevated); }
/* ── Share button ──────────────────────────────── */
.zonal-share-btn {
    width: 100%;
    padding: 6px 12px;
    font-size: 11px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface);
    color: var(--text-primary);
    cursor: pointer;
    margin-bottom: 6px;
    transition: background 0.15s;
}
.zonal-share-btn:hover { background: var(--surface-elevated); }
/* ── Zonal flux summary ────────────────────────── */
.zonal-flux-summary {
    margin-top: 6px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
}
.zonal-flux-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 10px;
    margin: 3px 0 1px 0;
}
.zonal-flux-label {
    font-weight: bold;
    opacity: 0.7;
}
.zonal-flux-val {
    font-weight: bold;
    font-variant-numeric: tabular-nums;
}
.zonal-flux-bar-row {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 9px;
    margin-bottom: 1px;
}
.zonal-flux-zone {
    min-width: 42px;
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
}
.zonal-flux-bar {
    flex: 1;
    height: 8px;
    background: var(--border);
    border-radius: 4px;
    overflow: hidden;
}
.zonal-flux-bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.2s;
}
.zonal-flux-down { background: #3b82f6; }
.zonal-flux-up { background: #f59e0b; }
.zonal-flux-pct {
    min-width: 32px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
}
        "#}
        </style>
    }
}
