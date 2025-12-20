//! Beam Angle Diagram component - Wikipedia-style IES vs CIE comparison
//!
//! Shows polar intensity distribution with beam and field angle overlays,
//! comparing IES (max intensity based) and CIE (center-beam based) definitions.
//! Particularly useful for batwing distributions where IES ≠ CIE.

use crate::i18n::use_locale;
use eulumdat::{
    diagram::{PolarDiagram as CorePolarDiagram, SvgTheme},
    Eulumdat, PhotometricCalculations,
};
use leptos::prelude::*;

#[component]
pub fn BeamAngleDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    view! {
        <div class="polar-diagram beam-angle-diagram" inner_html=move || {
            let ldt = ldt.get();
            let polar = CorePolarDiagram::from_eulumdat(&ldt);
            let analysis = PhotometricCalculations::beam_field_analysis(&ldt);
            let theme = SvgTheme::css_variables_with_locale(&locale.get());

            // Show both IES and CIE definitions when they differ (batwing)
            let show_both = analysis.is_batwing;

            polar.to_svg_with_beam_field_angles(500.0, 500.0, &theme, &analysis, show_both)
        } />
    }
}
