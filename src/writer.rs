//! LDT file writer.
//!
//! Writes Eulumdat format files according to the official specification.

use crate::eulumdat::{Eulumdat, Symmetry, TypeIndicator};

/// Writer for Eulumdat (LDT) files.
pub struct Writer;

impl Writer {
    /// Write an Eulumdat structure to LDT format string.
    pub fn write(ldt: &Eulumdat) -> String {
        let mut output = String::new();

        // Line 1: Identification
        output.push_str(&ldt.identification);
        output.push('\n');

        // Line 2: Type indicator (Ityp)
        // Normalize type indicator based on symmetry (as in original C++ code)
        let ityp = if ldt.type_indicator != TypeIndicator::Linear {
            if ldt.symmetry == Symmetry::VerticalAxis {
                1
            } else {
                3
            }
        } else {
            2
        };
        output.push_str(&ityp.to_string());
        output.push('\n');

        // Line 3: Symmetry indicator (Isym)
        output.push_str(&ldt.symmetry.as_int().to_string());
        output.push('\n');

        // Line 4: Number of C-planes (Nc)
        output.push_str(&ldt.num_c_planes.to_string());
        output.push('\n');

        // Line 5: Distance between C-planes (Dc)
        output.push_str(&Self::format_float(ldt.c_plane_distance));
        output.push('\n');

        // Line 6: Number of G-planes (Ng)
        output.push_str(&ldt.num_g_planes.to_string());
        output.push('\n');

        // Line 7: Distance between G-planes (Dg)
        output.push_str(&Self::format_float(ldt.g_plane_distance));
        output.push('\n');

        // Line 8: Measurement report number
        output.push_str(&ldt.measurement_report_number);
        output.push('\n');

        // Line 9: Luminaire name
        output.push_str(&ldt.luminaire_name);
        output.push('\n');

        // Line 10: Luminaire number
        output.push_str(&ldt.luminaire_number);
        output.push('\n');

        // Line 11: File name
        output.push_str(&ldt.file_name);
        output.push('\n');

        // Line 12: Date/user
        output.push_str(&ldt.date_user);
        output.push('\n');

        // Line 13: Length/diameter of luminaire (mm)
        output.push_str(&Self::format_float(ldt.length));
        output.push('\n');

        // Line 14: Width of luminaire (mm)
        output.push_str(&Self::format_float(ldt.width));
        output.push('\n');

        // Line 15: Height of luminaire (mm)
        output.push_str(&Self::format_float(ldt.height));
        output.push('\n');

        // Line 16: Length/diameter of luminous area (mm)
        output.push_str(&Self::format_float(ldt.luminous_area_length));
        output.push('\n');

        // Line 17: Width of luminous area (mm)
        output.push_str(&Self::format_float(ldt.luminous_area_width));
        output.push('\n');

        // Line 18: Height at C0 plane (mm)
        output.push_str(&Self::format_float(ldt.height_c0));
        output.push('\n');

        // Line 19: Height at C90 plane (mm)
        output.push_str(&Self::format_float(ldt.height_c90));
        output.push('\n');

        // Line 20: Height at C180 plane (mm)
        output.push_str(&Self::format_float(ldt.height_c180));
        output.push('\n');

        // Line 21: Height at C270 plane (mm)
        output.push_str(&Self::format_float(ldt.height_c270));
        output.push('\n');

        // Line 22: Downward flux fraction (DFF) %
        output.push_str(&Self::format_float(ldt.downward_flux_fraction));
        output.push('\n');

        // Line 23: Light output ratio of luminaire (LORL) %
        output.push_str(&Self::format_float(ldt.light_output_ratio));
        output.push('\n');

        // Line 24: Conversion factor for luminous intensities (CFLI)
        output.push_str(&Self::format_float(ldt.conversion_factor));
        output.push('\n');

        // Line 25: Tilt angle during measurement
        output.push_str(&Self::format_float(ldt.tilt_angle));
        output.push('\n');

        // Line 26: Number of standard lamp sets
        output.push_str(&ldt.lamp_sets.len().to_string());
        output.push('\n');

        // Lines 26a-26f: Lamp set data
        for lamp_set in &ldt.lamp_sets {
            // 26a: Number of lamps
            output.push_str(&lamp_set.num_lamps.to_string());
            output.push('\n');

            // 26b: Type of lamps
            output.push_str(&lamp_set.lamp_type);
            output.push('\n');

            // 26c: Total luminous flux
            output.push_str(&Self::format_float(lamp_set.total_luminous_flux));
            output.push('\n');

            // 26d: Color appearance
            output.push_str(&lamp_set.color_appearance);
            output.push('\n');

            // 26e: Color rendering group
            output.push_str(&lamp_set.color_rendering_group);
            output.push('\n');

            // 26f: Wattage including ballast
            output.push_str(&Self::format_float(lamp_set.wattage_with_ballast));
            output.push('\n');
        }

        // Lines 27a-27j: Direct ratios
        for ratio in &ldt.direct_ratios {
            output.push_str(&Self::format_float(*ratio));
            output.push('\n');
        }

        // Lines 28: C-plane angles
        for angle in &ldt.c_angles {
            output.push_str(&Self::format_float(*angle));
            output.push('\n');
        }

        // Lines 29: G-plane angles
        for angle in &ldt.g_angles {
            output.push_str(&Self::format_float(*angle));
            output.push('\n');
        }

        // Lines 30+: Luminous intensities
        for row in &ldt.intensities {
            for intensity in row {
                output.push_str(&Self::format_float(*intensity));
                output.push('\n');
            }
        }

        output
    }

    /// Format a float value for output.
    ///
    /// Uses a reasonable precision and removes trailing zeros.
    fn format_float(value: f64) -> String {
        if value == value.trunc() {
            // Integer value
            format!("{}", value as i64)
        } else {
            // Format with up to 6 decimal places, remove trailing zeros
            let s = format!("{:.6}", value);
            let s = s.trim_end_matches('0');
            let s = s.trim_end_matches('.');
            s.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_float() {
        assert_eq!(Writer::format_float(100.0), "100");
        assert_eq!(Writer::format_float(100.5), "100.5");
        assert_eq!(Writer::format_float(100.123456), "100.123456");
        assert_eq!(Writer::format_float(100.100000), "100.1");
        assert_eq!(Writer::format_float(0.0), "0");
    }

    #[test]
    fn test_roundtrip() {
        let mut ldt = Eulumdat::new();
        ldt.identification = "Test Luminaire".to_string();
        ldt.type_indicator = TypeIndicator::PointSourceSymmetric;
        ldt.symmetry = Symmetry::VerticalAxis;
        ldt.num_c_planes = 1;
        ldt.c_plane_distance = 0.0;
        ldt.num_g_planes = 3;
        ldt.g_plane_distance = 45.0;
        ldt.luminaire_name = "Test".to_string();
        ldt.lamp_sets.push(crate::eulumdat::LampSet {
            num_lamps: 1,
            lamp_type: "LED".to_string(),
            total_luminous_flux: 1000.0,
            color_appearance: "3000K".to_string(),
            color_rendering_group: "80".to_string(),
            wattage_with_ballast: 10.0,
        });
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 45.0, 90.0];
        ldt.intensities = vec![vec![100.0, 80.0, 50.0]];

        let output = Writer::write(&ldt);
        assert!(output.contains("Test Luminaire"));
        assert!(output.contains("LED"));
        assert!(output.contains("1000"));
    }
}
