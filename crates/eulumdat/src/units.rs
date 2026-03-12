//! Unit system for photometric output (SI / Imperial).
//!
//! All internal values remain in SI (lux, meters, millimeters).
//! This module provides conversion and formatting helpers so that
//! CLI output, SVG labels, and UI displays can show either system.

/// Conversion factor: 1 foot-candle = 10.764 lux
const LUX_PER_FC: f64 = 10.764;

/// Conversion factor: 1 foot = 0.3048 meters
const M_PER_FT: f64 = 0.3048;

/// Conversion factor: 1 inch = 25.4 millimeters
const MM_PER_IN: f64 = 25.4;

/// Unit system for photometric output.
///
/// Internal data is always stored in SI units. This enum selects
/// which unit system to use when *displaying* values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnitSystem {
    /// SI / Metric: lux, meters, millimeters
    #[default]
    Metric,
    /// Imperial: foot-candles, feet, inches
    Imperial,
}

impl UnitSystem {
    // ── Illuminance ────────────────────────────────────────────

    /// Unit abbreviation for illuminance ("lx" or "fc").
    pub fn illuminance_label(&self) -> &'static str {
        match self {
            Self::Metric => "lx",
            Self::Imperial => "fc",
        }
    }

    /// Convert a value from lux to the selected unit.
    pub fn convert_lux(&self, lux: f64) -> f64 {
        match self {
            Self::Metric => lux,
            Self::Imperial => lux / LUX_PER_FC,
        }
    }

    // ── Distance (meters) ──────────────────────────────────────

    /// Unit abbreviation for macro distances ("m" or "ft").
    pub fn distance_label(&self) -> &'static str {
        match self {
            Self::Metric => "m",
            Self::Imperial => "ft",
        }
    }

    /// Convert a value from meters to the selected unit.
    pub fn convert_meters(&self, m: f64) -> f64 {
        match self {
            Self::Metric => m,
            Self::Imperial => m / M_PER_FT,
        }
    }

    /// Convert a value from the selected unit back to meters.
    pub fn to_meters(&self, val: f64) -> f64 {
        match self {
            Self::Metric => val,
            Self::Imperial => val * M_PER_FT,
        }
    }

    // ── Dimensions (millimeters) ───────────────────────────────

    /// Unit abbreviation for small dimensions ("mm" or "in").
    pub fn dimension_label(&self) -> &'static str {
        match self {
            Self::Metric => "mm",
            Self::Imperial => "in",
        }
    }

    /// Convert a value from millimeters to the selected unit.
    pub fn convert_mm(&self, mm: f64) -> f64 {
        match self {
            Self::Metric => mm,
            Self::Imperial => mm / MM_PER_IN,
        }
    }

    /// Convert a value from the selected unit back to millimeters.
    pub fn to_mm(&self, val: f64) -> f64 {
        match self {
            Self::Metric => val,
            Self::Imperial => val * MM_PER_IN,
        }
    }

    // ── Format helpers ─────────────────────────────────────────

    /// Format an illuminance value (input in lux).
    pub fn format_lux(&self, lux: f64) -> String {
        let v = self.convert_lux(lux);
        format!("{v:.1} {}", self.illuminance_label())
    }

    /// Format a macro distance (input in meters).
    pub fn format_distance(&self, m: f64) -> String {
        let v = self.convert_meters(m);
        format!("{v:.1} {}", self.distance_label())
    }

    /// Format a small dimension (input in millimeters).
    pub fn format_dimension(&self, mm: f64) -> String {
        match self {
            Self::Metric => format!("{mm:.0} mm"),
            Self::Imperial => format!("{:.1} in", mm / MM_PER_IN),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_is_identity() {
        let u = UnitSystem::Metric;
        assert!((u.convert_lux(100.0) - 100.0).abs() < 1e-9);
        assert!((u.convert_meters(5.0) - 5.0).abs() < 1e-9);
        assert!((u.convert_mm(200.0) - 200.0).abs() < 1e-9);
    }

    #[test]
    fn imperial_conversions() {
        let u = UnitSystem::Imperial;
        // 10.764 lux = 1 fc
        assert!((u.convert_lux(10.764) - 1.0).abs() < 1e-3);
        // 0.3048 m = 1 ft
        assert!((u.convert_meters(0.3048) - 1.0).abs() < 1e-6);
        // 25.4 mm = 1 in
        assert!((u.convert_mm(25.4) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn labels() {
        assert_eq!(UnitSystem::Metric.illuminance_label(), "lx");
        assert_eq!(UnitSystem::Imperial.illuminance_label(), "fc");
        assert_eq!(UnitSystem::Metric.distance_label(), "m");
        assert_eq!(UnitSystem::Imperial.distance_label(), "ft");
        assert_eq!(UnitSystem::Metric.dimension_label(), "mm");
        assert_eq!(UnitSystem::Imperial.dimension_label(), "in");
    }

    #[test]
    fn format_helpers() {
        let m = UnitSystem::Metric;
        assert_eq!(m.format_lux(100.0), "100.0 lx");
        assert_eq!(m.format_distance(5.0), "5.0 m");
        assert_eq!(m.format_dimension(200.0), "200 mm");

        let i = UnitSystem::Imperial;
        assert_eq!(i.format_lux(10.764), "1.0 fc");
        assert_eq!(i.format_distance(0.3048), "1.0 ft");
        assert_eq!(i.format_dimension(25.4), "1.0 in");
    }

    #[test]
    fn default_is_metric() {
        assert_eq!(UnitSystem::default(), UnitSystem::Metric);
    }
}
