//! Cartesian diagram component for intensity visualization
//! Uses eulumdat-core diagram module for SVG generation

use eulumdat::{
    diagram::{CartesianDiagram as CoreCartesianDiagram, SvgTheme},
    Eulumdat,
};
use leptos::prelude::*;

#[component]
pub fn CartesianDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    view! {
        <div class="cartesian-diagram" inner_html=move || {
            let ldt = ldt.get();
            let cartesian = CoreCartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 8);
            cartesian.to_svg(500.0, 380.0, &SvgTheme::css_variables())
        } />
    }
}
