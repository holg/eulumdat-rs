//! Interior designer: a room from an IFC building model (`eulumdat-ifc`,
//! read with bimifc in the browser). The user opens a file, picks a storey
//! and a room, and takes over its outline and height, the luminaires in it
//! (positions, mounting height, their photometry) and the exits.
//!
//! The model itself is not kept: what the designer takes goes into its
//! own state and share link (`l`, `w`, `h`, `shape`, `sus`, `lfix`, `emp`).

use eulumdat_ifc::{Building, RoomPlan};
use leptos::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use crate::i18n::use_locale;

/// What the designer takes from the model.
#[derive(Clone)]
pub struct IfcRoomChoice {
    pub plan: RoomPlan,
    /// Place the luminaires where the model has them.
    pub luminaires: bool,
    /// Add the exits as emergency points.
    pub exits: bool,
    /// Open this photometry as the luminaire: (file name, LDT text).
    pub photometry: Option<(String, String)>,
}

fn fill(t: &str, pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .fold(t.to_string(), |acc, (k, v)| acc.replace(k, v))
}

/// The most used photometry among a room's luminaires.
fn main_photometry(plan: &RoomPlan) -> Option<usize> {
    let mut counts: Vec<(usize, usize)> = Vec::new();
    for l in plan.luminaires.iter().filter(|l| !l.emergency) {
        if let Some(p) = l.photometry {
            match counts.iter_mut().find(|c| c.0 == p) {
                Some(c) => c.1 += 1,
                None => counts.push((p, 1)),
            }
        }
    }
    counts.into_iter().max_by_key(|c| c.1).map(|c| c.0)
}

#[component]
pub fn IfcImport(on_apply: Callback<IfcRoomChoice>) -> impl IntoView {
    let locale = use_locale();
    let t = move || locale.get().zonal_designer.ifc.clone();
    let building = RwSignal::new(None::<Arc<Building>>);
    let file_name = RwSignal::new(String::new());
    let status = RwSignal::new(None::<String>);
    let storey = RwSignal::new(None::<usize>);
    let room = RwSignal::new(None::<usize>);
    let use_lums = RwSignal::new(true);
    let use_exits = RwSignal::new(true);
    let use_phot = RwSignal::new(true);
    let applied = RwSignal::new(None::<String>);

    // Storeys that have rooms (None = rooms without a storey).
    let storeys = Memo::new(move |_| {
        building.with(|b| {
            let Some(b) = b else {
                return Vec::new();
            };
            let mut v: Vec<Option<usize>> = Vec::new();
            for r in &b.rooms {
                if !v.contains(&r.storey) {
                    v.push(r.storey);
                }
            }
            v
        })
    });
    let plan = Memo::new(move |_| {
        let i = room.get()?;
        building.with(|b| b.as_ref().filter(|b| i < b.rooms.len()).map(|b| b.plan(i)))
    });

    let pick_defaults = move |b: &Building| {
        // the first room with luminaires, else the first room
        let r = b.rooms.iter().position(|_| true).map(|first| {
            (0..b.rooms.len())
                .find(|&i| b.luminaires_in(i).next().is_some())
                .unwrap_or(first)
        });
        storey.set(r.map(|i| b.rooms[i].storey).unwrap_or(None));
        room.set(r);
    };

    let on_file = move |ev: leptos::ev::Event| {
        let input: web_sys::HtmlInputElement = event_target(&ev);
        let Some(file) = input.files().and_then(|f| f.get(0)) else {
            return;
        };
        let name = file.name();
        input.set_value("");
        file_name.set(name.clone());
        applied.set(None);
        status.set(Some(fill(&t().reading, &[("{name}", name)])));
        wasm_bindgen_futures::spawn_local(async move {
            let text = match gloo_file::futures::read_as_text(&file.into()).await {
                Ok(t) => t,
                Err(e) => {
                    status.set(Some(fill(&t().failed, &[("{error}", e.to_string())])));
                    return;
                }
            };
            // let the "Reading…" line paint before the parse blocks
            set_timeout(
                move || match Building::parse(&text) {
                    Ok(b) => {
                        pick_defaults(&b);
                        building.set(Some(Arc::new(b)));
                        status.set(None);
                    }
                    Err(e) => {
                        building.set(None);
                        status.set(Some(fill(&t().failed, &[("{error}", e)])));
                    }
                },
                Duration::from_millis(40),
            );
        });
    };

    let apply = move |_| {
        let Some(plan) = plan.get_untracked() else {
            return;
        };
        let photometry = if use_phot.get_untracked() {
            main_photometry(&plan).and_then(|p| {
                building.with_untracked(|b| {
                    b.as_ref().map(|b| {
                        let ph = &b.photometries[p];
                        let base = file_name.get_untracked();
                        let base = base.trim_end_matches(".ifc").trim_end_matches(".IFC");
                        (format!("{base} · {}.ldt", ph.name.trim()), ph.ldt.clone())
                    })
                })
            })
        } else {
            None
        };
        applied.set(Some(plan.name.clone()));
        on_apply.run(IfcRoomChoice {
            luminaires: use_lums.get_untracked() && plan.luminaires.iter().any(|l| !l.emergency),
            exits: use_exits.get_untracked() && plan.doors.iter().any(|d| d.exit),
            photometry,
            plan,
        });
    };

    view! {
        <div class="zonal-sidebar-section">
            <h4>{move || t().title}</h4>
            <p class="zonal-sensors-hint">{move || t().intro}</p>
            <label class="zonal-share-btn" style="display: inline-block; cursor: pointer;">
                {move || t().open}
                <input type="file" accept=".ifc,.ifcx" style="display: none;" on:change=on_file />
            </label>
            {move || status.get().map(|s| view! { <p class="zonal-sensors-hint">{s}</p> })}
            {move || building.get().map(|b| {
                let tt = t();
                let summary = fill(&tt.summary, &[
                    ("{rooms}", b.rooms.len().to_string()),
                    ("{storeys}", b.storeys.len().max(1).to_string()),
                    ("{luminaires}", b.luminaires.len().to_string()),
                    ("{doors}", b.doors.len().to_string()),
                    ("{windows}", b.windows.len().to_string()),
                ]);
                let b2 = b.clone();
                let b3 = b.clone();
                let b4 = b.clone();
                view! {
                    <p class="zonal-sensors-hint"><b>{file_name.get_untracked()}</b>{format!(" · {summary}")}</p>
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{tt.storey.clone()}</span>
                        <select on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let s = v.parse::<usize>().ok();
                            storey.set(s);
                            room.set(b2.rooms.iter().position(|r| r.storey == s));
                        }>
                            {move || storeys.get().into_iter().map(|s| {
                                let label = match s {
                                    Some(i) => b3.storeys[i].name.clone(),
                                    None => "–".to_string(),
                                };
                                let value = s.map(|i| i.to_string()).unwrap_or_default();
                                view! { <option value=value selected=move || storey.get() == s>{label}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                    <div class="zonal-prop-row">
                        <span class="zonal-prop-label">{tt.room.clone()}</span>
                        <select on:change=move |ev| room.set(event_target_value(&ev).parse().ok())>
                            {move || {
                                let s = storey.get();
                                b4.rooms.iter().enumerate().filter(|(_, r)| r.storey == s).map(|(i, r)| {
                                    let label = format!("{} · {:.1} m²", r.name, r.area);
                                    view! { <option value=i.to_string() selected=move || room.get() == Some(i)>{label}</option> }
                                }).collect_view()
                            }}
                        </select>
                    </div>
                    {(!b.warnings.is_empty()).then(|| view! {
                        <details class="zonal-sensors-hint">
                            <summary>{tt.notes.clone()}</summary>
                            {b.warnings.iter().map(|w| view! { <div>{w.clone()}</div> }).collect_view()}
                        </details>
                    })}
                }
            })}
            {move || plan.get().map(|p| {
                let tt = t();
                let with = p
                    .luminaires
                    .iter()
                    .filter(|l| !l.emergency && l.photometry.is_some())
                    .count();
                let exits = p.doors.iter().filter(|d| d.exit).count();
                let phot_name = main_photometry(&p).and_then(|i| {
                    building.with(|b| b.as_ref().map(|b| b.photometries[i].name.clone()))
                });
                let n_lums = p.luminaires.iter().filter(|l| !l.emergency).count();
                view! {
                    <p class="zonal-sensors-hint">
                        {fill(&tt.room_info, &[("{area}", format!("{:.1}", p.area)), ("{height}", format!("{:.2}", p.height))])}
                        <br/>
                        {fill(&tt.in_room, &[
                            ("{luminaires}", n_lums.to_string()),
                            ("{with}", with.to_string()),
                            ("{doors}", p.doors.len().to_string()),
                            ("{exits}", exits.to_string()),
                            ("{windows}", p.windows.len().to_string()),
                        ])}
                    </p>
                    {(n_lums > 0).then(|| view! {
                        <label class="zonal-toggle">
                            <input type="checkbox" prop:checked=move || use_lums.get()
                                on:change=move |ev| use_lums.set(event_target_checked(&ev)) />
                            {fill(&tt.use_luminaires, &[("{n}", n_lums.to_string())])}
                        </label>
                    })}
                    {match phot_name {
                        Some(name) => view! {
                            <label class="zonal-toggle">
                                <input type="checkbox" prop:checked=move || use_phot.get()
                                    on:change=move |ev| use_phot.set(event_target_checked(&ev)) />
                                {fill(&tt.use_photometry, &[("{name}", name)])}
                            </label>
                        }.into_any(),
                        None if n_lums > 0 => view! { <p class="zonal-sensors-hint">{tt.no_photometry.clone()}</p> }.into_any(),
                        None => ().into_any(),
                    }}
                    {(exits > 0).then(|| view! {
                        <label class="zonal-toggle">
                            <input type="checkbox" prop:checked=move || use_exits.get()
                                on:change=move |ev| use_exits.set(event_target_checked(&ev)) />
                            {fill(&tt.use_exits, &[("{n}", exits.to_string())])}
                        </label>
                    })}
                    <button class="zonal-share-btn" on:click=apply>{tt.apply.clone()}</button>
                }
            })}
            {move || applied.get().map(|n| view! {
                <p class="zonal-sensors-hint">{fill(&t().applied, &[("{name}", n)])}</p>
            })}
        </div>
    }
}
