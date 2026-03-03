//! Marching squares contour line generation
//!
//! Generates SVG path strings for contour lines at given threshold values
//! on a 2D scalar grid. Used by isolux footprint and isocandela diagrams.

/// A contour line at a specific threshold value
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContourLine {
    /// SVG path segments (each is a connected polyline as SVG path string)
    pub paths: Vec<String>,
}

/// Generate contour lines using the marching squares algorithm.
///
/// # Arguments
/// * `grid` - 2D scalar field, indexed as `grid[row][col]`
/// * `x_coords` - X screen coordinates for each column
/// * `y_coords` - Y screen coordinates for each row
/// * `threshold` - The value at which to generate the contour
///
/// # Returns
/// A `ContourLine` with SVG path strings for the given threshold
pub fn marching_squares(
    grid: &[Vec<f64>],
    x_coords: &[f64],
    y_coords: &[f64],
    threshold: f64,
) -> ContourLine {
    let rows = grid.len();
    if rows < 2 {
        return ContourLine { paths: Vec::new() };
    }
    let cols = grid[0].len();
    if cols < 2 || x_coords.len() < cols || y_coords.len() < rows {
        return ContourLine { paths: Vec::new() };
    }

    // Collect all line segments from marching squares cells
    let mut segments: Vec<((f64, f64), (f64, f64))> = Vec::new();

    for row in 0..rows - 1 {
        for col in 0..cols - 1 {
            let v00 = grid[row][col];
            let v10 = grid[row][col + 1];
            let v01 = grid[row + 1][col];
            let v11 = grid[row + 1][col + 1];

            // Classify corners: 1 if >= threshold, 0 if < threshold
            let case = ((v00 >= threshold) as u8) << 3
                | ((v10 >= threshold) as u8) << 2
                | ((v11 >= threshold) as u8) << 1
                | ((v01 >= threshold) as u8);

            if case == 0 || case == 15 {
                continue; // No contour in this cell
            }

            let x0 = x_coords[col];
            let x1 = x_coords[col + 1];
            let y0 = y_coords[row];
            let y1 = y_coords[row + 1];

            // Interpolation helpers for edge midpoints
            let top = lerp_x(x0, x1, v00, v10, threshold);
            let bottom = lerp_x(x0, x1, v01, v11, threshold);
            let left = lerp_y(y0, y1, v00, v01, threshold);
            let right = lerp_y(y0, y1, v10, v11, threshold);

            let top_pt = (top, y0);
            let bottom_pt = (bottom, y1);
            let left_pt = (x0, left);
            let right_pt = (x1, right);

            match case {
                1 | 14 => segments.push((left_pt, bottom_pt)),
                2 | 13 => segments.push((bottom_pt, right_pt)),
                3 | 12 => segments.push((left_pt, right_pt)),
                4 | 11 => segments.push((top_pt, right_pt)),
                5 => {
                    // Saddle point — use average to disambiguate
                    let avg = (v00 + v10 + v01 + v11) / 4.0;
                    if avg >= threshold {
                        segments.push((left_pt, top_pt));
                        segments.push((bottom_pt, right_pt));
                    } else {
                        segments.push((left_pt, bottom_pt));
                        segments.push((top_pt, right_pt));
                    }
                }
                6 | 9 => segments.push((top_pt, bottom_pt)),
                7 | 8 => segments.push((left_pt, top_pt)),
                10 => {
                    // Saddle point
                    let avg = (v00 + v10 + v01 + v11) / 4.0;
                    if avg >= threshold {
                        segments.push((left_pt, bottom_pt));
                        segments.push((top_pt, right_pt));
                    } else {
                        segments.push((left_pt, top_pt));
                        segments.push((bottom_pt, right_pt));
                    }
                }
                _ => {} // 0 and 15 already handled
            }
        }
    }

    // Chain segments into polylines
    let paths = chain_segments_to_svg(segments);

    ContourLine { paths }
}

/// Linear interpolation along X edge
fn lerp_x(x0: f64, x1: f64, v0: f64, v1: f64, threshold: f64) -> f64 {
    if (v1 - v0).abs() < 1e-12 {
        (x0 + x1) / 2.0
    } else {
        let t = (threshold - v0) / (v1 - v0);
        x0 + t * (x1 - x0)
    }
}

/// Linear interpolation along Y edge
fn lerp_y(y0: f64, y1: f64, v0: f64, v1: f64, threshold: f64) -> f64 {
    if (v1 - v0).abs() < 1e-12 {
        (y0 + y1) / 2.0
    } else {
        let t = (threshold - v0) / (v1 - v0);
        y0 + t * (y1 - y0)
    }
}

/// Chain disconnected line segments into connected polylines and output SVG path strings.
fn chain_segments_to_svg(segments: Vec<((f64, f64), (f64, f64))>) -> Vec<String> {
    if segments.is_empty() {
        return Vec::new();
    }

    let eps = 0.5; // pixel tolerance for connecting segments
    let mut used = vec![false; segments.len()];
    let mut paths = Vec::new();

    for start_idx in 0..segments.len() {
        if used[start_idx] {
            continue;
        }
        used[start_idx] = true;

        let mut chain: Vec<(f64, f64)> = vec![segments[start_idx].0, segments[start_idx].1];

        // Extend forward
        loop {
            let tail = *chain.last().unwrap();
            let mut found = false;
            for i in 0..segments.len() {
                if used[i] {
                    continue;
                }
                let (a, b) = segments[i];
                if dist(tail, a) < eps {
                    used[i] = true;
                    chain.push(b);
                    found = true;
                    break;
                } else if dist(tail, b) < eps {
                    used[i] = true;
                    chain.push(a);
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }

        // Extend backward
        loop {
            let head = chain[0];
            let mut found = false;
            for i in 0..segments.len() {
                if used[i] {
                    continue;
                }
                let (a, b) = segments[i];
                if dist(head, b) < eps {
                    used[i] = true;
                    chain.insert(0, a);
                    found = true;
                    break;
                } else if dist(head, a) < eps {
                    used[i] = true;
                    chain.insert(0, b);
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }

        // Convert chain to SVG path
        if chain.len() >= 2 {
            let mut path = format!("M {:.1} {:.1}", chain[0].0, chain[0].1);
            for pt in &chain[1..] {
                path.push_str(&format!(" L {:.1} {:.1}", pt.0, pt.1));
            }
            paths.push(path);
        }
    }

    paths
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_contour() {
        // 3x3 grid with a circle-like pattern
        let grid = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        let x = vec![0.0, 50.0, 100.0];
        let y = vec![0.0, 50.0, 100.0];

        let contour = marching_squares(&grid, &x, &y, 0.5);
        assert!(
            !contour.paths.is_empty(),
            "Should generate at least one contour path"
        );
        // Each path should start with M
        for path in &contour.paths {
            assert!(path.starts_with("M "), "Path should start with M");
        }
    }

    #[test]
    fn test_no_contour_all_below() {
        let grid = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let x = vec![0.0, 100.0];
        let y = vec![0.0, 100.0];

        let contour = marching_squares(&grid, &x, &y, 0.5);
        assert!(contour.paths.is_empty(), "No contour when all below");
    }

    #[test]
    fn test_no_contour_all_above() {
        let grid = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let x = vec![0.0, 100.0];
        let y = vec![0.0, 100.0];

        let contour = marching_squares(&grid, &x, &y, 0.5);
        assert!(contour.paths.is_empty(), "No contour when all above");
    }

    #[test]
    fn test_horizontal_contour() {
        // Top row above, bottom row below → horizontal contour
        let grid = vec![vec![1.0, 1.0], vec![0.0, 0.0]];
        let x = vec![0.0, 100.0];
        let y = vec![0.0, 100.0];

        let contour = marching_squares(&grid, &x, &y, 0.5);
        assert_eq!(contour.paths.len(), 1);
    }
}
