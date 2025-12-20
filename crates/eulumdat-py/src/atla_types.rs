//! ATLA (LuminaireOpticalData) types for Python bindings
//!
//! This module provides Python bindings for the ATLA S001 photometric data format,
//! which is the internal representation used for all photometric data.

use pyo3::prelude::*;

use atla::LuminaireOpticalData as CoreAtla;

use crate::diagram::SvgTheme;
use crate::error::{atla_to_py_err, io_to_py_err, to_py_err};

/// Spectral distribution data (SPD)
#[pyclass]
#[derive(Clone)]
pub struct SpectralDistribution {
    /// Wavelengths in nanometers
    #[pyo3(get)]
    pub wavelengths: Vec<f64>,
    /// Spectral power values
    #[pyo3(get)]
    pub values: Vec<f64>,
    /// Whether values are relative (normalized) or absolute (W/nm)
    #[pyo3(get)]
    pub is_relative: bool,
}

#[pymethods]
impl SpectralDistribution {
    #[new]
    #[pyo3(signature = (wavelengths, values, is_relative=true))]
    fn new(wavelengths: Vec<f64>, values: Vec<f64>, is_relative: bool) -> Self {
        Self {
            wavelengths,
            values,
            is_relative,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SpectralDistribution(wavelengths={}, values={}, relative={})",
            self.wavelengths.len(),
            self.values.len(),
            self.is_relative
        )
    }
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
#[pyclass]
#[derive(Clone)]
pub struct ColorRendering {
    /// CRI Ra value (0-100)
    #[pyo3(get, set)]
    pub ra: Option<f64>,
    /// CRI R9 value (red rendering, can be negative)
    #[pyo3(get, set)]
    pub r9: Option<f64>,
    /// TM-30 Fidelity index Rf
    #[pyo3(get, set)]
    pub rf: Option<f64>,
    /// TM-30 Gamut index Rg
    #[pyo3(get, set)]
    pub rg: Option<f64>,
}

#[pymethods]
impl ColorRendering {
    #[new]
    #[pyo3(signature = (ra=None, r9=None, rf=None, rg=None))]
    fn new(ra: Option<f64>, r9: Option<f64>, rf: Option<f64>, rg: Option<f64>) -> Self {
        Self { ra, r9, rf, rg }
    }

    fn __repr__(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ra) = self.ra {
            parts.push(format!("Ra={:.0}", ra));
        }
        if let Some(r9) = self.r9 {
            parts.push(format!("R9={:.0}", r9));
        }
        if let Some(rf) = self.rf {
            parts.push(format!("Rf={:.0}", rf));
        }
        if let Some(rg) = self.rg {
            parts.push(format!("Rg={:.0}", rg));
        }
        format!("ColorRendering({})", parts.join(", "))
    }
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

impl From<&ColorRendering> for atla::ColorRendering {
    fn from(cr: &ColorRendering) -> Self {
        atla::ColorRendering {
            ra: cr.ra,
            r9: cr.r9,
            rf: cr.rf,
            rg: cr.rg,
        }
    }
}

/// Emitter (light source) data
#[pyclass]
#[derive(Clone)]
pub struct Emitter {
    /// Description of the emitter
    #[pyo3(get, set)]
    pub description: Option<String>,
    /// Number of identical emitters
    #[pyo3(get, set)]
    pub quantity: u32,
    /// Rated luminous flux in lumens
    #[pyo3(get, set)]
    pub rated_lumens: Option<f64>,
    /// Measured luminous flux in lumens
    #[pyo3(get, set)]
    pub measured_lumens: Option<f64>,
    /// Input power in watts
    #[pyo3(get, set)]
    pub input_watts: Option<f64>,
    /// Correlated color temperature in Kelvin
    #[pyo3(get, set)]
    pub cct: Option<f64>,
    /// Color rendering metrics
    #[pyo3(get, set)]
    pub color_rendering: Option<ColorRendering>,
    /// Spectral distribution (if available)
    #[pyo3(get)]
    pub spectral_distribution: Option<SpectralDistribution>,
}

#[pymethods]
impl Emitter {
    #[new]
    #[pyo3(signature = (
        description=None,
        quantity=1,
        rated_lumens=None,
        measured_lumens=None,
        input_watts=None,
        cct=None,
        color_rendering=None
    ))]
    fn new(
        description: Option<String>,
        quantity: u32,
        rated_lumens: Option<f64>,
        measured_lumens: Option<f64>,
        input_watts: Option<f64>,
        cct: Option<f64>,
        color_rendering: Option<ColorRendering>,
    ) -> Self {
        Self {
            description,
            quantity,
            rated_lumens,
            measured_lumens,
            input_watts,
            cct,
            color_rendering,
            spectral_distribution: None,
        }
    }

    /// Set spectral distribution
    fn set_spectral(&mut self, spd: SpectralDistribution) {
        self.spectral_distribution = Some(spd);
    }

    /// Get luminous flux (measured if available, otherwise rated)
    fn luminous_flux(&self) -> Option<f64> {
        self.measured_lumens.or(self.rated_lumens)
    }

    /// Calculate efficacy in lm/W
    fn efficacy(&self) -> Option<f64> {
        match (self.luminous_flux(), self.input_watts) {
            (Some(flux), Some(watts)) if watts > 0.0 => Some(flux / watts),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        let desc = self.description.as_deref().unwrap_or("unknown");
        let flux = self
            .luminous_flux()
            .map(|f| format!("{:.0} lm", f))
            .unwrap_or_else(|| "? lm".to_string());
        format!(
            "Emitter(description='{}', quantity={}, flux={})",
            desc, self.quantity, flux
        )
    }
}

/// ATLA Luminaire Optical Data - comprehensive photometric data structure
///
/// This is the primary data structure used internally, supporting:
/// - Spectral data (SPD)
/// - Color rendering metrics (Ra, R9, Rf, Rg)
/// - Multiple emitters
/// - XML and JSON serialization
#[pyclass]
pub struct AtlaDocument {
    inner: CoreAtla,
}

#[pymethods]
impl AtlaDocument {
    /// Create a new empty ATLA document
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreAtla::default(),
        }
    }

    /// Parse from ATLA XML string
    #[staticmethod]
    fn parse_xml(content: &str) -> PyResult<Self> {
        atla::xml::parse(content)
            .map(|inner| Self { inner })
            .map_err(atla_to_py_err)
    }

    /// Parse from ATLA JSON string
    #[staticmethod]
    fn parse_json(content: &str) -> PyResult<Self> {
        atla::json::parse(content)
            .map(|inner| Self { inner })
            .map_err(atla_to_py_err)
    }

    /// Parse from LDT string (converts to ATLA internally)
    #[staticmethod]
    fn from_ldt(content: &str) -> PyResult<Self> {
        let ldt = eulumdat::Eulumdat::parse(content).map_err(to_py_err)?;
        Ok(Self {
            inner: CoreAtla::from_eulumdat(&ldt),
        })
    }

    /// Parse from IES string (converts to ATLA internally)
    #[staticmethod]
    fn from_ies(content: &str) -> PyResult<Self> {
        let ldt = eulumdat::IesParser::parse(content).map_err(to_py_err)?;
        Ok(Self {
            inner: CoreAtla::from_eulumdat(&ldt),
        })
    }

    /// Load from file (auto-detects format by extension)
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let path = std::path::Path::new(path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let content = std::fs::read_to_string(path).map_err(io_to_py_err)?;

        match ext.as_str() {
            "xml" => Self::parse_xml(&content),
            "json" => Self::parse_json(&content),
            "ldt" => Self::from_ldt(&content),
            "ies" => Self::from_ies(&content),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown file extension: {} (expected .xml, .json, .ldt, or .ies)",
                ext
            ))),
        }
    }

    /// Export to ATLA XML string
    fn to_xml(&self) -> PyResult<String> {
        atla::xml::write(&self.inner).map_err(atla_to_py_err)
    }

    /// Export to ATLA JSON string
    fn to_json(&self) -> PyResult<String> {
        atla::json::write(&self.inner).map_err(atla_to_py_err)
    }

    /// Export to LDT string
    fn to_ldt(&self) -> String {
        self.inner.to_eulumdat().to_ldt()
    }

    /// Export to IES string
    fn to_ies(&self) -> String {
        eulumdat::IesExporter::export(&self.inner.to_eulumdat())
    }

    // === Metadata ===

    /// Manufacturer name
    #[getter]
    fn manufacturer(&self) -> Option<String> {
        self.inner.header.manufacturer.clone()
    }

    #[setter]
    fn set_manufacturer(&mut self, value: Option<String>) {
        self.inner.header.manufacturer = value;
    }

    /// Description/luminaire name
    #[getter]
    fn description(&self) -> Option<String> {
        self.inner.header.description.clone()
    }

    #[setter]
    fn set_description(&mut self, value: Option<String>) {
        self.inner.header.description = value;
    }

    /// Catalog number
    #[getter]
    fn catalog_number(&self) -> Option<String> {
        self.inner.header.catalog_number.clone()
    }

    #[setter]
    fn set_catalog_number(&mut self, value: Option<String>) {
        self.inner.header.catalog_number = value;
    }

    // === Emitter Access ===

    /// Get all emitters
    #[getter]
    fn emitters(&self) -> Vec<Emitter> {
        self.inner
            .emitters
            .iter()
            .map(|e| Emitter {
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
            })
            .collect()
    }

    /// Get the primary (first) emitter
    fn primary_emitter(&self) -> Option<Emitter> {
        self.inner.emitters.first().map(|e| Emitter {
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
        })
    }

    // === Computed Values ===

    /// Total luminous flux from all emitters
    fn total_luminous_flux(&self) -> f64 {
        self.inner.total_luminous_flux()
    }

    /// Total input power from all emitters
    fn total_input_watts(&self) -> f64 {
        self.inner.total_input_watts()
    }

    /// System efficacy in lm/W
    fn efficacy(&self) -> Option<f64> {
        let flux = self.total_luminous_flux();
        let watts = self.total_input_watts();
        if watts > 0.0 {
            Some(flux / watts)
        } else {
            None
        }
    }

    /// Get CCT from primary emitter
    fn cct(&self) -> Option<f64> {
        self.inner.emitters.first().and_then(|e| e.cct)
    }

    /// Get CRI (Ra) from primary emitter
    fn cri(&self) -> Option<f64> {
        self.inner
            .emitters
            .first()
            .and_then(|e| e.color_rendering.as_ref())
            .and_then(|cr| cr.ra)
    }

    /// Check if spectral data is available
    fn has_spectral_data(&self) -> bool {
        self.inner
            .emitters
            .iter()
            .any(|e| e.spectral_distribution.is_some())
    }

    // === Diagram Generation ===

    /// Generate spectral diagram SVG
    ///
    /// Uses actual spectral data if available, otherwise synthesizes from CCT/CRI.
    #[pyo3(signature = (width=700.0, height=400.0, dark=false))]
    fn spectral_svg(&self, width: f64, height: f64, dark: bool) -> PyResult<String> {
        let theme = if dark {
            atla::spectral::SpectralTheme::dark()
        } else {
            atla::spectral::SpectralTheme::light()
        };

        // Try to get spectral data from emitters
        if let Some(spd) = self
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
        if let Some(emitter) = self.inner.emitters.first() {
            if let Some(cct) = emitter.cct {
                let cri = emitter.color_rendering.as_ref().and_then(|cr| cr.ra);
                let spd = atla::spectral::synthesize_spectrum(cct, cri);
                let diagram = atla::spectral::SpectralDiagram::from_spectral(&spd);
                return Ok(diagram.to_svg(width, height, &theme));
            }
        }

        Err(pyo3::exceptions::PyValueError::new_err(
            "No spectral data or CCT available for spectral diagram",
        ))
    }

    /// Generate greenhouse/PPFD diagram SVG
    ///
    /// Shows PPFD at various mounting distances for horticultural lighting.
    #[pyo3(signature = (width=600.0, height=450.0, max_height=2.0, dark=false))]
    fn greenhouse_svg(&self, width: f64, height: f64, max_height: f64, dark: bool) -> String {
        let theme = if dark {
            atla::greenhouse::GreenhouseTheme::dark()
        } else {
            atla::greenhouse::GreenhouseTheme::light()
        };
        let diagram =
            atla::greenhouse::GreenhouseDiagram::from_atla_with_height(&self.inner, max_height);
        diagram.to_svg(width, height, &theme)
    }

    /// Generate polar diagram SVG
    #[pyo3(signature = (width=500.0, height=500.0, theme=SvgTheme::Light))]
    fn polar_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate butterfly diagram SVG
    #[pyo3(signature = (width=500.0, height=400.0, rotation=60.0, theme=SvgTheme::Light))]
    fn butterfly_svg(&self, width: f64, height: f64, rotation: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram =
            eulumdat::diagram::ButterflyDiagram::from_eulumdat(&ldt, width, height, rotation);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate cartesian diagram SVG
    #[pyo3(signature = (width=600.0, height=400.0, max_curves=8, theme=SvgTheme::Light))]
    fn cartesian_svg(&self, width: f64, height: f64, max_curves: usize, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram =
            eulumdat::diagram::CartesianDiagram::from_eulumdat(&ldt, width, height, max_curves);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate heatmap diagram SVG
    #[pyo3(signature = (width=700.0, height=500.0, theme=SvgTheme::Light))]
    fn heatmap_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram = eulumdat::diagram::HeatmapDiagram::from_eulumdat(&ldt, width, height);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate cone diagram SVG
    #[pyo3(signature = (width=600.0, height=450.0, mounting_height=3.0, theme=SvgTheme::Light))]
    fn cone_svg(&self, width: f64, height: f64, mounting_height: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram = eulumdat::diagram::ConeDiagram::from_eulumdat(&ldt, mounting_height);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate beam angle diagram SVG
    #[pyo3(signature = (width=600.0, height=600.0, theme=SvgTheme::Light))]
    fn beam_angle_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram = eulumdat::diagram::PolarDiagram::from_eulumdat(&ldt);
        let analysis = eulumdat::PhotometricCalculations::beam_field_analysis(&ldt);
        let show_both = analysis.is_batwing;
        diagram.to_svg_with_beam_field_angles(width, height, &theme.to_core(), &analysis, show_both)
    }

    /// Generate BUG rating diagram SVG
    #[pyo3(signature = (width=400.0, height=350.0, theme=SvgTheme::Light))]
    fn bug_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram = eulumdat::BugDiagram::from_eulumdat(&ldt);
        diagram.to_svg(width, height, &theme.to_core())
    }

    /// Generate LCS classification diagram SVG
    #[pyo3(signature = (width=510.0, height=315.0, theme=SvgTheme::Light))]
    fn lcs_svg(&self, width: f64, height: f64, theme: SvgTheme) -> String {
        let ldt = self.inner.to_eulumdat();
        let diagram = eulumdat::BugDiagram::from_eulumdat(&ldt);
        diagram.to_lcs_svg(width, height, &theme.to_core())
    }

    fn __repr__(&self) -> String {
        let desc = self
            .inner
            .header
            .description
            .as_deref()
            .unwrap_or("unnamed");
        let flux = self.total_luminous_flux();
        let has_spd = if self.has_spectral_data() {
            ", has SPD"
        } else {
            ""
        };
        format!(
            "AtlaDocument(description='{}', flux={:.0} lm, emitters={}{})",
            desc,
            flux,
            self.inner.emitters.len(),
            has_spd
        )
    }
}

impl AtlaDocument {
    /// Get the inner ATLA document (for internal use)
    pub(crate) fn inner(&self) -> &CoreAtla {
        &self.inner
    }
}
