//! Zonal Cavity Method computation engine.
//!
//! Implements the IES Zonal Cavity Method for calculating the number of
//! luminaires needed to achieve a target illuminance in a rectangular room.

use crate::area::{compute_area_illuminance, LuminairePlace};
use crate::calculations::{CuTable, PhotometricCalculations};
use crate::Eulumdat;

/// Rectangular room parameters for zonal cavity calculation.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Room {
    /// Length in meters
    pub length: f64,
    /// Width in meters
    pub width: f64,
    /// Total floor-to-ceiling height in meters
    pub height: f64,
    /// Workplane height above floor in meters (typically 0.80)
    pub workplane_height: f64,
    /// Suspension length from ceiling in meters
    pub suspension_length: f64,
}

impl Room {
    pub fn new(
        length: f64,
        width: f64,
        height: f64,
        workplane_height: f64,
        suspension_length: f64,
    ) -> Self {
        Self {
            length,
            width,
            height,
            workplane_height,
            suspension_length,
        }
    }

    /// Room Cavity height: distance from luminaire plane to workplane.
    pub fn hrc(&self) -> f64 {
        (self.height - self.suspension_length - self.workplane_height).max(0.0)
    }

    /// Ceiling Cavity height: suspension length.
    pub fn hcc(&self) -> f64 {
        self.suspension_length
    }

    /// Floor Cavity height: workplane height above floor.
    pub fn hfc(&self) -> f64 {
        self.workplane_height
    }

    /// Floor area in square meters.
    pub fn area(&self) -> f64 {
        self.length * self.width
    }

    /// Perimeter in meters.
    pub fn perimeter(&self) -> f64 {
        2.0 * (self.length + self.width)
    }
}

/// Surface reflectances (0.0 to 1.0).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Reflectances {
    /// Ceiling reflectance
    pub ceiling: f64,
    /// Wall reflectance
    pub wall: f64,
    /// Floor reflectance
    pub floor: f64,
}

impl Reflectances {
    pub fn new(ceiling: f64, wall: f64, floor: f64) -> Self {
        Self {
            ceiling,
            wall,
            floor,
        }
    }
}

/// Light Loss Factor components.
#[derive(Debug, Clone, PartialEq)]
pub struct LightLossFactor {
    /// Lamp Lumen Depreciation (LLD)
    pub lld: f64,
    /// Luminaire Dirt Depreciation (LDD)
    pub ldd: f64,
    /// Ballast Factor (BF)
    pub ballast_factor: f64,
    /// Room Surface Dirt Depreciation (RSDD)
    pub rsdd: f64,
}

impl LightLossFactor {
    pub fn new(lld: f64, ldd: f64, ballast_factor: f64, rsdd: f64) -> Self {
        Self {
            lld,
            ldd,
            ballast_factor,
            rsdd,
        }
    }

    /// Total LLF = product of all factors.
    pub fn total(&self) -> f64 {
        self.lld * self.ldd * self.ballast_factor * self.rsdd
    }
}

/// Cavity ratio results.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CavityResults {
    /// Room Cavity Ratio
    pub rcr: f64,
    /// Ceiling Cavity Ratio
    pub ccr: f64,
    /// Floor Cavity Ratio
    pub fcr: f64,
    /// Effective ceiling cavity reflectance
    pub rho_cc_eff: f64,
    /// Effective floor cavity reflectance
    pub rho_fc_eff: f64,
}

/// Luminaire layout on the ceiling grid.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LuminaireLayout {
    /// Number of luminaires along length
    pub rows: usize,
    /// Number of luminaires along width
    pub cols: usize,
    /// Total luminaire count
    pub count: usize,
    /// Spacing along length in meters
    pub spacing_x: f64,
    /// Spacing along width in meters
    pub spacing_y: f64,
    /// Offset from wall along length
    pub offset_x: f64,
    /// Offset from wall along width
    pub offset_y: f64,
    /// Spacing-to-mounting-height ratio along length
    pub s_mh_x: f64,
    /// Spacing-to-mounting-height ratio along width
    pub s_mh_y: f64,
    /// Whether spacing meets the criterion
    pub spacing_ok: bool,
}

/// Solve mode for the zonal calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveMode {
    /// Given target illuminance, find number of luminaires
    TargetToCount,
    /// Given fixed count, find achieved illuminance
    CountToIlluminance,
    /// Given target LPD, find count and achieved illuminance
    TargetToLpd,
}

/// Point-by-point overlay result.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PpbResult {
    /// Illuminance grid (row-major)
    pub lux_grid: Vec<Vec<f64>>,
    pub min_lux: f64,
    pub avg_lux: f64,
    pub max_lux: f64,
    /// U₀ = min/avg
    pub uniformity_min_avg: f64,
    /// Ud = min/max
    pub uniformity_min_max: f64,
    pub grid_resolution: usize,
}

/// Complete result of zonal cavity calculation.
#[derive(Debug, Clone, PartialEq)]
pub struct ZonalResult {
    /// Cavity ratio results
    pub cavity: CavityResults,
    /// Interpolated Coefficient of Utilization (as fraction, 0–1.5)
    pub cu: f64,
    /// Luminaire layout
    pub layout: LuminaireLayout,
    /// Target illuminance (lux)
    pub target_illuminance: f64,
    /// Achieved illuminance (lux)
    pub achieved_illuminance: f64,
    /// Luminaire power in watts
    pub luminaire_watts: f64,
    /// Luminaire lumens
    pub luminaire_lumens: f64,
    /// Lighting Power Density (W/m²)
    pub lpd: f64,
    /// Total LLF used
    pub llf_total: f64,
    /// Spacing criterion (S/MH) from IES
    pub spacing_criterion: f64,
    /// Optional point-by-point overlay
    pub ppb: Option<PpbResult>,
}

// ─── Cavity ratio computation ───────────────────────────────────────────────

/// Compute cavity ratio: CR = 5·h·(L+W)/(L·W)
fn cavity_ratio(h: f64, length: f64, width: f64) -> f64 {
    if length * width <= 0.0 {
        return 0.0;
    }
    5.0 * h * (length + width) / (length * width)
}

/// Effective cavity reflectance using IES approximation.
///
/// When CR=0, the cavity collapses to a flat surface → return rho_top directly.
pub fn effective_cavity_reflectance(rho_top: f64, rho_wall: f64, cr: f64) -> f64 {
    if cr < 0.01 {
        return rho_top;
    }
    // IES formula: ρ_eff = ρ_top · ρ_wall / (ρ_top + ρ_wall · (1 - ρ_top) · cr / 5)
    // Simplified approximation that matches published tables:
    let a = rho_top * rho_wall;
    let b = rho_wall * (1.0 - rho_top);
    let denom = rho_top + b * (cr / 5.0);
    if denom > 0.001 {
        (a / denom).clamp(0.0, 1.0)
    } else {
        // Both reflectances near zero
        0.0
    }
}

/// Compute all cavity ratios and effective reflectances.
pub fn compute_cavity_ratios(room: &Room, reflectances: &Reflectances) -> CavityResults {
    let rcr = cavity_ratio(room.hrc(), room.length, room.width);
    let ccr = cavity_ratio(room.hcc(), room.length, room.width);
    let fcr = cavity_ratio(room.hfc(), room.length, room.width);

    let rho_cc_eff = effective_cavity_reflectance(reflectances.ceiling, reflectances.wall, ccr);
    let rho_fc_eff = effective_cavity_reflectance(reflectances.floor, reflectances.wall, fcr);

    CavityResults {
        rcr,
        ccr,
        fcr,
        rho_cc_eff,
        rho_fc_eff,
    }
}

// ─── CU interpolation ──────────────────────────────────────────────────────

/// Interpolate CU from a pre-computed CU table.
///
/// Uses bilinear interpolation between RCR rows and finds the closest
/// reflectance columns matching (ceiling%, wall%).
pub fn interpolate_cu(cu_table: &CuTable, rcr: f64, rho_cc_eff: f64, rho_w: f64) -> f64 {
    if cu_table.values.is_empty() {
        return 0.5; // fallback
    }

    // Find best column index for reflectance match
    let col_idx = find_best_reflectance_column(cu_table, rho_cc_eff, rho_w);

    // RCR interpolation: RCR values are integers 0..10
    let rcr_clamped = rcr.clamp(0.0, 10.0);
    let rcr_low = rcr_clamped.floor() as usize;
    let rcr_high = (rcr_low + 1).min(10);
    let rcr_frac = rcr_clamped - rcr_low as f64;

    let val_low = cu_table
        .values
        .get(rcr_low)
        .and_then(|row| row.get(col_idx))
        .copied()
        .unwrap_or(50.0);
    let val_high = cu_table
        .values
        .get(rcr_high)
        .and_then(|row| row.get(col_idx))
        .copied()
        .unwrap_or(50.0);

    let cu_pct = val_low + (val_high - val_low) * rcr_frac;
    cu_pct / 100.0 // return as fraction
}

/// Find the CU table column that best matches the given ceiling and wall reflectances.
fn find_best_reflectance_column(cu_table: &CuTable, rho_cc: f64, rho_w: f64) -> usize {
    let rc_pct = (rho_cc * 100.0).round() as i32;
    let rw_pct = (rho_w * 100.0).round() as i32;

    cu_table
        .reflectances
        .iter()
        .enumerate()
        .min_by_key(|(_, &(rc, rw, _))| {
            let dc = (rc as i32 - rc_pct).abs();
            let dw = (rw as i32 - rw_pct).abs();
            dc * 2 + dw // weight ceiling match slightly more
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

// ─── Layout generator ───────────────────────────────────────────────────────

/// Find the best rectangular layout for N luminaires in the room.
///
/// Tries all factor pairs of N, picks the one whose spacing ratio best
/// matches the room aspect ratio. Offsets are half-spacing from walls.
pub fn find_best_layout(
    n: usize,
    room_length: f64,
    room_width: f64,
    spacing_criterion: f64,
    hrc: f64,
) -> LuminaireLayout {
    if n == 0 {
        return LuminaireLayout {
            rows: 0,
            cols: 0,
            count: 0,
            spacing_x: 0.0,
            spacing_y: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            s_mh_x: 0.0,
            s_mh_y: 0.0,
            spacing_ok: true,
        };
    }

    let room_ratio = if room_width > 0.0 {
        room_length / room_width
    } else {
        1.0
    };

    // Try factor pairs of n, n+1, n+2.  Accept the first candidate whose
    // spacing passes the S/MH criterion.  If none passes, fall back to the
    // best-aspect-ratio layout for the original n.
    let mut fallback_rows = 1;
    let mut fallback_cols = n;
    let mut fallback_diff = f64::MAX;
    let mut found_valid: Option<(usize, usize, usize)> = None; // (rows, cols, count)

    'outer: for candidate in [n, n + 1, n + 2] {
        let mut best_rows = 1;
        let mut best_cols = candidate;
        let mut best_diff = f64::MAX;

        for r in 1..=candidate {
            if candidate % r == 0 {
                let c = candidate / r;
                let ratio = if c > 0 { r as f64 / c as f64 } else { 1.0 };
                let diff = (ratio - room_ratio).abs();
                if diff < best_diff {
                    best_diff = diff;
                    best_rows = r;
                    best_cols = c;
                }
            }
        }

        let (rows, cols) = if room_length >= room_width {
            (best_rows, best_cols)
        } else {
            (best_cols, best_rows)
        };

        let sx = room_length / rows as f64;
        let sy = room_width / cols as f64;
        let smh_x = if hrc > 0.0 { sx / hrc } else { 0.0 };
        let smh_y = if hrc > 0.0 { sy / hrc } else { 0.0 };

        // Record the first candidate (original n) as fallback
        if candidate == n && best_diff < fallback_diff {
            fallback_diff = best_diff;
            fallback_rows = rows;
            fallback_cols = cols;
        }

        if smh_x <= spacing_criterion && smh_y <= spacing_criterion {
            found_valid = Some((rows, cols, candidate));
            break 'outer;
        }
    }

    let (rows, cols, count) = found_valid.unwrap_or((fallback_rows, fallback_cols, n));

    let spacing_x = room_length / rows as f64;
    let spacing_y = room_width / cols as f64;
    let offset_x = spacing_x / 2.0;
    let offset_y = spacing_y / 2.0;

    let s_mh_x = if hrc > 0.0 { spacing_x / hrc } else { 0.0 };
    let s_mh_y = if hrc > 0.0 { spacing_y / hrc } else { 0.0 };

    let spacing_ok = s_mh_x <= spacing_criterion && s_mh_y <= spacing_criterion;

    LuminaireLayout {
        rows,
        cols,
        count,
        spacing_x,
        spacing_y,
        offset_x,
        offset_y,
        s_mh_x,
        s_mh_y,
        spacing_ok,
    }
}

// ─── Main compute function ──────────────────────────────────────────────────

/// Main zonal cavity computation.
///
/// Depending on `mode`:
/// - `TargetToCount`: computes N from target illuminance
/// - `CountToIlluminance`: computes E from fixed count
/// - `TargetToLpd`: computes N from target LPD, then E
pub fn compute_zonal(
    ldt: &Eulumdat,
    room: &Room,
    reflectances: &Reflectances,
    llf: &LightLossFactor,
    target_illuminance: f64,
    cu_table: &CuTable,
    mode: SolveMode,
    fixed_count: Option<usize>,
    target_lpd: Option<f64>,
) -> ZonalResult {
    // Extract luminaire data
    let luminaire_lumens: f64 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.total_luminous_flux * ls.num_lamps.unsigned_abs() as f64)
        .sum();
    let luminaire_watts: f64 = ldt
        .lamp_sets
        .iter()
        .map(|ls| ls.wattage_with_ballast * ls.num_lamps.unsigned_abs() as f64)
        .sum();
    let luminaire_watts = if luminaire_watts > 0.0 {
        luminaire_watts
    } else {
        luminaire_lumens / 100.0 // rough fallback: 100 lm/W
    };

    // Cavity ratios
    let cavity = compute_cavity_ratios(room, reflectances);

    // CU interpolation
    let cu = interpolate_cu(cu_table, cavity.rcr, cavity.rho_cc_eff, reflectances.wall);

    // Spacing criterion
    let sc_c0 = PhotometricCalculations::spacing_criterion_ies(ldt, 0.0, 0.7);
    let sc_c90 = PhotometricCalculations::spacing_criterion_ies(ldt, 90.0, 0.7);
    let spacing_criterion = sc_c0.min(sc_c90);

    let llf_total = llf.total();
    let area = room.area();
    let hrc = room.hrc();

    // Compute luminaire count based on mode
    let count = match mode {
        SolveMode::TargetToCount => {
            // N = E × A / (Φ × CU × LLF)
            let denom = luminaire_lumens * cu * llf_total;
            if denom > 0.0 {
                ((target_illuminance * area / denom).ceil() as usize).max(1)
            } else {
                1
            }
        }
        SolveMode::CountToIlluminance => fixed_count.unwrap_or(1),
        SolveMode::TargetToLpd => {
            // N = target_LPD × A / W_luminaire
            let lpd_target = target_lpd.unwrap_or(10.0);
            if luminaire_watts > 0.0 {
                ((lpd_target * area / luminaire_watts).ceil() as usize).max(1)
            } else {
                1
            }
        }
    };

    // Layout
    let layout = find_best_layout(count, room.length, room.width, spacing_criterion, hrc);

    // Achieved illuminance: E = N × Φ × CU × LLF / A
    let achieved = if area > 0.0 {
        layout.count as f64 * luminaire_lumens * cu * llf_total / area
    } else {
        0.0
    };

    // LPD
    let lpd = if area > 0.0 {
        layout.count as f64 * luminaire_watts / area
    } else {
        0.0
    };

    ZonalResult {
        cavity,
        cu,
        layout,
        target_illuminance,
        achieved_illuminance: achieved,
        luminaire_watts,
        luminaire_lumens,
        lpd,
        llf_total,
        spacing_criterion,
        ppb: None,
    }
}

// ─── Point-by-point overlay ─────────────────────────────────────────────────

/// Compute point-by-point illuminance overlay on the workplane.
///
/// Constructs `LuminairePlace` instances from the layout grid and calls
/// `compute_area_illuminance()`, then adds a uniform reflected component
/// to approximate the inter-reflected light from zonal cavity.
pub fn compute_ppb_overlay(
    ldt: &Eulumdat,
    layout: &LuminaireLayout,
    room: &Room,
    grid_resolution: usize,
    llf_total: f64,
    _cu: f64,
    avg_zonal: f64,
) -> PpbResult {
    let hrc = room.hrc();

    // Build placements from layout grid
    let mut placements = Vec::with_capacity(layout.count);
    let mut id = 0;
    for r in 0..layout.rows {
        for c in 0..layout.cols {
            let x = layout.offset_x + r as f64 * layout.spacing_x;
            let y = layout.offset_y + c as f64 * layout.spacing_y;
            placements.push(LuminairePlace::simple(id, x, y, hrc));
            id += 1;
        }
    }

    // Compute direct illuminance using area compute engine
    let area_result = compute_area_illuminance(
        ldt,
        &placements,
        room.length,
        room.width,
        grid_resolution,
        llf_total,
    );

    // Add uniform reflected component:
    // The zonal cavity method gives total average E (direct + reflected).
    // The area compute gives direct-only average.
    // Reflected component ≈ avg_zonal - avg_direct
    let reflected = (avg_zonal - area_result.avg_lux).max(0.0);

    let n = grid_resolution;
    let mut lux_grid = area_result.lux_grid;
    let mut min_lux = f64::MAX;
    let mut max_lux = 0.0_f64;
    let mut sum = 0.0;
    let mut count = 0;

    for row in lux_grid.iter_mut() {
        for val in row.iter_mut() {
            let val: &mut f64 = val;
            *val += reflected;
            min_lux = min_lux.min(*val);
            max_lux = max_lux.max(*val);
            sum += *val;
            count += 1;
        }
    }

    let avg = if count > 0 { sum / count as f64 } else { 0.0 };
    let u_min_avg = if avg > 0.0 { min_lux / avg } else { 0.0 };
    let u_min_max = if max_lux > 0.0 {
        min_lux / max_lux
    } else {
        0.0
    };

    PpbResult {
        lux_grid,
        min_lux,
        avg_lux: avg,
        max_lux,
        uniformity_min_avg: u_min_avg,
        uniformity_min_max: u_min_max,
        grid_resolution: n,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CU_REFLECTANCES;

    #[test]
    fn test_cavity_ratio_formula() {
        // CR = 5 * h * (L + W) / (L * W)
        // For 10×8 room, h=2.2: CR = 5 * 2.2 * 18 / 80 = 2.475
        let cr = cavity_ratio(2.2, 10.0, 8.0);
        assert!((cr - 2.475).abs() < 0.001, "CR = {cr}");
    }

    #[test]
    fn test_cavity_ratio_square_room() {
        // For square room L=W=S: CR = 5 * h * 2S / S² = 10h/S
        let cr = cavity_ratio(3.0, 10.0, 10.0);
        assert!((cr - 3.0).abs() < 0.001, "CR = {cr}");
    }

    #[test]
    fn test_effective_cavity_reflectance_cr_zero() {
        // When CR=0, should return rho_top directly
        let rho = effective_cavity_reflectance(0.70, 0.50, 0.0);
        assert!((rho - 0.70).abs() < 0.001, "rho = {rho}");
    }

    #[test]
    fn test_effective_cavity_reflectance_nonzero() {
        let rho = effective_cavity_reflectance(0.70, 0.50, 2.0);
        // Should be less than 0.70 (degraded by wall interaction)
        assert!(rho < 0.70, "rho = {rho}");
        assert!(rho > 0.30, "rho = {rho}");
    }

    #[test]
    fn test_room_cavity_heights() {
        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.15);
        assert!((room.hrc() - 2.05).abs() < 0.001);
        assert!((room.hcc() - 0.15).abs() < 0.001);
        assert!((room.hfc() - 0.80).abs() < 0.001);
        assert!((room.area() - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_layout_square_room() {
        let layout = find_best_layout(9, 10.0, 10.0, 1.5, 2.2);
        assert_eq!(layout.rows, 3);
        assert_eq!(layout.cols, 3);
        assert_eq!(layout.count, 9);
        assert!((layout.spacing_x - 10.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_layout_rectangular_room() {
        // 27 luminaires in 20×15: should be 9×3 or 3×9
        let layout = find_best_layout(27, 20.0, 15.0, 1.5, 2.2);
        assert_eq!(layout.count, 27);
        // 27 = 3×9 or 9×3
        assert!(
            (layout.rows == 9 && layout.cols == 3) || (layout.rows == 3 && layout.cols == 9),
            "Got {}x{}",
            layout.rows,
            layout.cols
        );
    }

    #[test]
    fn test_llf_total() {
        let llf = LightLossFactor::new(0.90, 0.95, 1.0, 0.98);
        let total = llf.total();
        let expected = 0.90 * 0.95 * 1.0 * 0.98;
        assert!((total - expected).abs() < 0.001);
    }

    #[test]
    fn test_find_best_reflectance_column() {
        let cu_table = CuTable::default();
        // Exact match for RC=80, RW=50 → column 1
        let col = find_best_reflectance_column(&cu_table, 0.80, 0.50);
        assert_eq!(CU_REFLECTANCES[col], (80, 50, 20));
    }

    #[test]
    fn test_find_best_layout_fallback_to_n_plus_1() {
        // Prime number 7 only factors as 1×7 or 7×1.
        // In a nearly-square room with tight spacing criterion, the 1×7
        // layout may fail. The fallback should try 8 = 2×4 which fits better.
        let layout = find_best_layout(7, 6.0, 5.0, 1.0, 2.0);
        // Should still return a valid layout (may be 7 or 8 depending on spacing)
        assert!(layout.count >= 7);
        assert!(layout.count <= 9); // at most n+2
        assert!(layout.rows > 0 && layout.cols > 0);
    }

    #[test]
    fn test_zonal_e2e_target_to_count() {
        let ldt_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../eulumdat-wasm/templates/fluorescent_luminaire.ldt"
        );
        let ldt =
            crate::Eulumdat::from_file(ldt_path).expect("Failed to load fluorescent_luminaire.ldt");

        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.0);
        let reflectances = Reflectances::new(0.70, 0.50, 0.20);
        let llf = LightLossFactor::new(0.90, 0.95, 1.0, 0.98);

        let cu_table = CuTable::calculate(&ldt);

        let result = compute_zonal(
            &ldt,
            &room,
            &reflectances,
            &llf,
            500.0,
            &cu_table,
            SolveMode::TargetToCount,
            None,
            None,
        );

        assert!(
            result.layout.count > 0,
            "Should need at least one luminaire, got {}",
            result.layout.count
        );
        assert!(
            result.achieved_illuminance > 0.0,
            "Achieved illuminance should be positive, got {}",
            result.achieved_illuminance
        );
        assert!(
            result.cu > 0.0 && result.cu < 1.0,
            "CU should be between 0 and 1, got {}",
            result.cu
        );
        assert!(
            result.cavity.rcr > 0.0,
            "RCR should be positive, got {}",
            result.cavity.rcr
        );
        assert!(
            result.lpd > 0.0,
            "LPD should be positive, got {}",
            result.lpd
        );
        assert!(
            result.layout.rows > 0 && result.layout.cols > 0,
            "Layout should have positive rows ({}) and cols ({})",
            result.layout.rows,
            result.layout.cols
        );
    }

    #[test]
    fn test_zonal_e2e_count_to_illuminance() {
        let ldt_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../eulumdat-wasm/templates/fluorescent_luminaire.ldt"
        );
        let ldt =
            crate::Eulumdat::from_file(ldt_path).expect("Failed to load fluorescent_luminaire.ldt");

        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.0);
        let reflectances = Reflectances::new(0.70, 0.50, 0.20);
        let llf = LightLossFactor::new(0.90, 0.95, 1.0, 0.98);

        let cu_table = CuTable::calculate(&ldt);

        let result = compute_zonal(
            &ldt,
            &room,
            &reflectances,
            &llf,
            0.0, // not used in CountToIlluminance mode
            &cu_table,
            SolveMode::CountToIlluminance,
            Some(12),
            None,
        );

        assert_eq!(
            result.layout.count, 12,
            "Fixed count should be 12, got {}",
            result.layout.count
        );
        assert!(
            result.achieved_illuminance > 0.0,
            "Achieved illuminance should be positive with 12 luminaires, got {}",
            result.achieved_illuminance
        );
        assert!(
            result.cu > 0.0 && result.cu < 1.0,
            "CU should be between 0 and 1, got {}",
            result.cu
        );
        assert!(
            result.layout.rows > 0 && result.layout.cols > 0,
            "Layout should have positive rows ({}) and cols ({})",
            result.layout.rows,
            result.layout.cols
        );
    }
}
