//! IES (IESNA LM-63) file format support.
//!
//! This module provides parsing and export of IES photometric files according to
//! ANSI/IES LM-63-2019 and compatible older formats (LM-63-2002, LM-63-1995, LM-63-1991).
//!
//! ## IES File Format Overview
//!
//! The IES format is the North American standard for photometric data exchange,
//! developed by the Illuminating Engineering Society (IES).
//!
//! ### File Structure
//!
//! 1. **Version header**: `IES:LM-63-2019`, `IESNA:LM-63-2002`, or `IESNA91` (older)
//! 2. **Keywords**: `[KEYWORD] value` format (TEST, MANUFAC, LUMINAIRE, etc.)
//!    - Required: `[TEST]`, `[TESTLAB]`, `[ISSUEDATE]`, `[MANUFAC]`
//!    - Optional: `[LUMCAT]`, `[LUMINAIRE]`, `[LAMP]`, `[LAMPCAT]`, `[BALLAST]`, etc.
//! 3. **TILT specification**: `TILT=NONE` or `TILT=INCLUDE`
//! 4. **Photometric data**:
//!    - Line 1: num_lamps, lumens_per_lamp, multiplier, n_vert, n_horiz, photo_type, units, width, length, height
//!    - Line 2: ballast_factor, file_generation_type, input_watts
//!    - Vertical angles (n_vert values)
//!    - Horizontal angles (n_horiz values)
//!    - Candela values (n_horiz sets of n_vert values each)
//!
//! ### Photometric Types
//!
//! - **Type A**: Automotive (horizontal angles in horizontal plane)
//! - **Type B**: Adjustable luminaires (horizontal angles in vertical plane)
//! - **Type C**: Most common - architectural (vertical angles from nadir)
//!
//! ### File Generation Types (LM-63-2019)
//!
//! - 1.00001: Undefined
//! - 1.00010: Computer simulation
//! - 1.00000: Test at unaccredited lab
//! - 1.10000: Test at accredited lab
//! - See [`FileGenerationType`] for full list
//!
//! ## Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, IesParser, IesExporter};
//!
//! // Import from IES
//! let ldt = IesParser::parse_file("luminaire.ies")?;
//! println!("Luminaire: {}", ldt.luminaire_name);
//!
//! // Export to IES
//! let ies_content = IesExporter::export(&ldt);
//! std::fs::write("output.ies", ies_content)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::{anyhow, Result};
use crate::eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use crate::symmetry::SymmetryHandler;

/// IES file format parser.
///
/// Parses IES LM-63 format files (versions 1991, 1995, 2002, 2019).
pub struct IesParser;

/// IES file format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IesVersion {
    /// LM-63-1991 (IESNA91)
    Lm63_1991,
    /// LM-63-1995 (IESNA:LM-63-1995)
    Lm63_1995,
    /// LM-63-2002 (IESNA:LM-63-2002)
    #[default]
    Lm63_2002,
    /// LM-63-2019 (IES:LM-63-2019)
    Lm63_2019,
}

impl IesVersion {
    /// Parse version from header string.
    pub fn from_header(header: &str) -> Self {
        let header_upper = header.to_uppercase();
        if header_upper.contains("LM-63-2019") || header_upper.starts_with("IES:LM-63") {
            Self::Lm63_2019
        } else if header_upper.contains("LM-63-2002") {
            Self::Lm63_2002
        } else if header_upper.contains("LM-63-1995") {
            Self::Lm63_1995
        } else {
            Self::Lm63_1991
        }
    }

    /// Get the header string for this version.
    pub fn header(&self) -> &'static str {
        match self {
            Self::Lm63_1991 => "IESNA91",
            Self::Lm63_1995 => "IESNA:LM-63-1995",
            Self::Lm63_2002 => "IESNA:LM-63-2002",
            Self::Lm63_2019 => "IES:LM-63-2019",
        }
    }
}

/// File generation type (LM-63-2019 Section 5.13, Table 2).
///
/// Describes how the IES file was generated.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FileGenerationType {
    /// 1.00001 - Undefined or older file
    #[default]
    Undefined,
    /// 1.00010 - Computer simulation (raytracing)
    ComputerSimulation,
    /// 1.00000 - Test at unaccredited lab
    UnaccreditedLab,
    /// 1.00100 - Test at unaccredited lab, lumen scaled
    UnaccreditedLabScaled,
    /// 1.01000 - Test at unaccredited lab, interpolated angles
    UnaccreditedLabInterpolated,
    /// 1.01100 - Test at unaccredited lab, interpolated and scaled
    UnaccreditedLabInterpolatedScaled,
    /// 1.10000 - Test at accredited lab
    AccreditedLab,
    /// 1.10100 - Test at accredited lab, lumen scaled
    AccreditedLabScaled,
    /// 1.11000 - Test at accredited lab, interpolated angles
    AccreditedLabInterpolated,
    /// 1.11100 - Test at accredited lab, interpolated and scaled
    AccreditedLabInterpolatedScaled,
}

impl FileGenerationType {
    /// Parse from decimal value.
    pub fn from_value(value: f64) -> Self {
        // Round to 5 decimal places to handle floating point precision
        let rounded = (value * 100000.0).round() / 100000.0;
        match rounded {
            v if (v - 1.00001).abs() < 0.000001 => Self::Undefined,
            v if (v - 1.00010).abs() < 0.000001 => Self::ComputerSimulation,
            v if (v - 1.00000).abs() < 0.000001 => Self::UnaccreditedLab,
            v if (v - 1.00100).abs() < 0.000001 => Self::UnaccreditedLabScaled,
            v if (v - 1.01000).abs() < 0.000001 => Self::UnaccreditedLabInterpolated,
            v if (v - 1.01100).abs() < 0.000001 => Self::UnaccreditedLabInterpolatedScaled,
            v if (v - 1.10000).abs() < 0.000001 => Self::AccreditedLab,
            v if (v - 1.10100).abs() < 0.000001 => Self::AccreditedLabScaled,
            v if (v - 1.11000).abs() < 0.000001 => Self::AccreditedLabInterpolated,
            v if (v - 1.11100).abs() < 0.000001 => Self::AccreditedLabInterpolatedScaled,
            // For legacy files, treat ballast-lamp factor as undefined
            _ => Self::Undefined,
        }
    }

    /// Get decimal value for this type.
    pub fn value(&self) -> f64 {
        match self {
            Self::Undefined => 1.00001,
            Self::ComputerSimulation => 1.00010,
            Self::UnaccreditedLab => 1.00000,
            Self::UnaccreditedLabScaled => 1.00100,
            Self::UnaccreditedLabInterpolated => 1.01000,
            Self::UnaccreditedLabInterpolatedScaled => 1.01100,
            Self::AccreditedLab => 1.10000,
            Self::AccreditedLabScaled => 1.10100,
            Self::AccreditedLabInterpolated => 1.11000,
            Self::AccreditedLabInterpolatedScaled => 1.11100,
        }
    }

    /// Get title for this type (per LM-63-2019 Table 2).
    pub fn title(&self) -> &'static str {
        match self {
            Self::Undefined => "Undefined",
            Self::ComputerSimulation => "Computer Simulation",
            Self::UnaccreditedLab => "Test at an unaccredited lab",
            Self::UnaccreditedLabScaled => "Test at an unaccredited lab that has been lumen scaled",
            Self::UnaccreditedLabInterpolated => {
                "Test at an unaccredited lab with interpolated angle set"
            }
            Self::UnaccreditedLabInterpolatedScaled => {
                "Test at an unaccredited lab with interpolated angle set that has been lumen scaled"
            }
            Self::AccreditedLab => "Test at an accredited lab",
            Self::AccreditedLabScaled => "Test at an accredited lab that has been lumen scaled",
            Self::AccreditedLabInterpolated => {
                "Test at an accredited lab with interpolated angle set"
            }
            Self::AccreditedLabInterpolatedScaled => {
                "Test at an accredited lab with interpolated angle set that has been lumen scaled"
            }
        }
    }

    /// Check if this is from an accredited lab.
    pub fn is_accredited(&self) -> bool {
        matches!(
            self,
            Self::AccreditedLab
                | Self::AccreditedLabScaled
                | Self::AccreditedLabInterpolated
                | Self::AccreditedLabInterpolatedScaled
        )
    }

    /// Check if lumen values were scaled.
    pub fn is_scaled(&self) -> bool {
        matches!(
            self,
            Self::UnaccreditedLabScaled
                | Self::UnaccreditedLabInterpolatedScaled
                | Self::AccreditedLabScaled
                | Self::AccreditedLabInterpolatedScaled
        )
    }

    /// Check if angles were interpolated.
    pub fn is_interpolated(&self) -> bool {
        matches!(
            self,
            Self::UnaccreditedLabInterpolated
                | Self::UnaccreditedLabInterpolatedScaled
                | Self::AccreditedLabInterpolated
                | Self::AccreditedLabInterpolatedScaled
        )
    }
}

/// Luminous opening shape (LM-63-2019 Section 5.11, Table 1).
///
/// Determined by the signs of width, length, and height dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LuminousShape {
    /// Point source (all dimensions = 0)
    #[default]
    Point,
    /// Rectangular opening (width > 0, length > 0, height = 0)
    Rectangular,
    /// Rectangular with luminous sides (all > 0)
    RectangularWithSides,
    /// Circular (width = length < 0, height = 0)
    Circular,
    /// Ellipse (width < 0, length < 0, height = 0)
    Ellipse,
    /// Vertical cylinder (width = length < 0, height > 0)
    VerticalCylinder,
    /// Vertical ellipsoidal cylinder
    VerticalEllipsoidalCylinder,
    /// Sphere (all negative, equal)
    Sphere,
    /// Ellipsoidal spheroid (all negative, not equal)
    EllipsoidalSpheroid,
    /// Horizontal cylinder along photometric horizontal
    HorizontalCylinderAlong,
    /// Horizontal ellipsoidal cylinder along photometric horizontal
    HorizontalEllipsoidalCylinderAlong,
    /// Horizontal cylinder perpendicular to photometric horizontal
    HorizontalCylinderPerpendicular,
    /// Horizontal ellipsoidal cylinder perpendicular to photometric horizontal
    HorizontalEllipsoidalCylinderPerpendicular,
    /// Vertical circle facing photometric horizontal
    VerticalCircle,
    /// Vertical ellipse facing photometric horizontal
    VerticalEllipse,
}

impl LuminousShape {
    /// Determine shape from width, length, height values.
    pub fn from_dimensions(width: f64, length: f64, height: f64) -> Self {
        let w_zero = width.abs() < 0.0001;
        let l_zero = length.abs() < 0.0001;
        let h_zero = height.abs() < 0.0001;
        let w_neg = width < 0.0;
        let l_neg = length < 0.0;
        let h_neg = height < 0.0;
        let w_pos = width > 0.0;
        let l_pos = length > 0.0;
        let h_pos = height > 0.0;

        // Check for equal negative values (circular shapes)
        let wl_equal = (width - length).abs() < 0.0001;
        let all_equal = wl_equal && (width - height).abs() < 0.0001;

        match (
            w_zero, l_zero, h_zero, w_neg, l_neg, h_neg, w_pos, l_pos, h_pos,
        ) {
            // Point: all zero
            (true, true, true, _, _, _, _, _, _) => Self::Point,
            // Rectangular: width > 0, length > 0, height = 0
            (_, _, true, _, _, _, true, true, _) => Self::Rectangular,
            // Rectangular with sides: all positive
            (_, _, _, _, _, _, true, true, true) => Self::RectangularWithSides,
            // Circular: width = length < 0, height = 0
            (_, _, true, true, true, _, _, _, _) if wl_equal => Self::Circular,
            // Ellipse: width < 0, length < 0, height = 0
            (_, _, true, true, true, _, _, _, _) => Self::Ellipse,
            // Sphere: all negative and equal
            (_, _, _, true, true, true, _, _, _) if all_equal => Self::Sphere,
            // Ellipsoidal spheroid: all negative
            (_, _, _, true, true, true, _, _, _) => Self::EllipsoidalSpheroid,
            // Vertical cylinder: width = length < 0, height > 0
            (_, _, _, true, true, _, _, _, true) if wl_equal => Self::VerticalCylinder,
            // Vertical ellipsoidal cylinder: width < 0, length < 0, height > 0
            (_, _, _, true, true, _, _, _, true) => Self::VerticalEllipsoidalCylinder,
            // Horizontal cylinder along: width < 0, length > 0, height < 0
            (_, _, _, true, _, true, _, true, _) if (width - height).abs() < 0.0001 => {
                Self::HorizontalCylinderAlong
            }
            // Horizontal ellipsoidal cylinder along
            (_, _, _, true, _, true, _, true, _) => Self::HorizontalEllipsoidalCylinderAlong,
            // Horizontal cylinder perpendicular: width > 0, length < 0, height < 0
            (_, _, _, _, true, true, true, _, _) if (length - height).abs() < 0.0001 => {
                Self::HorizontalCylinderPerpendicular
            }
            // Horizontal ellipsoidal cylinder perpendicular
            (_, _, _, _, true, true, true, _, _) => {
                Self::HorizontalEllipsoidalCylinderPerpendicular
            }
            // Vertical circle: width < 0, length = 0, height < 0
            (_, true, _, true, _, true, _, _, _) if (width - height).abs() < 0.0001 => {
                Self::VerticalCircle
            }
            // Vertical ellipse: width < 0, length = 0, height < 0
            (_, true, _, true, _, true, _, _, _) => Self::VerticalEllipse,
            // Default to point for any unrecognized combination
            _ => Self::Point,
        }
    }

    /// Get description of the shape.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Point => "Point source",
            Self::Rectangular => "Rectangular luminous opening",
            Self::RectangularWithSides => "Rectangular with luminous sides",
            Self::Circular => "Circular luminous opening",
            Self::Ellipse => "Elliptical luminous opening",
            Self::VerticalCylinder => "Vertical cylinder",
            Self::VerticalEllipsoidalCylinder => "Vertical ellipsoidal cylinder",
            Self::Sphere => "Spherical luminous opening",
            Self::EllipsoidalSpheroid => "Ellipsoidal spheroid",
            Self::HorizontalCylinderAlong => "Horizontal cylinder along photometric horizontal",
            Self::HorizontalEllipsoidalCylinderAlong => {
                "Horizontal ellipsoidal cylinder along photometric horizontal"
            }
            Self::HorizontalCylinderPerpendicular => {
                "Horizontal cylinder perpendicular to photometric horizontal"
            }
            Self::HorizontalEllipsoidalCylinderPerpendicular => {
                "Horizontal ellipsoidal cylinder perpendicular to photometric horizontal"
            }
            Self::VerticalCircle => "Vertical circle facing photometric horizontal",
            Self::VerticalEllipse => "Vertical ellipse facing photometric horizontal",
        }
    }
}

/// TILT data for luminaires with position-dependent output.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TiltData {
    /// Lamp to luminaire geometry (1-3)
    /// 1 = vertical base-up or base-down
    /// 2 = horizontal, stays horizontal when tilted
    /// 3 = horizontal, tilts with luminaire
    pub lamp_geometry: i32,
    /// Tilt angles in degrees
    pub angles: Vec<f64>,
    /// Multiplying factors corresponding to angles
    pub factors: Vec<f64>,
}

/// Lamp position within luminaire (LM-63-2019 Annex E).
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LampPosition {
    /// Horizontal position angle (0-360°)
    pub horizontal: f64,
    /// Vertical position angle (0-180°)
    pub vertical: f64,
}

/// Photometric measurement type.
///
/// ## Coordinate System Differences
///
/// - **Type C**: Vertical polar axis (0° = nadir, 180° = zenith). Standard for downlights, streetlights.
/// - **Type B**: Horizontal polar axis (0H 0V = beam center). Used for floodlights, sports lighting.
///   - ⚠️ **TODO**: Implement 90° coordinate rotation for Type B → Type C conversion
///   - Required transformation matrix: R_x(90°) to align horizontal axis to vertical
/// - **Type A**: Automotive coordinates. Rare in architectural lighting.
///   - Currently parsed but may render incorrectly without coordinate mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PhotometricType {
    /// Type A - Automotive photometry (rare)
    TypeA = 3,
    /// Type B - Adjustable luminaires (floodlights, theatrical)
    TypeB = 2,
    /// Type C - Architectural (most common)
    #[default]
    TypeC = 1,
}

impl PhotometricType {
    /// Create from integer value.
    pub fn from_int(value: i32) -> Result<Self> {
        match value {
            1 => Ok(Self::TypeC),
            2 => Ok(Self::TypeB),
            3 => Ok(Self::TypeA),
            _ => Err(anyhow!("Invalid photometric type: {}", value)),
        }
    }
}

/// Unit type for dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnitType {
    /// Dimensions in feet
    Feet = 1,
    /// Dimensions in meters
    #[default]
    Meters = 2,
}

impl UnitType {
    /// Create from integer value.
    pub fn from_int(value: i32) -> Result<Self> {
        match value {
            1 => Ok(Self::Feet),
            2 => Ok(Self::Meters),
            _ => Err(anyhow!("Invalid unit type: {}", value)),
        }
    }

    /// Conversion factor to millimeters.
    pub fn to_mm_factor(&self) -> f64 {
        match self {
            UnitType::Feet => 304.8,    // 1 foot = 304.8 mm
            UnitType::Meters => 1000.0, // 1 meter = 1000 mm
        }
    }
}

/// Parsed IES data before conversion to Eulumdat.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IesData {
    /// Version (parsed from header)
    pub version: IesVersion,
    /// Version string as found in file
    pub version_string: String,
    /// All keyword metadata
    pub keywords: HashMap<String, String>,

    // === Required Keywords (LM-63-2019) ===
    /// `[TEST]` Test report number
    pub test: String,
    /// `[TESTLAB]` Photometric testing laboratory
    pub test_lab: String,
    /// `[ISSUEDATE]` Date manufacturer issued the file
    pub issue_date: String,
    /// `[MANUFAC]` Manufacturer of luminaire
    pub manufacturer: String,

    // === Common Optional Keywords ===
    /// `[LUMCAT]` Luminaire catalog number
    pub luminaire_catalog: String,
    /// `[LUMINAIRE]` Luminaire description
    pub luminaire: String,
    /// `[LAMPCAT]` Lamp catalog number
    pub lamp_catalog: String,
    /// `[LAMP]` Lamp description
    pub lamp: String,
    /// `[BALLAST]` Ballast description
    pub ballast: String,
    /// `[BALLASTCAT]` Ballast catalog number
    pub ballast_catalog: String,
    /// `[TESTDATE]` Date of photometric test
    pub test_date: String,
    /// `[MAINTCAT]` IES maintenance category (1-6)
    pub maintenance_category: Option<i32>,
    /// `[DISTRIBUTION]` Distribution description
    pub distribution: String,
    /// `[FLASHAREA]` Flash area in m²
    pub flash_area: Option<f64>,
    /// `[COLORCONSTANT]` Color constant for glare
    pub color_constant: Option<f64>,
    /// `[LAMPPOSITION]` Lamp position angles
    pub lamp_position: Option<LampPosition>,
    /// `[NEARFIELD]` Near field distances (D1, D2, D3)
    pub near_field: Option<(f64, f64, f64)>,
    /// `[FILEGENINFO]` Additional file generation info
    pub file_gen_info: String,
    /// `[SEARCH]` User search string
    pub search: String,
    /// `[OTHER]` lines (can appear multiple times)
    pub other: Vec<String>,

    // === Photometric Parameters ===
    /// Number of lamps
    pub num_lamps: i32,
    /// Lumens per lamp (-1 = absolute photometry)
    pub lumens_per_lamp: f64,
    /// Candela multiplier
    pub multiplier: f64,
    /// Number of vertical angles
    pub n_vertical: usize,
    /// Number of horizontal angles
    pub n_horizontal: usize,
    /// Photometric type (1=C, 2=B, 3=A)
    pub photometric_type: PhotometricType,
    /// Unit type (1=feet, 2=meters)
    pub unit_type: UnitType,
    /// Luminous opening width (negative = rounded shape)
    pub width: f64,
    /// Luminous opening length (negative = rounded shape)
    pub length: f64,
    /// Luminous opening height (negative = rounded shape)
    pub height: f64,
    /// Derived luminous shape
    pub luminous_shape: LuminousShape,
    /// Ballast factor
    pub ballast_factor: f64,
    /// File generation type (LM-63-2019) or ballast-lamp factor (older)
    pub file_generation_type: FileGenerationType,
    /// Raw file generation type value (for preservation)
    pub file_generation_value: f64,
    /// Input watts
    pub input_watts: f64,

    // === TILT Data ===
    /// TILT mode (NONE or INCLUDE)
    pub tilt_mode: String,
    /// TILT data if INCLUDE
    pub tilt_data: Option<TiltData>,

    // === Angle Data ===
    /// Vertical angles (gamma)
    pub vertical_angles: Vec<f64>,
    /// Horizontal angles (C-planes)
    pub horizontal_angles: Vec<f64>,
    /// Candela values `[horizontal_index][vertical_index]`
    pub candela_values: Vec<Vec<f64>>,
}

impl IesParser {
    /// Parse an IES file from a file path.
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Eulumdat> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow!("Failed to read IES file: {}", e))?;
        Self::parse(&content)
    }

    /// Parse IES content from a string.
    pub fn parse(content: &str) -> Result<Eulumdat> {
        let ies_data = Self::parse_ies_data(content)?;
        Self::convert_to_eulumdat(ies_data)
    }

    /// Parse IES content and return raw IES data structure.
    ///
    /// This is useful for accessing IES-specific fields that don't map to Eulumdat.
    pub fn parse_to_ies_data(content: &str) -> Result<IesData> {
        Self::parse_ies_data(content)
    }

    /// Parse IES format into intermediate structure.
    fn parse_ies_data(content: &str) -> Result<IesData> {
        let mut data = IesData::default();
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Err(anyhow!("Empty IES file"));
        }

        let mut line_idx = 0;

        // Parse version header
        let first_line = lines[line_idx].trim();
        // LM-63-2019: IES:LM-63-2019
        // LM-63-2002: IESNA:LM-63-2002
        // LM-63-1991: IESNA91 or IESNA:LM-63-1991
        if first_line.to_uppercase().starts_with("IES")
            || first_line.to_uppercase().starts_with("IESNA")
        {
            data.version_string = first_line.to_string();
            data.version = IesVersion::from_header(first_line);
            line_idx += 1;
        } else {
            // Older format without explicit version
            data.version_string = "IESNA91".to_string();
            data.version = IesVersion::Lm63_1991;
        }

        // Parse keywords until TILT, handling [MORE] continuation
        let mut current_keyword = String::new();
        let mut current_value = String::new();
        let mut last_stored_keyword = String::new(); // Track last keyword for [MORE]

        while line_idx < lines.len() {
            let line = lines[line_idx].trim();

            if line.to_uppercase().starts_with("TILT=") || line.to_uppercase().starts_with("TILT ")
            {
                // Save last keyword if any
                if !current_keyword.is_empty() {
                    Self::store_keyword(&mut data, &current_keyword, &current_value);
                }
                break;
            }

            // Parse [KEYWORD] value format
            if line.starts_with('[') {
                // Save previous keyword
                if !current_keyword.is_empty() {
                    Self::store_keyword(&mut data, &current_keyword, &current_value);
                    last_stored_keyword = current_keyword.clone();
                }

                if let Some(end_bracket) = line.find(']') {
                    current_keyword = line[1..end_bracket].to_uppercase();
                    current_value = line[end_bracket + 1..].trim().to_string();

                    // Handle [MORE] continuation (Annex A)
                    if current_keyword == "MORE" && !last_stored_keyword.is_empty() {
                        // Append to previous keyword's value
                        if let Some(existing) = data.keywords.get_mut(&last_stored_keyword) {
                            existing.push('\n');
                            existing.push_str(&current_value);
                        }
                        current_keyword.clear();
                        current_value.clear();
                    }
                }
            }

            line_idx += 1;
        }

        // Handle TILT
        if line_idx < lines.len() {
            let tilt_line = lines[line_idx].trim().to_uppercase();
            data.tilt_mode = if tilt_line.contains("INCLUDE") {
                "INCLUDE".to_string()
            } else {
                "NONE".to_string()
            };

            line_idx += 1;

            if tilt_line.contains("INCLUDE") {
                // Parse TILT data (Annex F)
                let mut tilt = TiltData::default();

                // Lamp to luminaire geometry
                if line_idx < lines.len() {
                    if let Ok(geom) = lines[line_idx].trim().parse::<i32>() {
                        tilt.lamp_geometry = geom;
                    }
                    line_idx += 1;
                }

                // Number of angle-factor pairs
                if line_idx < lines.len() {
                    if let Ok(n_pairs) = lines[line_idx].trim().parse::<usize>() {
                        line_idx += 1;

                        // Collect angle values
                        let mut angle_values: Vec<f64> = Vec::new();
                        while angle_values.len() < n_pairs && line_idx < lines.len() {
                            for token in lines[line_idx].split_whitespace() {
                                if let Ok(val) = token.replace(',', ".").parse::<f64>() {
                                    angle_values.push(val);
                                }
                            }
                            line_idx += 1;
                        }
                        tilt.angles = angle_values;

                        // Collect factor values
                        let mut factor_values: Vec<f64> = Vec::new();
                        while factor_values.len() < n_pairs && line_idx < lines.len() {
                            for token in lines[line_idx].split_whitespace() {
                                if let Ok(val) = token.replace(',', ".").parse::<f64>() {
                                    factor_values.push(val);
                                }
                            }
                            line_idx += 1;
                        }
                        tilt.factors = factor_values;
                    }
                }

                data.tilt_data = Some(tilt);
            }
        }

        // Collect remaining numeric data
        let mut numeric_values: Vec<f64> = Vec::new();
        while line_idx < lines.len() {
            let line = lines[line_idx].trim();
            for token in line.split_whitespace() {
                if let Ok(val) = token.replace(',', ".").parse::<f64>() {
                    numeric_values.push(val);
                }
            }
            line_idx += 1;
        }

        // Parse photometric data
        if numeric_values.len() < 13 {
            return Err(anyhow!(
                "Insufficient photometric data: expected at least 13 values, found {}",
                numeric_values.len()
            ));
        }

        let mut idx = 0;

        // Line 1: num_lamps, lumens_per_lamp, multiplier, n_vert, n_horiz, photo_type, units, width, length, height
        data.num_lamps = numeric_values[idx] as i32;
        idx += 1;
        data.lumens_per_lamp = numeric_values[idx];
        idx += 1;
        data.multiplier = numeric_values[idx];
        idx += 1;
        data.n_vertical = numeric_values[idx] as usize;
        idx += 1;
        data.n_horizontal = numeric_values[idx] as usize;
        idx += 1;
        data.photometric_type = PhotometricType::from_int(numeric_values[idx] as i32)?;
        idx += 1;
        data.unit_type = UnitType::from_int(numeric_values[idx] as i32)?;
        idx += 1;
        data.width = numeric_values[idx];
        idx += 1;
        data.length = numeric_values[idx];
        idx += 1;
        data.height = numeric_values[idx];
        idx += 1;

        // Determine luminous shape from dimensions
        data.luminous_shape = LuminousShape::from_dimensions(data.width, data.length, data.height);

        // Line 2: ballast_factor, file_generation_type, input_watts
        data.ballast_factor = numeric_values[idx];
        idx += 1;
        data.file_generation_value = numeric_values[idx];
        data.file_generation_type = FileGenerationType::from_value(data.file_generation_value);
        idx += 1;
        data.input_watts = numeric_values[idx];
        idx += 1;

        // Vertical angles
        if idx + data.n_vertical > numeric_values.len() {
            return Err(anyhow!("Insufficient vertical angle data"));
        }
        data.vertical_angles = numeric_values[idx..idx + data.n_vertical].to_vec();
        idx += data.n_vertical;

        // Horizontal angles
        if idx + data.n_horizontal > numeric_values.len() {
            return Err(anyhow!("Insufficient horizontal angle data"));
        }
        data.horizontal_angles = numeric_values[idx..idx + data.n_horizontal].to_vec();
        idx += data.n_horizontal;

        // Candela values: n_horizontal sets of n_vertical values
        let expected_candela = data.n_horizontal * data.n_vertical;
        if idx + expected_candela > numeric_values.len() {
            return Err(anyhow!(
                "Insufficient candela data: expected {}, remaining {}",
                expected_candela,
                numeric_values.len() - idx
            ));
        }

        for _ in 0..data.n_horizontal {
            let row: Vec<f64> = numeric_values[idx..idx + data.n_vertical].to_vec();
            data.candela_values.push(row);
            idx += data.n_vertical;
        }

        Ok(data)
    }

    /// Store a keyword value in the IesData structure.
    fn store_keyword(data: &mut IesData, keyword: &str, value: &str) {
        // Store in generic keywords map
        data.keywords.insert(keyword.to_string(), value.to_string());

        // Also store in specific fields for easy access
        match keyword {
            "TEST" => data.test = value.to_string(),
            "TESTLAB" => data.test_lab = value.to_string(),
            "ISSUEDATE" => data.issue_date = value.to_string(),
            "MANUFAC" => data.manufacturer = value.to_string(),
            "LUMCAT" => data.luminaire_catalog = value.to_string(),
            "LUMINAIRE" => data.luminaire = value.to_string(),
            "LAMPCAT" => data.lamp_catalog = value.to_string(),
            "LAMP" => data.lamp = value.to_string(),
            "BALLAST" => data.ballast = value.to_string(),
            "BALLASTCAT" => data.ballast_catalog = value.to_string(),
            "TESTDATE" => data.test_date = value.to_string(),
            "MAINTCAT" => data.maintenance_category = value.trim().parse().ok(),
            "DISTRIBUTION" => data.distribution = value.to_string(),
            "FLASHAREA" => data.flash_area = value.trim().parse().ok(),
            "COLORCONSTANT" => data.color_constant = value.trim().parse().ok(),
            "LAMPPOSITION" => {
                let parts: Vec<f64> = value
                    .split([' ', ','])
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if parts.len() >= 2 {
                    data.lamp_position = Some(LampPosition {
                        horizontal: parts[0],
                        vertical: parts[1],
                    });
                }
            }
            "NEARFIELD" => {
                let parts: Vec<f64> = value
                    .split([' ', ','])
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if parts.len() >= 3 {
                    data.near_field = Some((parts[0], parts[1], parts[2]));
                }
            }
            "FILEGENINFO" => {
                if data.file_gen_info.is_empty() {
                    data.file_gen_info = value.to_string();
                } else {
                    data.file_gen_info.push('\n');
                    data.file_gen_info.push_str(value);
                }
            }
            "SEARCH" => data.search = value.to_string(),
            "OTHER" => data.other.push(value.to_string()),
            _ => {
                // User-defined keywords (starting with _) are stored in generic map only
            }
        }
    }

    /// Convert parsed IES data to Eulumdat structure.
    fn convert_to_eulumdat(ies: IesData) -> Result<Eulumdat> {
        let mut ldt = Eulumdat::new();

        // Convert keywords to Eulumdat fields (use parsed fields, not raw keywords)
        ldt.identification = ies.manufacturer.clone();
        ldt.luminaire_name = ies.luminaire.clone();
        ldt.luminaire_number = ies.luminaire_catalog.clone();
        ldt.measurement_report_number = ies.test.clone();
        ldt.file_name = ies.test_lab.clone();
        // Store issue date in date_user field
        ldt.date_user = ies.issue_date.clone();

        // Type indicator based on dimensions
        ldt.type_indicator = if ies.length > ies.width * 2.0 {
            TypeIndicator::Linear
        } else {
            TypeIndicator::PointSourceSymmetric
        };

        // Determine symmetry from horizontal angles
        ldt.symmetry = Self::detect_symmetry(&ies.horizontal_angles);

        // Store angles
        ldt.c_angles = ies.horizontal_angles.clone();
        ldt.g_angles = ies.vertical_angles.clone();
        ldt.num_c_planes = ies.n_horizontal;
        ldt.num_g_planes = ies.n_vertical;

        // Calculate angle spacing
        if ldt.c_angles.len() >= 2 {
            ldt.c_plane_distance = ldt.c_angles[1] - ldt.c_angles[0];
        }
        if ldt.g_angles.len() >= 2 {
            ldt.g_plane_distance = ldt.g_angles[1] - ldt.g_angles[0];
        }

        // Convert dimensions to mm
        let mm_factor = ies.unit_type.to_mm_factor();
        ldt.length = ies.length * mm_factor;
        ldt.width = ies.width * mm_factor;
        ldt.height = ies.height * mm_factor;

        // Luminous area (assume same as luminaire for now)
        ldt.luminous_area_length = ldt.length;
        ldt.luminous_area_width = ldt.width;

        // Lamp set
        // CRITICAL: Handle absolute photometry (LED fixtures)
        // IES Standard: lumens_per_lamp = -1 signals absolute photometry
        // Eulumdat Convention: num_lamps must be NEGATIVE to signal absolute photometry
        // This is the "single most important fix for LED compatibility"
        let (num_lamps, total_flux) = if ies.lumens_per_lamp < 0.0 {
            // Absolute photometry: negative lamp count in LDT
            (-1, ies.lumens_per_lamp.abs() * ies.num_lamps.abs() as f64)
        } else {
            // Relative photometry: positive lamp count
            (ies.num_lamps, ies.lumens_per_lamp * ies.num_lamps as f64)
        };

        ldt.lamp_sets.push(LampSet {
            num_lamps,
            lamp_type: if ies.lamp.is_empty() {
                "Unknown".to_string()
            } else {
                ies.lamp.clone()
            },
            total_luminous_flux: total_flux,
            color_appearance: ies.keywords.get("COLORTEMP").cloned().unwrap_or_default(),
            color_rendering_group: ies.keywords.get("CRI").cloned().unwrap_or_default(),
            wattage_with_ballast: ies.input_watts,
        });

        // Store intensities (IES candela values are absolute, convert to cd/klm)
        // Eulumdat uses cd/1000lm, IES uses absolute candela
        let cd_to_cdklm = if total_flux > 0.0 {
            1000.0 / total_flux
        } else {
            1.0
        };

        ldt.intensities = ies
            .candela_values
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&v| v * cd_to_cdklm * ies.multiplier)
                    .collect()
            })
            .collect();

        // Photometric parameters
        ldt.conversion_factor = ies.multiplier;
        ldt.downward_flux_fraction = 0.0; // Will be calculated
        ldt.light_output_ratio = 100.0; // Default

        Ok(ldt)
    }

    /// Detect symmetry type from horizontal angles.
    fn detect_symmetry(h_angles: &[f64]) -> Symmetry {
        if h_angles.is_empty() {
            return Symmetry::None;
        }

        let min_angle = h_angles.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_angle = h_angles.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if h_angles.len() == 1 {
            // Single horizontal angle = rotationally symmetric
            Symmetry::VerticalAxis
        } else if (max_angle - 90.0).abs() < 0.1 && min_angle.abs() < 0.1 {
            // 0° to 90° = quadrant symmetry
            Symmetry::BothPlanes
        } else if (max_angle - 180.0).abs() < 0.1 && min_angle.abs() < 0.1 {
            // 0° to 180° = bilateral symmetry
            Symmetry::PlaneC0C180
        } else if (min_angle - 90.0).abs() < 0.1 && (max_angle - 270.0).abs() < 0.1 {
            // 90° to 270°
            Symmetry::PlaneC90C270
        } else {
            // Full 360° or other
            Symmetry::None
        }
    }
}

/// IES file format exporter.
///
/// Exports Eulumdat data to IES LM-63 format (2002 or 2019).
pub struct IesExporter;

/// Export options for IES files.
#[derive(Debug, Clone)]
pub struct IesExportOptions {
    /// IES version to export (default: LM-63-2019)
    pub version: IesVersion,
    /// File generation type (default: Undefined)
    pub file_generation_type: FileGenerationType,
    /// Issue date (required for LM-63-2019)
    pub issue_date: Option<String>,
    /// Additional file generation info
    pub file_gen_info: Option<String>,
    /// Test lab name
    pub test_lab: Option<String>,
}

impl Default for IesExportOptions {
    fn default() -> Self {
        Self {
            version: IesVersion::Lm63_2019,
            file_generation_type: FileGenerationType::Undefined,
            issue_date: None,
            file_gen_info: None,
            test_lab: None,
        }
    }
}

impl IesExporter {
    /// Export Eulumdat data to IES LM-63-2019 format (default).
    pub fn export(ldt: &Eulumdat) -> String {
        Self::export_with_options(ldt, &IesExportOptions::default())
    }

    /// Export Eulumdat data to IES LM-63-2002 format (legacy).
    pub fn export_2002(ldt: &Eulumdat) -> String {
        Self::export_with_options(
            ldt,
            &IesExportOptions {
                version: IesVersion::Lm63_2002,
                ..Default::default()
            },
        )
    }

    /// Export with custom options.
    pub fn export_with_options(ldt: &Eulumdat, options: &IesExportOptions) -> String {
        let mut output = String::new();

        // Header based on version
        output.push_str(options.version.header());
        output.push('\n');

        // Required keywords
        Self::write_keyword(&mut output, "TEST", &ldt.measurement_report_number);

        // TESTLAB - required in LM-63-2019
        let test_lab = options.test_lab.as_deref().unwrap_or(&ldt.file_name);
        if !test_lab.is_empty() {
            Self::write_keyword(&mut output, "TESTLAB", test_lab);
        }

        // ISSUEDATE - required in LM-63-2019
        if options.version == IesVersion::Lm63_2019 {
            let issue_date = options
                .issue_date
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if !ldt.date_user.is_empty() {
                        &ldt.date_user
                    } else {
                        // Default to current date if not provided
                        "01-JAN-2025"
                    }
                });
            Self::write_keyword(&mut output, "ISSUEDATE", issue_date);
        }

        // MANUFAC - required
        if !ldt.identification.is_empty() {
            Self::write_keyword(&mut output, "MANUFAC", &ldt.identification);
        }

        // Optional but recommended keywords
        Self::write_keyword(&mut output, "LUMCAT", &ldt.luminaire_number);
        Self::write_keyword(&mut output, "LUMINAIRE", &ldt.luminaire_name);

        if !ldt.lamp_sets.is_empty() {
            Self::write_keyword(&mut output, "LAMP", &ldt.lamp_sets[0].lamp_type);
            if ldt.lamp_sets[0].total_luminous_flux > 0.0 {
                Self::write_keyword(
                    &mut output,
                    "LAMPCAT",
                    &format!("{:.0} lm", ldt.lamp_sets[0].total_luminous_flux),
                );
            }
        }

        // FILEGENINFO - new in LM-63-2019
        if options.version == IesVersion::Lm63_2019 {
            if let Some(ref info) = options.file_gen_info {
                Self::write_keyword(&mut output, "FILEGENINFO", info);
            }
        }

        // TILT=NONE (most common)
        output.push_str("TILT=NONE\n");

        // Line 1: Number of lamps, lumens per lamp, multiplier, number of vertical angles,
        //         number of horizontal angles, photometric type, units type, width, length, height
        let num_lamps = ldt.lamp_sets.iter().map(|ls| ls.num_lamps).sum::<i32>();
        let total_flux = ldt.total_luminous_flux();

        // CRITICAL: Absolute photometry handling for LED fixtures
        // LDT Convention: negative num_lamps signals absolute photometry
        // IES Standard: lumens_per_lamp = -1 signals absolute photometry
        let lumens_per_lamp = if num_lamps < 0 {
            // Absolute photometry: output -1 to signal absolute mode
            -1.0
        } else if num_lamps > 0 {
            // Relative photometry: divide total flux by lamp count
            total_flux / num_lamps as f64
        } else {
            // Fallback: treat as absolute
            total_flux
        };

        // Expand to full distribution for IES
        let (h_angles, v_angles, intensities) = Self::prepare_photometric_data(ldt);

        // Photometric type: 1 = Type C (vertical angles from 0 at nadir)
        let photometric_type = 1;
        // Units: 1 = feet, 2 = meters
        let units_type = 2;

        // Dimensions in meters (convert from mm)
        let width = ldt.width / 1000.0;
        let length = ldt.length / 1000.0;
        let height = ldt.height / 1000.0;

        // For IES output, num_lamps should always be positive (1 for absolute mode)
        let ies_num_lamps = num_lamps.abs().max(1);

        output.push_str(&format!(
            "{} {:.1} {:.6} {} {} {} {} {:.4} {:.4} {:.4}\n",
            ies_num_lamps,
            lumens_per_lamp,
            ldt.conversion_factor.max(1.0),
            v_angles.len(),
            h_angles.len(),
            photometric_type,
            units_type,
            width,
            length,
            height
        ));

        // Line 2: Ballast factor, file generation type (LM-63-2019) or ballast-lamp factor, input watts
        let total_watts = ldt.total_wattage();
        let file_gen_value = if options.version == IesVersion::Lm63_2019 {
            options.file_generation_type.value()
        } else {
            1.0 // Legacy ballast-lamp photometric factor
        };
        output.push_str(&format!("1.0 {:.5} {:.1}\n", file_gen_value, total_watts));

        // Vertical angles
        output.push_str(&Self::format_values_multiline(&v_angles, 10));
        output.push('\n');

        // Horizontal angles
        output.push_str(&Self::format_values_multiline(&h_angles, 10));
        output.push('\n');

        // Candela values for each horizontal angle
        // Convert from cd/klm back to absolute candela
        let cdklm_to_cd = total_flux / 1000.0;
        for row in &intensities {
            let absolute_candela: Vec<f64> = row.iter().map(|&v| v * cdklm_to_cd).collect();
            output.push_str(&Self::format_values_multiline(&absolute_candela, 10));
            output.push('\n');
        }

        output
    }

    /// Write a keyword line.
    fn write_keyword(output: &mut String, keyword: &str, value: &str) {
        if !value.is_empty() {
            output.push_str(&format!("[{}] {}\n", keyword, value));
        }
    }

    /// Prepare photometric data for IES export.
    ///
    /// Returns (horizontal_angles, vertical_angles, intensities).
    fn prepare_photometric_data(ldt: &Eulumdat) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
        // IES uses vertical angles (0 = down, 90 = horizontal, 180 = up)
        // Same as Eulumdat G-angles
        let v_angles = ldt.g_angles.clone();

        // Horizontal angles depend on symmetry
        let (h_angles, intensities) = match ldt.symmetry {
            Symmetry::VerticalAxis => {
                // Single horizontal angle (0°)
                (
                    vec![0.0],
                    vec![ldt.intensities.first().cloned().unwrap_or_default()],
                )
            }
            Symmetry::PlaneC0C180 => {
                // 0° to 180°
                let expanded = SymmetryHandler::expand_to_full(ldt);
                let h = SymmetryHandler::expand_c_angles(ldt);
                // Select only the angles and intensities from 0° to 180°
                let mut h_filtered = Vec::new();
                let mut i_filtered = Vec::new();
                for (i, &angle) in h.iter().enumerate() {
                    if angle <= 180.0 && i < expanded.len() {
                        h_filtered.push(angle);
                        i_filtered.push(expanded[i].clone());
                    }
                }
                (h_filtered, i_filtered)
            }
            Symmetry::PlaneC90C270 => {
                // Full 0° to 360° for C90-C270 symmetry
                // IES format needs the complete distribution
                let expanded = SymmetryHandler::expand_to_full(ldt);
                let h = SymmetryHandler::expand_c_angles(ldt);
                (h, expanded)
            }
            Symmetry::BothPlanes => {
                // 0° to 90°
                let h: Vec<f64> = ldt
                    .c_angles
                    .iter()
                    .filter(|&&a| a <= 90.0)
                    .copied()
                    .collect();
                let i: Vec<Vec<f64>> = ldt.intensities.iter().take(h.len()).cloned().collect();
                (h, i)
            }
            Symmetry::None => {
                // Full 0° to 360°
                (ldt.c_angles.clone(), ldt.intensities.clone())
            }
        };

        (h_angles, v_angles, intensities)
    }

    /// Format values with line wrapping.
    fn format_values_multiline(values: &[f64], per_line: usize) -> String {
        values
            .chunks(per_line)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|&v| format!("{:.2}", v))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ies_export() {
        let mut ldt = Eulumdat::new();
        ldt.identification = "Test Manufacturer".to_string();
        ldt.luminaire_name = "Test Luminaire".to_string();
        ldt.luminaire_number = "LUM-001".to_string();
        ldt.measurement_report_number = "TEST-001".to_string();
        ldt.symmetry = Symmetry::VerticalAxis;
        ldt.num_c_planes = 1;
        ldt.num_g_planes = 5;
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 22.5, 45.0, 67.5, 90.0];
        ldt.intensities = vec![vec![1000.0, 900.0, 700.0, 400.0, 100.0]];
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        ldt.conversion_factor = 1.0;
        ldt.length = 100.0;
        ldt.width = 100.0;
        ldt.height = 50.0;

        let ies = IesExporter::export(&ldt);

        // Default export is now LM-63-2019
        assert!(ies.contains("IES:LM-63-2019"));
        assert!(ies.contains("[LUMINAIRE] Test Luminaire"));
        assert!(ies.contains("[MANUFAC] Test Manufacturer"));
        assert!(ies.contains("[ISSUEDATE]")); // Required in 2019
        assert!(ies.contains("TILT=NONE"));

        // Test legacy 2002 export
        let ies_2002 = IesExporter::export_2002(&ldt);
        assert!(ies_2002.contains("IESNA:LM-63-2002"));
        assert!(!ies_2002.contains("[ISSUEDATE]")); // Not required in 2002
    }

    #[test]
    fn test_ies_parse() {
        let ies_content = r#"IESNA:LM-63-2002
[TEST] TEST-001
[MANUFAC] Test Company
[LUMINAIRE] Test Fixture
[LAMP] LED Module
TILT=NONE
1 1000.0 1.0 5 1 1 2 0.1 0.1 0.05
1.0 1.0 10.0
0.0 22.5 45.0 67.5 90.0
0.0
1000.0 900.0 700.0 400.0 100.0
"#;

        let ldt = IesParser::parse(ies_content).expect("Failed to parse IES");

        assert_eq!(ldt.luminaire_name, "Test Fixture");
        assert_eq!(ldt.identification, "Test Company");
        assert_eq!(ldt.measurement_report_number, "TEST-001");
        assert_eq!(ldt.g_angles.len(), 5);
        assert_eq!(ldt.c_angles.len(), 1);
        assert_eq!(ldt.symmetry, Symmetry::VerticalAxis);
        assert!(!ldt.intensities.is_empty());
    }

    #[test]
    fn test_ies_roundtrip() {
        let mut ldt = Eulumdat::new();
        ldt.identification = "Roundtrip Test".to_string();
        ldt.luminaire_name = "Test Luminaire".to_string();
        ldt.symmetry = Symmetry::VerticalAxis;
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        ldt.intensities = vec![vec![500.0, 400.0, 200.0]];
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            ..Default::default()
        });
        ldt.length = 100.0;
        ldt.width = 100.0;
        ldt.height = 50.0;

        // Export to IES
        let ies = IesExporter::export(&ldt);

        // Parse back
        let parsed = IesParser::parse(&ies).expect("Failed to parse exported IES");

        // Verify key fields
        assert_eq!(parsed.luminaire_name, ldt.luminaire_name);
        assert_eq!(parsed.g_angles.len(), ldt.g_angles.len());
        assert_eq!(parsed.symmetry, Symmetry::VerticalAxis);
    }

    #[test]
    fn test_detect_symmetry() {
        assert_eq!(IesParser::detect_symmetry(&[0.0]), Symmetry::VerticalAxis);
        assert_eq!(
            IesParser::detect_symmetry(&[0.0, 45.0, 90.0]),
            Symmetry::BothPlanes
        );
        assert_eq!(
            IesParser::detect_symmetry(&[0.0, 45.0, 90.0, 135.0, 180.0]),
            Symmetry::PlaneC0C180
        );
        assert_eq!(
            IesParser::detect_symmetry(&[0.0, 90.0, 180.0, 270.0, 360.0]),
            Symmetry::None
        );
    }

    #[test]
    fn test_photometric_type() {
        assert_eq!(
            PhotometricType::from_int(1).unwrap(),
            PhotometricType::TypeC
        );
        assert_eq!(
            PhotometricType::from_int(2).unwrap(),
            PhotometricType::TypeB
        );
        assert_eq!(
            PhotometricType::from_int(3).unwrap(),
            PhotometricType::TypeA
        );
        assert!(PhotometricType::from_int(0).is_err());
    }

    #[test]
    fn test_unit_conversion() {
        assert!((UnitType::Feet.to_mm_factor() - 304.8).abs() < 0.01);
        assert!((UnitType::Meters.to_mm_factor() - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_ies_version_parsing() {
        assert_eq!(
            IesVersion::from_header("IES:LM-63-2019"),
            IesVersion::Lm63_2019
        );
        assert_eq!(
            IesVersion::from_header("IESNA:LM-63-2002"),
            IesVersion::Lm63_2002
        );
        assert_eq!(
            IesVersion::from_header("IESNA:LM-63-1995"),
            IesVersion::Lm63_1995
        );
        assert_eq!(IesVersion::from_header("IESNA91"), IesVersion::Lm63_1991);
    }

    #[test]
    fn test_file_generation_type() {
        assert_eq!(
            FileGenerationType::from_value(1.00001),
            FileGenerationType::Undefined
        );
        assert_eq!(
            FileGenerationType::from_value(1.00010),
            FileGenerationType::ComputerSimulation
        );
        assert_eq!(
            FileGenerationType::from_value(1.10000),
            FileGenerationType::AccreditedLab
        );
        assert_eq!(
            FileGenerationType::from_value(1.10100),
            FileGenerationType::AccreditedLabScaled
        );

        // Test accredited lab check
        assert!(FileGenerationType::AccreditedLab.is_accredited());
        assert!(!FileGenerationType::UnaccreditedLab.is_accredited());

        // Test scaled check
        assert!(FileGenerationType::AccreditedLabScaled.is_scaled());
        assert!(!FileGenerationType::AccreditedLab.is_scaled());
    }

    #[test]
    fn test_luminous_shape() {
        // Point source
        assert_eq!(
            LuminousShape::from_dimensions(0.0, 0.0, 0.0),
            LuminousShape::Point
        );

        // Rectangular
        assert_eq!(
            LuminousShape::from_dimensions(0.5, 0.6, 0.0),
            LuminousShape::Rectangular
        );

        // Circular (negative equal width/length)
        assert_eq!(
            LuminousShape::from_dimensions(-0.3, -0.3, 0.0),
            LuminousShape::Circular
        );

        // Sphere (all negative equal)
        assert_eq!(
            LuminousShape::from_dimensions(-0.2, -0.2, -0.2),
            LuminousShape::Sphere
        );
    }

    #[test]
    fn test_ies_2019_parse() {
        let ies_content = r#"IES:LM-63-2019
[TEST] ABC1234
[TESTLAB] ABC Laboratories
[ISSUEDATE] 28-FEB-2019
[MANUFAC] Test Company
[LUMCAT] SKYVIEW-123
[LUMINAIRE] LED Wide beam flood
[LAMP] LED Module
[FILEGENINFO] This file was generated from test data
TILT=NONE
1 -1 1.0 5 1 1 2 0.1 0.1 0.0
1.0 1.10000 50.0
0.0 22.5 45.0 67.5 90.0
0.0
1000.0 900.0 700.0 400.0 100.0
"#;

        let ies_data = IesParser::parse_to_ies_data(ies_content).expect("Failed to parse IES");

        assert_eq!(ies_data.version, IesVersion::Lm63_2019);
        assert_eq!(ies_data.test, "ABC1234");
        assert_eq!(ies_data.test_lab, "ABC Laboratories");
        assert_eq!(ies_data.issue_date, "28-FEB-2019");
        assert_eq!(ies_data.manufacturer, "Test Company");
        assert_eq!(
            ies_data.file_generation_type,
            FileGenerationType::AccreditedLab
        );
        assert_eq!(
            ies_data.file_gen_info,
            "This file was generated from test data"
        );
        assert_eq!(ies_data.lumens_per_lamp, -1.0); // Absolute photometry
    }

    #[test]
    fn test_ies_tilt_include() {
        let ies_content = r#"IES:LM-63-2019
[TEST] TILT-TEST
[TESTLAB] Test Lab
[ISSUEDATE] 01-JAN-2020
[MANUFAC] Test Mfg
TILT=INCLUDE
1
7
0 15 30 45 60 75 90
1.0 0.95 0.94 0.90 0.88 0.87 0.94
1 1000.0 1.0 3 1 1 2 0.1 0.1 0.0
1.0 1.00001 10.0
0.0 45.0 90.0
0.0
100.0 80.0 50.0
"#;

        let ies_data = IesParser::parse_to_ies_data(ies_content).expect("Failed to parse");

        assert_eq!(ies_data.tilt_mode, "INCLUDE");
        assert!(ies_data.tilt_data.is_some());

        let tilt = ies_data.tilt_data.as_ref().unwrap();
        assert_eq!(tilt.lamp_geometry, 1);
        assert_eq!(tilt.angles.len(), 7);
        assert_eq!(tilt.factors.len(), 7);
        assert!((tilt.angles[0] - 0.0).abs() < 0.001);
        assert!((tilt.factors[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_more_continuation() {
        let ies_content = r#"IES:LM-63-2019
[TEST] MORE-TEST
[TESTLAB] Test Lab
[ISSUEDATE] 01-JAN-2020
[MANUFAC] Test Manufacturer
[OTHER] This is the first line of other info
[MORE] This is the second line of other info
TILT=NONE
1 1000.0 1.0 3 1 1 2 0.1 0.1 0.0
1.0 1.00001 10.0
0.0 45.0 90.0
0.0
100.0 80.0 50.0
"#;

        let ies_data = IesParser::parse_to_ies_data(ies_content).expect("Failed to parse");

        // Check that OTHER contains multi-line content via MORE
        let other_value = ies_data.keywords.get("OTHER").expect("OTHER not found");
        assert!(other_value.contains("first line"));
        assert!(other_value.contains("second line"));
    }
}

// === IES Validation ===

/// IES-specific validation warning.
#[derive(Debug, Clone, PartialEq)]
pub struct IesValidationWarning {
    /// Warning code
    pub code: &'static str,
    /// Human-readable message
    pub message: String,
    /// Severity level
    pub severity: IesValidationSeverity,
}

/// Severity level for IES validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IesValidationSeverity {
    /// Required by LM-63-2019 specification
    Required,
    /// Recommended but not strictly required
    Recommended,
    /// Informational warning
    Info,
}

impl std::fmt::Display for IesValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity {
            IesValidationSeverity::Required => "ERROR",
            IesValidationSeverity::Recommended => "WARNING",
            IesValidationSeverity::Info => "INFO",
        };
        write!(f, "[{}:{}] {}", self.code, severity, self.message)
    }
}

/// Validate IES data according to LM-63-2019 specification.
pub fn validate_ies(data: &IesData) -> Vec<IesValidationWarning> {
    let mut warnings = Vec::new();

    // === Required Keywords (Section 5.2) ===

    // IES001: [TEST] is required
    if data.test.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES001",
            message: "Missing required keyword [TEST]".to_string(),
            severity: IesValidationSeverity::Required,
        });
    }

    // IES002: [TESTLAB] is required
    if data.test_lab.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES002",
            message: "Missing required keyword [TESTLAB]".to_string(),
            severity: IesValidationSeverity::Required,
        });
    }

    // IES003: [ISSUEDATE] is required (LM-63-2019)
    if data.version == IesVersion::Lm63_2019 && data.issue_date.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES003",
            message: "Missing required keyword [ISSUEDATE] for LM-63-2019".to_string(),
            severity: IesValidationSeverity::Required,
        });
    }

    // IES004: [MANUFAC] is required
    if data.manufacturer.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES004",
            message: "Missing required keyword [MANUFAC]".to_string(),
            severity: IesValidationSeverity::Required,
        });
    }

    // === Recommended Keywords ===

    // IES005: [LUMCAT] recommended
    if data.luminaire_catalog.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES005",
            message: "Missing recommended keyword [LUMCAT]".to_string(),
            severity: IesValidationSeverity::Recommended,
        });
    }

    // IES006: [LUMINAIRE] recommended
    if data.luminaire.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES006",
            message: "Missing recommended keyword [LUMINAIRE]".to_string(),
            severity: IesValidationSeverity::Recommended,
        });
    }

    // IES007: [LAMP] recommended
    if data.lamp.is_empty() {
        warnings.push(IesValidationWarning {
            code: "IES007",
            message: "Missing recommended keyword [LAMP]".to_string(),
            severity: IesValidationSeverity::Recommended,
        });
    }

    // === Photometric Type Validation (Section 5.9) ===

    // IES010: Valid photometric type
    let photo_type = data.photometric_type as i32;
    if !(1..=3).contains(&photo_type) {
        warnings.push(IesValidationWarning {
            code: "IES010",
            message: format!(
                "Invalid photometric type: {} (must be 1, 2, or 3)",
                photo_type
            ),
            severity: IesValidationSeverity::Required,
        });
    }

    // === Unit Type Validation (Section 5.10.1) ===

    let unit_type = data.unit_type as i32;
    if !(1..=2).contains(&unit_type) {
        warnings.push(IesValidationWarning {
            code: "IES011",
            message: format!(
                "Invalid unit type: {} (must be 1=feet or 2=meters)",
                unit_type
            ),
            severity: IesValidationSeverity::Required,
        });
    }

    // === Vertical Angle Validation (Section 5.15) ===

    if !data.vertical_angles.is_empty() {
        let first_v = data.vertical_angles[0];
        let last_v = *data.vertical_angles.last().unwrap();

        // Type C photometry
        if data.photometric_type == PhotometricType::TypeC {
            // First angle must be 0 or 90
            if (first_v - 0.0).abs() > 0.01 && (first_v - 90.0).abs() > 0.01 {
                warnings.push(IesValidationWarning {
                    code: "IES020",
                    message: format!(
                        "Type C: First vertical angle ({}) must be 0 or 90 degrees",
                        first_v
                    ),
                    severity: IesValidationSeverity::Required,
                });
            }

            // Last angle must be 90 or 180
            if (last_v - 90.0).abs() > 0.01 && (last_v - 180.0).abs() > 0.01 {
                warnings.push(IesValidationWarning {
                    code: "IES021",
                    message: format!(
                        "Type C: Last vertical angle ({}) must be 90 or 180 degrees",
                        last_v
                    ),
                    severity: IesValidationSeverity::Required,
                });
            }
        }

        // Type A or B photometry
        if data.photometric_type == PhotometricType::TypeA
            || data.photometric_type == PhotometricType::TypeB
        {
            // First angle must be -90 or 0
            if (first_v - 0.0).abs() > 0.01 && (first_v + 90.0).abs() > 0.01 {
                warnings.push(IesValidationWarning {
                    code: "IES022",
                    message: format!(
                        "Type A/B: First vertical angle ({}) must be -90 or 0 degrees",
                        first_v
                    ),
                    severity: IesValidationSeverity::Required,
                });
            }

            // Last angle must be 90
            if (last_v - 90.0).abs() > 0.01 {
                warnings.push(IesValidationWarning {
                    code: "IES023",
                    message: format!(
                        "Type A/B: Last vertical angle ({}) must be 90 degrees",
                        last_v
                    ),
                    severity: IesValidationSeverity::Required,
                });
            }
        }

        // Check ascending order
        for i in 1..data.vertical_angles.len() {
            if data.vertical_angles[i] <= data.vertical_angles[i - 1] {
                warnings.push(IesValidationWarning {
                    code: "IES024",
                    message: format!("Vertical angles not in ascending order at index {}", i),
                    severity: IesValidationSeverity::Required,
                });
                break;
            }
        }
    }

    // === Horizontal Angle Validation (Section 5.16) ===

    if !data.horizontal_angles.is_empty() {
        let first_h = data.horizontal_angles[0];
        let last_h = *data.horizontal_angles.last().unwrap();

        // Type C: first must be 0
        if data.photometric_type == PhotometricType::TypeC {
            if (first_h - 0.0).abs() > 0.01 {
                warnings.push(IesValidationWarning {
                    code: "IES030",
                    message: format!(
                        "Type C: First horizontal angle ({}) must be 0 degrees",
                        first_h
                    ),
                    severity: IesValidationSeverity::Required,
                });
            }

            // Last must be 0, 90, 180, or 360
            let valid_last = [
                (0.0, "laterally symmetric"),
                (90.0, "quadrant symmetric"),
                (180.0, "bilateral symmetric"),
                (360.0, "no lateral symmetry"),
            ];
            let mut found_valid = false;
            for (angle, _) in &valid_last {
                if (last_h - angle).abs() < 0.01 {
                    found_valid = true;
                    break;
                }
            }
            if !found_valid && data.horizontal_angles.len() > 1 {
                warnings.push(IesValidationWarning {
                    code: "IES031",
                    message: format!(
                        "Type C: Last horizontal angle ({}) must be 0, 90, 180, or 360 degrees",
                        last_h
                    ),
                    severity: IesValidationSeverity::Required,
                });
            }
        }

        // Check ascending order
        for i in 1..data.horizontal_angles.len() {
            if data.horizontal_angles[i] <= data.horizontal_angles[i - 1] {
                warnings.push(IesValidationWarning {
                    code: "IES032",
                    message: format!("Horizontal angles not in ascending order at index {}", i),
                    severity: IesValidationSeverity::Required,
                });
                break;
            }
        }
    }

    // === Data Dimension Validation ===

    // IES040: Verify candela data dimensions
    if data.candela_values.len() != data.n_horizontal {
        warnings.push(IesValidationWarning {
            code: "IES040",
            message: format!(
                "Candela data has {} horizontal planes, expected {}",
                data.candela_values.len(),
                data.n_horizontal
            ),
            severity: IesValidationSeverity::Required,
        });
    }

    for (i, row) in data.candela_values.iter().enumerate() {
        if row.len() != data.n_vertical {
            warnings.push(IesValidationWarning {
                code: "IES041",
                message: format!(
                    "Candela row {} has {} values, expected {}",
                    i,
                    row.len(),
                    data.n_vertical
                ),
                severity: IesValidationSeverity::Required,
            });
        }
    }

    // === File Generation Type Validation (LM-63-2019) ===

    if data.version == IesVersion::Lm63_2019 {
        // Check if file generation type is valid
        let valid_values = [
            1.00001, 1.00010, 1.00000, 1.00100, 1.01000, 1.01100, 1.10000, 1.10100, 1.11000,
            1.11100,
        ];
        let mut found = false;
        for &v in &valid_values {
            if (data.file_generation_value - v).abs() < 0.000001 {
                found = true;
                break;
            }
        }
        // Only warn if it looks like a non-legacy value
        if !found && data.file_generation_value > 1.0 {
            warnings.push(IesValidationWarning {
                code: "IES050",
                message: format!(
                    "File generation type value ({}) is not a standard LM-63-2019 value",
                    data.file_generation_value
                ),
                severity: IesValidationSeverity::Info,
            });
        }
    }

    // === Ballast Factor Validation ===

    if data.ballast_factor <= 0.0 || data.ballast_factor > 2.0 {
        warnings.push(IesValidationWarning {
            code: "IES060",
            message: format!(
                "Unusual ballast factor: {} (typically 0.5-1.5)",
                data.ballast_factor
            ),
            severity: IesValidationSeverity::Info,
        });
    }

    // === Candela Value Validation ===

    let mut has_negative = false;
    let mut max_cd = 0.0f64;
    for row in &data.candela_values {
        for &cd in row {
            if cd < 0.0 {
                has_negative = true;
            }
            max_cd = max_cd.max(cd);
        }
    }

    if has_negative {
        warnings.push(IesValidationWarning {
            code: "IES070",
            message: "Negative candela values found".to_string(),
            severity: IesValidationSeverity::Required,
        });
    }

    if max_cd > 1_000_000.0 {
        warnings.push(IesValidationWarning {
            code: "IES071",
            message: format!(
                "Very high candela value: {:.0} (verify data correctness)",
                max_cd
            ),
            severity: IesValidationSeverity::Info,
        });
    }

    warnings
}

/// Get required warnings only (errors).
pub fn validate_ies_strict(data: &IesData) -> Vec<IesValidationWarning> {
    validate_ies(data)
        .into_iter()
        .filter(|w| w.severity == IesValidationSeverity::Required)
        .collect()
}
