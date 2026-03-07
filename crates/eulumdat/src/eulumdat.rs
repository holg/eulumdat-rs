//! Core Eulumdat data structure.

use std::path::Path;

use crate::error::{anyhow, invalid_value, Result};
use crate::parser::Parser;
use crate::validation::{ValidationError, ValidationWarning};
use crate::writer::Writer;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Read file with encoding fallback.
///
/// Tries UTF-8 first, then falls back to ISO-8859-1 (Latin-1) which is common
/// for LDT/IES files from Windows tools.
fn read_with_encoding_fallback<P: AsRef<Path>>(path: P) -> Result<String> {
    let bytes = std::fs::read(path.as_ref()).map_err(|e| anyhow!("Failed to read file: {}", e))?;

    // Try UTF-8 first
    match String::from_utf8(bytes.clone()) {
        Ok(content) => Ok(content),
        Err(_) => {
            // Fall back to ISO-8859-1 (Latin-1)
            // Every byte is valid in ISO-8859-1, so this always succeeds
            Ok(bytes.iter().map(|&b| b as char).collect())
        }
    }
}

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
    ///
    /// Automatically handles both UTF-8 and ISO-8859-1 (Latin-1) encoded files.
    /// This is necessary because many LDT files from Windows-based tools use
    /// ISO-8859-1 encoding.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = read_with_encoding_fallback(path)?;
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

    /// Rotate the C-plane data by the given number of degrees.
    ///
    /// This is useful when different manufacturers mount luminaires differently
    /// on goniophotometers, so C0 may point along the length or width axis.
    ///
    /// The method:
    /// 1. Expands symmetric data to full 360°
    /// 2. Resamples all intensities at shifted C-angles
    /// 3. Sets symmetry to None (full data)
    /// 4. For exact 90° multiples, rotates height_c0/c90/c180/c270 and swaps length/width
    ///
    /// # Arguments
    /// * `degrees` - Rotation angle in degrees (positive = counter-clockwise when viewed from above)
    pub fn rotate_c_planes(&mut self, degrees: f64) {
        // Normalize to 0-360
        let rotation = degrees.rem_euclid(360.0);
        if rotation.abs() < 0.001 || (rotation - 360.0).abs() < 0.001 {
            return;
        }

        // Expand to full 360° data
        let full_intensities = crate::symmetry::SymmetryHandler::expand_to_full(self);
        let full_c_angles = crate::symmetry::SymmetryHandler::expand_c_angles(self);

        if full_intensities.is_empty() || full_c_angles.is_empty() {
            return;
        }

        // Build a temporary Eulumdat with full (Isym=0) data for sampling
        let temp = Eulumdat {
            symmetry: Symmetry::None,
            c_angles: full_c_angles.clone(),
            g_angles: self.g_angles.clone(),
            intensities: full_intensities,
            num_c_planes: full_c_angles.len(),
            ..self.clone()
        };

        // Resample: for each output C-angle, sample at (c - rotation) from temp
        let num_g = self.g_angles.len();
        let mut new_intensities = Vec::with_capacity(full_c_angles.len());
        for &c in &full_c_angles {
            let source_c = (c - rotation).rem_euclid(360.0);
            let mut row = Vec::with_capacity(num_g);
            for &g in &self.g_angles {
                row.push(temp.sample(source_c, g));
            }
            new_intensities.push(row);
        }

        // Update self with full data
        self.symmetry = Symmetry::None;
        self.c_angles = full_c_angles;
        self.intensities = new_intensities;
        self.num_c_planes = self.c_angles.len();
        self.c_plane_distance = if self.num_c_planes > 1 {
            self.c_angles[1] - self.c_angles[0]
        } else {
            0.0
        };

        // For exact 90° multiples, rotate height values and swap dimensions
        let steps = (rotation / 90.0).round() as i32;
        if (rotation - steps as f64 * 90.0).abs() < 0.001 {
            let steps_mod = steps.rem_euclid(4);
            let [h0, h90, h180, h270] = [
                self.height_c0,
                self.height_c90,
                self.height_c180,
                self.height_c270,
            ];
            match steps_mod {
                1 => {
                    // 90° CCW: C0→C90, C90→C180, C180→C270, C270→C0
                    self.height_c0 = h270;
                    self.height_c90 = h0;
                    self.height_c180 = h90;
                    self.height_c270 = h180;
                    std::mem::swap(&mut self.length, &mut self.width);
                    std::mem::swap(
                        &mut self.luminous_area_length,
                        &mut self.luminous_area_width,
                    );
                }
                2 => {
                    // 180°: swap opposite pairs
                    self.height_c0 = h180;
                    self.height_c90 = h270;
                    self.height_c180 = h0;
                    self.height_c270 = h90;
                }
                3 => {
                    // 270° CCW (= 90° CW)
                    self.height_c0 = h90;
                    self.height_c90 = h180;
                    self.height_c180 = h270;
                    self.height_c270 = h0;
                    std::mem::swap(&mut self.length, &mut self.width);
                    std::mem::swap(
                        &mut self.luminous_area_length,
                        &mut self.luminous_area_width,
                    );
                }
                _ => {} // 0 or 360 — already handled by early return
            }
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

#[cfg(test)]
mod rotation_tests {
    use super::*;

    fn create_asymmetric_ldt() -> Eulumdat {
        // Isym=0, 4 C-planes at 90° intervals, 3 gamma angles
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::None;
        ldt.num_c_planes = 4;
        ldt.c_plane_distance = 90.0;
        ldt.num_g_planes = 3;
        ldt.g_plane_distance = 45.0;
        ldt.c_angles = vec![0.0, 90.0, 180.0, 270.0];
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 80.0, 50.0],   // C0
            vec![200.0, 160.0, 100.0], // C90
            vec![300.0, 240.0, 150.0], // C180
            vec![400.0, 320.0, 200.0], // C270
        ];
        ldt.length = 1000.0;
        ldt.width = 500.0;
        ldt.height = 100.0;
        ldt.height_c0 = 10.0;
        ldt.height_c90 = 20.0;
        ldt.height_c180 = 30.0;
        ldt.height_c270 = 40.0;
        ldt.luminous_area_length = 800.0;
        ldt.luminous_area_width = 400.0;
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        ldt
    }

    #[test]
    fn test_rotate_90_shifts_intensity() {
        let mut ldt = create_asymmetric_ldt();
        // Before rotation: C0 nadir=100, C90 nadir=200
        assert!((ldt.sample(0.0, 0.0) - 100.0).abs() < 0.01);
        assert!((ldt.sample(90.0, 0.0) - 200.0).abs() < 0.01);

        ldt.rotate_c_planes(90.0);

        // After 90° rotation: what was at C270 should now be at C0
        // (rotating CCW by 90° means each new C-angle samples from C-90°)
        assert!((ldt.sample(0.0, 0.0) - 400.0).abs() < 0.01);
        assert!((ldt.sample(90.0, 0.0) - 100.0).abs() < 0.01);
        assert!((ldt.sample(180.0, 0.0) - 200.0).abs() < 0.01);
        assert!((ldt.sample(270.0, 0.0) - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_rotate_180_double_roundtrip() {
        let original = create_asymmetric_ldt();
        let mut ldt = original.clone();

        ldt.rotate_c_planes(180.0);
        ldt.rotate_c_planes(180.0);

        // After 360° total rotation, intensities should match original
        for &g in &[0.0, 45.0, 90.0] {
            for &c in &[0.0, 90.0, 180.0, 270.0] {
                let orig_val = original.sample(c, g);
                let rotated_val = ldt.sample(c, g);
                assert!(
                    (orig_val - rotated_val).abs() < 0.5,
                    "Mismatch at C={} G={}: original={}, rotated={}",
                    c,
                    g,
                    orig_val,
                    rotated_val
                );
            }
        }
    }

    #[test]
    fn test_rotate_expands_symmetric_data() {
        // Start with Isym=4 data
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::BothPlanes;
        ldt.num_c_planes = 36;
        ldt.c_plane_distance = 10.0;
        ldt.num_g_planes = 3;
        ldt.g_plane_distance = 45.0;
        ldt.c_angles = vec![0.0, 45.0, 90.0]; // Mc=36/4+1 simplified to 3 for test
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 80.0, 50.0], // C0
            vec![95.0, 75.0, 45.0],  // C45
            vec![90.0, 80.0, 50.0],  // C90
        ];
        ldt.length = 600.0;
        ldt.height = 80.0;
        ldt.lamp_sets.push(LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });

        ldt.rotate_c_planes(45.0);

        // After rotation, symmetry should be None
        assert_eq!(ldt.symmetry, Symmetry::None);
        // Should have expanded C-plane data
        assert!(ldt.c_angles.len() > 3);
    }

    #[test]
    fn test_rotate_height_values() {
        let mut ldt = create_asymmetric_ldt();
        // Before: h_c0=10, h_c90=20, h_c180=30, h_c270=40
        // length=1000, width=500

        ldt.rotate_c_planes(90.0);

        // After 90° CCW: C0←C270, C90←C0, C180←C90, C270←C180
        assert!((ldt.height_c0 - 40.0).abs() < 0.01);
        assert!((ldt.height_c90 - 10.0).abs() < 0.01);
        assert!((ldt.height_c180 - 20.0).abs() < 0.01);
        assert!((ldt.height_c270 - 30.0).abs() < 0.01);

        // Length and width should be swapped
        assert!((ldt.length - 500.0).abs() < 0.01);
        assert!((ldt.width - 1000.0).abs() < 0.01);
        assert!((ldt.luminous_area_length - 400.0).abs() < 0.01);
        assert!((ldt.luminous_area_width - 800.0).abs() < 0.01);
    }

    #[test]
    fn test_rotate_zero_is_noop() {
        let original = create_asymmetric_ldt();
        let mut ldt = original.clone();

        ldt.rotate_c_planes(0.0);

        // Should be unchanged
        assert_eq!(ldt.symmetry, original.symmetry);
        assert_eq!(ldt.c_angles, original.c_angles);
        assert_eq!(ldt.intensities, original.intensities);
    }
}
