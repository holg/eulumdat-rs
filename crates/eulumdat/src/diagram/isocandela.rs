//! Isocandela contour plot
//!
//! 2D heatmap + contour lines on Type B (H, V) axes showing
//! equal-intensity lines at percentages of I_max.

use super::color::{heatmap_color, Color};
use super::contour::{marching_squares, ContourLine};
use crate::type_b_conversion::TypeBConversion;
use crate::Eulumdat;

/// A single cell in the isocandela grid
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsocandelaCell {
    /// Horizontal angle in degrees
    pub h_angle: f64,
    /// Vertical angle in degrees
    pub v_angle: f64,
    /// Screen X position
    pub sx: f64,
    /// Screen Y position
    pub sy: f64,
    /// Cell width in pixels
    pub width: f64,
    /// Cell height in pixels
    pub height: f64,
    /// Intensity in cd/klm
    pub intensity: f64,
    /// Normalized intensity (0–1)
    pub normalized: f64,
    /// Cell color
    pub color: Color,
}

/// A contour line at a specific intensity percentage
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsocandelaContour {
    /// Intensity value for this contour (cd/klm)
    pub intensity: f64,
    /// Percentage of I_max
    pub percentage: f64,
    /// SVG path strings
    pub paths: Vec<String>,
    /// Label (e.g., "50%")
    pub label: String,
}

/// Isocandela contour plot diagram
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsocandelaDiagram {
    /// Grid cells with intensity data
    pub cells: Vec<IsocandelaCell>,
    /// Contour lines
    pub contours: Vec<IsocandelaContour>,
    /// Maximum intensity (cd/klm)
    pub i_max: f64,
    /// H range (typically -90 to +90)
    pub h_min: f64,
    pub h_max: f64,
    /// V range (typically -90 to +90)
    pub v_min: f64,
    pub v_max: f64,
    /// Grid resolution
    pub grid_size: usize,
    /// Plot dimensions
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

impl IsocandelaDiagram {
    /// Generate isocandela diagram from Eulumdat data.
    pub fn from_eulumdat(ldt: &Eulumdat, width: f64, height: f64) -> Self {
        let margin_left = 60.0;
        let margin_right = 80.0;
        let margin_top = 40.0;
        let margin_bottom = 55.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;

        let h_min = -90.0;
        let h_max = 90.0;
        let v_min = -90.0;
        let v_max = 90.0;

        let grid_size = 90_usize; // 2° resolution
        let h_step = (h_max - h_min) / grid_size as f64;
        let v_step = (v_max - v_min) / grid_size as f64;
        let cell_w = plot_width / grid_size as f64;
        let cell_h = plot_height / grid_size as f64;

        // Build intensity grid
        let mut intensity_grid: Vec<Vec<f64>> = vec![vec![0.0; grid_size]; grid_size];
        let mut i_max: f64 = 0.0;

        for (row, grid_row) in intensity_grid.iter_mut().enumerate() {
            let v = v_max - (row as f64 + 0.5) * v_step; // top = +90°
            for (col, cell_val) in grid_row.iter_mut().enumerate() {
                let h = h_min + (col as f64 + 0.5) * h_step;
                let intensity = TypeBConversion::intensity_at_type_b(ldt, h, v);
                *cell_val = intensity;
                if intensity > i_max {
                    i_max = intensity;
                }
            }
        }

        // Build cells
        let mut cells = Vec::with_capacity(grid_size * grid_size);
        for (row, grid_row) in intensity_grid.iter().enumerate() {
            let v = v_max - (row as f64 + 0.5) * v_step;
            for (col, &intensity) in grid_row.iter().enumerate() {
                let h = h_min + (col as f64 + 0.5) * h_step;
                let normalized = if i_max > 0.0 { intensity / i_max } else { 0.0 };

                cells.push(IsocandelaCell {
                    h_angle: h,
                    v_angle: v,
                    sx: margin_left + col as f64 * cell_w,
                    sy: margin_top + row as f64 * cell_h,
                    width: cell_w,
                    height: cell_h,
                    intensity,
                    normalized,
                    color: heatmap_color(normalized),
                });
            }
        }

        // Generate contour lines at percentage levels
        let percentages = [0.10, 0.25, 0.50, 0.75, 0.90];
        let x_coords: Vec<f64> = (0..grid_size)
            .map(|col| margin_left + (col as f64 + 0.5) * cell_w)
            .collect();
        let y_coords: Vec<f64> = (0..grid_size)
            .map(|row| margin_top + (row as f64 + 0.5) * cell_h)
            .collect();

        let contours: Vec<IsocandelaContour> = percentages
            .iter()
            .filter_map(|&pct| {
                let threshold = i_max * pct;
                if threshold <= 0.0 {
                    return None;
                }
                let cl: ContourLine =
                    marching_squares(&intensity_grid, &x_coords, &y_coords, threshold);
                if cl.paths.is_empty() {
                    return None;
                }
                Some(IsocandelaContour {
                    intensity: threshold,
                    percentage: pct * 100.0,
                    paths: cl.paths,
                    label: format!("{:.0}%", pct * 100.0),
                })
            })
            .collect();

        Self {
            cells,
            contours,
            i_max,
            h_min,
            h_max,
            v_min,
            v_max,
            grid_size,
            plot_width,
            plot_height,
            margin_left,
            margin_top,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LampSet;

    fn create_test_ldt() -> Eulumdat {
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
    fn test_isocandela_generation() {
        let ldt = create_test_ldt();
        let diagram = IsocandelaDiagram::from_eulumdat(&ldt, 600.0, 500.0);

        assert_eq!(diagram.cells.len(), 90 * 90);
        assert!(diagram.i_max > 0.0);
    }

    #[test]
    fn test_isocandela_contours() {
        let ldt = create_test_ldt();
        let diagram = IsocandelaDiagram::from_eulumdat(&ldt, 600.0, 500.0);

        // Should have contour lines
        assert!(
            !diagram.contours.is_empty(),
            "Should generate at least one contour level"
        );
    }

    #[test]
    fn test_isocandela_symmetry() {
        let ldt = create_test_ldt();
        let diagram = IsocandelaDiagram::from_eulumdat(&ldt, 600.0, 500.0);

        // For this symmetric test data, the peak should be near center
        let center_cell = diagram
            .cells
            .iter()
            .find(|c| c.h_angle.abs() < 2.0 && c.v_angle.abs() < 2.0);
        assert!(center_cell.is_some());
    }
}
