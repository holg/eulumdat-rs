//! Spherical goniophotometer detector for photon collection and binning.

use nalgebra::Vector3;
use std::f64::consts::PI;

/// Spherical detector that collects escaping photons and bins them by direction.
///
/// Uses CIE photometric coordinates:
/// - C-angle: azimuth (0-360), C0 = +X, C90 = +Y
/// - Gamma: from nadir (0-180), gamma=0 = -Z (down), gamma=180 = +Z (up)
#[derive(Debug, Clone)]
pub struct Detector {
    /// Accumulated energy per bin: `bins[c_index][g_index]`.
    bins: Vec<Vec<f64>>,
    /// Photon count per bin: `counts[c_index][g_index]`.
    counts: Vec<Vec<u64>>,
    /// C-angle resolution in degrees.
    c_resolution_deg: f64,
    /// Gamma resolution in degrees.
    g_resolution_deg: f64,
    /// Number of C bins (360 / c_resolution).
    num_c: usize,
    /// Number of gamma bins (180 / g_resolution + 1).
    num_g: usize,
    /// Total accumulated energy (sum of all bins).
    total_energy: f64,
}

impl Detector {
    /// Create a new detector with the given angular resolution.
    pub fn new(c_resolution_deg: f64, g_resolution_deg: f64) -> Self {
        let num_c = (360.0 / c_resolution_deg).round() as usize;
        let num_g = (180.0 / g_resolution_deg).round() as usize + 1;
        Self {
            bins: vec![vec![0.0; num_g]; num_c],
            counts: vec![vec![0; num_g]; num_c],
            c_resolution_deg,
            g_resolution_deg,
            num_c,
            num_g,
            total_energy: 0.0,
        }
    }

    /// Record an escaping photon by its world-space direction and energy.
    pub fn record(&mut self, direction: &Vector3<f64>, energy: f64) {
        let (c, g) = direction_to_cg(direction);
        let ci = ((c / self.c_resolution_deg).floor() as usize).min(self.num_c - 1);
        let gi = ((g / self.g_resolution_deg).round() as usize).min(self.num_g - 1);
        self.bins[ci][gi] += energy;
        self.counts[ci][gi] += 1;
        self.total_energy += energy;
    }

    /// Convert accumulated bins to candela values.
    ///
    /// `cd = (energy_in_bin / solid_angle_of_bin) * (source_flux / total_energy)`
    pub fn to_candela(&self, source_flux_lm: f64) -> Vec<Vec<f64>> {
        if self.total_energy <= 0.0 {
            return self.bins.clone();
        }

        let dc_rad = self.c_resolution_deg.to_radians();
        let dg_rad = self.g_resolution_deg.to_radians();
        let flux_per_energy = source_flux_lm / self.total_energy;

        let mut candela = vec![vec![0.0; self.num_g]; self.num_c];

        for ci in 0..self.num_c {
            for gi in 0..self.num_g {
                let g_center_rad = (gi as f64 * self.g_resolution_deg).to_radians();
                let solid_angle = solid_angle_for_bin(g_center_rad, dg_rad, dc_rad);

                if solid_angle > 0.0 {
                    // Flux in bin = energy * flux_per_energy
                    // Intensity (cd) = flux / solid_angle / (4*pi) ...
                    // Actually: cd = luminous_flux_in_bin / solid_angle
                    let flux_in_bin = self.bins[ci][gi] * flux_per_energy;
                    candela[ci][gi] = flux_in_bin / solid_angle;
                }
            }
        }

        candela
    }

    /// Total detected flux in lumens (for energy conservation validation).
    pub fn total_flux(&self, source_flux_lm: f64) -> f64 {
        if self.total_energy <= 0.0 {
            return 0.0;
        }
        self.total_energy * (source_flux_lm / self.total_energy)
    }

    /// Total accumulated energy.
    pub fn total_energy(&self) -> f64 {
        self.total_energy
    }

    /// Number of C bins.
    pub fn num_c(&self) -> usize {
        self.num_c
    }

    /// Number of gamma bins.
    pub fn num_g(&self) -> usize {
        self.num_g
    }

    /// C resolution in degrees.
    pub fn c_resolution_deg(&self) -> f64 {
        self.c_resolution_deg
    }

    /// Gamma resolution in degrees.
    pub fn g_resolution_deg(&self) -> f64 {
        self.g_resolution_deg
    }

    /// Access raw bins (energy).
    pub fn bins(&self) -> &Vec<Vec<f64>> {
        &self.bins
    }

    /// Access raw counts.
    pub fn counts(&self) -> &Vec<Vec<u64>> {
        &self.counts
    }

    /// Merge another detector's data into this one (for parallel accumulation).
    pub fn merge(&mut self, other: &Detector) {
        assert_eq!(self.num_c, other.num_c);
        assert_eq!(self.num_g, other.num_g);
        for ci in 0..self.num_c {
            for gi in 0..self.num_g {
                self.bins[ci][gi] += other.bins[ci][gi];
                self.counts[ci][gi] += other.counts[ci][gi];
            }
        }
        self.total_energy += other.total_energy;
    }

    /// Sample the candela value at a specific (C, gamma) angle pair.
    ///
    /// Uses bilinear interpolation from the fine-resolution bins.
    /// This allows extracting cd values at the source LDT's exact
    /// (non-uniform) C-plane angles.
    pub fn candela_at(&self, c_deg: f64, g_deg: f64, source_flux_lm: f64) -> f64 {
        if self.total_energy <= 0.0 {
            return 0.0;
        }

        let flux_per_energy = source_flux_lm / self.total_energy;
        let dc_rad = self.c_resolution_deg.to_radians();
        let dg_rad = self.g_resolution_deg.to_radians();

        // Find surrounding bin indices
        let c_norm = c_deg.rem_euclid(360.0);
        let ci_f = c_norm / self.c_resolution_deg;
        let gi_f = g_deg / self.g_resolution_deg;

        let ci0 = (ci_f.floor() as usize).min(self.num_c - 1);
        let ci1 = (ci0 + 1) % self.num_c;
        let gi0 = (gi_f.floor() as usize).min(self.num_g - 1);
        let gi1 = (gi0 + 1).min(self.num_g - 1);

        let cf = ci_f - ci_f.floor(); // fractional part
        let gf = gi_f - gi_f.floor();

        // Get candela for the 4 surrounding bins
        let cd = |ci: usize, gi: usize| -> f64 {
            let g_center_rad = (gi as f64 * self.g_resolution_deg).to_radians();
            let sa = solid_angle_for_bin(g_center_rad, dg_rad, dc_rad);
            if sa > 0.0 {
                self.bins[ci][gi] * flux_per_energy / sa
            } else {
                0.0
            }
        };

        // Bilinear interpolation
        let v00 = cd(ci0, gi0);
        let v10 = cd(ci1, gi0);
        let v01 = cd(ci0, gi1);
        let v11 = cd(ci1, gi1);

        let v0 = v00 * (1.0 - cf) + v10 * cf;
        let v1 = v01 * (1.0 - cf) + v11 * cf;
        v0 * (1.0 - gf) + v1 * gf
    }

    /// Resample to a coarser resolution for export.
    pub fn resample(&self, c_step_deg: f64, g_step_deg: f64) -> Detector {
        let mut resampled = Detector::new(c_step_deg, g_step_deg);

        // Map each bin in self to the corresponding bin in resampled
        for ci in 0..self.num_c {
            let c_angle = ci as f64 * self.c_resolution_deg;
            let new_ci = ((c_angle / c_step_deg).floor() as usize).min(resampled.num_c - 1);

            for gi in 0..self.num_g {
                let g_angle = gi as f64 * self.g_resolution_deg;
                let new_gi = ((g_angle / g_step_deg).round() as usize).min(resampled.num_g - 1);

                resampled.bins[new_ci][new_gi] += self.bins[ci][gi];
                resampled.counts[new_ci][new_gi] += self.counts[ci][gi];
            }
        }
        resampled.total_energy = self.total_energy;

        resampled
    }
}

// ---------------------------------------------------------------------------
// Coordinate conversion
// ---------------------------------------------------------------------------

/// Convert a world-space direction to CIE photometric coordinates (C, gamma).
///
/// Convention:
/// - gamma = 0 → nadir (-Z, straight down)
/// - gamma = 90 → horizontal
/// - gamma = 180 → zenith (+Z, straight up)
/// - C = 0 → +X, C = 90 → +Y
fn direction_to_cg(dir: &Vector3<f64>) -> (f64, f64) {
    // gamma: angle from -Z axis
    let gamma = (-dir.z).acos().to_degrees();

    // C: azimuth from +X towards +Y
    let c = dir.y.atan2(dir.x).to_degrees();
    let c = if c < 0.0 { c + 360.0 } else { c };

    (c, gamma)
}

/// Solid angle of a detector bin centered at gamma, with angular widths dg and dc.
///
/// Uses the exact integral: omega = dc * |cos(g - dg/2) - cos(g + dg/2)|
/// This is correct for all bins including the poles.
fn solid_angle_for_bin(g_center_rad: f64, dg_rad: f64, dc_rad: f64) -> f64 {
    let g_lo = (g_center_rad - dg_rad / 2.0).max(0.0);
    let g_hi = (g_center_rad + dg_rad / 2.0).min(PI);
    let d_cos = (g_lo.cos() - g_hi.cos()).abs();
    dc_rad * d_cos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_to_cg_nadir() {
        let (c, g) = direction_to_cg(&Vector3::new(0.0, 0.0, -1.0));
        assert!(g.abs() < 1.0, "Nadir should be gamma ~0, got {g}");
        let _ = c; // C is undefined at poles
    }

    #[test]
    fn direction_to_cg_zenith() {
        let (_, g) = direction_to_cg(&Vector3::new(0.0, 0.0, 1.0));
        assert!((g - 180.0).abs() < 1.0, "Zenith should be gamma ~180, got {g}");
    }

    #[test]
    fn direction_to_cg_horizontal_front() {
        let (c, g) = direction_to_cg(&Vector3::new(1.0, 0.0, 0.0));
        assert!((g - 90.0).abs() < 1.0, "Horizontal should be gamma ~90, got {g}");
        assert!(c.abs() < 1.0 || (c - 360.0).abs() < 1.0, "C0 = +X, got {c}");
    }

    #[test]
    fn direction_to_cg_horizontal_right() {
        let (c, g) = direction_to_cg(&Vector3::new(0.0, 1.0, 0.0));
        assert!((g - 90.0).abs() < 1.0);
        assert!((c - 90.0).abs() < 1.0, "C90 = +Y, got {c}");
    }

    #[test]
    fn detector_records_and_merges() {
        let mut d1 = Detector::new(10.0, 5.0);
        let mut d2 = Detector::new(10.0, 5.0);

        d1.record(&Vector3::new(0.0, 0.0, -1.0), 1.0);
        d2.record(&Vector3::new(0.0, 0.0, -1.0), 2.0);

        d1.merge(&d2);
        assert!((d1.total_energy - 3.0).abs() < 1e-10);
    }

    #[test]
    fn solid_angle_equator_larger_than_pole() {
        let dg = 1.0f64.to_radians();
        let dc = 1.0f64.to_radians();
        let sa_equator = solid_angle_for_bin(90.0f64.to_radians(), dg, dc);
        let sa_pole = solid_angle_for_bin(0.0, dg, dc);
        assert!(sa_equator > sa_pole);
    }
}
