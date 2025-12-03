//! IESNA BUG Rating Component
//! Uses eulumdat-core bug_rating module for calculations and SVG generation

use eulumdat::{diagram::SvgTheme, BugDiagram, Eulumdat};
use yew::prelude::*;

/// BUG rating values re-exported for WASM use
pub use eulumdat::bug_rating::BugRating as BugRatingValue;

#[derive(Properties, PartialEq)]
pub struct BugRatingProps {
    pub ldt: Eulumdat,
    #[prop_or_default]
    pub rating_override: Option<BugRatingValue>,
    #[prop_or_default]
    pub on_rating_change: Callback<BugRatingValue>,
}

/// IESNA classification view mode
#[derive(Clone, Copy, PartialEq)]
pub enum IesnaViewMode {
    /// TM-15-11: BUG (Backlight, Uplight, Glare) Rating
    Tm1511Bug,
    /// TM-15-07: Luminaire Classification System
    Tm1507Lcs,
}

#[function_component(BugRating)]
pub fn bug_rating_component(props: &BugRatingProps) -> Html {
    let ldt = &props.ldt;

    // Use core library for calculations
    let diagram = BugDiagram::from_eulumdat(ldt);
    let calculated_rating = diagram.rating;
    let rating = props.rating_override.unwrap_or(calculated_rating);

    // View mode state
    let view_mode = use_state(|| IesnaViewMode::Tm1511Bug);

    // Rating state
    let b_rating = use_state(|| rating.b);
    let u_rating = use_state(|| rating.u);
    let g_rating = use_state(|| rating.g);

    {
        let b_rating = b_rating.clone();
        let u_rating = u_rating.clone();
        let g_rating = g_rating.clone();
        use_effect_with(rating, move |r| {
            b_rating.set(r.b);
            u_rating.set(r.u);
            g_rating.set(r.g);
        });
    }

    let on_b_change = {
        let b_rating = b_rating.clone();
        let u_rating = u_rating.clone();
        let g_rating = g_rating.clone();
        let callback = props.on_rating_change.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse::<u8>() {
                let val = val.min(5);
                b_rating.set(val);
                callback.emit(BugRatingValue::new(val, *u_rating, *g_rating));
            }
        })
    };

    let on_u_change = {
        let b_rating = b_rating.clone();
        let u_rating = u_rating.clone();
        let g_rating = g_rating.clone();
        let callback = props.on_rating_change.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse::<u8>() {
                let val = val.min(5);
                u_rating.set(val);
                callback.emit(BugRatingValue::new(*b_rating, val, *g_rating));
            }
        })
    };

    let on_g_change = {
        let b_rating = b_rating.clone();
        let u_rating = u_rating.clone();
        let g_rating = g_rating.clone();
        let callback = props.on_rating_change.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse::<u8>() {
                let val = val.min(5);
                g_rating.set(val);
                callback.emit(BugRatingValue::new(*b_rating, *u_rating, val));
            }
        })
    };

    let on_reset = {
        let b_rating = b_rating.clone();
        let u_rating = u_rating.clone();
        let g_rating = g_rating.clone();
        let callback = props.on_rating_change.clone();
        let calc = calculated_rating;
        Callback::from(move |_: MouseEvent| {
            b_rating.set(calc.b);
            u_rating.set(calc.u);
            g_rating.set(calc.g);
            callback.emit(calc);
        })
    };

    let toggle_view = {
        let view_mode = view_mode.clone();
        Callback::from(move |_: MouseEvent| {
            view_mode.set(match *view_mode {
                IesnaViewMode::Tm1511Bug => IesnaViewMode::Tm1507Lcs,
                IesnaViewMode::Tm1507Lcs => IesnaViewMode::Tm1511Bug,
            });
        })
    };

    let current_b = *b_rating;
    let current_u = *u_rating;
    let current_g = *g_rating;
    let max_rating = current_b.max(current_u).max(current_g);

    let current_mode = *view_mode;
    let header_title = match current_mode {
        IesnaViewMode::Tm1511Bug => "IESNA TM-15-11 BUG Rating",
        IesnaViewMode::Tm1507Lcs => "IESNA TM-15-07 LCS",
    };
    let toggle_label = match current_mode {
        IesnaViewMode::Tm1511Bug => "Switch to TM-15-07 LCS",
        IesnaViewMode::Tm1507Lcs => "Switch to TM-15-11 BUG",
    };
    let footer_text = match current_mode {
        IesnaViewMode::Tm1511Bug => "IESNA TM-15-11 Backlight, Uplight, Glare Rating",
        IesnaViewMode::Tm1507Lcs => {
            "IESNA TM-15-07 Luminaire Classification System For Outdoor Luminaires"
        }
    };

    // Generate SVG from core
    let theme = SvgTheme::css_variables();
    let svg = match current_mode {
        IesnaViewMode::Tm1511Bug => diagram.to_svg(400.0, 350.0, &theme),
        IesnaViewMode::Tm1507Lcs => diagram.to_lcs_svg(510.0, 315.0, &theme),
    };

    html! {
        <div class="bug-rating-container">
            <div class="bug-rating-header">
                <h3>{header_title}</h3>
                <div class="bug-rating-edit">
                    <div class="rating-inputs">
                        <div class="rating-input-group">
                            <label>{"B"}</label>
                            <input type="number" min="0" max="5" value={current_b.to_string()}
                                   onchange={on_b_change}
                                   style={format!("border-color: {}", rating_color(current_b))} />
                        </div>
                        <div class="rating-input-group">
                            <label>{"U"}</label>
                            <input type="number" min="0" max="5" value={current_u.to_string()}
                                   onchange={on_u_change}
                                   style={format!("border-color: {}", rating_color(current_u))} />
                        </div>
                        <div class="rating-input-group">
                            <label>{"G"}</label>
                            <input type="number" min="0" max="5" value={current_g.to_string()}
                                   onchange={on_g_change}
                                   style={format!("border-color: {}", rating_color(current_g))} />
                        </div>
                    </div>
                    <button class="btn btn-sm btn-secondary" onclick={on_reset}
                            title={format!("Reset to calculated: {}", calculated_rating)}>
                        {"Reset"}
                    </button>
                    <button class="btn btn-sm btn-secondary" onclick={toggle_view}>
                        {toggle_label}
                    </button>
                </div>
                <div class="bug-rating-value" style={format!("background: {}; border-color: {}",
                    rating_bg_color(max_rating),
                    rating_color(max_rating))}>
                    {format!("B{} U{} G{}", current_b, current_u, current_g)}
                </div>
            </div>

            <div class="bug-rating-content">
                {Html::from_html_unchecked(AttrValue::from(svg))}
            </div>

            <div class="bug-rating-footer">
                {footer_text}
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
