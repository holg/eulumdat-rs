//! Photometry for the path tracer: the intensity table as a GPU texture, a
//! matching CPU sampler, the colour-temperature tint and the exposure key.

use super::scene::{BuiltScene, SceneKind};
use eulumdat::Eulumdat;
use glam::{Mat3, Vec3};

/// Absolute candela on a uniform (C, γ) grid: `c_size` columns over 0..360°,
/// `g_size` rows over 0..180°. Row-major, rows are γ.
#[derive(Clone, Debug)]
pub struct IesTable {
    pub c_size: u32,
    pub g_size: u32,
    pub data: Vec<f32>,
    pub max_cd: f32,
    /// cd/klm → cd multiplier that was applied (lamp lumens / 1000).
    pub klm_scale: f32,
    /// Tables stacked in `data` (one block of `g_size` rows each): several
    /// luminaire types in one scene, picked per lamp.
    pub tables: u32,
}

impl IesTable {
    /// 1° in C, 0.5° in γ: 360 × 361 texels, 520 KB as R32Float.
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let c_size = 360u32;
        let g_size = 361u32;
        let flux = ldt.luminous_flux();
        let klm_scale = if flux > 0.0 {
            flux as f32 / 1000.0
        } else {
            1.0
        };
        let mut data = Vec::with_capacity((c_size * g_size) as usize);
        let mut max_cd: f32 = 0.0;
        for j in 0..g_size {
            let g = j as f64 * 180.0 / (g_size - 1) as f64;
            for i in 0..c_size {
                let c = i as f64 * 360.0 / c_size as f64;
                let v = (ldt.sample(c, g) as f32 * klm_scale).max(0.0);
                max_cd = max_cd.max(v);
                data.push(v);
            }
        }
        Self {
            c_size,
            g_size,
            data,
            max_cd,
            klm_scale,
            tables: 1,
        }
    }

    /// Several tables (same grid) stacked into one: table `k` is rows
    /// `k × g_size ..`. `max_cd` and `klm_scale` are the first table's.
    pub fn stack(tables: &[&IesTable]) -> Self {
        let first = tables[0];
        let mut data = Vec::with_capacity(first.data.len() * tables.len());
        for t in tables {
            assert_eq!((t.c_size, t.g_size), (first.c_size, first.g_size));
            data.extend_from_slice(&t.data[..(t.c_size * t.g_size) as usize]);
        }
        Self {
            c_size: first.c_size,
            g_size: first.g_size,
            data,
            max_cd: first.max_cd,
            klm_scale: first.klm_scale,
            tables: tables.len() as u32,
        }
    }

    /// Rows of the whole stack (the texture height).
    pub fn rows(&self) -> u32 {
        self.g_size * self.tables.max(1)
    }

    /// Bilinear candela at `(c°, γ°)`, matching the shader's lookup.
    pub fn sample(&self, c_deg: f32, g_deg: f32) -> f32 {
        self.sample_in(0, c_deg, g_deg)
    }

    /// [`Self::sample`] in table `table` of a stack.
    pub fn sample_in(&self, table: u32, c_deg: f32, g_deg: f32) -> f32 {
        let cs = self.c_size as i32;
        let gs = self.g_size as i32;
        let c = c_deg.rem_euclid(360.0);
        let g = g_deg.clamp(0.0, 180.0);
        let x = c / 360.0 * cs as f32;
        let y = g / 180.0 * (gs - 1) as f32;
        let xf = x.floor();
        let yf = y.floor();
        let tx = x - xf;
        let ty = (y - yf).clamp(0.0, 1.0);
        let x0 = (xf as i32).rem_euclid(cs);
        let x1 = (x0 + 1) % cs;
        let y0 = (yf as i32).clamp(0, gs - 1);
        let y1 = (y0 + 1).min(gs - 1);
        let base = (table.min(self.tables.max(1) - 1) * self.g_size) as i32;
        let at = |ix: i32, iy: i32| self.data[((base + iy) * cs + ix) as usize];
        let a = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
        let b = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
        a * (1.0 - ty) + b * ty
    }

    /// Candela towards world direction `dir` (unit, from the lamp) for a lamp
    /// with local→world `rotation`.
    pub fn sample_dir(&self, rotation: &Mat3, dir: Vec3) -> f32 {
        self.sample_dir_in(0, rotation, dir)
    }

    /// [`Self::sample_dir`] in table `table` of a stack.
    pub fn sample_dir_in(&self, table: u32, rotation: &Mat3, dir: Vec3) -> f32 {
        let l = rotation.transpose() * dir;
        let g = (-l.y).clamp(-1.0, 1.0).acos().to_degrees();
        let c = l.z.atan2(l.x).to_degrees();
        self.sample_in(table, c, g)
    }

    /// Little-endian f32 bytes, row-major, for an `R32Float` texture of
    /// `c_size × rows()` texels.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}

/// Size of the luminous opening and housing, in the luminaire frame
/// (`half_x` along C0, `half_z` along C90).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuminaireShape {
    pub half_x: f32,
    pub half_z: f32,
    pub round: bool,
    /// Housing half extents (x, y, z); the body sits above the luminous face.
    pub housing_half: Vec3,
}

pub fn luminaire_shape(ldt: &Eulumdat, kind: SceneKind) -> LuminaireShape {
    // LDT: width 0 marks a circular luminaire; luminous area dims preferred.
    let mut round = ldt.width <= 0.0 && ldt.length > 0.0;
    let mut l = if ldt.luminous_area_length > 0.0 {
        ldt.luminous_area_length
    } else {
        ldt.length
    } as f32
        / 1000.0;
    let mut w = if ldt.luminous_area_width > 0.0 {
        ldt.luminous_area_width
    } else {
        ldt.width
    } as f32
        / 1000.0;
    if round {
        w = l;
    }
    if l < 0.01 && w < 0.01 {
        if kind == SceneKind::Street {
            l = 0.55;
            w = 0.26;
            round = false;
        } else {
            l = 0.14;
            w = 0.14;
            round = true;
        }
    }
    if round {
        let r = (l.max(w) / 2.0).clamp(0.02, 0.4);
        LuminaireShape {
            half_x: r,
            half_z: r,
            round: true,
            housing_half: Vec3::new(r + 0.015, 0.035, r + 0.015),
        }
    } else {
        let hx = (l / 2.0).clamp(0.01, 0.8);
        let hz = (w / 2.0).clamp(0.01, 0.8);
        LuminaireShape {
            half_x: hx,
            half_z: hz,
            round: false,
            housing_half: Vec3::new(hx + 0.02, 0.035, hz + 0.02),
        }
    }
}

/// Correlated colour temperature → linear sRGB tint, luminance-normalised so
/// the temperature only shifts hue (Kim et al. Planckian locus fit, anchored
/// at 6500 K).
pub fn kelvin_to_linear_rgb(kelvin: f32) -> Vec3 {
    let raw = cct_to_linear_srgb_raw(kelvin).max(Vec3::ZERO);
    let d65 = cct_to_linear_srgb_raw(6500.0);
    let rel = raw / d65;
    let y = 0.2126 * rel.x + 0.7152 * rel.y + 0.0722 * rel.z;
    if y <= 1e-6 {
        return Vec3::ZERO;
    }
    rel / y
}

fn cct_to_linear_srgb_raw(k: f32) -> Vec3 {
    let t = k.clamp(1000.0, 15000.0) as f64;
    let t2 = t * t;
    let u = (0.860117757 + 1.54118254e-4 * t + 1.28641212e-7 * t2)
        / (1.0 + 8.42420235e-4 * t + 7.08145163e-7 * t2);
    let v = (0.317398726 + 4.22806245e-5 * t + 4.20481691e-8 * t2)
        / (1.0 - 2.89741816e-5 * t + 1.61456053e-7 * t2);
    let d = 2.0 * u - 8.0 * v + 4.0;
    let x = 3.0 * u / d;
    let y = 2.0 * v / d;
    let xx = x / y;
    let zz = (1.0 - x - y) / y;
    let r = 3.2406 * xx - 1.5372 - 0.4986 * zz;
    let g = -0.9689 * xx + 1.8758 + 0.0415 * zz;
    let b = 0.0557 * xx - 0.204 + 1.057 * zz;
    Vec3::new(r as f32, g as f32, b as f32)
}

/// sRGB hex for a swatch in the UI.
pub fn kelvin_to_srgb(kelvin: f32) -> Vec3 {
    let lin = cct_to_linear_srgb_raw(kelvin).max(Vec3::ZERO);
    let m = lin.max_element().max(1e-6);
    let enc = |c: f32| {
        let c = (c / m).clamp(0.0, 1.0);
        if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    };
    Vec3::new(enc(lin.x), enc(lin.y), enc(lin.z))
}

/// Direct illuminance on the receivers ignoring occlusion; used for the
/// exposure key and the false-colour scale.
pub fn estimate_illuminance(scene: &BuiltScene, table: &IesTable, dimming: f32) -> (f32, f32) {
    let mut max: f32 = 0.0;
    let mut sum = 0.0;
    let mut count = 0usize;
    const N: usize = 28;
    for r in &scene.receivers {
        for i in 0..=N {
            for j in 0..=N {
                let s = i as f32 / N as f32;
                let t = j as f32 / N as f32;
                let p = r.origin + r.u * s + r.v * t;
                let mut e = 0.0;
                for lamp in &scene.lamps {
                    let d = p - lamp.position;
                    let d2 = d.length_squared();
                    let dist = d2.sqrt();
                    if dist < 0.05 {
                        continue;
                    }
                    let dir = d / dist;
                    let cos_i = -r.normal.dot(dir);
                    if cos_i <= 0.0 {
                        continue;
                    }
                    e += table.sample_dir(&lamp.rotation, dir) * dimming * cos_i / d2;
                }
                max = max.max(e);
                if e > 0.0 {
                    sum += e;
                    count += 1;
                }
            }
        }
    }
    (max, if count > 0 { sum / count as f32 } else { 0.0 })
}

/// Rounds up to 1, 2, 2.5 or 5 × 10ⁿ.
pub fn nice_ceil(v: f32) -> f32 {
    if v.is_nan() || v <= 0.0 {
        return 1.0;
    }
    let p = 10f32.powf(v.log10().floor());
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if v <= m * p * 1.0001 {
            return m * p;
        }
    }
    10.0 * p
}

/// Illuminance at which a 30 % grey diffuse surface renders as mid-grey at
/// 0 EV: a fixed camera per setting, like a photographer's night-street and
/// interior presets.
pub fn camera_key_lux(scene: &BuiltScene) -> f32 {
    scene
        .key_lux
        .unwrap_or(if scene.outdoor { 30.0 } else { 150.0 })
}

pub fn exposure_for(key_lux: f32, ev: f32) -> f32 {
    (0.45 * std::f32::consts::PI) / (0.3 * key_lux) * 2f32.powf(ev)
}

/// µmol of PAR photons per lux-second for a spectrum given as (nm, value)
/// samples (relative or absolute units; the ratio cancels the scale).
#[cfg(feature = "spectrum")]
pub fn ppfd_per_lux(wavelengths_nm: &[f64], values: &[f64]) -> Option<f32> {
    use eulumdat_spectrum::constants::KM_PHOTOPIC;
    use eulumdat_spectrum::lut::photopic;
    use eulumdat_spectrum::metrics::ppfd_integral;
    use eulumdat_spectrum::spd::Spd;
    if wavelengths_nm.len() < 2 || wavelengths_nm.len() != values.len() {
        return None;
    }
    let coarse = Spd::new(wavelengths_nm, values);
    // 1 nm resampling so the 400–700 nm PAR edges are sharp.
    let start = wavelengths_nm[0].floor();
    let end = wavelengths_nm[wavelengths_nm.len() - 1].ceil();
    let n = ((end - start) as usize) + 1;
    let fine: Vec<f64> = (0..n).map(|i| coarse.value_at(start + i as f64)).collect();
    let spd = Spd::uniform(start, 1.0, &fine);
    let photopic_integral = spd.integral_weighted(photopic);
    if photopic_integral <= 1e-12 {
        return None;
    }
    let f = ppfd_integral(&spd) / (KM_PHOTOPIC * photopic_integral);
    (f.is_finite() && f > 0.0).then_some(f as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_ceil_steps() {
        assert_eq!(nice_ceil(3.0), 5.0);
        assert_eq!(nice_ceil(23.0), 25.0);
        assert_eq!(nice_ceil(100.0), 100.0);
        assert_eq!(nice_ceil(0.0), 1.0);
    }

    #[test]
    fn tint_is_neutral_at_d65_and_warm_below() {
        let n = kelvin_to_linear_rgb(6500.0);
        assert!(
            (n.x - 1.0).abs() < 0.02 && (n.z - 1.0).abs() < 0.02,
            "{n:?}"
        );
        let w = kelvin_to_linear_rgb(2700.0);
        assert!(w.x > w.z, "{w:?}");
    }
}
