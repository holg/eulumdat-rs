//! Cartesian diagram component for intensity visualization
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{CartesianDiagram as CoreCartesianDiagram, SvgTheme},
    Eulumdat,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct CartesianDiagramProps {
    pub ldt: Eulumdat,
}

#[function_component(CartesianDiagram)]
pub fn cartesian_diagram(props: &CartesianDiagramProps) -> Html {
    let ldt = &props.ldt;

    // Generate cartesian diagram data from core (max 8 curves)
    let cartesian = CoreCartesianDiagram::from_eulumdat(ldt, 500.0, 380.0, 8);

    // Generate SVG with CSS variables for theme support
    let svg = cartesian.to_svg(500.0, 380.0, &SvgTheme::css_variables());

    html! {
        <div class="cartesian-diagram">
            {Html::from_html_unchecked(AttrValue::from(svg))}
        </div>
    }
}
