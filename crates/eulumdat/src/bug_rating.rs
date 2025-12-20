//! IESNA BUG (Backlight, Uplight, Glare) Rating calculations
//!
//! Implements two IESNA classification systems:
//! - TM-15-11: BUG (Backlight, Uplight, Glare) Rating
//! - TM-15-07: Luminaire Classification System (LCS)
//!
//! # Example
//!
//! ```rust,no_run
//! use eulumdat::{Eulumdat, bug_rating::{ZoneLumens, BugRating}};
//!
//! let ldt = Eulumdat::from_file("luminaire.ldt").unwrap();
//! let zones = ZoneLumens::from_eulumdat(&ldt);
//! let rating = BugRating::from_zone_lumens(&zones);
//! println!("BUG Rating: {}", rating);
//! ```

use crate::Eulumdat;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// BUG zone lumens distribution
///
/// All angles are from nadir (0° = down, 90° = horizontal, 180° = up)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ZoneLumens {
    /// Backlight Low: 0-30°
    pub bl: f64,
    /// Backlight Mid: 30-60°
    pub bm: f64,
    /// Backlight High: 60-80°
    pub bh: f64,
    /// Backlight Very High: 80-90°
    pub bvh: f64,
    /// Forward Low: 0-30°
    pub fl: f64,
    /// Forward Mid: 30-60°
    pub fm: f64,
    /// Forward High: 60-80°
    pub fh: f64,
    /// Forward Very High: 80-90°
    pub fvh: f64,
    /// Uplight Low: 90-100°
    pub ul: f64,
    /// Uplight High: 100-180°
    pub uh: f64,
}

impl ZoneLumens {
    /// Calculate zone lumens from Eulumdat intensity data
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let mut zones = ZoneLumens::default();

        if ldt.intensities.is_empty() || ldt.g_angles.is_empty() || ldt.c_angles.is_empty() {
            return zones;
        }

        let total_flux: f64 = ldt
            .lamp_sets
            .iter()
            .map(|ls| ls.total_luminous_flux * ls.num_lamps as f64)
            .sum();

        let scale = total_flux / 1000.0;

        for (c_idx, c_angle) in ldt.c_angles.iter().enumerate() {
            if c_idx >= ldt.intensities.len() {
                continue;
            }

            let intensities = &ldt.intensities[c_idx];
            let is_forward = *c_angle <= 180.0;

            let dc = if ldt.c_angles.len() > 1 {
                let dc_before = if c_idx > 0 {
                    c_angle - ldt.c_angles[c_idx - 1]
                } else {
                    ldt.c_angles[1] - ldt.c_angles[0]
                };
                let dc_after = if c_idx < ldt.c_angles.len() - 1 {
                    ldt.c_angles[c_idx + 1] - c_angle
                } else {
                    ldt.c_angles[ldt.c_angles.len() - 1] - ldt.c_angles[ldt.c_angles.len() - 2]
                };
                (dc_before + dc_after) / 2.0
            } else {
                360.0
            };

            for (g_idx, &g_angle) in ldt.g_angles.iter().enumerate() {
                if g_idx >= intensities.len() {
                    continue;
                }

                let intensity = intensities[g_idx] * scale;

                let dg = if ldt.g_angles.len() > 1 {
                    let dg_before = if g_idx > 0 {
                        g_angle - ldt.g_angles[g_idx - 1]
                    } else {
                        ldt.g_angles[1] - ldt.g_angles[0]
                    };
                    let dg_after = if g_idx < ldt.g_angles.len() - 1 {
                        ldt.g_angles[g_idx + 1] - g_angle
                    } else {
                        ldt.g_angles[ldt.g_angles.len() - 1] - ldt.g_angles[ldt.g_angles.len() - 2]
                    };
                    (dg_before + dg_after) / 2.0
                } else {
                    5.0
                };

                let g_rad = g_angle.to_radians();
                let dg_rad = dg.to_radians();
                let dc_rad = dc.to_radians();
                let lumens = intensity * g_rad.sin() * dg_rad * dc_rad;

                if g_angle < 30.0 {
                    if is_forward {
                        zones.fl += lumens;
                    } else {
                        zones.bl += lumens;
                    }
                } else if g_angle < 60.0 {
                    if is_forward {
                        zones.fm += lumens;
                    } else {
                        zones.bm += lumens;
                    }
                } else if g_angle < 80.0 {
                    if is_forward {
                        zones.fh += lumens;
                    } else {
                        zones.bh += lumens;
                    }
                } else if g_angle <= 90.0 {
                    if is_forward {
                        zones.fvh += lumens;
                    } else {
                        zones.bvh += lumens;
                    }
                } else if g_angle <= 100.0 {
                    zones.ul += lumens;
                } else {
                    zones.uh += lumens;
                }
            }
        }

        zones
    }

    /// Total lumens across all zones
    pub fn total(&self) -> f64 {
        self.fl
            + self.fm
            + self.fh
            + self.fvh
            + self.bl
            + self.bm
            + self.bh
            + self.bvh
            + self.ul
            + self.uh
    }

    /// Forward light total
    pub fn forward_total(&self) -> f64 {
        self.fl + self.fm + self.fh + self.fvh
    }

    /// Back light total
    pub fn back_total(&self) -> f64 {
        self.bl + self.bm + self.bh + self.bvh
    }

    /// Uplight total
    pub fn uplight_total(&self) -> f64 {
        self.ul + self.uh
    }
}

/// BUG rating values (0-5 scale for each component)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BugRating {
    /// Backlight rating (0-5)
    pub b: u8,
    /// Uplight rating (0-5)
    pub u: u8,
    /// Glare rating (0-5)
    pub g: u8,
}

impl BugRating {
    /// Create a new BUG rating
    pub fn new(b: u8, u: u8, g: u8) -> Self {
        Self {
            b: b.min(5),
            u: u.min(5),
            g: g.min(5),
        }
    }

    /// Calculate BUG rating from zone lumens
    pub fn from_zone_lumens(zones: &ZoneLumens) -> Self {
        // Backlight thresholds: (BH max, BM max, BL max)
        let b_thresholds = [
            (110.0, 220.0, 110.0),
            (500.0, 1000.0, 500.0),
            (1000.0, 2500.0, 1000.0),
            (2500.0, 5000.0, 2500.0),
            (5000.0, 8500.0, 5000.0),
        ];

        let b = b_thresholds
            .iter()
            .position(|(bh_max, bm_max, bl_max)| {
                zones.bh <= *bh_max && zones.bm <= *bm_max && zones.bl <= *bl_max
            })
            .unwrap_or(5) as u8;

        // Uplight thresholds: (UL max, UH max)
        let u_thresholds = [
            (0.0, 0.0),
            (10.0, 10.0),
            (50.0, 50.0),
            (500.0, 500.0),
            (1000.0, 1000.0),
        ];

        let u = u_thresholds
            .iter()
            .position(|(ul_max, uh_max)| zones.ul <= *ul_max && zones.uh <= *uh_max)
            .unwrap_or(5) as u8;

        // Glare thresholds: (FH/BH max, FVH/BVH max)
        let g_thresholds = [
            (10.0, 10.0),
            (180.0, 10.0),
            (600.0, 50.0),
            (2000.0, 200.0),
            (4000.0, 400.0),
        ];

        let max_fh = zones.fh.max(zones.bh);
        let max_fvh = zones.fvh.max(zones.bvh);

        let g = g_thresholds
            .iter()
            .position(|(fh_max, fvh_max)| max_fh <= *fh_max && max_fvh <= *fvh_max)
            .unwrap_or(5) as u8;

        Self { b, u, g }
    }

    /// Calculate from Eulumdat directly
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let zones = ZoneLumens::from_eulumdat(ldt);
        Self::from_zone_lumens(&zones)
    }

    /// Maximum of all three ratings
    pub fn max_rating(&self) -> u8 {
        self.b.max(self.u).max(self.g)
    }
}

impl std::fmt::Display for BugRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "B{} U{} G{}", self.b, self.u, self.g)
    }
}

/// BUG diagram with SVG rendering support
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BugDiagram {
    pub zones: ZoneLumens,
    pub rating: BugRating,
    pub total_lumens: f64,
}

impl BugDiagram {
    /// Create a BUG diagram from Eulumdat data
    pub fn from_eulumdat(ldt: &Eulumdat) -> Self {
        let zones = ZoneLumens::from_eulumdat(ldt);
        let rating = BugRating::from_zone_lumens(&zones);
        let total_lumens = zones.total();
        Self {
            zones,
            rating,
            total_lumens,
        }
    }

    /// Generate SVG for TM-15-11 BUG Rating view
    pub fn to_svg(&self, width: f64, height: f64, theme: &crate::diagram::SvgTheme) -> String {
        let cx = width / 2.0;
        let cy = height / 2.0 + 20.0;
        let radius = (width.min(height) / 2.0 - 50.0).max(80.0);

        let mut svg = format!(
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">
<rect width="{}" height="{}" fill="{}"/>"#,
            width, height, width, height, theme.background
        );

        // Percentage arcs
        svg.push_str(&self.render_percentage_arcs(cx, cy, radius, theme));

        // Main circle
        svg.push_str(&format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="2"/>"#,
            cx, cy, radius, theme.grid
        ));

        // Zone markers and fills
        svg.push_str(&self.render_zone_markers(cx, cy, radius, theme));
        svg.push_str(&self.render_zone_fills(cx, cy, radius, theme));

        // Luminaire symbol
        svg.push_str(&format!(
            r#"<rect x="{}" y="{}" width="30" height="8" fill="{}" rx="2"/>
<circle cx="{}" cy="{}" r="2" fill="{}"/>"#,
            cx - 15.0,
            cy - 4.0,
            theme.text,
            cx,
            cy,
            theme.background
        ));

        // Labels
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" font-size="12" fill="{}" font-weight="600">BACK</text>
<text x="{}" y="{}" text-anchor="middle" font-size="12" fill="{}" font-weight="600">FRONT</text>"#,
            cx - radius - 40.0, cy, theme.text,
            cx + radius + 40.0, cy, theme.text
        ));

        // Nadir line
        svg.push_str(&format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" stroke-dasharray="4,4"/>
<text x="{}" y="{}" text-anchor="middle" font-size="9" fill="{}">0° (Nadir)</text>"#,
            cx, cy, cx, cy + radius + 15.0, theme.grid,
            cx, cy + radius + 28.0, theme.text_secondary
        ));

        // Rating display
        svg.push_str(&format!(
            r#"<text x="{}" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="{}">BUG Rating: {}</text>"#,
            cx, theme.text, self.rating
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate SVG with detailed zone lumens breakdown.
    ///
    /// Includes a table showing exact lumen values for each BUG zone.
    pub fn to_svg_with_details(
        &self,
        width: f64,
        height: f64,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        let cx = width * 0.35;
        let cy = height / 2.0 + 20.0;
        let radius = (width.min(height) / 2.0 - 70.0).max(70.0);
        let table_x = width * 0.62;

        let mut svg = format!(
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">
<rect width="{}" height="{}" fill="{}"/>"#,
            width, height, width, height, theme.background
        );

        // Percentage arcs
        svg.push_str(&self.render_percentage_arcs(cx, cy, radius, theme));

        // Main circle
        svg.push_str(&format!(
            r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="2"/>"#,
            cx, cy, radius, theme.grid
        ));

        // Zone markers and fills
        svg.push_str(&self.render_zone_markers(cx, cy, radius, theme));
        svg.push_str(&self.render_zone_fills(cx, cy, radius, theme));

        // Luminaire symbol
        svg.push_str(&format!(
            r#"<rect x="{}" y="{}" width="30" height="8" fill="{}" rx="2"/>
<circle cx="{}" cy="{}" r="2" fill="{}"/>"#,
            cx - 15.0,
            cy - 4.0,
            theme.text,
            cx,
            cy,
            theme.background
        ));

        // Rating display
        svg.push_str(&format!(
            r#"<text x="{}" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="{}">BUG Rating: {}</text>"#,
            width / 2.0, theme.text, self.rating
        ));

        // === ZONE LUMENS TABLE ===
        svg.push_str(&format!(
            r#"<text x="{}" y="55" font-size="11" font-weight="bold" fill="{}">Zone Lumens (lm)</text>"#,
            table_x, theme.text
        ));

        // Table headers
        let col1 = table_x;
        let col2 = table_x + 45.0;
        let col3 = table_x + 100.0;
        let mut y = 75.0;
        let row_h = 18.0;

        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{}">Zone</text>
<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{}">Forward</text>
<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{}">Back</text>"#,
            col1,
            y,
            theme.text_secondary,
            col2,
            y,
            theme.text_secondary,
            col3,
            y,
            theme.text_secondary
        ));
        y += row_h;

        // Zone rows
        let rows = [
            ("VH (80-90°)", self.zones.fvh, self.zones.bvh, "U"),
            ("H (60-80°)", self.zones.fh, self.zones.bh, "G"),
            ("M (30-60°)", self.zones.fm, self.zones.bm, "G"),
            ("L (0-30°)", self.zones.fl, self.zones.bl, "B"),
        ];

        for (label, fwd, back, category) in rows {
            // Row background for visual grouping
            let bg_color = match category {
                "U" => "rgba(239,68,68,0.1)",  // Red for uplight
                "G" => "rgba(245,158,11,0.1)", // Orange for glare
                _ => "rgba(59,130,246,0.1)",   // Blue for backlight
            };
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="140" height="{}" fill="{}" rx="2"/>"#,
                col1 - 5.0,
                y - 12.0,
                row_h,
                bg_color
            ));

            svg.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="9" fill="{}">{}</text>
<text x="{}" y="{}" font-size="9" fill="{}">{:.0}</text>
<text x="{}" y="{}" font-size="9" fill="{}">{:.0}</text>"#,
                col1, y, theme.text, label, col2, y, theme.text, fwd, col3, y, theme.text, back
            ));
            y += row_h;
        }

        // Totals
        y += 5.0;
        svg.push_str(&format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1"/>"#,
            col1 - 5.0,
            y - 10.0,
            col1 + 135.0,
            y - 10.0,
            theme.grid
        ));

        let fwd_total = self.zones.fl + self.zones.fm + self.zones.fh + self.zones.fvh;
        let back_total = self.zones.bl + self.zones.bm + self.zones.bh + self.zones.bvh;

        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{}">Total</text>
<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{}">{:.0}</text>
<text x="{}" y="{}" font-size="9" font-weight="bold" fill="{}">{:.0}</text>"#,
            col1, y, theme.text, col2, y, theme.text, fwd_total, col3, y, theme.text, back_total
        ));

        // Uplight total
        y += row_h + 5.0;
        let uplight = self.zones.fvh + self.zones.bvh;
        let uplight_pct = if self.total_lumens > 0.0 {
            uplight / self.total_lumens * 100.0
        } else {
            0.0
        };
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}">Uplight: {:.0} lm ({:.1}%)</text>"#,
            col1, y, theme.text_secondary, uplight, uplight_pct
        ));

        // B, U, G breakdown
        y += row_h;
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}">B{} (back: {:.0}lm)</text>"#,
            col1, y, theme.text_secondary, self.rating.b, back_total
        ));
        y += row_h - 4.0;
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}">U{} (up: {:.0}lm)</text>"#,
            col1, y, theme.text_secondary, self.rating.u, uplight
        ));
        y += row_h - 4.0;
        let glare = self.zones.fh + self.zones.fm + self.zones.bh + self.zones.bm;
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}">G{} (glare: {:.0}lm)</text>"#,
            col1, y, theme.text_secondary, self.rating.g, glare
        ));

        svg.push_str("</svg>");
        svg
    }

    /// Generate SVG for TM-15-07 LCS view
    pub fn to_lcs_svg(&self, width: f64, height: f64, theme: &crate::diagram::SvgTheme) -> String {
        let cx = width * 0.3;
        let cy = height * 0.55;
        let radius = (width.min(height) * 0.4).max(100.0);
        let table_x = width * 0.6;

        let mut svg = format!(
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">
<rect width="{}" height="{}" fill="{}"/>"#,
            width, height, width, height, theme.background
        );

        // Zone wedges
        svg.push_str(&self.render_lcs_wedges(cx, cy, radius, theme));

        // Concentric circles
        for ratio in [1.0, 0.75, 0.5, 0.25] {
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width=".5"/>"#,
                cx,
                cy,
                radius * ratio,
                theme.axis
            ));
        }

        // Radial lines
        svg.push_str(&self.render_lcs_radial_lines(cx, cy, radius, theme));

        // Labels
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" font-size="9" fill="{}">BACK</text>
<text x="{}" y="{}" text-anchor="middle" font-size="9" fill="{}">FRONT</text>"#,
            cx - radius / 2.0,
            cy - 20.0,
            theme.text_secondary,
            cx + radius / 2.0,
            cy - 20.0,
            theme.text_secondary
        ));

        // Rating
        svg.push_str(&format!(
            r#"<text x="{}" y="25" font-size="11" fill="{}">Rating: {}</text>"#,
            table_x, theme.text_secondary, self.rating
        ));

        // Tables
        svg.push_str(&self.render_lcs_tables(table_x, 50.0, theme));

        svg.push_str("</svg>");
        svg
    }

    fn render_percentage_arcs(
        &self,
        cx: f64,
        cy: f64,
        radius: f64,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        if self.total_lumens <= 0.0 {
            return String::new();
        }

        let mut result = String::new();
        let arc_radius = radius + 25.0;
        let arc_width = 15.0;

        // Forward zones
        let forward_zones = [
            (self.zones.fvh, &theme.curve_c90_c270),
            (self.zones.fh, &theme.curve_c90_c270),
            (self.zones.fm, &theme.curve_c0_c180),
            (self.zones.fl, &theme.curve_c0_c180),
        ];
        result.push_str(&self.render_side_arc(
            cx,
            cy,
            arc_radius,
            arc_width,
            &forward_zones,
            true,
            theme,
        ));

        // Back zones
        let back_zones = [
            (self.zones.bvh, &theme.curve_c90_c270),
            (self.zones.bh, &theme.curve_c90_c270),
            (self.zones.bm, &theme.curve_c0_c180),
            (self.zones.bl, &theme.curve_c0_c180),
        ];
        result.push_str(&self.render_side_arc(
            cx,
            cy,
            arc_radius,
            arc_width,
            &back_zones,
            false,
            theme,
        ));

        result
    }

    #[allow(clippy::too_many_arguments)]
    fn render_side_arc(
        &self,
        cx: f64,
        cy: f64,
        radius: f64,
        width: f64,
        zones: &[(f64, &String); 4],
        is_right: bool,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        let mut result = String::new();
        let mut cumulative_percent = 0.0;

        for (lumens, color) in zones.iter() {
            if *lumens <= 0.0 {
                continue;
            }

            let percent = lumens / self.total_lumens * 100.0;
            let start_angle = cumulative_percent / 100.0 * 90.0;
            let end_angle = (cumulative_percent + percent) / 100.0 * 90.0;
            cumulative_percent += percent;

            let start_rad = start_angle.to_radians();
            let end_rad = end_angle.to_radians();
            let inner_r = radius;
            let outer_r = radius + width;
            let dir = if is_right { 1.0 } else { -1.0 };

            let x1 = cx + dir * inner_r * end_rad.cos();
            let y1 = cy + inner_r * end_rad.sin();
            let x2 = cx + dir * outer_r * end_rad.cos();
            let y2 = cy + outer_r * end_rad.sin();
            let x3 = cx + dir * outer_r * start_rad.cos();
            let y3 = cy + outer_r * start_rad.sin();
            let x4 = cx + dir * inner_r * start_rad.cos();
            let y4 = cy + inner_r * start_rad.sin();

            let sweep = if is_right { 0 } else { 1 };

            result.push_str(&format!(
                r#"<path d="M {} {} A {} {} 0 0 {} {} {} L {} {} A {} {} 0 0 {} {} {} Z" fill="{}" opacity="0.7" stroke="{}" stroke-width="1"/>"#,
                x1, y1, inner_r, inner_r, sweep, x4, y4,
                x3, y3, outer_r, outer_r, 1 - sweep, x2, y2,
                color, theme.background
            ));
        }

        result
    }

    fn render_zone_markers(
        &self,
        cx: f64,
        cy: f64,
        radius: f64,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        let angles: [f64; 5] = [30.0, 60.0, 80.0, 90.0, 100.0];
        let labels = ["30°", "60°", "80°", "90°", "100°"];
        let mut result = String::new();

        for (angle, label) in angles.iter().zip(labels.iter()) {
            let rad = angle.to_radians();
            let x1 = cx + radius * rad.sin();
            let y1 = cy + radius * rad.cos();
            let x2 = cx - radius * rad.sin();

            let (y1_adj, _y2_adj) = if *angle > 90.0 {
                let adj = cy - radius * (angle - 90.0_f64).to_radians().sin();
                (adj, adj)
            } else {
                (y1, y1)
            };

            result.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" stroke-dasharray="2,2"/>
<text x="{}" y="{}" font-size="9" fill="{}">{}</text>"#,
                x1, y1_adj, x2, y1_adj, theme.grid,
                cx + radius + 15.0, y1_adj + 4.0, theme.text, label
            ));
        }

        result
    }

    fn render_zone_fills(
        &self,
        cx: f64,
        cy: f64,
        radius: f64,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        let zone_data: [(f64, f64, f64, &str, bool); 8] = [
            (0.0, 30.0, self.zones.fl, "FL", true),
            (30.0, 60.0, self.zones.fm, "FM", true),
            (60.0, 80.0, self.zones.fh, "FH", true),
            (80.0, 90.0, self.zones.fvh, "FVH", true),
            (0.0, 30.0, self.zones.bl, "BL", false),
            (30.0, 60.0, self.zones.bm, "BM", false),
            (60.0, 80.0, self.zones.bh, "BH", false),
            (80.0, 90.0, self.zones.bvh, "BVH", false),
        ];

        let mut result = String::new();

        for (start, end, lumens, label, is_forward) in zone_data.iter() {
            let start_rad = (*start).to_radians();
            let end_rad = (*end).to_radians();
            let inner_r = radius * 0.3;
            let outer_r = radius * 0.95;

            let sx = if *is_forward { 1.0 } else { -1.0 };

            let x1 = cx + sx * inner_r * start_rad.sin();
            let y1 = cy + inner_r * start_rad.cos();
            let x2 = cx + sx * outer_r * start_rad.sin();
            let y2 = cy + outer_r * start_rad.cos();
            let x3 = cx + sx * outer_r * end_rad.sin();
            let y3 = cy + outer_r * end_rad.cos();
            let x4 = cx + sx * inner_r * end_rad.sin();
            let y4 = cy + inner_r * end_rad.cos();

            let intensity = (*lumens / 1000.0).min(1.0);
            let opacity = 0.2 + intensity * 0.6;
            let fill_color = if *lumens > 100.0 {
                &theme.curve_c90_c270
            } else {
                &theme.curve_c0_c180
            };

            let mid_angle = (start + end) / 2.0;
            let mid_r = (inner_r + outer_r) / 2.0;
            let label_x = cx + sx * mid_r * mid_angle.to_radians().sin();
            let label_y = cy + mid_r * mid_angle.to_radians().cos();

            let sweep_outer = if *is_forward { 1 } else { 0 };
            let sweep_inner = if *is_forward { 0 } else { 1 };

            result.push_str(&format!(
                r#"<path d="M {} {} L {} {} A {} {} 0 0 {} {} {} L {} {} A {} {} 0 0 {} {} {} Z" fill="{}" stroke="{}" stroke-width="0.5" opacity="{}"/>
<text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" font-size="8" fill="{}">{}: {:.0}</text>"#,
                x1, y1, x2, y2,
                outer_r, outer_r, sweep_outer, x3, y3,
                x4, y4,
                inner_r, inner_r, sweep_inner, x1, y1,
                fill_color, theme.grid, opacity,
                label_x, label_y, theme.text, label, lumens
            ));
        }

        // Uplight zones
        result.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" font-size="8" fill="{}">UL: {:.0} lm</text>
<text x="{}" y="{}" text-anchor="middle" font-size="8" fill="{}">UH: {:.0} lm</text>"#,
            cx,
            cy - radius * 0.15 - 10.0,
            theme.text,
            self.zones.ul,
            cx,
            cy - radius - 15.0,
            theme.text,
            self.zones.uh
        ));

        result
    }

    fn render_lcs_wedges(
        &self,
        cx: f64,
        cy: f64,
        max_radius: f64,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        if self.total_lumens <= 0.0 {
            return String::new();
        }

        let zone_percents = [
            self.zones.fl,
            self.zones.fm,
            self.zones.fh,
            self.zones.fvh,
            self.zones.bl,
            self.zones.bm,
            self.zones.bh,
            self.zones.bvh,
        ];
        let max_percent = zone_percents
            .iter()
            .copied()
            .fold(0.0_f64, f64::max)
            .max(0.01)
            / self.total_lumens;
        let scale = max_radius / max_percent;

        let zone_data: [(f64, f64, f64, f64, bool); 8] = [
            (0.0, 10.0, self.zones.fvh, 0.3137, true),
            (10.0, 30.0, self.zones.fh, 0.4706, true),
            (30.0, 60.0, self.zones.fm, 0.7059, true),
            (60.0, 90.0, self.zones.fl, 0.8627, true),
            (0.0, 10.0, self.zones.bvh, 0.3137, false),
            (10.0, 30.0, self.zones.bh, 0.4706, false),
            (30.0, 60.0, self.zones.bm, 0.7059, false),
            (60.0, 90.0, self.zones.bl, 0.8627, false),
        ];

        let mut result = String::new();

        for (start, end, lumens, opacity, is_forward) in zone_data.iter() {
            let percent = lumens / self.total_lumens;
            let zone_radius = (percent * scale).min(max_radius);

            if zone_radius < 0.5 {
                continue;
            }

            let start_rad = start.to_radians();
            let end_rad = end.to_radians();
            let dir = if *is_forward { 1.0 } else { -1.0 };

            let x1 = cx + dir * zone_radius * start_rad.cos();
            let y1 = cy + zone_radius * start_rad.sin();
            let x2 = cx + dir * zone_radius * end_rad.cos();
            let y2 = cy + zone_radius * end_rad.sin();

            let sweep = if *is_forward { 1 } else { 0 };
            let color = if *is_forward {
                &theme.curve_c0_c180
            } else {
                &theme.curve_c90_c270
            };

            result.push_str(&format!(
                r#"<path d="M {} {} L {} {} A {} {} 0 0 {} {} {} Z" fill="{}" fill-opacity="{}" stroke="{}" stroke-width=".5"/>"#,
                cx, cy, x1, y1, zone_radius, zone_radius, sweep, x2, y2,
                color, opacity, theme.axis
            ));
        }

        result
    }

    fn render_lcs_radial_lines(
        &self,
        cx: f64,
        cy: f64,
        radius: f64,
        theme: &crate::diagram::SvgTheme,
    ) -> String {
        let zone_angles: [f64; 3] = [10.0, 30.0, 60.0];
        let mut result = String::new();

        // Horizontal and vertical axes
        result.push_str(&format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width=".5"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width=".5"/>"#,
            cx - radius,
            cy,
            cx + radius,
            cy,
            theme.axis,
            cx,
            cy,
            cx,
            cy + radius + 35.0,
            theme.axis
        ));

        // Zone boundary lines
        for angle in zone_angles {
            let rad = angle.to_radians();
            let x_off = radius * rad.cos();
            let y_off = radius * rad.sin();

            result.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width=".5"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width=".5"/>"#,
                cx,
                cy,
                cx + x_off,
                cy + y_off,
                theme.axis,
                cx,
                cy,
                cx - x_off,
                cy + y_off,
                theme.axis
            ));
        }

        result
    }

    fn render_lcs_tables(&self, x: f64, y: f64, theme: &crate::diagram::SvgTheme) -> String {
        let lh = 18.0;
        let col_lumens = x + 100.0;
        let col_percent = x + 150.0;

        let format_lumens = |v: f64| -> String {
            if v >= 1000.0 {
                format!("{:.0}", v)
            } else if v >= 100.0 {
                format!("{:.1}", v)
            } else {
                format!("{:.2}", v)
            }
        };

        let format_percent = |v: f64| -> String {
            if self.total_lumens <= 0.0 {
                return "0%".to_string();
            }
            let p = v / self.total_lumens * 100.0;
            format!("{:.2}%", p)
        };

        let mut result = String::new();
        let mut row = 0;

        // Forward Light
        result.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}" font-weight="bold">{}</text>"#,
            x,
            y + lh * row as f64,
            theme.text_secondary,
            theme.labels.bug_forward_light
        ));
        row += 1;
        for (label, value) in [
            (
                format!("{} (0-30°)", theme.labels.bug_zone_low),
                self.zones.fl,
            ),
            (
                format!("{} (30-60°)", theme.labels.bug_zone_medium),
                self.zones.fm,
            ),
            (
                format!("{} (60-80°)", theme.labels.bug_zone_high),
                self.zones.fh,
            ),
            (
                format!("{} (80-90°)", theme.labels.bug_zone_very_high),
                self.zones.fvh,
            ),
        ] {
            result.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text>"#,
                x, y + lh * row as f64, theme.text_secondary, label,
                col_lumens, y + lh * row as f64, theme.text_secondary, format_lumens(value),
                col_percent, y + lh * row as f64, theme.text_secondary, format_percent(value)
            ));
            row += 1;
        }

        // Back Light
        result.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}" font-weight="bold">{}</text>"#,
            x,
            y + lh * row as f64,
            theme.text_secondary,
            theme.labels.bug_back_light
        ));
        row += 1;
        for (label, value) in [
            (
                format!("{} (0-30°)", theme.labels.bug_zone_low),
                self.zones.bl,
            ),
            (
                format!("{} (30-60°)", theme.labels.bug_zone_medium),
                self.zones.bm,
            ),
            (
                format!("{} (60-80°)", theme.labels.bug_zone_high),
                self.zones.bh,
            ),
            (
                format!("{} (80-90°)", theme.labels.bug_zone_very_high),
                self.zones.bvh,
            ),
        ] {
            result.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text>"#,
                x, y + lh * row as f64, theme.text_secondary, label,
                col_lumens, y + lh * row as f64, theme.text_secondary, format_lumens(value),
                col_percent, y + lh * row as f64, theme.text_secondary, format_percent(value)
            ));
            row += 1;
        }

        // Uplight
        result.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}" font-weight="bold">{}</text>"#,
            x,
            y + lh * row as f64,
            theme.text_secondary,
            theme.labels.bug_uplight
        ));
        row += 1;
        for (label, value) in [
            (
                format!("{} (90-100°)", theme.labels.bug_zone_low),
                self.zones.ul,
            ),
            (
                format!("{} (100-180°)", theme.labels.bug_zone_high),
                self.zones.uh,
            ),
        ] {
            result.push_str(&format!(
                r#"<text x="{}" y="{}" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text>"#,
                x, y + lh * row as f64, theme.text_secondary, label,
                col_lumens, y + lh * row as f64, theme.text_secondary, format_lumens(value),
                col_percent, y + lh * row as f64, theme.text_secondary, format_percent(value)
            ));
            row += 1;
        }

        // Total
        result.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}" font-weight="bold">{}</text>"#,
            x,
            y + lh * row as f64,
            theme.text_secondary,
            theme.labels.bug_total
        ));
        row += 1;
        result.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">{}</text><text x="{}" y="{}" text-anchor="end" font-size="9" fill="{}">100%</text>"#,
            x, y + lh * row as f64, theme.text_secondary, theme.labels.bug_sum,
            col_lumens, y + lh * row as f64, theme.text_secondary, format_lumens(self.total_lumens),
            col_percent, y + lh * row as f64, theme.text_secondary
        ));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bug_rating_display() {
        let rating = BugRating::new(2, 1, 3);
        assert_eq!(rating.to_string(), "B2 U1 G3");
    }

    #[test]
    fn test_bug_rating_max() {
        let rating = BugRating::new(2, 4, 3);
        assert_eq!(rating.max_rating(), 4);
    }

    #[test]
    fn test_zone_lumens_totals() {
        let zones = ZoneLumens {
            fl: 100.0,
            fm: 200.0,
            fh: 50.0,
            fvh: 10.0,
            bl: 80.0,
            bm: 150.0,
            bh: 40.0,
            bvh: 8.0,
            ul: 5.0,
            uh: 2.0,
        };
        assert!((zones.forward_total() - 360.0).abs() < 0.01);
        assert!((zones.back_total() - 278.0).abs() < 0.01);
        assert!((zones.uplight_total() - 7.0).abs() < 0.01);
        assert!((zones.total() - 645.0).abs() < 0.01);
    }

    #[test]
    fn test_bug_rating_from_zones() {
        // Low lumens should give low ratings
        let zones = ZoneLumens {
            fl: 10.0,
            fm: 10.0,
            fh: 5.0,
            fvh: 1.0,
            bl: 10.0,
            bm: 10.0,
            bh: 5.0,
            bvh: 1.0,
            ul: 0.0,
            uh: 0.0,
        };
        let rating = BugRating::from_zone_lumens(&zones);
        assert_eq!(rating.b, 0);
        assert_eq!(rating.u, 0);
        assert_eq!(rating.g, 0);
    }
}
