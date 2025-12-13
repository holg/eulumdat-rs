//! Intensity Heatmap component
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{HeatmapDiagram, SvgTheme},
    Eulumdat,
};
use leptos::prelude::*;

#[component]
pub fn IntensityHeatmap(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    view! {
        <div class="intensity-heatmap" inner_html=move || {
            let ldt = ldt.get();
            let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);
            heatmap.to_svg(700.0, 500.0, &SvgTheme::css_variables())
        } />
    }
}
