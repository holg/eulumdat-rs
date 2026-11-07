//! Detection volume of a sensor LDT and its footprint on a plane.

use crate::Eulumdat;

/// Angular step (degrees) when searching a C-plane for the volume boundary.
const GAMMA_STEP: f64 = 0.1;

/// A sensor LDT read as a detection volume (values = slant range in metres).
#[derive(Debug, Clone)]
pub struct DetectionVolume {
    /// The table expanded to all C-planes (no symmetry), so every lookup
    /// takes the fast path; [`Self::file_symmetry`] keeps the file's setting.
    ldt: Eulumdat,
    gamma_max: f64,
    file_symmetry: crate::Symmetry,
    /// The table has no symmetry and sorted angle lists: look values up by
    /// bisection ([`Self::lookup`]) instead of the shared linear search.
    fast: bool,
}

/// Cut of a [`DetectionVolume`] with a horizontal plane.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Footprint {
    /// Distance of the plane below the sensor (m).
    pub depth: f64,
    /// C step of `reach` / `outline` (degrees), starting at C0.
    pub c_step_deg: f64,
    /// Farthest detected horizontal distance per C-plane (m).
    pub reach: Vec<f64>,
    /// Outline in sensor-local plan coordinates (m): C0 = +x, C90 = +y.
    pub outline: Vec<(f64, f64)>,
    /// Enclosed area (m²).
    pub area_m2: f64,
    /// Largest `reach` (m).
    pub max_range: f64,
}

impl Footprint {
    /// Diameter of the circle with the same area (m).
    pub fn equivalent_diameter(&self) -> f64 {
        2.0 * (self.area_m2 / core::f64::consts::PI).sqrt()
    }

    /// True if nothing is detected on this plane.
    pub fn is_empty(&self) -> bool {
        self.max_range <= 0.0
    }

    /// Range towards the C angle `c_deg` (linear between the tabulated
    /// C-planes).
    pub fn reach_towards(&self, c_deg: f64) -> f64 {
        let n = self.reach.len();
        if n == 0 || self.c_step_deg <= 0.0 {
            return 0.0;
        }
        let pos = c_deg.rem_euclid(360.0) / self.c_step_deg;
        let i = (pos.floor() as usize) % n;
        let f = pos - pos.floor();
        self.reach[i] * (1.0 - f) + self.reach[(i + 1) % n] * f
    }

    /// The point (`dx`, `dy`) in sensor-local plan coordinates (m; C0 = +x,
    /// C90 = +y) lies in the detection area.
    pub fn contains(&self, dx: f64, dy: f64) -> bool {
        if dx.abs() > self.max_range || dy.abs() > self.max_range {
            return false;
        }
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1e-9 {
            return self.max_range > 0.0;
        }
        d <= self.reach_towards(dy.atan2(dx).to_degrees())
    }
}

/// Lower index and fraction of `target` in a sorted list: the pair the
/// shared linear search (`SymmetryHandler`) finds, by bisection.
fn bracket(angles: &[f64], target: f64) -> (usize, f64) {
    if angles.is_empty() || target <= angles[0] {
        return (0, 0.0);
    }
    let last = angles.len() - 1;
    if target >= angles[last] {
        return (last, 0.0);
    }
    let j = angles.partition_point(|a| *a < target);
    let i = j - 1;
    (i, (target - angles[i]) / (angles[j] - angles[i]))
}

/// A symmetric table expanded to every C-plane (`Symmetry::None`): a
/// symmetric lookup folds the angle and rebuilds the stored plane list on
/// every sample, ~30× slower, and cutting a volume takes ~10⁵ samples.
/// The values are the same.
fn full_circle(mut ldt: Eulumdat) -> Eulumdat {
    use crate::symmetry::SymmetryHandler;
    if ldt.symmetry == crate::Symmetry::None {
        return ldt;
    }
    let intensities = SymmetryHandler::expand_to_full(&ldt);
    let c_angles = SymmetryHandler::expand_c_angles(&ldt);
    if intensities.is_empty() || intensities.len() != c_angles.len() {
        return ldt;
    }
    ldt.symmetry = crate::Symmetry::None;
    ldt.num_c_planes = c_angles.len();
    ldt.c_angles = c_angles;
    ldt.intensities = intensities;
    ldt
}

/// A detection volume sampled per C-plane ([`DetectionVolume::depth_profile`]).
#[derive(Debug, Clone)]
pub struct DepthProfile {
    c_step_deg: f64,
    gammas: Vec<f64>,
    tans: Vec<f64>,
    /// Per C-plane: horizontal range at γ = 90°, and k·r(C, γ)·cos γ.
    planes: Vec<(f64, Vec<f64>)>,
}

impl DepthProfile {
    /// Horizontal reach of plane `i` at `depth` (same crossing rule as
    /// [`DetectionVolume::reach_scaled`]).
    fn reach(&self, i: usize, depth: f64) -> f64 {
        let (horizontal, vertical) = &self.planes[i];
        if depth <= 0.0 {
            return *horizontal;
        }
        let mut best = 0.0f64;
        let mut f0 = vertical[0] - depth;
        for (j, &v1) in vertical.iter().enumerate().skip(1) {
            let f1 = v1 - depth;
            if f0 >= 0.0 {
                let (g0, g1) = (self.gammas[j - 1], self.gammas[j]);
                let tan_edge = if f1 >= 0.0 {
                    self.tans[j]
                } else {
                    (g0 + (g1 - g0) * f0 / (f0 - f1)).to_radians().tan()
                };
                best = best.max(depth * tan_edge);
            }
            f0 = f1;
        }
        best
    }

    /// The cut at `depth` below the sensor (equal to
    /// [`DetectionVolume::footprint_scaled`] with the same step and scale).
    pub fn footprint(&self, depth: f64) -> Footprint {
        let n = self.planes.len();
        let step = self.c_step_deg;
        let reach: Vec<f64> = (0..n).map(|i| self.reach(i, depth)).collect();
        let outline: Vec<(f64, f64)> = reach
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                let a = (i as f64 * step).to_radians();
                (r * a.cos(), r * a.sin())
            })
            .collect();
        let s = step.to_radians().sin();
        let area_m2 = (0..n)
            .map(|i| 0.5 * reach[i] * reach[(i + 1) % n] * s)
            .sum();
        let max_range = reach.iter().copied().fold(0.0, f64::max);
        Footprint {
            depth,
            c_step_deg: step,
            reach,
            outline,
            area_m2,
            max_range,
        }
    }
}

impl DetectionVolume {
    /// Wrap a parsed sensor LDT. The caller knows it is a sensor; nothing
    /// is inferred from the file.
    pub fn new(ldt: Eulumdat) -> Self {
        let gamma_max = ldt.g_angles.last().copied().unwrap_or(0.0);
        let file_symmetry = ldt.symmetry;
        let ldt = full_circle(ldt);
        let sorted = |v: &[f64]| v.windows(2).all(|w| w[0] < w[1]);
        let fast =
            ldt.symmetry == crate::Symmetry::None && sorted(&ldt.c_angles) && sorted(&ldt.g_angles);
        Self {
            ldt,
            gamma_max,
            file_symmetry,
            fast,
        }
    }

    /// The symmetry the file declares (the table held here is expanded).
    pub fn file_symmetry(&self) -> crate::Symmetry {
        self.file_symmetry
    }

    /// The underlying EULUMDAT data.
    pub fn ldt(&self) -> &Eulumdat {
        &self.ldt
    }

    /// The parameter block (EULUMDAT lines 33–42).
    pub fn params(&self) -> super::SensorParams {
        super::SensorParams::from_ldt(&self.ldt)
    }

    /// Distance (m) from the sensor to the volume boundary along (C, γ).
    /// Zero outside the measured γ range.
    pub fn range(&self, c_deg: f64, gamma_deg: f64) -> f64 {
        if gamma_deg < 0.0 || gamma_deg > self.gamma_max + 1e-9 {
            return 0.0;
        }
        if self.fast {
            self.lookup(c_deg, gamma_deg).max(0.0)
        } else {
            self.ldt.sample(c_deg, gamma_deg).max(0.0)
        }
    }

    /// `Eulumdat::sample` for a table without symmetry and with sorted
    /// angles, with the index found by bisection (the same bracket and the
    /// same interpolation, including the wrap from the last C-plane to C0).
    fn lookup(&self, c_deg: f64, gamma_deg: f64) -> f64 {
        let l = &self.ldt;
        let c = c_deg.rem_euclid(360.0);
        let g = gamma_deg.clamp(0.0, 180.0);
        let (gi, gf) = bracket(&l.g_angles, g);
        let get = |ci: usize, gi: usize| {
            l.intensities
                .get(ci)
                .and_then(|row| row.get(gi))
                .copied()
                .unwrap_or(0.0)
        };
        let mix = |c0: usize, c1: usize, cf: f64| {
            let i0 = get(c0, gi) * (1.0 - gf) + get(c0, gi + 1) * gf;
            let i1 = get(c1, gi) * (1.0 - gf) + get(c1, gi + 1) * gf;
            i0 * (1.0 - cf) + i1 * cf
        };
        let ca = &l.c_angles;
        if ca.len() > 1 {
            let (first, last) = (ca[0], ca[ca.len() - 1]);
            if c > last {
                let span = (360.0 - last) + first;
                if span > 0.0 {
                    return mix(ca.len() - 1, 0, (c - last) / span);
                }
            }
        }
        let (ci, cf) = bracket(ca, c);
        mix(ci, ci + 1, cf)
    }

    /// Is the point `(x, y)` (m, sensor-local, C0 = +x) on the plane `depth`
    /// metres below the sensor inside the volume?
    pub fn contains(&self, x: f64, y: f64, depth: f64) -> bool {
        let rho = x.hypot(y);
        let d = rho.hypot(depth);
        if d <= 0.0 {
            return true;
        }
        let c = y.atan2(x).to_degrees().rem_euclid(360.0);
        let g = rho.atan2(depth).to_degrees();
        d <= self.range(c, g)
    }

    /// Farthest horizontal distance (m) inside the volume in C-plane `c_deg`
    /// on the plane `depth` metres below the sensor.
    ///
    /// Along the plane, the direction γ hits it at slant distance
    /// `depth / cos γ`; the point is inside while the boundary's vertical
    /// depth `range(C, γ)·cos γ` is at least `depth`. The last crossing is
    /// refined linearly.
    pub fn reach(&self, c_deg: f64, depth: f64) -> f64 {
        self.reach_scaled(c_deg, depth, 1.0)
    }

    /// [`Self::reach`] of the volume with every range multiplied by `k`
    /// (how the radial and presence areas are derived, see [`Motion`]).
    ///
    /// [`Motion`]: super::Motion
    pub fn reach_scaled(&self, c_deg: f64, depth: f64, k: f64) -> f64 {
        if depth <= 0.0 {
            return k * self.range(c_deg, 90.0);
        }
        let g_end = self.gamma_max.min(89.9);
        let f = |g: f64| k * self.range(c_deg, g) * g.to_radians().cos() - depth;
        let mut best = 0.0f64;
        let mut g0 = 0.0;
        let mut f0 = f(g0);
        while g0 < g_end {
            let g1 = (g0 + GAMMA_STEP).min(g_end);
            let f1 = f(g1);
            if f0 >= 0.0 {
                // inside at g0: the boundary is at g1 or between g0 and g1
                let g_edge = if f1 >= 0.0 {
                    g1
                } else {
                    g0 + (g1 - g0) * f0 / (f0 - f1)
                };
                best = best.max(depth * g_edge.to_radians().tan());
            }
            g0 = g1;
            f0 = f1;
        }
        best
    }

    /// Footprint on the plane `depth` metres below the sensor.
    pub fn footprint(&self, depth: f64, c_step_deg: f64) -> Footprint {
        self.footprint_scaled(depth, c_step_deg, 1.0)
    }

    /// Footprint of one motion type: the volume scaled by the factor from
    /// the parameter block ([`SensorParams::scale`]). `None` if the file has
    /// no data for that motion type.
    ///
    /// [`SensorParams::scale`]: super::SensorParams::scale
    pub fn motion_footprint(
        &self,
        motion: super::Motion,
        depth: f64,
        c_step_deg: f64,
    ) -> Option<Footprint> {
        let k = self.params().scale(motion)?;
        Some(self.footprint_scaled(depth, c_step_deg, k))
    }

    /// Footprint of the volume with every range multiplied by `k`.
    pub fn footprint_scaled(&self, depth: f64, c_step_deg: f64, k: f64) -> Footprint {
        let step = if c_step_deg > 0.0 { c_step_deg } else { 1.0 };
        let n = (360.0 / step).round().max(4.0) as usize;
        let step = 360.0 / n as f64;
        let reach: Vec<f64> = (0..n)
            .map(|i| self.reach_scaled(i as f64 * step, depth, k))
            .collect();
        let outline: Vec<(f64, f64)> = reach
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                let a = (i as f64 * step).to_radians();
                (r * a.cos(), r * a.sin())
            })
            .collect();
        let s = step.to_radians().sin();
        let area_m2 = (0..n)
            .map(|i| 0.5 * reach[i] * reach[(i + 1) % n] * s)
            .sum();
        let max_range = reach.iter().copied().fold(0.0, f64::max);
        Footprint {
            depth,
            c_step_deg: step,
            reach,
            outline,
            area_m2,
            max_range,
        }
    }

    /// The volume scaled by `k`, sampled once per C-plane, for cutting it
    /// at many depths (range over the mounting height): sampling the table
    /// is the slow part of [`Self::footprint_scaled`]. Cuts from the profile
    /// equal [`Self::footprint_scaled`] exactly.
    pub fn depth_profile(&self, c_step_deg: f64, k: f64) -> DepthProfile {
        let step = if c_step_deg > 0.0 { c_step_deg } else { 1.0 };
        let n = (360.0 / step).round().max(4.0) as usize;
        let step = 360.0 / n as f64;
        let g_end = self.gamma_max.min(89.9);
        // γ grid exactly as `reach_scaled` walks it
        let mut gammas = vec![0.0];
        let mut g0 = 0.0;
        while g0 < g_end {
            let g1 = (g0 + GAMMA_STEP).min(g_end);
            gammas.push(g1);
            g0 = g1;
        }
        let tans: Vec<f64> = gammas.iter().map(|g| g.to_radians().tan()).collect();
        let planes = (0..n)
            .map(|i| {
                let c = i as f64 * step;
                let vertical: Vec<f64> = gammas
                    .iter()
                    .map(|&g| k * self.range(c, g) * g.to_radians().cos())
                    .collect();
                (k * self.range(c, 90.0), vertical)
            })
            .collect();
        DepthProfile {
            c_step_deg: step,
            gammas,
            tans,
            planes,
        }
    }

    /// Deepest point (m below the sensor) of the volume scaled by `k`: below
    /// it nothing is detected, whatever the mounting height.
    pub fn depth_limit(&self, k: f64) -> f64 {
        let mut deepest = 0.0f64;
        let mut c = 0.0;
        while c < 360.0 {
            let mut g = 0.0;
            while g <= self.gamma_max.min(90.0) {
                deepest = deepest.max(k * self.range(c, g) * g.to_radians().cos());
                g += 0.5;
            }
            c += 5.0;
        }
        deepest
    }

    /// [`Self::depth_limit`] of one motion type (`None` without data).
    pub fn motion_depth_limit(&self, motion: super::Motion) -> Option<f64> {
        Some(self.depth_limit(self.params().scale(motion)?))
    }

    /// Farthest horizontal detection distance on the plane `depth` below the
    /// sensor, over all C-planes at 1° steps.
    pub fn max_range(&self, depth: f64) -> f64 {
        self.footprint(depth, 1.0).max_range
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::Symmetry;

    /// Sensor LDT of a box volume |x| ≤ a, |y| ≤ b, 0 ≤ z ≤ h below the
    /// sensor (the shape of a corridor ultrasonic sensor).
    pub(crate) fn box_ldt(a: f64, b: f64, h: f64) -> Eulumdat {
        let mut ldt = Eulumdat::new();
        ldt.symmetry = Symmetry::None;
        ldt.num_c_planes = 72;
        ldt.c_plane_distance = 5.0;
        ldt.num_g_planes = 91;
        ldt.g_plane_distance = 1.0;
        ldt.c_angles = (0..72).map(|i| i as f64 * 5.0).collect();
        ldt.g_angles = (0..91).map(|i| i as f64).collect();
        ldt.intensities = ldt
            .c_angles
            .iter()
            .map(|&c| {
                ldt.g_angles
                    .iter()
                    .map(|&g| {
                        let (c, g) = (c.to_radians(), g.to_radians());
                        let (dx, dy, dz) = (g.sin() * c.cos(), g.sin() * c.sin(), g.cos());
                        let mut t = f64::INFINITY;
                        if dx.abs() > 1e-12 {
                            t = t.min(a / dx.abs());
                        }
                        if dy.abs() > 1e-12 {
                            t = t.min(b / dy.abs());
                        }
                        if dz > 1e-12 {
                            t = t.min(h / dz);
                        }
                        if t.is_finite() {
                            t
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect();
        ldt
    }

    #[test]
    fn box_footprint_is_the_rectangle() {
        let v = DetectionVolume::new(box_ldt(10.0, 1.5, 4.25));
        let fp = v.footprint(2.0, 1.0);
        assert!((fp.reach[0] - 10.0).abs() < 0.05, "C0 {}", fp.reach[0]);
        assert!((fp.reach[90] - 1.5).abs() < 0.05, "C90 {}", fp.reach[90]);
        // corner of the 20 m × 3 m rectangle
        assert!(
            (fp.max_range - 10.0f64.hypot(1.5)).abs() < 0.15,
            "{}",
            fp.max_range
        );
        assert!((fp.area_m2 - 60.0).abs() < 2.0, "area {}", fp.area_m2);
    }

    #[test]
    fn plane_below_the_volume_is_empty() {
        let v = DetectionVolume::new(box_ldt(10.0, 1.5, 4.25));
        assert!(v.footprint(5.0, 5.0).is_empty());
        assert!(!v.footprint(4.0, 5.0).is_empty());
    }

    #[test]
    fn contains_matches_reach() {
        let v = DetectionVolume::new(box_ldt(6.0, 3.0, 3.0));
        assert!(v.contains(5.9, 0.0, 2.0));
        assert!(!v.contains(6.2, 0.0, 2.0));
        assert!(v.contains(0.0, 2.9, 2.0));
        assert!(!v.contains(0.0, 3.1, 2.0));
        assert!(!v.contains(0.0, 0.0, 3.5));
    }

    #[test]
    fn motion_types_scale_the_volume() {
        use crate::sensor::Motion;
        let mut ldt = box_ldt(10.0, 10.0, 6.0);
        // tangential 1, radial 0.5, presence 0.25 (fields 2, 1, 3)
        ldt.direct_ratios = [0.5, 1.0, 0.25, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 0.0];
        let v = DetectionVolume::new(ldt);
        let t = v.motion_footprint(Motion::Tangential, 1.0, 5.0).unwrap();
        let r = v.motion_footprint(Motion::Radial, 1.0, 5.0).unwrap();
        let p = v.motion_footprint(Motion::Presence, 1.0, 5.0).unwrap();
        assert!((t.reach[0] - 10.0).abs() < 0.05);
        assert!((r.reach[0] - 5.0).abs() < 0.05);
        assert!((p.reach[0] - 2.5).abs() < 0.05);
        // no presence data when field 3 is 0
        let mut ldt = box_ldt(10.0, 10.0, 6.0);
        ldt.direct_ratios = [0.5, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        assert!(DetectionVolume::new(ldt)
            .motion_footprint(Motion::Presence, 1.0, 5.0)
            .is_none());
    }

    #[test]
    fn depth_limit_is_where_footprints_end() {
        let mut ldt = box_ldt(10.0, 1.5, 4.25);
        ldt.direct_ratios = [0.5, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let v = DetectionVolume::new(ldt);
        assert!((v.depth_limit(1.0) - 4.25).abs() < 0.01);
        let radial = v.motion_depth_limit(crate::sensor::Motion::Radial).unwrap();
        assert!((radial - 2.125).abs() < 0.01);
        assert!(v
            .motion_depth_limit(crate::sensor::Motion::Presence)
            .is_none());
        assert!(!v.footprint(4.2, 5.0).is_empty());
        assert!(v.footprint(4.3, 5.0).is_empty());
    }

    #[test]
    fn range_is_zero_beyond_measured_gamma() {
        let mut ldt = box_ldt(5.0, 5.0, 3.0);
        ldt.g_angles.truncate(61);
        ldt.num_g_planes = 61;
        for row in &mut ldt.intensities {
            row.truncate(61);
        }
        let v = DetectionVolume::new(ldt);
        assert!(v.range(0.0, 60.0) > 0.0);
        assert_eq!(v.range(0.0, 70.0), 0.0);
    }

    /// Cutting a sampled profile gives exactly the direct cut.
    #[test]
    fn depth_profile_matches_direct_cut() {
        let v = DetectionVolume::new(box_ldt(10.0, 1.5, 4.25));
        for k in [1.0, 0.5] {
            let p = v.depth_profile(1.0, k);
            for depth in [0.0, 0.5, 2.0, 3.9, 4.3, 6.0] {
                let direct = v.footprint_scaled(depth, 1.0, k);
                let fast = p.footprint(depth);
                assert_eq!(direct.reach, fast.reach, "k {k} depth {depth}");
                assert_eq!(direct.area_m2, fast.area_m2);
            }
        }
    }

    /// Expanding a symmetric sensor LDT keeps every value.
    #[test]
    fn expanded_table_samples_the_same() {
        let text = include_str!("../../tests/fixtures/sensors/demo_corridor.ldt");
        let raw = Eulumdat::parse(text).unwrap();
        assert_ne!(raw.symmetry, Symmetry::None);
        let v = DetectionVolume::new(raw.clone());
        assert_eq!(v.file_symmetry(), raw.symmetry);
        for c in (0..360).step_by(7) {
            for g in (0..=90).step_by(3) {
                let (c, g) = (c as f64 + 0.3, g as f64);
                let a = raw.sample(c, g).max(0.0);
                let b = v.range(c, g);
                assert!((a - b).abs() < 1e-9, "C{c} γ{g}: {a} vs {b}");
                // the bisection lookup equals the shared lookup exactly
                assert_eq!(v.ldt().sample(c, g).max(0.0), b);
            }
        }
    }

    /// The bisection lookup equals `Eulumdat::sample` on a table without
    /// symmetry, also across the wrap from the last C-plane to C0.
    #[test]
    fn fast_lookup_equals_sample() {
        let ldt = box_ldt(10.0, 1.5, 4.25);
        let v = DetectionVolume::new(ldt.clone());
        assert!(v.fast);
        for c in [
            0.0, 0.5, 4.99, 5.0, 77.7, 354.9, 355.0, 357.5, 359.99, 360.0, -3.0, 725.0,
        ] {
            for g in [0.0, 0.25, 1.0, 44.4, 89.9, 90.0] {
                assert_eq!(ldt.sample(c, g).max(0.0), v.range(c, g), "C{c} γ{g}");
            }
        }
    }
}
