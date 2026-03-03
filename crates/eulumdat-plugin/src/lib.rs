//! Eulumdat WASM Plugin
//!
//! A lightweight WASM plugin for photometric analysis that can be embedded in GLDF files
//! and discovered by the plugin system. Provides parsing, calculations, and diagram generation.
//!
//! ## Plugin Manifest
//!
//! This plugin self-describes its capabilities via manifest.json, allowing the host
//! to discover available functions dynamically.

use eulumdat::{
    diagram::{
        ButterflyDiagram, CartesianDiagram, ConeDiagram, HeatmapDiagram, PolarDiagram, SvgTheme,
    },
    validate, BugDiagram, BugRating, CieFluxCodes, CuTable, Eulumdat, IesExporter, IesParser,
    PhotometricCalculations, PhotometricSummary, UgrTable, ZonalLumens30, ZoneLumens,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Plugin engine holding parsed photometric data
#[wasm_bindgen]
pub struct EulumdatEngine {
    data: Option<Eulumdat>,
}

#[wasm_bindgen]
impl EulumdatEngine {
    /// Create a new engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { data: None }
    }

    /// Check if data is loaded
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    /// Parse LDT (EULUMDAT) file content
    pub fn parse_ldt(&mut self, content: &str) -> Result<String, JsValue> {
        let ldt = Eulumdat::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let json = serde_json::to_string(&ldt).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.data = Some(ldt);
        Ok(json)
    }

    /// Parse IES file content
    pub fn parse_ies(&mut self, content: &str) -> Result<String, JsValue> {
        let ldt = IesParser::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let json = serde_json::to_string(&ldt).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.data = Some(ldt);
        Ok(json)
    }

    /// Export current data to LDT format
    pub fn export_ldt(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        Ok(ldt.to_ldt())
    }

    /// Export current data to IES format
    pub fn export_ies(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        Ok(IesExporter::export(ldt))
    }

    /// Get luminaire name
    pub fn get_name(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        Ok(ldt.luminaire_name.clone())
    }

    /// Get current data as JSON
    pub fn get_data_json(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        serde_json::to_string(ldt).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Clear loaded data
    pub fn clear(&mut self) {
        self.data = None;
    }

    // =========================================================================
    // Calculations
    // =========================================================================

    /// Calculate beam angle (angle at 50% max intensity)
    pub fn beam_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::beam_angle(ldt))
    }

    /// Calculate field angle (angle at 10% max intensity)
    pub fn field_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::field_angle(ldt))
    }

    /// Calculate half beam angle
    pub fn half_beam_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::half_beam_angle(ldt))
    }

    /// Calculate half field angle
    pub fn half_field_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::half_field_angle(ldt))
    }

    /// Calculate total luminous flux
    pub fn total_flux(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::total_output(ldt))
    }

    /// Calculate downward flux to a specific angle
    pub fn downward_flux(&self, angle: f64) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::downward_flux(ldt, angle))
    }

    /// Calculate luminaire efficacy (lm/W)
    pub fn efficacy(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::luminaire_efficacy(ldt))
    }

    /// Calculate luminaire efficiency (downward flux fraction)
    pub fn efficiency(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::luminaire_efficiency(ldt))
    }

    /// Calculate cut-off angle
    pub fn cut_off_angle(&self) -> Result<f64, JsValue> {
        let ldt = self.get_data()?;
        Ok(PhotometricCalculations::cut_off_angle(ldt))
    }

    /// Get photometric summary (all key metrics)
    pub fn get_summary(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let summary = PhotometricSummary::from_eulumdat(ldt);
        let wrapper = SummaryWrapper::from(&summary);
        serde_json::to_string(&wrapper).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get beam/field analysis
    pub fn beam_field_analysis(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let analysis = PhotometricCalculations::beam_field_analysis(ldt);
        let wrapper = BeamFieldWrapper::from(&analysis);
        serde_json::to_string(&wrapper).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate zonal lumens (30-degree zones)
    pub fn zonal_lumens_30(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let zonal = PhotometricCalculations::zonal_lumens_30deg(ldt);
        let wrapper = ZonalWrapper::from(&zonal);
        serde_json::to_string(&wrapper).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate CIE flux codes
    pub fn cie_flux_codes(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let codes = PhotometricCalculations::cie_flux_codes(ldt);
        let wrapper = CieFluxWrapper::from(&codes);
        serde_json::to_string(&wrapper).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate spacing criteria
    pub fn spacing_criteria(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let (forward, backward) = PhotometricCalculations::spacing_criteria(ldt);
        let result = SpacingCriteria { forward, backward };
        serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // =========================================================================
    // Tables
    // =========================================================================

    /// Calculate coefficient of utilization (CU) table
    pub fn cu_table(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let table = CuTable::calculate(ldt);
        let wrapper = CuTableWrapper::from(&table);
        serde_json::to_string(&wrapper).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate UGR (Unified Glare Rating) table
    pub fn ugr_table(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let table = UgrTable::calculate(ldt);
        let wrapper = UgrTableWrapper::from(&table);
        serde_json::to_string(&wrapper).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate direct ratios for standard room indices
    /// shr: Spacing to Height Ratio ("1.0", "1.5", "2.0", etc.)
    pub fn direct_ratios(&self, shr: &str) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let ratios = PhotometricCalculations::calculate_direct_ratios(ldt, shr);
        serde_json::to_string(&ratios).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // =========================================================================
    // BUG Rating
    // =========================================================================

    /// Calculate BUG rating
    pub fn bug_rating(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let rating = BugRating::from_eulumdat(ldt);
        serde_json::to_string(&rating).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate zone lumens for BUG rating
    pub fn zone_lumens(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let zones = ZoneLumens::from_eulumdat(ldt);
        serde_json::to_string(&zones).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Get BUG diagram data
    pub fn bug_diagram_data(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = BugDiagram::from_eulumdat(ldt);
        serde_json::to_string(&diagram).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate photometric data
    pub fn validate(&self) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let warnings = validate(ldt);
        let wrappers: Vec<_> = warnings.iter().map(ValidationWrapper::from).collect();
        serde_json::to_string(&wrappers).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    // =========================================================================
    // SVG Diagram Generation
    // =========================================================================

    /// Generate polar diagram SVG
    pub fn polar_svg(&self, width: f64, height: f64, theme: &str) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = PolarDiagram::from_eulumdat(ldt);
        let theme = parse_theme(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    /// Generate Cartesian diagram SVG
    pub fn cartesian_svg(
        &self,
        width: f64,
        height: f64,
        theme: &str,
        max_curves: usize,
    ) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = CartesianDiagram::from_eulumdat(ldt, width, height, max_curves);
        let theme = parse_theme(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    /// Generate butterfly diagram SVG
    pub fn butterfly_svg(
        &self,
        width: f64,
        height: f64,
        theme: &str,
        rotation: f64,
    ) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = ButterflyDiagram::from_eulumdat(ldt, width, height, rotation);
        let theme = parse_theme(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    /// Generate heatmap diagram SVG
    pub fn heatmap_svg(&self, width: f64, height: f64, theme: &str) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = HeatmapDiagram::from_eulumdat(ldt, width, height);
        let theme = parse_theme(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    /// Generate cone diagram SVG
    /// mounting_height: luminaire mounting height in meters (default 3.0)
    pub fn cone_svg(
        &self,
        width: f64,
        height: f64,
        theme: &str,
        mounting_height: f64,
    ) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = ConeDiagram::from_eulumdat(ldt, mounting_height);
        let theme = parse_theme(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    /// Generate BUG rating diagram SVG (TM-15-11)
    pub fn bug_svg(&self, width: f64, height: f64, theme: &str) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = BugDiagram::from_eulumdat(ldt);
        let theme = parse_theme(theme);
        Ok(diagram.to_svg(width, height, &theme))
    }

    /// Generate LCS classification diagram SVG (TM-15-07)
    pub fn lcs_svg(&self, width: f64, height: f64, theme: &str) -> Result<String, JsValue> {
        let ldt = self.get_data()?;
        let diagram = BugDiagram::from_eulumdat(ldt);
        let theme = parse_theme(theme);
        Ok(diagram.to_lcs_svg(width, height, &theme))
    }

    // =========================================================================
    // Helper
    // =========================================================================

    fn get_data(&self) -> Result<&Eulumdat, JsValue> {
        self.data.as_ref().ok_or_else(|| {
            JsValue::from_str("No data loaded. Call parse_ldt() or parse_ies() first.")
        })
    }
}

impl Default for EulumdatEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Static Functions (stateless operations)
// =============================================================================

/// Parse LDT content (stateless)
#[wasm_bindgen]
pub fn parse_ldt(content: &str) -> Result<String, JsValue> {
    let ldt = Eulumdat::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&ldt).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse IES content (stateless)
#[wasm_bindgen]
pub fn parse_ies(content: &str) -> Result<String, JsValue> {
    let ldt = IesParser::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&ldt).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate LDT content (stateless)
#[wasm_bindgen]
pub fn validate_ldt(content: &str) -> String {
    match Eulumdat::parse(content) {
        Ok(ldt) => {
            let warnings = validate(&ldt);
            let wrappers: Vec<_> = warnings.iter().map(ValidationWrapper::from).collect();
            serde_json::to_string(&wrappers).unwrap_or_else(|_| "[]".to_string())
        }
        Err(e) => {
            let errors = vec![ValidationWrapper {
                code: "PARSE_ERROR".to_string(),
                message: e.to_string(),
            }];
            serde_json::to_string(&errors).unwrap_or_else(|_| "[]".to_string())
        }
    }
}

/// Convert LDT to IES (stateless)
#[wasm_bindgen]
pub fn ldt_to_ies(content: &str) -> Result<String, JsValue> {
    let ldt = Eulumdat::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(IesExporter::export(&ldt))
}

/// Convert IES to LDT (stateless)
#[wasm_bindgen]
pub fn ies_to_ldt(content: &str) -> Result<String, JsValue> {
    let ldt = IesParser::parse(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(ldt.to_ldt())
}

/// Get plugin info
#[wasm_bindgen]
pub fn engine_info() -> String {
    serde_json::to_string(&EngineInfo {
        name: "Eulumdat Photometric Engine".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Parse, analyze, and visualize LDT/IES photometric files".to_string(),
    })
    .unwrap()
}

// =============================================================================
// Serializable Wrapper Types
// =============================================================================

#[derive(Serialize, Deserialize)]
struct SpacingCriteria {
    forward: f64,
    backward: f64,
}

#[derive(Serialize, Deserialize)]
struct EngineInfo {
    name: String,
    version: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
struct ValidationWrapper {
    code: String,
    message: String,
}

impl From<&eulumdat::ValidationWarning> for ValidationWrapper {
    fn from(w: &eulumdat::ValidationWarning) -> Self {
        Self {
            code: w.code.to_string(),
            message: w.message.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SummaryWrapper {
    total_lamp_flux: f64,
    calculated_flux: f64,
    lor: f64,
    dlor: f64,
    ulor: f64,
    lamp_efficacy: f64,
    luminaire_efficacy: f64,
    total_wattage: f64,
    beam_angle: f64,
    field_angle: f64,
    beam_angle_cie: f64,
    field_angle_cie: f64,
    is_batwing: bool,
    max_intensity: f64,
    min_intensity: f64,
    avg_intensity: f64,
    spacing_c0: f64,
    spacing_c90: f64,
}

impl From<&PhotometricSummary> for SummaryWrapper {
    fn from(s: &PhotometricSummary) -> Self {
        Self {
            total_lamp_flux: s.total_lamp_flux,
            calculated_flux: s.calculated_flux,
            lor: s.lor,
            dlor: s.dlor,
            ulor: s.ulor,
            lamp_efficacy: s.lamp_efficacy,
            luminaire_efficacy: s.luminaire_efficacy,
            total_wattage: s.total_wattage,
            beam_angle: s.beam_angle,
            field_angle: s.field_angle,
            beam_angle_cie: s.beam_angle_cie,
            field_angle_cie: s.field_angle_cie,
            is_batwing: s.is_batwing,
            max_intensity: s.max_intensity,
            min_intensity: s.min_intensity,
            avg_intensity: s.avg_intensity,
            spacing_c0: s.spacing_c0,
            spacing_c90: s.spacing_c90,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct BeamFieldWrapper {
    beam_angle_ies: f64,
    field_angle_ies: f64,
    beam_angle_cie: f64,
    field_angle_cie: f64,
    max_intensity: f64,
    center_intensity: f64,
    max_intensity_gamma: f64,
    is_batwing: bool,
    beam_threshold_ies: f64,
    beam_threshold_cie: f64,
    field_threshold_ies: f64,
    field_threshold_cie: f64,
}

impl From<&eulumdat::BeamFieldAnalysis> for BeamFieldWrapper {
    fn from(a: &eulumdat::BeamFieldAnalysis) -> Self {
        Self {
            beam_angle_ies: a.beam_angle_ies,
            field_angle_ies: a.field_angle_ies,
            beam_angle_cie: a.beam_angle_cie,
            field_angle_cie: a.field_angle_cie,
            max_intensity: a.max_intensity,
            center_intensity: a.center_intensity,
            max_intensity_gamma: a.max_intensity_gamma,
            is_batwing: a.is_batwing,
            beam_threshold_ies: a.beam_threshold_ies,
            beam_threshold_cie: a.beam_threshold_cie,
            field_threshold_ies: a.field_threshold_ies,
            field_threshold_cie: a.field_threshold_cie,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ZonalWrapper {
    zone_0_30: f64,
    zone_30_60: f64,
    zone_60_90: f64,
    zone_90_120: f64,
    zone_120_150: f64,
    zone_150_180: f64,
    downward_total: f64,
    upward_total: f64,
}

impl From<&ZonalLumens30> for ZonalWrapper {
    fn from(z: &ZonalLumens30) -> Self {
        Self {
            zone_0_30: z.zone_0_30,
            zone_30_60: z.zone_30_60,
            zone_60_90: z.zone_60_90,
            zone_90_120: z.zone_90_120,
            zone_120_150: z.zone_120_150,
            zone_150_180: z.zone_150_180,
            downward_total: z.downward_total(),
            upward_total: z.upward_total(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CieFluxWrapper {
    n1: f64,
    n2: f64,
    n3: f64,
    n4: f64,
    n5: f64,
}

impl From<&CieFluxCodes> for CieFluxWrapper {
    fn from(c: &CieFluxCodes) -> Self {
        Self {
            n1: c.n1,
            n2: c.n2,
            n3: c.n3,
            n4: c.n4,
            n5: c.n5,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CuTableWrapper {
    rcr_values: Vec<u8>,
    reflectances: Vec<ReflectanceSet>,
    values: Vec<Vec<f64>>,
}

#[derive(Serialize, Deserialize)]
struct ReflectanceSet {
    ceiling: u8,
    wall: u8,
    floor: u8,
}

impl From<&CuTable> for CuTableWrapper {
    fn from(t: &CuTable) -> Self {
        Self {
            rcr_values: t.rcr_values.clone(),
            reflectances: t
                .reflectances
                .iter()
                .map(|r| ReflectanceSet {
                    ceiling: r.0,
                    wall: r.1,
                    floor: r.2,
                })
                .collect(),
            values: t.values.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UgrTableWrapper {
    room_sizes: Vec<RoomSize>,
    reflectances: Vec<ReflectanceSet>,
    crosswise: Vec<Vec<f64>>,
    endwise: Vec<Vec<f64>>,
    max_ugr: f64,
}

#[derive(Serialize, Deserialize)]
struct RoomSize {
    x: f64,
    y: f64,
}

impl From<&UgrTable> for UgrTableWrapper {
    fn from(t: &UgrTable) -> Self {
        Self {
            room_sizes: t
                .room_sizes
                .iter()
                .map(|r| RoomSize { x: r.0, y: r.1 })
                .collect(),
            reflectances: t
                .reflectances
                .iter()
                .map(|r| ReflectanceSet {
                    ceiling: r.0,
                    wall: r.1,
                    floor: r.2,
                })
                .collect(),
            crosswise: t.crosswise.clone(),
            endwise: t.endwise.clone(),
            max_ugr: t.max_ugr,
        }
    }
}

/// Parse theme string to SvgTheme
fn parse_theme(theme: &str) -> SvgTheme {
    match theme.to_lowercase().as_str() {
        "dark" => SvgTheme::dark(),
        "css" => SvgTheme::css_variables(),
        _ => SvgTheme::light(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = EulumdatEngine::new();
        assert!(!engine.has_data());
    }

    #[test]
    fn test_engine_info() {
        let info = engine_info();
        assert!(info.contains("Eulumdat"));
    }
}
