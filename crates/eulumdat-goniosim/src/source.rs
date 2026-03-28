//! Photon emission source models.

use crate::ray::Ray;
use eulumdat::Eulumdat;
use nalgebra::{Point3, Rotation3, Unit, Vector3};
use rand::Rng;
use std::f64::consts::PI;

/// Light source types for photon emission.
#[derive(Debug, Clone)]
pub enum Source {
    /// Uniform emission in all directions (4pi steradians).
    Isotropic {
        position: Point3<f64>,
        flux_lm: f64,
    },

    /// Cosine-weighted hemisphere (ideal diffuse emitter).
    Lambertian {
        position: Point3<f64>,
        normal: Unit<Vector3<f64>>,
        flux_lm: f64,
    },

    /// Directional LED with beam angle.
    Led {
        position: Point3<f64>,
        direction: Unit<Vector3<f64>>,
        half_angle_deg: f64,
        flux_lm: f64,
    },

    /// Line source (LED strip) — samples random point along segment.
    LineSource {
        start: Point3<f64>,
        end: Point3<f64>,
        normal: Unit<Vector3<f64>>,
        half_angle_deg: f64,
        flux_lm: f64,
    },

    /// Emit according to an existing LDT/IES distribution (for validation).
    FromLvk {
        position: Point3<f64>,
        orientation: Rotation3<f64>,
        eulumdat: Box<Eulumdat>,
        flux_lm: f64,
        /// Pre-computed CDF for efficient sampling (built automatically).
        cdf: LvkCdf,
    },
}

impl Source {
    /// Create a FromLvk source from an Eulumdat, pre-computing the sampling CDF.
    pub fn from_lvk(
        position: Point3<f64>,
        orientation: Rotation3<f64>,
        eulumdat: Eulumdat,
        flux_lm: f64,
    ) -> Self {
        let cdf = LvkCdf::build(&eulumdat);
        Source::FromLvk {
            position,
            orientation,
            eulumdat: Box::new(eulumdat),
            flux_lm,
            cdf,
        }
    }

    /// Total luminous flux of the source in lumens.
    pub fn flux_lm(&self) -> f64 {
        match self {
            Source::Isotropic { flux_lm, .. } => *flux_lm,
            Source::Lambertian { flux_lm, .. } => *flux_lm,
            Source::Led { flux_lm, .. } => *flux_lm,
            Source::LineSource { flux_lm, .. } => *flux_lm,
            Source::FromLvk { flux_lm, .. } => *flux_lm,
        }
    }

    /// Sample a photon ray from this source.
    pub fn sample<R: Rng>(&self, rng: &mut R) -> Ray {
        match self {
            Source::Isotropic { position, .. } => {
                let dir = random_sphere(rng);
                Ray::new(*position, dir)
            }

            Source::Lambertian {
                position, normal, ..
            } => {
                let dir = random_cosine_hemisphere(normal, rng);
                Ray::new(*position, dir)
            }

            Source::Led {
                position,
                direction,
                half_angle_deg,
                ..
            } => {
                let dir = random_cone(direction, *half_angle_deg, rng);
                Ray::new(*position, dir)
            }

            Source::LineSource {
                start,
                end,
                normal,
                half_angle_deg,
                ..
            } => {
                // Random point along line segment
                let t: f64 = rng.random();
                let origin = start + t * (end - start);
                let dir = random_cone(normal, *half_angle_deg, rng);
                Ray::new(origin, dir)
            }

            Source::FromLvk {
                position,
                orientation,
                cdf,
                ..
            } => {
                // Sample direction from pre-computed CDF (no rejection, no bias)
                let (c_angle, g_angle) = cdf.sample(rng);

                let g_rad = g_angle.to_radians();
                let c_rad = c_angle.to_radians();

                // In photometric coords: gamma=0 is nadir (-Z), C0 is +X
                let dir_local = Vector3::new(
                    g_rad.sin() * c_rad.cos(),
                    g_rad.sin() * c_rad.sin(),
                    -g_rad.cos(),
                );

                let dir_world = orientation * dir_local;
                Ray::new(*position, Unit::new_normalize(dir_world))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Random direction helpers
// ---------------------------------------------------------------------------

/// Uniform random direction on the unit sphere.
fn random_sphere<R: Rng>(rng: &mut R) -> Unit<Vector3<f64>> {
    let z: f64 = 2.0 * rng.random::<f64>() - 1.0;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let phi = 2.0 * PI * rng.random::<f64>();
    Unit::new_normalize(Vector3::new(r * phi.cos(), r * phi.sin(), z))
}

/// Cosine-weighted random direction in hemisphere around normal.
fn random_cosine_hemisphere<R: Rng>(
    normal: &Unit<Vector3<f64>>,
    rng: &mut R,
) -> Unit<Vector3<f64>> {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    let r = u1.sqrt();
    let theta = 2.0 * PI * u2;
    let x = r * theta.cos();
    let y = r * theta.sin();
    let z = (1.0 - u1).sqrt();

    let (tangent, bitangent) = build_onb(normal);
    let dir = x * tangent.as_ref() + y * bitangent.as_ref() + z * normal.as_ref();
    Unit::new_normalize(dir)
}

/// Random direction within a cone around a given axis.
fn random_cone<R: Rng>(
    axis: &Unit<Vector3<f64>>,
    half_angle_deg: f64,
    rng: &mut R,
) -> Unit<Vector3<f64>> {
    let cos_max = (half_angle_deg.to_radians()).cos();
    let u: f64 = rng.random();
    let cos_theta = 1.0 - u * (1.0 - cos_max);
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    let phi = 2.0 * PI * rng.random::<f64>();

    let (tangent, bitangent) = build_onb(axis);
    let dir = sin_theta * phi.cos() * tangent.as_ref()
        + sin_theta * phi.sin() * bitangent.as_ref()
        + cos_theta * axis.as_ref();
    Unit::new_normalize(dir)
}

/// Build an orthonormal basis from a single vector.
fn build_onb(n: &Unit<Vector3<f64>>) -> (Unit<Vector3<f64>>, Unit<Vector3<f64>>) {
    let a = if n.x.abs() > 0.9 {
        Vector3::y_axis()
    } else {
        Vector3::x_axis()
    };
    let t = Unit::new_normalize(n.cross(a.as_ref()));
    let b = Unit::new_normalize(n.cross(t.as_ref()));
    (t, b)
}

// ---------------------------------------------------------------------------
// LVK importance sampling via pre-computed CDF
// ---------------------------------------------------------------------------

/// Pre-computed CDF table for efficient LVK sampling.
/// Built once per LDT, then sampled in O(log n) via binary search.
#[derive(Debug, Clone)]
struct LvkCdf {
    /// Gamma angles sampled at 1° resolution
    g_steps: usize,
    /// C angles sampled at 5° resolution
    c_steps: usize,
    /// g_max from LDT data
    g_max: f64,
    /// Marginal CDF over gamma: P(G <= g)
    marginal_g: Vec<f64>,
    /// Conditional CDF over C for each gamma bin: P(C <= c | G=g)
    conditional_c: Vec<Vec<f64>>,
}

impl LvkCdf {
    fn build(ldt: &Eulumdat) -> Self {
        let g_max = ldt.g_angles.last().copied().unwrap_or(180.0);
        let g_step: f64 = 1.0; // 1° resolution
        let c_step: f64 = 5.0; // 5° resolution
        let g_steps = (g_max / g_step).ceil() as usize + 1;
        let c_steps = (360.0 / c_step).ceil() as usize;

        // Build the 2D PDF: f(c,g) = I(c,g) * sin(g)
        let mut pdf = vec![vec![0.0f64; c_steps]; g_steps];
        for gi in 0..g_steps {
            let g = (gi as f64 * g_step).min(g_max);
            let sin_g = g.to_radians().sin();
            for ci in 0..c_steps {
                let c = ci as f64 * c_step;
                pdf[gi][ci] = ldt.sample(c, g) * sin_g;
            }
        }

        // Marginal over gamma: sum over C for each g
        let marginal_unnorm: Vec<f64> = pdf.iter()
            .map(|row| row.iter().sum::<f64>())
            .collect();

        // Build marginal CDF
        let mut marginal_g = vec![0.0; g_steps];
        let mut cum = 0.0;
        for gi in 0..g_steps {
            cum += marginal_unnorm[gi];
            marginal_g[gi] = cum;
        }
        // Normalize
        if cum > 0.0 {
            for v in &mut marginal_g {
                *v /= cum;
            }
        }

        // Build conditional CDF over C for each gamma
        let mut conditional_c = vec![vec![0.0; c_steps]; g_steps];
        for gi in 0..g_steps {
            let row_sum: f64 = pdf[gi].iter().sum();
            let mut ccum = 0.0;
            for ci in 0..c_steps {
                ccum += pdf[gi][ci];
                conditional_c[gi][ci] = if row_sum > 0.0 { ccum / row_sum } else { (ci + 1) as f64 / c_steps as f64 };
            }
        }

        Self {
            g_steps,
            c_steps,
            g_max,
            marginal_g,
            conditional_c,
        }
    }

    fn sample<R: Rng>(&self, rng: &mut R) -> (f64, f64) {
        // Sample gamma from marginal CDF
        let u_g: f64 = rng.random();
        let gi = match self.marginal_g.binary_search_by(|v| v.partial_cmp(&u_g).unwrap()) {
            Ok(i) => i,
            Err(i) => i.min(self.g_steps - 1),
        };
        let g = (gi as f64 / (self.g_steps - 1).max(1) as f64) * self.g_max;

        // Sample C from conditional CDF at this gamma
        let u_c: f64 = rng.random();
        let ci = match self.conditional_c[gi].binary_search_by(|v| v.partial_cmp(&u_c).unwrap()) {
            Ok(i) => i,
            Err(i) => i.min(self.c_steps - 1),
        };
        let c = ci as f64 * (360.0 / self.c_steps as f64);

        (c, g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256PlusPlus;

    #[test]
    fn isotropic_samples_all_directions() {
        let source = Source::Isotropic {
            position: Point3::origin(),
            flux_lm: 1000.0,
        };
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let mut sum = Vector3::zeros();
        let n = 10000;
        for _ in 0..n {
            let ray = source.sample(&mut rng);
            sum += ray.direction.as_ref();
        }
        // Mean direction should be near zero for isotropic
        let mean = sum / n as f64;
        assert!(mean.norm() < 0.05, "Mean direction should be near zero");
    }

    #[test]
    fn lambertian_samples_hemisphere() {
        let source = Source::Lambertian {
            position: Point3::origin(),
            normal: Vector3::z_axis(),
            flux_lm: 1000.0,
        };
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        for _ in 0..1000 {
            let ray = source.sample(&mut rng);
            assert!(
                ray.direction.z >= -1e-6,
                "Lambertian should only emit into positive hemisphere"
            );
        }
    }

    #[test]
    fn led_samples_within_cone() {
        let source = Source::Led {
            position: Point3::origin(),
            direction: Vector3::z_axis(),
            half_angle_deg: 30.0,
            flux_lm: 1000.0,
        };
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let cos_limit = 30.0f64.to_radians().cos();
        for _ in 0..1000 {
            let ray = source.sample(&mut rng);
            let cos_angle = ray.direction.dot(Vector3::z_axis().as_ref());
            assert!(
                cos_angle >= cos_limit - 1e-6,
                "LED should sample within cone"
            );
        }
    }
}
