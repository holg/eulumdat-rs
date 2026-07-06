//! Spectral power distribution and importance sampler.
//!
//! A [`Spd`] is a set of (wavelength, value) samples on an arbitrary — possibly
//! non-uniform — grid. All integration is trapezoidal so irregular grids
//! (a common quirk of vendor SPD exports) integrate correctly without
//! resampling. Values are *relative radiant power*; absolute scaling is applied
//! at the source (luminous flux) and never baked into the curve.

/// A spectral power distribution on an arbitrary wavelength grid (nm).
///
/// Invariant: `wavelengths` is strictly ascending and the same length as
/// `values`. Use [`Spd::new`] which enforces this (duplicate wavelengths are
/// collapsed by averaging, then sorted).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Spd {
    wavelengths: Vec<f64>,
    values: Vec<f64>,
}

impl Spd {
    /// Build an SPD from parallel wavelength/value slices.
    ///
    /// Points are sorted ascending, duplicate wavelengths are averaged, and
    /// negative values are clamped to zero (a measured SPD is non-negative;
    /// small negatives are sensor noise). Panics only on length mismatch.
    pub fn new(wavelengths: &[f64], values: &[f64]) -> Self {
        assert_eq!(
            wavelengths.len(),
            values.len(),
            "wavelengths and values must be the same length"
        );
        let mut pairs: Vec<(f64, f64)> = wavelengths
            .iter()
            .zip(values.iter())
            .map(|(&w, &v)| (w, v.max(0.0)))
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Collapse duplicate wavelengths by averaging.
        let mut w = Vec::with_capacity(pairs.len());
        let mut vals = Vec::with_capacity(pairs.len());
        let mut i = 0;
        while i < pairs.len() {
            let wl = pairs[i].0;
            let mut sum = 0.0;
            let mut n = 0;
            while i < pairs.len() && (pairs[i].0 - wl).abs() < 1e-9 {
                sum += pairs[i].1;
                n += 1;
                i += 1;
            }
            w.push(wl);
            vals.push(sum / n as f64);
        }
        Self {
            wavelengths: w,
            values: vals,
        }
    }

    /// Build a uniform-grid SPD from a start wavelength, step and values.
    pub fn uniform(start_nm: f64, step_nm: f64, values: &[f64]) -> Self {
        let wl: Vec<f64> = (0..values.len())
            .map(|i| start_nm + i as f64 * step_nm)
            .collect();
        Self::new(&wl, values)
    }

    /// Wavelength grid (ascending, nm).
    pub fn wavelengths(&self) -> &[f64] {
        &self.wavelengths
    }

    /// Spectral values, parallel to [`wavelengths`](Self::wavelengths).
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.wavelengths.len()
    }

    /// True if the SPD has no samples.
    pub fn is_empty(&self) -> bool {
        self.wavelengths.is_empty()
    }

    /// Linear interpolation at an arbitrary wavelength (0 outside support).
    pub fn value_at(&self, wl_nm: f64) -> f64 {
        let w = &self.wavelengths;
        if w.is_empty() || wl_nm < w[0] || wl_nm > w[w.len() - 1] {
            return 0.0;
        }
        match w.binary_search_by(|p| p.partial_cmp(&wl_nm).unwrap()) {
            Ok(i) => self.values[i],
            Err(i) => {
                // i is the insertion point; interpolate between i-1 and i.
                let i1 = i.min(w.len() - 1);
                let i0 = i1 - 1;
                let t = (wl_nm - w[i0]) / (w[i1] - w[i0]);
                self.values[i0] * (1.0 - t) + self.values[i1] * t
            }
        }
    }

    /// Trapezoidal integral of the raw SPD over its full support.
    pub fn integral(&self) -> f64 {
        self.integral_weighted(|_| 1.0)
    }

    /// Trapezoidal integral of `SPD(λ)·weight(λ)` over the SPD grid.
    ///
    /// The weight is evaluated at each SPD sample point, so the accuracy is
    /// bounded by the SPD's own resolution — which is exactly what we want when
    /// integrating against the eye's response: the SPD is the measured quantity.
    pub fn integral_weighted(&self, weight: impl Fn(f64) -> f64) -> f64 {
        let w = &self.wavelengths;
        if w.len() < 2 {
            return 0.0;
        }
        let mut acc = 0.0;
        let mut prev = self.values[0] * weight(w[0]);
        for i in 1..w.len() {
            let cur = self.values[i] * weight(w[i]);
            acc += 0.5 * (prev + cur) * (w[i] - w[i - 1]);
            prev = cur;
        }
        acc
    }

    /// A copy scaled so its raw integral equals 1.0 (no-op on an empty SPD).
    pub fn normalized(&self) -> Spd {
        let total = self.integral();
        if total <= 0.0 {
            return self.clone();
        }
        Spd {
            wavelengths: self.wavelengths.clone(),
            values: self.values.iter().map(|v| v / total).collect(),
        }
    }
}

/// Importance sampler over an SPD's radiant power.
///
/// Built once per source. [`sample`](Self::sample) inverts the cumulative
/// radiant-power distribution to draw a wavelength ∝ SPD(λ). Piecewise-linear
/// inversion within each grid interval keeps the draw unbiased even on coarse
/// grids.
#[derive(Debug, Clone)]
pub struct SpectralSampler {
    wavelengths: Vec<f64>,
    /// Cumulative distribution at each grid point, cdf[0] = 0, cdf[last] = 1.
    cdf: Vec<f64>,
}

impl SpectralSampler {
    /// Build a sampler from an SPD. Returns a degenerate 555 nm sampler if the
    /// SPD carries no power (so a black spectrum never divides by zero).
    pub fn from_spd(spd: &Spd) -> Self {
        let w = spd.wavelengths();
        let v = spd.values();
        if w.len() < 2 || spd.integral() <= 0.0 {
            return Self {
                wavelengths: vec![555.0, 555.0],
                cdf: vec![0.0, 1.0],
            };
        }
        let mut cdf = vec![0.0; w.len()];
        let mut cum = 0.0;
        for i in 1..w.len() {
            // Trapezoid area of segment i-1..i.
            cum += 0.5 * (v[i - 1] + v[i]) * (w[i] - w[i - 1]);
            cdf[i] = cum;
        }
        for c in &mut cdf {
            *c /= cum;
        }
        Self {
            wavelengths: w.to_vec(),
            cdf,
        }
    }

    /// Draw a wavelength given a uniform random `xi` in [0, 1).
    pub fn sample(&self, xi: f64) -> f64 {
        let cdf = &self.cdf;
        let xi = xi.clamp(0.0, 1.0);
        // Find the segment whose CDF interval contains xi.
        let i = match cdf.binary_search_by(|c| c.partial_cmp(&xi).unwrap()) {
            Ok(i) => i.max(1),
            Err(i) => i.clamp(1, cdf.len() - 1),
        };
        let c0 = cdf[i - 1];
        let c1 = cdf[i];
        let w0 = self.wavelengths[i - 1];
        let w1 = self.wavelengths[i];
        if c1 <= c0 {
            return w0;
        }
        let t = (xi - c0) / (c1 - c0);
        w0 + t * (w1 - w0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn integral_of_unit_box_is_width() {
        // Flat SPD value 1 over 400..500 nm → integral 100.
        let spd = Spd::uniform(400.0, 10.0, &[1.0; 11]);
        assert_relative_eq!(spd.integral(), 100.0, epsilon = 1e-9);
    }

    #[test]
    fn duplicate_wavelengths_are_averaged() {
        let spd = Spd::new(&[500.0, 500.0, 510.0], &[2.0, 4.0, 1.0]);
        assert_eq!(spd.len(), 2);
        assert_relative_eq!(spd.value_at(500.0), 3.0, epsilon = 1e-9);
    }

    #[test]
    fn value_at_interpolates() {
        let spd = Spd::uniform(400.0, 100.0, &[0.0, 10.0]);
        assert_relative_eq!(spd.value_at(450.0), 5.0, epsilon = 1e-9);
        assert_eq!(spd.value_at(399.0), 0.0);
        assert_eq!(spd.value_at(501.0), 0.0);
    }

    #[test]
    fn sampler_mean_matches_spd_centroid() {
        // Triangular SPD peaking at 600 nm: mean wavelength should be pulled
        // toward the peak.
        let wl: Vec<f64> = (400..=700).step_by(10).map(|w| w as f64).collect();
        let vals: Vec<f64> = wl.iter().map(|&w| (300.0 - (w - 600.0).abs())).collect();
        let spd = Spd::new(&wl, &vals);
        let sampler = SpectralSampler::from_spd(&spd);

        // Deterministic stratified draw → empirical mean ≈ power-weighted mean.
        let n = 20_000;
        let mut sum = 0.0;
        for i in 0..n {
            let xi = (i as f64 + 0.5) / n as f64;
            sum += sampler.sample(xi);
        }
        let empirical = sum / n as f64;

        // Analytic power-weighted centroid.
        let num = spd.integral_weighted(|w| w);
        let centroid = num / spd.integral();
        assert_relative_eq!(empirical, centroid, epsilon = 1.0);
    }

    #[test]
    fn black_spd_sampler_is_degenerate_not_nan() {
        let spd = Spd::uniform(400.0, 10.0, &[0.0; 11]);
        let sampler = SpectralSampler::from_spd(&spd);
        assert_eq!(sampler.sample(0.5), 555.0);
    }
}
