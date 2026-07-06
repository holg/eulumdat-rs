//! Solar and daylight physical constants.

/// Solar constant — mean extraterrestrial irradiance at 1 AU, W/m².
/// (IES / WMO value.)
pub const SOLAR_CONSTANT_W_M2: f64 = 1367.0;

/// Mean extraterrestrial **illuminance** on a surface normal to the beam, lux.
/// Solar constant × luminous efficacy of extraterrestrial sunlight (~94 lm/W).
pub const EXTRATERRESTRIAL_ILLUMINANCE_LUX: f64 = 133_800.0;

/// Angular **radius** of the solar disk seen from Earth, radians (~0.266°).
pub const SOLAR_DISK_ANGULAR_RADIUS_RAD: f64 = 0.004_654;

/// Solid angle subtended by the solar disk, steradians (~6.8e-5 sr).
pub const SOLAR_DISK_SOLID_ANGLE_SR: f64 = 6.794e-5;

/// Luminous efficacy of daylight, lm/W — typical clear-sky values used only
/// when a caller starts from irradiance (W/m²) instead of illuminance (lux).
/// A future spectral path overrides these per band.
pub mod efficacy {
    /// Direct beam (global ~ slightly lower than diffuse), lm/W.
    pub const DIRECT_LM_PER_W: f64 = 105.0;
    /// Diffuse sky, lm/W.
    pub const DIFFUSE_LM_PER_W: f64 = 125.0;
    /// Global horizontal, lm/W.
    pub const GLOBAL_LM_PER_W: f64 = 110.0;
}
