//! Natural light in the Outdoor designer: the date, the time and the place
//! (globe), and what the sun, the twilight sky or the moon adds to the
//! luminaires' illuminance on open ground (`eulumdat_scene::moon::SkyLight`,
//! from `eulumdat_daylight`). The luminaires' results stay their own; the
//! natural light is shown next to them, with how far the luminaires could
//! be dimmed then (not at all needed by day).

use super::location_picker::{GlobePicker, PickedPlace};
use crate::i18n::use_locale;
use eulumdat_scene::moon::{MoonTime, SkyKind, SkyLight};
use leptos::prelude::*;

/// The moon section's state.
#[derive(Clone, Copy)]
pub struct MoonState {
    pub on: RwSignal<bool>,
    /// "YYYY-MM-DD", local wall-clock hour.
    pub date: RwSignal<String>,
    pub hour: RwSignal<f64>,
    pub place: RwSignal<Option<PickedPlace>>,
}

impl MoonState {
    pub fn new() -> Self {
        let d = js_sys::Date::new_0();
        Self {
            on: RwSignal::new(false),
            date: RwSignal::new(format!(
                "{:04}-{:02}-{:02}",
                d.get_full_year(),
                d.get_month() + 1,
                d.get_date()
            )),
            hour: RwSignal::new(23.0),
            place: RwSignal::new(PickedPlace::load_last()),
        }
    }

    /// When and where (tracked); Berlin when no place was picked.
    pub fn time(&self) -> MoonTime {
        let (lat, lon, utc, dst) = match self.place.get() {
            Some(p) => {
                let (utc, dst) = p.clock();
                let key = match dst {
                    eulumdat_daylight::DstRule::Eu => "eu",
                    eulumdat_daylight::DstRule::Us => "us",
                    eulumdat_daylight::DstRule::SouthernEu => "south",
                    eulumdat_daylight::DstRule::None => "none",
                };
                (p.lat, p.lon, utc as f32, key)
            }
            None => (52.52, 13.40, 1.0, "eu"),
        };
        MoonTime {
            date: self.date.get(),
            hour: self.hour.get() as f32,
            latitude: lat,
            longitude: lon,
            utc_offset: utc,
            dst: dst.into(),
            north_deg: 0.0,
        }
    }

    /// The natural light when the section is on (tracked).
    pub fn sky(&self) -> Option<SkyLight> {
        self.on.get().then(|| self.time().sky()).flatten()
    }
}

impl Default for MoonState {
    fn default() -> Self {
        Self::new()
    }
}

/// Lux as people read them: 112 000, 75, 2.4, 0.25, 0.002.
pub fn lx(v: f64) -> String {
    if v >= 1000.0 {
        let n = v.round() as u64;
        let t = n.to_string();
        let mut out = String::new();
        for (i, c) in t.chars().enumerate() {
            if i > 0 && (t.len() - i).is_multiple_of(3) {
                out.push('\u{202f}');
            }
            out.push(c);
        }
        return out;
    }
    if v >= 100.0 {
        return format!("{v:.0}");
    }
    if v >= 1.0 {
        format!("{v:.1}")
    } else if v >= 0.01 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

fn phase_icon(f: f32, waxing: bool) -> &'static str {
    match (f, waxing) {
        (f, _) if f < 0.04 => "🌑",
        (f, _) if f > 0.96 => "🌕",
        (f, true) if f < 0.4 => "🌒",
        (f, true) if f < 0.6 => "🌓",
        (_, true) => "🌔",
        (f, false) if f < 0.4 => "🌘",
        (f, false) if f < 0.6 => "🌗",
        _ => "🌖",
    }
}

/// The sidebar section: on / off, night, time, place, what the moon gives.
#[component]
pub fn MoonSidebar(state: MoonState) -> impl IntoView {
    let locale = use_locale();
    let t = move || locale.get().ui.light_scene.clone();
    let globe = RwSignal::new(false);
    view! {
        <div class="zonal-sidebar-section">
            <h4>{move || t().moon_title}</h4>
            <label class="zonal-toggle">
                <input type="checkbox" prop:checked=move || state.on.get()
                    on:change=move |ev| state.on.set(event_target_checked(&ev)) />
                {move || format!(" {}", t().moon_on)}
            </label>
            <p class="zonal-sensors-hint">{move || locale.get().area_designer.moon.desc.clone()}</p>
            {move || state.on.get().then(|| view! {
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || t().moon_date}</span>
                    <input type="date" prop:value=move || state.date.get()
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            if eulumdat_scene::moon::parse_date(&v).is_some() {
                                state.date.set(v);
                            }
                        } />
                </div>
                <button class="zonal-move-btn" on:click=move |_| {
                    if let Some(d) = eulumdat_scene::moon::next_full_moon(&state.time()) {
                        state.date.set(d);
                    }
                }>{move || t().moon_full}</button>
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || format!("{} · {:02}:{:02}", t().moon_time, state.hour.get() as u32, ((state.hour.get().fract()) * 60.0).round() as u32)}</span>
                    <input type="range" min="0" max="23.75" step="0.25" prop:value=move || state.hour.get().to_string()
                        on:input=move |ev| { if let Ok(v) = event_target_value(&ev).parse::<f64>() { state.hour.set(v); } } />
                </div>
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || t().moon_place.replace("{place}", &state.place.get().map(|p| p.label()).unwrap_or_else(|| "Berlin".into()))}</span>
                    <button class="zonal-move-btn" on:click=move |_| globe.set(true)>{move || t().moon_pick}</button>
                </div>
                <GlobePicker open=globe
                    at=Signal::derive(move || { let mt = state.time(); (mt.latitude, mt.longitude) })
                    on_pick=Callback::new(move |p: PickedPlace| state.place.set(Some(p))) />
                <p class="zonal-sensors-hint" style="color: #79c0ff;">{move || {
                    let tt = t();
                    match state.sky() {
                        Some(sk) if sk.kind == SkyKind::Sun => tt.sun_readout
                            .replace("{alt}", &format!("{:.0}", sk.sun_altitude_deg))
                            .replace("{lx}", &lx(f64::from(sk.horizontal_lux)))
                            .replace("{beam}", &lx(f64::from(sk.beam_lux)))
                            .replace("{sky}", &lx(f64::from(sk.sky_lux))),
                        Some(sk) if sk.kind == SkyKind::Twilight => tt.twilight_readout
                            .replace("{alt}", &format!("{:.0}", sk.sun_altitude_deg))
                            .replace("{lx}", &lx(f64::from(sk.horizontal_lux))),
                        Some(sk) if sk.moon.altitude_deg > 0.0 => {
                            let m = sk.moon;
                            tt.moon_readout
                                .replace("{phase}", phase_icon(m.illuminated, m.waxing))
                                .replace("{pct}", &format!("{:.0}", m.illuminated * 100.0))
                                .replace("{alt}", &format!("{:.0}", m.altitude_deg))
                                .replace("{lx}", &lx(f64::from(m.horizontal_lux)))
                                .replace("{beam}", &lx(f64::from(m.normal_lux)))
                                .replace("{sky}", &lx(f64::from(m.sky_lux)))
                        }
                        Some(sk) => format!("{} {}", tt.moon_below, tt.night_readout.replace("{lx}", &lx(f64::from(sk.horizontal_lux)))),
                        None => String::new(),
                    }
                }}</p>
            })}
        </div>
    }
}

/// What the moon adds to the luminaires' minimum / average, and how far
/// they could be dimmed on that night for the `target` minimum (lx).
pub fn moon_results(min_lux: f64, avg_lux: f64, target: f64, moon: f64) -> MoonNumbers {
    let dim = if min_lux > 0.0 && min_lux >= target {
        Some(((target - moon) / min_lux).clamp(0.0, 1.0))
    } else {
        None
    };
    MoonNumbers {
        min_with: min_lux + moon,
        avg_with: avg_lux + moon,
        dim,
        alone_share: if target > 0.0 { moon / target } else { 0.0 },
    }
}

/// [`moon_results`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoonNumbers {
    pub min_with: f64,
    pub avg_with: f64,
    /// Output (0–1) that keeps the target minimum with the moon; `None`
    /// when the luminaires alone do not reach it.
    pub dim: Option<f64>,
    /// The moon as a share of the target.
    pub alone_share: f64,
}

/// The results block under the luminaires' statistics.
#[component]
pub fn MoonResults(
    state: MoonState,
    /// Luminaires' min and average (lx), the optimizer's target minimum.
    stats: Signal<(f64, f64)>,
    target: Signal<f64>,
) -> impl IntoView {
    let locale = use_locale();
    move || {
        let sk = state.sky().filter(|s| s.horizontal_lux > 0.0)?;
        let (min, avg) = stats.get();
        let target = target.get();
        let moon = f64::from(sk.horizontal_lux);
        let n = moon_results(min, avg, target, moon);
        let s = locale.get().area_designer.moon.clone();
        let d = locale.get().designer.clone();
        let tgt = format!("{target:.0}");
        Some(view! {
            <div class="area-stats" style="margin-top: 6px;">
                <div style="font-weight: 600; color: #79c0ff;">{s.with_moon.replace("{lx}", &lx(moon))}</div>
                <table class="area-stats-table">
                    <tr>
                        <td>{d.min.clone()}</td><td class="val">{format!("{} lx", lx(n.min_with))}</td>
                        <td>{d.avg.clone()}</td><td class="val">{format!("{} lx", lx(n.avg_with))}</td>
                    </tr>
                </table>
                <p class="zonal-sensors-hint">{match n.dim {
                    // by day (or in a bright moon over a low target)
                    _ if moon >= target => s.not_needed.replace("{lx}", &lx(moon)).replace("{target}", &tgt),
                    Some(k) => s.dim.replace("{target}", &tgt).replace("{pct}", &format!("{:.0}", k * 100.0)),
                    None => s.dim_none.replace("{target}", &tgt).replace("{lx}", &lx(moon)),
                }}</p>
                {(moon < target).then(|| view! {
                    <p class="zonal-sensors-hint">{s.alone.replace("{target}", &tgt).replace("{pct}", &format!("{:.1}", n.alone_share * 100.0))}</p>
                })}
            </div>
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_moon_lets_a_path_be_dimmed() {
        // a park path for 2 lx minimum: the luminaires give 2.5 lx, the moon 0.3
        let n = moon_results(2.5, 6.0, 2.0, 0.3);
        assert!((n.min_with - 2.8).abs() < 1e-12);
        assert!((n.dim.unwrap() - 0.68).abs() < 1e-12, "{:?}", n.dim);
        assert!((n.alone_share - 0.15).abs() < 1e-12);
        // a road for 20 lx: next to nothing to gain
        let road = moon_results(25.0, 40.0, 20.0, 0.3);
        assert!(road.dim.unwrap() > 0.78);
        // the luminaires alone below the target: no dimming
        assert_eq!(moon_results(1.5, 3.0, 2.0, 0.3).dim, None);
        // a bright moon covers a 0.2 lx minimum by itself
        assert_eq!(moon_results(1.0, 2.0, 0.2, 0.3).dim, Some(0.0));
    }
}
