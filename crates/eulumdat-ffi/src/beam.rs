//! Beam/field angles and primary lamp-set values.
//!
//! Beam (field) angle = full width at 50 % (10 %) of the peak intensity in a
//! vertical section C–(C+180) (CIE S 017 17-27-077 / IES). Angles are
//! optional: `None` when the lobe does not drop below the threshold on both
//! sides of the section (e.g. up-lights).

use eulumdat::{PhotometricCalculations, SectionBeamField};

use crate::types::{to_core_eulumdat, Eulumdat};

/// Beam and field angle of one vertical section.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SectionAngles {
    /// Section label, e.g. "C0–C180"
    pub section: String,
    /// C angle of the positive side of the section (degrees)
    pub c_plane: f64,
    /// Full width at 50 % of the section peak (degrees)
    pub beam: Option<f64>,
    /// Full width at 10 % of the section peak (degrees)
    pub field: Option<f64>,
    /// Peak intensity within the section (cd/klm)
    pub peak_intensity: f64,
}

impl From<&SectionBeamField> for SectionAngles {
    fn from(s: &SectionBeamField) -> Self {
        Self {
            section: s.label(),
            c_plane: s.c_plane,
            beam: s.beam,
            field: s.field,
            peak_intensity: s.peak_intensity,
        }
    }
}

/// Beam/field analysis of a luminaire.
#[derive(Debug, Clone, uniffi::Record)]
pub struct BeamFieldResult {
    /// Section through the peak direction (C0–C180 for a nadir peak); its
    /// beam/field are the luminaire's beam/field angle
    pub peak_section: SectionAngles,
    /// Section C0–C180
    pub c0_c180: SectionAngles,
    /// Section C90–C270
    pub c90_c270: SectionAngles,
    /// Centre-beam variant: 50 % of I(γ = 0°), C0–C180 (degrees)
    pub beam_angle_centre: Option<f64>,
    /// Centre-beam variant: 10 % of I(γ = 0°), C0–C180 (degrees)
    pub field_angle_centre: Option<f64>,
    /// C angle of the global peak (degrees)
    pub peak_c: f64,
    /// γ angle of the global peak (degrees)
    pub peak_gamma: f64,
    /// Global peak intensity (cd/klm)
    pub peak_intensity: f64,
    /// Lamp flux of the primary (first) lamp set (lm); converts cd/klm to cd
    pub luminous_flux: f64,
    /// Wattage of the primary (first) lamp set (W)
    pub wattage: f64,
}

/// Beam/field angles of the luminaire, per section, and its peak direction.
#[uniffi::export]
pub fn beam_field_analysis(ldt: &Eulumdat) -> BeamFieldResult {
    let core = to_core_eulumdat(ldt);
    let a = PhotometricCalculations::beam_field_analysis(&core);
    BeamFieldResult {
        peak_section: (&a.peak_section).into(),
        c0_c180: (&a.c0_c180).into(),
        c90_c270: (&a.c90_c270).into(),
        beam_angle_centre: a.beam_angle_cie,
        field_angle_centre: a.field_angle_cie,
        peak_c: a.max_intensity_c,
        peak_gamma: a.max_intensity_gamma,
        peak_intensity: a.max_intensity,
        luminous_flux: core.luminous_flux(),
        wattage: core.wattage(),
    }
}

/// Beam and field angle of the section C–(C+180), e.g. `c_plane = 90` for
/// C90–C270.
#[uniffi::export]
pub fn section_angles(ldt: &Eulumdat, c_plane: f64) -> SectionAngles {
    let core = to_core_eulumdat(ldt);
    (&PhotometricCalculations::section_beam_field(&core, c_plane)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uplight_has_no_beam() {
        let core = eulumdat::Eulumdat {
            symmetry: eulumdat::Symmetry::VerticalAxis,
            c_angles: vec![0.0],
            g_angles: vec![0.0, 60.0, 120.0, 180.0],
            intensities: vec![vec![0.0, 0.0, 50.0, 100.0]],
            ..Default::default()
        };
        let r = beam_field_analysis(&Eulumdat::from(&core));
        assert_eq!(r.peak_section.beam, None);
        assert_eq!(r.peak_gamma, 180.0);
    }
}
