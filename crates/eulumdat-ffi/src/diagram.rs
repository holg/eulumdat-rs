//! Diagram types and functions for FFI

use eulumdat::diagram::{
    ButterflyDiagram, CartesianDiagram, ConeDiagram, HeatmapDiagram, PolarDiagram, SvgTheme,
    WatchFaceStyle,
};
use eulumdat::{PhotometricCalculations, PhotometricSummary};
use eulumdat_i18n::{Language as CoreLanguage, Locale};

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
