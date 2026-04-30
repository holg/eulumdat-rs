//! Greenhouse/Horticultural PPFD diagram generation
//!
//! Creates SVG visualizations showing PPFD (Photosynthetic Photon Flux Density)
//! at different distances from the luminaire for horticultural applications.

use crate::atla::types::{LuminaireOpticalData, SpectralDistribution};

/// Localized labels for greenhouse/PPFD diagram
#[derive(Debug, Clone)]
pub struct GreenhouseLabels {
    /// Title: "Greenhouse PPFD at Distance"
    pub title: String,
    /// Unit: "µmol/m²/s"
    pub unit: String,
    /// "PPF" label
    pub ppf: String,
    /// "Efficacy" label
    pub efficacy: String,
    /// "Beam" label
    pub beam: String,
    /// "Power" label
    pub power: String,
    /// "Flowering" growth stage
    pub flowering: String,
    /// "Veg" / "Vegetative" growth stage
    pub vegetative: String,
    /// "Seedling" growth stage
    pub seedling: String,
}

impl Default for GreenhouseLabels {
    fn default() -> Self {
        Self {
            title: "Greenhouse PPFD at Distance".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Efficacy".to_string(),
            beam: "Beam".to_string(),
            power: "Power".to_string(),
            flowering: "Flowering".to_string(),
            vegetative: "Veg".to_string(),
            seedling: "Seedling".to_string(),
        }
    }
}

impl GreenhouseLabels {
    /// German labels
    pub fn german() -> Self {
        Self {
            title: "Gewächshaus PPFD nach Abstand".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Effizienz".to_string(),
            beam: "Strahl".to_string(),
            power: "Leistung".to_string(),
            flowering: "Blüte".to_string(),
            vegetative: "Wachstum".to_string(),
            seedling: "Sämling".to_string(),
        }
    }

    /// Chinese labels
    pub fn chinese() -> Self {
        Self {
            title: "温室PPFD随距离变化".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "效能".to_string(),
            beam: "光束".to_string(),
            power: "功率".to_string(),
            flowering: "开花".to_string(),
            vegetative: "营养".to_string(),
            seedling: "幼苗".to_string(),
        }
    }

    /// French labels
    pub fn french() -> Self {
        Self {
            title: "PPFD Serre par Distance".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Efficacité".to_string(),
            beam: "Faisceau".to_string(),
            power: "Puissance".to_string(),
            flowering: "Floraison".to_string(),
            vegetative: "Croissance".to_string(),
            seedling: "Semis".to_string(),
        }
    }

    /// Italian labels
    pub fn italian() -> Self {
        Self {
            title: "PPFD Serra per Distanza".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Efficienza".to_string(),
            beam: "Fascio".to_string(),
            power: "Potenza".to_string(),
            flowering: "Fioritura".to_string(),
            vegetative: "Crescita".to_string(),
            seedling: "Piantina".to_string(),
        }
    }

    /// Russian labels
    pub fn russian() -> Self {
        Self {
            title: "PPFD теплицы по расстоянию".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Эффективность".to_string(),
            beam: "Луч".to_string(),
            power: "Мощность".to_string(),
            flowering: "Цветение".to_string(),
            vegetative: "Рост".to_string(),
            seedling: "Рассада".to_string(),
        }
    }

    /// Spanish labels
    pub fn spanish() -> Self {
        Self {
            title: "PPFD Invernadero por Distancia".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Eficacia".to_string(),
            beam: "Haz".to_string(),
            power: "Potencia".to_string(),
            flowering: "Floración".to_string(),
            vegetative: "Crecimiento".to_string(),
            seedling: "Plántula".to_string(),
        }
    }

    /// Portuguese (Brazil) labels
    pub fn portuguese_brazil() -> Self {
        Self {
            title: "PPFD Estufa por Distância".to_string(),
            unit: "µmol/m²/s".to_string(),
            ppf: "PPF".to_string(),
            efficacy: "Eficácia".to_string(),
            beam: "Feixe".to_string(),
            power: "Potência".to_string(),
            flowering: "Floração".to_string(),
            vegetative: "Crescimento".to_string(),
            seedling: "Muda".to_string(),
        }
    }
}

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
    pub ppfd: f64,       // µmol/m²/s
    pub coverage_m: f64, // beam diameter at this distance
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
    /// Create greenhouse diagram from ATLA document with default max height (2.0m)
    pub fn from_atla(doc: &LuminaireOpticalData) -> Self {
        Self::from_atla_with_height(doc, 2.0)
    }

    /// Create greenhouse diagram from ATLA document with custom max height
    pub fn from_atla_with_height(doc: &LuminaireOpticalData, max_height: f64) -> Self {
        let emitter = doc.emitters.first();

        // Get power and lumens. Filter out zero values explicitly: many
        // EULUMDAT files store 0 in `wattage_with_ballast` or
        // `total_luminous_flux` when the field is unknown, and the
        // atla converter passes those through as `Some(0.0)` rather
        // than `None`. A plain `.or` chain wouldn't fall back through
        // `Some(0.0)`, leaving us with PPF = 0 and the entire grid
        // rendering empty.
        let watts = emitter
            .and_then(|e| e.input_watts)
            .filter(|&v| v > 0.0)
            .unwrap_or(100.0);
        let lumens = emitter
            .and_then(|e| {
                e.measured_lumens
                    .filter(|&v| v > 0.0)
                    .or(e.rated_lumens.filter(|&v| v > 0.0))
            })
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
            .map(estimate_beam_angle)
            .unwrap_or(120.0);

        // Calculate PPFD at various distances based on max_height
        // Clamp the beam angle into a sane range so the coverage maths
        // doesn't degenerate. Anything ≤ 0° would zero out the area;
        // anything ≥ 180° drives `tan(beam/2)` to infinity. 5°/170° are
        // the practical bounds of realistic luminaires.
        let beam_angle = beam_angle.clamp(5.0, 170.0);

        // Generate 6-7 distance levels from 0.15*max to max
        let distances = Self::generate_distances(max_height);
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

    /// Generate distance levels based on max height
    fn generate_distances(max_height: f64) -> Vec<f64> {
        if max_height <= 1.0 {
            // For short distances, use finer steps
            vec![0.1, 0.2, 0.3, 0.5, 0.75, max_height]
                .into_iter()
                .filter(|&d| d <= max_height)
                .collect()
        } else if max_height <= 2.0 {
            // Standard range
            vec![0.3, 0.5, 0.75, 1.0, 1.5, max_height]
                .into_iter()
                .filter(|&d| d <= max_height)
                .collect()
        } else if max_height <= 4.0 {
            // Extended range
            vec![0.5, 1.0, 1.5, 2.0, 3.0, max_height]
                .into_iter()
                .filter(|&d| d <= max_height)
                .collect()
        } else {
            // Very tall (industrial)
            vec![1.0, 2.0, 3.0, 4.0, 5.0, max_height]
                .into_iter()
                .filter(|&d| d <= max_height)
                .collect()
        }
    }

    /// Get max distance from ppfd_levels
    pub fn max_distance(&self) -> f64 {
        self.ppfd_levels
            .iter()
            .map(|l| l.distance_m)
            .fold(0.0_f64, f64::max)
    }

    /// Generate SVG visualization
    pub fn to_svg(&self, width: f64, height: f64, theme: &GreenhouseTheme) -> String {
        self.to_svg_with_labels(width, height, theme, &GreenhouseLabels::default())
    }

    /// Generate SVG visualization with localized labels
    pub fn to_svg_with_labels(
        &self,
        width: f64,
        height: f64,
        theme: &GreenhouseTheme,
        labels: &GreenhouseLabels,
    ) -> String {
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
            "  <text x=\"{}\" y=\"25\" fill=\"{}\" font-size=\"14\" font-family=\"{}\" font-weight=\"bold\" text-anchor=\"middle\">{}</text>\n",
            width / 2.0, theme.foreground, theme.font_family, labels.title
        ));

        // Draw greenhouse outline
        let greenhouse_y = margin + 20.0;
        let greenhouse_height = plot_height - 40.0;

        // Ground/bench
        svg.push_str(&format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"10\" fill=\"#8b5a2b\" rx=\"2\"/>\n",
            margin,
            greenhouse_y + greenhouse_height,
            plot_width
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
        let beam_bottom_width =
            2.0 * greenhouse_height * (self.beam_angle / 2.0_f64).to_radians().tan();
        let beam_left = width / 2.0 - beam_bottom_width / 2.0;
        let beam_right = width / 2.0 + beam_bottom_width / 2.0;

        svg.push_str(&format!(
            "  <polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" opacity=\"0.15\"/>\n",
            width / 2.0 - lum_width / 4.0,
            lum_y + 12.0,
            beam_left.max(margin),
            greenhouse_y + greenhouse_height,
            beam_right.min(margin + plot_width),
            greenhouse_y + greenhouse_height,
            theme.beam_color
        ));
        svg.push_str(&format!(
            "  <polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" opacity=\"0.25\"/>\n",
            width / 2.0 - lum_width / 6.0,
            lum_y + 12.0,
            (width / 2.0 - beam_bottom_width / 4.0).max(margin),
            greenhouse_y + greenhouse_height,
            (width / 2.0 + beam_bottom_width / 4.0).min(margin + plot_width),
            greenhouse_y + greenhouse_height,
            theme.beam_color
        ));

        // Distance markers and PPFD values
        let max_distance = self.max_distance();
        for level in &self.ppfd_levels {
            if level.distance_m <= max_distance {
                let y = greenhouse_y
                    + 12.0
                    + (level.distance_m / max_distance) * (greenhouse_height - 12.0);

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
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\" text-anchor=\"end\">{}</text>\n",
            margin + plot_width + 45.0, greenhouse_y + 15.0, theme.foreground, theme.font_family, labels.unit
        ));

        // Info box at bottom
        let info_y = height - 55.0;
        svg.push_str(&format!(
            "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"50\" fill=\"{}\" opacity=\"0.5\" rx=\"4\"/>\n",
            margin, info_y, plot_width, theme.grid_color
        ));

        // PPF, Efficacy, Beam Angle
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">{}:</tspan> {:.0} µmol/s</text>\n",
            margin + 10.0, info_y + 18.0, theme.foreground, theme.font_family, labels.ppf, self.ppf
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">{}:</tspan> {:.2} µmol/J</text>\n",
            margin + 10.0, info_y + 35.0, theme.foreground, theme.font_family, labels.efficacy, self.efficacy
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">{}:</tspan> {:.0}°</text>\n",
            margin + 150.0, info_y + 18.0, theme.foreground, theme.font_family, labels.beam, self.beam_angle
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"11\" font-family=\"{}\"><tspan font-weight=\"bold\">{}:</tspan> {:.0}W</text>\n",
            margin + 150.0, info_y + 35.0, theme.foreground, theme.font_family, labels.power, self.watts
        ));

        // PPFD legend
        svg.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            margin + 280.0,
            info_y + 14.0,
            theme.ppfd_high
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\">&gt;800 ({})</text>\n",
            margin + 290.0, info_y + 18.0, theme.foreground, theme.font_family, labels.flowering
        ));
        svg.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            margin + 280.0,
            info_y + 30.0,
            theme.ppfd_medium
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\">400-800 ({})</text>\n",
            margin + 290.0, info_y + 34.0, theme.foreground, theme.font_family, labels.vegetative
        ));
        svg.push_str(&format!(
            "  <circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\"/>\n",
            margin + 280.0,
            info_y + 46.0,
            theme.ppfd_low
        ));
        svg.push_str(&format!(
            "  <text x=\"{}\" y=\"{}\" fill=\"{}\" font-size=\"9\" font-family=\"{}\">&lt;400 ({})</text>\n",
            margin + 290.0, info_y + 50.0, theme.foreground, theme.font_family, labels.seedling
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

/// Estimate beam angle from intensity distribution.
///
/// Beam angle ≈ 2 × the angular distance from the peak intensity to the
/// 50%-of-peak shoulder. For a symmetric downlight the peak sits at
/// gamma=0 and the old "scan from index 0" heuristic worked, but for
/// asymmetric road luminaires (and OxyTech-style office downlights
/// where the peak is offset from nadir) the peak is somewhere in the
/// middle of the distribution. We anchor on the actual peak index and
/// walk outward.
fn estimate_beam_angle(dist: &crate::atla::types::IntensityDistribution) -> f64 {
    if dist.intensities.is_empty() || dist.vertical_angles.is_empty() {
        return 120.0;
    }

    let intensities = &dist.intensities[0];
    if intensities.is_empty() {
        return 120.0;
    }

    // Locate the peak.
    let mut peak_idx = 0;
    let mut peak = intensities[0];
    for (i, &v) in intensities.iter().enumerate() {
        if v > peak {
            peak = v;
            peak_idx = i;
        }
    }
    if peak <= 0.0 {
        return 120.0;
    }
    let half_peak = peak * 0.5;

    // Find the half-peak shoulder above the peak (larger gamma side).
    let high_shoulder: Option<f64> = intensities
        .iter()
        .enumerate()
        .skip(peak_idx)
        .find(|(_, &v)| v < half_peak)
        .and_then(|(i, _)| dist.vertical_angles.get(i).copied());

    // And below the peak (smaller gamma side).
    let low_shoulder: Option<f64> = intensities
        .iter()
        .enumerate()
        .take(peak_idx + 1)
        .rev()
        .find(|(_, &v)| v < half_peak)
        .and_then(|(i, _)| dist.vertical_angles.get(i).copied());

    let peak_angle = dist.vertical_angles.get(peak_idx).copied().unwrap_or(0.0);
    match (low_shoulder, high_shoulder) {
        (Some(lo), Some(hi)) => (hi - lo).abs(),
        // One-sided shoulder: assume symmetric distribution, mirror it.
        (None, Some(hi)) => 2.0 * (hi - peak_angle).abs(),
        (Some(lo), None) => 2.0 * (peak_angle - lo).abs(),
        // No shoulder reached within the sampled distribution — the beam
        // is wider than the angular range we have. Fall back to wide.
        (None, None) => 120.0,
    }
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

    /// Regression: an emitter whose peak intensity sits *off* gamma=0
    /// (typical of asymmetric office downlights and road luminaires)
    /// previously produced beam_angle = 0 and PPFD = 0 everywhere.
    /// The peak-anchored estimator now finds the half-peak shoulder
    /// correctly.
    #[test]
    fn asymmetric_distribution_yields_nonzero_ppfd() {
        use crate::atla::types::{Emitter, IntensityDistribution};

        // Synthetic distribution: peak at gamma=20°, ~half-peak at
        // gamma=10° and gamma=40°. Modeled after the BIOLUX / Pinco-
        // shape downlights where the brightest direction is offset
        // from nadir.
        let intensities = vec![vec![
            100.0, // 0°
            300.0, // 5°
            450.0, // 10°
            550.0, // 15°
            600.0, // 20° ← peak
            500.0, // 25°
            400.0, // 30°
            300.0, // 35°
            200.0, // 40°
            100.0, // 45°
            50.0,  // 50°
            10.0,  // 55°
            0.0,   // 60°
        ]];
        let vertical = (0..13).map(|i| (i as f64) * 5.0).collect();
        let dist = IntensityDistribution {
            vertical_angles: vertical,
            horizontal_angles: vec![0.0],
            intensities,
            ..Default::default()
        };
        let emitter = Emitter {
            quantity: 1,
            rated_lumens: Some(4247.0),
            input_watts: Some(84.5),
            intensity_distribution: Some(dist),
            ..Default::default()
        };
        let mut doc = LuminaireOpticalData::new();
        doc.emitters.push(emitter);

        let diagram = GreenhouseDiagram::from_atla_with_height(&doc, 2.0);
        // PPF should be lumens × ~1.2 / 1000 × 15 ≈ 76 µmol/s.
        assert!(
            (70.0..90.0).contains(&diagram.ppf),
            "PPF out of expected band: {}",
            diagram.ppf
        );
        // Beam angle from the synthetic peak-at-20 / half-at-10/40
        // distribution should be ~30°, definitely not 0.
        assert!(
            diagram.beam_angle > 5.0,
            "beam_angle collapsed to {}; estimator regressed",
            diagram.beam_angle
        );
        // And PPFD at the closest distance must be > 0.
        assert!(
            diagram.ppfd_levels.iter().any(|l| l.ppfd > 1.0),
            "every PPFD row was below 1 µmol/m²/s; diagram = {:#?}",
            diagram.ppfd_levels
        );
    }

    /// Regression: an emitter where the lamp set stored 0.0 for the
    /// flux/wattage fields (rzb.ldt-class) used to produce PPFD = 0.
    /// The zero-filtering on `lumens`/`watts` lookup falls back to the
    /// sane defaults in that case so the diagram still renders.
    #[test]
    fn zero_lamp_flux_falls_back_to_defaults() {
        use crate::atla::types::Emitter;

        let emitter = Emitter {
            quantity: 1,
            rated_lumens: Some(0.0),
            measured_lumens: Some(0.0),
            input_watts: Some(0.0),
            ..Default::default()
        };
        let mut doc = LuminaireOpticalData::new();
        doc.emitters.push(emitter);

        let diagram = GreenhouseDiagram::from_atla_with_height(&doc, 2.0);
        assert!(
            diagram.ppf > 0.0,
            "zero-flux fallback failed; PPF = {}",
            diagram.ppf
        );
    }
}
