//! DIN EN 13201-1 — lighting-class selection matrix (Annex A).
//!
//! The user specifies their road's characteristics (design speed, traffic
//! composition, ambient luminance, etc.) and this module computes:
//! - A weighted sum **Vws** per EN 13201-1:2014 §A.2 / §A.3.
//! - The recommended lighting class (C-family conflict area or P-family
//!   pedestrian area) per the mapping in §A.2 Table A.1 / §A.3 Table A.2.
//!
//! Weights reflect the 2014 edition. **Confirm against the current edition
//! before using in certified designs** — the CEN/CENELEC revision cycle
//! adjusts individual weights from time to time.
//!
//! Only C and P families are implemented — the M-family (motorized,
//! luminance method) requires R-tables and veiling-luminance math that
//! [`super::en13201`] explicitly defers. The selector here shares that
//! scope decision so the two modules stay in step.

use super::en13201::En13201Class;

// ─────────────────────────────────────────────────────────────────────────
// C-family (conflict area) parameters
// ─────────────────────────────────────────────────────────────────────────

/// Design speed (km/h) on the road being classified.
///
/// EN 13201-1 §A.2 Table A.1 groups design speeds into four bands. The
/// higher the speed, the higher the recommended lighting class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DesignSpeed {
    /// > 60 km/h.
    VeryHigh,
    /// 30–60 km/h.
    High,
    /// 5–30 km/h (sometimes called "low").
    Moderate,
    /// Walking speed (pedestrian zones, shared spaces).
    Low,
}

impl DesignSpeed {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::VeryHigh => 2,
            Self::High => 1,
            Self::Moderate => 0,
            Self::Low => -1,
        }
    }
}

/// Traffic volume on the road.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrafficVolume {
    /// > 80% of roadway maximum capacity.
    VeryHigh,
    /// 65–80% of maximum.
    High,
    /// 35–65% of maximum.
    Moderate,
    /// 15–35% of maximum.
    Low,
    /// < 15% of maximum.
    VeryLow,
}

impl TrafficVolume {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::VeryHigh => 1,
            Self::High => 1,
            Self::Moderate => 0,
            Self::Low => 0,
            Self::VeryLow => -1,
        }
    }
}

/// Traffic composition: mix of motorized / slow / non-motorized users.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrafficComposition {
    /// Mixed (motor + slow + non-motor), e.g. urban with cyclists + pedestrians.
    Mixed,
    /// Mixed, predominantly motor.
    MixedMotorDominant,
    /// Motor only (segregated cycle lane / footway).
    MotorOnly,
}

impl TrafficComposition {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::Mixed => 2,
            Self::MixedMotorDominant => 1,
            Self::MotorOnly => 0,
        }
    }
}

/// Whether the opposing carriageway is separated (e.g. divided highway).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Separation {
    /// Separated carriageways (median / divider).
    Separated,
    /// Not separated (opposing lanes share surface).
    NotSeparated,
}

impl Separation {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::Separated => 0,
            Self::NotSeparated => 1,
        }
    }
}

/// Density of signalized or at-grade junctions per kilometer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JunctionDensity {
    /// > 3 per km.
    High,
    /// ≤ 3 per km.
    Moderate,
}

impl JunctionDensity {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::High => 1,
            Self::Moderate => 0,
        }
    }
}

/// Parked vehicles along the carriageway edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParkedVehicles {
    Present,
    NotPresent,
}

impl ParkedVehicles {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::Present => 1,
            Self::NotPresent => 0,
        }
    }
}

/// Ambient luminance — light from shops, billboards, adjacent development.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AmbientLuminance {
    High,
    Moderate,
    Low,
}

impl AmbientLuminance {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::High => 1,
            Self::Moderate => 0,
            Self::Low => -1,
        }
    }
    fn vws_weight_p(self) -> i32 {
        match self {
            Self::High => 1,
            Self::Moderate => 0,
            Self::Low => -1,
        }
    }
}

/// Need for visual guidance / navigation aid (signage legibility, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NavigationalTask {
    /// Complex road geometry, heavy signage, difficult to navigate.
    VeryDifficult,
    Difficult,
    Easy,
}

impl NavigationalTask {
    fn vws_weight_c(self) -> i32 {
        match self {
            Self::VeryDifficult => 2,
            Self::Difficult => 1,
            Self::Easy => 0,
        }
    }
}

/// The full set of parameters that drive a C-family (conflict area) class
/// recommendation per EN 13201-1 §A.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CParamsEn13201 {
    pub design_speed: DesignSpeed,
    pub traffic_volume: TrafficVolume,
    pub traffic_composition: TrafficComposition,
    pub separation: Separation,
    pub junction_density: JunctionDensity,
    pub parked_vehicles: ParkedVehicles,
    pub ambient_luminance: AmbientLuminance,
    pub navigational_task: NavigationalTask,
}

impl Default for CParamsEn13201 {
    fn default() -> Self {
        Self {
            design_speed: DesignSpeed::High,
            traffic_volume: TrafficVolume::Moderate,
            traffic_composition: TrafficComposition::MixedMotorDominant,
            separation: Separation::NotSeparated,
            junction_density: JunctionDensity::Moderate,
            parked_vehicles: ParkedVehicles::NotPresent,
            ambient_luminance: AmbientLuminance::Moderate,
            navigational_task: NavigationalTask::Easy,
        }
    }
}

impl CParamsEn13201 {
    /// Compute Vws per EN 13201-1 §A.2. Range roughly -1..+9 — the mapping
    /// to a class clamps to C0..C5.
    pub fn vws(&self) -> i32 {
        self.design_speed.vws_weight_c()
            + self.traffic_volume.vws_weight_c()
            + self.traffic_composition.vws_weight_c()
            + self.separation.vws_weight_c()
            + self.junction_density.vws_weight_c()
            + self.parked_vehicles.vws_weight_c()
            + self.ambient_luminance.vws_weight_c()
            + self.navigational_task.vws_weight_c()
    }

    /// Recommended C-class for these parameters.
    ///
    /// Mapping (2014 edition, §A.2 Table A.1):
    ///   Vws ≤ 0 → C5,  1 → C4,  2 → C3,  3 → C2,  4 → C1,  ≥5 → C0
    pub fn recommended_class(&self) -> En13201Class {
        match self.vws() {
            i if i <= 0 => En13201Class::C5,
            1 => En13201Class::C4,
            2 => En13201Class::C3,
            3 => En13201Class::C2,
            4 => En13201Class::C1,
            _ => En13201Class::C0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// P-family (pedestrian area) parameters
// ─────────────────────────────────────────────────────────────────────────

/// Speed of main user on a pedestrian/low-speed area.
///
/// P-family is for areas where pedestrians dominate — the "speed" question
/// refers to whether *any* motor traffic is allowed at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PedestrianSpeed {
    /// Walking speed (strictly pedestrian).
    WalkingOnly,
    /// Walking + occasional slow motor (access only).
    SlowMixed,
    /// Regular low-speed motor + pedestrians (residential streets).
    LowSpeedMotor,
}

impl PedestrianSpeed {
    fn vws_weight_p(self) -> i32 {
        match self {
            Self::WalkingOnly => -1,
            Self::SlowMixed => 0,
            Self::LowSpeedMotor => 1,
        }
    }
}

/// User volume (ped + occasional vehicle) per hour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UserDensity {
    High,
    Moderate,
    Low,
}

impl UserDensity {
    fn vws_weight_p(self) -> i32 {
        match self {
            Self::High => 1,
            Self::Moderate => 0,
            Self::Low => -1,
        }
    }
}

/// Need for facial recognition (safety in crime-risk areas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FacialRecognition {
    /// Necessary (safety-sensitive area).
    Necessary,
    /// Useful but not essential.
    Useful,
    /// Not needed.
    NotNeeded,
}

impl FacialRecognition {
    fn vws_weight_p(self) -> i32 {
        match self {
            Self::Necessary => 1,
            Self::Useful => 0,
            Self::NotNeeded => -1,
        }
    }
}

/// Full set of P-family (pedestrian) parameters per EN 13201-1 §A.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PParamsEn13201 {
    pub pedestrian_speed: PedestrianSpeed,
    pub user_density: UserDensity,
    pub ambient_luminance: AmbientLuminance,
    pub facial_recognition: FacialRecognition,
}

impl Default for PParamsEn13201 {
    fn default() -> Self {
        Self {
            pedestrian_speed: PedestrianSpeed::SlowMixed,
            user_density: UserDensity::Moderate,
            ambient_luminance: AmbientLuminance::Moderate,
            facial_recognition: FacialRecognition::Useful,
        }
    }
}

impl PParamsEn13201 {
    /// Vws per §A.3 — sum of weights, range roughly -3..+3.
    pub fn vws(&self) -> i32 {
        self.pedestrian_speed.vws_weight_p()
            + self.user_density.vws_weight_p()
            + self.ambient_luminance.vws_weight_p()
            + self.facial_recognition.vws_weight_p()
    }

    /// Recommended P-class.
    ///
    /// Mapping (2014 edition, §A.3 Table A.2):
    ///   Vws ≥ 2 → P1, 1 → P2, 0 → P3, -1 → P4, -2 → P5, ≤ -3 → P6
    pub fn recommended_class(&self) -> En13201Class {
        match self.vws() {
            i if i >= 2 => En13201Class::P1,
            1 => En13201Class::P2,
            0 => En13201Class::P3,
            -1 => En13201Class::P4,
            -2 => En13201Class::P5,
            _ => En13201Class::P6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_params_defaults_give_mid_class() {
        // Default (High speed, moderate traffic, mixed-motor dominant…) →
        // Vws = 1 + 0 + 1 + 1 + 0 + 0 + 0 + 0 = 3 → C2.
        let p = CParamsEn13201::default();
        assert_eq!(p.vws(), 3);
        assert_eq!(p.recommended_class(), En13201Class::C2);
    }

    #[test]
    fn c_very_high_everything_tops_out_at_c0() {
        let p = CParamsEn13201 {
            design_speed: DesignSpeed::VeryHigh,
            traffic_volume: TrafficVolume::VeryHigh,
            traffic_composition: TrafficComposition::Mixed,
            separation: Separation::NotSeparated,
            junction_density: JunctionDensity::High,
            parked_vehicles: ParkedVehicles::Present,
            ambient_luminance: AmbientLuminance::High,
            navigational_task: NavigationalTask::VeryDifficult,
        };
        assert!(p.vws() >= 5);
        assert_eq!(p.recommended_class(), En13201Class::C0);
    }

    #[test]
    fn c_minimum_everything_bottoms_out_at_c5() {
        let p = CParamsEn13201 {
            design_speed: DesignSpeed::Low,
            traffic_volume: TrafficVolume::VeryLow,
            traffic_composition: TrafficComposition::MotorOnly,
            separation: Separation::Separated,
            junction_density: JunctionDensity::Moderate,
            parked_vehicles: ParkedVehicles::NotPresent,
            ambient_luminance: AmbientLuminance::Low,
            navigational_task: NavigationalTask::Easy,
        };
        assert!(p.vws() <= 0);
        assert_eq!(p.recommended_class(), En13201Class::C5);
    }

    #[test]
    fn p_defaults_produce_mid_class() {
        // Default P-params (slow mixed, moderate density, moderate ambient,
        // useful facial recognition) → Vws = 0 + 0 + 0 + 0 = 0 → P3.
        let p = PParamsEn13201::default();
        assert_eq!(p.vws(), 0);
        assert_eq!(p.recommended_class(), En13201Class::P3);
    }

    #[test]
    fn p_high_everything_tops_out_at_p1() {
        let p = PParamsEn13201 {
            pedestrian_speed: PedestrianSpeed::LowSpeedMotor,
            user_density: UserDensity::High,
            ambient_luminance: AmbientLuminance::High,
            facial_recognition: FacialRecognition::Necessary,
        };
        assert!(p.vws() >= 2);
        assert_eq!(p.recommended_class(), En13201Class::P1);
    }

    #[test]
    fn p_minimum_everything_bottoms_at_p6() {
        let p = PParamsEn13201 {
            pedestrian_speed: PedestrianSpeed::WalkingOnly,
            user_density: UserDensity::Low,
            ambient_luminance: AmbientLuminance::Low,
            facial_recognition: FacialRecognition::NotNeeded,
        };
        assert!(p.vws() <= -3);
        assert_eq!(p.recommended_class(), En13201Class::P6);
    }

    #[test]
    fn every_c_class_is_reachable() {
        use En13201Class::*;
        let base = CParamsEn13201 {
            design_speed: DesignSpeed::Moderate,
            traffic_volume: TrafficVolume::Moderate,
            traffic_composition: TrafficComposition::MotorOnly,
            separation: Separation::Separated,
            junction_density: JunctionDensity::Moderate,
            parked_vehicles: ParkedVehicles::NotPresent,
            ambient_luminance: AmbientLuminance::Moderate,
            navigational_task: NavigationalTask::Easy,
        };
        // Walk Vws 0..5 by tweaking one dial at a time.
        assert_eq!(base.recommended_class(), C5); // Vws = 0
        let mut p = base;
        p.traffic_volume = TrafficVolume::High;
        assert_eq!(p.recommended_class(), C4); // Vws = 1
        p.ambient_luminance = AmbientLuminance::High;
        assert_eq!(p.recommended_class(), C3); // Vws = 2
        p.navigational_task = NavigationalTask::Difficult;
        assert_eq!(p.recommended_class(), C2); // Vws = 3
        p.traffic_composition = TrafficComposition::MixedMotorDominant;
        assert_eq!(p.recommended_class(), C1); // Vws = 4
        p.traffic_composition = TrafficComposition::Mixed;
        assert_eq!(p.recommended_class(), C0); // Vws = 5
    }
}
