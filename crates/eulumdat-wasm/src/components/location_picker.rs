//! Pick a place on the geodb-rs globe.
//!
//! The globe is the WebGPU "mini" build of `geodb-globe` (sibling repository
//! `geodb-rs`), copied to `app/globe/` by `build-wasm-split.sh`. Opened with
//! `?pick=1` it shows a "Use this place" bar and posts the chosen city to the
//! page that embeds it:
//! `{type: "geodb-pick", name, state, country, emoji, lat, lon, timezone,
//! view_lat, view_lon, view_dist}` (`view_*` = the exact spot in the middle
//! of the screen, for a site rather than its town). In street mode
//! ([`GlobePicker`] `street`) it opens close in with the OpenStreetMap
//! tiles of geodb-globe (`?tiles=osm`).
//! [`GlobePicker`] shows it in a dialog and hands the place over; the IANA
//! timezone becomes the standard UTC offset and the DST regime through the
//! browser's own timezone data (`Intl.DateTimeFormat`), so no tz database is
//! linked in.

use crate::i18n::use_locale;
use eulumdat_daylight::DstRule;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// localStorage key of the last place picked.
const STORE_KEY: &str = "eulumdat_location";

/// A place picked on the globe.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PickedPlace {
    pub name: String,
    pub state: String,
    pub country: String,
    pub emoji: String,
    pub lat: f64,
    pub lon: f64,
    /// IANA timezone ("Europe/Zurich"); empty when the globe did not know it.
    pub timezone: String,
    /// The exact spot looked at when the place was picked (lat, lon; the
    /// screen's middle), for a site; `None` from older globes.
    #[serde(default)]
    pub view: Option<(f64, f64)>,
}

impl PickedPlace {
    /// "🇨🇭 Zürich, Switzerland".
    pub fn label(&self) -> String {
        let mut s = String::new();
        if !self.emoji.is_empty() {
            s.push_str(&self.emoji);
            s.push(' ');
        }
        s.push_str(&self.name);
        if !self.country.is_empty() {
            s.push_str(", ");
            s.push_str(&self.country);
        }
        s
    }

    /// Standard (non-DST) UTC offset in hours and the DST regime of the
    /// place's timezone; from the longitude (15° per hour, no DST) when the
    /// timezone is unknown to the browser.
    pub fn clock(&self) -> (f64, DstRule) {
        tz_clock(&self.timezone).unwrap_or(((self.lon / 15.0).round(), DstRule::None))
    }

    /// The last place picked in this browser.
    pub fn load_last() -> Option<Self> {
        let text = web_sys::window()?
            .local_storage()
            .ok()??
            .get_item(STORE_KEY)
            .ok()??;
        serde_json::from_str(&text).ok()
    }

    fn store(&self) {
        if let (Some(storage), Ok(text)) = (
            web_sys::window().and_then(|w| w.local_storage().ok().flatten()),
            serde_json::to_string(self),
        ) {
            let _ = storage.set_item(STORE_KEY, &text);
        }
    }

    /// From a `geodb-pick` message.
    fn from_message(data: &wasm_bindgen::JsValue) -> Option<Self> {
        let get = |k: &str| js_sys::Reflect::get(data, &k.into()).ok();
        let text = |k: &str| get(k).and_then(|v| v.as_string()).unwrap_or_default();
        if text("type") != "geodb-pick" {
            return None;
        }
        let lat = get("lat")?.as_f64()?;
        let lon = get("lon")?.as_f64()?;
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return None;
        }
        let view = match (
            get("view_lat").and_then(|v| v.as_f64()),
            get("view_lon").and_then(|v| v.as_f64()),
        ) {
            (Some(a), Some(o)) if (-90.0..=90.0).contains(&a) => {
                Some((a, (o + 180.0).rem_euclid(360.0) - 180.0))
            }
            _ => None,
        };
        Some(Self {
            name: text("name"),
            state: text("state"),
            country: text("country"),
            emoji: text("emoji"),
            lat,
            lon,
            timezone: text("timezone"),
            view,
        })
    }
}

/// UTC offset (hours) of `tz` at the instant `utc_ms`, from the browser's
/// timezone data; `None` for an unknown zone.
fn tz_offset_hours(tz: &str, utc_ms: f64) -> Option<f64> {
    use js_sys::{Array, Function, Object, Reflect};
    if tz.is_empty() {
        return None;
    }
    let intl = Reflect::get(&js_sys::global(), &"Intl".into()).ok()?;
    let ctor: Function = Reflect::get(&intl, &"DateTimeFormat".into())
        .ok()?
        .dyn_into()
        .ok()?;
    let opts = Object::new();
    Reflect::set(&opts, &"timeZone".into(), &tz.into()).ok()?;
    Reflect::set(&opts, &"timeZoneName".into(), &"longOffset".into()).ok()?;
    // an unknown zone throws a RangeError: caught here
    let dtf = Reflect::construct(&ctor, &Array::of2(&"en-US".into(), &opts)).ok()?;
    let parts: Array = Reflect::get(&dtf, &"formatToParts".into())
        .ok()?
        .dyn_into::<Function>()
        .ok()?
        .call1(&dtf, &js_sys::Date::new(&utc_ms.into()))
        .ok()?
        .dyn_into()
        .ok()?;
    let name = parts.iter().find_map(|p| {
        let kind = Reflect::get(&p, &"type".into()).ok()?.as_string()?;
        (kind == "timeZoneName")
            .then(|| Reflect::get(&p, &"value".into()).ok()?.as_string())
            .flatten()
    })?;
    parse_gmt_offset(&name)
}

/// "GMT+01:00" / "GMT-03:30" / "GMT" → hours.
fn parse_gmt_offset(s: &str) -> Option<f64> {
    let rest = s.trim().strip_prefix("GMT")?;
    if rest.is_empty() {
        return Some(0.0);
    }
    let sign = match rest.chars().next()? {
        '+' => 1.0,
        '-' | '−' => -1.0,
        _ => return None,
    };
    let digits = &rest[rest.chars().next()?.len_utf8()..];
    let (h, m) = digits.split_once(':').unwrap_or((digits, "0"));
    Some(sign * (h.parse::<f64>().ok()? + m.parse::<f64>().ok()? / 60.0))
}

/// Standard offset and DST regime of `tz`: the smaller of the January and
/// July offsets is standard time; summer time in July = northern (US when
/// it has started by 20 March, else EU), in January = southern.
fn tz_clock(tz: &str) -> Option<(f64, DstRule)> {
    let at = |month: u32, day: u32| {
        // noon UTC of that day in 2026
        let ms =
            js_sys::Date::utc(2026.0, (month - 1) as f64) + (day as f64 - 1.0 + 0.5) * 86_400_000.0;
        tz_offset_hours(tz, ms)
    };
    let (jan, jul) = (at(1, 15)?, at(7, 15)?);
    let standard = jan.min(jul);
    let dst = if (jan - jul).abs() < 1e-6 {
        DstRule::None
    } else if jul > jan {
        if at(3, 20)? > standard + 1e-6 {
            DstRule::Us
        } else {
            DstRule::Eu
        }
    } else {
        DstRule::SouthernEu
    };
    Some((standard, dst))
}

/// The globe in a dialog while `open`; a place picked there goes to
/// `on_pick` (and is remembered for the next visit). `at` = where the globe
/// starts (lat, lon). `street`: close in, with the OpenStreetMap tiles
/// (finding a site; its spot comes back in [`PickedPlace::view`]).
#[component]
pub fn GlobePicker(
    open: RwSignal<bool>,
    at: Signal<(f64, f64)>,
    on_pick: Callback<PickedPlace>,
    #[prop(optional)] street: bool,
) -> impl IntoView {
    let t = Memo::new(move |_| use_locale().get().spectrum_lab.clone());
    let handle = window_event_listener_untyped("message", move |ev| {
        if !open.get_untracked() {
            return;
        }
        let Ok(ev) = ev.dyn_into::<web_sys::MessageEvent>() else {
            return;
        };
        // only the globe of this site
        let own = web_sys::window().and_then(|w| w.location().origin().ok());
        if own.as_deref() != Some(ev.origin().as_str()) {
            return;
        }
        if let Some(place) = PickedPlace::from_message(&ev.data()) {
            place.store();
            open.set(false);
            on_pick.run(place);
        }
    });
    on_cleanup(move || handle.remove());
    move || {
        open.get().then(|| {
            let (lat, lon) = at.get_untracked();
            let src = if street {
                // ~2 km above the ground: streets and buildings in view
                format!("globe/index.html?pick=1&tiles=osm&at={lat:.5},{lon:.5},1.0003")
            } else {
                format!("globe/index.html?pick=1&at={lat:.4},{lon:.4},1.6")
            };
            view! {
                <div class="globe-picker-backdrop" on:click=move |_| open.set(false)>
                    <div class="globe-picker" on:click=|ev| ev.stop_propagation()>
                        <div class="globe-picker-head">
                            <b>{move || t.get().globe_title}</b>
                            <span>{move || t.get().globe_hint}</span>
                            <button on:click=move |_| open.set(false)>{move || format!("✕ {}", t.get().globe_close)}</button>
                        </div>
                        <iframe src=src title=move || t.get().globe_title allow="fullscreen"></iframe>
                    </div>
                </div>
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_gmt_offset;

    #[test]
    fn gmt_offsets() {
        assert_eq!(parse_gmt_offset("GMT"), Some(0.0));
        assert_eq!(parse_gmt_offset("GMT+01:00"), Some(1.0));
        assert_eq!(parse_gmt_offset("GMT-03:30"), Some(-3.5));
        assert_eq!(parse_gmt_offset("GMT+05:45"), Some(5.75));
        assert_eq!(parse_gmt_offset("UTC+1"), None);
    }
}
