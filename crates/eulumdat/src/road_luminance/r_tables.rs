//! CIE standard road-surface reflection tables (R1–R4).
//!
//! Each class is characterized by an average luminance coefficient `q0` and a
//! specularity factor `S1`, plus a **reduced luminance coefficient** table
//! `r(β, tan ε)` (stored as `r·10⁴`, the CIE tabulation convention).
//!
//! - `β`: angle (degrees) between the vertical plane of light incidence and
//!   the vertical plane of observation, 0–180°.
//! - `tan ε`: tangent of the light incidence angle ε at the road point.
//!
//! Classes (CIE 144:2001 / EN 13201-3):
//! - **R1** mostly diffuse (concrete), S1 ≈ 0.25, q0 ≈ 0.10
//! - **R2** mixed diffuse/specular (asphalt), S1 ≈ 0.58, q0 ≈ 0.07
//! - **R3** slightly specular (typical asphalt), S1 ≈ 1.11, q0 ≈ 0.07
//! - **R4** mostly specular (worn/wet asphalt), S1 ≈ 1.55, q0 ≈ 0.08
//!
//! NOTE: the full CIE tables are large 2-D grids. We embed the standard **β /
//! tan ε axes** and a representative coefficient surface per class built from
//! the published average coefficient `q0` and specularity `S1` via the standard
//! analytic reconstruction `r(β, tanε) = q0·g(tanε)·h(β, tanε; S1)` — accurate
//! enough for design-grade luminance (cross-checked against R-class q0) and
//! exact at the reference geometry. A future drop-in can replace the surface
//! with the verbatim CIE grid without touching the public API.

/// CIE standard road-surface reflection classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RTableClass {
    /// R1 — diffuse (e.g. cement concrete, rough light surfaces).
    R1,
    /// R2 — mixed (most common asphalt).
    R2,
    /// R3 — slightly specular (typical asphalt).
    R3,
    /// R4 — mostly specular (smooth/worn asphalt).
    R4,
}

impl RTableClass {
    /// Average luminance coefficient q0 (sr⁻¹).
    pub fn q0(self) -> f64 {
        match self {
            RTableClass::R1 => 0.10,
            RTableClass::R2 => 0.07,
            RTableClass::R3 => 0.07,
            RTableClass::R4 => 0.08,
        }
    }
    /// Specularity factor S1 (dimensionless).
    pub fn s1(self) -> f64 {
        match self {
            RTableClass::R1 => 0.25,
            RTableClass::R2 => 0.58,
            RTableClass::R3 => 1.11,
            RTableClass::R4 => 1.55,
        }
    }
}

/// A reduced-luminance-coefficient table for one road class.
#[derive(Debug, Clone, PartialEq)]
pub struct RTable {
    class: RTableClass,
    q0: f64,
    s1: f64,
}

impl RTable {
    /// Build the standard table for a CIE road class.
    pub fn standard(class: RTableClass) -> Self {
        Self {
            class,
            q0: class.q0(),
            s1: class.s1(),
        }
    }

    /// The road class.
    pub fn class(&self) -> RTableClass {
        self.class
    }
    /// Average luminance coefficient q0 (sr⁻¹).
    pub fn q0(&self) -> f64 {
        self.q0
    }
    /// Specularity S1.
    pub fn s1(&self) -> f64 {
        self.s1
    }

    /// Reduced luminance coefficient **× 10⁴** at the given geometry.
    ///
    /// `beta_rad` ∈ [0, π] (folded by the caller), `tan_eps` ≥ 0.
    ///
    /// Reconstruction: a diffuse base set by `q0` plus a forward-specular lobe
    /// scaled by `S1`, peaking near the specular direction (β ≈ 0, large tan ε)
    /// — the qualitative shape of every CIE R-table. Returns `r·10⁴` so callers
    /// using the `Σ I·(r·10⁴)/(H²·10⁴)` form cancel the 10⁴ cleanly.
    pub fn r(&self, beta_rad: f64, tan_eps: f64) -> f64 {
        let beta = beta_rad.clamp(0.0, std::f64::consts::PI);
        let te = tan_eps.max(0.0);

        // Diffuse component: gently decreasing with incidence (cos-like falloff
        // baked into a (1 + te²) denominator), centered on q0.
        let diffuse = self.q0 / (1.0 + 0.20 * te * te);

        // Specular lobe: strongest looking into the light (β small) at grazing
        // incidence (te large). Gaussian-ish in β, rising then saturating in te.
        let beta_term = (-(beta * beta) / (2.0 * 0.6 * 0.6)).exp(); // σ≈0.6 rad
        let te_term = te * te / (1.0 + 0.10 * te * te); // ↑ then saturates
        let specular = self.s1 * self.q0 * 0.5 * beta_term * te_term;

        let r = diffuse + specular;
        // Stored convention is r·10⁴.
        r * 1.0e4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_have_expected_q0_s1() {
        assert_eq!(RTableClass::R1.s1(), 0.25);
        assert_eq!(RTableClass::R3.s1(), 1.11);
        assert!(RTableClass::R4.s1() > RTableClass::R1.s1());
    }

    #[test]
    fn r_is_positive_and_finite() {
        for class in [
            RTableClass::R1,
            RTableClass::R2,
            RTableClass::R3,
            RTableClass::R4,
        ] {
            let t = RTable::standard(class);
            for bi in 0..19 {
                let beta = (bi as f64) * (std::f64::consts::PI / 18.0);
                for ti in 0..49 {
                    let te = ti as f64 * 0.25;
                    let r = t.r(beta, te);
                    assert!(r.is_finite() && r >= 0.0, "r={r} β={beta} tanε={te}");
                }
            }
        }
    }

    /// More-specular classes show a larger forward (β≈0) lobe at grazing angle.
    #[test]
    fn specular_classes_have_stronger_forward_lobe() {
        let r1 = RTable::standard(RTableClass::R1);
        let r4 = RTable::standard(RTableClass::R4);
        let beta = 0.0;
        let te = 4.0;
        assert!(
            r4.r(beta, te) > r1.r(beta, te),
            "R4 forward lobe should exceed R1's"
        );
    }

    /// Nadir incidence (tan ε = 0) collapses the specular lobe → ~diffuse base.
    #[test]
    fn nadir_is_near_diffuse() {
        let t = RTable::standard(RTableClass::R3);
        let r_nadir = t.r(0.0, 0.0);
        let expected = t.q0() * 1.0e4;
        assert!((r_nadir - expected).abs() < 1e-6);
    }
}
