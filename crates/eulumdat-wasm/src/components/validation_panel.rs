use crate::i18n::use_locale;
use atla::validate::{validate_with_schema, ValidationSchema};
use atla::LuminaireOpticalData;
use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn ValidationPanel(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();

    move || {
        let ldt = ldt.get();
        let l = locale.get();
        let warnings = ldt.validate();
        let strict_result = ldt.validate_strict();

        let has_errors = strict_result.is_err();
        let errors = strict_result.err().unwrap_or_default();

        // Convert to ATLA for schema validation
        let atla_doc = LuminaireOpticalData::from_eulumdat(&ldt);
        let s001_result = validate_with_schema(&atla_doc, ValidationSchema::AtlaS001);
        let tm33_result = validate_with_schema(&atla_doc, ValidationSchema::Tm3323);

        let s001_valid = s001_result.is_valid();
        let tm33_valid = tm33_result.is_valid();

        view! {
            <div class="validation-panel">
                // ATLA Schema Validation Section
                <div class="schema-validation-section">
                    <div class="validation-header">"ATLA Schema Validation"</div>

                    // ATLA S001 Status
                    <div class={if s001_valid { "validation-item success" } else { "validation-item error" }}>
                        <span class="schema-badge">"S001"</span>
                        <span>{if s001_valid { "VALID" } else { "INVALID" }}</span>
                        {if !s001_valid {
                            view! {
                                <span class="error-count">
                                    {format!("({} errors)", s001_result.errors.len())}
                                </span>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>

                    // TM-33-23 Status
                    <div class={if tm33_valid { "validation-item success" } else { "validation-item warning" }}>
                        <span class="schema-badge">"TM-33-23"</span>
                        <span>{if tm33_valid { "VALID" } else { "INVALID" }}</span>
                        {if !tm33_valid {
                            view! {
                                <span class="error-count">
                                    {format!("({} errors)", tm33_result.errors.len())}
                                </span>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>

                    // Show TM-33-23 errors if any (collapsed by default)
                    {if !tm33_valid {
                        view! {
                            <details class="tm33-errors">
                                <summary class="text-muted">"TM-33-23 missing fields"</summary>
                                <div class="error-details">
                                    {tm33_result.errors.iter().map(|err| {
                                        view! {
                                            <div class="validation-item error small">
                                                <span class="validation-code">{err.code.clone()}</span>
                                                <span>{err.message.clone()}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </details>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>

                <hr class="validation-divider" />

                // LDT Validation Section
                <div class="ldt-validation-section">
                    <div class="validation-header">"LDT/IES Validation"</div>

                    {if warnings.is_empty() && !has_errors {
                        view! {
                            <div class="validation-item success">
                                <span>"✓"</span>
                                <span>{l.ui.validation_panel.all_passed.clone()}</span>
                            </div>
                        }.into_any()
                    } else {
                        let error_count_str = l.ui.validation_panel.error_count
                            .replace("{errors}", &errors.len().to_string())
                            .replace("{warnings}", &warnings.len().to_string());

                        view! {
                            <div>
                                // Critical errors first
                                {errors.iter().map(|error| {
                                    let code = error.code;
                                    let message = error.message.clone();
                                    view! {
                                        <div class="validation-item error">
                                            <span class="validation-code">{code}</span>
                                            <span>{message}</span>
                                        </div>
                                    }
                                }).collect_view()}

                                // Warnings
                                {warnings.iter().map(|warning| {
                                    let code = warning.code;
                                    let message = warning.message.clone();
                                    view! {
                                        <div class="validation-item warning">
                                            <span class="validation-code">{code}</span>
                                            <span>{message}</span>
                                        </div>
                                    }
                                }).collect_view()}

                                // Summary
                                <div class="text-muted mt-1" style="font-size: 0.75rem;">
                                    {error_count_str}
                                </div>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        }
        .into_any()
    }
}
