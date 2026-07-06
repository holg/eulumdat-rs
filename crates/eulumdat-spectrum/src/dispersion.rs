//! Sellmeier index-of-refraction dispersion, `n(λ)`.
//!
//! The Sellmeier equation gives the refractive index as a function of
//! wavelength for transparent optical media:
//!
//! ```text
//! n²(λ) = 1 + Σ_i  B_i · λ² / (λ² − C_i)
//! ```
//!
//! with λ in **micrometres** and C_i in µm². A material's covers therefore
//! bend blue light more than red — real chromatic behaviour that a scalar IOR
//! cannot reproduce (prismatic edges, colour-fringing of clear optics).

/// Sellmeier coefficients (B and C terms, λ in µm).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sellmeier {
    /// B coefficients (dimensionless).
    pub b: [f64; 3],
    /// C coefficients (µm²).
    pub c: [f64; 3],
}

impl Sellmeier {
    /// Refractive index at wavelength `wl_nm`.
    pub fn n(&self, wl_nm: f64) -> f64 {
        let l2 = (wl_nm / 1000.0).powi(2); // µm²
        let mut n2 = 1.0;
        for i in 0..3 {
            let denom = l2 - self.c[i];
            if denom.abs() > 1e-12 {
                n2 += self.b[i] * l2 / denom;
            }
        }
        n2.max(1.0).sqrt()
    }

    /// PMMA (poly(methyl methacrylate)), Sultanova et al. 2009.
    pub const PMMA: Sellmeier = Sellmeier {
        b: [0.99654, 0.18964, 0.00411],
        c: [0.00787, 0.02191, 3.85727],
    };

    /// Borosilicate crown glass BK7 (Schott).
    pub const BK7: Sellmeier = Sellmeier {
        b: [1.03961212, 0.231792344, 1.01046945],
        c: [0.00600069867, 0.0200179144, 103.560653],
    };

    /// Polycarbonate, Sultanova et al. 2009 (strong dispersion).
    pub const POLYCARBONATE: Sellmeier = Sellmeier {
        b: [1.4182, 0.0, 0.0],
        c: [0.021304, 0.0, 0.0],
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn pmma_is_dispersive_blue_higher_than_red() {
        let n_blue = Sellmeier::PMMA.n(450.0);
        let n_red = Sellmeier::PMMA.n(650.0);
        assert!(n_blue > n_red, "blue should refract more than red");
        // PMMA n_d ≈ 1.49 at 589 nm.
        assert_relative_eq!(Sellmeier::PMMA.n(589.0), 1.492, epsilon = 0.006);
    }

    #[test]
    fn bk7_matches_catalog_nd() {
        // BK7 n_d = 1.5168 at 587.6 nm.
        assert_relative_eq!(Sellmeier::BK7.n(587.6), 1.5168, epsilon = 0.001);
    }

    #[test]
    fn polycarbonate_higher_index_than_pmma() {
        assert!(Sellmeier::POLYCARBONATE.n(589.0) > Sellmeier::PMMA.n(589.0));
    }
}
