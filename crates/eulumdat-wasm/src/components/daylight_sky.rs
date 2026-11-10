//! Daylight & Sky — location, date/time, sun + moon, and the sky's light.
//!
//! Pick a place and a local date/time; the sun and moon positions update live
//! (altitude/azimuth/phase, no trace needed), and four visuals show the sky:
//! a colour dome, a scene lit by that sky (road / room / plaza), the sun's path
//! for the day, and a 24-hour day→night timeline. Press Trace to run the Monte
//! Carlo engine and read the sky's illuminance + colour metrics.
//!
//! The luminaire-SPD side lives in the sibling **Spectrum Lab** tab
//! ([`crate::components::spectrum_demo`]); both share
//! [`crate::components::spectral_ui`].

use crate::components::location_picker::{GlobePicker, PickedPlace};
use crate::components::sky_viz::{self, SceneKind};
use crate::components::spectral_ui::*;
use crate::i18n::use_locale;
use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn DaylightSky(
    /// Accepted for signature parity with the other tabs; the sky trace does
    /// not use the editor LDT.
    #[prop(optional)]
    _ldc: Option<ReadSignal<Eulumdat>>,
) -> impl IntoView {
    let locale = use_locale();

    let (result, set_result) = signal::<Option<SpectralResult>>(None);
    let (running, set_running) = signal(false);
    let (photons, set_photons) = signal(200_000u64);
    let (road_luminance, set_road_luminance) = signal(1.0f64);

    // Location & datetime: a preset, the place picked on the globe (option
    // after the presets, remembered in this browser) or custom coordinates.
    let picked_idx = eulumdat_daylight::LOCATIONS.len();
    let custom_idx = picked_idx + 1;
    let picked = RwSignal::new(PickedPlace::load_last());
    let default_loc_idx = if picked.get_untracked().is_some() {
        picked_idx
    } else {
        eulumdat_daylight::LOCATIONS
            .iter()
            .position(|l| l.name == "Berlin")
            .unwrap_or(0)
    };
    let (loc_idx, set_loc_idx) = signal(default_loc_idx);
    let globe_open = RwSignal::new(false);
    let (month, set_month) = signal(6u32);
    let (day, set_day) = signal(21u32);
    let (hour, set_hour) = signal(13.0f64);
    let (scene_kind, set_scene_kind) = signal(SceneKind::Road);
    let (custom_lat, set_custom_lat) = signal(51.77f64);
    let (custom_lon, set_custom_lon) = signal(7.44f64);
    let (custom_utc, set_custom_utc) = signal(1.0f64);

    let resolve_location = move || -> eulumdat_daylight::NamedLocation {
        let i = loc_idx.get();
        if i < picked_idx {
            eulumdat_daylight::LOCATIONS[i]
        } else if let Some(p) = picked.get().filter(|_| i == picked_idx) {
            let (utc, dst) = p.clock();
            eulumdat_daylight::NamedLocation::with_dst("Globe", p.lat, p.lon, utc, dst)
        } else {
            eulumdat_daylight::NamedLocation::with_dst(
                "Custom",
                custom_lat.get().clamp(-90.0, 90.0),
                custom_lon.get().clamp(-180.0, 180.0),
                custom_utc.get().clamp(-12.0, 14.0),
                if custom_lat.get().abs() > 23.5 {
                    eulumdat_daylight::DstRule::Eu
                } else {
                    eulumdat_daylight::DstRule::None
                },
            )
        }
    };

    let sun = Memo::new(move |_| {
        let loc = resolve_location();
        let dt = eulumdat_daylight::LocalDateTime::new(2026, month.get(), day.get(), hour.get());
        (dt.solar_position(&loc), loc)
    });
    let moon = Memo::new(move |_| {
        let loc = resolve_location();
        let dt = eulumdat_daylight::LocalDateTime::new(2026, month.get(), day.get(), hour.get());
        dt.moon_position(&loc)
    });
    // the night the chosen time belongs to: from noon to noon (a morning
    // hour belongs to the night that began the day before)
    let night_start = move || {
        let dt = eulumdat_daylight::LocalDateTime::new(2026, month.get(), day.get(), 12.0);
        if hour.get() < 12.0 {
            dt.prev_day()
        } else {
            dt
        }
    };
    let night = Memo::new(move |_| {
        let loc = resolve_location();
        eulumdat_daylight::NightSummary::of(night_start(), &loc, 2.5, 10.0)
    });
    // the most moonlight of each of the next 30 nights
    let month_bars = Memo::new(move |_| {
        let loc = resolve_location();
        let mut d = night_start();
        (0..30)
            .map(|_| {
                let n = eulumdat_daylight::NightSummary::of(d, &loc, 2.5, 30.0);
                let bar = (
                    d,
                    sky_viz::NightBar {
                        max_lux: n.moon_max_lux,
                        illuminated: n.moon_at_midnight.illuminated_fraction,
                        waxing: n.moon_at_midnight.waxing,
                    },
                );
                d = d.next_day();
                bar
            })
            .collect::<Vec<_>>()
    });
    let pick_night = move |i: usize| {
        let Some(d) = month_bars.with_untracked(|b| b.get(i).map(|(d, _)| *d)) else {
            return;
        };
        // keep the clock: a morning hour shows the night before its date
        let d = if hour.get_untracked() < 12.0 {
            d.next_day()
        } else {
            d
        };
        set_month.set(d.month);
        set_day.set(d.day);
    };
    let current_scene = move || {
        let loc = resolve_location();
        let dt = eulumdat_daylight::LocalDateTime::new(2026, month.get(), day.get(), hour.get());
        (loc, dt)
    };

    let trace = move |_: web_sys::MouseEvent| {
        set_running.set(true);
        let n = photons.get_untracked();
        let road_l = road_luminance.get_untracked();
        let (sun_pos, _loc) = sun.get_untracked();
        let moon_pos = moon.get_untracked();
        wasm_bindgen_futures::spawn_local(async move {
            yield_to_browser().await;
            let out = run_sky_trace(&sun_pos, &moon_pos, n, road_l);
            set_result.set(Some(out));
            set_running.set(false);
        });
    };

    view! {
        <div style="width: 100%; padding: 20px; box-sizing: border-box; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;">
            <div style="max-width: 1000px; margin: 0 auto;">
                <div style="margin-bottom: 8px;">
                    <h2 style="margin: 0 0 4px; font-size: 1.3rem;">{move || format!("🌇 {}", locale.get().spectrum_lab.sky_tab_title)}</h2>
                    <p style="margin: 0; color: var(--text-muted, #8b949e); font-size: 0.9rem;">
                        {move || locale.get().spectrum_lab.sky_tab_intro.clone()}
                    </p>
                </div>

                // ── Location & datetime panel (live sun + moon readout) ──
                <div style="display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-end; padding: 16px; background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; margin-bottom: 12px;">
                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">{move || locale.get().spectrum_lab.location.clone()}</label>
                        <select
                            style="padding: 6px 10px; background: var(--background, #0d1117); color: inherit; border: 1px solid var(--border, #30363d); border-radius: 6px;"
                            prop:value=move || loc_idx.get().to_string()
                            on:change=move |ev| { if let Ok(i)=event_target_value(&ev).parse::<usize>() { set_loc_idx.set(i); } }
                        >
                            // `selected` per option: the select's value is set
                            // before its options exist, so it alone showed the
                            // first entry
                            {eulumdat_daylight::LOCATIONS.iter().enumerate().map(|(i, l)| {
                                view! { <option value=i.to_string() selected=move || loc_idx.get() == i>{l.name}</option> }
                            }).collect::<Vec<_>>()}
                            {move || picked.get().map(|p| view! {
                                <option value=picked_idx.to_string() selected=move || loc_idx.get() == picked_idx>{format!("📍 {}", p.label())}</option>
                            })}
                            <option value=custom_idx.to_string() selected=move || loc_idx.get() == custom_idx>{move || format!("✎ {}", locale.get().spectrum_lab.location_custom)}</option>
                        </select>
                    </div>
                    <div>
                        <button
                            style="padding: 6px 10px; background: var(--background, #0d1117); color: inherit; border: 1px solid var(--border, #30363d); border-radius: 6px; cursor: pointer;"
                            on:click=move |_| globe_open.set(true)
                        >{move || locale.get().spectrum_lab.globe_pick.clone()}</button>
                        <GlobePicker
                            open=globe_open
                            at=Signal::derive(move || {
                                let l = resolve_location();
                                (l.latitude_deg, l.longitude_deg)
                            })
                            on_pick=Callback::new(move |p: PickedPlace| {
                                picked.set(Some(p));
                                set_loc_idx.set(picked_idx);
                            })
                        />
                    </div>

                    {move || (loc_idx.get() == custom_idx).then(|| view! {
                        <div style="display: flex; gap: 8px; align-items: flex-end;">
                            <div>
                                <label style="display:block;font-size:0.68rem;color:var(--text-muted,#8b949e);text-transform:uppercase;margin-bottom:3px;">{move || locale.get().spectrum_lab.latitude.clone()}</label>
                                <input type="number" step="0.01" min="-90" max="90"
                                    style="width:76px;padding:5px 6px;background:var(--background,#0d1117);color:inherit;border:1px solid var(--border,#30363d);border-radius:5px;"
                                    prop:value=move || format!("{:.2}", custom_lat.get())
                                    on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse::<f64>() { set_custom_lat.set(v); } } />
                            </div>
                            <div>
                                <label style="display:block;font-size:0.68rem;color:var(--text-muted,#8b949e);text-transform:uppercase;margin-bottom:3px;">{move || locale.get().spectrum_lab.longitude.clone()}</label>
                                <input type="number" step="0.01" min="-180" max="180"
                                    style="width:82px;padding:5px 6px;background:var(--background,#0d1117);color:inherit;border:1px solid var(--border,#30363d);border-radius:5px;"
                                    prop:value=move || format!("{:.2}", custom_lon.get())
                                    on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse::<f64>() { set_custom_lon.set(v); } } />
                            </div>
                            <div>
                                <label style="display:block;font-size:0.68rem;color:var(--text-muted,#8b949e);text-transform:uppercase;margin-bottom:3px;">"UTC"</label>
                                <input type="number" step="0.5" min="-12" max="14"
                                    style="width:60px;padding:5px 6px;background:var(--background,#0d1117);color:inherit;border:1px solid var(--border,#30363d);border-radius:5px;"
                                    prop:value=move || format!("{:+.1}", custom_utc.get())
                                    on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse::<f64>() { set_custom_utc.set(v); } } />
                            </div>
                        </div>
                    })}

                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">{move || locale.get().spectrum_lab.date.clone()}</label>
                        <input type="date"
                            style="padding: 6px 10px; background: var(--background, #0d1117); color: inherit; border: 1px solid var(--border, #30363d); border-radius: 6px;"
                            prop:value=move || format!("2026-{:02}-{:02}", month.get(), day.get())
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                let parts: Vec<&str> = v.split('-').collect();
                                if parts.len() == 3 {
                                    if let (Ok(m), Ok(d)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                                        set_month.set(m.clamp(1, 12)); set_day.set(d.clamp(1, 31));
                                    }
                                }
                            } />
                    </div>

                    <div style="min-width: 200px;">
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">
                            {move || format!("{} — {}", locale.get().spectrum_lab.time_of_day, fmt_hour(hour.get()))}
                        </label>
                        <input type="range" min="0" max="23.75" step="0.25"
                            style="width: 100%; accent-color: #f0883e; vertical-align: middle;"
                            prop:value=move || hour.get().to_string()
                            on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse::<f64>() { set_hour.set(v); } } />
                    </div>

                    // Live sun + moon readout.
                    <div style="margin-left: auto; text-align: right; min-width: 190px;">
                        {move || {
                            let (s, loc) = sun.get();
                            let m = moon.get();
                            let sl = locale.get().spectrum_lab;
                            let day = s.is_daytime();
                            let (badge_bg, badge_txt, icon) = if day {
                                ("rgba(240,136,62,0.18)", "#f0883e", "☀️")
                            } else if s.altitude_deg() > -6.0 {
                                ("rgba(163,113,247,0.18)", "#a371f7", "🌆")
                            } else {
                                ("rgba(88,166,255,0.18)", "#58a6ff", "🌙")
                            };
                            let regime = if day && s.altitude_deg() > 6.0 { sl.regime_day.clone() }
                                else if s.altitude_deg() > -6.0 { sl.regime_twilight.clone() }
                                else { sl.regime_night.clone() };
                            let moon_line = (!day).then(|| {
                                let up = if m.is_up() { format!(" · {:.0}°", m.altitude_deg()) } else { format!(" · {}", sl.moon_below.clone()) };
                                format!("{} {} {:.0}%{}", m.phase_name().emoji(), moon_phase_label(&sl, m.phase_name()), m.illuminated_fraction * 100.0, up)
                            });
                            // the moon's light on the ground, in lux
                            let moon_lux = m.is_up().then(|| {
                                if day {
                                    sl.moonlight_day.clone()
                                } else {
                                    let e = eulumdat_daylight::Moonlight::of(&m, 2.5).global_horizontal_lux;
                                    sl.moonlight_now.replace("{lx}", &fmt_lux(e))
                                }
                            });
                            view! {
                                <div style=format!("display:inline-block; padding:2px 10px; border-radius:12px; background:{badge_bg}; color:{badge_txt}; font-size:0.8rem; font-weight:600; margin-bottom:4px;")>
                                    {format!("{icon} {regime}")}
                                </div>
                                <div style="font-size:0.82rem; color:var(--text-muted,#8b949e); line-height:1.5;">
                                    <div>{format!("{}: {:.1}°", sl.sun_altitude, s.altitude_deg())}</div>
                                    <div>{format!("{}: {:.0}°", sl.sun_azimuth, s.azimuth_deg())}</div>
                                    {moon_line.map(|ml| view! { <div style="color:#a9c4e8;">{ml}</div> })}
                                    {moon_lux.map(|ml| view! { <div style="color:#79c0ff; font-weight:600;">{ml}</div> })}
                                    <div style="opacity:0.7;">{format!("{:.4}°, {:.4}° · UTC{:+.1}", loc.latitude_deg, loc.longitude_deg, loc.utc_offset_hours)}</div>
                                </div>
                            }
                        }}
                    </div>
                </div>

                // ── Live sky visualisations ──
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 12px;">
                    <div style="background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; padding: 10px;">
                        <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">{move || locale.get().spectrum_lab.viz_sky_dome.clone()}</div>
                        <div inner_html=move || { let (loc, dt) = current_scene(); sky_viz::sky_dome_svg(&dt, &loc, 2.5, 400.0, 200.0) } />
                    </div>
                    <div style="background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; padding: 10px;">
                        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px;">
                            <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px;">{move || locale.get().spectrum_lab.viz_scene.clone()}</div>
                            // Scene picker: road / room / plaza.
                            <div style="display:flex; gap:3px;">
                                {[SceneKind::Road, SceneKind::Room, SceneKind::Plaza].iter().map(|&k| {
                                    view! {
                                        <button
                                            style=move || format!("padding:2px 8px;font-size:0.7rem;border-radius:5px;cursor:pointer;border:1px solid {};background:{};color:inherit;",
                                                if scene_kind.get()==k {"#f0883e"} else {"var(--border,#30363d)"},
                                                if scene_kind.get()==k {"rgba(240,136,62,0.15)"} else {"transparent"})
                                            on:click=move |_| set_scene_kind.set(k)
                                        >{move || scene_label(&locale.get().spectrum_lab, k)}</button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                        <div inner_html=move || { let (loc, dt) = current_scene(); sky_viz::scene_render_svg(&dt, &loc, scene_kind.get(), 2.5, 400.0, 200.0) } />
                    </div>
                    <div style="background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; padding: 10px;">
                        <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">{move || locale.get().spectrum_lab.viz_sun_path.clone()}</div>
                        <div inner_html=move || { let (loc, dt) = current_scene(); sky_viz::sun_path_svg(&dt, &loc, 400.0, 170.0) } />
                    </div>
                    <div style="background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; justify-content: center;">
                        <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">{move || locale.get().spectrum_lab.viz_timeline.clone()}</div>
                        <div inner_html=move || { let (loc, dt) = current_scene(); sky_viz::day_timeline_svg(&dt, &loc, 2.5, 600.0, 48.0) } />
                    </div>
                </div>

                // ── Moonlight tonight + the next 30 nights ──
                <div style="background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; padding: 12px; margin-bottom: 12px;">
                    <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;">{move || locale.get().spectrum_lab.moon_card_title.clone()}</div>
                    {move || {
                        let sl = locale.get().spectrum_lab;
                        let n = night.get();
                        let m = n.moon_at_midnight;
                        let ratio = (eulumdat_daylight::lunar::MEAN_DISTANCE_KM / m.distance_km).powi(2);
                        let dist = if ratio >= 1.0 {
                            sl.moon_dist_close.replace("{pct}", &format!("{:.0}", (ratio - 1.0) * 100.0))
                        } else {
                            sl.moon_dist_far.replace("{pct}", &format!("{:.0}", (1.0 - ratio) * 100.0))
                        };
                        let summary = sl.moon_summary
                            .replace("{phase}", &format!("{} {}", m.phase_name().emoji(), moon_phase_label(&sl, m.phase_name())))
                            .replace("{pct}", &format!("{:.0}", m.illuminated_fraction * 100.0))
                            .replace("{km}", &group_thousands(m.distance_km))
                            .replace("{dist}", &dist);
                        let clock = |h: Option<f64>| h.map(sky_viz::clock_after_noon).unwrap_or_else(|| "–".into());
                        let times = (n.moon_max_lux > 1e-4).then(|| sl.moon_times
                            .replace("{rise}", &clock(n.moonrise))
                            .replace("{set}", &clock(n.moonset))
                            .replace("{max}", &fmt_lux(n.moon_max_lux))
                            .replace("{at}", &clock(n.moon_max_at)));
                        let hours = if n.moon_max_lux > 1e-4 {
                            sl.moon_hours
                                .replace("{h2}", &format!("{:.1}", n.moon_hours_above(0.2)))
                                .replace("{h1}", &format!("{:.1}", n.moon_hours_above(0.1)))
                        } else {
                            sl.moon_dark.clone()
                        };
                        let labels = sky_viz::NightChartLabels { total: sl.moon_legend_total.clone(), moon: sl.moon_legend_moon.clone() };
                        let now = (hour.get() - 12.0).rem_euclid(24.0);
                        view! {
                            <div style="font-size:0.85rem; line-height:1.5; margin-bottom:6px;">
                                <div style="font-weight:600;">{summary}</div>
                                {times.map(|t| view! { <div>{t}</div> })}
                                <div style="color:#79c0ff;">{hours}</div>
                            </div>
                            <div inner_html=sky_viz::night_light_svg(&n, Some(now), &labels, 900.0, 230.0) />
                            <p style="margin:6px 0 10px; font-size:0.75rem; color:var(--text-muted,#8b949e);">{sl.moon_ref_note.clone()}</p>
                        }
                    }}
                    <div style="font-size: 0.72rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin: 4px 0 4px;">{move || locale.get().spectrum_lab.moon_month_title.clone()}</div>
                    <div
                        title=move || locale.get().spectrum_lab.moon_month_hint.clone()
                        on:click=move |ev| {
                            use wasm_bindgen::JsCast;
                            let hit = ev.target()
                                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                                .and_then(|e| e.closest("[data-night]").ok().flatten())
                                .and_then(|e| e.get_attribute("data-night"))
                                .and_then(|v| v.parse::<usize>().ok());
                            if let Some(i) = hit {
                                pick_night(i);
                            }
                        }
                        inner_html=move || month_bars.with(|bars| {
                            let nights: Vec<sky_viz::NightBar> = bars.iter().map(|(_, b)| sky_viz::NightBar { max_lux: b.max_lux, illuminated: b.illuminated, waxing: b.waxing }).collect();
                            let labels: Vec<String> = bars.iter().enumerate().map(|(i, (d, _))| {
                                if i == 0 || d.day == 1 || d.day % 5 == 0 { format!("{}.{}", d.day, d.month) } else { String::new() }
                            }).collect();
                            sky_viz::moon_month_svg(&nights, 0, &labels, 900.0, 92.0)
                        })
                    />
                    <div style="font-size:0.72rem; color:var(--text-muted,#8b949e); margin-top:2px;">{move || locale.get().spectrum_lab.moon_month_hint.clone()}</div>
                </div>

                // Controls row (photons + road luminance for the mesopic card + trace).
                <div style="display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-end; padding: 16px; background: var(--surface-elevated, #161b22); border: 1px solid var(--border, #30363d); border-radius: 8px; margin-bottom: 16px;">
                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">{move || locale.get().spectrum_lab.photons.clone()}</label>
                        <div style="display: flex; gap: 4px;">
                            {[(50_000u64, "50K"), (200_000, "200K"), (500_000, "500K"), (1_000_000, "1M")].iter().map(|&(count, label)| {
                                view! {
                                    <button
                                        style=move || format!("padding: 5px 10px; font-size: 0.8rem; border-radius: 6px; cursor: pointer; border: 1px solid {}; background: {}; color: inherit;",
                                            if photons.get() == count { "#58a6ff" } else { "var(--border, #30363d)" },
                                            if photons.get() == count { "rgba(88,166,255,0.15)" } else { "transparent" })
                                        on:click=move |_| set_photons.set(count)
                                    >{label}</button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                    <div>
                        <label style="display: block; font-size: 0.75rem; color: var(--text-muted, #8b949e); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;">{move || locale.get().spectrum_lab.road_luminance.clone()}</label>
                        <input type="range" min="0.1" max="3.0" step="0.1"
                            style="width: 140px; accent-color: #58a6ff; vertical-align: middle;"
                            prop:value=move || road_luminance.get().to_string()
                            on:input=move |ev| { if let Ok(v)=event_target_value(&ev).parse::<f64>() { set_road_luminance.set(v); } } />
                        <span style="margin-left: 8px; font-size: 0.85rem;">{move || format!("{:.1}", road_luminance.get())}</span>
                    </div>
                    <button
                        style="padding: 8px 20px; background: #238636; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 0.9rem; font-weight: 600; margin-left: auto;"
                        disabled=move || running.get()
                        on:click=trace
                    >
                        {move || { let sl = locale.get().spectrum_lab; if running.get() { sl.tracing } else { sl.trace } }}
                    </button>
                </div>

                // Results
                {move || {
                    let sl = locale.get().spectrum_lab;
                    match result.get() {
                        None => view! {
                            <div style="padding: 40px; text-align: center; color: var(--text-muted, #8b949e); border: 1px dashed var(--border, #30363d); border-radius: 8px;">
                                {sl.sky_empty_hint.clone()}
                            </div>
                        }.into_any(),
                        Some(r) => render_result(&r, &sl, road_luminance.get_untracked()).into_any(),
                    }
                }}
            </div>
        </div>
    }
}

/// Localised label for a scene kind.
fn scene_label(sl: &eulumdat_i18n::SpectrumLabLocale, k: SceneKind) -> String {
    match k {
        SceneKind::Road => sl.scene_road.clone(),
        SceneKind::Room => sl.scene_room.clone(),
        SceneKind::Plaza => sl.scene_plaza.clone(),
    }
}

/// Lux with as many decimals as small values need.
fn fmt_lux(e: f64) -> String {
    if e >= 10.0 {
        format!("{e:.0}")
    } else if e >= 1.0 {
        format!("{e:.1}")
    } else if e >= 0.01 {
        format!("{e:.2}")
    } else {
        format!("{e:.3}")
    }
}

/// 384400 → "384 400".
fn group_thousands(v: f64) -> String {
    let n = v.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}
