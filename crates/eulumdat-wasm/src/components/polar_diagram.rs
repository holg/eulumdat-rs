//! Polar diagram component for intensity visualization
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{PolarDiagram as CorePolarDiagram, SvgTheme},
    Eulumdat,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PolarDiagramProps {
    pub ldt: Eulumdat,
}

#[function_component(PolarDiagram)]
pub fn polar_diagram(props: &PolarDiagramProps) -> Html {
    let ldt = &props.ldt;

    // Generate polar diagram data from core
    let polar = CorePolarDiagram::from_eulumdat(ldt);

    // Generate SVG with CSS variables for theme support
    let svg = polar.to_svg(500.0, 500.0, &SvgTheme::css_variables());

    html! {
        <div class="polar-diagram">
            {Html::from_html_unchecked(AttrValue::from(svg))}
        </div>
    }
}
