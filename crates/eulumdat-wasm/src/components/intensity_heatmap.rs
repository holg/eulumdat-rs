//! Intensity Heatmap component
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{HeatmapDiagram, SvgTheme},
    Eulumdat,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct IntensityHeatmapProps {
    pub ldt: Eulumdat,
}

#[function_component(IntensityHeatmap)]
pub fn intensity_heatmap(props: &IntensityHeatmapProps) -> Html {
    let ldt = &props.ldt;

    // Generate heatmap diagram data from core
    let heatmap = HeatmapDiagram::from_eulumdat(ldt, 700.0, 500.0);

    // Generate SVG with CSS variables for theme support
    let svg = heatmap.to_svg(700.0, 500.0, &SvgTheme::css_variables());

    html! {
        <div class="intensity-heatmap">
            {Html::from_html_unchecked(AttrValue::from(svg))}
        </div>
    }
}
