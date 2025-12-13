use eulumdat::Eulumdat;
use leptos::prelude::*;

#[component]
pub fn ValidationPanel(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    move || {
        let ldt = ldt.get();
        let warnings = ldt.validate();
        let strict_result = ldt.validate_strict();

        let has_errors = strict_result.is_err();
        let errors = strict_result.err().unwrap_or_default();

        if warnings.is_empty() && !has_errors {
            return view! {
                <div class="validation-item success">
                    <span>"✓"</span>
                    <span>"All validation checks passed"</span>
                </div>
            }.into_any();
        }

        view! {
            <div class="validation-panel">
                // Critical errors first
                {errors.iter().map(|error| {
                    let code = error.code.clone();
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
                    {format!("{} error(s), {} warning(s)", errors.len(), warnings.len())}
                </div>
            </div>
        }.into_any()
    }
}
