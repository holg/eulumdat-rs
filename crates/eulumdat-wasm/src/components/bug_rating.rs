//! IESNA BUG Rating Component
//! Uses eulumdat-core bug_rating module for calculations and SVG generation

use crate::i18n::use_locale;
use eulumdat::{diagram::SvgTheme, BugDiagram, Eulumdat};
use leptos::ev;
use leptos::prelude::*;

/// BUG Rating view mode
#[derive(Clone, Copy, PartialEq, Default)]
pub enum BugViewMode {
    #[default]
    /// Basic BUG rating diagram
    Basic,
    /// Detailed view with zone lumens table
    Detailed,
}

#[component]
pub fn BugRating(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let locale = use_locale();
    let (view_mode, set_view_mode) = signal(BugViewMode::default());

    // Calculate rating from LDT
    let calculated_rating = move || {
        let ldt = ldt.get();
        let diagram = BugDiagram::from_eulumdat(&ldt);
        diagram.rating
    };

    let toggle_view = move |_: ev::MouseEvent| {
        set_view_mode.update(|m| {
            *m = match *m {
                BugViewMode::Basic => BugViewMode::Detailed,
                BugViewMode::Detailed => BugViewMode::Basic,
            };
        });
    };

    view! {
        <div class="bug-rating-container">
            <div class="bug-rating-header">
                <h3>{move || {
                    let l = locale.get();
                    match view_mode.get() {
                        BugViewMode::Basic => l.ui.bug_rating.title.clone(),
                        BugViewMode::Detailed => l.ui.bug_rating.title_detailed.clone(),
                    }
                }}</h3>
                <div class="bug-rating-controls">
                    <button class="btn btn-sm btn-secondary" on:click=toggle_view>
                        {move || {
                            let l = locale.get();
                            match view_mode.get() {
                                BugViewMode::Basic => l.ui.bug_rating.show_details.clone(),
                                BugViewMode::Detailed => l.ui.bug_rating.hide_details.clone(),
                            }
                        }}
                    </button>
                </div>
                <div
                    class="bug-rating-value"
                    style=move || {
                        let rating = calculated_rating();
                        let max_rating = rating.b.max(rating.u).max(rating.g);
                        format!("background: {}; border-color: {}",
                            rating_bg_color(max_rating),
                            rating_color(max_rating))
                    }
                >
                    {move || {
                        let rating = calculated_rating();
                        format!("B{} U{} G{}", rating.b, rating.u, rating.g)
                    }}
                </div>
            </div>

            <div class="bug-rating-content" inner_html=move || {
                let ldt = ldt.get();
                let diagram = BugDiagram::from_eulumdat(&ldt);
                let theme = SvgTheme::css_variables_with_locale(&locale.get());
                match view_mode.get() {
                    BugViewMode::Basic => diagram.to_svg(400.0, 350.0, &theme),
                    BugViewMode::Detailed => diagram.to_svg_with_details(550.0, 350.0, &theme),
                }
            } />

            <div class="bug-rating-footer">
                {move || {
                    let l = locale.get();
                    match view_mode.get() {
                        BugViewMode::Basic => l.ui.bug_rating.footer_basic.clone(),
                        BugViewMode::Detailed => l.ui.bug_rating.footer_detailed.clone(),
                    }
                }}
            </div>
        </div>
    }
}

fn rating_color(rating: u8) -> &'static str {
    match rating {
        0 | 1 => "var(--rating-good)",
        2 => "var(--rating-ok)",
        3 => "var(--rating-warn)",
        _ => "var(--rating-bad)",
    }
}

fn rating_bg_color(rating: u8) -> &'static str {
    match rating {
        0 | 1 => "var(--rating-bg-good)",
        2 => "var(--rating-bg-ok)",
        3 => "var(--rating-bg-warn)",
        _ => "var(--rating-bg-bad)",
    }
}
