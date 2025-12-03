//! IES file export.
//!
//! Converts Eulumdat data to IESNA LM-63 format.

use crate::eulumdat::{Eulumdat, Symmetry};
use crate::symmetry::SymmetryHandler;

/// IES file format exporter.
pub struct IesExporter;

impl IesExporter {
    /// Export Eulumdat data to IES (IESNA LM-63-2002) format.
    pub fn export(ldt: &Eulumdat) -> String {
        let mut output = String::new();

        // Header
        output.push_str("IESNA:LM-63-2002\n");

        // Keyword section
        Self::write_keyword(&mut output, "TEST", &ldt.measurement_report_number);
        Self::write_keyword(&mut output, "MANUFAC", "");
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

        Self::write_keyword(&mut output, "MORE", "");

        // TILT=NONE (most common)
        output.push_str("TILT=NONE\n");

        // Line 1: Number of lamps, lumens per lamp, multiplier, number of vertical angles,
        //         number of horizontal angles, photometric type, units type, width, length, height
        let num_lamps = ldt.lamp_sets.iter().map(|ls| ls.num_lamps).sum::<i32>();
        let total_flux = ldt.total_luminous_flux();
        let lumens_per_lamp = if num_lamps > 0 {
            total_flux / num_lamps as f64
        } else {
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

        output.push_str(&format!(
            "{} {:.1} {:.6} {} {} {} {} {:.4} {:.4} {:.4}\n",
            num_lamps.max(1),
            lumens_per_lamp,
            ldt.conversion_factor,
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

        // Line 3: Vertical angles
        output.push_str(&Self::format_angle_line(&v_angles));
        output.push('\n');

        // Line 4: Horizontal angles
        output.push_str(&Self::format_angle_line(&h_angles));
        output.push('\n');

        // Candela values for each horizontal angle
        for row in &intensities {
            output.push_str(&Self::format_candela_line(row));
            output.push('\n');
        }

        output
    }

    /// Write a keyword line.
    fn write_keyword(output: &mut String, keyword: &str, value: &str) {
        output.push_str(&format!("[{}] {}\n", keyword, value));
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
                (vec![0.0], vec![ldt.intensities.get(0).cloned().unwrap_or_default()])
            }
            Symmetry::PlaneC0C180 => {
                // 0° to 180°
                let expanded = SymmetryHandler::expand_to_full(ldt);
                let h = SymmetryHandler::expand_c_angles(ldt);
                let h_filtered: Vec<f64> = h.iter().filter(|&&a| a <= 180.0).copied().collect();
                let i_filtered: Vec<Vec<f64>> = expanded
                    .into_iter()
                    .take(h_filtered.len())
                    .collect();
                (h_filtered, i_filtered)
            }
            Symmetry::PlaneC90C270 => {
                // 90° to 270° (or equivalent)
                let expanded = SymmetryHandler::expand_to_full(ldt);
                let h = SymmetryHandler::expand_c_angles(ldt);
                (h, expanded)
            }
            Symmetry::BothPlanes => {
                // 0° to 90°
                let h: Vec<f64> = ldt.c_angles.iter().filter(|&&a| a <= 90.0).copied().collect();
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

    /// Format a line of angles with max ~10 values per line.
    fn format_angle_line(angles: &[f64]) -> String {
        angles
            .iter()
            .map(|&a| format!("{:.1}", a))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Format a line of candela values.
    fn format_candela_line(values: &[f64]) -> String {
        values
            .iter()
            .map(|&v| format!("{:.1}", v))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eulumdat::LampSet;

    #[test]
    fn test_ies_export() {
        let mut ldt = Eulumdat::new();
        ldt.identification = "Test".to_string();
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
        assert!(ies.contains("TILT=NONE"));
        assert!(ies.contains("1000.0")); // Intensity value
    }
}
