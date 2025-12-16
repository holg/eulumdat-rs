//! Photometric calculations Python bindings

use pyo3::prelude::*;
use pyo3::types::PyDict;

use ::eulumdat as core;
use core::{
    CieFluxCodes as CoreCieFluxCodes, GldfPhotometricData as CoreGldfPhotometricData,
    PhotometricCalculations as CoreCalcs, PhotometricSummary as CorePhotoSummary,
    UgrParams as CoreUgrParams, ZonalLumens30 as CoreZonalLumens30,
};

/// CIE Flux Code values (N1-N5).
///
/// The CIE flux code describes the light distribution of a luminaire:
/// - N1: % flux in lower hemisphere (0-90°) - equivalent to DLOR
/// - N2: % flux in 0-60° zone
/// - N3: % flux in 0-40° zone
/// - N4: % flux in upper hemisphere (90-180°) - equivalent to ULOR
/// - N5: % flux in 90-120° zone (near-horizontal uplight)
#[pyclass]
#[derive(Clone, Debug)]
pub struct CieFluxCodes {
    /// N1: % flux in lower hemisphere (0-90°)
    #[pyo3(get)]
    pub n1: f64,
    /// N2: % flux in 0-60° zone
    #[pyo3(get)]
    pub n2: f64,
    /// N3: % flux in 0-40° zone
    #[pyo3(get)]
    pub n3: f64,
    /// N4: % flux in upper hemisphere (90-180°)
    #[pyo3(get)]
    pub n4: f64,
    /// N5: % flux in 90-120° zone
    #[pyo3(get)]
    pub n5: f64,
}

#[pymethods]
impl CieFluxCodes {
    /// Format as standard CIE flux code string "N1 N2 N3 N4 N5"
    fn __str__(&self) -> String {
        format!(
            "{:.0} {:.0} {:.0} {:.0} {:.0}",
            self.n1.round(),
            self.n2.round(),
            self.n3.round(),
            self.n4.round(),
            self.n5.round()
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "CieFluxCodes(n1={:.1}, n2={:.1}, n3={:.1}, n4={:.1}, n5={:.1})",
            self.n1, self.n2, self.n3, self.n4, self.n5
        )
    }

    /// Convert to dictionary
    fn to_dict<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("n1", self.n1).unwrap();
        dict.set_item("n2", self.n2).unwrap();
        dict.set_item("n3", self.n3).unwrap();
        dict.set_item("n4", self.n4).unwrap();
        dict.set_item("n5", self.n5).unwrap();
        dict
    }
}

impl From<CoreCieFluxCodes> for CieFluxCodes {
    fn from(c: CoreCieFluxCodes) -> Self {
        Self {
            n1: c.n1,
            n2: c.n2,
            n3: c.n3,
            n4: c.n4,
            n5: c.n5,
        }
    }
}

/// Zonal lumens in 30° zones.
///
/// Flux percentages distributed across 6 zones from nadir to zenith.
#[pyclass]
#[derive(Clone, Debug)]
pub struct ZonalLumens30 {
    /// 0-30° zone (nadir to 30°)
    #[pyo3(get)]
    pub zone_0_30: f64,
    /// 30-60° zone
    #[pyo3(get)]
    pub zone_30_60: f64,
    /// 60-90° zone (approaching horizontal)
    #[pyo3(get)]
    pub zone_60_90: f64,
    /// 90-120° zone (above horizontal)
    #[pyo3(get)]
    pub zone_90_120: f64,
    /// 120-150° zone
    #[pyo3(get)]
    pub zone_120_150: f64,
    /// 150-180° zone (zenith region)
    #[pyo3(get)]
    pub zone_150_180: f64,
}

#[pymethods]
impl ZonalLumens30 {
    /// Get total downward flux (0-90°)
    fn downward_total(&self) -> f64 {
        self.zone_0_30 + self.zone_30_60 + self.zone_60_90
    }

    /// Get total upward flux (90-180°)
    fn upward_total(&self) -> f64 {
        self.zone_90_120 + self.zone_120_150 + self.zone_150_180
    }

    fn __repr__(&self) -> String {
        format!(
            "ZonalLumens30(down={:.1}%, up={:.1}%)",
            self.downward_total(),
            self.upward_total()
        )
    }

    /// Convert to dictionary
    fn to_dict<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("zone_0_30", self.zone_0_30).unwrap();
        dict.set_item("zone_30_60", self.zone_30_60).unwrap();
        dict.set_item("zone_60_90", self.zone_60_90).unwrap();
        dict.set_item("zone_90_120", self.zone_90_120).unwrap();
        dict.set_item("zone_120_150", self.zone_120_150).unwrap();
        dict.set_item("zone_150_180", self.zone_150_180).unwrap();
        dict
    }
}

impl From<CoreZonalLumens30> for ZonalLumens30 {
    fn from(z: CoreZonalLumens30) -> Self {
        Self {
            zone_0_30: z.zone_0_30,
            zone_30_60: z.zone_30_60,
            zone_60_90: z.zone_60_90,
            zone_90_120: z.zone_90_120,
            zone_120_150: z.zone_120_150,
            zone_150_180: z.zone_150_180,
        }
    }
}

/// Complete photometric summary with all calculated values.
///
/// Provides a comprehensive overview of luminaire performance
/// that can be used for reports, GLDF export, or display.
#[pyclass]
#[derive(Clone, Debug)]
pub struct PhotometricSummary {
    // Flux and efficiency
    /// Total lamp flux (lm)
    #[pyo3(get)]
    pub total_lamp_flux: f64,
    /// Calculated flux from intensity integration (lm)
    #[pyo3(get)]
    pub calculated_flux: f64,
    /// Light Output Ratio (%)
    #[pyo3(get)]
    pub lor: f64,
    /// Downward Light Output Ratio (%)
    #[pyo3(get)]
    pub dlor: f64,
    /// Upward Light Output Ratio (%)
    #[pyo3(get)]
    pub ulor: f64,

    // Efficacy
    /// Lamp efficacy (lm/W)
    #[pyo3(get)]
    pub lamp_efficacy: f64,
    /// Luminaire efficacy (lm/W)
    #[pyo3(get)]
    pub luminaire_efficacy: f64,
    /// Total system wattage (W)
    #[pyo3(get)]
    pub total_wattage: f64,

    // Beam characteristics
    /// Beam angle - 50% intensity (degrees)
    #[pyo3(get)]
    pub beam_angle: f64,
    /// Field angle - 10% intensity (degrees)
    #[pyo3(get)]
    pub field_angle: f64,

    // Intensity statistics
    /// Maximum intensity (cd/klm)
    #[pyo3(get)]
    pub max_intensity: f64,
    /// Minimum intensity (cd/klm)
    #[pyo3(get)]
    pub min_intensity: f64,
    /// Average intensity (cd/klm)
    #[pyo3(get)]
    pub avg_intensity: f64,

    // Spacing criterion
    /// S/H ratio for C0 plane
    #[pyo3(get)]
    pub spacing_c0: f64,
    /// S/H ratio for C90 plane
    #[pyo3(get)]
    pub spacing_c90: f64,

    // Internal storage for cie_flux_codes and zonal_lumens
    inner_cie_codes: CoreCieFluxCodes,
    inner_zonal_lumens: CoreZonalLumens30,
}

#[pymethods]
impl PhotometricSummary {
    /// CIE flux codes (N1-N5)
    #[getter]
    fn cie_flux_codes(&self) -> CieFluxCodes {
        self.inner_cie_codes.into()
    }

    /// Zonal lumens in 30° zones
    #[getter]
    fn zonal_lumens(&self) -> ZonalLumens30 {
        self.inner_zonal_lumens.into()
    }

    /// Format as multi-line text report.
    fn to_text(&self) -> String {
        format!(
            r#"PHOTOMETRIC SUMMARY
==================

Luminous Flux
  Total Lamp Flux:     {:.0} lm
  Calculated Flux:     {:.0} lm
  LOR:                 {:.1}%
  DLOR / ULOR:         {:.1}% / {:.1}%

Efficacy
  Lamp Efficacy:       {:.1} lm/W
  Luminaire Efficacy:  {:.1} lm/W
  Total Wattage:       {:.1} W

CIE Flux Code:         {}

Beam Characteristics
  Beam Angle (50%):    {:.1}°
  Field Angle (10%):   {:.1}°

Intensity (cd/klm)
  Maximum:             {:.1}
  Minimum:             {:.1}
  Average:             {:.1}

Spacing Criterion (S/H)
  C0 Plane:            {:.2}
  C90 Plane:           {:.2}

Zonal Lumens (%)
  0-30°:               {:.1}%
  30-60°:              {:.1}%
  60-90°:              {:.1}%
  90-120°:             {:.1}%
  120-150°:            {:.1}%
  150-180°:            {:.1}%
"#,
            self.total_lamp_flux,
            self.calculated_flux,
            self.lor,
            self.dlor,
            self.ulor,
            self.lamp_efficacy,
            self.luminaire_efficacy,
            self.total_wattage,
            self.inner_cie_codes,
            self.beam_angle,
            self.field_angle,
            self.max_intensity,
            self.min_intensity,
            self.avg_intensity,
            self.spacing_c0,
            self.spacing_c90,
            self.inner_zonal_lumens.zone_0_30,
            self.inner_zonal_lumens.zone_30_60,
            self.inner_zonal_lumens.zone_60_90,
            self.inner_zonal_lumens.zone_90_120,
            self.inner_zonal_lumens.zone_120_150,
            self.inner_zonal_lumens.zone_150_180,
        )
    }

    /// Format as single-line compact summary.
    fn to_compact(&self) -> String {
        format!(
            "CIE:{} Beam:{:.0}° Field:{:.0}° Eff:{:.0}lm/W S/H:{:.1}×{:.1}",
            self.inner_cie_codes,
            self.beam_angle,
            self.field_angle,
            self.luminaire_efficacy,
            self.spacing_c0,
            self.spacing_c90,
        )
    }

    fn __str__(&self) -> String {
        self.to_text()
    }

    fn __repr__(&self) -> String {
        format!(
            "PhotometricSummary(flux={:.0}lm, beam={:.1}°, field={:.1}°, eff={:.1}lm/W)",
            self.total_lamp_flux, self.beam_angle, self.field_angle, self.luminaire_efficacy
        )
    }

    /// Convert to dictionary
    fn to_dict<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("total_lamp_flux", self.total_lamp_flux)
            .unwrap();
        dict.set_item("calculated_flux", self.calculated_flux)
            .unwrap();
        dict.set_item("lor", self.lor).unwrap();
        dict.set_item("dlor", self.dlor).unwrap();
        dict.set_item("ulor", self.ulor).unwrap();
        dict.set_item("lamp_efficacy", self.lamp_efficacy).unwrap();
        dict.set_item("luminaire_efficacy", self.luminaire_efficacy)
            .unwrap();
        dict.set_item("total_wattage", self.total_wattage).unwrap();
        dict.set_item("beam_angle", self.beam_angle).unwrap();
        dict.set_item("field_angle", self.field_angle).unwrap();
        dict.set_item("max_intensity", self.max_intensity).unwrap();
        dict.set_item("min_intensity", self.min_intensity).unwrap();
        dict.set_item("avg_intensity", self.avg_intensity).unwrap();
        dict.set_item("spacing_c0", self.spacing_c0).unwrap();
        dict.set_item("spacing_c90", self.spacing_c90).unwrap();
        dict.set_item(
            "cie_flux_code",
            format!("{}", self.inner_cie_codes).as_str(),
        )
        .unwrap();
        dict.set_item("cie_n1", self.inner_cie_codes.n1).unwrap();
        dict.set_item("cie_n2", self.inner_cie_codes.n2).unwrap();
        dict.set_item("cie_n3", self.inner_cie_codes.n3).unwrap();
        dict.set_item("cie_n4", self.inner_cie_codes.n4).unwrap();
        dict.set_item("cie_n5", self.inner_cie_codes.n5).unwrap();
        dict.set_item("zonal_0_30", self.inner_zonal_lumens.zone_0_30)
            .unwrap();
        dict.set_item("zonal_30_60", self.inner_zonal_lumens.zone_30_60)
            .unwrap();
        dict.set_item("zonal_60_90", self.inner_zonal_lumens.zone_60_90)
            .unwrap();
        dict.set_item("zonal_90_120", self.inner_zonal_lumens.zone_90_120)
            .unwrap();
        dict.set_item("zonal_120_150", self.inner_zonal_lumens.zone_120_150)
            .unwrap();
        dict.set_item("zonal_150_180", self.inner_zonal_lumens.zone_150_180)
            .unwrap();
        dict
    }
}

impl From<CorePhotoSummary> for PhotometricSummary {
    fn from(s: CorePhotoSummary) -> Self {
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
            max_intensity: s.max_intensity,
            min_intensity: s.min_intensity,
            avg_intensity: s.avg_intensity,
            spacing_c0: s.spacing_c0,
            spacing_c90: s.spacing_c90,
            inner_cie_codes: s.cie_flux_codes,
            inner_zonal_lumens: s.zonal_lumens,
        }
    }
}

/// GLDF-compatible photometric data export.
///
/// Contains all properties required by the GLDF (Global Lighting Data Format)
/// specification for photometric data exchange.
#[pyclass]
#[derive(Clone, Debug)]
pub struct GldfPhotometricData {
    /// CIE Flux Code (e.g., "45 72 95 100 100")
    #[pyo3(get)]
    pub cie_flux_code: String,
    /// Light Output Ratio - total efficiency (%)
    #[pyo3(get)]
    pub light_output_ratio: f64,
    /// Luminous efficacy (lm/W)
    #[pyo3(get)]
    pub luminous_efficacy: f64,
    /// Downward Flux Fraction (%)
    #[pyo3(get)]
    pub downward_flux_fraction: f64,
    /// Downward Light Output Ratio (%)
    #[pyo3(get)]
    pub downward_light_output_ratio: f64,
    /// Upward Light Output Ratio (%)
    #[pyo3(get)]
    pub upward_light_output_ratio: f64,
    /// Luminaire luminance (cd/m²)
    #[pyo3(get)]
    pub luminaire_luminance: f64,
    /// Cut-off angle (degrees)
    #[pyo3(get)]
    pub cut_off_angle: f64,
    /// Photometric classification code
    #[pyo3(get)]
    pub photometric_code: String,
    /// Half peak (beam) divergence C0 (degrees)
    #[pyo3(get)]
    pub half_peak_c0: f64,
    /// Half peak (beam) divergence C90 (degrees)
    #[pyo3(get)]
    pub half_peak_c90: f64,
    /// Tenth peak (field) divergence C0 (degrees)
    #[pyo3(get)]
    pub tenth_peak_c0: f64,
    /// Tenth peak (field) divergence C90 (degrees)
    #[pyo3(get)]
    pub tenth_peak_c90: f64,
    /// BUG rating string
    #[pyo3(get)]
    pub bug_rating: String,
    /// UGR crosswise (if available)
    #[pyo3(get)]
    pub ugr_crosswise: Option<f64>,
    /// UGR endwise (if available)
    #[pyo3(get)]
    pub ugr_endwise: Option<f64>,
}

#[pymethods]
impl GldfPhotometricData {
    /// Format as text report
    fn to_text(&self) -> String {
        let mut s = String::from("GLDF PHOTOMETRIC DATA\n");
        s.push_str("=====================\n\n");

        s.push_str(&format!(
            "CIE Flux Code:           {}\n",
            self.cie_flux_code
        ));
        s.push_str(&format!(
            "Light Output Ratio:      {:.1}%\n",
            self.light_output_ratio
        ));
        s.push_str(&format!(
            "Luminous Efficacy:       {:.1} lm/W\n",
            self.luminous_efficacy
        ));
        s.push_str(&format!(
            "Downward Flux Fraction:  {:.1}%\n",
            self.downward_flux_fraction
        ));
        s.push_str(&format!(
            "DLOR:                    {:.1}%\n",
            self.downward_light_output_ratio
        ));
        s.push_str(&format!(
            "ULOR:                    {:.1}%\n",
            self.upward_light_output_ratio
        ));
        s.push_str(&format!(
            "Luminaire Luminance:     {:.0} cd/m²\n",
            self.luminaire_luminance
        ));
        s.push_str(&format!(
            "Cut-off Angle:           {:.1}°\n",
            self.cut_off_angle
        ));

        if let (Some(cross), Some(end)) = (self.ugr_crosswise, self.ugr_endwise) {
            s.push_str(&format!(
                "UGR (4H×8H, 70/50/20):   C: {:.1} / E: {:.1}\n",
                cross, end
            ));
        }

        s.push_str(&format!(
            "Photometric Code:        {}\n",
            self.photometric_code
        ));
        s.push_str(&format!(
            "Half Peak Divergence:    {:.1}° / {:.1}° (C0/C90)\n",
            self.half_peak_c0, self.half_peak_c90
        ));
        s.push_str(&format!(
            "Tenth Peak Divergence:   {:.1}° / {:.1}° (C0/C90)\n",
            self.tenth_peak_c0, self.tenth_peak_c90
        ));
        s.push_str(&format!("BUG Rating:              {}\n", self.bug_rating));

        s
    }

    fn __str__(&self) -> String {
        self.to_text()
    }

    fn __repr__(&self) -> String {
        format!(
            "GldfPhotometricData(cie='{}', lor={:.1}%, eff={:.1}lm/W, bug='{}')",
            self.cie_flux_code, self.light_output_ratio, self.luminous_efficacy, self.bug_rating
        )
    }

    /// Convert to dictionary for JSON serialization
    fn to_dict<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        let dict = PyDict::new(py);
        dict.set_item("cie_flux_code", &self.cie_flux_code).unwrap();
        dict.set_item("light_output_ratio", self.light_output_ratio)
            .unwrap();
        dict.set_item("luminous_efficacy", self.luminous_efficacy)
            .unwrap();
        dict.set_item("downward_flux_fraction", self.downward_flux_fraction)
            .unwrap();
        dict.set_item(
            "downward_light_output_ratio",
            self.downward_light_output_ratio,
        )
        .unwrap();
        dict.set_item("upward_light_output_ratio", self.upward_light_output_ratio)
            .unwrap();
        dict.set_item("luminaire_luminance", self.luminaire_luminance)
            .unwrap();
        dict.set_item("cut_off_angle", self.cut_off_angle).unwrap();
        dict.set_item("photometric_code", &self.photometric_code)
            .unwrap();
        dict.set_item("half_peak_divergence_c0", self.half_peak_c0)
            .unwrap();
        dict.set_item("half_peak_divergence_c90", self.half_peak_c90)
            .unwrap();
        dict.set_item("tenth_peak_divergence_c0", self.tenth_peak_c0)
            .unwrap();
        dict.set_item("tenth_peak_divergence_c90", self.tenth_peak_c90)
            .unwrap();
        dict.set_item("bug_rating", &self.bug_rating).unwrap();
        if let Some(ugr_c) = self.ugr_crosswise {
            dict.set_item("ugr_crosswise", ugr_c).unwrap();
        }
        if let Some(ugr_e) = self.ugr_endwise {
            dict.set_item("ugr_endwise", ugr_e).unwrap();
        }
        dict
    }
}

impl From<CoreGldfPhotometricData> for GldfPhotometricData {
    fn from(g: CoreGldfPhotometricData) -> Self {
        Self {
            cie_flux_code: g.cie_flux_code,
            light_output_ratio: g.light_output_ratio,
            luminous_efficacy: g.luminous_efficacy,
            downward_flux_fraction: g.downward_flux_fraction,
            downward_light_output_ratio: g.downward_light_output_ratio,
            upward_light_output_ratio: g.upward_light_output_ratio,
            luminaire_luminance: g.luminaire_luminance,
            cut_off_angle: g.cut_off_angle,
            photometric_code: g.photometric_code,
            half_peak_c0: g.half_peak_divergence.0,
            half_peak_c90: g.half_peak_divergence.1,
            tenth_peak_c0: g.tenth_peak_divergence.0,
            tenth_peak_c90: g.tenth_peak_divergence.1,
            bug_rating: g.light_distribution_bug_rating,
            ugr_crosswise: g.ugr_4h_8h_705020.as_ref().map(|u| u.crosswise),
            ugr_endwise: g.ugr_4h_8h_705020.as_ref().map(|u| u.endwise),
        }
    }
}

/// Parameters for UGR calculation.
#[pyclass]
#[derive(Clone, Debug)]
pub struct UgrParams {
    /// Room length (m)
    #[pyo3(get, set)]
    pub room_length: f64,
    /// Room width (m)
    #[pyo3(get, set)]
    pub room_width: f64,
    /// Mounting height above floor (m)
    #[pyo3(get, set)]
    pub mounting_height: f64,
    /// Observer eye height (m)
    #[pyo3(get, set)]
    pub eye_height: f64,
    /// Observer X position (m)
    #[pyo3(get, set)]
    pub observer_x: f64,
    /// Observer Y position (m)
    #[pyo3(get, set)]
    pub observer_y: f64,
    /// Ceiling reflectance (0-1)
    #[pyo3(get, set)]
    pub rho_ceiling: f64,
    /// Wall reflectance (0-1)
    #[pyo3(get, set)]
    pub rho_wall: f64,
    /// Floor reflectance (0-1)
    #[pyo3(get, set)]
    pub rho_floor: f64,
    /// Target illuminance (lux)
    #[pyo3(get, set)]
    pub illuminance: f64,
    /// Luminaire positions as (x, y) tuples
    luminaire_positions: Vec<(f64, f64)>,
}

#[pymethods]
impl UgrParams {
    /// Create new UGR parameters with default values.
    #[new]
    #[pyo3(signature = (room_length=8.0, room_width=4.0, mounting_height=2.8, eye_height=1.2, observer_x=4.0, observer_y=2.0))]
    fn new(
        room_length: f64,
        room_width: f64,
        mounting_height: f64,
        eye_height: f64,
        observer_x: f64,
        observer_y: f64,
    ) -> Self {
        Self {
            room_length,
            room_width,
            mounting_height,
            eye_height,
            observer_x,
            observer_y,
            luminaire_positions: vec![(2.0, 2.0), (6.0, 2.0)],
            rho_ceiling: 0.7,
            rho_wall: 0.5,
            rho_floor: 0.2,
            illuminance: 500.0,
        }
    }

    /// Create params for a standard office room.
    #[staticmethod]
    fn standard_office() -> Self {
        let core = CoreUgrParams::standard_office();
        Self::from(core)
    }

    /// Get luminaire positions
    #[getter]
    fn get_luminaire_positions(&self) -> Vec<(f64, f64)> {
        self.luminaire_positions.clone()
    }

    /// Set luminaire positions
    #[setter]
    fn set_luminaire_positions(&mut self, positions: Vec<(f64, f64)>) {
        self.luminaire_positions = positions;
    }

    /// Add a luminaire position
    fn add_luminaire(&mut self, x: f64, y: f64) {
        self.luminaire_positions.push((x, y));
    }

    /// Clear all luminaire positions
    fn clear_luminaires(&mut self) {
        self.luminaire_positions.clear();
    }

    fn __repr__(&self) -> String {
        format!(
            "UgrParams(room={:.1}×{:.1}m, h={:.1}m, {} luminaires)",
            self.room_length,
            self.room_width,
            self.mounting_height,
            self.luminaire_positions.len()
        )
    }
}

impl From<CoreUgrParams> for UgrParams {
    fn from(p: CoreUgrParams) -> Self {
        Self {
            room_length: p.room_length,
            room_width: p.room_width,
            mounting_height: p.mounting_height,
            eye_height: p.eye_height,
            observer_x: p.observer_x,
            observer_y: p.observer_y,
            luminaire_positions: p.luminaire_positions,
            rho_ceiling: p.rho_ceiling,
            rho_wall: p.rho_wall,
            rho_floor: p.rho_floor,
            illuminance: p.illuminance,
        }
    }
}

impl From<&UgrParams> for CoreUgrParams {
    fn from(p: &UgrParams) -> Self {
        Self {
            room_length: p.room_length,
            room_width: p.room_width,
            mounting_height: p.mounting_height,
            eye_height: p.eye_height,
            observer_x: p.observer_x,
            observer_y: p.observer_y,
            luminaire_positions: p.luminaire_positions.clone(),
            rho_ceiling: p.rho_ceiling,
            rho_wall: p.rho_wall,
            rho_floor: p.rho_floor,
            illuminance: p.illuminance,
        }
    }
}

/// Helper functions for photometric calculations (implemented on Eulumdat)
pub struct PhotometricCalcs;

impl PhotometricCalcs {
    pub fn photometric_summary(ldt: &core::Eulumdat) -> PhotometricSummary {
        CorePhotoSummary::from_eulumdat(ldt).into()
    }

    pub fn gldf_data(ldt: &core::Eulumdat) -> GldfPhotometricData {
        CoreGldfPhotometricData::from_eulumdat(ldt).into()
    }

    pub fn cie_flux_codes(ldt: &core::Eulumdat) -> CieFluxCodes {
        CoreCalcs::cie_flux_codes(ldt).into()
    }

    pub fn zonal_lumens_30(ldt: &core::Eulumdat) -> ZonalLumens30 {
        CoreCalcs::zonal_lumens_30deg(ldt).into()
    }

    pub fn beam_angle(ldt: &core::Eulumdat) -> f64 {
        CoreCalcs::beam_angle(ldt)
    }

    pub fn field_angle(ldt: &core::Eulumdat) -> f64 {
        CoreCalcs::field_angle(ldt)
    }

    pub fn spacing_criteria(ldt: &core::Eulumdat) -> (f64, f64) {
        CoreCalcs::spacing_criteria(ldt)
    }

    pub fn downward_flux(ldt: &core::Eulumdat, arc: f64) -> f64 {
        CoreCalcs::downward_flux(ldt, arc)
    }

    pub fn cut_off_angle(ldt: &core::Eulumdat) -> f64 {
        CoreCalcs::cut_off_angle(ldt)
    }

    pub fn photometric_code(ldt: &core::Eulumdat) -> String {
        CoreCalcs::photometric_code(ldt)
    }

    pub fn luminaire_efficacy(ldt: &core::Eulumdat) -> f64 {
        CoreCalcs::luminaire_efficacy(ldt)
    }

    pub fn ugr(ldt: &core::Eulumdat, params: &UgrParams) -> f64 {
        let core_params = CoreUgrParams::from(params);
        CoreCalcs::ugr(ldt, &core_params)
    }
}
