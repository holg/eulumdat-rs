//! Daylight physics for photometric daytime calculations.
//!
//! Pure `f64` models, no engine dependencies — usable by the Monte Carlo
//! tracers, the Bevy viewer, and the core `eulumdat` crate alike.
//!
//! - [`solar`] — sun position (altitude/azimuth) from latitude, longitude, date, time.
//! - [`location`] — named world locations + local-wall-clock ⇄ UTC conversion.
//! - [`sky`] — analytic sky luminance distribution (Perez all-weather + CIE standard skies).
//! - [`availability`] — direct/diffuse/global daylight illuminance (lux) at a location/time.
//! - [`coords`] — the single, frozen adapter between the astronomical (horizon),
//!   Bevy world (+Y up), and photometric Type-C (gamma-from-nadir) frames.
//! - [`constants`] — solar / daylight physical constants.
//!
//! Everything is **photometric** (lux, cd/m²). Radiometric inputs (W/m²) are
//! converted via the luminous efficacy of daylight in [`constants`]. A spectral
//! path can later override the efficacy per band without changing these APIs.
//!
//! # Example
//! ```
//! use eulumdat_daylight::{solar::solar_position, availability::DaylightAvailability};
//!
//! // Madrid, summer solstice, ~solar noon UTC.
//! let sun = solar_position(2026, 6, 21, 12.0, 40.4, -3.7);
//! assert!(sun.altitude_deg() > 60.0); // high summer sun
//!
//! let avail = DaylightAvailability::clear_sky(&sun, 2.5);
//! assert!(avail.ghi_lux > 80_000.0); // bright clear noon
//! ```

#![forbid(unsafe_code)]

pub mod availability;
pub mod constants;
pub mod coords;
pub mod location;
pub mod lunar;
pub mod sky;
pub mod solar;

pub use availability::{daylight_cct, DaylightAvailability};
pub use location::{location_by_name, DstRule, LocalDateTime, NamedLocation, LOCATIONS};
pub use lunar::{moon_position, MoonPhase, MoonPosition};
pub use sky::{CieSky, PerezSky, SkyParams, SkyRadiance};
pub use solar::SolarPosition;
