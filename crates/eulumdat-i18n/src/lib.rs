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
    #[serde(default)]
    pub sensor: UiSensor,
    #[serde(default)]
    pub colour_quality: UiColourQuality,
    #[serde(default)]
    pub sensor_compare: UiSensorCompare,
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
    /// File menu: sections and export names.
    #[serde(default)]
    pub file_menu: UiFileMenu,
}

/// File menu sections and export names (missing keys fall back to English).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiFileMenu {
    pub section_project: String,
    pub export_project: String,
    pub export_project_tooltip: String,
    pub include_scenes: String,
    pub project_demo_note: String,
    pub section_luminaire: String,
    pub ldt: String,
    pub ies: String,
    pub tm33_xml: String,
    pub tm33_json: String,
    pub tm32_xml: String,
    pub tm32_json: String,
    pub tm33_tooltip: String,
    pub tm32_tooltip: String,
    pub has_photometry: String,
    pub has_spectrum: String,
    pub has_colour: String,
    pub has_bim: String,
    pub has_custom: String,
    pub atla: String,
    pub atla_tooltip: String,
    pub tm27: String,
    pub tm27_hint: String,
    pub tm27_tooltip: String,
    pub section_diagram: String,
}

impl Default for UiFileMenu {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            section_project: s("Project"),
            export_project: s("Project file (.candelabra.json)"),
            export_project_tooltip: s("All files of the project in one file; open it again with File › Open…"),
            include_scenes: s("with the saved Light scenes ({n})"),
            project_demo_note: s("The Demo is built in. Open a file to start your own project."),
            section_luminaire: s("Luminaire"),
            ldt: s("EULUMDAT (.ldt)"),
            ies: s("IES LM-63 (.ies)"),
            tm33_xml: s("IES TM-33-23 (XML)"),
            tm33_json: s("IES TM-33-23 (JSON)"),
            tm32_xml: s("IES TM-33-23 / TM-32-24 (XML)"),
            tm32_json: s("IES TM-33-23 / TM-32-24 (JSON)"),
            tm33_tooltip: s("The IES format for luminaire optical data: photometry, spectrum and colour values in one file (dimensions in metres, intensities in candela)."),
            tm32_tooltip: s("TM-33-23 file that also carries the TM-32-24 BIM parameters of this luminaire (electrical, mechanical, mounting …)."),
            has_photometry: s("photometry"),
            has_spectrum: s("spectrum"),
            has_colour: s("colour values"),
            has_bim: s("TM-32-24 BIM data"),
            has_custom: s("custom data"),
            atla: s("ATLA S001 / UNI 11733"),
            atla_tooltip: s("The same data in the older European layout; most tools that read ATLA also read TM-33-23."),
            tm27: s("IES TM-27 SPDX (.spdx)"),
            tm27_hint: s("spectrum only"),
            tm27_tooltip: s("The luminaire's spectrum alone as an IES TM-27 SPDX document (TM-27-20), for spectral, colour and rendering tools."),
            section_diagram: s("Diagram and report"),
        }
    }
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
    /// Moonlight over the area (English when a locale lacks it).
    #[serde(default)]
    pub moon: AreaDesignerMoon,
    /// The OpenStreetMap site map under the plan.
    #[serde(default)]
    pub sitemap: AreaDesignerSiteMap,
}

/// The site map (OpenStreetMap) in the Outdoor designer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AreaDesignerSiteMap {
    pub title: String,
    pub show: String,
    pub desc: String,
    pub pick: String,
    pub site: String,
    pub lat: String,
    pub lon: String,
    pub opacity: String,
    pub move_map: String,
    pub move_hint: String,
    pub done: String,
    pub no_site: String,
    pub style: String,
}

impl Default for AreaDesignerSiteMap {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            title: s("🗺 Site map"),
            show: s("Show the OpenStreetMap map under the plan"),
            desc: s("Find the site on the globe (zoom in until streets and buildings show, then “Use this place”), draw the area on the map and place the poles. North is up; the site is the middle of the area."),
            pick: s("🌍 Find the site on the globe"),
            site: s("Site: {place}"),
            lat: s("Latitude"),
            lon: s("Longitude"),
            opacity: s("Map opacity"),
            move_map: s("✋ Move the map"),
            move_hint: s("Drag on the plan to move the map under the area."),
            done: s("Done"),
            no_site: s("No site yet: find it on the globe or enter its coordinates."),
            style: s("Map style"),
        }
    }
}

/// Moonlight in the Outdoor designer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AreaDesignerMoon {
    pub desc: String,
    pub with_moon: String,
    pub dim: String,
    pub dim_none: String,
    pub alone: String,
    pub not_needed: String,
}

impl Default for AreaDesignerMoon {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            desc: s("Natural light on open ground under a clear sky at the chosen date and time (the sun by day, the twilight sky at dusk, the moon at night), the same everywhere. The illuminance results stay the luminaires' own (codes do not count natural light); it is shown next to them."),
            with_moon: s("With the natural light then (+{lx} lx)"),
            dim: s("To keep the {target} lx minimum (optimizer target), the luminaires could run at {pct} % tonight."),
            dim_none: s("The luminaires alone stay below the {target} lx minimum; the natural light adds {lx} lx."),
            alone: s("The natural light alone gives {pct} % of the {target} lx minimum."),
            not_needed: s("The luminaires are not needed then: the natural light alone gives {lx} lx (minimum {target} lx)."),
        }
    }
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
    #[serde(default)]
    pub sensors: ZonalSensors,
    #[serde(default)]
    pub emergency: ZonalEmergency,
    #[serde(default)]
    pub ifc: ZonalIfc,
}

/// Emergency lighting (EN 1838) in the designers. Missing keys fall back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZonalEmergency {
    pub title: String,
    pub intro: String,
    pub enable: String,
    pub luminaire: String,
    pub lum_demo_route: String,
    pub lum_demo_open: String,
    pub lum_loaded: String,
    pub lum_room: String,
    pub flux: String,
    pub flux_auto: String,
    pub pct: String,
    pub height: String,
    pub mf: String,
    pub spacing_note: String,
    pub routes: String,
    pub draw_route: String,
    pub route_hint: String,
    pub finish: String,
    pub cancel: String,
    pub width: String,
    pub route_item: String,
    pub remove: String,
    pub open_area: String,
    pub open_area_hint: String,
    pub points: String,
    pub place: String,
    pub point_hint: String,
    pub done: String,
    pub kind_fire: String,
    pub kind_first_aid: String,
    pub kind_call: String,
    pub kind_sign: String,
    pub kind_exit: String,
    pub point_item: String,
    pub results_title: String,
    pub pass: String,
    pub fail: String,
    pub count: String,
    pub col_what: String,
    pub col_value: String,
    pub col_required: String,
    pub route_centre: String,
    pub route_band: String,
    pub route_ratio: String,
    pub point_vertical: String,
    pub glare: String,
    pub ra: String,
    pub ra_unknown: String,
    pub nothing: String,
    pub method: String,
    pub move_label: String,
    pub standard: String,
    pub place_corridor: String,
    pub place_stairs: String,
    pub place_crowded: String,
    pub place_other: String,
    pub room_place: String,
    pub no_requirement: String,
    pub view_normal: String,
    pub view_emergency: String,
    pub floor_legend: String,
    pub lum_off: String,
    pub band_half: String,
    pub band_full: String,
    pub band_pct: String,
    pub route_name: String,
    pub route_band_min: String,
    pub route_avg: String,
    pub route_ratio_band: String,
    pub route_width: String,
    pub route_width_req: String,
    pub room_place_name: String,
    pub open_name: String,
    pub room_name: String,
    pub area_min_of: String,
    pub area_avg_of: String,
    pub area_ratio_of: String,
    pub end_avg: String,
    pub end_min: String,
    pub not_required_above: String,
    pub end_output: String,
    pub end_output_hint: String,
    pub room_floor_hint: String,
    pub report_pdf: String,
    pub point_horizontal: String,
    pub exit_floor: String,
    pub lum_flux_min: String,
    pub count_of: String,
    pub route_gap: String,
    pub area_flux: String,
    pub area_spacing: String,
    pub for_information: String,
    pub kind_door_release: String,
    pub kind_escape_plan: String,
    pub kind_evac_aid: String,
    pub group_europe: String,
    pub group_americas: String,
    pub group_asia: String,
    /// Profile names and notes by profile id ([`EmergencyStandardText`]);
    /// ids missing in a locale fall back to English.
    pub standards: std::collections::BTreeMap<String, EmergencyStandardText>,
}

/// Name and note of one emergency-lighting standard in the picker.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EmergencyStandardText {
    /// Shown in the picker, e.g. "EN 1838 (Europe)".
    pub label: String,
    /// One paragraph under the picker: what it checks.
    pub note: String,
    /// The standard's own words for kinds of place, by place key
    /// (`corridor`, `stairs`, `crowded`, `other`); missing = the general
    /// labels.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub places: std::collections::BTreeMap<String, String>,
}

impl ZonalEmergency {
    /// Name of a standard in the picker (English when this locale lacks it).
    pub fn std_label(&self, id: &str) -> String {
        self.std_text(id).label
    }

    /// Note of a standard (English when this locale lacks it).
    pub fn std_note(&self, id: &str) -> String {
        self.std_text(id).note
    }

    /// A standard's own word for a kind of place (English when this
    /// locale lacks it; `None` = use the general label).
    pub fn std_place(&self, id: &str, place: &str) -> Option<String> {
        self.standards
            .get(id)
            .and_then(|t| t.places.get(place).cloned())
            .or_else(|| {
                default_emergency_standards()
                    .remove(id)
                    .and_then(|t| t.places.get(place).cloned())
            })
    }

    fn std_text(&self, id: &str) -> EmergencyStandardText {
        let own = self.standards.get(id).cloned().unwrap_or_default();
        let en = default_emergency_standards().remove(id).unwrap_or_default();
        EmergencyStandardText {
            label: if own.label.is_empty() {
                en.label
            } else {
                own.label
            },
            note: if own.note.is_empty() {
                en.note
            } else {
                own.note
            },
            places: if own.places.is_empty() {
                en.places
            } else {
                own.places
            },
        }
    }
}

/// English names and notes of the emergency-lighting standards.
fn default_emergency_standards() -> std::collections::BTreeMap<String, EmergencyStandardText> {
    let t = |label: &str, note: &str, places: &[(&str, &str)]| EmergencyStandardText {
        label: label.into(),
        note: note.into(),
        places: places
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    };
    [
        ("en1838", t("EN 1838:2013 (Europe)", "Escape routes up to 2 m wide: at least 1 lx on the centre line and 0.5 lx over the central half, Emax / Emin ≤ 40. Open areas over 60 m²: 0.5 lx without a 0.5 m border, ≤ 40. 5 lx vertical at fire equipment, first aid and call points; glare limits; Ra ≥ 40. Valid until 18 June 2027.", &[])),
        ("en1838-2024", t("EN 1838:2024 (Europe)", "Escape routes: at least 1 lx over the width less a strip of min(W / 4, 0.5 m) along each side (up to 2 m: the central half), Emax / Emin ≤ 40, at least two luminaires per route and open area. Open areas over 60 m² as before; 5 lx vertical also at door releases, escape plans and evacuation aids; glare limits; Ra ≥ 40.", &[])),
        ("bs5266", t("BS 5266-1:2025 (United Kingdom)", "The values of BS EN 1838:2024 (1 lx over the route less the edge strips, open areas 0.5 lx, 5 lx at equipment). Duration 3 h for sleeping accommodation, entertainment venues, phased evacuation or early re-occupation; 1 h only with immediate evacuation.", &[])),
        ("cte-sua4", t("CTE DB SUA 4 / REBT (Spain)", "Escape routes up to 2 m: 1 lx on the axis, 0.5 lx over the central half, Emax / Emin ≤ 40. Safety equipment and distribution boards: 5 lx horizontal. Anti-panic (REBT ITC-BT-28): 0.5 lx in the whole space up to 1 m, any size, no border. Zero reflectance, Ra ≥ 40, 1 h.", &[])),
        ("fr-erp", t("ERP, arrêté du 25 juin 1980 (France)", "No lux values: evacuation luminaires at most 15 m apart along circulation routes, 45 lm or more each, two or more on routes over 15 m; ambient lighting in rooms for 100 people or more: 5 lm per m², neighbours at most 4 mounting heights apart, two or more per room. Illuminance is shown for information.", &[])),
        ("sp52", t("SP 52.13330.2016 (Russia)", "Escape routes up to 2 m: 1 lx on the axis, 0.5 lx over half the width, Emin / Emax ≥ 1 : 40. 5 lx on the floor in front of every exit and at equipment and evacuation plans. Anti-panic over 60 m² (and 30 people or more): 0.5 lx without a 0.5 m border. Glare limits, Ra ≥ 40; 100 % within 10 s, 1 h.", &[])),
        ("nfpa101", t("NFPA 101 / IBC (United States)", "Along the path of egress at floor level: average at least 1 fc (10.8 lx), at any point at least 0.1 fc (1.1 lx), max / min ≤ 40, over the route width and the room's floor. After 1.5 h the values may fall to 6.5 lx average and 0.65 lx minimum: enter the output then. Externally lit exit signs 5 fc (54 lx).", &[])),
        ("nbcc", t("NBC Canada 2020", "At floor or tread level: an average of at least 10 lx and a minimum of 1 lx over exits, public corridors, principal routes and the listed rooms; no uniformity ratio. Duration 30 min, 1 h (care, treatment, detention) or 2 h (high buildings).", &[])),
        ("nbr10898", t("ABNT NBR 10898 (Brazil)", "Horizontal on the floor: at least 3 lx on flat areas, 5 lx where the level changes (stairs, ramps, obstacles), max / min ≤ 20 along the route. Duration at least 1 h.", &[("corridor", "Flat area (corridor, hall, refuge)"), ("crowded", "Flat area (room)"), ("stairs", "Level change (stairs, ramp, obstacle)")])),
        ("gb55037", t("GB 55037-2022 (China)", "Minimum horizontal illuminance on the floor by place (GB 55037-2022 §10.1.10): stairs, front rooms and refuge spaces 10 lx, evacuation corridors and crowded places 3 lx, other places 1 lx; routes over the central half of their width, rooms over the whole floor, no uniformity ratio. Durations and switch-over times (GB 51309-2018) are not calculated.", &[])),
        ("jp-bsl", t("Building Standard Law (Japan)", "Direct light only: at least 2 lx on the floor at every point of the room and its escape corridors and stairs, corners included (fluorescent and LED sources; 1 lx for incandescent lamps). 30 min.", &[])),
        ("nbc-in", t("NBC India 2016", "Escape routes up to 2 m: at least 10 lx on the centre line and 5 lx over half the width, no uniformity ratio; rooms checked at 5 lx. Duration 1.5 h, on within 5 s.", &[])),
        ("as2293", t("AS/NZS 2293.1:2018 (Australia)", "Escape routes: at least 0.2 lx and an average of 0.5 lx on the floor, stairs 1 lx; measured away from the walls. From public summaries of the 2018 edition (the 2026 edition is not yet checked). 90 min in service.", &[])),
        ("nz-f6", t("NZBC F6/AS1 (New Zealand)", "Direct illuminance on the floor: 1 lx in exitways and at changes of level, 0.2 lx everywhere else; points within 0.5 m of walls left out. 90 min.", &[("corridor", "Exitway"), ("stairs", "Change of level"), ("other", "Elsewhere on the escape route")])),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

impl Default for ZonalEmergency {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            title: s("Emergency lighting"),
            intro: s("Escape routes, open areas and safety equipment, lit by the emergency luminaires alone: direct light, emergency flux, maintenance factor."),
            enable: s("Plan emergency lighting"),
            luminaire: s("Emergency luminaire"),
            lum_demo_route: s("Demo: escape-route optic (180 lm)"),
            lum_demo_open: s("Demo: anti-panic optic (220 lm)"),
            lum_loaded: s("Loaded luminaire: {name}"),
            lum_room: s("The room's luminaires in emergency mode"),
            flux: s("Emergency flux (lm)"),
            flux_auto: s("from the file: {lm} lm"),
            pct: s("Emergency flux (% of normal)"),
            height: s("Mounting height (m)"),
            mf: s("Maintenance factor"),
            spacing_note: s("At {h} m: escape route every {r} m, open area grid {g} m (from the luminaire's photometry)."),
            routes: s("Escape routes"),
            draw_route: s("✏️ Draw an escape route"),
            route_hint: s("Click the points of the route's centre line; double-click or “Finish” ends it."),
            finish: s("Finish"),
            cancel: s("Cancel"),
            width: s("Width (m)"),
            route_item: s("Route {n}: {len} m, {w} m wide"),
            remove: s("Remove"),
            open_area: s("Room as open (anti-panic) area"),
            open_area_hint: s("Required above {limit} m²; the room has {a} m²."),
            points: s("Points of emphasis"),
            place: s("Place on the plan:"),
            point_hint: s("Click on the plan next to the wall the equipment is on (it faces into the room)."),
            done: s("Done"),
            kind_fire: s("Fire equipment"),
            kind_first_aid: s("First aid"),
            kind_call: s("Call point"),
            kind_sign: s("Externally lit sign"),
            kind_exit: s("Exit"),
            point_item: s("{kind} at ({x}, {y})"),
            results_title: s("Emergency lighting"),
            pass: s("✓ All requirements met"),
            fail: s("✗ Not all requirements met"),
            count: s("{n} emergency luminaires, {lm} lm each, at {h} m"),
            col_what: s("Check"),
            col_value: s("Value"),
            col_required: s("Required"),
            route_centre: s("{name}: centre line min"),
            route_band: s("{name}: central band min"),
            route_ratio: s("{name}: Emax / Emin"),
            point_vertical: s("{kind}: vertical"),
            glare: s("Glare 60°–90° (highest)"),
            ra: s("Colour rendering Ra"),
            ra_unknown: s("unknown"),
            nothing: s("Draw an escape route, keep the room as open area, or place safety equipment to check it."),
            method: s("Direct light only, maintenance factor {mf}; response times and the rated duration are not calculated."),
            move_label: s("Emergency luminaires"),
            standard: s("Standard"),
            place_corridor: s("Evacuation corridor"),
            place_stairs: s("Stairs, front room, refuge"),
            place_crowded: s("Crowded place"),
            place_other: s("Other place"),
            room_place: s("The room is"),
            no_requirement: s("no requirement"),
            view_normal: s("💡 Normal lighting"),
            view_emergency: s("🚨 Emergency mode"),
            floor_legend: s("Floor in emergency mode: direct light of the emergency luminaires, maintenance factor {mf}; min {min} lx, max {max} lx."),
            lum_off: s("○ normal luminaires, off"),
            band_half: s("central half"),
            band_full: s("full width"),
            band_pct: s("central {pct} %"),
            route_name: s("Route {n}"),
            route_band_min: s("{name}: floor min ({band})"),
            route_avg: s("{name}: average ({band})"),
            route_ratio_band: s("{name}: Emax / Emin ({band})"),
            route_width: s("{name}: width"),
            route_width_req: s("≤ {w} m: split into strips or check as an open area"),
            room_place_name: s("Room ({place})"),
            open_name: s("Open area"),
            room_name: s("Room"),
            area_min_of: s("{name}: floor min"),
            area_avg_of: s("{name}: average"),
            area_ratio_of: s("{name}: Emax / Emin"),
            end_avg: s("{name}: average after {h} h ({pct} % output)"),
            end_min: s("{name}: min after {h} h ({pct} % output)"),
            not_required_above: s("not required (≤ {a} m²)"),
            end_output: s("Output at the end of the duration (%)"),
            end_output_hint: s("{std} allows lower values after {h} h: average {avg} lx, minimum {min} lx. Enter the output the luminaires still give then; constant-output LED emergency luminaires keep 100 %."),
            room_floor_hint: s("Checked over the whole floor; the room has {a} m²."),
            report_pdf: s("🚨 Emergency lighting report (PDF) · {std}"),
            point_horizontal: s("{kind}: horizontal"),
            exit_floor: s("{name}: floor 0.5 m in front"),
            lum_flux_min: s("Emergency flux per luminaire (lowest)"),
            count_of: s("{name}: luminaires"),
            route_gap: s("{name}: largest gap between luminaires"),
            area_flux: s("{name}: luminaire flux per m²"),
            area_spacing: s("{name}: nearest luminaire ÷ mounting height"),
            for_information: s("for information"),
            kind_door_release: s("Door release"),
            kind_escape_plan: s("Escape plan"),
            kind_evac_aid: s("Evacuation aid"),
            group_europe: s("Europe"),
            group_americas: s("Americas"),
            group_asia: s("Asia and Pacific"),
            standards: default_emergency_standards(),
        }
    }
}

/// Interior designer: a room from an IFC building model. Missing keys
/// fall back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZonalIfc {
    pub title: String,
    pub intro: String,
    pub open: String,
    pub reading: String,
    pub failed: String,
    pub summary: String,
    pub storey: String,
    pub room: String,
    pub room_info: String,
    pub in_room: String,
    pub use_luminaires: String,
    pub use_photometry: String,
    pub use_exits: String,
    pub apply: String,
    pub applied: String,
    pub fixed_note: String,
    pub back_to_grid: String,
    pub no_photometry: String,
    pub notes: String,
    pub export_ifc: String,
    pub export_ifc_tip: String,
}

impl Default for ZonalIfc {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            title: s("🏢 Room from an IFC model"),
            intro: s("Open a building model (IFC2x3, IFC4) and take a room from it: outline, height, the luminaires in it and the exits. The file is read in your browser."),
            open: s("Open IFC file…"),
            reading: s("Reading {name}…"),
            failed: s("Could not read this IFC file: {error}"),
            summary: s("{rooms} rooms on {storeys} storeys · {luminaires} luminaires · {doors} doors · {windows} windows"),
            storey: s("Storey"),
            room: s("Room"),
            room_info: s("{area} m² · {height} m high"),
            in_room: s("In this room: {luminaires} luminaires ({with} with photometry) · {doors} doors ({exits} exits) · {windows} windows"),
            use_luminaires: s("Luminaire positions from the model ({n})"),
            use_photometry: s("Open their photometry: {name}"),
            use_exits: s("Exits as emergency points ({n})"),
            apply: s("Use this room"),
            applied: s("Room from the model: {name}"),
            fixed_note: s("{n} luminaires placed as in the IFC model."),
            back_to_grid: s("Use the calculated layout instead"),
            no_photometry: s("The luminaires here carry no photometry; the loaded luminaire is used."),
            notes: s("Notes"),
            export_ifc: s("🏢 IFC model (.ifc)"),
            export_ifc_tip: s("The room as an IFC4 model: outline, walls, luminaires with their photometry (emergency luminaires too) and the exits, for BIM tools, bimifc.de and the import here."),
        }
    }
}

/// Presence sensors in the interior designer: coverage on the plan and the
/// EN 15193-1 energy estimate. Missing keys fall back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZonalSensors {
    pub title: String,
    pub intro: String,
    pub sensor: String,
    pub none: String,
    pub load_demo: String,
    pub no_files: String,
    pub placement: String,
    pub place_auto: String,
    pub place_grid: String,
    pub place_luminaires: String,
    pub along_length: String,
    pub along_width: String,
    pub plan_for: String,
    pub show_all: String,
    pub orientation: String,
    pub margin: String,
    pub mounting: String,
    pub auto_result: String,
    pub auto_failed: String,
    pub col_motion: String,
    pub col_plane: String,
    pub col_covered: String,
    pub col_overlap: String,
    pub col_diameter: String,
    pub plane_floor: String,
    pub plane_workplane: String,
    pub not_detected: String,
    pub summary: String,
    pub legend_gap: String,
    pub legend_area: String,
    pub legend_sensor: String,
    pub energy_title: String,
    pub control: String,
    /// Labels of the six control types (EN 15193-1 Annex E, F_OC table order).
    pub controls: Vec<String>,
    pub space: String,
    /// Labels of the nine space types (F_A table order).
    pub spaces: Vec<String>,
    pub hold_time: String,
    pub hours: String,
    pub hours_note: String,
    pub power: String,
    pub manual: String,
    pub with_sensors: String,
    pub saving: String,
    pub energy: String,
    pub cond_hold: String,
    pub cond_group: String,
    pub cond_coverage: String,
    pub not_credited: String,
    pub method: String,
    pub outline_hint: String,
    pub outline_area: String,
    pub move_title: String,
    pub move_luminaires: String,
    pub move_poles: String,
    pub move_sensors: String,
    pub moved: String,
    pub reset_moved: String,
    pub drag_hint: String,
    pub spacing_moved: String,
    pub outdoor_intro: String,
    pub on_poles: String,
    pub sensor_height: String,
    pub plane_ground: String,
    pub outdoor_method: String,
}

impl Default for ZonalSensors {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            title: s("Presence sensors"),
            intro: s("Place detectors in the room, see what they detect and estimate the energy saved with presence control (EN 15193-1)."),
            sensor: s("Sensor"),
            none: s("No sensors"),
            load_demo: s("Load demo sensors"),
            no_files: s("Open or drop sensor files (sensor LDT, SensXML, GLDF), or load the demo sensors."),
            placement: s("Placement"),
            place_auto: s("Automatic (fewest sensors)"),
            place_grid: s("Grid"),
            place_luminaires: s("One per luminaire"),
            along_length: s("Along the length"),
            along_width: s("Along the width"),
            plan_for: s("Plan for"),
            show_all: s("Show all motion types"),
            orientation: s("C0 direction"),
            margin: s("Wall margin"),
            mounting: s("Sensors in the ceiling at {h} m"),
            auto_result: s("{n} sensors ({nx} × {ny}) detect ≥ {pct} % of the area"),
            auto_failed: s("No grid of up to {n} sensors reaches {pct} %; showing {n} sensors."),
            col_motion: s("Motion"),
            col_plane: s("Plane"),
            col_covered: s("Covered"),
            col_overlap: s("Overlap"),
            col_diameter: s("Ø per sensor"),
            plane_floor: s("floor"),
            plane_workplane: s("workplane"),
            not_detected: s("not detected"),
            summary: s("{n} sensors · {a} m² floor area per sensor"),
            legend_gap: s("not detected"),
            legend_area: s("evaluated area"),
            legend_sensor: s("sensor"),
            energy_title: s("Energy estimate (EN 15193-1)"),
            control: s("Control"),
            controls: [
                "Manual on/off",
                "Manual on/off + automatic sweep-off",
                "Auto on / dimmed",
                "Auto on / auto off",
                "Manual on / dimmed",
                "Manual on / auto off",
            ]
            .map(String::from)
            .to_vec(),
            space: s("Space type"),
            spaces: [
                "Office building, overall",
                "Cellular office, 1 person",
                "Cellular office, 2–6 persons",
                "Open-plan office",
                "Office corridor (dimmed)",
                "Conference room",
                "Classroom",
                "Toilet",
                "Car park, public",
            ]
            .map(String::from)
            .to_vec(),
            hold_time: s("Lights off within 15 min of the last presence"),
            hours: s("Operating hours per year"),
            hours_note: s("Assumed value; enter the hours of your building."),
            power: s("Installed power"),
            manual: s("Manual switching"),
            with_sensors: s("With presence control"),
            saving: s("Saving"),
            energy: s("Energy"),
            cond_hold: s("Hold time ≤ 15 min"),
            cond_group: s("Control group ≤ 30 m² ({a} m² per sensor)"),
            cond_coverage: s("Detection covers ≥ {min} % of the area ({pct} %)"),
            not_credited: s("Not all conditions are met: counted as manual switching."),
            method: s("Coverage: the detection area of each sensor at the mounting height, cut at the floor (walking) and at the workplane (seated presence); wall margin excluded, walls stop detection. F_O from EN 15193-1 Annex E (F_OC by control type, F_A by space type; open plan interpolated between 10 and 30 m² per sensor), one control group per sensor, without daylight or constant-illuminance factors."),
            move_title: s("Move on the plan"),
            move_luminaires: s("Luminaires"),
            move_poles: s("Poles"),
            move_sensors: s("Sensors"),
            moved: s("{n} moved"),
            reset_moved: s("Reset positions"),
            drag_hint: s("Drag a symbol on the plan to move it."),
            spacing_moved: s("moved by hand"),
            outdoor_intro: s("Place motion detectors in the area and see what they detect on the ground."),
            on_poles: s("One per pole"),
            sensor_height: s("Sensor height"),
            plane_ground: s("ground"),
            outdoor_method: s("Coverage: the detection area of each sensor at the sensor height, cut at the ground; one height for all sensors, nothing in the area blocks detection. No energy estimate: EN 15193-1 covers buildings."),
            outline_hint: s("Click the corners on the plan (snaps to 10 cm and to straight lines); click the first corner again to close the room."),
            outline_area: s("Room outline: {a} m² floor, {p} m of walls"),
        }
    }
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

/// Projects on the dashboard (named sets of luminaires and sensors).
/// Missing keys fall back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardProjects {
    pub project: String,
    pub demo: String,
    pub untitled: String,
    pub my_files: String,
    pub new: String,
    pub save_as: String,
    pub copy_suffix: String,
    pub export: String,
    pub import: String,
    pub delete: String,
    pub confirm_delete: String,
    pub demo_note: String,
    pub empty: String,
    pub import_error: String,
    pub counts: String,
    pub sensors_title: String,
    pub luminaires_title: String,
    #[serde(default = "dp_scenes_title")]
    pub scenes_title: String,
    pub sensor_ldt: String,
    pub sensxml: String,
    pub range: String,
    pub remove: String,
    pub clear_luminaires: String,
    pub name_hint: String,
    pub open_sensor: String,
    pub compare_hint: String,
    pub compare_selected: String,
}

impl Default for DashboardProjects {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            project: s("Project"),
            demo: s("Demo project"),
            untitled: s("Untitled project"),
            my_files: s("My files"),
            new: s("New"),
            save_as: s("Save as copy"),
            copy_suffix: s("copy"),
            export: s("Export"),
            import: s("Import…"),
            delete: s("Delete"),
            confirm_delete: s("Delete “{name}”? Click again."),
            demo_note: s("The demo project is read-only: opening a file starts a new project."),
            empty: s("This project is empty: open or drop luminaire and sensor files (LDT, IES, ATLA, GLDF, SensXML)."),
            import_error: s("{name} is not a project file."),
            counts: s("{l} luminaires · {s} sensors"),
            sensors_title: s("Sensors"),
            luminaires_title: s("Luminaires"),
            scenes_title: dp_scenes_title(),
            sensor_ldt: s("sensor LDT"),
            sensxml: s("SensXML"),
            range: s("max {r} m at {h} m"),
            remove: s("Remove"),
            clear_luminaires: s("Remove all luminaires ({n})"),
            name_hint: s("Project name"),
            open_sensor: s("All sensor views"),
            compare_hint: s("Click a sensor for its diagrams · Ctrl/⌘ + click a second one, then Compare"),
            compare_selected: s("⇄ Compare {a} with {b}"),
        }
    }
}

/// Dashboard translations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DashboardLocale {
    #[serde(default)]
    pub guide: DashboardGuide,
    #[serde(default)]
    pub projects: DashboardProjects,
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
    #[serde(default = "default_slab_globe_pick")]
    pub globe_pick: String,
    #[serde(default = "default_slab_globe_title")]
    pub globe_title: String,
    #[serde(default = "default_slab_globe_hint")]
    pub globe_hint: String,
    #[serde(default = "default_slab_globe_close")]
    pub globe_close: String,
    #[serde(default = "default_slab_moon_card_title")]
    pub moon_card_title: String,
    #[serde(default = "default_slab_moonlight_now")]
    pub moonlight_now: String,
    #[serde(default = "default_slab_moonlight_day")]
    pub moonlight_day: String,
    #[serde(default = "default_slab_moon_summary")]
    pub moon_summary: String,
    #[serde(default = "default_slab_moon_dist_close")]
    pub moon_dist_close: String,
    #[serde(default = "default_slab_moon_dist_far")]
    pub moon_dist_far: String,
    #[serde(default = "default_slab_moon_times")]
    pub moon_times: String,
    #[serde(default = "default_slab_moon_hours")]
    pub moon_hours: String,
    #[serde(default = "default_slab_moon_dark")]
    pub moon_dark: String,
    #[serde(default = "default_slab_moon_ref_note")]
    pub moon_ref_note: String,
    #[serde(default = "default_slab_moon_legend_total")]
    pub moon_legend_total: String,
    #[serde(default = "default_slab_moon_legend_moon")]
    pub moon_legend_moon: String,
    #[serde(default = "default_slab_moon_month_title")]
    pub moon_month_title: String,
    #[serde(default = "default_slab_moon_month_hint")]
    pub moon_month_hint: String,
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
fn default_slab_moon_card_title() -> String {
    "🌙 Moonlight tonight".into()
}
fn default_slab_moonlight_now() -> String {
    "Moonlight {lx} lx".into()
}
fn default_slab_moonlight_day() -> String {
    "Moon up · daylight dominates".into()
}
fn default_slab_moon_summary() -> String {
    "{phase} {pct} % · {km} km ({dist})".into()
}
fn default_slab_moon_dist_close() -> String {
    "close: {pct} % brighter than at mean distance".into()
}
fn default_slab_moon_dist_far() -> String {
    "far: {pct} % dimmer than at mean distance".into()
}
fn default_slab_moon_times() -> String {
    "Moonrise {rise} · moonset {set} · up to {max} lx at {at}".into()
}
fn default_slab_moon_hours() -> String {
    "The moon alone gives 0.2 lx or more for {h2} h and 0.1 lx or more for {h1} h of the dark hours.".into()
}
fn default_slab_moon_dark() -> String {
    "The moon stays below the horizon while it is dark.".into()
}
fn default_slab_moon_ref_note() -> String {
    "Horizontal illuminance on open ground, clear sky. Reference lines: 0.2 lx = escape-route minimum of AS/NZS 2293 and NZBC F6 · 0.5 lx = EN 1838 open areas · 1 lx = EN 1838 escape routes · 2 lx = EN 13201 P6 average. Clouds, trees and buildings take much of it away.".into()
}
fn default_slab_moon_legend_total() -> String {
    "all natural light".into()
}
fn default_slab_moon_legend_moon() -> String {
    "moon alone".into()
}
fn default_slab_moon_month_title() -> String {
    "Moonlight over the next 30 nights (the most per night)".into()
}
fn default_slab_moon_month_hint() -> String {
    "Click a night to show it above.".into()
}
fn default_slab_globe_pick() -> String {
    "🌍 Pick on the globe".into()
}
fn default_slab_globe_title() -> String {
    "Pick a place".into()
}
fn default_slab_globe_hint() -> String {
    "Search a city or click the globe, then “Use this place”: its latitude, longitude and time zone are taken over.".into()
}
fn default_slab_globe_close() -> String {
    "Close".into()
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
                ("globe_pick", &sl.globe_pick),
                ("globe_title", &sl.globe_title),
                ("globe_hint", &sl.globe_hint),
                ("globe_close", &sl.globe_close),
                ("moon_card_title", &sl.moon_card_title),
                ("moonlight_now", &sl.moonlight_now),
                ("moonlight_day", &sl.moonlight_day),
                ("moon_summary", &sl.moon_summary),
                ("moon_dist_close", &sl.moon_dist_close),
                ("moon_dist_far", &sl.moon_dist_far),
                ("moon_times", &sl.moon_times),
                ("moon_hours", &sl.moon_hours),
                ("moon_dark", &sl.moon_dark),
                ("moon_ref_note", &sl.moon_ref_note),
                ("moon_legend_total", &sl.moon_legend_total),
                ("moon_legend_moon", &sl.moon_legend_moon),
                ("moon_month_title", &sl.moon_month_title),
                ("moon_month_hint", &sl.moon_month_hint),
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

    /// Raw locale JSON (not the deserialised struct: serde defaults would
    /// hide a key that is missing and silently show English).
    fn raw_locale(code: &str) -> serde_json::Value {
        let text = match code {
            "en" => include_str!("../locales/en.json"),
            "de" => include_str!("../locales/de.json"),
            "fr" => include_str!("../locales/fr.json"),
            "es" => include_str!("../locales/es.json"),
            "it" => include_str!("../locales/it.json"),
            "ru" => include_str!("../locales/ru.json"),
            "pt-BR" => include_str!("../locales/pt-BR.json"),
            "zh" => include_str!("../locales/zh.json"),
            other => panic!("unknown locale {other}"),
        };
        serde_json::from_str(text).expect("locale JSON")
    }

    fn key_paths(v: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
        if let serde_json::Value::Object(map) = v {
            for (k, child) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                key_paths(child, &path, out);
            }
        } else {
            out.push(prefix.to_string());
        }
    }

    fn at<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
        path.split('.').try_fold(v, |cur, k| cur.get(k))
    }

    const OTHER_LOCALES: [&str; 7] = ["de", "fr", "es", "it", "ru", "pt-BR", "zh"];

    /// Every key of en.json exists in every other locale file.
    #[test]
    fn every_locale_has_every_key() {
        let en = raw_locale("en");
        let mut paths = Vec::new();
        key_paths(&en, "", &mut paths);
        assert!(paths.len() > 1000, "{} keys", paths.len());
        for code in OTHER_LOCALES {
            let loc = raw_locale(code);
            let missing: Vec<&String> = paths.iter().filter(|p| at(&loc, p).is_none()).collect();
            assert!(missing.is_empty(), "{code}: missing {missing:?}");
        }
    }

    /// The colour-quality card, the sensor tab and their dashboard tiles:
    /// present, non-empty, placeholders kept, and the sentences translated.
    #[test]
    fn colour_quality_and_sensor_strings_translated() {
        let en = raw_locale("en");
        // (block path, keys that must differ from English, (key, placeholder))
        type Block<'a> = (&'a str, &'a [&'a str], &'a [(&'a str, &'a str)]);
        let blocks: [Block; 5] = [
            (
                "dashboard.projects",
                &[
                    "project",
                    "demo",
                    "untitled",
                    "my_files",
                    "new",
                    "save_as",
                    "export",
                    "delete",
                    "confirm_delete",
                    "demo_note",
                    "empty",
                    "import_error",
                    "counts",
                    "sensors_title",
                    "remove",
                    "clear_luminaires",
                    "name_hint",
                    "open_sensor",
                    "compare_hint",
                    "compare_selected",
                ],
                &[
                    ("confirm_delete", "{name}"),
                    ("import_error", "{name}"),
                    ("counts", "{l}"),
                    ("counts", "{s}"),
                    ("range", "{r}"),
                    ("range", "{h}"),
                    ("clear_luminaires", "{n}"),
                    ("compare_selected", "{a}"),
                    ("compare_selected", "{b}"),
                ],
            ),
            (
                "ui.sensor_compare",
                &[
                    "title",
                    "intro",
                    "need_two",
                    "mount",
                    "workplane",
                    "view_plan",
                    "view_height",
                    "view_room",
                    "motion",
                    "plan_note",
                    "angular_note",
                    "quality_note",
                    "height_x",
                    "height_y",
                    "measured",
                    "height_note",
                    "room_length",
                    "room_width",
                    "plan_for",
                    "price",
                    "row_covered",
                    "row_m2",
                    "row_cost",
                    "turned",
                    "not_reached",
                    "room_note",
                    "share",
                    "link_note",
                    "method",
                ],
                &[],
            ),
            (
                "ui.colour_quality",
                &[
                    "subtab",
                    "title",
                    "cvg_title",
                    "cvg_text",
                    "cvg_note",
                    "duv_title",
                    "black_body_line",
                    "measured",
                    "cct",
                    "cri_limit",
                    "method",
                    "ri_title",
                    "download",
                ],
                &[("cvg_note", "{rg}"), ("cri_limit", "{limit}")],
            ),
            (
                "ui.sensor",
                &[
                    "title",
                    "intro",
                    "demo",
                    "demo_note",
                    "derived",
                    "height_hint",
                    "nothing_detected",
                    "not_detected",
                    "planning",
                    "tangential",
                    "radial",
                    "presence",
                    "volume_ends",
                    "report",
                    "recognised",
                    "open_as_luminaire",
                ],
                &[
                    ("volume_ends", "{d}"),
                    ("derived", "{h}"),
                    ("recognised", "{name}"),
                ],
            ),
            (
                "zonal_designer.sensors",
                &[
                    "title",
                    "intro",
                    "no_files",
                    "placement",
                    "place_auto",
                    "along_length",
                    "along_width",
                    "plan_for",
                    "show_all",
                    "margin",
                    "mounting",
                    "auto_result",
                    "auto_failed",
                    "not_detected",
                    "summary",
                    "legend_area",
                    "energy_title",
                    "controls",
                    "spaces",
                    "hold_time",
                    "hours",
                    "hours_note",
                    "power",
                    "manual",
                    "with_sensors",
                    "saving",
                    "cond_hold",
                    "cond_group",
                    "cond_coverage",
                    "not_credited",
                    "method",
                    "move_title",
                    "move_poles",
                    "reset_moved",
                    "drag_hint",
                    "spacing_moved",
                    "outdoor_intro",
                    "on_poles",
                    "sensor_height",
                    "plane_ground",
                    "outdoor_method",
                    "outline_hint",
                    "outline_area",
                ],
                &[
                    ("outline_area", "{a}"),
                    ("outline_area", "{p}"),
                    ("moved", "{n}"),
                    ("mounting", "{h}"),
                    ("auto_result", "{n}"),
                    ("auto_result", "{nx}"),
                    ("auto_result", "{ny}"),
                    ("auto_result", "{pct}"),
                    ("auto_failed", "{n}"),
                    ("auto_failed", "{pct}"),
                    ("summary", "{n}"),
                    ("summary", "{a}"),
                    ("cond_group", "{a}"),
                    ("cond_coverage", "{min}"),
                    ("cond_coverage", "{pct}"),
                ],
            ),
        ];
        for code in OTHER_LOCALES {
            let loc = raw_locale(code);
            for (block, must_differ, placeholders) in blocks {
                let en_block = at(&en, block).and_then(|v| v.as_object()).unwrap();
                let loc_block = at(&loc, block)
                    .and_then(|v| v.as_object())
                    .unwrap_or_else(|| panic!("{code}: {block} missing"));
                for (key, en_val) in en_block {
                    let val = loc_block
                        .get(key)
                        .unwrap_or_else(|| panic!("{code}: {block}.{key} missing"));
                    match (en_val, val) {
                        (serde_json::Value::String(_), serde_json::Value::String(s)) => {
                            assert!(!s.trim().is_empty(), "{code}: {block}.{key} empty")
                        }
                        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                            assert_eq!(a.len(), b.len(), "{code}: {block}.{key} length");
                            assert!(
                                b.iter()
                                    .all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty())),
                                "{code}: {block}.{key} has empty entries"
                            );
                        }
                        _ => panic!("{code}: {block}.{key} has the wrong type"),
                    }
                }
                for key in must_differ {
                    assert_ne!(
                        loc_block[*key], en_block[*key],
                        "{code}: {block}.{key} not translated (still English)"
                    );
                }
                for (key, ph) in placeholders {
                    let s = loc_block[*key].as_str().unwrap();
                    assert!(s.contains(ph), "{code}: {block}.{key} lost {ph}: {s}");
                }
            }
            // 16 hue names for the TM-30 bins, at least half of them translated.
            let hues = at(&loc, "ui.colour_quality.hues")
                .unwrap()
                .as_array()
                .unwrap();
            let en_hues = at(&en, "ui.colour_quality.hues")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(hues.len(), 16, "{code}: hues");
            let translated = hues.iter().zip(en_hues).filter(|(a, b)| a != b).count();
            assert!(
                translated >= 8,
                "{code}: only {translated}/16 hue names translated"
            );
            // Hue-shift legend now says radians, the dashboard tiles exist.
            for path in [
                "ui.spectral.hue_legend.rhs",
                "dashboard.features.colour_quality_title",
                "dashboard.features.colour_quality_desc",
                "dashboard.features.sensor_title",
                "dashboard.features.sensor_desc",
            ] {
                let v = at(&loc, path).and_then(|v| v.as_str()).unwrap_or("");
                assert!(!v.trim().is_empty(), "{code}: {path} empty");
                assert_ne!(
                    Some(v),
                    at(&en, path).and_then(|v| v.as_str()),
                    "{code}: {path} not translated"
                );
            }
        }
        // The deserialised struct sees the same values (no silent fallback).
        let de = Locale::for_code("de");
        assert_eq!(de.ui.colour_quality.title, "Farbqualität");
        assert_eq!(de.ui.colour_quality.hues.len(), 16);
        assert_eq!(de.dashboard.features.colour_quality_title, "Farbqualität");
        assert_eq!(de.zonal_designer.sensors.title, "Präsenzmelder");
        assert_eq!(de.zonal_designer.sensors.controls.len(), 6);
        assert_eq!(de.zonal_designer.sensors.spaces.len(), 9);
        // English struct defaults equal en.json (the fallback is the same text).
        let en_loc = Locale::for_code("en");
        assert_eq!(en_loc.zonal_designer.sensors, ZonalSensors::default());
        assert_eq!(en_loc.ui.sensor_compare, UiSensorCompare::default());
        assert_eq!(en_loc.dashboard.projects, DashboardProjects::default());
        assert_eq!(de.ui.sensor_compare.title, "Sensoren vergleichen");
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
    #[serde(default = "ls_mel_title")]
    pub mel_title: String,
    #[serde(default = "ls_mel_values")]
    pub mel_values: String,
    #[serde(default = "ls_mel_day_ok")]
    pub mel_day_ok: String,
    #[serde(default = "ls_mel_day_low")]
    pub mel_day_low: String,
    #[serde(default = "ls_mel_no_spd")]
    pub mel_no_spd: String,
    #[serde(default = "ls_mel_note")]
    pub mel_note: String,
    #[serde(default = "ls_mel_card_der")]
    pub mel_card_der: String,
    #[serde(default = "ls_mel_card_edi")]
    pub mel_card_edi: String,
    #[serde(default = "ls_mel_card_need")]
    pub mel_card_need: String,
    #[serde(default = "ls_mel_card_estimate")]
    pub mel_card_estimate: String,
    #[serde(default = "ls_dli")]
    pub dli: String,
    #[serde(default = "ls_dli_explain")]
    pub dli_explain: String,
    #[serde(default = "ls_hint_ppfd")]
    pub hint_ppfd: String,
    #[serde(default = "ls_unit_ppfd")]
    pub unit_ppfd: String,
    #[serde(default = "ls_step_sensors")]
    pub step_sensors: String,
    #[serde(default = "ls_sensors_desc")]
    pub sensors_desc: String,
    #[serde(default = "ls_sensor_file")]
    pub sensor_file: String,
    #[serde(default = "ls_sensor_none")]
    pub sensor_none: String,
    #[serde(default = "ls_sensor_load_demo")]
    pub sensor_load_demo: String,
    #[serde(default = "ls_sensor_count")]
    pub sensor_count: String,
    #[serde(default = "ls_sensor_height")]
    pub sensor_height: String,
    #[serde(default = "ls_sensor_motion")]
    pub sensor_motion: String,
    #[serde(default = "ls_control_together")]
    pub control_together: String,
    #[serde(default = "ls_control_nearest")]
    pub control_nearest: String,
    #[serde(default = "ls_idle_level")]
    pub idle_level: String,
    #[serde(default = "ls_idle_off")]
    pub idle_off: String,
    #[serde(default = "ls_move_objects")]
    pub move_objects: String,
    #[serde(default = "ls_move_hint")]
    pub move_hint: String,
    #[serde(default = "ls_reset_moves")]
    pub reset_moves: String,
    #[serde(default = "ls_status_detected")]
    pub status_detected: String,
    #[serde(default = "ls_status_idle")]
    pub status_idle: String,
    /// Checkbox: the sensors switch the luminaires.
    #[serde(default = "ls_sensor_dim")]
    pub sensor_dim: String,
    /// Under it, while off.
    #[serde(default = "ls_sensor_dim_off_note")]
    pub sensor_dim_off_note: String,
    /// Detection pill while the sensors only detect.
    #[serde(default = "ls_status_dim_off")]
    pub status_dim_off: String,
    #[serde(default = "ls_status_person")]
    pub status_person: String,
    #[serde(default = "ls_status_cars")]
    pub status_cars: String,
    #[serde(default = "ls_status_no_area")]
    pub status_no_area: String,
    #[serde(default = "ls_sensor_area")]
    pub sensor_area: String,
    #[serde(default = "ls_quality")]
    pub quality: String,
    #[serde(default = "ls_quality_quick")]
    pub quality_quick: String,
    #[serde(default = "ls_quality_balanced")]
    pub quality_balanced: String,
    #[serde(default = "ls_quality_high")]
    pub quality_high: String,
    #[serde(default = "ls_quality_desc_quick")]
    pub quality_desc_quick: String,
    #[serde(default = "ls_quality_desc_balanced")]
    pub quality_desc_balanced: String,
    #[serde(default = "ls_quality_desc_high")]
    pub quality_desc_high: String,
    #[serde(default = "ls_gpu_reset_note")]
    pub gpu_reset_note: String,
    #[serde(default = "ls_gpu_gone_note")]
    pub gpu_gone_note: String,
    #[serde(default = "ls_auto_quick")]
    pub auto_quick: String,
    #[serde(default = "ls_denoise")]
    pub denoise: String,
    #[serde(default = "ls_scene_planned")]
    pub scene_planned: String,
    #[serde(default = "ls_from_interior")]
    pub from_interior: String,
    #[serde(default = "ls_from_outdoor")]
    pub from_outdoor: String,
    #[serde(default = "ls_from_street")]
    pub from_street: String,
    #[serde(default = "ls_desc_planned")]
    pub desc_planned: String,
    #[serde(default = "ls_open_in_light_scene")]
    pub open_in_light_scene: String,
    #[serde(default = "ls_open_in_light_scene_tip")]
    pub open_in_light_scene_tip: String,
    #[serde(default = "ls_planned_mixed_note")]
    pub planned_mixed_note: String,
    #[serde(default = "ls_save_scene")]
    pub save_scene: String,
    #[serde(default = "ls_scene_name")]
    pub scene_name: String,
    #[serde(default = "ls_scene_saved")]
    pub scene_saved: String,
    #[serde(default = "ls_scene_unsaved")]
    pub scene_unsaved: String,
    #[serde(default = "ls_sensor_volume_3d")]
    pub sensor_volume_3d: String,
    #[serde(default = "ls_em_title")]
    pub em_title: String,
    #[serde(default = "ls_em_mode")]
    pub em_mode: String,
    #[serde(default = "ls_em_mode_desc")]
    pub em_mode_desc: String,
    #[serde(default = "ls_em_marks")]
    pub em_marks: String,
    #[serde(default = "ls_em_marks_off")]
    pub em_marks_off: String,
    #[serde(default = "ls_em_walk")]
    pub em_walk: String,
    #[serde(default = "ls_em_route")]
    pub em_route: String,
    #[serde(default = "ls_em_walk_at")]
    pub em_walk_at: String,
    #[serde(default = "ls_em_walk_play")]
    pub em_walk_play: String,
    #[serde(default = "ls_em_walk_pause")]
    pub em_walk_pause: String,
    #[serde(default = "ls_em_walk_exit")]
    pub em_walk_exit: String,
    #[serde(default = "ls_em_check_route")]
    pub em_check_route: String,
    #[serde(default = "ls_em_check_open")]
    pub em_check_open: String,
    #[serde(default = "ls_em_check_points")]
    pub em_check_points: String,
    #[serde(default = "ls_em_check_min")]
    pub em_check_min: String,
    #[serde(default = "ls_em_check_ratio")]
    pub em_check_ratio: String,
    #[serde(default = "ls_em_check_avg")]
    pub em_check_avg: String,
    #[serde(default = "ls_moon_title")]
    pub moon_title: String,
    #[serde(default = "ls_moon_on")]
    pub moon_on: String,
    #[serde(default = "ls_moon_desc")]
    pub moon_desc: String,
    #[serde(default = "ls_moon_date")]
    pub moon_date: String,
    #[serde(default = "ls_moon_time")]
    pub moon_time: String,
    #[serde(default = "ls_moon_full")]
    pub moon_full: String,
    #[serde(default = "ls_moon_place")]
    pub moon_place: String,
    #[serde(default = "ls_moon_pick")]
    pub moon_pick: String,
    #[serde(default = "ls_moon_north")]
    pub moon_north: String,
    #[serde(default = "ls_moon_lamps_off")]
    pub moon_lamps_off: String,
    #[serde(default = "ls_moon_readout")]
    pub moon_readout: String,
    #[serde(default = "ls_moon_below")]
    pub moon_below: String,
    #[serde(default = "ls_moon_indoor")]
    pub moon_indoor: String,
    #[serde(default = "ls_sun_readout")]
    pub sun_readout: String,
    #[serde(default = "ls_twilight_readout")]
    pub twilight_readout: String,
    #[serde(default = "ls_night_readout")]
    pub night_readout: String,
    #[serde(default = "ls_em_check_note")]
    pub em_check_note: String,
    #[serde(default = "ls_em_off_hint")]
    pub em_off_hint: String,
    #[serde(default = "ls_em_measure")]
    pub em_measure: String,
    #[serde(default = "ls_em_check_band")]
    pub em_check_band: String,
    #[serde(default = "ls_em_check_stairs")]
    pub em_check_stairs: String,
    #[serde(default = "ls_em_check_room")]
    pub em_check_room: String,
    #[serde(default = "ls_em_adapting")]
    pub em_adapting: String,
    #[serde(default = "ls_em_adapted")]
    pub em_adapted: String,
    #[serde(default = "ls_em_replay")]
    pub em_replay: String,
    #[serde(default = "ls_back_to_designer")]
    pub back_to_designer: String,
}

/// Colour-quality card (TM-30 vector graphic, Duv, CRI) in the spectral
/// view. Every field falls back to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiColourQuality {
    #[serde(default = "cq_subtab")]
    pub subtab: String,
    #[serde(default = "cq_title")]
    pub title: String,
    #[serde(default = "cq_cvg_title")]
    pub cvg_title: String,
    #[serde(default = "cq_cvg_text")]
    pub cvg_text: String,
    #[serde(default = "cq_cvg_note")]
    pub cvg_note: String,
    #[serde(default = "cq_duv_title")]
    pub duv_title: String,
    #[serde(default = "cq_magenta")]
    pub magenta: String,
    #[serde(default = "cq_green")]
    pub green: String,
    #[serde(default = "cq_black_body_line")]
    pub black_body_line: String,
    #[serde(default = "cq_measured")]
    pub measured: String,
    #[serde(default = "cq_cct")]
    pub cct: String,
    #[serde(default = "cq_duv")]
    pub duv: String,
    #[serde(default = "cq_rf_rg")]
    pub rf_rg: String,
    #[serde(default = "cq_ra")]
    pub ra: String,
    #[serde(default = "cq_r9")]
    pub r9: String,
    #[serde(default = "cq_re")]
    pub re: String,
    #[serde(default = "cq_cri_limit")]
    pub cri_limit: String,
    #[serde(default = "cq_method")]
    pub method: String,
    #[serde(default = "cq_ri_title")]
    pub ri_title: String,
    #[serde(default = "cq_download")]
    pub download: String,
    /// Names of the 16 TM-30 hue bins.
    #[serde(default = "cq_hues")]
    pub hues: Vec<String>,
}

impl Default for UiColourQuality {
    fn default() -> Self {
        Self {
            subtab: cq_subtab(),
            title: cq_title(),
            cvg_title: cq_cvg_title(),
            cvg_text: cq_cvg_text(),
            cvg_note: cq_cvg_note(),
            duv_title: cq_duv_title(),
            magenta: cq_magenta(),
            green: cq_green(),
            black_body_line: cq_black_body_line(),
            measured: cq_measured(),
            cct: cq_cct(),
            duv: cq_duv(),
            rf_rg: cq_rf_rg(),
            ra: cq_ra(),
            r9: cq_r9(),
            re: cq_re(),
            cri_limit: cq_cri_limit(),
            method: cq_method(),
            ri_title: cq_ri_title(),
            download: cq_download(),
            hues: cq_hues(),
        }
    }
}

fn cq_subtab() -> String {
    "Colour quality".into()
}
fn cq_title() -> String {
    "Colour quality".into()
}
fn cq_cvg_title() -> String {
    "TM-30 colour vector graphic.".into()
}
fn cq_cvg_text() -> String {
    "Dashed circle = reference. Outside = more saturated, inside = duller.".into()
}
fn cq_cvg_note() -> String {
    "Rg {rg} is an average over all hues. The shape shows where colours are pushed and pulled."
        .into()
}
fn cq_duv_title() -> String {
    "Duv: distance to the black-body line".into()
}
fn cq_magenta() -> String {
    "magenta".into()
}
fn cq_green() -> String {
    "green".into()
}
fn cq_black_body_line() -> String {
    "black-body line".into()
}
fn cq_measured() -> String {
    "measured".into()
}
fn cq_cct() -> String {
    "CCT".into()
}
fn cq_duv() -> String {
    "Duv".into()
}
fn cq_rf_rg() -> String {
    "TM-30 Rf / Rg".into()
}
fn cq_ra() -> String {
    "CRI Ra".into()
}
fn cq_r9() -> String {
    "CRI R9".into()
}
fn cq_re() -> String {
    "CRI Re (R1–R15)".into()
}
fn cq_cri_limit() -> String {
    "* CRI is only defined for |Duv| < {limit}.".into()
}
fn cq_method() -> String {
    "Calculated from the spectrum: ANSI/IES TM-30-20 (CIE 224:2017) and CIE 13.3.".into()
}
fn cq_ri_title() -> String {
    "Special colour rendering indices R1–R15".into()
}
fn cq_download() -> String {
    "Download card (SVG)".into()
}
fn cq_hues() -> Vec<String> {
    [
        "red",
        "red-orange",
        "orange",
        "yellow-orange",
        "yellow",
        "yellow-green",
        "green",
        "green",
        "cyan",
        "cyan",
        "blue",
        "blue",
        "blue-violet",
        "violet",
        "purple",
        "pink",
    ]
    .map(String::from)
    .to_vec()
}

/// Sensor comparison (Compare tab, "Sensors" mode). Missing keys fall back
/// to English.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSensorCompare {
    pub mode_luminaires: String,
    pub mode_sensors: String,
    pub title: String,
    pub intro: String,
    pub sensor_a: String,
    pub sensor_b: String,
    pub swap: String,
    pub need_two: String,
    pub drop_hint: String,
    pub open_files: String,
    pub mount: String,
    pub workplane: String,
    pub view_plan: String,
    pub view_polar: String,
    pub view_cartesian: String,
    pub view_height: String,
    pub view_room: String,
    pub motion: String,
    pub plan_note: String,
    pub angular_note: String,
    pub col_motion: String,
    pub col_plane: String,
    pub col_range: String,
    pub col_area: String,
    pub delta: String,
    pub not_detected: String,
    pub no_data: String,
    pub quality_note: String,
    pub heights: String,
    pub height_x: String,
    pub height_y: String,
    pub measured: String,
    pub height_note: String,
    pub room_length: String,
    pub room_width: String,
    pub plan_for: String,
    pub price: String,
    pub row_sensors: String,
    pub row_covered: String,
    pub row_overlap: String,
    pub row_m2: String,
    pub row_fo: String,
    pub row_cost: String,
    pub turned: String,
    pub not_reached: String,
    pub room_note: String,
    pub report: String,
    pub share: String,
    pub copied: String,
    pub link_note: String,
    pub method: String,
}

impl Default for UiSensorCompare {
    fn default() -> Self {
        let s = |v: &str| v.to_string();
        Self {
            mode_luminaires: s("Luminaires"),
            mode_sensors: s("Sensors"),
            title: s("Compare sensors"),
            intro: s("Two detectors under the same conditions: the same mounting height, walking on the floor, seated presence on the workplane. Sensor LDT and SensXML can be mixed."),
            sensor_a: s("Sensor A"),
            sensor_b: s("Sensor B"),
            swap: s("Swap"),
            need_two: s("Load at least two sensors: drop sensor files (sensor LDT, SensXML, GLDF) here or anywhere, or load the demo sensors."),
            drop_hint: s("Drop sensor files here (sensor LDT, SensXML, GLDF) or"),
            open_files: s("Open files…"),
            mount: s("Mounting height"),
            workplane: s("Workplane"),
            view_plan: s("Detection areas"),
            view_polar: s("Polar"),
            view_cartesian: s("Cartesian"),
            view_height: s("Range over height"),
            view_room: s("Room check"),
            motion: s("Motion type"),
            plan_note: s("A filled, B dashed; dashed line: the C-plane cut with the reach of both sensors."),
            angular_note: s("Ranges in metres from the sensor to the edge of the detection volume (SensXML: rebuilt from the measured heights)."),
            col_motion: s("Motion"),
            col_plane: s("Plane"),
            col_range: s("Range"),
            col_area: s("Area"),
            delta: s("Δ"),
            not_detected: s("not detected"),
            no_data: s("no data"),
            quality_note: s("Δ = (B − A) / A. (i) interpolated, (e) extrapolated between / beyond the measured heights of a SensXML file."),
            heights: s("Heights"),
            height_x: s("Mounting height (m)"),
            height_y: s("Ø detection area (m)"),
            measured: s("measured height"),
            height_note: s("Diameter of the circle with the same area. A solid, B dashed; circles: measured heights of a SensXML file; a gap: not detected at that height."),
            room_length: s("Length"),
            room_width: s("Width"),
            plan_for: s("Plan for"),
            price: s("Price per sensor"),
            row_sensors: s("Sensors"),
            row_covered: s("Covered"),
            row_overlap: s("Overlap"),
            row_m2: s("Floor area per sensor"),
            row_fo: s("F_O (EN 15193-1)"),
            row_cost: s("Cost"),
            turned: s("turned 90°"),
            not_reached: s("target not reached"),
            room_note: s("Fewest sensors on a regular grid that detect ≥ 95 % of the room (wall margin 0.5 m, both orientations tried); EN 15193-1 with one control group per sensor."),
            report: s("PDF report"),
            share: s("Copy link"),
            copied: s("Copied"),
            link_note: s("Only demo sensors travel in a link; uploaded files stay in your browser."),
            method: s("Both sensors are read under the same conditions: a sensor LDT is cut at mounting height − plane, a SensXML file is read at the mounting height. Data quality and test conditions of the two sources may differ."),
        }
    }
}

/// Sensors tab (presence / motion detectors). Every field falls back to
/// English so an older locale file still renders the panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSensor {
    #[serde(default = "se_tab")]
    pub tab: String,
    #[serde(default = "se_title")]
    pub title: String,
    #[serde(default = "se_intro")]
    pub intro: String,
    #[serde(default = "se_source")]
    pub source: String,
    #[serde(default = "se_none")]
    pub none: String,
    #[serde(default = "se_use_loaded")]
    pub use_loaded: String,
    #[serde(default = "se_loaded_ldt")]
    pub loaded_ldt: String,
    #[serde(default = "se_view")]
    pub view: String,
    #[serde(default = "se_view_plan")]
    pub view_plan: String,
    #[serde(default = "se_view_polar")]
    pub view_polar: String,
    #[serde(default = "se_view_cartesian")]
    pub view_cartesian: String,
    #[serde(default = "se_view_heatmap")]
    pub view_heatmap: String,
    #[serde(default = "se_view_3d")]
    pub view_3d: String,
    #[serde(default = "se_height_ldt")]
    pub height_ldt: String,
    #[serde(default = "se_height_xml")]
    pub height_xml: String,
    #[serde(default = "se_height_hint")]
    pub height_hint: String,
    #[serde(default = "se_planning")]
    pub planning: String,
    #[serde(default = "se_motion")]
    pub motion: String,
    #[serde(default = "se_tangential")]
    pub tangential: String,
    #[serde(default = "se_radial")]
    pub radial: String,
    #[serde(default = "se_presence")]
    pub presence: String,
    #[serde(default = "se_max_range")]
    pub max_range: String,
    #[serde(default = "se_area")]
    pub area: String,
    #[serde(default = "se_diameter")]
    pub diameter: String,
    #[serde(default = "se_quality")]
    pub quality: String,
    #[serde(default = "se_measured")]
    pub measured: String,
    #[serde(default = "se_interpolated")]
    pub interpolated: String,
    #[serde(default = "se_extrapolated")]
    pub extrapolated: String,
    #[serde(default = "se_no_data")]
    pub no_data: String,
    #[serde(default = "se_mounting")]
    pub mounting: String,
    #[serde(default = "se_wall")]
    pub wall: String,
    #[serde(default = "se_ceiling")]
    pub ceiling: String,
    #[serde(default = "se_wall_ceiling")]
    pub wall_ceiling: String,
    #[serde(default = "se_parameters")]
    pub parameters: String,
    #[serde(default = "se_not_applicable")]
    pub not_applicable: String,
    #[serde(default = "se_warnings")]
    pub warnings: String,
    #[serde(default = "se_demo")]
    pub demo: String,
    #[serde(default = "se_demo_note")]
    pub demo_note: String,
    #[serde(default = "se_not_detected")]
    pub not_detected: String,
    #[serde(default = "se_volume_ends")]
    pub volume_ends: String,
    #[serde(default = "se_nothing_detected")]
    pub nothing_detected: String,
    #[serde(default = "se_derived")]
    pub derived: String,
    #[serde(default = "se_report")]
    pub report: String,
    #[serde(default = "se_recognised")]
    pub recognised: String,
    #[serde(default = "se_open_as_luminaire")]
    pub open_as_luminaire: String,
}

impl Default for UiSensor {
    fn default() -> Self {
        Self {
            tab: se_tab(),
            title: se_title(),
            intro: se_intro(),
            source: se_source(),
            none: se_none(),
            use_loaded: se_use_loaded(),
            loaded_ldt: se_loaded_ldt(),
            view: se_view(),
            view_plan: se_view_plan(),
            view_polar: se_view_polar(),
            view_cartesian: se_view_cartesian(),
            view_heatmap: se_view_heatmap(),
            view_3d: se_view_3d(),
            height_ldt: se_height_ldt(),
            height_xml: se_height_xml(),
            height_hint: se_height_hint(),
            planning: se_planning(),
            motion: se_motion(),
            tangential: se_tangential(),
            radial: se_radial(),
            presence: se_presence(),
            max_range: se_max_range(),
            area: se_area(),
            diameter: se_diameter(),
            quality: se_quality(),
            measured: se_measured(),
            interpolated: se_interpolated(),
            extrapolated: se_extrapolated(),
            no_data: se_no_data(),
            mounting: se_mounting(),
            wall: se_wall(),
            ceiling: se_ceiling(),
            wall_ceiling: se_wall_ceiling(),
            parameters: se_parameters(),
            not_applicable: se_not_applicable(),
            warnings: se_warnings(),
            demo: se_demo(),
            demo_note: se_demo_note(),
            not_detected: se_not_detected(),
            volume_ends: se_volume_ends(),
            nothing_detected: se_nothing_detected(),
            derived: se_derived(),
            report: se_report(),
            recognised: se_recognised(),
            open_as_luminaire: se_open_as_luminaire(),
        }
    }
}

fn se_tab() -> String {
    "Sensors".into()
}
fn se_title() -> String {
    "Presence & motion sensors".into()
}
fn se_intro() -> String {
    "A sensor LDT stores the detection volume in metres; SensXML stores measured detection areas per mounting height. Sensor files are recognised when you open or drop them; your sensors are listed on the dashboard.".into()
}
fn se_source() -> String {
    "Sensor".into()
}
fn se_none() -> String {
    "No sensor loaded yet. Open or drop a sensor file (sensor LDT, SensXML or a GLDF with sensors), load the demo sensors, or view the loaded LDT as a sensor.".into()
}
fn se_use_loaded() -> String {
    "View the loaded LDT as a sensor".into()
}
fn se_loaded_ldt() -> String {
    "Loaded LDT (as sensor)".into()
}
fn se_view() -> String {
    "View".into()
}
fn se_view_plan() -> String {
    "Detection areas".into()
}
fn se_view_polar() -> String {
    "Polar (range)".into()
}
fn se_view_cartesian() -> String {
    "Cartesian (range)".into()
}
fn se_view_heatmap() -> String {
    "Range heatmap".into()
}
fn se_view_3d() -> String {
    "Detection volume (3D)".into()
}
fn se_height_ldt() -> String {
    "Height above the detection plane".into()
}
fn se_height_xml() -> String {
    "Mounting height".into()
}
fn se_height_hint() -> String {
    "Reference height: 2.0 m".into()
}
fn se_planning() -> String {
    "Idealised planning shapes".into()
}
fn se_motion() -> String {
    "Movement".into()
}
fn se_tangential() -> String {
    "Tangential walking".into()
}
fn se_radial() -> String {
    "Radial walking".into()
}
fn se_presence() -> String {
    "Presence (seated)".into()
}
fn se_max_range() -> String {
    "Max range".into()
}
fn se_area() -> String {
    "Area".into()
}
fn se_diameter() -> String {
    "Ø equivalent".into()
}
fn se_quality() -> String {
    "Data".into()
}
fn se_measured() -> String {
    "measured".into()
}
fn se_interpolated() -> String {
    "interpolated".into()
}
fn se_extrapolated() -> String {
    "extrapolated".into()
}
fn se_no_data() -> String {
    "no data".into()
}
fn se_mounting() -> String {
    "Mounting".into()
}
fn se_wall() -> String {
    "wall".into()
}
fn se_ceiling() -> String {
    "ceiling".into()
}
fn se_wall_ceiling() -> String {
    "wall or ceiling".into()
}
fn se_parameters() -> String {
    "Parameter block (lines 33–42)".into()
}
fn se_not_applicable() -> String {
    "Cone, beam angle, isolux and the other light-based views do not apply to sensors.".into()
}
fn se_demo() -> String {
    "Load demo sensors".into()
}
fn se_not_detected() -> String {
    "not detected at this height".into()
}
fn se_volume_ends() -> String {
    "volume ends {d} m below the sensor".into()
}
fn se_nothing_detected() -> String {
    "Nothing is detected at this height. Lower the height or pick another sensor.".into()
}
fn se_derived() -> String {
    "Volume rebuilt from the measured heights ({h}); below the deepest height nothing is known."
        .into()
}
fn se_report() -> String {
    "PDF report".into()
}
fn se_recognised() -> String {
    "{name} looks like a sensor LDT (ranges in metres, a mounting code in lines 33–42), so it opened here as a sensor.".into()
}
fn se_open_as_luminaire() -> String {
    "Open as luminaire instead".into()
}
fn se_demo_note() -> String {
    "Demo sensors are synthetic data, not products.".into()
}
fn se_warnings() -> String {
    "Notes from the file".into()
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
            mel_title: ls_mel_title(),
            mel_values: ls_mel_values(),
            mel_day_ok: ls_mel_day_ok(),
            mel_day_low: ls_mel_day_low(),
            mel_no_spd: ls_mel_no_spd(),
            mel_note: ls_mel_note(),
            mel_card_der: ls_mel_card_der(),
            mel_card_edi: ls_mel_card_edi(),
            mel_card_need: ls_mel_card_need(),
            mel_card_estimate: ls_mel_card_estimate(),
            dli: ls_dli(),
            dli_explain: ls_dli_explain(),
            hint_ppfd: ls_hint_ppfd(),
            unit_ppfd: ls_unit_ppfd(),
            step_sensors: ls_step_sensors(),
            sensors_desc: ls_sensors_desc(),
            sensor_file: ls_sensor_file(),
            sensor_none: ls_sensor_none(),
            sensor_load_demo: ls_sensor_load_demo(),
            sensor_count: ls_sensor_count(),
            sensor_height: ls_sensor_height(),
            sensor_motion: ls_sensor_motion(),
            control_together: ls_control_together(),
            control_nearest: ls_control_nearest(),
            idle_level: ls_idle_level(),
            idle_off: ls_idle_off(),
            move_objects: ls_move_objects(),
            move_hint: ls_move_hint(),
            reset_moves: ls_reset_moves(),
            status_detected: ls_status_detected(),
            status_idle: ls_status_idle(),
            sensor_dim: ls_sensor_dim(),
            sensor_dim_off_note: ls_sensor_dim_off_note(),
            status_dim_off: ls_status_dim_off(),
            status_person: ls_status_person(),
            status_cars: ls_status_cars(),
            status_no_area: ls_status_no_area(),
            sensor_area: ls_sensor_area(),
            quality: ls_quality(),
            quality_quick: ls_quality_quick(),
            quality_balanced: ls_quality_balanced(),
            quality_high: ls_quality_high(),
            quality_desc_quick: ls_quality_desc_quick(),
            quality_desc_balanced: ls_quality_desc_balanced(),
            quality_desc_high: ls_quality_desc_high(),
            gpu_reset_note: ls_gpu_reset_note(),
            gpu_gone_note: ls_gpu_gone_note(),
            auto_quick: ls_auto_quick(),
            denoise: ls_denoise(),
            scene_planned: ls_scene_planned(),
            from_interior: ls_from_interior(),
            from_outdoor: ls_from_outdoor(),
            from_street: ls_from_street(),
            desc_planned: ls_desc_planned(),
            open_in_light_scene: ls_open_in_light_scene(),
            open_in_light_scene_tip: ls_open_in_light_scene_tip(),
            planned_mixed_note: ls_planned_mixed_note(),
            save_scene: ls_save_scene(),
            scene_name: ls_scene_name(),
            scene_saved: ls_scene_saved(),
            scene_unsaved: ls_scene_unsaved(),
            sensor_volume_3d: ls_sensor_volume_3d(),
            em_title: ls_em_title(),
            em_mode: ls_em_mode(),
            em_mode_desc: ls_em_mode_desc(),
            em_marks: ls_em_marks(),
            em_marks_off: ls_em_marks_off(),
            em_walk: ls_em_walk(),
            em_route: ls_em_route(),
            em_walk_at: ls_em_walk_at(),
            em_walk_play: ls_em_walk_play(),
            em_walk_pause: ls_em_walk_pause(),
            em_walk_exit: ls_em_walk_exit(),
            em_check_route: ls_em_check_route(),
            em_check_open: ls_em_check_open(),
            em_check_points: ls_em_check_points(),
            em_check_min: ls_em_check_min(),
            em_check_ratio: ls_em_check_ratio(),
            em_check_avg: ls_em_check_avg(),
            moon_title: ls_moon_title(),
            moon_on: ls_moon_on(),
            moon_desc: ls_moon_desc(),
            moon_date: ls_moon_date(),
            moon_time: ls_moon_time(),
            moon_full: ls_moon_full(),
            moon_place: ls_moon_place(),
            moon_pick: ls_moon_pick(),
            moon_north: ls_moon_north(),
            moon_lamps_off: ls_moon_lamps_off(),
            moon_readout: ls_moon_readout(),
            moon_below: ls_moon_below(),
            moon_indoor: ls_moon_indoor(),
            sun_readout: ls_sun_readout(),
            twilight_readout: ls_twilight_readout(),
            night_readout: ls_night_readout(),
            em_check_note: ls_em_check_note(),
            em_off_hint: ls_em_off_hint(),
            em_measure: ls_em_measure(),
            em_check_band: ls_em_check_band(),
            em_check_stairs: ls_em_check_stairs(),
            em_check_room: ls_em_check_room(),
            em_adapting: ls_em_adapting(),
            em_adapted: ls_em_adapted(),
            em_replay: ls_em_replay(),
            back_to_designer: ls_back_to_designer(),
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
fn dp_scenes_title() -> String {
    "Scenes".into()
}
fn ls_person_for_scale() -> String {
    "Person for scale".into()
}
fn ls_sensor_volume_3d() -> String {
    "Show the detection volume in 3D".into()
}
fn ls_em_title() -> String {
    "🚨 Emergency lighting".into()
}
fn ls_em_mode() -> String {
    "Emergency mode (mains failure)".into()
}
fn ls_em_mode_desc() -> String {
    "Normal lighting off, emergency luminaires on at their emergency flux × maintenance factor. The picture starts dark, as at the moment the power fails, and adapts like an eye; points take direct light only, as the emergency lighting standards count it.".into()
}
fn ls_em_marks() -> String {
    "Show the plan markings (route, open area, equipment)".into()
}
fn ls_em_marks_off() -> String {
    "As a person escaping sees it: no markings.".into()
}
fn ls_em_walk() -> String {
    "Walk the route at eye height (1.6 m)".into()
}
fn ls_em_route() -> String {
    "Route {n}".into()
}
fn ls_em_walk_at() -> String {
    "{m} of {len} m".into()
}
fn ls_em_walk_play() -> String {
    "▶ Walk".into()
}
fn ls_em_walk_pause() -> String {
    "⏸ Pause".into()
}
fn ls_em_walk_exit() -> String {
    "Back to the overview".into()
}
fn ls_em_check_route() -> String {
    "Escape route (centre line)".into()
}
fn ls_em_check_open() -> String {
    "Open area (without the 0.5 m border)".into()
}
fn ls_em_check_points() -> String {
    "Equipment (vertical)".into()
}
fn ls_em_check_min() -> String {
    "Emin {v} lx (≥ {req})".into()
}
fn ls_em_check_ratio() -> String {
    "Emax / Emin {v} (≤ {req})".into()
}
fn ls_moon_title() -> String {
    "☀️🌙 Sun & moon".into()
}
fn ls_moon_on() -> String {
    "Daylight and moonlight".into()
}
fn ls_moon_desc() -> String {
    "Natural light for a date, time and place under a clear sky: by day the sun with its shadows and the daylit sky, at dusk the twilight sky, at night the moon and the stars; in the picture and in the lux values. Codes do not count it, emergency mode leaves it out.".into()
}
fn ls_moon_date() -> String {
    "Date".into()
}
fn ls_moon_time() -> String {
    "Local time".into()
}
fn ls_moon_full() -> String {
    "→ Next full moon".into()
}
fn ls_moon_place() -> String {
    "Place: {place}".into()
}
fn ls_moon_pick() -> String {
    "🌍 Pick on the globe".into()
}
fn ls_moon_north() -> String {
    "North in the scene".into()
}
fn ls_moon_lamps_off() -> String {
    "Luminaires off (natural light alone)".into()
}
fn ls_moon_readout() -> String {
    "{phase} {pct} % · {alt}° up · {lx} lx on open ground (beam {beam} lx, sky {sky} lx)".into()
}
fn ls_moon_below() -> String {
    "The moon is below the horizon at this time.".into()
}
fn ls_sun_readout() -> String {
    "☀️ Sun {alt}° up · {lx} lx on open ground (beam {beam} lx, sky {sky} lx)".into()
}
fn ls_twilight_readout() -> String {
    "🌆 Twilight (sun {alt}°) · {lx} lx on open ground".into()
}
fn ls_night_readout() -> String {
    "✨ Starlight · {lx} lx".into()
}
fn ls_moon_indoor() -> String {
    "The moon lights outdoor scenes.".into()
}
fn ls_em_check_avg() -> String {
    "Eavg {v} lx (≥ {req})".into()
}
fn ls_em_check_note() -> String {
    "Check in the Light scene: direct light, points every 0.5 m. The Interior designer’s results are the reference.".into()
}
fn ls_em_off_hint() -> String {
    "Switch emergency mode on to measure the route.".into()
}
fn ls_em_measure() -> String {
    "📍 Measure (Grid view)".into()
}
fn ls_em_check_band() -> String {
    "Escape route (central half)".into()
}
fn ls_em_check_stairs() -> String {
    "Stairs / refuge (central half)".into()
}
fn ls_em_check_room() -> String {
    "Room (whole floor)".into()
}
fn ls_em_adapting() -> String {
    "👁 Eyes adapting to the dark… {pct} %".into()
}
fn ls_em_adapted() -> String {
    "👁 Adapted: {stops} stops brighter than when the power failed. Kept below full adaptation, so emergency light still looks dimmer than normal light.".into()
}
fn ls_em_replay() -> String {
    "⚡ Power failure again".into()
}
fn ls_back_to_designer() -> String {
    "✏️ Back to the {from}".into()
}
fn ls_save_scene() -> String {
    "💾 Save in the project".into()
}
fn ls_scene_name() -> String {
    "Scene name".into()
}
fn ls_scene_saved() -> String {
    "Saved in “{project}”; changes are saved automatically.".into()
}
fn ls_scene_unsaved() -> String {
    "Kept in this browser; save it to put it into the project file.".into()
}
fn ls_scene_planned() -> String {
    "📐 Plan".into()
}
fn ls_from_interior() -> String {
    "Interior designer".into()
}
fn ls_from_street() -> String {
    "Street designer".into()
}
fn ls_from_outdoor() -> String {
    "Outdoor designer".into()
}
fn ls_desc_planned() -> String {
    "From the {from}: {name}. Move luminaires, sensors and the person here to try alternatives; the plan itself stays as it is.".into()
}
fn ls_open_in_light_scene() -> String {
    "🎬 Open in Light scene".into()
}
fn ls_open_in_light_scene_tip() -> String {
    "Check this plan path traced: the room or area with its luminaires and sensors.".into()
}
fn ls_planned_mixed_note() -> String {
    "Several luminaire types in the plan: all are shown with the main luminaire for now.".into()
}
fn ls_auto_quick() -> String {
    "Quick preview while the camera or an object moves".into()
}
fn ls_denoise() -> String {
    "Smooth the noise of the reflected light (picture only, not the lux values)".into()
}
fn ls_quality() -> String {
    "Quality".into()
}
fn ls_quality_quick() -> String {
    "⚡ Quick".into()
}
fn ls_quality_balanced() -> String {
    "Balanced".into()
}
fn ls_quality_high() -> String {
    "✨ High".into()
}
fn ls_quality_desc_quick() -> String {
    "Approximate preview: direct light plus an estimated even fill for the reflected light, lower resolution, no haze. Fast for moving things around; lux values are direct light only.".into()
}
fn ls_quality_desc_balanced() -> String {
    "One bounce of reflected light, every lamp at the first hit and one random lamp after it, medium resolution.".into()
}
fn ls_quality_desc_high() -> String {
    "Full path tracing at full resolution, every lamp at every bounce: the reference picture, slow on small GPUs.".into()
}
fn ls_gpu_reset_note() -> String {
    "The browser reset the GPU: quality lowered to Quick.".into()
}
fn ls_gpu_gone_note() -> String {
    "No GPU after the reset: tracing on the CPU. Reload the page to use the GPU again.".into()
}
fn ls_step_sensors() -> String {
    "4 · Sensors".into()
}
fn ls_sensors_desc() -> String {
    "Presence / motion detectors switch the luminaires: drag the person or a car into a detection area and the light comes on.".into()
}
fn ls_sensor_file() -> String {
    "Sensor".into()
}
fn ls_sensor_none() -> String {
    "No sensor files in the project".into()
}
fn ls_sensor_load_demo() -> String {
    "Load demo sensors".into()
}
fn ls_sensor_count() -> String {
    "Sensors".into()
}
fn ls_sensor_height() -> String {
    "Sensor height".into()
}
fn ls_sensor_motion() -> String {
    "Detect".into()
}
fn ls_control_together() -> String {
    "All luminaires together".into()
}
fn ls_control_nearest() -> String {
    "Each luminaire with its nearest sensor".into()
}
fn ls_idle_level() -> String {
    "Without detection".into()
}
fn ls_idle_off() -> String {
    "off".into()
}
fn ls_move_objects() -> String {
    "✋ Move objects".into()
}
fn ls_move_hint() -> String {
    "Drag a luminaire, the person, a car or a sensor on the picture.".into()
}
fn ls_reset_moves() -> String {
    "Put objects back".into()
}
fn ls_status_detected() -> String {
    "Detected: {what} · {on} of {n} luminaires on".into()
}
fn ls_status_idle() -> String {
    "Nothing detected · luminaires at {pct} %".into()
}
fn ls_sensor_dim() -> String {
    "Sensors switch the luminaires".into()
}
fn ls_sensor_dim_off_note() -> String {
    "Off: the sensors only detect, the luminaires stay at full output (as in the designer). Turn it on and move the person or a car to try the control.".into()
}
fn ls_status_dim_off() -> String {
    "Sensor control off · luminaires at full output".into()
}
fn ls_status_person() -> String {
    "person".into()
}
fn ls_status_cars() -> String {
    "{n} car(s)".into()
}
fn ls_status_no_area() -> String {
    "This sensor detects nothing at {h} m.".into()
}
fn ls_sensor_area() -> String {
    "Detection area Ø {d} m at {h} m".into()
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
    "Outdoor designer".into()
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
    #[serde(default = "df_colour_quality_title")]
    pub colour_quality_title: String,
    #[serde(default = "df_colour_quality_desc")]
    pub colour_quality_desc: String,
    #[serde(default = "df_sensor_title")]
    pub sensor_title: String,
    #[serde(default = "df_sensor_desc")]
    pub sensor_desc: String,
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
            colour_quality_title: df_colour_quality_title(),
            colour_quality_desc: df_colour_quality_desc(),
            sensor_title: df_sensor_title(),
            sensor_desc: df_sensor_desc(),
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
    "Outdoor designer".into()
}
fn df_interior_title() -> String {
    "Interior designer".into()
}
fn df_maps_title() -> String {
    "Site map".into()
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
    "Plan on an OpenStreetMap map of the site: find it on the globe, draw the area, place the poles"
        .into()
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
fn ls_mel_title() -> String {
    "Melanopic EDI at the eye".into()
}
fn ls_mel_values() -> String {
    "{mean} lx mean · {min} lx at the darkest position (vertical illuminance at eye height, 1.2 m, × melanopic DER {der} of the luminaire's spectrum, CIE S 026)".into()
}
fn ls_mel_day_ok() -> String {
    "The daytime recommendation of at least 250 lx melanopic EDI is met at every eye position."
        .into()
}
fn ls_mel_day_low() -> String {
    "The daytime recommendation of at least 250 lx melanopic EDI is not met everywhere: {min} lx at the darkest position.".into()
}
fn ls_mel_no_spd() -> String {
    "No spectrum in the file, so no melanopic values. Vertical illuminance at eye height: {mean} lx mean, {min} lx minimum. Load a TM-33 / ATLA file or attach a spectrum.".into()
}
fn ls_mel_note() -> String {
    "Recommendations (Brown et al. 2022): by day at least 250 lx, in the evening at most 10 lx, at night at most 1 lx melanopic EDI at the eye. Electric light only; daylight adds to it.".into()
}
fn ls_mel_card_der() -> String {
    "Melanopic DER {der} (D65 daylight = 1)".into()
}
fn ls_mel_card_edi() -> String {
    "{lux} lx at the eye give {edi} lx melanopic EDI".into()
}
fn ls_mel_card_need() -> String {
    "250 lx melanopic EDI need {lux} lx at the eye".into()
}
fn ls_mel_card_estimate() -> String {
    "Estimated from the colour temperature: this file has no spectrum.".into()
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
fn df_colour_quality_title() -> String {
    "Colour quality".into()
}
fn df_colour_quality_desc() -> String {
    "TM-30 colour vector graphic, Rf / Rg, CRI Ra, R9 and R1–R15 and Duv from the spectrum, as one card; demo LED included".into()
}
fn df_sensor_title() -> String {
    "Sensors".into()
}
fn df_sensor_desc() -> String {
    "Presence and motion detectors from GLDF, sensor LDT or SensXML: detection areas per motion type and mounting height, range diagrams".into()
}
fn df_spectrum_lab_title() -> String {
    "Spectrum Lab".into()
}
fn df_spectrum_lab_desc() -> String {
    "Photon-by-photon spectral Monte Carlo: night vision (S/P), circadian (melanopic DER), mesopic road luminance, colour over angle and a dark-sky report".into()
}
