//! Polar diagram component for intensity visualization
//! Uses PolarDiagram::render_svg() from eulumdat-core

use crate::i18n::use_locale;
use eulumdat::{
    diagram::{PolarDiagram as CorePolarDiagram, SvgTheme},
    Eulumdat,
};
use leptos::prelude::*;

#[component]
pub fn PolarDiagram(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    let c_planes = Memo::new(move |_| CorePolarDiagram::available_c_planes(&ldt.get()));

    let (selected_plane, set_selected_plane) = signal::<Option<f64>>(None);
    let (slider_idx, set_slider_idx) = signal(0usize);

    view! {
        {move || {
            let planes = c_planes.get();
            if !planes.is_empty() {
                let max_idx = planes.len() - 1;
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
                                                    let planes = c_planes.get();
                                                    if let Some(&angle) = planes.get(idx) {
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
                                            {move || locale.get().diagram.title.polar.clone()}
                                        </span>
                                        <button class="btn btn-sm btn-outline"
                                            on:click=move |_| {
                                                set_selected_plane.set(Some(0.0));
                                                set_slider_idx.set(0);
                                            }
                                        >
                                            {move || locale.get().ui.butterfly.c_plane_selector.clone()}
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
        <div class="polar-diagram" inner_html=move || {
            let theme = SvgTheme::css_variables_with_locale(&locale.get());
            CorePolarDiagram::render_svg(&ldt.get(), selected_plane.get(), 500.0, 500.0, &theme)
        } />
    }
}
