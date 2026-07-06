//! Photometric / radiometric constants for spectral weighting.

/// Maximum luminous efficacy of radiation, photopic (lm/W at 555 nm).
pub const KM_PHOTOPIC: f64 = 683.0;

/// Maximum luminous efficacy of radiation, scotopic (lm/W at 507 nm).
pub const KM_SCOTOPIC: f64 = 1700.0;

/// Melanopic-to-photopic ratio of the D65 reference illuminant, as produced by
/// this crate's [`crate::lut`] tables (∫D65·s_mel / ∫D65·V).
///
/// Multiplying photopic lux by the melanopic DER of a source gives melanopic
/// EDI (equivalent daylight illuminance) lux, per CIE S 026. Dividing a
/// source's own mel/photopic ratio by this constant normalises the DER so that
/// D65 has DER = 1.0 by construction. The value is self-consistent with the
/// approximate melanopic action spectrum tabulated here (the exact CIE S 026
/// figure is ≈0.906 mel-lm/lm on the toolbox tables); anchoring to our own D65
/// keeps the whole pipeline internally coherent.
pub const MEL_PER_LUM_D65: f64 = 0.978_735;

/// Visible band lower/upper bound used for "blue content" and integrations (nm).
pub const VISIBLE_START_NM: f64 = 380.0;
/// Visible band upper bound (nm).
pub const VISIBLE_END_NM: f64 = 780.0;

/// Blue band bounds for dark-sky "blue content %" (nm), per common ordinances.
pub const BLUE_START_NM: f64 = 400.0;
/// Blue band upper bound (nm).
pub const BLUE_END_NM: f64 = 500.0;

/// Planck's constant (J·s).
pub const PLANCK_H: f64 = 6.626_070_15e-34;
/// Speed of light in vacuum (m/s).
pub const SPEED_OF_LIGHT: f64 = 2.997_924_58e8;
/// Avogadro constant (1/mol).
pub const AVOGADRO: f64 = 6.022_140_76e23;

/// Energy of one mole of photons at wavelength `wl_nm`, in joules per micromole.
///
/// `E = h·c/λ` per photon; ×N_A per mole; ×1e-6 for micromoles. Dividing
/// radiant watts in a band by this gives µmol·s⁻¹ (the quantum unit PPFD uses).
pub fn joules_per_micromole(wl_nm: f64) -> f64 {
    let lambda_m = wl_nm * 1e-9;
    let e_photon = PLANCK_H * SPEED_OF_LIGHT / lambda_m; // J per photon
    e_photon * AVOGADRO * 1e-6 // J per µmol
}
