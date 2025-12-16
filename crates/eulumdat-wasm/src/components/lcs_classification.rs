//! LCS (Luminaire Classification System) Component
//! IES TM-15-07 classification for outdoor luminaires

use eulumdat::{diagram::SvgTheme, BugDiagram, Eulumdat};
use leptos::prelude::*;

#[component]
pub fn LcsClassification(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    view! {
        <div class="lcs-container">
            <div class="lcs-content" inner_html=move || {
                let ldt = ldt.get();
                let diagram = BugDiagram::from_eulumdat(&ldt);
                let theme = SvgTheme::css_variables();
                diagram.to_lcs_svg(510.0, 315.0, &theme)
            } />

            <div class="lcs-footer">
                "IES TM-15-07 Luminaire Classification System For Outdoor Luminaires"
            </div>
        </div>
    }
}
