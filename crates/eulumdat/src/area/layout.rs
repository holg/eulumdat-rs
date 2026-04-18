//! Layout presets — generate pole positions and luminaire placements.

use super::compute::LuminairePlace;

/// Pole arrangement type — how many luminaires per pole and their orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ArrangementType {
    /// One luminaire per pole
    Single,
    /// Two luminaires, opposite directions (180°)
    BackToBack,
    /// Two luminaires, same side, slight lateral offset
    TwinArm,
    /// Four luminaires at 90° each
    Quad,
    /// Mounted on vertical surface, not pole
    WallMounted,
}

impl ArrangementType {
    /// Number of luminaires generated per pole position.
    pub fn luminaires_per_pole(&self) -> usize {
        match self {
            Self::Single | Self::WallMounted => 1,
            Self::BackToBack | Self::TwinArm => 2,
            Self::Quad => 4,
        }
    }

    /// All arrangement types for UI display.
    pub fn all() -> &'static [ArrangementType] {
        &[
            Self::Single,
            Self::BackToBack,
            Self::TwinArm,
            Self::Quad,
            Self::WallMounted,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::BackToBack => "Back-to-Back",
            Self::TwinArm => "Twin Arm",
            Self::Quad => "Quad",
            Self::WallMounted => "Wall Mounted",
        }
    }
}

/// Physical pole configuration.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PoleConfig {
    pub arrangement: ArrangementType,
    /// Overhang from pole center in meters
    pub arm_length: f64,
    /// Arm droop angle from horizontal in degrees (0 = level, + = drooping)
    pub arm_droop: f64,
}

impl Default for PoleConfig {
    fn default() -> Self {
        Self {
            arrangement: ArrangementType::Single,
            arm_length: 0.0,
            arm_droop: 0.0,
        }
    }
}

/// Grid preset — defines how many poles and their pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GridPreset {
    /// Single pole at center
    Single,
    /// Two poles in a row
    Pair,
    /// Three poles in a row
    Row3,
    /// 2×2 grid
    Grid2x2,
    /// 2×3 grid
    Grid2x3,
    /// 3×3 grid
    Grid3x3,
    /// Perimeter placement (poles distributed around edges)
    Perimeter(usize),
}

impl GridPreset {
    /// For grid presets, returns (rows, cols). Perimeter returns (0, 0).
    pub fn rows_cols(&self) -> (usize, usize) {
        match self {
            Self::Single => (1, 1),
            Self::Pair => (1, 2),
            Self::Row3 => (1, 3),
            Self::Grid2x2 => (2, 2),
            Self::Grid2x3 => (2, 3),
            Self::Grid3x3 => (3, 3),
            Self::Perimeter(_) => (0, 0),
        }
    }

    /// Whether this preset uses perimeter placement.
    pub fn is_perimeter(&self) -> bool {
        matches!(self, Self::Perimeter(_))
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Single => "Single",
            Self::Pair => "Pair",
            Self::Row3 => "Row of 3",
            Self::Grid2x2 => "2×2 Grid",
            Self::Grid2x3 => "2×3 Grid",
            Self::Grid3x3 => "3×3 Grid",
            Self::Perimeter(_) => "Perimeter",
        }
    }

    pub fn all() -> &'static [GridPreset] {
        &[
            Self::Single,
            Self::Pair,
            Self::Row3,
            Self::Grid2x2,
            Self::Grid2x3,
            Self::Grid3x3,
            Self::Perimeter(8),
        ]
    }
}

/// Generate pole positions evenly distributed in a rectangle.
///
/// Returns positions as `(x, y)` pairs within `(0..area_width, 0..area_depth)`.
/// Poles are inset from edges by half a spacing step.
pub fn generate_pole_positions(
    rows: usize,
    cols: usize,
    area_width: f64,
    area_depth: f64,
) -> Vec<(f64, f64)> {
    let mut positions = Vec::with_capacity(rows * cols);
    let sx = area_width / cols as f64;
    let sy = area_depth / rows as f64;

    for r in 0..rows {
        for c in 0..cols {
            let x = (c as f64 + 0.5) * sx;
            let y = (r as f64 + 0.5) * sy;
            positions.push((x, y));
        }
    }
    positions
}

/// Generate pole positions in a grid within a polygon boundary.
///
/// Uses the polygon's bounding box for the grid, then filters out positions
/// that fall outside the polygon. Over-generates by increasing the grid density
/// to try to place at least `rows * cols` poles inside.
pub fn generate_pole_positions_in_polygon(
    rows: usize,
    cols: usize,
    polygon: &super::polygon::AreaPolygon,
) -> Vec<(f64, f64)> {
    let (x0, y0, x1, y1) = polygon.bounding_box();
    let w = x1 - x0;
    let h = y1 - y0;

    // Generate on the bounding box grid, then filter
    let sx = w / cols as f64;
    let sy = h / rows as f64;

    let mut positions = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let x = x0 + (c as f64 + 0.5) * sx;
            let y = y0 + (r as f64 + 0.5) * sy;
            if polygon.contains(x, y) {
                positions.push((x, y));
            }
        }
    }

    // If too few poles ended up inside, try a denser grid
    if positions.len() < rows * cols && rows * cols > 1 {
        let target = rows * cols;
        let scale = 2;
        let dense_rows = rows * scale;
        let dense_cols = cols * scale;
        let dsx = w / dense_cols as f64;
        let dsy = h / dense_rows as f64;

        let mut dense = Vec::new();
        for r in 0..dense_rows {
            for c in 0..dense_cols {
                let x = x0 + (c as f64 + 0.5) * dsx;
                let y = y0 + (r as f64 + 0.5) * dsy;
                if polygon.contains(x, y) {
                    dense.push((x, y));
                }
            }
        }

        // Sub-sample evenly to get target count
        if dense.len() >= target {
            positions.clear();
            let step = dense.len() as f64 / target as f64;
            for i in 0..target {
                positions.push(dense[(i as f64 * step) as usize]);
            }
        } else {
            positions = dense;
        }
    }

    positions
}

/// Generate pole positions distributed evenly around the perimeter of a rectangle.
///
/// Poles are inset from edges by a margin (5% of the shorter dimension).
/// They are distributed as evenly as possible, with corners always occupied first.
pub fn generate_perimeter_positions(
    count: usize,
    area_width: f64,
    area_depth: f64,
) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![(area_width / 2.0, area_depth / 2.0)];
    }

    let margin = area_width.min(area_depth) * 0.05;
    let x0 = margin;
    let y0 = margin;
    let x1 = area_width - margin;
    let y1 = area_depth - margin;

    // Walk the perimeter clockwise: top-left → top-right → bottom-right → bottom-left
    let perimeter = 2.0 * ((x1 - x0) + (y1 - y0));
    let step = perimeter / count as f64;

    let mut positions = Vec::with_capacity(count);
    let w = x1 - x0;
    let h = y1 - y0;

    for i in 0..count {
        let d = i as f64 * step;
        let (x, y) = if d < w {
            // Top edge (left to right)
            (x0 + d, y0)
        } else if d < w + h {
            // Right edge (top to bottom)
            (x1, y0 + (d - w))
        } else if d < 2.0 * w + h {
            // Bottom edge (right to left)
            (x1 - (d - w - h), y1)
        } else {
            // Left edge (bottom to top)
            (x0, y1 - (d - 2.0 * w - h))
        };
        positions.push((x, y));
    }

    positions
}

/// Generate pole positions distributed along the edges of a polygon.
///
/// Walks the polygon perimeter and distributes `count` poles evenly.
pub fn generate_perimeter_positions_polygon(
    count: usize,
    polygon: &super::polygon::AreaPolygon,
) -> Vec<(f64, f64)> {
    let verts = &polygon.vertices;
    let n = verts.len();
    if count == 0 || n < 3 {
        return Vec::new();
    }

    // Compute edge lengths and total perimeter
    let mut edge_lengths = Vec::with_capacity(n);
    let mut total = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = verts[j].0 - verts[i].0;
        let dy = verts[j].1 - verts[i].1;
        let len = (dx * dx + dy * dy).sqrt();
        edge_lengths.push(len);
        total += len;
    }

    let step = total / count as f64;
    // Inset slightly from vertices — start at half a step
    let start_offset = step * 0.5;
    let mut positions = Vec::with_capacity(count);

    for i in 0..count {
        let target_dist = start_offset + i as f64 * step;
        let d = target_dist % total; // wrap around

        let mut accumulated = 0.0;
        for edge in 0..n {
            let next_acc = accumulated + edge_lengths[edge];
            if d <= next_acc || edge == n - 1 {
                let t = (d - accumulated) / edge_lengths[edge];
                let j = (edge + 1) % n;
                let x = verts[edge].0 + t * (verts[j].0 - verts[edge].0);
                let y = verts[edge].1 + t * (verts[j].1 - verts[edge].1);
                positions.push((x, y));
                break;
            }
            accumulated = next_acc;
        }
    }

    positions
}

/// Generate luminaire placements from pole positions and configuration.
///
/// Each pole generates one or more luminaires according to the arrangement type.
pub fn generate_placements(
    pole_positions: &[(f64, f64)],
    mounting_height: f64,
    pole_config: &PoleConfig,
    base_rotation: f64,
) -> Vec<LuminairePlace> {
    let mut placements = Vec::new();
    let mut id = 0;

    for &(px, py) in pole_positions {
        let effective_tilt = pole_config.arm_droop;

        match pole_config.arrangement {
            ArrangementType::Single => {
                placements.push(LuminairePlace {
                    id,
                    x: px,
                    y: py,
                    mounting_height,
                    tilt_angle: effective_tilt,
                    rotation: base_rotation,
                    arm_length: pole_config.arm_length,
                    arm_direction: base_rotation,
                });
                id += 1;
            }
            ArrangementType::WallMounted => {
                // Wall-mounted: tilt 90° outward from wall (horizontal beam)
                placements.push(LuminairePlace {
                    id,
                    x: px,
                    y: py,
                    mounting_height,
                    tilt_angle: 90.0 + effective_tilt,
                    rotation: base_rotation,
                    arm_length: 0.0,
                    arm_direction: base_rotation,
                });
                id += 1;
            }
            ArrangementType::BackToBack => {
                // Two luminaires facing opposite directions
                for offset in [0.0, 180.0] {
                    let rot = base_rotation + offset;
                    placements.push(LuminairePlace {
                        id,
                        x: px,
                        y: py,
                        mounting_height,
                        tilt_angle: effective_tilt,
                        rotation: rot % 360.0,
                        arm_length: pole_config.arm_length,
                        arm_direction: rot % 360.0,
                    });
                    id += 1;
                }
            }
            ArrangementType::TwinArm => {
                // Two luminaires, same direction, slight lateral offset
                let offset_m = 0.3; // 30cm lateral offset
                let dir_rad = base_rotation.to_radians();
                let perp_x = -dir_rad.cos() * offset_m;
                let perp_y = dir_rad.sin() * offset_m;

                for sign in [-1.0, 1.0] {
                    placements.push(LuminairePlace {
                        id,
                        x: px + sign * perp_x,
                        y: py + sign * perp_y,
                        mounting_height,
                        tilt_angle: effective_tilt,
                        rotation: base_rotation,
                        arm_length: pole_config.arm_length,
                        arm_direction: base_rotation,
                    });
                    id += 1;
                }
            }
            ArrangementType::Quad => {
                for i in 0..4 {
                    let rot = base_rotation + i as f64 * 90.0;
                    placements.push(LuminairePlace {
                        id,
                        x: px,
                        y: py,
                        mounting_height,
                        tilt_angle: effective_tilt,
                        rotation: rot % 360.0,
                        arm_length: pole_config.arm_length,
                        arm_direction: rot % 360.0,
                    });
                    id += 1;
                }
            }
        }
    }

    placements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_positions_center_correctly() {
        let positions = generate_pole_positions(2, 3, 60.0, 40.0);
        assert_eq!(positions.len(), 6);

        // First pole should be at (10, 10) — inset by half spacing
        assert!((positions[0].0 - 10.0).abs() < 0.01);
        assert!((positions[0].1 - 10.0).abs() < 0.01);

        // Last pole should be at (50, 30)
        assert!((positions[5].0 - 50.0).abs() < 0.01);
        assert!((positions[5].1 - 30.0).abs() < 0.01);
    }

    #[test]
    fn arrangement_multiplies_luminaires() {
        let poles = vec![(10.0, 10.0), (30.0, 10.0)];
        let cfg = PoleConfig {
            arrangement: ArrangementType::BackToBack,
            arm_length: 1.5,
            arm_droop: 0.0,
        };
        let placements = generate_placements(&poles, 10.0, &cfg, 0.0);
        assert_eq!(placements.len(), 4); // 2 poles × 2 luminaires
    }

    #[test]
    fn quad_generates_four() {
        let poles = vec![(20.0, 20.0)];
        let cfg = PoleConfig {
            arrangement: ArrangementType::Quad,
            arm_length: 1.0,
            arm_droop: 0.0,
        };
        let placements = generate_placements(&poles, 12.0, &cfg, 0.0);
        assert_eq!(placements.len(), 4);

        // Check rotations are 0, 90, 180, 270
        let rotations: Vec<f64> = placements.iter().map(|p| p.rotation).collect();
        assert!((rotations[0] - 0.0).abs() < 0.01);
        assert!((rotations[1] - 90.0).abs() < 0.01);
        assert!((rotations[2] - 180.0).abs() < 0.01);
        assert!((rotations[3] - 270.0).abs() < 0.01);
    }

    #[test]
    fn perimeter_generates_correct_count() {
        let positions = generate_perimeter_positions(8, 60.0, 40.0);
        assert_eq!(positions.len(), 8);

        // All positions should be near the edges (within margin + a bit)
        let margin = 40.0 * 0.05; // 2.0
        for (x, y) in &positions {
            let near_edge = *x <= margin + 0.1
                || *x >= 60.0 - margin - 0.1
                || *y <= margin + 0.1
                || *y >= 40.0 - margin - 0.1;
            assert!(near_edge, "({x:.1}, {y:.1}) not on perimeter");
        }
    }

    #[test]
    fn perimeter_single_goes_center() {
        let positions = generate_perimeter_positions(1, 60.0, 40.0);
        assert_eq!(positions.len(), 1);
        assert!((positions[0].0 - 30.0).abs() < 0.01);
        assert!((positions[0].1 - 20.0).abs() < 0.01);
    }

    #[test]
    fn perimeter_four_hits_corners() {
        let positions = generate_perimeter_positions(4, 40.0, 40.0);
        assert_eq!(positions.len(), 4);
        // With a square area, 4 poles should be near the corners
        let m = 40.0 * 0.05; // 2.0
                             // First should be near top-left
        assert!((positions[0].0 - m).abs() < 0.1);
        assert!((positions[0].1 - m).abs() < 0.1);
    }

    #[test]
    fn polygon_grid_all_inside() {
        use crate::area::polygon::AreaPolygon;
        let poly = AreaPolygon::rectangle(60.0, 40.0);
        let positions = generate_pole_positions_in_polygon(2, 3, &poly);
        assert_eq!(positions.len(), 6);
        // All positions must be inside the polygon
        for (x, y) in &positions {
            assert!(poly.contains(*x, *y), "({x}, {y}) outside polygon");
        }
    }

    #[test]
    fn polygon_grid_filters_outside() {
        use crate::area::polygon::AreaPolygon;
        // Triangle: only ~half the bounding box is inside
        let poly = AreaPolygon::new(vec![(0.0, 0.0), (60.0, 0.0), (30.0, 40.0)]);
        let positions = generate_pole_positions_in_polygon(3, 3, &poly);
        // Should have some poles, but all must be inside
        assert!(!positions.is_empty());
        for (x, y) in &positions {
            assert!(poly.contains(*x, *y), "({x}, {y}) outside polygon");
        }
    }

    #[test]
    fn perimeter_polygon_on_edges() {
        use crate::area::polygon::AreaPolygon;
        let poly = AreaPolygon::rectangle(60.0, 40.0);
        let positions = generate_perimeter_positions_polygon(8, &poly);
        assert_eq!(positions.len(), 8);
        // All positions should be on or very near the polygon edges
        for (x, y) in &positions {
            let near_edge = (*x).abs() < 0.5
                || (*x - 60.0).abs() < 0.5
                || (*y).abs() < 0.5
                || (*y - 40.0).abs() < 0.5;
            assert!(near_edge, "({x:.1}, {y:.1}) not on perimeter");
        }
    }
}
