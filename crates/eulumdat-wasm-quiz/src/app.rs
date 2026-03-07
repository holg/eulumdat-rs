use eulumdat::diagram::{CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme};
use eulumdat::Eulumdat;
use eulumdat_quiz::i18n::QuizLocale;
use eulumdat_quiz::{Category, QuizScore, QuizSession};
use leptos::prelude::*;

use crate::config_screen::ConfigScreen;
use crate::i18n::{use_quiz_locale, I18nProvider, LanguageSelector};
use crate::quiz_view::QuizView;
use crate::results_view::ResultsView;

/// Embedded template LDT files for diagram rendering
const FLUORESCENT_LDT: &str =
    include_str!("../../eulumdat-wasm/templates/fluorescent_luminaire.ldt");
const ROAD_LDT: &str = include_str!("../../eulumdat-wasm/templates/road_luminaire.ldt");
const PROJECTOR_LDT: &str = include_str!("../../eulumdat-wasm/templates/projector.ldt");

/// Pre-parsed template luminaires for diagram generation
pub struct TemplateLuminaires {
    pub fluorescent: Eulumdat,
    pub road: Eulumdat,
    pub projector: Eulumdat,
}

impl TemplateLuminaires {
    fn load() -> Self {
        Self {
            fluorescent: Eulumdat::parse(FLUORESCENT_LDT).expect("fluorescent template"),
            road: Eulumdat::parse(ROAD_LDT).expect("road template"),
            projector: Eulumdat::parse(PROJECTOR_LDT).expect("projector template"),
        }
    }

    /// Get a contextual SVG diagram for a question based on its category
    pub fn diagram_svg(
        &self,
        category: &Category,
        is_dark: bool,
        locale: &QuizLocale,
    ) -> Option<String> {
        let theme = if is_dark {
            SvgTheme::dark()
        } else {
            SvgTheme::light()
        };

        match category {
            Category::DiagramTypes | Category::CoordinateSystems => {
                let polar = PolarDiagram::from_eulumdat(&self.fluorescent);
                let polar_svg = polar.to_svg(280.0, 280.0, &theme);
                let cart = CartesianDiagram::from_eulumdat(&self.fluorescent, 280.0, 200.0, 4);
                let cart_svg = cart.to_svg(280.0, 200.0, &theme);
                let polar_label = &locale.ui.polar_diagram;
                let cart_label = &locale.ui.cartesian_diagram;
                Some(format!(
                    r#"<div class="diagram-pair"><div class="diagram-item"><div class="diagram-label">{polar_label}</div>{polar_svg}</div><div class="diagram-item"><div class="diagram-label">{cart_label}</div>{cart_svg}</div></div>"#
                ))
            }
            Category::Symmetry => {
                let polar_sym = PolarDiagram::from_eulumdat(&self.fluorescent);
                let svg_sym = polar_sym.to_svg(250.0, 250.0, &theme);
                let polar_asym = PolarDiagram::from_eulumdat(&self.road);
                let svg_asym = polar_asym.to_svg(250.0, 250.0, &theme);
                let sym_label = &locale.ui.symmetric;
                let asym_label = &locale.ui.asymmetric;
                Some(format!(
                    r#"<div class="diagram-pair"><div class="diagram-item"><div class="diagram-label">{sym_label}</div>{svg_sym}</div><div class="diagram-item"><div class="diagram-label">{asym_label}</div>{svg_asym}</div></div>"#
                ))
            }
            Category::PhotometricCalc => {
                let polar = PolarDiagram::from_eulumdat(&self.projector);
                let svg = polar.to_svg(300.0, 300.0, &theme);
                let label = &locale.ui.projector_narrow;
                Some(format!(
                    r#"<div class="diagram-single"><div class="diagram-label">{label}</div>{svg}</div>"#
                ))
            }
            // DiagramReading uses per-question routing via diagram_svg_for_question()
            Category::DiagramReading => Some(self.fluorescent_polar_svg(&theme, locale)),
            _ => None,
        }
    }

    /// Get a contextual SVG diagram for a specific DiagramReading question.
    pub fn diagram_svg_for_question(
        &self,
        question_id: u32,
        is_dark: bool,
        locale: &QuizLocale,
    ) -> Option<String> {
        let theme = if is_dark {
            SvgTheme::dark()
        } else {
            SvgTheme::light()
        };

        Some(match question_id {
            // Symmetric vs asymmetric comparison
            17006 | 17012 => {
                let svg_sym =
                    PolarDiagram::from_eulumdat(&self.fluorescent).to_svg(250.0, 250.0, &theme);
                let svg_asym = PolarDiagram::from_eulumdat(&self.road).to_svg(250.0, 250.0, &theme);
                let sym_label = &locale.ui.symmetric;
                let asym_label = &locale.ui.asymmetric;
                format!(
                    r#"<div class="diagram-pair"><div class="diagram-item"><div class="diagram-label">{sym_label}</div>{svg_sym}</div><div class="diagram-item"><div class="diagram-label">{asym_label}</div>{svg_asym}</div></div>"#
                )
            }
            // Road luminaire (asymmetric)
            17007 => {
                let svg = PolarDiagram::from_eulumdat(&self.road).to_svg(300.0, 300.0, &theme);
                let label = &locale.ui.asymmetric;
                format!(
                    r#"<div class="diagram-single"><div class="diagram-label">{label}</div>{svg}</div>"#
                )
            }
            // Projector (narrow beam)
            17008 | 17011 => {
                let svg = PolarDiagram::from_eulumdat(&self.projector).to_svg(300.0, 300.0, &theme);
                let label = &locale.ui.projector_narrow;
                format!(
                    r#"<div class="diagram-single"><div class="diagram-label">{label}</div>{svg}</div>"#
                )
            }
            // Heatmap questions
            17013..=17020 => self.fluorescent_heatmap_svg(&theme, locale),
            // All other DiagramReading questions — fluorescent polar
            _ => self.fluorescent_polar_svg(&theme, locale),
        })
    }

    fn fluorescent_polar_svg(&self, theme: &SvgTheme, locale: &QuizLocale) -> String {
        let polar = PolarDiagram::from_eulumdat(&self.fluorescent);
        let svg = polar.to_svg(300.0, 300.0, theme);
        let label = &locale.ui.polar_diagram;
        format!(
            r#"<div class="diagram-single"><div class="diagram-label">{label}</div>{svg}</div>"#
        )
    }

    fn fluorescent_heatmap_svg(&self, theme: &SvgTheme, locale: &QuizLocale) -> String {
        let heatmap = HeatmapDiagram::from_eulumdat(&self.fluorescent, 400.0, 300.0);
        let svg = heatmap.to_svg(400.0, 300.0, theme);
        let label = locale.ui.heatmap.as_deref().unwrap_or("Heatmap");
        format!(
            r#"<div class="diagram-single"><div class="diagram-label">{label}</div>{svg}</div>"#
        )
    }
}

/// App screen state
#[derive(Clone)]
pub enum Screen {
    Config,
    Quiz(QuizSession),
    Results(QuizScore),
}

/// Theme mode
#[derive(Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    pub fn class_name(&self) -> &'static str {
        match self {
            ThemeMode::Light => "theme-light",
            ThemeMode::Dark => "theme-dark",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }

    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeMode::Dark)
    }
}

fn detect_system_theme() -> ThemeMode {
    if let Ok(result) = js_sys::eval(
        "window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches",
    ) {
        if result.as_bool().unwrap_or(false) {
            return ThemeMode::Dark;
        }
    }
    ThemeMode::Light
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <I18nProvider>
            <AppInner />
        </I18nProvider>
    }
}

#[component]
fn AppInner() -> impl IntoView {
    let templates = StoredValue::new(TemplateLuminaires::load());
    let (theme, set_theme) = signal(detect_system_theme());
    let (screen, set_screen) = signal(Screen::Config);
    let ql = use_quiz_locale();

    let toggle_theme = move |_| {
        set_theme.set(theme.get().toggle());
    };

    view! {
        <div class=move || format!("quiz-app {}", theme.get().class_name())>
            <header class="quiz-header">
                <div class="header-content">
                    <h1 class="header-title">
                        <span class="header-icon">"💡"</span>
                        " "
                        {move || ql.get().ui.title.clone()}
                    </h1>
                    <div class="header-controls">
                        <LanguageSelector />
                        <button class="theme-toggle" on:click=toggle_theme title="Toggle dark mode">
                            {move || if theme.get().is_dark() { "☀️" } else { "🌙" }}
                        </button>
                    </div>
                </div>
            </header>
            <main class="quiz-main">
                {move || {
                    let current_screen = screen.get();
                    match current_screen {
                        Screen::Config => {
                            view! { <ConfigScreen set_screen=set_screen /> }.into_any()
                        }
                        Screen::Quiz(session) => {
                            let is_dark = theme.get().is_dark();
                            view! {
                                <QuizView
                                    session=session
                                    templates=templates
                                    is_dark=is_dark
                                    set_screen=set_screen
                                />
                            }
                            .into_any()
                        }
                        Screen::Results(score) => {
                            view! { <ResultsView score=score set_screen=set_screen /> }.into_any()
                        }
                    }
                }}
            </main>
            <footer class="quiz-footer">
                <a href="https://eulumdat.icu" class="footer-link">
                    {move || ql.get().ui.back_to_editor.clone()}
                </a>
                <span class="footer-sep">"|"</span>
                <span class="footer-text">{move || ql.get().ui.powered_by.clone()}</span>
            </footer>
        </div>
    }
}
