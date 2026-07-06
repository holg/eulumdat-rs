//! Daylight sources and illuminance-plane detection.
//!
//! The tracer's native frame is photometric Type-C: **+Z is up (zenith)**,
//! **−Z is nadir (γ=0, straight down)**, C0 = +X. The `eulumdat-daylight` crate
//! works in an ENU frame (**+Y up**). [`world_to_sim`] is the single bridge
//! between them — convert once, here, never ad hoc elsewhere.
//!
//! Two sources consume the daylight physics:
//! - [`sun_source`] — a near-parallel beam from the sun disc (0.533°), photons
//!   arriving *down-going*.
//! - [`SkyDomeSource`] — the luminance-weighted hemisphere, importance-sampled
//!   from a [`SkyRadiance`] via a 2-D CDF.
//!
//! Because these emit into a scene of surfaces and openings, the natural
//! detector is an illuminance plane ([`PlaneDetector`]) rather than the
//! goniophotometer sphere: "how much light lands on the desk", the quantity
//! daylight factor is built from.

use crate::ray::Ray;
use crate::spectrum::{ChannelWeights, SourceSpectrum};
use eulumdat_daylight::coords::WorldDir;
use eulumdat_daylight::sky::SkyRadiance;
use eulumdat_daylight::solar::SolarPosition;
use nalgebra::{Point3, Unit, Vector3};
use rand::Rng;
use std::f64::consts::PI;

/// Convert an ENU world direction (+X East, +Y up, +Z South) to the tracer's
/// Type-C frame (+X = C0, +Y = C90, +Z = zenith/up). ENU-up (+Y) becomes
/// sim-up (+Z); ENU-South (+Z) becomes sim −Y so the handedness is preserved.
pub fn world_to_sim(d: &WorldDir) -> Vector3<f64> {
    Vector3::new(d.x, -d.z, d.y)
}

/// The sun's angular radius (half of the 0.533° disc), radians.
const SUN_ANGULAR_RADIUS_RAD: f64 = 0.00465;

/// Build a near-parallel [`Source::Sun`](crate::Source) beam.
///
/// The photons *arrive from* the sun, i.e. travel **down-going** — the negative
/// of the up-going sun direction. `flux_lm` should be the beam power the scene
/// aperture intercepts (DNI × aperture area / efficacy, or simply a chosen
/// lumen budget for a relative study).
pub fn sun_source(
    sun: &SolarPosition,
    origin: Point3<f64>,
    half_extent: f64,
    flux_lm: f64,
) -> crate::Source {
    let up = world_to_sim(&sun.world_direction());
    let arrival = Unit::new_normalize(-up); // down-going
    crate::Source::SunDisc {
        position: origin,
        direction: arrival,
        angular_radius_rad: SUN_ANGULAR_RADIUS_RAD,
        half_extent,
        flux_lm,
    }
}

/// A sky-dome emission source: samples directions over the upper hemisphere
/// proportional to `luminance(θ,φ)·cosθ` (the radiant contribution weighting)
/// and emits photons *arriving from* those directions (down-going).
#[derive(Debug, Clone)]
pub struct SkyDomeSource {
    /// Emission origin (typically the aperture centre or the scene origin).
    pub position: Point3<f64>,
    /// Total downward flux the dome delivers to a horizontal surface (lm),
    /// used for normalisation — equals the DHI times the collecting area.
    pub flux_lm: f64,
    /// 2-D CDF over (theta, phi) grid cells, row-major, normalised to 1.
    cdf: Vec<f64>,
    /// theta grid nodes (from zenith), radians.
    thetas: Vec<f64>,
    /// phi grid nodes (from North, CW), radians.
    phis: Vec<f64>,
    /// Optional colour of the sky (a single D-series SPD, or per-direction via
    /// [`SkyDomeSource::with_spectrum`]).
    spectrum: Option<SourceSpectrum>,
    /// Mean cosθ of the sampling distribution — the average projection of an
    /// emitted photon onto a horizontal collector. Used to size the emitted
    /// flux so a horizontal plane reads back the DHI (see [`collector_flux`]).
    mean_cos: f64,
    /// Half-extent of the horizontal emission square (m).
    half_extent: f64,
    /// Emission height (z) of the square (m).
    emit_z: f64,
}

impl SkyDomeSource {
    /// Build a dome source from an absolute [`SkyRadiance`], sampling the
    /// hemisphere on a `n_theta × n_phi` grid.
    pub fn new(
        radiance: &SkyRadiance,
        position: Point3<f64>,
        flux_lm: f64,
        n_theta: usize,
        n_phi: usize,
    ) -> Self {
        Self::with_extent(radiance, position, flux_lm, 0.5, n_theta, n_phi)
    }

    /// Build a dome source emitting over a horizontal square of `half_extent`
    /// centred at `position`, so a like-sized collector below reads the correct
    /// illuminance. `flux_lm` is the total emitted flux; use
    /// [`collector_flux`](Self::collector_flux) to derive it from a target DHI.
    pub fn with_extent(
        radiance: &SkyRadiance,
        position: Point3<f64>,
        flux_lm: f64,
        half_extent: f64,
        n_theta: usize,
        n_phi: usize,
    ) -> Self {
        let n_theta = n_theta.max(2);
        let n_phi = n_phi.max(2);
        let mut thetas = Vec::with_capacity(n_theta);
        let mut phis = Vec::with_capacity(n_phi);
        // theta in (0, π/2): cell centres so no degenerate zenith/horizon.
        for i in 0..n_theta {
            thetas.push((i as f64 + 0.5) / n_theta as f64 * (PI / 2.0));
        }
        for j in 0..n_phi {
            phis.push((j as f64 + 0.5) / n_phi as f64 * 2.0 * PI);
        }

        // Weight each cell by luminance·sinθ (sinθ = solid-angle Jacobian).
        //
        // We deliberately do NOT fold in cosθ here: directions are sampled ∝
        // radiance·dΩ, and the cosine projection onto the (horizontal) collector
        // is applied by the plane detector's `energy·|cosθ|`. Splitting it this
        // way keeps one clean cosine, so a plane normalised to `flux_lm` reads
        // back exactly that illuminance.
        let mut weights = Vec::with_capacity(n_theta * n_phi);
        let mut total = 0.0;
        for &t in &thetas {
            for &p in &phis {
                let w = radiance.relative_luminance(t, p).max(0.0) * t.sin();
                weights.push(w);
                total += w;
            }
        }
        // Cumulative distribution + probability-weighted mean cosθ.
        let mut cdf = Vec::with_capacity(weights.len());
        let mut cum = 0.0;
        let mut mean_cos = 0.0;
        let mut k = 0;
        for &t in &thetas {
            for _ in &phis {
                let prob = if total > 0.0 { weights[k] / total } else { 0.0 };
                cum += prob;
                cdf.push(cum);
                mean_cos += prob * t.cos().max(0.0);
                k += 1;
            }
        }
        if mean_cos <= 0.0 {
            mean_cos = 1.0;
        }
        Self {
            position,
            flux_lm,
            cdf,
            thetas,
            phis,
            spectrum: None,
            mean_cos,
            half_extent,
            emit_z: position.z,
        }
    }

    /// Total emitted flux (lm) so that a horizontal collector inside the
    /// uniformly-lit emission footprint reads back `dhi_lux`.
    ///
    /// The emission square has area `(2·half_extent)²`. Directions are sampled ∝
    /// radiance·dΩ (no cosine) and the plane applies the cosine, so the emitted
    /// flux is `dhi · A_emit / mean_cosθ`. A collector smaller than the emission
    /// square, positioned in its interior, then reads the correct illuminance
    /// (edge photons that drift off are compensated by the surrounding field).
    pub fn collector_flux(&self, dhi_lux: f64) -> f64 {
        let area_emit = (2.0 * self.half_extent).powi(2);
        dhi_lux * area_emit / self.mean_cos
    }

    /// Set the emitted flux (lm) directly.
    pub fn with_flux(mut self, flux_lm: f64) -> Self {
        self.flux_lm = flux_lm;
        self
    }

    /// Attach a single sky spectrum (e.g. D65 for the whole dome).
    pub fn with_spectrum(mut self, spectrum: SourceSpectrum) -> Self {
        self.spectrum = Some(spectrum);
        self
    }

    /// The sky spectrum, if any.
    pub fn spectrum(&self) -> Option<&SourceSpectrum> {
        self.spectrum.as_ref()
    }

    /// Sample a photon ray arriving from the sky (down-going).
    pub fn sample<R: Rng>(&self, rng: &mut R) -> Ray {
        let xi: f64 = rng.random();
        let idx = match self.cdf.binary_search_by(|c| c.partial_cmp(&xi).unwrap()) {
            Ok(i) => i,
            Err(i) => i.min(self.cdf.len() - 1),
        };
        let n_phi = self.phis.len();
        let ti = idx / n_phi;
        let pj = idx % n_phi;
        let theta = self.thetas[ti.min(self.thetas.len() - 1)];
        let phi = self.phis[pj];

        let up = eulumdat_daylight::coords::dome_to_world(theta, phi);
        let arrival = Unit::new_normalize(-world_to_sim(&up)); // down-going

        // Emit from a random point on the horizontal emission square, so the
        // sky illuminates a finite collector footprint uniformly.
        let ox = (rng.random::<f64>() * 2.0 - 1.0) * self.half_extent;
        let oy = (rng.random::<f64>() * 2.0 - 1.0) * self.half_extent;
        let origin = Point3::new(self.position.x + ox, self.position.y + oy, self.emit_z);
        Ray::new(origin, arrival)
    }
}

/// A finite illuminance-measuring plane.
///
/// Photons crossing the plane deposit `energy·|cosθ|` into the grid cell they
/// hit, which is exactly the illuminance definition (lux = lm·|cosθ| per area).
/// Runs alongside a trace as a workpiece surface — the daylight-factor grid.
#[derive(Debug, Clone)]
pub struct PlaneDetector {
    center: Point3<f64>,
    normal: Unit<Vector3<f64>>,
    u_axis: Unit<Vector3<f64>>,
    v_axis: Unit<Vector3<f64>>,
    half_u: f64,
    half_v: f64,
    nu: usize,
    nv: usize,
    /// Accumulated cos-weighted energy per cell (row-major, `[iu*nv + iv]`).
    cells: Vec<f64>,
    /// Photon count per cell.
    counts: Vec<u64>,
    /// Optional spectral channels per cell (present when spectral).
    channels: Option<Vec<ChannelWeights>>,
}

impl PlaneDetector {
    /// A horizontal plane (normal +Z, facing up) at height `z`, centred at
    /// `(cx, cy)`, of size `2·half_u × 2·half_v`, gridded `nu × nv`.
    pub fn horizontal(
        center: Point3<f64>,
        half_u: f64,
        half_v: f64,
        nu: usize,
        nv: usize,
        spectral: bool,
    ) -> Self {
        Self::new(
            center,
            Vector3::z_axis(),
            Vector3::x_axis(),
            half_u,
            half_v,
            nu,
            nv,
            spectral,
        )
    }

    /// A plane with an explicit orientation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: Point3<f64>,
        normal: Unit<Vector3<f64>>,
        u_axis: Unit<Vector3<f64>>,
        half_u: f64,
        half_v: f64,
        nu: usize,
        nv: usize,
        spectral: bool,
    ) -> Self {
        let v_axis = Unit::new_normalize(normal.cross(u_axis.as_ref()));
        let nu = nu.max(1);
        let nv = nv.max(1);
        Self {
            center,
            normal,
            u_axis,
            v_axis,
            half_u,
            half_v,
            nu,
            nv,
            cells: vec![0.0; nu * nv],
            counts: vec![0; nu * nv],
            channels: spectral.then(|| vec![ChannelWeights::default(); nu * nv]),
        }
    }

    /// Test a ray segment against the plane; if it crosses within the finite
    /// extent, record `energy·|cosθ|` into the hit cell.
    ///
    /// Returns `true` if the photon was recorded (so a caller can treat the
    /// plane as absorbing, or continue tracing through it as a virtual sensor).
    pub fn record_ray(&mut self, ray: &Ray, energy: f64, wavelength: f64, max_t: f64) -> bool {
        let denom = ray.direction.dot(self.normal.as_ref());
        if denom.abs() < 1e-12 {
            return false; // parallel to plane
        }
        let t = (self.center - ray.origin).dot(self.normal.as_ref()) / denom;
        if t < 1e-6 || t > max_t {
            return false;
        }
        let p = ray.origin + ray.direction.as_ref() * t;
        let rel = p - self.center;
        let u = rel.dot(self.u_axis.as_ref());
        let v = rel.dot(self.v_axis.as_ref());
        if u.abs() > self.half_u || v.abs() > self.half_v {
            return false;
        }
        let iu = (((u + self.half_u) / (2.0 * self.half_u)) * self.nu as f64)
            .floor()
            .clamp(0.0, (self.nu - 1) as f64) as usize;
        let iv = (((v + self.half_v) / (2.0 * self.half_v)) * self.nv as f64)
            .floor()
            .clamp(0.0, (self.nv - 1) as f64) as usize;
        let idx = iu * self.nv + iv;
        let cos = denom.abs();
        self.cells[idx] += energy * cos;
        self.counts[idx] += 1;
        if let Some(ch) = self.channels.as_mut() {
            let w = ChannelWeights::for_photon(wavelength, energy * cos);
            ch[idx].add(&w);
        }
        true
    }

    /// Cell dimensions `(nu, nv)`.
    pub fn dims(&self) -> (usize, usize) {
        (self.nu, self.nv)
    }

    /// Convert accumulated cos-weighted energy to illuminance (lux) per cell,
    /// given the source flux and total emitted energy the plane samples from.
    ///
    /// `flux_lm` is the emitting source's luminous flux; `total_emitted_energy`
    /// is the number of photons emitted (each carrying energy 1.0), so
    /// `lm per photon-energy = flux_lm / total_emitted_energy`. Each cell's
    /// illuminance is its cos-weighted flux divided by cell area.
    pub fn illuminance(&self, flux_lm: f64, total_emitted_energy: f64) -> Vec<Vec<f64>> {
        let cell_area = (2.0 * self.half_u / self.nu as f64) * (2.0 * self.half_v / self.nv as f64);
        let lm_per_e = if total_emitted_energy > 0.0 {
            flux_lm / total_emitted_energy
        } else {
            0.0
        };
        let mut out = vec![vec![0.0; self.nv]; self.nu];
        for (iu, row) in out.iter_mut().enumerate() {
            for (iv, cell) in row.iter_mut().enumerate() {
                let e = self.cells[iu * self.nv + iv];
                *cell = e * lm_per_e / cell_area;
            }
        }
        out
    }

    /// Average illuminance over all cells (lux).
    pub fn average_illuminance(&self, flux_lm: f64, total_emitted_energy: f64) -> f64 {
        let grid = self.illuminance(flux_lm, total_emitted_energy);
        let mut sum = 0.0;
        let mut n = 0;
        for row in &grid {
            for &v in row {
                sum += v;
                n += 1;
            }
        }
        if n > 0 {
            sum / n as f64
        } else {
            0.0
        }
    }

    /// Merge another plane detector's accumulation (parallel reduction).
    pub fn merge(&mut self, other: &PlaneDetector) {
        for i in 0..self.cells.len() {
            self.cells[i] += other.cells[i];
            self.counts[i] += other.counts[i];
        }
        if let (Some(a), Some(b)) = (self.channels.as_mut(), other.channels.as_ref()) {
            for i in 0..a.len() {
                let w = b[i];
                a[i].add(&w);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn world_to_sim_zenith_is_up() {
        // ENU zenith (+Y) → sim +Z.
        let up = WorldDir {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let s = world_to_sim(&up);
        assert_relative_eq!(s.z, 1.0, epsilon = 1e-12);
        assert_relative_eq!(s.x, 0.0, epsilon = 1e-12);
        assert_relative_eq!(s.y, 0.0, epsilon = 1e-12);
    }

    #[test]
    fn plane_records_normal_incidence() {
        // A downward photon hitting a horizontal plane deposits full energy.
        let mut plane = PlaneDetector::horizontal(Point3::origin(), 1.0, 1.0, 1, 1, false);
        let ray = Ray::new(
            Point3::new(0.0, 0.0, 1.0),
            Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
        );
        assert!(plane.record_ray(&ray, 1.0, 555.0, 10.0));
        // cos = 1, area = 4, one photon of energy 1: illuminance = flux/area.
        let e = plane.illuminance(4.0, 1.0);
        assert_relative_eq!(e[0][0], 1.0, epsilon = 1e-9);
    }

    #[test]
    fn plane_misses_outside_extent() {
        let mut plane = PlaneDetector::horizontal(Point3::origin(), 1.0, 1.0, 1, 1, false);
        let ray = Ray::new(
            Point3::new(5.0, 0.0, 1.0),
            Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
        );
        assert!(!plane.record_ray(&ray, 1.0, 555.0, 10.0));
    }
}
