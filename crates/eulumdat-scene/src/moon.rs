//! Natural light over an outdoor scene for a date, a wall-clock time and a
//! place, under a clear sky ([`SkyLight`]): by day the sun's beam and the
//! daylit sky ([`eulumdat_daylight::DaylightAvailability::clear_sky`]), at
//! dusk the twilight sky, at night the moon ([`eulumdat_daylight::night`]:
//! phase, distance, opposition surge, eclipses) and the starlit sky.
//!
//! Scene frame: Y up, east = +X, north = −Z (south = +Z, the daylight
//! crate's world frame), turned by `north_deg` (the direction of north in
//! the scene, degrees from −Z towards +X).

use eulumdat_daylight::location::{DstRule, LocalDateTime, NamedLocation};
use eulumdat_daylight::night::{twilight_lux, STARLIGHT_LUX, SUNSET_LUX};
use eulumdat_daylight::{daylight_cct, DaylightAvailability, Moonlight};
use glam::Vec3;

/// Clear-sky turbidity for the moonlight (as the daylight's default).
pub const MOON_TURBIDITY: f64 = 2.5;

/// Correlated colour temperature of moonlight (K): sunlight reddened by
/// the regolith.
pub const MOON_CCT_K: f32 = 4100.0;

/// The moon over a scene.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MoonSky {
    /// Unit vector toward the moon in the scene frame.
    pub dir: Vec3,
    /// Illuminance normal to the beam, the moonlit sky on a horizontal
    /// surface, both together on a horizontal surface (lx).
    pub normal_lux: f32,
    pub sky_lux: f32,
    pub horizontal_lux: f32,
    pub altitude_deg: f32,
    pub azimuth_deg: f32,
    /// Illuminated fraction of the disc (0 new … 1 full).
    pub illuminated: f32,
    #[serde(default)]
    pub waxing: bool,
    pub distance_km: f32,
}

/// What lights the scene most at that moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkyKind {
    /// The sun above the horizon.
    Sun,
    /// The sun up to 18° below the horizon.
    Twilight,
    Moon,
    /// Starlight and airglow.
    Night,
}

/// The natural light over a scene: one beam (sun or moon, with shadows)
/// and the sky.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkyLight {
    pub kind: SkyKind,
    /// Unit vector toward the sun or moon (scene frame), the beam's
    /// illuminance normal to it (lx; 0 = no beam) and its colour (K).
    pub dir: Vec3,
    pub beam_lux: f32,
    pub beam_cct: f32,
    /// The sky on a horizontal surface, and everything together (lx).
    pub sky_lux: f32,
    pub horizontal_lux: f32,
    pub sun_altitude_deg: f32,
    pub sun_azimuth_deg: f32,
    /// The moon then (for the panel).
    pub moon: MoonSky,
}

/// When and where: the moon's inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct MoonTime {
    /// "YYYY-MM-DD".
    pub date: String,
    /// Local wall-clock hours (0–24).
    pub hour: f32,
    pub latitude: f64,
    pub longitude: f64,
    /// Standard UTC offset (h) and the daylight-saving rule
    /// ("eu" | "us" | "south" | "none").
    pub utc_offset: f32,
    pub dst: String,
    /// North in the scene (degrees from −Z towards +X).
    pub north_deg: f32,
}

/// The date ("YYYY-MM-DD") of the night with the fullest moon among the
/// 30 nights after `date` (illuminated fraction at 23:00 local time).
pub fn next_full_moon(t: &MoonTime) -> Option<String> {
    let (y, m, d) = parse_date(&t.date)?;
    let mut day = LocalDateTime::new(y, m, d, 23.0);
    let mut best = (0.0f32, None);
    for _ in 0..30 {
        day = day.next_day();
        let probe = MoonTime {
            date: format!("{:04}-{:02}-{:02}", day.year, day.month, day.day),
            hour: 23.0,
            ..t.clone()
        };
        if let Some(mo) = probe.moon() {
            if mo.illuminated > best.0 {
                best = (mo.illuminated, Some(probe.date));
            }
        }
    }
    best.1
}

/// Parse "YYYY-MM-DD".
pub fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.trim().splitn(3, '-');
    let y = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some((y, m, d))
}

/// The DST rule of a key.
pub fn dst_rule(key: &str) -> DstRule {
    match key {
        "eu" => DstRule::Eu,
        "us" => DstRule::Us,
        "south" => DstRule::SouthernEu,
        _ => DstRule::None,
    }
}

impl MoonTime {
    fn place_and_time(&self) -> Option<(NamedLocation, LocalDateTime)> {
        let (y, m, d) = parse_date(&self.date)?;
        let loc = NamedLocation::with_dst(
            "Scene",
            self.latitude.clamp(-90.0, 90.0),
            self.longitude.clamp(-180.0, 180.0),
            f64::from(self.utc_offset.clamp(-12.0, 14.0)),
            dst_rule(&self.dst),
        );
        Some((
            loc,
            LocalDateTime::new(y, m, d, f64::from(self.hour.clamp(0.0, 23.999))),
        ))
    }

    /// Altitude / azimuth (rad, azimuth from north clockwise) → unit vector
    /// in the scene frame.
    fn scene_dir(&self, alt: f64, az: f64) -> Vec3 {
        let (east, up, north) = (alt.cos() * az.sin(), alt.sin(), alt.cos() * az.cos());
        let t = self.north_deg.to_radians();
        let north_axis = Vec3::new(t.sin(), 0.0, -t.cos());
        let east_axis = Vec3::new(t.cos(), 0.0, t.sin());
        (east_axis * east as f32 + Vec3::Y * up as f32 + north_axis * north as f32)
            .normalize_or_zero()
    }

    /// All natural light then and there: the sun by day (its beam, the
    /// daylit sky, at least 400 lx while it is up), the twilight sky at
    /// dusk, the moon and the starlit sky at night; `None` for an
    /// unreadable date.
    pub fn sky(&self) -> Option<SkyLight> {
        let (loc, dt) = self.place_and_time()?;
        let moon = self.moon()?;
        let sun = dt.solar_position(&loc);
        let alt = sun.altitude_deg();
        let sun_dir = self.scene_dir(sun.altitude_rad, sun.azimuth_rad);
        let moon_sky = (moon.horizontal_lux - moon.dir.y.max(0.0) * moon.normal_lux).max(0.0);
        let (kind, dir, beam, cct, sky) = if alt > 0.0 {
            let a = DaylightAvailability::clear_sky(&sun, MOON_TURBIDITY);
            let beam_h = a.dni_lux * sun.altitude_rad.sin();
            // the clear-sky model has little sky light with a low sun: the
            // twilight value keeps the total at 400 lx at least
            let sky = a.dhi_lux.max(SUNSET_LUX - beam_h);
            let cct = daylight_cct(alt, MOON_TURBIDITY) as f32;
            (SkyKind::Sun, sun_dir, a.dni_lux as f32, cct, sky as f32)
        } else {
            let twilight = if alt > -18.0 {
                (twilight_lux(alt) - STARLIGHT_LUX).max(0.0) as f32
            } else {
                0.0
            };
            let kind = if twilight > moon.horizontal_lux {
                SkyKind::Twilight
            } else if moon.horizontal_lux > STARLIGHT_LUX as f32 {
                SkyKind::Moon
            } else {
                SkyKind::Night
            };
            (
                kind,
                moon.dir,
                if moon.altitude_deg > 0.0 {
                    moon.normal_lux
                } else {
                    0.0
                },
                MOON_CCT_K,
                twilight + STARLIGHT_LUX as f32 + moon_sky,
            )
        };
        Some(SkyLight {
            kind,
            dir,
            beam_lux: beam,
            beam_cct: cct,
            sky_lux: sky,
            horizontal_lux: beam * dir.y.max(0.0) + sky,
            sun_altitude_deg: alt as f32,
            sun_azimuth_deg: sun.azimuth_deg() as f32,
            moon,
        })
    }

    /// The moon then and there; `None` for an unreadable date.
    pub fn moon(&self) -> Option<MoonSky> {
        let (loc, dt) = self.place_and_time()?;
        let pos = dt.moon_position(&loc);
        let light = Moonlight::of(&pos, MOON_TURBIDITY);
        let dir = self.scene_dir(pos.altitude_rad, pos.azimuth_rad);
        Some(MoonSky {
            dir,
            normal_lux: light.normal_lux as f32,
            sky_lux: light.diffuse_horizontal_lux as f32,
            horizontal_lux: light.global_horizontal_lux as f32,
            altitude_deg: pos.altitude_deg() as f32,
            azimuth_deg: pos.azimuth_deg() as f32,
            illuminated: pos.illuminated_fraction as f32,
            waxing: pos.waxing,
            distance_km: pos.distance_km as f32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn berlin(date: &str, hour: f32, north: f32) -> MoonTime {
        MoonTime {
            date: date.into(),
            hour,
            latitude: 52.52,
            longitude: 13.40,
            utc_offset: 1.0,
            dst: "eu".into(),
            north_deg: north,
        }
    }

    #[test]
    fn a_winter_full_moon_stands_high_in_the_south_at_midnight() {
        let m = berlin("2026-01-03", 0.5, 0.0).moon().unwrap();
        assert!(m.illuminated > 0.95 && m.altitude_deg > 45.0, "{m:?}");
        // south = +Z in the scene, up = +Y
        assert!(m.dir.z > 0.3 && m.dir.y > 0.7, "{:?}", m.dir);
        assert!((m.dir.length() - 1.0).abs() < 1e-5);
        assert!(
            (0.2..0.4).contains(&m.horizontal_lux),
            "{}",
            m.horizontal_lux
        );
        assert!(m.sky_lux > 0.0 && m.sky_lux < m.horizontal_lux);
        // the scene turned so north is +X: south is −X
        let turned = berlin("2026-01-03", 0.5, 90.0).moon().unwrap();
        assert!(turned.dir.x < -0.3, "{:?}", turned.dir);
        assert!((turned.dir.y - m.dir.y).abs() < 1e-6);
    }

    #[test]
    fn the_next_full_moon_after_new_year_2026() {
        // full moons 2026-01-03 and 2026-02-01
        assert_eq!(
            next_full_moon(&berlin("2026-01-01", 23.0, 0.0)).as_deref(),
            Some("2026-01-03")
        );
        let feb = next_full_moon(&berlin("2026-01-05", 23.0, 0.0)).unwrap();
        assert!(feb == "2026-02-01" || feb == "2026-01-31", "{feb}");
    }

    #[test]
    fn a_summer_noon_is_sunlit_and_a_winter_midnight_moonlit() {
        // 13:00 on 21 June in Berlin: the sun high in the south, ~100 klx
        let noon = berlin("2026-06-21", 13.0, 0.0).sky().unwrap();
        assert_eq!(noon.kind, SkyKind::Sun);
        assert!(noon.sun_altitude_deg > 55.0, "{noon:?}");
        assert!(
            (60_000.0..130_000.0).contains(&noon.horizontal_lux),
            "{}",
            noon.horizontal_lux
        );
        assert!(noon.dir.z > 0.3 && noon.dir.y > 0.8, "{:?}", noon.dir);
        assert!(noon.beam_lux > noon.sky_lux && noon.sky_lux > 5_000.0);
        // just after sunset: twilight, a few hundred lux down to a few
        let dusk = berlin("2026-06-21", 22.0, 0.0).sky().unwrap();
        assert_eq!(dusk.kind, SkyKind::Twilight, "{dusk:?}");
        assert!(dusk.horizontal_lux < 400.0 && dusk.horizontal_lux > 0.01);
        // the full moon at midnight in January: moonlight, no sun
        let night = berlin("2026-01-03", 0.5, 0.0).sky().unwrap();
        assert_eq!(night.kind, SkyKind::Moon);
        assert!(
            (night.horizontal_lux - night.moon.horizontal_lux).abs() < 0.01,
            "{night:?}"
        );
        // a moonless midnight: starlight
        let dark = berlin("2026-01-18", 0.0, 0.0).sky().unwrap();
        assert_eq!(dark.kind, SkyKind::Night);
        assert!(dark.horizontal_lux < 0.01 && dark.beam_lux == 0.0);
    }

    #[test]
    fn no_moon_light_below_the_horizon_or_for_a_bad_date() {
        // new moon: up by day, below the horizon at midnight
        let m = berlin("2026-01-18", 0.0, 0.0).moon().unwrap();
        assert_eq!(m.horizontal_lux, 0.0);
        assert!(berlin("26-13-01", 0.0, 0.0).moon().is_none());
    }
}
