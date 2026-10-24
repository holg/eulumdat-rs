//! Beam/field angles per section (CIE S 017 17-27-077 / IES) and the primary
//! lamp set, checked against the LumaScope comparison of 18 real files.
//!
//! Section C–(C+180): C side = +γ, C+180 side = −γ; beam (field) angle =
//! width of the lobe around the section peak at 50 % (10 %) of that peak.
//! "—" in LumaScope (lobe never drops below the threshold within the
//! section, e.g. up-lights) is `None` here.
//!
//! Only the LEDVANCE and ITALO files are committed. The 14 third-party
//! files (see `tests/fixtures/beam_field/README.md`) are git-ignored and
//! checked only when present locally; the synthetic tests at the end cover
//! the same edge cases without them.

use eulumdat::{Eulumdat, IesParser, LampSet, PhotometricCalculations, Symmetry};
use std::path::PathBuf;

const TOL_DEG: f64 = 0.1;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/beam_field")
        .join(name)
}

fn load(name: &str) -> Eulumdat {
    let path = fixture(name);
    if name.to_ascii_lowercase().ends_with(".ies") {
        IesParser::parse_file(&path).unwrap_or_else(|e| panic!("{name}: {e}"))
    } else {
        Eulumdat::from_file(&path).unwrap_or_else(|e| panic!("{name}: {e}"))
    }
}

/// A third-party fixture, or `None` (with a note) when it is not present.
fn optional(name: &str) -> Option<Eulumdat> {
    if fixture(name).exists() {
        Some(load(name))
    } else {
        eprintln!("skipping {name}: third-party fixture not present");
        None
    }
}

/// Reference values from LumaScope. LumaScope leaves C90–C270 empty for
/// rotationally symmetric data; the correct value there equals C0–C180.
struct Case {
    file: &'static str,
    /// (beam, field) of C0–C180
    c0: (Option<f64>, Option<f64>),
    /// (beam, field) of C90–C270
    c90: (Option<f64>, Option<f64>),
    /// Peak γ in whole degrees
    peak_gamma: f64,
    /// Not committed (no redistribution licence)
    third_party: bool,
}

const fn case(
    file: &'static str,
    c0: (Option<f64>, Option<f64>),
    c90: (Option<f64>, Option<f64>),
    peak_gamma: f64,
    third_party: bool,
) -> Case {
    Case {
        file,
        c0,
        c90,
        peak_gamma,
        third_party,
    }
}

const NONE: (Option<f64>, Option<f64>) = (None, None);

const CASES: &[Case] = &[
    // Committed (LEDVANCE downloads, AEC template)
    case(
        "4058075580596_FL_MAX_LUM_600W_757_SYM_30_WAL.ldt",
        (Some(33.9), Some(67.1)),
        (Some(34.4), Some(67.9)),
        2.0,
        false,
    ),
    case(
        "4058075580657_FL_MAX_LUM_900W_757_ASYM_50X110_WAL.ldt",
        (Some(27.9), Some(133.3)),
        (Some(128.1), Some(152.6)),
        55.0,
        false,
    ),
    case(
        "4058075580695_FL_MAX_LUM_1200W_757_ASYM_50X110WAL.ldt",
        (Some(46.1), Some(133.7)),
        (Some(128.0), Some(152.7)),
        55.0,
        false,
    ),
    case(
        "ITALO 1 5P5 S05 3.140-3M.IES",
        (Some(69.4), Some(145.0)),
        (Some(151.4), Some(162.4)),
        60.0,
        false,
    ),
    // Third-party, local only
    case(
        "1.IES",
        (Some(37.2), Some(148.4)),
        (Some(37.2), Some(148.4)),
        0.0,
        true,
    ),
    case(
        "2.IES",
        (Some(36.9), Some(145.1)),
        (Some(36.9), Some(145.1)),
        8.0,
        true,
    ),
    case(
        "3.IES",
        (Some(39.7), Some(129.8)),
        (Some(39.7), Some(129.8)),
        0.0,
        true,
    ),
    case(
        "4.IES",
        (Some(7.9), Some(14.6)),
        (Some(7.9), Some(14.6)),
        0.0,
        true,
    ),
    case(
        "sample1.ldt",
        (Some(95.4), Some(166.5)),
        (Some(95.2), Some(153.3)),
        0.0,
        true,
    ),
    case("sample10.ldt", NONE, NONE, 155.0, true),
    case(
        "sample2.ldt",
        (Some(34.3), Some(99.3)),
        (Some(92.5), Some(109.3)),
        20.0,
        true,
    ),
    case("sample3.ldt", NONE, NONE, 150.0, true),
    case(
        "sample4.ldt",
        (Some(75.9), Some(131.5)),
        (Some(76.3), Some(135.4)),
        20.0,
        true,
    ),
    case(
        "sample5.ldt",
        (Some(70.3), Some(106.0)),
        (Some(70.3), Some(106.0)),
        8.0,
        true,
    ),
    case(
        "sample6.ldt",
        (Some(135.2), Some(163.8)),
        (Some(37.6), Some(161.8)),
        45.0,
        true,
    ),
    case(
        "sample7.ldt",
        (Some(93.6), Some(148.3)),
        (Some(94.0), Some(148.3)),
        2.0,
        true,
    ),
    case(
        "sample8.ldt",
        (Some(30.3), Some(151.7)),
        (Some(30.2), Some(151.8)),
        67.0,
        true,
    ),
    case(
        "sample9.ldt",
        (Some(129.5), Some(233.5)),
        (Some(108.5), Some(158.5)),
        2.0,
        true,
    ),
];

/// Loaded cases: committed ones must exist, third-party ones are skipped
/// when absent.
fn loaded_cases() -> impl Iterator<Item = (&'static Case, Eulumdat)> {
    CASES.iter().filter_map(|c| {
        if c.third_party {
            optional(c.file).map(|l| (c, l))
        } else {
            Some((c, load(c.file)))
        }
    })
}

fn assert_angle(file: &str, what: &str, got: Option<f64>, want: Option<f64>) {
    match (got, want) {
        (Some(g), Some(w)) => assert!(
            (g - w).abs() <= TOL_DEG,
            "{file}: {what} = {g:.2}°, expected {w:.1}°"
        ),
        (None, None) => {}
        _ => panic!("{file}: {what} = {got:?}, expected {want:?}"),
    }
}

#[test]
fn principal_sections_match_reference() {
    for (c, ldt) in loaded_cases() {
        let s = PhotometricCalculations::beam_field_sections(&ldt);
        assert_angle(c.file, "C0–C180 beam", s.c0_c180.beam, c.c0.0);
        assert_angle(c.file, "C0–C180 field", s.c0_c180.field, c.c0.1);
        assert_angle(c.file, "C90–C270 beam", s.c90_c270.beam, c.c90.0);
        assert_angle(c.file, "C90–C270 field", s.c90_c270.field, c.c90.1);

        // The single-section helpers agree with the combined result.
        assert_eq!(
            PhotometricCalculations::beam_angle_section(&ldt, 0.0),
            s.c0_c180.beam
        );
        assert_eq!(
            PhotometricCalculations::field_angle_section(&ldt, 90.0),
            s.c90_c270.field
        );
    }
}

#[test]
fn global_peak_direction_matches_reference() {
    for (c, ldt) in loaded_cases() {
        let peak = PhotometricCalculations::peak_direction(&ldt);
        // The reference shows whole degrees (2.IES: 7.5° → "8°").
        assert!(
            (peak.gamma - c.peak_gamma).abs() <= 0.5,
            "{}: peak at γ {} expected {}",
            c.file,
            peak.gamma,
            c.peak_gamma
        );
        assert_eq!(peak.intensity, ldt.max_intensity(), "{}", c.file);

        let analysis = PhotometricCalculations::beam_field_analysis(&ldt);
        assert_eq!(analysis.max_intensity_gamma, peak.gamma, "{}", c.file);
        assert_eq!(analysis.max_intensity_c, peak.c, "{}", c.file);
    }
    // sample6: the first stored plane peaks at nadir, the global peak is at 45°.
    if let Some(ldt) = optional("sample6.ldt") {
        assert_eq!(PhotometricCalculations::peak_direction(&ldt).gamma, 45.0);
    }
}

#[test]
fn whole_luminaire_angle_uses_section_through_peak() {
    // (file, section of the whole-luminaire value, third-party)
    let expected = [
        (
            "4058075580596_FL_MAX_LUM_600W_757_SYM_30_WAL.ldt",
            0.0,
            false,
        ), // peak C180 γ2.5, centre-peaked
        (
            "4058075580657_FL_MAX_LUM_900W_757_ASYM_50X110_WAL.ldt",
            135.0,
            false,
        ), // peak C315 γ55
        (
            "4058075580695_FL_MAX_LUM_1200W_757_ASYM_50X110WAL.ldt",
            135.0,
            false,
        ),
        ("ITALO 1 5P5 S05 3.140-3M.IES", 45.0, false),
        ("1.IES", 0.0, true),       // nadir peak
        ("2.IES", 0.0, true),       // rotationally symmetric
        ("sample1.ldt", 0.0, true), // nadir peak
        ("sample9.ldt", 0.0, true), // peak at γ 2.5°, I(0) ≥ 95 % of peak
        ("sample2.ldt", 25.0, true),
        ("sample6.ldt", 85.0, true), // peak C265 γ45 → C85–C265
    ];
    for (file, c, third_party) in expected {
        let Some(ldt) = (if third_party {
            optional(file)
        } else {
            Some(load(file))
        }) else {
            continue;
        };
        let section = PhotometricCalculations::peak_section(&ldt);
        assert_eq!(section.c_plane, c, "{file}");
        assert_eq!(
            PhotometricCalculations::beam_angle(&ldt),
            section.beam,
            "{file}"
        );
        assert_eq!(
            PhotometricCalculations::field_angle(&ldt),
            section.field,
            "{file}"
        );
        if section.c_plane != 0.0 {
            // An off-axis section through the peak contains the global peak.
            let peak = PhotometricCalculations::peak_direction(&ldt);
            assert!(
                (section.peak_intensity - peak.intensity).abs() < 1e-9,
                "{file}"
            );
        }
    }
    // The 900W "ASYM 50X110" flood: 50 % beam ≈ 50° through its peak.
    let ldt = load("4058075580657_FL_MAX_LUM_900W_757_ASYM_50X110_WAL.ldt");
    let beam = PhotometricCalculations::beam_angle(&ldt).unwrap();
    assert!((beam - 50.4).abs() < 0.1, "{beam}");
}

#[test]
fn uplights_have_no_beam_angle() {
    for file in ["sample3.ldt", "sample10.ldt"] {
        let Some(ldt) = optional(file) else {
            continue;
        };
        assert_eq!(PhotometricCalculations::beam_angle(&ldt), None, "{file}");
        assert_eq!(PhotometricCalculations::field_angle(&ldt), None, "{file}");
        let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldt);
        assert!(summary.to_text().contains("Beam Angle"));
        assert!(
            !summary.to_text().contains("360.0"),
            "{file}: old 360° fallback"
        );
    }
}

#[test]
fn field_wider_than_180_is_kept() {
    // sample9: the 10 % lobe reaches past horizontal on both sides.
    let Some(ldt) = optional("sample9.ldt") else {
        return;
    };
    let s = PhotometricCalculations::section_beam_field(&ldt, 0.0);
    assert!(s.field.unwrap() > 180.0);
    let (left, right) = s.field_edges.unwrap();
    assert!(left < -90.0 && right > 90.0);
}

#[test]
fn sample1_uses_the_primary_lamp_set() {
    // Two alternative lamp sets: 1983 lm / 17.6 W and 2643 lm / 24.9 W.
    let Some(ldt) = optional("sample1.ldt") else {
        return;
    };
    assert_eq!(ldt.lamp_sets.len(), 2);
    assert!((ldt.luminous_flux() - 1983.0).abs() < 1e-9);
    assert!((ldt.wattage() - 17.6).abs() < 1e-9);
    assert!((ldt.luminous_efficacy() - 112.7).abs() < 0.05);
    assert!((ldt.sum_of_lamp_sets_flux() - 4626.0).abs() < 1e-9);

    // Peak 703 cd at γ 0° (cd/klm × primary flux / 1000).
    let peak = PhotometricCalculations::peak_direction(&ldt);
    assert_eq!(peak.gamma, 0.0);
    let peak_cd = peak.intensity * ldt.luminous_flux() / 1000.0;
    assert!((peak_cd - 703.0).abs() < 0.5, "peak {peak_cd:.1} cd");

    let summary = eulumdat::PhotometricSummary::from_eulumdat(&ldt);
    assert!((summary.total_lamp_flux - 1983.0).abs() < 1e-9);
    assert!((summary.total_wattage - 17.6).abs() < 1e-9);

    // ATLA: one emitter (primary set), alternatives kept for the round trip.
    let doc = eulumdat::atla::LuminaireOpticalData::from_eulumdat(&ldt);
    assert_eq!(doc.emitters.len(), 1);
    assert!((doc.total_luminous_flux() - 1983.0).abs() < 1e-9);
    // ATLA normalises colour labels ("nw" → "4000K"); compare the sets by
    // lamps, type, flux and power.
    let key = |ls: &LampSet| {
        (
            ls.num_lamps,
            ls.lamp_type.clone(),
            ls.total_luminous_flux,
            ls.wattage_with_ballast,
        )
    };
    let back = doc.to_eulumdat();
    assert_eq!(
        back.lamp_sets.iter().map(key).collect::<Vec<_>>(),
        ldt.lamp_sets.iter().map(key).collect::<Vec<_>>()
    );
}

#[test]
fn nema_spreads_are_measured_through_the_beam_axis() {
    // Symmetric 30° flood: field angle ≈ 67° in both sections → NEMA 4 × 4.
    let n = PhotometricCalculations::nema_classification(&load(
        "4058075580596_FL_MAX_LUM_600W_757_SYM_30_WAL.ldt",
    ));
    assert_eq!(n.designation, "NEMA 4H x 4V");
    assert!((n.horizontal_spread.unwrap() - 67.2).abs() < 0.5, "{n:?}");
    assert!((n.vertical_spread.unwrap() - 67.2).abs() < 0.5, "{n:?}");

    // Asymmetric flood: the beam axis is off the luminaire axis.
    let n = PhotometricCalculations::nema_classification(&load(
        "4058075580657_FL_MAX_LUM_900W_757_ASYM_50X110_WAL.ldt",
    ));
    assert_eq!(n.designation, "NEMA 7H x 6V");
    assert!(n.peak_v.abs() > 30.0, "{n:?}");

    // Narrow buried flood: spread equals the 14.6° field angle.
    if let Some(ldt) = optional("4.IES") {
        let n = PhotometricCalculations::nema_classification(&ldt);
        assert_eq!(n.designation, "NEMA 1H x 1V");
        assert!((n.horizontal_spread.unwrap() - 14.6).abs() < 0.1, "{n:?}");
        assert_eq!((n.peak_h, n.peak_v), (0.0, 0.0));
    }
}

// ---------------------------------------------------------------------------
// Synthetic cases (no fixture files needed)
// ---------------------------------------------------------------------------

fn gammas(step: f64) -> Vec<f64> {
    (0..=(180.0 / step) as usize)
        .map(|k| k as f64 * step)
        .collect()
}

fn rotational(g: &[f64], i: &[f64]) -> Eulumdat {
    Eulumdat {
        symmetry: Symmetry::VerticalAxis,
        num_c_planes: 1,
        num_g_planes: g.len(),
        c_angles: vec![0.0],
        g_angles: g.to_vec(),
        intensities: vec![i.to_vec()],
        ..Default::default()
    }
}

/// Four full planes C0/C90/C180/C270 (no symmetry).
fn four_planes(g: &[f64], planes: [Vec<f64>; 4]) -> Eulumdat {
    Eulumdat {
        symmetry: Symmetry::None,
        num_c_planes: 4,
        num_g_planes: g.len(),
        c_angles: vec![0.0, 90.0, 180.0, 270.0],
        g_angles: g.to_vec(),
        intensities: planes.to_vec(),
        ..Default::default()
    }
}

#[test]
fn synthetic_downlight_is_symmetric_and_interpolated() {
    // Linear fall-off: 100 → 0 over 0–90°; 50 % at 45°, 10 % at 81°.
    let g = gammas(10.0);
    let i: Vec<f64> = g
        .iter()
        .map(|&a| (100.0 - a * 100.0 / 90.0).max(0.0))
        .collect();
    let ldt = rotational(&g, &i);
    let s = PhotometricCalculations::section_beam_field(&ldt, 0.0);
    assert!((s.beam.unwrap() - 90.0).abs() < 1e-9);
    assert!((s.field.unwrap() - 162.0).abs() < 1e-9);
    assert_eq!(s.beam_edges, Some((-45.0, 45.0)));
}

#[test]
fn synthetic_field_wider_than_180() {
    // 100 → 0 over 0–150°: 50 % at 75°, 10 % at 135° → field 270°.
    let g = gammas(10.0);
    let i: Vec<f64> = g
        .iter()
        .map(|&a| (100.0 - a * 100.0 / 150.0).max(0.0))
        .collect();
    let ldt = rotational(&g, &i);
    let s = PhotometricCalculations::section_beam_field(&ldt, 0.0);
    assert!((s.beam.unwrap() - 150.0).abs() < 1e-9);
    assert!((s.field.unwrap() - 270.0).abs() < 1e-9);
    assert_eq!(s.field_edges, Some((-135.0, 135.0)));
}

#[test]
fn synthetic_uplight_is_none_not_360() {
    let g = gammas(10.0);
    let i: Vec<f64> = g
        .iter()
        .map(|&a| if a >= 120.0 { a - 100.0 } else { 0.0 })
        .collect();
    let ldt = rotational(&g, &i);
    assert_eq!(PhotometricCalculations::beam_angle(&ldt), None);
    assert_eq!(PhotometricCalculations::half_beam_angle(&ldt), None);
    assert_eq!(PhotometricCalculations::beam_angle_cie(&ldt), None);
}

#[test]
fn synthetic_upward_lobe_is_measured_across_the_zenith() {
    // Up-light peaking at the zenith, 100 → 0 over 90°: 50 % at ±45° from
    // the zenith. The old hemisphere scan stopped at γ = 180° and reported
    // half of this.
    let g = gammas(10.0);
    let i: Vec<f64> = g
        .iter()
        .map(|&a| (100.0 - (180.0 - a) * 100.0 / 90.0).max(0.0))
        .collect();
    let ldt = rotational(&g, &i);
    let up = PhotometricCalculations::upward_beam_angle(&ldt).unwrap();
    let up_field = PhotometricCalculations::upward_field_angle(&ldt).unwrap();
    assert!((up - 90.0).abs() < 1e-9, "{up}");
    assert!((up_field - 162.0).abs() < 1e-9, "{up_field}");
    assert_eq!(PhotometricCalculations::downward_beam_angle(&ldt), None);
    // There is no downward beam, so the luminaire beam angle is "—".
    assert_eq!(PhotometricCalculations::beam_angle(&ldt), None);
    let a = PhotometricCalculations::comprehensive_beam_analysis(&ldt);
    assert_eq!(a.primary_beam_angle(), Some(up));
}

#[test]
fn uplights_have_an_upward_beam() {
    for file in ["sample3.ldt", "sample10.ldt"] {
        let Some(ldt) = optional(file) else {
            continue;
        };
        let up = PhotometricCalculations::upward_beam_angle(&ldt);
        assert!(up.is_some_and(|a| a > 0.0 && a < 180.0), "{file}: {up:?}");
    }
}

#[test]
fn synthetic_global_peak_outside_the_first_plane() {
    // C0/C180/C270: nadir peak 100; C90: lobe of 200 at γ 45°.
    let g = gammas(15.0);
    let nadir: Vec<f64> = g
        .iter()
        .map(|&a| (100.0 - a * 100.0 / 90.0).max(0.0))
        .collect();
    let lobe: Vec<f64> = g
        .iter()
        .map(|&a| (200.0 - (a - 45.0).abs() * 200.0 / 45.0).max(0.0))
        .collect();
    let ldt = four_planes(&g, [nadir.clone(), lobe, nadir.clone(), nadir]);

    let peak = PhotometricCalculations::peak_direction(&ldt);
    assert_eq!((peak.c, peak.gamma, peak.intensity), (90.0, 45.0, 200.0));
    let a = PhotometricCalculations::beam_field_analysis(&ldt);
    assert_eq!((a.max_intensity_c, a.max_intensity_gamma), (90.0, 45.0));
    // Off-axis peak: the luminaire's beam is measured in C90–C270.
    assert_eq!(a.peak_section.c_plane, 90.0);
    assert_eq!(a.beam_angle_ies, a.c90_c270.beam);
    assert!(a.is_batwing);
}

#[test]
fn synthetic_near_nadir_peak_uses_c0_c180() {
    // Every plane peaks at nadir with 100 except C90, which has 101 at 15°:
    // the axis intensity is ≥ 95 % of the peak, so the distribution counts
    // as centre-peaked and C0–C180 is used.
    let g = gammas(15.0);
    let base: Vec<f64> = g
        .iter()
        .map(|&a| (100.0 - a * 100.0 / 90.0).max(0.0))
        .collect();
    let mut bump = base.clone();
    bump[1] = 101.0;
    let ldt = four_planes(&g, [base.clone(), bump, base.clone(), base]);
    assert_eq!(PhotometricCalculations::peak_direction(&ldt).gamma, 15.0);
    assert_eq!(PhotometricCalculations::peak_section_c(&ldt), 0.0);
}

#[test]
fn centre_beam_variant_is_wider_for_batwing() {
    // Batwing: nadir 60, peak 100 at 30°, falls to 0 at 90°.
    let g = [0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0];
    let i = [60.0, 75.0, 90.0, 100.0, 90.0, 70.0, 45.0, 25.0, 10.0, 0.0];
    let ldt = rotational(&g, &i);
    let peak_ref = PhotometricCalculations::beam_angle(&ldt).unwrap();
    let centre_ref = PhotometricCalculations::beam_angle_cie(&ldt).unwrap();
    assert!(
        centre_ref > peak_ref,
        "centre {centre_ref} vs peak {peak_ref}"
    );
    assert!(PhotometricCalculations::beam_field_analysis(&ldt).is_batwing);
}

#[test]
fn synthetic_alternative_lamp_sets_use_the_first() {
    let set = |lm: f64, w: f64| LampSet {
        num_lamps: 1,
        total_luminous_flux: lm,
        wattage_with_ballast: w,
        ..Default::default()
    };
    let ldt = Eulumdat {
        lamp_sets: vec![set(1000.0, 10.0), set(3000.0, 25.0)],
        ..Default::default()
    };
    assert_eq!(ldt.primary_lamp_set().unwrap().total_luminous_flux, 1000.0);
    assert_eq!(ldt.luminous_flux(), 1000.0);
    assert_eq!(ldt.wattage(), 10.0);
    assert_eq!(ldt.luminous_efficacy(), 100.0);
    assert_eq!(ldt.sum_of_lamp_sets_flux(), 4000.0);
    assert_eq!(ldt.sum_of_lamp_sets_wattage(), 35.0);
    assert_eq!(Eulumdat::default().luminous_flux(), 0.0);
}
