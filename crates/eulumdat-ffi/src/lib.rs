//! FFI bindings for eulumdat-core
//!
//! This crate provides UniFFI bindings to expose the eulumdat-core library
//! to Swift, Kotlin, Python, and other languages.
//!
//! # Generating Bindings
//!
//! To generate bindings for different languages:
//!
//! ```bash
//! # Swift
//! cargo run --bin uniffi-bindgen generate --library target/release/libeulumdat_ffi.dylib --language swift --out-dir ./bindings/swift
//!
//! # Kotlin
//! cargo run --bin uniffi-bindgen generate --library target/release/libeulumdat_ffi.dylib --language kotlin --out-dir ./bindings/kotlin
//!
//! # Python
//! cargo run --bin uniffi-bindgen generate --library target/release/libeulumdat_ffi.dylib --language python --out-dir ./bindings/python
//! ```

use eulumdat::{
    diagram::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme},
    Eulumdat as CoreEulumdat, LampSet as CoreLampSet, Symmetry as CoreSymmetry,
    TypeIndicator as CoreTypeIndicator,
};

uniffi::setup_scaffolding!();

// Error type for FFI
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum EulumdatError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

// FFI-compatible types

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Symmetry {
    None,
    VerticalAxis,
    PlaneC0C180,
    PlaneC90C270,
    BothPlanes,
}

impl From<CoreSymmetry> for Symmetry {
    fn from(s: CoreSymmetry) -> Self {
        match s {
            CoreSymmetry::None => Symmetry::None,
            CoreSymmetry::VerticalAxis => Symmetry::VerticalAxis,
            CoreSymmetry::PlaneC0C180 => Symmetry::PlaneC0C180,
            CoreSymmetry::PlaneC90C270 => Symmetry::PlaneC90C270,
            CoreSymmetry::BothPlanes => Symmetry::BothPlanes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TypeIndicator {
    PointSourceSymmetric,
    Linear,
    PointSourceOther,
}

impl From<CoreTypeIndicator> for TypeIndicator {
    fn from(t: CoreTypeIndicator) -> Self {
        match t {
            CoreTypeIndicator::PointSourceSymmetric => TypeIndicator::PointSourceSymmetric,
            CoreTypeIndicator::Linear => TypeIndicator::Linear,
            CoreTypeIndicator::PointSourceOther => TypeIndicator::PointSourceOther,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LampSet {
    pub num_lamps: i32,
    pub lamp_type: String,
    pub total_luminous_flux: f64,
    pub color_appearance: String,
    pub color_rendering_group: String,
    pub wattage_with_ballast: f64,
}

impl From<&CoreLampSet> for LampSet {
    fn from(ls: &CoreLampSet) -> Self {
        Self {
            num_lamps: ls.num_lamps,
            lamp_type: ls.lamp_type.clone(),
            total_luminous_flux: ls.total_luminous_flux,
            color_appearance: ls.color_appearance.clone(),
            color_rendering_group: ls.color_rendering_group.clone(),
            wattage_with_ballast: ls.wattage_with_ballast,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Eulumdat {
    // Identification
    pub identification: String,
    pub type_indicator: TypeIndicator,
    pub symmetry: Symmetry,

    // Grid
    pub num_c_planes: u32,
    pub c_plane_distance: f64,
    pub num_g_planes: u32,
    pub g_plane_distance: f64,

    // Metadata
    pub measurement_report_number: String,
    pub luminaire_name: String,
    pub luminaire_number: String,
    pub file_name: String,
    pub date_user: String,

    // Physical dimensions (mm)
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub luminous_area_length: f64,
    pub luminous_area_width: f64,
    pub height_c0: f64,
    pub height_c90: f64,
    pub height_c180: f64,
    pub height_c270: f64,

    // Optical properties
    pub downward_flux_fraction: f64,
    pub light_output_ratio: f64,
    pub conversion_factor: f64,
    pub tilt_angle: f64,

    // Lamp configuration
    pub lamp_sets: Vec<LampSet>,

    // Utilization factors
    pub direct_ratios: Vec<f64>,

    // Photometric data
    pub c_angles: Vec<f64>,
    pub g_angles: Vec<f64>,
    pub intensities: Vec<Vec<f64>>,

    // Computed values
    pub max_intensity: f64,
    pub total_luminous_flux: f64,
}

impl From<&CoreEulumdat> for Eulumdat {
    fn from(ldt: &CoreEulumdat) -> Self {
        Self {
            identification: ldt.identification.clone(),
            type_indicator: ldt.type_indicator.into(),
            symmetry: ldt.symmetry.into(),
            num_c_planes: ldt.num_c_planes as u32,
            c_plane_distance: ldt.c_plane_distance,
            num_g_planes: ldt.num_g_planes as u32,
            g_plane_distance: ldt.g_plane_distance,
            measurement_report_number: ldt.measurement_report_number.clone(),
            luminaire_name: ldt.luminaire_name.clone(),
            luminaire_number: ldt.luminaire_number.clone(),
            file_name: ldt.file_name.clone(),
            date_user: ldt.date_user.clone(),
            length: ldt.length,
            width: ldt.width,
            height: ldt.height,
            luminous_area_length: ldt.luminous_area_length,
            luminous_area_width: ldt.luminous_area_width,
            height_c0: ldt.height_c0,
            height_c90: ldt.height_c90,
            height_c180: ldt.height_c180,
            height_c270: ldt.height_c270,
            downward_flux_fraction: ldt.downward_flux_fraction,
            light_output_ratio: ldt.light_output_ratio,
            conversion_factor: ldt.conversion_factor,
            tilt_angle: ldt.tilt_angle,
            lamp_sets: ldt.lamp_sets.iter().map(|ls| ls.into()).collect(),
            direct_ratios: ldt.direct_ratios.to_vec(),
            c_angles: ldt.c_angles.clone(),
            g_angles: ldt.g_angles.clone(),
            intensities: ldt.intensities.clone(),
            max_intensity: ldt.max_intensity(),
            total_luminous_flux: ldt.total_luminous_flux(),
        }
    }
}

// Diagram types

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

// FFI functions

/// Parse LDT content and return an Eulumdat object
#[uniffi::export]
pub fn parse_ldt(content: String) -> Result<Eulumdat, EulumdatError> {
    CoreEulumdat::parse(&content)
        .map(|ldt| (&ldt).into())
        .map_err(|e| EulumdatError::ParseError(e.to_string()))
}

/// Generate polar diagram data
#[uniffi::export]
pub fn generate_polar_diagram(ldt: &Eulumdat) -> PolarDiagramData {
    // Convert back to core type for processing
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

// SVG Theme enum for FFI
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SvgThemeType {
    Light,
    Dark,
    CssVariables,
}

impl SvgThemeType {
    fn to_core(self) -> SvgTheme {
        match self {
            SvgThemeType::Light => SvgTheme::light(),
            SvgThemeType::Dark => SvgTheme::dark(),
            SvgThemeType::CssVariables => SvgTheme::css_variables(),
        }
    }
}

/// Generate polar diagram as SVG string
#[uniffi::export]
pub fn generate_polar_svg(ldt: &Eulumdat, width: f64, height: f64, theme: SvgThemeType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let polar = PolarDiagram::from_eulumdat(&core_ldt);
    polar.to_svg(width, height, &theme.to_core())
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

// BUG Rating types and functions

/// Zone lumens data for BUG rating
#[derive(Debug, Clone, uniffi::Record)]
pub struct ZoneLumens {
    pub bl: f64,  // Backlight Low
    pub bm: f64,  // Backlight Mid
    pub bh: f64,  // Backlight High
    pub bvh: f64, // Backlight Very High
    pub fl: f64,  // Forward Low
    pub fm: f64,  // Forward Mid
    pub fh: f64,  // Forward High
    pub fvh: f64, // Forward Very High
    pub ul: f64,  // Uplight Low
    pub uh: f64,  // Uplight High
}

impl From<eulumdat::ZoneLumens> for ZoneLumens {
    fn from(z: eulumdat::ZoneLumens) -> Self {
        Self {
            bl: z.bl,
            bm: z.bm,
            bh: z.bh,
            bvh: z.bvh,
            fl: z.fl,
            fm: z.fm,
            fh: z.fh,
            fvh: z.fvh,
            ul: z.ul,
            uh: z.uh,
        }
    }
}

/// BUG rating values (0-5 scale)
#[derive(Debug, Clone, uniffi::Record)]
pub struct BugRatingData {
    pub b: u8,
    pub u: u8,
    pub g: u8,
}

impl From<eulumdat::BugRating> for BugRatingData {
    fn from(r: eulumdat::BugRating) -> Self {
        Self {
            b: r.b,
            u: r.u,
            g: r.g,
        }
    }
}

/// Complete BUG diagram data
#[derive(Debug, Clone, uniffi::Record)]
pub struct BugDiagramData {
    pub zones: ZoneLumens,
    pub rating: BugRatingData,
    pub total_lumens: f64,
}

/// Calculate BUG rating from Eulumdat data
#[uniffi::export]
pub fn calculate_bug_rating(ldt: &Eulumdat) -> BugRatingData {
    let core_ldt = to_core_eulumdat(ldt);
    eulumdat::BugRating::from_eulumdat(&core_ldt).into()
}

/// Generate BUG diagram data
#[uniffi::export]
pub fn generate_bug_diagram(ldt: &Eulumdat) -> BugDiagramData {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    BugDiagramData {
        zones: diagram.zones.into(),
        rating: diagram.rating.into(),
        total_lumens: diagram.total_lumens,
    }
}

/// Generate BUG rating diagram as SVG (TM-15-11 view)
#[uniffi::export]
pub fn generate_bug_svg(ldt: &Eulumdat, width: f64, height: f64, theme: SvgThemeType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate LCS diagram as SVG (TM-15-07 view)
#[uniffi::export]
pub fn generate_lcs_svg(ldt: &Eulumdat, width: f64, height: f64, theme: SvgThemeType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    diagram.to_lcs_svg(width, height, &theme.to_core())
}

// Helper to convert FFI Eulumdat back to core type
fn to_core_eulumdat(ldt: &Eulumdat) -> CoreEulumdat {
    CoreEulumdat {
        identification: ldt.identification.clone(),
        type_indicator: match ldt.type_indicator {
            TypeIndicator::PointSourceSymmetric => CoreTypeIndicator::PointSourceSymmetric,
            TypeIndicator::Linear => CoreTypeIndicator::Linear,
            TypeIndicator::PointSourceOther => CoreTypeIndicator::PointSourceOther,
        },
        symmetry: match ldt.symmetry {
            Symmetry::None => CoreSymmetry::None,
            Symmetry::VerticalAxis => CoreSymmetry::VerticalAxis,
            Symmetry::PlaneC0C180 => CoreSymmetry::PlaneC0C180,
            Symmetry::PlaneC90C270 => CoreSymmetry::PlaneC90C270,
            Symmetry::BothPlanes => CoreSymmetry::BothPlanes,
        },
        num_c_planes: ldt.num_c_planes as usize,
        c_plane_distance: ldt.c_plane_distance,
        num_g_planes: ldt.num_g_planes as usize,
        g_plane_distance: ldt.g_plane_distance,
        measurement_report_number: ldt.measurement_report_number.clone(),
        luminaire_name: ldt.luminaire_name.clone(),
        luminaire_number: ldt.luminaire_number.clone(),
        file_name: ldt.file_name.clone(),
        date_user: ldt.date_user.clone(),
        length: ldt.length,
        width: ldt.width,
        height: ldt.height,
        luminous_area_length: ldt.luminous_area_length,
        luminous_area_width: ldt.luminous_area_width,
        height_c0: ldt.height_c0,
        height_c90: ldt.height_c90,
        height_c180: ldt.height_c180,
        height_c270: ldt.height_c270,
        downward_flux_fraction: ldt.downward_flux_fraction,
        light_output_ratio: ldt.light_output_ratio,
        conversion_factor: ldt.conversion_factor,
        tilt_angle: ldt.tilt_angle,
        lamp_sets: ldt
            .lamp_sets
            .iter()
            .map(|ls| CoreLampSet {
                num_lamps: ls.num_lamps,
                lamp_type: ls.lamp_type.clone(),
                total_luminous_flux: ls.total_luminous_flux,
                color_appearance: ls.color_appearance.clone(),
                color_rendering_group: ls.color_rendering_group.clone(),
                wattage_with_ballast: ls.wattage_with_ballast,
            })
            .collect(),
        direct_ratios: ldt.direct_ratios.clone().try_into().unwrap_or([0.0; 10]),
        c_angles: ldt.c_angles.clone(),
        g_angles: ldt.g_angles.clone(),
        intensities: ldt.intensities.clone(),
    }
}
