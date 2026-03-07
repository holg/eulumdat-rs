use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::QuizScore;
use leptos::prelude::*;

use crate::app::Screen;
use crate::i18n::use_quiz_locale;

#[component]
pub fn ResultsView(score: QuizScore, set_screen: WriteSignal<Screen>) -> impl IntoView {
    let pct = score.percentage();
    let ql = use_quiz_locale();

    let grade_class = if pct >= 90.0 {
        "grade-excellent"
    } else if pct >= 70.0 {
        "grade-good"
    } else if pct >= 50.0 {
        "grade-ok"
    } else {
        "grade-poor"
    };

    let grade_text = move || {
        let locale = ql.get();
        if pct >= 90.0 {
            locale.ui.excellent.clone()
        } else if pct >= 70.0 {
            locale.ui.good_job.clone()
        } else if pct >= 50.0 {
            locale.ui.keep_learning.clone()
        } else {
            locale.ui.try_again.clone()
        }
    };

    let on_restart = move |_| {
        set_screen.set(Screen::Config);
    };

    let score_detail = {
        let correct = score.correct;
        let wrong = score.wrong;
        let skipped = score.skipped;
        let total = score.total;
        move || {
            QuizLocale::format(
                &ql.get().ui.score_detail,
                &[&correct, &wrong, &skipped, &total],
            )
        }
    };

    view! {
        <div class="results-screen">
            <div class="results-card">
                <div class=format!("grade-badge {}", grade_class)>
                    <span class="grade-text">{grade_text}</span>
                </div>

                <div class="score-display">
                    <span class="score-pct">{format!("{:.0}%", pct)}</span>
                    <span class="score-detail">{score_detail}</span>
                </div>

                <div class="breakdown-section">
                    <h3>{move || ql.get().ui.by_category.clone()}</h3>
                    <div class="breakdown-list">
                        {score
                            .by_category
                            .iter()
                            .filter(|cs| cs.total > 0)
                            .map(|cs| {
                                let cat_pct = if cs.total > 0 {
                                    cs.correct as f64 / cs.total as f64 * 100.0
                                } else {
                                    0.0
                                };
                                let bar_class = if cat_pct >= 80.0 {
                                    "bar-fill good"
                                } else if cat_pct >= 50.0 {
                                    "bar-fill ok"
                                } else {
                                    "bar-fill poor"
                                };
                                let cat = cs.category;
                                let detail = format!("{}/{}", cs.correct, cs.total);
                                view! {
                                    <div class="breakdown-item">
                                        <div class="breakdown-label">
                                            <span>
                                                {move || ql.get().category_label(&cat).to_string()}
                                            </span>
                                            <span class="breakdown-score">{detail.clone()}</span>
                                        </div>
                                        <div class="breakdown-bar">
                                            <div
                                                class=bar_class
                                                style=format!("width: {}%", cat_pct)
                                            ></div>
                                        </div>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>

                <div class="breakdown-section">
                    <h3>{move || ql.get().ui.by_difficulty.clone()}</h3>
                    <div class="breakdown-list">
                        {score
                            .by_difficulty
                            .iter()
                            .filter(|ds| ds.total > 0)
                            .map(|ds| {
                                let diff_pct = if ds.total > 0 {
                                    ds.correct as f64 / ds.total as f64 * 100.0
                                } else {
                                    0.0
                                };
                                let bar_class = if diff_pct >= 80.0 {
                                    "bar-fill good"
                                } else if diff_pct >= 50.0 {
                                    "bar-fill ok"
                                } else {
                                    "bar-fill poor"
                                };
                                let diff = ds.difficulty;
                                let detail = format!("{}/{}", ds.correct, ds.total);
                                view! {
                                    <div class="breakdown-item">
                                        <div class="breakdown-label">
                                            <span>
                                                {move || {
                                                    ql.get().difficulty_label(&diff).to_string()
                                                }}
                                            </span>
                                            <span class="breakdown-score">{detail.clone()}</span>
                                        </div>
                                        <div class="breakdown-bar">
                                            <div
                                                class=bar_class
                                                style=format!("width: {}%", diff_pct)
                                            ></div>
                                        </div>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>

                <button class="start-btn" on:click=on_restart>
                    {move || ql.get().ui.try_again_btn.clone()}
                </button>
            </div>
        </div>
    }
}
