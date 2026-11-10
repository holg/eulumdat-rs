//! The site map of the Outdoor designer: OpenStreetMap tiles under the plan
//! (`eulumdat::area::sitemap`), around a site found on the geodb-rs globe
//! (street mode: close in, with its OpenStreetMap tiles; the exact spot in
//! the middle of the screen comes back) or typed in. North is up; the site
//! is the middle of the area. Replaces the Google Maps designer.
//!
//! Link keys: `map=1`, `site=lat,lon`, `mapop=0.85`, `mapstyle=hot` (the
//! tile style, `eulumdat::area::sitemap::STYLES`). The last site is kept
//! in localStorage `eulumdat_site`, the style in `eulumdat_site_style`; "Maps" (dashboard tile, `#tab=maps`)
//! opens the Outdoor designer with the map on ([`request_site_map`]).

use super::location_picker::{GlobePicker, PickedPlace};
use crate::i18n::use_locale;
use eulumdat::area::sitemap::STYLES;
use eulumdat::area::{SiteMap, TileStyle};
use leptos::prelude::*;

const SITE_KEY: &str = "eulumdat_site";
const STYLE_KEY: &str = "eulumdat_site_style";
const REQUEST_KEY: &str = "eulumdat_sitemap_request";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Ask the Outdoor designer to show its site map when it opens next.
pub fn request_site_map() {
    if let Some(s) = storage() {
        let _ = s.set_item(REQUEST_KEY, "1");
    }
}

/// Whether the site map was asked for (the request is used up).
fn take_request() -> bool {
    let Some(s) = storage() else {
        return false;
    };
    let asked = s.get_item(REQUEST_KEY).ok().flatten().is_some();
    let _ = s.remove_item(REQUEST_KEY);
    asked
}

/// A map drag: where it started on the plan (m) and the site then.
type MapDrag = ((f64, f64), (f64, f64));

/// The site map's state.
#[derive(Clone, Copy)]
pub struct SiteMapState {
    pub on: RwSignal<bool>,
    /// Latitude, longitude of the site (the middle of the area).
    pub site: RwSignal<Option<(f64, f64)>>,
    /// The place the globe named, if the site came from there.
    pub label: RwSignal<Option<String>>,
    pub opacity: RwSignal<f64>,
    /// Tile style key ([`STYLES`]).
    pub style: RwSignal<String>,
    /// Dragging on the plan moves the map (not the poles).
    pub moving: RwSignal<bool>,
    /// A map drag in progress: start (plan m) and the site then.
    pub drag: RwSignal<Option<MapDrag>>,
    pub globe: RwSignal<bool>,
}

impl SiteMapState {
    /// From the link (`get` reads a key), else the last site of this
    /// browser; on when the link says so or "Maps" asked for it.
    pub fn from_link(get: impl Fn(&str) -> Option<String>) -> Self {
        let link_site = get("site").and_then(|v| {
            let (a, b) = v.split_once(',')?;
            let (lat, lon): (f64, f64) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
            ((-85.0..=85.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
        });
        let stored: Option<(f64, f64, String)> = storage()
            .and_then(|s| s.get_item(SITE_KEY).ok().flatten())
            .and_then(|t| serde_json::from_str(&t).ok());
        let on = get("map").is_some_and(|v| v == "1") || take_request();
        let site = link_site.or(stored.as_ref().map(|s| (s.0, s.1)));
        Self {
            on: RwSignal::new(on),
            site: RwSignal::new(site),
            label: RwSignal::new(if link_site.is_some() {
                None
            } else {
                stored.map(|s| s.2).filter(|l| !l.is_empty())
            }),
            opacity: RwSignal::new(
                get("mapop")
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.85)
                    .clamp(0.2, 1.0),
            ),
            style: RwSignal::new(
                get("mapstyle")
                    .or_else(|| storage().and_then(|s| s.get_item(STYLE_KEY).ok().flatten()))
                    .map(|k| TileStyle::by_key(&k).key.to_string())
                    .unwrap_or_else(|| STYLES[0].key.to_string()),
            ),
            moving: RwSignal::new(false),
            drag: RwSignal::new(None),
            // asked for the map without a site: straight to the globe
            globe: RwSignal::new(on && site.is_none()),
        }
    }

    /// The map for an area whose middle is `anchor` (plan m), when shown
    /// (tracked).
    pub fn map(&self, anchor: (f64, f64)) -> Option<SiteMap> {
        if !self.on.get() {
            return None;
        }
        let (lat, lon) = self.site.get()?;
        Some(SiteMap {
            lat,
            lon,
            anchor,
            opacity: self.opacity.get(),
            style: TileStyle::by_key(&self.style.get()),
        })
    }

    /// Link keys (tracked).
    pub fn link_query(&self) -> Option<String> {
        let (lat, lon) = self.site.get()?;
        let mut q = format!("site={lat:.6},{lon:.6}");
        if self.on.get() {
            q.push_str("&map=1");
        }
        if (self.opacity.get() - 0.85).abs() > 1e-6 {
            q.push_str(&format!("&mapop={:.2}", self.opacity.get()));
        }
        if self.style.get() != STYLES[0].key {
            q.push_str(&format!("&mapstyle={}", self.style.get()));
        }
        Some(q)
    }

    /// Remember the site for the next visit.
    pub fn store(&self) {
        if let (Some(s), Some((lat, lon))) = (storage(), self.site.get_untracked()) {
            let label = self.label.get_untracked().unwrap_or_default();
            if let Ok(t) = serde_json::to_string(&(lat, lon, label)) {
                let _ = s.set_item(SITE_KEY, &t);
            }
        }
    }

    /// A place picked on the globe: its exact spot, its name.
    pub fn set_from_pick(&self, p: &PickedPlace) {
        self.site.set(Some(p.view.unwrap_or((p.lat, p.lon))));
        self.label.set(Some(p.label()));
        self.on.set(true);
        self.store();
    }

    /// Drag on the plan from `start` to `now` (plan m), the area's middle
    /// at `anchor`: the map follows the pointer.
    pub fn drag_to(&self, now: (f64, f64), anchor: (f64, f64)) {
        let Some((start, (lat, lon))) = self.drag.get_untracked() else {
            return;
        };
        let m = SiteMap {
            lat,
            lon,
            anchor,
            opacity: 1.0,
            style: STYLES[0],
        };
        let (dx, dy) = (now.0 - start.0, now.1 - start.1);
        self.site.set(Some(m.to_geo(anchor.0 - dx, anchor.1 - dy)));
        self.label.set(None);
    }
}

/// The sidebar section. `on_pick` hears of places picked on the globe
/// (the moon takes the place too).
#[component]
pub fn SiteMapSidebar(state: SiteMapState, on_pick: Callback<PickedPlace>) -> impl IntoView {
    let locale = use_locale();
    let s = move || locale.get().area_designer.sitemap.clone();
    let num = move |label: Signal<String>, value: Signal<String>, on: Callback<f64>| {
        view! {
            <div class="zonal-prop-row">
                <span class="zonal-prop-label">{move || label.get()}</span>
                <input type="number" step="0.00001" prop:value=move || value.get()
                    on:change=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            on.run(v);
                        }
                    } />
            </div>
        }
    };
    view! {
        <div class="zonal-sidebar-section">
            <h4>{move || s().title}</h4>
            <label class="zonal-toggle">
                <input type="checkbox" prop:checked=move || state.on.get()
                    on:change=move |ev| state.on.set(event_target_checked(&ev)) />
                {move || format!(" {}", s().show)}
            </label>
            <p class="zonal-sensors-hint">{move || s().desc}</p>
            <button class="zonal-share-btn" on:click=move |_| state.globe.set(true)>{move || s().pick}</button>
            <GlobePicker open=state.globe street=true
                at=Signal::derive(move || state.site.get().unwrap_or((52.52, 13.40)))
                on_pick=Callback::new(move |p: PickedPlace| {
                    state.set_from_pick(&p);
                    on_pick.run(p);
                }) />
            <p class="zonal-sensors-hint">{move || match state.site.get() {
                Some((lat, lon)) => s().site.replace(
                    "{place}",
                    &state.label.get().map(|l| format!("{l} · {lat:.5}°, {lon:.5}°")).unwrap_or_else(|| format!("{lat:.5}°, {lon:.5}°")),
                ),
                None => s().no_site,
            }}</p>
            {num(
                Signal::derive(move || s().lat),
                Signal::derive(move || state.site.get().map(|v| format!("{:.5}", v.0)).unwrap_or_default()),
                Callback::new(move |v: f64| {
                    let lon = state.site.get_untracked().map_or(13.40, |s| s.1);
                    state.site.set(Some((v.clamp(-85.0, 85.0), lon)));
                    state.label.set(None);
                    state.store();
                }),
            )}
            {num(
                Signal::derive(move || s().lon),
                Signal::derive(move || state.site.get().map(|v| format!("{:.5}", v.1)).unwrap_or_default()),
                Callback::new(move |v: f64| {
                    let lat = state.site.get_untracked().map_or(52.52, |s| s.0);
                    state.site.set(Some((lat, (v + 180.0).rem_euclid(360.0) - 180.0)));
                    state.label.set(None);
                    state.store();
                }),
            )}
            {move || (state.on.get() && state.site.get().is_some()).then(|| view! {
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || s().style}</span>
                    <select on:change=move |ev| {
                        let k = event_target_value(&ev);
                        if let Some(st) = storage() {
                            let _ = st.set_item(STYLE_KEY, &k);
                        }
                        state.style.set(k);
                    }>
                        {STYLES.iter().map(|t| {
                            let key = t.key;
                            view! { <option value=key selected=move || state.style.get() == key>{t.label}</option> }
                        }).collect_view()}
                    </select>
                </div>
                <div class="zonal-prop-row">
                    <span class="zonal-prop-label">{move || format!("{} · {:.0} %", s().opacity, state.opacity.get() * 100.0)}</span>
                    <input type="range" min="0.2" max="1" step="0.05" prop:value=move || state.opacity.get().to_string()
                        on:input=move |ev| { if let Ok(v) = event_target_value(&ev).parse::<f64>() { state.opacity.set(v); } } />
                </div>
                <button class="zonal-move-btn" class:active=move || state.moving.get()
                    on:click=move |_| state.moving.update(|m| *m = !*m)
                >{move || if state.moving.get() { s().done } else { s().move_map }}</button>
                {move || state.moving.get().then(|| view! { <p class="zonal-sensors-hint">{move || s().move_hint}</p> })}
            })}
        </div>
    }
}
