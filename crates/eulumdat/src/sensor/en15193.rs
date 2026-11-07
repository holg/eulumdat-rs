//! EN 15193-1:2017+A1:2021, Annex E: occupancy dependency factor F_O, the
//! part of the lighting energy that presence control saves.
//!
//! ```text
//! F_O = 1 − (1 − F_OC)·F_A / 0.2     0   ≤ F_A < 0.2
//! F_O = F_OC + 0.2 − F_A              0.2 ≤ F_A < 0.9
//! F_O = (7 − 10·F_OC)·(F_A − 1)       0.9 ≤ F_A ≤ 1
//! ```
//!
//! F_OC comes from the control type, F_A (absence factor) from the space.
//! Credit for automatic control needs: lights off (or ≤ 20 %) within 15 min
//! of the last presence, a control group of ≤ 30 m² (meeting rooms
//! excepted), and detection that matches the lit area, which the coverage
//! check supplies. Daylight (F_D) and constant illuminance (F_C) are not
//! part of this estimate.

/// How the lighting is switched (Annex E table of F_OC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccupancyControl {
    ManualOnOff,
    ManualSweepOff,
    AutoOnDimmed,
    AutoOnAutoOff,
    ManualOnDimmed,
    ManualOnAutoOff,
}

impl OccupancyControl {
    pub const ALL: [OccupancyControl; 6] = [
        OccupancyControl::ManualOnOff,
        OccupancyControl::ManualSweepOff,
        OccupancyControl::AutoOnDimmed,
        OccupancyControl::AutoOnAutoOff,
        OccupancyControl::ManualOnDimmed,
        OccupancyControl::ManualOnAutoOff,
    ];

    /// F_OC.
    pub fn f_oc(self) -> f64 {
        match self {
            OccupancyControl::ManualOnOff => 1.00,
            OccupancyControl::ManualSweepOff => 0.95,
            OccupancyControl::AutoOnDimmed => 0.95,
            OccupancyControl::AutoOnAutoOff => 0.90,
            OccupancyControl::ManualOnDimmed => 0.90,
            OccupancyControl::ManualOnAutoOff => 0.80,
        }
    }

    /// Uses presence detection (the coverage conditions apply).
    pub fn uses_detection(self) -> bool {
        self != OccupancyControl::ManualOnOff
    }

    /// Stable key (links, translations).
    pub fn key(self) -> &'static str {
        match self {
            OccupancyControl::ManualOnOff => "manual_on_off",
            OccupancyControl::ManualSweepOff => "manual_sweep_off",
            OccupancyControl::AutoOnDimmed => "auto_on_dimmed",
            OccupancyControl::AutoOnAutoOff => "auto_on_auto_off",
            OccupancyControl::ManualOnDimmed => "manual_on_dimmed",
            OccupancyControl::ManualOnAutoOff => "manual_on_auto_off",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == key)
    }

    /// English label.
    pub fn label(self) -> &'static str {
        match self {
            OccupancyControl::ManualOnOff => "Manual on/off",
            OccupancyControl::ManualSweepOff => "Manual + automatic sweep-off",
            OccupancyControl::AutoOnDimmed => "Auto on / dimmed",
            OccupancyControl::AutoOnAutoOff => "Auto on / auto off",
            OccupancyControl::ManualOnDimmed => "Manual on / dimmed",
            OccupancyControl::ManualOnAutoOff => "Manual on / auto off",
        }
    }
}

/// Space types of the Annex E table of F_A.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceType {
    OfficeOverall,
    CellularOffice1,
    CellularOffice2To6,
    /// F_A from the floor area per sensor (10 m² → 0.20, 30 m² → 0.00).
    OpenPlan,
    OfficeCorridor,
    ConferenceRoom,
    Classroom,
    Toilet,
    CarParkPublic,
}

impl SpaceType {
    pub const ALL: [SpaceType; 9] = [
        SpaceType::OfficeOverall,
        SpaceType::CellularOffice1,
        SpaceType::CellularOffice2To6,
        SpaceType::OpenPlan,
        SpaceType::OfficeCorridor,
        SpaceType::ConferenceRoom,
        SpaceType::Classroom,
        SpaceType::Toilet,
        SpaceType::CarParkPublic,
    ];

    /// Absence factor F_A. Open plan: the table gives 0.20 for one sensor
    /// per 10 m² and 0.00 for one per 30 m²; in between it is interpolated
    /// linearly (our reading), outside it is held.
    pub fn f_a(self, m2_per_sensor: f64) -> f64 {
        match self {
            SpaceType::OfficeOverall => 0.20,
            SpaceType::CellularOffice1 => 0.40,
            SpaceType::CellularOffice2To6 => 0.30,
            SpaceType::OpenPlan => {
                let t = ((m2_per_sensor - 10.0) / 20.0).clamp(0.0, 1.0);
                0.20 * (1.0 - t)
            }
            SpaceType::OfficeCorridor => 0.40,
            SpaceType::ConferenceRoom => 0.50,
            SpaceType::Classroom => 0.25,
            SpaceType::Toilet => 0.90,
            SpaceType::CarParkPublic => 0.80,
        }
    }

    /// The 30 m² group limit applies (meeting rooms are excepted).
    pub fn group_limit_applies(self) -> bool {
        self != SpaceType::ConferenceRoom
    }

    pub fn key(self) -> &'static str {
        match self {
            SpaceType::OfficeOverall => "office_overall",
            SpaceType::CellularOffice1 => "cellular_office_1",
            SpaceType::CellularOffice2To6 => "cellular_office_2_6",
            SpaceType::OpenPlan => "open_plan",
            SpaceType::OfficeCorridor => "office_corridor",
            SpaceType::ConferenceRoom => "conference_room",
            SpaceType::Classroom => "classroom",
            SpaceType::Toilet => "toilet",
            SpaceType::CarParkPublic => "car_park_public",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == key)
    }

    /// English label.
    pub fn label(self) -> &'static str {
        match self {
            SpaceType::OfficeOverall => "Office building, overall",
            SpaceType::CellularOffice1 => "Cellular office, 1 person",
            SpaceType::CellularOffice2To6 => "Cellular office, 2–6 persons",
            SpaceType::OpenPlan => "Open-plan office",
            SpaceType::OfficeCorridor => "Office corridor (dimmed)",
            SpaceType::ConferenceRoom => "Conference room",
            SpaceType::Classroom => "Classroom",
            SpaceType::Toilet => "Toilet",
            SpaceType::CarParkPublic => "Car park, public",
        }
    }
}

/// F_O from F_OC and F_A (Annex E).
pub fn f_o(f_oc: f64, f_a: f64) -> f64 {
    let f_a = f_a.clamp(0.0, 1.0);
    if f_a < 0.2 {
        1.0 - (1.0 - f_oc) * f_a / 0.2
    } else if f_a < 0.9 {
        f_oc + 0.2 - f_a
    } else {
        (7.0 - 10.0 * f_oc) * (f_a - 1.0)
    }
}

/// Coverage needed for "detection matches the lit area" (our threshold).
pub const MIN_COVERAGE: f64 = 0.95;
/// Largest control group (m²) for automatic control credit.
pub const MAX_GROUP_M2: f64 = 30.0;

/// Inputs of the assessment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoInput {
    pub control: OccupancyControl,
    pub space: SpaceType,
    /// Lights off or ≤ 20 % within 15 min of the last presence (confirmed by the planner).
    pub hold_time_ok: bool,
    /// Floor area switched together (m²); with one group per sensor the
    /// room area over the sensor count.
    pub group_m2: f64,
    /// Floor area per sensor (m²), for the open-plan F_A.
    pub m2_per_sensor: f64,
    /// Share of the area seen by the sensors (presence on the workplane).
    pub coverage: f64,
}

/// Result: F_O with the sensors, and with manual switching for comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoAssessment {
    pub f_oc: f64,
    pub f_a: f64,
    /// F_O of the chosen control (1.0-credit rules applied).
    pub f_o: f64,
    /// F_O with manual on/off, the baseline.
    pub f_o_manual: f64,
    pub hold_time_ok: bool,
    pub group_ok: bool,
    pub coverage_ok: bool,
    /// All conditions for automatic control are met (always true for manual).
    pub credited: bool,
}

/// F_O for the input; if the automatic-control conditions are not met the
/// result falls back to manual on/off (no credit for the detectors).
pub fn assess(input: &FoInput) -> FoAssessment {
    let f_a = input.space.f_a(input.m2_per_sensor);
    let f_o_manual = f_o(OccupancyControl::ManualOnOff.f_oc(), f_a);
    let group_ok = !input.space.group_limit_applies() || input.group_m2 <= MAX_GROUP_M2 + 1e-9;
    let coverage_ok = input.coverage >= MIN_COVERAGE;
    let credited =
        !input.control.uses_detection() || (input.hold_time_ok && group_ok && coverage_ok);
    let f_oc = if credited { input.control.f_oc() } else { 1.0 };
    FoAssessment {
        f_oc,
        f_a,
        f_o: f_o(f_oc, f_a),
        f_o_manual,
        hold_time_ok: input.hold_time_ok,
        group_ok,
        coverage_ok,
        credited,
    }
}

/// Annual lighting energy (kWh/a) = power × hours × F_O (no daylight or
/// constant-illuminance factors).
pub fn annual_kwh(power_w: f64, hours: f64, f_o: f64) -> f64 {
    power_w * hours * f_o / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_e_formula() {
        // The worked value from the plan: F_OC 0.9, F_A 0.5 → 0.6.
        assert!((f_o(0.9, 0.5) - 0.6).abs() < 1e-12);
        // Continuous at the breakpoints, 0 at F_A = 1, 1 at F_A = 0.
        for f_oc in [0.8, 0.9, 0.95, 1.0] {
            assert!((f_o(f_oc, 0.2 - 1e-9) - f_o(f_oc, 0.2)).abs() < 1e-6);
            assert!((f_o(f_oc, 0.9 - 1e-9) - f_o(f_oc, 0.9)).abs() < 1e-6);
            assert!(f_o(f_oc, 1.0).abs() < 1e-12);
            assert!((f_o(f_oc, 0.0) - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn open_plan_absence_factor() {
        assert_eq!(SpaceType::OpenPlan.f_a(10.0), 0.20);
        assert_eq!(SpaceType::OpenPlan.f_a(30.0), 0.0);
        assert!((SpaceType::OpenPlan.f_a(20.0) - 0.10).abs() < 1e-12);
        assert_eq!(SpaceType::OpenPlan.f_a(5.0), 0.20);
    }

    #[test]
    fn credit_needs_all_conditions() {
        let ok = FoInput {
            control: OccupancyControl::AutoOnAutoOff,
            space: SpaceType::CellularOffice2To6,
            hold_time_ok: true,
            group_m2: 20.0,
            m2_per_sensor: 20.0,
            coverage: 0.99,
        };
        let a = assess(&ok);
        assert!(a.credited);
        assert!((a.f_o - (0.9 + 0.2 - 0.3)).abs() < 1e-12);
        assert!((a.f_o_manual - (1.0 + 0.2 - 0.3)).abs() < 1e-12);
        for bad in [
            FoInput {
                hold_time_ok: false,
                ..ok
            },
            FoInput {
                group_m2: 40.0,
                ..ok
            },
            FoInput {
                coverage: 0.8,
                ..ok
            },
        ] {
            let b = assess(&bad);
            assert!(!b.credited);
            assert_eq!(b.f_o, b.f_o_manual);
        }
        // Meeting rooms are excepted from the group limit.
        let meeting = assess(&FoInput {
            space: SpaceType::ConferenceRoom,
            group_m2: 60.0,
            ..ok
        });
        assert!(meeting.credited && meeting.group_ok);
    }

    #[test]
    fn keys_round_trip() {
        for c in OccupancyControl::ALL {
            assert_eq!(OccupancyControl::from_key(c.key()), Some(c));
        }
        for s in SpaceType::ALL {
            assert_eq!(SpaceType::from_key(s.key()), Some(s));
        }
    }
}
