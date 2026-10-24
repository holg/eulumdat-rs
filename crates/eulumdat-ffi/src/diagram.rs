//! Diagram types and functions for FFI

use eulumdat::diagram::{
    ButterflyDiagram, CartesianDiagram, ConeDiagram, FloodlightCartesianDiagram, HeatmapDiagram,
    IsocandelaDiagram, IsoluxDiagram, IsoluxParams, PolarDiagram, SvgTheme, ThemeOverrides,
    WatchFaceStyle, YScale,
};
use eulumdat::{PhotometricCalculations, PhotometricSummary, SymmetryHandler};
use eulumdat_i18n::{Language as CoreLanguage, Locale};

use crate::error::EulumdatError;
use crate::types::{to_core_eulumdat, Eulumdat, Symmetry};

/// Supported UI languages for localization
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Language {
    English,
    German,
    Chinese,
    French,
    Italian,
    Russian,
    Spanish,
    PortugueseBrazil,
}

impl Language {
    fn to_core(self) -> CoreLanguage {
        match self {
            Language::English => CoreLanguage::English,
            Language::German => CoreLanguage::German,
            Language::Chinese => CoreLanguage::Chinese,
            Language::French => CoreLanguage::French,
            Language::Italian => CoreLanguage::Italian,
            Language::Russian => CoreLanguage::Russian,
            Language::Spanish => CoreLanguage::Spanish,
            Language::PortugueseBrazil => CoreLanguage::PortugueseBrazil,
        }
    }

    pub(crate) fn to_locale(self) -> Locale {
        Locale::for_language(self.to_core())
    }
}

// Base diagram types

#[derive(Debug, Clone, uniffi::Record)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<eulumdat::diagram::Color> for Color {
    fn from(c: eulumdat::diagram::Color) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl From<eulumdat::diagram::Point2D> for Point2D {
    fn from(p: eulumdat::diagram::Point2D) -> Self {
        Self { x: p.x, y: p.y }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DiagramScale {
    pub max_intensity: f64,
    pub scale_max: f64,
    pub grid_values: Vec<f64>,
}

impl From<&eulumdat::diagram::DiagramScale> for DiagramScale {
    fn from(s: &eulumdat::diagram::DiagramScale) -> Self {
        Self {
            max_intensity: s.max_intensity,
            scale_max: s.scale_max,
            grid_values: s.grid_values.clone(),
        }
    }
}

// Polar diagram types

#[derive(Debug, Clone, uniffi::Record)]
pub struct PolarPoint {
    pub x: f64,
    pub y: f64,
    pub gamma: f64,
    pub intensity: f64,
}

impl From<&eulumdat::diagram::PolarPoint> for PolarPoint {
    fn from(p: &eulumdat::diagram::PolarPoint) -> Self {
        Self {
            x: p.x,
            y: p.y,
            gamma: p.gamma,
            intensity: p.intensity,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct PolarCurve {
    pub points: Vec<PolarPoint>,
    pub c_angle: f64,
    pub label: String,
}

impl From<&eulumdat::diagram::PolarCurve> for PolarCurve {
    fn from(c: &eulumdat::diagram::PolarCurve) -> Self {
        Self {
            points: c.points.iter().map(|p| p.into()).collect(),
            c_angle: c.c_angle,
            label: c.label.clone(),
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct PolarDiagramData {
    pub c0_c180_curve: PolarCurve,
    pub c90_c270_curve: PolarCurve,
    pub scale: DiagramScale,
    pub symmetry: Symmetry,
    pub show_c90_c270: bool,
}

// Butterfly diagram types

#[derive(Debug, Clone, uniffi::Record)]
pub struct ButterflyWing {
    pub c_angle: f64,
    pub points: Vec<Point2D>,
    pub fill_color: Color,
    pub stroke_color: Color,
    pub svg_path: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CPlaneDirection {
    pub angle: f64,
    pub start: Point2D,
    pub end_point: Point2D,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ButterflyDiagramData {
    pub wings: Vec<ButterflyWing>,
    pub grid_circles: Vec<Vec<Point2D>>,
    pub c_plane_lines: Vec<CPlaneDirection>,
    pub scale: DiagramScale,
    pub g_angles: Vec<f64>,
    pub symmetry: Symmetry,
}

// Cartesian diagram types

#[derive(Debug, Clone, uniffi::Record)]
pub struct CartesianPoint {
    pub x: f64,
    pub y: f64,
    pub gamma: f64,
    pub intensity: f64,
}

impl From<&eulumdat::diagram::CartesianPoint> for CartesianPoint {
    fn from(p: &eulumdat::diagram::CartesianPoint) -> Self {
        Self {
            x: p.x,
            y: p.y,
            gamma: p.gamma,
            intensity: p.intensity,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CartesianCurve {
    pub points: Vec<CartesianPoint>,
    pub c_angle: f64,
    pub color: Color,
    pub label: String,
    pub svg_path: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct CartesianDiagramData {
    pub curves: Vec<CartesianCurve>,
    pub x_ticks: Vec<f64>,
    pub y_ticks: Vec<f64>,
    pub scale: DiagramScale,
    pub max_gamma: f64,
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

// Heatmap diagram types

#[derive(Debug, Clone, uniffi::Record)]
pub struct HeatmapCell {
    pub c_index: u32,
    pub g_index: u32,
    pub c_angle: f64,
    pub g_angle: f64,
    pub intensity: f64,
    pub candela: f64,
    pub normalized: f64,
    pub color: Color,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LegendEntry {
    pub normalized: f64,
    pub color: Color,
    pub candela: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct HeatmapDiagramData {
    pub cells: Vec<HeatmapCell>,
    pub scale: DiagramScale,
    pub max_candela: f64,
    pub total_flux: f64,
    pub c_angles: Vec<f64>,
    pub g_angles: Vec<f64>,
    pub legend_entries: Vec<LegendEntry>,
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

// SVG Theme enum for FFI

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SvgThemeType {
    Light,
    Dark,
    CssVariables,
}

impl SvgThemeType {
    pub(crate) fn to_core(self) -> SvgTheme {
        match self {
            SvgThemeType::Light => SvgTheme::light(),
            SvgThemeType::Dark => SvgTheme::dark(),
            SvgThemeType::CssVariables => SvgTheme::css_variables(),
        }
    }

    pub(crate) fn to_core_with_locale(self, locale: &Locale) -> SvgTheme {
        match self {
            SvgThemeType::Light => SvgTheme::light_with_locale(locale),
            SvgThemeType::Dark => SvgTheme::dark_with_locale(locale),
            SvgThemeType::CssVariables => SvgTheme::css_variables_with_locale(locale),
        }
    }

    fn base_name(self) -> &'static str {
        match self {
            SvgThemeType::Light => "light",
            SvgThemeType::Dark => "dark",
            SvgThemeType::CssVariables => "css",
        }
    }
}

// ── Custom theme configuration (for the native theme-editor UI) ──────────

/// A full custom SVG theme the host app builds from its colour/thickness/font
/// pickers, then hands to the `*_themed` diagram functions.
///
/// It starts from a base preset (`base`: "light" | "dark" | "css") and overrides
/// only the fields that are set (`None` inherits the base). All fields render as
/// inline SVG attributes, so they display in every renderer (CoreGraphics/
/// `NSImage` included, which ignores CSS `<style>` blocks).
#[derive(Debug, Clone, uniffi::Record)]
pub struct SvgThemeConfig {
    /// Base preset the overrides layer on: "light", "dark", or "css".
    #[uniffi(default = "light")]
    pub base: String,
    #[uniffi(default = None)]
    pub background: Option<String>,
    #[uniffi(default = None)]
    pub surface: Option<String>,
    #[uniffi(default = None)]
    pub grid: Option<String>,
    #[uniffi(default = None)]
    pub axis: Option<String>,
    #[uniffi(default = None)]
    pub text: Option<String>,
    #[uniffi(default = None)]
    pub text_secondary: Option<String>,
    #[uniffi(default = None)]
    pub legend_bg: Option<String>,
    /// C0–C180 curve colour.
    #[uniffi(default = None)]
    pub curve_c0: Option<String>,
    #[uniffi(default = None)]
    pub curve_c0_fill: Option<String>,
    /// C90–C270 curve colour.
    #[uniffi(default = None)]
    pub curve_c90: Option<String>,
    #[uniffi(default = None)]
    pub curve_c90_fill: Option<String>,
    #[uniffi(default = None)]
    pub font_family: Option<String>,
    /// Curve stroke width in px. `None` inherits the base preset (2.5).
    /// Applied inline so it renders in CoreGraphics/`NSImage` (which ignore CSS).
    #[uniffi(default = None)]
    pub curve_width: Option<f64>,
}

impl Default for SvgThemeConfig {
    fn default() -> Self {
        Self {
            base: "light".into(),
            background: None,
            surface: None,
            grid: None,
            axis: None,
            text: None,
            text_secondary: None,
            legend_bg: None,
            curve_c0: None,
            curve_c0_fill: None,
            curve_c90: None,
            curve_c90_fill: None,
            font_family: None,
            curve_width: None,
        }
    }
}

impl SvgThemeConfig {
    /// Convert to a core [`SvgTheme`] (base preset + inline overrides).
    pub(crate) fn to_core(&self) -> SvgTheme {
        let overrides = ThemeOverrides {
            background: self.background.clone(),
            surface: self.surface.clone(),
            grid: self.grid.clone(),
            axis: self.axis.clone(),
            text: self.text.clone(),
            text_secondary: self.text_secondary.clone(),
            legend_bg: self.legend_bg.clone(),
            curve_c0_c180: self.curve_c0.clone(),
            curve_c0_c180_fill: self.curve_c0_fill.clone(),
            curve_c90_c270: self.curve_c90.clone(),
            curve_c90_c270_fill: self.curve_c90_fill.clone(),
            font_family: self.font_family.clone(),
            curve_width: self.curve_width,
        };
        SvgTheme::from_base_and_overrides(&self.base, &overrides)
    }

    /// Same, with localized diagram labels.
    pub(crate) fn to_core_with_locale(&self, locale: &Locale) -> SvgTheme {
        use eulumdat::diagram::SvgLabels;
        let mut theme = self.to_core();
        theme.labels = SvgLabels::from_locale(locale);
        theme
    }

    fn from_core(theme: &SvgTheme, base: &str) -> Self {
        Self {
            base: base.to_string(),
            background: Some(theme.background.clone()),
            surface: Some(theme.surface.clone()),
            grid: Some(theme.grid.clone()),
            axis: Some(theme.axis.clone()),
            text: Some(theme.text.clone()),
            text_secondary: Some(theme.text_secondary.clone()),
            legend_bg: Some(theme.legend_bg.clone()),
            curve_c0: Some(theme.curve_c0_c180.clone()),
            curve_c0_fill: Some(theme.curve_c0_c180_fill.clone()),
            curve_c90: Some(theme.curve_c90_c270.clone()),
            curve_c90_fill: Some(theme.curve_c90_c270_fill.clone()),
            font_family: Some(theme.font_family.clone()),
            curve_width: Some(theme.curve_width),
        }
    }
}

/// A fully-populated theme config from a built-in preset ("light", "dark",
/// "css"), so the UI can seed its pickers with real starting values.
#[uniffi::export]
pub fn theme_preset(name: String) -> SvgThemeConfig {
    let base = match name.as_str() {
        "dark" => "dark",
        "css" | "css_variables" => "css",
        _ => "light",
    };
    let theme = SvgThemeConfig {
        base: base.to_string(),
        ..Default::default()
    }
    .to_core();
    SvgThemeConfig::from_core(&theme, base)
}

/// Seed a config from an `SvgThemeType`, for callers still using the enum.
#[uniffi::export]
pub fn theme_config_from_type(theme_type: SvgThemeType) -> SvgThemeConfig {
    theme_preset(theme_type.base_name().to_string())
}

/// Parse a theme file (TOML) string into a config. Errors on malformed input.
#[uniffi::export]
pub fn theme_config_from_toml(toml: String) -> Result<SvgThemeConfig, EulumdatError> {
    let theme = SvgTheme::from_toml_str(&toml, None)
        .map_err(|e| EulumdatError::ParseError(e.to_string()))?;
    // Keep the file's chosen base preset (default "light") for the round trip.
    let base = SvgTheme::toml_base(&toml)
        .map_err(|e| EulumdatError::ParseError(e.to_string()))?
        .unwrap_or_else(|| "light".to_string());
    Ok(SvgThemeConfig::from_core(&theme, &base))
}

/// Serialize a config back to a TOML theme file string (for save/export).
#[uniffi::export]
pub fn theme_config_to_toml(config: SvgThemeConfig) -> String {
    // Emit a compact, human-editable theme file: base + the concrete fields.
    let mut out = String::new();
    out.push_str(&format!("base = \"{}\"\n", config.base));
    let line = |k: &str, v: &Option<String>| v.as_ref().map(|s| format!("{k} = \"{s}\"\n"));
    for (k, v) in [
        ("background", &config.background),
        ("surface", &config.surface),
        ("grid", &config.grid),
        ("axis", &config.axis),
        ("text", &config.text),
        ("text_secondary", &config.text_secondary),
        ("legend_bg", &config.legend_bg),
        ("curve_c0_c180", &config.curve_c0),
        ("curve_c0_c180_fill", &config.curve_c0_fill),
        ("curve_c90_c270", &config.curve_c90),
        ("curve_c90_c270_fill", &config.curve_c90_fill),
        ("font_family", &config.font_family),
    ] {
        if let Some(l) = line(k, v) {
            out.push_str(&l);
        }
    }
    if let Some(w) = config.curve_width {
        out.push_str(&format!("curve_width = {w}\n"));
    }
    out
}

// ── Themed diagram generators (take a full custom SvgThemeConfig) ─────────
//
// These mirror the `SvgThemeType`-based generators above but accept the
// UI-built `SvgThemeConfig`, so the native theme editor can drive colours,
// curve thickness, and font live. The `*_localized` variants also apply
// translated diagram labels.

/// Polar diagram SVG rendered with a custom theme config.
#[uniffi::export]
pub fn generate_polar_svg_themed(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    config: SvgThemeConfig,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_svg(width, height, &config.to_core())
}

/// Polar diagram SVG with a custom theme config and localized labels.
#[uniffi::export]
pub fn generate_polar_svg_themed_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    config: SvgThemeConfig,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    let summary = PhotometricSummary::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    polar.to_svg_with_summary(
        width,
        height,
        &config.to_core_with_locale(&locale),
        &summary,
    )
}

/// Cartesian diagram SVG rendered with a custom theme config.
#[uniffi::export]
pub fn generate_cartesian_svg_themed(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    max_curves: u32,
    config: SvgThemeConfig,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cartesian = CartesianDiagram::from_eulumdat(&core_ldt, width, height, max_curves as usize);
    cartesian.to_svg(width, height, &config.to_core())
}

/// Cartesian diagram SVG with a custom theme config and localized labels.
#[uniffi::export]
pub fn generate_cartesian_svg_themed_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    max_curves: u32,
    config: SvgThemeConfig,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cartesian = CartesianDiagram::from_eulumdat(&core_ldt, width, height, max_curves as usize);
    let summary = PhotometricSummary::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    cartesian.to_svg_with_summary(
        width,
        height,
        &config.to_core_with_locale(&locale),
        &summary,
    )
}

/// Heatmap diagram SVG rendered with a custom theme config.
#[uniffi::export]
pub fn generate_heatmap_svg_themed(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    config: SvgThemeConfig,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let heatmap = HeatmapDiagram::from_eulumdat(&core_ldt, width, height);
    heatmap.to_svg(width, height, &config.to_core())
}

/// Butterfly diagram SVG rendered with a custom theme config.
#[uniffi::export]
pub fn generate_butterfly_svg_themed(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    tilt_degrees: f64,
    config: SvgThemeConfig,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let butterfly = ButterflyDiagram::from_eulumdat(&core_ldt, width, height, tilt_degrees);
    butterfly.to_svg(width, height, &config.to_core())
}

// FFI functions for diagrams

/// Generate polar diagram data
#[uniffi::export]
pub fn generate_polar_diagram(ldt: &Eulumdat) -> PolarDiagramData {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);

    PolarDiagramData {
        c0_c180_curve: (&polar.c0_c180_curve).into(),
        c90_c270_curve: (&polar.c90_c270_curve).into(),
        scale: (&polar.scale).into(),
        symmetry: polar.symmetry.into(),
        show_c90_c270: polar.show_c90_c270(),
    }
}

/// Generate butterfly diagram data
#[uniffi::export]
pub fn generate_butterfly_diagram(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    tilt_degrees: f64,
) -> ButterflyDiagramData {
    let core_ldt = to_core_eulumdat(ldt);
    let butterfly = ButterflyDiagram::from_eulumdat(&core_ldt, width, height, tilt_degrees);

    ButterflyDiagramData {
        wings: butterfly
            .wings
            .iter()
            .map(|w| ButterflyWing {
                c_angle: w.c_angle,
                points: w.points.iter().map(|p| (*p).into()).collect(),
                fill_color: w.fill_color.into(),
                stroke_color: w.stroke_color.into(),
                svg_path: w.to_svg_path(),
            })
            .collect(),
        grid_circles: butterfly
            .grid_circles
            .iter()
            .map(|c| c.iter().map(|p| (*p).into()).collect())
            .collect(),
        c_plane_lines: butterfly
            .c_plane_lines
            .iter()
            .map(|(angle, start, end)| CPlaneDirection {
                angle: *angle,
                start: (*start).into(),
                end_point: (*end).into(),
            })
            .collect(),
        scale: (&butterfly.scale).into(),
        g_angles: butterfly.g_angles.clone(),
        symmetry: butterfly.symmetry.into(),
    }
}

/// Generate cartesian diagram data
#[uniffi::export]
pub fn generate_cartesian_diagram(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    max_curves: u32,
) -> CartesianDiagramData {
    let core_ldt = to_core_eulumdat(ldt);
    let cartesian = CartesianDiagram::from_eulumdat(&core_ldt, width, height, max_curves as usize);

    CartesianDiagramData {
        curves: cartesian
            .curves
            .iter()
            .map(|c| CartesianCurve {
                points: c.points.iter().map(|p| p.into()).collect(),
                c_angle: c.c_angle,
                color: c.color.into(),
                label: c.label.clone(),
                svg_path: c.to_svg_path(),
            })
            .collect(),
        x_ticks: cartesian.x_ticks.clone(),
        y_ticks: cartesian.y_ticks.clone(),
        scale: (&cartesian.scale).into(),
        max_gamma: cartesian.max_gamma,
        plot_width: cartesian.plot_width,
        plot_height: cartesian.plot_height,
        margin_left: cartesian.margin_left,
        margin_top: cartesian.margin_top,
    }
}

/// Generate heatmap diagram data
#[uniffi::export]
pub fn generate_heatmap_diagram(ldt: &Eulumdat, width: f64, height: f64) -> HeatmapDiagramData {
    let core_ldt = to_core_eulumdat(ldt);
    let heatmap = HeatmapDiagram::from_eulumdat(&core_ldt, width, height);

    HeatmapDiagramData {
        cells: heatmap
            .cells
            .iter()
            .map(|c| HeatmapCell {
                c_index: c.c_index as u32,
                g_index: c.g_index as u32,
                c_angle: c.c_angle,
                g_angle: c.g_angle,
                intensity: c.intensity,
                candela: c.candela,
                normalized: c.normalized,
                color: c.color.into(),
                x: c.x,
                y: c.y,
                width: c.width,
                height: c.height,
            })
            .collect(),
        scale: (&heatmap.scale).into(),
        max_candela: heatmap.max_candela,
        total_flux: heatmap.total_flux,
        c_angles: heatmap.c_angles.clone(),
        g_angles: heatmap.g_angles.clone(),
        legend_entries: heatmap
            .legend_entries
            .iter()
            .map(|(normalized, color, candela)| LegendEntry {
                normalized: *normalized,
                color: (*color).into(),
                candela: *candela,
            })
            .collect(),
        plot_width: heatmap.plot_width,
        plot_height: heatmap.plot_height,
        margin_left: heatmap.margin_left,
        margin_top: heatmap.margin_top,
    }
}

/// Generate polar diagram as SVG string
#[uniffi::export]
pub fn generate_polar_svg(ldt: &Eulumdat, width: f64, height: f64, theme: SvgThemeType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_svg(width, height, &theme.to_core())
}

/// Generate polar diagram as SVG string with localized labels
#[uniffi::export]
pub fn generate_polar_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    let summary = PhotometricSummary::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    polar.to_svg_with_summary(width, height, &theme.to_core_with_locale(&locale), &summary)
}

/// Generate cartesian diagram as SVG string
#[uniffi::export]
pub fn generate_cartesian_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    max_curves: u32,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cartesian = CartesianDiagram::from_eulumdat(&core_ldt, width, height, max_curves as usize);
    cartesian.to_svg(width, height, &theme.to_core())
}

/// Generate cartesian diagram as SVG string with localized labels
#[uniffi::export]
pub fn generate_cartesian_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    max_curves: u32,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cartesian = CartesianDiagram::from_eulumdat(&core_ldt, width, height, max_curves as usize);
    let summary = PhotometricSummary::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    cartesian.to_svg_with_summary(width, height, &theme.to_core_with_locale(&locale), &summary)
}

/// Generate heatmap diagram as SVG string
#[uniffi::export]
pub fn generate_heatmap_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let heatmap = HeatmapDiagram::from_eulumdat(&core_ldt, width, height);
    heatmap.to_svg(width, height, &theme.to_core())
}

/// Generate heatmap diagram as SVG string with localized labels
#[uniffi::export]
pub fn generate_heatmap_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let heatmap = HeatmapDiagram::from_eulumdat(&core_ldt, width, height);
    let summary = PhotometricSummary::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    heatmap.to_svg_with_summary(width, height, &theme.to_core_with_locale(&locale), &summary)
}

/// Generate butterfly diagram as SVG string
#[uniffi::export]
pub fn generate_butterfly_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    tilt_degrees: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let butterfly = ButterflyDiagram::from_eulumdat(&core_ldt, width, height, tilt_degrees);
    butterfly.to_svg(width, height, &theme.to_core())
}

/// Generate cone diagram as SVG string showing beam/field angle spread at mounting height
///
/// # Arguments
/// * `ldt` - The luminaire data
/// * `width` - SVG width in pixels
/// * `height` - SVG height in pixels
/// * `mounting_height` - Mounting height in meters
/// * `theme` - SVG color theme
#[uniffi::export]
pub fn generate_cone_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    mounting_height: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cone = ConeDiagram::from_eulumdat(&core_ldt, mounting_height);
    cone.to_svg(width, height, &theme.to_core())
}

/// Generate cone diagram as SVG string with localized labels
///
/// # Arguments
/// * `ldt` - The luminaire data
/// * `width` - SVG width in pixels
/// * `height` - SVG height in pixels
/// * `mounting_height` - Mounting height in meters
/// * `theme` - SVG color theme
/// * `language` - Language for labels
#[uniffi::export]
pub fn generate_cone_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    mounting_height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    use eulumdat::diagram::ConeDiagramLabels;

    let core_ldt = to_core_eulumdat(ldt);
    let cone = ConeDiagram::from_eulumdat(&core_ldt, mounting_height);
    let labels = match language {
        Language::German => ConeDiagramLabels::german(),
        Language::Chinese => ConeDiagramLabels::chinese(),
        Language::French => ConeDiagramLabels::french(),
        Language::Italian => ConeDiagramLabels::italian(),
        Language::Russian => ConeDiagramLabels::russian(),
        Language::Spanish => ConeDiagramLabels::spanish(),
        Language::PortugueseBrazil => ConeDiagramLabels::portuguese_brazil(),
        Language::English => ConeDiagramLabels::default(),
    };
    cone.to_svg_with_labels(width, height, &theme.to_core(), &labels)
}

/// Generate beam angle diagram as SVG comparing IES and CIE definitions
///
/// Shows 50% (beam) and 10% (field) intensity angles with annotations.
/// For batwing distributions, shows both main and secondary peaks.
///
/// # Arguments
/// * `ldt` - The luminaire data
/// * `width` - SVG width in pixels
/// * `height` - SVG height in pixels
/// * `theme` - SVG color theme
#[uniffi::export]
pub fn generate_beam_angle_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    let analysis = PhotometricCalculations::beam_field_analysis(&core_ldt);
    let show_both = analysis.is_batwing;
    polar.to_svg_with_beam_field_angles(width, height, &theme.to_core(), &analysis, show_both)
}

/// Generate beam angle diagram as SVG with localized labels
///
/// Shows 50% (beam) and 10% (field) intensity angles with annotations.
/// For batwing distributions, shows both main and secondary peaks.
///
/// # Arguments
/// * `ldt` - The luminaire data
/// * `width` - SVG width in pixels
/// * `height` - SVG height in pixels
/// * `theme` - SVG color theme
/// * `language` - Language for labels
#[uniffi::export]
pub fn generate_beam_angle_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    let analysis = PhotometricCalculations::beam_field_analysis(&core_ldt);
    let show_both = analysis.is_batwing;
    let locale = language.to_locale();
    polar.to_svg_with_beam_field_angles(
        width,
        height,
        &theme.to_core_with_locale(&locale),
        &analysis,
        show_both,
    )
}

/// Generate butterfly diagram as SVG string with localized labels
#[uniffi::export]
pub fn generate_butterfly_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    tilt_degrees: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let butterfly = ButterflyDiagram::from_eulumdat(&core_ldt, width, height, tilt_degrees);
    let locale = language.to_locale();
    butterfly.to_svg(width, height, &theme.to_core_with_locale(&locale))
}

// BUG and LCS diagram functions are in bug_rating.rs

// === Isolux diagram ===

/// Generate isolux footprint diagram as SVG
#[uniffi::export]
pub fn generate_isolux_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    mounting_height: f64,
    tilt_angle: f64,
    area_size: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let params = IsoluxParams {
        mounting_height,
        tilt_angle,
        area_half_width: area_size / 2.0,
        area_half_depth: area_size / 2.0,
        grid_resolution: 80,
    };
    let diagram = IsoluxDiagram::from_eulumdat(&core_ldt, width, height, params);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate isolux footprint diagram as SVG with localized labels
#[allow(clippy::too_many_arguments)]
#[uniffi::export]
pub fn generate_isolux_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    mounting_height: f64,
    tilt_angle: f64,
    area_size: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let locale = language.to_locale();
    let params = IsoluxParams {
        mounting_height,
        tilt_angle,
        area_half_width: area_size / 2.0,
        area_half_depth: area_size / 2.0,
        grid_resolution: 80,
    };
    let diagram = IsoluxDiagram::from_eulumdat(&core_ldt, width, height, params);
    diagram.to_svg(width, height, &theme.to_core_with_locale(&locale))
}

// === Isocandela diagram ===

/// Generate isocandela contour diagram as SVG
#[uniffi::export]
pub fn generate_isocandela_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = IsocandelaDiagram::from_eulumdat(&core_ldt, width, height);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate isocandela contour diagram as SVG with localized labels
#[uniffi::export]
pub fn generate_isocandela_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let locale = language.to_locale();
    let diagram = IsocandelaDiagram::from_eulumdat(&core_ldt, width, height);
    diagram.to_svg(width, height, &theme.to_core_with_locale(&locale))
}

// === Floodlight cartesian diagram ===

/// Generate floodlight V-H cartesian diagram as SVG
#[uniffi::export]
pub fn generate_floodlight_cartesian_svg(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    log_scale: bool,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let y_scale = if log_scale {
        YScale::Logarithmic
    } else {
        YScale::Linear
    };
    let diagram = FloodlightCartesianDiagram::from_eulumdat(&core_ldt, width, height, y_scale);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate floodlight V-H cartesian diagram as SVG with localized labels
#[uniffi::export]
pub fn generate_floodlight_cartesian_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    log_scale: bool,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let locale = language.to_locale();
    let y_scale = if log_scale {
        YScale::Logarithmic
    } else {
        YScale::Linear
    };
    let diagram = FloodlightCartesianDiagram::from_eulumdat(&core_ldt, width, height, y_scale);
    diagram.to_svg(width, height, &theme.to_core_with_locale(&locale))
}

// === Per-C-plane diagram variants ===

/// Generate polar diagram SVG for a specific C-plane
#[uniffi::export]
pub fn generate_polar_svg_for_plane(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    c_plane: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat_for_plane(&core_ldt, c_plane);
    polar.to_svg(width, height, &theme.to_core())
}

/// Generate cartesian diagram SVG for a specific C-plane
#[uniffi::export]
pub fn generate_cartesian_svg_for_plane(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    c_plane: f64,
    _max_curves: u32,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cartesian = CartesianDiagram::from_eulumdat_for_plane(&core_ldt, c_plane, width, height);
    cartesian.to_svg(width, height, &theme.to_core())
}

/// Generate cone diagram SVG for a specific C-plane
#[uniffi::export]
pub fn generate_cone_svg_for_plane(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    mounting_height: f64,
    c_plane: f64,
    theme: SvgThemeType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let cone = ConeDiagram::from_eulumdat_for_plane(&core_ldt, mounting_height, c_plane);
    cone.to_svg(width, height, &theme.to_core())
}

/// Check if luminaire has significant C-plane variation (asymmetric)
#[uniffi::export]
pub fn has_c_plane_variation(ldt: &Eulumdat) -> bool {
    let core_ldt = to_core_eulumdat(ldt);
    ConeDiagram::has_c_plane_variation(&core_ldt)
}

/// Get the expanded list of C-plane angles (accounting for symmetry)
#[uniffi::export]
pub fn get_expanded_c_angles(ldt: &Eulumdat) -> Vec<f64> {
    let core_ldt = to_core_eulumdat(ldt);
    SymmetryHandler::expand_c_angles(&core_ldt)
}

// === Overlay diagrams for comparison ===

/// Generate polar overlay SVG comparing two luminaires
#[allow(clippy::too_many_arguments)]
#[uniffi::export]
pub fn generate_polar_overlay_svg(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    label_a: String,
    label_b: String,
    c_plane_a: Option<f64>,
    c_plane_b: Option<f64>,
) -> String {
    let core_a = to_core_eulumdat(ldt_a);
    let core_b = to_core_eulumdat(ldt_b);
    let polar_a = match c_plane_a {
        Some(c) => PolarDiagram::from_eulumdat_for_plane(&core_a, c),
        None => PolarDiagram::from_eulumdat(&core_a),
    };
    let polar_b = match c_plane_b {
        Some(c) => PolarDiagram::from_eulumdat_for_plane(&core_b, c),
        None => PolarDiagram::from_eulumdat(&core_b),
    };
    PolarDiagram::to_overlay_svg(
        &polar_a,
        &polar_b,
        width,
        height,
        &theme.to_core(),
        &label_a,
        &label_b,
    )
}

/// Generate cartesian overlay SVG comparing two luminaires
#[allow(clippy::too_many_arguments)]
#[uniffi::export]
pub fn generate_cartesian_overlay_svg(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    label_a: String,
    label_b: String,
    c_plane_a: Option<f64>,
    c_plane_b: Option<f64>,
) -> String {
    let core_a = to_core_eulumdat(ldt_a);
    let core_b = to_core_eulumdat(ldt_b);
    let cartesian_a = match c_plane_a {
        Some(c) => CartesianDiagram::from_eulumdat_for_plane(&core_a, c, width, height),
        None => CartesianDiagram::from_eulumdat(&core_a, width, height, 8),
    };
    let cartesian_b = match c_plane_b {
        Some(c) => CartesianDiagram::from_eulumdat_for_plane(&core_b, c, width, height),
        None => CartesianDiagram::from_eulumdat(&core_b, width, height, 8),
    };
    CartesianDiagram::to_overlay_svg(
        &cartesian_a,
        &cartesian_b,
        width,
        height,
        &theme.to_core(),
        &label_a,
        &label_b,
    )
}

// Watch face types and functions

/// Watch face style preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WatchFaceStyleType {
    /// Dark style with cyan curves (default)
    Dark,
    /// Light style with blue curves
    Light,
    /// Minimal style - curves only, less grid
    Minimal,
    /// Complication style - optimized for 120x120
    Complication,
    /// California style - warm amber tones
    California,
}

impl WatchFaceStyleType {
    fn to_core(self) -> WatchFaceStyle {
        match self {
            WatchFaceStyleType::Dark => WatchFaceStyle::dark(),
            WatchFaceStyleType::Light => WatchFaceStyle::light(),
            WatchFaceStyleType::Minimal => WatchFaceStyle::minimal(),
            WatchFaceStyleType::Complication => WatchFaceStyle::complication(),
            WatchFaceStyleType::California => WatchFaceStyle::california(),
        }
    }
}

/// Custom watch face style configuration
#[derive(Debug, Clone, uniffi::Record)]
pub struct WatchFaceStyleCustom {
    /// Background color (use "transparent" for PNG with alpha)
    pub background: String,
    /// Grid/hour marker color
    pub grid_color: String,
    /// Main curve color (C0-C180)
    pub curve_primary: String,
    /// Secondary curve color (C90-C270)
    pub curve_secondary: String,
    /// Fill opacity for curves (0.0-1.0)
    pub fill_opacity: f64,
    /// Whether to show hour markers (12, 3, 6, 9)
    pub show_hour_markers: bool,
    /// Whether to show minute tick marks
    pub show_minute_ticks: bool,
    /// Whether to show the secondary (C90-C270) curve
    pub show_secondary_curve: bool,
    /// Stroke width for curves
    pub curve_stroke_width: f64,
    /// Grid line width
    pub grid_stroke_width: f64,
}

impl From<&WatchFaceStyleCustom> for WatchFaceStyle {
    fn from(s: &WatchFaceStyleCustom) -> Self {
        WatchFaceStyle {
            background: s.background.clone(),
            grid_color: s.grid_color.clone(),
            curve_primary: s.curve_primary.clone(),
            curve_secondary: s.curve_secondary.clone(),
            fill_opacity: s.fill_opacity,
            show_hour_markers: s.show_hour_markers,
            show_minute_ticks: s.show_minute_ticks,
            show_secondary_curve: s.show_secondary_curve,
            curve_stroke_width: s.curve_stroke_width,
            grid_stroke_width: s.grid_stroke_width,
        }
    }
}

/// Generate watch face SVG with a preset style
///
/// Creates a circular SVG suitable for Apple Watch faces.
/// The polar grid doubles as watch hour/minute markers.
///
/// # Arguments
/// * `ldt` - The luminaire data
/// * `size` - Width and height in pixels (e.g., 396 for 45mm watch)
/// * `style` - Watch face style preset
#[uniffi::export]
pub fn generate_watch_face_svg(ldt: &Eulumdat, size: u32, style: WatchFaceStyleType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_watch_face_svg(size, size, &style.to_core())
}

/// Generate watch face SVG with a custom style
#[uniffi::export]
pub fn generate_watch_face_svg_custom(
    ldt: &Eulumdat,
    size: u32,
    style: &WatchFaceStyleCustom,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_watch_face_svg(size, size, &style.into())
}

/// Generate complication SVG (120x120 max for accessoryCircular)
#[uniffi::export]
pub fn generate_complication_svg(ldt: &Eulumdat, size: u32) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_complication_svg(size)
}

/// Generate Photos face SVG (for Apple Watch Photos face background)
///
/// # Arguments
/// * `ldt` - The luminaire data
/// * `width` - Width in pixels (e.g., 396 for 45mm)
/// * `height` - Height in pixels (e.g., 484 for 45mm)
/// * `style` - Watch face style preset
#[uniffi::export]
pub fn generate_photos_face_svg(
    ldt: &Eulumdat,
    width: u32,
    height: u32,
    style: WatchFaceStyleType,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_photos_face_svg(width, height, &style.to_core())
}

#[cfg(test)]
mod theme_config_tests {
    use super::*;

    #[test]
    fn preset_fills_every_colour_field() {
        let light = theme_preset("light".into());
        assert_eq!(light.base, "light");
        assert!(light.background.is_some());
        assert!(light.curve_c0.is_some());
        assert!(light.font_family.is_some());

        let dark = theme_preset("dark".into());
        assert_eq!(dark.base, "dark");
        // Light and dark backgrounds must differ.
        assert_ne!(light.background, dark.background);
    }

    #[test]
    fn unknown_preset_name_falls_back_to_light() {
        let cfg = theme_preset("chartreuse".into());
        assert_eq!(cfg.base, "light");
    }

    #[test]
    fn config_overrides_reach_the_core_theme() {
        let cfg = SvgThemeConfig {
            base: "dark".into(),
            curve_c0: Some("#e4002b".into()),
            font_family: Some("Georgia".into()),
            ..Default::default()
        };
        let core = cfg.to_core();
        // Overridden fields win over the dark base.
        assert_eq!(core.curve_c0_c180, "#e4002b");
        assert_eq!(core.font_family, "Georgia");
        // Un-overridden field still comes from the dark base (non-empty).
        assert!(!core.background.is_empty());
    }

    #[test]
    fn curve_width_reaches_the_core_theme() {
        let cfg = SvgThemeConfig {
            curve_width: Some(5.0),
            ..Default::default()
        };
        assert_eq!(cfg.to_core().curve_width, 5.0);
    }

    #[test]
    fn config_survives_toml_round_trip() {
        let cfg = SvgThemeConfig {
            base: "dark".into(),
            background: Some("#101018".into()),
            curve_c0: Some("#e4002b".into()),
            font_family: Some("Georgia".into()),
            curve_width: Some(4.0),
            ..Default::default()
        };

        let toml = theme_config_to_toml(cfg.clone());
        let parsed = theme_config_from_toml(toml).expect("valid TOML");

        // The base preset, concrete colours and width all survive.
        assert_eq!(parsed.base, "dark");
        assert_eq!(parsed.background.as_deref(), Some("#101018"));
        assert_eq!(parsed.curve_c0.as_deref(), Some("#e4002b"));
        assert_eq!(parsed.font_family.as_deref(), Some("Georgia"));
        assert_eq!(parsed.curve_width, Some(4.0));
    }

    #[test]
    fn malformed_toml_is_a_clean_error() {
        let err = theme_config_from_toml("this is not = = valid toml".into());
        assert!(err.is_err());
    }
}
