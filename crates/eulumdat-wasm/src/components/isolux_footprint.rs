//! Isolux ground footprint component with tilt/height/area sliders
//!
//! Slider ranges adapt to the active unit system so that Imperial users see
//! round ft values (e.g. 10–100 ft) instead of ugly metric→ft conversions.

use super::app::use_unit_system;
use crate::i18n::use_locale;
use eulumdat::diagram::{IsoluxDiagram, IsoluxParams, SvgTheme};
use eulumdat::{Eulumdat, UnitSystem};
use leptos::ev;
use leptos::prelude::*;

/// Slider range definition that adapts to the unit system.
struct SliderRange {
    min: f64,
    max: f64,
    step: f64,
}

/// Return slider range for mounting height in the user's native unit.
fn height_range(units: UnitSystem) -> SliderRange {
    match units {
        UnitSystem::Metric   => SliderRange { min: 3.0,  max: 30.0,  step: 0.5 },
        UnitSystem::Imperial => SliderRange { min: 10.0, max: 100.0, step: 1.0 },
    }
}

/// Return slider range for area half-size in the user's native unit.
fn area_range(units: UnitSystem) -> SliderRange {
    match units {
        UnitSystem::Metric   => SliderRange { min: 10.0,  max: 100.0, step: 5.0  },
        UnitSystem::Imperial => SliderRange { min: 30.0,  max: 300.0, step: 10.0 },
    }
}

#[component]
pub fn IsoluxFootprint(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let unit_system = use_unit_system();

    // Internal values stored in meters
    let (mounting_height, set_mounting_height) = signal(10.0_f64);
    let (tilt_angle, set_tilt_angle) = signal(0.0_f64);
    let (area_size, set_area_size) = signal(20.0_f64);

    // Slider operates in the user's unit; convert to meters on change
    let on_height_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Ok(v) = target.value().parse::<f64>() {
            set_mounting_height.set(unit_system.get().to_meters(v));
        }
    };

    let on_tilt_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Ok(v) = target.value().parse::<f64>() {
            set_tilt_angle.set(v);
        }
    };

    let on_area_change = move |ev: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Ok(v) = target.value().parse::<f64>() {
            set_area_size.set(unit_system.get().to_meters(v));
        }
    };

    // Generate SVG reactively
    let svg_content = move || {
        let ldt_val = ldt.get();
        let units = unit_system.get();
        let params = IsoluxParams {
            mounting_height: mounting_height.get(),
            tilt_angle: tilt_angle.get(),
            area_half_width: area_size.get(),
            area_half_depth: area_size.get(),
            grid_resolution: 60,
        };
        let theme = SvgTheme::css_variables_with_locale(&locale.get());
        let diagram =
            IsoluxDiagram::from_eulumdat_with_units(&ldt_val, 600.0, 500.0, params, units);
        diagram.to_svg_with_units(600.0, 500.0, &theme, units)
    };

    view! {
        <div class="isolux-container">
            <div class="isolux-controls">
                <label class="control-group">
                    <span>{move || locale.get().ui.floodlight.mounting_height.clone()}</span>
                    <input
                        type="range"
                        prop:min=move || height_range(unit_system.get()).min.to_string()
                        prop:max=move || height_range(unit_system.get()).max.to_string()
                        prop:step=move || height_range(unit_system.get()).step.to_string()
                        prop:value=move || unit_system.get().convert_meters(mounting_height.get()).to_string()
                        on:input=on_height_change
                    />
                    <span class="control-value">{move || unit_system.get().format_distance(mounting_height.get())}</span>
                </label>

                <label class="control-group">
                    <span>{move || locale.get().ui.floodlight.tilt_angle.clone()}</span>
                    <input
                        type="range"
                        min="0" max="80" step="1"
                        prop:value=move || tilt_angle.get().to_string()
                        on:input=on_tilt_change
                    />
                    <span class="control-value">{move || format!("{:.0}°", tilt_angle.get())}</span>
                </label>

                <label class="control-group">
                    <span>{move || locale.get().ui.floodlight.area_size.clone()}</span>
                    <input
                        type="range"
                        prop:min=move || area_range(unit_system.get()).min.to_string()
                        prop:max=move || area_range(unit_system.get()).max.to_string()
                        prop:step=move || area_range(unit_system.get()).step.to_string()
                        prop:value=move || unit_system.get().convert_meters(area_size.get()).to_string()
                        on:input=on_area_change
                    />
                    <span class="control-value">{move || unit_system.get().format_distance(area_size.get())}</span>
                </label>
            </div>

            <div class="isolux-diagram" inner_html=svg_content />
        </div>
    }
}
