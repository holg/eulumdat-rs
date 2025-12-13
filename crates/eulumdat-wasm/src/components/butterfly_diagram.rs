//! 3D Butterfly diagram component for intensity visualization (SVG version)
//! Uses eulumdat-core diagram module for SVG generation
//! Note: This is the SVG fallback version. The main 3D version uses Canvas (butterfly_3d.rs)

use eulumdat::{
    diagram::{ButterflyDiagram as CoreButterflyDiagram, SvgTheme},
    Eulumdat,
};
use leptos::prelude::*;

#[allow(dead_code)]
#[component]
pub fn ButterflyDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    view! {
        <div class="butterfly-diagram" inner_html=move || {
            let ldt = ldt.get();
            let butterfly = CoreButterflyDiagram::from_eulumdat(&ldt, 500.0, 450.0, 60.0);
            butterfly.to_svg(500.0, 450.0, &SvgTheme::css_variables())
        } />
    }
}
