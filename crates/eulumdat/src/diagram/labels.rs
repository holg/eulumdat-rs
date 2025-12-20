//! Internationalization labels for diagram text
//!
//! This module provides all translatable strings used in SVG diagrams.
//! Pass a custom `DiagramLabels` instance for localization.

/// Labels for photometric diagrams (polar, cartesian, heatmap)
#[derive(Debug, Clone)]
pub struct DiagramLabels {
    // Units
    /// Intensity unit label (default: "cd/1000lm")
    pub intensity_unit: &'static str,
    /// Short intensity unit (default: "cd/klm")
    pub intensity_unit_short: &'static str,

    // Axis labels
    /// Gamma angle axis label (default: "Gamma (γ)")
    pub gamma_axis: &'static str,
    /// Intensity axis label (default: "Intensity (cd/klm)")
    pub intensity_axis: &'static str,
    /// C-plane angle axis label (default: "C-Plane Angle (°)")
    pub c_plane_axis: &'static str,
    /// Gamma angle axis label for heatmap (default: "Gamma Angle (°)")
    pub gamma_angle_axis: &'static str,

    // Plane names
    /// C0-C180 plane label
    pub plane_c0_c180: &'static str,
    /// C90-C270 plane label
    pub plane_c90_c270: &'static str,

    // Angle types
    /// Beam angle label (default: "Beam")
    pub beam: &'static str,
    /// Field angle label (default: "Field")
    pub field: &'static str,
    /// Beam percentage label (default: "Beam 50%")
    pub beam_50_percent: &'static str,
    /// Field percentage label (default: "Field 10%")
    pub field_10_percent: &'static str,

    // Metric labels
    /// CIE classification label (default: "CIE:")
    pub cie_label: &'static str,
    /// Efficacy label (default: "Eff:")
    pub efficacy_label: &'static str,
    /// Maximum label (default: "Max:")
    pub max_label: &'static str,
    /// Spacing/height ratio label (default: "S/H:")
    pub sh_ratio_label: &'static str,

    // Titles
    /// Heatmap title (default: "Intensity Heatmap (Candela)")
    pub heatmap_title: &'static str,

    // Placeholders
    /// No data placeholder (default: "No data")
    pub no_data: &'static str,
}

impl Default for DiagramLabels {
    fn default() -> Self {
        Self::english()
    }
}

impl DiagramLabels {
    /// English labels (default)
    pub const fn english() -> Self {
        Self {
            intensity_unit: "cd/1000lm",
            intensity_unit_short: "cd/klm",
            gamma_axis: "Gamma (γ)",
            intensity_axis: "Intensity (cd/klm)",
            c_plane_axis: "C-Plane Angle (°)",
            gamma_angle_axis: "Gamma Angle (°)",
            plane_c0_c180: "C0-C180",
            plane_c90_c270: "C90-C270",
            beam: "Beam",
            field: "Field",
            beam_50_percent: "Beam 50%",
            field_10_percent: "Field 10%",
            cie_label: "CIE:",
            efficacy_label: "Eff:",
            max_label: "Max:",
            sh_ratio_label: "S/H:",
            heatmap_title: "Intensity Heatmap (Candela)",
            no_data: "No data",
        }
    }

    /// German labels
    pub const fn german() -> Self {
        Self {
            intensity_unit: "cd/1000lm",
            intensity_unit_short: "cd/klm",
            gamma_axis: "Gamma (γ)",
            intensity_axis: "Lichtstärke (cd/klm)",
            c_plane_axis: "C-Ebene Winkel (°)",
            gamma_angle_axis: "Gamma Winkel (°)",
            plane_c0_c180: "C0-C180",
            plane_c90_c270: "C90-C270",
            beam: "Strahl",
            field: "Feld",
            beam_50_percent: "Strahl 50%",
            field_10_percent: "Feld 10%",
            cie_label: "CIE:",
            efficacy_label: "Eff:",
            max_label: "Max:",
            sh_ratio_label: "A/H:",
            heatmap_title: "Lichtstärke-Heatmap (Candela)",
            no_data: "Keine Daten",
        }
    }

    /// French labels
    pub const fn french() -> Self {
        Self {
            intensity_unit: "cd/1000lm",
            intensity_unit_short: "cd/klm",
            gamma_axis: "Gamma (γ)",
            intensity_axis: "Intensité (cd/klm)",
            c_plane_axis: "Angle plan C (°)",
            gamma_angle_axis: "Angle Gamma (°)",
            plane_c0_c180: "C0-C180",
            plane_c90_c270: "C90-C270",
            beam: "Faisceau",
            field: "Champ",
            beam_50_percent: "Faisceau 50%",
            field_10_percent: "Champ 10%",
            cie_label: "CIE:",
            efficacy_label: "Eff:",
            max_label: "Max:",
            sh_ratio_label: "E/H:",
            heatmap_title: "Carte de chaleur d'intensité (Candela)",
            no_data: "Pas de données",
        }
    }

    /// Chinese (Simplified) labels
    pub const fn chinese() -> Self {
        Self {
            intensity_unit: "cd/1000lm",
            intensity_unit_short: "cd/klm",
            gamma_axis: "伽马角 (γ)",
            intensity_axis: "光强 (cd/klm)",
            c_plane_axis: "C面角度 (°)",
            gamma_angle_axis: "伽马角度 (°)",
            plane_c0_c180: "C0-C180",
            plane_c90_c270: "C90-C270",
            beam: "光束",
            field: "场",
            beam_50_percent: "光束 50%",
            field_10_percent: "场 10%",
            cie_label: "CIE:",
            efficacy_label: "效率:",
            max_label: "最大:",
            sh_ratio_label: "间高比:",
            heatmap_title: "光强热图 (坎德拉)",
            no_data: "无数据",
        }
    }

    /// Japanese labels
    pub const fn japanese() -> Self {
        Self {
            intensity_unit: "cd/1000lm",
            intensity_unit_short: "cd/klm",
            gamma_axis: "ガンマ角 (γ)",
            intensity_axis: "光度 (cd/klm)",
            c_plane_axis: "C面角度 (°)",
            gamma_angle_axis: "ガンマ角度 (°)",
            plane_c0_c180: "C0-C180",
            plane_c90_c270: "C90-C270",
            beam: "ビーム",
            field: "フィールド",
            beam_50_percent: "ビーム 50%",
            field_10_percent: "フィールド 10%",
            cie_label: "CIE:",
            efficacy_label: "効率:",
            max_label: "最大:",
            sh_ratio_label: "S/H:",
            heatmap_title: "光度ヒートマップ (カンデラ)",
            no_data: "データなし",
        }
    }

    /// Spanish labels
    pub const fn spanish() -> Self {
        Self {
            intensity_unit: "cd/1000lm",
            intensity_unit_short: "cd/klm",
            gamma_axis: "Gamma (γ)",
            intensity_axis: "Intensidad (cd/klm)",
            c_plane_axis: "Ángulo plano C (°)",
            gamma_angle_axis: "Ángulo Gamma (°)",
            plane_c0_c180: "C0-C180",
            plane_c90_c270: "C90-C270",
            beam: "Haz",
            field: "Campo",
            beam_50_percent: "Haz 50%",
            field_10_percent: "Campo 10%",
            cie_label: "CIE:",
            efficacy_label: "Ef:",
            max_label: "Máx:",
            sh_ratio_label: "E/A:",
            heatmap_title: "Mapa de calor de intensidad (Candela)",
            no_data: "Sin datos",
        }
    }

    /// Get labels for a language code (ISO 639-1)
    pub fn for_language(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "de" => Self::german(),
            "fr" => Self::french(),
            "zh" => Self::chinese(),
            "ja" => Self::japanese(),
            "es" => Self::spanish(),
            _ => Self::english(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_english() {
        let labels = DiagramLabels::default();
        assert_eq!(labels.beam, "Beam");
        assert_eq!(labels.no_data, "No data");
    }

    #[test]
    fn test_language_lookup() {
        let german = DiagramLabels::for_language("de");
        assert_eq!(german.beam, "Strahl");

        let french = DiagramLabels::for_language("fr");
        assert_eq!(french.beam, "Faisceau");

        // Unknown falls back to English
        let unknown = DiagramLabels::for_language("xx");
        assert_eq!(unknown.beam, "Beam");
    }
}
