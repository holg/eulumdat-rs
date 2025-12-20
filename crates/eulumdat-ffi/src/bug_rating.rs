//! BUG rating types and functions for FFI

use crate::diagram::{Language, SvgThemeType};
use crate::types::{to_core_eulumdat, Eulumdat};

/// Zone lumens data for BUG rating
#[derive(Debug, Clone, uniffi::Record)]
pub struct ZoneLumens {
    pub bl: f64,  // Backlight Low
    pub bm: f64,  // Backlight Mid
    pub bh: f64,  // Backlight High
    pub bvh: f64, // Backlight Very High
    pub fl: f64,  // Forward Low
    pub fm: f64,  // Forward Mid
    pub fh: f64,  // Forward High
    pub fvh: f64, // Forward Very High
    pub ul: f64,  // Uplight Low
    pub uh: f64,  // Uplight High
}

impl From<eulumdat::ZoneLumens> for ZoneLumens {
    fn from(z: eulumdat::ZoneLumens) -> Self {
        Self {
            bl: z.bl,
            bm: z.bm,
            bh: z.bh,
            bvh: z.bvh,
            fl: z.fl,
            fm: z.fm,
            fh: z.fh,
            fvh: z.fvh,
            ul: z.ul,
            uh: z.uh,
        }
    }
}

/// BUG rating values (0-5 scale)
#[derive(Debug, Clone, uniffi::Record)]
pub struct BugRatingData {
    pub b: u8,
    pub u: u8,
    pub g: u8,
}

impl From<eulumdat::BugRating> for BugRatingData {
    fn from(r: eulumdat::BugRating) -> Self {
        Self {
            b: r.b,
            u: r.u,
            g: r.g,
        }
    }
}

/// Complete BUG diagram data
#[derive(Debug, Clone, uniffi::Record)]
pub struct BugDiagramData {
    pub zones: ZoneLumens,
    pub rating: BugRatingData,
    pub total_lumens: f64,
}

/// Calculate BUG rating from Eulumdat data
#[uniffi::export]
pub fn calculate_bug_rating(ldt: &Eulumdat) -> BugRatingData {
    let core_ldt = to_core_eulumdat(ldt);
    eulumdat::BugRating::from_eulumdat(&core_ldt).into()
}

/// Generate BUG diagram data
#[uniffi::export]
pub fn generate_bug_diagram(ldt: &Eulumdat) -> BugDiagramData {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    BugDiagramData {
        zones: diagram.zones.into(),
        rating: diagram.rating.into(),
        total_lumens: diagram.total_lumens,
    }
}

/// Generate BUG rating diagram as SVG (TM-15-11 view)
#[uniffi::export]
pub fn generate_bug_svg(ldt: &Eulumdat, width: f64, height: f64, theme: SvgThemeType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    diagram.to_svg(width, height, &theme.to_core())
}

/// Generate BUG rating diagram as SVG with localized labels (TM-15-11 view)
#[uniffi::export]
pub fn generate_bug_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    diagram.to_svg(width, height, &theme.to_core_with_locale(&locale))
}

/// Generate LCS diagram as SVG (TM-15-07 view)
#[uniffi::export]
pub fn generate_lcs_svg(ldt: &Eulumdat, width: f64, height: f64, theme: SvgThemeType) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    diagram.to_lcs_svg(width, height, &theme.to_core())
}

/// Generate LCS diagram as SVG with localized labels (TM-15-07 view)
#[uniffi::export]
pub fn generate_lcs_svg_localized(
    ldt: &Eulumdat,
    width: f64,
    height: f64,
    theme: SvgThemeType,
    language: Language,
) -> String {
    let core_ldt = to_core_eulumdat(ldt);
    let diagram = eulumdat::BugDiagram::from_eulumdat(&core_ldt);
    let locale = language.to_locale();
    diagram.to_lcs_svg(width, height, &theme.to_core_with_locale(&locale))
}
