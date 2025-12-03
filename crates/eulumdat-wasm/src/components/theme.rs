//! Centralized theme system for consistent colors across all diagrams
//!
//! Provides light and dark mode color schemes with system preference detection

use yew::prelude::*;

/// Theme mode
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn toggle(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self {
            ThemeMode::Light => "theme-light",
            ThemeMode::Dark => "theme-dark",
        }
    }
}

/// Diagram color scheme - consistent across all visualizations
#[derive(Clone, Copy, PartialEq)]
pub struct DiagramColors {
    // Background colors
    pub background: &'static str,
    pub surface: &'static str,

    // Grid and lines
    pub grid: &'static str,
    pub grid_minor: &'static str,
    pub axis: &'static str,

    // Text colors
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub text_muted: &'static str,

    // C-plane line colors (consistent across all diagrams)
    pub c0: &'static str,   // C0/C180 plane - Red
    pub c90: &'static str,  // C90/C270 plane - Blue
    pub c45: &'static str,  // C45/C225 plane - Green
    pub c135: &'static str, // C135/C315 plane - Orange

    // Zone colors for BUG/LCS
    pub forward_zone: &'static str, // Forward light - Green
    pub back_zone: &'static str,    // Back light - Blue
    pub uplight_zone: &'static str, // Uplight - Red/Orange

    // Rating colors
    pub rating_good: &'static str, // Rating 0-1
    pub rating_ok: &'static str,   // Rating 2
    pub rating_warn: &'static str, // Rating 3
    pub rating_bad: &'static str,  // Rating 4-5

    // Heatmap colors
    pub heatmap_low: &'static str,
    pub heatmap_mid: &'static str,
    pub heatmap_high: &'static str,
}

impl DiagramColors {
    pub const LIGHT: Self = Self {
        // Background
        background: "#ffffff",
        surface: "#f8fafc",

        // Grid
        grid: "#e0e0e0",
        grid_minor: "#f0f0f0",
        axis: "#333333",

        // Text
        text_primary: "#333333",
        text_secondary: "#666666",
        text_muted: "#999999",

        // C-plane colors
        c0: "#ef4444",   // Red
        c90: "#3b82f6",  // Blue
        c45: "#22c55e",  // Green
        c135: "#f97316", // Orange

        // Zone colors
        forward_zone: "#22c55e", // Green
        back_zone: "#3b82f6",    // Blue
        uplight_zone: "#ef4444", // Red

        // Rating colors
        rating_good: "#22c55e",
        rating_ok: "#eab308",
        rating_warn: "#f97316",
        rating_bad: "#ef4444",

        // Heatmap
        heatmap_low: "#142850",
        heatmap_mid: "#22c55e",
        heatmap_high: "#ef4444",
    };

    pub const DARK: Self = Self {
        // Background
        background: "#1a1a2e",
        surface: "#16213e",

        // Grid
        grid: "#404060",
        grid_minor: "#2a2a4a",
        axis: "#959595",

        // Text
        text_primary: "#e0e0e0",
        text_secondary: "#bebebe",
        text_muted: "#808080",

        // C-plane colors (slightly brighter for dark mode)
        c0: "#f87171",   // Red (lighter)
        c90: "#60a5fa",  // Blue (lighter)
        c45: "#4ade80",  // Green (lighter)
        c135: "#fb923c", // Orange (lighter)

        // Zone colors (matching reference SVG)
        forward_zone: "#62c58a", // Green
        back_zone: "#42b4ff",    // Blue
        uplight_zone: "#f87171", // Red

        // Rating colors
        rating_good: "#4ade80",
        rating_ok: "#facc15",
        rating_warn: "#fb923c",
        rating_bad: "#f87171",

        // Heatmap
        heatmap_low: "#1e3a5f",
        heatmap_mid: "#4ade80",
        heatmap_high: "#f87171",
    };

    pub fn for_mode(mode: ThemeMode) -> &'static Self {
        match mode {
            ThemeMode::Light => &Self::LIGHT,
            ThemeMode::Dark => &Self::DARK,
        }
    }

    /// Get color for a C-plane angle
    #[allow(dead_code)]
    pub fn c_plane_color(&self, angle: f64) -> &'static str {
        let normalized = ((angle % 360.0) + 360.0) % 360.0;
        if !(22.5..337.5).contains(&normalized) || (157.5..202.5).contains(&normalized) {
            self.c0 // C0/C180
        } else if (67.5..112.5).contains(&normalized) || (247.5..292.5).contains(&normalized) {
            self.c90 // C90/C270
        } else if (22.5..67.5).contains(&normalized) || (202.5..247.5).contains(&normalized) {
            self.c45 // C45/C225
        } else {
            self.c135 // C135/C315
        }
    }
}

/// Context provider for theme
#[derive(Clone, PartialEq)]
pub struct ThemeContext {
    pub mode: ThemeMode,
    pub colors: &'static DiagramColors,
}

impl Default for ThemeContext {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Light,
            colors: &DiagramColors::LIGHT,
        }
    }
}

/// Properties for ThemeProvider
#[derive(Properties, PartialEq)]
pub struct ThemeProviderProps {
    pub children: Children,
    pub mode: ThemeMode,
}

/// Theme provider component
#[function_component(ThemeProvider)]
pub fn theme_provider(props: &ThemeProviderProps) -> Html {
    let context = ThemeContext {
        mode: props.mode,
        colors: DiagramColors::for_mode(props.mode),
    };

    html! {
        <ContextProvider<ThemeContext> context={context}>
            {props.children.clone()}
        </ContextProvider<ThemeContext>>
    }
}

/// Hook to use theme context
#[hook]
pub fn use_theme() -> ThemeContext {
    use_context::<ThemeContext>().unwrap_or_default()
}

/// Detect system color scheme preference (standalone function)
pub fn detect_system_theme() -> ThemeMode {
    // Use eval to check prefers-color-scheme since match_media needs specific feature flag
    if let Ok(result) = js_sys::eval(
        "window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches",
    ) {
        if result.as_bool().unwrap_or(false) {
            return ThemeMode::Dark;
        }
    }
    ThemeMode::Light
}
