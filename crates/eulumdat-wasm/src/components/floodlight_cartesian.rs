//! Floodlight V-H Cartesian diagram component with NEMA classification badge

use crate::i18n::use_locale;
use eulumdat::diagram::{FloodlightCartesianDiagram, SvgTheme, YScale};
use eulumdat::{Eulumdat, PhotometricCalculations};
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn FloodlightCartesian(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let (log_scale, set_log_scale) = signal(false);

    let toggle_scale = move |_: ev::MouseEvent| {
        set_log_scale.update(|v| *v = !*v);
    };

    // Generate SVG reactively
    let svg_content = move || {
        let ldt_val = ldt.get();
        let y_scale = if log_scale.get() {
            YScale::Logarithmic
        } else {
            YScale::Linear
        };
        let theme = SvgTheme::css_variables_with_locale(&locale.get());
        let diagram = FloodlightCartesianDiagram::from_eulumdat(&ldt_val, 600.0, 400.0, y_scale);
        diagram.to_svg(600.0, 400.0, &theme)
    };

    // NEMA classification
    let nema = move || {
        let ldt_val = ldt.get();
        PhotometricCalculations::nema_classification(&ldt_val)
    };

    view! {
        <div class="floodlight-vh-container">
            <div class="floodlight-controls">
                <button
                    class=move || format!("btn btn-sm {}", if log_scale.get() { "btn-primary" } else { "btn-secondary" })
                    on:click=toggle_scale
                >
                    {move || {
                        let l = locale.get();
                        if log_scale.get() {
                            l.ui.floodlight.log_scale.clone()
                        } else {
                            l.ui.floodlight.linear_scale.clone()
                        }
                    }}
                </button>

                <div class="nema-badge" title=move || {
                    let n = nema();
                    format!("H: {:.1}° (Type {}) | V: {:.1}° (Type {})", n.horizontal_spread, n.horizontal_type, n.vertical_spread, n.vertical_type)
                }>
                    <span class="nema-label">{move || locale.get().ui.floodlight.nema_classification.clone()}</span>
                    <span class="nema-value">{move || nema().designation}</span>
                </div>
            </div>

            <div class="floodlight-diagram" inner_html=svg_content />

            <div class="nema-details">
                <table class="nema-table">
                    <tr>
                        <th>{move || locale.get().ui.floodlight.h_plane.clone()}</th>
                        <td>{move || format!("{:.1}°", nema().horizontal_spread)}</td>
                        <td>{move || format!("Type {}", nema().horizontal_type)}</td>
                    </tr>
                    <tr>
                        <th>{move || locale.get().ui.floodlight.v_plane.clone()}</th>
                        <td>{move || format!("{:.1}°", nema().vertical_spread)}</td>
                        <td>{move || format!("Type {}", nema().vertical_type)}</td>
                    </tr>
                    <tr>
                        <th>"I_max"</th>
                        <td colspan="2">{move || format!("{:.0} cd/klm", nema().i_max)}</td>
                    </tr>
                </table>
            </div>
        </div>
    }
}
