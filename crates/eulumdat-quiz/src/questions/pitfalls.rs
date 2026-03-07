use crate::{Category, Difficulty, Question};

/// Questions about common pitfalls, real-world issues, and practical gotchas
/// encountered when working with photometric data files.
/// Sources: bug reports, user feedback, manufacturer data inconsistencies.
pub fn questions() -> Vec<Question> {
    vec![
        // --- C-plane rotation pitfalls ---
        Question {
            id: 16001,
            category: Category::CoordinateSystems,
            difficulty: Difficulty::Expert,
            text: "A road luminaire's C0 plane points along the road axis in its LDT file, but the lighting design software assumes C0 points perpendicular to the road. What is needed?".into(),
            options: vec![
                "Convert the file from LDT to IES".into(),
                "Rotate the C-plane data by 90° (swap C0↔C90 reference)".into(),
                "Mirror the gamma angles".into(),
                "Change the symmetry type to Isym=0".into(),
            ],
            correct_index: 1,
            explanation: "The EULUMDAT spec defines C0 as 'front' but doesn't prescribe what 'front' means physically. Different manufacturers and software use different conventions — some align C0 along the road, others perpendicular. A 90° C-plane rotation resolves this mismatch without changing the photometric data.".into(),
            reference: Some("CIE 102:1993, manufacturer-dependent convention".into()),
        },
        Question {
            id: 16002,
            category: Category::CoordinateSystems,
            difficulty: Difficulty::Intermediate,
            text: "Why can two LDT files for the same physical luminaire show different polar diagrams even though both are correct?".into(),
            options: vec![
                "One file has more gamma angle resolution".into(),
                "The C0 reference direction was chosen differently by each lab".into(),
                "One uses cd/klm and the other uses cd".into(),
                "The symmetry types differ".into(),
            ],
            correct_index: 1,
            explanation: "The EULUMDAT format does not enforce a universal C0 reference direction. One lab may define C0 as the luminaire's length axis, another as the width axis. Both are valid but produce 90°-rotated polar diagrams. This is a common source of confusion when comparing files from different sources.".into(),
            reference: Some("CIE 102:1993".into()),
        },
        Question {
            id: 16003,
            category: Category::CoordinateSystems,
            difficulty: Difficulty::Expert,
            text: "When performing a 90° C-plane rotation on asymmetric photometric data, which mapping is correct?".into(),
            options: vec![
                "Add 90° to all gamma angles".into(),
                "Shift intensity data: I(C,γ) → I(C+90°,γ), wrapping at 360°".into(),
                "Swap the C0-C180 and C90-C270 curves only".into(),
                "Multiply all C-plane angles by cos(90°)".into(),
            ],
            correct_index: 1,
            explanation: "A C-plane rotation shifts the azimuthal reference. Each intensity value at angle (C,γ) moves to (C+90°,γ). For asymmetric data (Isym=0), the full dataset must be re-indexed with C-angles offset by 90° and wrapped at 360°. For symmetric data, the symmetry may need to change.".into(),
            reference: Some("CIE 102:1993".into()),
        },

        // --- Measured data vs. editable parameters ---
        Question {
            id: 16004,
            category: Category::PhotometricCalc,
            difficulty: Difficulty::Beginner,
            text: "Where do the intensity values in a photometric file come from?".into(),
            options: vec![
                "They are calculated from the luminaire's geometry".into(),
                "They are measured with a goniophotometer or integrating sphere".into(),
                "They are estimated by the manufacturer's design software".into(),
                "They are derived from the lamp specification sheet".into(),
            ],
            correct_index: 1,
            explanation: "Photometric data files contain laboratory-measured intensity distributions. A goniophotometer rotates the luminaire (or detector) through many angles and records actual light intensity. An integrating sphere measures total flux. The data represents real physical measurements, not theoretical calculations.".into(),
            reference: Some("CIE 121:1996, IESNA LM-79".into()),
        },
        Question {
            id: 16005,
            category: Category::PhotometricCalc,
            difficulty: Difficulty::Intermediate,
            text: "A user wants to edit the lumen value in a photometric file to match a different LED driver setting. Is this appropriate?".into(),
            options: vec![
                "Yes, lumen values can be freely adjusted".into(),
                "Only if the intensity distribution shape stays the same".into(),
                "No — changing lumens requires re-measurement since the distribution may change with different drive currents".into(),
                "Yes, but only in IES files, not EULUMDAT".into(),
            ],
            correct_index: 2,
            explanation: "Photometric data represents a specific operating condition. Changing the drive current affects not just total flux but potentially the intensity distribution shape, color temperature, and thermal behavior. New operating conditions require new measurements. Scaling lumens linearly is only a rough approximation.".into(),
            reference: Some("IESNA LM-79, CIE 121:1996".into()),
        },
        Question {
            id: 16006,
            category: Category::PhotometricCalc,
            difficulty: Difficulty::Expert,
            text: "Under what specific condition is it valid to linearly scale intensity values in a photometric file?".into(),
            options: vec![
                "When the luminaire uses an LED source".into(),
                "When the intensity distribution shape is independent of flux level (relative photometry, same optics)".into(),
                "Whenever the user needs a different lumen package".into(),
                "Linear scaling is never valid".into(),
            ],
            correct_index: 1,
            explanation: "Linear scaling is only valid when the distribution shape remains constant across flux levels — i.e., the optics don't change and thermal effects are negligible. In relative photometry (cd/klm), the distribution is already normalized. For absolute photometry, scaling assumes the source is a perfect dimmer, which real LEDs and lamps are not.".into(),
            reference: Some("CIE 121:1996, IESNA TM-25-13".into()),
        },

        // --- Measurement equipment ---
        Question {
            id: 16007,
            category: Category::PhotometricCalc,
            difficulty: Difficulty::Intermediate,
            text: "What is the key difference between a goniophotometer and an integrating sphere?".into(),
            options: vec![
                "Goniophotometers are more accurate".into(),
                "A goniophotometer measures directional intensity distribution; an integrating sphere measures total luminous flux".into(),
                "Integrating spheres work only with LEDs".into(),
                "Goniophotometers are used only for IES files".into(),
            ],
            correct_index: 1,
            explanation: "A goniophotometer measures intensity at many angles (C-plane × gamma) to build the full distribution used in LDT/IES files. An integrating sphere captures all emitted light to measure total luminous flux. Both may be used together: goniophotometer for distribution shape, integrating sphere for absolute flux calibration.".into(),
            reference: Some("CIE 121:1996, IESNA LM-79-19".into()),
        },

        // --- Common file quality pitfalls ---
        Question {
            id: 16008,
            category: Category::Validation,
            difficulty: Difficulty::Intermediate,
            text: "An LDT file shows LOR = 101.5%. What is the most likely cause?".into(),
            options: vec![
                "The luminaire amplifies light".into(),
                "Measurement uncertainty or rounding errors in the test lab".into(),
                "The file uses absolute photometry".into(),
                "The lamp flux was measured separately from the luminaire".into(),
            ],
            correct_index: 1,
            explanation: "LOR > 100% is physically impossible (a luminaire cannot output more light than the bare lamps). Values slightly above 100% typically result from measurement uncertainty, rounding in the file format, or differences between the lamp flux used for normalization and the actual lamp flux during luminaire testing. Good validators flag this as a warning.".into(),
            reference: Some("CIE 121:1996".into()),
        },
        Question {
            id: 16009,
            category: Category::Validation,
            difficulty: Difficulty::Expert,
            text: "An LDT file has intensity values in C0 that look like they belong in C90 (the lengthwise and crosswise distributions appear swapped). What happened?".into(),
            options: vec![
                "The file is corrupted".into(),
                "The manufacturer's goniophotometer was oriented 90° differently than the viewer expects".into(),
                "The symmetry flag is wrong".into(),
                "The file uses Type B photometry".into(),
            ],
            correct_index: 1,
            explanation: "This is the classic C0/C90 rotation issue. There is no universal standard for which physical direction C0 maps to — it depends on how the luminaire was mounted on the goniophotometer. Some labs align C0 with the luminaire's long axis, others with the short axis. The data is correct; the reference orientation differs.".into(),
            reference: Some("CIE 102:1993, real-world manufacturer variation".into()),
        },
        Question {
            id: 16010,
            category: Category::Validation,
            difficulty: Difficulty::Intermediate,
            text: "A photometric file shows 0 total luminous flux but has non-zero intensity values. What does this indicate?".into(),
            options: vec![
                "The file is corrupt".into(),
                "The luminaire emits UV light only".into(),
                "The file uses absolute photometry (negative lamp count)".into(),
                "The intensity values are in wrong units".into(),
            ],
            correct_index: 2,
            explanation: "In EULUMDAT, a negative number of lamps signals absolute photometry mode where intensity values are in candela (cd), not cd/klm. The lamp flux field may be zero since flux is implicit in the absolute intensity values. Validators should check for this before flagging zero flux as an error.".into(),
            reference: Some("EULUMDAT specification, CIE 121:1996".into()),
        },

        // --- Format interoperability pitfalls ---
        Question {
            id: 16011,
            category: Category::EulumdatFormat,
            difficulty: Difficulty::Expert,
            text: "When converting an IES file with Type A photometry to EULUMDAT, what is the main challenge?".into(),
            options: vec![
                "IES files cannot store enough decimal places".into(),
                "Type A uses a completely different angular coordinate system that requires trigonometric transformation".into(),
                "EULUMDAT cannot represent the same number of measurement points".into(),
                "The conversion is straightforward with no challenges".into(),
            ],
            correct_index: 1,
            explanation: "IES Type A photometry uses a coordinate system where the first rotation axis is horizontal (like tilting), making it incompatible with EULUMDAT's Type C system. Converting requires spherical trigonometric transformations similar to Type B→C conversion, and may introduce interpolation artifacts at the poles.".into(),
            reference: Some("ANSI/IESNA LM-63, CIE 102:1993".into()),
        },
        Question {
            id: 16012,
            category: Category::EulumdatFormat,
            difficulty: Difficulty::Intermediate,
            text: "An LDT file downloaded from a German manufacturer website uses commas as decimal separators (e.g., '123,45' instead of '123.45'). What should a parser do?".into(),
            options: vec![
                "Reject the file as invalid".into(),
                "Replace commas with dots before parsing numeric values".into(),
                "Parse only integer values and ignore decimals".into(),
                "Use the system locale to determine the separator".into(),
            ],
            correct_index: 1,
            explanation: "Many European LDT files use commas as decimal separators, reflecting local number formatting conventions. Robust parsers should convert commas to dots before parsing. Using the system locale would be unreliable since the file locale may differ from the parser's locale.".into(),
            reference: Some("EULUMDAT specification (German origin)".into()),
        },

        // --- Practical diagram interpretation ---
        Question {
            id: 16013,
            category: Category::DiagramTypes,
            difficulty: Difficulty::Intermediate,
            text: "A polar diagram shows a narrow spike at gamma=0° and nearly zero intensity elsewhere. What type of luminaire is this?".into(),
            options: vec![
                "A wall washer with wide distribution".into(),
                "A narrow-beam spotlight or projector".into(),
                "A fluorescent troffer".into(),
                "An indirect uplight".into(),
            ],
            correct_index: 1,
            explanation: "A narrow spike at nadir (γ=0°) with minimal intensity at other angles indicates a tightly focused beam — typical of spotlights, projectors, or narrow-beam downlights with reflector optics. The beam angle would be very small (< 15°).".into(),
            reference: Some("CIE S 017:2020".into()),
        },
        Question {
            id: 16014,
            category: Category::DiagramTypes,
            difficulty: Difficulty::Expert,
            text: "Two polar diagrams look identical, but one shows intensity in cd/klm and the other in cd. Can you directly compare them?".into(),
            options: vec![
                "Yes, cd and cd/klm are the same unit".into(),
                "No — cd/klm (relative) must be multiplied by total flux/1000 to convert to cd (absolute)".into(),
                "Only if both luminaires have the same LOR".into(),
                "No, they use incompatible coordinate systems".into(),
            ],
            correct_index: 1,
            explanation: "cd/klm (candelas per kilolumen) is a relative unit normalized to 1000 lumens total flux. To convert to absolute candela: I(cd) = I(cd/klm) × Φ/1000, where Φ is total luminous flux in lumens. Directly comparing cd/klm values only compares distribution shapes, not actual light levels.".into(),
            reference: Some("CIE S 017:2020, EULUMDAT specification".into()),
        },

        // --- Symmetry pitfalls ---
        Question {
            id: 16015,
            category: Category::Symmetry,
            difficulty: Difficulty::Expert,
            text: "A manufacturer declares Isym=4 (both-planes symmetry) for an LED panel, but measured C0 and C90 curves differ by 15%. What is the correct interpretation?".into(),
            options: vec![
                "The file is valid — 15% difference is within normal tolerance".into(),
                "The symmetry flag is wrong; the file should use Isym=2 or Isym=0 for accurate representation".into(),
                "The measurement equipment was miscalibrated".into(),
                "LED panels always have Isym=4 regardless of actual distribution".into(),
            ],
            correct_index: 1,
            explanation: "Isym=4 stores only quarter data (0-90°) and mirrors it to fill 360°, meaning C0 and C90 intensities are forced to be identical. A real 15% difference indicates the luminaire is not truly symmetric about both planes. Using Isym=0 or Isym=2 would preserve the actual asymmetric distribution. Forcing Isym=4 silently discards real photometric differences.".into(),
            reference: Some("EULUMDAT specification".into()),
        },
    ]
}
