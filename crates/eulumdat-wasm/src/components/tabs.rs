use eulumdat::Eulumdat;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use super::app::Msg;

// ============================================================================
// General Tab
// ============================================================================

#[derive(Properties, PartialEq)]
pub struct GeneralTabProps {
    pub ldt: Eulumdat,
    pub on_update: Callback<Msg>,
}

#[function_component(GeneralTab)]
pub fn general_tab(props: &GeneralTabProps) -> Html {
    let ldt = &props.ldt;
    let on_update = &props.on_update;

    let on_iden_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            cb.emit(Msg::UpdateIdentification(input.value()));
        })
    };

    let on_name_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            cb.emit(Msg::UpdateLuminaireName(input.value()));
        })
    };

    let on_number_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            cb.emit(Msg::UpdateLuminaireNumber(input.value()));
        })
    };

    let on_report_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            cb.emit(Msg::UpdateMeasurementReport(input.value()));
        })
    };

    let on_filename_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            cb.emit(Msg::UpdateFileName(input.value()));
        })
    };

    let on_date_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            cb.emit(Msg::UpdateDateUser(input.value()));
        })
    };

    let on_type_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<i32>() {
                cb.emit(Msg::UpdateTypeIndicator(v));
            }
        })
    };

    let on_sym_change = {
        let cb = on_update.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<i32>() {
                cb.emit(Msg::UpdateSymmetry(v));
            }
        })
    };

    html! {
        <>
            <div class="form-group">
                <label>{"Identification"}</label>
                <input
                    type="text"
                    value={ldt.identification.clone()}
                    onchange={on_iden_change}
                    maxlength="80"
                />
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label>{"Type Indicator"}</label>
                    <select value={ldt.type_indicator.as_int().to_string()} onchange={on_type_change}>
                        <option value="1" selected={ldt.type_indicator.as_int() == 1}>
                            {"1 - Point source (vertical symmetry)"}
                        </option>
                        <option value="2" selected={ldt.type_indicator.as_int() == 2}>
                            {"2 - Linear luminaire"}
                        </option>
                        <option value="3" selected={ldt.type_indicator.as_int() == 3}>
                            {"3 - Point source (other symmetry)"}
                        </option>
                    </select>
                </div>

                <div class="form-group">
                    <label>{"Symmetry"}</label>
                    <select value={ldt.symmetry.as_int().to_string()} onchange={on_sym_change}>
                        <option value="0" selected={ldt.symmetry.as_int() == 0}>
                            {"0 - No symmetry"}
                        </option>
                        <option value="1" selected={ldt.symmetry.as_int() == 1}>
                            {"1 - Vertical axis"}
                        </option>
                        <option value="2" selected={ldt.symmetry.as_int() == 2}>
                            {"2 - C0-C180 plane"}
                        </option>
                        <option value="3" selected={ldt.symmetry.as_int() == 3}>
                            {"3 - C90-C270 plane"}
                        </option>
                        <option value="4" selected={ldt.symmetry.as_int() == 4}>
                            {"4 - Both planes"}
                        </option>
                    </select>
                </div>
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label>{"Number of C-Planes (Nc)"}</label>
                    <input type="text" value={ldt.num_c_planes.to_string()} readonly=true />
                </div>
                <div class="form-group">
                    <label>{"C-Plane Distance (Dc)"}</label>
                    <input type="text" value={format!("{:.1}°", ldt.c_plane_distance)} readonly=true />
                </div>
                <div class="form-group">
                    <label>{"Number of G-Planes (Ng)"}</label>
                    <input type="text" value={ldt.num_g_planes.to_string()} readonly=true />
                </div>
                <div class="form-group">
                    <label>{"G-Plane Distance (Dg)"}</label>
                    <input type="text" value={format!("{:.1}°", ldt.g_plane_distance)} readonly=true />
                </div>
            </div>

            <div class="form-group">
                <label>{"Measurement Report Number"}</label>
                <input
                    type="text"
                    value={ldt.measurement_report_number.clone()}
                    onchange={on_report_change}
                    maxlength="80"
                />
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label>{"Luminaire Name"}</label>
                    <input
                        type="text"
                        value={ldt.luminaire_name.clone()}
                        onchange={on_name_change}
                        maxlength="80"
                    />
                </div>
                <div class="form-group">
                    <label>{"Luminaire Number"}</label>
                    <input
                        type="text"
                        value={ldt.luminaire_number.clone()}
                        onchange={on_number_change}
                        maxlength="80"
                    />
                </div>
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label>{"File Name"}</label>
                    <input
                        type="text"
                        value={ldt.file_name.clone()}
                        onchange={on_filename_change}
                        maxlength="80"
                    />
                </div>
                <div class="form-group">
                    <label>{"Date / User"}</label>
                    <input
                        type="text"
                        value={ldt.date_user.clone()}
                        onchange={on_date_change}
                        maxlength="80"
                    />
                </div>
            </div>
        </>
    }
}

// ============================================================================
// Dimensions Tab
// ============================================================================

#[derive(Properties, PartialEq)]
pub struct DimensionsTabProps {
    pub ldt: Eulumdat,
    pub on_update: Callback<Msg>,
}

#[function_component(DimensionsTab)]
pub fn dimensions_tab(props: &DimensionsTabProps) -> Html {
    let ldt = &props.ldt;
    let on_update = &props.on_update;

    macro_rules! float_input {
        ($label:expr, $value:expr, $msg:ident) => {{
            let cb = on_update.clone();
            html! {
                <div class="form-group">
                    <label>{$label}</label>
                    <input
                        type="number"
                        step="0.1"
                        value={$value.to_string()}
                        onchange={Callback::from(move |e: Event| {
                            let input: HtmlInputElement = e.target_unchecked_into();
                            if let Ok(v) = input.value().parse::<f64>() {
                                cb.emit(Msg::$msg(v));
                            }
                        })}
                    />
                </div>
            }
        }};
    }

    html! {
        <>
            <h4 class="mb-1">{"Luminaire Dimensions (mm)"}</h4>
            <div class="form-row">
                {float_input!("Length / Diameter (L)", ldt.length, UpdateLength)}
                {float_input!("Width (B)", ldt.width, UpdateWidth)}
                {float_input!("Height (H)", ldt.height, UpdateHeight)}
            </div>

            <h4 class="mb-1 mt-1">{"Luminous Area (mm)"}</h4>
            <div class="form-row">
                {float_input!("Length / Diameter (La)", ldt.luminous_area_length, UpdateLuminousAreaLength)}
                {float_input!("Width (B1)", ldt.luminous_area_width, UpdateLuminousAreaWidth)}
            </div>

            <h4 class="mb-1 mt-1">{"Luminous Area Height at C-Planes (mm)"}</h4>
            <div class="form-row">
                {float_input!("HC0", ldt.height_c0, UpdateHeightC0)}
                {float_input!("HC90", ldt.height_c90, UpdateHeightC90)}
                {float_input!("HC180", ldt.height_c180, UpdateHeightC180)}
                {float_input!("HC270", ldt.height_c270, UpdateHeightC270)}
            </div>

            <h4 class="mb-1 mt-1">{"Optical Properties"}</h4>
            <div class="form-row">
                {float_input!("Downward Flux Fraction (%)", ldt.downward_flux_fraction, UpdateDownwardFlux)}
                {float_input!("Light Output Ratio (%)", ldt.light_output_ratio, UpdateLightOutputRatio)}
                {float_input!("Conversion Factor", ldt.conversion_factor, UpdateConversionFactor)}
                {float_input!("Tilt Angle (°)", ldt.tilt_angle, UpdateTiltAngle)}
            </div>
        </>
    }
}

// ============================================================================
// Lamp Sets Tab
// ============================================================================

#[derive(Properties, PartialEq)]
pub struct LampSetsTabProps {
    pub ldt: Eulumdat,
    pub selected: usize,
    pub on_update: Callback<Msg>,
}

#[function_component(LampSetsTab)]
pub fn lamp_sets_tab(props: &LampSetsTabProps) -> Html {
    let ldt = &props.ldt;
    let selected = props.selected;
    let on_update = &props.on_update;

    let lamp_set = ldt.lamp_sets.get(selected);

    let on_select = |idx: usize| {
        let cb = on_update.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(Msg::SelectLampSet(idx));
        })
    };

    let on_add = {
        let cb = on_update.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(Msg::AddLampSet);
        })
    };

    let on_remove = {
        let cb = on_update.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit(Msg::RemoveLampSet);
        })
    };

    html! {
        <>
            <div class="lamp-set-tabs">
                {for ldt.lamp_sets.iter().enumerate().map(|(i, _)| {
                    html! {
                        <button
                            class={classes!("lamp-set-tab", (i == selected).then_some("active"))}
                            onclick={on_select(i)}
                        >
                            {format!("Set {}", i + 1)}
                        </button>
                    }
                })}
                <button class="lamp-set-tab" onclick={on_add}>{"+"}</button>
            </div>

            if let Some(ls) = lamp_set {
                <div class="form-row">
                    <div class="form-group">
                        <label>{"Number of Lamps"}</label>
                        <input
                            type="number"
                            value={ls.num_lamps.to_string()}
                            onchange={{
                                let cb = on_update.clone();
                                Callback::from(move |e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    if let Ok(v) = input.value().parse::<i32>() {
                                        cb.emit(Msg::UpdateLampSetNumLamps(v));
                                    }
                                })
                            }}
                        />
                    </div>
                    <div class="form-group">
                        <label>{"Total Luminous Flux (lm)"}</label>
                        <input
                            type="number"
                            step="1"
                            value={ls.total_luminous_flux.to_string()}
                            onchange={{
                                let cb = on_update.clone();
                                Callback::from(move |e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    if let Ok(v) = input.value().parse::<f64>() {
                                        cb.emit(Msg::UpdateLampSetFlux(v));
                                    }
                                })
                            }}
                        />
                    </div>
                    <div class="form-group">
                        <label>{"Wattage incl. Ballast (W)"}</label>
                        <input
                            type="number"
                            step="0.1"
                            value={ls.wattage_with_ballast.to_string()}
                            onchange={{
                                let cb = on_update.clone();
                                Callback::from(move |e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    if let Ok(v) = input.value().parse::<f64>() {
                                        cb.emit(Msg::UpdateLampSetWattage(v));
                                    }
                                })
                            }}
                        />
                    </div>
                </div>

                <div class="form-group">
                    <label>{"Lamp Type"}</label>
                    <input
                        type="text"
                        value={ls.lamp_type.clone()}
                        onchange={{
                            let cb = on_update.clone();
                            Callback::from(move |e: Event| {
                                let input: HtmlInputElement = e.target_unchecked_into();
                                cb.emit(Msg::UpdateLampSetType(input.value()));
                            })
                        }}
                        maxlength="40"
                    />
                </div>

                <div class="form-row">
                    <div class="form-group">
                        <label>{"Color Appearance / CCT"}</label>
                        <input
                            type="text"
                            value={ls.color_appearance.clone()}
                            onchange={{
                                let cb = on_update.clone();
                                Callback::from(move |e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    cb.emit(Msg::UpdateLampSetColorAppearance(input.value()));
                                })
                            }}
                            maxlength="40"
                        />
                    </div>
                    <div class="form-group">
                        <label>{"Color Rendering Group / CRI"}</label>
                        <input
                            type="text"
                            value={ls.color_rendering_group.clone()}
                            onchange={{
                                let cb = on_update.clone();
                                Callback::from(move |e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    cb.emit(Msg::UpdateLampSetColorRendering(input.value()));
                                })
                            }}
                            maxlength="40"
                        />
                    </div>
                </div>

                if ldt.lamp_sets.len() > 1 {
                    <button class="btn btn-secondary mt-1" onclick={on_remove}>
                        {"Remove this lamp set"}
                    </button>
                }
            }
        </>
    }
}

// ============================================================================
// Direct Ratios Tab
// ============================================================================

#[derive(Properties, PartialEq)]
pub struct DirectRatiosTabProps {
    pub ldt: Eulumdat,
    pub on_update: Callback<Msg>,
}

#[function_component(DirectRatiosTab)]
pub fn direct_ratios_tab(props: &DirectRatiosTabProps) -> Html {
    let ldt = &props.ldt;
    let on_update = &props.on_update;

    let k_values = [
        "0.60", "0.80", "1.00", "1.25", "1.50", "2.00", "2.50", "3.00", "4.00", "5.00",
    ];

    html! {
        <>
            <p class="text-muted mb-1">
                {"Direct ratios for room indices (k values). These are utilization factors."}
            </p>

            <div class="ratios-grid">
                {for k_values.iter().enumerate().map(|(i, k)| {
                    let cb = on_update.clone();
                    html! {
                        <div class="ratio-item">
                            <label>{format!("k={}", k)}</label>
                            <input
                                type="number"
                                step="0.001"
                                min="0"
                                max="1"
                                value={format!("{:.3}", ldt.direct_ratios[i])}
                                onchange={Callback::from(move |e: Event| {
                                    let input: HtmlInputElement = e.target_unchecked_into();
                                    if let Ok(v) = input.value().parse::<f64>() {
                                        cb.emit(Msg::UpdateDirectRatio(i, v));
                                    }
                                })}
                            />
                        </div>
                    }
                })}
            </div>

            <div class="mt-1">
                <button class="btn btn-secondary" onclick={{
                    let cb = on_update.clone();
                    let ldt_clone = ldt.clone();
                    Callback::from(move |_: MouseEvent| {
                        // Calculate new direct ratios
                        let ratios = eulumdat::PhotometricCalculations::calculate_direct_ratios(&ldt_clone, "1.00");
                        for (i, r) in ratios.iter().enumerate() {
                            cb.emit(Msg::UpdateDirectRatio(i, *r));
                        }
                    })
                }}>
                    {"Calculate from intensity data"}
                </button>
            </div>
        </>
    }
}
