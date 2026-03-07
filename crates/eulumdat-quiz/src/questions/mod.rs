mod bim;
mod bug_rating;
mod calculations;
mod color_science;
mod coordinates;
mod diagram_reading;
mod diagrams;
mod eulumdat_format;
mod horticultural;
mod ies_format;
mod modern_formats;
mod pitfalls;
mod standards;
mod symmetry;
mod ugr_glare;
mod units;
mod validation;

use crate::Question;

/// Collect all questions from every category module.
pub fn all_questions() -> Vec<Question> {
    let mut qs = Vec::with_capacity(250);
    qs.extend(eulumdat_format::questions());
    qs.extend(ies_format::questions());
    qs.extend(symmetry::questions());
    qs.extend(coordinates::questions());
    qs.extend(calculations::questions());
    qs.extend(bug_rating::questions());
    qs.extend(ugr_glare::questions());
    qs.extend(color_science::questions());
    qs.extend(horticultural::questions());
    qs.extend(bim::questions());
    qs.extend(modern_formats::questions());
    qs.extend(validation::questions());
    qs.extend(units::questions());
    qs.extend(diagrams::questions());
    qs.extend(diagram_reading::questions());
    qs.extend(standards::questions());
    qs.extend(pitfalls::questions());
    qs
}
