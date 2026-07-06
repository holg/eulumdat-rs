//! Spectral emission and multi-channel detection for the tracer.
//!
//! The tracer stays **radiometric**: a photon's `energy` is a power-proportional
//! weight, and its `wavelength` is drawn once at emission from the source
//! spectrum. All photometric/actinic weighting happens here, at detection, so a
//! single trace yields the photopic LDT *and* scotopic, melanopic, PPFD and
//! colour-over-angle — see [`crate::spectrum::WeightedChannels`].
//!
//! When a source carries no spectrum the photon keeps the default 555 nm and
//! the photopic path is bit-for-bit what the monochromatic tracer produced.

use eulumdat_spectrum::lut;
use eulumdat_spectrum::{Spd, SpectralSampler};

/// A source spectrum plus its wavelength sampler.
///
/// Built once per source. `sample_wavelength` draws a wavelength ∝ radiant
/// power; the source's luminous flux is applied separately at normalisation so
/// the SPD only needs to be a *relative* curve.
#[derive(Debug, Clone)]
pub struct SourceSpectrum {
    spd: Spd,
    sampler: SpectralSampler,
}

impl SourceSpectrum {
    /// Build from a relative SPD.
    pub fn new(spd: Spd) -> Self {
        let sampler = SpectralSampler::from_spd(&spd);
        Self { spd, sampler }
    }

    /// Build from a colour temperature via the D-series / Planckian model.
    pub fn from_cct(cct_k: f64) -> Self {
        Self::new(eulumdat_spectrum::synth::synthesize(cct_k))
    }

    /// Build from parallel wavelength/value slices (nm, relative power).
    pub fn from_samples(wavelengths: &[f64], values: &[f64]) -> Self {
        Self::new(Spd::new(wavelengths, values))
    }

    /// Build from a core `eulumdat` spectral distribution (e.g. a loaded vendor
    /// SPD). Bridges `eulumdat::SpectralDistribution` → the spectrum crate's
    /// [`Spd`], so a measured spectrum can drive the tracer directly.
    pub fn from_eulumdat(spd: &eulumdat::atla::SpectralDistribution) -> Self {
        Self::from_samples(&spd.wavelengths, &spd.values)
    }

    /// The underlying spectral distribution.
    pub fn spd(&self) -> &Spd {
        &self.spd
    }

    /// Draw a wavelength given a uniform random value in [0, 1).
    pub fn sample_wavelength(&self, xi: f64) -> f64 {
        self.sampler.sample(xi)
    }
}

/// How the detector accumulates escaping photons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetectorMode {
    /// One scalar per direction bin (radiant/photopic weight). Default — the
    /// original monochromatic behaviour, LDT export unchanged.
    #[default]
    Photopic,
    /// Seven weighted accumulators per direction bin (X, Y, Z, scotopic,
    /// melanopic, PAR-quanta, radiant). Cheap (~7 mul-adds per detected
    /// photon) and yields colour/scotopic/melanopic/PPFD over angle.
    WeightedChannels,
}

/// The seven per-photon channel weights for a given wavelength.
///
/// `radiant` is the wavelength-independent weight (1.0 × photon energy); the
/// rest are the photon energy times the corresponding response at `wl_nm`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChannelWeights {
    /// CIE X.
    pub x: f64,
    /// CIE Y (= photopic; the LDT channel).
    pub y: f64,
    /// CIE Z.
    pub z: f64,
    /// Scotopic V′.
    pub scotopic: f64,
    /// Melanopic s_mel.
    pub melanopic: f64,
    /// PAR quanta (µmol-weight) — radiant weight × 1/(J per µmol) inside band.
    pub par: f64,
    /// Radiant (unweighted) energy.
    pub radiant: f64,
}

impl ChannelWeights {
    /// Weights for a photon of `energy` at `wl_nm`.
    pub fn for_photon(wl_nm: f64, energy: f64) -> Self {
        let par_w = if lut::par_weight(wl_nm) > 0.0 {
            1.0 / eulumdat_spectrum::constants::joules_per_micromole(wl_nm)
        } else {
            0.0
        };
        Self {
            x: energy * lut::cie_x(wl_nm),
            y: energy * lut::cie_y(wl_nm),
            z: energy * lut::cie_z(wl_nm),
            scotopic: energy * lut::scotopic(wl_nm),
            melanopic: energy * lut::melanopic(wl_nm),
            par: energy * par_w,
            radiant: energy,
        }
    }

    /// Accumulate another photon's weights.
    pub fn add(&mut self, o: &ChannelWeights) {
        self.x += o.x;
        self.y += o.y;
        self.z += o.z;
        self.scotopic += o.scotopic;
        self.melanopic += o.melanopic;
        self.par += o.par;
        self.radiant += o.radiant;
    }
}

/// Per-direction spectral channel accumulator, parallel to the scalar
/// [`crate::detector::Detector`].
///
/// Indexed `[c_index][g_index]` like the base detector, so the two share a
/// coordinate convention. `Y` is the photopic channel and matches the base
/// detector's scalar bins exactly (given identical photon streams).
#[derive(Debug, Clone)]
pub struct WeightedChannels {
    bins: Vec<Vec<ChannelWeights>>,
    num_c: usize,
    num_g: usize,
}

impl WeightedChannels {
    /// Allocate an accumulator matching a detector's bin layout.
    pub fn new(num_c: usize, num_g: usize) -> Self {
        Self {
            bins: vec![vec![ChannelWeights::default(); num_g]; num_c],
            num_c,
            num_g,
        }
    }

    /// Record a photon's channel weights into bin `(ci, gi)`.
    pub fn record(&mut self, ci: usize, gi: usize, w: &ChannelWeights) {
        self.bins[ci][gi].add(w);
    }

    /// Merge another accumulator (parallel reduction).
    pub fn merge(&mut self, other: &WeightedChannels) {
        for ci in 0..self.num_c {
            for gi in 0..self.num_g {
                let o = other.bins[ci][gi];
                self.bins[ci][gi].add(&o);
            }
        }
    }

    /// Raw channel bins.
    pub fn bins(&self) -> &Vec<Vec<ChannelWeights>> {
        &self.bins
    }

    /// Number of C bins.
    pub fn num_c(&self) -> usize {
        self.num_c
    }

    /// Number of gamma bins.
    pub fn num_g(&self) -> usize {
        self.num_g
    }

    /// Sum of a channel over all bins, selected by `pick`.
    pub fn total(&self, pick: impl Fn(&ChannelWeights) -> f64) -> f64 {
        let mut s = 0.0;
        for row in &self.bins {
            for w in row {
                s += pick(w);
            }
        }
        s
    }

    /// Colorimetry of the angularly-integrated spectrum (all bins summed),
    /// giving the luminaire's overall CCT/Duv. Returns `None` if no light was
    /// collected.
    pub fn integrated_colorimetry(&self) -> Option<eulumdat_spectrum::Colorimetry> {
        let mut agg = ChannelWeights::default();
        for row in &self.bins {
            for w in row {
                agg.add(w);
            }
        }
        colorimetry_from_xyz(agg.x, agg.y, agg.z)
    }

    /// Colorimetry of a single direction bin (for colour-over-angle plots).
    pub fn colorimetry_at(&self, ci: usize, gi: usize) -> Option<eulumdat_spectrum::Colorimetry> {
        let w = self.bins[ci][gi];
        colorimetry_from_xyz(w.x, w.y, w.z)
    }

    /// Scotopic/photopic ratio integrated over all directions.
    pub fn integrated_sp_ratio(&self) -> f64 {
        let p = self.total(|w| w.y);
        let s = self.total(|w| w.scotopic);
        if p <= 0.0 {
            return 0.0;
        }
        (eulumdat_spectrum::constants::KM_SCOTOPIC * s)
            / (eulumdat_spectrum::constants::KM_PHOTOPIC * p)
    }
}

/// Turn accumulated tristimulus into a full `Colorimetry` via the D65-anchored
/// chromaticity (CCT/Duv come from the shared spectrum crate). Builds a tiny
/// delta-SPD proxy only to reuse the locus math would be wasteful — instead we
/// go straight from X,Y,Z to chromaticity and reuse the Planckian solver.
fn colorimetry_from_xyz(x: f64, y: f64, z: f64) -> Option<eulumdat_spectrum::Colorimetry> {
    let sum = x + y + z;
    if sum <= 0.0 {
        return None;
    }
    let (cx, cy) = (x / sum, y / sum);
    Some(eulumdat_spectrum::colorimetry::from_chromaticity(cx, cy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn none_spectrum_keeps_555() {
        // A photon with no spectrum path keeps 555 nm; its Y weight ≈ energy.
        let w = ChannelWeights::for_photon(555.0, 1.0);
        assert_relative_eq!(w.y, 1.0, epsilon = 1e-6);
        assert_relative_eq!(w.radiant, 1.0, epsilon = 1e-12);
    }

    #[test]
    fn far_red_photon_has_energy_but_no_lumens() {
        // The load-bearing convention: 730 nm carries radiant energy but ~0 Y.
        let w = ChannelWeights::for_photon(730.0, 1.0);
        assert_relative_eq!(w.radiant, 1.0, epsilon = 1e-12);
        assert!(w.y < 0.01, "730 nm should be nearly invisible, Y={}", w.y);
        // 730 nm is beyond the PAR band (400–700 nm), so its quantum weight is 0.
        assert_eq!(w.par, 0.0);
        // A red photon inside the PAR band does carry quantum weight.
        let par_photon = ChannelWeights::for_photon(660.0, 1.0);
        assert!(par_photon.par > 0.0);
    }

    #[test]
    fn weighted_channels_integrated_cct() {
        // Feed a 3000 K spectrum's sampled photons; integrated CCT ≈ 3000 K.
        let spec = SourceSpectrum::from_cct(3000.0);
        let mut wc = WeightedChannels::new(1, 1);
        let n = 50_000;
        for i in 0..n {
            let xi = (i as f64 + 0.5) / n as f64;
            let wl = spec.sample_wavelength(xi);
            let w = ChannelWeights::for_photon(wl, 1.0);
            wc.record(0, 0, &w);
        }
        let col = wc.integrated_colorimetry().expect("has light");
        assert!(
            (col.cct_k - 3000.0).abs() < 60.0,
            "integrated CCT should be ~3000 K, got {}",
            col.cct_k
        );
    }
}
