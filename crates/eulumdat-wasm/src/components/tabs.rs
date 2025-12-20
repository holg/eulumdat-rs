use crate::i18n::use_locale;
use eulumdat::{
    Eulumdat, GldfPhotometricData, LampSet, PhotometricSummary, Symmetry, TypeIndicator,
};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

// ============================================================================
// General Tab
// ============================================================================

#[component]
pub fn GeneralTab(ldt: ReadSignal<Eulumdat>, set_ldt: WriteSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

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
            <label>{move || locale.get().luminaire.info.identification.clone()}</label>
            <input
                type="text"
                prop:value=move || ldt.get().identification.clone()
                on:change=on_iden_change
                maxlength="80"
            />
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.type_indicator.clone()}</label>
                <select prop:value=move || ldt.get().type_indicator.as_int().to_string() on:change=on_type_change>
                    <option value="1">{move || locale.get().luminaire.info.type_indicator_1.clone()}</option>
                    <option value="2">{move || locale.get().luminaire.info.type_indicator_2.clone()}</option>
                    <option value="3">{move || locale.get().luminaire.info.type_indicator_3.clone()}</option>
                </select>
            </div>

            <div class="form-group">
                <label>{move || locale.get().luminaire.info.symmetry.clone()}</label>
                <select prop:value=move || ldt.get().symmetry.as_int().to_string() on:change=on_sym_change>
                    <option value="0">{move || locale.get().luminaire.info.symmetry_0.clone()}</option>
                    <option value="1">{move || locale.get().luminaire.info.symmetry_1.clone()}</option>
                    <option value="2">{move || locale.get().luminaire.info.symmetry_2.clone()}</option>
                    <option value="3">{move || locale.get().luminaire.info.symmetry_3.clone()}</option>
                    <option value="4">{move || locale.get().luminaire.info.symmetry_4.clone()}</option>
                </select>
            </div>
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.num_c_planes.clone()}</label>
                <input type="text" prop:value=move || ldt.get().num_c_planes.to_string() readonly />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.c_plane_distance.clone()}</label>
                <input type="text" prop:value=move || format!("{:.1}°", ldt.get().c_plane_distance) readonly />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.num_g_planes.clone()}</label>
                <input type="text" prop:value=move || ldt.get().num_g_planes.to_string() readonly />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.g_plane_distance.clone()}</label>
                <input type="text" prop:value=move || format!("{:.1}°", ldt.get().g_plane_distance) readonly />
            </div>
        </div>

        <div class="form-group">
            <label>{move || locale.get().luminaire.info.measurement_report.clone()}</label>
            <input
                type="text"
                prop:value=move || ldt.get().measurement_report_number.clone()
                on:change=on_report_change
                maxlength="80"
            />
        </div>

        <div class="form-row">
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.luminaire_name.clone()}</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().luminaire_name.clone()
                    on:change=on_name_change
                    maxlength="80"
                />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.luminaire_number.clone()}</label>
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
                <label>{move || locale.get().luminaire.info.file_name.clone()}</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().file_name.clone()
                    on:change=on_filename_change
                    maxlength="80"
                />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.info.date_user.clone()}</label>
                <input
                    type="text"
                    prop:value=move || ldt.get().date_user.clone()
                    on:change=on_date_change
                    maxlength="80"
                />
            </div>
        </div>

        // Photometric Summary (Calculated Values)
        <div class="calculated-summary">
            <h4 class="summary-header">{move || locale.get().luminaire.summary.title.clone()}</h4>
            <p class="text-muted text-small">{move || locale.get().luminaire.summary.description.clone()}</p>

            <div class="info-grid-wide">
                // Flux & Efficiency
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.total_lamp_flux.clone()}</div>
                    <div class="info-value">{move || format!("{:.0} lm", ldt.get().total_luminous_flux())}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.total_wattage.clone()}</div>
                    <div class="info-value">{move || format!("{:.1} W", ldt.get().total_wattage())}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.lamp_efficacy.clone()}</div>
                    <div class="info-value">{move || format!("{:.1} lm/W", ldt.get().luminous_efficacy())}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.luminaire_efficacy.clone()}</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1} lm/W", summary.luminaire_efficacy)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.lor.clone()}</div>
                    <div class="info-value">{move || format!("{:.1}%", ldt.get().light_output_ratio)}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">"DLOR / ULOR"</div>
                    <div class="info-value">{move || {
                        let l = ldt.get();
                        format!("{:.1}% / {:.1}%", l.downward_flux_fraction, 100.0 - l.downward_flux_fraction)
                    }}</div>
                </div>
            </div>

            <h5 class="subsection-header">{move || locale.get().luminaire.summary.beam_characteristics.clone()}</h5>
            <div class="info-grid-wide">
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.max_intensity.clone()}</div>
                    <div class="info-value">{move || format!("{:.1} cd/klm", ldt.get().max_intensity())}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.beam_angle_50.clone()}</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1}°", summary.beam_angle)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.field_angle_10.clone()}</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1}°", summary.field_angle)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.spacing_criterion.clone()}</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.2} × {:.2}", summary.spacing_c0, summary.spacing_c90)
                    }}</div>
                </div>
            </div>

            <h5 class="subsection-header">{move || locale.get().luminaire.photometric.cie_class.clone()}</h5>
            <div class="info-grid-wide">
                <div class="info-item" style="grid-column: span 2;">
                    <div class="info-label">"CIE Flux Code"</div>
                    <div class="info-value mono">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{}", summary.cie_flux_codes)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.photometric_code.clone()}</div>
                    <div class="info-value">{move || {
                        let gldf = GldfPhotometricData::from_eulumdat(&ldt.get());
                        gldf.photometric_code
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.photometric.cutoff_angle.clone()}</div>
                    <div class="info-value">{move || {
                        let gldf = GldfPhotometricData::from_eulumdat(&ldt.get());
                        format!("{:.1}°", gldf.cut_off_angle)
                    }}</div>
                </div>
            </div>

            <h5 class="subsection-header">{move || locale.get().luminaire.summary.zonal_lumens.clone()}</h5>
            <div class="info-grid-wide">
                <div class="info-item">
                    <div class="info-label">"0-30°"</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1}%", summary.zonal_lumens.zone_0_30)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">"30-60°"</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1}%", summary.zonal_lumens.zone_30_60)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">"60-90°"</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1}%", summary.zonal_lumens.zone_60_90)
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">"90-180°"</div>
                    <div class="info-value">{move || {
                        let summary = PhotometricSummary::from_eulumdat(&ldt.get());
                        format!("{:.1}%", summary.zonal_lumens.upward_total())
                    }}</div>
                </div>
            </div>

            <h5 class="subsection-header">{move || locale.get().luminaire.summary.glare_assessment.clone()}</h5>
            <div class="info-grid-wide">
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.summary.luminaire_luminance.clone()}</div>
                    <div class="info-value">{move || {
                        let gldf = GldfPhotometricData::from_eulumdat(&ldt.get());
                        if gldf.luminaire_luminance > 0.0 {
                            format!("{:.0} cd/m²", gldf.luminaire_luminance)
                        } else {
                            "N/A".to_string()
                        }
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">"UGR Crosswise (C90)"</div>
                    <div class="info-value">{move || {
                        let gldf = GldfPhotometricData::from_eulumdat(&ldt.get());
                        match &gldf.ugr_4h_8h_705020 {
                            Some(ugr) => format!("{:.1}", ugr.crosswise),
                            None => "N/A".to_string(),
                        }
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">"UGR Endwise (C0)"</div>
                    <div class="info-value">{move || {
                        let gldf = GldfPhotometricData::from_eulumdat(&ldt.get());
                        match &gldf.ugr_4h_8h_705020 {
                            Some(ugr) => format!("{:.1}", ugr.endwise),
                            None => "N/A".to_string(),
                        }
                    }}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">{move || locale.get().luminaire.summary.room_config.clone()}</div>
                    <div class="info-value text-muted">"4H×8H, 70/50/20"</div>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Dimensions Tab
// ============================================================================

#[component]
pub fn DimensionsTab(ldt: ReadSignal<Eulumdat>, set_ldt: WriteSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    view! {
        <h4 class="mb-1">{move || locale.get().luminaire.physical.dimensions_mm.clone()}</h4>
        <div class="form-row">
            <div class="form-group">
                <label>{move || locale.get().luminaire.physical.length_diameter.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().length.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.length = v); }
                    } />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.physical.width_b.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().width.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.width = v); }
                    } />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.physical.height_h.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().height.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.height = v); }
                    } />
            </div>
        </div>

        <h4 class="mb-1 mt-1">{move || locale.get().luminaire.physical.luminous_area_mm.clone()}</h4>
        <div class="form-row">
            <div class="form-group">
                <label>{move || locale.get().luminaire.physical.luminous_length.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().luminous_area_length.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.luminous_area_length = v); }
                    } />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.physical.luminous_width.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().luminous_area_width.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.luminous_area_width = v); }
                    } />
            </div>
        </div>

        <h4 class="mb-1 mt-1">{move || locale.get().luminaire.physical.luminous_height_c_planes.clone()}</h4>
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

        <h4 class="mb-1 mt-1">{move || locale.get().luminaire.optical.title.clone()}</h4>
        <div class="form-row">
            <div class="form-group">
                <label>{move || locale.get().luminaire.optical.downward_flux_fraction.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().downward_flux_fraction.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.downward_flux_fraction = v); }
                    } />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.optical.light_output_ratio.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().light_output_ratio.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.light_output_ratio = v); }
                    } />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.optical.conversion_factor.clone()}</label>
                <input type="number" step="0.1" prop:value=move || ldt.get().conversion_factor.to_string()
                    on:change=move |e: ev::Event| {
                        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
                        if let Ok(v) = input.value().parse::<f64>() { set_ldt.update(|l| l.conversion_factor = v); }
                    } />
            </div>
            <div class="form-group">
                <label>{move || locale.get().luminaire.optical.tilt_angle.clone()}</label>
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
    let locale = use_locale();

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
        <div class="lamp-set-header">
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
                            {move || locale.get().luminaire.lamp_set.set_n.replace("{n}", &(i + 1).to_string())}
                        </button>
                    }
                }).collect_view()}
            </div>
            <button class="btn btn-primary btn-add-lamp" on:click=on_add>
                "+ Add"
            </button>
        </div>

        {move || {
            let ldt_val = ldt.get();
            let sel = selected.get();
            ldt_val.lamp_sets.get(sel).map(|ls| {
                let ls = ls.clone();
                view! {
                    <div class="form-row">
                        <div class="form-group">
                            <label>{move || locale.get().luminaire.lamp_set.num_lamps.clone()}</label>
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
                            <label>{move || locale.get().luminaire.lamp_set.luminous_flux.clone()}</label>
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
                            <label>{move || locale.get().luminaire.lamp_set.wattage.clone()}</label>
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
                        <label>{move || locale.get().luminaire.lamp_set.lamp_type.clone()}</label>
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
                            <label>{move || locale.get().luminaire.lamp_set.color_appearance.clone()}</label>
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
                            <label>{move || locale.get().luminaire.lamp_set.color_rendering.clone()}</label>
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
                                {move || locale.get().luminaire.lamp_set.remove.clone()}
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
pub fn DirectRatiosTab(ldt: ReadSignal<Eulumdat>, set_ldt: WriteSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

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
            {move || locale.get().luminaire.direct_ratios.description.clone()}
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
                {move || locale.get().luminaire.direct_ratios.calculate.clone()}
            </button>
        </div>
    }
}
