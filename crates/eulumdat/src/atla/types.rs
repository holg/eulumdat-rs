//! Core types for ATLA S001 / ANSI/IES TM-33 / UNI 11733 luminaire optical data
//!
//! This module defines the data structures that represent luminaire optical data
//! as specified in the ATLA S001 standard (equivalent to TM-33-18 / UNI 11733:2019)
//! and TM-33-23 (IESTM33-22 v1.1).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Schema Version Types (TM-33-23 support)
// ============================================================================

/// Schema version for format detection and validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SchemaVersion {
    /// ATLA S001 / TM-33-18 / UNI 11733:2019
    #[default]
    AtlaS001,
    /// TM-33-23 (IESTM33-22 v1.1)
    Tm3323,
    /// Future TM-33-24 (placeholder for version detection)
    Tm3324,
}

impl SchemaVersion {
    /// Get the version string for this schema
    pub fn version_string(&self) -> &'static str {
        match self {
            SchemaVersion::AtlaS001 => "1.0",
            SchemaVersion::Tm3323 => "1.1",
            SchemaVersion::Tm3324 => "1.2",
        }
    }

    /// Get the root element name for this schema
    pub fn root_element(&self) -> &'static str {
        match self {
            SchemaVersion::AtlaS001 => "LuminaireOpticalData",
            SchemaVersion::Tm3323 | SchemaVersion::Tm3324 => "IESTM33-22",
        }
    }
}

/// Symmetry type enumeration (TM-33-23)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SymmetryType {
    /// No symmetry - full 360° data
    #[default]
    None,
    /// Bilateral symmetry about 0° plane
    Bi0,
    /// Bilateral symmetry about 90° plane
    Bi90,
    /// Quadrilateral symmetry (90° data)
    Quad,
    /// Full rotational symmetry (single plane)
    Full,
    /// Arbitrary symmetry pattern
    Arbitrary,
}

impl SymmetryType {
    /// Parse from TM-33-23 string format (e.g., "Symm _ None")
    pub fn from_tm33_str(s: &str) -> Self {
        let normalized = s.trim().replace(" _ ", "_").replace(" ", "").to_uppercase();

        match normalized.as_str() {
            "SYMM_NONE" | "NONE" => SymmetryType::None,
            "SYMM_BI_0" | "SYMM_BI0" | "BI0" | "BI_0" => SymmetryType::Bi0,
            "SYMM_BI_90" | "SYMM_BI90" | "BI90" | "BI_90" => SymmetryType::Bi90,
            "SYMM_QUAD" | "QUAD" => SymmetryType::Quad,
            "SYMM_FULL" | "FULL" => SymmetryType::Full,
            "SYMM_ARBITRARY" | "ARBITRARY" => SymmetryType::Arbitrary,
            _ => SymmetryType::None,
        }
    }

    /// Convert to TM-33-23 string format
    pub fn to_tm33_str(&self) -> &'static str {
        match self {
            SymmetryType::None => "Symm _ None",
            SymmetryType::Bi0 => "Symm _ Bi _ 0",
            SymmetryType::Bi90 => "Symm _ Bi _90",
            SymmetryType::Quad => "Symm _ Quad",
            SymmetryType::Full => "Symm _ Full",
            SymmetryType::Arbitrary => "Symm _ Arbitrary",
        }
    }
}

/// Gonioradiometer type with both naming conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GoniometerTypeEnum {
    /// CIE Type A (vertical axis along luminaire)
    CieA,
    /// CIE Type B (horizontal axis along luminaire)
    CieB,
    /// CIE Type C (vertical axis through nadir) - most common
    #[default]
    CieC,
    /// IES Type A
    IesA,
    /// IES Type B
    IesB,
    /// IES Type C
    IesC,
    /// Custom goniometer type
    Custom,
}

impl GoniometerTypeEnum {
    /// Parse from various string formats (TM-33-23 "CIE _ A" or legacy "TypeA")
    pub fn parse(s: &str) -> Self {
        let normalized = s.trim().replace(" _ ", "_").replace(" ", "").to_uppercase();

        match normalized.as_str() {
            "CIE_A" | "CIEA" | "TYPEA" | "A" => GoniometerTypeEnum::CieA,
            "CIE_B" | "CIEB" | "TYPEB" | "B" => GoniometerTypeEnum::CieB,
            "CIE_C" | "CIEC" | "TYPEC" | "C" => GoniometerTypeEnum::CieC,
            "IES_A" | "IESA" => GoniometerTypeEnum::IesA,
            "IES_B" | "IESB" => GoniometerTypeEnum::IesB,
            "IES_C" | "IESC" => GoniometerTypeEnum::IesC,
            "CUSTOM" => GoniometerTypeEnum::Custom,
            _ => GoniometerTypeEnum::CieC,
        }
    }

    /// Convert to TM-33-23 string format
    pub fn to_tm33_str(&self) -> &'static str {
        match self {
            GoniometerTypeEnum::CieA => "CIE _ A",
            GoniometerTypeEnum::CieB => "CIE _ B",
            GoniometerTypeEnum::CieC => "CIE _ C",
            GoniometerTypeEnum::IesA => "IES _ A",
            GoniometerTypeEnum::IesB => "IES _ B",
            GoniometerTypeEnum::IesC => "IES _ C",
            GoniometerTypeEnum::Custom => "CUSTOM",
        }
    }

    /// Convert to legacy ATLA S001 string format
    pub fn to_atla_str(&self) -> &'static str {
        match self {
            GoniometerTypeEnum::CieA | GoniometerTypeEnum::IesA => "TypeA",
            GoniometerTypeEnum::CieB | GoniometerTypeEnum::IesB => "TypeB",
            GoniometerTypeEnum::CieC | GoniometerTypeEnum::IesC => "TypeC",
            GoniometerTypeEnum::Custom => "CUSTOM",
        }
    }
}

/// Regulatory value type (TM-33-23)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RegulatoryValue {
    /// Actually measured value
    Measured,
    /// Nominal/typical value
    Nominal,
    /// Rated/specified value
    Rated,
}

impl RegulatoryValue {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "measured" => Some(RegulatoryValue::Measured),
            "nominal" => Some(RegulatoryValue::Nominal),
            "rated" => Some(RegulatoryValue::Rated),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            RegulatoryValue::Measured => "Measured",
            RegulatoryValue::Nominal => "Nominal",
            RegulatoryValue::Rated => "Rated",
        }
    }
}

/// Root document for ATLA S001 / TM-33 / UNI 11733 luminaire optical data
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LuminaireOpticalData {
    /// Detected/target schema version
    pub schema_version: SchemaVersion,

    /// Schema version string (e.g., "1.0" for S001, "1.1" for TM-33-23)
    pub version: String,

    /// Required header information
    pub header: Header,

    /// Optional luminaire physical description
    pub luminaire: Option<Luminaire>,

    /// Optional measurement equipment information
    pub equipment: Option<Equipment>,

    /// Required emitter(s) information - at least one
    pub emitters: Vec<Emitter>,

    /// Optional application-specific custom data (ATLA S001 style - single)
    pub custom_data: Option<CustomData>,

    /// Multiple custom data items (TM-33-23 style)
    pub custom_data_items: Vec<CustomDataItem>,
}

/// Header section containing general luminaire identification
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Header {
    /// Manufacturer name
    pub manufacturer: Option<String>,

    /// Catalog/product number
    pub catalog_number: Option<String>,

    /// Product description (required in TM-33-23)
    pub description: Option<String>,

    /// Global Trade Item Number (GTIN/UPC/EAN) as string
    pub gtin: Option<String>,

    /// GTIN as integer (TM-33-23 requires xs:integer)
    pub gtin_int: Option<i64>,

    /// Universally Unique Identifier for version control
    pub uuid: Option<String>,

    /// Reference to related documents (single, for S001 compatibility)
    pub reference: Option<String>,

    /// Multiple references (TM-33-23 allows unbounded)
    pub references: Vec<String>,

    /// URI for additional product information
    pub more_info_uri: Option<String>,

    /// Test laboratory name (required in TM-33-23)
    pub laboratory: Option<String>,

    /// Test report number (required in TM-33-23)
    pub report_number: Option<String>,

    /// Report date in xs:date format YYYY-MM-DD (required in TM-33-23)
    pub report_date: Option<String>,

    /// Test date (ISO 8601 format) - legacy S001 field
    pub test_date: Option<String>,

    /// Document issue date
    pub issue_date: Option<String>,

    /// Document creator (TM-33-23)
    pub document_creator: Option<String>,

    /// Document creation date (TM-33-23)
    pub document_creation_date: Option<String>,

    /// Unique identifier (TM-33-23)
    pub unique_identifier: Option<String>,

    /// Luminaire type description
    pub luminaire_type: Option<String>,

    /// Additional comments/notes (single, for S001 compatibility)
    pub comments: Option<String>,

    /// Multiple comments (TM-33-23 allows unbounded)
    pub comments_list: Vec<String>,
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

    /// Emitter description/name (required in TM-33-23)
    pub description: Option<String>,

    /// Catalog number (TM-33-23)
    pub catalog_number: Option<String>,

    /// Number of identical emitters (required in TM-33-23)
    pub quantity: u32,

    /// Rated luminous flux in lumens
    pub rated_lumens: Option<f64>,

    /// Measured luminous flux in lumens
    pub measured_lumens: Option<f64>,

    /// Input power in watts (required in TM-33-23 as InputWattage)
    pub input_watts: Option<f64>,

    /// Power factor (0.0 - 1.0)
    pub power_factor: Option<f64>,

    /// Ballast factor (TM-33-23)
    pub ballast_factor: Option<f64>,

    /// Correlated color temperature in Kelvin
    pub cct: Option<f64>,

    /// Color rendering metrics
    pub color_rendering: Option<ColorRendering>,

    /// Duv color shift (TM-33-23)
    pub duv: Option<f64>,

    /// Scotopic-to-photopic ratio (S/P)
    pub sp_ratio: Option<f64>,

    /// Data generation information (measured vs simulated)
    pub data_generation: Option<DataGeneration>,

    /// Intensity distribution data
    pub intensity_distribution: Option<IntensityDistribution>,

    /// Spectral power distribution
    pub spectral_distribution: Option<SpectralDistribution>,

    /// Angular spectral data - 4D intensity (TM-33-23)
    pub angular_spectral: Option<AngularSpectralData>,

    /// Angular color data - CIE x,y per angle (TM-33-23)
    pub angular_color: Option<AngularColorData>,

    /// Tilt angles (TM-33-23)
    pub tilt_angles: Option<TiltAngles>,

    /// Regulatory tracking flags (TM-33-23)
    pub regulatory: Option<Regulatory>,
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

    // TM-33-23 specific fields
    /// Symmetry type (TM-33-23)
    pub symmetry: Option<SymmetryType>,

    /// Multiplier for intensity values (TM-33-23)
    /// When present, actual intensity = stored_value * multiplier
    pub multiplier: Option<f32>,

    /// Whether data is absolute photometry (TM-33-23)
    pub absolute_photometry: Option<bool>,

    /// Number of measured points (TM-33-23)
    pub number_measured: Option<i32>,
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

/// Custom data container for application-specific extensions (ATLA S001)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CustomData {
    /// Namespace/application identifier
    pub namespace: Option<String>,
    /// Raw custom data (preserved as-is)
    pub data: String,
}

// ============================================================================
// TM-33-23 Specific Types
// ============================================================================

/// Custom data item (TM-33-23 style with Name and UniqueIdentifier)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CustomDataItem {
    /// Name identifier for the custom data
    pub name: String,
    /// Unique identifier (UUID or similar)
    pub unique_identifier: String,
    /// Raw XML content preserved as-is for lossless round-trip
    pub raw_content: String,
}

/// Angular spectral data - intensity as function of angle AND wavelength (TM-33-23)
/// This is a 4D dataset: (horizontal, vertical, wavelength) -> intensity
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AngularSpectralData {
    /// Whether data is absolute (vs normalized)
    pub absolute: Option<bool>,
    /// Symmetry type
    pub symmetry: Option<SymmetryType>,
    /// Multiplier for values
    pub multiplier: Option<f32>,
    /// Number of measured points
    pub number_measured: i32,
    /// Number of horizontal angles
    pub number_horz: i32,
    /// Number of vertical angles
    pub number_vert: i32,
    /// Number of wavelengths
    pub number_wavelength: i32,
    /// Data points (h, v, w, value)
    pub data_points: Vec<AngularSpectralPoint>,
}

/// Single data point in angular spectral data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AngularSpectralPoint {
    /// Horizontal angle (degrees)
    pub h: f64,
    /// Vertical angle (degrees)
    pub v: f64,
    /// Wavelength (nm)
    pub w: f64,
    /// Intensity value
    pub value: f32,
}

/// Angular color data - CIE x,y chromaticity as function of angle (TM-33-23)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AngularColorData {
    /// Symmetry type
    pub symmetry: Option<SymmetryType>,
    /// Multiplier for values
    pub multiplier: Option<f32>,
    /// Number of measured points
    pub number_measured: i32,
    /// Number of horizontal angles
    pub number_horz: i32,
    /// Number of vertical angles
    pub number_vert: i32,
    /// Color data points with CIE x,y per angle
    pub data_points: Vec<AngularColorPoint>,
}

/// Single data point in angular color data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AngularColorPoint {
    /// Horizontal angle (degrees)
    pub h: f64,
    /// Vertical angle (degrees)
    pub v: f64,
    /// CIE x chromaticity coordinate
    pub x: f64,
    /// CIE y chromaticity coordinate
    pub y: f64,
}

/// Tilt angles support (TM-33-23)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TiltAngles {
    /// Number of tilt angles
    pub number_angles: i32,
    /// Tilt angle values in degrees
    pub angles: Vec<f64>,
}

/// Regulatory tracking flags (TM-33-23)
/// Indicates whether each value is Measured, Nominal, or Rated
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Regulatory {
    pub input_wattage: Option<RegulatoryValue>,
    pub power_factor: Option<RegulatoryValue>,
    pub ballast_factor: Option<RegulatoryValue>,
    pub color_temperature: Option<RegulatoryValue>,
    pub cie_cri: Option<RegulatoryValue>,
    pub ies_tm30: Option<RegulatoryValue>,
    pub duv: Option<RegulatoryValue>,
    pub sp_ratio: Option<RegulatoryValue>,
    pub luminous_intensity: Option<RegulatoryValue>,
    pub luminous_flux: Option<RegulatoryValue>,
    pub radiant_intensity: Option<RegulatoryValue>,
    pub radiant_flux: Option<RegulatoryValue>,
    pub photon_intensity: Option<RegulatoryValue>,
    pub photon_flux: Option<RegulatoryValue>,
    pub spectral_power: Option<RegulatoryValue>,
    pub spectral_intensity: Option<RegulatoryValue>,
    pub angular_color: Option<RegulatoryValue>,
    pub illuminance: Option<RegulatoryValue>,
    pub irradiance: Option<RegulatoryValue>,
    pub photon_flux_density: Option<RegulatoryValue>,
    pub spectral_irradiance: Option<RegulatoryValue>,
}

/// Extended color rendering with full TM-30 hue bin data (TM-33-23)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Tm30ColorRendering {
    /// Rf (fidelity index)
    pub rf: i32,
    /// Rg (gamut index)
    pub rg: i32,
    /// Rf per hue bin (01-16)
    pub rfh: [Option<i32>; 16],
    /// Rcs per hue bin (chroma shift, 01-16)
    pub rcsh: [Option<i32>; 16],
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
