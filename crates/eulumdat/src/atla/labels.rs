//! Internationalization labels for spectral diagrams
//!
//! This module provides all translatable strings used in spectral visualizations.

/// Labels for spectral diagrams (SPD, TM-30, etc.)
#[derive(Debug, Clone)]
pub struct SpectralLabels {
    // Axis labels
    /// Wavelength axis label (default: "Wavelength (nm)")
    pub wavelength_axis: &'static str,
    /// Relative power axis label (default: "Relative Power")
    pub relative_power_axis: &'static str,

    // Titles
    /// SPD diagram title (default: "Spectral Power Distribution")
    pub spd_title: &'static str,
    /// TM-30 CVG title (default: "Color Vector Graphic")
    pub cvg_title: &'static str,
    /// TM-30 Hue title (default: "Hue Bin Fidelity")
    pub hue_title: &'static str,

    // Spectral regions
    /// UV-A region label
    pub uv_a: &'static str,
    /// Visible region label
    pub visible: &'static str,
    /// Near-IR region label
    pub near_ir: &'static str,

    // Color names (for visible spectrum)
    /// Blue region
    pub blue: &'static str,
    /// Green region
    pub green: &'static str,
    /// Red region
    pub red: &'static str,

    // Warnings
    /// UV + thermal combined warning
    pub uv_thermal_hazard: &'static str,
    /// UV exposure warning
    pub uv_exposure_risk: &'static str,
    /// High thermal output warning
    pub high_thermal: &'static str,

    // Units
    /// Watts per nanometer
    pub watts_per_nm: &'static str,
    /// Relative units
    pub relative: &'static str,

    // TM-30 metrics
    /// Fidelity index label (default: "Rf")
    pub rf_label: &'static str,
    /// Gamut index label (default: "Rg")
    pub rg_label: &'static str,
    /// Reference label
    pub reference: &'static str,
    /// Test label
    pub test: &'static str,

    // Spectral metrics
    /// Energy distribution title
    pub energy_distribution: &'static str,
    /// UV percentage label
    pub uv_percent: &'static str,
    /// Visible percentage label
    pub visible_percent: &'static str,
    /// IR percentage label
    pub ir_percent: &'static str,
    /// Red to far-red ratio label
    pub r_fr_ratio: &'static str,
}

impl Default for SpectralLabels {
    fn default() -> Self {
        Self::english()
    }
}

impl SpectralLabels {
    /// English labels (default)
    pub const fn english() -> Self {
        Self {
            wavelength_axis: "Wavelength (nm)",
            relative_power_axis: "Relative Power",
            spd_title: "Spectral Power Distribution",
            cvg_title: "Color Vector Graphic",
            hue_title: "Hue Bin Fidelity",
            uv_a: "UV-A",
            visible: "Visible",
            near_ir: "Near-IR",
            blue: "Blue",
            green: "Green",
            red: "Red",
            uv_thermal_hazard: "UV + Thermal hazard",
            uv_exposure_risk: "UV exposure risk",
            high_thermal: "High thermal output",
            watts_per_nm: "W/nm",
            relative: "Relative",
            rf_label: "Rf",
            rg_label: "Rg",
            reference: "Reference",
            test: "Test",
            energy_distribution: "Energy Distribution",
            uv_percent: "UV",
            visible_percent: "Visible",
            ir_percent: "IR",
            r_fr_ratio: "R:FR Ratio",
        }
    }

    /// German labels
    pub const fn german() -> Self {
        Self {
            wavelength_axis: "Wellenlänge (nm)",
            relative_power_axis: "Relative Leistung",
            spd_title: "Spektrale Leistungsverteilung",
            cvg_title: "Farbvektorgrafik",
            hue_title: "Farbton-Wiedergabe",
            uv_a: "UV-A",
            visible: "Sichtbar",
            near_ir: "Nah-IR",
            blue: "Blau",
            green: "Grün",
            red: "Rot",
            uv_thermal_hazard: "UV + Wärme-Gefahr",
            uv_exposure_risk: "UV-Expositionsrisiko",
            high_thermal: "Hohe Wärmeabgabe",
            watts_per_nm: "W/nm",
            relative: "Relativ",
            rf_label: "Rf",
            rg_label: "Rg",
            reference: "Referenz",
            test: "Test",
            energy_distribution: "Energieverteilung",
            uv_percent: "UV",
            visible_percent: "Sichtbar",
            ir_percent: "IR",
            r_fr_ratio: "R:FR Verhältnis",
        }
    }

    /// French labels
    pub const fn french() -> Self {
        Self {
            wavelength_axis: "Longueur d'onde (nm)",
            relative_power_axis: "Puissance relative",
            spd_title: "Distribution spectrale de puissance",
            cvg_title: "Graphique vectoriel couleur",
            hue_title: "Fidélité de teinte",
            uv_a: "UV-A",
            visible: "Visible",
            near_ir: "Proche-IR",
            blue: "Bleu",
            green: "Vert",
            red: "Rouge",
            uv_thermal_hazard: "Risque UV + thermique",
            uv_exposure_risk: "Risque d'exposition UV",
            high_thermal: "Émission thermique élevée",
            watts_per_nm: "W/nm",
            relative: "Relatif",
            rf_label: "Rf",
            rg_label: "Rg",
            reference: "Référence",
            test: "Test",
            energy_distribution: "Distribution d'énergie",
            uv_percent: "UV",
            visible_percent: "Visible",
            ir_percent: "IR",
            r_fr_ratio: "Ratio R:FR",
        }
    }

    /// Chinese (Simplified) labels
    pub const fn chinese() -> Self {
        Self {
            wavelength_axis: "波长 (nm)",
            relative_power_axis: "相对功率",
            spd_title: "光谱功率分布",
            cvg_title: "色向量图",
            hue_title: "色调保真度",
            uv_a: "UV-A",
            visible: "可见光",
            near_ir: "近红外",
            blue: "蓝",
            green: "绿",
            red: "红",
            uv_thermal_hazard: "UV + 热危害",
            uv_exposure_risk: "UV暴露风险",
            high_thermal: "高热输出",
            watts_per_nm: "W/nm",
            relative: "相对",
            rf_label: "Rf",
            rg_label: "Rg",
            reference: "参考",
            test: "测试",
            energy_distribution: "能量分布",
            uv_percent: "UV",
            visible_percent: "可见光",
            ir_percent: "红外",
            r_fr_ratio: "红光:远红光比",
        }
    }

    /// Japanese labels
    pub const fn japanese() -> Self {
        Self {
            wavelength_axis: "波長 (nm)",
            relative_power_axis: "相対パワー",
            spd_title: "分光分布",
            cvg_title: "カラーベクトルグラフィック",
            hue_title: "色相忠実度",
            uv_a: "UV-A",
            visible: "可視光",
            near_ir: "近赤外",
            blue: "青",
            green: "緑",
            red: "赤",
            uv_thermal_hazard: "UV + 熱危険",
            uv_exposure_risk: "UV曝露リスク",
            high_thermal: "高熱出力",
            watts_per_nm: "W/nm",
            relative: "相対",
            rf_label: "Rf",
            rg_label: "Rg",
            reference: "基準",
            test: "テスト",
            energy_distribution: "エネルギー分布",
            uv_percent: "UV",
            visible_percent: "可視光",
            ir_percent: "赤外",
            r_fr_ratio: "R:FR比",
        }
    }

    /// Spanish labels
    pub const fn spanish() -> Self {
        Self {
            wavelength_axis: "Longitud de onda (nm)",
            relative_power_axis: "Potencia relativa",
            spd_title: "Distribución espectral de potencia",
            cvg_title: "Gráfico de vector de color",
            hue_title: "Fidelidad de tono",
            uv_a: "UV-A",
            visible: "Visible",
            near_ir: "IR cercano",
            blue: "Azul",
            green: "Verde",
            red: "Rojo",
            uv_thermal_hazard: "Riesgo UV + térmico",
            uv_exposure_risk: "Riesgo de exposición UV",
            high_thermal: "Alta emisión térmica",
            watts_per_nm: "W/nm",
            relative: "Relativo",
            rf_label: "Rf",
            rg_label: "Rg",
            reference: "Referencia",
            test: "Prueba",
            energy_distribution: "Distribución de energía",
            uv_percent: "UV",
            visible_percent: "Visible",
            ir_percent: "IR",
            r_fr_ratio: "Ratio R:FR",
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
        let labels = SpectralLabels::default();
        assert_eq!(labels.spd_title, "Spectral Power Distribution");
        assert_eq!(labels.uv_a, "UV-A");
    }

    #[test]
    fn test_language_lookup() {
        let german = SpectralLabels::for_language("de");
        assert_eq!(german.visible, "Sichtbar");

        let chinese = SpectralLabels::for_language("zh");
        assert_eq!(chinese.visible, "可见光");
    }
}
