//! Greenhouse/Horticultural PPFD diagram generation
//!
//! Creates SVG visualizations showing PPFD (Photosynthetic Photon Flux Density)
//! at different distances from the luminaire for horticultural applications.

use crate::types::{LuminaireOpticalData, SpectralDistribution};

/// Theme for greenhouse diagram
#[derive(Debug, Clone)]
pub struct GreenhouseTheme {
    pub background: String,
    pub foreground: String,
    pub luminaire_color: String,
    pub beam_color: String,
    pub plant_color: String,
    pub grid_color: String,
    pub ppfd_high: String,   // > 800 µmol
    pub ppfd_medium: String, // 400-800 µmol
    pub ppfd_low: String,    // < 400 µmol
    pub font_family: String,
}

impl Default for GreenhouseTheme {
    fn default() -> Self {
        Self::light()
    }
}

impl GreenhouseTheme {
    pub fn light() -> Self {
        Self {
            background: "#f8fafc".to_string(),
            foreground: "#1e293b".to_string(),
            luminaire_color: "#475569".to_string(),
            beam_color: "#fbbf24".to_string(),
            plant_color: "#22c55e".to_string(),
            grid_color: "#e2e8f0".to_string(),
            ppfd_high: "#22c55e".to_string(),
            ppfd_medium: "#eab308".to_string(),
            ppfd_low: "#f97316".to_string(),
            font_family: "system-ui, sans-serif".to_string(),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: "#1e293b".to_string(),
            foreground: "#f1f5f9".to_string(),
            luminaire_color: "#94a3b8".to_string(),
            beam_color: "#fbbf24".to_string(),
            plant_color: "#4ade80".to_string(),
            grid_color: "#334155".to_string(),
            ppfd_high: "#4ade80".to_string(),
            ppfd_medium: "#facc15".to_string(),
            ppfd_low: "#fb923c".to_string(),
            font_family: "system-ui, sans-serif".to_string(),
        }
    }
}

/// PPFD at a specific distance
#[derive(Debug, Clone)]
pub struct PpfdAtDistance {
    pub distance_m: f64,
    pub ppfd: f64,        // µmol/m²/s
    pub coverage_m: f64,  // beam diameter at this distance
}

/// Greenhouse PPFD diagram data
#[derive(Debug, Clone)]
pub struct GreenhouseDiagram {
    /// Total PPF (Photosynthetic Photon Flux) in µmol/s
    pub ppf: f64,
    /// Luminaire power in watts
    pub watts: f64,
    /// Efficacy in µmol/J
    pub efficacy: f64,
    /// PPFD at various distances
    pub ppfd_levels: Vec<PpfdAtDistance>,
    /// Beam angle (half-angle at 50% intensity)
    pub beam_angle: f64,
    /// Recommended mounting heights for different growth stages
    pub recommendations: Vec<(String, f64, f64)>, // (stage, min_height, max_height)
}

impl GreenhouseDiagram {
    /// Create greenhouse diagram from ATLA document
    pub fn from_atla(doc: &LuminaireOpticalData) -> Self {
        let emitter = doc.emitters.first();

        // Get power and lumens
        let watts = emitter.and_then(|e| e.input_watts).unwrap_or(100.0);
        let lumens = emitter
            .and_then(|e| e.measured_lumens.or(e.rated_lumens))
            .unwrap_or(10000.0);

        // Estimate PPF from lumens (conversion factor depends on spectrum)
        // For full spectrum white: ~1 µmol/lm, for red-heavy: ~1.5 µmol/lm
        let ppf_factor = if let Some(spd) = emitter.and_then(|e| e.spectral_distribution.as_ref()) {
            estimate_ppf_factor(spd)
        } else {
            1.2 // Default for typical grow light
        };

        let ppf = lumens * ppf_factor / 1000.0 * 15.0; // Scaled for typical grow light
        let efficacy = if watts > 0.0 { ppf / watts } else { 2.5 };

        // Estimate beam angle from intensity distribution
        let beam_angle = emitter
            .and_then(|e| e.intensity_distribution.as_ref())
            .map(|dist| estimate_beam_angle(dist))
            .unwrap_or(120.0);

        // Calculate PPFD at various distances
        let distances = vec![0.3, 0.5, 0.75, 1.0, 1.5, 2.0];
        let ppfd_levels: Vec<PpfdAtDistance> = distances
            .iter()
            .map(|&d| {
                let coverage = 2.0 * d * (beam_angle / 2.0_f64).to_radians().tan();
                let area = std::f64::consts::PI * (coverage / 2.0).powi(2);
                let ppfd = if area > 0.0 { ppf / area } else { 0.0 };
                PpfdAtDistance {
                    distance_m: d,
                    ppfd,
                    coverage_m: coverage,
                }
            })
            .collect();

        // Growth stage recommendations based on typical PPFD requirements
        let recommendations = vec![
            ("Seedling/Clone".to_string(), 1.5, 2.0),
            ("Vegetative".to_string(), 0.75, 1.0),
            ("Flowering".to_string(), 0.3, 0.5),
        ];

        Self {
            ppf,
            watts,
            efficacy,
            ppfd_levels,
            beam_angle,
            recommendations,
        }
    }

    /// Generate SVG visualization
    pub fn to_svg(&self, width: f64, height: f64, theme: &GreenhouseTheme) -> String {
        let margin = 40.0;
        let plot_width = width - 2.0 * margin;
        let plot_height = height - 2.0 * margin - 60.0; // Extra space for legend

        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">
  <rect width="{}" height="{}" fill="{}"/>
"#,
            width, height, width, height, width, height, theme.background
        );

        // Title
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"25\" fill=\"{}\" font-size=\"14\" font-family=\"{}\" font-weight=\"bold\" text-anchor=\"middle\">Greenhouse PPFD at Distance</text>\n",
            width / 2.0, theme.foreground, theme.font_family
        ));

        // Draw greenhouse outline
        let greenhouse_y = margin + 20.0;
        let greenhouse_height = plot_height - 40.0;

        // Ground/bench
        svg.push_str(&format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"10\" fill=\"#8b5a2b\" rx=\"2\"/>\n",
            margin, greenhouse_y + greenhouse_height, plot_width
        ));

        // Plants on bench
        for i in 0..8 {
            let plant_x = margin + 30.0 + i as f64 * (plot_width - 60.0) / 7.0;
            let plant_height = 20.0 + (i % 3) as f64 * 10.0;
            svg.push_str(&format!(
                "  <ellipse cx=\"{}\" cy=\"{}\" rx=\"15\" ry=\"{}\" fill=\"{}\" opacity=\"0.8\"/>\n",
                plant_x, greenhouse_y + greenhouse_height - plant_height / 2.0, plant_height / 2.0, theme.plant_color
            ));
        }

        // Luminaire at top
        let lum_width = 80.0;
        let lum_x = width / 2.0 - lum_width / 2.0;
        let lum_y = greenhouse_y;

        svg.push_str(&format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"12\" fill=\"{}\" rx=\"3\"/>\n",
            lum_x, lum_y, lum_width, theme.luminaire_color
        ));

        // Light beam (cone)
        let beam_bottom_width = 2.0 * greenhouse_height * (self.beam_angle / 2.0_f64).to_radians().tan();
        let beam_left = width / 2.0 - beam_bottom_width / 2.0;
        let beam_right = width / 2.0 + beam_bottom_width / 2.0;

        svg.push_str(&format!(
            "  <polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" opacity=\"0.15\"/>\n",
            width / 2.0 - lum_width / 4.0, lum_y + 12.0,
            beam_left.max(margin), greenhouse_y + greenhouse_height,
            beam_right.min(margin + plot_width), greenhouse_y + greenhouse_height,
            theme.beam_color
        ));
        svg.push_str(&format!(
            "  <polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" opacity=\"0.25\"/>\n",
            width / 2.0 - lum_width / 6.0, lum_y + 12.0,
            (width / 2.0 - beam_bottom_width / 4.0).max(margin), greenhouse_y + greenhouse_height,
            (width / 2.0 + beam_bottom_width / 4.0).min(margin + plot_width), greenhouse_y + greenhouse_height,
            theme.beam_color
        ));

        // Distance markers and PPFD values
        let max_distance = 2.0;
        for level in &self.ppfd_levels {
            if level.distance_m <= max_distance {
                let y = greenhouse_y + 12.0 + (level.distance_m / max_distance) * (greenhouse_height - 12.0);

                // Horizontal line
                svg.push_str(&format!(
                    "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"4,4\"/>\n",
                    margin, y, margin + plot_width, y, theme.grid_color
                ));

                // Distance label on left
                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"10\" font-family=\"{}\" text-anchor=\"end\">{:.1}m</text>\n",
                    margin - 5.0, y + 4.0, theme.foreground, theme.font_family, level.distance_m
                ));

                // PPFD value on right with color coding
                let ppfd_color = if level.ppfd > 800.0 {
                    &theme.ppfd_high
                } else if level.ppfd > 400.0 {
                    &theme.ppfd_medium
                } else {
                    &theme.ppfd_low
                };

                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\" font-weight=\"bold\">{:.0}</text>\n",
                    margin + plot_width + 5.0, y + 4.0, ppfd_color, theme.font_family, level.ppfd
                ));
            }
        }

        // Unit label
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\" text-anchor=\"end\">µmol/m²/s</text>\n",
            margin + plot_width + 45.0, greenhouse_y + 15.0, theme.foreground, theme.font_family
        ));

        // Info box at bottom
        let info_y = height - 55.0;
        svg.push_str(&format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"50\" fill=\"{}\" opacity=\"0.5\" rx=\"4\"/>\n",
            margin, info_y, plot_width, theme.grid_color
        ));

        // PPF, Efficacy, Beam Angle
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">PPF:</tspan> {:.0} µmol/s</text>\n",
            margin + 10.0, info_y + 18.0, theme.foreground, theme.font_family, self.ppf
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">Efficacy:</tspan> {:.2} µmol/J</text>\n",
            margin + 10.0, info_y + 35.0, theme.foreground, theme.font_family, self.efficacy
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">Beam:</tspan> {:.0}°</text>\n",
            margin + 150.0, info_y + 18.0, theme.foreground, theme.font_family, self.beam_angle
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">Power:</tspan> {:.0}W</text>\n",
            margin + 150.0, info_y + 35.0, theme.foreground, theme.font_family, self.watts
        ));

        // PPFD legend
        svg.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            margin + 280.0, info_y + 14.0, theme.ppfd_high
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\">&gt;800 (Flowering)</text>\n",
            margin + 290.0, info_y + 18.0, theme.foreground, theme.font_family
        ));
        svg.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            margin + 280.0, info_y + 30.0, theme.ppfd_medium
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\">400-800 (Veg)</text>\n",
            margin + 290.0, info_y + 34.0, theme.foreground, theme.font_family
        ));
        svg.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            margin + 280.0, info_y + 46.0, theme.ppfd_low
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\">&lt;400 (Seedling)</text>\n",
            margin + 290.0, info_y + 50.0, theme.foreground, theme.font_family
        ));

        svg.push_str("</svg>");
        svg
    }
}

/// Estimate PPF conversion factor from spectral distribution
fn estimate_ppf_factor(spd: &SpectralDistribution) -> f64 {
    // Weight spectrum by PAR region (400-700nm)
    // Red-heavy spectra have higher µmol/lm conversion
    let mut par_sum = 0.0;
    let mut red_sum = 0.0;

    for (i, &wl) in spd.wavelengths.iter().enumerate() {
        let val = spd.values.get(i).copied().unwrap_or(0.0);

        if (400.0..=700.0).contains(&wl) {
            par_sum += val;
            if (600.0..=700.0).contains(&wl) {
                red_sum += val;
            }
        }
    }

    if par_sum > 0.0 {
        let red_ratio = red_sum / par_sum;
        // More red = higher conversion factor
        1.0 + red_ratio * 0.8
    } else {
        1.2
    }
}

/// Estimate beam angle from intensity distribution
fn estimate_beam_angle(dist: &crate::types::IntensityDistribution) -> f64 {
    // Find angle where intensity drops to 50% of peak
    if dist.intensities.is_empty() || dist.vertical_angles.is_empty() {
        return 120.0;
    }

    // Get first C-plane
    let intensities = &dist.intensities[0];
    let peak = intensities.iter().copied().fold(0.0_f64, f64::max);
    let half_peak = peak * 0.5;

    for (i, &val) in intensities.iter().enumerate() {
        if val < half_peak {
            if let Some(&angle) = dist.vertical_angles.get(i) {
                return angle * 2.0; // Full beam angle is 2x half-angle
            }
        }
    }

    120.0 // Default wide beam
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greenhouse_diagram_creation() {
        let doc = LuminaireOpticalData::new();
        let diagram = GreenhouseDiagram::from_atla(&doc);
        assert!(diagram.ppf > 0.0);
        assert!(!diagram.ppfd_levels.is_empty());
    }

    #[test]
    fn test_greenhouse_svg_generation() {
        let doc = LuminaireOpticalData::new();
        let diagram = GreenhouseDiagram::from_atla(&doc);
        let svg = diagram.to_svg(500.0, 400.0, &GreenhouseTheme::light());
        assert!(svg.contains("<svg"));
        assert!(svg.contains("PPFD"));
        assert!(svg.contains("µmol"));
    }
}
