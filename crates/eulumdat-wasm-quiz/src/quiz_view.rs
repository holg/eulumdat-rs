use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::{AnswerResult, Category, Question, QuizSession};
use leptos::prelude::*;

use crate::app::{Screen, TemplateLuminaires};
use crate::i18n::use_quiz_locale;

#[component]
pub fn QuizView(
    session: QuizSession,
    templates: StoredValue<TemplateLuminaires>,
    is_dark: bool,
    set_screen: WriteSignal<Screen>,
) -> impl IntoView {
    let (session, set_session) = signal(session);
    // Tracks the answered question + result. We snapshot the question at answer time
    // because session.answer() advances current_index immediately.
    let (answer_state, set_answer_state) = signal::<Option<(Question, AnswerResult, u8)>>(None);
    let ql = use_quiz_locale();

    // The question to display: either the "just answered" question (frozen) or the current one
    let display_question = move || -> Option<Question> {
        if let Some((ref q, _, _)) = answer_state.get() {
            Some(q.clone())
        } else {
            session.get().current_question()
        }
    };

    let progress = move || {
        let s = session.get();
        let idx = s.current_index;
        let total = s.questions.len();
        if answer_state.get().is_some() {
            // We already advanced past the answered question, show previous index
            (idx.saturating_sub(1), total)
        } else {
            (idx, total)
        }
    };

    let live_score = move || session.get().score();

    let on_answer = move |choice: u8| {
        if answer_state.get().is_some() {
            return;
        }
        // Snapshot the current question BEFORE answering (which advances index)
        let q = session.get().current_question();
        set_session.update(|s| {
            let result = s.answer(choice);
            if let Some(question) = q {
                set_answer_state.set(Some((question, result, choice)));
            }
        });
    };

    let on_next = move |_| {
        set_answer_state.set(None);
        let s = session.get();
        if s.is_finished() {
            set_screen.set(Screen::Results(s.score()));
        }
    };

    let on_skip = move |_| {
        set_session.update(|s| {
            s.skip();
        });
        set_answer_state.set(None);
        let s = session.get();
        if s.is_finished() {
            set_screen.set(Screen::Results(s.score()));
        }
    };

    view! {
        <div class="quiz-view">
            <div class="progress-section">
                <div class="progress-info">
                    <span class="progress-text">
                        {move || {
                            let (current, total) = progress();
                            QuizLocale::format(
                                &ql.get().ui.question_of,
                                &[&(current + 1), &total],
                            )
                        }}
                    </span>
                    <span class="score-text">
                        {move || {
                            let s = live_score();
                            QuizLocale::format(
                                &ql.get().ui.correct_count,
                                &[&s.correct, &s.wrong],
                            )
                        }}
                    </span>
                </div>
                <div class="progress-bar">
                    <div
                        class="progress-fill"
                        style=move || {
                            let (current, total) = progress();
                            let pct = if total > 0 {
                                ((current + 1) as f64 / total as f64 * 100.0).min(100.0)
                            } else {
                                0.0
                            };
                            format!("width: {}%", pct)
                        }
                    ></div>
                </div>
            </div>

            {move || {
                display_question().map(|question| {
                    let category = question.category;
                    let question_id = question.id;
                    let locale = ql.get();
                    let cat_label = locale.category_label(&category).to_string();
                    let diagram_html = if category == Category::DiagramReading {
                        templates.with_value(|t| {
                            t.diagram_svg_for_question(question_id, is_dark, &locale)
                        })
                    } else {
                        templates.with_value(|t| t.diagram_svg(&category, is_dark, &locale))
                    };
                    let diff_key = question.difficulty.key().to_string();
                    let diff_label = locale.difficulty_label(&question.difficulty).to_string();

                    // Get translated question content, falling back to compiled English
                    let translated = locale.question(question.id);
                    let q_text = translated
                        .map(|t| t.text.clone())
                        .unwrap_or_else(|| question.text.clone());
                    let q_options: Vec<String> = translated
                        .map(|t| t.options.clone())
                        .unwrap_or_else(|| question.options.clone());

                    view! {
                        <div class="question-card">
                            <div class="question-meta">
                                <span class="question-category">{cat_label}</span>
                                <span class=format!("question-difficulty diff-{}", diff_key)>
                                    {diff_label}
                                </span>
                            </div>
                            <h3 class="question-text">{q_text}</h3>

                            {diagram_html
                                .map(|html| {
                                    view! { <div class="question-diagram" inner_html=html></div> }
                                })}

                            <div class="options-list">
                                {q_options
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, opt)| {
                                        let idx = idx as u8;
                                        let opt_text = opt.clone();
                                        let letter = (b'A' + idx) as char;
                                        let option_class = move || {
                                            let state = answer_state.get();
                                            match state {
                                                None => "option-btn",
                                                Some((_, ref res, chosen)) => {
                                                    if idx == res.correct_index {
                                                        "option-btn correct"
                                                    } else if idx == chosen && !res.is_correct {
                                                        "option-btn wrong"
                                                    } else {
                                                        "option-btn disabled"
                                                    }
                                                }
                                            }
                                        };
                                        view! {
                                            <button
                                                class=option_class
                                                on:click=move |_| on_answer(idx)
                                                disabled=move || answer_state.get().is_some()
                                            >
                                                <span class="option-letter">
                                                    {format!("{}", letter)}
                                                </span>
                                                <span class="option-text">{opt_text.clone()}</span>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>

                            {move || {
                                answer_state.get().map(|(_, res, _)| {
                                    let locale = ql.get();
                                    let feedback_class = if res.is_correct {
                                        "feedback correct"
                                    } else {
                                        "feedback wrong"
                                    };
                                    let icon = if res.is_correct {
                                        locale.ui.correct.clone()
                                    } else {
                                        locale.ui.wrong.clone()
                                    };

                                    // Translate explanation
                                    let q_id = question.id;
                                    let translated_expl = locale.question(q_id);
                                    let explanation = translated_expl
                                        .map(|t| t.explanation.clone())
                                        .unwrap_or_else(|| res.explanation.clone());

                                    let reference = res.reference.clone();
                                    let ref_label = locale.ui.reference.clone();
                                    view! {
                                        <div class=feedback_class>
                                            <div class="feedback-header">{icon}</div>
                                            <p class="feedback-explanation">{explanation}</p>
                                            {reference
                                                .map(|r| {
                                                    view! {
                                                        <p class="feedback-reference">
                                                            <strong>{ref_label.clone()}</strong>
                                                            {r}
                                                        </p>
                                                    }
                                                })}
                                        </div>
                                    }
                                })
                            }}

                            <div class="question-nav">
                                {move || {
                                    let locale = ql.get();
                                    if answer_state.get().is_some() {
                                        let label = if session.get().is_finished() {
                                            locale.ui.see_results.clone()
                                        } else {
                                            locale.ui.next_question.clone()
                                        };
                                        view! {
                                            <button class="nav-btn primary" on:click=on_next>
                                                {label}
                                            </button>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <button class="nav-btn secondary" on:click=on_skip>
                                                {locale.ui.skip.clone()}
                                            </button>
                                        }
                                            .into_any()
                                    }
                                }}
                            </div>
                        </div>
                    }
                })
            }}
        </div>
    }
}
