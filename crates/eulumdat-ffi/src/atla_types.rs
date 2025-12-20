//! ATLA (LuminaireOpticalData) types for FFI bindings
//!
//! This module provides uniffi bindings for the ATLA S001 photometric data format,
//! which is the internal representation used for all photometric data.

use atla::LuminaireOpticalData as CoreAtla;

use crate::diagram::SvgThemeType;
use crate::error::EulumdatError;
use crate::Language;

/// Spectral distribution data (SPD)
#[derive(Debug, Clone, uniffi::Record)]
pub struct SpectralDistribution {
    /// Wavelengths in nanometers
    pub wavelengths: Vec<f64>,
    /// Spectral power values
    pub values: Vec<f64>,
    /// Whether values are relative (normalized) or absolute (W/nm)
    pub is_relative: bool,
}

impl From<&atla::SpectralDistribution> for SpectralDistribution {
    fn from(spd: &atla::SpectralDistribution) -> Self {
        Self {
            wavelengths: spd.wavelengths.clone(),
            values: spd.values.clone(),
            is_relative: matches!(spd.units, atla::SpectralUnits::Relative),
        }
    }
}

impl From<&SpectralDistribution> for atla::SpectralDistribution {
    fn from(spd: &SpectralDistribution) -> Self {
        atla::SpectralDistribution {
            wavelengths: spd.wavelengths.clone(),
            values: spd.values.clone(),
            units: if spd.is_relative {
                atla::SpectralUnits::Relative
            } else {
                atla::SpectralUnits::WattsPerNanometer
            },
            start_wavelength: None,
            wavelength_interval: None,
        }
    }
}

/// Color rendering metrics
#[derive(Debug, Clone, uniffi::Record)]
pub struct ColorRendering {
    /// CRI Ra value (0-100)
    pub ra: Option<f64>,
    /// CRI R9 value (red rendering, can be negative)
    pub r9: Option<f64>,
    /// TM-30 Fidelity index Rf
    pub rf: Option<f64>,
    /// TM-30 Gamut index Rg
    pub rg: Option<f64>,
}

impl From<&atla::ColorRendering> for ColorRendering {
    fn from(cr: &atla::ColorRendering) -> Self {
        Self {
            ra: cr.ra,
            r9: cr.r9,
            rf: cr.rf,
            rg: cr.rg,
        }
    }
}

/// Emitter (light source) data
#[derive(Debug, Clone, uniffi::Record)]
pub struct Emitter {
    /// Description of the emitter
    pub description: Option<String>,
    /// Number of identical emitters
    pub quantity: u32,
    /// Rated luminous flux in lumens
    pub rated_lumens: Option<f64>,
    /// Measured luminous flux in lumens
    pub measured_lumens: Option<f64>,
    /// Input power in watts
    pub input_watts: Option<f64>,
    /// Correlated color temperature in Kelvin
    pub cct: Option<f64>,
    /// Color rendering metrics
    pub color_rendering: Option<ColorRendering>,
    /// Spectral distribution (if available)
    pub spectral_distribution: Option<SpectralDistribution>,
}

impl From<&atla::Emitter> for Emitter {
    fn from(e: &atla::Emitter) -> Self {
        Self {
            description: e.description.clone(),
            quantity: e.quantity,
            rated_lumens: e.rated_lumens,
            measured_lumens: e.measured_lumens,
            input_watts: e.input_watts,
            cct: e.cct,
            color_rendering: e.color_rendering.as_ref().map(ColorRendering::from),
            spectral_distribution: e
                .spectral_distribution
                .as_ref()
                .map(SpectralDistribution::from),
        }
    }
}

/// ATLA Document - comprehensive photometric data structure
///
/// This is the primary data structure used internally, supporting:
/// - Spectral data (SPD)
/// - Color rendering metrics (Ra, R9, Rf, Rg)
/// - Multiple emitters
/// - XML and JSON serialization
#[derive(Clone, uniffi::Object)]
pub struct AtlaDocument {
    inner: CoreAtla,
}

impl Default for AtlaDocument {
    fn default() -> Self {
        Self::new()
    }
}

#[uniffi::export]
impl AtlaDocument {
    /// Create a new empty ATLA document
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            inner: CoreAtla::default(),
        }
    }

    /// Parse from ATLA XML string
    #[uniffi::constructor]
    pub fn parse_xml(content: &str) -> Result<Self, EulumdatError> {
        atla::xml::parse(content)
            .map(|inner| Self { inner })
            .map_err(|e| EulumdatError::ParseError(e.to_string()))
    }

    /// Parse from ATLA JSON string
    #[uniffi::constructor]
    pub fn parse_json(content: &str) -> Result<Self, EulumdatError> {
        atla::json::parse(content)
            .map(|inner| Self { inner })
            .map_err(|e| EulumdatError::ParseError(e.to_string()))
    }

    /// Parse from LDT string (converts to ATLA internally)
    #[uniffi::constructor]
    pub fn from_ldt(content: &str) -> Result<Self, EulumdatError> {
        let ldt = eulumdat::Eulumdat::parse(content)
            .map_err(|e| EulumdatError::ParseError(e.to_string()))?;
        Ok(Self {
            inner: CoreAtla::from_eulumdat(&ldt),
        })
    }

    /// Parse from IES string (converts to ATLA internally)
    #[uniffi::constructor]
    pub fn from_ies(content: &str) -> Result<Self, EulumdatError> {
        let ldt = eulumdat::IesParser::parse(content)
            .map_err(|e| EulumdatError::ParseError(e.to_string()))?;
        Ok(Self {
            inner: CoreAtla::from_eulumdat(&ldt),
        })
    }

    /// Export to ATLA XML string
    pub fn to_xml(&self) -> Result<String, EulumdatError> {
        atla::xml::write(&self.inner).map_err(|e| EulumdatError::ExportError(e.to_string()))
    }

    /// Export to ATLA JSON string
    pub fn to_json(&self) -> Result<String, EulumdatError> {
        atla::json::write(&self.inner).map_err(|e| EulumdatError::ExportError(e.to_string()))
    }

    /// Export to LDT string
    pub fn to_ldt(&self) -> String {
        self.inner.to_eulumdat().to_ldt()
    }

    /// Export to IES string
    pub fn to_ies(&self) -> String {
        eulumdat::IesExporter::export(&self.inner.to_eulumdat())
    }

    // === Metadata ===

    /// Manufacturer name
    pub fn manufacturer(&self) -> Option<String> {
        self.inner.header.manufacturer.clone()
    }

    /// Description/luminaire name
    pub fn description(&self) -> Option<String> {
        self.inner.header.description.clone()
    }

    /// Catalog number
    pub fn catalog_number(&self) -> Option<String> {
        self.inner.header.catalog_number.clone()
    }

    // === Emitter Access ===

    /// Get all emitters
    pub fn emitters(&self) -> Vec<Emitter> {
        self.inner.emitters.iter().map(Emitter::from).collect()
    }

    /// Get the primary (first) emitter
    pub fn primary_emitter(&self) -> Option<Emitter> {
        self.inner.emitters.first().map(Emitter::from)
    }

    // === Computed Values ===

    /// Total luminous flux from all emitters
    pub fn total_luminous_flux(&self) -> f64 {
        self.inner.total_luminous_flux()
    }

    /// Total input power from all emitters
    pub fn total_input_watts(&self) -> f64 {
        self.inner.total_input_watts()
    }

    /// System efficacy in lm/W
    pub fn efficacy(&self) -> Option<f64> {
        let flux = self.total_luminous_flux();
        let watts = self.total_input_watts();
        if watts > 0.0 {
            Some(flux / watts)
        } else {
            None
        }
    }

    /// Get CCT from primary emitter
    pub fn cct(&self) -> Option<f64> {
        self.inner.emitters.first().and_then(|e| e.cct)
    }

    /// Get CRI (Ra) from primary emitter
    pub fn cri(&self) -> Option<f64> {
        self.inner
            .emitters
            .first()
            .and_then(|e| e.color_rendering.as_ref())
            .and_then(|cr| cr.ra)
    }

    /// Check if spectral data is available
    pub fn has_spectral_data(&self) -> bool {
        self.inner
            .emitters
            .iter()
            .any(|e| e.spectral_distribution.is_some())
    }
}

// === Diagram Generation Functions ===

/// Generate spectral diagram SVG from ATLA document
///
/// Uses actual spectral data if available, otherwise synthesizes from CCT/CRI.
#[uniffi::export]
pub fn generate_spectral_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    dark: bool,
) -> Result<String, EulumdatError> {
    let theme = if dark {
        atla::spectral::SpectralTheme::dark()
    } else {
        atla::spectral::SpectralTheme::light()
    };

    // Try to get spectral data from emitters
    if let Some(spd) = doc
        .inner
        .emitters
        .iter()
        .filter_map(|e| e.spectral_distribution.as_ref())
        .next()
    {
        let diagram = atla::spectral::SpectralDiagram::from_spectral(spd);
        return Ok(diagram.to_svg(width, height, &theme));
    }

    // Try to synthesize from CCT/CRI
    if let Some(emitter) = doc.inner.emitters.first() {
        if let Some(cct) = emitter.cct {
            let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
            let spd = atla::spectral::synthesize_spectrum(cct, cri);
            let diagram = atla::spectral::SpectralDiagram::from_spectral(&spd);
            return Ok(diagram.to_svg(width, height, &theme));
        }
    }

    Err(EulumdatError::ValidationError(
        "No spectral data or CCT available for spectral diagram".to_string(),
    ))
}

/// Generate spectral diagram SVG from ATLA document with localized labels
///
/// Uses actual spectral data if available, otherwise synthesizes from CCT/CRI.
#[uniffi::export]
pub fn generate_spectral_svg_localized(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    dark: bool,
    language: Language,
) -> Result<String, EulumdatError> {
    use eulumdat_i18n::{Language as CoreLanguage, Locale};

    let core_lang = match language {
        Language::German => CoreLanguage::German,
        Language::Chinese => CoreLanguage::Chinese,
        Language::French => CoreLanguage::French,
        Language::Italian => CoreLanguage::Italian,
        Language::Russian => CoreLanguage::Russian,
        Language::Spanish => CoreLanguage::Spanish,
        Language::PortugueseBrazil => CoreLanguage::PortugueseBrazil,
        Language::English => CoreLanguage::English,
    };
    let locale = Locale::for_language(core_lang);

    let theme = if dark {
        atla::spectral::SpectralTheme::dark_with_locale(&locale)
    } else {
        atla::spectral::SpectralTheme::light_with_locale(&locale)
    };

    // Try to get spectral data from emitters
    if let Some(spd) = doc
        .inner
        .emitters
        .iter()
        .filter_map(|e| e.spectral_distribution.as_ref())
        .next()
    {
        let diagram = atla::spectral::SpectralDiagram::from_spectral(spd);
        return Ok(diagram.to_svg(width, height, &theme));
    }

    // Try to synthesize from CCT/CRI
    if let Some(emitter) = doc.inner.emitters.first() {
        if let Some(cct) = emitter.cct {
            let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
            let spd = atla::spectral::synthesize_spectrum(cct, cri);
            let diagram = atla::spectral::SpectralDiagram::from_spectral(&spd);
            return Ok(diagram.to_svg(width, height, &theme));
        }
    }

    Err(EulumdatError::ValidationError(
        "No spectral data or CCT available for spectral diagram".to_string(),
    ))
}

/// Generate greenhouse/PPFD diagram SVG from ATLA document
///
/// Shows PPFD at various mounting distances for horticultural lighting.
#[uniffi::export]
pub fn generate_greenhouse_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    max_height: f64,
    dark: bool,
) -> String {
    let theme = if dark {
        atla::greenhouse::GreenhouseTheme::dark()
    } else {
        atla::greenhouse::GreenhouseTheme::light()
    };
    let diagram =
        atla::greenhouse::GreenhouseDiagram::from_atla_with_height(&doc.inner, max_height);
    diagram.to_svg(width, height, &theme)
}

/// Generate localized greenhouse/PPFD diagram SVG from ATLA document
///
/// Shows PPFD at various mounting distances for horticultural lighting.
#[uniffi::export]
pub fn generate_greenhouse_svg_localized(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    max_height: f64,
    dark: bool,
    language: Language,
) -> String {
    let theme = if dark {
        atla::greenhouse::GreenhouseTheme::dark()
    } else {
        atla::greenhouse::GreenhouseTheme::light()
    };
    let labels = match language {
        Language::German => atla::GreenhouseLabels::german(),
        Language::Chinese => atla::GreenhouseLabels::chinese(),
        Language::French => atla::GreenhouseLabels::french(),
        Language::Italian => atla::GreenhouseLabels::italian(),
        Language::Russian => atla::GreenhouseLabels::russian(),
        Language::Spanish => atla::GreenhouseLabels::spanish(),
        Language::PortugueseBrazil => atla::GreenhouseLabels::portuguese_brazil(),
        Language::English => atla::GreenhouseLabels::default(),
    };
    let diagram =
        atla::greenhouse::GreenhouseDiagram::from_atla_with_height(&doc.inner, max_height);
    diagram.to_svg_with_labels(width, height, &theme, &labels)
}

/// Generate polar diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_polar_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate butterfly diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_butterfly_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    tilt_degrees: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram =
        eulumdat::diagram::ButterflyDiagram::from_eulumdat(&ldt, width, height, tilt_degrees);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate cartesian diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_cartesian_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    max_curves: u32,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::diagram::CartesianDiagram::from_eulumdat(
        &ldt,
        width,
        height,
        max_curves as usize,
    );
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate heatmap diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_heatmap_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::diagram::HeatmapDiagram::from_eulumdat(&ldt, width, height);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate cone diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_cone_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    mounting_height: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::diagram::ConeDiagram::from_eulumdat(&ldt, mounting_height);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate beam angle diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_beam_angle_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
    let analysis = eulumdat::PhotometricCalculations::beam_field_analysis(&ldt);
    let show_both = analysis.is_batwing;
    diagram.to_svg_with_beam_field_angles(width, height, &theme.to_core(), &analysis, show_both)
}

/// Generate BUG rating diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_bug_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::BugDiagram::from_eulumdat(&ldt);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate LCS classification diagram SVG from ATLA document
#[uniffi::export]
pub fn generate_atla_lcs_svg(
    doc: &AtlaDocument,
    width: f64,
    height: f64,
    theme: SvgThemeType,
) -> String {
    let ldt = doc.inner.to_eulumdat();
    let diagram = eulumdat::BugDiagram::from_eulumdat(&ldt);
    diagram.to_lcs_svg(width, height, &theme.to_core())
}
