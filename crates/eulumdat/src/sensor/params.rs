//! The parameter block of a sensor LDT (EULUMDAT lines 33–42, the
//! "direct ratios" of a luminaire file).
//!
//! What is known comes from observing the Relux Net sensor corpus
//! (B.E.G., ESYLUX, Steinel, Theben; 2026-11-04), not from a specification:
//!
//! | field (line) | observed |
//! |---|---|
//! | 1 (33) | scale of the **radial** walking volume (0.17 … 1) |
//! | 2 (34) | scale of the **tangential** walking volume (mostly 1; ESYLUX 0.46 … 2.512) |
//! | 3 (35) | scale of the **presence** volume; 0 = no presence detection |
//! | 4 (36) | 0, 1, 5, 8, 11 |
//! | 6 (38) | 1 for presence detectors, 0 for motion detectors of the same family |
//! | 8 (40) | mounting: 1 wall, 2 ceiling, 3 wall/ceiling (as SensXML `MountingType`) |
//! | 9 (41) | 0 or an angle (20, 30, 90, 150, 180, 270) |
//!
//! The motion types follow from Relux's plan renders of these files: the
//! outer (tangential) curve is the volume × field 2, the radial curve the
//! volume × field 1, the presence curve the volume × field 3, all cut at the
//! same plane. Checked on 165 renders scaled by Relux's `maximum_range`:
//! outer shape within 5 % in 99 %, radial in 94 %, presence in 90 %
//! (median errors 1.9 / 2.7 / 4.2 %); scaling the outer outline instead fails
//! (≈ 50 % error). The one exception is B.E.G. PD4, whose cached render
//! probably predates its current file. Relux evaluates at a plane
//! [`REFERENCE_DEPTH`] below the sensor (median 2.00 m for ceiling
//! sensors, 1.95 m for mounting type 3, 1.75 m for wall sensors).
//! Field 8 uses the SensXML `MountingType` coding (1 wall, 2 ceiling,
//! 3 wall/ceiling). This is an observed convention, not a specification.
//!
//! Only the clearest fields are exposed; everything else is available raw.

use crate::Eulumdat;

/// Reference depth below the sensor (m) for sensor plan views and the
/// maximum range: the common planning convention, 2.0 m (reference plan views
/// of sensor LDTs and SensXML planning shapes both match at this depth).
pub const REFERENCE_DEPTH: f64 = 2.0;

/// Kind of movement a detection area refers to (IEC 63180 terms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Motion {
    /// Walking across the sensor's view (largest area).
    Tangential,
    /// Walking towards the sensor.
    Radial,
    /// Seated, minor movements (smallest area).
    Presence,
}

impl Motion {
    pub const ALL: [Motion; 3] = [Motion::Tangential, Motion::Radial, Motion::Presence];

    pub fn label(&self) -> &'static str {
        match self {
            Motion::Tangential => "tangential walking",
            Motion::Radial => "radial walking",
            Motion::Presence => "presence (seated)",
        }
    }
}

/// Mounting suggested by field 8 of the parameter block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MountingHint {
    Wall,
    Ceiling,
    /// 3 = wall or ceiling (SensXML `MountingType` coding)
    WallOrCeiling,
    Other(i32),
}

/// Raw parameter block of a sensor LDT.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SensorParams {
    /// Fields 1–10 (EULUMDAT lines 33–42) as read.
    pub raw: [f64; 10],
}

impl SensorParams {
    pub fn from_ldt(ldt: &Eulumdat) -> Self {
        Self {
            raw: ldt.direct_ratios,
        }
    }

    /// Field `n` (1-based, as numbered in the table above).
    pub fn field(&self, n: usize) -> Option<f64> {
        n.checked_sub(1).and_then(|i| self.raw.get(i)).copied()
    }

    /// Scale of the detection volume for a motion type, or `None` if the
    /// file has no data for it (field ≤ 0). Tangential falls back to 1.
    pub fn scale(&self, motion: Motion) -> Option<f64> {
        let v = match motion {
            Motion::Tangential => {
                if self.raw[1] > 0.0 {
                    self.raw[1]
                } else {
                    1.0
                }
            }
            Motion::Radial => self.raw[0],
            Motion::Presence => self.raw[2],
        };
        (v > 0.0).then_some(v)
    }

    /// Field 3 is non-zero exactly when the sensor also detects presence
    /// (seated, minor motion).
    pub fn has_presence_hint(&self) -> bool {
        self.raw[2] > 0.0
    }

    /// Field 8: 1 = wall, 2 = ceiling, 3 = wall or ceiling.
    pub fn mounting_hint(&self) -> MountingHint {
        match self.raw[7].round() as i32 {
            1 => MountingHint::Wall,
            2 => MountingHint::Ceiling,
            3 => MountingHint::WallOrCeiling,
            n => MountingHint::Other(n),
        }
    }
}

/// How a footprint was obtained (SensXML: measured heights only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Quality {
    /// At a measured mounting height.
    Measured,
    /// Between two measured heights (linear per azimuth).
    Interpolated,
    /// Outside the measured heights (below: scaled with height; above: held).
    Extrapolated,
}

impl Quality {
    /// English label.
    pub fn label(self) -> &'static str {
        match self {
            Quality::Measured => "measured",
            Quality::Interpolated => "interpolated",
            Quality::Extrapolated => "extrapolated",
        }
    }
}
