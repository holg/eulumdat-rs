use eulumdat::{Eulumdat, Symmetry};
use web_sys::HtmlInputElement;
use yew::prelude::*;

use super::app::Msg;

#[derive(Properties, PartialEq)]
pub struct DataTableProps {
    pub ldt: Eulumdat,
    pub on_update: Callback<Msg>,
}

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
/// For symmetry 3 (PlaneC90C270), intensity data starts at C90
fn get_c_angle_start_index(ldt: &Eulumdat) -> usize {
    match ldt.symmetry {
        Symmetry::PlaneC90C270 => {
            // Find index where C-angle >= 90
            ldt.c_angles.iter().position(|&c| c >= 90.0).unwrap_or(0)
        }
        _ => 0,
    }
}

#[function_component(DataTable)]
pub fn data_table(props: &DataTableProps) -> Html {
    let ldt = &props.ldt;
    let on_update = &props.on_update;

    if ldt.intensities.is_empty() || ldt.g_angles.is_empty() {
        return html! {
            <div class="text-center text-muted">
                {"No intensity data available"}
            </div>
        };
    }

    // Number of intensity data planes (Mc)
    let mc = ldt.intensities.len();

    // Starting index in c_angles array for this symmetry type
    let c_start = get_c_angle_start_index(ldt);

    // Copy to clipboard handler
    let ldt_clone = ldt.clone();
    let on_copy = Callback::from(move |_: MouseEvent| {
        let mut text = String::new();
        let mc_copy = ldt_clone.intensities.len();
        let c_start_copy = get_c_angle_start_index(&ldt_clone);

        // Header row with C angles
        text.push_str("γ\\C");
        for i in 0..mc_copy {
            text.push('\t');
            if let Some(&angle) = ldt_clone.c_angles.get(c_start_copy + i) {
                text.push_str(&format_angle(angle));
            }
        }
        text.push('\n');

        // Data rows
        for (g_idx, g_angle) in ldt_clone.g_angles.iter().enumerate() {
            text.push_str(&format_angle(*g_angle));
            for c_idx in 0..mc_copy {
                text.push('\t');
                if let Some(intensity) = ldt_clone
                    .intensities
                    .get(c_idx)
                    .and_then(|row| row.get(g_idx))
                {
                    text.push_str(&format_value(*intensity));
                }
            }
            text.push('\n');
        }

        // Copy to clipboard
        let window = gloo::utils::window();
        let clipboard = window.navigator().clipboard();
        let _ = clipboard.write_text(&text);
        gloo::console::log!("Copied to clipboard");
    });

    // Get the C-angles that correspond to intensity data
    let display_c_angles: Vec<f64> = ldt
        .c_angles
        .iter()
        .skip(c_start)
        .take(mc)
        .copied()
        .collect();

    html! {
        <div class="data-table-wrapper">
            <div class="data-table-toolbar">
                <button class="btn btn-sm btn-secondary" onclick={on_copy}>
                    {"Copy to Clipboard"}
                </button>
            </div>

            <div class="data-table-container">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th class="angle-header">{"γ \\ C"}</th>
                            {for display_c_angles.iter().enumerate().map(|(i, angle)| {
                                html! {
                                    <th key={i} class="c-angle-header">{format_angle(*angle)}</th>
                                }
                            })}
                        </tr>
                    </thead>
                    <tbody>
                        {for ldt.g_angles.iter().enumerate().map(|(g_idx, g_angle)| {
                            html! {
                                <tr key={g_idx}>
                                    <th class="g-angle-header">{format_angle(*g_angle)}</th>
                                    {for (0..mc).map(|c_idx| {
                                        let intensity = ldt.intensities.get(c_idx)
                                            .and_then(|row| row.get(g_idx))
                                            .copied()
                                            .unwrap_or(0.0);

                                        let cb = on_update.clone();
                                        html! {
                                            <td key={c_idx}>
                                                <input
                                                    type="number"
                                                    step="1"
                                                    value={format_value(intensity)}
                                                    class="intensity-input"
                                                    onchange={Callback::from(move |e: Event| {
                                                        let input: HtmlInputElement = e.target_unchecked_into();
                                                        if let Ok(v) = input.value().parse::<f64>() {
                                                            cb.emit(Msg::UpdateIntensity(c_idx, g_idx, v));
                                                        }
                                                    })}
                                                />
                                            </td>
                                        }
                                    })}
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
