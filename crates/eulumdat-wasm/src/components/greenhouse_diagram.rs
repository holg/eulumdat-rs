//! Greenhouse PPFD diagram component
//!
//! Displays PPFD (µmol/m²/s) at different mounting distances for horticultural lighting.

use atla::greenhouse::{GreenhouseDiagram, GreenhouseTheme};
use atla::LuminaireOpticalData;
use leptos::prelude::*;

/// Greenhouse PPFD diagram component
#[component]
pub fn GreenhouseDiagramView(
    atla_doc: ReadSignal<LuminaireOpticalData>,
    dark: Memo<bool>,
    /// Maximum mounting height in meters
    max_height: ReadSignal<f64>,
) -> impl IntoView {
    let svg_content = move || {
        let doc = atla_doc.get();
        let height = max_height.get();
        let diagram = GreenhouseDiagram::from_atla_with_height(&doc, height);
        let theme = if dark.get() {
            GreenhouseTheme::dark()
        } else {
            GreenhouseTheme::light()
        };
        diagram.to_svg(600.0, 450.0, &theme)
    };

    view! {
        <div class="greenhouse-diagram-container">
            <div class="greenhouse-diagram" inner_html=svg_content />
        </div>
    }
}
