//! LDT file parser.
//!
//! Parses Eulumdat format files according to the official specification.

use crate::error::{parse_error, Result};
use crate::eulumdat::{Eulumdat, LampSet, Symmetry, TypeIndicator};

/// Parser for Eulumdat (LDT) files.
pub struct Parser;

impl Parser {
    /// Parse an Eulumdat file from string content.
    pub fn parse(content: &str) -> Result<Eulumdat> {
        let mut ldt = Eulumdat::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut idx = 0;

        // Helper to get next line
        let next_line = |i: &mut usize| -> Result<&str> {
            if *i >= lines.len() {
                return Err(parse_error(*i, "unexpected end of file"));
            }
            let line = lines[*i];
            *i += 1;
            Ok(line)
        };

        // Line 1: Identification
        ldt.identification = next_line(&mut idx)?.to_string();

        // Line 2: Type indicator (Ityp)
        let ityp_line = idx;
        let ityp = Self::parse_int(next_line(&mut idx)?, ityp_line)?;
        ldt.type_indicator = TypeIndicator::from_int(ityp)?;

        // Line 3: Symmetry indicator (Isym)
        let isym_line = idx;
        let isym = Self::parse_int(next_line(&mut idx)?, isym_line)?;
        ldt.symmetry = Symmetry::from_int(isym)?;

        // Line 4: Number of C-planes (Mc)
        let nc_line = idx;
        ldt.num_c_planes = Self::parse_int(next_line(&mut idx)?, nc_line)? as usize;

        // Line 5: Distance between C-planes (Dc)
        let dc_line = idx;
        ldt.c_plane_distance = Self::parse_float(next_line(&mut idx)?, dc_line)?;

        // Line 6: Number of G-planes (Ng)
        let ng_line = idx;
        ldt.num_g_planes = Self::parse_int(next_line(&mut idx)?, ng_line)? as usize;

        // Line 7: Distance between G-planes (Dg)
        let dg_line = idx;
        ldt.g_plane_distance = Self::parse_float(next_line(&mut idx)?, dg_line)?;

        // Line 8: Measurement report number
        ldt.measurement_report_number = next_line(&mut idx)?.to_string();

        // Line 9: Luminaire name
        ldt.luminaire_name = next_line(&mut idx)?.to_string();

        // Line 10: Luminaire number
        ldt.luminaire_number = next_line(&mut idx)?.to_string();

        // Line 11: File name
        ldt.file_name = next_line(&mut idx)?.to_string();

        // Line 12: Date/user
        ldt.date_user = next_line(&mut idx)?.to_string();

        // Line 13: Length/diameter of luminaire (mm)
        let l_line = idx;
        ldt.length = Self::parse_float(next_line(&mut idx)?, l_line)?;

        // Line 14: Width of luminaire (mm), 0 for circular
        let b_line = idx;
        ldt.width = Self::parse_float(next_line(&mut idx)?, b_line)?;

        // Line 15: Height of luminaire (mm)
        let h_line = idx;
        ldt.height = Self::parse_float(next_line(&mut idx)?, h_line)?;

        // Line 16: Length/diameter of luminous area (mm)
        let la_line = idx;
        ldt.luminous_area_length = Self::parse_float(next_line(&mut idx)?, la_line)?;

        // Line 17: Width of luminous area (mm), 0 for circular
        let b1_line = idx;
        ldt.luminous_area_width = Self::parse_float(next_line(&mut idx)?, b1_line)?;

        // Line 18: Height of luminous area at C0 plane (mm)
        let hc0_line = idx;
        ldt.height_c0 = Self::parse_float(next_line(&mut idx)?, hc0_line)?;

        // Line 19: Height of luminous area at C90 plane (mm)
        let hc90_line = idx;
        ldt.height_c90 = Self::parse_float(next_line(&mut idx)?, hc90_line)?;

        // Line 20: Height of luminous area at C180 plane (mm)
        let hc180_line = idx;
        ldt.height_c180 = Self::parse_float(next_line(&mut idx)?, hc180_line)?;

        // Line 21: Height of luminous area at C270 plane (mm)
        let hc270_line = idx;
        ldt.height_c270 = Self::parse_float(next_line(&mut idx)?, hc270_line)?;

        // Line 22: Downward flux fraction (DFF) %
        let dff_line = idx;
        ldt.downward_flux_fraction = Self::parse_float(next_line(&mut idx)?, dff_line)?;

        // Line 23: Light output ratio of luminaire (LORL) %
        let lorl_line = idx;
        ldt.light_output_ratio = Self::parse_float(next_line(&mut idx)?, lorl_line)?;

        // Line 24: Conversion factor for luminous intensities (CFLI)
        let cfli_line = idx;
        ldt.conversion_factor = Self::parse_float(next_line(&mut idx)?, cfli_line)?;

        // Line 25: Tilt angle during measurement
        let tilt_line = idx;
        ldt.tilt_angle = Self::parse_float(next_line(&mut idx)?, tilt_line)?;

        // Line 26: Number of standard lamp sets (n, 1-20)
        let n_line = idx;
        let num_lamp_sets = Self::parse_int(next_line(&mut idx)?, n_line)? as usize;

        // Lines 26a-26f: For each lamp set (repeated n times)
        for _ in 0..num_lamp_sets {
            let mut lamp_set = LampSet::default();

            // 26a: Number of lamps
            let nl_line = idx;
            lamp_set.num_lamps = Self::parse_int(next_line(&mut idx)?, nl_line)?;

            // 26b: Type of lamps
            lamp_set.lamp_type = next_line(&mut idx)?.to_string();

            // 26c: Total luminous flux
            let tlf_line = idx;
            lamp_set.total_luminous_flux = Self::parse_float(next_line(&mut idx)?, tlf_line)?;

            // 26d: Color appearance
            lamp_set.color_appearance = next_line(&mut idx)?.to_string();

            // 26e: Color rendering group
            lamp_set.color_rendering_group = next_line(&mut idx)?.to_string();

            // 26f: Wattage including ballast
            let wb_line = idx;
            lamp_set.wattage_with_ballast = Self::parse_float(next_line(&mut idx)?, wb_line)?;

            // Store all lamp sets (some files have more than 20)
            ldt.lamp_sets.push(lamp_set);
        }

        // Lines 27a-27j: Direct ratios for 10 room indices
        for i in 0..10 {
            let dr_line = idx;
            ldt.direct_ratios[i] = Self::parse_float(next_line(&mut idx)?, dr_line)?;
        }

        // Lines 28: C-plane angles (Nc values) - read ALL C-plane angles first
        for _ in 0..ldt.num_c_planes {
            let c_line = idx;
            let angle = Self::parse_float(next_line(&mut idx)?, c_line)?;
            ldt.c_angles.push(angle);
        }

        // Lines 29: G-plane angles (Ng values)
        for _ in 0..ldt.num_g_planes {
            let g_line = idx;
            let angle = Self::parse_float(next_line(&mut idx)?, g_line)?;
            ldt.g_angles.push(angle);
        }

        // Calculate actual number of C-planes based on symmetry (Mc)
        // This determines how many intensity data planes are stored
        let mc = ldt.symmetry.calc_mc(ldt.num_c_planes);

        // Lines 30+: Luminous intensities (Mc x Ng values)
        // Only Mc planes of intensity data are stored in the file
        for c in 0..mc {
            let mut row = Vec::with_capacity(ldt.num_g_planes);
            for g in 0..ldt.num_g_planes {
                let i_line = idx;
                let intensity = Self::parse_float(next_line(&mut idx)?, i_line).map_err(|_| {
                    parse_error(
                        i_line,
                        format!("error reading intensity at C[{}] G[{}]", c, g),
                    )
                })?;
                row.push(intensity);
            }
            ldt.intensities.push(row);
        }

        Ok(ldt)
    }

    /// Parse an integer from string.
    fn parse_int(s: &str, line: usize) -> Result<i32> {
        Self::parse_number(s)
            .map(|f| f as i32)
            .map_err(|_| parse_error(line, format!("expected integer, got '{}'", s)))
    }

    /// Parse a float from string.
    fn parse_float(s: &str, line: usize) -> Result<f64> {
        Self::parse_number(s)
            .map_err(|_| parse_error(line, format!("expected number, got '{}'", s)))
    }

    /// Parse a number from string, handling European format (comma as decimal separator).
    fn parse_number(s: &str) -> Result<f64> {
        let cleaned = s.trim().replace('_', "").replace(',', ".");

        cleaned
            .parse::<f64>()
            .map_err(|_| parse_error(0, format!("cannot parse '{}' as number", s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        assert!((Parser::parse_number("123.45").unwrap() - 123.45).abs() < 0.001);
        assert!((Parser::parse_number("123,45").unwrap() - 123.45).abs() < 0.001);
        assert!((Parser::parse_number("  123.45  ").unwrap() - 123.45).abs() < 0.001);
        assert!((Parser::parse_number("1_000").unwrap() - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_minimal() {
        let content = r#"Test Luminaire
1
1
1
0
19
5
Report123
Test Light
LUM-001
test.ldt
2024-01-01
100
50
30
80
40
0
0
0
0
100
85
1
0
1
1
LED Module
1000
3000K
80
10
0.5
0.55
0.6
0.65
0.7
0.75
0.8
0.82
0.85
0.88
0
0
5
10
15
20
25
30
35
40
45
50
55
60
65
70
75
80
85
90
100
200
300
400
350
300
250
200
150
100
75
50
25
15
10
5
2
1
0
"#;

        let ldt = Parser::parse(content).unwrap();
        assert_eq!(ldt.identification, "Test Luminaire");
        assert_eq!(ldt.type_indicator, TypeIndicator::PointSourceSymmetric);
        assert_eq!(ldt.symmetry, Symmetry::VerticalAxis);
        assert_eq!(ldt.num_g_planes, 19);
        assert_eq!(ldt.luminaire_name, "Test Light");
        assert_eq!(ldt.lamp_sets.len(), 1);
        assert_eq!(ldt.lamp_sets[0].num_lamps, 1);
        assert!((ldt.lamp_sets[0].total_luminous_flux - 1000.0).abs() < 0.001);
    }
}
