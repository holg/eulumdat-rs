//! Zonal Cavity Method computation engine.
//!
//! Implements the IES Zonal Cavity Method for calculating the number of
//! luminaires needed to achieve a target illuminance in a rectangular room.

use crate::area::{compute_area_illuminance, LuminairePlace};
use crate::calculations::{CuTable, PhotometricCalculations};
use crate::Eulumdat;

/// Room parameters for zonal cavity calculation: a rectangle, or any
/// floor outline inside the `length` × `width` bounds.
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
    /// Floor outline (x along the length, y along the width, m) inside
    /// `0..length` × `0..width`; `None` = the full rectangle.
    #[cfg_attr(feature = "serde", serde(default))]
    pub outline: Option<Vec<(f64, f64)>>,
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
            outline: None,
        }
    }

    /// The same room with a floor outline (ignored with fewer than 3 points).
    pub fn with_outline(mut self, outline: Option<Vec<(f64, f64)>>) -> Self {
        self.outline = outline.filter(|o| o.len() >= 3);
        self
    }

    /// The point (x, y) lies on the floor (inside the outline).
    pub fn contains(&self, x: f64, y: f64) -> bool {
        match self.outline.as_ref() {
            Some(o) => crate::area::AreaPolygon::new(o.clone()).contains(x, y),
            None => (0.0..=self.length).contains(&x) && (0.0..=self.width).contains(&y),
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
        match self.outline.as_ref() {
            Some(o) => crate::area::AreaPolygon::new(o.clone()).area(),
            None => self.length * self.width,
        }
    }

    /// Perimeter in meters.
    pub fn perimeter(&self) -> f64 {
        match self.outline.as_ref() {
            Some(o) => (0..o.len())
                .map(|i| {
                    let (a, b) = (o[i], o[(i + 1) % o.len()]);
                    (b.0 - a.0).hypot(b.1 - a.1)
                })
                .sum(),
            None => 2.0 * (self.length + self.width),
        }
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
    /// Luminaires moved off the grid by hand: (index in `positions()`
    /// order, x, y) in m. The spacing values describe the grid, not these.
    #[cfg_attr(feature = "serde", serde(default))]
    pub moved: Vec<(usize, f64, f64)>,
    /// A room with an outline: the grid points that lie on its floor
    /// (indices in `grid_positions()` order); `None` = all of them.
    #[cfg_attr(feature = "serde", serde(default))]
    pub keep: Option<Vec<usize>>,
}

impl LuminaireLayout {
    /// Positions on the regular grid (m), row by row: x along the room
    /// length, y along the width.
    pub fn grid_positions(&self) -> Vec<(f64, f64)> {
        let mut v = Vec::with_capacity(self.rows * self.cols);
        for r in 0..self.rows {
            for c in 0..self.cols {
                v.push((
                    self.offset_x + r as f64 * self.spacing_x,
                    self.offset_y + c as f64 * self.spacing_y,
                ));
            }
        }
        v
    }

    /// Luminaire positions in plan coordinates (m): the grid points on the
    /// floor, with the moved luminaires at their new places.
    pub fn positions(&self) -> Vec<(f64, f64)> {
        let grid = self.grid_positions();
        let mut v: Vec<(f64, f64)> = match self.keep.as_ref() {
            Some(keep) => keep.iter().filter_map(|&i| grid.get(i).copied()).collect(),
            None => grid,
        };
        for &(i, x, y) in &self.moved {
            if let Some(p) = v.get_mut(i) {
                *p = (x, y);
            }
        }
        v
    }

    /// Some luminaires are off the grid.
    pub fn is_moved(&self) -> bool {
        self.moved.iter().any(|&(i, _, _)| i < self.count)
    }
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
    /// Cells on the floor of a room with an outline (`None` = all); the
    /// statistics count only these.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mask: Option<Vec<Vec<bool>>>,
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
#[cfg(test)]
fn cavity_ratio(h: f64, length: f64, width: f64) -> f64 {
    cavity_ratio_general(h, length * width, 2.0 * (length + width))
}

/// Cavity ratio of any floor shape: CR = 2.5·h·P/A (= 5·h·(L+W)/(L·W)
/// for a rectangle).
fn cavity_ratio_general(h: f64, area: f64, perimeter: f64) -> f64 {
    if area <= 0.0 {
        return 0.0;
    }
    2.5 * h * perimeter / area
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
    let (area, perimeter) = (room.area(), room.perimeter());
    let rcr = cavity_ratio_general(room.hrc(), area, perimeter);
    let ccr = cavity_ratio_general(room.hcc(), area, perimeter);
    let fcr = cavity_ratio_general(room.hfc(), area, perimeter);

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
            moved: Vec::new(),
            keep: None,
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
        moved: Vec::new(),
        keep: None,
    }
}

/// Layout for a room with an outline: the regular grid over the bounds,
/// refined until at least `n` of its points lie on the floor (`keep` lists
/// them; `count` is their number). A rectangle gets `find_best_layout`.
pub fn find_layout_in_room(n: usize, room: &Room, spacing_criterion: f64) -> LuminaireLayout {
    let hrc = room.hrc();
    if room.outline.is_none() || n == 0 {
        return find_best_layout(n, room.length, room.width, spacing_criterion, hrc);
    }
    let fill = (room.area() / (room.length * room.width).max(1e-9)).clamp(0.05, 1.0);
    let mut candidate = ((n as f64 / fill).ceil() as usize).max(n);
    let mut best: Option<LuminaireLayout> = None;
    for _ in 0..60 {
        let mut layout =
            find_best_layout(candidate, room.length, room.width, spacing_criterion, hrc);
        let keep: Vec<usize> = layout
            .grid_positions()
            .iter()
            .enumerate()
            .filter(|(_, p)| room.contains(p.0, p.1))
            .map(|(i, _)| i)
            .collect();
        layout.count = keep.len();
        layout.keep = Some(keep);
        let enough = layout.count >= n;
        best = Some(layout);
        if enough {
            break;
        }
        candidate += (candidate / 8).max(1);
    }
    best.unwrap_or_else(|| find_best_layout(n, room.length, room.width, spacing_criterion, hrc))
}

// ─── Main compute function ──────────────────────────────────────────────────

/// Main zonal cavity computation.
///
/// Depending on `mode`:
/// - `TargetToCount`: computes N from target illuminance
/// - `CountToIlluminance`: computes E from fixed count
/// - `TargetToLpd`: computes N from target LPD, then E
#[allow(clippy::too_many_arguments)]
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
    let luminaire_lumens: f64 = ldt.luminous_flux();
    let luminaire_watts: f64 = ldt.wattage();
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

    // Layout (only grid points on the floor of a room with an outline)
    let layout = find_layout_in_room(count, room, spacing_criterion);

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
/// Constructs `LuminairePlace` instances from the layout positions (moved
/// luminaires included) and calls
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

    // Build placements from the layout (moved luminaires included)
    let placements: Vec<LuminairePlace> = layout
        .positions()
        .into_iter()
        .enumerate()
        .map(|(id, (x, y))| LuminairePlace::simple(id, x, y, hrc))
        .collect();

    // Compute direct illuminance using area compute engine (cells on the
    // floor only for a room with an outline)
    let area_result = match room.outline.as_ref() {
        Some(o) => crate::area::compute_area_illuminance_polygon(
            ldt,
            &placements,
            &crate::area::AreaPolygon::new(o.clone()),
            grid_resolution,
            llf_total,
        ),
        None => compute_area_illuminance(
            ldt,
            &placements,
            room.length,
            room.width,
            grid_resolution,
            llf_total,
        ),
    };
    let mask = area_result.mask.clone();

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

    for (r, row) in lux_grid.iter_mut().enumerate() {
        for (c, val) in row.iter_mut().enumerate() {
            let val: &mut f64 = val;
            *val += reflected;
            if mask.as_ref().is_some_and(|m| !m[r][c]) {
                continue;
            }
            min_lux = min_lux.min(*val);
            max_lux = max_lux.max(*val);
            sum += *val;
            count += 1;
        }
    }
    if count == 0 {
        min_lux = 0.0;
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
        mask,
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

    #[test]
    fn moved_luminaires_change_positions_ppb_and_plan() {
        let ldt_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../eulumdat-wasm/templates/fluorescent_luminaire.ldt"
        );
        let ldt = crate::Eulumdat::from_file(ldt_path).unwrap();
        let room = Room::new(10.0, 8.0, 3.0, 0.80, 0.0);
        let mut layout = find_best_layout(9, 10.0, 8.0, 1.5, room.hrc());
        assert!(!layout.is_moved());
        let grid = layout.grid_positions();
        assert_eq!(layout.positions(), grid);

        let ppb = |l: &LuminaireLayout| compute_ppb_overlay(&ldt, l, &room, 30, 0.8, 0.6, 500.0);
        let before = ppb(&layout);
        let svg_before = crate::zonal::ZonalSvg::room_plan(
            &room,
            &layout,
            &crate::diagram::SvgTheme::light(),
            crate::units::UnitSystem::Metric,
        );
        assert!(svg_before.contains("Sx="));

        // all luminaires pushed into one corner: much less uniform
        layout.moved = (0..9).map(|i| (i, 1.0 + 0.1 * i as f64, 1.0)).collect();
        // an index past the grid is ignored
        layout.moved.push((99, 5.0, 5.0));
        assert!(layout.is_moved());
        let pos = layout.positions();
        assert_eq!(pos.len(), 9);
        assert_eq!(pos[4], (1.4, 1.0));
        let after = ppb(&layout);
        // the reflected part is uniform (zonal average); the direct part
        // piles up under the corner
        assert!(
            after.max_lux > 2.0 * before.max_lux,
            "{} vs {}",
            after.max_lux,
            before.max_lux
        );
        assert!(after.uniformity_min_avg < before.uniformity_min_avg);
        assert!((after.avg_lux - before.avg_lux).abs() < 1e-6);

        let svg = crate::zonal::ZonalSvg::room_plan(
            &room,
            &layout,
            &crate::diagram::SvgTheme::light(),
            crate::units::UnitSystem::Metric,
        );
        assert!(!svg.contains("Sx="), "a moved layout has no grid spacing");
        assert!(svg.contains("(moved)"));
        // the dot of luminaire 4 sits at (1.4 m, 1.0 m)
        let f = crate::zonal::PlanFrame::for_room(&room);
        let (x, y) = (f.ox + 1.4 * f.scale, f.oy + 1.0 * f.scale);
        assert!(svg.contains(&format!(r#"cx="{x}" cy="{y}""#)));
        let (rx, ry) = f.to_room(x, y);
        assert!((rx - 1.4).abs() < 1e-9 && (ry - 1.0).abs() < 1e-9);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn layout_without_moved_field_still_deserializes() {
        let layout = find_best_layout(4, 6.0, 6.0, 1.5, 2.2);
        let mut v = serde_json::to_value(&layout).unwrap();
        v.as_object_mut().unwrap().remove("moved");
        let back: LuminaireLayout = serde_json::from_value(v).unwrap();
        assert_eq!(back, layout);
    }

    #[test]
    fn shaped_room_uses_its_floor() {
        let ldt_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../eulumdat-wasm/templates/fluorescent_luminaire.ldt"
        );
        let ldt = crate::Eulumdat::from_file(ldt_path).unwrap();
        // an L: 10 × 8 m minus the 6 × 4 m corner
        let l_shape = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 4.0),
            (4.0, 4.0),
            (4.0, 8.0),
            (0.0, 8.0),
        ];
        let rect = Room::new(10.0, 8.0, 3.0, 0.8, 0.0);
        let room = rect.clone().with_outline(Some(l_shape.clone()));
        assert!((room.area() - 56.0).abs() < 1e-9);
        assert!((room.perimeter() - 36.0).abs() < 1e-9);
        assert!(room.contains(2.0, 6.0) && !room.contains(7.0, 6.0));
        // CR = 2.5 h P / A; the rectangle keeps 5 h (L + W) / (L W)
        let refl = Reflectances::new(0.7, 0.5, 0.2);
        let c_rect = compute_cavity_ratios(&rect, &refl);
        assert!((c_rect.rcr - cavity_ratio(2.2, 10.0, 8.0)).abs() < 1e-9);
        let c_l = compute_cavity_ratios(&room, &refl);
        assert!((c_l.rcr - 2.5 * 2.2 * 36.0 / 56.0).abs() < 1e-9);
        assert!(c_l.rcr > c_rect.rcr, "an L has more wall per floor area");

        let llf = LightLossFactor::new(0.9, 0.95, 1.0, 0.98);
        let ct = CuTable::calculate(&ldt);
        let r = compute_zonal(
            &ldt,
            &room,
            &refl,
            &llf,
            500.0,
            &ct,
            SolveMode::TargetToCount,
            None,
            None,
        );
        let pos = r.layout.positions();
        assert_eq!(pos.len(), r.layout.count);
        assert!(r.layout.count >= 1);
        for p in &pos {
            assert!(room.contains(p.0, p.1), "{p:?} outside the L");
        }
        // fewer luminaires than for the whole rectangle at the same target
        let rr = compute_zonal(
            &ldt,
            &rect,
            &refl,
            &llf,
            500.0,
            &ct,
            SolveMode::TargetToCount,
            None,
            None,
        );
        assert!(
            r.layout.count < rr.layout.count,
            "{} vs {}",
            r.layout.count,
            rr.layout.count
        );
        assert!(
            r.achieved_illuminance >= 450.0,
            "{}",
            r.achieved_illuminance
        );
        // point by point: only the floor counts
        let ppb = compute_ppb_overlay(
            &ldt,
            &r.layout,
            &room,
            20,
            r.llf_total,
            r.cu,
            r.achieved_illuminance,
        );
        let mask = ppb.mask.as_ref().expect("mask for a shaped room");
        let inside = mask.iter().flatten().filter(|&&m| m).count();
        assert!(inside > 0 && inside < 400);
        // the plan draws the outline and clips to it
        let svg = crate::zonal::ZonalSvg::room_plan(
            &room,
            &r.layout,
            &crate::diagram::SvgTheme::light(),
            crate::units::UnitSystem::Metric,
        );
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("room-floor-clip"));
        // a rectangle as an outline gives the rectangle's numbers
        let as_outline =
            rect.clone()
                .with_outline(Some(vec![(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)]));
        let c_o = compute_cavity_ratios(&as_outline, &refl);
        assert!((c_o.rcr - c_rect.rcr).abs() < 1e-9);
    }
}
