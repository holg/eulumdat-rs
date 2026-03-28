//! Monte Carlo photon tracer.

use crate::detector::Detector;
use crate::material::Interaction;
use crate::ray::Photon;
use crate::scene::Scene;
use nalgebra::Point3;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::time::{Duration, Instant};

/// Configuration for a trace run.
#[derive(Debug, Clone)]
pub struct TracerConfig {
    /// Total photons to trace.
    pub num_photons: u64,
    /// Maximum interactions per photon before termination.
    pub max_bounces: u32,
    /// Energy threshold below which Russian roulette kicks in.
    pub russian_roulette_threshold: f64,
    /// RNG seed for reproducibility.
    pub seed: u64,
    /// Detector C-angle resolution in degrees.
    pub detector_c_resolution: f64,
    /// Detector gamma resolution in degrees.
    pub detector_g_resolution: f64,
    /// Number of photon trails to record for visualization (0 to disable).
    pub max_trails: usize,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            num_photons: 1_000_000,
            max_bounces: 50,
            russian_roulette_threshold: 0.01,
            seed: 42,
            detector_c_resolution: 1.0,
            detector_g_resolution: 1.0,
            max_trails: 0,
        }
    }
}

/// Result of a completed trace.
#[derive(Debug, Clone)]
pub struct TracerResult {
    /// Filled detector with accumulated photon data.
    pub detector: Detector,
    /// Statistics about the trace run.
    pub stats: TracerStats,
    /// Recorded photon trails for visualization.
    pub trails: Vec<PhotonTrail>,
}

/// Statistics about a completed trace.
#[derive(Debug, Clone, Default)]
pub struct TracerStats {
    pub photons_traced: u64,
    pub photons_detected: u64,
    pub photons_absorbed: u64,
    pub photons_max_bounces: u64,
    pub photons_russian_roulette: u64,
    pub total_energy_emitted: f64,
    pub total_energy_detected: f64,
    pub elapsed: Duration,
}

/// Progress information for the progress callback.
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub photons_done: u64,
    pub photons_total: u64,
    pub photons_per_second: f64,
    pub current_stats: TracerStats,
}

/// A recorded photon path for visualization.
#[derive(Debug, Clone)]
pub struct PhotonTrail {
    pub points: Vec<TrailPoint>,
}

/// A point along a photon's path.
#[derive(Debug, Clone)]
pub struct TrailPoint {
    pub position: Point3<f64>,
    pub event: TrailEvent,
}

/// Type of event at a trail point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailEvent {
    Emitted,
    Reflected,
    Transmitted,
    Scattered,
    Absorbed,
    Detected,
}

/// The Monte Carlo photon tracer.
pub struct Tracer;

impl Tracer {
    /// Trace all photons through the scene.
    pub fn trace(scene: &Scene, config: &TracerConfig) -> TracerResult {
        Self::trace_with_progress(scene, config, |_| {})
    }

    /// Trace with a progress callback (called periodically).
    pub fn trace_with_progress(
        scene: &Scene,
        config: &TracerConfig,
        callback: impl Fn(ProgressInfo) + Send + Sync,
    ) -> TracerResult {
        let start = Instant::now();

        #[cfg(feature = "parallel")]
        let result = trace_parallel(scene, config, &callback, start);

        #[cfg(not(feature = "parallel"))]
        let result = trace_sequential(scene, config, &callback, start);

        result
    }
}

/// Trace photons sequentially (single-threaded).
#[cfg(not(feature = "parallel"))]
fn trace_sequential(
    scene: &Scene,
    config: &TracerConfig,
    callback: &(impl Fn(ProgressInfo) + Send + Sync),
    start: Instant,
) -> TracerResult {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(config.seed);
    let mut detector = Detector::new(config.detector_c_resolution, config.detector_g_resolution);
    let mut stats = TracerStats::default();
    let mut trails = Vec::new();

    let batch_size = 10_000u64;
    let num_sources = scene.sources.len();
    assert!(num_sources > 0, "Scene must have at least one source");

    for i in 0..config.num_photons {
        // Round-robin across sources
        let source = &scene.sources[(i as usize) % num_sources];
        let ray = source.sample(&mut rng);
        let record_trail = trails.len() < config.max_trails;

        let result = trace_one_photon(scene, config, ray, &mut rng, record_trail);

        stats.total_energy_emitted += 1.0;
        stats.photons_traced += 1;

        match result.outcome {
            PhotonOutcome::Detected { energy } => {
                detector.record(result.final_direction.as_ref().unwrap(), energy);
                stats.photons_detected += 1;
                stats.total_energy_detected += energy;
            }
            PhotonOutcome::Absorbed => {
                stats.photons_absorbed += 1;
            }
            PhotonOutcome::MaxBounces => {
                stats.photons_max_bounces += 1;
            }
            PhotonOutcome::RussianRoulette => {
                stats.photons_russian_roulette += 1;
            }
        }

        if let Some(trail) = result.trail {
            trails.push(trail);
        }

        // Progress callback
        if (i + 1) % batch_size == 0 || i + 1 == config.num_photons {
            let elapsed = start.elapsed();
            stats.elapsed = elapsed;
            callback(ProgressInfo {
                photons_done: i + 1,
                photons_total: config.num_photons,
                photons_per_second: (i + 1) as f64 / elapsed.as_secs_f64(),
                current_stats: stats.clone(),
            });
        }
    }

    stats.elapsed = start.elapsed();

    TracerResult {
        detector,
        stats,
        trails,
    }
}

/// Trace photons in parallel using Rayon.
#[cfg(feature = "parallel")]
fn trace_parallel(
    scene: &Scene,
    config: &TracerConfig,
    callback: &(impl Fn(ProgressInfo) + Send + Sync),
    start: Instant,
) -> TracerResult {
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    let num_threads = rayon::current_num_threads();
    let photons_per_thread = config.num_photons / num_threads as u64;
    let progress_counter = AtomicU64::new(0);

    let thread_results: Vec<(Detector, TracerStats, Vec<PhotonTrail>)> = (0..num_threads)
        .into_par_iter()
        .map(|thread_idx| {
            let mut rng =
                Xoshiro256PlusPlus::seed_from_u64(config.seed.wrapping_add(thread_idx as u64));
            let mut detector =
                Detector::new(config.detector_c_resolution, config.detector_g_resolution);
            let mut stats = TracerStats::default();
            let mut trails = Vec::new();
            let num_sources = scene.sources.len();

            // First thread gets any remainder photons
            let n = if thread_idx == 0 {
                photons_per_thread + (config.num_photons % num_threads as u64)
            } else {
                photons_per_thread
            };

            for i in 0..n {
                let source_idx =
                    ((thread_idx as u64 * photons_per_thread + i) as usize) % num_sources;
                let source = &scene.sources[source_idx];
                let ray = source.sample(&mut rng);
                let record_trail =
                    thread_idx == 0 && trails.len() < config.max_trails;

                let result = trace_one_photon(scene, config, ray, &mut rng, record_trail);

                stats.total_energy_emitted += 1.0;
                stats.photons_traced += 1;

                match result.outcome {
                    PhotonOutcome::Detected { energy } => {
                        detector.record(result.final_direction.as_ref().unwrap(), energy);
                        stats.photons_detected += 1;
                        stats.total_energy_detected += energy;
                    }
                    PhotonOutcome::Absorbed => stats.photons_absorbed += 1,
                    PhotonOutcome::MaxBounces => stats.photons_max_bounces += 1,
                    PhotonOutcome::RussianRoulette => stats.photons_russian_roulette += 1,
                }

                if let Some(trail) = result.trail {
                    trails.push(trail);
                }

                // Progress (only thread 0 reports)
                if thread_idx == 0 && (i + 1) % 10_000 == 0 {
                    let total_done = progress_counter.load(Ordering::Relaxed) + i + 1;
                    let elapsed = start.elapsed();
                    callback(ProgressInfo {
                        photons_done: total_done,
                        photons_total: config.num_photons,
                        photons_per_second: total_done as f64 / elapsed.as_secs_f64(),
                        current_stats: stats.clone(),
                    });
                }
            }

            progress_counter.fetch_add(n, Ordering::Relaxed);
            (detector, stats, trails)
        })
        .collect();

    // Merge results
    let mut detector = Detector::new(config.detector_c_resolution, config.detector_g_resolution);
    let mut stats = TracerStats::default();
    let mut trails = Vec::new();

    for (d, s, t) in thread_results {
        detector.merge(&d);
        stats.photons_traced += s.photons_traced;
        stats.photons_detected += s.photons_detected;
        stats.photons_absorbed += s.photons_absorbed;
        stats.photons_max_bounces += s.photons_max_bounces;
        stats.photons_russian_roulette += s.photons_russian_roulette;
        stats.total_energy_emitted += s.total_energy_emitted;
        stats.total_energy_detected += s.total_energy_detected;
        trails.extend(t);
    }

    stats.elapsed = start.elapsed();

    TracerResult {
        detector,
        stats,
        trails,
    }
}

// ---------------------------------------------------------------------------
// Single photon tracing
// ---------------------------------------------------------------------------

enum PhotonOutcome {
    Detected { energy: f64 },
    Absorbed,
    MaxBounces,
    RussianRoulette,
}

struct SinglePhotonResult {
    outcome: PhotonOutcome,
    final_direction: Option<nalgebra::Vector3<f64>>,
    trail: Option<PhotonTrail>,
}

fn trace_one_photon(
    scene: &Scene,
    config: &TracerConfig,
    initial_ray: crate::ray::Ray,
    rng: &mut Xoshiro256PlusPlus,
    record_trail: bool,
) -> SinglePhotonResult {
    use rand::Rng;

    let mut photon = Photon::new(initial_ray);
    let mut trail_points = if record_trail {
        vec![TrailPoint {
            position: photon.ray.origin,
            event: TrailEvent::Emitted,
        }]
    } else {
        Vec::new()
    };

    loop {
        // Find nearest intersection
        let hit = scene.intersect(&photon.ray, 1e-6, 1e10);

        match hit {
            None => {
                // Photon escaped — record on detector
                if record_trail {
                    let far_point = photon.ray.at(1.0);
                    trail_points.push(TrailPoint {
                        position: far_point,
                        event: TrailEvent::Detected,
                    });
                }
                return SinglePhotonResult {
                    outcome: PhotonOutcome::Detected {
                        energy: photon.energy,
                    },
                    final_direction: Some(*photon.ray.direction.as_ref()),
                    trail: if record_trail {
                        Some(PhotonTrail {
                            points: trail_points,
                        })
                    } else {
                        None
                    },
                };
            }

            Some(hit) => {
                let material = scene.material(hit.material);
                let interaction = material.interact(&photon, &hit, rng);

                match interaction {
                    Interaction::Absorbed => {
                        if record_trail {
                            trail_points.push(TrailPoint {
                                position: hit.point,
                                event: TrailEvent::Absorbed,
                            });
                        }
                        return SinglePhotonResult {
                            outcome: PhotonOutcome::Absorbed,
                            final_direction: None,
                            trail: if record_trail {
                                Some(PhotonTrail {
                                    points: trail_points,
                                })
                            } else {
                                None
                            },
                        };
                    }

                    Interaction::Reflected {
                        new_ray,
                        attenuation,
                    } => {
                        if record_trail {
                            trail_points.push(TrailPoint {
                                position: hit.point,
                                event: TrailEvent::Reflected,
                            });
                        }
                        photon.ray = new_ray;
                        photon.energy *= attenuation;
                    }

                    Interaction::Transmitted {
                        new_ray,
                        attenuation,
                    } => {
                        if record_trail {
                            trail_points.push(TrailPoint {
                                position: hit.point,
                                event: TrailEvent::Transmitted,
                            });
                        }
                        photon.ray = new_ray;
                        photon.energy *= attenuation;
                    }
                }

                photon.bounces += 1;

                // Max bounces check
                if photon.bounces >= config.max_bounces {
                    return SinglePhotonResult {
                        outcome: PhotonOutcome::MaxBounces,
                        final_direction: None,
                        trail: if record_trail {
                            Some(PhotonTrail {
                                points: trail_points,
                            })
                        } else {
                            None
                        },
                    };
                }

                // Russian roulette
                if photon.energy < config.russian_roulette_threshold {
                    let survive_prob = photon.energy / config.russian_roulette_threshold;
                    if rng.random::<f64>() > survive_prob {
                        return SinglePhotonResult {
                            outcome: PhotonOutcome::RussianRoulette,
                            final_direction: None,
                            trail: if record_trail {
                                Some(PhotonTrail {
                                    points: trail_points,
                                })
                            } else {
                                None
                            },
                        };
                    }
                    // Survived — boost energy to compensate
                    photon.energy = config.russian_roulette_threshold;
                }
            }
        }
    }
}
