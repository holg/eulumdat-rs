//! Sensor comparison: the "Sensors" mode of the Compare tab. Two detectors
//! under the same conditions (mounting height, floor / workplane, C-plane):
//! plan / polar / cartesian overlays, range over the mounting height, room
//! check, numbers with Δ %, PDF report, share link (demo sensors).
//! The numbers come from `eulumdat::sensor::compare`.

use std::sync::Arc;

use eulumdat::diagram::{CartesianDiagram, PolarDiagram, SvgTheme};
use eulumdat::sensor::compare::{
    height_chart_svg, plan_overlay_svg, room_check, CompareConditions, HeightChartLabels,
    RoomCheck, SensorComparison, SideValues,
};
use eulumdat::sensor::coverage::{CoverageRoom, DEFAULT_WALL_MARGIN};
use eulumdat::sensor::en15193::{OccupancyControl, SpaceType};
use eulumdat::sensor::{diagrams, DetectionVolume, Motion, Quality};
use eulumdat::Significance;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::sensor_view::{add_demo, demo_sensors, SensorFiles};
use super::zonal_sensors::{motion_label, CoreSensor};
use crate::i18n::use_locale;

/// Views of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpView {
    Plan,
    Polar,
    Cartesian,
    Height,
    Room,
}

impl CmpView {
    const ALL: [CmpView; 5] = [
        CmpView::Plan,
        CmpView::Polar,
        CmpView::Cartesian,
        CmpView::Height,
        CmpView::Room,
    ];

    fn key(self) -> &'static str {
        match self {
            CmpView::Plan => "plan",
            CmpView::Polar => "polar",
            CmpView::Cartesian => "cartesian",
            CmpView::Height => "height",
            CmpView::Room => "room",
        }
    }

    fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.key() == k)
    }

    fn uses_c_plane(self) -> bool {
        matches!(self, CmpView::Plan | CmpView::Polar | CmpView::Cartesian)
    }
}

/// Requests to the Compare tab from the rest of the app (context, provided
/// by `App`): show the sensor mode, select a sensor that was just opened.
#[derive(Clone, Copy)]
pub struct CompareRequests {
    /// The Compare tab shows sensors instead of luminaires.
    pub sensors_mode: RwSignal<bool>,
    /// Name of a sensor to select (taken and cleared by the comparison).
    pub pick: RwSignal<Option<String>>,
    /// Both sensors to compare (A, B), e.g. from the dashboard.
    pub pair: RwSignal<Option<(String, String)>>,
}

/// A deep link asks for the sensor mode (`cmpkind=sensors`).
pub fn link_wants_sensors() -> bool {
    use_context::<super::app::RtLinkParams>()
        .map(|p| {
            p.0.get_value()
                .iter()
                .any(|(k, v)| k == "cmpkind" && v == "sensors")
        })
        .unwrap_or(false)
}

/// Both motion volumes, compared by identity (recomputed = changed).
#[derive(Clone)]
struct Volumes(Arc<DetectionVolume>, Arc<DetectionVolume>);

impl PartialEq for Volumes {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn quality_mark(q: Option<Quality>) -> &'static str {
    match q {
        Some(Quality::Interpolated) => " (i)",
        Some(Quality::Extrapolated) => " (e)",
        _ => "",
    }
}

fn demo_index(name: &str) -> Option<usize> {
    demo_sensors().iter().position(|d| d.name == name)
}

#[component]
pub fn SensorCompare() -> impl IntoView {
    let locale = use_locale();
    let files = use_context::<SensorFiles>()
        .map(|f| f.0)
        .unwrap_or_else(|| RwSignal::new(Vec::new()));

    // ─── State, first from the link ─────────────────────────────────────
    let link: Vec<(String, String)> = use_context::<super::app::RtLinkParams>()
        .map(|p| p.0.get_value())
        .unwrap_or_default();
    let get = |k: &str| link.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    let demo_name = |v: Option<String>| {
        v.and_then(|v| v.strip_prefix("demo")?.parse::<usize>().ok())
            .and_then(|i| demo_sensors().get(i).map(|d| d.name.clone()))
    };
    let (link_a, link_b) = (demo_name(get("sa")), demo_name(get("sb")));
    if link_a.is_some() || link_b.is_some() {
        add_demo(files);
    }
    let num = |k: &str, d: f64| get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
    let a_name = RwSignal::new(link_a);
    let b_name = RwSignal::new(link_b);
    let view = RwSignal::new(
        get("sview")
            .and_then(|v| CmpView::from_key(&v))
            .unwrap_or(CmpView::Plan),
    );
    let mount = RwSignal::new(num("smount", 3.0).clamp(1.0, 20.0));
    let workplane = RwSignal::new(num("swp", 0.8).clamp(0.0, 3.0));
    let planning = RwSignal::new(false);
    let c_plane = RwSignal::new(num("scplane", 0.0).rem_euclid(360.0));
    let motion = RwSignal::new(Motion::Tangential);
    let h_min = RwSignal::new(2.0_f64);
    let h_max = RwSignal::new(15.0_f64);
    let room_l = RwSignal::new(10.0_f64);
    let room_w = RwSignal::new(8.0_f64);
    let room_motion = RwSignal::new(Motion::Presence);
    let control = RwSignal::new(OccupancyControl::AutoOnAutoOff);
    let space = RwSignal::new(SpaceType::OfficeOverall);
    let price = RwSignal::new(None::<f64>);
    let copied = RwSignal::new(false);
    let pdf_busy = RwSignal::new(false);
    let drag = RwSignal::new(false);
    let open_files = use_context::<super::app::OpenFiles>();

    // ─── Derived ────────────────────────────────────────────────────────
    // Until the user picks: the first and the second sensor of the list.
    // A chosen sensor that was removed from the project falls back too.
    let listed = move |n: &String| files.with(|v| v.iter().any(|f| &f.name == n));
    let name_a = Memo::new(move |_| {
        a_name
            .get()
            .filter(listed)
            .or_else(|| files.with(|v| v.first().map(|f| f.name.clone())))
    });
    let name_b = Memo::new(move |_| {
        b_name
            .get()
            .filter(listed)
            .or_else(|| files.with(|v| v.get(1).or(v.first()).map(|f| f.name.clone())))
    });
    // Keep A and B on listed sensors: (re)pinned to the first / second
    // sensor when unset or gone (project loaded or switched), so opening a
    // file again (it moves to the top) never swaps what is compared. A
    // sensor opened while the Compare tab is shown (either drop area, the
    // file picker, a GLDF) is held until it is listed, then becomes B
    // (unless it is A already).
    let pending = RwSignal::new(None::<String>);
    let pending_pair = RwSignal::new(None::<(String, String)>);
    if let Some(req) = use_context::<CompareRequests>() {
        Effect::new(move |_| {
            if let Some(p) = req.pair.get() {
                req.pair.set(None);
                pending_pair.set(Some(p));
            }
        });
        Effect::new(move |_| {
            if let Some(n) = req.pick.get() {
                req.pick.set(None);
                pending.set(Some(n));
            }
        });
    }
    Effect::new(move |_| {
        let names: Vec<String> = files.with(|v| v.iter().map(|f| f.name.clone()).collect());
        let listed = |n: &Option<String>| n.as_ref().is_some_and(|n| names.contains(n));
        if !listed(&a_name.get_untracked()) {
            a_name.set(names.first().cloned());
        }
        if !listed(&b_name.get_untracked()) {
            b_name.set(names.get(1).or(names.first()).cloned());
        }
        if let Some((pa, pb)) = pending_pair
            .get()
            .filter(|(a, b)| names.contains(a) && names.contains(b))
        {
            pending_pair.set(None);
            a_name.set(Some(pa));
            b_name.set(Some(pb));
        }
        if let Some(p) = pending.get().filter(|p| names.contains(p)) {
            pending.set(None);
            if a_name.get_untracked().as_ref() != Some(&p) {
                b_name.set(Some(p));
            }
        }
    });
    let core_of = move |name: Memo<Option<String>>| {
        Memo::new(move |_| {
            let n = name.get()?;
            files.with(|v| {
                v.iter()
                    .find(|f| f.name == n)
                    .map(|f| CoreSensor(Arc::new(f.data.to_core())))
            })
        })
    };
    let core_a = core_of(name_a);
    let core_b = core_of(name_b);
    let cond = Memo::new(move |_| CompareConditions {
        mount: mount.get(),
        walk_plane: 0.0,
        presence_plane: workplane.get(),
        planning: planning.get(),
    });
    let cmp = Memo::new(move |_| {
        let (a, b) = (core_a.get()?, core_b.get()?);
        Some(SensorComparison::new(
            &a.0,
            &b.0,
            &cond.get(),
            &name_a.get().unwrap_or_default(),
            &name_b.get().unwrap_or_default(),
        ))
    });
    // The controls below only appear / vanish with this (a drag on the
    // height slider must not rebuild them).
    let has_cmp = Memo::new(move |_| cmp.with(|c| c.is_some()));
    // Motion types both sensors have (polar / cartesian need both volumes).
    let shared_motions = Memo::new(move |_| {
        cmp.with(|c| {
            c.as_ref()
                .map(|c| {
                    c.rows
                        .iter()
                        .filter(|r| r.a.is_some() && r.b.is_some())
                        .map(|r| r.motion)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let any_sensxml = Memo::new(move |_| {
        [core_a.get(), core_b.get()]
            .iter()
            .flatten()
            .any(|c| c.0.is_sensxml())
    });
    let volumes = Memo::new(move |_| {
        let (a, b) = (core_a.get()?, core_b.get()?);
        let shared = shared_motions.get();
        let m = if shared.contains(&motion.get()) {
            motion.get()
        } else {
            *shared.first()?
        };
        let va = a.0.motion_volume(m, planning.get())?;
        let vb = b.0.motion_volume(m, planning.get())?;
        Some(Volumes(Arc::new(va), Arc::new(vb)))
    });

    // ─── Share link ─────────────────────────────────────────────────────
    let link_hash = Memo::new(move |_| {
        let (a, b) = (name_a.get()?, name_b.get()?);
        let (ia, ib) = (demo_index(&a)?, demo_index(&b)?);
        Some(format!(
            "#tab=compare&cmpkind=sensors&sa=demo{ia}&sb=demo{ib}&smount={:.1}&swp={:.2}&scplane={:.0}&sview={}",
            mount.get(),
            workplane.get(),
            c_plane.get(),
            view.get().key()
        ))
    });
    let on_share = move |_| {
        let Some(hash) = link_hash.get_untracked() else {
            return;
        };
        if let Some(w) = web_sys::window() {
            let loc = w.location();
            let url = format!(
                "{}{}{}",
                loc.origin().unwrap_or_default(),
                loc.pathname().unwrap_or_default(),
                hash
            );
            let _ = w.navigator().clipboard().write_text(&url);
            copied.set(true);
            let cb = wasm_bindgen::closure::Closure::once(move || copied.set(false));
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                2000,
            );
            cb.forget();
        }
    };

    // ─── Room check rows ────────────────────────────────────────────────
    let room_checks = Memo::new(move |_| {
        let (a, b) = (core_a.get()?, core_b.get()?);
        let c = cond.get();
        let room = CoverageRoom {
            length: room_l.get(),
            width: room_w.get(),
            mount_height: c.mount,
            walk_plane: c.walk_plane,
            presence_plane: c.presence_plane,
            margin: DEFAULT_WALL_MARGIN,
        };
        let ctl = Some((control.get(), space.get(), true));
        let m = room_motion.get();
        Some((
            room_check(&a.0, &room, c.planning, m, 100, ctl),
            room_check(&b.0, &room, c.planning, m, 100, ctl),
        ))
    });

    // ─── PDF report ─────────────────────────────────────────────────────
    let on_report = move |_| {
        let (Some(a), Some(b)) = (core_a.get_untracked(), core_b.get_untracked()) else {
            return;
        };
        let demo = |n: &Option<String>| n.as_deref().and_then(demo_index).is_some();
        let opt = eulumdat_typst::SensorCompareOptions {
            label_a: name_a.get_untracked().unwrap_or_default(),
            label_b: name_b.get_untracked().unwrap_or_default(),
            conditions: cond.get_untracked(),
            c_plane: c_plane.get_untracked(),
            motion: motion.get_untracked(),
            heights: (h_min.get_untracked(), h_max.get_untracked()),
            room: Some(eulumdat_typst::CompareRoom {
                length: room_l.get_untracked(),
                width: room_w.get_untracked(),
                motion: room_motion.get_untracked(),
                control: control.get_untracked(),
                space: space.get_untracked(),
                hold_ok: true,
                price: price.get_untracked(),
            }),
            date: eulumdat_typst::sensor_report::today(),
            demo: demo(&name_a.get_untracked()) || demo(&name_b.get_untracked()),
        };
        let source = eulumdat_typst::generate_sensor_comparison_report(&a.0, &b.0, &opt);
        pdf_busy.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match super::app::compile_typst_to_pdf(&source).await {
                Ok(pdf) => super::file_handler::download_bytes(
                    "sensor_comparison.pdf",
                    &pdf,
                    "application/pdf",
                ),
                Err(e) => {
                    web_sys::console::error_1(&format!("sensor comparison PDF failed: {e}").into());
                    super::file_handler::download_file(
                        "sensor_comparison.typ",
                        &source,
                        "text/plain",
                    );
                }
            }
            pdf_busy.set(false);
        });
    };

    // ─── Diagram (the only closure that reads the C-plane) ─────────────
    let diagram_svg = move || -> String {
        let l = locale.get();
        let theme = diagrams::sensor_theme(&SvgTheme::css_variables_with_locale(&l));
        let (Some(a), Some(b)) = (core_a.get(), core_b.get()) else {
            return String::new();
        };
        let (la, lb) = (
            name_a.get().unwrap_or_default(),
            name_b.get().unwrap_or_default(),
        );
        let ls = l.ui.sensor.clone();
        let mlabel = |m: Motion| motion_label(&ls, m);
        match view.get() {
            CmpView::Plan => plan_overlay_svg(
                &a.0,
                &b.0,
                &cond.get(),
                &Motion::ALL,
                520.0,
                &theme,
                &mlabel,
                Some(c_plane.get()),
            ),
            CmpView::Polar => volumes
                .get()
                .map(|v| {
                    let c = c_plane.get();
                    let pa = PolarDiagram::from_eulumdat_for_plane(v.0.ldt(), c);
                    let pb = PolarDiagram::from_eulumdat_for_plane(v.1.ldt(), c);
                    PolarDiagram::to_overlay_svg(&pa, &pb, 500.0, 500.0, &theme, &la, &lb)
                })
                .unwrap_or_default(),
            CmpView::Cartesian => volumes
                .get()
                .map(|v| {
                    let c = c_plane.get();
                    let ca = CartesianDiagram::from_eulumdat_for_plane(v.0.ldt(), c, 640.0, 400.0);
                    let cb = CartesianDiagram::from_eulumdat_for_plane(v.1.ldt(), c, 640.0, 400.0);
                    CartesianDiagram::to_overlay_svg(&ca, &cb, 640.0, 400.0, &theme, &la, &lb)
                })
                .unwrap_or_default(),
            CmpView::Height => {
                let sc = &l.ui.sensor_compare;
                let labels = HeightChartLabels {
                    x_axis: sc.height_x.clone(),
                    y_axis: sc.height_y.clone(),
                    measured: sc.measured.clone(),
                };
                let (lo, hi) = (h_min.get(), h_max.get());
                height_chart_svg(
                    &a.0,
                    &b.0,
                    &cond.get(),
                    &Motion::ALL,
                    (lo.min(hi - 0.5), hi.max(lo + 0.5)),
                    680.0,
                    400.0,
                    &theme,
                    &labels,
                    &mlabel,
                    &la,
                    &lb,
                )
            }
            CmpView::Room => String::new(),
        }
    };

    // ─── Pickers ────────────────────────────────────────────────────────
    // The whole select is rebuilt when the list or the choice changes,
    // with the chosen option preselected: patching the options of a live
    // select leaves browsers showing a stale entry when the list re-orders.
    let picker = move |which: RwSignal<Option<String>>, current: Memo<Option<String>>| {
        move || {
            let chosen = current.get();
            let options = files
                .get()
                .into_iter()
                .map(|f| {
                    let selected = chosen.as_deref() == Some(f.name.as_str());
                    let value = f.name.clone();
                    view! { <option value=value selected=selected>{f.name}</option> }
                })
                .collect_view();
            view! {
                <select class="sensor-compare-select"
                    on:change=move |ev| which.set(Some(event_target_value(&ev)))
                >{options}</select>
            }
        }
    };

    let num_input = move |sig: RwSignal<f64>, min: f64, max: f64, step: f64| {
        view! {
            <input type="number" style="width:5.5em"
                prop:min=min.to_string() prop:max=max.to_string() prop:step=step.to_string()
                prop:value=move || format!("{}", (sig.get() * 100.0).round() / 100.0)
                on:change=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                        sig.set(v.clamp(min, max));
                    }
                }
            />
        }
    };

    view! {
        <div class="sensor-compare">
            <h3>{move || locale.get().ui.sensor_compare.title.clone()}</h3>
            <p class="area-hint">{move || locale.get().ui.sensor_compare.intro.clone()}</p>

            <div class="isolux-controls">
                <div class="control-group">
                    <span class="sensor-compare-tag sensor-compare-a">"A"</span>
                    <span>{move || locale.get().ui.sensor_compare.sensor_a.clone()}</span>
                    {picker(a_name, name_a)}
                </div>
                <button class="btn btn-sm btn-outline"
                    title=move || locale.get().ui.sensor_compare.swap.clone()
                    on:click=move |_| {
                        let (a, b) = (name_a.get_untracked(), name_b.get_untracked());
                        a_name.set(b);
                        b_name.set(a);
                    }
                >"⇄"</button>
                <div class="control-group">
                    <span class="sensor-compare-tag sensor-compare-b">"B"</span>
                    <span>{move || locale.get().ui.sensor_compare.sensor_b.clone()}</span>
                    {picker(b_name, name_b)}
                </div>
                <button class="btn btn-sm btn-outline" on:click=move |_| add_demo(files)>
                    {move || locale.get().ui.sensor.demo.clone()}
                </button>
            </div>

            // Drop zone: files go through the app's normal open path
            // (sensor or luminaire is recognised) and join the project.
            <div class=move || if drag.get() { "sensor-compare-drop active" } else { "sensor-compare-drop" }
                on:dragover=move |ev: leptos::ev::DragEvent| { ev.prevent_default(); drag.set(true); }
                on:dragleave=move |_| drag.set(false)
                on:drop=move |ev: leptos::ev::DragEvent| {
                    ev.prevent_default();
                    drag.set(false);
                    if let (Some(open), Some(list)) = (open_files, ev.data_transfer().and_then(|d| d.files())) {
                        open.open((0..list.length()).filter_map(|i| list.get(i)).collect());
                    }
                }
            >
                <span>{move || locale.get().ui.sensor_compare.drop_hint.clone()}</span>
                " "
                <label class="btn btn-sm btn-primary">
                    {move || locale.get().ui.sensor_compare.open_files.clone()}
                    <input type="file" multiple accept=".ldt,.xml,.gldf,.json" style="display:none"
                        on:change=move |ev| {
                            let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
                            if let (Some(open), Some(list)) = (open_files, input.files()) {
                                open.open((0..list.length()).filter_map(|i| list.get(i)).collect());
                            }
                            input.set_value("");
                        }
                    />
                </label>
            </div>
            {move || (files.with(|v| v.len()) < 2).then(|| view! {
                <p class="area-hint">{move || locale.get().ui.sensor_compare.need_two.clone()}</p>
            })}

            {move || has_cmp.get().then(|| view! {
                // Conditions
                <div class="isolux-controls">
                    <div class="control-group">
                        <span>{move || locale.get().ui.sensor_compare.mount.clone()}</span>
                        <input type="range" min="1" max="15" step="0.1"
                            prop:value=move || mount.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    mount.set(v);
                                }
                            }
                        />
                        <span class="control-value">{move || format!("{:.1} m", mount.get())}</span>
                    </div>
                    <div class="control-group">
                        <span>{move || locale.get().ui.sensor_compare.workplane.clone()}</span>
                        {num_input(workplane, 0.0, 3.0, 0.05)}
                        <span>"m"</span>
                    </div>
                    {move || any_sensxml.get().then(|| view! {
                        <label class="control-group">
                            <input type="checkbox"
                                prop:checked=move || planning.get()
                                on:change=move |ev| planning.set(event_target_checked(&ev))
                            />
                            <span>{move || locale.get().ui.sensor.planning.clone()}</span>
                        </label>
                    })}
                </div>

                // Views
                <div class="compare-diagram-toggle">
                    {CmpView::ALL.into_iter().map(|v| view! {
                        <button class=move || if view.get() == v { "active" } else { "" }
                            on:click=move |_| view.set(v)
                        >{move || {
                            let l = locale.get();
                            let s = &l.ui.sensor_compare;
                            match v {
                                CmpView::Plan => s.view_plan.clone(),
                                CmpView::Polar => s.view_polar.clone(),
                                CmpView::Cartesian => s.view_cartesian.clone(),
                                CmpView::Height => s.view_height.clone(),
                                CmpView::Room => s.view_room.clone(),
                            }
                        }}</button>
                    }).collect_view()}
                </div>

                // View controls (do not read the C-plane: a drag stays alive)
                {move || {
                    let v = view.get();
                    view! {
                        {v.uses_c_plane().then(|| view! {
                            <div class="isolux-controls">
                                <div class="control-group">
                                    <span>{move || locale.get().diagram.axis.c_plane.clone()}</span>
                                    <input type="range" min="0" max="355" step="5"
                                        prop:value=move || c_plane.get().to_string()
                                        on:input=move |ev| {
                                            if let Ok(c) = event_target_value(&ev).parse::<f64>() {
                                                c_plane.set(c);
                                            }
                                        }
                                    />
                                    <span class="control-value">{move || format!("C {:.0}°", c_plane.get())}</span>
                                </div>
                                {matches!(v, CmpView::Polar | CmpView::Cartesian).then(|| view! {
                                    <div class="control-group">
                                        <span>{move || locale.get().ui.sensor_compare.motion.clone()}</span>
                                        {move || shared_motions.get().into_iter().map(|m| view! {
                                            <button
                                                class=move || format!("btn btn-sm {}", if motion.get() == m { "btn-primary" } else { "btn-outline" })
                                                on:click=move |_| motion.set(m)
                                            >{move || motion_label(&locale.get().ui.sensor, m)}</button>
                                        }).collect_view()}
                                    </div>
                                })}
                            </div>
                        })}
                        {(v == CmpView::Height).then(|| view! {
                            <div class="isolux-controls">
                                <div class="control-group">
                                    <span>{move || locale.get().ui.sensor_compare.heights.clone()}</span>
                                    {num_input(h_min, 0.5, 30.0, 0.5)}
                                    <span>"–"</span>
                                    {num_input(h_max, 1.0, 40.0, 0.5)}
                                    <span>"m"</span>
                                </div>
                            </div>
                        })}
                        {(v == CmpView::Room).then(|| view! {
                            <div class="isolux-controls">
                                <div class="control-group">
                                    <span>{move || locale.get().ui.sensor_compare.room_length.clone()}</span>
                                    {num_input(room_l, 1.0, 200.0, 0.5)}
                                    <span>{move || locale.get().ui.sensor_compare.room_width.clone()}</span>
                                    {num_input(room_w, 1.0, 200.0, 0.5)}
                                    <span>"m"</span>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().ui.sensor_compare.plan_for.clone()}</span>
                                    <select on:change=move |ev| {
                                        let v = event_target_value(&ev);
                                        if let Some(m) = Motion::ALL.into_iter().find(|m| format!("{m:?}") == v) {
                                            room_motion.set(m);
                                        }
                                    }>
                                        {Motion::ALL.into_iter().map(|m| view! {
                                            <option value={format!("{m:?}")} selected=move || room_motion.get() == m>
                                                {move || motion_label(&locale.get().ui.sensor, m)}
                                            </option>
                                        }).collect_view()}
                                    </select>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().zonal_designer.sensors.control.clone()}</span>
                                    <select on:change=move |ev| {
                                        if let Some(c) = OccupancyControl::from_key(&event_target_value(&ev)) {
                                            control.set(c);
                                        }
                                    }>
                                        {OccupancyControl::ALL.into_iter().enumerate().map(|(i, c)| view! {
                                            <option value={c.key()} selected=move || control.get() == c>
                                                {move || locale.get().zonal_designer.sensors.controls.get(i).cloned().unwrap_or_else(|| c.label().to_string())}
                                            </option>
                                        }).collect_view()}
                                    </select>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().zonal_designer.sensors.space.clone()}</span>
                                    <select on:change=move |ev| {
                                        if let Some(s) = SpaceType::from_key(&event_target_value(&ev)) {
                                            space.set(s);
                                        }
                                    }>
                                        {SpaceType::ALL.into_iter().enumerate().map(|(i, s)| view! {
                                            <option value={s.key()} selected=move || space.get() == s>
                                                {move || locale.get().zonal_designer.sensors.spaces.get(i).cloned().unwrap_or_else(|| s.label().to_string())}
                                            </option>
                                        }).collect_view()}
                                    </select>
                                </div>
                                <div class="control-group">
                                    <span>{move || locale.get().ui.sensor_compare.price.clone()}</span>
                                    <input type="number" min="0" step="1" style="width:6em"
                                        prop:value=move || price.get().map(|p| format!("{p}")).unwrap_or_default()
                                        on:change=move |ev| {
                                            price.set(event_target_value(&ev).parse::<f64>().ok().filter(|p| *p >= 0.0));
                                        }
                                    />
                                </div>
                            </div>
                        })}
                    }
                }}

                // Diagram or room table
                {move || if view.get() == CmpView::Room {
                    room_table(locale, room_checks, price, name_a, name_b).into_any()
                } else {
                    view! { <div class="compare-overlay-diagram sensor-compare-diagram" inner_html=diagram_svg></div> }.into_any()
                }}
                <p class="area-hint">{move || {
                    let l = locale.get();
                    let s = &l.ui.sensor_compare;
                    match view.get() {
                        CmpView::Plan => s.plan_note.clone(),
                        CmpView::Polar | CmpView::Cartesian => s.angular_note.clone(),
                        CmpView::Height => s.height_note.clone(),
                        CmpView::Room => s.room_note.clone(),
                    }
                }}</p>

                // Numbers
                {move || numbers_table(locale, cmp)}

                <div class="compare-export-buttons">
                    <button class="compare-export-btn" prop:disabled=move || pdf_busy.get() on:click=on_report>
                        {move || if pdf_busy.get() { "…".to_string() } else { format!("📄 {}", locale.get().ui.sensor_compare.report) }}
                    </button>
                    <button class="compare-export-btn secondary" prop:disabled=move || link_hash.with(|h| h.is_none()) on:click=on_share>
                        {move || if copied.get() { locale.get().ui.sensor_compare.copied.clone() } else { format!("🔗 {}", locale.get().ui.sensor_compare.share) }}
                    </button>
                </div>
                {move || link_hash.with(|h| h.is_none()).then(|| view! {
                    <p class="area-hint">{move || locale.get().ui.sensor_compare.link_note.clone()}</p>
                })}
                <p class="area-hint sensor-compare-method">{move || locale.get().ui.sensor_compare.method.clone()}</p>
            })}
        </div>
    }
}

/// A | B | Δ per motion type for range and area.
fn numbers_table(
    locale: Memo<eulumdat_i18n::Locale>,
    cmp: Memo<Option<SensorComparison>>,
) -> Option<impl IntoView> {
    let c = cmp.get()?;
    let l = locale.get();
    let s = l.ui.sensor_compare.clone();
    let ls = l.ui.sensor.clone();
    let cell = |v: Option<SideValues>, f: fn(&SideValues) -> f64, unit: &str| match v {
        Some(x) if x.detected => format!("{:.2} {unit}{}", f(&x), quality_mark(x.quality)),
        Some(_) => s.not_detected.clone(),
        None => s.no_data.clone(),
    };
    let rows = c
        .rows
        .iter()
        .map(|r| {
            let class = match r.area_significance() {
                Some(Significance::Negligible) => "delta-ok",
                Some(Significance::Minor) => "delta-minor",
                Some(Significance::Moderate) => "delta-moderate",
                Some(Significance::Major) => "delta-major",
                None => "",
            };
            let pct = |d: Option<f64>| {
                d.map(|d| format!("{d:+.0} %"))
                    .unwrap_or_else(|| "–".into())
            };
            view! {
                <tr class=class>
                    <td class="metric-name">{motion_label(&ls, r.motion)}</td>
                    <td class="metric-value">{format!("{:.2} m", r.plane)}</td>
                    <td class="metric-value">{cell(r.a, |x| x.max_range, "m")}</td>
                    <td class="metric-value">{cell(r.b, |x| x.max_range, "m")}</td>
                    <td class="metric-percent">{pct(r.delta_pct(|x| x.max_range))}</td>
                    <td class="metric-value">{cell(r.a, |x| x.area_m2, "m²")}</td>
                    <td class="metric-value">{cell(r.b, |x| x.area_m2, "m²")}</td>
                    <td class="metric-percent">{pct(r.delta_pct(|x| x.area_m2))}</td>
                </tr>
            }
        })
        .collect_view();
    Some(view! {
        <div class="compare-table-wrapper">
            <table class="compare-table">
                <thead><tr>
                    <th>{s.col_motion.clone()}</th>
                    <th>{s.col_plane.clone()}</th>
                    <th>{format!("A · {}", s.col_range)}</th>
                    <th>{format!("B · {}", s.col_range)}</th>
                    <th>{s.delta.clone()}</th>
                    <th>{format!("A · {}", s.col_area)}</th>
                    <th>{format!("B · {}", s.col_area)}</th>
                    <th>{s.delta.clone()}</th>
                </tr></thead>
                <tbody>{rows}</tbody>
            </table>
            <p class="area-hint">{s.quality_note.clone()}</p>
        </div>
    })
}

/// Sensors needed for the room, A next to B.
fn room_table(
    locale: Memo<eulumdat_i18n::Locale>,
    checks: Memo<Option<(Option<RoomCheck>, Option<RoomCheck>)>>,
    price: RwSignal<Option<f64>>,
    name_a: Memo<Option<String>>,
    name_b: Memo<Option<String>>,
) -> impl IntoView {
    move || {
        let (ra, rb) = checks.get()?;
        let l = locale.get();
        let s = l.ui.sensor_compare.clone();
        let p = price.get();
        let col = |r: &Option<RoomCheck>| -> Vec<String> {
            match r {
                None => vec![s.not_detected.clone(); 6],
                Some(r) => vec![
                    format!(
                        "{} ({} × {}{}){}",
                        r.count,
                        r.nx,
                        r.ny,
                        if r.rotation != 0.0 {
                            format!(", {}", s.turned)
                        } else {
                            String::new()
                        },
                        if r.reached {
                            String::new()
                        } else {
                            format!(" · {}", s.not_reached)
                        }
                    ),
                    format!("{:.1} %", r.covered * 100.0),
                    format!("{:.1} %", r.overlap * 100.0),
                    format!("{:.1} m²", r.m2_per_sensor),
                    r.fo.map(|f| format!("{:.2}", f.f_o)).unwrap_or_default(),
                    p.map(|p| format!("{:.0}", p * r.count as f64))
                        .unwrap_or_else(|| "–".into()),
                ],
            }
        };
        let (ca, cb) = (col(&ra), col(&rb));
        let labels = [
            s.row_sensors.clone(),
            s.row_covered.clone(),
            s.row_overlap.clone(),
            s.row_m2.clone(),
            s.row_fo.clone(),
            s.row_cost.clone(),
        ];
        // fewer sensors = the better row for B / A
        let better = match (&ra, &rb) {
            (Some(a), Some(b)) if a.count != b.count => Some(b.count < a.count),
            _ => None,
        };
        Some(view! {
            <div class="compare-table-wrapper">
                <table class="compare-table sensor-room-table">
                    <thead><tr>
                        <th></th>
                        <th>{format!("A · {}", name_a.get().unwrap_or_default())}</th>
                        <th>{format!("B · {}", name_b.get().unwrap_or_default())}</th>
                    </tr></thead>
                    <tbody>
                        {labels.into_iter().enumerate().map(|(i, label)| {
                            let (ka, kb) = match (i, better) {
                                (0, Some(true)) => ("metric-value", "metric-value sensor-room-better"),
                                (0, Some(false)) => ("metric-value sensor-room-better", "metric-value"),
                                _ => ("metric-value", "metric-value"),
                            };
                            view! {
                                <tr>
                                    <td class="metric-name">{label}</td>
                                    <td class=ka>{ca[i].clone()}</td>
                                    <td class=kb>{cb[i].clone()}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        })
    }
}
