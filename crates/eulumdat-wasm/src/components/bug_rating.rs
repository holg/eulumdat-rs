//! IESNA BUG Rating Component
//! Uses eulumdat-core bug_rating module for calculations and SVG generation

use eulumdat::{diagram::SvgTheme, BugDiagram, Eulumdat};
use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// BUG rating values re-exported for WASM use
pub use eulumdat::bug_rating::BugRating as BugRatingValue;

/// IESNA classification view mode
#[derive(Clone, Copy, PartialEq, Default)]
pub enum IesnaViewMode {
    #[default]
    /// TM-15-11: BUG (Backlight, Uplight, Glare) Rating
    Tm1511Bug,
    /// TM-15-07: Luminaire Classification System
    Tm1507Lcs,
}

#[component]
pub fn BugRating(ldt: ReadSignal<Eulumdat>) -> impl IntoView {
    let (view_mode, set_view_mode) = signal(IesnaViewMode::default());

    // Calculate rating from LDT
    let calculated_rating = move || {
        let ldt = ldt.get();
        let diagram = BugDiagram::from_eulumdat(&ldt);
        diagram.rating
    };

    // Editable ratings
    let (b_rating, set_b_rating) = signal(calculated_rating().b);
    let (u_rating, set_u_rating) = signal(calculated_rating().u);
    let (g_rating, set_g_rating) = signal(calculated_rating().g);

    // Sync when LDT changes
    Effect::new(move |_| {
        let rating = calculated_rating();
        set_b_rating.set(rating.b);
        set_u_rating.set(rating.u);
        set_g_rating.set(rating.g);
    });

    let on_b_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        if let Ok(val) = input.value().parse::<u8>() {
            set_b_rating.set(val.min(5));
        }
    };

    let on_u_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        if let Ok(val) = input.value().parse::<u8>() {
            set_u_rating.set(val.min(5));
        }
    };

    let on_g_change = move |e: ev::Event| {
        let input: HtmlInputElement = e.target().unwrap().unchecked_into();
        if let Ok(val) = input.value().parse::<u8>() {
            set_g_rating.set(val.min(5));
        }
    };

    let on_reset = move |_: ev::MouseEvent| {
        let rating = calculated_rating();
        set_b_rating.set(rating.b);
        set_u_rating.set(rating.u);
        set_g_rating.set(rating.g);
    };

    let toggle_view = move |_: ev::MouseEvent| {
        set_view_mode.update(|m| {
            *m = match *m {
                IesnaViewMode::Tm1511Bug => IesnaViewMode::Tm1507Lcs,
                IesnaViewMode::Tm1507Lcs => IesnaViewMode::Tm1511Bug,
            };
        });
    };

    view! {
        <div class="bug-rating-container">
            <div class="bug-rating-header">
                <h3>{move || match view_mode.get() {
                    IesnaViewMode::Tm1511Bug => "IESNA TM-15-11 BUG Rating",
                    IesnaViewMode::Tm1507Lcs => "IESNA TM-15-07 LCS",
                }}</h3>
                <div class="bug-rating-edit">
                    <div class="rating-inputs">
                        <div class="rating-input-group">
                            <label>"B"</label>
                            <input
                                type="number"
                                min="0"
                                max="5"
                                prop:value=move || b_rating.get().to_string()
                                on:change=on_b_change
                                style=move || format!("border-color: {}", rating_color(b_rating.get()))
                            />
                        </div>
                        <div class="rating-input-group">
                            <label>"U"</label>
                            <input
                                type="number"
                                min="0"
                                max="5"
                                prop:value=move || u_rating.get().to_string()
                                on:change=on_u_change
                                style=move || format!("border-color: {}", rating_color(u_rating.get()))
                            />
                        </div>
                        <div class="rating-input-group">
                            <label>"G"</label>
                            <input
                                type="number"
                                min="0"
                                max="5"
                                prop:value=move || g_rating.get().to_string()
                                on:change=on_g_change
                                style=move || format!("border-color: {}", rating_color(g_rating.get()))
                            />
                        </div>
                    </div>
                    <button class="btn btn-sm btn-secondary" on:click=on_reset
                            title=move || format!("Reset to calculated: {}", calculated_rating())>
                        "Reset"
                    </button>
                    <button class="btn btn-sm btn-secondary" on:click=toggle_view>
                        {move || match view_mode.get() {
                            IesnaViewMode::Tm1511Bug => "Switch to TM-15-07 LCS",
                            IesnaViewMode::Tm1507Lcs => "Switch to TM-15-11 BUG",
                        }}
                    </button>
                </div>
                <div
                    class="bug-rating-value"
                    style=move || {
                        let max_rating = b_rating.get().max(u_rating.get()).max(g_rating.get());
                        format!("background: {}; border-color: {}",
                            rating_bg_color(max_rating),
                            rating_color(max_rating))
                    }
                >
                    {move || format!("B{} U{} G{}", b_rating.get(), u_rating.get(), g_rating.get())}
                </div>
            </div>

            <div class="bug-rating-content" inner_html=move || {
                let ldt = ldt.get();
                let diagram = BugDiagram::from_eulumdat(&ldt);
                let theme = SvgTheme::css_variables();
                match view_mode.get() {
                    IesnaViewMode::Tm1511Bug => diagram.to_svg(400.0, 350.0, &theme),
                    IesnaViewMode::Tm1507Lcs => diagram.to_lcs_svg(510.0, 315.0, &theme),
                }
            } />

            <div class="bug-rating-footer">
                {move || match view_mode.get() {
                    IesnaViewMode::Tm1511Bug => "IESNA TM-15-11 Backlight, Uplight, Glare Rating",
                    IesnaViewMode::Tm1507Lcs => "IESNA TM-15-07 Luminaire Classification System For Outdoor Luminaires",
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
