//! Multi-luminaire illuminance computation on a rectangular ground plane.

use crate::Eulumdat;

/// A single luminaire placement on the ground plane.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LuminairePlace {
    pub id: usize,
    /// Ground X position in meters
    pub x: f64,
    /// Ground Y position in meters
    pub y: f64,
    /// Mounting height in meters
    pub mounting_height: f64,
    /// Tilt angle in degrees (0 = straight down, positive = tilted forward)
    pub tilt_angle: f64,
    /// C0 direction in degrees (0 = +Y, clockwise)
    pub rotation: f64,
    /// Lateral offset from pole center in meters
    pub arm_length: f64,
    /// Direction of arm overhang in degrees
    pub arm_direction: f64,
}

impl LuminairePlace {
    /// Create a simple placement at (x, y) with given height and no tilt/rotation.
    pub fn simple(id: usize, x: f64, y: f64, mounting_height: f64) -> Self {
        Self {
            id,
            x,
            y,
            mounting_height,
            tilt_angle: 0.0,
            rotation: 0.0,
            arm_length: 0.0,
            arm_direction: 0.0,
        }
    }

    /// Effective luminaire position accounting for arm offset.
    pub fn effective_position(&self) -> (f64, f64) {
        if self.arm_length <= 0.0 {
            return (self.x, self.y);
        }
        let dir_rad = self.arm_direction.to_radians();
        (
            self.x + self.arm_length * dir_rad.sin(),
            self.y + self.arm_length * dir_rad.cos(),
        )
    }
}

/// Result of multi-luminaire illuminance calculation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AreaResult {
    /// Combined illuminance grid (row-major: [row][col])
    pub lux_grid: Vec<Vec<f64>>,
    pub min_lux: f64,
    pub avg_lux: f64,
    pub max_lux: f64,
    /// U₀ = min / avg
    pub uniformity_min_avg: f64,
    /// avg / min
    pub uniformity_avg_min: f64,
    /// Ud = min / max
    pub uniformity_min_max: f64,
    /// Area width in meters (bounding box width for polygon areas)
    pub area_width: f64,
    /// Area depth in meters (bounding box height for polygon areas)
    pub area_depth: f64,
    /// Grid resolution (cells per axis)
    pub grid_resolution: usize,
    /// Optional polygon mask. true = cell is inside the polygon.
    /// Statistics only include masked-in cells. None = rectangular area.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "Option::is_none", default)
    )]
    pub mask: Option<Vec<Vec<bool>>>,
}

/// Compute combined illuminance from multiple luminaire placements.
///
/// The area is defined as a rectangle from (0, 0) to (area_width, area_depth).
/// Each luminaire contributes to every grid point via superposition.
pub fn compute_area_illuminance(
    ldt: &Eulumdat,
    placements: &[LuminairePlace],
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,
) -> AreaResult {
    let n = grid_resolution;
    let dx = area_width / n as f64;
    let dy = area_depth / n as f64;

    // Flux scale from lamp data
    let total_flux: f64 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
        .sum();
    let flux_scale = total_flux / 1000.0 * proration_factor;

    let mut lux_grid = vec![vec![0.0_f64; n]; n];

    for placement in placements {
        let (lx, ly) = placement.effective_position();
        let h = placement.mounting_height;
        let tilt_rad = placement.tilt_angle.to_radians();
        let rot_rad = placement.rotation.to_radians();

        for (row, grid_row) in lux_grid.iter_mut().enumerate() {
            let gy = (row as f64 + 0.5) * dy;
            for (col, cell_val) in grid_row.iter_mut().enumerate() {
                let gx = (col as f64 + 0.5) * dx;

                let lux = compute_single_illuminance(
                    ldt,
                    gx - lx,
                    gy - ly,
                    h,
                    tilt_rad,
                    rot_rad,
                    flux_scale,
                );
                *cell_val += lux;
            }
        }
    }

    finalize_grid(lux_grid, area_width, area_depth, grid_resolution)
}

/// Compute combined illuminance from mixed luminaire types.
///
/// Each placement has an associated LDT index into `ldts`. This allows
/// different poles to use different photometric data files.
pub fn compute_area_illuminance_mixed(
    ldts: &[&Eulumdat],
    placements: &[LuminairePlace],
    ldt_indices: &[usize],
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,
) -> AreaResult {
    let n = grid_resolution;
    let dx = area_width / n as f64;
    let dy = area_depth / n as f64;

    // Precompute flux scales per LDT
    let flux_scales: Vec<f64> = ldts
        .iter()
        .map(|ldt| {
            let total_flux: f64 = ldt
                .lamp_sets
                .iter()
                .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
                .sum();
            total_flux / 1000.0 * proration_factor
        })
        .collect();

    let mut lux_grid = vec![vec![0.0_f64; n]; n];

    for (pi, placement) in placements.iter().enumerate() {
        let ldt_idx = ldt_indices
            .get(pi)
            .copied()
            .unwrap_or(0)
            .min(ldts.len() - 1);
        let ldt = ldts[ldt_idx];
        let fs = flux_scales[ldt_idx];

        let (lx, ly) = placement.effective_position();
        let h = placement.mounting_height;
        let tilt_rad = placement.tilt_angle.to_radians();
        let rot_rad = placement.rotation.to_radians();

        for (row, grid_row) in lux_grid.iter_mut().enumerate() {
            let gy = (row as f64 + 0.5) * dy;
            for (col, cell_val) in grid_row.iter_mut().enumerate() {
                let gx = (col as f64 + 0.5) * dx;
                let lux =
                    compute_single_illuminance(ldt, gx - lx, gy - ly, h, tilt_rad, rot_rad, fs);
                *cell_val += lux;
            }
        }
    }

    finalize_grid(lux_grid, area_width, area_depth, grid_resolution)
}

/// Build AreaResult statistics from a completed lux grid.
fn finalize_grid(
    lux_grid: Vec<Vec<f64>>,
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
) -> AreaResult {
    let n = grid_resolution;
    let mut min_lux = f64::MAX;
    let mut max_lux: f64 = 0.0;
    let mut sum_lux: f64 = 0.0;
    let total_cells = (n * n) as f64;

    for row in &lux_grid {
        for &lux in row {
            if lux < min_lux {
                min_lux = lux;
            }
            if lux > max_lux {
                max_lux = lux;
            }
            sum_lux += lux;
        }
    }

    let avg_lux = if total_cells > 0.0 {
        sum_lux / total_cells
    } else {
        0.0
    };
    if min_lux == f64::MAX {
        min_lux = 0.0;
    }

    AreaResult {
        lux_grid,
        min_lux,
        avg_lux,
        max_lux,
        uniformity_min_avg: if avg_lux > 0.0 {
            min_lux / avg_lux
        } else {
            0.0
        },
        uniformity_avg_min: if min_lux > 0.0 {
            avg_lux / min_lux
        } else {
            f64::INFINITY
        },
        uniformity_min_max: if max_lux > 0.0 {
            min_lux / max_lux
        } else {
            0.0
        },
        area_width,
        area_depth,
        grid_resolution,
        mask: None,
    }
}

/// Build AreaResult with a polygon mask — only masked-in cells count for stats.
fn finalize_grid_masked(
    lux_grid: Vec<Vec<f64>>,
    mask: Vec<Vec<bool>>,
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
) -> AreaResult {
    let mut min_lux = f64::MAX;
    let mut max_lux: f64 = 0.0;
    let mut sum_lux: f64 = 0.0;
    let mut count: usize = 0;

    for (row_idx, row) in lux_grid.iter().enumerate() {
        for (col_idx, &lux) in row.iter().enumerate() {
            if mask[row_idx][col_idx] {
                if lux < min_lux {
                    min_lux = lux;
                }
                if lux > max_lux {
                    max_lux = lux;
                }
                sum_lux += lux;
                count += 1;
            }
        }
    }

    let avg_lux = if count > 0 {
        sum_lux / count as f64
    } else {
        0.0
    };
    if min_lux == f64::MAX {
        min_lux = 0.0;
    }

    AreaResult {
        lux_grid,
        min_lux,
        avg_lux,
        max_lux,
        uniformity_min_avg: if avg_lux > 0.0 {
            min_lux / avg_lux
        } else {
            0.0
        },
        uniformity_avg_min: if min_lux > 0.0 {
            avg_lux / min_lux
        } else {
            f64::INFINITY
        },
        uniformity_min_max: if max_lux > 0.0 {
            min_lux / max_lux
        } else {
            0.0
        },
        area_width,
        area_depth,
        grid_resolution,
        mask: Some(mask),
    }
}

/// Compute combined illuminance within a polygon area.
///
/// The computation grid covers the polygon's bounding box. Statistics only
/// include cells whose center falls inside the polygon.
pub fn compute_area_illuminance_polygon(
    ldt: &Eulumdat,
    placements: &[LuminairePlace],
    polygon: &crate::area::polygon::AreaPolygon,
    grid_resolution: usize,
    proration_factor: f64,
) -> AreaResult {
    let (x0, y0, x1, y1) = polygon.bounding_box();
    let area_width = x1 - x0;
    let area_depth = y1 - y0;
    let n = grid_resolution;
    let dx = area_width / n as f64;
    let dy = area_depth / n as f64;

    let total_flux: f64 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
        .sum();
    let flux_scale = total_flux / 1000.0 * proration_factor;

    let mut lux_grid = vec![vec![0.0_f64; n]; n];

    for placement in placements {
        let (lx, ly) = placement.effective_position();
        let h = placement.mounting_height;
        let tilt_rad = placement.tilt_angle.to_radians();
        let rot_rad = placement.rotation.to_radians();

        for (row, grid_row) in lux_grid.iter_mut().enumerate() {
            let gy = y0 + (row as f64 + 0.5) * dy;
            for (col, cell_val) in grid_row.iter_mut().enumerate() {
                let gx = x0 + (col as f64 + 0.5) * dx;
                let lux = compute_single_illuminance(
                    ldt,
                    gx - lx,
                    gy - ly,
                    h,
                    tilt_rad,
                    rot_rad,
                    flux_scale,
                );
                *cell_val += lux;
            }
        }
    }

    let mask = polygon.build_mask(grid_resolution);
    finalize_grid_masked(lux_grid, mask, area_width, area_depth, grid_resolution)
}

/// Compute illuminance contribution from one luminaire at one ground point.
///
/// `dx`, `dy`: vector from luminaire to ground point in meters.
/// `h`: mounting height. `tilt_rad`: tilt angle in radians.
/// `rot_rad`: C0 rotation in radians. `flux_scale`: total_flux/1000 × proration.
fn compute_single_illuminance(
    ldt: &Eulumdat,
    dx: f64,
    dy: f64,
    h: f64,
    tilt_rad: f64,
    rot_rad: f64,
    flux_scale: f64,
) -> f64 {
    let dz = -h;
    let r = (dx * dx + dy * dy + dz * dz).sqrt();
    if r < 1e-6 {
        return 0.0;
    }

    // Rotate ground vector into luminaire's local coordinate system.
    // First apply rotation (C0 direction), then tilt.

    // Rotate around Z by -rotation to align C0 with +X
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();
    let dx_r = dx * cos_r + dy * sin_r;
    let dy_r = -dx * sin_r + dy * cos_r;
    let dz_r = dz;

    // Rotate around Y by -tilt
    let cos_t = tilt_rad.cos();
    let sin_t = tilt_rad.sin();
    let dx_rot = dx_r * cos_t + dz_r * sin_t;
    let dy_rot = dy_r;
    let dz_rot = -dx_r * sin_t + dz_r * cos_t;

    // Convert to Type C angles
    let gamma = (-dz_rot / r).clamp(-1.0, 1.0).acos();
    let c = dy_rot.atan2(dx_rot);

    let mut c_deg = c.to_degrees();
    if c_deg < 0.0 {
        c_deg += 360.0;
    }
    let gamma_deg = gamma.to_degrees();

    let intensity = ldt.sample(c_deg, gamma_deg);

    // E = I × cos(θ_incidence) / r²
    let cos_incidence = h / r;
    let illuminance = intensity * flux_scale * cos_incidence / (r * r);

    illuminance.max(0.0)
}

/// Compute illuminance at an arbitrary point with an arbitrary surface normal.
///
/// `point`: (x, y, z) of the surface point.
/// `normal`: (nx, ny, nz) outward-facing surface unit normal.
/// Uses E = I × cos(θ_incidence) / r² where θ_incidence is the angle between
/// the incoming light direction and the surface normal.
pub(crate) fn compute_illuminance_at_point(
    ldt: &Eulumdat,
    placement: &LuminairePlace,
    point: (f64, f64, f64),
    normal: (f64, f64, f64),
    flux_scale: f64,
) -> f64 {
    let (lx, ly) = placement.effective_position();
    let lz = placement.mounting_height;

    // Vector from luminaire to surface point
    let dx = point.0 - lx;
    let dy = point.1 - ly;
    let dz = point.2 - lz;
    let r = (dx * dx + dy * dy + dz * dz).sqrt();
    if r < 1e-6 {
        return 0.0;
    }

    // Rotate into luminaire's local coordinate system
    let tilt_rad = placement.tilt_angle.to_radians();
    let rot_rad = placement.rotation.to_radians();

    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();
    let dx_r = dx * cos_r + dy * sin_r;
    let dy_r = -dx * sin_r + dy * cos_r;
    let dz_r = dz;

    let cos_t = tilt_rad.cos();
    let sin_t = tilt_rad.sin();
    let dx_rot = dx_r * cos_t + dz_r * sin_t;
    let dy_rot = dy_r;
    let dz_rot = -dx_r * sin_t + dz_r * cos_t;

    // C/gamma angles
    let gamma = (-dz_rot / r).clamp(-1.0, 1.0).acos();
    let c = dy_rot.atan2(dx_rot);
    let mut c_deg = c.to_degrees();
    if c_deg < 0.0 {
        c_deg += 360.0;
    }
    let gamma_deg = gamma.to_degrees();

    let intensity = ldt.sample(c_deg, gamma_deg);

    // Direction from surface point to luminaire (incoming light)
    let inv_r = 1.0 / r;
    let to_lum = (-dx * inv_r, -dy * inv_r, -dz * inv_r);

    // cos(incidence) = dot(normal, to_luminaire)
    let cos_incidence = normal.0 * to_lum.0 + normal.1 * to_lum.1 + normal.2 * to_lum.2;
    if cos_incidence <= 0.0 {
        return 0.0; // light comes from behind the surface
    }

    (intensity * flux_scale * cos_incidence / (r * r)).max(0.0)
}

/// Compute illuminance on a vertical wall surface.
///
/// The wall is defined by its grid of sample points and outward normal.
/// Returns a 2D grid [rows][cols] of lux values.
pub fn compute_wall_illuminance(
    ldt: &Eulumdat,
    placements: &[LuminairePlace],
    wall_points: &[Vec<(f64, f64, f64)>],
    normal: (f64, f64, f64),
    proration_factor: f64,
) -> Vec<Vec<f64>> {
    let total_flux: f64 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
        .sum();
    let flux_scale = total_flux / 1000.0 * proration_factor;

    wall_points
        .iter()
        .map(|row| {
            row.iter()
                .map(|&pt| {
                    placements
                        .iter()
                        .map(|p| compute_illuminance_at_point(ldt, p, pt, normal, flux_scale))
                        .sum()
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LampSet;

    fn test_ldt() -> Eulumdat {
        Eulumdat {
            c_angles: vec![0.0, 90.0, 180.0, 270.0],
            g_angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            intensities: vec![
                vec![300.0, 280.0, 220.0, 140.0, 60.0, 15.0, 3.0],
                vec![300.0, 270.0, 200.0, 120.0, 50.0, 12.0, 2.0],
                vec![300.0, 280.0, 220.0, 140.0, 60.0, 15.0, 3.0],
                vec![300.0, 270.0, 200.0, 120.0, 50.0, 12.0, 2.0],
            ],
            lamp_sets: vec![LampSet {
                num_lamps: 1,
                total_luminous_flux: 10000.0,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn single_luminaire_center() {
        let ldt = test_ldt();
        let placements = vec![LuminairePlace::simple(0, 20.0, 20.0, 10.0)];
        let result = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 20, 1.0);

        assert!(result.max_lux > 0.0, "Should produce illuminance");
        assert!(result.min_lux >= 0.0);
        assert!(result.avg_lux > 0.0);
        assert!(result.uniformity_min_avg <= 1.0);
        assert_eq!(result.lux_grid.len(), 20);
    }

    #[test]
    fn two_luminaires_higher_avg() {
        let ldt = test_ldt();
        let one = vec![LuminairePlace::simple(0, 20.0, 20.0, 10.0)];
        let two = vec![
            LuminairePlace::simple(0, 15.0, 20.0, 10.0),
            LuminairePlace::simple(1, 25.0, 20.0, 10.0),
        ];

        let r1 = compute_area_illuminance(&ldt, &one, 40.0, 40.0, 20, 1.0);
        let r2 = compute_area_illuminance(&ldt, &two, 40.0, 40.0, 20, 1.0);

        assert!(
            r2.avg_lux > r1.avg_lux,
            "Two luminaires should have higher average"
        );
    }

    #[test]
    fn proration_reduces_illuminance() {
        let ldt = test_ldt();
        let placements = vec![LuminairePlace::simple(0, 20.0, 20.0, 10.0)];

        let full = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 20, 1.0);
        let half = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 20, 0.5);

        let ratio = half.max_lux / full.max_lux;
        assert!(
            (ratio - 0.5).abs() < 0.01,
            "Half proration should halve illuminance, got ratio {ratio}"
        );
    }

    #[test]
    fn symmetric_luminaire_produces_symmetric_pattern() {
        // A rotationally symmetric luminaire at center should produce a symmetric grid
        let mut ldt = Eulumdat::new();
        ldt.symmetry = crate::Symmetry::VerticalAxis;
        ldt.num_c_planes = 1;
        ldt.c_plane_distance = 0.0;
        ldt.num_g_planes = 7;
        ldt.g_plane_distance = 15.0;
        ldt.c_angles = vec![0.0];
        ldt.g_angles = vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
        ldt.intensities = vec![vec![300.0, 280.0, 220.0, 140.0, 60.0, 15.0, 3.0]];
        ldt.lamp_sets = vec![LampSet {
            num_lamps: 1,
            total_luminous_flux: 10000.0,
            ..Default::default()
        }];

        let placements = vec![LuminairePlace::simple(0, 20.0, 20.0, 6.0)];
        let r = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 10, 1.0);

        let n = r.lux_grid.len();

        // Check left-right symmetry: grid[r][c] ≈ grid[r][n-1-c]
        let mut max_lr_diff = 0.0_f64;
        for row in 0..n {
            for col in 0..n / 2 {
                let diff = (r.lux_grid[row][col] - r.lux_grid[row][n - 1 - col]).abs();
                max_lr_diff = max_lr_diff.max(diff);
            }
        }
        assert!(
            max_lr_diff < 0.01,
            "Left-right symmetry broken: max diff = {max_lr_diff:.2}"
        );

        // Check top-bottom symmetry: grid[r][c] ≈ grid[n-1-r][c]
        let mut max_tb_diff = 0.0_f64;
        for row in 0..n / 2 {
            for col in 0..n {
                let diff = (r.lux_grid[row][col] - r.lux_grid[n - 1 - row][col]).abs();
                max_tb_diff = max_tb_diff.max(diff);
            }
        }
        assert!(
            max_tb_diff < 0.01,
            "Top-bottom symmetry broken: max diff = {max_tb_diff:.2}"
        );

        // Check diagonal symmetry: grid[r][c] ≈ grid[c][r]
        let mut max_diag_diff = 0.0_f64;
        for row in 0..n {
            for col in 0..n {
                let diff = (r.lux_grid[row][col] - r.lux_grid[col][row]).abs();
                max_diag_diff = max_diag_diff.max(diff);
            }
        }
        assert!(
            max_diag_diff < 0.01,
            "Diagonal symmetry broken: max diff = {max_diag_diff:.2}"
        );
    }

    #[test]
    fn asymmetric_4plane_c_angle_wraparound() {
        // test_ldt: C0=C180 and C90=C270, so the pattern must be LR and TB symmetric.
        // This specifically tests that C-angle interpolation wraps correctly at 360°→0°.
        let ldt = test_ldt();
        let placements = vec![LuminairePlace::simple(0, 20.0, 20.0, 6.0)];
        let r = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 10, 1.0);

        let n = r.lux_grid.len();

        // Check LR symmetry (C90 == C270 in test_ldt)
        let mut max_lr_diff = 0.0_f64;
        for row in 0..n {
            for col in 0..n / 2 {
                let diff = (r.lux_grid[row][col] - r.lux_grid[row][n - 1 - col]).abs();
                max_lr_diff = max_lr_diff.max(diff);
            }
        }
        assert!(
            max_lr_diff < 0.1,
            "Left-right symmetry broken: max diff = {max_lr_diff:.2}"
        );

        // Check TB symmetry (C0 == C180 in test_ldt)
        let mut max_tb_diff = 0.0_f64;
        for row in 0..n / 2 {
            for col in 0..n {
                let diff = (r.lux_grid[row][col] - r.lux_grid[n - 1 - row][col]).abs();
                max_tb_diff = max_tb_diff.max(diff);
            }
        }
        assert!(
            max_tb_diff < 0.1,
            "Top-bottom symmetry broken: max diff = {max_tb_diff:.2}"
        );
    }

    #[test]
    fn sample_c_angle_mirror_symmetry() {
        // test_ldt: C0=C180, C90=C270. So sample(C, g) == sample(360-C, g).
        let ldt = test_ldt();
        let g = 30.0;
        assert!((ldt.sample(31.0, g) - ldt.sample(329.0, g)).abs() < 0.01);
        assert!((ldt.sample(149.0, g) - ldt.sample(211.0, g)).abs() < 0.01);
        assert!((ldt.sample(45.0, g) - ldt.sample(135.0, g)).abs() < 0.01);
    }

    #[test]
    fn shifted_luminaire_same_isolated_pattern() {
        // A single luminaire at different positions produces identical patterns.
        let ldt = test_ldt();
        let left = vec![LuminairePlace::simple(0, 10.0, 10.0, 6.0)];
        let right = vec![LuminairePlace::simple(0, 30.0, 10.0, 6.0)];
        let r_l = compute_area_illuminance(&ldt, &left, 40.0, 40.0, 40, 1.0);
        let r_r = compute_area_illuminance(&ldt, &right, 40.0, 40.0, 40, 1.0);

        assert!(
            (r_l.lux_grid[9][9] - r_r.lux_grid[9][29]).abs() < 0.01,
            "Peak mismatch"
        );
        assert!(
            (r_l.lux_grid[5][12] - r_r.lux_grid[5][32]).abs() < 0.01,
            "Offset mismatch"
        );
    }

    #[test]
    fn four_luminaires_180_degree_rotational_symmetry() {
        // 2x2 grid has 180° rotational symmetry: cell (r,c) must equal cell (39-r,39-c).
        let ldt = test_ldt();
        let four = vec![
            LuminairePlace::simple(0, 10.0, 10.0, 6.0),
            LuminairePlace::simple(1, 30.0, 10.0, 6.0),
            LuminairePlace::simple(2, 10.0, 30.0, 6.0),
            LuminairePlace::simple(3, 30.0, 30.0, 6.0),
        ];
        let r = compute_area_illuminance(&ldt, &four, 40.0, 40.0, 40, 1.0);

        let mut max_diff = 0.0_f64;
        for row in 0..20 {
            for col in 0..40 {
                let diff = (r.lux_grid[row][col] - r.lux_grid[39 - row][39 - col]).abs();
                max_diff = max_diff.max(diff);
            }
        }
        assert!(
            max_diff < 0.001,
            "180° symmetry violated: max diff = {max_diff:.6}"
        );
    }

    #[test]
    fn rotation_shifts_pattern() {
        let ldt = test_ldt();
        let p0 = LuminairePlace::simple(0, 20.0, 20.0, 10.0);
        let mut p90 = p0.clone();
        p90.rotation = 90.0;

        let r0 = compute_area_illuminance(&ldt, &[p0], 40.0, 40.0, 20, 1.0);
        let r90 = compute_area_illuminance(&ldt, &[p90], 40.0, 40.0, 20, 1.0);

        // Max lux should be the same (just rotated)
        assert!(
            (r0.max_lux - r90.max_lux).abs() < 1.0,
            "Rotation should not change max lux"
        );
    }

    #[test]
    fn mixed_matches_single_when_all_same() {
        let ldt = test_ldt();
        let placements = vec![
            LuminairePlace::simple(0, 15.0, 20.0, 10.0),
            LuminairePlace::simple(1, 25.0, 20.0, 10.0),
        ];
        let indices = vec![0, 0]; // both use same LDT

        let r_single = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 20, 1.0);
        let r_mixed =
            compute_area_illuminance_mixed(&[&ldt], &placements, &indices, 40.0, 40.0, 20, 1.0);

        assert!(
            (r_single.avg_lux - r_mixed.avg_lux).abs() < 0.001,
            "Mixed with same LDT should equal single: {} vs {}",
            r_single.avg_lux,
            r_mixed.avg_lux,
        );
    }

    #[test]
    fn mixed_two_different_ldts() {
        let ldt1 = test_ldt();
        // Create a dimmer LDT (half intensity)
        let mut ldt2 = test_ldt();
        for plane in &mut ldt2.intensities {
            for val in plane.iter_mut() {
                *val *= 0.5;
            }
        }

        let placements = vec![
            LuminairePlace::simple(0, 15.0, 20.0, 10.0),
            LuminairePlace::simple(1, 25.0, 20.0, 10.0),
        ];

        // Both bright
        let r_both_bright = compute_area_illuminance(&ldt1, &placements, 40.0, 40.0, 20, 1.0);
        // Mixed: first bright, second dim
        let r_mixed = compute_area_illuminance_mixed(
            &[&ldt1, &ldt2],
            &placements,
            &[0, 1],
            40.0,
            40.0,
            20,
            1.0,
        );
        // Both dim
        let r_both_dim = compute_area_illuminance(&ldt2, &placements, 40.0, 40.0, 20, 1.0);

        // Mixed average should be between all-bright and all-dim
        assert!(
            r_mixed.avg_lux < r_both_bright.avg_lux,
            "Mixed should be less than all bright"
        );
        assert!(
            r_mixed.avg_lux > r_both_dim.avg_lux,
            "Mixed should be more than all dim"
        );
    }

    #[test]
    fn polygon_rectangle_matches_standard() {
        use crate::area::polygon::AreaPolygon;

        let ldt = test_ldt();
        let placements = vec![LuminairePlace::simple(0, 20.0, 20.0, 10.0)];

        let r_rect = compute_area_illuminance(&ldt, &placements, 40.0, 40.0, 20, 1.0);
        let poly = AreaPolygon::rectangle(40.0, 40.0);
        let r_poly = compute_area_illuminance_polygon(&ldt, &placements, &poly, 20, 1.0);

        // Should match since rectangular polygon = rectangle
        assert!(
            (r_rect.avg_lux - r_poly.avg_lux).abs() < 0.01,
            "Polygon rectangle should match standard: {:.2} vs {:.2}",
            r_rect.avg_lux,
            r_poly.avg_lux,
        );
        assert!(r_poly.mask.is_some());
    }

    #[test]
    fn polygon_triangle_excludes_cells() {
        use crate::area::polygon::AreaPolygon;

        let ldt = test_ldt();
        let placements = vec![LuminairePlace::simple(0, 20.0, 20.0, 10.0)];

        // Triangle covering roughly half the 40x40 area
        let poly = AreaPolygon::new(vec![(0.0, 0.0), (40.0, 0.0), (20.0, 40.0)]);
        let r = compute_area_illuminance_polygon(&ldt, &placements, &poly, 20, 1.0);

        // Mask should exist and ~half the cells should be inside
        let mask = r.mask.as_ref().unwrap();
        let inside: usize = mask.iter().flat_map(|r| r.iter()).filter(|&&v| v).count();
        let total = 20 * 20;
        let ratio = inside as f64 / total as f64;
        assert!(
            ratio > 0.35 && ratio < 0.65,
            "Triangle should mask ~50% of cells, got {ratio:.2}"
        );
    }
}
