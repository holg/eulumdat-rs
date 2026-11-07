//! Presence / motion sensors stored as EULUMDAT ("sensor LDT").
//!
//! GLDF calls these files `sensor/sensldt`; Relux Net ships them as the
//! `lvk` file of sensor articles. The file layout is plain EULUMDAT and is
//! read by the normal parser, but the numbers mean something else:
//!
//! * **The intensity matrix is a detection volume.** The value at (C, γ) is
//!   the distance in metres from the sensor to the boundary of the volume in
//!   which a person is detected, along that direction (γ = 0 straight down,
//!   C0 = +x). It is the photometric solid of a luminaire, with "range"
//!   instead of "intensity". Example (Steinel Dual US corridor sensor): the
//!   values in C0 are exactly `4.25 / cos γ` up to 19°, then `1.5 / sin γ`,
//!   i.e. a slab 4.25 m deep and 3 m wide.
//! * **A footprint is a cut through that volume.** For a sensor mounted at
//!   height `H` and a person at height `z` (floor for walking, desk height
//!   for seated presence) the plane lies `depth = H − z` below the sensor.
//! * **The direct-ratio block (lines 33–42) is a parameter block**, see
//!   [`SensorParams`]: fields 2, 1 and 3 scale the volume for tangential
//!   walking, radial walking and presence ([`Motion`]).
//! * The lamp block, flux and LORL carry no meaning for the detection area.
//!
//! SensXML files (GLDF `sensor/sensxml`) are read by [`sensxml`].
//!
//! Nothing in the file says "this is a sensor": callers decide from context
//! (GLDF content type, the Relux article type). See `docs/sensors-plan.md`.

pub mod compare;
pub mod coverage;
mod detect;
pub mod diagrams;
pub mod en15193;
mod file;
mod params;
#[cfg(feature = "xml")]
pub mod sensxml;
mod svg;
mod volume;

pub use detect::{has_sensor_parameters, is_sensor_ldt};
pub use diagrams::SensorDiagram;
pub use file::{DetectionArea, SensorFile, SensorWarning, MAX_PLAUSIBLE_RANGE_M};
pub use params::{Motion, MountingHint, Quality, SensorParams, REFERENCE_DEPTH};
#[cfg(feature = "xml")]
pub use sensxml::SensXml;
pub use svg::{SensorPlanSvg, SensorPlanTrace, TRACE_COLORS};
pub use volume::{DepthProfile, DetectionVolume, Footprint};
