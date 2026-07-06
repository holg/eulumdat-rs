//! Road-surface luminance (cd/m²) — the observer-dependent metric that road
//! lighting standards (EN 13201, RP-8, CJJ 45) actually judge, alongside
//! illuminance.
//!
//! Unlike illuminance (`E = I·cosε/r²`, observer-independent), **luminance**
//! depends on the road's reflection properties and the *observer's* position:
//! a wet/specular road throws light toward a low observer very differently than
//! a matte one. The CIE captures this with **reduced luminance coefficient**
//! tables `r(β, tan ε)` per road class (R1 dry-diffuse … R4 wet-specular):
//!
//! ```text
//! L = Σ_sources  I(C, γ) · r(β, tan ε) / (H² · 10⁴)        [cd/m²]
//! ```
//!
//! where, for each source seen from a road point:
//! - `I(C, γ)` is the luminous intensity toward the point (cd), from the LDT;
//! - `H` is the mounting height (m);
//! - `ε` is the angle of light incidence at the point (from vertical) — so
//!   `tan ε = horizontal_distance / H`;
//! - `β` is the angle between the **vertical plane of incidence** and the
//!   **vertical plane of observation** (observer → point), in degrees.
//!
//! The same machinery serves **night** (luminaires as sources) and **day**
//! (sun + sky patches as sources) — that is the whole point of putting it here.
//!
//! Geometry conventions mirror [`crate::area::compute`] exactly (road along +X,
//! +Z up, luminaire Type-C with C0 via `rotation`, `gamma = acos(-dz/r)`).

mod r_tables;

pub use r_tables::{RTable, RTableClass};

use crate::Eulumdat;

/// An observer for road-luminance assessment (EN 13201 / RP-8 geometry).
///
/// Standard placement: eye 1.5 m above the road, looking along the road in the
/// direction of travel, with assessed road points 60–160 m ahead (≈1° downward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Observer {
    /// Observer eye position (x, y, z) in meters; road surface at z = 0.
    pub position: [f64; 3],
    /// Unit-ish view direction (will be normalized); typically along +road, slightly down.
    pub view_dir: [f64; 3],
}

impl Observer {
    /// EN 13201 standard observer: 1.5 m eye height, on the given lane center
    /// `x`-coordinate (across road), at `y_behind` meters before the assessed
    /// field, looking along +Y (the road direction here).
    pub fn en13201(lane_x: f64, y_behind: f64) -> Self {
        Self {
            position: [lane_x, y_behind, 1.5],
            view_dir: [0.0, 1.0, 0.0],
        }
    }
}

/// One light source contributing to road luminance: where it is, how high, and
/// how its intensity is sampled toward a road point.
///
/// For a **luminaire** this wraps an [`Eulumdat`] + placement. For **daylight**
/// (sun / sky patch) the intensity is a fixed luminous-intensity value with a
/// fixed world direction (built by the `daylight` feature helpers).
#[derive(Clone)]
pub struct LuminanceSource<'a> {
    /// Effective source position (x, y, z) in meters (luminaire head, z = H).
    pub position: [f64; 3],
    /// Mounting height H (m) used in the `/H²` term. For daylight use the
    /// reference height the intensity was expressed at (see helpers).
    pub mounting_height: f64,
    /// How to obtain luminous intensity (cd) toward a given road point.
    pub intensity: IntensityModel<'a>,
}

/// Strategy for the luminous intensity a source sends toward a road point.
#[derive(Clone)]
pub enum IntensityModel<'a> {
    /// LDT-driven luminaire: sample `ldt` at the Type-C `(C, γ)` toward the
    /// point, scaled by `flux_scale` (= total_flux/1000 × maintenance) and
    /// oriented by `tilt_rad` / `rotation_rad` (same as `area::compute`).
    Luminaire {
        ldt: &'a Eulumdat,
        flux_scale: f64,
        tilt_rad: f64,
        rotation_rad: f64,
    },
    /// Constant luminous intensity (cd) regardless of direction — used for a
    /// distant uniform contribution (e.g. a sky patch or the sun beam already
    /// resolved to an intensity at the reference height).
    Constant(f64),
}

/// Result of evaluating a luminance grid.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LuminanceMetrics {
    /// Average road luminance, cd/m².
    pub l_avg: f64,
    /// Minimum road luminance, cd/m².
    pub l_min: f64,
    /// Maximum road luminance, cd/m².
    pub l_max: f64,
    /// Overall uniformity U0 = L_min / L_avg.
    pub u0: f64,
    /// Longitudinal uniformity Ul = L_min / L_max (along the lane).
    pub ul: f64,
    /// Threshold Increment, % (disability glare). `None` semantics → 0.0 here.
    pub ti_pct: f64,
}

/// Luminance (cd/m²) at one road point, summed over all sources, for a given
/// observer and road class.
pub fn point_luminance(
    obs: &Observer,
    point: [f64; 3],
    sources: &[LuminanceSource<'_>],
    rtable: &RTable,
) -> f64 {
    let mut l = 0.0;
    // Observation azimuth: horizontal direction from the road point to the
    // observer (the vertical plane of observation).
    let obs_az = {
        let ox = obs.position[0] - point[0];
        let oy = obs.position[1] - point[1];
        oy.atan2(ox)
    };

    for s in sources {
        let intensity = source_intensity(s, point);
        if intensity <= 0.0 {
            continue;
        }
        let h = s.mounting_height.max(1e-6);

        // Incidence geometry at the road point.
        let dx = s.position[0] - point[0];
        let dy = s.position[1] - point[1];
        let horiz = (dx * dx + dy * dy).sqrt();
        let tan_eps = horiz / h;

        // β: angle between the vertical plane of incidence (point → source,
        // horizontally) and the vertical plane of observation (point → observer).
        let inc_az = dy.atan2(dx);
        let mut beta = (inc_az - obs_az).abs();
        // Fold into [0, π]; the r-table is symmetric in β.
        if beta > std::f64::consts::PI {
            beta = 2.0 * std::f64::consts::PI - beta;
        }

        let r_coeff = rtable.r(beta, tan_eps); // already r·10⁴
        // L = I · (r·10⁴) / (H² · 10⁴) = I · r / H²; the 10⁴ cancels because the
        // table stores r·10⁴ and the CIE formula divides by 10⁴.
        l += intensity * r_coeff / (h * h * 1.0e4);
    }
    l.max(0.0)
}

/// Luminous intensity (cd) a source directs at a road point.
fn source_intensity(s: &LuminanceSource<'_>, point: [f64; 3]) -> f64 {
    match &s.intensity {
        IntensityModel::Constant(i) => *i,
        IntensityModel::Luminaire {
            ldt,
            flux_scale,
            tilt_rad,
            rotation_rad,
        } => {
            let dx = point[0] - s.position[0];
            let dy = point[1] - s.position[1];
            let dz = point[2] - s.position[2]; // negative (point below head)
            let r = (dx * dx + dy * dy + dz * dz).sqrt();
            if r < 1e-6 {
                return 0.0;
            }
            // Mirror area::compute_single_illuminance: rotate by -rotation (Z),
            // then -tilt (Y), into luminaire-local; derive Type-C (C, γ).
            let (cos_r, sin_r) = (rotation_rad.cos(), rotation_rad.sin());
            let dx_r = dx * cos_r + dy * sin_r;
            let dy_r = -dx * sin_r + dy * cos_r;
            let dz_r = dz;
            let (cos_t, sin_t) = (tilt_rad.cos(), tilt_rad.sin());
            let dx_rot = dx_r * cos_t + dz_r * sin_t;
            let dy_rot = dy_r;
            let dz_rot = -dx_r * sin_t + dz_r * cos_t;

            let gamma = (-dz_rot / r).clamp(-1.0, 1.0).acos();
            let mut c_deg = dy_rot.atan2(dx_rot).to_degrees();
            if c_deg < 0.0 {
                c_deg += 360.0;
            }
            (ldt.sample(c_deg, gamma.to_degrees()) * flux_scale).max(0.0)
        }
    }
}

/// Evaluate a grid of road points → luminance metrics.
///
/// `grid[row]` runs **along the road** (longitudinal), columns across — so
/// longitudinal uniformity `Ul` is computed per-lane (min/max along the worst
/// column). Each cell is a `[x, y, 0.0]` road point.
pub fn evaluate_grid(
    obs: &Observer,
    rows: &[Vec<[f64; 3]>],
    sources: &[LuminanceSource<'_>],
    rtable: &RTable,
) -> LuminanceMetrics {
    if rows.is_empty() || rows[0].is_empty() {
        return LuminanceMetrics::default();
    }
    let n_cols = rows[0].len();
    let mut lum = vec![vec![0.0f64; n_cols]; rows.len()];
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut l_min = f64::INFINITY;
    let mut l_max: f64 = 0.0;

    for (ri, row) in rows.iter().enumerate() {
        for (ci, &pt) in row.iter().enumerate() {
            let l = point_luminance(obs, pt, sources, rtable);
            lum[ri][ci] = l;
            sum += l;
            count += 1;
            l_min = l_min.min(l);
            l_max = l_max.max(l);
        }
    }
    let l_avg = if count > 0 { sum / count as f64 } else { 0.0 };
    if !l_min.is_finite() {
        l_min = 0.0;
    }

    // Longitudinal uniformity: worst (smallest) min/max taken column-by-column
    // along the road (each column is a fixed transverse position).
    let mut ul: f64 = 1.0;
    for ci in 0..n_cols {
        let mut cmin = f64::INFINITY;
        let mut cmax = 0.0f64;
        for row in &lum {
            cmin = cmin.min(row[ci]);
            cmax = cmax.max(row[ci]);
        }
        if cmax > 0.0 {
            ul = ul.min(cmin / cmax);
        }
    }

    LuminanceMetrics {
        l_avg,
        l_min,
        l_max,
        u0: if l_avg > 0.0 { l_min / l_avg } else { 0.0 },
        ul,
        ti_pct: 0.0,
    }
}

/// Daytime helpers: build sun + sky [`LuminanceSource`]s from a solar position
/// and sky availability, so road luminance/illuminance works under daylight
/// using the exact same machinery as night luminaires.
///
/// Enabled by the `daylight` feature (pulls in `eulumdat-daylight`).
#[cfg(feature = "daylight")]
pub mod daytime {
    use super::{IntensityModel, LuminanceSource};
    use eulumdat_daylight::{
        coords::{dome_to_world, WorldDir},
        solar::SolarPosition,
        SkyRadiance,
    };

    /// Reference distance (m) at which a daylight source's luminous intensity is
    /// expressed, so it folds into the `/H²` term of [`super::point_luminance`].
    /// Daylight is effectively at infinity; we use a large fixed "mounting
    /// height" and convert the illuminance/luminance arriving at the road into
    /// an equivalent point-source intensity `I = E · H²` at that distance, which
    /// makes the `I·r/(H²·10⁴)` formula reproduce the intended road luminance.
    const DAYLIGHT_REF_HEIGHT_M: f64 = 100.0;

    /// Build a single far-field **sun** source from DNI and the sun direction.
    ///
    /// The sun delivers `dni_lux` on a surface normal to the beam. On the road
    /// (horizontal) the incidence factor is handled by the r-table geometry; we
    /// express the beam as an equivalent intensity `E·H²` at the reference
    /// height placed along the (up-going) sun direction.
    pub fn sun_source(sun: &SolarPosition, dni_lux: f64) -> Option<LuminanceSource<'static>> {
        if !sun.is_daytime() || dni_lux <= 0.0 {
            return None;
        }
        let d: WorldDir = sun.world_direction();
        let h = DAYLIGHT_REF_HEIGHT_M;
        // Place the "source" up along the sun direction at the reference height.
        let up = d.up().max(1e-3);
        let scale = h / up; // so the source sits at height h above the road
        Some(LuminanceSource {
            position: [d.x * scale, d.z * scale, h],
            mounting_height: h,
            // Equivalent intensity so I/H² reproduces the normal illuminance.
            intensity: IntensityModel::Constant(dni_lux * h * h),
        })
    }

    /// Discretize the sky dome into `n_theta × n_phi` patches, each an
    /// equivalent far-field [`LuminanceSource`], so the diffuse sky contributes
    /// to road luminance through the same r-table path.
    pub fn sky_sources(sky: &SkyRadiance, n_theta: usize, n_phi: usize) -> Vec<LuminanceSource<'static>> {
        use std::f64::consts::{FRAC_PI_2, PI};
        let mut out = Vec::with_capacity(n_theta * n_phi);
        let dtheta = FRAC_PI_2 / n_theta as f64;
        let dphi = 2.0 * PI / n_phi as f64;
        let h = DAYLIGHT_REF_HEIGHT_M;
        for i in 0..n_theta {
            let theta = (i as f64 + 0.5) * dtheta;
            // Solid angle of this patch: sinθ dθ dφ.
            let domega = theta.sin() * dtheta * dphi;
            for j in 0..n_phi {
                let phi = (j as f64 + 0.5) * dphi;
                let lum = sky.luminance(theta, phi); // cd/m²
                if lum <= 0.0 {
                    continue;
                }
                let d = dome_to_world(theta, phi);
                let up = d.up().max(1e-3);
                // Illuminance this patch puts on a horizontal surface = L·cosθ·dΩ.
                let e_patch = lum * theta.cos() * domega;
                let scale = h / up;
                out.push(LuminanceSource {
                    position: [d.x * scale, d.z * scale, h],
                    mounting_height: h,
                    intensity: IntensityModel::Constant(e_patch * h * h),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_ldt(intensity_cd_klm: f64) -> Eulumdat {
        // Minimal rotationally-symmetric LDT: constant intensity everywhere.
        let mut ldt = Eulumdat::new();
        ldt.symmetry = crate::Symmetry::VerticalAxis;
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0];
        ldt.num_c_planes = 1;
        ldt.num_g_planes = ldt.g_angles.len();
        ldt.intensities = vec![vec![intensity_cd_klm; ldt.g_angles.len()]];
        ldt.lamp_sets = vec![crate::LampSet {
            num_lamps: 1,
            total_luminous_flux: 1000.0,
            ..Default::default()
        }];
        ldt
    }

    #[test]
    fn night_luminaire_luminance_in_m_class_band() {
        // A single luminaire over a lane, R3 road. Check the road luminance at a
        // point ahead of the observer lands in a plausible M-class band.
        let ldt = flat_ldt(300.0); // 300 cd/klm → 300 cd at 1000 lm
        let h = 8.0;
        let src = LuminanceSource {
            position: [0.0, 0.0, h],
            mounting_height: h,
            intensity: IntensityModel::Luminaire {
                ldt: &ldt,
                flux_scale: 1.0, // 1000 lm / 1000
                tilt_rad: 0.0,
                rotation_rad: 0.0,
            },
        };
        let obs = Observer::en13201(0.0, -60.0);
        let rtable = RTable::standard(RTableClass::R3);
        // Point 30 m ahead, same lane.
        let l = point_luminance(&obs, [0.0, 30.0, 0.0], &[src], &rtable);
        assert!(
            (0.05..=5.0).contains(&l),
            "road luminance {l} cd/m² outside plausible band"
        );
    }

    #[test]
    fn zero_intensity_gives_zero_luminance() {
        let ldt = flat_ldt(0.0);
        let src = LuminanceSource {
            position: [0.0, 0.0, 8.0],
            mounting_height: 8.0,
            intensity: IntensityModel::Luminaire {
                ldt: &ldt,
                flux_scale: 1.0,
                tilt_rad: 0.0,
                rotation_rad: 0.0,
            },
        };
        let obs = Observer::en13201(0.0, -60.0);
        let rtable = RTable::standard(RTableClass::R3);
        assert_eq!(point_luminance(&obs, [0.0, 30.0, 0.0], &[src], &rtable), 0.0);
    }

    #[cfg(feature = "daylight")]
    #[test]
    fn daytime_sun_sky_illuminate_road() {
        use eulumdat_daylight::{
            availability::DaylightAvailability, sky::PerezSky, sky::SkyParams, solar::solar_position,
            SkyRadiance,
        };

        // Clear summer noon at 40°N.
        let sun = solar_position(2026, 6, 21, 12.0, 40.0, 0.0);
        let avail = DaylightAvailability::clear_sky(&sun, 2.5);
        assert!(avail.ghi_lux > 60_000.0);

        // Build sun + sky luminance sources.
        let mut sources = Vec::new();
        if let Some(s) = super::daytime::sun_source(&sun, avail.dni_lux) {
            sources.push(s);
        }
        let perez = PerezSky::new(sun, SkyParams::from_turbidity(2.5));
        let sky = SkyRadiance::perez_from_dhi(perez, avail.dhi_lux);
        sources.extend(super::daytime::sky_sources(&sky, 6, 12));
        assert!(sources.len() > 1, "should have sun + sky patches");

        // Road luminance under daylight, R3 road, standard observer.
        let obs = Observer::en13201(0.0, -60.0);
        let rtable = RTable::standard(RTableClass::R3);
        let l = point_luminance(&obs, [0.0, 30.0, 0.0], &sources, &rtable);
        // Daytime road luminance is far above any night M-class value.
        assert!(
            l > 100.0,
            "daytime road luminance {l} cd/m² unexpectedly low"
        );
    }

    #[test]
    fn grid_uniformities_bounded() {
        let ldt = flat_ldt(300.0);
        let h = 8.0;
        let sources: Vec<LuminanceSource> = (0..3)
            .map(|i| LuminanceSource {
                position: [0.0, i as f64 * 30.0, h],
                mounting_height: h,
                intensity: IntensityModel::Luminaire {
                    ldt: &ldt,
                    flux_scale: 1.0,
                    tilt_rad: 0.0,
                    rotation_rad: 0.0,
                },
            })
            .collect();
        let obs = Observer::en13201(0.0, -60.0);
        let rtable = RTable::standard(RTableClass::R3);
        let rows: Vec<Vec<[f64; 3]>> = (0..10)
            .map(|r| {
                (0..4)
                    .map(|c| [c as f64 * 1.0 - 1.5, 10.0 + r as f64 * 3.0, 0.0])
                    .collect()
            })
            .collect();
        let m = evaluate_grid(&obs, &rows, &sources, &rtable);
        assert!(m.l_avg > 0.0);
        assert!((0.0..=1.0).contains(&m.u0));
        assert!((0.0..=1.0).contains(&m.ul));
        assert!(m.l_min <= m.l_avg && m.l_avg <= m.l_max);
    }
}

/// Mesopic (CIE 191:2010) road-luminance correction.
///
/// Road luminances (0.3–2 cd/m²) fall in the mesopic range where the eye is
/// neither photopic nor scotopic, so a photopic-only luminance misstates what a
/// driver perceives — and the error depends on the lamp's spectrum (via its S/P
/// ratio). This turns the photopic [`LuminanceMetrics`] into mesopic-corrected
/// metrics using the shared [`eulumdat_spectrum::mesopic`] solver.
///
/// Enabled by the `spectrum` feature (pulls in `eulumdat-spectrum`).
#[cfg(feature = "spectrum")]
pub mod mesopic {
    use super::LuminanceMetrics;
    use eulumdat_spectrum::mesopic::mesopic_luminance;

    /// Convert photopic road-luminance metrics to CIE 191 mesopic ones, given
    /// the luminaire's scotopic/photopic ratio.
    ///
    /// Each luminance (avg/min/max) is mapped through the CIE 191 fixed-point
    /// system independently; the uniformities are recomputed from the corrected
    /// avg/min/max. A high-S/P (bluish) source raises the perceived luminance at
    /// these levels; a low-S/P (amber) source lowers it.
    pub fn correct(metrics: &LuminanceMetrics, sp_ratio: f64) -> LuminanceMetrics {
        let l_avg = mesopic_luminance(metrics.l_avg, sp_ratio);
        let l_min = mesopic_luminance(metrics.l_min, sp_ratio);
        let l_max = mesopic_luminance(metrics.l_max, sp_ratio);
        LuminanceMetrics {
            l_avg,
            l_min,
            l_max,
            u0: if l_avg > 0.0 { l_min / l_avg } else { 0.0 },
            // Longitudinal uniformity is a ratio of luminances; the monotone
            // mesopic map does not preserve it exactly, so approximate from the
            // corrected extremes (a tighter estimate needs per-point correction).
            ul: if l_max > 0.0 { l_min / l_max } else { 0.0 },
            ti_pct: metrics.ti_pct,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn cool_source_raises_perceived_road_luminance() {
            let photopic = LuminanceMetrics {
                l_avg: 1.0,
                l_min: 0.6,
                l_max: 1.8,
                u0: 0.6,
                ul: 0.33,
                ti_pct: 0.0,
            };
            let cool = correct(&photopic, 2.2);
            let warm = correct(&photopic, 0.6);
            assert!(cool.l_avg > photopic.l_avg, "high S/P raises mesopic luminance");
            assert!(warm.l_avg < photopic.l_avg, "low S/P lowers mesopic luminance");
            assert!(cool.l_avg > warm.l_avg);
        }
    }
}
