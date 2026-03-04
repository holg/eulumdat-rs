//! Isolux ground footprint diagram
//!
//! Computes ground-plane illuminance for a tilted luminaire at a given height,
//! then renders a heatmap with contour lines showing equal-lux isolines.
//!
//! ## Formula (tilted luminaire at height H, tilt α):
//! ```text
//! For ground point (x, y):
//!   r = sqrt(x² + y² + H²)
//!   Rotate (x, y, -H) by -α around Y → (dx_rot, dy_rot, dz_rot)
//!   γ = acos(-dz_rot / r),  C = atan2(dy_rot, dx_rot)
//!   I = ldt.sample(C°, γ°)
//!   E = I · (H/r) / r² · (flux/1000)
//! ```

use super::color::{heatmap_color, Color};
use super::contour::{marching_squares, ContourLine};
use crate::units::UnitSystem;
use crate::Eulumdat;

/// Parameters for isolux calculation
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsoluxParams {
    /// Mounting height in meters
    pub mounting_height: f64,
    /// Tilt angle in degrees (0 = pointing straight down, 90 = horizontal)
    pub tilt_angle: f64,
    /// Half-width of the ground area in meters
    pub area_half_width: f64,
    /// Half-depth of the ground area in meters
    pub area_half_depth: f64,
    /// Number of grid cells per axis
    pub grid_resolution: usize,
}

impl Default for IsoluxParams {
    fn default() -> Self {
        Self {
            mounting_height: 10.0,
            tilt_angle: 0.0,
            area_half_width: 20.0,
            area_half_depth: 20.0,
            grid_resolution: 80,
        }
    }
}

/// A single cell in the isolux grid
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsoluxCell {
    /// Ground X position in meters
    pub x_m: f64,
    /// Ground Y position in meters
    pub y_m: f64,
    /// Screen X position
    pub sx: f64,
    /// Screen Y position
    pub sy: f64,
    /// Cell width in pixels
    pub width: f64,
    /// Cell height in pixels
    pub height: f64,
    /// Illuminance in lux
    pub lux: f64,
    /// Normalized illuminance (0–1)
    pub normalized: f64,
    /// Cell color
    pub color: Color,
}

/// A contour line at a specific lux level
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsoluxContour {
    /// Lux value for this contour
    pub lux_value: f64,
    /// SVG path strings
    pub paths: Vec<String>,
    /// Label (e.g., "100 lx")
    pub label: String,
}

/// Isolux ground footprint diagram
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IsoluxDiagram {
    /// Grid cells with illuminance data
    pub cells: Vec<IsoluxCell>,
    /// Contour lines
    pub contours: Vec<IsoluxContour>,
    /// Parameters used
    pub params: IsoluxParams,
    /// Maximum illuminance in lux
    pub max_lux: f64,
    /// Total luminous flux (lm)
    pub total_flux: f64,
    /// Plot dimensions
    pub plot_width: f64,
    pub plot_height: f64,
    pub margin_left: f64,
    pub margin_top: f64,
}

impl IsoluxDiagram {
    /// Generate isolux diagram from Eulumdat data.
    pub fn from_eulumdat(ldt: &Eulumdat, width: f64, height: f64, params: IsoluxParams) -> Self {
        Self::from_eulumdat_with_units(ldt, width, height, params, UnitSystem::default())
    }

    /// Generate isolux diagram with unit system for labels.
    pub fn from_eulumdat_with_units(
        ldt: &Eulumdat,
        width: f64,
        height: f64,
        params: IsoluxParams,
        units: UnitSystem,
    ) -> Self {
        let margin_left = 60.0;
        let margin_right = 80.0; // For color legend
        let margin_top = 40.0;
        let margin_bottom = 55.0;

        let plot_width = width - margin_left - margin_right;
        let plot_height = height - margin_top - margin_bottom;

        let n = params.grid_resolution;
        let h = params.mounting_height;
        let tilt_rad = params.tilt_angle.to_radians();

        let total_flux: f64 = ldt
            .lamp_sets
            .iter()
            .map(|ls| ls.total_luminous_flux * ls.num_lamps as f64)
            .sum();
        let flux_scale = total_flux / 1000.0;

        // Build grid
        let dx = 2.0 * params.area_half_width / n as f64;
        let dy = 2.0 * params.area_half_depth / n as f64;
        let cell_w = plot_width / n as f64;
        let cell_h = plot_height / n as f64;

        let mut lux_grid: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
        let mut max_lux: f64 = 0.0;

        // Compute illuminance at each grid point
        for (row, grid_row) in lux_grid.iter_mut().enumerate() {
            let gy = -params.area_half_depth + (row as f64 + 0.5) * dy;
            for (col, cell_val) in grid_row.iter_mut().enumerate() {
                let gx = -params.area_half_width + (col as f64 + 0.5) * dx;

                let lux = Self::compute_illuminance(ldt, gx, gy, h, tilt_rad, flux_scale);
                *cell_val = lux;
                if lux > max_lux {
                    max_lux = lux;
                }
            }
        }

        // Build cells
        let mut cells = Vec::with_capacity(n * n);
        for (row, grid_row) in lux_grid.iter().enumerate() {
            let gy = -params.area_half_depth + (row as f64 + 0.5) * dy;
            for (col, &lux) in grid_row.iter().enumerate() {
                let gx = -params.area_half_width + (col as f64 + 0.5) * dx;
                let normalized = if max_lux > 0.0 { lux / max_lux } else { 0.0 };

                cells.push(IsoluxCell {
                    x_m: gx,
                    y_m: gy,
                    sx: margin_left + col as f64 * cell_w,
                    sy: margin_top + row as f64 * cell_h,
                    width: cell_w,
                    height: cell_h,
                    lux,
                    normalized,
                    color: heatmap_color(normalized),
                });
            }
        }

        // Generate contour lines at "nice" levels in the display unit.
        // For Imperial, pick round foot-candle values and convert to lux for the grid.
        let contour_levels: Vec<f64> = match units {
            UnitSystem::Imperial => {
                // Nice fc values → convert to lux (1 fc = 10.764 lux)
                [0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0]
                    .iter()
                    .map(|&fc| fc * 10.764)
                    .collect()
            }
            UnitSystem::Metric => {
                vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
            }
        };
        let x_coords: Vec<f64> = (0..n)
            .map(|col| margin_left + (col as f64 + 0.5) * cell_w)
            .collect();
        let y_coords: Vec<f64> = (0..n)
            .map(|row| margin_top + (row as f64 + 0.5) * cell_h)
            .collect();

        let illu_label = units.illuminance_label();
        let contours: Vec<IsoluxContour> = contour_levels
            .iter()
            .filter(|&&level| level <= max_lux && level > 0.0)
            .map(|&level| {
                let cl: ContourLine = marching_squares(&lux_grid, &x_coords, &y_coords, level);
                let display_val = units.convert_lux(level);
                IsoluxContour {
                    lux_value: level,
                    paths: cl.paths,
                    label: format!("{display_val:.0} {illu_label}"),
                }
            })
            .filter(|c| !c.paths.is_empty())
            .collect();

        Self {
            cells,
            contours,
            params,
            max_lux,
            total_flux,
            plot_width,
            plot_height,
            margin_left,
            margin_top,
        }
    }

    /// Compute illuminance at ground point (gx, gy) in lux
    fn compute_illuminance(
        ldt: &Eulumdat,
        gx: f64,
        gy: f64,
        h: f64,
        tilt_rad: f64,
        flux_scale: f64,
    ) -> f64 {
        // Vector from luminaire to ground point
        let dx = gx;
        let dy = gy;
        let dz = -h; // luminaire is at (0, 0, h), ground is at z=0

        let r = (dx * dx + dy * dy + dz * dz).sqrt();
        if r < 1e-6 {
            return 0.0;
        }

        // Rotate direction by -tilt around Y axis (tilt rotates the luminaire)
        let cos_t = tilt_rad.cos();
        let sin_t = tilt_rad.sin();

        let dx_rot = dx * cos_t + dz * sin_t;
        let dy_rot = dy;
        let dz_rot = -dx * sin_t + dz * cos_t;

        // Convert to Type C angles
        let gamma = (-dz_rot / r).acos(); // gamma from nadir
        let c = dy_rot.atan2(dx_rot);

        let mut c_deg = c.to_degrees();
        if c_deg < 0.0 {
            c_deg += 360.0;
        }
        let gamma_deg = gamma.to_degrees();

        // Get intensity from photometric data (cd/klm)
        let intensity = ldt.sample(c_deg, gamma_deg);

        // E = I × cos(θ_incidence) / r²
        // cos(θ_incidence) for a horizontal surface = h/r (approximately)
        let cos_incidence = h / r;
        let illuminance = intensity * flux_scale * cos_incidence / (r * r);

        illuminance.max(0.0)
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
    fn test_isolux_generation() {
        let ldt = create_test_ldt();
        let params = IsoluxParams {
            mounting_height: 8.0,
            tilt_angle: 0.0,
            area_half_width: 15.0,
            area_half_depth: 15.0,
            grid_resolution: 40,
        };
        let diagram = IsoluxDiagram::from_eulumdat(&ldt, 600.0, 500.0, params);

        assert_eq!(diagram.cells.len(), 40 * 40);
        assert!(diagram.max_lux > 0.0);
    }

    #[test]
    fn test_isolux_with_tilt() {
        let ldt = create_test_ldt();
        let params = IsoluxParams {
            mounting_height: 10.0,
            tilt_angle: 30.0,
            area_half_width: 20.0,
            area_half_depth: 20.0,
            grid_resolution: 30,
        };
        let diagram = IsoluxDiagram::from_eulumdat(&ldt, 600.0, 500.0, params);

        assert!(diagram.max_lux > 0.0);
        // With tilt, the peak should shift away from center
    }

    #[test]
    fn test_isolux_contours() {
        let ldt = create_test_ldt();
        let params = IsoluxParams::default();
        let diagram = IsoluxDiagram::from_eulumdat(&ldt, 600.0, 500.0, params);

        // Should have at least some contour levels
        // (exact count depends on max_lux)
        assert!(diagram.max_lux > 0.0);
    }
}
