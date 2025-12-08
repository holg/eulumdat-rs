//! IES (IESNA LM-63) file format support.
//!
//! This module provides parsing and export of IES photometric files according to
//! IESNA LM-63-2002 and compatible older formats (LM-63-1995, LM-63-1991).
//!
//! ## IES File Format Overview
//!
//! The IES format is the North American standard for photometric data exchange,
//! developed by the Illuminating Engineering Society of North America (IESNA).
//!
//! ### File Structure
//!
//! 1. **Version header**: `IESNA:LM-63-2002` or `IESNA91` (older)
//! 2. **Keywords**: `[KEYWORD] value` format (TEST, MANUFAC, LUMINAIRE, etc.)
//! 3. **TILT specification**: `TILT=NONE`, `TILT=INCLUDE`, or `TILT=<filename>`
//! 4. **Photometric data**:
//!    - Line 1: num_lamps, lumens_per_lamp, multiplier, n_vert, n_horiz, photo_type, units, width, length, height
//!    - Line 2: ballast_factor, lamp_ballast_factor, input_watts
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

use crate::error::{anyhow, Result};
use crate::eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};
use crate::symmetry::SymmetryHandler;

/// IES file format parser.
///
/// Parses IESNA LM-63 format files (versions 1991, 1995, 2002).
pub struct IesParser;

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
#[derive(Debug, Default)]
struct IesData {
    /// Version string (e.g., "LM-63-2002")
    version: String,
    /// Keyword metadata
    keywords: HashMap<String, String>,
    /// Number of lamps
    num_lamps: i32,
    /// Lumens per lamp
    lumens_per_lamp: f64,
    /// Candela multiplier
    multiplier: f64,
    /// Number of vertical angles
    n_vertical: usize,
    /// Number of horizontal angles
    n_horizontal: usize,
    /// Photometric type (1=C, 2=B, 3=A)
    photometric_type: PhotometricType,
    /// Unit type (1=feet, 2=meters)
    unit_type: UnitType,
    /// Luminous opening width
    width: f64,
    /// Luminous opening length
    length: f64,
    /// Luminous opening height
    height: f64,
    /// Ballast factor
    ballast_factor: f64,
    /// Ballast-lamp photometric factor
    lamp_ballast_factor: f64,
    /// Input watts
    input_watts: f64,
    /// Vertical angles (gamma)
    vertical_angles: Vec<f64>,
    /// Horizontal angles (C-planes)
    horizontal_angles: Vec<f64>,
    /// Candela values `[horizontal_index][vertical_index]`
    candela_values: Vec<Vec<f64>>,
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
        if first_line.starts_with("IESNA") {
            data.version = first_line.to_string();
            line_idx += 1;
        } else {
            // Older format without explicit version
            data.version = "IESNA91".to_string();
        }

        // Parse keywords until TILT
        while line_idx < lines.len() {
            let line = lines[line_idx].trim();

            if line.starts_with("TILT=") || line.starts_with("TILT ") {
                break;
            }

            // Parse [KEYWORD] value format
            if line.starts_with('[') {
                if let Some(end_bracket) = line.find(']') {
                    let keyword = line[1..end_bracket].to_string();
                    let value = line[end_bracket + 1..].trim().to_string();
                    data.keywords.insert(keyword, value);
                }
            }

            line_idx += 1;
        }

        // Handle TILT
        if line_idx < lines.len() {
            let tilt_line = lines[line_idx].trim();
            if tilt_line.contains("INCLUDE") {
                // Skip TILT data (lamp-to-luminaire geometry, angles, factors)
                line_idx += 1;
                // Skip lamp-to-luminaire geometry line
                if line_idx < lines.len() {
                    line_idx += 1;
                }
                // Parse number of angle-factor pairs
                if line_idx < lines.len() {
                    if let Ok(n_pairs) = lines[line_idx].trim().parse::<usize>() {
                        line_idx += 1;
                        // Skip the angle and factor lines
                        let pairs_per_line = 10; // typically
                        let angle_lines = n_pairs.div_ceil(pairs_per_line);
                        line_idx += angle_lines * 2; // angles and factors
                    }
                }
            } else {
                // TILT=NONE or TILT=<filename>
                line_idx += 1;
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

        // Line 2: ballast_factor, lamp_ballast_factor, input_watts
        data.ballast_factor = numeric_values[idx];
        idx += 1;
        data.lamp_ballast_factor = numeric_values[idx];
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

    /// Convert parsed IES data to Eulumdat structure.
    fn convert_to_eulumdat(ies: IesData) -> Result<Eulumdat> {
        let mut ldt = Eulumdat::new();

        // Convert keywords to Eulumdat fields
        ldt.identification = ies.keywords.get("MANUFAC").cloned().unwrap_or_default();
        ldt.luminaire_name = ies.keywords.get("LUMINAIRE").cloned().unwrap_or_default();
        ldt.luminaire_number = ies.keywords.get("LUMCAT").cloned().unwrap_or_default();
        ldt.measurement_report_number = ies.keywords.get("TEST").cloned().unwrap_or_default();
        ldt.file_name = ies.keywords.get("TESTLAB").cloned().unwrap_or_default();

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
            lamp_type: ies
                .keywords
                .get("LAMP")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
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
/// Exports Eulumdat data to IESNA LM-63-2002 format.
pub struct IesExporter;

impl IesExporter {
    /// Export Eulumdat data to IES (IESNA LM-63-2002) format.
    pub fn export(ldt: &Eulumdat) -> String {
        let mut output = String::new();

        // Header
        output.push_str("IESNA:LM-63-2002\n");

        // Keyword section
        Self::write_keyword(&mut output, "TEST", &ldt.measurement_report_number);
        if !ldt.identification.is_empty() {
            Self::write_keyword(&mut output, "MANUFAC", &ldt.identification);
        }
        Self::write_keyword(&mut output, "LUMCAT", &ldt.luminaire_number);
        Self::write_keyword(&mut output, "LUMINAIRE", &ldt.luminaire_name);

        if !ldt.lamp_sets.is_empty() {
            Self::write_keyword(&mut output, "LAMP", &ldt.lamp_sets[0].lamp_type);
            Self::write_keyword(
                &mut output,
                "LAMPCAT",
                &format!("{} lm", ldt.lamp_sets[0].total_luminous_flux),
            );
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

        // Line 2: Ballast factor, ballast-lamp photometric factor, input watts
        let total_watts = ldt.total_wattage();
        output.push_str(&format!("1.0 1.0 {:.1}\n", total_watts));

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

        assert!(ies.contains("IESNA:LM-63-2002"));
        assert!(ies.contains("[LUMINAIRE] Test Luminaire"));
        assert!(ies.contains("[MANUFAC] Test Manufacturer"));
        assert!(ies.contains("TILT=NONE"));
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
}
