//! Intensity Heatmap component
//! Uses eulumdat-core diagram module for SVG generation

use crate::i18n::use_locale;
use eulumdat::{
    diagram::{HeatmapDiagram, SvgTheme},
    Eulumdat, PhotometricSummary,
};
use leptos::prelude::*;

#[component]
pub fn IntensityHeatmap(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    view! {
        <div class="intensity-heatmap" inner_html=move || {
            let ldt = ldt.get();
            let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);
            let summary = PhotometricSummary::from_eulumdat(&ldt);
            let theme = SvgTheme::css_variables_with_locale(&locale.get());
            heatmap.to_svg_with_summary(700.0, 500.0, &theme, &summary)
        } />
    }
}
