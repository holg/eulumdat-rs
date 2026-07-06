//! Illuminance tracing: emit photons and collect them on [`PlaneDetector`]s.
//!
//! Distinct from the goniophotometer path in [`crate::tracer`], which bins
//! *escaping* directions on a sphere. Here the question is "how much light lands
//! on this surface", so photons are traced through the scene geometry and each
//! plane sensor records the crossings — the basis for daylight factor and
//! interior-illuminance studies.

use crate::daylight::PlaneDetector;
use crate::material::Interaction;
use crate::ray::Photon;
use crate::scene::Scene;
use crate::tracer::TracerConfig;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

/// Result of an illuminance trace: the filled plane detectors plus the total
/// emitted photon energy (for lux normalisation).
#[derive(Debug, Clone)]
pub struct IlluminanceResult {
    /// Plane sensors, in the order supplied.
    pub planes: Vec<PlaneDetector>,
    /// Total emitted photon energy (= number of photons, each energy 1.0).
    pub total_emitted_energy: f64,
    /// Total source flux (lumens) across all sources.
    pub total_flux_lm: f64,
}

impl IlluminanceResult {
    /// Average illuminance (lux) on plane `i`.
    pub fn average_lux(&self, i: usize) -> f64 {
        self.planes[i].average_illuminance(self.total_flux_lm, self.total_emitted_energy)
    }
}

/// Trace photons through the scene, recording plane-sensor crossings.
///
/// Each photon is followed through reflections/transmissions; on every straight
/// segment it is tested against all plane sensors, so a sensor sees both direct
/// and inter-reflected light. Photons that escape (no hit) still get one final
/// unbounded segment tested against the planes.
pub fn trace_illuminance(
    scene: &Scene,
    config: &TracerConfig,
    mut planes: Vec<PlaneDetector>,
) -> IlluminanceResult {
    let num_sources = scene.sources.len();
    assert!(num_sources > 0, "scene must have at least one source");

    let mut rng = Xoshiro256PlusPlus::seed_from_u64(config.seed);
    let far = 1e7;

    // Per-source luminous weight. Photons are round-robined equally across
    // sources, but each source carries its own flux. To make the downstream
    // `total_flux / num_photons` normalisation credit every source with exactly
    // its own flux, scale each photon's recorded energy by
    // `source_flux · num_sources / total_flux`. For a single source this is 1.0,
    // so single-source results are unchanged.
    let total_flux = scene.total_source_flux().max(f64::MIN_POSITIVE);
    let source_weights: Vec<f64> = (0..num_sources)
        .map(|s| scene.sources[s].flux_lm() * num_sources as f64 / total_flux)
        .collect();

    for i in 0..config.num_photons {
        let src_idx = (i as usize) % num_sources;
        let source = &scene.sources[src_idx];
        let source_weight = source_weights[src_idx];
        let ray = source.sample(&mut rng);
        let wavelength = scene
            .spectrum(src_idx)
            .map(|s| s.sample_wavelength(rng.random::<f64>()))
            .unwrap_or(555.0);

        let mut photon = Photon::new(ray);
        photon.wavelength = wavelength;

        loop {
            let hit = scene.intersect(&photon.ray, 1e-6, far);
            let seg_len = hit.as_ref().map(|h| h.t).unwrap_or(far);

            // Record this straight segment against every plane sensor, scaled
            // by the source's luminous weight so mixed-flux scenes superpose
            // correctly (e.g. daylight + electric light).
            for plane in &mut planes {
                plane.record_ray(
                    &photon.ray,
                    photon.energy * source_weight,
                    photon.wavelength,
                    seg_len,
                );
            }

            let Some(hit) = hit else {
                break; // escaped
            };

            let material = scene.material(hit.material);
            match material.interact(&photon, &hit, &mut rng) {
                Interaction::Absorbed => break,
                Interaction::Reflected {
                    new_ray,
                    attenuation,
                }
                | Interaction::Transmitted {
                    new_ray,
                    attenuation,
                } => {
                    photon.ray = new_ray;
                    photon.energy *= attenuation;
                }
            }

            photon.bounces += 1;
            if photon.bounces >= config.max_bounces {
                break;
            }
            if photon.energy < config.russian_roulette_threshold {
                let survive = photon.energy / config.russian_roulette_threshold;
                if rng.random::<f64>() > survive {
                    break;
                }
                photon.energy = config.russian_roulette_threshold;
            }
        }
    }

    IlluminanceResult {
        planes,
        total_emitted_energy: config.num_photons as f64,
        total_flux_lm: scene.total_source_flux(),
    }
}

/// Daylight factor grid: interior illuminance under a sky as a percentage of
/// the unobstructed exterior horizontal illuminance.
///
/// `interior` is the interior sensor after a trace; `exterior_lux` is the DHI
/// the same sky delivers to an unobstructed horizontal plane. DF = 100·E_in/E_ext.
pub fn daylight_factor_grid(
    interior: &PlaneDetector,
    flux_lm: f64,
    total_emitted_energy: f64,
    exterior_lux: f64,
) -> Vec<Vec<f64>> {
    let grid = interior.illuminance(flux_lm, total_emitted_energy);
    grid.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|e| if exterior_lux > 0.0 { 100.0 * e / exterior_lux } else { 0.0 })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daylight::PlaneDetector;
    use crate::tracer::TracerConfig;
    use crate::{Scene, Source};
    use nalgebra::{Point3, Unit, Vector3};

    fn downlight(flux: f64, z: f64) -> Source {
        Source::Led {
            position: Point3::new(0.0, 0.0, z),
            direction: Unit::new_normalize(Vector3::new(0.0, 0.0, -1.0)),
            half_angle_deg: 10.0, // tight beam so ~all flux lands on the sensor
            flux_lm: flux,
        }
    }

    /// Mixed-flux scenes must superpose: the plane illuminance from two sources
    /// together equals the sum of each traced alone. This guards the per-source
    /// luminous weighting that a daylight+electric ("dim as sun rises") feature
    /// depends on.
    #[test]
    fn mixed_flux_sources_superpose() {
        let cfg = TracerConfig {
            num_photons: 200_000,
            max_bounces: 2,
            seed: 5,
            ..TracerConfig::default()
        };
        let lux = |sources: &[Source]| {
            let mut scene = Scene::new();
            for s in sources {
                scene.add_source(s.clone());
            }
            let plane = PlaneDetector::horizontal(Point3::origin(), 0.5, 0.5, 1, 1, false);
            trace_illuminance(&scene, &cfg, vec![plane]).average_lux(0)
        };

        // Two very different fluxes (10:1) so a naive equal-weighting would be
        // badly wrong.
        let a = downlight(10_000.0, 2.0);
        let b = downlight(1_000.0, 2.0);
        let only_a = lux(std::slice::from_ref(&a));
        let only_b = lux(std::slice::from_ref(&b));
        let both = lux(&[a, b]);

        assert!(only_a > only_b, "brighter lamp reads higher");
        let rel = (both - (only_a + only_b)).abs() / both;
        assert!(
            rel < 0.03,
            "superposition: both {both:.0} == a {only_a:.0} + b {only_b:.0} (rel {rel:.4})"
        );
    }

    /// A single source is unaffected by the weighting (weight == 1.0).
    #[test]
    fn single_source_weight_is_identity() {
        let cfg = TracerConfig {
            num_photons: 100_000,
            max_bounces: 2,
            seed: 9,
            ..TracerConfig::default()
        };
        let mut scene = Scene::new();
        scene.add_source(downlight(5_000.0, 2.0));
        let plane = PlaneDetector::horizontal(Point3::origin(), 0.5, 0.5, 1, 1, false);
        let e = trace_illuminance(&scene, &cfg, vec![plane]).average_lux(0);
        assert!(e > 0.0, "single source should light the plane");
    }
}
