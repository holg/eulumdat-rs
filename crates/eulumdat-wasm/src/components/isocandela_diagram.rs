//! Isocandela contour plot component

use crate::i18n::use_locale;
use eulumdat::diagram::{IsocandelaDiagram, SvgTheme};
use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn IsocandelaDiagramView(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    // Generate SVG reactively
    let svg_content = move || {
        let ldt_val = ldt.get();
        let theme = SvgTheme::css_variables_with_locale(&locale.get());
        let diagram = IsocandelaDiagram::from_eulumdat(&ldt_val, 600.0, 500.0);
        diagram.to_svg(600.0, 500.0, &theme)
    };

    view! {
        <div class="isocandela-diagram" inner_html=svg_content />
    }
}
