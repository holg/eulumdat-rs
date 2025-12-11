//! Core Eulumdat data structure.

use crate::error::{invalid_value, Result};
use crate::parser::Parser;
use crate::validation::{ValidationError, ValidationWarning};
use crate::writer::Writer;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type indicator for the luminaire.
///
/// Defines the type of light source and its symmetry characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TypeIndicator {
    /// Point source with symmetry about the vertical axis (Ityp = 1)
    #[default]
    PointSourceSymmetric = 1,
    /// Linear luminaire (Ityp = 2)
    Linear = 2,
    /// Point source with any other symmetry (Ityp = 3)
    PointSourceOther = 3,
}

impl TypeIndicator {
    /// Create from integer value.
    pub fn from_int(value: i32) -> Result<Self> {
        match value {
            1 => Ok(Self::PointSourceSymmetric),
            2 => Ok(Self::Linear),
            3 => Ok(Self::PointSourceOther),
            _ => Err(invalid_value(
                "type_indicator",
                format!("value {} is out of range (1-3)", value),
            )),
        }
    }

    /// Convert to integer value.
    pub fn as_int(&self) -> i32 {
        *self as i32
    }
}

/// Symmetry indicator for the luminaire.
///
/// Defines how the luminous intensity distribution is symmetric,
/// which affects how much data needs to be stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Symmetry {
    /// No symmetry (Isym = 0) - full 360° data required
    #[default]
    None = 0,
    /// Symmetry about the vertical axis (Isym = 1) - only 1 C-plane needed
    VerticalAxis = 1,
    /// Symmetry to plane C0-C180 (Isym = 2) - half the C-planes needed
    PlaneC0C180 = 2,
    /// Symmetry to plane C90-C270 (Isym = 3) - half the C-planes needed
    PlaneC90C270 = 3,
    /// Symmetry to both planes C0-C180 and C90-C270 (Isym = 4) - quarter C-planes needed
    BothPlanes = 4,
}

impl Symmetry {
    /// Create from integer value.
    pub fn from_int(value: i32) -> Result<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::VerticalAxis),
            2 => Ok(Self::PlaneC0C180),
            3 => Ok(Self::PlaneC90C270),
            4 => Ok(Self::BothPlanes),
            _ => Err(invalid_value(
                "symmetry",
                format!("value {} is out of range (0-4)", value),
            )),
        }
    }

    /// Convert to integer value.
    pub fn as_int(&self) -> i32 {
        *self as i32
    }

    /// Calculate the actual number of C-planes needed based on symmetry.
    ///
    /// This is the key optimization that reduces storage requirements:
    /// - No symmetry: all Nc planes
    /// - Vertical axis: 1 plane (360x reduction!)
    /// - C0-C180: Nc/2 + 1 planes (2x reduction)
    /// - C90-C270: Nc/2 + 1 planes (2x reduction)
    /// - Both planes: Nc/4 + 1 planes (4x reduction)
    pub fn calc_mc(&self, nc: usize) -> usize {
        match self {
            Symmetry::None => nc,
            Symmetry::VerticalAxis => 1,
            Symmetry::PlaneC0C180 | Symmetry::PlaneC90C270 => nc / 2 + 1,
            Symmetry::BothPlanes => nc / 4 + 1,
        }
    }

    /// Get human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Symmetry::None => "no symmetry",
            Symmetry::VerticalAxis => "symmetry about the vertical axis",
            Symmetry::PlaneC0C180 => "symmetry to plane C0-C180",
            Symmetry::PlaneC90C270 => "symmetry to plane C90-C270",
            Symmetry::BothPlanes => "symmetry to plane C0-C180 and to plane C90-C270",
        }
    }
}

/// Lamp set configuration.
///
/// A luminaire can have up to 20 lamp sets, each describing a group of lamps.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LampSet {
    /// Number of lamps in this set.
    pub num_lamps: i32,
    /// Type of lamps (description string).
    pub lamp_type: String,
    /// Total luminous flux of this lamp set in lumens.
    pub total_luminous_flux: f64,
    /// Color appearance / color temperature.
    pub color_appearance: String,
    /// Color rendering group / CRI.
    pub color_rendering_group: String,
    /// Wattage including ballast in watts.
    pub wattage_with_ballast: f64,
}

/// Main Eulumdat data structure.
///
/// This struct contains all data from an Eulumdat (LDT) file.
/// The structure follows the official Eulumdat specification.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Eulumdat {
    // === Identification ===
    /// Identification string (line 1).
    pub identification: String,

    // === Type and Symmetry ===
    /// Type indicator (1-3).
    pub type_indicator: TypeIndicator,
    /// Symmetry indicator (0-4).
    pub symmetry: Symmetry,

    // === Grid Definition ===
    /// Number of C-planes between 0° and 360° (Nc, 0-721).
    pub num_c_planes: usize,
    /// Distance between C-planes in degrees (Dc).
    pub c_plane_distance: f64,
    /// Number of gamma/G-planes between 0° and 180° (Ng, 0-361).
    pub num_g_planes: usize,
    /// Distance between G-planes in degrees (Dg).
    pub g_plane_distance: f64,

    // === Metadata ===
    /// Measurement report number.
    pub measurement_report_number: String,
    /// Luminaire name.
    pub luminaire_name: String,
    /// Luminaire number.
    pub luminaire_number: String,
    /// File name.
    pub file_name: String,
    /// Date/user field.
    pub date_user: String,

    // === Physical Dimensions (in mm) ===
    /// Length/diameter of luminaire (L).
    pub length: f64,
    /// Width of luminaire (B), 0 for circular.
    pub width: f64,
    /// Height of luminaire (H).
    pub height: f64,
    /// Length/diameter of luminous area (La).
    pub luminous_area_length: f64,
    /// Width of luminous area (B1), 0 for circular.
    pub luminous_area_width: f64,
    /// Height of luminous area at C0 plane (HC0).
    pub height_c0: f64,
    /// Height of luminous area at C90 plane (HC90).
    pub height_c90: f64,
    /// Height of luminous area at C180 plane (HC180).
    pub height_c180: f64,
    /// Height of luminous area at C270 plane (HC270).
    pub height_c270: f64,

    // === Optical Properties ===
    /// Downward flux fraction (DFF) in percent.
    pub downward_flux_fraction: f64,
    /// Light output ratio of luminaire (LORL) in percent.
    pub light_output_ratio: f64,
    /// Conversion factor for luminous intensities (CFLI).
    pub conversion_factor: f64,
    /// Tilt angle during measurement in degrees.
    pub tilt_angle: f64,

    // === Lamp Configuration ===
    /// Lamp sets (1-20).
    pub lamp_sets: Vec<LampSet>,

    // === Utilization Factors ===
    /// Direct ratios for room indices k = 0.60, 0.80, 1.00, 1.25, 1.50, 2.00, 2.50, 3.00, 4.00, 5.00
    pub direct_ratios: [f64; 10],

    // === Photometric Data ===
    /// C-plane angles in degrees.
    pub c_angles: Vec<f64>,
    /// G-plane (gamma) angles in degrees.
    pub g_angles: Vec<f64>,
    /// Luminous intensity distribution in cd/klm.
    /// Indexed as `intensities[c_plane_index][g_plane_index]`.
    pub intensities: Vec<Vec<f64>>,
}

impl Default for Eulumdat {
    fn default() -> Self {
        Self {
            identification: String::new(),
            type_indicator: TypeIndicator::default(),
            symmetry: Symmetry::default(),
            num_c_planes: 0,
            c_plane_distance: 0.0,
            num_g_planes: 0,
            g_plane_distance: 0.0,
            measurement_report_number: String::new(),
            luminaire_name: String::new(),
            luminaire_number: String::new(),
            file_name: String::new(),
            date_user: String::new(),
            length: 0.0,
            width: 0.0,
            height: 0.0,
            luminous_area_length: 0.0,
            luminous_area_width: 0.0,
            height_c0: 0.0,
            height_c90: 0.0,
            height_c180: 0.0,
            height_c270: 0.0,
            downward_flux_fraction: 0.0,
            light_output_ratio: 0.0,
            conversion_factor: 1.0,
            tilt_angle: 0.0,
            lamp_sets: Vec::new(),
            direct_ratios: [0.0; 10],
            c_angles: Vec::new(),
            g_angles: Vec::new(),
            intensities: Vec::new(),
        }
    }
}

impl Eulumdat {
    /// Create a new empty Eulumdat structure.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from a file path.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse from a string.
    pub fn parse(content: &str) -> Result<Self> {
        Parser::parse(content)
    }

    /// Save to a file path.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let content = self.to_ldt();
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to LDT format string.
    pub fn to_ldt(&self) -> String {
        Writer::write(self)
    }

    /// Validate the data and return any warnings.
    pub fn validate(&self) -> Vec<ValidationWarning> {
        crate::validation::validate(self)
    }

    /// Validate strictly and return errors if validation fails.
    pub fn validate_strict(&self) -> std::result::Result<(), Vec<ValidationError>> {
        crate::validation::validate_strict(self)
    }

    /// Get the actual number of C-planes based on symmetry (Mc).
    pub fn actual_c_planes(&self) -> usize {
        self.symmetry.calc_mc(self.num_c_planes)
    }

    /// Get total luminous flux from all lamp sets.
    pub fn total_luminous_flux(&self) -> f64 {
        self.lamp_sets.iter().map(|ls| ls.total_luminous_flux).sum()
    }

    /// Get total wattage from all lamp sets.
    pub fn total_wattage(&self) -> f64 {
        self.lamp_sets
            .iter()
            .map(|ls| ls.wattage_with_ballast)
            .sum()
    }

    /// Get luminous efficacy in lm/W.
    pub fn luminous_efficacy(&self) -> f64 {
        let wattage = self.total_wattage();
        if wattage > 0.0 {
            self.total_luminous_flux() / wattage
        } else {
            0.0
        }
    }

    /// Get intensity at a specific C and G angle.
    ///
    /// Returns None if the indices are out of bounds.
    pub fn get_intensity(&self, c_index: usize, g_index: usize) -> Option<f64> {
        self.intensities
            .get(c_index)
            .and_then(|row| row.get(g_index).copied())
    }

    /// Get the maximum intensity value.
    pub fn max_intensity(&self) -> f64 {
        self.intensities
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .fold(0.0, f64::max)
    }

    /// Get the minimum intensity value.
    pub fn min_intensity(&self) -> f64 {
        self.intensities
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .fold(f64::MAX, f64::min)
    }

    /// Get the average intensity value.
    pub fn avg_intensity(&self) -> f64 {
        let total: f64 = self.intensities.iter().flat_map(|row| row.iter()).sum();
        let count = self.intensities.iter().map(|row| row.len()).sum::<usize>();
        if count > 0 {
            total / count as f64
        } else {
            0.0
        }
    }

    /// Sample intensity at any C and G angle using bilinear interpolation.
    ///
    /// This is the key method for generating beam meshes and smooth geometry.
    /// It handles symmetry automatically and interpolates between stored data points.
    ///
    /// # Arguments
    /// * `c_angle` - C-plane angle in degrees (0-360, will be normalized)
    /// * `g_angle` - Gamma angle in degrees (0-180, will be clamped)
    ///
    /// # Returns
    /// Interpolated intensity value in cd/klm
    ///
    /// # Example
    /// ```rust,no_run
    /// use eulumdat::Eulumdat;
    ///
    /// let ldt = Eulumdat::from_file("luminaire.ldt")?;
    ///
    /// // Sample at exact stored angles
    /// let intensity = ldt.sample(0.0, 45.0);
    ///
    /// // Sample at arbitrary angles (will interpolate)
    /// let intensity = ldt.sample(22.5, 67.5);
    ///
    /// // Generate smooth beam mesh at 5° intervals
    /// for c in (0..360).step_by(5) {
    ///     for g in (0..=180).step_by(5) {
    ///         let intensity = ldt.sample(c as f64, g as f64);
    ///         // Use intensity for mesh generation...
    ///     }
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn sample(&self, c_angle: f64, g_angle: f64) -> f64 {
        crate::symmetry::SymmetryHandler::get_intensity_at(self, c_angle, g_angle)
    }
}
