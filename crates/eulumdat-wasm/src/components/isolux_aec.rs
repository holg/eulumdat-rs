//! AEC-style isolux diagram component with discrete colored contour bands.
//!
//! Renders the flat ground-plane illuminance with distinct color zones
//! (red/orange/yellow/green/cyan/purple) and a stepped legend showing
//! percentage levels and lux values — matching typical AEC photometric sheets.

use super::app::use_unit_system;
use crate::i18n::use_locale;
use eulumdat::diagram::{IsoluxDiagram, IsoluxParams, SvgTheme};
use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn IsoluxAec(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let unit_system = use_unit_system();

    let svg_content = move || {
        let ldt_val = ldt.get();
        let units = unit_system.get();
        let params = IsoluxParams {
            mounting_height: 10.0,
            tilt_angle: 0.0,
            area_half_width: 20.0,
            area_half_depth: 20.0,
            grid_resolution: 60,
        };
        let theme = SvgTheme::css_variables_with_locale(&locale.get());
        let diagram =
            IsoluxDiagram::from_eulumdat_with_units(&ldt_val, 500.0, 500.0, params, units);
        diagram.to_svg_aec(500.0, 500.0, &theme, units)
    };

    view! {
        <div class="isolux-aec-container">
            <div class="isolux-aec-diagram" inner_html=svg_content />
        </div>
    }
}
