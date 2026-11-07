//! Sensor coverage of a rectangular room: which part of the floor (walking)
//! and of the workplane (seated presence) the placed sensors detect.
//!
//! Plan coordinates as in the interior designer: x along the room length,
//! y along the width, origin in a corner, metres. A sensor's C0 points
//! along +x unless it is rotated. The evaluated area excludes a wall margin
//! (EN 12464-1 uses 0.5 m as the border zone of a room).

use super::{DetectionArea, Footprint, Motion, SensorFile, TRACE_COLORS};
use core::fmt::Write;

/// Default wall margin excluded from the evaluation (m).
pub const DEFAULT_WALL_MARGIN: f64 = 0.5;
/// Upper bound for the number of evaluation cells (keeps big rooms fast).
const MAX_CELLS: f64 = 40_000.0;

/// A rectangular room for the coverage check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageRoom {
    /// Along x (m).
    pub length: f64,
    /// Along y (m).
    pub width: f64,
    /// Height of the sensors above the floor (m).
    pub mount_height: f64,
    /// Plane for walking (tangential / radial), usually the floor (m).
    pub walk_plane: f64,
    /// Plane for seated presence, usually the workplane (m).
    pub presence_plane: f64,
    /// Border excluded from the evaluation (m).
    pub margin: f64,
}

impl CoverageRoom {
    /// Detection plane of a motion type.
    pub fn plane(&self, motion: Motion) -> f64 {
        match motion {
            Motion::Presence => self.presence_plane,
            Motion::Tangential | Motion::Radial => self.walk_plane,
        }
    }

    /// Area of the evaluated rectangle (m²).
    pub fn evaluated_area(&self) -> f64 {
        (self.length - 2.0 * self.margin).max(0.0) * (self.width - 2.0 * self.margin).max(0.0)
    }
}

/// Where a sensor sits in the room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorPosition {
    pub x: f64,
    pub y: f64,
    /// Rotation of C0 from +x (degrees), for asymmetric sensors.
    pub rotation_deg: f64,
}

/// `nx` × `ny` sensors in the centres of equal cells (like a luminaire grid).
pub fn grid_positions(length: f64, width: f64, nx: usize, ny: usize) -> Vec<SensorPosition> {
    let (nx, ny) = (nx.max(1), ny.max(1));
    let mut v = Vec::with_capacity(nx * ny);
    for i in 0..nx {
        for j in 0..ny {
            v.push(SensorPosition {
                x: (i as f64 + 0.5) * length / nx as f64,
                y: (j as f64 + 0.5) * width / ny as f64,
                rotation_deg: 0.0,
            });
        }
    }
    v
}

/// The grid with the most sensors up to `count` whose cells are closest to
/// square (fallback when no grid reaches the target).
pub fn square_grid(length: f64, width: f64, count: usize) -> (usize, usize) {
    let count = count.max(1);
    let squareness = |nx: usize, ny: usize| {
        ((length / nx as f64) / (width / ny as f64).max(1e-9))
            .ln()
            .abs()
    };
    let mut best = (1, 1);
    for nx in 1..=count {
        let ny = count / nx;
        if ny == 0 {
            break;
        }
        let (bn, cn) = (best.0 * best.1, nx * ny);
        if cn > bn || (cn == bn && squareness(nx, ny) < squareness(best.0, best.1)) {
            best = (nx, ny);
        }
    }
    best
}

/// The evaluation grid: `nx` × `ny` cells over the whole room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageGrid {
    pub nx: usize,
    pub ny: usize,
    /// Cell size along x and y (m).
    pub dx: f64,
    pub dy: f64,
}

impl CoverageGrid {
    fn for_room(room: &CoverageRoom) -> Self {
        let area = (room.length * room.width).max(1e-6);
        let cell = (area / MAX_CELLS).sqrt().max(0.1);
        let nx = ((room.length / cell).round() as usize).max(1);
        let ny = ((room.width / cell).round() as usize).max(1);
        Self {
            nx,
            ny,
            dx: room.length / nx as f64,
            dy: room.width / ny as f64,
        }
    }

    /// Centre of cell (i, j).
    pub fn centre(&self, i: usize, j: usize) -> (f64, f64) {
        ((i as f64 + 0.5) * self.dx, (j as f64 + 0.5) * self.dy)
    }
}

/// Coverage of one motion type.
#[derive(Debug, Clone)]
pub struct MotionCoverage {
    pub motion: Motion,
    /// Detection plane above the floor (m).
    pub plane_height: f64,
    /// Footprint of one sensor on that plane (`None`: no data or nothing
    /// detected at that height).
    pub area: Option<DetectionArea>,
    /// Share of the evaluated area seen by at least one sensor.
    pub covered_fraction: f64,
    /// Share of the evaluated area seen by two or more sensors.
    pub overlap_fraction: f64,
    /// Sensors seeing each cell, row-major over x (`i + j * nx`).
    pub counts: Vec<u8>,
}

/// Coverage of a sensor layout.
#[derive(Debug, Clone)]
pub struct Coverage {
    pub room: CoverageRoom,
    pub positions: Vec<SensorPosition>,
    pub grid: CoverageGrid,
    pub motions: Vec<MotionCoverage>,
    /// Evaluated cells, row-major over x (`i + j * nx`): inside the wall
    /// margin and inside the area's outline ([`coverage_in`]).
    pub mask: Vec<bool>,
}

impl Coverage {
    /// Coverage of one motion type.
    pub fn get(&self, motion: Motion) -> Option<&MotionCoverage> {
        self.motions.iter().find(|m| m.motion == motion)
    }

    /// Room floor area per sensor (m²).
    pub fn m2_per_sensor(&self) -> f64 {
        self.room.length * self.room.width / self.positions.len().max(1) as f64
    }

    /// Cell (i, j) lies in the evaluated area (inside the wall margin and
    /// the area's outline).
    pub fn evaluated(&self, i: usize, j: usize) -> bool {
        self.mask
            .get(i + j * self.grid.nx)
            .copied()
            .unwrap_or(false)
    }
}

/// Evaluated cells of `grid`: inside the wall margin and accepted by
/// `inside` (the area's outline; always true for a rectangular room).
fn evaluation_mask(
    room: &CoverageRoom,
    grid: &CoverageGrid,
    inside: &dyn Fn(f64, f64) -> bool,
) -> Vec<bool> {
    let m = room.margin;
    let mut mask = Vec::with_capacity(grid.nx * grid.ny);
    for j in 0..grid.ny {
        for i in 0..grid.nx {
            let (x, y) = grid.centre(i, j);
            mask.push(
                x >= m && x <= room.length - m && y >= m && y <= room.width - m && inside(x, y),
            );
        }
    }
    mask
}

/// Whether a sensor at `s` with footprint `fp` sees the point (px, py).
fn sees(fp: &Footprint, s: &SensorPosition, px: f64, py: f64) -> bool {
    let (dx, dy) = (px - s.x, py - s.y);
    if dx.abs() > fp.max_range || dy.abs() > fp.max_range {
        return false;
    }
    // into the sensor's own frame (C0 turned by the rotation)
    let r = -s.rotation_deg.to_radians();
    let (c, sn) = (r.cos(), r.sin());
    fp.contains(dx * c - dy * sn, dx * sn + dy * c)
}

/// Detection areas of a sensor on the planes of a room, one per motion
/// type of the file. They depend on the mounting height, the planes and the
/// planning flag only, so a planner can keep them while the room size, the
/// wall margin or the layout changes (cutting a sensor LDT is the slow part).
#[derive(Debug, Clone)]
pub struct PlaneAreas {
    /// Mounting height the areas were cut for (m).
    pub mount_height: f64,
    /// (motion, plane height, area; `None` = nothing detected there).
    pub areas: Vec<(Motion, f64, Option<DetectionArea>)>,
}

impl PlaneAreas {
    /// Areas of every motion type the file has.
    pub fn new(file: &SensorFile, room: &CoverageRoom, planning: bool) -> Self {
        Self::for_motions(file, room, planning, &file.motions())
    }

    /// Areas of the given motion types only.
    pub fn for_motions(
        file: &SensorFile,
        room: &CoverageRoom,
        planning: bool,
        motions: &[Motion],
    ) -> Self {
        let areas = motions
            .iter()
            .map(|&m| {
                let plane = room.plane(m);
                let area = file
                    .area_at(m, room.mount_height, plane, planning)
                    .filter(|a| a.detected());
                (m, plane, area)
            })
            .collect();
        Self {
            mount_height: room.mount_height,
            areas,
        }
    }

    /// Motion types (with or without a detected area).
    pub fn motions(&self) -> Vec<Motion> {
        self.areas.iter().map(|a| a.0).collect()
    }

    /// Detected area of one motion type.
    pub fn get(&self, motion: Motion) -> Option<&DetectionArea> {
        self.areas
            .iter()
            .find(|a| a.0 == motion)
            .and_then(|a| a.2.as_ref())
    }
}

/// Sensors seeing each cell: every sensor is rasterised over the cells of
/// its own bounding box only.
fn count_cells(fp: &Footprint, positions: &[SensorPosition], grid: &CoverageGrid) -> Vec<u8> {
    let mut counts = vec![0u8; grid.nx * grid.ny];
    let r = fp.max_range;
    let span = |lo: f64, hi: f64, step: f64, n: usize| {
        let a = ((lo / step).floor().max(0.0) as usize).min(n);
        let b = ((hi / step).ceil().max(0.0) as usize).min(n);
        a..b
    };
    for s in positions {
        for j in span(s.y - r, s.y + r, grid.dy, grid.ny) {
            for i in span(s.x - r, s.x + r, grid.dx, grid.nx) {
                let (x, y) = grid.centre(i, j);
                if sees(fp, s, x, y) {
                    let c = &mut counts[i + j * grid.nx];
                    *c = c.saturating_add(1);
                }
            }
        }
    }
    counts
}

fn motion_coverage(
    area: Option<&DetectionArea>,
    motion: Motion,
    plane: f64,
    positions: &[SensorPosition],
    grid: &CoverageGrid,
    mask: &[bool],
) -> MotionCoverage {
    let counts = match area {
        Some(a) => count_cells(&a.footprint, positions, grid),
        None => vec![0u8; grid.nx * grid.ny],
    };
    let (mut evaluated, mut covered, mut overlap) = (0usize, 0usize, 0usize);
    for (n, _) in counts.iter().zip(mask).filter(|(_, &m)| m) {
        evaluated += 1;
        if *n >= 1 {
            covered += 1;
        }
        if *n >= 2 {
            overlap += 1;
        }
    }
    let ev = evaluated.max(1) as f64;
    MotionCoverage {
        motion,
        plane_height: plane,
        area: area.cloned(),
        covered_fraction: covered as f64 / ev,
        overlap_fraction: overlap as f64 / ev,
        counts,
    }
}

/// Coverage of all motion types the file has, for sensors at `positions`.
pub fn coverage(
    file: &SensorFile,
    room: &CoverageRoom,
    positions: &[SensorPosition],
    planning: bool,
) -> Coverage {
    coverage_with(&PlaneAreas::new(file, room, planning), room, positions)
}

/// [`coverage`] with areas cut beforehand ([`PlaneAreas`]); `room`
/// supplies the size and the wall margin.
pub fn coverage_with(
    areas: &PlaneAreas,
    room: &CoverageRoom,
    positions: &[SensorPosition],
) -> Coverage {
    coverage_in(areas, room, positions, &|_, _| true)
}

/// [`coverage_with`] for an area that is not a rectangle (an outdoor area
/// drawn as a polygon): `room` is its bounding rectangle from (0, 0) and
/// only points `inside` accepts are evaluated.
pub fn coverage_in(
    areas: &PlaneAreas,
    room: &CoverageRoom,
    positions: &[SensorPosition],
    inside: &dyn Fn(f64, f64) -> bool,
) -> Coverage {
    let grid = CoverageGrid::for_room(room);
    let mask = evaluation_mask(room, &grid, inside);
    let motions = areas
        .areas
        .iter()
        .map(|(m, plane, a)| motion_coverage(a.as_ref(), *m, *plane, positions, &grid, &mask))
        .collect();
    Coverage {
        room: *room,
        positions: positions.to_vec(),
        grid,
        motions,
        mask,
    }
}

/// Fewest sensors on a regular grid (`nx` × `ny`, at most `max_count`) that
/// see at least `target` (0–1) of the evaluated area for `motion`. Among
/// layouts with the same count the one with the most square cells wins.
/// `None` if no grid up to `max_count` reaches the target.
pub fn auto_grid(
    file: &SensorFile,
    room: &CoverageRoom,
    planning: bool,
    motion: Motion,
    target: f64,
    max_count: usize,
) -> Option<(usize, usize)> {
    let areas = PlaneAreas::for_motions(file, room, planning, &[motion]);
    auto_grid_with(&areas, room, motion, target, max_count)
}

/// [`auto_grid`] with areas cut beforehand.
pub fn auto_grid_with(
    areas: &PlaneAreas,
    room: &CoverageRoom,
    motion: Motion,
    target: f64,
    max_count: usize,
) -> Option<(usize, usize)> {
    let grid = CoverageGrid::for_room(room);
    let mask = evaluation_mask(room, &grid, &|_, _| true);
    search_grid(areas, room, motion, target, max_count, 0.0, &mask)
}

fn search_grid(
    areas: &PlaneAreas,
    room: &CoverageRoom,
    motion: Motion,
    target: f64,
    max_count: usize,
    rotation_deg: f64,
    mask: &[bool],
) -> Option<(usize, usize)> {
    let one = areas.get(motion)?;
    let plane = room.plane(motion);
    let grid = CoverageGrid::for_room(room);
    // Lower bound: the evaluated area over one footprint.
    let evaluated_m2 = mask.iter().filter(|&&m| m).count() as f64 * grid.dx * grid.dy;
    let lower = ((evaluated_m2 / one.area_m2().max(1e-6)).floor() as usize).max(1);
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    for nx in 1..=max_count {
        for ny in 1..=max_count / nx {
            if nx * ny >= lower {
                candidates.push((nx, ny));
            }
        }
    }
    let squareness = |(nx, ny): (usize, usize)| {
        let cx = room.length / nx as f64;
        let cy = room.width / ny as f64;
        (cx / cy).ln().abs()
    };
    candidates.sort_by(|a, b| {
        (a.0 * a.1)
            .cmp(&(b.0 * b.1))
            .then(squareness(*a).total_cmp(&squareness(*b)))
    });
    candidates.into_iter().find(|&(nx, ny)| {
        let pos = rotated(
            grid_positions(room.length, room.width, nx, ny),
            rotation_deg,
        );
        motion_coverage(Some(one), motion, plane, &pos, &grid, mask).covered_fraction >= target
    })
}

/// The same positions with every sensor turned by `deg` (C0 from +x
/// towards +y).
pub fn rotated(mut positions: Vec<SensorPosition>, deg: f64) -> Vec<SensorPosition> {
    for p in &mut positions {
        p.rotation_deg = deg;
    }
    positions
}

/// [`auto_grid_with`] trying each sensor rotation in `rotations` (degrees):
/// the layout with the fewest sensors wins, the earlier rotation on a tie.
/// Returns (`nx`, `ny`, rotation). Elongated footprints (corridor sensors)
/// need far fewer sensors when their long axis runs along the room.
pub fn auto_grid_oriented(
    areas: &PlaneAreas,
    room: &CoverageRoom,
    motion: Motion,
    target: f64,
    max_count: usize,
    rotations: &[f64],
) -> Option<(usize, usize, f64)> {
    auto_grid_oriented_in(
        areas,
        room,
        &|_, _| true,
        motion,
        target,
        max_count,
        rotations,
    )
}

/// [`auto_grid_oriented`] for an area that is not a rectangle (see
/// [`coverage_in`]): only points `inside` accepts count.
pub fn auto_grid_oriented_in(
    areas: &PlaneAreas,
    room: &CoverageRoom,
    inside: &dyn Fn(f64, f64) -> bool,
    motion: Motion,
    target: f64,
    max_count: usize,
    rotations: &[f64],
) -> Option<(usize, usize, f64)> {
    let grid = CoverageGrid::for_room(room);
    let mask = evaluation_mask(room, &grid, inside);
    let mut best: Option<(usize, usize, f64)> = None;
    for &rot in rotations {
        let limit = best.map(|b| b.0 * b.1 - 1).unwrap_or(max_count);
        if limit == 0 {
            break;
        }
        if let Some((nx, ny)) = search_grid(areas, room, motion, target, limit, rot, &mask) {
            best = Some((nx, ny, rot));
        }
    }
    best
}

/// SVG elements for a plan whose room corner sits at (`ox`, `oy`) pixels
/// with `scale` pixels per metre (as `zonal::ZonalSvg::room_plan_with_overlay`
/// hands them over): footprints of the `shown` motion types, the cells of
/// the evaluated area that `gaps_for` does not cover (hatched red), the
/// evaluation border and the sensor marks.
pub fn overlay_svg(
    cov: &Coverage,
    ox: f64,
    oy: f64,
    scale: f64,
    shown: &[Motion],
    gaps_for: Option<Motion>,
) -> String {
    overlay_svg_xy(cov, ox, oy, scale, scale, shown, gaps_for, true)
}

/// [`overlay_svg`] for a plan drawn with different scales along x and y
/// (`sx`, `sy` pixels per metre, e.g. the outdoor area plan, which fills
/// its frame). `clip`: footprints end at the room outline (walls); off for
/// open ground.
#[allow(clippy::too_many_arguments)]
pub fn overlay_svg_xy(
    cov: &Coverage,
    ox: f64,
    oy: f64,
    sx: f64,
    sy: f64,
    shown: &[Motion],
    gaps_for: Option<Motion>,
    clip: bool,
) -> String {
    let mut s = String::new();
    let px = |x: f64| ox + x * sx;
    let py = |y: f64| oy + y * sy;
    if clip {
        // Walls stop detection: footprints are clipped to the room.
        let _ = write!(
            s,
            r#"<defs><clipPath id="sensor-cov-clip"><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/></clipPath></defs><g clip-path="url(#sensor-cov-clip)">"#,
            ox,
            oy,
            cov.room.length * sx,
            cov.room.width * sy
        );
    } else {
        s.push_str("<g>");
    }
    // Uncovered cells, merged into runs per row.
    if let Some(mc) = gaps_for.and_then(|m| cov.get(m)) {
        let g = &cov.grid;
        s.push_str(r##"<g fill="#ef4444" fill-opacity="0.35">"##);
        for j in 0..g.ny {
            let mut i = 0;
            while i < g.nx {
                if cov.evaluated(i, j) && mc.counts[i + j * g.nx] == 0 {
                    let start = i;
                    while i < g.nx && cov.evaluated(i, j) && mc.counts[i + j * g.nx] == 0 {
                        i += 1;
                    }
                    let _ = write!(
                        s,
                        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/>"#,
                        px(start as f64 * g.dx),
                        py(j as f64 * g.dy),
                        (i - start) as f64 * g.dx * sx,
                        g.dy * sy
                    );
                } else {
                    i += 1;
                }
            }
        }
        s.push_str("</g>");
    }
    // Footprints, largest first so the smaller ones stay visible.
    let mut motions: Vec<&MotionCoverage> = cov
        .motions
        .iter()
        .filter(|m| shown.contains(&m.motion) && m.area.is_some())
        .collect();
    motions.sort_by(|a, b| {
        let area = |m: &MotionCoverage| m.area.as_ref().map(|a| a.area_m2()).unwrap_or(0.0);
        area(b).total_cmp(&area(a))
    });
    for mc in motions {
        let color = TRACE_COLORS[Motion::ALL
            .iter()
            .position(|m| *m == mc.motion)
            .unwrap_or(0)];
        let fp = &mc.area.as_ref().unwrap().footprint;
        // The planned motion type (the one with gaps) is filled, the others
        // are thin dashed outlines.
        let style = if gaps_for == Some(mc.motion) {
            r#"fill-opacity="0.12" stroke-width="1.5" stroke-opacity="0.9""#
        } else {
            r#"fill-opacity="0" stroke-width="1" stroke-opacity="0.7" stroke-dasharray="5,3""#
        };
        let _ = write!(s, r#"<g fill="{color}" stroke="{color}" {style}>"#);
        for p in &cov.positions {
            let r = p.rotation_deg.to_radians();
            let pts: Vec<String> = fp
                .outline
                .iter()
                .map(|&(x, y)| {
                    let (xr, yr) = (x * r.cos() - y * r.sin(), x * r.sin() + y * r.cos());
                    format!("{:.1},{:.1}", px(p.x + xr), py(p.y + yr))
                })
                .collect();
            let _ = write!(s, r#"<polygon points="{}"/>"#, pts.join(" "));
        }
        s.push_str("</g>");
    }
    s.push_str("</g>");
    // Evaluated area (inside the wall margin).
    let m = cov.room.margin;
    if m > 0.0 {
        let _ = write!(
            s,
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="#64748b" stroke-width="1" stroke-dasharray="4,3"/>"##,
            px(m),
            py(m),
            (cov.room.length - 2.0 * m).max(0.0) * sx,
            (cov.room.width - 2.0 * m).max(0.0) * sy
        );
    }
    // Sensors.
    for p in &cov.positions {
        let (x, y) = (px(p.x), py(p.y));
        let _ = write!(
            s,
            r##"<g><circle cx="{x:.1}" cy="{y:.1}" r="6" fill="#ffffff" stroke="#7c3aed" stroke-width="2"/><circle cx="{x:.1}" cy="{y:.1}" r="2" fill="#7c3aed"/></g>"##
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ceiling() -> SensorFile {
        let ldt = include_str!("../../tests/fixtures/sensors/demo_ceiling_360.ldt");
        SensorFile::from_bytes(ldt.as_bytes(), "sensor/sensldt").unwrap()
    }

    fn office() -> CoverageRoom {
        CoverageRoom {
            length: 10.0,
            width: 8.0,
            mount_height: 3.0,
            walk_plane: 0.0,
            presence_plane: 0.8,
            margin: DEFAULT_WALL_MARGIN,
        }
    }

    #[test]
    fn grid_positions_are_cell_centres() {
        let p = grid_positions(10.0, 8.0, 2, 2);
        assert_eq!(p.len(), 4);
        assert_eq!((p[0].x, p[0].y), (2.5, 2.0));
        assert_eq!((p[3].x, p[3].y), (7.5, 6.0));
    }

    #[test]
    fn one_sensor_covers_a_circle() {
        let f = ceiling();
        let room = office();
        let cov = coverage(&f, &room, &grid_positions(10.0, 8.0, 1, 1), false);
        let pres = cov.get(Motion::Presence).unwrap();
        // Presence footprint at 3.0 − 0.8 = 2.2 m below the sensor.
        let a = pres.area.as_ref().unwrap().area_m2();
        let expect = a / room.evaluated_area();
        assert!(
            (pres.covered_fraction - expect).abs() < 0.03,
            "{} vs {expect}",
            pres.covered_fraction
        );
        assert_eq!(pres.overlap_fraction, 0.0);
        // Walking at floor level is cut deeper (3.0 m) and is larger.
        let tang = cov.get(Motion::Tangential).unwrap();
        assert!(tang.covered_fraction > pres.covered_fraction);
    }

    #[test]
    fn auto_grid_reaches_full_presence_coverage() {
        let f = ceiling();
        let room = office();
        let (nx, ny) = auto_grid(&f, &room, false, Motion::Presence, 1.0, 400).unwrap();
        let cov = coverage(&f, &room, &grid_positions(10.0, 8.0, nx, ny), false);
        assert!(cov.get(Motion::Presence).unwrap().covered_fraction >= 1.0);
        // One sensor fewer in either direction must not be enough.
        for (a, b) in [(nx.saturating_sub(1), ny), (nx, ny.saturating_sub(1))] {
            if a * b == 0 {
                continue;
            }
            let c = coverage(&f, &room, &grid_positions(10.0, 8.0, a, b), false);
            assert!(
                c.get(Motion::Presence).unwrap().covered_fraction < 1.0,
                "{a}x{b}"
            );
        }
    }

    #[test]
    fn too_low_mounting_detects_nothing() {
        let f = ceiling();
        let room = CoverageRoom {
            mount_height: 0.5,
            ..office()
        };
        let cov = coverage(&f, &room, &grid_positions(10.0, 8.0, 2, 2), false);
        assert_eq!(cov.get(Motion::Presence).unwrap().covered_fraction, 0.0);
        assert!(auto_grid(&f, &room, false, Motion::Presence, 1.0, 100).is_none());
    }

    #[test]
    fn overlay_has_sensors_footprints_and_gaps() {
        let f = ceiling();
        let cov = coverage(&f, &office(), &grid_positions(10.0, 8.0, 1, 1), false);
        let svg = overlay_svg(&cov, 50.0, 50.0, 40.0, &Motion::ALL, Some(Motion::Presence));
        assert!(svg.contains("<polygon") && svg.contains("#ef4444") && svg.contains("<circle"));
        // Footprints are clipped to the room outline.
        assert!(svg.contains(r#"clip-path="url(#sensor-cov-clip)""#));
        assert!(svg.contains(r#"width="400.00" height="320.00""#));
    }

    #[test]
    fn rasterised_counts_match_brute_force() {
        let f = ceiling();
        let room = office();
        let areas = PlaneAreas::new(&f, &room, false);
        let fp = &areas.get(Motion::Presence).unwrap().footprint;
        let mut pos = grid_positions(10.0, 8.0, 3, 2);
        pos.push(SensorPosition {
            x: 0.2,
            y: 7.9,
            rotation_deg: 30.0,
        });
        let grid = CoverageGrid::for_room(&room);
        let fast = count_cells(fp, &pos, &grid);
        for j in 0..grid.ny {
            for i in 0..grid.nx {
                let (x, y) = grid.centre(i, j);
                let n = pos.iter().filter(|s| sees(fp, s, x, y)).count() as u8;
                assert_eq!(fast[i + j * grid.nx], n, "cell {i},{j}");
            }
        }
        // Precut areas give the same coverage as the one-call version.
        let a = coverage(&f, &room, &pos, false);
        let b = coverage_with(&areas, &room, &pos);
        for (x, y) in a.motions.iter().zip(&b.motions) {
            assert_eq!(x.counts, y.counts);
        }
    }

    #[test]
    fn corridor_sensor_is_turned_along_the_corridor() {
        let ldt = include_str!("../../tests/fixtures/sensors/demo_corridor.ldt");
        let f = SensorFile::from_bytes(ldt.as_bytes(), "sensor/sensldt").unwrap();
        let room = CoverageRoom {
            length: 30.0,
            width: 2.0,
            mount_height: 2.8,
            walk_plane: 0.0,
            presence_plane: 0.8,
            margin: DEFAULT_WALL_MARGIN,
        };
        let areas = PlaneAreas::new(&f, &room, false);
        let fixed = auto_grid_with(&areas, &room, Motion::Tangential, 0.95, 100).unwrap();
        let (nx, ny, rot) =
            auto_grid_oriented(&areas, &room, Motion::Tangential, 0.95, 100, &[0.0, 90.0]).unwrap();
        assert!(nx * ny < fixed.0 * fixed.1, "{nx}x{ny}@{rot} vs {fixed:?}");
        let pos = rotated(grid_positions(30.0, 2.0, nx, ny), rot);
        let cov = coverage_with(&areas, &room, &pos);
        assert!(cov.get(Motion::Tangential).unwrap().covered_fraction >= 0.95);
        // A round footprint keeps the first rotation.
        let c = ceiling();
        let r = office();
        let a = PlaneAreas::new(&c, &r, false);
        let (_, _, rot) =
            auto_grid_oriented(&a, &r, Motion::Presence, 0.95, 100, &[0.0, 90.0]).unwrap();
        assert_eq!(rot, 0.0);
    }

    #[test]
    fn square_grid_uses_the_count_with_square_cells() {
        assert_eq!(square_grid(10.0, 10.0, 100), (10, 10));
        assert_eq!(square_grid(20.0, 5.0, 100), (20, 5));
        assert_eq!(square_grid(10.0, 8.0, 1), (1, 1));
        let (nx, ny) = square_grid(30.0, 2.0, 12);
        assert_eq!((nx, ny), (12, 1));
    }

    #[test]
    fn coverage_in_counts_only_the_outline() {
        let f = ceiling();
        let room = CoverageRoom {
            margin: 0.0,
            ..office()
        };
        let areas = PlaneAreas::new(&f, &room, false);
        let pos = grid_positions(10.0, 8.0, 1, 1);
        let full = coverage_with(&areas, &room, &pos);
        // the left half only: the centred sensor sees the same share of it
        // (symmetric footprint)
        let half = coverage_in(&areas, &room, &pos, &|x, _| x < 5.0);
        let m = Motion::Presence;
        let (a, b) = (
            full.get(m).unwrap().covered_fraction,
            half.get(m).unwrap().covered_fraction,
        );
        assert!((b - a).abs() < 0.05, "symmetric footprint: {a} vs {b}");
        assert!(half.mask.iter().filter(|&&v| v).count() * 2 <= full.mask.len() + full.grid.ny);
        assert!(!half.evaluated(full.grid.nx - 1, 0));
        // a sensor in the left half covers more of the left half
        let left = grid_positions(5.0, 8.0, 1, 1);
        let cov_left = coverage_in(&areas, &room, &left, &|x, _| x < 5.0);
        let l = cov_left.get(m).unwrap().covered_fraction;
        assert!(
            (l - 2.0 * a).abs() < 0.02,
            "half the area, the same footprint: {l} vs {a}"
        );
        // automatic placement over the half needs no more sensors
        let all = auto_grid_oriented(&areas, &room, m, 0.95, 60, &[0.0]).unwrap();
        let some =
            auto_grid_oriented_in(&areas, &room, &|x, _| x < 5.0, m, 0.95, 60, &[0.0]).unwrap();
        assert!(some.0 * some.1 <= all.0 * all.1, "{some:?} vs {all:?}");
    }

    #[test]
    fn overlay_with_two_scales() {
        let f = ceiling();
        let room = office();
        let cov = coverage(&f, &room, &grid_positions(10.0, 8.0, 1, 1), false);
        let one = overlay_svg(&cov, 10.0, 20.0, 30.0, &[Motion::Tangential], None);
        let same = overlay_svg_xy(
            &cov,
            10.0,
            20.0,
            30.0,
            30.0,
            &[Motion::Tangential],
            None,
            true,
        );
        assert_eq!(one, same);
        let open = overlay_svg_xy(
            &cov,
            10.0,
            20.0,
            30.0,
            15.0,
            &[Motion::Tangential],
            None,
            false,
        );
        assert!(!open.contains("clipPath"));
        // the sensor mark: x scaled by 30, y by 15
        assert!(open.contains(r#"cx="160.0" cy="80.0""#), "{open}");
    }
}
