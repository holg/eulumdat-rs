//! Cone Diagram component - shows beam/field angle spread visualization
//! Uses eulumdat-core diagram module for SVG generation

use crate::i18n::use_locale;
use eulumdat::{
    diagram::{ConeDiagram, ConeDiagramLabels, SvgTheme},
    Eulumdat,
};
use leptos::prelude::*;

/// Cone diagram view showing beam and field angle spread
#[component]
pub fn ConeDiagramView(
    ldt: ReadSignal<Eulumdat>,
    /// Mounting height in meters
    mounting_height: ReadSignal<f64>,
) -> impl IntoView {
    let locale = use_locale();

    view! {
        <div class="cone-diagram" inner_html=move || {
            let ldt = ldt.get();
            let height = mounting_height.get();
            let cone = ConeDiagram::from_eulumdat(&ldt, height);
            let theme = SvgTheme::css_variables_with_locale(&locale.get());

            // Create localized labels from the locale
            let loc = locale.get();
            let labels = ConeDiagramLabels {
                beam_angle: loc.diagram.cone.beam_angle.clone(),
                field_angle: loc.diagram.cone.field_angle.clone(),
                mounting_height: loc.diagram.cone.mounting_height.clone(),
                beam_diameter: loc.diagram.cone.beam_diameter.clone(),
                field_diameter: loc.diagram.cone.field_diameter.clone(),
                intensity_50: loc.diagram.cone.intensity_50.clone(),
                intensity_10: loc.diagram.cone.intensity_10.clone(),
                floor: loc.diagram.cone.floor.clone(),
                meter: loc.diagram.cone.meter.clone(),
            };

            cone.to_svg_with_labels(500.0, 400.0, &theme, &labels)
        } />
    }
}
