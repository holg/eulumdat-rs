//! Telling a sensor LDT from a luminaire LDT by its content.
//!
//! EULUMDAT has no "sensor" flag, but a sensor LDT (detection volume in
//! metres, parameter block in lines 33–42, see [`super::params`]) has a
//! fingerprint no luminaire file shows:
//!
//! 1. every table value is a range in metres: 0 < max ≤ [`MAX_PLAUSIBLE_RANGE_M`]
//!    (a luminaire's cd/klm values are far larger);
//! 2. field 8 of the "direct ratio" block is a mounting code 1, 2 or 3
//!    (wall, ceiling, both); a luminaire's direct ratio there is a fraction
//!    0 … 1;
//! 3. the ten values do not rise with the room index, as a luminaire's
//!    direct ratios always do (sensor parameters look like 0.5, 1, 0.35, 0, …).
//!
//! Measured 2026-11-05: all 1 187 sensor LDTs of the Relux Net corpus
//! (8 manufacturers) match, none of 34 916 luminaire LDTs does (6 480 GLDF
//! files from 81 manufacturers, plus 100 luminaire LDTs of luminaires with a
//! built-in sensor that the corpus files under sensors). Field 8 = 2 or 3
//! alone matches 93 % of the sensors and no luminaire.
//!
//! Every known sensor LDT follows this one (Relux) layout; a sensor LDT
//! written in some other layout is not recognised and stays a luminaire.
//! An explicit statement from the caller (GLDF `contentType="sensor/…"`,
//! a "sensor" upload) still decides first.
//!
//! [`MAX_PLAUSIBLE_RANGE_M`]: super::MAX_PLAUSIBLE_RANGE_M

use super::MAX_PLAUSIBLE_RANGE_M;
use crate::Eulumdat;

/// The parameter block (lines 33–42) holds sensor parameters: field 8 is a
/// mounting code and the values do not rise like a luminaire's direct ratios.
pub fn has_sensor_parameters(ldt: &Eulumdat) -> bool {
    let d = &ldt.direct_ratios;
    let mounting_code = [1.0, 2.0, 3.0].contains(&d[7]);
    let rising = d.windows(2).all(|w| w[1] >= w[0] - 1e-6);
    mounting_code && !rising
}

/// Largest table value (for a sensor LDT: the longest range in metres).
fn max_value(ldt: &Eulumdat) -> f64 {
    ldt.intensities
        .iter()
        .flatten()
        .copied()
        .fold(0.0f64, f64::max)
}

/// The file looks like a sensor LDT (see the module docs for the rule and
/// how it was measured).
pub fn is_sensor_ldt(ldt: &Eulumdat) -> bool {
    let max = max_value(ldt);
    max > 0.0 && max <= MAX_PLAUSIBLE_RANGE_M && has_sensor_parameters(ldt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Eulumdat {
        Eulumdat::parse(s).unwrap()
    }

    #[test]
    fn demo_sensors_are_recognised() {
        for s in [
            include_str!("../../tests/fixtures/sensors/demo_ceiling_360.ldt"),
            include_str!("../../tests/fixtures/sensors/demo_corridor.ldt"),
        ] {
            assert!(is_sensor_ldt(&parse(s)));
        }
    }

    /// The GLDF demo's "sensor.ldt" is a luminaire file copied into the
    /// sensor folder: it must not pass.
    #[test]
    fn luminaire_copied_as_sensor_is_not_a_sensor() {
        let l = parse(include_str!("../../tests/fixtures/sensor.ldt"));
        assert!(!is_sensor_ldt(&l));
    }

    /// Every luminaire template of the web editor stays a luminaire.
    #[test]
    fn luminaire_templates_are_not_sensors() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../eulumdat-wasm/templates");
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ldt")) {
                continue;
            }
            let b = std::fs::read(&p).unwrap();
            let text: String = match std::str::from_utf8(&b) {
                Ok(s) => s.to_string(),
                Err(_) => b.iter().map(|&c| c as char).collect(),
            };
            if let Ok(l) = Eulumdat::parse(&text) {
                n += 1;
                assert!(!is_sensor_ldt(&l), "{}", p.display());
            }
        }
        assert!(n >= 10, "only {n} templates read");
    }

    #[test]
    fn a_rising_block_or_large_values_are_a_luminaire() {
        let mut l = parse(include_str!(
            "../../tests/fixtures/sensors/demo_ceiling_360.ldt"
        ));
        assert!(is_sensor_ldt(&l));
        // Values in cd/klm.
        let mut big = l.clone();
        big.intensities[0][0] = 450.0;
        assert!(!is_sensor_ldt(&big));
        // Direct ratios that rise like a luminaire's, even ending in 1.
        l.direct_ratios = [0.4, 0.5, 0.6, 0.7, 0.75, 0.8, 0.9, 1.0, 1.0, 1.0];
        assert!(!is_sensor_ldt(&l));
    }

    /// `SENSOR_LDT_CORPUS=<dir> cargo test -p eulumdat sensor::detect`:
    /// every sensor LDT with ranges ≤ 60 m is recognised, every file with
    /// larger values (luminaire LDTs filed as sensors) is not.
    #[test]
    fn corpus() {
        let Ok(dir) = std::env::var("SENSOR_LDT_CORPUS") else {
            return;
        };
        let (mut sensors, mut found, mut lum, mut lum_flagged) = (0, 0, 0, 0);
        let mut stack = vec![std::path::PathBuf::from(dir)];
        while let Some(p) = stack.pop() {
            if p.is_dir() {
                for e in std::fs::read_dir(&p).unwrap().flatten() {
                    stack.push(e.path());
                }
                continue;
            }
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ldt")) {
                continue;
            }
            let b = std::fs::read(&p).unwrap();
            let text: String = match std::str::from_utf8(&b) {
                Ok(s) => s.to_string(),
                Err(_) => b.iter().map(|&c| c as char).collect(),
            };
            let Ok(l) = Eulumdat::parse(&text) else {
                continue;
            };
            if max_value(&l) <= MAX_PLAUSIBLE_RANGE_M {
                sensors += 1;
                found += is_sensor_ldt(&l) as usize;
            } else {
                lum += 1;
                lum_flagged += is_sensor_ldt(&l) as usize;
            }
        }
        eprintln!(
            "sensor LDTs {found}/{sensors} recognised, luminaire LDTs flagged {lum_flagged}/{lum}"
        );
        assert_eq!(found, sensors);
        assert_eq!(lum_flagged, 0);
    }
}
