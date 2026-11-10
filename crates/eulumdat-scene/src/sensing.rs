//! Sensors in the light scene: which objects each sensor detects, and the
//! light output of each lamp under presence / motion control.
//!
//! All sensors use one sensor file and one height
//! (`scene::sensor_mount`). Detection is tested in 3D against the sensor's
//! detection volume for the motion type (`SensorFile::motion_volume`):
//! points over the object's ground outline (centre and corners) at several
//! heights up its body (person 0.1–1.6 m, car 0.3–1.3 m) — a person whose
//! feet are outside the floor footprint can still be seen at chest height
//! (wall sensors, high mounting near the end of the volume). Without a
//! volume the footprint on the ground (`SensorFile::area_at`, plane 0 m,
//! as the planning tools cut the walking plane) decides. Lamps run at full
//! output while their sensor detects something, else at
//! `SensorControl::idle_level`; with `SensorControl::dims` off the sensors
//! only detect and every lamp stays at full output (a plan from a designer
//! arrives so, to show the designed lighting).

use eulumdat::sensor::{DetectionArea, DetectionVolume, Motion};
use glam::{Vec2, Vec3};

use crate::scene::{BuiltScene, Movable, ObjectId};

/// Which sensor switches which lamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlMode {
    /// One group: every lamp is on while any sensor detects something.
    Together,
    /// Each lamp follows the sensor nearest to it.
    Nearest,
}

impl ControlMode {
    pub fn id(self) -> &'static str {
        match self {
            ControlMode::Together => "together",
            ControlMode::Nearest => "nearest",
        }
    }

    pub fn from_id(id: &str) -> Self {
        if id == "nearest" {
            ControlMode::Nearest
        } else {
            ControlMode::Together
        }
    }
}

/// How the sensors control the lamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorControl {
    pub mode: ControlMode,
    /// Output (0–1) of a lamp whose sensor detects nothing.
    pub idle_level: f32,
    /// Motion type the detection area is taken for.
    pub motion: Motion,
    /// The sensors switch the lamps; off = they only detect, every lamp
    /// at full output.
    pub dims: bool,
}

impl Default for SensorControl {
    fn default() -> Self {
        Self {
            mode: ControlMode::Together,
            idle_level: 0.1,
            motion: Motion::Tangential,
            dims: true,
        }
    }
}

/// What the sensors see and what the lamps do.
#[derive(Clone, Debug, Default)]
pub struct Sensing {
    /// Detection area of one sensor on the ground (`None`: no sensor file,
    /// no data for the motion type, or nothing detected at that height).
    pub area: Option<DetectionArea>,
    /// Per sensor (`ObjectId::Sensor` order): the objects it detects.
    pub detected: Vec<Vec<ObjectId>>,
    /// Per lamp: output 0–1.
    pub lamp_level: Vec<f32>,
}

impl Sensing {
    /// Some sensor detects something.
    pub fn any(&self) -> bool {
        self.detected.iter().any(|d| !d.is_empty())
    }

    /// Lamps at full output.
    pub fn lamps_on(&self) -> usize {
        self.lamp_level.iter().filter(|&&l| l >= 0.999).count()
    }
}

/// Ground points of an object for the detection test: its centre and the
/// corners of its outline.
fn target_points(m: &Movable) -> Vec<Vec2> {
    let c = Vec2::new(m.anchor.x, m.anchor.z);
    if m.half == Vec2::ZERO {
        return vec![c];
    }
    let (sin, cos) = m.yaw_deg.to_radians().sin_cos();
    // local +X turned by the yaw around the vertical (towards −Z for a
    // positive yaw, as `Mat3::from_rotation_y` does)
    let along = Vec2::new(cos, -sin);
    let across = Vec2::new(sin, cos);
    let mut pts = vec![c];
    for (a, b) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        pts.push(c + along * (a * m.half.x) + across * (b * m.half.y));
    }
    pts
}

/// Heights (m above the object's base) tested up a person's / a car's body.
const PERSON_HEIGHTS: [f32; 4] = [0.1, 0.6, 1.1, 1.6];
const CAR_HEIGHTS: [f32; 3] = [0.3, 0.8, 1.3];

/// Points of an object for the detection test: its ground points (centre,
/// corners) at the heights up its body.
fn target_points_3d(m: &Movable) -> Vec<Vec3> {
    let heights: &[f32] = match m.id {
        ObjectId::Car(_) => &CAR_HEIGHTS,
        _ => &PERSON_HEIGHTS,
    };
    let mut pts = Vec::new();
    for g in target_points(m) {
        for &h in heights {
            pts.push(Vec3::new(g.x, m.anchor.y + h, g.y));
        }
    }
    pts
}

/// The point `p` lies in the detection volume of a sensor whose head is at
/// `head` (C0 = +X, C90 = +Z, depth below the head).
pub fn volume_contains(volume: &DetectionVolume, head: Vec3, p: Vec3) -> bool {
    let d = p - head;
    volume.contains(d.x as f64, d.z as f64, -d.y as f64)
}

/// Sensors, targets and lamp levels for a built scene: `volume` (3D test)
/// when there is one, else the ground `area`.
pub fn evaluate(
    scene: &BuiltScene,
    area: Option<&DetectionArea>,
    volume: Option<&DetectionVolume>,
    control: &SensorControl,
) -> Sensing {
    let mut sensors: Vec<(u8, Vec3)> = scene
        .movables
        .iter()
        .filter_map(|m| match m.id {
            ObjectId::Sensor(i) => Some((i, m.handle)),
            _ => None,
        })
        .collect();
    sensors.sort_by_key(|s| s.0);
    let targets: Vec<(ObjectId, Vec<Vec2>, Vec<Vec3>)> = scene
        .movables
        .iter()
        .filter(|m| matches!(m.id, ObjectId::Person | ObjectId::Car(_)))
        .map(|m| (m.id, target_points(m), target_points_3d(m)))
        .collect();
    let detected: Vec<Vec<ObjectId>> = sensors
        .iter()
        .map(|&(_, head)| {
            let s = Vec2::new(head.x, head.z);
            targets
                .iter()
                .filter(|(_, ground, body)| match (volume, area) {
                    (Some(v), _) => body.iter().any(|p| volume_contains(v, head, *p)),
                    (None, Some(a)) if !a.footprint.is_empty() => ground.iter().any(|p| {
                        let d = *p - s;
                        // plan x = world x, plan y = world z (C90 = +Z)
                        a.footprint.contains(d.x as f64, d.y as f64)
                    }),
                    _ => false,
                })
                .map(|(id, _, _)| *id)
                .collect()
        })
        .collect();
    let idle = control.idle_level.clamp(0.0, 1.0);
    let lamp_level = if sensors.is_empty() || !control.dims {
        vec![1.0; scene.lamps.len()]
    } else {
        match control.mode {
            ControlMode::Together => {
                let on = detected.iter().any(|d| !d.is_empty());
                vec![if on { 1.0 } else { idle }; scene.lamps.len()]
            }
            ControlMode::Nearest => scene
                .lamps
                .iter()
                .map(|l| {
                    let lp = Vec2::new(l.position.x, l.position.z);
                    let xz = |v: Vec3| Vec2::new(v.x, v.z);
                    let nearest = sensors
                        .iter()
                        .enumerate()
                        .min_by(|a, b| {
                            (xz(a.1 .1) - lp)
                                .length_squared()
                                .total_cmp(&(xz(b.1 .1) - lp).length_squared())
                        })
                        .map(|(k, _)| k);
                    match nearest {
                        Some(k) if !detected[k].is_empty() => 1.0,
                        _ => idle,
                    }
                })
                .collect(),
        }
    };
    Sensing {
        area: area.cloned(),
        detected,
        lamp_level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{build_scene, SceneKind, SceneParams, SceneSensor};
    use eulumdat::sensor::SensorFile;

    fn ceiling() -> SensorFile {
        let ldt = include_str!("../../eulumdat/tests/fixtures/sensors/demo_ceiling_360.ldt");
        SensorFile::from_bytes(ldt.as_bytes(), "sensor/sensldt").unwrap()
    }

    #[test]
    fn person_under_a_sensor_switches_the_lamps() {
        let file = ceiling();
        let mut p = SceneParams::defaults(SceneKind::Parking);
        p.count = 3;
        p.sensor_height = 3.0;
        let area = file.area_at(Motion::Tangential, 3.0, 0.0, false).unwrap();
        let person = build_scene(&p)
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap()
            .anchor;
        let control = SensorControl::default();

        // a sensor right above the person
        p.sensors = vec![SceneSensor {
            x: person.x,
            z: person.z,
        }];
        let s = evaluate(&build_scene(&p), Some(&area), None, &control);
        // (parked cars nearby are seen too: the area is ~15 m across)
        assert!(s.detected[0].contains(&ObjectId::Person));
        assert_eq!(s.lamp_level, vec![1.0; 3]);
        assert_eq!(s.lamps_on(), 3);

        // far away: nothing detected, the lamps idle
        p.sensors[0].x += 40.0;
        let s = evaluate(&build_scene(&p), Some(&area), None, &control);
        assert!(!s.any());
        assert_eq!(s.lamp_level, vec![0.1; 3]);
        // sensors that only detect (a plan from a designer): full output
        let only_detect = SensorControl {
            dims: false,
            ..control
        };
        let s = evaluate(&build_scene(&p), Some(&area), None, &only_detect);
        assert!(!s.any());
        assert_eq!(s.lamp_level, vec![1.0; 3]);

        // the person moves under the sensor
        p.moves = vec![(ObjectId::Person, Vec2::new(p.sensors[0].x, p.sensors[0].z))];
        let s = evaluate(&build_scene(&p), Some(&area), None, &control);
        assert_eq!(s.detected, vec![vec![ObjectId::Person]]);

        // no sensor file: nothing detected
        let s = evaluate(&build_scene(&p), None, None, &control);
        assert!(!s.any());
        assert_eq!(s.lamp_level, vec![0.1; 3]);
    }

    #[test]
    fn nearest_sensor_controls_its_lamps() {
        let file = ceiling();
        let mut p = SceneParams::defaults(SceneKind::Parking);
        p.count = 3;
        let scene = build_scene(&p);
        let lamps: Vec<Vec2> = scene
            .lamps
            .iter()
            .map(|l| Vec2::new(l.position.x, l.position.z))
            .collect();
        // one sensor at the first and one at the last lamp; the person
        // under the last one
        p.sensors = vec![
            SceneSensor {
                x: lamps[0].x,
                z: lamps[0].y,
            },
            SceneSensor {
                x: lamps[2].x,
                z: lamps[2].y,
            },
        ];
        p.moves = vec![(ObjectId::Person, lamps[2])];
        p.sensor_height = 3.0;
        let area = file.area_at(Motion::Tangential, 3.0, 0.0, false).unwrap();
        let control = SensorControl {
            mode: ControlMode::Nearest,
            idle_level: 0.0,
            ..Default::default()
        };
        let s = evaluate(&build_scene(&p), Some(&area), None, &control);
        assert!(s.detected[0].is_empty());
        assert_eq!(s.detected[1], vec![ObjectId::Person]);
        assert_eq!(s.lamp_level[0], 0.0);
        assert_eq!(s.lamp_level[2], 1.0);
    }

    #[test]
    fn a_car_is_detected_by_its_corner() {
        let file = ceiling();
        let mut p = SceneParams::defaults(SceneKind::Parking);
        p.show_figure = false;
        let scene = build_scene(&p);
        let car = scene
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Car(0))
            .unwrap()
            .clone();
        let area = file.area_at(Motion::Tangential, 3.0, 0.0, false).unwrap();
        let reach = area.footprint.max_range as f32;
        // the sensor stands beyond the car's centre reach but close to a corner
        let corner = target_points(&car)[1];
        let dir = (corner - Vec2::new(car.anchor.x, car.anchor.z)).normalize();
        let at = corner + dir * (0.9 * reach);
        let centre = Vec2::new(car.anchor.x, car.anchor.z) - at;
        assert!(!area.footprint.contains(centre.x as f64, centre.y as f64));
        p.sensor_height = 3.0;
        p.sensors = vec![SceneSensor { x: at.x, z: at.y }];
        let s = evaluate(
            &build_scene(&p),
            Some(&area),
            None,
            &SensorControl::default(),
        );
        assert!(s.detected[0].contains(&ObjectId::Car(0)));
    }

    #[test]
    fn the_body_is_seen_where_the_floor_is_out_of_reach() {
        // a ceiling sensor mounted above the end of its volume: nothing at
        // the floor, but a person standing below is seen at chest height
        let file = ceiling();
        let volume = file.motion_volume(Motion::Tangential, false).unwrap();
        let limit = volume.depth_limit(1.0) as f32;
        let mount = limit + 0.4;
        assert!(file
            .area_at(Motion::Tangential, mount as f64, 0.0, false)
            .is_none_or(|a| a.footprint.is_empty()));
        let mut p = SceneParams::defaults(SceneKind::Parking);
        p.sensor_height = mount;
        let person = build_scene(&p)
            .movables
            .iter()
            .find(|m| m.id == ObjectId::Person)
            .unwrap()
            .anchor;
        p.sensors = vec![SceneSensor {
            x: person.x,
            z: person.z,
        }];
        let scene = build_scene(&p);
        let control = SensorControl::default();
        let flat = evaluate(&scene, None, None, &control);
        assert!(!flat.any());
        let solid = evaluate(&scene, None, Some(&volume), &control);
        assert!(solid.detected[0].contains(&ObjectId::Person));
        // the 3D test agrees with the floor footprint near the floor
        let head = Vec3::new(person.x, 3.0, person.z);
        let low = file.motion_volume(Motion::Tangential, false).unwrap();
        let area = file.area_at(Motion::Tangential, 3.0, 0.0, false).unwrap();
        for k in 0..36 {
            let c = (k as f32 * 10.0).to_radians();
            let r = area.footprint.reach_towards((k * 10) as f64) as f32;
            let dir = Vec3::new(c.cos(), 0.0, c.sin());
            let inside = head + dir * (0.97 * r) - Vec3::Y * 2.99;
            let outside = head + dir * (1.03 * r + 0.02) - Vec3::Y * 2.99;
            assert!(volume_contains(&low, head, inside), "C{}", k * 10);
            assert!(!volume_contains(&low, head, outside), "C{}", k * 10);
        }
    }
}
