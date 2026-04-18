//! Isometric 3D isolux diagram component (AEC "ISO view" style)
//!
//! Renders a ground-plane illuminance footprint in isometric projection
//! with a mini polar diagram on a pole at the luminaire position.

use super::app::use_unit_system;
use crate::i18n::use_locale;
use eulumdat::diagram::{IsoluxDiagram, IsoluxParams, IsometricConfig, SvgTheme};
use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn IsoluxIsometric(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let unit_system = use_unit_system();

    let svg_content = move || {
        let ldt_val = ldt.get();
        let units = unit_system.get();
        let params = IsoluxParams {
            mounting_height: 10.0,
            tilt_angle: 0.0,
            area_half_width: 30.0,
            area_half_depth: 15.0,
            grid_resolution: 50,
        };
        let theme = SvgTheme::css_variables_with_locale(&locale.get());
        let diagram =
            IsoluxDiagram::from_eulumdat_with_units(&ldt_val, 600.0, 500.0, params, units);

        // Build title from luminaire name
        let iso_label = locale.get().ui.diagram.title_iso_view.clone();
        let title = if !ldt_val.luminaire_name.is_empty() {
            format!("{} {}", iso_label, ldt_val.luminaire_name)
        } else if !ldt_val.identification.is_empty() {
            format!("{} {}", iso_label, ldt_val.identification)
        } else {
            iso_label
        };

        diagram.to_svg_isometric(
            600.0,
            500.0,
            &theme,
            &IsometricConfig::default(),
            units,
            &ldt_val,
            &title,
        )
    };

    view! {
        <div class="isolux-isometric-container">
            <div class="isolux-isometric-diagram" inner_html=svg_content />
        </div>
    }
}
