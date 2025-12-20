//! PhotometricData trait - the abstraction layer for photometric data sources.
//!
//! This trait allows the photometric module to work with any data source
//! (LDT, IES, GLDF, etc.) without knowing the concrete type.
//!
//! When extracting to a standalone `bevy_photometry` crate, this file
//! can be copied as-is - it has no dependencies on `eulumdat`.

use std::fmt::Debug;

/// Trait for photometric data sources (LDT, IES, GLDF, etc.)
///
/// Implement this trait for your data type to use it with `PhotometricLight`.
///
/// # Example
///
/// ```ignore
/// use eulumdat_bevy::photometric::PhotometricData;
///
/// struct MyLightData {
///     intensities: Vec<Vec<f64>>,
///     // ...
/// }
///
/// impl PhotometricData for MyLightData {
///     fn sample(&self, c_angle: f64, g_angle: f64) -> f64 {
///         // Return intensity at the given angles
///         self.intensities[c_index][g_index]
///     }
///     // ... implement other methods
/// }
/// ```
pub trait PhotometricData: Send + Sync + Clone + Debug + 'static {
    /// Sample intensity at C-plane and gamma angles.
    ///
    /// # Arguments
    /// * `c_angle` - C-plane angle in degrees (0-360, 0° = front, 90° = right)
    /// * `g_angle` - Gamma angle in degrees (0-180, 0° = nadir/down, 90° = horizontal, 180° = zenith/up)
    ///
    /// # Returns
    /// Intensity in cd/klm (candelas per kilolumen)
    fn sample(&self, c_angle: f64, g_angle: f64) -> f64;

    /// Maximum intensity value across all angles.
    fn max_intensity(&self) -> f64;

    /// Total luminous flux in lumens.
    fn total_flux(&self) -> f64;

    /// Light output ratio (0.0 - 1.0).
    ///
    /// The ratio of light emitted by the luminaire to the total lamp flux.
    fn light_output_ratio(&self) -> f64;

    /// Downward flux fraction (0.0 - 1.0).
    ///
    /// The fraction of light emitted below the horizontal plane.
    /// - 1.0 = pure downlight
    /// - 0.0 = pure uplight
    /// - 0.5 = equal up/down
    fn downward_fraction(&self) -> f64;

    /// Physical dimensions (width, length, height) in meters.
    ///
    /// For cylindrical luminaires, width may be 0 and length is the diameter.
    fn dimensions(&self) -> (f32, f32, f32);

    /// Color temperature in Kelvin (None if unknown).
    ///
    /// Common values: 2700K (warm), 4000K (neutral), 6500K (cool)
    fn color_temperature(&self) -> Option<f32>;

    /// Color Rendering Index 0-100 (None if unknown).
    ///
    /// Higher values indicate better color rendering:
    /// - 90-100: Excellent
    /// - 80-89: Good
    /// - 70-79: Fair
    /// - <70: Poor
    fn cri(&self) -> Option<f32>;

    /// Beam angle in radians (half-angle from nadir to 50% intensity).
    ///
    /// This is the IES definition: angle where intensity drops to 50% of maximum.
    fn beam_angle(&self) -> f64;

    /// Whether the luminaire is cylindrical (width ≈ 0, length = diameter).
    fn is_cylindrical(&self) -> bool {
        let (w, _, _) = self.dimensions();
        w < 0.01
    }

    /// Upward flux fraction (1.0 - downward_fraction).
    fn upward_fraction(&self) -> f64 {
        1.0 - self.downward_fraction()
    }
}
