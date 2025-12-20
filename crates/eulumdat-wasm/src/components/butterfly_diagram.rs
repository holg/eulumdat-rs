//! 3D Butterfly diagram component for intensity visualization (SVG version)
//! Uses eulumdat-core diagram module for SVG generation
//! Note: This is the SVG fallback version. The main 3D version uses Canvas (butterfly_3d.rs)

use crate::i18n::use_locale;
use eulumdat::{
    diagram::{ButterflyDiagram as CoreButterflyDiagram, SvgTheme},
    Eulumdat,
};
use leptos::prelude::*;

#[allow(dead_code)]
#[component]
pub fn ButterflyDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    view! {
        <div class="butterfly-diagram" inner_html=move || {
            let ldt = ldt.get();
            let butterfly = CoreButterflyDiagram::from_eulumdat(&ldt, 500.0, 450.0, 60.0);
            let theme = SvgTheme::css_variables_with_locale(&locale.get());
            butterfly.to_svg(500.0, 450.0, &theme)
        } />
    }
}
