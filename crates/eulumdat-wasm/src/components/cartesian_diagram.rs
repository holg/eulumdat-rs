//! Cartesian diagram component for intensity visualization
//! Uses eulumdat-core diagram module for SVG generation

use crate::i18n::use_locale;
use eulumdat::{
    diagram::{CartesianDiagram as CoreCartesianDiagram, ConeDiagram, SvgTheme},
    Eulumdat, PhotometricSummary, SymmetryHandler,
};
use leptos::prelude::*;

#[component]
pub fn CartesianDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    let has_variation = Memo::new(move |_| ConeDiagram::has_c_plane_variation(&ldt.get()));

    let half_angles = Memo::new(move |_| {
        let angles = SymmetryHandler::expand_c_angles(&ldt.get());
        angles
            .into_iter()
            .filter(|&a| a <= 360.0)
            .collect::<Vec<f64>>()
    });

    let (selected_plane, set_selected_plane) = signal::<Option<f64>>(None);
    let (slider_idx, set_slider_idx) = signal(0usize);

    view! {
        {move || {
            if has_variation.get() {
                let ha = half_angles.get();
                let max_idx = if ha.is_empty() { 0 } else { ha.len() - 1 };
                view! {
                    <div class="isolux-controls">
                        <div class="control-group">
                            <span>{move || locale.get().diagram.axis.c_plane.clone()}</span>
                            {move || {
                                if selected_plane.get().is_some() {
                                    view! {
                                        <input type="range"
                                            min="0"
                                            prop:max=max_idx.to_string()
                                            prop:value=move || slider_idx.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(idx) = event_target_value(&ev).parse::<usize>() {
                                                    set_slider_idx.set(idx);
                                                    let ha = half_angles.get();
                                                    if let Some(&angle) = ha.get(idx) {
                                                        set_selected_plane.set(Some(angle));
                                                    }
                                                }
                                            }
                                        />
                                        <span class="control-value">
                                            {move || format!("C {:.0}°", selected_plane.get().unwrap_or(0.0))}
                                        </span>
                                        <button class="btn btn-sm btn-outline"
                                            on:click=move |_| {
                                                set_selected_plane.set(None);
                                                set_slider_idx.set(0);
                                            }
                                        >
                                            "✕"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span style="color: var(--text-muted); font-size: 0.8rem;">
                                            {move || locale.get().diagram.title.cartesian.clone()}
                                        </span>
                                        <button class="btn btn-sm btn-outline"
                                            on:click=move |_| {
                                                set_selected_plane.set(Some(0.0));
                                                set_slider_idx.set(0);
                                            }
                                        >
                                            "C-Plane ▸"
                                        </button>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <div /> }.into_any()
            }
        }}
        <div class="cartesian-diagram" inner_html=move || {
            let ldt = ldt.get();
            let theme = SvgTheme::css_variables_with_locale(&locale.get());
            let summary = PhotometricSummary::from_eulumdat(&ldt);
            if let Some(cp) = selected_plane.get() {
                let cartesian = CoreCartesianDiagram::from_eulumdat_for_plane(&ldt, cp, 500.0, 380.0);
                cartesian.to_svg_with_summary(500.0, 380.0, &theme, &summary)
            } else {
                let cartesian = CoreCartesianDiagram::from_eulumdat(&ldt, 500.0, 380.0, 8);
                cartesian.to_svg_with_summary(500.0, 380.0, &theme, &summary)
            }
        } />
    }
}
