//! Conversion between ATLA and Eulumdat formats
//!
//! Provides bidirectional conversion between ATLA S001 / TM-33 / UNI 11733
//! and the traditional EULUMDAT (LDT) format.

use crate::types::*;

#[cfg(feature = "eulumdat")]
use eulumdat::{Eulumdat, LampSet, Symmetry as EulumdatSymmetry, TypeIndicator};

#[cfg(feature = "eulumdat")]
impl From<&Eulumdat> for LuminaireOpticalData {
    fn from(ldt: &Eulumdat) -> Self {
        let mut doc = LuminaireOpticalData::new();

        // Header
        doc.header = Header {
            manufacturer: if ldt.identification.is_empty() {
                None
            } else {
                Some(ldt.identification.clone())
            },
            catalog_number: if ldt.luminaire_number.is_empty() {
                None
            } else {
                Some(ldt.luminaire_number.clone())
            },
            description: if ldt.luminaire_name.is_empty() {
                None
            } else {
                Some(ldt.luminaire_name.clone())
            },
            report_number: if ldt.measurement_report_number.is_empty() {
                None
            } else {
                Some(ldt.measurement_report_number.clone())
            },
            test_date: if ldt.date_user.is_empty() {
                None
            } else {
                Some(ldt.date_user.clone())
            },
            ..Default::default()
        };

        // Luminaire dimensions
        if ldt.length > 0.0 || ldt.width > 0.0 || ldt.height > 0.0 {
            let shape = if ldt.width == 0.0 {
                LuminousOpeningShape::Circular
            } else {
                LuminousOpeningShape::Rectangular
            };

            doc.luminaire = Some(Luminaire {
                dimensions: Some(Dimensions {
                    length: ldt.length,
                    width: ldt.width,
                    height: ldt.height,
                }),
                luminous_openings: vec![LuminousOpening {
                    shape,
                    dimensions: OpeningDimensions {
                        length: ldt.luminous_area_length,
                        width: if ldt.luminous_area_width > 0.0 {
                            Some(ldt.luminous_area_width)
                        } else {
                            None
                        },
                    },
                    position: None,
                }],
                mounting: None,
                num_emitters: Some(ldt.lamp_sets.iter().map(|ls| ls.num_lamps as u32).sum()),
            });
        }

        // Create emitter from lamp sets and intensity data
        let emitter = create_emitter_from_ldt(ldt);
        doc.emitters.push(emitter);

        doc
    }
}

#[cfg(feature = "eulumdat")]
fn create_emitter_from_ldt(ldt: &Eulumdat) -> Emitter {
    // Calculate total flux and power from lamp sets
    let total_lumens: f64 = ldt.lamp_sets.iter().map(|ls| ls.total_luminous_flux).sum();
    let total_watts: f64 = ldt.lamp_sets.iter().map(|ls| ls.wattage_with_ballast).sum();

    // Get CCT from first lamp set if available
    let cct = ldt.lamp_sets.first().and_then(|ls| parse_cct(&ls.color_appearance));

    // Get CRI from first lamp set if available
    let color_rendering = ldt.lamp_sets.first().and_then(|ls| {
        parse_cri(&ls.color_rendering_group).map(|ra| ColorRendering {
            ra: Some(ra),
            r9: None,
            rf: None,
            rg: None,
        })
    });

    // Build intensity distribution
    let intensity_distribution = if !ldt.intensities.is_empty() {
        Some(IntensityDistribution {
            photometry_type: PhotometryType::TypeC,
            metric: IntensityMetric::Luminous,
            units: IntensityUnits::CandelaPerKilolumen,
            horizontal_angles: ldt.c_angles.clone(),
            vertical_angles: ldt.g_angles.clone(),
            intensities: ldt.intensities.clone(),
        })
    } else {
        None
    };

    // Combine lamp descriptions
    let description = if ldt.lamp_sets.is_empty() {
        None
    } else {
        let lamp_desc: Vec<String> = ldt
            .lamp_sets
            .iter()
            .filter(|ls| !ls.lamp_type.is_empty())
            .map(|ls| format!("{}x {}", ls.num_lamps, ls.lamp_type))
            .collect();
        if lamp_desc.is_empty() {
            None
        } else {
            Some(lamp_desc.join(", "))
        }
    };

    Emitter {
        id: None,
        description,
        quantity: ldt.lamp_sets.iter().map(|ls| ls.num_lamps as u32).sum::<u32>().max(1),
        rated_lumens: if total_lumens > 0.0 {
            Some(total_lumens)
        } else {
            None
        },
        measured_lumens: None,
        input_watts: if total_watts > 0.0 {
            Some(total_watts)
        } else {
            None
        },
        power_factor: None,
        cct,
        color_rendering,
        sp_ratio: None,
        data_generation: Some(DataGeneration {
            source: DataSource::Measured,
            scaled: false,
            interpolated: false,
            software: None,
            uncertainty: None,
        }),
        intensity_distribution,
        spectral_distribution: None,
    }
}

#[cfg(feature = "eulumdat")]
impl From<&LuminaireOpticalData> for Eulumdat {
    fn from(doc: &LuminaireOpticalData) -> Self {
        let mut ldt = Eulumdat::default();

        // Header -> identification fields
        if let Some(ref mfr) = doc.header.manufacturer {
            ldt.identification = mfr.clone();
        }
        if let Some(ref cat) = doc.header.catalog_number {
            ldt.luminaire_number = cat.clone();
        }
        if let Some(ref desc) = doc.header.description {
            ldt.luminaire_name = desc.clone();
        }
        if let Some(ref report) = doc.header.report_number {
            ldt.measurement_report_number = report.clone();
        }
        if let Some(ref date) = doc.header.test_date {
            ldt.date_user = date.clone();
        }

        // Luminaire dimensions
        if let Some(ref luminaire) = doc.luminaire {
            if let Some(ref dims) = luminaire.dimensions {
                ldt.length = dims.length;
                ldt.width = dims.width;
                ldt.height = dims.height;
            }

            // Luminous opening -> luminous area
            if let Some(opening) = luminaire.luminous_openings.first() {
                ldt.luminous_area_length = opening.dimensions.length;
                ldt.luminous_area_width = opening.dimensions.width.unwrap_or(0.0);
            }
        }

        // Emitters -> lamp sets and intensity data
        if let Some(emitter) = doc.emitters.first() {
            // Create lamp set
            let lamp_set = LampSet {
                num_lamps: emitter.quantity as i32,
                lamp_type: emitter.description.clone().unwrap_or_default(),
                total_luminous_flux: emitter
                    .measured_lumens
                    .or(emitter.rated_lumens)
                    .unwrap_or(0.0),
                color_appearance: emitter
                    .cct
                    .map(|cct| format!("{}K", cct as i32))
                    .unwrap_or_default(),
                color_rendering_group: emitter
                    .color_rendering
                    .as_ref()
                    .and_then(|cr| cr.ra)
                    .map(cri_to_group)
                    .unwrap_or_default(),
                wattage_with_ballast: emitter.input_watts.unwrap_or(0.0),
            };
            ldt.lamp_sets.push(lamp_set);

            // Intensity distribution
            if let Some(ref dist) = emitter.intensity_distribution {
                ldt.c_angles = dist.horizontal_angles.clone();
                ldt.g_angles = dist.vertical_angles.clone();
                ldt.intensities = dist.intensities.clone();

                // Calculate grid parameters
                ldt.num_c_planes = if dist.horizontal_angles.len() > 1 {
                    dist.horizontal_angles.len()
                } else {
                    1
                };
                ldt.num_g_planes = dist.vertical_angles.len();

                if dist.horizontal_angles.len() > 1 {
                    ldt.c_plane_distance =
                        dist.horizontal_angles[1] - dist.horizontal_angles[0];
                }
                if dist.vertical_angles.len() > 1 {
                    ldt.g_plane_distance = dist.vertical_angles[1] - dist.vertical_angles[0];
                }

                // Determine symmetry from data
                ldt.symmetry = determine_symmetry(&dist.horizontal_angles);
            }
        }

        // Calculate light output ratio and downward flux fraction
        if !ldt.intensities.is_empty() && !ldt.g_angles.is_empty() {
            let (dff, lor) = calculate_flux_fractions(&ldt);
            ldt.downward_flux_fraction = dff;
            ldt.light_output_ratio = lor;
        }

        // Set type indicator based on dimensions
        ldt.type_indicator = if ldt.width == 0.0 {
            TypeIndicator::PointSourceSymmetric
        } else if ldt.length > ldt.width * 2.0 {
            TypeIndicator::Linear
        } else {
            TypeIndicator::PointSourceOther
        };

        ldt
    }
}

/// Parse CCT from color appearance string
///
/// Supported formats:
/// - "3000K", "4000", "3000.0" - direct numeric
/// - "tw/6500", "TW-6500" - tunable white with CCT
/// - "ww/3000", "cw/5000" - warm/cool white with CCT
/// - "CT3000", "CCT:3000" - prefixed CCT
/// - "warm white", "daylight", "neutral" - named temperatures
/// - "LED 3000K", "3000K LED" - mixed with other text
fn parse_cct(color_appearance: &str) -> Option<f64> {
    if color_appearance.is_empty() {
        return None;
    }

    let s = color_appearance.trim();

    // Skip "n/a", "none", "unknown" etc.
    let lower = s.to_lowercase();
    if lower == "n/a" || lower == "na" || lower == "none" || lower == "unknown" || lower == "-" {
        return None;
    }

    // Extract all numeric sequences from the string
    let numbers: Vec<f64> = extract_numbers(s);

    // Find a number that looks like a CCT (1000-20000)
    for &num in &numbers {
        if (1000.0..=20000.0).contains(&num) {
            return Some(num);
        }
    }

    // Common color temperature names (only if no valid number found)
    if lower.contains("warm") || lower.contains("ww") || lower.starts_with("ww") {
        return Some(2700.0);
    }
    if lower.contains("neutral") || lower.contains("nw") || lower.starts_with("nw") {
        return Some(4000.0);
    }
    if lower.contains("cool") || lower.contains("cold") || lower.contains("cw") || lower.starts_with("cw") {
        return Some(5000.0);
    }
    if lower.contains("daylight") || lower.contains("tw") || lower.starts_with("tw") {
        return Some(6500.0);
    }

    None
}

/// Extract all numeric values from a string
fn extract_numbers(s: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    let mut has_dot = false;

    for c in s.chars() {
        if c.is_ascii_digit() {
            current.push(c);
        } else if c == '.' && !has_dot && !current.is_empty() {
            current.push(c);
            has_dot = true;
        } else {
            if !current.is_empty() {
                if let Ok(num) = current.trim_end_matches('.').parse::<f64>() {
                    numbers.push(num);
                }
                current.clear();
                has_dot = false;
            }
        }
    }

    // Don't forget the last number
    if !current.is_empty() {
        if let Ok(num) = current.trim_end_matches('.').parse::<f64>() {
            numbers.push(num);
        }
    }

    numbers
}

/// Parse CRI from color rendering group string
///
/// Supported formats:
/// - "1A", "1B", "2A", "2B", "3", "4" - CIE groups
/// - "1B/86", "1A-95" - CIE group with numeric Ra value
/// - "80", "Ra>90", "CRI 85", "Ra80", "R80" - direct numeric
/// - "80%", ">80", ">=90" - with symbols
fn parse_cri(color_rendering_group: &str) -> Option<f64> {
    if color_rendering_group.is_empty() {
        return None;
    }

    let s = color_rendering_group.trim();

    // Skip "n/a", "none", "unknown" etc.
    let lower = s.to_lowercase();
    if lower == "n/a" || lower == "na" || lower == "none" || lower == "unknown" || lower == "-" {
        return None;
    }

    let upper = s.to_uppercase();

    // Extract all numbers from the string
    let numbers = extract_numbers(s);

    // Find a number that looks like a CRI (20-100)
    // Prefer larger numbers (more precise measurement over group approximation)
    let cri_candidates: Vec<f64> = numbers
        .iter()
        .copied()
        .filter(|&n| (20.0..=100.0).contains(&n))
        .collect();

    if let Some(&cri) = cri_candidates.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
        return Some(cri);
    }

    // Fall back to CIE color rendering groups
    // Check each part for group codes
    for part in upper.split(&['/', '-', ' ', ':', '(', ')'][..]) {
        let part = part.trim();
        match part {
            "1A" => return Some(90.0),
            "1B" => return Some(80.0),
            "2A" | "2" => return Some(70.0),
            "2B" => return Some(60.0),
            "3" => return Some(50.0),
            "4" => return Some(40.0),
            _ => {}
        }
    }

    // Try matching group at start of string (e.g., "1Bxxx")
    if upper.starts_with("1A") {
        return Some(90.0);
    }
    if upper.starts_with("1B") {
        return Some(80.0);
    }
    if upper.starts_with("2A") {
        return Some(70.0);
    }
    if upper.starts_with("2B") {
        return Some(60.0);
    }

    None
}

/// Convert CRI value to color rendering group
fn cri_to_group(cri: f64) -> String {
    match cri as i32 {
        90..=100 => "1A".to_string(),
        80..=89 => "1B".to_string(),
        70..=79 => "2A".to_string(),
        60..=69 => "2B".to_string(),
        40..=59 => "3".to_string(),
        _ => "4".to_string(),
    }
}

/// Determine symmetry from horizontal angles
#[cfg(feature = "eulumdat")]
fn determine_symmetry(horizontal_angles: &[f64]) -> EulumdatSymmetry {
    if horizontal_angles.len() <= 1 {
        return EulumdatSymmetry::VerticalAxis;
    }

    let max_angle = horizontal_angles.iter().copied().fold(0.0_f64, f64::max);
    let min_angle = horizontal_angles.iter().copied().fold(360.0_f64, f64::min);

    // Check range of angles
    if (max_angle - min_angle) < 1.0 {
        EulumdatSymmetry::VerticalAxis
    } else if max_angle <= 90.5 {
        EulumdatSymmetry::BothPlanes
    } else if max_angle <= 180.5 {
        if min_angle < 0.5 {
            EulumdatSymmetry::PlaneC0C180
        } else {
            EulumdatSymmetry::PlaneC90C270
        }
    } else {
        EulumdatSymmetry::None
    }
}

/// Calculate downward flux fraction and light output ratio
#[cfg(feature = "eulumdat")]
fn calculate_flux_fractions(ldt: &Eulumdat) -> (f64, f64) {
    // Simple approximation based on intensity data
    // Real calculation would require proper integration

    let mut downward_flux = 0.0;
    let mut total_flux = 0.0;

    for (g_idx, &g_angle) in ldt.g_angles.iter().enumerate() {
        let sin_g = g_angle.to_radians().sin();

        for c_plane in &ldt.intensities {
            if let Some(&intensity) = c_plane.get(g_idx) {
                let flux_contribution = intensity * sin_g;
                total_flux += flux_contribution;

                // Downward: 0-90 degrees
                if g_angle <= 90.0 {
                    downward_flux += flux_contribution;
                }
            }
        }
    }

    let dff = if total_flux > 0.0 {
        (downward_flux / total_flux * 100.0).min(100.0)
    } else {
        0.0
    };

    // LOR is typically provided separately, estimate from data
    let lor = 100.0; // Default to 100% if not calculable

    (dff, lor)
}

#[cfg(feature = "eulumdat")]
impl LuminaireOpticalData {
    /// Convert from Eulumdat format
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        ldt.into()
    }

    /// Convert to Eulumdat format
    pub fn to_eulumdat(&self) -> Eulumdat {
        self.into()
    }
}

#[cfg(all(test, feature = "eulumdat"))]
mod tests {
    use super::*;

    #[test]
    fn test_cct_parsing() {
        // Direct numeric
        assert_eq!(parse_cct("3000K"), Some(3000.0));
        assert_eq!(parse_cct("4000"), Some(4000.0));
        assert_eq!(parse_cct("3000.0"), Some(3000.0));
        // With prefix (tw=tunable white, ww=warm white, cw=cool white)
        assert_eq!(parse_cct("tw/6500"), Some(6500.0));
        assert_eq!(parse_cct("ww/2700"), Some(2700.0));
        assert_eq!(parse_cct("cw/5000"), Some(5000.0));
        assert_eq!(parse_cct("TW-6500"), Some(6500.0));
        // Named temperatures
        assert_eq!(parse_cct("warm white"), Some(2700.0));
        assert_eq!(parse_cct("daylight"), Some(6500.0));
        assert_eq!(parse_cct("neutral"), Some(4000.0));
        // Mixed with other text
        assert_eq!(parse_cct("LED 3000K"), Some(3000.0));
        assert_eq!(parse_cct("3000K LED"), Some(3000.0));
        assert_eq!(parse_cct("CCT:4000"), Some(4000.0));
        assert_eq!(parse_cct("CT3000"), Some(3000.0));
        // Empty/invalid
        assert_eq!(parse_cct(""), None);
        assert_eq!(parse_cct("n/a"), None);
        assert_eq!(parse_cct("none"), None);
        assert_eq!(parse_cct("-"), None);
    }

    #[test]
    fn test_cri_parsing() {
        // CIE groups
        assert_eq!(parse_cri("1A"), Some(90.0));
        assert_eq!(parse_cri("1B"), Some(80.0));
        assert_eq!(parse_cri("2A"), Some(70.0));
        // With numeric value (should prefer numeric)
        assert_eq!(parse_cri("1B/86"), Some(86.0));
        assert_eq!(parse_cri("1A/95"), Some(95.0));
        assert_eq!(parse_cri("1A-95"), Some(95.0));
        // Direct numeric
        assert_eq!(parse_cri("80"), Some(80.0));
        assert_eq!(parse_cri("Ra>90"), Some(90.0));
        assert_eq!(parse_cri("CRI 85"), Some(85.0));
        assert_eq!(parse_cri("Ra80"), Some(80.0));
        assert_eq!(parse_cri("R80"), Some(80.0));
        assert_eq!(parse_cri(">80"), Some(80.0));
        assert_eq!(parse_cri(">=90"), Some(90.0));
        // Group at start
        assert_eq!(parse_cri("1Bxyz"), Some(80.0));
        // Empty/invalid
        assert_eq!(parse_cri(""), None);
        assert_eq!(parse_cri("n/a"), None);
        assert_eq!(parse_cri("none"), None);
    }

    #[test]
    fn test_cri_to_group() {
        assert_eq!(cri_to_group(95.0), "1A");
        assert_eq!(cri_to_group(85.0), "1B");
        assert_eq!(cri_to_group(75.0), "2A");
    }
}
