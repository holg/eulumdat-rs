//! Coordinate-frame adapter — the single, frozen boundary between the
//! conventions in play. Define conversions ONCE here; never re-derive them
//! ad hoc in consumers (that is where sign errors creep in).
//!
//! Frames:
//! - **Astronomical horizontal** (this crate): `altitude` from the horizon
//!   (0 = horizon, +90° = zenith); `azimuth` measured **from North, clockwise
//!   toward East** (N=0°, E=90°, S=180°, W=270°). Zenith angle `θ = 90° − alt`.
//! - **ENU world, +Y up** (Bevy / the renderers): right-handed,
//!   **+X = East, +Y = up (zenith), +Z = South**. (North = −Z.)
//! - **Sky-dome polar** (sky model): `theta` from the **zenith** (0 = straight
//!   up), `phi` azimuth sharing the astronomical reference (from North, CW).
//!
//! ## The negation rule (read before touching any consumer)
//! A direction *toward* a sky element / the sun is **up-going**. A photon
//! *arriving from* that element travels **down-going** — the negation. Radiance
//! lookups and camera escape rays use the up-going direction; Monte Carlo
//! emission and surface-incidence use its negative. Negate **exactly once**, at
//! the point of use, never inside these conversions.

/// A unit direction in the ENU world frame (+X East, +Y up, +Z South).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldDir {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl WorldDir {
    /// The vertical (up) component. Positive = above the horizon.
    pub fn up(&self) -> f64 {
        self.y
    }
}

/// Convert astronomical altitude/azimuth (degrees) to an **up-going** unit
/// vector in the ENU world frame (+X East, +Y up, +Z South).
///
/// Derivation: with azimuth `A` from North clockwise and altitude `a`,
/// East  = cos(a)·sin(A), Up = sin(a), North = cos(a)·cos(A); and North = −Z.
pub fn altaz_to_world(altitude_deg: f64, azimuth_deg: f64) -> WorldDir {
    let a = altitude_deg.to_radians();
    let az = azimuth_deg.to_radians();
    let cos_a = a.cos();
    WorldDir {
        x: cos_a * az.sin(),  // East
        y: a.sin(),           // up (zenith component)
        z: -cos_a * az.cos(), // South = −North
    }
}

/// Convert a sky-dome polar direction (`theta` from zenith, `phi` azimuth from
/// North clockwise, both radians) to an **up-going** ENU world unit vector.
pub fn dome_to_world(theta_rad: f64, phi_rad: f64) -> WorldDir {
    let sin_t = theta_rad.sin();
    WorldDir {
        x: sin_t * phi_rad.sin(),  // East
        y: theta_rad.cos(),        // up
        z: -sin_t * phi_rad.cos(), // South
    }
}

/// Angle (radians) between two world directions — used for the scattering angle
/// between a sky element and the sun in the Perez indicatrix.
pub fn angle_between(a: &WorldDir, b: &WorldDir) -> f64 {
    let dot = (a.x * b.x + a.y * b.y + a.z * b.z).clamp(-1.0, 1.0);
    dot.acos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn zenith_points_straight_up() {
        let d = altaz_to_world(90.0, 0.0);
        close(d.x, 0.0);
        close(d.y, 1.0);
        close(d.z, 0.0);
    }

    #[test]
    fn horizon_cardinals() {
        // Due East on the horizon → +X.
        let e = altaz_to_world(0.0, 90.0);
        close(e.x, 1.0);
        close(e.y, 0.0);
        close(e.z, 0.0);
        // Due North on the horizon → −Z.
        let n = altaz_to_world(0.0, 0.0);
        close(n.x, 0.0);
        close(n.z, -1.0);
        // Due South → +Z.
        let s = altaz_to_world(0.0, 180.0);
        close(s.z, 1.0);
    }

    #[test]
    fn dome_zenith_matches_altaz_zenith() {
        // theta=0 (zenith) must equal altitude=90.
        let dome = dome_to_world(0.0, 0.0);
        let alt = altaz_to_world(90.0, 0.0);
        close(dome.x, alt.x);
        close(dome.y, alt.y);
        close(dome.z, alt.z);
    }

    #[test]
    fn angle_between_orthogonal() {
        let up = altaz_to_world(90.0, 0.0);
        let east = altaz_to_world(0.0, 90.0);
        close(angle_between(&up, &east), std::f64::consts::FRAC_PI_2);
    }
}
