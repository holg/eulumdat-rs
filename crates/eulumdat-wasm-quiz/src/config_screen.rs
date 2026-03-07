use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::{Category, Difficulty, QuizBank, QuizConfig, QuizSession};
use leptos::prelude::*;

use crate::app::Screen;
use crate::i18n::use_quiz_locale;

#[component]
pub fn ConfigScreen(set_screen: WriteSignal<Screen>) -> impl IntoView {
    let all_categories = Category::all();
    let category_counts: Vec<(Category, u32)> = QuizBank::categories();
    let total_count = QuizBank::total_count();
    let ql = use_quiz_locale();

    let (selected_cats, set_selected_cats) = signal(
        all_categories
            .iter()
            .map(|c| (*c, true))
            .collect::<Vec<_>>(),
    );
    let (difficulty, set_difficulty) = signal::<Option<Difficulty>>(None);
    let (num_questions, set_num_questions) = signal(10u32);

    let available_count = move || {
        let sel = selected_cats.get();
        let selected: Vec<Category> = sel
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(c, _)| *c)
            .collect();
        let diff = difficulty.get();
        let all_q = QuizBank::all_questions();
        all_q
            .iter()
            .filter(|q| {
                (selected.is_empty() || selected.contains(&q.category))
                    && diff.is_none_or(|d| q.difficulty == d)
            })
            .count() as u32
    };

    let on_submit = move |_| {
        let sel = selected_cats.get();
        let categories: Vec<Category> = sel
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(c, _)| *c)
            .collect();
        // Use current timestamp as seed for random shuffle each run
        let seed = js_sys::Date::now() as u64;
        let config = QuizConfig {
            categories,
            difficulty: difficulty.get(),
            num_questions: num_questions.get(),
            shuffle: true,
            seed: Some(seed),
        };
        let session = QuizSession::new(config);
        set_screen.set(Screen::Quiz(session));
    };

    let toggle_category = move |idx: usize| {
        set_selected_cats.update(|cats| {
            if let Some(cat) = cats.get_mut(idx) {
                cat.1 = !cat.1;
            }
        });
    };

    let select_all = move |_| {
        set_selected_cats.update(|cats| {
            for cat in cats.iter_mut() {
                cat.1 = true;
            }
        });
    };

    let select_none = move |_| {
        set_selected_cats.update(|cats| {
            for cat in cats.iter_mut() {
                cat.1 = false;
            }
        });
    };

    let num_cats = all_categories.len();

    view! {
        <div class="config-screen">
            <div class="config-card">
                <h2 class="config-title">{move || ql.get().ui.configure.clone()}</h2>
                <p class="config-subtitle">
                    {move || {
                        QuizLocale::format(
                            &ql.get().ui.questions_across,
                            &[&total_count, &num_cats],
                        )
                    }}
                </p>

                <section class="config-section">
                    <div class="section-header">
                        <h3>{move || ql.get().ui.categories.clone()}</h3>
                        <div class="section-actions">
                            <button class="link-btn" on:click=select_all>
                                {move || ql.get().ui.select_all.clone()}
                            </button>
                            <button class="link-btn" on:click=select_none>
                                {move || ql.get().ui.select_none.clone()}
                            </button>
                        </div>
                    </div>
                    <div class="category-grid">
                        {all_categories
                            .iter()
                            .enumerate()
                            .map(|(idx, cat)| {
                                let cat = *cat;
                                let count = category_counts
                                    .iter()
                                    .find(|(c, _)| *c == cat)
                                    .map(|(_, n)| *n)
                                    .unwrap_or(0);
                                view! {
                                    <label class="category-item">
                                        <input
                                            type="checkbox"
                                            checked=move || {
                                                selected_cats
                                                    .get()
                                                    .get(idx)
                                                    .map(|(_, c)| *c)
                                                    .unwrap_or(false)
                                            }
                                            on:change=move |_| toggle_category(idx)
                                        />
                                        <span class="category-label">
                                            {move || ql.get().category_label(&cat).to_string()}
                                        </span>
                                        <span class="category-count">{count}</span>
                                    </label>
                                }
                            })
                            .collect_view()}
                    </div>
                </section>

                <section class="config-section">
                    <h3>{move || ql.get().ui.difficulty.clone()}</h3>
                    <div class="difficulty-options">
                        <label class="radio-item">
                            <input
                                type="radio"
                                name="difficulty"
                                checked=move || difficulty.get().is_none()
                                on:change=move |_| set_difficulty.set(None)
                            />
                            <span>{move || ql.get().ui.all_levels.clone()}</span>
                        </label>
                        {[Difficulty::Beginner, Difficulty::Intermediate, Difficulty::Expert]
                            .into_iter()
                            .map(|d| {
                                view! {
                                    <label class="radio-item">
                                        <input
                                            type="radio"
                                            name="difficulty"
                                            checked=move || difficulty.get() == Some(d)
                                            on:change=move |_| set_difficulty.set(Some(d))
                                        />
                                        <span>
                                            {move || ql.get().difficulty_label(&d).to_string()}
                                        </span>
                                    </label>
                                }
                            })
                            .collect_view()}
                    </div>
                </section>

                <section class="config-section">
                    <h3>{move || ql.get().ui.num_questions.clone()}</h3>
                    <div class="count-options">
                        {[5u32, 10, 20, 50]
                            .into_iter()
                            .map(|n| {
                                view! {
                                    <button
                                        class=move || {
                                            if num_questions.get() == n {
                                                "count-btn active"
                                            } else {
                                                "count-btn"
                                            }
                                        }
                                        on:click=move |_| set_num_questions.set(n)
                                    >
                                        {n}
                                    </button>
                                }
                            })
                            .collect_view()}
                        <button
                            class=move || {
                                if num_questions.get() == 0 {
                                    "count-btn active"
                                } else {
                                    "count-btn"
                                }
                            }
                            on:click=move |_| set_num_questions.set(0)
                        >
                            "All"
                        </button>
                    </div>
                </section>

                <div class="config-summary">
                    <p class="available-count">
                        {move || {
                            let locale = ql.get();
                            let avail = available_count();
                            let num = num_questions.get();
                            if num == 0 || num >= avail {
                                QuizLocale::format(
                                    &locale.ui.questions_available,
                                    &[&avail],
                                )
                            } else {
                                QuizLocale::format(
                                    &locale.ui.questions_selected,
                                    &[&num, &avail],
                                )
                            }
                        }}
                    </p>
                    <button
                        class="start-btn"
                        on:click=on_submit
                        disabled=move || available_count() == 0
                    >
                        {move || ql.get().ui.start_quiz.clone()}
                    </button>
                </div>
            </div>
        </div>
    }
}
