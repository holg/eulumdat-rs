//! TM-32-24 BIM Parameters panel component
//!
//! Displays BIM (Building Information Modeling) parameters extracted from
//! ATLA/TM-33 photometric files when available.

use atla::{BimParameters, LuminaireOpticalData};
use crate::i18n::use_locale;
use leptos::prelude::*;

/// Check if an ATLA document has meaningful BIM data
///
/// Returns true if the document has enough data to display a BIM panel
pub fn has_bim_data(doc: &LuminaireOpticalData) -> bool {
    let bim = BimParameters::from_atla(doc);
    bim.populated_count() >= 3 // At least 3 parameters populated
}

/// BIM Panel component for displaying TM-32-24 parameters
#[component]
pub fn BimPanel(atla_doc: ReadSignal<LuminaireOpticalData>) -> impl IntoView {
    let locale = use_locale();
    view! {
        <div class="bim-panel">
            <div class="bim-header">
                <h3>{move || locale.get().ui.bim.title.clone()}</h3>
                <span class="bim-count">
                    {move || {
                        let params = BimParameters::from_atla(&atla_doc.get());
                        format!("{} parameters", params.populated_count())
                    }}
                </span>
            </div>

            // Summary line
            {move || {
                let params = BimParameters::from_atla(&atla_doc.get());
                let s = params.summary();
                if !s.is_empty() {
                    Some(view! {
                        <div class="bim-summary">{s}</div>
                    })
                } else {
                    None
                }
            }}

            // Parameter groups
            <div class="bim-groups">
                {move || {
                    let params = BimParameters::from_atla(&atla_doc.get());
                    let rows = params.to_table_rows();

                    // Group by category
                    #[allow(clippy::type_complexity)]
                    let mut groups: Vec<(String, Vec<(String, String, String)>)> = Vec::new();
                    let mut current_group: Option<&str> = None;

                    for (group, key, value, unit) in rows {
                        if current_group != Some(group) {
                            groups.push((group.to_string(), Vec::new()));
                            current_group = Some(group);
                        }
                        if let Some((_, items)) = groups.last_mut() {
                            items.push((key.to_string(), value, unit.to_string()));
                        }
                    }

                    groups.into_iter().map(|(group_name, items)| {
                        view! {
                            <div class="bim-group">
                                <h4 class="bim-group-title">{group_name}</h4>
                                <table class="bim-table">
                                    <tbody>
                                        {items.into_iter().map(|(key, value, unit)| {
                                            let display_value = if unit.is_empty() {
                                                value.clone()
                                            } else {
                                                format!("{} {}", value, unit)
                                            };
                                            view! {
                                                <tr>
                                                    <td class="bim-key">{key}</td>
                                                    <td class="bim-value">{display_value}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>

            // Export buttons
            <div class="bim-actions">
                <button
                    class="btn btn-secondary"
                    on:click=move |_| {
                        let params = BimParameters::from_atla(&atla_doc.get());
                        let csv = params.to_csv();
                        super::file_handler::download_file("bim_parameters.csv", &csv, "text/csv");
                    }
                >
                    {move || locale.get().ui.bim.export_csv.clone()}
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| {
                        let params = BimParameters::from_atla(&atla_doc.get());
                        let report = params.to_text_report();
                        super::file_handler::download_file("bim_parameters.txt", &report, "text/plain");
                    }
                >
                    {move || locale.get().ui.bim.export_report.clone()}
                </button>
            </div>

            // Info note
            <div class="bim-info">
                <p class="text-muted">
                    {move || locale.get().ui.bim.info_text.clone()}
                </p>
            </div>
        </div>
    }
}

/// Empty BIM panel shown when no BIM data is available
#[component]
pub fn BimPanelEmpty() -> impl IntoView {
    let locale = use_locale();
    view! {
        <div class="bim-panel bim-empty">
            <div class="bim-empty-message">
                <h3>{move || locale.get().ui.bim.no_data_title.clone()}</h3>
                <p>
                    {move || locale.get().ui.bim.no_data_text.clone()}
                </p>
                <p class="text-muted">
                    {move || locale.get().ui.bim.template_hint.clone()}
                </p>
            </div>
        </div>
    }
}
