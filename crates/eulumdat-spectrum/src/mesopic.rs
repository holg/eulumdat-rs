//! CIE 191:2010 mesopic photometry.
//!
//! Between roughly 0.005 and 5 cd/m² the visual system transitions from
//! photopic to scotopic. Road-lighting luminances (0.3–2 cd/m²) sit squarely
//! in this range, so a photopic-only road calculation misstates what a driver
//! actually perceives — and the error is spectrum-dependent (a high-S/P white
//! source appears brighter at night than its photopic luminance implies).
//!
//! The CIE 191 system defines a mesopic luminance `L_mes` via a coefficient
//! `m(L_mes)` blending photopic and scotopic luminances:
//!
//! ```text
//! L_mes = M(m) · [ m · L_p + (1 − m) · (V′/V)_ref · L_s ]
//! ```
//!
//! where `M(m)` normalises so that `V_mes(555 nm) = 1`. The coefficient `m`
//! itself depends on `L_mes`, so the system is solved iteratively.

/// Lower bound of the mesopic range (cd/m²); below this, pure scotopic.
pub const MESOPIC_LOW: f64 = 0.005;
/// Upper bound of the mesopic range (cd/m²); above this, pure photopic.
pub const MESOPIC_HIGH: f64 = 5.0;

// CIE 191 normalisation: the scotopic luminance is expressed relative to the
// photopic via the 555 nm anchor, giving the constant 683/1700 built into S/P.
// Here we work directly with the photopic luminance `l_p` and the source S/P
// ratio, which is how road-lighting practice states the problem.

/// Mesopic luminance from photopic luminance and the source S/P ratio.
///
/// Solves the CIE 191 fixed-point iteration. Outside the mesopic range this
/// returns the photopic (bright) or scotopic-adjusted (dark) limit.
///
/// * `l_photopic` — photopic luminance of the adaptation field (cd/m²).
/// * `sp` — scotopic/photopic ratio of the source (see [`crate::metrics::sp_ratio`]).
pub fn mesopic_luminance(l_photopic: f64, sp: f64) -> f64 {
    if l_photopic <= 0.0 {
        return 0.0;
    }
    // Scotopic luminance implied by the source S/P at this photopic level.
    let l_scotopic = sp * l_photopic;

    if l_photopic >= MESOPIC_HIGH {
        return l_photopic;
    }

    // CIE 191 fixed point. m is a function of L_mes; iterate to convergence.
    // m(L_mes) = (log10 L_mes − log10 L_low) / (log10 L_high − log10 L_low),
    // clamped to [0,1]. The blended luminance uses m on photopic and (1−m) on
    // scotopic, with the 555 nm normalisation folded into the S/P convention.
    let log_lo = MESOPIC_LOW.log10();
    let log_hi = MESOPIC_HIGH.log10();

    let mut l_mes = l_photopic; // start from photopic
    for _ in 0..50 {
        let m = ((l_mes.log10() - log_lo) / (log_hi - log_lo)).clamp(0.0, 1.0);
        let new = m * l_photopic + (1.0 - m) * l_scotopic;
        // Normalisation M(m): CIE 191 keeps V_mes(555)=1, which for the
        // S/P-based formulation reduces to dividing by [m + (1−m)] = 1, so the
        // blend above is already normalised at the 555 nm anchor.
        if (new - l_mes).abs() < 1e-9 {
            l_mes = new;
            break;
        }
        l_mes = new;
    }
    l_mes
}

/// The CIE 191 mesopic weighting coefficient `m` at a given mesopic luminance
/// (0 = fully scotopic, 1 = fully photopic).
pub fn mesopic_coefficient(l_mes: f64) -> f64 {
    if l_mes <= MESOPIC_LOW {
        return 0.0;
    }
    if l_mes >= MESOPIC_HIGH {
        return 1.0;
    }
    let log_lo = MESOPIC_LOW.log10();
    let log_hi = MESOPIC_HIGH.log10();
    ((l_mes.log10() - log_lo) / (log_hi - log_lo)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bright_field_is_pure_photopic() {
        // Above 5 cd/m² the mesopic luminance equals the photopic one.
        let l = mesopic_luminance(10.0, 2.0);
        assert!((l - 10.0).abs() < 1e-9);
    }

    #[test]
    fn high_sp_source_brighter_at_night() {
        // At a mesopic road level, a high-S/P (bluish) source yields a higher
        // mesopic luminance than a low-S/P (amber) source at the same photopic
        // luminance — the whole point of CIE 191 for road lighting.
        let l_photopic = 1.0; // cd/m², typical M-class road
        let cool = mesopic_luminance(l_photopic, 2.2);
        let warm = mesopic_luminance(l_photopic, 0.6);
        assert!(cool > warm);
        assert!(cool > l_photopic, "S/P>1 boosts mesopic luminance");
        assert!(warm < l_photopic, "S/P<1 depresses mesopic luminance");
    }

    #[test]
    fn sp_unity_is_neutral() {
        // S/P == 1 → mesopic luminance == photopic luminance at any level.
        let l = mesopic_luminance(0.5, 1.0);
        assert!((l - 0.5).abs() < 1e-6);
    }

    #[test]
    fn coefficient_monotone() {
        assert!(mesopic_coefficient(0.01) < mesopic_coefficient(1.0));
        assert_eq!(mesopic_coefficient(0.001), 0.0);
        assert_eq!(mesopic_coefficient(10.0), 1.0);
    }
}
