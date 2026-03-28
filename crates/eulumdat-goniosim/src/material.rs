//! Two-layer material system: user-facing `MaterialParams` and internal `Material`.
//!
//! Users work with `MaterialParams` (datasheet values like reflectance %, IOR,
//! transmittance %, thickness, diffusion %). The tracer converts them to the
//! internal `Material` enum with physics coefficients automatically.

use crate::ray::{HitRecord, Photon, Ray};
use nalgebra::{Unit, Vector3};
use rand::Rng;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// User-facing: MaterialParams
// ---------------------------------------------------------------------------

/// Material description using manufacturer datasheet values.
///
/// These are the values you find on a material datasheet. A lighting designer
/// can fill these in without understanding Monte Carlo internals.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaterialParams {
    /// Human-readable name, e.g. "PMMA opal 3mm".
    pub name: String,

    /// Reflexionsgrad \[%\], 0-100.
    ///
    /// How much light is reflected at the surface.
    /// For opaque materials: total reflectance (diffuse + specular combined).
    /// For transparent materials: Fresnel reflectance is computed from IOR,
    /// this field is ignored (set to 0).
    pub reflectance_pct: f64,

    /// Brechungsindex (index of refraction).
    ///
    /// How much light bends when entering the material.
    /// PMMA: 1.49, glass: 1.52, polycarbonate: 1.585.
    /// Set to 0.0 for opaque materials (metal, paint).
    pub ior: f64,

    /// Lichtdurchlässigkeit \[%\], 0-100 at the given thickness.
    ///
    /// How much light passes through (measured at normal incidence).
    /// 0 = fully opaque, 92 = clear PMMA 3mm, 50 = heavy opal PMMA 3mm.
    pub transmittance_pct: f64,

    /// Dicke \[mm\].
    ///
    /// Material thickness. Affects volume scattering path length
    /// and Beer-Lambert absorption. Ignored for opaque materials.
    pub thickness_mm: f64,

    /// Streuungsgrad \[%\], 0-100.
    ///
    /// Degree of light diffusion/scattering.
    /// - 0 = perfectly clear (or mirror-specular for opaque)
    /// - 25 = satin/frosted
    /// - 60 = light opal
    /// - 95 = heavy opal (near-Lambertian exit distribution)
    /// - 100 = fully diffuse (matte paint for opaque)
    ///
    /// Maps directly to haze values in datasheets (e.g. Evonik Plexiglas).
    pub diffusion_pct: f64,
}

impl MaterialParams {
    /// Convert user-facing parameters to internal physics `Material`.
    pub fn to_material(&self) -> Material {
        let is_transparent = self.transmittance_pct > 0.0;
        let is_near_absorber = self.reflectance_pct < 2.0 && !is_transparent;

        if is_near_absorber {
            return Material::Absorber;
        }

        if is_transparent {
            let ior = if self.ior > 0.0 { self.ior } else { 1.49 };

            let min_refl = self.reflectance_pct / 100.0;

            if self.diffusion_pct < 5.0 {
                // Clear transmitter: Fresnel + Snell, Beer-Lambert absorption
                Material::ClearTransmitter {
                    ior,
                    transmittance: self.transmittance_pct / 100.0,
                    min_reflectance: min_refl,
                }
            } else {
                // Diffuse transmitter: volume scattering
                let thickness_m = self.thickness_mm / 1000.0;
                let tau = (self.transmittance_pct / 100.0).max(0.001);

                // The transmittance specifies the total fraction of light that
                // passes through the material. We need absorption to enforce this.
                //
                // Beer-Lambert gives us the absorption coefficient directly:
                // tau = exp(-mu_a * d)  →  mu_a = -ln(tau) / d
                //
                // Scattering (diffusion) redistributes light direction but does
                // not remove energy. The scattering coefficient controls how
                // diffuse the output is, independent of transmittance.
                let mu_a = -(tau.ln()) / thickness_m;

                // Scattering coefficient: controls angular spread, not loss.
                // Higher diffusion = more scattering events = more diffuse output.
                // Scale so 100% diffusion gives heavy scattering.
                let mu_s = (self.diffusion_pct / 100.0) * 2000.0; // [1/m]

                // Henyey-Greenstein asymmetry:
                // Low diffusion = forward-biased (g near 0.9)
                // High diffusion = near-isotropic (g near 0.0)
                let g = 0.9 * (1.0 - self.diffusion_pct / 100.0);

                Material::DiffuseTransmitter {
                    ior,
                    scattering_coeff: mu_s,
                    absorption_coeff: mu_a,
                    asymmetry: g,
                    thickness: thickness_m,
                    min_reflectance: min_refl,
                }
            }
        } else {
            // Opaque material
            let rho = self.reflectance_pct / 100.0;

            if self.diffusion_pct < 1.0 {
                Material::SpecularReflector { reflectance: rho }
            } else if self.diffusion_pct > 99.0 {
                Material::DiffuseReflector { reflectance: rho }
            } else {
                Material::MixedReflector {
                    reflectance: rho,
                    specular_fraction: 1.0 - self.diffusion_pct / 100.0,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal: Material enum (physics)
// ---------------------------------------------------------------------------

/// Internal material representation with physics coefficients.
/// Users don't construct these directly — they are derived from `MaterialParams`.
#[derive(Debug, Clone)]
pub enum Material {
    /// Perfect absorber — photon dies.
    Absorber,

    /// Lambertian (diffuse) reflector.
    DiffuseReflector {
        /// Reflectance rho, 0..1.
        reflectance: f64,
    },

    /// Specular (mirror) reflector.
    SpecularReflector {
        /// Reflectance rho, 0..1.
        reflectance: f64,
    },

    /// Mixed reflector (partly specular, partly diffuse).
    MixedReflector {
        /// Total reflectance rho, 0..1.
        reflectance: f64,
        /// Fraction of reflected light that is specular (0 = fully diffuse, 1 = fully specular).
        specular_fraction: f64,
    },

    /// Clear dielectric (glass, clear PMMA).
    ClearTransmitter {
        /// Index of refraction.
        ior: f64,
        /// Transmittance at normal incidence for the given thickness.
        transmittance: f64,
        /// Minimum reflectance at normal incidence (user-specified, 0..1).
        /// Used as max(fresnel, this) to allow coated surfaces.
        min_reflectance: f64,
    },

    /// Diffuse transmitter (opal/satin PMMA) with volume scattering.
    DiffuseTransmitter {
        /// Index of refraction.
        ior: f64,
        /// Scattering coefficient mu_s \[1/m\].
        scattering_coeff: f64,
        /// Absorption coefficient mu_a \[1/m\].
        absorption_coeff: f64,
        /// Henyey-Greenstein asymmetry parameter g (-1..1).
        asymmetry: f64,
        /// Slab thickness \[m\].
        thickness: f64,
        /// Minimum reflectance at normal incidence (user-specified, 0..1).
        min_reflectance: f64,
    },
}

/// Result of a photon-material interaction.
#[derive(Debug, Clone)]
pub enum Interaction {
    /// Photon was absorbed.
    Absorbed,
    /// Photon was reflected with a new ray and energy attenuation.
    Reflected {
        new_ray: Ray,
        attenuation: f64,
    },
    /// Photon was transmitted through the material.
    Transmitted {
        new_ray: Ray,
        attenuation: f64,
    },
}

impl Material {
    /// Compute the interaction of a photon with this material at a hit point.
    pub fn interact<R: Rng>(&self, photon: &Photon, hit: &HitRecord, rng: &mut R) -> Interaction {
        match self {
            Material::Absorber => Interaction::Absorbed,

            Material::DiffuseReflector { reflectance } => {
                // Russian roulette on reflectance
                if rng.random::<f64>() > *reflectance {
                    return Interaction::Absorbed;
                }
                let new_dir = random_cosine_hemisphere(&hit.normal, rng);
                Interaction::Reflected {
                    new_ray: Ray::new(hit.point + new_dir.as_ref() * 1e-6, new_dir),
                    attenuation: 1.0, // reflectance handled by RR above
                }
            }

            Material::SpecularReflector { reflectance } => {
                if rng.random::<f64>() > *reflectance {
                    return Interaction::Absorbed;
                }
                let reflected = reflect(&photon.ray.direction, &hit.normal);
                Interaction::Reflected {
                    new_ray: Ray::new(hit.point + reflected.as_ref() * 1e-6, reflected),
                    attenuation: 1.0,
                }
            }

            Material::MixedReflector {
                reflectance,
                specular_fraction,
            } => {
                if rng.random::<f64>() > *reflectance {
                    return Interaction::Absorbed;
                }
                let new_dir = if rng.random::<f64>() < *specular_fraction {
                    reflect(&photon.ray.direction, &hit.normal)
                } else {
                    random_cosine_hemisphere(&hit.normal, rng)
                };
                Interaction::Reflected {
                    new_ray: Ray::new(hit.point + new_dir.as_ref() * 1e-6, new_dir),
                    attenuation: 1.0,
                }
            }

            Material::ClearTransmitter { ior, transmittance, min_reflectance } => {
                interact_clear_transmitter(photon, hit, *ior, *transmittance, *min_reflectance, rng)
            }

            Material::DiffuseTransmitter {
                ior,
                scattering_coeff,
                absorption_coeff,
                asymmetry,
                thickness,
                min_reflectance,
            } => interact_diffuse_transmitter(
                photon,
                hit,
                *ior,
                *scattering_coeff,
                *absorption_coeff,
                *asymmetry,
                *thickness,
                *min_reflectance,
                rng,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Fresnel equations
// ---------------------------------------------------------------------------

/// Fresnel reflectance for unpolarized light (Schlick approximation).
fn fresnel_schlick(cos_theta: f64, ior_ratio: f64) -> f64 {
    let r0 = ((1.0 - ior_ratio) / (1.0 + ior_ratio)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5)
}

// ---------------------------------------------------------------------------
// Reflection / refraction helpers
// ---------------------------------------------------------------------------

/// Reflect a direction vector around a normal.
fn reflect(incoming: &Unit<Vector3<f64>>, normal: &Unit<Vector3<f64>>) -> Unit<Vector3<f64>> {
    let d = incoming.as_ref();
    let n = normal.as_ref();
    Unit::new_normalize(d - 2.0 * d.dot(n) * n)
}

/// Refract a direction vector through a surface (Snell's law).
/// Returns None for total internal reflection.
fn refract(
    incoming: &Unit<Vector3<f64>>,
    normal: &Unit<Vector3<f64>>,
    eta_ratio: f64,
) -> Option<Unit<Vector3<f64>>> {
    let cos_i = (-incoming.as_ref()).dot(normal.as_ref()).min(1.0);
    let sin2_t = eta_ratio * eta_ratio * (1.0 - cos_i * cos_i);
    if sin2_t > 1.0 {
        return None; // total internal reflection
    }
    let cos_t = (1.0 - sin2_t).sqrt();
    let refracted = eta_ratio * incoming.as_ref() + (eta_ratio * cos_i - cos_t) * normal.as_ref();
    Some(Unit::new_normalize(refracted))
}

// ---------------------------------------------------------------------------
// Random direction sampling
// ---------------------------------------------------------------------------

/// Sample a direction from cosine-weighted hemisphere around a normal.
fn random_cosine_hemisphere<R: Rng>(normal: &Unit<Vector3<f64>>, rng: &mut R) -> Unit<Vector3<f64>> {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    let r = u1.sqrt();
    let theta = 2.0 * PI * u2;
    let x = r * theta.cos();
    let y = r * theta.sin();
    let z = (1.0 - u1).sqrt();

    // Build orthonormal basis from normal
    let (tangent, bitangent) = build_onb(normal);
    let dir = x * tangent.as_ref() + y * bitangent.as_ref() + z * normal.as_ref();
    Unit::new_normalize(dir)
}

/// Sample a direction from the Henyey-Greenstein phase function.
fn sample_henyey_greenstein<R: Rng>(
    incoming: &Unit<Vector3<f64>>,
    g: f64,
    rng: &mut R,
) -> Unit<Vector3<f64>> {
    let xi: f64 = rng.random();
    let cos_theta = if g.abs() < 1e-6 {
        // Isotropic: uniform on sphere
        1.0 - 2.0 * xi
    } else {
        let term = (1.0 - g * g) / (1.0 - g + 2.0 * g * xi);
        (1.0 + g * g - term * term) / (2.0 * g)
    };
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    let phi = 2.0 * PI * rng.random::<f64>();

    let (tangent, bitangent) = build_onb(incoming);
    let dir = sin_theta * phi.cos() * tangent.as_ref()
        + sin_theta * phi.sin() * bitangent.as_ref()
        + cos_theta * incoming.as_ref();
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
// Clear transmitter interaction
// ---------------------------------------------------------------------------

fn interact_clear_transmitter<R: Rng>(
    photon: &Photon,
    hit: &HitRecord,
    ior: f64,
    transmittance: f64,
    min_reflectance: f64,
    rng: &mut R,
) -> Interaction {
    let (eta_ratio, cos_i) = if hit.front_face {
        (1.0 / ior, (-photon.ray.direction.as_ref()).dot(hit.normal.as_ref()).min(1.0))
    } else {
        (ior, (-photon.ray.direction.as_ref()).dot(hit.normal.as_ref()).min(1.0))
    };

    // Use the higher of Fresnel or user-specified reflectance
    let fresnel_r = fresnel_schlick(cos_i.abs(), eta_ratio).max(min_reflectance);

    if rng.random::<f64>() < fresnel_r {
        // Reflect
        let reflected = reflect(&photon.ray.direction, &hit.normal);
        Interaction::Reflected {
            new_ray: Ray::new(hit.point + reflected.as_ref() * 1e-6, reflected),
            attenuation: 1.0,
        }
    } else {
        // Transmit
        match refract(&photon.ray.direction, &hit.normal, eta_ratio) {
            Some(refracted) => {
                // Apply Beer-Lambert absorption for one surface pass.
                // Full transmittance is for both surfaces, so per-surface ~ sqrt(tau).
                let per_surface_tau = transmittance.sqrt();
                Interaction::Transmitted {
                    new_ray: Ray::new(hit.point + refracted.as_ref() * 1e-6, refracted),
                    attenuation: per_surface_tau,
                }
            }
            None => {
                // Total internal reflection
                let reflected = reflect(&photon.ray.direction, &hit.normal);
                Interaction::Reflected {
                    new_ray: Ray::new(hit.point + reflected.as_ref() * 1e-6, reflected),
                    attenuation: 1.0,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Diffuse transmitter interaction (volume scattering)
// ---------------------------------------------------------------------------

/// Diffuse transmitter: simplified thin-sheet model.
///
/// Instead of a full volume random walk, this treats the cover as a thin sheet:
/// 1. Fresnel reflection at entry surface (from IOR)
/// 2. Absorption: photon survives with probability = transmittance
/// 3. Diffusion: direction is scattered based on diffusion strength (HG parameter g)
/// 4. Fresnel at exit surface
///
/// This guarantees the specified transmittance is respected exactly,
/// while the scattering coefficient controls angular spread independently.
fn interact_diffuse_transmitter<R: Rng>(
    photon: &Photon,
    hit: &HitRecord,
    ior: f64,
    mu_s: f64,
    mu_a: f64,
    g: f64,
    thickness: f64,
    min_reflectance: f64,
    rng: &mut R,
) -> Interaction {
    let (eta_ratio, cos_i) = if hit.front_face {
        (1.0 / ior, (-photon.ray.direction.as_ref()).dot(hit.normal.as_ref()).min(1.0))
    } else {
        (ior, (-photon.ray.direction.as_ref()).dot(hit.normal.as_ref()).min(1.0))
    };

    // Entry surface: use the higher of Fresnel or user-specified reflectance
    let fresnel_r = fresnel_schlick(cos_i.abs(), eta_ratio).max(min_reflectance);
    if rng.random::<f64>() < fresnel_r {
        let reflected = reflect(&photon.ray.direction, &hit.normal);
        return Interaction::Reflected {
            new_ray: Ray::new(hit.point + reflected.as_ref() * 1e-6, reflected),
            attenuation: 1.0,
        };
    }

    // Absorption: transmittance = exp(-mu_a * thickness)
    // Photon survives with this probability
    let transmittance = (-mu_a * thickness).exp();
    if rng.random::<f64>() > transmittance {
        return Interaction::Absorbed;
    }

    // Refract into the material
    let refracted = match refract(&photon.ray.direction, &hit.normal, eta_ratio) {
        Some(r) => r,
        None => {
            let reflected = reflect(&photon.ray.direction, &hit.normal);
            return Interaction::Reflected {
                new_ray: Ray::new(hit.point + reflected.as_ref() * 1e-6, reflected),
                attenuation: 1.0,
            };
        }
    };

    // Apply angular diffusion: scatter the direction
    // mu_s > 0 means diffusion is active; g controls forward bias
    let exit_dir_internal = if mu_s > 0.0 {
        sample_henyey_greenstein(&refracted, g, rng)
    } else {
        refracted
    };

    // Exit surface: Fresnel
    let exit_eta = if hit.front_face { ior } else { 1.0 / ior };
    let cos_exit = exit_dir_internal.dot(hit.normal.as_ref()).abs().min(1.0);
    let exit_fresnel = fresnel_schlick(cos_exit, exit_eta);
    if rng.random::<f64>() < exit_fresnel {
        // Reflected back — treat as absorbed for simplicity
        // (in reality it would bounce around, but for a thin sheet this is rare)
        return Interaction::Absorbed;
    }

    // Refract out
    let exit_normal = if hit.front_face {
        Unit::new_unchecked(-hit.normal.into_inner())
    } else {
        hit.normal
    };
    let exit_dir = match refract(&exit_dir_internal, &exit_normal, exit_eta) {
        Some(d) => d,
        None => {
            return Interaction::Absorbed; // TIR
        }
    };

    let exit_point = hit.point + exit_normal.as_ref() * thickness + exit_dir.as_ref() * 1e-6;
    Interaction::Transmitted {
        new_ray: Ray::new(exit_point, exit_dir),
        attenuation: 1.0, // absorption already handled above
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn clear_pmma_produces_clear_transmitter() {
        let params = catalog::clear_pmma_3mm();
        let mat = params.to_material();
        match mat {
            Material::ClearTransmitter { ior, transmittance, min_reflectance } => {
                assert!((ior - 1.49).abs() < 0.01);
                assert!((transmittance - 0.92).abs() < 0.01);
                assert!((min_reflectance - 0.04).abs() < 0.01);
            }
            _ => panic!("Expected ClearTransmitter, got {:?}", mat),
        }
    }

    #[test]
    fn opal_pmma_produces_diffuse_transmitter() {
        let params = catalog::opal_pmma_3mm();
        let mat = params.to_material();
        match mat {
            Material::DiffuseTransmitter {
                ior,
                scattering_coeff,
                absorption_coeff,
                asymmetry,
                thickness,
                min_reflectance,
            } => {
                assert!((ior - 1.49).abs() < 0.01);
                assert!(scattering_coeff > 0.0);
                assert!(absorption_coeff > 0.0);
                assert!(asymmetry < 0.1, "High diffusion should give low asymmetry");
                assert!((thickness - 0.003).abs() < 0.0001);
                assert!((min_reflectance - 0.04).abs() < 0.01);
            }
            _ => panic!("Expected DiffuseTransmitter, got {:?}", mat),
        }
    }

    #[test]
    fn white_paint_produces_diffuse_reflector() {
        let params = catalog::white_paint();
        let mat = params.to_material();
        match mat {
            Material::DiffuseReflector { reflectance } => {
                assert!((reflectance - 0.85).abs() < 0.01);
            }
            _ => panic!("Expected DiffuseReflector, got {:?}", mat),
        }
    }

    #[test]
    fn mirror_produces_specular_reflector() {
        let params = catalog::mirror_aluminum();
        let mat = params.to_material();
        match mat {
            Material::SpecularReflector { reflectance } => {
                assert!((reflectance - 0.95).abs() < 0.01);
            }
            _ => panic!("Expected SpecularReflector, got {:?}", mat),
        }
    }

    #[test]
    fn matte_black_near_absorber() {
        let params = catalog::matte_black();
        let mat = params.to_material();
        // 5% reflectance is above the 2% absorber threshold
        match mat {
            Material::DiffuseReflector { reflectance } => {
                assert!((reflectance - 0.05).abs() < 0.01);
            }
            _ => panic!("Expected DiffuseReflector, got {:?}", mat),
        }
    }

    #[test]
    fn anodized_aluminum_produces_mixed_reflector() {
        let params = catalog::anodized_aluminum();
        let mat = params.to_material();
        match mat {
            Material::MixedReflector {
                reflectance,
                specular_fraction,
            } => {
                assert!((reflectance - 0.80).abs() < 0.01);
                assert!((specular_fraction - 0.30).abs() < 0.01);
            }
            _ => panic!("Expected MixedReflector, got {:?}", mat),
        }
    }
}
