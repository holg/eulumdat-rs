//! Intensity Heatmap component
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{HeatmapDiagram, SvgTheme},
    Eulumdat, PhotometricSummary,
};
use leptos::prelude::*;

#[component]
pub fn IntensityHeatmap(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    view! {
        <div class="intensity-heatmap" inner_html=move || {
            let ldt = ldt.get();
            let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);
            let summary = PhotometricSummary::from_eulumdat(&ldt);
            heatmap.to_svg_with_summary(700.0, 500.0, &SvgTheme::css_variables(), &summary)
        } />
    }
}
