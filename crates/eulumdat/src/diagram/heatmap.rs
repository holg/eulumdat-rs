//! Heatmap diagram data generation
//!
//! Generates the data needed for 2D intensity heatmaps showing
//! candela values across C-planes (x-axis) and gamma angles (y-axis).

use super::color::{heatmap_color, Color};
use super::DiagramScale;
use crate::Eulumdat;

/// A single cell in the heatmap
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeatmapCell {
    /// X position (C-plane index)
    pub c_index: usize,
    /// Y position (gamma index)
    pub g_index: usize,
    /// C-plane angle in degrees
    pub c_angle: f64,
    /// Gamma angle in degrees
    pub g_angle: f64,
    /// Intensity value in cd/klm
    pub intensity: f64,
    /// Intensity value in candela (scaled by lamp flux)
    pub candela: f64,
    /// Normalized intensity (0.0 to 1.0)
    pub normalized: f64,
    /// Color for this cell
    pub color: Color,
    /// Screen position X (top-left)
    pub x: f64,
    /// Screen position Y (top-left)
    pub y: f64,
    /// Cell width
    pub width: f64,
    /// Cell height
    pub height: f64,
}

/// Complete heatmap diagram data
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeatmapDiagram {
    /// All cells in the heatmap
    pub cells: Vec<HeatmapCell>,
    /// Scale information
    pub scale: DiagramScale,
    /// Maximum candela value
    pub max_candela: f64,
    /// Total lamp flux used for scaling
    pub total_flux: f64,
    /// C-plane angles for axis labels
    pub c_angles: Vec<f64>,
    /// Gamma angles for axis labels
    pub g_angles: Vec<f64>,
    /// Color legend entries (normalized_value, color, candela_value)
    pub legend_entries: Vec<(f64, Color, f64)>,
    /// Plot dimensions (for reference)
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

impl HeatmapDiagram {
    /// Generate heatmap diagram data from Eulumdat
    ///
    /// # Arguments
    /// * `ldt` - The Eulumdat data
    /// * `width` - Total output width in pixels
    /// * `height` - Total output height in pixels
    pub fn from_eulumdat(ldt: &Eulumdat, width: f64, height: f64) -> Self {
        let margin_left = 60.0;
        let margin_right = 100.0; // Space for color legend
        let margin_top = 50.0;
        let margin_bottom = 60.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;

        let num_c = ldt.c_angles.len();
        let num_g = ldt.g_angles.len();

        if num_c == 0 || num_g == 0 || ldt.intensities.is_empty() {
            return Self::empty();
        }

        // Calculate max intensity
        let max_intensity = ldt.max_intensity();

        // Get total flux for candela conversion
        let total_flux: f64 = ldt
            .lamp_sets
            .iter()
            .map(|ls| ls.total_luminous_flux * ls.num_lamps as f64)
            .sum();
        let scale_factor = total_flux / 1000.0; // cd/klm to cd
        let max_candela = max_intensity * scale_factor;

        // Cell dimensions
        let cell_width = plot_width / num_c as f64;
        let cell_height = plot_height / num_g as f64;

        // Generate cells
        let cells = generate_cells(
            ldt,
            margin_left,
            margin_top,
            cell_width,
            cell_height,
            max_intensity,
            scale_factor,
        );

        // Generate legend entries
        let legend_entries = generate_legend_entries(max_candela, 50);

        let scale = DiagramScale::from_max_intensity(max_intensity, 5);

        Self {
            cells,
            scale,
            max_candela,
            total_flux,
            c_angles: ldt.c_angles.clone(),
            g_angles: ldt.g_angles.clone(),
            legend_entries,
            plot_width,
            plot_height,
            margin_left,
            margin_top,
        }
    }

    /// Create an empty heatmap (for when there's no data)
    fn empty() -> Self {
        Self {
            cells: Vec::new(),
            scale: DiagramScale {
                max_intensity: 0.0,
                scale_max: 100.0,
                grid_values: vec![0.0, 25.0, 50.0, 75.0, 100.0],
            },
            max_candela: 0.0,
            total_flux: 0.0,
            c_angles: Vec::new(),
            g_angles: Vec::new(),
            legend_entries: Vec::new(),
            plot_width: 0.0,
            plot_height: 0.0,
            margin_left: 0.0,
            margin_top: 0.0,
        }
    }

    /// Check if the heatmap has data
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Get cell at specific indices
    pub fn get_cell(&self, c_index: usize, g_index: usize) -> Option<&HeatmapCell> {
        self.cells
            .iter()
            .find(|c| c.c_index == c_index && c.g_index == g_index)
    }

    /// Get intensity at specific C and G angles (interpolated if needed)
    pub fn intensity_at(&self, c_angle: f64, g_angle: f64) -> Option<f64> {
        // Find the closest cell
        let c_idx = self
            .c_angles
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((*a - c_angle).abs())
                    .partial_cmp(&((*b - c_angle).abs()))
                    .unwrap()
            })?
            .0;

        let g_idx = self
            .g_angles
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((*a - g_angle).abs())
                    .partial_cmp(&((*b - g_angle).abs()))
                    .unwrap()
            })?
            .0;

        self.get_cell(c_idx, g_idx).map(|c| c.intensity)
    }

    /// Get X-axis labels (C-angles) with positions
    pub fn x_labels(&self, step: usize) -> Vec<(f64, f64, String)> {
        self.c_angles
            .iter()
            .enumerate()
            .filter(|(i, _)| i % step == 0)
            .map(|(i, &c)| {
                let x = self.margin_left
                    + (i as f64 + 0.5) * (self.plot_width / self.c_angles.len() as f64);
                (x, c, format!("{:.0}", c))
            })
            .collect()
    }

    /// Get Y-axis labels (G-angles) with positions
    pub fn y_labels(&self, step: usize) -> Vec<(f64, f64, String)> {
        self.g_angles
            .iter()
            .enumerate()
            .filter(|(i, _)| i % step == 0)
            .map(|(i, &g)| {
                let y = self.margin_top
                    + (i as f64 + 0.5) * (self.plot_height / self.g_angles.len() as f64);
                (y, g, format!("{:.0}", g))
            })
            .collect()
    }
}

/// Generate heatmap cells
fn generate_cells(
    ldt: &Eulumdat,
    margin_left: f64,
    margin_top: f64,
    cell_width: f64,
    cell_height: f64,
    max_intensity: f64,
    scale_factor: f64,
) -> Vec<HeatmapCell> {
    let mut cells = Vec::new();

    for (c_idx, &c_angle) in ldt.c_angles.iter().enumerate() {
        if c_idx >= ldt.intensities.len() {
            continue;
        }

        for (g_idx, &g_angle) in ldt.g_angles.iter().enumerate() {
            if g_idx >= ldt.intensities[c_idx].len() {
                continue;
            }

            let intensity = ldt.intensities[c_idx][g_idx];
            let candela = intensity * scale_factor;
            let normalized = if max_intensity > 0.0 {
                intensity / max_intensity
            } else {
                0.0
            };

            let x = margin_left + c_idx as f64 * cell_width;
            let y = margin_top + g_idx as f64 * cell_height;
            let color = heatmap_color(normalized);

            cells.push(HeatmapCell {
                c_index: c_idx,
                g_index: g_idx,
                c_angle,
                g_angle,
                intensity,
                candela,
                normalized,
                color,
                x,
                y,
                width: cell_width,
                height: cell_height,
            });
        }
    }

    cells
}

/// Generate color legend entries
fn generate_legend_entries(max_candela: f64, num_segments: usize) -> Vec<(f64, Color, f64)> {
    (0..num_segments)
        .map(|i| {
            let normalized = (num_segments - 1 - i) as f64 / (num_segments - 1) as f64;
            let candela = max_candela * normalized;
            let color = heatmap_color(normalized);
            (normalized, color, candela)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LampSet;

    #[allow(clippy::field_reassign_with_default)]
    fn create_test_ldt() -> Eulumdat {
        let mut ldt = Eulumdat::default();
        ldt.c_angles = vec![0.0, 90.0, 180.0, 270.0];
        ldt.g_angles = vec![0.0, 30.0, 60.0, 90.0];
        ldt.intensities = vec![
            vec![100.0, 80.0, 50.0, 20.0],
            vec![90.0, 70.0, 45.0, 18.0],
            vec![100.0, 80.0, 50.0, 20.0],
            vec![90.0, 70.0, 45.0, 18.0],
        ];
        ldt.lamp_sets = vec![LampSet {
            num_lamps: 1,
            total_luminous_flux: 1000.0,
            ..Default::default()
        }];
        ldt
    }

    #[test]
    fn test_heatmap_generation() {
        let ldt = create_test_ldt();
        let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);

        // Should have cells for all C x G combinations
        assert_eq!(heatmap.cells.len(), 4 * 4);

        // Max candela should be intensity * flux/1000
        assert!((heatmap.max_candela - 100.0).abs() < 0.01);

        // Should have legend entries
        assert!(!heatmap.legend_entries.is_empty());
    }

    #[test]
    fn test_heatmap_cell_colors() {
        let ldt = create_test_ldt();
        let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);

        // High intensity cells should be more red
        let high_cell = heatmap.get_cell(0, 0).unwrap();
        assert!(high_cell.normalized > 0.9);

        // Low intensity cells should be more blue
        let low_cell = heatmap.get_cell(1, 3).unwrap();
        assert!(low_cell.normalized < 0.2);
    }

    #[test]
    fn test_empty_heatmap() {
        let ldt = Eulumdat::default();
        let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);

        assert!(heatmap.is_empty());
    }

    #[test]
    fn test_intensity_lookup() {
        let ldt = create_test_ldt();
        let heatmap = HeatmapDiagram::from_eulumdat(&ldt, 700.0, 500.0);

        let intensity = heatmap.intensity_at(0.0, 0.0);
        assert!(intensity.is_some());
        assert!((intensity.unwrap() - 100.0).abs() < 0.01);
    }
}
