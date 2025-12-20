use crate::i18n::use_locale;
use eulumdat::{Eulumdat, Symmetry};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// Format a number: show as integer if it's a whole number, otherwise with decimals
fn format_value(v: f64) -> String {
    if (v - v.round()).abs() < 0.001 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

/// Format angle: show half degrees like 47.5, otherwise integer
fn format_angle(v: f64) -> String {
    if (v - v.round()).abs() < 0.001 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}

/// Get the starting index for C-angles based on symmetry type
fn get_c_angle_start_index(ldt: &Eulumdat) -> usize {
    match ldt.symmetry {
        Symmetry::PlaneC90C270 => ldt.c_angles.iter().position(|&c| c >= 90.0).unwrap_or(0),
        _ => 0,
    }
}

#[component]
pub fn DataTable(ldt: ReadSignal<Eulumdat>, set_ldt: WriteSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let on_copy = move |_: ev::MouseEvent| {
        let ldt = ldt.get();
        let mut text = String::new();
        let mc = ldt.intensities.len();
        let c_start = get_c_angle_start_index(&ldt);

        // Header row with C angles
        text.push_str("γ\\C");
        for i in 0..mc {
            text.push('\t');
            if let Some(&angle) = ldt.c_angles.get(c_start + i) {
                text.push_str(&format_angle(angle));
            }
        }
        text.push('\n');

        // Data rows
        for (g_idx, g_angle) in ldt.g_angles.iter().enumerate() {
            text.push_str(&format_angle(*g_angle));
            for c_idx in 0..mc {
                text.push('\t');
                if let Some(intensity) = ldt.intensities.get(c_idx).and_then(|row| row.get(g_idx)) {
                    text.push_str(&format_value(*intensity));
                }
            }
            text.push('\n');
        }

        // Copy to clipboard
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&text);
        }
    };

    move || {
        let ldt_val = ldt.get();
        let l = locale.get();

        if ldt_val.intensities.is_empty() || ldt_val.g_angles.is_empty() {
            return view! {
                <div class="text-center text-muted">
                    {l.ui.data_table.no_data.clone()}
                </div>
            }
            .into_any();
        }

        let mc = ldt_val.intensities.len();
        let c_start = get_c_angle_start_index(&ldt_val);

        let display_c_angles: Vec<f64> = ldt_val
            .c_angles
            .iter()
            .skip(c_start)
            .take(mc)
            .copied()
            .collect();

        view! {
            <div class="data-table-wrapper">
                <div class="data-table-toolbar">
                    <button class="btn btn-sm btn-secondary" on:click=on_copy>
                        {l.ui.data_table.copy_to_clipboard.clone()}
                    </button>
                </div>

                <div class="data-table-container">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th class="angle-header">"γ \\ C"</th>
                                {display_c_angles.iter().map(|angle| {
                                    view! {
                                        <th class="c-angle-header">{format_angle(*angle)}</th>
                                    }
                                }).collect_view()}
                            </tr>
                        </thead>
                        <tbody>
                            {ldt_val.g_angles.iter().enumerate().map(|(g_idx, g_angle)| {
                                view! {
                                    <tr>
                                        <th class="g-angle-header">{format_angle(*g_angle)}</th>
                                        {(0..mc).map(|c_idx| {
                                            let intensity = ldt_val.intensities.get(c_idx)
                                                .and_then(|row| row.get(g_idx))
                                                .copied()
                                                .unwrap_or(0.0);

                                            let on_change = move |e: ev::Event| {
                                                let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                                if let Ok(v) = input.value().parse::<f64>() {
                                                    set_ldt.update(|ldt| {
                                                        if let Some(row) = ldt.intensities.get_mut(c_idx) {
                                                            if let Some(cell) = row.get_mut(g_idx) {
                                                                *cell = v;
                                                            }
                                                        }
                                                    });
                                                }
                                            };

                                            view! {
                                                <td>
                                                    <input
                                                        type="number"
                                                        step="1"
                                                        prop:value=format_value(intensity)
                                                        class="intensity-input"
                                                        on:change=on_change
                                                    />
                                                </td>
                                            }
                                        }).collect_view()}
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        }.into_any()
    }
}
