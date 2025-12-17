//! Core types for ATLA S001 / ANSI/IES TM-33 / UNI 11733 luminaire optical data
//!
//! This module defines the data structures that represent luminaire optical data
//! as specified in the ATLA S001 standard (equivalent to TM-33-18 / UNI 11733:2019).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Root document for ATLA S001 / TM-33 / UNI 11733 luminaire optical data
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LuminaireOpticalData {
    /// Schema version (e.g., "1.0")
    pub version: String,

    /// Required header information
    pub header: Header,

    /// Optional luminaire physical description
    pub luminaire: Option<Luminaire>,

    /// Optional measurement equipment information
    pub equipment: Option<Equipment>,

    /// Required emitter(s) information - at least one
    pub emitters: Vec<Emitter>,

    /// Optional application-specific custom data
    pub custom_data: Option<CustomData>,
}

/// Header section containing general luminaire identification
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Header {
    /// Manufacturer name
    pub manufacturer: Option<String>,

    /// Catalog/product number
    pub catalog_number: Option<String>,

    /// Product description
    pub description: Option<String>,

    /// Global Trade Item Number (GTIN/UPC/EAN)
    pub gtin: Option<String>,

    /// Universally Unique Identifier for version control
    pub uuid: Option<String>,

    /// Reference to related documents
    pub reference: Option<String>,

    /// URI for additional product information
    pub more_info_uri: Option<String>,

    /// Test laboratory name
    pub laboratory: Option<String>,

    /// Test report number
    pub report_number: Option<String>,

    /// Test date (ISO 8601 format)
    pub test_date: Option<String>,

    /// Document issue date
    pub issue_date: Option<String>,

    /// Luminaire type description
    pub luminaire_type: Option<String>,

    /// Additional comments/notes
    pub comments: Option<String>,
}

/// Luminaire physical description
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Luminaire {
    /// Bounding box dimensions
    pub dimensions: Option<Dimensions>,

    /// Luminous openings / emission areas
    pub luminous_openings: Vec<LuminousOpening>,

    /// Mounting type (e.g., "Recessed", "Surface", "Pendant")
    pub mounting: Option<String>,

    /// Number of emitters in the luminaire
    pub num_emitters: Option<u32>,
}

/// Physical dimensions in millimeters
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Dimensions {
    /// Length (along C0-C180 axis) in mm
    pub length: f64,
    /// Width (along C90-C270 axis) in mm
    pub width: f64,
    /// Height in mm
    pub height: f64,
}

/// Luminous opening / emission area description
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LuminousOpening {
    /// Shape of the opening
    pub shape: LuminousOpeningShape,
    /// Dimensions of the opening in mm
    pub dimensions: OpeningDimensions,
    /// Position offset from center
    pub position: Option<Position3D>,
}

/// Shape of luminous opening
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LuminousOpeningShape {
    #[default]
    Rectangular,
    Circular,
    Elliptical,
    Point,
}

/// Dimensions for different opening shapes
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OpeningDimensions {
    /// Length or diameter in mm
    pub length: f64,
    /// Width in mm (for rectangular/elliptical)
    pub width: Option<f64>,
}

/// 3D position offset
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Measurement equipment information
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Equipment {
    /// Goniophotometer/goniometer information
    pub goniometer: Option<GoniometerInfo>,

    /// Integrating sphere information
    pub integrating_sphere: Option<IntegratingSphereInfo>,

    /// Spectroradiometer information
    pub spectroradiometer: Option<SpectroradiometerInfo>,

    /// Laboratory accreditation details
    pub accreditation: Option<Accreditation>,
}

/// Goniometer/goniophotometer details
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GoniometerInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// Type: "Type A", "Type B", "Type C"
    pub goniometer_type: Option<String>,
    /// Measurement distance in meters
    pub distance: Option<f64>,
}

/// Integrating sphere details
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IntegratingSphereInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// Diameter in meters
    pub diameter: Option<f64>,
}

/// Spectroradiometer details
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpectroradiometerInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    /// Wavelength range in nm
    pub wavelength_min: Option<f64>,
    pub wavelength_max: Option<f64>,
    /// Spectral resolution in nm
    pub resolution: Option<f64>,
}

/// Laboratory accreditation information
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Accreditation {
    /// Accrediting body (e.g., "NVLAP", "IAS")
    pub body: Option<String>,
    /// Accreditation number
    pub number: Option<String>,
    /// Scope of accreditation
    pub scope: Option<String>,
}

/// Emitter information (lamp, LED module, etc.)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Emitter {
    /// Emitter identifier
    pub id: Option<String>,

    /// Emitter description/name
    pub description: Option<String>,

    /// Number of identical emitters
    pub quantity: u32,

    /// Rated luminous flux in lumens
    pub rated_lumens: Option<f64>,

    /// Measured luminous flux in lumens
    pub measured_lumens: Option<f64>,

    /// Input power in watts
    pub input_watts: Option<f64>,

    /// Power factor (0.0 - 1.0)
    pub power_factor: Option<f64>,

    /// Correlated color temperature in Kelvin
    pub cct: Option<f64>,

    /// Color rendering metrics
    pub color_rendering: Option<ColorRendering>,

    /// Scotopic-to-photopic ratio (S/P)
    pub sp_ratio: Option<f64>,

    /// Data generation information (measured vs simulated)
    pub data_generation: Option<DataGeneration>,

    /// Intensity distribution data
    pub intensity_distribution: Option<IntensityDistribution>,

    /// Spectral power distribution
    pub spectral_distribution: Option<SpectralDistribution>,
}

/// Color rendering metrics
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColorRendering {
    /// CIE Ra (general color rendering index)
    pub ra: Option<f64>,
    /// CIE R9 (red rendering)
    pub r9: Option<f64>,
    /// IES TM-30 Rf (fidelity index)
    pub rf: Option<f64>,
    /// IES TM-30 Rg (gamut index)
    pub rg: Option<f64>,
}

/// Information about how data was generated
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DataGeneration {
    /// Source of data
    pub source: DataSource,
    /// Whether intensity data was scaled
    pub scaled: bool,
    /// Whether measurement angles were interpolated
    pub interpolated: bool,
    /// Software used for simulation (if applicable)
    pub software: Option<String>,
    /// Measurement uncertainty percentage
    pub uncertainty: Option<f64>,
}

/// Source of photometric data
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DataSource {
    #[default]
    Measured,
    Simulated,
    Derived,
}

/// Intensity distribution (photometric web)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IntensityDistribution {
    /// Type of photometric system (Type A, B, or C)
    pub photometry_type: PhotometryType,

    /// Metric type for intensity values
    pub metric: IntensityMetric,

    /// Units for intensity values
    pub units: IntensityUnits,

    /// Horizontal (C-plane) angles in degrees
    pub horizontal_angles: Vec<f64>,

    /// Vertical (gamma) angles in degrees
    pub vertical_angles: Vec<f64>,

    /// Intensity values - outer vec is horizontal angles, inner is vertical
    /// `intensities[h_index][v_index]` = intensity at `horizontal_angles[h_index]`, `vertical_angles[v_index]`
    pub intensities: Vec<Vec<f64>>,
}

/// Photometry coordinate system type
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PhotometryType {
    /// Type A: Vertical axis along luminaire axis (automotive)
    TypeA,
    /// Type B: Horizontal axis along luminaire axis (floodlights)
    TypeB,
    /// Type C: Vertical axis through nadir (architectural) - most common
    #[default]
    TypeC,
}

/// Intensity metric type
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IntensityMetric {
    /// Luminous intensity (human vision)
    #[default]
    Luminous,
    /// Radiant intensity (UV, IR applications)
    Radiant,
    /// Photon intensity (horticultural PAR)
    Photon,
    /// Spectral intensity (per wavelength)
    Spectral,
}

/// Units for intensity values
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IntensityUnits {
    /// Candelas (cd) - absolute
    Candela,
    /// Candelas per kilolumen (cd/klm) - normalized
    #[default]
    CandelaPerKilolumen,
    /// Watts per steradian (W/sr) - radiant
    WattsPerSteradian,
    /// Micromoles per steradian per second (umol/sr/s) - photon
    MicromolesPerSteradianPerSecond,
    /// Watts per steradian per nanometer (W/sr/nm) - spectral
    WattsPerSteradianPerNanometer,
}

/// Spectral power distribution
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpectralDistribution {
    /// Wavelengths in nanometers
    pub wavelengths: Vec<f64>,

    /// Spectral values (radiant flux per wavelength)
    pub values: Vec<f64>,

    /// Units for spectral values
    pub units: SpectralUnits,

    /// Start wavelength if constant interval
    pub start_wavelength: Option<f64>,

    /// Wavelength interval if constant
    pub wavelength_interval: Option<f64>,
}

/// Units for spectral values
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SpectralUnits {
    /// Watts per nanometer (W/nm)
    #[default]
    WattsPerNanometer,
    /// Relative (normalized to peak = 1.0)
    Relative,
}

/// Custom data container for application-specific extensions
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CustomData {
    /// Namespace/application identifier
    pub namespace: Option<String>,
    /// Raw custom data (preserved as-is)
    pub data: String,
}

impl LuminaireOpticalData {
    /// Create a new empty document with default version
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            ..Default::default()
        }
    }

    /// Get total luminous flux from all emitters
    pub fn total_luminous_flux(&self) -> f64 {
        self.emitters
            .iter()
            .filter_map(|e| e.measured_lumens.or(e.rated_lumens))
            .sum()
    }

    /// Get total input power from all emitters
    pub fn total_input_watts(&self) -> f64 {
        self.emitters.iter().filter_map(|e| e.input_watts).sum()
    }

    /// Calculate luminous efficacy (lm/W)
    pub fn efficacy(&self) -> Option<f64> {
        let flux = self.total_luminous_flux();
        let watts = self.total_input_watts();
        if watts > 0.0 {
            Some(flux / watts)
        } else {
            None
        }
    }
}

impl IntensityDistribution {
    /// Get intensity at specific angles (with interpolation if needed)
    pub fn sample(&self, horizontal: f64, vertical: f64) -> Option<f64> {
        // Find indices
        let h_idx = self
            .horizontal_angles
            .iter()
            .position(|&a| (a - horizontal).abs() < 0.001)?;
        let v_idx = self
            .vertical_angles
            .iter()
            .position(|&a| (a - vertical).abs() < 0.001)?;

        self.intensities.get(h_idx)?.get(v_idx).copied()
    }

    /// Get maximum intensity value
    pub fn max_intensity(&self) -> f64 {
        self.intensities
            .iter()
            .flat_map(|row| row.iter())
            .fold(0.0_f64, |max, &val| max.max(val))
    }
}
