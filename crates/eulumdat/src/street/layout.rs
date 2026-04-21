//! Linear road layout: a length of road with poles placed along one or both sides.

use crate::area::{compute_area_illuminance_mixed, AreaResult, LuminairePlace};
use crate::standards::DesignResult;
use crate::Eulumdat;

/// How luminaires are arranged along a road.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Arrangement {
    /// Poles on one side only, equally spaced.
    SingleSide,
    /// Poles on both sides, aligned opposite each other.
    Opposite,
    /// Poles on both sides, offset by half the spacing so they alternate.
    Staggered,
}

impl std::fmt::Display for Arrangement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleSide => write!(f, "Single-side"),
            Self::Opposite => write!(f, "Opposite"),
            Self::Staggered => write!(f, "Staggered"),
        }
    }
}

/// A linear road section with poles arranged along its length.
///
/// Coordinates: road runs along +X, roadway occupies Y ∈ [0, num_lanes * lane_width_m].
/// Poles sit at `-pole_offset_m` (near side) or `num_lanes*lane_width_m + pole_offset_m`
/// (far side), with overhang placing the luminaire head over the roadway.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreetLayout {
    /// Length of the analyzed road section, in meters.
    pub length_m: f64,
    /// Width of a single lane, in meters (e.g. 3.5 m).
    pub lane_width_m: f64,
    /// Number of traffic lanes across the roadway.
    pub num_lanes: usize,
    /// Distance between consecutive poles (same side), in meters.
    pub pole_spacing_m: f64,
    /// How poles are arranged along the road.
    pub arrangement: Arrangement,
    /// Mounting height of the luminaire head, in meters.
    pub mounting_height_m: f64,
    /// Horizontal overhang of the arm from the pole toward the roadway, in meters.
    pub overhang_m: f64,
    /// Luminaire tilt angle in degrees (0 = horizontal arm, positive = tipped upward).
    pub tilt_deg: f64,
    /// Lateral offset of the pole base from the curb, in meters.
    pub pole_offset_m: f64,
}

impl Default for StreetLayout {
    fn default() -> Self {
        Self {
            length_m: 120.0,
            lane_width_m: 3.5,
            num_lanes: 2,
            pole_spacing_m: 30.0,
            arrangement: Arrangement::Staggered,
            mounting_height_m: 10.0,
            overhang_m: 1.5,
            tilt_deg: 0.0,
            pole_offset_m: 0.5,
        }
    }
}

impl StreetLayout {
    /// Total roadway width (num_lanes * lane width), excluding shoulders/verges.
    pub fn roadway_width_m(&self) -> f64 {
        self.num_lanes as f64 * self.lane_width_m
    }

    /// Generate the list of pole placements for a single LDT.
    ///
    /// The arm direction is chosen so the luminaire head sits over the
    /// roadway (arm points from curb toward the opposite curb).
    pub fn placements(&self) -> Vec<LuminairePlace> {
        let mut out = Vec::new();
        let road_width = self.roadway_width_m();

        // Near-curb Y (side 0) and far-curb Y (side 1)
        let y_near = -self.pole_offset_m;
        let y_far = road_width + self.pole_offset_m;

        // Arm direction in degrees (0 = +Y per LuminairePlace::effective_position).
        // Near-curb arms point toward +Y (into the road), far-curb toward -Y.
        let arm_near_deg = 0.0;
        let arm_far_deg = 180.0;

        let n = if self.pole_spacing_m > 0.0 {
            (self.length_m / self.pole_spacing_m).floor() as usize + 1
        } else {
            1
        };

        let mut id = 0usize;
        for i in 0..n {
            let x = i as f64 * self.pole_spacing_m;
            if x > self.length_m + 1e-6 {
                break;
            }
            match self.arrangement {
                Arrangement::SingleSide => {
                    out.push(self.place(id, x, y_near, arm_near_deg));
                    id += 1;
                }
                Arrangement::Opposite => {
                    out.push(self.place(id, x, y_near, arm_near_deg));
                    id += 1;
                    out.push(self.place(id, x, y_far, arm_far_deg));
                    id += 1;
                }
                Arrangement::Staggered => {
                    if i % 2 == 0 {
                        out.push(self.place(id, x, y_near, arm_near_deg));
                    } else {
                        out.push(self.place(id, x, y_far, arm_far_deg));
                    }
                    id += 1;
                }
            }
        }
        out
    }

    fn place(&self, id: usize, x: f64, y: f64, arm_dir_deg: f64) -> LuminairePlace {
        LuminairePlace {
            id,
            x,
            y,
            mounting_height: self.mounting_height_m,
            tilt_angle: self.tilt_deg,
            // Align luminaire's C0 axis with the road direction (+X).
            // LuminairePlace::rotation uses 0° = +Y, clockwise, so 90° → +X.
            rotation: 90.0,
            arm_length: self.overhang_m,
            arm_direction: arm_dir_deg,
        }
    }

    /// Compute the illuminance grid for this layout using the given luminaire.
    ///
    /// `maintenance_factor` (aka light loss factor) scales the computed values
    /// to account for lumen depreciation, dirt, etc. — typically 0.7–0.85 for
    /// outdoor installations. Pass `1.0` for a bare calculation.
    ///
    /// The evaluation area covers the full roadway width, spanning one
    /// spacing cycle by default (so uniformity reflects the repeating pattern
    /// rather than end-effects of the analyzed strip).
    pub fn compute(&self, ldt: &Eulumdat, maintenance_factor: f64) -> AreaResult {
        // Evaluate over one full pole-spacing cycle centered in the road to
        // capture the worst-case uniformity between poles. Grid sized to give
        // ~0.5 m cells: that's the resolution RP-8 examples typically use.
        let eval_len = self.pole_spacing_m.max(1.0);
        let eval_width = self.roadway_width_m();
        let grid_resolution = ((eval_len.max(eval_width) / 0.5).round() as usize).max(16);

        // Place luminaires over a strip 3× the pole spacing so the central
        // section is far from edge effects. Callers who want the full road
        // strip instead should use `placements()` directly.
        let cycle_length = eval_len * 3.0;
        let cycle_layout = Self {
            length_m: cycle_length,
            ..self.clone()
        };
        let placements = cycle_layout.placements();

        let ldts = [ldt];

        // Shift evaluation window to the middle cycle: [eval_len, 2*eval_len] in X.
        let mut translated: Vec<LuminairePlace> = placements
            .into_iter()
            .map(|mut p| {
                p.x -= eval_len;
                p
            })
            .collect();
        // Drop placements that end up far outside the evaluation area (they
        // contribute negligible light anyway, but trimming speeds things up).
        translated.retain(|p| p.x >= -eval_len - 1.0 && p.x <= 2.0 * eval_len + 1.0);
        let translated_indices: Vec<usize> = vec![0; translated.len()];

        compute_area_illuminance_mixed(
            &ldts,
            &translated,
            &translated_indices,
            eval_len,
            eval_width,
            grid_resolution,
            maintenance_factor,
        )
    }

    /// Convert an [`AreaResult`] into a [`DesignResult`] suitable for passing
    /// to regional compliance standards.
    pub fn design_result(&self, area: &AreaResult) -> DesignResult {
        DesignResult {
            avg_illuminance_lux: area.avg_lux,
            min_illuminance_lux: area.min_lux,
            max_illuminance_lux: area.max_lux,
            avg_luminance_cd_m2: None, // luminance method deferred
            uniformity_overall: area.uniformity_min_avg,
            uniformity_longitudinal: None, // longitudinal uniformity deferred
            threshold_increment_pct: None, // TI requires luminance method
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_road() -> Eulumdat {
        let p = "../eulumdat-wasm/templates/road_luminaire.ldt";
        let content = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {p}: {e}"));
        Eulumdat::parse(&content).unwrap()
    }

    #[test]
    fn single_side_produces_one_pole_per_spacing() {
        let l = StreetLayout {
            length_m: 100.0,
            pole_spacing_m: 25.0,
            arrangement: Arrangement::SingleSide,
            ..Default::default()
        };
        // 0, 25, 50, 75, 100 → 5 poles
        assert_eq!(l.placements().len(), 5);
    }

    #[test]
    fn opposite_produces_two_poles_per_spacing() {
        let l = StreetLayout {
            length_m: 60.0,
            pole_spacing_m: 30.0,
            arrangement: Arrangement::Opposite,
            ..Default::default()
        };
        // 0, 30, 60 → 3 spacings × 2 sides = 6 poles
        assert_eq!(l.placements().len(), 6);
    }

    #[test]
    fn staggered_alternates_sides() {
        let l = StreetLayout {
            length_m: 90.0,
            pole_spacing_m: 30.0,
            arrangement: Arrangement::Staggered,
            ..Default::default()
        };
        let p = l.placements();
        // 0, 30, 60, 90 → 4 poles, alternating sides
        assert_eq!(p.len(), 4);
        // Even indices on near curb (y < road), odd on far curb.
        let road_w = l.roadway_width_m();
        assert!(p[0].y < 0.0);
        assert!(p[1].y > road_w);
        assert!(p[2].y < 0.0);
        assert!(p[3].y > road_w);
    }

    #[test]
    fn compute_returns_sensible_grid() {
        let l = StreetLayout::default();
        let ldt = load_road();
        let result = l.compute(&ldt, 1.0);
        assert!(result.avg_lux > 0.0, "average illuminance must be positive");
        assert!(result.min_lux >= 0.0);
        assert!(result.max_lux >= result.avg_lux);
        assert!(
            (0.0..=1.0).contains(&result.uniformity_min_avg),
            "uniformity must be a ratio in [0,1]: {}",
            result.uniformity_min_avg
        );
    }

    #[test]
    fn closer_pole_spacing_improves_uniformity() {
        // Tighter pole spacing → smaller gaps between pools of light →
        // higher min/avg uniformity. This is a smoke check that the pipeline
        // responds to layout changes in the expected direction.
        let ldt = load_road();
        let wide = StreetLayout {
            pole_spacing_m: 60.0,
            ..Default::default()
        };
        let tight = StreetLayout {
            pole_spacing_m: 20.0,
            ..Default::default()
        };
        let wide_u = wide.compute(&ldt, 1.0).uniformity_min_avg;
        let tight_u = tight.compute(&ldt, 1.0).uniformity_min_avg;
        assert!(
            tight_u >= wide_u,
            "tight spacing uniformity {tight_u:.3} should be ≥ wide {wide_u:.3}"
        );
    }

    #[test]
    fn design_result_carries_illuminance_metrics_only() {
        let l = StreetLayout::default();
        let ldt = load_road();
        let area = l.compute(&ldt, 1.0);
        let design = l.design_result(&area);
        assert_eq!(design.avg_illuminance_lux, area.avg_lux);
        assert!(
            design.avg_luminance_cd_m2.is_none(),
            "luminance method not implemented yet"
        );
        assert!(design.threshold_increment_pct.is_none());
    }
}
