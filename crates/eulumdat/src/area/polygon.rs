//! Custom area polygon — point-in-polygon test and bounding box.

/// A simple polygon defining the area boundary.
///
/// Vertices are in order (clockwise or counter-clockwise). Must have >= 3 points.
/// Coordinates are in meters, same coordinate system as pole positions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AreaPolygon {
    pub vertices: Vec<(f64, f64)>,
}

impl AreaPolygon {
    /// Create a new polygon from vertices.
    pub fn new(vertices: Vec<(f64, f64)>) -> Self {
        Self { vertices }
    }

    /// Create a rectangular polygon (equivalent to the default rectangle area).
    pub fn rectangle(width: f64, depth: f64) -> Self {
        Self {
            vertices: vec![(0.0, 0.0), (width, 0.0), (width, depth), (0.0, depth)],
        }
    }

    /// Whether the polygon has enough vertices to be valid.
    pub fn is_valid(&self) -> bool {
        self.vertices.len() >= 3
    }

    /// Axis-aligned bounding box: (min_x, min_y, max_x, max_y).
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for &(x, y) in &self.vertices {
            if x < min_x {
                min_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if x > max_x {
                max_x = x;
            }
            if y > max_y {
                max_y = y;
            }
        }
        (min_x, min_y, max_x, max_y)
    }

    /// Bounding box width and height.
    pub fn bbox_size(&self) -> (f64, f64) {
        let (x0, y0, x1, y1) = self.bounding_box();
        (x1 - x0, y1 - y0)
    }

    /// Test if a point (x, y) is inside the polygon using ray-casting algorithm.
    ///
    /// Works correctly for both convex and concave simple (non-self-intersecting) polygons.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }

        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = self.vertices[i];
            let (xj, yj) = self.vertices[j];

            // Ray from (x, y) going in +X direction
            if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// Build a mask grid: true for cells whose center is inside the polygon.
    ///
    /// The grid covers the bounding box with the given resolution.
    pub fn build_mask(&self, grid_resolution: usize) -> Vec<Vec<bool>> {
        let (x0, y0, x1, y1) = self.bounding_box();
        let w = x1 - x0;
        let h = y1 - y0;
        let n = grid_resolution;
        let dx = w / n as f64;
        let dy = h / n as f64;

        (0..n)
            .map(|row| {
                let cy = y0 + (row as f64 + 0.5) * dy;
                (0..n)
                    .map(|col| {
                        let cx = x0 + (col as f64 + 0.5) * dx;
                        self.contains(cx, cy)
                    })
                    .collect()
            })
            .collect()
    }

    /// Generate SVG polygon points string, transformed to SVG coordinates.
    ///
    /// Maps polygon vertices from world coords to SVG coords using the given
    /// bounding box origin, scale, and margin.
    pub fn to_svg_points(
        &self,
        origin_x: f64,
        origin_y: f64,
        scale_x: f64,
        scale_y: f64,
        margin: f64,
    ) -> String {
        self.vertices
            .iter()
            .map(|&(x, y)| {
                let sx = margin + (x - origin_x) * scale_x;
                let sy = margin + (y - origin_y) * scale_y;
                format!("{sx:.1},{sy:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Area of the polygon using the shoelace formula.
    pub fn area(&self) -> f64 {
        let n = self.vertices.len();
        if n < 3 {
            return 0.0;
        }
        let mut sum = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            let (xi, yi) = self.vertices[i];
            let (xj, yj) = self.vertices[j];
            sum += xi * yj - xj * yi;
        }
        (sum / 2.0).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_contains() {
        let poly = AreaPolygon::rectangle(10.0, 8.0);
        assert!(poly.contains(5.0, 4.0)); // center
        assert!(poly.contains(1.0, 1.0)); // near corner
        assert!(poly.contains(9.0, 7.0)); // near far corner
        assert!(!poly.contains(-1.0, 4.0)); // outside left
        assert!(!poly.contains(11.0, 4.0)); // outside right
        assert!(!poly.contains(5.0, -1.0)); // outside top
        assert!(!poly.contains(5.0, 9.0)); // outside bottom
    }

    #[test]
    fn triangle_contains() {
        let poly = AreaPolygon::new(vec![(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)]);
        assert!(poly.contains(5.0, 3.0)); // inside
        assert!(!poly.contains(0.5, 9.0)); // outside (left of hypotenuse)
        assert!(!poly.contains(9.5, 9.0)); // outside (right of hypotenuse)
    }

    #[test]
    fn concave_l_shape() {
        // L-shaped polygon
        let poly = AreaPolygon::new(vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (5.0, 5.0),
            (5.0, 10.0),
            (0.0, 10.0),
        ]);
        assert!(poly.contains(2.0, 2.0)); // inside bottom-left
        assert!(poly.contains(8.0, 2.0)); // inside bottom-right
        assert!(poly.contains(2.0, 8.0)); // inside top-left (the L's arm)
        assert!(!poly.contains(8.0, 8.0)); // outside (the L's gap)
    }

    #[test]
    fn bounding_box_correct() {
        let poly = AreaPolygon::new(vec![(2.0, 3.0), (8.0, 1.0), (6.0, 9.0)]);
        assert_eq!(poly.bounding_box(), (2.0, 1.0, 8.0, 9.0));
    }

    #[test]
    fn area_rectangle() {
        let poly = AreaPolygon::rectangle(10.0, 8.0);
        assert!((poly.area() - 80.0).abs() < 0.01);
    }

    #[test]
    fn area_triangle() {
        let poly = AreaPolygon::new(vec![(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)]);
        assert!((poly.area() - 50.0).abs() < 0.01);
    }

    #[test]
    fn mask_triangle_roughly_half() {
        // Triangle inscribed in a 10x10 square → ~50% of cells inside
        let poly = AreaPolygon::new(vec![(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)]);
        let mask = poly.build_mask(20);
        let inside: usize = mask.iter().flat_map(|r| r.iter()).filter(|&&v| v).count();
        let total = 20 * 20;
        let ratio = inside as f64 / total as f64;
        // Should be approximately 0.5 (triangle area = 50, bbox area = 100)
        assert!(
            ratio > 0.4 && ratio < 0.6,
            "Triangle mask ratio: {ratio:.2}"
        );
    }

    #[test]
    fn invalid_polygon() {
        let poly = AreaPolygon::new(vec![(0.0, 0.0), (1.0, 0.0)]); // only 2 points
        assert!(!poly.is_valid());
        assert!(!poly.contains(0.5, 0.0));
    }
}
