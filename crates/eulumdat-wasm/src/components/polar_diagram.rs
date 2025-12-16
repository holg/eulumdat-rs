//! Polar diagram component for intensity visualization
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{PolarDiagram as CorePolarDiagram, SvgTheme},
    Eulumdat, PhotometricSummary,
};
use leptos::prelude::*;

#[component]
pub fn PolarDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    view! {
        <div class="polar-diagram" inner_html=move || {
            let ldt = ldt.get();
            let polar = CorePolarDiagram::from_eulumdat(&ldt);
            let summary = PhotometricSummary::from_eulumdat(&ldt);
            polar.to_svg_with_summary(500.0, 500.0, &SvgTheme::css_variables(), &summary)
        } />
    }
}
