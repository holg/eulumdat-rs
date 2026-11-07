//! ANSI/IES TM-30-20 colour rendition (fidelity Rf = CIE 224:2017).
//!
//! From a spectrum: Rf (fidelity), Rg (gamut), the 16 hue-bin values
//! Rf,hj / Rcs,hj / Rhs,hj and the normalised colour vector graphic.
//!
//! Method (CIE 224:2017, TM-30-20):
//! - spectrum on 380–780 nm at 5 nm, zero outside the measured range;
//! - CCT / Duv with the CIE 1931 2° observer (`colour_common::cct_duv`);
//! - reference: Planckian below 4000 K, CIE daylight above 5000 K, a blend
//!   of both (each normalised to Y = 1) in between;
//! - 99 colour evaluation samples, CIE 1964 10° observer, CIECAM02 with
//!   L_A = 100 cd/m², Y_b = 20, average surround, complete adaptation;
//!   CAM02-UCS J′a′b′;
//! - Rf = 10·ln(1 + exp((100 − 6.73·ΔE)/10)) on the mean ΔE (and per sample /
//!   per bin);
//! - 16 hue bins of 22.5° from the reference hue angle; Rg from the areas of
//!   the bin-average polygons; Rcs,hj / Rhs,hj are the radial / tangential
//!   shift of the bin average (bin-centre direction) over the reference
//!   chroma.
//!
//! The implementation is our own; its results are compared with an
//! independent implementation (colour-science) in the tests.

use super::cie_tables::N;
use super::colour_common::{
    covers_visible, daylight, measured_cct_duv, planck, resample, tristimulus, Grid, OBSERVER_10,
    OBSERVER_2,
};
use crate::atla::types::SpectralDistribution;

/// Number of TM-30 colour evaluation samples.
pub const NUM_CES: usize = 99;
/// Number of hue bins.
pub const NUM_BINS: usize = 16;

/// The standard TM-30 hue-bin colours (bins 1–16), used for the vector
/// graphic and the per-bin charts.
pub const TM30_BIN_COLOURS: [&str; NUM_BINS] = [
    "#E62828", "#E74B4B", "#FB812E", "#FFB529", "#CBCA46", "#7EB94C", "#41C06D", "#009C7C",
    "#16BCB0", "#00A4BF", "#0085C3", "#3B62AA", "#4568AE", "#6A4E85", "#9D69A1", "#A74F81",
];

/// Reference illuminant the test source is compared with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tm30Reference {
    /// Planckian radiator (CCT < 4000 K).
    Planckian,
    /// CIE daylight (CCT > 5000 K).
    Daylight,
    /// Blend for 4000–5000 K; `daylight_share` = (CCT − 4000) / 1000.
    Blend { daylight_share: f64 },
}

/// TM-30 calculation results.
#[derive(Debug, Clone)]
pub struct Tm30Result {
    /// Fidelity index Rf (0–100).
    pub rf: f64,
    /// Gamut index Rg (100 = same average saturation as the reference).
    pub rg: f64,
    /// Correlated colour temperature (K), CIE 1931 2° observer.
    pub cct: f64,
    /// Distance from the Planckian locus in CIE 1960 uv (positive = above).
    pub duv: f64,
    /// Local fidelity Rf,hj per hue bin.
    pub rf_hue: [f64; NUM_BINS],
    /// Local chroma shift Rcs,hj per hue bin, in percent (+ = more saturated).
    pub rcs_hue: [f64; NUM_BINS],
    /// Local hue shift Rhs,hj per hue bin (radians, as in TM-30).
    pub rhs_hue: [f64; NUM_BINS],
    /// Bin averages in CAM02-UCS: (test a′, test b′, reference a′, reference b′).
    pub color_vectors: [(f64, f64, f64, f64); NUM_BINS],
    /// Mean reference hue angle of the samples in each bin (degrees).
    pub ref_hue_deg: [f64; NUM_BINS],
    /// Fidelity of each colour evaluation sample, Rf,CES1…CES99.
    pub rf_ces: Vec<f64>,
    /// Reference illuminant used.
    pub reference: Tm30Reference,
}

/// One bin of the normalised colour vector graphic: the reference point on
/// the unit circle and the test point.
#[derive(Debug, Clone, Copy)]
pub struct CvgPoint {
    pub reference: (f64, f64),
    pub test: (f64, f64),
}

/// Theme for the TM-30 graphics.
#[derive(Debug, Clone)]
pub struct Tm30Theme {
    pub background: String,
    pub foreground: String,
    pub grid_color: String,
    pub reference_color: String,
    pub test_color: String,
    pub font_family: String,
}

impl Default for Tm30Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Tm30Theme {
    pub fn light() -> Self {
        Self {
            background: "#ffffff".to_string(),
            foreground: "#1f2937".to_string(),
            grid_color: "#d1d5db".to_string(),
            reference_color: "#6b7280".to_string(),
            test_color: "#c2410c".to_string(),
            font_family: "system-ui, -apple-system, sans-serif".to_string(),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: "#111827".to_string(),
            foreground: "#e5e7eb".to_string(),
            grid_color: "#374151".to_string(),
            reference_color: "#9ca3af".to_string(),
            test_color: "#fb923c".to_string(),
            font_family: "system-ui, -apple-system, sans-serif".to_string(),
        }
    }
}

// TM-30 colour evaluation samples (99 reflectances, 380–780 nm at 5 nm).
#[allow(clippy::approx_constant)]
mod ces_data {
    include!("tm30_ces_data.rs");
}
use ces_data::CES_REFLECTANCE;

// ============================================================================
// CIECAM02 → CAM02-UCS (viewing conditions of CIE 224:2017)
// ============================================================================

const M_CAT02: [[f64; 3]; 3] = [
    [0.7328, 0.4296, -0.1624],
    [-0.7036, 1.6975, 0.0061],
    [0.0030, 0.0136, 0.9834],
];
const M_HPE: [[f64; 3]; 3] = [
    [0.38971, 0.68898, -0.07868],
    [-0.22981, 1.18340, 0.04641],
    [0.0, 0.0, 1.0],
];
const L_A: f64 = 100.0;
const Y_B: f64 = 20.0;
const SURROUND_C: f64 = 0.69;
const SURROUND_NC: f64 = 1.0;

fn mat_vec(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

fn mat_inv(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let inv = 1.0 / det;
    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ]
}

/// Signed power, as used by CIECAM02 for (rare) negative responses.
fn spow(x: f64, p: f64) -> f64 {
    x.signum() * x.abs().powf(p)
}

/// Viewing-condition dependent quantities for one adopted white.
struct Cam02 {
    d_rgb: [f64; 3],
    to_hpe: [[f64; 3]; 3],
    f_l: f64,
    n: f64,
    z: f64,
    n_bb: f64,
    a_w: f64,
}

impl Cam02 {
    fn new(xyz_w: [f64; 3]) -> Self {
        let y_w = xyz_w[1];
        let k = 1.0 / (5.0 * L_A + 1.0);
        let k4 = k.powi(4);
        let f_l = 0.2 * k4 * (5.0 * L_A) + 0.1 * (1.0 - k4).powi(2) * (5.0 * L_A).powf(1.0 / 3.0);
        let n = Y_B / y_w;
        let n_bb = 0.725 * (1.0 / n).powf(0.2);
        let z = 1.48 + n.sqrt();
        let rgb_w = mat_vec(&M_CAT02, xyz_w);
        // Complete adaptation (D = 1, "discount the illuminant").
        let d_rgb = [y_w / rgb_w[0], y_w / rgb_w[1], y_w / rgb_w[2]];
        let to_hpe = mat_mul(&M_HPE, &mat_inv(&M_CAT02));
        let mut me = Self {
            d_rgb,
            to_hpe,
            f_l,
            n,
            z,
            n_bb,
            a_w: 1.0,
        };
        let rgb_aw = me.compressed(xyz_w);
        me.a_w = (2.0 * rgb_aw[0] + rgb_aw[1] + rgb_aw[2] / 20.0 - 0.305) * n_bb;
        me
    }

    /// Post-adaptation, compressed cone responses R′a G′a B′a.
    fn compressed(&self, xyz: [f64; 3]) -> [f64; 3] {
        let rgb = mat_vec(&M_CAT02, xyz);
        let rgb_c = [
            self.d_rgb[0] * rgb[0],
            self.d_rgb[1] * rgb[1],
            self.d_rgb[2] * rgb[2],
        ];
        let rgb_p = mat_vec(&self.to_hpe, rgb_c);
        rgb_p.map(|x| {
            let q = (self.f_l * x.abs() / 100.0).powf(0.42);
            400.0 * x.signum() * q / (27.13 + q) + 0.1
        })
    }

    /// CAM02-UCS J′, a′, b′ and the CIECAM02 hue angle h (degrees).
    fn ucs(&self, xyz: [f64; 3]) -> (f64, f64, f64, f64) {
        let [ra, ga, ba] = self.compressed(xyz);
        let a = ra - 12.0 * ga / 11.0 + ba / 11.0;
        let b = (ra + ga - 2.0 * ba) / 9.0;
        let h = b.atan2(a).to_degrees().rem_euclid(360.0);
        let e_t = 0.25 * ((h.to_radians() + 2.0).cos() + 3.8);
        let a_resp = (2.0 * ra + ga + ba / 20.0 - 0.305) * self.n_bb;
        let j = 100.0 * spow(a_resp / self.a_w, SURROUND_C * self.z);
        let denom = ra + ga + 21.0 * ba / 20.0;
        let t = if denom.abs() > 1e-30 {
            (50000.0 / 13.0) * SURROUND_NC * self.n_bb * e_t * (a * a + b * b).sqrt() / denom
        } else {
            0.0
        };
        let c = spow(t, 0.9) * spow(j / 100.0, 0.5) * (1.64 - 0.29f64.powf(self.n)).powf(0.73);
        let m = c * self.f_l.powf(0.25);
        let jp = 1.7 * j / (1.0 + 0.007 * j);
        let mp = (1.0 + 0.0228 * m).ln() / 0.0228;
        let hr = h.to_radians();
        (jp, mp * hr.cos(), mp * hr.sin(), h)
    }
}

/// ΔE → fidelity, CIE 224:2017.
fn delta_e_to_rf(de: f64) -> f64 {
    10.0 * ((100.0 - 6.73 * de) / 10.0).exp().ln_1p()
}

/// TM-30 / CIE 224 reference illuminant for a CCT.
fn reference_illuminant(cct: f64) -> (Grid, Tm30Reference) {
    let norm_y = |s: Grid| {
        let y = tristimulus(&s, None, &OBSERVER_2)[1];
        if y > 0.0 {
            s.map(|v| v / y)
        } else {
            s
        }
    };
    if cct < 4000.0 {
        (planck(cct), Tm30Reference::Planckian)
    } else if cct <= 5000.0 {
        let p = norm_y(planck(cct));
        let d = norm_y(daylight(cct));
        let m = (cct - 4000.0) / 1000.0;
        let mut s = [0.0; N];
        for i in 0..N {
            s[i] = (1.0 - m) * p[i] + m * d[i];
        }
        (s, Tm30Reference::Blend { daylight_share: m })
    } else {
        (daylight(cct), Tm30Reference::Daylight)
    }
}

/// J′a′b′ and hue of all 99 samples under an illuminant (10° observer,
/// illuminant normalised to Y = 100).
fn sample_colours(s: &Grid) -> Vec<(f64, f64, f64, f64)> {
    let w = tristimulus(s, None, &OBSERVER_10);
    let k = 100.0 / w[1];
    let xyz_w = w.map(|v| v * k);
    let cam = Cam02::new(xyz_w);
    CES_REFLECTANCE
        .iter()
        .map(|refl| {
            let xyz = tristimulus(s, Some(*refl), &OBSERVER_10).map(|v| v * k);
            cam.ucs(xyz)
        })
        .collect()
}

fn polygon_area(points: &[(f64, f64)]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i].0 * points[j].1 - points[j].0 * points[i].1;
    }
    (area / 2.0).abs()
}

/// Calculate the TM-30 metrics of a spectrum.
///
/// Returns `None` when the spectrum does not cover at least 400–700 nm.
pub fn calculate_tm30(spd: &SpectralDistribution) -> Option<Tm30Result> {
    if !covers_visible(spd) {
        return None;
    }
    let test = resample(spd);
    let (cct, duv) = measured_cct_duv(spd);
    if !cct.is_finite() {
        return None;
    }
    let (reference, ref_kind) = reference_illuminant(cct);

    let t = sample_colours(&test);
    let r = sample_colours(&reference);

    let mut de = [0.0; NUM_CES];
    for i in 0..NUM_CES {
        de[i] = ((t[i].0 - r[i].0).powi(2) + (t[i].1 - r[i].1).powi(2) + (t[i].2 - r[i].2).powi(2))
            .sqrt();
    }
    let mean_de = de.iter().sum::<f64>() / NUM_CES as f64;
    let rf = delta_e_to_rf(mean_de);
    let rf_ces: Vec<f64> = de.iter().map(|&d| delta_e_to_rf(d)).collect();

    // Hue bins from the reference hue angle.
    let mut count = [0usize; NUM_BINS];
    let mut sum_t = [(0.0, 0.0); NUM_BINS];
    let mut sum_r = [(0.0, 0.0); NUM_BINS];
    let mut sum_de = [0.0; NUM_BINS];
    let mut sum_h = [0.0; NUM_BINS];
    for i in 0..NUM_CES {
        let bin = ((r[i].3 / 22.5).floor() as usize).min(NUM_BINS - 1);
        count[bin] += 1;
        sum_t[bin].0 += t[i].1;
        sum_t[bin].1 += t[i].2;
        sum_r[bin].0 += r[i].1;
        sum_r[bin].1 += r[i].2;
        sum_de[bin] += de[i];
        sum_h[bin] += r[i].3;
    }

    let mut rf_hue = [0.0; NUM_BINS];
    let mut rcs_hue = [0.0; NUM_BINS];
    let mut rhs_hue = [0.0; NUM_BINS];
    let mut color_vectors = [(0.0, 0.0, 0.0, 0.0); NUM_BINS];
    let mut ref_hue_deg = [0.0; NUM_BINS];
    let mut poly_t = Vec::with_capacity(NUM_BINS);
    let mut poly_r = Vec::with_capacity(NUM_BINS);
    for j in 0..NUM_BINS {
        let centre = (22.5 * j as f64 + 11.25).to_radians();
        if count[j] == 0 {
            // Not reachable with the 99 CES; keep the graphic well-defined.
            ref_hue_deg[j] = centre.to_degrees();
            rf_hue[j] = 100.0;
            continue;
        }
        let c = count[j] as f64;
        let (ta, tb) = (sum_t[j].0 / c, sum_t[j].1 / c);
        let (ra, rb) = (sum_r[j].0 / c, sum_r[j].1 / c);
        color_vectors[j] = (ta, tb, ra, rb);
        poly_t.push((ta, tb));
        poly_r.push((ra, rb));
        ref_hue_deg[j] = sum_h[j] / c;
        rf_hue[j] = delta_e_to_rf(sum_de[j] / c);
        let norm = (ra * ra + rb * rb).sqrt().max(1e-12);
        let (da, db) = (ta - ra, tb - rb);
        rcs_hue[j] = 100.0 * (da * centre.cos() + db * centre.sin()) / norm;
        rhs_hue[j] = (-da * centre.sin() + db * centre.cos()) / norm;
    }
    let area_r = polygon_area(&poly_r);
    let rg = if area_r > 0.0 {
        100.0 * polygon_area(&poly_t) / area_r
    } else {
        100.0
    };

    Some(Tm30Result {
        rf,
        rg,
        cct,
        duv,
        rf_hue,
        rcs_hue,
        rhs_hue,
        color_vectors,
        ref_hue_deg,
        rf_ces,
        reference: ref_kind,
    })
}

// ============================================================================
// Graphics
// ============================================================================

impl Tm30Result {
    /// Points of the normalised colour vector graphic (TM-30-20): each
    /// reference bin average on the unit circle at its mean hue angle, the
    /// test point displaced by the bin's (test − reference) average over the
    /// reference chroma. Outside the circle = more saturated.
    pub fn cvg_points(&self) -> [CvgPoint; NUM_BINS] {
        let mut out = [CvgPoint {
            reference: (0.0, 0.0),
            test: (0.0, 0.0),
        }; NUM_BINS];
        for (j, o) in out.iter_mut().enumerate() {
            let h = self.ref_hue_deg[j].to_radians();
            let (ta, tb, ra, rb) = self.color_vectors[j];
            let norm = (ra * ra + rb * rb).sqrt();
            let rp = (h.cos(), h.sin());
            let tp = if norm > 1e-12 {
                (rp.0 + (ta - ra) / norm, rp.1 + (tb - rb) / norm)
            } else {
                rp
            };
            *o = CvgPoint {
                reference: rp,
                test: tp,
            };
        }
        out
    }

    /// Normalised colour vector graphic as a standalone SVG (square).
    pub fn to_svg(&self, width: f64, height: f64, theme: &Tm30Theme) -> String {
        let size = width.min(height);
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="{font}">
<rect width="{width}" height="{height}" fill="{bg}"/>
"#,
            font = theme.font_family,
            bg = theme.background,
        );
        let cx = width / 2.0;
        let cy = height / 2.0;
        let radius = size * 0.30;
        svg.push_str(&cvg_body(self, cx, cy, radius, theme, true));
        svg.push_str(&format!(
            r#"<text x="12" y="{y1:.1}" font-size="13" fill="{fg}">Rf {rf:.0}</text>
<text x="12" y="{y2:.1}" font-size="13" fill="{fg}">Rg {rg:.0}</text>
<text x="{xr:.1}" y="{y1:.1}" font-size="13" fill="{fg}" text-anchor="end">{cct:.0} K</text>
<text x="{xr:.1}" y="{y2:.1}" font-size="13" fill="{fg}" text-anchor="end">Duv {duv:+.4}</text>
</svg>"#,
            y1 = height - 30.0,
            y2 = height - 12.0,
            xr = width - 12.0,
            fg = theme.foreground,
            rf = self.rf,
            rg = self.rg,
            cct = self.cct,
            duv = self.duv,
        ));
        svg
    }

    /// Local fidelity Rf,hj as a bar chart (bins in the TM-30 colours).
    pub fn rf_hue_svg(&self, width: f64, height: f64, theme: &Tm30Theme) -> String {
        let (ml, mr, mt, mb) = (40.0, 50.0, 30.0, 40.0);
        let pw = width - ml - mr;
        let ph = height - mt - mb;
        let slot = pw / NUM_BINS as f64;
        let bar_w = slot - 4.0;
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="{font}">
<rect width="{width}" height="{height}" fill="{bg}"/>
<text x="{tx:.1}" y="20" fill="{fg}" font-size="14" font-weight="bold" text-anchor="middle">Rf,h</text>
"#,
            font = theme.font_family,
            bg = theme.background,
            fg = theme.foreground,
            tx = width / 2.0,
        );
        for v in [0, 20, 40, 60, 80, 100] {
            let y = mt + ph * (1.0 - v as f64 / 100.0);
            svg.push_str(&format!(
                r#"<line x1="{ml}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="{grid}" stroke-width="1"/>
<text x="{tx:.1}" y="{y:.1}" fill="{fg}" font-size="10" text-anchor="end" dominant-baseline="middle">{v}</text>
"#,
                x2 = ml + pw,
                grid = theme.grid_color,
                tx = ml - 5.0,
                fg = theme.foreground,
            ));
        }
        for (i, &rf) in self.rf_hue.iter().enumerate() {
            let x = ml + i as f64 * slot + 2.0;
            let h = rf.clamp(0.0, 100.0) / 100.0 * ph;
            let y = mt + ph - h;
            svg.push_str(&format!(
                r#"<rect x="{x:.1}" y="{y:.1}" width="{bar_w:.1}" height="{h:.1}" fill="{c}" rx="2"/>
<text x="{lx:.1}" y="{ly:.1}" fill="{fg}" font-size="9" text-anchor="middle">{rf:.0}</text>
<text x="{lx:.1}" y="{by:.1}" fill="{fg}" font-size="9" text-anchor="middle">{n}</text>
"#,
                c = TM30_BIN_COLOURS[i],
                lx = x + bar_w / 2.0,
                ly = y - 3.0,
                by = mt + ph + 15.0,
                fg = theme.foreground,
                n = i + 1,
            ));
        }
        let ry = mt + ph * (1.0 - self.rf.clamp(0.0, 100.0) / 100.0);
        svg.push_str(&format!(
            r#"<line x1="{ml}" y1="{ry:.1}" x2="{x2:.1}" y2="{ry:.1}" stroke="{c}" stroke-width="2" stroke-dasharray="6,3"/>
<text x="{tx:.1}" y="{ty:.1}" fill="{c}" font-size="10">Rf {rf:.0}</text>
</svg>"#,
            x2 = ml + pw,
            c = theme.test_color,
            tx = ml + pw + 5.0,
            ty = ry + 4.0,
            rf = self.rf,
        ));
        svg
    }
}

/// SVG elements of the normalised vector graphic centred at (cx, cy) with
/// the reference circle of `radius` (shared by `to_svg` and the colour
/// quality card).
pub(crate) fn cvg_body(
    tm: &Tm30Result,
    cx: f64,
    cy: f64,
    radius: f64,
    theme: &Tm30Theme,
    bin_numbers: bool,
) -> String {
    let mut s = String::new();
    let outer = radius * 1.5;
    // Hue wedges (bin colours, light).
    for (j, colour) in TM30_BIN_COLOURS.iter().enumerate() {
        let a0 = (22.5 * j as f64).to_radians();
        let a1 = (22.5 * (j + 1) as f64).to_radians();
        // SVG y axis points down: hue angles run counter-clockwise.
        let (x0, y0) = (cx + outer * a0.cos(), cy - outer * a0.sin());
        let (x1, y1) = (cx + outer * a1.cos(), cy - outer * a1.sin());
        s.push_str(&format!(
            r#"<path d="M{cx:.2},{cy:.2} L{x0:.2},{y0:.2} A{outer:.2},{outer:.2} 0 0 0 {x1:.2},{y1:.2} Z" fill="{c}" fill-opacity="0.16"/>
"#,
            c = colour,
        ));
    }
    // ±20 % rings and the reference circle.
    for (r, dash, w) in [(0.8, "2,3", 0.8), (1.2, "2,3", 0.8)] {
        s.push_str(&format!(
            r#"<circle cx="{cx:.2}" cy="{cy:.2}" r="{rr:.2}" fill="none" stroke="{g}" stroke-width="{w}" stroke-dasharray="{dash}"/>
"#,
            rr = radius * r,
            g = theme.grid_color,
        ));
    }
    s.push_str(&format!(
        r#"<circle cx="{cx:.2}" cy="{cy:.2}" r="{radius:.2}" fill="none" stroke="{c}" stroke-width="1.6" stroke-dasharray="6,5"/>
"#,
        c = theme.reference_color,
    ));
    if bin_numbers {
        for j in 0..NUM_BINS {
            let a = (22.5 * j as f64 + 11.25).to_radians();
            let rr = outer - 12.0;
            s.push_str(&format!(
                r#"<text x="{x:.2}" y="{y:.2}" font-size="10" fill="{c}" text-anchor="middle" dominant-baseline="middle">{n}</text>
"#,
                x = cx + rr * a.cos(),
                y = cy - rr * a.sin(),
                c = theme.reference_color,
                n = j + 1,
            ));
        }
    }
    // Test polygon and points.
    let pts = tm.cvg_points();
    let poly: Vec<String> = pts
        .iter()
        .map(|p| {
            format!(
                "{:.2},{:.2}",
                cx + radius * p.test.0,
                cy - radius * p.test.1
            )
        })
        .collect();
    s.push_str(&format!(
        r#"<polygon points="{}" fill="{c}" fill-opacity="0.06" stroke="{c}" stroke-width="2.6" stroke-linejoin="round"/>
"#,
        poly.join(" "),
        c = theme.test_color,
    ));
    for (j, p) in pts.iter().enumerate() {
        s.push_str(&format!(
            r#"<circle cx="{x:.2}" cy="{y:.2}" r="4.6" fill="{c}" stroke="{bg}" stroke-width="1.2"/>
"#,
            x = cx + radius * p.test.0,
            y = cy - radius * p.test.1,
            c = TM30_BIN_COLOURS[j],
            bg = theme.background,
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atla::colour_common::wavelength;

    fn spd_from_grid(g: &Grid) -> SpectralDistribution {
        SpectralDistribution {
            wavelengths: (0..N).map(wavelength).collect(),
            values: g.to_vec(),
            units: crate::atla::types::SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        }
    }

    #[test]
    fn a_reference_illuminant_renders_itself_perfectly() {
        for cct in [2700.0, 3500.0, 4500.0, 6500.0] {
            let (r, _) = reference_illuminant(cct);
            let tm = calculate_tm30(&spd_from_grid(&r)).unwrap();
            assert!(tm.rf > 99.9, "{cct}: Rf {}", tm.rf);
            assert!((tm.rg - 100.0).abs() < 0.1, "{cct}: Rg {}", tm.rg);
            for p in tm.cvg_points() {
                let d = ((p.test.0 - p.reference.0).powi(2) + (p.test.1 - p.reference.1).powi(2))
                    .sqrt();
                assert!(d < 1e-3, "{cct}: CVG offset {d}");
            }
        }
    }

    #[test]
    fn reference_points_lie_on_the_unit_circle() {
        let (r, _) = reference_illuminant(3000.0);
        let mut s = r;
        // A crude two-band source to get visible shifts.
        for (i, v) in s.iter_mut().enumerate() {
            let wl = wavelength(i);
            *v = (-((wl - 450.0) / 15.0).powi(2)).exp() + (-((wl - 590.0) / 60.0).powi(2)).exp();
        }
        let tm = calculate_tm30(&spd_from_grid(&s)).unwrap();
        for p in tm.cvg_points() {
            let r = (p.reference.0.powi(2) + p.reference.1.powi(2)).sqrt();
            assert!((r - 1.0).abs() < 1e-12);
        }
        assert!(tm.rf < 95.0, "Rf {}", tm.rf);
    }

    #[test]
    fn reference_choice_follows_the_cct() {
        assert_eq!(reference_illuminant(3000.0).1, Tm30Reference::Planckian);
        assert_eq!(reference_illuminant(6500.0).1, Tm30Reference::Daylight);
        match reference_illuminant(4250.0).1 {
            Tm30Reference::Blend { daylight_share } => {
                assert!((daylight_share - 0.25).abs() < 1e-12)
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn narrow_spectra_are_refused() {
        let spd = SpectralDistribution {
            wavelengths: vec![500.0, 600.0],
            values: vec![1.0, 1.0],
            units: crate::atla::types::SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        };
        assert!(calculate_tm30(&spd).is_none());
    }
}
