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
    #[serde(default)]
    pub spectrum_lab: SpectrumLabLocale,
    #[serde(default)]
    pub street: StreetLocale,
    /// Flat key→string map for ad-hoc translations from sibling crates
    /// (gldf-rs, light-other-rs, etc.). Keys should be namespaced
    /// (`gldf.nav.overview`, `light_other.foo.bar`, …) so different
    /// consumers don't collide.
    #[serde(default)]
    pub extra: std::collections::HashMap<String, String>,
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
    #[serde(default)]
    pub light_scene: UiLightScene,
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

fn default_scene_rt() -> String {
    "Light scene".to_string()
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
    #[serde(default = "default_scene_rt")]
    pub scene_rt: String,
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
    #[serde(default = "default_street_design_tab")]
    pub street_design: String,
}

fn default_street_design_tab() -> String {
    "🛣️ Street Design".into()
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
    #[serde(default)]
    pub guide: DashboardGuide,
    #[serde(default)]
    pub features: DashboardFeatures,
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

fn default_goniosim_title() -> String {
    "Virtual Goniophotometer".into()
}
fn default_goniosim_subtitle() -> String {
    "CIE 171:2006 validated".into()
}
fn default_goniosim_trace() -> String {
    "Trace".into()
}
fn default_goniosim_pause() -> String {
    "Pause".into()
}
fn default_goniosim_resume() -> String {
    "Resume".into()
}
fn default_goniosim_reset() -> String {
    "Reset".into()
}
fn default_goniosim_input() -> String {
    "Input Luminaire".into()
}
fn default_goniosim_upload() -> String {
    "Upload .ldt / .ies".into()
}
fn default_goniosim_cover() -> String {
    "Cover Material".into()
}
fn default_goniosim_no_cover() -> String {
    "No cover (free space)".into()
}
fn default_goniosim_custom() -> String {
    "Custom".into()
}
fn default_goniosim_reflectance() -> String {
    "Reflectance".into()
}
fn default_goniosim_diffusion() -> String {
    "Diffusion".into()
}
fn default_goniosim_transmittance() -> String {
    "Transmittance".into()
}
fn default_goniosim_ior() -> String {
    "Refractive index".into()
}
fn default_goniosim_thickness() -> String {
    "Thickness".into()
}
fn default_goniosim_distance() -> String {
    "Distance".into()
}
fn default_goniosim_statistics() -> String {
    "Statistics".into()
}
fn default_goniosim_photons() -> String {
    "Photons".into()
}
fn default_goniosim_detected() -> String {
    "Detected".into()
}
fn default_goniosim_absorbed() -> String {
    "Absorbed".into()
}
fn default_goniosim_export() -> String {
    "Export .ldt".into()
}
fn default_goniosim_original() -> String {
    "Original LVK".into()
}
fn default_goniosim_simulated() -> String {
    "Simulated (through cover)".into()
}
fn default_goniosim_select() -> String {
    "Select or upload a luminaire".into()
}
fn default_goniosim_click_trace() -> String {
    "Click Trace to simulate".into()
}
fn default_goniosim_c_plane() -> String {
    "C-plane".into()
}
fn default_goniosim_all() -> String {
    "All".into()
}
fn default_mat_clear_pmma() -> String {
    "PMMA clear 3mm".into()
}
fn default_mat_satin_pmma() -> String {
    "PMMA satin 3mm".into()
}
fn default_mat_opal_light_pmma() -> String {
    "PMMA opal light 3mm".into()
}
fn default_mat_opal_pmma() -> String {
    "PMMA opal 3mm".into()
}
fn default_mat_clear_glass() -> String {
    "Glass clear 4mm".into()
}
fn default_mat_satin_glass() -> String {
    "Glass satin 4mm".into()
}

// ─────────────────────────────────────────────────────────────────────────
// Spectrum Lab (spectral Monte Carlo demo) translations
// ─────────────────────────────────────────────────────────────────────────

/// Translations for the Spectrum Lab demo tab (spectral trace → CCT, S/P,
/// melanopic, mesopic road luminance, dark-sky report). Every field has an
/// English `serde` default so missing keys fall back gracefully.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SpectrumLabLocale {
    #[serde(default = "default_slab_title")]
    pub title: String,
    #[serde(default = "default_slab_intro")]
    pub intro: String,
    #[serde(default = "default_slab_source_spectrum")]
    pub source_spectrum: String,
    #[serde(default = "default_slab_photons")]
    pub photons: String,
    #[serde(default = "default_slab_road_luminance")]
    pub road_luminance: String,
    #[serde(default = "default_slab_trace")]
    pub trace: String,
    #[serde(default = "default_slab_tracing")]
    pub tracing: String,
    #[serde(default = "default_slab_empty_hint")]
    pub empty_hint: String,
    #[serde(default = "default_slab_cct")]
    pub cct: String,
    #[serde(default = "default_slab_sp")]
    pub sp_ratio: String,
    #[serde(default = "default_slab_mel")]
    pub melanopic_der: String,
    #[serde(default = "default_slab_mesopic")]
    pub mesopic_road: String,
    #[serde(default = "default_slab_dark_sky")]
    pub dark_sky: String,
    #[serde(default = "default_slab_passes")]
    pub passes: String,
    #[serde(default = "default_slab_fails")]
    pub fails: String,
    #[serde(default = "default_slab_ulor")]
    pub ulor: String,
    #[serde(default = "default_slab_upward_cct")]
    pub upward_cct: String,
    #[serde(default = "default_slab_blue_content")]
    pub blue_content: String,
    #[serde(default = "default_slab_sp_cool")]
    pub sp_cool: String,
    #[serde(default = "default_slab_sp_neutral")]
    pub sp_neutral: String,
    #[serde(default = "default_slab_sp_warm")]
    pub sp_warm: String,
    #[serde(default = "default_slab_mel_high")]
    pub mel_high: String,
    #[serde(default = "default_slab_mel_moderate")]
    pub mel_moderate: String,
    #[serde(default = "default_slab_mel_low")]
    pub mel_low: String,
    /// Mesopic-card hint template. `{pct}` (signed % vs photopic) and `{photopic}`
    /// (the photopic luminance) are substituted.
    #[serde(default = "default_slab_mesopic_vs")]
    pub mesopic_vs: String,
    /// Footer stats template. `{photons}` and `{pct}` are substituted.
    #[serde(default = "default_slab_stats")]
    pub stats: String,
    // ── Location & datetime controls ──
    #[serde(default = "default_slab_location")]
    pub location: String,
    #[serde(default = "default_slab_date")]
    pub date: String,
    #[serde(default = "default_slab_time_of_day")]
    pub time_of_day: String,
    #[serde(default = "default_slab_mode")]
    pub mode: String,
    #[serde(default = "default_slab_mode_luminaire")]
    pub mode_luminaire: String,
    #[serde(default = "default_slab_mode_sky")]
    pub mode_sky: String,
    #[serde(default = "default_slab_sun_altitude")]
    pub sun_altitude: String,
    #[serde(default = "default_slab_sun_azimuth")]
    pub sun_azimuth: String,
    #[serde(default = "default_slab_regime_day")]
    pub regime_day: String,
    #[serde(default = "default_slab_regime_twilight")]
    pub regime_twilight: String,
    #[serde(default = "default_slab_regime_night")]
    pub regime_night: String,
    #[serde(default = "default_slab_sky_illuminance")]
    pub sky_illuminance: String,
    // ── Sky visualisation titles ──
    #[serde(default = "default_slab_viz_sky_dome")]
    pub viz_sky_dome: String,
    #[serde(default = "default_slab_viz_scene")]
    pub viz_scene: String,
    #[serde(default = "default_slab_viz_sun_path")]
    pub viz_sun_path: String,
    #[serde(default = "default_slab_viz_timeline")]
    pub viz_timeline: String,
    // ── Custom location ──
    #[serde(default = "default_slab_location_custom")]
    pub location_custom: String,
    #[serde(default = "default_slab_latitude")]
    pub latitude: String,
    #[serde(default = "default_slab_longitude")]
    pub longitude: String,
    // ── Moon phase ──
    #[serde(default = "default_slab_regime_moonlit")]
    pub regime_moonlit: String,
    #[serde(default = "default_slab_moon_below")]
    pub moon_below: String,
    #[serde(default = "default_slab_moon_new")]
    pub moon_new: String,
    #[serde(default = "default_slab_moon_waxing_crescent")]
    pub moon_waxing_crescent: String,
    #[serde(default = "default_slab_moon_first_quarter")]
    pub moon_first_quarter: String,
    #[serde(default = "default_slab_moon_waxing_gibbous")]
    pub moon_waxing_gibbous: String,
    #[serde(default = "default_slab_moon_full")]
    pub moon_full: String,
    #[serde(default = "default_slab_moon_waning_gibbous")]
    pub moon_waning_gibbous: String,
    #[serde(default = "default_slab_moon_last_quarter")]
    pub moon_last_quarter: String,
    #[serde(default = "default_slab_moon_waning_crescent")]
    pub moon_waning_crescent: String,
    // ── Daylight & Sky tab ──
    #[serde(default = "default_slab_sky_tab_title")]
    pub sky_tab_title: String,
    #[serde(default = "default_slab_sky_tab_intro")]
    pub sky_tab_intro: String,
    #[serde(default = "default_slab_sky_empty_hint")]
    pub sky_empty_hint: String,
    #[serde(default = "default_slab_scene_road")]
    pub scene_road: String,
    #[serde(default = "default_slab_scene_room")]
    pub scene_room: String,
    #[serde(default = "default_slab_scene_plaza")]
    pub scene_plaza: String,
}

fn default_slab_title() -> String {
    "Spectrum Lab".into()
}
fn default_slab_intro() -> String {
    "One spectral trace → colour-over-angle, night vision (S/P), circadian (melanopic), mesopic road luminance, and a dark-sky report. The photons that build the LDT give you every metric because weighting happens at the detector, not at emission.".into()
}
fn default_slab_source_spectrum() -> String {
    "Source spectrum".into()
}
fn default_slab_photons() -> String {
    "Photons".into()
}
fn default_slab_road_luminance() -> String {
    "Road luminance (cd/m²)".into()
}
fn default_slab_trace() -> String {
    "▶ Trace".into()
}
fn default_slab_tracing() -> String {
    "Tracing…".into()
}
fn default_slab_empty_hint() -> String {
    "Pick a spectrum and press Trace to run the spectral Monte Carlo engine.".into()
}
fn default_slab_cct() -> String {
    "Correlated colour temp".into()
}
fn default_slab_sp() -> String {
    "S/P ratio (night vision)".into()
}
fn default_slab_mel() -> String {
    "Melanopic DER (circadian)".into()
}
fn default_slab_mesopic() -> String {
    "Mesopic road luminance".into()
}
fn default_slab_dark_sky() -> String {
    "Dark-Sky Compliance".into()
}
fn default_slab_passes() -> String {
    "PASSES 3000K limit".into()
}
fn default_slab_fails() -> String {
    "FAILS 3000K limit".into()
}
fn default_slab_ulor() -> String {
    "Upward light (ULOR)".into()
}
fn default_slab_upward_cct() -> String {
    "Upward CCT".into()
}
fn default_slab_blue_content() -> String {
    "Blue content of spill".into()
}
fn default_slab_sp_cool() -> String {
    "cool — bright at night".into()
}
fn default_slab_sp_neutral() -> String {
    "neutral".into()
}
fn default_slab_sp_warm() -> String {
    "warm — dim at night".into()
}
fn default_slab_mel_high() -> String {
    "high circadian stimulus".into()
}
fn default_slab_mel_moderate() -> String {
    "moderate".into()
}
fn default_slab_mel_low() -> String {
    "low — sleep-friendly".into()
}
fn default_slab_mesopic_vs() -> String {
    "{pct}% vs photopic {photopic}".into()
}
fn default_slab_stats() -> String {
    "{photons} photons traced · {pct}% escaped to detector".into()
}
fn default_slab_location() -> String {
    "Location".into()
}
fn default_slab_date() -> String {
    "Date".into()
}
fn default_slab_time_of_day() -> String {
    "Local time".into()
}
fn default_slab_mode() -> String {
    "Trace mode".into()
}
fn default_slab_mode_luminaire() -> String {
    "Luminaire".into()
}
fn default_slab_mode_sky() -> String {
    "Sky".into()
}
fn default_slab_sun_altitude() -> String {
    "Sun altitude".into()
}
fn default_slab_sun_azimuth() -> String {
    "Sun azimuth".into()
}
fn default_slab_regime_day() -> String {
    "Daylight".into()
}
fn default_slab_regime_twilight() -> String {
    "Twilight".into()
}
fn default_slab_regime_night() -> String {
    "Night".into()
}
fn default_slab_sky_illuminance() -> String {
    "Sky illuminance".into()
}
fn default_slab_viz_sky_dome() -> String {
    "Sky colour".into()
}
fn default_slab_viz_scene() -> String {
    "Scene under this sky".into()
}
fn default_slab_viz_sun_path() -> String {
    "Sun path (today)".into()
}
fn default_slab_viz_timeline() -> String {
    "Day → night (24 h)".into()
}
fn default_slab_location_custom() -> String {
    "Custom…".into()
}
fn default_slab_latitude() -> String {
    "Lat".into()
}
fn default_slab_longitude() -> String {
    "Lon".into()
}
fn default_slab_regime_moonlit() -> String {
    "Moonlit".into()
}
fn default_slab_moon_below() -> String {
    "below horizon".into()
}
fn default_slab_moon_new() -> String {
    "New moon".into()
}
fn default_slab_moon_waxing_crescent() -> String {
    "Waxing crescent".into()
}
fn default_slab_moon_first_quarter() -> String {
    "First quarter".into()
}
fn default_slab_moon_waxing_gibbous() -> String {
    "Waxing gibbous".into()
}
fn default_slab_moon_full() -> String {
    "Full moon".into()
}
fn default_slab_moon_waning_gibbous() -> String {
    "Waning gibbous".into()
}
fn default_slab_moon_last_quarter() -> String {
    "Last quarter".into()
}
fn default_slab_moon_waning_crescent() -> String {
    "Waning crescent".into()
}
fn default_slab_sky_tab_title() -> String {
    "Daylight & Sky".into()
}
fn default_slab_sky_tab_intro() -> String {
    "Pick a place and a moment: the sun and moon update live, and the sky's light — daylight, twilight, moonlight — is traced onto a scene. Illuminance, colour and mesopic luminance come from the same Monte Carlo engine.".into()
}
fn default_slab_sky_empty_hint() -> String {
    "Adjust the location and time, then press Trace to compute the sky's illuminance and colour."
        .into()
}
fn default_slab_scene_road() -> String {
    "Road".into()
}
fn default_slab_scene_room() -> String {
    "Room".into()
}
fn default_slab_scene_plaza() -> String {
    "Plaza".into()
}

// Embedded locale JSON files
const EN_JSON: &str = include_str!("../locales/en.json");
const DE_JSON: &str = include_str!("../locales/de.json");
const ZH_JSON: &str = include_str!("../locales/zh.json");
const FR_JSON: &str = include_str!("../locales/fr.json");
const IT_JSON: &str = include_str!("../locales/it.json");
const RU_JSON: &str = include_str!("../locales/ru.json");
const ES_JSON: &str = include_str!("../locales/es.json");
const PT_BR_JSON: &str = include_str!("../locales/pt-BR.json");

// ─────────────────────────────────────────────────────────────────────────
// Street designer translations
// ─────────────────────────────────────────────────────────────────────────

/// Street lighting designer translations (layout form, compliance panel,
/// threshold controls, source badge). Everything is `Default`-derived so
/// missing keys fall back to empty strings — callers decide how to present
/// those.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetLocale {
    pub title: String,
    pub source: StreetSource,
    pub layout: StreetLayoutLocale,
    pub arrangement: StreetArrangement,
    pub threshold: StreetThreshold,
    pub stats: StreetStats,
    pub compliance: StreetCompliance,
    pub placeholder: String,
    /// Region picker at the top ("which standard is authoritative").
    pub standard_selector: StreetStandardSelector,
    /// RP-8 (US) decision tree.
    pub rp8_tree: StreetRp8Tree,
    /// EN 13201 (EU) decision tree.
    pub en13201_tree: StreetEn13201Tree,
    /// CJJ 45 (CN) decision tree.
    pub cjj45_tree: StreetCjj45Tree,
    /// Optimizer panel.
    pub optimizer: StreetOptimizer,
    /// Principal-planes (PV/PC) diagram toggle.
    #[serde(default)]
    pub principal_planes: StreetPrincipalPlanes,
    /// Export buttons (SVG / CSV / JSON / PDF).
    #[serde(default)]
    pub exports: StreetExports,
    /// Advisor recommendations on compliance failures.
    #[serde(default)]
    pub advisor: StreetAdvisor,
    /// Toggleable overlays on the plan view.
    #[serde(default)]
    pub plan_overlays: StreetPlanOverlays,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetAdvisor {
    /// Section heading, e.g., "Recommendations".
    #[serde(default)]
    pub heading: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetPlanOverlays {
    /// Toggle label for showing the calculation grid points
    /// (per EN 13201 / RP-8 prescribed grid locations).
    #[serde(default)]
    pub grid_points: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetExports {
    /// Section heading (e.g., "Export").
    #[serde(default)]
    pub heading: String,
    /// Plan-view SVG download button.
    #[serde(default)]
    pub plan_svg: String,
    /// Principal-planes (PV/PC) SVG download.
    #[serde(default)]
    pub principal_planes_svg: String,
    /// Layout trade-off chart SVG download.
    #[serde(default)]
    pub tradeoff_svg: String,
    /// Optimizer results CSV download.
    #[serde(default)]
    pub optimizer_csv: String,
    /// Optimizer results JSON download.
    #[serde(default)]
    pub optimizer_json: String,
    /// Full PDF report (Typst).
    #[serde(default)]
    pub report_pdf: String,
    /// Tooltip / hint shown on the report download button to clarify
    /// the workflow ("compile with `typst compile street-report.typ`").
    #[serde(default)]
    pub report_pdf_hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetPrincipalPlanes {
    /// "Show PV/PC distribution" toggle label.
    #[serde(default)]
    pub show: String,
    /// "Hide PV/PC distribution" toggle label.
    #[serde(default)]
    pub hide: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetStandardSelector {
    pub legend: String,
    pub rp8: String,
    pub en13201: String,
    pub cjj45: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetRp8Tree {
    pub legend: String,
    pub road_class: String,
    pub major: String,
    pub collector: String,
    pub local: String,
    pub pedestrian_conflict: String,
    pub ped_high: String,
    pub ped_medium: String,
    pub ped_low: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetEn13201Tree {
    pub legend: String,
    pub family: String,
    pub family_c: String,
    pub family_p: String,
    pub design_speed: String,
    pub speed_very_high: String,
    pub speed_high: String,
    pub speed_moderate: String,
    pub speed_low: String,
    pub traffic_volume: String,
    pub vol_very_high: String,
    pub vol_high: String,
    pub vol_moderate: String,
    pub vol_low: String,
    pub vol_very_low: String,
    pub traffic_composition: String,
    pub comp_mixed: String,
    pub comp_mixed_motor: String,
    pub comp_motor_only: String,
    pub separation: String,
    pub sep_yes: String,
    pub sep_no: String,
    pub junction_density: String,
    pub jd_high: String,
    pub jd_moderate: String,
    pub parked_vehicles: String,
    pub parked_yes: String,
    pub parked_no: String,
    pub ambient_luminance: String,
    pub amb_high: String,
    pub amb_moderate: String,
    pub amb_low: String,
    pub navigational_task: String,
    pub nav_very_difficult: String,
    pub nav_difficult: String,
    pub nav_easy: String,
    pub pedestrian_speed: String,
    pub pspeed_walking: String,
    pub pspeed_slow_mixed: String,
    pub pspeed_low_motor: String,
    pub user_density: String,
    pub ud_high: String,
    pub ud_moderate: String,
    pub ud_low: String,
    pub facial_recognition: String,
    pub fr_necessary: String,
    pub fr_useful: String,
    pub fr_not_needed: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetCjj45Tree {
    pub legend: String,
    pub road_grade: String,
    pub expressway: String,
    pub major_arterial: String,
    pub minor_arterial: String,
    /// 支路 (branch / local road) → Class III.
    #[serde(default)]
    pub branch: String,
    /// 居住区道路 (residential street) → Class IV.
    #[serde(default)]
    pub residential: String,
    /// Legacy combined "Branch or residential" key, kept so existing
    /// translations don't break the deserialization. New keys are
    /// `branch` and `residential`.
    #[serde(default, alias = "branch_or_residential")]
    pub branch_or_residential: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetOptimizer {
    pub legend: String,
    pub objective_label: String,
    pub obj_pole_count: String,
    pub obj_flux: String,
    pub obj_safety: String,
    pub run: String,
    pub running: String,
    pub no_results: String,
    pub col_spacing: String,
    pub col_height: String,
    pub col_arrangement: String,
    pub col_poles: String,
    pub col_flux: String,
    pub col_uniformity: String,
    pub col_apply: String,
    pub apply: String,
    /// "Show layout trade-off" toggle label (Pareto scatter plot of
    /// optimizer candidates).
    #[serde(default)]
    pub tradeoff_show: String,
    /// "Hide layout trade-off" toggle label.
    #[serde(default)]
    pub tradeoff_hide: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetSource {
    /// e.g. "Using luminaire from editor: "
    pub using: String,
    /// Hint when no luminaire is loaded anywhere.
    pub hint_none: String,
    /// Suffix "— live updates as you edit"
    pub live_updates: String,
    /// Placeholder for unnamed luminaires.
    pub unnamed: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetLayoutLocale {
    pub legend: String,
    pub length: String,
    pub lane_width: String,
    pub num_lanes: String,
    pub pole_spacing: String,
    pub mounting_height: String,
    pub overhang: String,
    pub tilt: String,
    pub pole_offset: String,
    pub sidewalk_width: String,
    pub arrangement_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetArrangement {
    pub single_side: String,
    pub opposite: String,
    pub staggered: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetThreshold {
    pub label: String,
    pub off: String,
    pub rp8: String,
    pub en13201: String,
    pub cjj45: String,
    pub custom: String,
    pub ratio: String,
    /// Template like "below {pct}% of avg" — caller does the substitution.
    pub below_ratio: String,
    /// Template like "below {lux} lux"
    pub below_lux: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetStats {
    pub avg: String,
    pub min: String,
    pub max: String,
    /// Unit suffix for illuminance — "lux" or "fc" depending on unit system.
    pub lux_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StreetCompliance {
    pub heading: String,
    pub pass: String,
    pub fail: String,
    pub criterion: String,
    pub required: String,
    pub achieved: String,
}

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

    /// Look up a flat translation key from the `extra` section.
    ///
    /// Returns `None` if the key is missing in this locale. Most callers
    /// should prefer the top-level [`crate::t`] / [`crate::t_or`] helpers,
    /// which add English fallback automatically.
    pub fn extra_value(&self, key: &str) -> Option<&str> {
        self.extra.get(key).map(|s| s.as_str())
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

/// Translate a flat key in the requested language, with English fallback.
///
/// Lookup order: target language's `extra` map → English `extra` map →
/// the key itself (so a missing translation produces a visible string
/// rather than empty space). Useful for sibling crates (gldf-rs etc.)
/// that want a single shared translation source for UI chrome without
/// adding new typed structs to the eulumdat-domain `Locale` model.
///
/// Keys should be namespaced (e.g. `gldf.nav.overview`) so different
/// consumers don't collide.
///
/// Note: this loads the locale from the embedded JSON on every call.
/// For perf-sensitive paths, cache a `Locale` instance and call
/// [`Locale::extra_value`] directly.
pub fn t(key: &str, lang: Language) -> String {
    if let Some(v) = Locale::for_language(lang).extra_value(key) {
        return v.to_string();
    }
    if !matches!(lang, Language::English) {
        if let Some(v) = Locale::english().extra_value(key) {
            return v.to_string();
        }
    }
    key.to_string()
}

/// Same as [`t`] but lets the caller supply the fallback string for a
/// missing key (instead of returning the key itself). Useful when the
/// English form is what we'd render anyway.
pub fn t_or(key: &str, lang: Language, fallback: &str) -> String {
    if let Some(v) = Locale::for_language(lang).extra_value(key) {
        return v.to_string();
    }
    if !matches!(lang, Language::English) {
        if let Some(v) = Locale::english().extra_value(key) {
            return v.to_string();
        }
    }
    fallback.to_string()
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

    /// Every locale must fully translate the Spectrum Lab tab: no field may be
    /// empty, and every non-English locale must actually differ from English
    /// on the translatable labels (guards against a JSON key going missing and
    /// silently falling back to the English `serde` default).
    #[test]
    fn spectrum_lab_fully_translated() {
        let en = Locale::english().spectrum_lab;
        assert!(!en.title.is_empty());

        for code in ["de", "fr", "es", "it", "ru", "zh", "pt-BR"] {
            let sl = Locale::for_code(code).spectrum_lab;
            // No empty strings anywhere.
            let fields = [
                ("title", &sl.title),
                ("intro", &sl.intro),
                ("source_spectrum", &sl.source_spectrum),
                ("photons", &sl.photons),
                ("road_luminance", &sl.road_luminance),
                ("trace", &sl.trace),
                ("tracing", &sl.tracing),
                ("empty_hint", &sl.empty_hint),
                ("cct", &sl.cct),
                ("sp_ratio", &sl.sp_ratio),
                ("melanopic_der", &sl.melanopic_der),
                ("mesopic_road", &sl.mesopic_road),
                ("dark_sky", &sl.dark_sky),
                ("passes", &sl.passes),
                ("fails", &sl.fails),
                ("ulor", &sl.ulor),
                ("upward_cct", &sl.upward_cct),
                ("blue_content", &sl.blue_content),
                ("sp_cool", &sl.sp_cool),
                ("sp_neutral", &sl.sp_neutral),
                ("sp_warm", &sl.sp_warm),
                ("mel_high", &sl.mel_high),
                ("mel_moderate", &sl.mel_moderate),
                ("mel_low", &sl.mel_low),
                ("mesopic_vs", &sl.mesopic_vs),
                ("stats", &sl.stats),
                ("location", &sl.location),
                ("date", &sl.date),
                ("time_of_day", &sl.time_of_day),
                ("mode", &sl.mode),
                ("mode_luminaire", &sl.mode_luminaire),
                ("mode_sky", &sl.mode_sky),
                ("sun_altitude", &sl.sun_altitude),
                ("sun_azimuth", &sl.sun_azimuth),
                ("regime_day", &sl.regime_day),
                ("regime_twilight", &sl.regime_twilight),
                ("regime_night", &sl.regime_night),
                ("sky_illuminance", &sl.sky_illuminance),
                ("viz_sky_dome", &sl.viz_sky_dome),
                ("viz_scene", &sl.viz_scene),
                ("viz_sun_path", &sl.viz_sun_path),
                ("viz_timeline", &sl.viz_timeline),
                ("location_custom", &sl.location_custom),
                ("latitude", &sl.latitude),
                ("longitude", &sl.longitude),
                ("regime_moonlit", &sl.regime_moonlit),
                ("moon_below", &sl.moon_below),
                ("moon_new", &sl.moon_new),
                ("moon_waxing_crescent", &sl.moon_waxing_crescent),
                ("moon_first_quarter", &sl.moon_first_quarter),
                ("moon_waxing_gibbous", &sl.moon_waxing_gibbous),
                ("moon_full", &sl.moon_full),
                ("moon_waning_gibbous", &sl.moon_waning_gibbous),
                ("moon_last_quarter", &sl.moon_last_quarter),
                ("moon_waning_crescent", &sl.moon_waning_crescent),
                ("sky_tab_title", &sl.sky_tab_title),
                ("sky_tab_intro", &sl.sky_tab_intro),
                ("sky_empty_hint", &sl.sky_empty_hint),
                ("scene_road", &sl.scene_road),
                ("scene_room", &sl.scene_room),
                ("scene_plaza", &sl.scene_plaza),
            ];
            for (name, val) in fields {
                assert!(!val.is_empty(), "{code}: spectrum_lab.{name} is empty");
            }
            // The title must be genuinely localized, not the English default.
            assert_ne!(
                sl.title, en.title,
                "{code}: spectrum_lab.title not translated (still English)"
            );
            // The stats template must keep the substitution placeholders.
            assert!(
                sl.stats.contains("{photons}") && sl.stats.contains("{pct}"),
                "{code}: spectrum_lab.stats lost its placeholders"
            );
            assert!(
                sl.mesopic_vs.contains("{pct}") && sl.mesopic_vs.contains("{photopic}"),
                "{code}: spectrum_lab.mesopic_vs lost its placeholders"
            );
        }
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

/// Light scene (path-traced viewer) translations. Every field falls back to
/// English so an older locale file still renders the panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLightScene {
    #[serde(default = "ls_intro_title")]
    pub intro_title: String,
    #[serde(default = "ls_intro_lead")]
    pub intro_lead: String,
    #[serde(default = "ls_intro_step1_title")]
    pub intro_step1_title: String,
    #[serde(default = "ls_intro_step1")]
    pub intro_step1: String,
    #[serde(default = "ls_intro_step2_title")]
    pub intro_step2_title: String,
    #[serde(default = "ls_intro_step2")]
    pub intro_step2: String,
    #[serde(default = "ls_intro_step3_title")]
    pub intro_step3_title: String,
    #[serde(default = "ls_intro_step3")]
    pub intro_step3: String,
    #[serde(default = "ls_intro_start")]
    pub intro_start: String,
    #[serde(default = "ls_intro_remember")]
    pub intro_remember: String,
    #[serde(default = "ls_help")]
    pub help: String,
    #[serde(default = "ls_hint_render")]
    pub hint_render: String,
    #[serde(default = "ls_hint_isolux")]
    pub hint_isolux: String,
    #[serde(default = "ls_hint_points")]
    pub hint_points: String,
    #[serde(default = "ls_refining")]
    pub refining: String,
    #[serde(default = "ls_complete")]
    pub complete: String,
    #[serde(default = "ls_measuring_points")]
    pub measuring_points: String,
    #[serde(default = "ls_step_scene")]
    pub step_scene: String,
    #[serde(default = "ls_step_view")]
    pub step_view: String,
    #[serde(default = "ls_step_placement")]
    pub step_placement: String,
    #[serde(default = "ls_light_camera")]
    pub light_camera: String,
    #[serde(default = "ls_advanced")]
    pub advanced: String,
    #[serde(default = "ls_scene_street")]
    pub scene_street: String,
    #[serde(default = "ls_scene_parking")]
    pub scene_parking: String,
    #[serde(default = "ls_scene_facade")]
    pub scene_facade: String,
    #[serde(default = "ls_scene_office")]
    pub scene_office: String,
    #[serde(default = "ls_desc_street")]
    pub desc_street: String,
    #[serde(default = "ls_desc_parking")]
    pub desc_parking: String,
    #[serde(default = "ls_desc_facade")]
    pub desc_facade: String,
    #[serde(default = "ls_desc_office")]
    pub desc_office: String,
    #[serde(default = "ls_mode_render")]
    pub mode_render: String,
    #[serde(default = "ls_mode_isolux")]
    pub mode_isolux: String,
    #[serde(default = "ls_mode_points")]
    pub mode_points: String,
    #[serde(default = "ls_mode_render_desc")]
    pub mode_render_desc: String,
    #[serde(default = "ls_mode_isolux_desc")]
    pub mode_isolux_desc: String,
    #[serde(default = "ls_mode_points_desc")]
    pub mode_points_desc: String,
    #[serde(default = "ls_isolines_on_photo")]
    pub isolines_on_photo: String,
    #[serde(default = "ls_legend_lines")]
    pub legend_lines: String,
    #[serde(default = "ls_col_grid")]
    pub col_grid: String,
    #[serde(default = "ls_col_em")]
    pub col_em: String,
    #[serde(default = "ls_col_emin")]
    pub col_emin: String,
    #[serde(default = "ls_col_uo")]
    pub col_uo: String,
    #[serde(default = "ls_measuring")]
    pub measuring: String,
    #[serde(default = "ls_stats_explain")]
    pub stats_explain: String,
    #[serde(default = "ls_mounting_height")]
    pub mounting_height: String,
    #[serde(default = "ls_height_above_floor")]
    pub height_above_floor: String,
    #[serde(default = "ls_ceiling_height")]
    pub ceiling_height: String,
    #[serde(default = "ls_distance_to_facade")]
    pub distance_to_facade: String,
    #[serde(default = "ls_tilt")]
    pub tilt: String,
    #[serde(default = "ls_aim_auto")]
    pub aim_auto: String,
    #[serde(default = "ls_rotation")]
    pub rotation: String,
    #[serde(default = "ls_luminaires")]
    pub luminaires: String,
    #[serde(default = "ls_spacing")]
    pub spacing: String,
    #[serde(default = "ls_layout_single")]
    pub layout_single: String,
    #[serde(default = "ls_layout_opposite")]
    pub layout_opposite: String,
    #[serde(default = "ls_layout_staggered")]
    pub layout_staggered: String,
    #[serde(default = "ls_facade_render")]
    pub facade_render: String,
    #[serde(default = "ls_facade_brick")]
    pub facade_brick: String,
    #[serde(default = "ls_person_for_scale")]
    pub person_for_scale: String,
    #[serde(default = "ls_cct_from_file")]
    pub cct_from_file: String,
    #[serde(default = "ls_cct")]
    pub cct: String,
    #[serde(default = "ls_haze")]
    pub haze: String,
    #[serde(default = "ls_exposure")]
    pub exposure: String,
    #[serde(default = "ls_glare")]
    pub glare: String,
    #[serde(default = "ls_bounces")]
    pub bounces: String,
    #[serde(default = "ls_reset_view")]
    pub reset_view: String,
    #[serde(default = "ls_restart")]
    pub restart: String,
    #[serde(default = "ls_restart_tip")]
    pub restart_tip: String,
    #[serde(default = "ls_nav_hint")]
    pub nav_hint: String,
    #[serde(default = "ls_compute")]
    pub compute: String,
    #[serde(default = "ls_per_pixel")]
    pub per_pixel: String,
    #[serde(default = "ls_per_point")]
    pub per_point: String,
    #[serde(default = "ls_per_frame")]
    pub per_frame: String,
    #[serde(default = "ls_samples_line")]
    pub samples_line: String,
    #[serde(default = "ls_cpu_note")]
    pub cpu_note: String,
    #[serde(default = "ls_no_webgpu")]
    pub no_webgpu: String,
    #[serde(default = "ls_starting")]
    pub starting: String,
    #[serde(default = "ls_waiting")]
    pub waiting: String,
    #[serde(default = "ls_summary")]
    pub summary: String,
    #[serde(default = "ls_scene_greenhouse")]
    pub scene_greenhouse: String,
    #[serde(default = "ls_desc_greenhouse")]
    pub desc_greenhouse: String,
    #[serde(default = "ls_canopy_height")]
    pub canopy_height: String,
    #[serde(default = "ls_photoperiod")]
    pub photoperiod: String,
    #[serde(default = "ls_ppfd_from_spd")]
    pub ppfd_from_spd: String,
    #[serde(default = "ls_ppfd_assumed")]
    pub ppfd_assumed: String,
    #[serde(default = "ls_dli")]
    pub dli: String,
    #[serde(default = "ls_dli_explain")]
    pub dli_explain: String,
    #[serde(default = "ls_hint_ppfd")]
    pub hint_ppfd: String,
    #[serde(default = "ls_unit_ppfd")]
    pub unit_ppfd: String,
}

impl Default for UiLightScene {
    fn default() -> Self {
        Self {
            intro_title: ls_intro_title(),
            intro_lead: ls_intro_lead(),
            intro_step1_title: ls_intro_step1_title(),
            intro_step1: ls_intro_step1(),
            intro_step2_title: ls_intro_step2_title(),
            intro_step2: ls_intro_step2(),
            intro_step3_title: ls_intro_step3_title(),
            intro_step3: ls_intro_step3(),
            intro_start: ls_intro_start(),
            intro_remember: ls_intro_remember(),
            help: ls_help(),
            hint_render: ls_hint_render(),
            hint_isolux: ls_hint_isolux(),
            hint_points: ls_hint_points(),
            refining: ls_refining(),
            complete: ls_complete(),
            measuring_points: ls_measuring_points(),
            step_scene: ls_step_scene(),
            step_view: ls_step_view(),
            step_placement: ls_step_placement(),
            light_camera: ls_light_camera(),
            advanced: ls_advanced(),
            scene_street: ls_scene_street(),
            scene_parking: ls_scene_parking(),
            scene_facade: ls_scene_facade(),
            scene_office: ls_scene_office(),
            desc_street: ls_desc_street(),
            desc_parking: ls_desc_parking(),
            desc_facade: ls_desc_facade(),
            desc_office: ls_desc_office(),
            mode_render: ls_mode_render(),
            mode_isolux: ls_mode_isolux(),
            mode_points: ls_mode_points(),
            mode_render_desc: ls_mode_render_desc(),
            mode_isolux_desc: ls_mode_isolux_desc(),
            mode_points_desc: ls_mode_points_desc(),
            isolines_on_photo: ls_isolines_on_photo(),
            legend_lines: ls_legend_lines(),
            col_grid: ls_col_grid(),
            col_em: ls_col_em(),
            col_emin: ls_col_emin(),
            col_uo: ls_col_uo(),
            measuring: ls_measuring(),
            stats_explain: ls_stats_explain(),
            mounting_height: ls_mounting_height(),
            height_above_floor: ls_height_above_floor(),
            ceiling_height: ls_ceiling_height(),
            distance_to_facade: ls_distance_to_facade(),
            tilt: ls_tilt(),
            aim_auto: ls_aim_auto(),
            rotation: ls_rotation(),
            luminaires: ls_luminaires(),
            spacing: ls_spacing(),
            layout_single: ls_layout_single(),
            layout_opposite: ls_layout_opposite(),
            layout_staggered: ls_layout_staggered(),
            facade_render: ls_facade_render(),
            facade_brick: ls_facade_brick(),
            person_for_scale: ls_person_for_scale(),
            cct_from_file: ls_cct_from_file(),
            cct: ls_cct(),
            haze: ls_haze(),
            exposure: ls_exposure(),
            glare: ls_glare(),
            bounces: ls_bounces(),
            reset_view: ls_reset_view(),
            restart: ls_restart(),
            restart_tip: ls_restart_tip(),
            nav_hint: ls_nav_hint(),
            compute: ls_compute(),
            per_pixel: ls_per_pixel(),
            per_point: ls_per_point(),
            per_frame: ls_per_frame(),
            samples_line: ls_samples_line(),
            cpu_note: ls_cpu_note(),
            no_webgpu: ls_no_webgpu(),
            starting: ls_starting(),
            waiting: ls_waiting(),
            summary: ls_summary(),
            scene_greenhouse: ls_scene_greenhouse(),
            desc_greenhouse: ls_desc_greenhouse(),
            canopy_height: ls_canopy_height(),
            photoperiod: ls_photoperiod(),
            ppfd_from_spd: ls_ppfd_from_spd(),
            ppfd_assumed: ls_ppfd_assumed(),
            dli: ls_dli(),
            dli_explain: ls_dli_explain(),
            hint_ppfd: ls_hint_ppfd(),
            unit_ppfd: ls_unit_ppfd(),
        }
    }
}

fn ls_intro_title() -> String {
    "See your luminaire at work".into()
}
fn ls_intro_lead() -> String {
    "Everything you see here is lit by the photometric data of {name}: a path tracer sends rays from the real intensity table into the scene. The picture starts grainy and sharpens as more rays arrive.".into()
}
fn ls_intro_step1_title() -> String {
    "Choose a scene".into()
}
fn ls_intro_step1() -> String {
    "Street, parking lot, facade or office. The luminaire is mounted at a typical height and aimed automatically.".into()
}
fn ls_intro_step2_title() -> String {
    "Choose a view".into()
}
fn ls_intro_step2() -> String {
    "Photo shows what the eye sees. Illuminance paints lux values in colour. Grid places standard measurement points and reports averages and uniformity.".into()
}
fn ls_intro_step3_title() -> String {
    "Look around and adjust".into()
}
fn ls_intro_step3() -> String {
    "Drag to orbit, scroll to zoom. Change mounting height, tilt, spacing and the number of luminaires in the panel on the left.".into()
}
fn ls_intro_start() -> String {
    "Let's go".into()
}
fn ls_intro_remember() -> String {
    "Don't show this again".into()
}
fn ls_help() -> String {
    "Help".into()
}
fn ls_hint_render() -> String {
    "Drag to look around · scroll to zoom · switch to Illuminance to see lux values".into()
}
fn ls_hint_isolux() -> String {
    "Colour = illuminance in lux on every surface (scale in the panel) · lines every {step} lx"
        .into()
}
fn ls_hint_points() -> String {
    "Points = measurement grids per EN 12464-1 / EN 13201-3 · the table shows average Em, minimum Emin and uniformity Uo".into()
}
fn ls_refining() -> String {
    "Refining image… {pct} %".into()
}
fn ls_complete() -> String {
    "Image complete".into()
}
fn ls_measuring_points() -> String {
    "Measuring {n} points…".into()
}
fn ls_step_scene() -> String {
    "1 · Scene".into()
}
fn ls_step_view() -> String {
    "2 · View".into()
}
fn ls_step_placement() -> String {
    "3 · Placement".into()
}
fn ls_light_camera() -> String {
    "Light and camera".into()
}
fn ls_advanced() -> String {
    "Rays (advanced)".into()
}
fn ls_scene_street() -> String {
    "Street".into()
}
fn ls_scene_parking() -> String {
    "Parking".into()
}
fn ls_scene_facade() -> String {
    "Facade".into()
}
fn ls_scene_office() -> String {
    "Office".into()
}
fn ls_desc_street() -> String {
    "Two-lane road with cycle lane, sidewalks and a facade that catches spill light.".into()
}
fn ls_desc_parking() -> String {
    "Parking bays in the open, measured on the ground.".into()
}
fn ls_desc_facade() -> String {
    "Grazing or washing light on a building front.".into()
}
fn ls_desc_office() -> String {
    "Workplaces, measured on the 0.75 m desk plane.".into()
}
fn ls_mode_render() -> String {
    "Photo".into()
}
fn ls_mode_isolux() -> String {
    "Illuminance".into()
}
fn ls_mode_points() -> String {
    "Grid".into()
}
fn ls_mode_render_desc() -> String {
    "What the eye sees at night.".into()
}
fn ls_mode_isolux_desc() -> String {
    "Lux on every surface as colour, with isolux lines.".into()
}
fn ls_mode_points_desc() -> String {
    "Standard measurement points with Em, Emin and Uo per grid.".into()
}
fn ls_isolines_on_photo() -> String {
    "Isolux lines on the photo".into()
}
fn ls_legend_lines() -> String {
    "lines every {step} lx".into()
}
fn ls_col_grid() -> String {
    "Grid".into()
}
fn ls_col_em() -> String {
    "Em".into()
}
fn ls_col_emin() -> String {
    "Emin".into()
}
fn ls_col_uo() -> String {
    "Uo".into()
}
fn ls_measuring() -> String {
    "measuring…".into()
}
fn ls_stats_explain() -> String {
    "Em = average, Emin = lowest point, Uo = Emin / Em (uniformity). Typical targets: roads Uo ≥ 0.4, office task areas Uo ≥ 0.6.".into()
}
fn ls_mounting_height() -> String {
    "Mounting height".into()
}
fn ls_height_above_floor() -> String {
    "Height above floor".into()
}
fn ls_ceiling_height() -> String {
    "Ceiling height".into()
}
fn ls_distance_to_facade() -> String {
    "Distance to facade".into()
}
fn ls_tilt() -> String {
    "Tilt".into()
}
fn ls_aim_auto() -> String {
    "Aim automatically (throw towards the road / facade)".into()
}
fn ls_rotation() -> String {
    "Rotation".into()
}
fn ls_luminaires() -> String {
    "Luminaires".into()
}
fn ls_spacing() -> String {
    "Spacing".into()
}
fn ls_layout_single() -> String {
    "One side".into()
}
fn ls_layout_opposite() -> String {
    "Opposite".into()
}
fn ls_layout_staggered() -> String {
    "Staggered".into()
}
fn ls_facade_render() -> String {
    "Rendered facade".into()
}
fn ls_facade_brick() -> String {
    "Brick facade".into()
}
fn ls_person_for_scale() -> String {
    "Person for scale".into()
}
fn ls_cct_from_file() -> String {
    "Colour temperature from the file".into()
}
fn ls_cct() -> String {
    "Colour temperature".into()
}
fn ls_haze() -> String {
    "Haze".into()
}
fn ls_exposure() -> String {
    "Exposure".into()
}
fn ls_glare() -> String {
    "Veiling glare".into()
}
fn ls_bounces() -> String {
    "Bounces".into()
}
fn ls_reset_view() -> String {
    "Reset view".into()
}
fn ls_restart() -> String {
    "Restart tracer".into()
}
fn ls_restart_tip() -> String {
    "Start the accumulation over; rebuilds the GPU if it was lost".into()
}
fn ls_nav_hint() -> String {
    "Drag to orbit, right-drag or shift-drag to pan, wheel to zoom.".into()
}
fn ls_compute() -> String {
    "Compute".into()
}
fn ls_per_pixel() -> String {
    "Per pixel".into()
}
fn ls_per_point() -> String {
    "Per point".into()
}
fn ls_per_frame() -> String {
    "Per frame".into()
}
fn ls_samples_line() -> String {
    "{s} / {smax} samples per pixel · {p} / {pmax} per point · key {key} lx".into()
}
fn ls_cpu_note() -> String {
    "The CPU runs the same transport in Rust at preview size: slow, but it works without WebGPU and cross-checks the grid values.".into()
}
fn ls_no_webgpu() -> String {
    "No WebGPU: tracing on the CPU at preview size".into()
}
fn ls_starting() -> String {
    "Starting WebGPU…".into()
}
fn ls_waiting() -> String {
    "Waiting for the viewer…".into()
}
fn ls_summary() -> String {
    "{name} · {lm} lm · peak {cd} cd · throw C{c} ({pct} %)".into()
}

/// Start-page guide ("what would you like to do?") on the dashboard.
/// Every field falls back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardGuide {
    #[serde(default = "dg_title")]
    pub title: String,
    #[serde(default = "dg_step1")]
    pub step1: String,
    #[serde(default = "dg_step2")]
    pub step2: String,
    #[serde(default = "dg_selected")]
    pub selected: String,
    #[serde(default = "dg_nothing_selected")]
    pub nothing_selected: String,
    #[serde(default = "dg_open_editor")]
    pub open_editor: String,
    #[serde(default = "dg_open_editor_desc")]
    pub open_editor_desc: String,
    #[serde(default = "dg_light_scene")]
    pub light_scene: String,
    #[serde(default = "dg_light_scene_desc")]
    pub light_scene_desc: String,
    #[serde(default = "dg_area_designer")]
    pub area_designer: String,
    #[serde(default = "dg_area_designer_desc")]
    pub area_designer_desc: String,
    #[serde(default = "dg_pdf")]
    pub pdf: String,
    #[serde(default = "dg_compare")]
    pub compare: String,
    #[serde(default = "dg_compare_hint")]
    pub compare_hint: String,
    #[serde(default = "dg_zoom")]
    pub zoom: String,
    #[serde(default = "dg_mode_overview")]
    pub mode_overview: String,
    #[serde(default = "dg_mode_beam_angles")]
    pub mode_beam_angles: String,
    #[serde(default = "dg_mode_beam_intensities")]
    pub mode_beam_intensities: String,
    #[serde(default = "dg_mode_ugr")]
    pub mode_ugr: String,
    #[serde(default = "dg_about_site")]
    pub about_site: String,
}

impl Default for DashboardGuide {
    fn default() -> Self {
        Self {
            title: dg_title(),
            step1: dg_step1(),
            step2: dg_step2(),
            selected: dg_selected(),
            nothing_selected: dg_nothing_selected(),
            open_editor: dg_open_editor(),
            open_editor_desc: dg_open_editor_desc(),
            light_scene: dg_light_scene(),
            light_scene_desc: dg_light_scene_desc(),
            area_designer: dg_area_designer(),
            area_designer_desc: dg_area_designer_desc(),
            pdf: dg_pdf(),
            compare: dg_compare(),
            compare_hint: dg_compare_hint(),
            zoom: dg_zoom(),
            mode_overview: dg_mode_overview(),
            mode_beam_angles: dg_mode_beam_angles(),
            mode_beam_intensities: dg_mode_beam_intensities(),
            mode_ugr: dg_mode_ugr(),
            about_site: dg_about_site(),
        }
    }
}

fn dg_title() -> String {
    "What would you like to do?".into()
}
fn dg_step1() -> String {
    "Pick a luminaire from the list below, or drop your own LDT / IES file anywhere on this page."
        .into()
}
fn dg_step2() -> String {
    "Then open it:".into()
}
fn dg_selected() -> String {
    "Selected:".into()
}
fn dg_nothing_selected() -> String {
    "nothing selected yet".into()
}
fn dg_open_editor() -> String {
    "Editor & diagrams".into()
}
fn dg_open_editor_desc() -> String {
    "Polar, cartesian, isolux, BUG rating, validation, export".into()
}
fn dg_light_scene() -> String {
    "Light scene".into()
}
fn dg_light_scene_desc() -> String {
    "Path-traced street, parking lot, facade or office with lux values".into()
}
fn dg_area_designer() -> String {
    "Area designer".into()
}
fn dg_area_designer_desc() -> String {
    "Place several luminaires and check the lux on the ground".into()
}
fn dg_pdf() -> String {
    "PDF report".into()
}
fn dg_compare() -> String {
    "Compare".into()
}
fn dg_compare_hint() -> String {
    "Ctrl / ⌘ + click a second row to compare two luminaires".into()
}
fn dg_zoom() -> String {
    "Zoom diagram".into()
}

/// Feature overview on the dashboard; every field falls back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardFeatures {
    #[serde(default = "df_heading")]
    pub heading: String,
    #[serde(default = "df_show")]
    pub show: String,
    #[serde(default = "df_hide")]
    pub hide: String,
    #[serde(default = "df_drop_hint")]
    pub drop_hint: String,
    #[serde(default = "df_editor_title")]
    pub editor_title: String,
    #[serde(default = "df_validation_title")]
    pub validation_title: String,
    #[serde(default = "df_bug_title")]
    pub bug_title: String,
    #[serde(default = "df_compare_title")]
    pub compare_title: String,
    #[serde(default = "df_bim_title")]
    pub bim_title: String,
    #[serde(default = "df_report_title")]
    pub report_title: String,
    #[serde(default = "df_light_scene_title")]
    pub light_scene_title: String,
    #[serde(default = "df_scene_3d_title")]
    pub scene_3d_title: String,
    #[serde(default = "df_area_title")]
    pub area_title: String,
    #[serde(default = "df_interior_title")]
    pub interior_title: String,
    #[serde(default = "df_maps_title")]
    pub maps_title: String,
    #[serde(default = "df_street_title")]
    pub street_title: String,
    #[serde(default = "df_spectral_title")]
    pub spectral_title: String,
    #[serde(default = "df_greenhouse_title")]
    pub greenhouse_title: String,
    #[serde(default = "df_daylight_title")]
    pub daylight_title: String,
    #[serde(default = "df_skyglow_title")]
    pub skyglow_title: String,
    #[serde(default = "df_gonio_title")]
    pub gonio_title: String,
    #[serde(default = "df_tm25_title")]
    pub tm25_title: String,
    #[serde(default = "df_digitizer_title")]
    pub digitizer_title: String,
    #[serde(default = "df_gldf_title")]
    pub gldf_title: String,
    #[serde(default = "df_quiz_title")]
    pub quiz_title: String,
    #[serde(default = "df_editor_desc")]
    pub editor_desc: String,
    #[serde(default = "df_validation_desc")]
    pub validation_desc: String,
    #[serde(default = "df_bug_desc")]
    pub bug_desc: String,
    #[serde(default = "df_compare_desc")]
    pub compare_desc: String,
    #[serde(default = "df_bim_desc")]
    pub bim_desc: String,
    #[serde(default = "df_report_desc")]
    pub report_desc: String,
    #[serde(default = "df_light_scene_desc")]
    pub light_scene_desc: String,
    #[serde(default = "df_scene_3d_desc")]
    pub scene_3d_desc: String,
    #[serde(default = "df_area_desc")]
    pub area_desc: String,
    #[serde(default = "df_interior_desc")]
    pub interior_desc: String,
    #[serde(default = "df_maps_desc")]
    pub maps_desc: String,
    #[serde(default = "df_street_desc")]
    pub street_desc: String,
    #[serde(default = "df_spectral_desc")]
    pub spectral_desc: String,
    #[serde(default = "df_greenhouse_desc")]
    pub greenhouse_desc: String,
    #[serde(default = "df_daylight_desc")]
    pub daylight_desc: String,
    #[serde(default = "df_skyglow_desc")]
    pub skyglow_desc: String,
    #[serde(default = "df_gonio_desc")]
    pub gonio_desc: String,
    #[serde(default = "df_tm25_desc")]
    pub tm25_desc: String,
    #[serde(default = "df_digitizer_desc")]
    pub digitizer_desc: String,
    #[serde(default = "df_gldf_desc")]
    pub gldf_desc: String,
    #[serde(default = "df_quiz_desc")]
    pub quiz_desc: String,
    #[serde(default = "df_spectrum_lab_title")]
    pub spectrum_lab_title: String,
    #[serde(default = "df_spectrum_lab_desc")]
    pub spectrum_lab_desc: String,
}

impl Default for DashboardFeatures {
    fn default() -> Self {
        Self {
            heading: df_heading(),
            show: df_show(),
            hide: df_hide(),
            drop_hint: df_drop_hint(),
            editor_title: df_editor_title(),
            validation_title: df_validation_title(),
            bug_title: df_bug_title(),
            compare_title: df_compare_title(),
            bim_title: df_bim_title(),
            report_title: df_report_title(),
            light_scene_title: df_light_scene_title(),
            scene_3d_title: df_scene_3d_title(),
            area_title: df_area_title(),
            interior_title: df_interior_title(),
            maps_title: df_maps_title(),
            street_title: df_street_title(),
            spectral_title: df_spectral_title(),
            greenhouse_title: df_greenhouse_title(),
            daylight_title: df_daylight_title(),
            skyglow_title: df_skyglow_title(),
            gonio_title: df_gonio_title(),
            tm25_title: df_tm25_title(),
            digitizer_title: df_digitizer_title(),
            gldf_title: df_gldf_title(),
            quiz_title: df_quiz_title(),
            editor_desc: df_editor_desc(),
            validation_desc: df_validation_desc(),
            bug_desc: df_bug_desc(),
            compare_desc: df_compare_desc(),
            bim_desc: df_bim_desc(),
            report_desc: df_report_desc(),
            light_scene_desc: df_light_scene_desc(),
            scene_3d_desc: df_scene_3d_desc(),
            area_desc: df_area_desc(),
            interior_desc: df_interior_desc(),
            maps_desc: df_maps_desc(),
            street_desc: df_street_desc(),
            spectral_desc: df_spectral_desc(),
            greenhouse_desc: df_greenhouse_desc(),
            daylight_desc: df_daylight_desc(),
            skyglow_desc: df_skyglow_desc(),
            gonio_desc: df_gonio_desc(),
            tm25_desc: df_tm25_desc(),
            digitizer_desc: df_digitizer_desc(),
            gldf_desc: df_gldf_desc(),
            quiz_desc: df_quiz_desc(),
            spectrum_lab_title: df_spectrum_lab_title(),
            spectrum_lab_desc: df_spectrum_lab_desc(),
        }
    }
}

fn df_heading() -> String {
    "Everything on this site".into()
}
fn df_show() -> String {
    "Show all features".into()
}
fn df_hide() -> String {
    "Hide features".into()
}
fn df_drop_hint() -> String {
    "Most tools work on the luminaire you pick or drop; the rest open their own page.".into()
}
fn df_editor_title() -> String {
    "Editor & diagrams".into()
}
fn df_validation_title() -> String {
    "Validation".into()
}
fn df_bug_title() -> String {
    "BUG & LCS rating".into()
}
fn df_compare_title() -> String {
    "Compare".into()
}
fn df_bim_title() -> String {
    "Export & BIM".into()
}
fn df_report_title() -> String {
    "PDF report".into()
}
fn df_light_scene_title() -> String {
    "Light scene".into()
}
fn df_scene_3d_title() -> String {
    "3D scene".into()
}
fn df_area_title() -> String {
    "Area designer".into()
}
fn df_interior_title() -> String {
    "Interior designer".into()
}
fn df_maps_title() -> String {
    "Maps designer".into()
}
fn df_street_title() -> String {
    "Street design".into()
}
fn df_spectral_title() -> String {
    "Spectral lab".into()
}
fn df_greenhouse_title() -> String {
    "Greenhouse".into()
}
fn df_daylight_title() -> String {
    "Daylight sky".into()
}
fn df_skyglow_title() -> String {
    "Skyglow".into()
}
fn df_gonio_title() -> String {
    "Virtual goniophotometer".into()
}
fn df_tm25_title() -> String {
    "TM-25 ray files".into()
}
fn df_digitizer_title() -> String {
    "Chart digitizer".into()
}
fn df_gldf_title() -> String {
    "GLDF containers".into()
}
fn df_quiz_title() -> String {
    "Lighting quiz".into()
}
fn df_editor_desc() -> String {
    "Edit LDT / IES / ATLA values; polar, cartesian, 3D, heatmap, cone, isolux and beam-angle diagrams".into()
}
fn df_validation_desc() -> String {
    "Checks the file against the EN 13032 / IES LM-63 rules and explains every warning".into()
}
fn df_bug_desc() -> String {
    "IES TM-15 backlight, uplight and glare rating with the luminaire classification system".into()
}
fn df_compare_desc() -> String {
    "Two luminaires side by side: diagrams, flux, efficacy and distribution differences".into()
}
fn df_bim_desc() -> String {
    "LDT, IES, ATLA XML / JSON, SVG, PNG and BIM geometry for your planning tools".into()
}
fn df_report_desc() -> String {
    "Datasheet-style report with diagrams and tables, typeset in the browser".into()
}
fn df_light_scene_desc() -> String {
    "Path-traced street, parking lot, facade or office lit by the real intensity table, with lux grids and isolux".into()
}
fn df_scene_3d_desc() -> String {
    "Walk through a room, road or parking lot with the luminaire as a game-engine light".into()
}
fn df_area_desc() -> String {
    "Place several luminaires on an area and read the illuminance on the ground".into()
}
fn df_interior_desc() -> String {
    "Rooms with zones, workplanes and uniformity per EN 12464-1".into()
}
fn df_maps_desc() -> String {
    "Plan luminaires on a real map and check the lighting of streets and places".into()
}
fn df_street_desc() -> String {
    "Multi-luminaire road lighting against EN 13201, RP-8, CJJ 45 and MLO".into()
}
fn df_spectral_desc() -> String {
    "Spectra, CCT, CRI, TM-30, melanopic and horticultural PPFD from SPD data".into()
}
fn df_greenhouse_desc() -> String {
    "Grow lights and daylight through spectral glazing with a canopy PPFD map and Monte Carlo check"
        .into()
}
fn df_daylight_desc() -> String {
    "Sun position, clear and overcast sky and daylight factors for any place and date".into()
}
fn df_skyglow_desc() -> String {
    "Night-sky brightness and light-pollution demo with real star data".into()
}
fn df_gonio_desc() -> String {
    "Monte Carlo measurement of lamps with covers and reflectors; LDT from a design".into()
}
fn df_tm25_desc() -> String {
    "Near-field LED ray sets: far-field solid, spectrum, luminance map and LDT / ATLA export. Also at tm25.eu.".into()
}
fn df_digitizer_desc() -> String {
    "Drop a spectrum chart image and get an SPD file attached to the luminaire".into()
}
fn df_gldf_desc() -> String {
    "Drop a .gldf: photometry plus the manufacturer's L3D body in the Light scene".into()
}
fn df_quiz_desc() -> String {
    "175 questions on photometry and lighting design in eight languages".into()
}
fn dg_mode_overview() -> String {
    "Overview".into()
}
fn dg_mode_beam_angles() -> String {
    "Beam angles".into()
}
fn dg_mode_beam_intensities() -> String {
    "Intensities".into()
}
fn dg_mode_ugr() -> String {
    "UGR table".into()
}
fn ls_scene_greenhouse() -> String {
    "Greenhouse".into()
}
fn ls_desc_greenhouse() -> String {
    "Glasshouse with two crop benches; values on the canopy in PPFD (µmol/m²/s) with the daily light integral.".into()
}
fn ls_canopy_height() -> String {
    "Canopy height".into()
}
fn ls_photoperiod() -> String {
    "Photoperiod".into()
}
fn ls_ppfd_from_spd() -> String {
    "PPFD from the luminaire's spectrum: {f} µmol per lx·s".into()
}
fn ls_ppfd_assumed() -> String {
    "No spectrum in the file: PPFD assumes a white LED, {f} µmol per lx·s. Load an ATLA / TM-33 file or attach an SPD for the real factor.".into()
}
fn ls_dli() -> String {
    "DLI".into()
}
fn ls_dli_explain() -> String {
    "DLI = PPFD average × photoperiod, in mol/m²/day. Leafy greens want 12–17, fruiting crops 20–30.".into()
}
fn ls_hint_ppfd() -> String {
    "Colour = PPFD in µmol/m²/s on every surface (scale in the panel) · lines every {step}".into()
}
fn ls_unit_ppfd() -> String {
    "µmol/m²/s".into()
}
fn dg_about_site() -> String {
    "What is Candelabra?".into()
}
fn df_spectrum_lab_title() -> String {
    "Spectrum Lab".into()
}
fn df_spectrum_lab_desc() -> String {
    "Photon-by-photon spectral Monte Carlo: night vision (S/P), circadian (melanopic DER), mesopic road luminance, colour over angle and a dark-sky report".into()
}
