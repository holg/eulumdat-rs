//! Implementation of PhotometricData for Eulumdat.
//!
//! This file bridges the eulumdat crate with the photometric module.
//! When extracting `bevy_photometry` as a standalone crate, this file
//! stays in `eulumdat-bevy` while the `photometric/` module is extracted.

use crate::photometric::{parse_color_temperature, parse_cri, PhotometricData};
use eulumdat::Eulumdat;

impl PhotometricData for Eulumdat {
    fn sample(&self, c_angle: f64, g_angle: f64) -> f64 {
        // Eulumdat::sample already handles symmetry expansion and interpolation
        Eulumdat::sample(self, c_angle, g_angle)
    }

    fn max_intensity(&self) -> f64 {
        Eulumdat::max_intensity(self)
    }

    fn total_flux(&self) -> f64 {
        self.total_luminous_flux()
    }

    fn light_output_ratio(&self) -> f64 {
        self.light_output_ratio / 100.0 // Convert from % to fraction
    }

    fn downward_fraction(&self) -> f64 {
        self.downward_flux_fraction / 100.0 // Convert from % to fraction
    }

    fn dimensions(&self) -> (f32, f32, f32) {
        // Convert from mm to meters
        (
            (self.width / 1000.0) as f32,
            (self.length / 1000.0) as f32,
            (self.height / 1000.0).max(0.05) as f32, // Minimum height for visibility
        )
    }

    fn color_temperature(&self) -> Option<f32> {
        self.lamp_sets
            .first()
            .and_then(|lamp| parse_color_temperature(&lamp.color_appearance))
    }

    fn cri(&self) -> Option<f32> {
        self.lamp_sets
            .first()
            .map(|lamp| parse_cri(&lamp.color_rendering_group))
    }

    fn beam_angle(&self) -> f64 {
        // Calculate beam angle: find gamma where intensity drops to 50% of max
        let max_intensity = self.max_intensity();
        if max_intensity <= 0.0 {
            return std::f64::consts::FRAC_PI_4; // 45 degrees default
        }

        let half_max = max_intensity * 0.5;

        // Scan from nadir (0°) outward to find 50% point
        for g in 0..90 {
            let intensity = self.sample(0.0, g as f64);
            if intensity < half_max {
                return (g as f64).to_radians();
            }
        }

        std::f64::consts::FRAC_PI_2 // 90 degrees if not found
    }
}

/// Type alias for convenience
pub type EulumdatLight = crate::photometric::PhotometricLight<Eulumdat>;

/// Type alias for convenience
pub type EulumdatLightBundle = crate::photometric::PhotometricLightBundle<Eulumdat>;
