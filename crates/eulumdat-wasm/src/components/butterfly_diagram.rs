//! 3D Butterfly diagram component for intensity visualization (SVG version)
//! Uses eulumdat-core diagram module for SVG generation
//! Note: This is the SVG fallback version. The main 3D version uses Canvas (butterfly_3d.rs)

use eulumdat::{
    diagram::{ButterflyDiagram as CoreButterflyDiagram, SvgTheme},
    Eulumdat,
};
use yew::prelude::*;

#[allow(dead_code)]
#[derive(Properties, PartialEq)]
pub struct ButterflyDiagramProps {
    pub ldt: Eulumdat,
}

#[function_component(ButterflyDiagram)]
pub fn butterfly_diagram(props: &ButterflyDiagramProps) -> Html {
    let ldt = &props.ldt;

    // Generate butterfly diagram data from core
    let butterfly = CoreButterflyDiagram::from_eulumdat(ldt, 500.0, 450.0, 60.0);

    // Generate SVG with CSS variables for theme support
    let svg = butterfly.to_svg(500.0, 450.0, &SvgTheme::css_variables());

    html! {
        <div class="butterfly-diagram">
            {Html::from_html_unchecked(AttrValue::from(svg))}
        </div>
    }
}
