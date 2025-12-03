use eulumdat::Eulumdat;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ValidationPanelProps {
    pub ldt: Eulumdat,
}

#[function_component(ValidationPanel)]
pub fn validation_panel(props: &ValidationPanelProps) -> Html {
    let ldt = &props.ldt;
    let warnings = ldt.validate();
    let strict_result = ldt.validate_strict();

    // Check for errors
    let has_errors = strict_result.is_err();
    let errors = strict_result.err().unwrap_or_default();

    if warnings.is_empty() && !has_errors {
        return html! {
            <div class="validation-item success">
                <span>{"✓"}</span>
                <span>{"All validation checks passed"}</span>
            </div>
        };
    }

    html! {
        <div class="validation-panel">
            // Critical errors first
            {for errors.iter().map(|error| {
                html! {
                    <div class="validation-item error">
                        <span class="validation-code">{&error.code}</span>
                        <span>{&error.message}</span>
                    </div>
                }
            })}

            // Warnings
            {for warnings.iter().map(|warning| {
                html! {
                    <div class="validation-item warning">
                        <span class="validation-code">{warning.code}</span>
                        <span>{&warning.message}</span>
                    </div>
                }
            })}

            // Summary
            <div class="text-muted mt-1" style="font-size: 0.75rem;">
                {format!("{} error(s), {} warning(s)", errors.len(), warnings.len())}
            </div>
        </div>
    }
}
