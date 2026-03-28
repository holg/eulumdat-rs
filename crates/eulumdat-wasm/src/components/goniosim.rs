//! Virtual Goniophotometer WASM demo.
//!
//! Load an LDT/IES file, configure cover materials (PMMA, glass, etc.),
//! trace photons through the cover, watch the new LVK build up in real-time.
//! Compare original vs. simulated side by side. Export as .ldt.
//!
//! Activated via `?wasm=goniosim` query parameter.

use crate::i18n::use_locale;
use eulumdat::diagram::{PolarDiagram as CorePolarDiagram, SvgTheme};
use eulumdat::Eulumdat;
use eulumdat_goniosim::nalgebra::{Point3, Vector3};
use eulumdat_goniosim::*;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Batch size: photons traced per requestAnimationFrame tick.
const BATCH_SIZE: u64 = 10_000;

/// Total photons target.
const TARGET_PHOTONS: u64 = 1_000_000;

/// Built-in template LDT files (name, content).
const TEMPLATES: &[(&str, &str)] = &[
    ("Fluorescent luminaire", include_str!("../../templates/fluorescent_luminaire.ldt")),
    ("Road luminaire", include_str!("../../templates/road_luminaire.ldt")),
    ("Projector", include_str!("../../templates/projector.ldt")),
    ("Floor uplight", include_str!("../../templates/floor_uplight.ldt")),
    ("Batwing", include_str!("../../templates/wiki-batwing.ldt")),
    ("Floodlight", include_str!("../../templates/wiki-flood.ldt")),
    ("Spotlight", include_str!("../../templates/wiki-spotlight.ldt")),
];

/// Cover material presets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CoverPreset {
    None,
    Custom,
    ClearPmma,
    SatinPmma,
    OpalLightPmma,
    OpalPmma,
    ClearGlass,
    SatinGlass,
}

impl CoverPreset {
    fn label(&self) -> &'static str {
        match self {
            Self::None => "No cover (free space)",
            Self::Custom => "Custom",
            Self::ClearPmma => "PMMA klar 3mm",
            Self::SatinPmma => "PMMA satin 3mm",
            Self::OpalLightPmma => "PMMA opal leicht 3mm",
            Self::OpalPmma => "PMMA opal 3mm",
            Self::ClearGlass => "Glas klar 4mm",
            Self::SatinGlass => "Glas satiniert 4mm",
        }
    }

    fn all() -> &'static [CoverPreset] {
        &[
            Self::None,
            Self::ClearPmma,
            Self::SatinPmma,
            Self::OpalLightPmma,
            Self::OpalPmma,
            Self::ClearGlass,
            Self::SatinGlass,
            Self::Custom,
        ]
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|p| p == self).unwrap_or(0)
    }

    fn from_index(i: usize) -> Self {
        Self::all().get(i).copied().unwrap_or(Self::None)
    }

    fn to_params(&self) -> Option<MaterialParams> {
        match self {
            Self::None | Self::Custom => None,
            Self::ClearPmma => Some(catalog::clear_pmma_3mm()),
            Self::SatinPmma => Some(catalog::satin_pmma_3mm()),
            Self::OpalLightPmma => Some(catalog::opal_light_pmma_3mm()),
            Self::OpalPmma => Some(catalog::opal_pmma_3mm()),
            Self::ClearGlass => Some(catalog::clear_glass_4mm()),
            Self::SatinGlass => Some(catalog::satin_glass_4mm()),
        }
    }

    fn has_cover(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[component]
pub fn GonioSimDemo() -> impl IntoView {
    let locale = use_locale();

    // --- Input LDT (the source luminaire) ---
    let (source_ldt, set_source_ldt) = signal::<Option<Eulumdat>>(None);
    let (source_name, set_source_name) = signal(String::new());

    // --- Cover material ---
    let (cover_preset, set_cover_preset) = signal(CoverPreset::None);
    let (reflectance_pct, set_reflectance_pct) = signal(4.0f64);
    let (diffusion_pct, set_diffusion_pct) = signal(95.0f64);
    let (transmittance_pct, set_transmittance_pct) = signal(50.0f64);
    let (ior, set_ior) = signal(1.49f64);
    let (thickness_mm, set_thickness_mm) = signal(3.0f64);
    let (cover_distance_mm, set_cover_distance_mm) = signal(40.0f64);

    // --- C-plane selector ---
    let (selected_plane, set_selected_plane) = signal::<Option<f64>>(None);
    let (slider_idx, set_slider_idx) = signal(0usize);

    let c_planes = Memo::new(move |_| {
        source_ldt.get().map_or(vec![], |l| CorePolarDiagram::available_c_planes(&l))
    });

    // --- Simulation state ---
    let (running, set_running) = signal(false);
    let (photons_done, set_photons_done) = signal(0u64);
    let (photons_detected, set_photons_detected) = signal(0u64);
    let (photons_absorbed, set_photons_absorbed) = signal(0u64);
    let (sim_ldt, set_sim_ldt) = signal::<Option<Eulumdat>>(None);
    let (generation, set_generation) = signal(0u32);
    let (export_ldt_string, set_export_ldt_string) = signal(String::new());

    // Apply cover preset params
    Effect::new(move |_| {
        let p = cover_preset.get();
        if let Some(m) = p.to_params() {
            set_reflectance_pct.set(m.reflectance_pct);
            set_diffusion_pct.set(m.diffusion_pct);
            set_transmittance_pct.set(m.transmittance_pct);
            set_ior.set(m.ior);
            set_thickness_mm.set(m.thickness_mm);
        }
    });

    // Load an LDT from string content
    let load_ldt = move |name: String, content: String| {
        match Eulumdat::parse(&content) {
            Ok(ldt) => {
                set_source_name.set(name);
                set_source_ldt.set(Some(ldt));
                // Reset simulation
                reset_sim(
                    set_running, set_photons_done, set_photons_detected,
                    set_photons_absorbed, set_sim_ldt, set_export_ldt_string, set_generation,
                );
            }
            Err(_) => {
                // Try IES
                if let Ok(ldt) = eulumdat::IesParser::parse(&content) {
                    set_source_name.set(name);
                    set_source_ldt.set(Some(ldt));
                    reset_sim(
                        set_running, set_photons_done, set_photons_detected,
                        set_photons_absorbed, set_sim_ldt, set_export_ldt_string, set_generation,
                    );
                }
            }
        }
    };

    // Load default template on mount
    Effect::new({
        let load_ldt = load_ldt;
        move |_| {
            if source_ldt.get_untracked().is_none() {
                let (name, content) = TEMPLATES[0];
                load_ldt(name.to_string(), content.to_string());
            }
        }
    });

    // File input handler
    let on_file_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.unchecked_into();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let name = file.name();
                let load_ldt = load_ldt;
                wasm_bindgen_futures::spawn_local(async move {
                    let text = gloo_file::futures::read_as_text(&file.into()).await;
                    if let Ok(content) = text {
                        load_ldt(name, content);
                    }
                });
            }
        }
    };

    // Template selector
    let on_template_select = move |ev: web_sys::Event| {
        if let Ok(i) = event_target_value(&ev).parse::<usize>() {
            if let Some((name, content)) = TEMPLATES.get(i) {
                load_ldt(name.to_string(), content.to_string());
            }
        }
    };

    // Reset simulation state
    let reset = move || {
        reset_sim(
            set_running, set_photons_done, set_photons_detected,
            set_photons_absorbed, set_sim_ldt, set_export_ldt_string, set_generation,
        );
    };

    // Build scene from current signals (called inside spawn_local)
    let build_scene = move || -> Option<Scene> {
        let ldt = source_ldt.get_untracked()?;
        // Use luminaire output flux (lamp flux * LOR), not raw lamp flux.
        // The LDT intensity values are already scaled by LOR.
        let lamp_flux = ldt.total_luminous_flux().max(1.0);
        let lor = ldt.light_output_ratio / 100.0;
        let luminaire_flux = if lor > 0.0 { lamp_flux * lor } else { lamp_flux };
        let cp = cover_preset.get_untracked();

        let mut scene = Scene::new();
        scene.add_source(Source::from_lvk(
            Point3::origin(),
            eulumdat_goniosim::nalgebra::Rotation3::identity(),
            ldt,
            luminaire_flux,
        ));

        if cp.has_cover() {
            let cover_mat = if cp == CoverPreset::Custom {
                MaterialParams {
                    name: "Custom cover".into(),
                    reflectance_pct: reflectance_pct.get_untracked(),
                    ior: ior.get_untracked(),
                    transmittance_pct: transmittance_pct.get_untracked(),
                    thickness_mm: thickness_mm.get_untracked(),
                    diffusion_pct: diffusion_pct.get_untracked(),
                }
            } else {
                cp.to_params().unwrap()
            };

            let dist = cover_distance_mm.get_untracked() / 1000.0;
            let mat_id = scene.add_material(cover_mat);
            // Place a cover sheet below the source
            scene.add_object(
                Primitive::Sheet {
                    center: Point3::new(0.0, 0.0, -dist),
                    normal: Vector3::z_axis(),
                    u_axis: Vector3::x_axis(),
                    half_width: 0.5,
                    half_height: 0.5,
                    thickness: thickness_mm.get_untracked() / 1000.0,
                },
                mat_id,
                "cover",
            );
        }

        Some(scene)
    };

    // Start tracing
    let start_tracing = move |_: web_sys::MouseEvent| {
        let scene = match build_scene() {
            Some(s) => s,
            None => return,
        };
        set_running.set(true);
        let gen = generation.get_untracked();
        let src = source_ldt.get_untracked().unwrap();
        let lamp_flux = src.total_luminous_flux().max(1.0);
        let lor = src.light_output_ratio / 100.0;
        let flux = if lor > 0.0 { lamp_flux * lor } else { lamp_flux };

        // Use source LDT's angular resolution for the detector
        let c_res = if src.c_plane_distance > 0.0 { src.c_plane_distance } else { 15.0 };
        let g_res = if src.g_plane_distance > 0.0 { src.g_plane_distance } else { 5.0 };

        // Copy metadata from source LDT for export
        let src_clone = src.clone();

        wasm_bindgen_futures::spawn_local(async move {
            use eulumdat_goniosim::rand::Rng;
            use eulumdat_goniosim::rand::SeedableRng;

            let mut det = Detector::new(c_res, g_res);
            let mut rng = rand_xoshiro::Xoshiro256PlusPlus::seed_from_u64(42);
            let mut total_done = 0u64;
            let mut total_detected = 0u64;
            let mut total_absorbed = 0u64;
            let num_sources = scene.sources.len().max(1);

            loop {
                if !running.get_untracked() || generation.get_untracked() != gen {
                    break;
                }
                if total_done >= TARGET_PHOTONS {
                    set_running.set(false);
                    break;
                }

                let batch = BATCH_SIZE.min(TARGET_PHOTONS - total_done);

                for i in 0..batch {
                    let source = &scene.sources[(i as usize) % num_sources];
                    let ray = source.sample(&mut rng);
                    let mut photon = Photon::new(ray);
                    let mut escaped = false;

                    for _ in 0..50u32 {
                        match scene.intersect(&photon.ray, 1e-6, 1e10) {
                            None => {
                                det.record(photon.ray.direction.as_ref(), photon.energy);
                                total_detected += 1;
                                escaped = true;
                                break;
                            }
                            Some(hit) => {
                                let mat = scene.material(hit.material);
                                match mat.interact(&photon, &hit, &mut rng) {
                                    Interaction::Absorbed => {
                                        total_absorbed += 1;
                                        escaped = true;
                                        break;
                                    }
                                    Interaction::Reflected { new_ray, attenuation } => {
                                        photon.ray = new_ray;
                                        photon.energy *= attenuation;
                                        photon.bounces += 1;
                                    }
                                    Interaction::Transmitted { new_ray, attenuation } => {
                                        photon.ray = new_ray;
                                        photon.energy *= attenuation;
                                        photon.bounces += 1;
                                    }
                                }
                                if photon.energy < 0.01 {
                                    let survive = photon.energy / 0.01;
                                    if rng.random::<f64>() > survive {
                                        total_absorbed += 1;
                                        escaped = true;
                                        break;
                                    }
                                    photon.energy = 0.01;
                                }
                            }
                        }
                    }
                    if !escaped {
                        total_absorbed += 1;
                    }
                }

                total_done += batch;
                set_photons_done.set(total_done);
                set_photons_detected.set(total_detected);
                set_photons_absorbed.set(total_absorbed);

                // Update simulated SVG — use source LDT metadata
                let export_cfg = ExportConfig {
                    c_step_deg: c_res,
                    g_step_deg: g_res,
                    luminaire_name: format!("{} (simulated)", src_clone.luminaire_name),
                    manufacturer: src_clone.identification.clone(),
                    luminaire_dimensions_mm: (src_clone.length, src_clone.width, src_clone.height),
                    luminous_area_mm: (src_clone.luminous_area_length, src_clone.luminous_area_width),
                    ..ExportConfig::default()
                };
                let mut ldt = detector_to_eulumdat_with_lamp_flux(&det, flux, lamp_flux, &export_cfg);
                // Copy lamp data from source
                ldt.lamp_sets = src_clone.lamp_sets.clone();
                ldt.type_indicator = src_clone.type_indicator;

                // LOR: scale by energy throughput (not photon count —
                // ClearTransmitter attenuates energy without killing photons)
                let energy_frac = det.total_energy() / total_done as f64;
                ldt.light_output_ratio = src_clone.light_output_ratio * energy_frac;
                set_export_ldt_string.set(ldt.to_ldt());
                set_sim_ldt.set(Some(ldt));

                // Yield to browser
                let promise = js_sys::Promise::new(&mut |resolve, _| {
                    let _ = web_sys::window()
                        .unwrap()
                        .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
                });
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            }
        });
    };

    // Export
    let export_ldt = move |_| {
        let s = export_ldt_string.get_untracked();
        if !s.is_empty() {
            download_string(&s, "goniosim-export.ldt");
        }
    };

    // UI
    view! {
        <div style="width: 100vw; height: 100vh; background: #0d1117; color: #c9d1d9; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace; display: flex; flex-direction: column; overflow: hidden;">
            // Header
            <div style="padding: 12px 20px; border-bottom: 1px solid #30363d; display: flex; align-items: center; gap: 16px; flex-shrink: 0;">
                <h1 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: #58a6ff;">{move || locale.get().goniosim.title.clone()}</h1>
                <span style="color: #484f58; font-size: 0.8rem;">{move || locale.get().goniosim.subtitle.clone()}</span>
                <div style="margin-left: auto; display: flex; gap: 8px;">
                    <button
                        style="padding: 6px 16px; background: #238636; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.85rem;"
                        disabled=move || source_ldt.get().is_none()
                        on:click=move |ev| {
                            if running.get() {
                                set_running.set(false);
                            } else {
                                start_tracing(ev);
                            }
                        }
                    >
                        {move || {
                            let g = locale.get().goniosim;
                            if running.get() { g.pause.clone() } else if photons_done.get() > 0 { g.resume.clone() } else { g.trace.clone() }
                        }}
                    </button>
                    <button
                        style="padding: 6px 16px; background: #21262d; color: #c9d1d9; border: 1px solid #30363d; border-radius: 6px; cursor: pointer; font-size: 0.85rem;"
                        on:click=move |_| reset()
                    >
                        {move || locale.get().goniosim.reset.clone()}
                    </button>
                </div>
            </div>

            // Main content
            <div style="display: flex; flex: 1; overflow: hidden;">
                // Left panel: controls
                <div style="width: 280px; padding: 16px; border-right: 1px solid #30363d; overflow-y: auto; flex-shrink: 0;">

                    // Source LDT input
                    <div style="margin-bottom: 16px;">
                        <label style="display: block; font-size: 0.75rem; color: #8b949e; margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.5px;">{move || locale.get().goniosim.input_luminaire.clone()}</label>
                        // Template selector
                        <select
                            style="width: 100%; padding: 6px; background: #161b22; color: #c9d1d9; border: 1px solid #30363d; border-radius: 4px; margin-bottom: 6px;"
                            on:change=on_template_select
                        >
                            {TEMPLATES.iter().enumerate().map(|(i, (name, _))| {
                                view! { <option value=i.to_string()>{*name}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                        // File upload
                        <label style="display: block; padding: 6px; text-align: center; background: #161b22; border: 1px dashed #30363d; border-radius: 4px; cursor: pointer; font-size: 0.8rem; color: #8b949e;">
                            {move || locale.get().goniosim.upload.clone()}
                            <input
                                type="file"
                                accept=".ldt,.LDT,.ies,.IES"
                                style="display: none;"
                                on:change=on_file_input
                            />
                        </label>
                        // Current file name
                        {move || {
                            let name = source_name.get();
                            if !name.is_empty() {
                                view! {
                                    <div style="margin-top: 4px; font-size: 0.75rem; color: #58a6ff;">{name}</div>
                                }.into_any()
                            } else {
                                view! { <div /> }.into_any()
                            }
                        }}
                    </div>

                    // Cover material
                    <div style="margin-bottom: 16px;">
                        <label style="display: block; font-size: 0.75rem; color: #8b949e; margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.5px;">{move || locale.get().goniosim.cover_material.clone()}</label>
                        <select
                            style="width: 100%; padding: 6px; background: #161b22; color: #c9d1d9; border: 1px solid #30363d; border-radius: 4px; margin-bottom: 8px;"
                            on:change=move |ev| {
                                if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                                    set_cover_preset.set(CoverPreset::from_index(i));
                                    reset();
                                }
                            }
                            prop:value=move || cover_preset.get().index().to_string()
                        >
                            {CoverPreset::all().iter().enumerate().map(|(i, p)| {
                                view! { <option value=i.to_string()>{p.label()}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                        // Sliders (shown when cover is active)
                        {move || {
                            if cover_preset.get().has_cover() {
                                view! {
                                    <div>
                                        {slider(locale.get().goniosim.reflectance.clone(), "%", reflectance_pct, set_reflectance_pct, 0.0, 50.0, 1.0, move || { set_cover_preset.set(CoverPreset::Custom); reset(); })}
                                        {slider(locale.get().goniosim.diffusion.clone(), "%", diffusion_pct, set_diffusion_pct, 0.0, 100.0, 1.0, move || { set_cover_preset.set(CoverPreset::Custom); reset(); })}
                                        {slider(locale.get().goniosim.transmittance.clone(), "%", transmittance_pct, set_transmittance_pct, 5.0, 98.0, 1.0, move || { set_cover_preset.set(CoverPreset::Custom); reset(); })}
                                        {slider(locale.get().goniosim.ior.clone(), "", ior, set_ior, 1.0, 2.0, 0.01, move || { set_cover_preset.set(CoverPreset::Custom); reset(); })}
                                        {slider(locale.get().goniosim.thickness.clone(), "mm", thickness_mm, set_thickness_mm, 1.0, 10.0, 0.5, move || { set_cover_preset.set(CoverPreset::Custom); reset(); })}
                                        {slider(locale.get().goniosim.distance.clone(), "mm", cover_distance_mm, set_cover_distance_mm, 5.0, 200.0, 5.0, move || reset())}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div /> }.into_any()
                            }
                        }}
                    </div>

                    // Stats
                    <div style="padding: 12px; background: #161b22; border-radius: 6px; border: 1px solid #30363d;">
                        <div style="font-size: 0.75rem; color: #8b949e; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px;">{move || locale.get().goniosim.statistics.clone()}</div>
                        <div style="font-size: 0.85rem; line-height: 1.8;">
                            <div>{move || locale.get().goniosim.photons.clone()} ": " {move || format!("{} / {}", format_number(photons_done.get()), format_number(TARGET_PHOTONS))}</div>
                            <div>{move || locale.get().goniosim.detected.clone()} ": " {move || {
                                let d = photons_done.get();
                                let det = photons_detected.get();
                                if d > 0 { format!("{:.1}%", det as f64 / d as f64 * 100.0) } else { "-".into() }
                            }}</div>
                            <div>{move || locale.get().goniosim.absorbed.clone()} ": " {move || {
                                let d = photons_done.get();
                                let a = photons_absorbed.get();
                                if d > 0 { format!("{:.1}%", a as f64 / d as f64 * 100.0) } else { "-".into() }
                            }}</div>
                        </div>
                    </div>

                    // Export
                    <div style="margin-top: 12px;">
                        <button
                            style="width: 100%; padding: 8px; background: #1f6feb; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.85rem;"
                            disabled=move || photons_done.get() == 0
                            on:click=export_ldt
                        >
                            {move || locale.get().goniosim.export_ldt.clone()}
                        </button>
                    </div>
                </div>

                // Right panel: side-by-side polar diagrams
                <div style="flex: 1; display: flex; flex-direction: column; overflow: hidden;">
                    // Labels + C-plane slider
                    <div style="display: flex; padding: 8px 20px 0; gap: 20px; flex-shrink: 0; align-items: center;">
                        <div style="flex: 1; text-align: center; font-size: 0.8rem; color: #8b949e; text-transform: uppercase; letter-spacing: 0.5px;">{move || locale.get().goniosim.original_lvk.clone()}</div>
                        // C-plane selector (only for asymmetric luminaires)
                        {move || {
                            let planes = c_planes.get();
                            if !planes.is_empty() {
                                let max_idx = planes.len() - 1;
                                view! {
                                    <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                                        {move || {
                                            if selected_plane.get().is_some() {
                                                view! {
                                                    <span style="font-size: 0.75rem; color: #58a6ff; white-space: nowrap;">
                                                        {move || format!("C {:.0}\u{00b0}", selected_plane.get().unwrap_or(0.0))}
                                                    </span>
                                                    <input type="range"
                                                        style="width: 100px; accent-color: #58a6ff;"
                                                        min="0"
                                                        prop:max=max_idx.to_string()
                                                        prop:value=move || slider_idx.get().to_string()
                                                        on:input=move |ev| {
                                                            if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                                set_slider_idx.set(idx);
                                                                let p = c_planes.get();
                                                                if let Some(&angle) = p.get(idx) {
                                                                    set_selected_plane.set(Some(angle));
                                                                }
                                                            }
                                                        }
                                                    />
                                                    <button style="background: none; border: 1px solid #30363d; color: #8b949e; border-radius: 4px; padding: 1px 6px; cursor: pointer; font-size: 0.75rem;"
                                                        on:click=move |_| {
                                                            set_selected_plane.set(None);
                                                            set_slider_idx.set(0);
                                                        }
                                                    >{move || locale.get().goniosim.all_planes.clone()}</button>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button style="background: none; border: 1px solid #30363d; color: #58a6ff; border-radius: 4px; padding: 2px 8px; cursor: pointer; font-size: 0.75rem;"
                                                        on:click=move |_| {
                                                            set_selected_plane.set(Some(0.0));
                                                            set_slider_idx.set(0);
                                                        }
                                                    >{move || locale.get().goniosim.c_plane.clone()}</button>
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div /> }.into_any()
                            }
                        }}
                        <div style="flex: 1; text-align: center; font-size: 0.8rem; color: #8b949e; text-transform: uppercase; letter-spacing: 0.5px;">{move || locale.get().goniosim.simulated.clone()}</div>
                    </div>
                    // Diagrams
                    <div style="flex: 1; display: flex; align-items: center; justify-content: center; padding: 10px 20px; gap: 20px; overflow: hidden;">
                        // Original — re-renders when selected_plane changes
                        <div style="flex: 1; max-width: 500px; display: flex; align-items: center; justify-content: center;">
                            {move || {
                                let ldt_opt = source_ldt.get();
                                let cp = selected_plane.get();
                                if ldt_opt.is_none() {
                                    view! {
                                        <div style="color: #484f58; text-align: center; font-size: 0.9rem;">{move || locale.get().goniosim.select_luminaire.clone()}</div>
                                    }.into_any()
                                } else {
                                    let ldt = ldt_opt.unwrap();
                                    let theme = SvgTheme::dark();
                                    let svg = CorePolarDiagram::render_svg(&ldt, cp, 450.0, 450.0, &theme);
                                    view! {
                                        <div style="width: 100%;" inner_html=svg />
                                    }.into_any()
                                }
                            }}
                        </div>
                        // Divider
                        <div style="width: 1px; height: 80%; background: #30363d; flex-shrink: 0;" />
                        // Simulated — re-renders when sim_ldt or selected_plane changes
                        <div style="flex: 1; max-width: 500px; display: flex; align-items: center; justify-content: center;">
                            {move || {
                                let ldt_opt = sim_ldt.get();
                                let cp = selected_plane.get();
                                if ldt_opt.is_none() {
                                    view! {
                                        <div style="color: #484f58; text-align: center; font-size: 0.9rem;">
                                            {move || locale.get().goniosim.click_trace.clone()}
                                        </div>
                                    }.into_any()
                                } else {
                                    let ldt = ldt_opt.unwrap();
                                    let theme = SvgTheme::dark();
                                    let svg = CorePolarDiagram::render_svg(&ldt, cp, 450.0, 450.0, &theme);
                                    view! {
                                        <div style="width: 100%;" inner_html=svg />
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </div>
            </div>

            // Progress bar
            <div style="height: 3px; background: #161b22; flex-shrink: 0;">
                <div style=move || format!(
                    "height: 100%; background: {}; width: {}%; transition: width 0.1s;",
                    if running.get() { "#238636" } else { "#1f6feb" },
                    if TARGET_PHOTONS > 0 { photons_done.get() as f64 / TARGET_PHOTONS as f64 * 100.0 } else { 0.0 }
                ) />
            </div>
        </div>
    }
}

fn reset_sim(
    set_running: WriteSignal<bool>,
    set_photons_done: WriteSignal<u64>,
    set_photons_detected: WriteSignal<u64>,
    set_photons_absorbed: WriteSignal<u64>,
    set_sim_ldt: WriteSignal<Option<Eulumdat>>,
    set_export_ldt_string: WriteSignal<String>,
    set_generation: WriteSignal<u32>,
) {
    set_running.set(false);
    set_photons_done.set(0);
    set_photons_detected.set(0);
    set_photons_absorbed.set(0);
    set_sim_ldt.set(None);
    set_export_ldt_string.set(String::new());
    set_generation.update(|g| *g += 1);
}

/// Labeled slider with value display.
fn slider(
    label: String,
    unit: &'static str,
    value: ReadSignal<f64>,
    set_value: WriteSignal<f64>,
    min: f64,
    max: f64,
    step: f64,
    on_change: impl Fn() + 'static + Copy,
) -> impl IntoView {
    view! {
        <div style="margin-bottom: 6px;">
            <div style="display: flex; justify-content: space-between; font-size: 0.8rem; margin-bottom: 2px;">
                <span style="color: #8b949e;">{label.clone()}</span>
                <span style="color: #c9d1d9;">{move || {
                    let v = value.get();
                    if step >= 1.0 { format!("{:.0}{unit}", v) }
                    else if step >= 0.1 { format!("{:.1}{unit}", v) }
                    else { format!("{:.2}{unit}", v) }
                }}</span>
            </div>
            <input type="range"
                style="width: 100%; accent-color: #58a6ff;"
                min=min.to_string()
                max=max.to_string()
                step=step.to_string()
                prop:value=move || value.get().to_string()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                        set_value.set(v);
                        on_change();
                    }
                }
            />
        </div>
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn download_string(content: &str, filename: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let opts = web_sys::BlobPropertyBag::new();
            opts.set_type("application/octet-stream");
            let blob = web_sys::Blob::new_with_str_sequence_and_options(
                &js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(content)),
                &opts,
            )
            .ok();
            if let Some(blob) = blob {
                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                    if let Ok(a) = document.create_element("a") {
                        let _ = a.set_attribute("href", &url);
                        let _ = a.set_attribute("download", filename);
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&a);
                            if let Some(el) = a.dyn_ref::<web_sys::HtmlElement>() {
                                el.click();
                            }
                            let _ = body.remove_child(&a);
                        }
                        let _ = web_sys::Url::revoke_object_url(&url);
                    }
                }
            }
        }
    }
}
