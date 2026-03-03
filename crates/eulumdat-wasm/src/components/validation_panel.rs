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
        let tm32_result = validate_with_schema(&atla_doc, ValidationSchema::Tm3224);

        let s001_valid = s001_result.is_valid();
        let tm33_valid = tm33_result.is_valid();

        // Count TM-32-24 specific errors/warnings (excluding TM-33-23 errors which are included)
        let tm32_errors: Vec<_> = tm32_result
            .errors
            .iter()
            .filter(|e| e.code.starts_with("TM32-"))
            .collect();
        let tm32_warnings: Vec<_> = tm32_result
            .warnings
            .iter()
            .filter(|w| w.code.starts_with("TM32-"))
            .collect();

        // TM-32-24 is valid if no TM-32 specific errors AND TM-33-23 is valid
        let tm32_valid = tm32_errors.is_empty() && tm32_warnings.is_empty() && tm33_valid;

        view! {
            <div class="validation-panel">
                // ATLA Schema Validation Section
                <div class="schema-validation-section">
                    <div class="validation-header">"Schema Validation"</div>

                    <div class="schema-grid">
                        // ATLA S001 Status
                        <div class={if s001_valid { "schema-card valid" } else { "schema-card invalid" }}>
                            <div class="schema-name">"ATLA S001"</div>
                            <div class="schema-status">
                                {if s001_valid { "✓ Valid" } else { "✗ Invalid" }}
                            </div>
                            {if !s001_valid {
                                view! {
                                    <div class="schema-count">
                                        {format!("{} errors", s001_result.errors.len())}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>

                        // TM-33-23 Status
                        <div class={if tm33_valid { "schema-card valid" } else { "schema-card warning" }}>
                            <div class="schema-name">"TM-33-23"</div>
                            <div class="schema-status">
                                {if tm33_valid { "✓ Valid" } else { "⚠ Missing fields" }}
                            </div>
                            {if !tm33_valid {
                                view! {
                                    <div class="schema-count">
                                        {format!("{} errors", tm33_result.errors.len())}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>

                        // TM-32-24 BIM Status
                        <div class={
                            if tm32_valid {
                                "schema-card valid"
                            } else if tm32_errors.is_empty() && !tm32_warnings.is_empty() {
                                "schema-card warning"
                            } else if !tm32_errors.is_empty() {
                                "schema-card invalid"
                            } else {
                                // No TM32 issues but TM33 has issues - show as dependent
                                "schema-card warning"
                            }
                        }>
                            <div class="schema-name">"TM-32-24 BIM"</div>
                            <div class="schema-status">
                                {if tm32_valid {
                                    "✓ Valid"
                                } else if !tm32_errors.is_empty() {
                                    "✗ Invalid"
                                } else if !tm32_warnings.is_empty() {
                                    "⚠ Warnings"
                                } else {
                                    // TM-33-23 has issues
                                    "⚠ See TM-33-23"
                                }}
                            </div>
                            {if !tm32_errors.is_empty() || !tm32_warnings.is_empty() {
                                view! {
                                    <div class="schema-count">
                                        {format!("{} errors, {} warnings", tm32_errors.len(), tm32_warnings.len())}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </div>
                    </div>

                    // Show TM-33-23 errors if any
                    {if !tm33_valid {
                        let errors_clone = tm33_result.errors.clone();
                        view! {
                            <details class="validation-details" open>
                                <summary>"TM-33-23 Issues"</summary>
                                <div class="validation-list">
                                    {errors_clone.into_iter().map(|err| {
                                        view! {
                                            <div class="validation-item error">
                                                <span class="validation-code">{err.code}</span>
                                                <span class="validation-message">{err.message}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </details>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}

                    // Show TM-32-24 errors/warnings if any
                    {if !tm32_errors.is_empty() || !tm32_warnings.is_empty() {
                        let errors_clone: Vec<_> = tm32_errors.iter().map(|e| (e.code.clone(), e.message.clone())).collect();
                        let warnings_clone: Vec<_> = tm32_warnings.iter().map(|w| (w.code.clone(), w.message.clone())).collect();
                        view! {
                            <details class="validation-details" open>
                                <summary>"TM-32-24 BIM Issues"</summary>
                                <div class="validation-list">
                                    {errors_clone.into_iter().map(|(code, message)| {
                                        view! {
                                            <div class="validation-item error">
                                                <span class="validation-code">{code}</span>
                                                <span class="validation-message">{message}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                    {warnings_clone.into_iter().map(|(code, message)| {
                                        view! {
                                            <div class="validation-item warning">
                                                <span class="validation-code">{code}</span>
                                                <span class="validation-message">{message}</span>
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

                // LDT Validation Section
                <div class="ldt-validation-section">
                    <div class="validation-header">"LDT/IES Validation"</div>

                    {if warnings.is_empty() && !has_errors {
                        view! {
                            <div class="validation-item success">
                                <span class="validation-icon">"✓"</span>
                                <span>{l.ui.validation_panel.all_passed.clone()}</span>
                            </div>
                        }.into_any()
                    } else {
                        let error_count_str = l.ui.validation_panel.error_count
                            .replace("{errors}", &errors.len().to_string())
                            .replace("{warnings}", &warnings.len().to_string());

                        view! {
                            <div class="validation-list">
                                // Critical errors first
                                {errors.iter().map(|error| {
                                    let code = error.code;
                                    let message = error.message.clone();
                                    view! {
                                        <div class="validation-item error">
                                            <span class="validation-code">{code}</span>
                                            <span class="validation-message">{message}</span>
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
                                            <span class="validation-message">{message}</span>
                                        </div>
                                    }
                                }).collect_view()}

                                // Summary
                                <div class="validation-summary">
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
