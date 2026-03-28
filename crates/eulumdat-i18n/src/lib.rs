//! Internationalization for Eulumdat/ATLA photometric libraries
//!
//! This crate provides localized strings for:
//! - SVG diagram labels (polar, cartesian, heatmap, spectral)
//! - UI strings for web and mobile apps
//! - Report generation
//!
//! # Supported Languages
//!
//! - English (en) - default
//! - German (de)
//! - Chinese Simplified (zh)
//! - French (fr)
//! - Italian (it)
//! - Russian (ru)
//! - Spanish (es)
//! - Portuguese Brazilian (pt-BR)
//!
//! # Usage
//!
//! ```rust
//! use eulumdat_i18n::{Locale, Language};
//!
//! // Get English locale (default)
//! let en = Locale::english();
//! assert_eq!(en.diagram.axis.gamma, "Gamma (γ)");
//!
//! // Get German locale
//! let de = Locale::german();
//! assert_eq!(de.diagram.axis.intensity, "Lichtstärke (cd/klm)");
//!
//! // Get by language code
//! let locale = Locale::for_language(Language::Chinese);
//! assert_eq!(locale.diagram.placeholder.no_data, "无数据");
//! ```

use serde::{Deserialize, Serialize};

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
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
    /// Get language from ISO 639-1 code
    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "en" => Self::English,
            "de" => Self::German,
            "zh" | "zh-cn" | "zh-hans" => Self::Chinese,
            "fr" => Self::French,
            "it" => Self::Italian,
            "ru" => Self::Russian,
            "es" => Self::Spanish,
            "pt" | "pt-br" => Self::PortugueseBrazil,
            _ => Self::English,
        }
    }

    /// Get ISO 639-1 code
    pub fn code(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::German => "de",
            Self::Chinese => "zh",
            Self::French => "fr",
            Self::Italian => "it",
            Self::Russian => "ru",
            Self::Spanish => "es",
            Self::PortugueseBrazil => "pt-BR",
        }
    }

    /// Get native language name
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
            Self::Chinese => "简体中文",
            Self::French => "Français",
            Self::Italian => "Italiano",
            Self::Russian => "Русский",
            Self::Spanish => "Español",
            Self::PortugueseBrazil => "Português (Brasil)",
        }
    }

    /// All available languages
    pub fn all() -> &'static [Language] {
        &[
            Self::English,
            Self::German,
            Self::Chinese,
            Self::French,
            Self::Italian,
            Self::Russian,
            Self::Spanish,
            Self::PortugueseBrazil,
        ]
    }
}

/// Complete locale with all translatable strings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Locale {
    pub meta: LocaleMeta,
    pub diagram: DiagramLocale,
    pub spectral: SpectralLocale,
    pub luminaire: LuminaireLocale,
    pub validation: ValidationLocale,
    pub ui: UiLocale,
    pub report: ReportLocale,
    #[serde(default)]
    pub comparison: ComparisonLocale,
    #[serde(default)]
    pub designer: DesignerLocale,
    #[serde(default)]
    pub area_designer: AreaDesignerLocale,
    #[serde(default)]
    pub zonal_designer: ZonalDesignerLocale,
    #[serde(default)]
    pub dashboard: DashboardLocale,
    #[serde(default)]
    pub maps_designer: MapsDesignerLocale,
    #[serde(default)]
    pub goniosim: GoniosimLocale,
}

/// Locale metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocaleMeta {
    pub language: String,
    pub code: String,
    pub direction: String,
}

/// Diagram-related translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramLocale {
    pub units: DiagramUnits,
    pub bug: DiagramBug,
    pub axis: DiagramAxis,
    pub plane: DiagramPlane,
    pub angle: DiagramAngle,
    pub metrics: DiagramMetrics,
    pub title: DiagramTitle,
    pub placeholder: DiagramPlaceholder,
    pub cone: DiagramCone,
    pub greenhouse: DiagramGreenhouse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramBug {
    pub forward_light: String,
    pub back_light: String,
    pub uplight: String,
    pub total: String,
    pub sum: String,
    pub zone_low: String,
    pub zone_medium: String,
    pub zone_high: String,
    pub zone_very_high: String,
    pub lumens: String,
    pub percent: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramUnits {
    pub intensity: String,
    pub intensity_short: String,
    pub candela: String,
    pub lumen: String,
    pub watt: String,
    pub lux: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramAxis {
    pub gamma: String,
    pub intensity: String,
    pub c_plane: String,
    pub gamma_angle: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramPlane {
    pub c0_c180: String,
    pub c90_c270: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramAngle {
    pub beam: String,
    pub field: String,
    pub beam_50: String,
    pub field_10: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramMetrics {
    pub cie: String,
    pub efficacy: String,
    pub max: String,
    pub sh_ratio: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramTitle {
    pub heatmap: String,
    pub polar: String,
    pub cartesian: String,
    pub cone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramPlaceholder {
    pub no_data: String,
}

/// Cone diagram translations (beam spread visualization)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramCone {
    pub beam_angle: String,
    pub field_angle: String,
    pub mounting_height: String,
    pub beam_diameter: String,
    pub field_diameter: String,
    pub intensity_50: String,
    pub intensity_10: String,
    pub floor: String,
    pub meter: String,
    pub classification: ConeClassification,
    pub c_plane: String,
    pub all_planes: String,
    pub symmetric_note: String,
    pub illuminance_table: ConeIlluminanceTableLocale,
}

/// Cone illuminance table translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConeIlluminanceTableLocale {
    pub title: String,
    pub height: String,
    pub beam_field_diameter: String,
    pub e_nadir: String,
    pub e_c0: String,
    pub e_c90: String,
    pub no_flux: String,
}

/// Beam classification labels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConeClassification {
    pub very_narrow_spot: String,
    pub narrow_spot: String,
    pub spot: String,
    pub medium_flood: String,
    pub wide_flood: String,
    pub very_wide_flood: String,
}

/// Greenhouse diagram translations (PPFD at distance)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramGreenhouse {
    pub max_height: String,
}

/// Spectral diagram translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralLocale {
    pub axis: SpectralAxis,
    pub title: SpectralTitle,
    pub region: SpectralRegion,
    pub warning: SpectralWarning,
    pub units: SpectralUnits,
    pub tm30: Tm30Locale,
    pub metrics: SpectralMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralAxis {
    pub wavelength: String,
    pub relative_power: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralTitle {
    pub spd: String,
    pub cvg: String,
    pub hue: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralRegion {
    pub uv_a: String,
    pub visible: String,
    pub near_ir: String,
    pub blue: String,
    pub green: String,
    pub red: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralWarning {
    pub uv_thermal: String,
    pub uv_exposure: String,
    pub high_thermal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralUnits {
    pub watts_per_nm: String,
    pub relative: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tm30Locale {
    pub rf: String,
    pub rg: String,
    pub reference: String,
    pub test: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralMetrics {
    pub energy_distribution: String,
    pub uv_percent: String,
    pub visible_percent: String,
    pub ir_percent: String,
    pub r_fr_ratio: String,
}

/// Luminaire information translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireLocale {
    pub info: LuminaireInfo,
    pub physical: LuminairePhysical,
    pub optical: LuminaireOptical,
    pub photometric: LuminairePhotometric,
    pub electrical: LuminaireElectrical,
    pub lamp_set: LampSetLocale,
    pub summary: SummaryLocale,
    pub direct_ratios: DirectRatiosLocale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireInfo {
    pub manufacturer: String,
    pub catalog_number: String,
    pub description: String,
    pub luminaire_name: String,
    pub luminaire_number: String,
    pub luminaire_type: String,
    pub lamp_type: String,
    pub test_date: String,
    pub report_number: String,
    pub laboratory: String,
    pub identification: String,
    pub type_indicator: String,
    pub type_indicator_1: String,
    pub type_indicator_2: String,
    pub type_indicator_3: String,
    pub symmetry: String,
    pub symmetry_0: String,
    pub symmetry_1: String,
    pub symmetry_2: String,
    pub symmetry_3: String,
    pub symmetry_4: String,
    pub num_c_planes: String,
    pub c_plane_distance: String,
    pub num_g_planes: String,
    pub g_plane_distance: String,
    pub measurement_report: String,
    pub file_name: String,
    pub date_user: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminairePhysical {
    pub dimensions: String,
    pub dimensions_mm: String,
    pub length: String,
    pub length_diameter: String,
    pub width: String,
    pub width_b: String,
    pub height: String,
    pub height_h: String,
    pub luminous_area: String,
    pub luminous_area_mm: String,
    pub luminous_length: String,
    pub luminous_width: String,
    pub luminous_height_c_planes: String,
    pub mounting: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireOptical {
    pub title: String,
    pub downward_flux_fraction: String,
    pub light_output_ratio: String,
    pub conversion_factor: String,
    pub tilt_angle: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminairePhotometric {
    pub total_flux: String,
    pub total_lamp_flux: String,
    pub total_wattage: String,
    pub lamp_efficacy: String,
    pub luminaire_efficacy: String,
    pub efficacy: String,
    pub lor: String,
    pub dlor: String,
    pub ulor: String,
    pub cct: String,
    pub cri: String,
    pub beam_angle: String,
    pub beam_angle_50: String,
    pub field_angle: String,
    pub field_angle_10: String,
    pub cie_class: String,
    pub symmetry: String,
    pub max_intensity: String,
    pub spacing_criterion: String,
    pub photometric_code: String,
    pub cutoff_angle: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LuminaireElectrical {
    pub power: String,
    pub voltage: String,
    pub current: String,
    pub power_factor: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LampSetLocale {
    pub title: String,
    pub set_n: String,
    pub num_lamps: String,
    pub luminous_flux: String,
    pub wattage: String,
    pub lamp_type: String,
    pub color_appearance: String,
    pub color_rendering: String,
    pub remove: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryLocale {
    pub title: String,
    pub description: String,
    pub beam_characteristics: String,
    pub zonal_lumens: String,
    pub glare_assessment: String,
    pub luminaire_luminance: String,
    pub room_config: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectRatiosLocale {
    pub description: String,
    pub calculate: String,
}

/// Validation message translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationLocale {
    pub level: ValidationLevel,
    pub messages: ValidationMessageLocale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationLevel {
    pub error: String,
    pub warning: String,
    pub info: String,
}

/// Validation message translations keyed by code (W001–W046, E001–E006).
/// Messages may contain `{0}`, `{1}`, … placeholders for dynamic values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationMessageLocale {
    pub w001: String,
    pub w002: String,
    pub w003: String,
    pub w004: String,
    pub w005: String,
    pub w006: String,
    pub w007: String,
    pub w008: String,
    pub w009: String,
    pub w010: String,
    pub w011: String,
    pub w012: String,
    pub w013: String,
    pub w014: String,
    pub w015: String,
    pub w016: String,
    pub w017: String,
    pub w018: String,
    pub w019: String,
    pub w020: String,
    pub w021: String,
    pub w022: String,
    pub w023: String,
    pub w024: String,
    pub w025: String,
    pub w026: String,
    pub w027: String,
    pub w028: String,
    pub w029: String,
    pub w030: String,
    pub w031: String,
    pub w032: String,
    pub w033: String,
    pub w034: String,
    pub w035: String,
    pub w036: String,
    pub w037: String,
    pub w038: String,
    pub w039: String,
    pub w040: String,
    pub w041: String,
    pub w042: String,
    pub w043: String,
    pub w044: String,
    pub w045: String,
    pub w046: String,
    pub w047: String,
    pub e001: String,
    pub e002: String,
    pub e003: String,
    pub e004: String,
    pub e005: String,
    pub e006: String,
}

/// UI translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLocale {
    pub header: UiHeader,
    pub tabs: UiTabs,
    pub subtabs: UiSubtabs,
    pub dropzone: UiDropzone,
    pub diagram: UiDiagram,
    pub intensity: UiIntensity,
    pub validation: UiValidation,
    pub spectral: UiSpectral,
    pub butterfly: UiButterfly,
    pub bevy_scene: UiBevyScene,
    pub bug_rating: UiBugRating,
    pub lcs: UiLcs,
    pub floodlight: UiFloodlight,
    pub data_table: UiDataTable,
    pub validation_panel: UiValidationPanel,
    pub spectral_badges: UiSpectralBadges,
    pub actions: UiActions,
    pub file: UiFile,
    pub theme: UiTheme,
    pub language: UiLanguage,
    pub template: UiTemplate,
    pub messages: UiMessages,
    pub compare: UiCompare,
    #[serde(default)]
    pub about: UiAbout,
    #[serde(default)]
    pub bim: UiBim,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiHeader {
    pub title: String,
    pub file: String,
    pub new: String,
    pub open: String,
    pub templates: String,
    pub save_ldt: String,
    pub export_ies: String,
    pub atla_xml: String,
    pub atla_json: String,
    pub switch_to_dark: String,
    pub switch_to_light: String,
    #[serde(default)]
    pub skip_link: String,
    #[serde(default)]
    pub rotate_c_planes: String,
    #[serde(default)]
    pub rotate_c_planes_tooltip: String,
    #[serde(default)]
    pub export_svg_prefix: String,
    #[serde(default)]
    pub export_png: String,
    #[serde(default)]
    pub export_jpeg: String,
    #[serde(default)]
    pub export_pdf: String,
    #[serde(default)]
    pub export_pdf_tooltip: String,
    #[serde(default)]
    pub export_typ: String,
    #[serde(default)]
    pub export_typ_tooltip: String,
    #[serde(default)]
    pub generating_pdf: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub switch_to_imperial: String,
    #[serde(default)]
    pub switch_to_metric: String,
    #[serde(default)]
    pub unit_si: String,
    #[serde(default)]
    pub unit_imp: String,
    #[serde(default)]
    pub loading: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTabs {
    // Sub-tabs
    pub general: String,
    pub dimensions: String,
    pub lamp_sets: String,
    pub direct_ratios: String,
    pub intensity: String,
    pub diagram_2d: String,
    pub diagram_3d: String,
    pub heatmap: String,
    pub spectral: String,
    pub greenhouse: String,
    pub bug_rating: String,
    pub lcs: String,
    pub validation: String,
    pub scene_3d: String,
    pub floodlight_vh: String,
    pub floodlight_isolux: String,
    pub floodlight_isocandela: String,
    // Main tab groups
    pub info: String,
    pub data: String,
    pub diagrams: String,
    pub analysis: String,
    pub floodlight: String,
    // Other
    pub polar: String,
    pub cartesian: String,
    pub cone: String,
    pub export: String,
    pub compare: String,
    #[serde(default)]
    pub bim: String,
    #[serde(default)]
    pub area_designer: String,
    #[serde(default)]
    pub zonal_designer: String,
    #[serde(default)]
    pub maps_designer: String,
    #[serde(default)]
    pub beam_angle: String,
    #[serde(default)]
    pub iso_view: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSubtabs {
    pub spd: String,
    pub tm30_cvg: String,
    pub tm30_hue: String,
    pub metrics: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiDropzone {
    pub text: String,
    pub current_file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiDiagram {
    pub title_2d: String,
    pub title_3d: String,
    pub title_heatmap: String,
    pub title_spectral: String,
    pub title_greenhouse: String,
    pub title_bug: String,
    pub title_lcs: String,
    pub title_scene: String,
    pub title_floodlight_vh: String,
    pub title_isolux: String,
    pub title_isocandela: String,
    pub polar: String,
    pub cartesian: String,
    pub zoom_hint: String,
    pub rotate_hint: String,
    pub scene_controls: String,
    #[serde(default)]
    pub beam_angle: String,
    #[serde(default)]
    pub beam_angle_tooltip: String,
    #[serde(default)]
    pub title_iso_view: String,
    #[serde(default)]
    pub desc_iso_view: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiIntensity {
    pub title: String,
    pub table_info: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiValidation {
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSpectral {
    pub subtitle: String,
    pub greenhouse_subtitle: String,
    pub bug_subtitle: String,
    pub lcs_subtitle: String,
    pub direct_spd: String,
    pub sample: String,
    pub load_hint: String,
    pub wavelength_range: String,
    pub peak: String,
    pub energy_distribution: String,
    pub par_distribution: String,
    pub par_total: String,
    pub hort_metrics: String,
    pub far_red: String,
    pub r_fr_hint: String,
    pub warnings: String,
    pub thermal_warning: String,
    pub uv_warning: String,
    pub cvg_legend1: String,
    pub cvg_legend2: String,
    pub hue_table: UiHueTable,
    pub hue_legend: UiHueLegend,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiHueTable {
    pub hue: String,
    pub rf: String,
    pub rcs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiHueLegend {
    pub rf: String,
    pub rcs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiButterfly {
    pub pause: String,
    pub auto: String,
    pub reset: String,
    pub drag_hint: String,
    pub max: String,
    #[serde(default)]
    pub c_plane_selector: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiBevyScene {
    pub title: String,
    pub click_to_load: String,
    pub load_button: String,
    pub loading: String,
    pub downloading: String,
    pub load_failed: String,
    pub try_again: String,
    // Viewer controls
    pub scene_type: String,
    pub scene_room: String,
    pub scene_road: String,
    pub scene_parking: String,
    pub scene_outdoor: String,
    pub room_width: String,
    pub room_length: String,
    pub room_height: String,
    pub mounting_height: String,
    pub pendulum_length: String,
    pub show_luminaire: String,
    pub show_solid: String,
    pub show_shadows: String,
    pub controls_hint: String,
    #[serde(default)]
    pub scene_designer_exterior: String,
    #[serde(default)]
    pub scene_designer_interior: String,
    #[serde(default)]
    pub hide: String,
    #[serde(default)]
    pub road_layout: String,
    #[serde(default)]
    pub lanes: String,
    #[serde(default)]
    pub lane_width: String,
    #[serde(default)]
    pub sidewalk_width: String,
    #[serde(default)]
    pub road_length: String,
    #[serde(default)]
    pub luminaire_settings: String,
    #[serde(default)]
    pub tilt_angle: String,
    #[serde(default)]
    pub pole_spacing: String,
    #[serde(default)]
    pub show_light_cones: String,
    #[serde(default)]
    pub show_cavity_zones: String,
    #[serde(default)]
    pub exterior_hint: String,
    #[serde(default)]
    pub interior_hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiBugRating {
    pub title: String,
    pub title_detailed: String,
    pub show_details: String,
    pub hide_details: String,
    pub footer_basic: String,
    pub footer_detailed: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLcs {
    pub footer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiFloodlight {
    pub vh_subtitle: String,
    pub isolux_subtitle: String,
    pub isocandela_subtitle: String,
    pub h_plane: String,
    pub v_plane: String,
    pub log_scale: String,
    pub linear_scale: String,
    pub mounting_height: String,
    pub tilt_angle: String,
    pub area_size: String,
    pub nema_classification: String,
    pub show_contours: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiDataTable {
    pub no_data: String,
    pub copy_to_clipboard: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiValidationPanel {
    pub all_passed: String,
    pub error_count: String,
    #[serde(default)]
    pub schema_validation: String,
    #[serde(default)]
    pub ldt_validation: String,
    #[serde(default)]
    pub valid: String,
    #[serde(default)]
    pub invalid: String,
    #[serde(default)]
    pub missing_fields: String,
    #[serde(default)]
    pub warnings: String,
    #[serde(default)]
    pub see_tm33: String,
    #[serde(default)]
    pub tm33_issues: String,
    #[serde(default)]
    pub tm32_issues: String,
    #[serde(default)]
    pub n_errors: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSpectralBadges {
    pub ir: String,
    pub uv: String,
    pub ir_high_title: String,
    pub ir_title: String,
    pub uv_high_title: String,
    pub uv_title: String,
    pub rg: String,
    pub duv: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiActions {
    pub load: String,
    pub save: String,
    pub export: String,
    pub import: String,
    pub clear: String,
    pub reset: String,
    pub apply: String,
    pub cancel: String,
    pub close: String,
    pub download: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiFile {
    pub open: String,
    pub save_as: String,
    pub export_ldt: String,
    pub export_ies: String,
    pub export_atla: String,
    pub export_svg: String,
    pub export_pdf: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTheme {
    pub light: String,
    pub dark: String,
    pub system: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLanguage {
    pub select: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTemplate {
    pub select: String,
    pub downlight: String,
    pub projector: String,
    pub linear: String,
    pub fluorescent: String,
    pub road: String,
    pub uplight: String,
    pub atla_fluorescent_xml: String,
    pub atla_fluorescent_json: String,
    pub atla_grow_light_fs: String,
    pub atla_grow_light_rb: String,
    pub halogen: String,
    pub incandescent: String,
    pub heat_lamp: String,
    pub uv_blacklight: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiMessages {
    pub loading: String,
    pub saving: String,
    pub error: String,
    pub success: String,
    pub file_loaded: String,
    pub file_saved: String,
    pub invalid_file: String,
    pub no_file: String,
}

/// Compare panel translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiCompare {
    pub title: String,
    pub drop_hint: String,
    pub browse: String,
    pub or: String,
    pub select_template: String,
    pub file_b: String,
    pub file_b_label: String,
    pub clear: String,
    pub similarity: String,
    pub export_pdf: String,
    pub export_typ: String,
    pub exporting: String,
    pub file_a_c_plane: String,
    pub file_b_c_plane: String,
    pub link_sliders: String,
    pub metric: String,
    pub file_a: String,
    pub delta: String,
    pub percent: String,
    pub empty_title: String,
    pub empty_hint: String,
}

/// Comparison locale (metric names for photometric comparison)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ComparisonLocale {
    pub metrics: ComparisonMetricLocale,
}

/// Comparison metric name translations keyed by metric key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ComparisonMetricLocale {
    pub total_lamp_flux: String,
    pub calculated_flux: String,
    pub lor: String,
    pub dlor: String,
    pub ulor: String,
    pub lamp_efficacy: String,
    pub luminaire_efficacy: String,
    pub total_wattage: String,
    pub beam_angle: String,
    pub field_angle: String,
    pub beam_angle_cie: String,
    pub field_angle_cie: String,
    pub upward_beam_angle: String,
    pub upward_field_angle: String,
    pub max_intensity: String,
    pub min_intensity: String,
    pub avg_intensity: String,
    pub spacing_c0: String,
    pub spacing_c90: String,
    pub zonal_0_30: String,
    pub zonal_30_60: String,
    pub zonal_60_90: String,
    pub zonal_90_120: String,
    pub zonal_120_150: String,
    pub zonal_150_180: String,
    pub cie_n1: String,
    pub cie_n2: String,
    pub cie_n3: String,
    pub cie_n4: String,
    pub cie_n5: String,
    pub bug_b: String,
    pub bug_u: String,
    pub bug_g: String,
    pub length: String,
    pub width: String,
    pub height: String,
}

/// About modal translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UiAbout {
    pub title: String,
    pub subtitle: String,
    pub desc_parsing: String,
    pub desc_diagrams: String,
    pub desc_platforms: String,
    pub close: String,
}

/// BIM panel translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UiBim {
    pub title: String,
    pub tooltip: String,
    pub export_csv: String,
    pub export_report: String,
    pub info_text: String,
    pub no_data_title: String,
    pub no_data_text: String,
    pub template_hint: String,
}

/// Shared designer labels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DesignerLocale {
    pub height: String,
    pub width: String,
    pub length: String,
    pub rotation: String,
    pub tilt: String,
    pub resolution: String,
    pub mounting_height: String,
    pub export_csv: String,
    pub export_pdf: String,
    pub export_svg: String,
    pub exporting_pdf: String,
    pub share_link: String,
    pub copied: String,
    pub save: String,
    pub load: String,
    pub undo: String,
    pub redo: String,
    pub remove: String,
    pub plan_view: String,
    pub room_view: String,
    pub min: String,
    pub avg: String,
    pub max: String,
    pub luminaire: String,
    pub luminaires: String,
}

/// Area lighting designer translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerLocale {
    pub title: String,
    pub layout: AreaDesignerLayout,
    pub arrangement: AreaDesignerArrangement,
    pub params: AreaDesignerParams,
    pub polygon: AreaDesignerPolygon,
    pub pole: AreaDesignerPole,
    pub presets: AreaDesignerPresets,
    pub mixed: AreaDesignerMixed,
    pub optimizer: AreaDesignerOptimizer,
    pub height_compare: AreaDesignerHeightCompare,
    pub combined: String,
    pub wall_mounted_hint: String,
    pub stats: AreaDesignerStats,
    pub export_iso_svg: String,
    pub export_plan_svg: String,
    pub export_room_svg: String,
    pub export_3d_svg: String,
    pub export_optimizer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerLayout {
    pub label: String,
    pub single: String,
    pub pair: String,
    pub row_of_3: String,
    pub grid_2x2: String,
    pub grid_2x3: String,
    pub grid_3x3: String,
    pub perimeter: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerArrangement {
    pub label: String,
    pub single: String,
    pub back_to_back: String,
    pub twin_arm: String,
    pub quad: String,
    pub wall_mounted: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerParams {
    pub arm: String,
    pub droop: String,
    pub area_w: String,
    pub area_d: String,
    pub proration: String,
    pub poles: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerPolygon {
    pub cancel: String,
    pub clear: String,
    pub draw: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerPole {
    pub selected: String,
    pub pole_n: String,
    pub x: String,
    pub y: String,
    pub reset_overrides: String,
    pub click_to_select: String,
    pub ldt: String,
    pub primary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerPresets {
    pub title: String,
    pub top_left: String,
    pub top: String,
    pub top_right: String,
    pub left: String,
    pub center: String,
    pub right: String,
    pub bottom_left: String,
    pub bottom: String,
    pub bottom_right: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerMixed {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerOptimizer {
    pub title: String,
    pub target: String,
    pub uniformity: String,
    pub heights: String,
    pub step: String,
    pub running: String,
    pub optimize: String,
    pub ht: String,
    pub spc: String,
    pub poles_col: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerHeightCompare {
    pub title: String,
    pub run_hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AreaDesignerStats {
    pub u0: String,
    pub ud: String,
    pub avg_min: String,
}

/// Zonal cavity designer translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerLocale {
    pub title: String,
    pub room: String,
    pub workplane: String,
    pub suspension: String,
    pub mode: ZonalDesignerMode,
    pub target: String,
    pub count: String,
    pub lpd: String,
    pub reflectances: ZonalDesignerReflectances,
    pub llf: ZonalDesignerLlf,
    pub views: ZonalDesignerViews,
    pub camera: String,
    pub info: ZonalDesignerInfo,
    pub results: ZonalDesignerResults,
    pub heatmap_title: String,
    pub computing_heatmap: String,
    pub cavity_section: String,
    pub cu_table_title: String,
    pub export_pdf: String,
    pub room_view_3d: String,
    pub room_view_hint: String,
    pub export_3d_svg: String,
    pub cavity_lines: String,
    pub light_cones: String,
    pub options_3d: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerMode {
    pub label: String,
    pub target_to_count: String,
    pub count_to_illuminance: String,
    pub target_lpd: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerReflectances {
    pub title: String,
    pub ceiling: String,
    pub wall: String,
    pub floor: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerLlf {
    pub title: String,
    pub preset: String,
    pub lld: String,
    pub ldd: String,
    pub bf: String,
    pub total: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerViews {
    pub heatmap: String,
    pub room_3d: String,
    pub section: String,
    pub cu_table: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerInfo {
    pub name: String,
    pub lumens: String,
    pub power: String,
    pub dff: String,
    pub smh: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ZonalDesignerResults {
    pub luminaires: String,
    pub achieved: String,
    pub cu: String,
    pub rcr: String,
    pub lpd: String,
    pub spacing: String,
    pub ok: String,
    pub exceeds: String,
}

/// Dashboard translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardLocale {
    pub templates: DashboardTemplates,
    pub modes: DashboardModes,
    pub columns: DashboardColumns,
    pub optics: DashboardOptics,
    pub loaded_file: String,
    pub loading_templates: String,
    pub luminaires_count: String,
    pub click_to_select: String,
    pub back: String,
    pub edit_data: String,
    pub zoom_diagram: String,
    pub open_compare: String,
    pub area_designer: String,
    pub export_pdf: String,
    pub polar_title: String,
    pub cartesian_title: String,
    pub iso_lux_title: String,
    pub iso_view_title: String,
    pub bug_title: String,
    pub iso_curve_title: String,
    pub beam_intensities_title: String,
    pub beam_angle_title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardTemplates {
    pub standard: String,
    pub aec: String,
    pub alternative: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardModes {
    pub overview: String,
    pub beam_angles: String,
    pub beam_intensities: String,
    pub ugr_table: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardColumns {
    pub name: String,
    pub cct: String,
    pub power: String,
    pub lumens: String,
    pub lor: String,
    pub bug: String,
    pub cri: String,
    pub beam: String,
    pub light_dist: String,
    pub optics: String,
    pub modules: String,
    pub current: String,
    pub spec: String,
    pub ies: String,
    pub cad: String,
    pub bim: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardOptics {
    pub narrow: String,
    pub medium: String,
    pub wide: String,
    pub very_wide: String,
}

/// Maps designer translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MapsDesignerLocale {
    pub title: String,
    pub instructions_title: String,
    pub instruction_polygon: String,
    pub instruction_luminaires: String,
    pub instruction_calculate: String,
    pub area_defined: String,
    pub luminaires_count: String,
    pub calculate: String,
    pub my_location: String,
    pub export_csv: String,
    pub show_values: String,
    pub hide_values: String,
    pub clear_all: String,
    pub results_title: String,
    pub min: String,
    pub max: String,
    pub average: String,
    pub uniformity: String,
    pub good_uniformity: String,
    pub acceptable_uniformity: String,
    pub poor_uniformity: String,
    pub legend_title: String,
    pub legend_low: String,
    pub legend_high: String,
    pub loading_description: String,
    pub load_maps: String,
    pub loading_maps: String,
    pub toolbar_hint: String,
    pub load_failed: String,
    pub try_again: String,
}

/// Report translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportLocale {
    pub title: String,
    pub generated: String,
    pub page: String,
    pub of: String,
    pub summary: String,
    pub details: String,
    pub appendix: String,
}

/// Translations for the virtual goniophotometer / simulation UI.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GoniosimLocale {
    #[serde(default = "default_goniosim_title")]
    pub title: String,
    #[serde(default = "default_goniosim_subtitle")]
    pub subtitle: String,
    #[serde(default = "default_goniosim_trace")]
    pub trace: String,
    #[serde(default = "default_goniosim_pause")]
    pub pause: String,
    #[serde(default = "default_goniosim_resume")]
    pub resume: String,
    #[serde(default = "default_goniosim_reset")]
    pub reset: String,
    #[serde(default = "default_goniosim_input")]
    pub input_luminaire: String,
    #[serde(default = "default_goniosim_upload")]
    pub upload: String,
    #[serde(default = "default_goniosim_cover")]
    pub cover_material: String,
    #[serde(default = "default_goniosim_no_cover")]
    pub no_cover: String,
    #[serde(default = "default_goniosim_custom")]
    pub custom: String,
    #[serde(default = "default_goniosim_reflectance")]
    pub reflectance: String,
    #[serde(default = "default_goniosim_diffusion")]
    pub diffusion: String,
    #[serde(default = "default_goniosim_transmittance")]
    pub transmittance: String,
    #[serde(default = "default_goniosim_ior")]
    pub ior: String,
    #[serde(default = "default_goniosim_thickness")]
    pub thickness: String,
    #[serde(default = "default_goniosim_distance")]
    pub distance: String,
    #[serde(default = "default_goniosim_statistics")]
    pub statistics: String,
    #[serde(default = "default_goniosim_photons")]
    pub photons: String,
    #[serde(default = "default_goniosim_detected")]
    pub detected: String,
    #[serde(default = "default_goniosim_absorbed")]
    pub absorbed: String,
    #[serde(default = "default_goniosim_export")]
    pub export_ldt: String,
    #[serde(default = "default_goniosim_original")]
    pub original_lvk: String,
    #[serde(default = "default_goniosim_simulated")]
    pub simulated: String,
    #[serde(default = "default_goniosim_select")]
    pub select_luminaire: String,
    #[serde(default = "default_goniosim_click_trace")]
    pub click_trace: String,
    #[serde(default = "default_goniosim_c_plane")]
    pub c_plane: String,
    #[serde(default = "default_goniosim_all")]
    pub all_planes: String,
    // Material names
    #[serde(default = "default_mat_clear_pmma")]
    pub mat_clear_pmma: String,
    #[serde(default = "default_mat_satin_pmma")]
    pub mat_satin_pmma: String,
    #[serde(default = "default_mat_opal_light_pmma")]
    pub mat_opal_light_pmma: String,
    #[serde(default = "default_mat_opal_pmma")]
    pub mat_opal_pmma: String,
    #[serde(default = "default_mat_clear_glass")]
    pub mat_clear_glass: String,
    #[serde(default = "default_mat_satin_glass")]
    pub mat_satin_glass: String,
}

fn default_goniosim_title() -> String { "Virtual Goniophotometer".into() }
fn default_goniosim_subtitle() -> String { "CIE 171:2006 validated".into() }
fn default_goniosim_trace() -> String { "Trace".into() }
fn default_goniosim_pause() -> String { "Pause".into() }
fn default_goniosim_resume() -> String { "Resume".into() }
fn default_goniosim_reset() -> String { "Reset".into() }
fn default_goniosim_input() -> String { "Input Luminaire".into() }
fn default_goniosim_upload() -> String { "Upload .ldt / .ies".into() }
fn default_goniosim_cover() -> String { "Cover Material".into() }
fn default_goniosim_no_cover() -> String { "No cover (free space)".into() }
fn default_goniosim_custom() -> String { "Custom".into() }
fn default_goniosim_reflectance() -> String { "Reflectance".into() }
fn default_goniosim_diffusion() -> String { "Diffusion".into() }
fn default_goniosim_transmittance() -> String { "Transmittance".into() }
fn default_goniosim_ior() -> String { "Refractive index".into() }
fn default_goniosim_thickness() -> String { "Thickness".into() }
fn default_goniosim_distance() -> String { "Distance".into() }
fn default_goniosim_statistics() -> String { "Statistics".into() }
fn default_goniosim_photons() -> String { "Photons".into() }
fn default_goniosim_detected() -> String { "Detected".into() }
fn default_goniosim_absorbed() -> String { "Absorbed".into() }
fn default_goniosim_export() -> String { "Export .ldt".into() }
fn default_goniosim_original() -> String { "Original LVK".into() }
fn default_goniosim_simulated() -> String { "Simulated (through cover)".into() }
fn default_goniosim_select() -> String { "Select or upload a luminaire".into() }
fn default_goniosim_click_trace() -> String { "Click Trace to simulate".into() }
fn default_goniosim_c_plane() -> String { "C-plane".into() }
fn default_goniosim_all() -> String { "All".into() }
fn default_mat_clear_pmma() -> String { "PMMA clear 3mm".into() }
fn default_mat_satin_pmma() -> String { "PMMA satin 3mm".into() }
fn default_mat_opal_light_pmma() -> String { "PMMA opal light 3mm".into() }
fn default_mat_opal_pmma() -> String { "PMMA opal 3mm".into() }
fn default_mat_clear_glass() -> String { "Glass clear 4mm".into() }
fn default_mat_satin_glass() -> String { "Glass satin 4mm".into() }

// Embedded locale JSON files
const EN_JSON: &str = include_str!("../locales/en.json");
const DE_JSON: &str = include_str!("../locales/de.json");
const ZH_JSON: &str = include_str!("../locales/zh.json");
const FR_JSON: &str = include_str!("../locales/fr.json");
const IT_JSON: &str = include_str!("../locales/it.json");
const RU_JSON: &str = include_str!("../locales/ru.json");
const ES_JSON: &str = include_str!("../locales/es.json");
const PT_BR_JSON: &str = include_str!("../locales/pt-BR.json");

impl Locale {
    /// Parse locale from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get English locale (default)
    pub fn english() -> Self {
        Self::from_json(EN_JSON).expect("embedded English locale is valid")
    }

    /// Get German locale
    pub fn german() -> Self {
        Self::from_json(DE_JSON).expect("embedded German locale is valid")
    }

    /// Get Chinese locale
    pub fn chinese() -> Self {
        Self::from_json(ZH_JSON).expect("embedded Chinese locale is valid")
    }

    /// Get French locale
    pub fn french() -> Self {
        Self::from_json(FR_JSON).expect("embedded French locale is valid")
    }

    /// Get Italian locale
    pub fn italian() -> Self {
        Self::from_json(IT_JSON).expect("embedded Italian locale is valid")
    }

    /// Get Russian locale
    pub fn russian() -> Self {
        Self::from_json(RU_JSON).expect("embedded Russian locale is valid")
    }

    /// Get Spanish locale
    pub fn spanish() -> Self {
        Self::from_json(ES_JSON).expect("embedded Spanish locale is valid")
    }

    /// Get Portuguese (Brazil) locale
    pub fn portuguese_brazil() -> Self {
        Self::from_json(PT_BR_JSON).expect("embedded Portuguese locale is valid")
    }

    /// Get locale for a specific language
    pub fn for_language(lang: Language) -> Self {
        match lang {
            Language::English => Self::english(),
            Language::German => Self::german(),
            Language::Chinese => Self::chinese(),
            Language::French => Self::french(),
            Language::Italian => Self::italian(),
            Language::Russian => Self::russian(),
            Language::Spanish => Self::spanish(),
            Language::PortugueseBrazil => Self::portuguese_brazil(),
        }
    }

    /// Get locale by ISO 639-1 code
    pub fn for_code(code: &str) -> Self {
        Self::for_language(Language::from_code(code))
    }

    /// Look up a validation message template by code (e.g. "W001", "E001").
    /// Returns the template string which may contain `{0}`, `{1}`, … placeholders.
    pub fn validation_message(&self, code: &str) -> Option<&str> {
        let m = &self.validation.messages;
        let s = match code {
            "W001" => &m.w001,
            "W002" => &m.w002,
            "W003" => &m.w003,
            "W004" => &m.w004,
            "W005" => &m.w005,
            "W006" => &m.w006,
            "W007" => &m.w007,
            "W008" => &m.w008,
            "W009" => &m.w009,
            "W010" => &m.w010,
            "W011" => &m.w011,
            "W012" => &m.w012,
            "W013" => &m.w013,
            "W014" => &m.w014,
            "W015" => &m.w015,
            "W016" => &m.w016,
            "W017" => &m.w017,
            "W018" => &m.w018,
            "W019" => &m.w019,
            "W020" => &m.w020,
            "W021" => &m.w021,
            "W022" => &m.w022,
            "W023" => &m.w023,
            "W024" => &m.w024,
            "W025" => &m.w025,
            "W026" => &m.w026,
            "W027" => &m.w027,
            "W028" => &m.w028,
            "W029" => &m.w029,
            "W030" => &m.w030,
            "W031" => &m.w031,
            "W032" => &m.w032,
            "W033" => &m.w033,
            "W034" => &m.w034,
            "W035" => &m.w035,
            "W036" => &m.w036,
            "W037" => &m.w037,
            "W038" => &m.w038,
            "W039" => &m.w039,
            "W040" => &m.w040,
            "W041" => &m.w041,
            "W042" => &m.w042,
            "W043" => &m.w043,
            "W044" => &m.w044,
            "W045" => &m.w045,
            "W046" => &m.w046,
            "W047" => &m.w047,
            "E001" => &m.e001,
            "E002" => &m.e002,
            "E003" => &m.e003,
            "E004" => &m.e004,
            "E005" => &m.e005,
            "E006" => &m.e006,
            _ => return None,
        };
        Some(s.as_str())
    }

    /// Look up a comparison metric name by key (e.g. "total_lamp_flux").
    pub fn comparison_metric_name(&self, key: &str) -> Option<&str> {
        let m = &self.comparison.metrics;
        let s = match key {
            "total_lamp_flux" => &m.total_lamp_flux,
            "calculated_flux" => &m.calculated_flux,
            "lor" => &m.lor,
            "dlor" => &m.dlor,
            "ulor" => &m.ulor,
            "lamp_efficacy" => &m.lamp_efficacy,
            "luminaire_efficacy" => &m.luminaire_efficacy,
            "total_wattage" => &m.total_wattage,
            "beam_angle" => &m.beam_angle,
            "field_angle" => &m.field_angle,
            "beam_angle_cie" => &m.beam_angle_cie,
            "field_angle_cie" => &m.field_angle_cie,
            "upward_beam_angle" => &m.upward_beam_angle,
            "upward_field_angle" => &m.upward_field_angle,
            "max_intensity" => &m.max_intensity,
            "min_intensity" => &m.min_intensity,
            "avg_intensity" => &m.avg_intensity,
            "spacing_c0" => &m.spacing_c0,
            "spacing_c90" => &m.spacing_c90,
            "zonal_0_30" => &m.zonal_0_30,
            "zonal_30_60" => &m.zonal_30_60,
            "zonal_60_90" => &m.zonal_60_90,
            "zonal_90_120" => &m.zonal_90_120,
            "zonal_120_150" => &m.zonal_120_150,
            "zonal_150_180" => &m.zonal_150_180,
            "cie_n1" => &m.cie_n1,
            "cie_n2" => &m.cie_n2,
            "cie_n3" => &m.cie_n3,
            "cie_n4" => &m.cie_n4,
            "cie_n5" => &m.cie_n5,
            "bug_b" => &m.bug_b,
            "bug_u" => &m.bug_u,
            "bug_g" => &m.bug_g,
            "length" => &m.length,
            "width" => &m.width,
            "height" => &m.height,
            _ => return None,
        };
        Some(s.as_str())
    }
}

/// Replace `{0}`, `{1}`, … placeholders in a template string with provided args.
pub fn format_template(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut result = template.to_string();
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), &arg.to_string());
    }
    result
}

impl Default for Locale {
    fn default() -> Self {
        Self::english()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_locale() {
        let en = Locale::english();
        assert_eq!(en.meta.code, "en");
        assert_eq!(en.diagram.axis.gamma, "Gamma (γ)");
        assert_eq!(en.ui.tabs.polar, "Polar");
    }

    #[test]
    fn test_german_locale() {
        let de = Locale::german();
        assert_eq!(de.meta.code, "de");
        assert_eq!(de.diagram.axis.intensity, "Lichtstärke (cd/klm)");
        assert_eq!(de.ui.tabs.polar, "Polar");
    }

    #[test]
    fn test_chinese_locale() {
        let zh = Locale::chinese();
        assert_eq!(zh.meta.code, "zh");
        assert_eq!(zh.diagram.placeholder.no_data, "无数据");
        assert_eq!(zh.ui.tabs.polar, "极坐标");
    }

    #[test]
    fn test_russian_locale() {
        let ru = Locale::russian();
        assert_eq!(ru.meta.code, "ru");
        assert_eq!(ru.diagram.placeholder.no_data, "Нет данных");
        assert_eq!(ru.luminaire.photometric.cct, "Цветовая температура");
    }

    #[test]
    fn test_spanish_locale() {
        let es = Locale::spanish();
        assert_eq!(es.meta.code, "es");
        assert_eq!(es.diagram.placeholder.no_data, "Sin datos");
        assert_eq!(es.ui.actions.save, "Guardar");
    }

    #[test]
    fn test_portuguese_brazil_locale() {
        let pt = Locale::portuguese_brazil();
        assert_eq!(pt.meta.code, "pt-BR");
        assert_eq!(pt.diagram.placeholder.no_data, "Sem dados");
        assert_eq!(pt.ui.actions.save, "Salvar");
    }

    #[test]
    fn test_french_locale() {
        let fr = Locale::french();
        assert_eq!(fr.meta.code, "fr");
        assert_eq!(fr.diagram.placeholder.no_data, "Aucune donnée");
        assert_eq!(fr.ui.actions.save, "Enregistrer");
    }

    #[test]
    fn test_italian_locale() {
        let it = Locale::italian();
        assert_eq!(it.meta.code, "it");
        assert_eq!(it.diagram.placeholder.no_data, "Nessun dato");
        assert_eq!(it.ui.actions.save, "Salva");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("de"), Language::German);
        assert_eq!(Language::from_code("zh-CN"), Language::Chinese);
        assert_eq!(Language::from_code("fr"), Language::French);
        assert_eq!(Language::from_code("it"), Language::Italian);
        assert_eq!(Language::from_code("ru"), Language::Russian);
        assert_eq!(Language::from_code("es"), Language::Spanish);
        assert_eq!(Language::from_code("pt-BR"), Language::PortugueseBrazil);
        assert_eq!(Language::from_code("unknown"), Language::English);
    }

    #[test]
    fn test_for_code() {
        let locale = Locale::for_code("de");
        assert_eq!(locale.meta.code, "de");

        let locale_ru = Locale::for_code("ru");
        assert_eq!(locale_ru.meta.code, "ru");
    }

    #[test]
    fn test_all_languages() {
        let all = Language::all();
        assert_eq!(all.len(), 8);

        // Verify all languages can load their locale
        for lang in all {
            let locale = Locale::for_language(*lang);
            assert_eq!(locale.meta.code, lang.code());
        }
    }
}
