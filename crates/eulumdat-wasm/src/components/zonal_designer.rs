//! Zonal Cavity Interior Lighting Designer — Leptos WASM component.

use eulumdat::diagram::SvgTheme;
use eulumdat::scene3d::{build_interior_scene, fit_scale, render_scene_svg, CameraPreset};
use eulumdat::zonal::{
    compute_cavity_ratios, compute_ppb_overlay, compute_zonal, LightLossFactor, LlfPreset,
    PlanFrame, ReflectancePreset, Reflectances, Room, RoomPreset, SolveMode, ZonalSvg,
};
use eulumdat::CuTable;
use eulumdat::Eulumdat;
use eulumdat::PhotometricCalculations;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::i18n::use_locale;
use eulumdat::sensor::coverage::CoverageRoom;

use super::plan_drag::{self, MoveWhat, Moves};
use super::zonal_emergency::{
    self, EmTool, EmergencyResults, EmergencySidebar, EmergencyState, PointDraft,
};
use super::zonal_sensors::{
    self, fill, motion_for_preset, space_for_preset, CoreSensor, SensorPlanState, SensorResults,
    SensorSidebar,
};

/// Key of the luminaire grid the hand moves belong to.
fn grid_key(layout: &eulumdat::zonal::LuminaireLayout) -> String {
    format!("{}x{}:{}", layout.rows, layout.cols, layout.count)
}

/// Room outline from the link (`u,v;u,v;…`, corners relative to the
/// room's bounds 0..1).
fn parse_shape(s: &str) -> Option<Vec<(f64, f64)>> {
    let v: Vec<(f64, f64)> = s
        .split(';')
        .filter_map(|p| {
            let (a, b) = p.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .filter(|(u, v): &(f64, f64)| u.is_finite() && v.is_finite())
        .collect();
    (v.len() >= 3).then_some(v)
}

fn encode_shape(v: &[(f64, f64)]) -> String {
    v.iter()
        .map(|(u, w)| format!("{u:.3},{w:.3}"))
        .collect::<Vec<_>>()
        .join(";")
}

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

/// The emergency-lighting part of a PDF (Typst): the plan with routes and
/// luminaires, the floor in emergency mode and the rows of the chosen
/// standard. Starts with a page break.
fn emergency_typst(
    er: &zonal_emergency::EmResult,
    routes: &[zonal_emergency::RouteDraft],
    room: &Room,
    layout: &eulumdat::zonal::LuminaireLayout,
    theme: &SvgTheme,
    u: eulumdat::UnitSystem,
    floor: &Room,
) -> String {
    let svg = ZonalSvg::room_plan_with_overlay(room, layout, theme, u, &|ox, oy, s| {
        zonal_emergency::overlay(Some(er), routes, &[], ox, oy, s, false)
    });
    // the floor in emergency mode, normal lighting off
    let g = zonal_emergency::floor(er, &room_polygon(floor));
    let floor_svg = ZonalSvg::room_plan_styled(
        room,
        layout,
        theme,
        u,
        &|ox, oy, s| {
            let mut o = zonal_emergency::floor_svg(
                &g,
                &zonal_emergency::floor_bands(&er.limits),
                ox,
                oy,
                s,
            );
            o.push_str(&zonal_emergency::overlay(
                Some(er),
                routes,
                &[],
                ox,
                oy,
                s,
                false,
            ));
            o
        },
        None,
        er.source.which != zonal_emergency::EmLuminaire::Room,
    );
    zonal_emergency::typst_section(er, &svg, Some((&floor_svg, &g)))
}

/// The room's floor outline (the drawn shape, else the rectangle).
fn room_polygon(room: &Room) -> Vec<(f64, f64)> {
    room.outline.clone().unwrap_or_else(|| {
        vec![
            (0.0, 0.0),
            (room.length, 0.0),
            (room.length, room.width),
            (0.0, room.width),
        ]
    })
}

/// Zonal Cavity Interior Lighting Designer component.
#[component]
pub fn ZonalDesigner(ldc: ReadSignal<Eulumdat>) -> impl IntoView {
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
    // Presence sensors (share-link keys s*).
    let sensors = SensorPlanState::from_link(url_get, init_room_preset);
    // Emergency lighting, EN 1838 (share-link keys em*).
    let emergency = EmergencyState::from_link(url_get);
    // Luminaires moved by hand on the plan (link key `lmove`), for the grid
    // they were moved in ("3x3").
    let lum_moves = RwSignal::new(
        url_get("lmove")
            .map(|v| Moves::decode(&v))
            .unwrap_or_default(),
    );
    // What a drag on the plan moves, and what is being dragged.
    let move_what = RwSignal::new(MoveWhat::Luminaires);
    let dragging = RwSignal::new(None::<(MoveWhat, usize)>);
    // Room outline (link key `shape`): corners relative to the room's
    // bounds (0..1), so the length / width sliders stretch it. Drawn by
    // clicking the corners on the plan.
    let room_shape = RwSignal::new(url_get("shape").and_then(|v| parse_shape(&v)));
    let drawing_shape = RwSignal::new(false);
    let shape_wip = RwSignal::new(Vec::<(f64, f64)>::new());
    let shape_m = move || {
        room_shape.get().map(|n| {
            let (l, w) = (room_length.get(), room_width.get());
            n.iter().map(|&(u, v)| (u * l, v * w)).collect::<Vec<_>>()
        })
    };
    let shape_m_untracked = move || {
        room_shape.get_untracked().map(|n| {
            let (l, w) = (room_length.get_untracked(), room_width.get_untracked());
            n.iter().map(|&(u, v)| (u * l, v * w)).collect::<Vec<_>>()
        })
    };

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
        let mut hash = match sensors.link_query() {
            Some(q) => format!("{hash}&{q}"),
            None => hash,
        };
        if let Some(q) = emergency.link_query() {
            hash.push_str(&format!("&{q}"));
        }
        if let Some(m) = lum_moves.with(|m| m.encode()) {
            hash.push_str(&format!("&lmove={m}"));
        }
        if let Some(sh) = room_shape.get() {
            hash.push_str(&format!("&shape={}", encode_shape(&sh)));
        }
        set_interior_url_hash(&hash);
    });

    // ─── Apply room preset ──────────────────────────────────────────────
    let apply_room_preset = move |preset: RoomPreset| {
        set_room_preset.set(preset);
        room_shape.set(None);
        let room = preset.to_room();
        set_room_length.set(room.length);
        set_room_width.set(room.width);
        set_room_height.set(room.height);
        set_workplane_height.set(room.workplane_height);
        set_suspension_length.set(room.suspension_length);
        set_target_illuminance.set(preset.target_lux());
        sensors.space.set(space_for_preset(preset));
        sensors.motion.set(motion_for_preset(preset));
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
    let cu_table = Memo::new(move |_| CuTable::calculate(&ldc.get()));

    // ─── Main computation (reactive) ────────────────────────────────────
    let result = Memo::new(move |_| {
        let room = Room::new(
            room_length.get(),
            room_width.get(),
            room_height.get(),
            workplane_height.get(),
            suspension_length.get(),
        )
        .with_outline(shape_m());
        let reflectances = Reflectances::new(rho_ceiling.get(), rho_wall.get(), rho_floor.get());
        let llf = LightLossFactor::new(lld.get(), ldd.get(), bf.get(), rsdd.get());
        let ct = cu_table.get();

        let mut zr = compute_zonal(
            &ldc.get(),
            &room,
            &reflectances,
            &llf,
            target_illuminance.get(),
            &ct,
            solve_mode.get(),
            Some(fixed_count.get()),
            Some(target_lpd.get()),
        );
        // Luminaires moved by hand (only for the grid they were moved in).
        let key = grid_key(&zr.layout);
        zr.layout.moved = lum_moves.with(|m| {
            m.for_key(&key)
                .iter()
                .map(|&(i, x, y)| (i, x.clamp(0.0, room.length), y.clamp(0.0, room.width)))
                .collect()
        });

        // PPB overlay (for heatmap view or 3D scene lighting)
        if (show_ppb.get() || show_3d.get()) && zr.layout.count > 0 {
            let ppb = compute_ppb_overlay(
                &ldc.get(),
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
        let l = ldc.get();
        // Primary lamp set; 26c/26f are already totals for all lamps of the set
        let flux = l.luminous_flux();
        let watts = l.wattage();
        let name = l.luminaire_name.clone();
        let dff = l.downward_flux_fraction;
        (name, flux, watts, dff)
    });

    // ─── Presence sensors: layout, coverage, EN 15193-1 ─────────────────
    let sensor_core = Memo::new(move |_| {
        zonal_sensors::chosen_data(&sensors)
            .map(|(name, d)| (name, CoreSensor(std::sync::Arc::new(d.to_core()))))
    });
    let sensor_cut = Memo::new(move |_| {
        let (name, core) = sensor_core.get()?;
        let cut = zonal_sensors::cut(&sensors, &core, room_height.get(), workplane_height.get());
        Some((name, core, cut))
    });
    let sensor_room = Memo::new(move |_| CoverageRoom {
        length: room_length.get(),
        width: room_width.get(),
        mount_height: room_height.get(),
        walk_plane: 0.0,
        presence_plane: workplane_height.get(),
        margin: sensors.margin.get(),
    });
    // Placement apart from the hand moves: dragging a sensor does not
    // rerun the automatic placement.
    // the floor of a shaped room (always true for a rectangle)
    let floor = Memo::new(move |_| {
        Room::new(room_length.get(), room_width.get(), 3.0, 0.8, 0.0).with_outline(shape_m())
    });
    let sensor_placed = Memo::new(move |_| {
        let (_, _, cut) = sensor_cut.get()?;
        let f = floor.get();
        Some(zonal_sensors::place(
            &sensors,
            &cut.0,
            &sensor_room.get(),
            &|x, y| f.contains(x, y),
            || result.with(|r| r.layout.positions()),
        ))
    });
    let sensor_plan = Memo::new(move |_| {
        let (name, core, cut) = sensor_cut.get()?;
        let placed = sensor_placed.get()?;
        let f = floor.get();
        Some(zonal_sensors::finish(
            &sensors,
            &placed,
            &core,
            &cut.0,
            &sensor_room.get(),
            &|x, y| f.contains(x, y),
            name,
        ))
    });
    // Emergency lighting: the luminaire and its spacing (the search runs
    // only when they change), then the check of routes / areas / points.
    let em_source =
        Memo::new(move |_| zonal_emergency::source(&emergency, &ldc.get(), room_height.get()));
    let em_result = Memo::new(move |_| {
        let src = em_source.get()?;
        let f = floor.get();
        let poly = room_polygon(&f);
        let positions = result.with(|r| r.layout.positions());
        Some(zonal_emergency::compute(
            &emergency, &src, &poly, &positions,
        ))
    });
    // The floor in emergency mode (only while the plan shows it)
    let em_floor = Memo::new(move |_| {
        if !emergency.show.get() {
            return None;
        }
        let poly = room_polygon(&floor.get());
        em_result.with(|r| r.as_ref().map(|r| zonal_emergency::floor(r, &poly)))
    });
    let em_points = move || -> Vec<(f64, f64)> {
        em_result.with_untracked(|r| {
            r.as_ref()
                .filter(|r| r.source.which != zonal_emergency::EmLuminaire::Room)
                .map(|r| r.positions.iter().map(|p| (p.0, p.1)).collect())
                .unwrap_or_default()
        })
    };
    // Moves made for another layout (another count or placement) are not
    // applied; they are kept, so a link opened before its luminaire has
    // loaded (another grid at first) still shows them.

    // ─── Dragging on the plan ───────────────────────────────────────────
    let plan_ref = NodeRef::<leptos::html::Div>::new();
    // Room point under the pointer, and pixels per metre of the drawing.
    let pointer_in_room = move |ev: &web_sys::PointerEvent| -> Option<(f64, f64, f64)> {
        let el = plan_ref.get_untracked()?;
        let room = Room::new(
            room_length.get_untracked(),
            room_width.get_untracked(),
            room_height.get_untracked(),
            workplane_height.get_untracked(),
            suspension_length.get_untracked(),
        );
        let f = PlanFrame::for_room(&room);
        let (vx, vy) = plan_drag::viewbox_point(
            &el,
            ev.client_x() as f64,
            ev.client_y() as f64,
            f.width,
            f.height,
        )?;
        let (x, y) = f.to_room(vx, vy);
        Some((x, y, f.scale))
    };
    let sensor_points = move || -> Vec<(f64, f64)> {
        sensor_plan.with_untracked(|p| {
            p.as_ref()
                .map(|p| p.cov.positions.iter().map(|s| (s.x, s.y)).collect())
                .unwrap_or_default()
        })
    };
    let on_plan_pointerdown = move |ev: web_sys::PointerEvent| {
        let Some((x, y, scale)) = pointer_in_room(&ev) else {
            return;
        };
        // Emergency lighting: drawing a route / placing safety equipment
        // (clicks off the room are ignored, near it they land on the wall)
        let tool = emergency.tool.get_untracked();
        if tool != EmTool::Off {
            let (l, w) = (room_length.get_untracked(), room_width.get_untracked());
            if x < -0.3 || y < -0.3 || x > l + 0.3 || y > w + 0.3 {
                return;
            }
        }
        let (x, y) = (
            x.clamp(0.0, room_length.get_untracked()),
            y.clamp(0.0, room_width.get_untracked()),
        );
        match tool {
            EmTool::Route => {
                ev.prevent_default();
                let (l, w) = (room_length.get_untracked(), room_width.get_untracked());
                let p = (
                    (x.clamp(0.0, l) * 10.0).round() / 10.0,
                    (y.clamp(0.0, w) * 10.0).round() / 10.0,
                );
                // a double click lands twice on the same point
                let last = emergency.wip.with_untracked(|v| v.last().copied());
                if last.is_none_or(|q| (q.0 - p.0).hypot(q.1 - p.1) > 0.05) {
                    emergency.wip.update(|v| v.push(p));
                }
                return;
            }
            EmTool::Point(kind) => {
                ev.prevent_default();
                let poly = room_polygon(&floor.get_untracked());
                let facing = zonal_emergency::facing_from_walls(&poly, x, y);
                emergency.points.update(|v| {
                    v.push(PointDraft {
                        kind,
                        x: (x * 100.0).round() / 100.0,
                        y: (y * 100.0).round() / 100.0,
                        facing,
                    })
                });
                return;
            }
            EmTool::Off => {}
        }
        // Drawing the room outline: each click adds a corner (10 cm grid,
        // straight to the previous corner and to the bounds when close);
        // the first corner closes it.
        if drawing_shape.get_untracked() {
            ev.prevent_default();
            let (l, w) = (room_length.get_untracked(), room_width.get_untracked());
            let snap = |v: f64, max: f64| {
                let v = (v.clamp(0.0, max) * 10.0).round() / 10.0;
                if v < 0.3 {
                    0.0
                } else if max - v < 0.3 {
                    max
                } else {
                    v
                }
            };
            let (mut px, mut py) = (snap(x, l), snap(y, w));
            let wip = shape_wip.get_untracked();
            if let Some(&(lx, ly)) = wip.last() {
                if (px - lx).abs() < 0.3 {
                    px = lx;
                }
                if (py - ly).abs() < 0.3 {
                    py = ly;
                }
            }
            if wip.len() >= 3 {
                let (fx, fy) = wip[0];
                if (x - fx).hypot(y - fy) < (14.0 / scale.max(1e-6)).max(0.3) {
                    // close: the outline's own bounds become the room
                    let (x0, x1) = wip
                        .iter()
                        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
                    let (y0, y1) = wip
                        .iter()
                        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
                    let (bw, bd) = ((x1 - x0).max(0.5), (y1 - y0).max(0.5));
                    let norm: Vec<(f64, f64)> = wip
                        .iter()
                        .map(|&(px, py)| ((px - x0) / bw, (py - y0) / bd))
                        .collect();
                    set_room_length.set(bw);
                    set_room_width.set(bd);
                    room_shape.set(Some(norm));
                    shape_wip.set(Vec::new());
                    drawing_shape.set(false);
                    return;
                }
            }
            shape_wip.update(|v| v.push((px, py)));
            return;
        }
        // within 14 px of the drawing
        let reach = 14.0 / scale.max(1e-6);
        let what = match move_what.get_untracked() {
            MoveWhat::Sensors if sensor_plan.with_untracked(|p| p.is_some()) => MoveWhat::Sensors,
            MoveWhat::Emergency if !em_points().is_empty() => MoveWhat::Emergency,
            _ => MoveWhat::Luminaires,
        };
        let hit = match what {
            MoveWhat::Luminaires => plan_drag::nearest(
                &result.with_untracked(|r| r.layout.positions()),
                (x, y),
                reach,
            ),
            MoveWhat::Sensors => plan_drag::nearest(&sensor_points(), (x, y), reach),
            MoveWhat::Emergency => plan_drag::nearest(&em_points(), (x, y), reach),
        };
        if let Some(i) = hit {
            dragging.set(Some((what, i)));
            if let Some(el) = ev
                .current_target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            {
                let _ = el.set_pointer_capture(ev.pointer_id());
            }
            ev.prevent_default();
        }
    };
    let on_plan_pointermove = move |ev: web_sys::PointerEvent| {
        let Some((what, i)) = dragging.get_untracked() else {
            return;
        };
        let Some((x, y, _)) = pointer_in_room(&ev) else {
            return;
        };
        let x = x.clamp(0.0, room_length.get_untracked());
        let y = y.clamp(0.0, room_width.get_untracked());
        match what {
            MoveWhat::Luminaires => {
                let key = result.with_untracked(|r| grid_key(&r.layout));
                lum_moves.update(|m| m.set(&key, i, x, y));
            }
            MoveWhat::Sensors => {
                if let Some(key) =
                    sensor_plan.with_untracked(|p| p.as_ref().map(|p| p.move_key.clone()))
                {
                    sensors.moves.update(|m| m.set(&key, i, x, y));
                }
            }
            MoveWhat::Emergency => {
                if let Some(key) =
                    em_result.with_untracked(|r| r.as_ref().map(|r| r.move_key.clone()))
                {
                    emergency.moves.update(|m| m.set(&key, i, x, y));
                }
            }
        }
    };
    let on_plan_pointerup = move |_: web_sys::PointerEvent| dragging.set(None);
    // How many luminaires / sensors are off their computed places.
    let moved_count = Memo::new(move |_| {
        result.with(|r| {
            r.layout
                .moved
                .iter()
                .filter(|e| e.0 < r.layout.count)
                .count()
        }) + sensor_plan.with(|p| p.as_ref().map(|p| p.moved).unwrap_or(0))
            + em_result.with(|r| r.as_ref().map(|r| r.moved).unwrap_or(0))
    });
    let sensor_energy = Memo::new(move |_| {
        let power = luminaire_info.with(|i| i.2) * result.with(|r| r.layout.count as f64);
        sensor_plan.with(|p| {
            p.as_ref()
                .map(|p| zonal_sensors::compute_energy(&sensors, p, power))
        })
    });

    // ─── Open the plan in the Light scene ─────────────────────────────────
    let open_light_scene = use_context::<super::app::OpenLightScene>();
    let make_plan = move || {
        use eulumdat_scene::glam::{Vec2, Vec3};
        use eulumdat_scene::scene::{
            LampRole, PlanEmergency, PlanLuminaire, PlanPhotometry, PlanPoint, ScenePlan,
        };
        let l = room_length.get_untracked() as f32;
        let w = room_width.get_untracked() as f32;
        let h = room_height.get_untracked() as f32;
        let sus = suspension_length.get_untracked() as f32;
        // luminous face below the ceiling (clear of it when surface mounted)
        let y = (h - sus).min(h - 0.12);
        // emergency lighting: in emergency mode the normal luminaires go
        // off, the emergency ones (type 1) light; the room's own luminaires
        // as "combined" at their emergency share
        let em = em_result.get_untracked();
        let room_mode = em
            .as_ref()
            .is_some_and(|r| r.source.which == zonal_emergency::EmLuminaire::Room);
        // the scene shows the emergency light with the plan's maintenance
        // factor, like the EN 1838 check
        let mf = em
            .as_ref()
            .map_or(1.0, |r| r.report.settings.maintenance_factor);
        let normal_role = match em.as_ref() {
            Some(r) if room_mode => LampRole::Combined {
                emergency: (mf * r.source.flux / r.source.own_flux.max(1e-9)) as f32,
            },
            _ => LampRole::Normal,
        };
        let mut luminaires: Vec<PlanLuminaire> = result
            .with_untracked(|r| r.layout.positions())
            .into_iter()
            .map(|(x, z)| PlanLuminaire {
                position: Vec3::new(x as f32, y, z as f32),
                rotation_deg: 0.0,
                tilt_deg: 0.0,
                pole: None,
                pendant: sus > 0.02,
                photometry: 0,
                role: normal_role,
            })
            .collect();
        let mut photometries = Vec::new();
        if let Some(r) = em.as_ref().filter(|_| !room_mode) {
            let ldt = eulumdat::emergency::at_flux(&r.source.ldt, mf * r.source.flux);
            photometries.push(PlanPhotometry {
                name: format!(
                    "{} (emergency, {:.0} lm × MF {mf:.2})",
                    r.source.name, r.source.flux
                ),
                ldt: ldt.to_ldt(),
            });
            // emergency luminaires sit at their mounting height (surface)
            let ey = (r.source.height as f32 - 0.05).min(h - 0.05);
            luminaires.extend(r.positions.iter().map(|&(x, z, rot)| PlanLuminaire {
                position: Vec3::new(x as f32, ey, z as f32),
                rotation_deg: rot as f32,
                tilt_deg: 0.0,
                pole: None,
                pendant: false,
                photometry: 1,
                role: LampRole::Emergency,
            }));
        }
        let emergency_mode = em.is_some() && emergency.show.get_untracked();
        // what the chosen standard requires, for the check in the scene
        let em_standard = em.as_ref().map(|r| {
            use eulumdat::emergency::PlaceCategory as P;
            let l = &r.limits;
            let finite = |v: f64| if v.is_finite() { v as f32 } else { 0.0 };
            let rl = l.route_limits(P::EscapeCorridor);
            let al = l.area_limits(emergency.open_place.get_untracked());
            eulumdat_scene::scene::EmStandard {
                name: l.name.clone(),
                route_centre_lx: rl.centre_min_lx as f32,
                route_band_lx: rl.band_min_lx as f32,
                route_ratio: finite(rl.max_ratio),
                route_avg_lx: rl.avg_min_lx as f32,
                route_band_share: rl.band_share as f32,
                route_edge_max_m: if rl.edge_max_m.is_finite() {
                    rl.edge_max_m as f32
                } else {
                    0.0
                },
                ratio_over_band: rl.ratio_over_band,
                area_avg_lx: al.avg_min_lx as f32,
                stairs_lx: l.route_limits(P::StairsOrRefuge).band_min_lx as f32,
                area_lx: al.min_lx as f32,
                area_ratio: finite(al.max_ratio),
                area_border_m: al.border_m as f32,
                vertical_lx: if l.checks_emphasis() {
                    l.emphasis_min_vertical_lx as f32
                } else {
                    0.0
                },
                lux_max: crate::components::zonal_emergency::floor_bands(l)[5] as f32,
            }
        });
        let emergency = em.as_ref().map(|r| PlanEmergency {
            route_stairs: emergency
                .routes
                .get_untracked()
                .iter()
                .map(|rt| rt.place == eulumdat::emergency::PlaceCategory::StairsOrRefuge)
                .collect(),
            standard: em_standard.clone().unwrap_or_default(),
            routes: emergency
                .routes
                .get_untracked()
                .iter()
                .map(|rt| {
                    (
                        rt.path
                            .iter()
                            .map(|&(x, y)| Vec2::new(x as f32, y as f32))
                            .collect(),
                        rt.width as f32,
                    )
                })
                .collect(),
            open_area: emergency.open_area.get_untracked().then(|| {
                room_polygon(&floor.get_untracked())
                    .iter()
                    .map(|&(x, y)| Vec2::new(x as f32, y as f32))
                    .collect()
            }),
            points: emergency
                .points
                .get_untracked()
                .iter()
                .zip(r.report.points.iter())
                .map(|(p, res)| PlanPoint {
                    position: Vec2::new(p.x as f32, p.y as f32),
                    height: r.report.settings.emphasis_height_m as f32,
                    facing_deg: p.facing as f32,
                    symbol: zonal_emergency::kind_icon(p.kind).to_string(),
                    vertical: res.vertical_lx.is_some(),
                })
                .collect(),
        });
        let sensors = sensor_plan.with_untracked(|p| {
            p.as_ref()
                .map(|p| {
                    p.cov
                        .positions
                        .iter()
                        .map(|s| Vec2::new(s.x as f32, s.y as f32))
                        .collect()
                })
                .unwrap_or_default()
        });
        let name = match shape_m_untracked() {
            Some(o) => {
                let a = Room::new(l as f64, w as f64, 3.0, 0.8, 0.0)
                    .with_outline(Some(o))
                    .area();
                format!("{} {a:.1} m²", room_preset.get_untracked().label())
            }
            None => format!("{} {l:.1} × {w:.1} m", room_preset.get_untracked().label()),
        };
        ScenePlan {
            name,
            indoor: true,
            outline: match shape_m_untracked() {
                Some(o) => o
                    .iter()
                    .map(|&(x, y)| Vec2::new(x as f32, y as f32))
                    .collect(),
                None => vec![
                    Vec2::new(0.0, 0.0),
                    Vec2::new(l, 0.0),
                    Vec2::new(l, w),
                    Vec2::new(0.0, w),
                ],
            },
            height: h,
            reflectances: [
                rho_ceiling.get_untracked() as f32,
                rho_wall.get_untracked() as f32,
                rho_floor.get_untracked() as f32,
            ],
            plane: workplane_height.get_untracked() as f32,
            luminaires,
            sensors,
            sensor_height: h,
            note: None,
            photometries,
            emergency,
            emergency_mode,
        }
    };

    // ─── Zonal lumen summary ────────────────────────────────────────────
    let zonal_lumens = Memo::new(move |_| PhotometricCalculations::zonal_lumens_30deg(&ldc.get()));

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
                    // Room outline: draw / cancel / back to the rectangle
                    {move || {
                        let l = locale.get();
                        if drawing_shape.get() {
                            let n = shape_wip.with(|v| v.len());
                            view! {
                                <button class="zonal-move-btn" on:click=move |_| {
                                    drawing_shape.set(false);
                                    shape_wip.set(Vec::new());
                                }>{format!("{} ({n})", l.area_designer.polygon.cancel)}</button>
                            }.into_any()
                        } else if room_shape.get().is_some() {
                            view! {
                                <button class="zonal-move-btn" on:click=move |_| room_shape.set(None)>
                                    {format!("▭ {}", l.area_designer.polygon.clear)}
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <button class="zonal-move-btn" on:click=move |_| {
                                    shape_wip.set(Vec::new());
                                    drawing_shape.set(true);
                                }>{format!("✏️ {}", l.area_designer.polygon.draw)}</button>
                            }.into_any()
                        }
                    }}

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
                                let l = ldc.get();
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

                    // Presence sensors
                    <SensorSidebar state=sensors plan=sensor_plan/>

                    // Emergency lighting (EN 1838)
                    <EmergencySidebar
                        state=emergency
                        source=em_source
                        room_area=Signal::derive(move || floor.get().area())
                        loaded_name=Signal::derive(move || ldc.with(|l| l.luminaire_name.clone()))
                    />

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
                        {open_light_scene.map(|open| view! {
                            <button class="zonal-share-btn zonal-light-scene-btn"
                                title=move || locale.get().ui.light_scene.open_in_light_scene_tip.clone()
                                on:click=move |_| open.0.run(make_plan())
                            >{move || locale.get().ui.light_scene.open_in_light_scene.clone()}</button>
                        })}
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
                                let ldt_val = ldc.get();
                                let u = units.get();
                                let r = result.get();
                                let ct = cu_table.get();

                                let room = Room::new(
                                    room_length.get(), room_width.get(), room_height.get(),
                                    workplane_height.get(), suspension_length.get(),
                                ).with_outline(shape_m());

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
                                if let Some(o) = room.outline.as_ref() {
                                    typ.push_str(&format!(
                                        "Room outline with {} corners, {:.1} m of walls; length and width are its bounds.\n\n",
                                        o.len(),
                                        room.perimeter()
                                    ));
                                }

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
                                let arrangement = if r.layout.is_moved() {
                                    "moved by hand".to_string()
                                } else {
                                    format!("{}×{}", r.layout.rows, r.layout.cols)
                                };
                                typ.push_str(&format!(
                                    "  [Luminaires], [*{} ({})*], [Target], [*{:.0} lux*], [Achieved], [*{:.0} lux*],\n",
                                    r.layout.count, arrangement, r.target_illuminance, r.achieved_illuminance
                                ));
                                let spacing_status = if r.layout.is_moved() {
                                    "grid moved by hand"
                                } else if r.layout.spacing_ok {
                                    "OK"
                                } else {
                                    "EXCEEDS"
                                };
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

                                if let Some(p) = sensor_plan.get_untracked() {
                                    let svg = ZonalSvg::room_plan_with_overlay(
                                        &room, &r.layout, &theme, u,
                                        &|ox, oy, s| p.overlay(ox, oy, s, sensors.show_all.get_untracked()),
                                    );
                                    let e = sensor_energy.get_untracked();
                                    typ.push_str(&zonal_sensors::typst_section(&p, e.as_ref(), &svg));
                                }
                                if let Some(er) = em_result.get_untracked() {
                                    let routes = emergency.routes.get_untracked();
                                    typ.push_str(&emergency_typst(&er, &routes, &room, &r.layout, &theme, u, &floor.get_untracked()));
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
                        // the emergency lighting alone, for the chosen standard
                        {move || em_result.with(|r| r.as_ref().map(|r| r.limits.name.clone())).map(|std_name| view! {
                            <button class="zonal-export-btn"
                                disabled=move || pdf_exporting.get()
                                on:click=move |_| {
                                    let Some(er) = em_result.get_untracked() else { return; };
                                    let u = units.get_untracked();
                                    let r = result.get_untracked();
                                    let room = Room::new(
                                        room_length.get_untracked(), room_width.get_untracked(), room_height.get_untracked(),
                                        workplane_height.get_untracked(), suspension_length.get_untracked(),
                                    ).with_outline(shape_m());
                                    let theme = SvgTheme::light();
                                    let routes = emergency.routes.get_untracked();
                                    let typ = zonal_emergency::report_typst(
                                        &er,
                                        &room,
                                        &emergency_typst(&er, &routes, &room, &r.layout, &theme, u, &floor.get_untracked()),
                                    );
                                    set_pdf_exporting.set(true);
                                    let file = format!("emergency_lighting_{}", er.limits.id);
                                    wasm_bindgen_futures::spawn_local(async move {
                                        match compile_typst_to_pdf(&typ).await {
                                            Ok(pdf_bytes) => {
                                                super::file_handler::download_bytes(
                                                    &format!("{file}.pdf"), &pdf_bytes, "application/pdf",
                                                );
                                            }
                                            Err(e) => {
                                                web_sys::console::error_1(&format!("PDF export failed: {e}").into());
                                                super::file_handler::download_file(&format!("{file}.typ"), &typ, "text/plain");
                                            }
                                        }
                                        set_pdf_exporting.set(false);
                                    });
                                }
                            >{move || fill(&locale.get().zonal_designer.emergency.report_pdf, &[("std", std_name.clone())])}</button>
                        })}
                    </div>
                </div>

                // ─── Views ────────────────────────────────────────────
                <div class="zonal-views">
                    // Results summary + plan view
                    <div class="zonal-panel">
                        <div class="zonal-plan-head">
                            <h3>{move || locale.get().designer.plan_view.clone()}</h3>
                            // normal lighting / emergency conditions
                            {move || em_result.with(|r| r.is_some()).then(|| view! {
                                <div class="zonal-em-switch">
                                    <button class:active=move || !emergency.show.get()
                                        on:click=move |_| emergency.show.set(false)
                                    >{move || locale.get().zonal_designer.emergency.view_normal.clone()}</button>
                                    <button class:active=move || emergency.show.get()
                                        on:click=move |_| emergency.show.set(true)
                                    >{move || locale.get().zonal_designer.emergency.view_emergency.clone()}</button>
                                </div>
                            })}
                        </div>
                        // Room plan SVG
                        <div class="zonal-plan-svg zonal-plan-drag" node_ref=plan_ref
                            class:dragging=move || dragging.get().is_some()
                            on:pointerdown=on_plan_pointerdown
                            on:pointermove=on_plan_pointermove
                            on:pointerup=on_plan_pointerup
                            on:pointercancel=on_plan_pointerup
                            on:dblclick=move |_| {
                                if emergency.tool.get_untracked() == EmTool::Route {
                                    emergency.finish_route();
                                }
                            }
                            class:zonal-plan-tool=move || emergency.tool.get() != EmTool::Off
                            inner_html=move || {
                            let r = result.get();
                            let room = Room::new(
                                room_length.get(), room_width.get(), room_height.get(),
                                workplane_height.get(), suspension_length.get(),
                            ).with_outline(shape_m());
                            let highlight = match dragging.get() {
                                Some((MoveWhat::Luminaires, i)) => Some(i),
                                _ => None,
                            };
                            let em = em_result.get();
                            let em_routes = emergency.routes.get();
                            let em_wip = emergency.wip.get();
                            let em_floor_map = em_floor.get();
                            // emergency conditions: the normal luminaires are
                            // off (unless they are the emergency lighting)
                            let lums_off = em_floor_map.is_some()
                                && em.as_ref().is_some_and(|er| er.source.which != zonal_emergency::EmLuminaire::Room);
                            let mut svg = sensor_plan.with(|p| {
                                let overlay = |ox, oy, s| {
                                    let mut o = String::new();
                                    // emergency conditions: the floor in the emergency light
                                    if let (Some(g), Some(er)) = (em_floor_map.as_ref(), em.as_ref()) {
                                        o.push_str(&zonal_emergency::floor_svg(g, &zonal_emergency::floor_bands(&er.limits), ox, oy, s));
                                    }
                                    o.push_str(&p.as_ref().map(|p| p.overlay(ox, oy, s, sensors.show_all.get())).unwrap_or_default());
                                    o.push_str(&zonal_emergency::overlay(em.as_ref(), &em_routes, &em_wip, ox, oy, s, true));
                                    o
                                };
                                ZonalSvg::room_plan_styled(
                                    &room, &r.layout, &SvgTheme::light(), units.get(), &overlay, highlight, lums_off,
                                )
                            });
                            // the outline being drawn
                            let wip = shape_wip.get();
                            if drawing_shape.get() && !wip.is_empty() && svg.ends_with("</svg>") {
                                let f = PlanFrame::for_room(&room);
                                let px = |x: f64| f.ox + x * f.scale;
                                let py = |y: f64| f.oy + y * f.scale;
                                svg.truncate(svg.len() - 6);
                                let pts: Vec<String> = wip.iter().map(|&(x, y)| format!("{:.1},{:.1}", px(x), py(y))).collect();
                                svg.push_str(&format!(
                                    r##"<polyline points="{}" fill="none" stroke="#dc2626" stroke-width="2" stroke-dasharray="5,3"/>"##,
                                    pts.join(" ")
                                ));
                                for (k, &(x, y)) in wip.iter().enumerate() {
                                    svg.push_str(&format!(
                                        r##"<circle cx="{:.1}" cy="{:.1}" r="{}" fill="rgba(220,38,38,0.75)" stroke="#991b1b"/>"##,
                                        px(x), py(y), if k == 0 { 6 } else { 4 }
                                    ));
                                }
                                svg.push_str("</svg>");
                            }
                            svg
                        }></div>
                        <zonal_emergency::FloorLegend grid=em_floor result=em_result/>
                        // Room outline: how to draw it / what it is
                        {move || {
                            let s = locale.get().zonal_designer.sensors.clone();
                            if drawing_shape.get() {
                                Some(view! { <p class="zonal-sensors-hint">{s.outline_hint.clone()}</p> }.into_any())
                            } else {
                                shape_m().map(|o| {
                                    let room = Room::new(room_length.get(), room_width.get(), 3.0, 0.8, 0.0).with_outline(Some(o));
                                    view! {
                                        <p class="zonal-sensors-hint">{fill(&s.outline_area, &[
                                            ("a", format!("{:.1}", room.area())),
                                            ("p", format!("{:.1}", room.perimeter())),
                                        ])}</p>
                                    }.into_any()
                                })
                            }
                        }}
                        // Moving luminaires / sensors on the plan
                        <div class="zonal-move-bar">
                            <span class="zonal-move-label">{move || locale.get().zonal_designer.sensors.move_title.clone()}</span>
                            <button class="zonal-move-btn" class:active=move || move_what.get() == MoveWhat::Luminaires || sensor_plan.with(|p| p.is_none())
                                on:click=move |_| move_what.set(MoveWhat::Luminaires)
                            >{move || format!("💡 {}", locale.get().zonal_designer.sensors.move_luminaires)}</button>
                            {move || sensor_plan.with(|p| p.is_some()).then(|| view! {
                                <button class="zonal-move-btn" class:active=move || move_what.get() == MoveWhat::Sensors
                                    on:click=move |_| move_what.set(MoveWhat::Sensors)
                                >{move || format!("📡 {}", locale.get().zonal_designer.sensors.move_sensors)}</button>
                            })}
                            {move || em_result.with(|r| r.as_ref().is_some_and(|r| r.source.which != zonal_emergency::EmLuminaire::Room && !r.positions.is_empty())).then(|| view! {
                                <button class="zonal-move-btn" class:active=move || move_what.get() == MoveWhat::Emergency
                                    on:click=move |_| move_what.set(MoveWhat::Emergency)
                                >{move || format!("🚨 {}", locale.get().zonal_designer.emergency.move_label)}</button>
                            })}
                            {move || {
                                let n = moved_count.get();
                                (n > 0).then(|| view! {
                                    <span class="zonal-move-count">{move || fill(&locale.get().zonal_designer.sensors.moved, &[("n", n.to_string())])}</span>
                                    <button class="zonal-move-btn" on:click=move |_| {
                                        lum_moves.set(Moves::default());
                                        sensors.moves.set(Moves::default());
                                        emergency.moves.set(Moves::default());
                                    }>{move || format!("↺ {}", locale.get().zonal_designer.sensors.reset_moved)}</button>
                                })
                            }}
                            <span class="zonal-move-hint">{move || locale.get().zonal_designer.sensors.drag_hint.clone()}</span>
                        </div>
                        // Results summary table
                        <table class="zonal-stats-table">
                            <tr>
                                <td>{move || locale.get().zonal_designer.results.luminaires.clone()}</td>
                                <td class="val">{move || {
                                    let r = result.get();
                                    if r.layout.is_moved() {
                                        format!("{} ({})", r.layout.count, locale.get().zonal_designer.sensors.spacing_moved)
                                    } else {
                                        format!("{} ({}×{})", r.layout.count, r.layout.rows, r.layout.cols)
                                    }
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
                                        if r.layout.is_moved() {
                                            return locale.get().zonal_designer.sensors.spacing_moved.clone();
                                        }
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
                        <SensorResults plan=sensor_plan energy=sensor_energy show_all=sensors.show_all/>
                        <EmergencyResults result=em_result/>
                    </div>

                    // Tab-driven view panel
                    {move || {
                        let tab = view_tab.get();
                        let r = result.get();
                        let room = Room::new(
                            room_length.get(), room_width.get(), room_height.get(),
                            workplane_height.get(), suspension_length.get(),
                        ).with_outline(shape_m());

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
.zonal-plan-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
}
.zonal-em-switch {
    display: flex;
    gap: 3px;
}
.zonal-em-switch button {
    padding: 3px 8px;
    font-size: 11px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--surface-elevated, transparent);
    color: var(--text-primary);
    border-radius: 4px;
}
.zonal-em-switch button.active {
    background: var(--primary-color);
    color: #fff;
}
.zonal-em-legend-bands {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    margin-top: 4px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
}
.zonal-em-legend-bands span {
    padding: 1px 5px;
    border-radius: 2px;
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
