use eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

// ============================================================================
// General Tab
// ============================================================================

#[component]
pub fn GeneralTab(
    ldt: ReadSignal<Eulumdat>,
    set_ldt: WriteSignal<Eulumdat>,
) -> impl IntoView {
    let on_iden_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        set_ldt.update(|ldt| ldt.identification = input.value());
    };

    let on_name_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        set_ldt.update(|ldt| ldt.luminaire_name = input.value());
    };

    let on_number_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        set_ldt.update(|ldt| ldt.luminaire_number = input.value());
    };

    let on_report_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        set_ldt.update(|ldt| ldt.measurement_report_number = input.value());
    };

    let on_filename_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        set_ldt.update(|ldt| ldt.file_name = input.value());
    };

    let on_date_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        set_ldt.update(|ldt| ldt.date_user = input.value());
    };

    let on_type_change = move |e: ev::Event| {
        let select: HtmlSelectElement = e.target().unwrap().unchecked_into();
        if let Ok(v) = select.value().parse::<i32>() {
            if let Ok(ti) = TypeIndicator::from_int(v) {
                set_ldt.update(|ldt| ldt.type_indicator = ti);
            }
        }
    };

    let on_sym_change = move |e: ev::Event| {
        let select: HtmlSelectElement = e.target().unwrap().unchecked_into();
        if let Ok(v) = select.value().parse::<i32>() {
            if let Ok(sym) = Symmetry::from_int(v) {
                set_ldt.update(|ldt| ldt.symmetry = sym);
            }
        }
    };

    view! {
        <div class="form-group">
            <label>"Identification"</label>
            <input
                type="text"
                prop:value=move || ldt.get().identification.clone()
                on:change=on_iden_change
                maxlength="80"
            />
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>"Type Indicator"</label>
                <select prop:value=move || ldt.get().type_indicator.as_int().to_string() on:change=on_type_change>
                    <option value="1">"1 - Point source (vertical symmetry)"</option>
                    <option value="2">"2 - Linear luminaire"</option>
                    <option value="3">"3 - Point source (other symmetry)"</option>
                </select>
            </div>

            <div class="form-group">
                <label>"Symmetry"</label>
                <select prop:value=move || ldt.get().symmetry.as_int().to_string() on:change=on_sym_change>
                    <option value="0">"0 - No symmetry"</option>
                    <option value="1">"1 - Vertical axis"</option>
                    <option value="2">"2 - C0-C180 plane"</option>
                    <option value="3">"3 - C90-C270 plane"</option>
                    <option value="4">"4 - Both planes"</option>
                </select>
            </div>
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>"Number of C-Planes (Nc)"</label>
                <input type="text" prop:value=move || ldt.get().num_c_planes.to_string() readonly />
            </div>
            <div class="form-group">
                <label>"C-Plane Distance (Dc)"</label>
                <input type="text" prop:value=move || format!("{:.1}°", ldt.get().c_plane_distance) readonly />
            </div>
            <div class="form-group">
                <label>"Number of G-Planes (Ng)"</label>
                <input type="text" prop:value=move || ldt.get().num_g_planes.to_string() readonly />
            </div>
            <div class="form-group">
                <label>"G-Plane Distance (Dg)"</label>
                <input type="text" prop:value=move || format!("{:.1}°", ldt.get().g_plane_distance) readonly />
            </div>
        </div>

        <div class="form-group">
            <label>"Measurement Report Number"</label>
            <input
                type="text"
                prop:value=move || ldt.get().measurement_report_number.clone()
                on:change=on_report_change
                maxlength="80"
            />
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>"Luminaire Name"</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().luminaire_name.clone()
                    on:change=on_name_change
                    maxlength="80"
                />
            </div>
            <div class="form-group">
                <label>"Luminaire Number"</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().luminaire_number.clone()
                    on:change=on_number_change
                    maxlength="80"
                />
            </div>
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>"File Name"</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().file_name.clone()
                    on:change=on_filename_change
                    maxlength="80"
                />
            </div>
            <div class="form-group">
                <label>"Date / User"</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().date_user.clone()
                    on:change=on_date_change
                    maxlength="80"
                />
            </div>
        </div>
    }
}

// ============================================================================
// Dimensions Tab
// ============================================================================

#[component]
pub fn DimensionsTab(
    ldt: ReadSignal<Eulumdat>,
    set_ldt: WriteSignal<Eulumdat>,
) -> impl IntoView {
    view! {
        <h4 class="mb-1">"Luminaire Dimensions (mm)"</h4>
        <div class="form-row">
            <div class="form-group">
                <label>"Length / Diameter (L)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().length.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.length = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"Width (B)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().width.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.width = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"Height (H)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().height.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.height = v); }
                    } />
            </div>
        </div>

        <h4 class="mb-1 mt-1">"Luminous Area (mm)"</h4>
        <div class="form-row">
            <div class="form-group">
                <label>"Length / Diameter (La)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().luminous_area_length.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.luminous_area_length = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"Width (B1)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().luminous_area_width.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.luminous_area_width = v); }
                    } />
            </div>
        </div>

        <h4 class="mb-1 mt-1">"Luminous Area Height at C-Planes (mm)"</h4>
        <div class="form-row">
            <div class="form-group">
                <label>"HC0"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().height_c0.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.height_c0 = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"HC90"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().height_c90.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.height_c90 = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"HC180"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().height_c180.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.height_c180 = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"HC270"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().height_c270.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.height_c270 = v); }
                    } />
            </div>
        </div>

        <h4 class="mb-1 mt-1">"Optical Properties"</h4>
        <div class="form-row">
            <div class="form-group">
                <label>"Downward Flux Fraction (%)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().downward_flux_fraction.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.downward_flux_fraction = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"Light Output Ratio (%)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().light_output_ratio.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.light_output_ratio = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"Conversion Factor"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().conversion_factor.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.conversion_factor = v); }
                    } />
            </div>
            <div class="form-group">
                <label>"Tilt Angle (°)"</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().tilt_angle.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.tilt_angle = v); }
                    } />
            </div>
        </div>
    }
}

// ============================================================================
// Lamp Sets Tab
// ============================================================================

#[component]
pub fn LampSetsTab(
    ldt: ReadSignal<Eulumdat>,
    set_ldt: WriteSignal<Eulumdat>,
    selected: ReadSignal<usize>,
    set_selected: WriteSignal<usize>,
) -> impl IntoView {
    let on_add = move |_: ev::MouseEvent| {
        set_ldt.update(|ldt| {
            ldt.lamp_sets.push(LampSet {
                num_lamps: 1,
                lamp_type: "LED".to_string(),
                total_luminous_flux: 1000.0,
                color_appearance: "3000K".to_string(),
                color_rendering_group: "80".to_string(),
                wattage_with_ballast: 10.0,
            });
        });
        let new_idx = ldt.get().lamp_sets.len() - 1;
        set_selected.set(new_idx);
    };

    let on_remove = move |_: ev::MouseEvent| {
        let sel = selected.get();
        set_ldt.update(|ldt| {
            if ldt.lamp_sets.len() > 1 {
                ldt.lamp_sets.remove(sel);
            }
        });
        if selected.get() >= ldt.get().lamp_sets.len() {
            set_selected.set(ldt.get().lamp_sets.len().saturating_sub(1));
        }
    };

    view! {
        <div class="lamp-set-tabs">
            {move || ldt.get().lamp_sets.iter().enumerate().map(|(i, _)| {
                let on_select = move |_: ev::MouseEvent| {
                    set_selected.set(i);
                };
                view! {
                    <button
                        class=move || format!("lamp-set-tab{}", if i == selected.get() { " active" } else { "" })
                        on:click=on_select
                    >
                        {format!("Set {}", i + 1)}
                    </button>
                }
            }).collect_view()}
            <button class="lamp-set-tab" on:click=on_add>"+"</button>
        </div>

        {move || {
            let ldt_val = ldt.get();
            let sel = selected.get();
            ldt_val.lamp_sets.get(sel).map(|ls| {
                let ls = ls.clone();
                view! {
                    <div class="form-row">
                        <div class="form-group">
                            <label>"Number of Lamps"</label>
                            <input
                                type="number"
                                prop:value=ls.num_lamps.to_string()
                                on:change=move |e: ev::Event| {
                                    let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                    if let Ok(v) = input.value().parse::<i32>() {
                                        set_ldt.update(|ldt| {
                                            if let Some(lamp) = ldt.lamp_sets.get_mut(sel) {
                                                lamp.num_lamps = v;
                                            }
                                        });
                                    }
                                }
                            />
                        </div>
                        <div class="form-group">
                            <label>"Total Luminous Flux (lm)"</label>
                            <input
                                type="number"
                                step="1"
                                prop:value=ls.total_luminous_flux.to_string()
                                on:change=move |e: ev::Event| {
                                    let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                    if let Ok(v) = input.value().parse::<f64>() {
                                        set_ldt.update(|ldt| {
                                            if let Some(lamp) = ldt.lamp_sets.get_mut(sel) {
                                                lamp.total_luminous_flux = v;
                                            }
                                        });
                                    }
                                }
                            />
                        </div>
                        <div class="form-group">
                            <label>"Wattage incl. Ballast (W)"</label>
                            <input
                                type="number"
                                step="0.1"
                                prop:value=ls.wattage_with_ballast.to_string()
                                on:change=move |e: ev::Event| {
                                    let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                    if let Ok(v) = input.value().parse::<f64>() {
                                        set_ldt.update(|ldt| {
                                            if let Some(lamp) = ldt.lamp_sets.get_mut(sel) {
                                                lamp.wattage_with_ballast = v;
                                            }
                                        });
                                    }
                                }
                            />
                        </div>
                    </div>

                    <div class="form-group">
                        <label>"Lamp Type"</label>
                        <input
                            type="text"
                            prop:value=ls.lamp_type.clone()
                            on:change=move |e: ev::Event| {
                                let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                set_ldt.update(|ldt| {
                                    if let Some(lamp) = ldt.lamp_sets.get_mut(sel) {
                                        lamp.lamp_type = input.value();
                                    }
                                });
                            }
                            maxlength="40"
                        />
                    </div>

                    <div class="form-row">
                        <div class="form-group">
                            <label>"Color Appearance / CCT"</label>
                            <input
                                type="text"
                                prop:value=ls.color_appearance.clone()
                                on:change=move |e: ev::Event| {
                                    let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                    set_ldt.update(|ldt| {
                                        if let Some(lamp) = ldt.lamp_sets.get_mut(sel) {
                                            lamp.color_appearance = input.value();
                                        }
                                    });
                                }
                                maxlength="40"
                            />
                        </div>
                        <div class="form-group">
                            <label>"Color Rendering Group / CRI"</label>
                            <input
                                type="text"
                                prop:value=ls.color_rendering_group.clone()
                                on:change=move |e: ev::Event| {
                                    let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                    set_ldt.update(|ldt| {
                                        if let Some(lamp) = ldt.lamp_sets.get_mut(sel) {
                                            lamp.color_rendering_group = input.value();
                                        }
                                    });
                                }
                                maxlength="40"
                            />
                        </div>
                    </div>

                    {if ldt_val.lamp_sets.len() > 1 {
                        Some(view! {
                            <button class="btn btn-secondary mt-1" on:click=on_remove>
                                "Remove this lamp set"
                            </button>
                        })
                    } else {
                        None
                    }}
                }
            })
        }}
    }
}

// ============================================================================
// Direct Ratios Tab
// ============================================================================

#[component]
pub fn DirectRatiosTab(
    ldt: ReadSignal<Eulumdat>,
    set_ldt: WriteSignal<Eulumdat>,
) -> impl IntoView {
    let k_values = [
        "0.60", "0.80", "1.00", "1.25", "1.50", "2.00", "2.50", "3.00", "4.00", "5.00",
    ];

    let on_calculate = move |_: ev::MouseEvent| {
        let ldt_val = ldt.get();
        let ratios = eulumdat::PhotometricCalculations::calculate_direct_ratios(&ldt_val, "1.00");
        set_ldt.update(|ldt| {
            for (i, r) in ratios.iter().enumerate() {
                if i < ldt.direct_ratios.len() {
                    ldt.direct_ratios[i] = *r;
                }
            }
        });
    };

    view! {
        <p class="text-muted mb-1">
            "Direct ratios for room indices (k values). These are utilization factors."
        </p>

        <div class="ratios-grid">
            {k_values.iter().enumerate().map(|(i, k)| {
                view! {
                    <div class="ratio-item">
                        <label>{format!("k={}", k)}</label>
                        <input
                            type="number"
                            step="0.001"
                            min="0"
                            max="1"
                            prop:value=move || format!("{:.3}", ldt.get().direct_ratios.get(i).copied().unwrap_or(0.0))
                            on:change=move |e: ev::Event| {
                                let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                                if let Ok(v) = input.value().parse::<f64>() {
                                    set_ldt.update(|ldt| {
                                        if i < ldt.direct_ratios.len() {
                                            ldt.direct_ratios[i] = v;
                                        }
                                    });
                                }
                            }
                        />
                    </div>
                }
            }).collect_view()}
        </div>

        <div class="mt-1">
            <button class="btn btn-secondary" on:click=on_calculate>
                "Calculate from intensity data"
            </button>
        </div>
    }
}
