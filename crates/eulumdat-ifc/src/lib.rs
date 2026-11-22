//! Rooms, luminaires, doors and windows from an IFC building model, for the
//! lighting designers (Interior designer, Light scene).
//!
//! The IFC file is read with bimifc (`bimifc-parser`, `bimifc-geometry`):
//! - **Rooms** (`IfcSpace`): the outline is the boundary of the space
//!   mesh's bottom faces (works for extrusions and breps), the height is
//!   the mesh's vertical extent, the floor its lowest point.
//! - **Luminaires** (`IfcLightFixture`): position and direction from the
//!   full placement chain (the luminaire's local X axis is taken as C0),
//!   height = the bottom of its body (≈ the luminous face); photometry
//!   from `Pset_Photometry.EulumdatData`, `Pset_GLDF_LDTRawContent`
//!   (base64, as gldf.icu writes it) or an `IfcLightSourceGoniometric`.
//! - **Doors and windows**: bounding box of their body; doors flagged
//!   `FireExit` in `Pset_DoorCommon` are exits.
//!
//! [`Building::plan`] turns one room into the designers' frame: origin at
//! the outline's bounds, x east, y south (north up on the plan).

use std::collections::HashMap;

#[cfg(feature = "export")]
pub mod export;

use base64::Engine;
use bimifc_geometry::GeometryRouter;
use bimifc_model::{DecodedEntity, EntityId, EntityResolver, IfcModel, IfcType, PropertySet};

/// A building storey.
#[derive(Clone, Debug)]
pub struct Storey {
    pub id: u32,
    pub name: String,
    /// Elevation (m).
    pub elevation: f64,
}

/// A room (`IfcSpace`).
#[derive(Clone, Debug)]
pub struct Room {
    pub id: u32,
    /// Long name and number ("Wohnen (1.01)"), else the name.
    pub name: String,
    /// Index in [`Building::storeys`].
    pub storey: Option<usize>,
    /// Outline in world x, y (m), counter-clockwise.
    pub outline: Vec<[f64; 2]>,
    /// Floor level (m).
    pub floor: f64,
    /// Clear height (m).
    pub height: f64,
    /// Floor area (m², net of holes).
    pub area: f64,
}

/// A luminaire (`IfcLightFixture`).
#[derive(Clone, Debug)]
pub struct Luminaire {
    pub id: u32,
    pub name: String,
    /// World position (m); z = the bottom of its body (the top for a body
    /// standing on the floor).
    pub position: [f64; 3],
    /// Direction of C0 (the luminaire's local X), degrees counter-clockwise
    /// from world +X.
    pub rotation_deg: f64,
    /// Index in [`Building::rooms`].
    pub room: Option<usize>,
    /// Index in [`Building::photometries`].
    pub photometry: Option<usize>,
    /// Emergency (security) lighting: `Pset_LightFixtureTypeSecurityLighting`
    /// on the luminaire, or IFC4 PredefinedType `SECURITYLIGHTING`.
    pub emergency: bool,
}

/// A door or a window.
#[derive(Clone, Debug)]
pub struct Opening {
    pub id: u32,
    pub name: String,
    /// Middle of the body in x, y; z = its bottom (m).
    pub centre: [f64; 3],
    /// Width (m, the longer horizontal side of the body).
    pub width: f64,
    pub height: f64,
    /// `Pset_DoorCommon.FireExit` (doors only).
    pub fire_exit: bool,
    /// Rooms it opens into (indices in [`Building::rooms`]).
    pub rooms: Vec<usize>,
}

impl Opening {
    /// A door into one room only: it leads outside (or to a space the
    /// model does not have).
    pub fn external(&self) -> bool {
        self.rooms.len() == 1
    }

    /// An exit for emergency planning: flagged `FireExit`, or external.
    pub fn exit(&self) -> bool {
        self.fire_exit || self.external()
    }
}

/// A photometry found in the model (identical LDTs are one entry).
#[derive(Clone, Debug)]
pub struct Photometry {
    /// Luminaire name of the LDT, else the first fixture's name.
    pub name: String,
    pub ldt: String,
    /// Luminaires that use it.
    pub count: usize,
}

#[cfg(feature = "export")]
impl From<&Photometry> for export::ExportPhotometry {
    fn from(p: &Photometry) -> Self {
        Self {
            name: p.name.clone(),
            ldt: p.ldt.clone(),
        }
    }
}

/// What a model holds for lighting design.
#[derive(Clone, Debug, Default)]
pub struct Building {
    pub storeys: Vec<Storey>,
    pub rooms: Vec<Room>,
    pub luminaires: Vec<Luminaire>,
    pub doors: Vec<Opening>,
    pub windows: Vec<Opening>,
    pub photometries: Vec<Photometry>,
    /// Things skipped or guessed (English).
    pub warnings: Vec<String>,
}

/// One room in the designers' frame: x east from the outline's west edge,
/// y south from its north edge (so north is up on the plan), z above the
/// floor; angles counter-clockwise from +x in this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomPlan {
    pub name: String,
    /// Extent along x (m).
    pub length: f64,
    /// Extent along y (m).
    pub width: f64,
    pub height: f64,
    pub area: f64,
    pub outline: Vec<(f64, f64)>,
    pub luminaires: Vec<PlanLuminaire>,
    pub doors: Vec<PlanOpening>,
    pub windows: Vec<PlanOpening>,
    /// Where the plan's origin lies in the world (x, y of the north-west
    /// corner of the bounds, floor z).
    pub origin: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanLuminaire {
    pub x: f64,
    pub y: f64,
    /// Above the floor (m).
    pub z: f64,
    /// C0 direction in the plan frame (deg).
    pub rotation_deg: f64,
    pub photometry: Option<usize>,
    pub name: String,
    /// Emergency (security) lighting.
    pub emergency: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanOpening {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Above the floor (m).
    pub sill: f64,
    pub fire_exit: bool,
    /// Flagged `FireExit` or leading outside (doors).
    pub exit: bool,
    pub name: String,
}

const WELD: f64 = 1e-3;

impl Building {
    /// Read an IFC file (IFC2x3 / IFC4 STEP, IFC5 IFCX).
    pub fn parse(text: &str) -> Result<Self, String> {
        let model = bimifc_parser::parse_auto(text).map_err(|e| e.to_string())?;
        Ok(Self::from_model(model.as_ref()))
    }

    /// Read a parsed model.
    pub fn from_model(model: &dyn IfcModel) -> Self {
        let resolver = model.resolver();
        let props = model.properties();
        let unit = model.unit_scale();
        let router = GeometryRouter::with_default_processors_and_unit_scale(unit);
        let mut b = Building::default();

        let mut storey_index: HashMap<u32, usize> = HashMap::new();
        // elevation from the storey itself (attribute 9, file units → m):
        // bimifc's StoreyInfo keeps the file's unit (mm in many files)
        let mut storeys: Vec<Storey> = model
            .spatial()
            .storeys()
            .into_iter()
            .map(|s| Storey {
                id: s.id.0,
                name: decode_step(&s.name),
                elevation: resolver
                    .get(s.id)
                    .and_then(|e| e.get_float(9))
                    .map_or(s.elevation as f64 * unit, |z| z * unit),
            })
            .collect();
        storeys.sort_by(|a, c| a.elevation.total_cmp(&c.elevation));
        for s in storeys {
            storey_index.insert(s.id, b.storeys.len());
            b.storeys.push(s);
        }
        let storey_of = |id: EntityId, z: f64, b: &Building| -> Option<usize> {
            model
                .spatial()
                .containing_storey(id)
                .and_then(|s| storey_index.get(&s.0).copied())
                .or_else(|| {
                    // the highest storey at or below z
                    b.storeys
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| s.elevation <= z + 0.05)
                        .map(|(i, _)| i)
                        .next_back()
                })
        };

        // Rooms.
        for e in resolver.entities_by_type(&IfcType::IfcSpace) {
            let Ok(mesh) = router.process_element(&e, resolver) else {
                b.warnings
                    .push(format!("Room {}: its geometry could not be read", e.id));
                continue;
            };
            let Some(floor) = footprint(&mesh.positions, &mesh.indices) else {
                b.warnings
                    .push(format!("Room {}: no floor found in its geometry", e.id));
                continue;
            };
            let name = room_name(&e, props.name(e.id));
            let storey = storey_of(e.id, floor.z0, &b);
            b.rooms.push(Room {
                id: e.id.0,
                name,
                storey,
                outline: floor.outline,
                floor: floor.z0,
                height: floor.z1 - floor.z0,
                area: floor.area,
            });
        }
        b.rooms
            .sort_by(|a, c| (a.storey, a.name.as_str()).cmp(&(c.storey, c.name.as_str())));

        // Photometry per fixture (gldf.icu exports, Relux-style goniometric
        // sources, embedded EULUMDAT).
        let lighting = bimifc_parser::extract_lighting_data(resolver);
        let mut from_sources: HashMap<u32, String> = HashMap::new();
        for f in &lighting.light_fixtures {
            if let Some(ldt) = f
                .light_sources
                .iter()
                .find_map(bimifc_parser::light_source_to_ldt)
            {
                from_sources.insert(f.id as u32, ldt);
            }
        }
        let mut ldt_index: HashMap<String, usize> = HashMap::new();

        for e in resolver.entities_by_type(&IfcType::IfcLightFixture) {
            let (origin, x_axis) = placement(&e, resolver, unit);
            let body = router
                .process_element(&e, resolver)
                .ok()
                .and_then(|m| bounds(&m.positions));
            // light centre: the bottom of the body; a body standing on the
            // floor (floor lamp, bollard) gives off light at its head
            let probe = [
                origin[0],
                origin[1],
                body.map_or(origin[2], |(lo, _)| lo[2]),
            ];
            let floor = room_at(&b.rooms, probe).map(|i| b.rooms[i].floor);
            let z = match (body, floor) {
                (Some((lo, hi)), Some(f)) if lo[2] - f < 0.3 && hi[2] - lo[2] > 0.5 => hi[2],
                (Some((lo, _)), _) => lo[2],
                (None, _) => origin[2],
            };
            let name = props
                .name(e.id)
                .map(|n| decode_step(&n))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("Luminaire {}", e.id));
            let sets = props.property_sets(e.id);
            // IfcLightFixture (IFC4): 8 PredefinedType
            let emergency = sets
                .iter()
                .any(|s| s.name == "Pset_LightFixtureTypeSecurityLighting")
                || e.get_enum(8)
                    .is_some_and(|t| t.eq_ignore_ascii_case("SECURITYLIGHTING"));
            let ldt = embedded_ldt(&sets)
                .or_else(|| from_sources.get(&e.id.0).cloned())
                .filter(|t| eulumdat::Eulumdat::parse(t).is_ok());
            let photometry = ldt.map(|t| {
                let n = b.photometries.len();
                let i = *ldt_index.entry(t.clone()).or_insert(n);
                if i == n {
                    let ldt_name = eulumdat::Eulumdat::parse(&t)
                        .ok()
                        .map(|l| decode_step(l.luminaire_name.trim()))
                        .filter(|s| !s.is_empty());
                    b.photometries.push(Photometry {
                        name: ldt_name.unwrap_or_else(|| name.clone()),
                        ldt: t,
                        count: 0,
                    });
                }
                b.photometries[i].count += 1;
                i
            });
            let position = [origin[0], origin[1], z];
            let room = room_at(&b.rooms, position);
            b.luminaires.push(Luminaire {
                id: e.id.0,
                name,
                position,
                rotation_deg: x_axis[1].atan2(x_axis[0]).to_degrees().rem_euclid(360.0),
                room,
                photometry,
                emergency,
            });
        }
        let without = b
            .luminaires
            .iter()
            .filter(|l| l.photometry.is_none())
            .count();
        if without > 0 {
            b.warnings.push(format!(
                "{without} luminaire(s) carry no photometry in the model"
            ));
        }

        // Doors and windows.
        for (kind, types) in [
            (true, ["IFCDOOR", "IFCDOORSTANDARDCASE"]),
            (false, ["IFCWINDOW", "IFCWINDOWSTANDARDCASE"]),
        ] {
            for t in types {
                for e in resolver.find_by_type_name(t) {
                    let Some((lo, hi)) = router
                        .process_element(&e, resolver)
                        .ok()
                        .and_then(|m| bounds(&m.positions))
                    else {
                        continue;
                    };
                    let sets = props.property_sets(e.id);
                    let fire_exit = kind && flag(&sets, "Pset_DoorCommon", "FireExit");
                    let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, lo[2]];
                    let o = Opening {
                        id: e.id.0,
                        name: props
                            .name(e.id)
                            .map(|n| decode_step(&n))
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| {
                                format!("{} {}", if kind { "Door" } else { "Window" }, e.id)
                            }),
                        centre,
                        width: (hi[0] - lo[0]).max(hi[1] - lo[1]),
                        height: hi[2] - lo[2],
                        fire_exit,
                        rooms: rooms_beside(&b.rooms, centre, hi[2]),
                    };
                    if kind {
                        b.doors.push(o);
                    } else {
                        b.windows.push(o);
                    }
                }
            }
        }
        b
    }

    /// Luminaires in room `room`.
    pub fn luminaires_in(&self, room: usize) -> impl Iterator<Item = &Luminaire> {
        self.luminaires.iter().filter(move |l| l.room == Some(room))
    }

    /// Room `room` in the designers' frame (see [`RoomPlan`]).
    pub fn plan(&self, room: usize) -> RoomPlan {
        let r = &self.rooms[room];
        let (x0, x1) = r
            .outline
            .iter()
            .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
        let (y0, y1) = r
            .outline
            .iter()
            .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
        // y flipped: north (world +y) up on a plan whose y runs down
        let to_plan = |x: f64, y: f64| (x - x0, y1 - y);
        let opening = |o: &Opening| {
            let (x, y) = to_plan(o.centre[0], o.centre[1]);
            PlanOpening {
                x,
                y,
                width: o.width,
                height: o.height,
                sill: o.centre[2] - r.floor,
                fire_exit: o.fire_exit,
                exit: o.exit(),
                name: o.name.clone(),
            }
        };
        RoomPlan {
            name: r.name.clone(),
            length: x1 - x0,
            width: y1 - y0,
            height: r.height,
            area: r.area,
            outline: r.outline.iter().map(|p| to_plan(p[0], p[1])).collect(),
            luminaires: self
                .luminaires_in(room)
                .map(|l| {
                    let (x, y) = to_plan(l.position[0], l.position[1]);
                    PlanLuminaire {
                        x,
                        y,
                        z: l.position[2] - r.floor,
                        rotation_deg: (-l.rotation_deg).rem_euclid(360.0),
                        photometry: l.photometry,
                        name: l.name.clone(),
                        emergency: l.emergency,
                    }
                })
                .collect(),
            doors: self
                .doors
                .iter()
                .filter(|d| d.rooms.contains(&room))
                .map(opening)
                .collect(),
            windows: self
                .windows
                .iter()
                .filter(|w| w.rooms.contains(&room))
                .map(opening)
                .collect(),
            origin: [x0, y1, r.floor],
        }
    }
}

/// Text from a STEP file with its escapes decoded (ISO 10303-21:
/// `\X2\00FC\X0\` UTF-16, `\X4\…\X0\` UTF-32, `\X\FC` and `\S\|`
/// Latin-1, `\\` backslash, `\P?\` page switches dropped).
pub fn decode_step(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let hex = |h: &[u8]| {
        std::str::from_utf8(h)
            .ok()
            .and_then(|t| u32::from_str_radix(t, 16).ok())
    };
    while i < b.len() {
        if b[i] != b'\\' {
            let ch = s[i..].chars().next().unwrap_or('?');
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"\\X2\\") || rest.starts_with(b"\\X4\\") {
            let w = if rest[2] == b'2' { 4 } else { 8 };
            let mut j = i + 4;
            let mut units: Vec<u16> = Vec::new();
            while j + w <= b.len() && b[j] != b'\\' {
                if let Some(v) = hex(&b[j..j + w]) {
                    if w == 4 {
                        units.push(v as u16);
                    } else if let Some(c) = char::from_u32(v) {
                        let mut buf = [0u16; 2];
                        units.extend_from_slice(c.encode_utf16(&mut buf));
                    }
                }
                j += w;
            }
            out.push_str(&String::from_utf16_lossy(&units));
            // skip the closing \X0\
            i = if b[j..].starts_with(b"\\X0\\") {
                j + 4
            } else {
                j
            };
        } else if rest.starts_with(b"\\X\\") && rest.len() >= 5 {
            if let Some(c) = hex(&rest[3..5]).and_then(char::from_u32) {
                out.push(c);
            }
            i += 5;
        } else if rest.starts_with(b"\\S\\") && rest.len() >= 4 {
            out.push(char::from_u32(rest[3] as u32 + 128).unwrap_or('?'));
            i += 4;
        } else if rest.starts_with(b"\\P") && rest.len() >= 4 && rest[3] == b'\\' {
            i += 4;
        } else if rest.starts_with(b"\\\\") {
            out.push('\\');
            i += 2;
        } else {
            out.push('\\');
            i += 1;
        }
    }
    out
}

/// "Long name (Name)" of a space, else whichever there is.
fn room_name(e: &DecodedEntity, name: Option<String>) -> String {
    // IfcSpace: 2 Name, 7 LongName
    let long = e
        .get_string(7)
        .map(|s| decode_step(s.trim()))
        .filter(|s| !s.is_empty());
    let name = name
        .map(|s| decode_step(s.trim()))
        .filter(|s| !s.is_empty());
    match (long, name) {
        (Some(l), Some(n)) if l != n => format!("{l} ({n})"),
        (Some(l), _) => l,
        (None, Some(n)) => n,
        (None, None) => format!("Room {}", e.id),
    }
}

/// World origin and local X axis of a product's placement (m).
fn placement(e: &DecodedEntity, resolver: &dyn EntityResolver, unit: f64) -> ([f64; 3], [f64; 2]) {
    // IfcProduct: 5 ObjectPlacement
    let m = e
        .get_ref(5)
        .and_then(|p| bimifc_geometry::transform::resolve_placement(p, resolver));
    match m {
        Some(m) => {
            let (ax, ay) = (m[(0, 0)], m[(1, 0)]);
            let len = ax.hypot(ay);
            (
                [m[(0, 3)] * unit, m[(1, 3)] * unit, m[(2, 3)] * unit],
                if len > 1e-9 {
                    [ax / len, ay / len]
                } else {
                    [1.0, 0.0]
                },
            )
        }
        None => ([0.0; 3], [1.0, 0.0]),
    }
}

/// Bounding box of a mesh (`positions` = x, y, z triples).
fn bounds(p: &[f32]) -> Option<([f64; 3], [f64; 3])> {
    if p.len() < 3 {
        return None;
    }
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for v in p.as_chunks::<3>().0 {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k] as f64);
            hi[k] = hi[k].max(v[k] as f64);
        }
    }
    Some((lo, hi))
}

/// LDT text embedded in a fixture's property sets.
fn embedded_ldt(sets: &[PropertySet]) -> Option<String> {
    let decode = |v: &str| -> Option<String> {
        let v = v.trim().trim_matches('\'');
        if looks_like_ldt(v) {
            return Some(v.to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(v.split_whitespace().collect::<String>())
            .ok()?;
        let text = String::from_utf8(bytes.clone())
            .unwrap_or_else(|_| bytes.iter().map(|&c| c as char).collect());
        looks_like_ldt(&text).then_some(text)
    };
    for s in sets {
        if s.name == "Pset_Photometry" {
            if let Some(t) = s
                .properties
                .iter()
                .filter(|p| p.name == "EulumdatData")
                .find_map(|p| decode(&p.value))
            {
                return Some(t);
            }
        }
        if s.name == "Pset_GLDF_LDTRawContent" {
            let mut parts: Vec<_> = s
                .properties
                .iter()
                .filter(|p| p.name.contains("Content"))
                .collect();
            parts.sort_by(|a, c| a.name.cmp(&c.name));
            if let Some(t) = parts.iter().find_map(|p| decode(&p.value)) {
                return Some(t);
            }
        }
    }
    None
}

/// Enough lines for an EULUMDAT header.
fn looks_like_ldt(t: &str) -> bool {
    t.lines().count() > 40
}

/// A boolean property ("true", ".T.", "TRUE", "1").
fn flag(sets: &[PropertySet], set: &str, name: &str) -> bool {
    sets.iter()
        .filter(|s| s.name == set)
        .flat_map(|s| s.properties.iter())
        .filter(|p| p.name == name)
        .any(|p| {
            let v = p.value.trim().trim_matches('.').to_ascii_lowercase();
            v == "t" || v == "true" || v == "1" || v == "yes"
        })
}

/// The room whose floor area holds `p` (x, y inside, z between its floor
/// and ceiling, with some slack for luminaires in the ceiling).
fn room_at(rooms: &[Room], p: [f64; 3]) -> Option<usize> {
    rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| p[2] >= r.floor - 0.1 && p[2] <= r.floor + r.height + 0.5)
        .filter(|(_, r)| contains(&r.outline, p[0], p[1]))
        .min_by(|a, c| a.1.area.total_cmp(&c.1.area))
        .map(|(i, _)| i)
}

/// Rooms a door or window opens into: its middle within 0.6 m of their
/// outline (it sits in the wall) and its bottom at their floor level.
fn rooms_beside(rooms: &[Room], c: [f64; 3], top: f64) -> Vec<usize> {
    rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| c[2] < r.floor + r.height && top > r.floor + 0.1)
        .filter(|(_, r)| {
            contains(&r.outline, c[0], c[1]) || distance_to_outline(&r.outline, c[0], c[1]) < 0.6
        })
        .map(|(i, _)| i)
        .collect()
}

/// Point in polygon (even–odd).
pub fn contains(poly: &[[f64; 2]], x: f64, y: f64) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

fn distance_to_outline(poly: &[[f64; 2]], x: f64, y: f64) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = (dx * dx + dy * dy).max(1e-12);
            let t = (((x - a[0]) * dx + (y - a[1]) * dy) / len2).clamp(0.0, 1.0);
            (a[0] + t * dx - x).hypot(a[1] + t * dy - y)
        })
        .fold(f64::MAX, f64::min)
}

struct Footprint {
    outline: Vec<[f64; 2]>,
    z0: f64,
    z1: f64,
    area: f64,
}

/// The floor of a closed mesh: the boundary of its bottom faces (the
/// largest loop; holes are left out of the outline but not of the area).
fn footprint(p: &[f32], idx: &[u32]) -> Option<Footprint> {
    let (lo, hi) = bounds(p)?;
    let (z0, z1) = (lo[2], hi[2]);
    let v = |i: u32| -> [f64; 3] {
        let k = i as usize * 3;
        [p[k] as f64, p[k + 1] as f64, p[k + 2] as f64]
    };
    // weld by position: one id per point (1 mm)
    let mut ids: HashMap<(i64, i64), usize> = HashMap::new();
    let mut pts: Vec<[f64; 2]> = Vec::new();
    let mut id_of = |q: [f64; 3]| -> usize {
        let key = ((q[0] / WELD).round() as i64, (q[1] / WELD).round() as i64);
        *ids.entry(key).or_insert_with(|| {
            pts.push([q[0], q[1]]);
            pts.len() - 1
        })
    };
    let mut edges: HashMap<(usize, usize), (usize, usize, i32)> = HashMap::new();
    let mut area = 0.0;
    for t in idx.as_chunks::<3>().0 {
        let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
        if [a, b, c].iter().any(|q| (q[2] - z0).abs() > 0.01) {
            continue;
        }
        let tri_area = ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0;
        if tri_area.abs() < 1e-9 {
            continue;
        }
        area += tri_area.abs();
        let (ia, ib, ic) = (id_of(a), id_of(b), id_of(c));
        for (s, e) in [(ia, ib), (ib, ic), (ic, ia)] {
            let key = (s.min(e), s.max(e));
            edges.entry(key).or_insert((s, e, 0)).2 += 1;
        }
    }
    if area < 0.05 {
        return None;
    }
    // boundary = edges used once; chain them
    let mut next: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(s, e, n) in edges.values() {
        if n == 1 {
            next.entry(s).or_default().push(e);
        }
    }
    let mut loops: Vec<Vec<usize>> = Vec::new();
    let mut starts: Vec<usize> = next.keys().copied().collect();
    starts.sort_unstable();
    for s in starts {
        while next.get(&s).is_some_and(|v| !v.is_empty()) {
            let mut lp = vec![s];
            let mut cur = s;
            while let Some(n) = next.get_mut(&cur).and_then(|v| v.pop()) {
                if n == s || lp.len() > pts.len() {
                    break;
                }
                lp.push(n);
                cur = n;
            }
            if lp.len() >= 3 {
                loops.push(lp);
            }
        }
    }
    let signed = |lp: &[usize]| -> f64 {
        let n = lp.len();
        (0..n)
            .map(|i| {
                let (a, b) = (pts[lp[i]], pts[lp[(i + 1) % n]]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            / 2.0
    };
    let best = loops
        .iter()
        .max_by(|a, c| signed(a).abs().total_cmp(&signed(c).abs()))?;
    let mut outline: Vec<[f64; 2]> = best.iter().map(|&i| pts[i]).collect();
    if signed(best) < 0.0 {
        outline.reverse();
    }
    let outline = simplify(outline);
    (outline.len() >= 3).then_some(Footprint {
        outline,
        z0,
        z1,
        area,
    })
}

/// Drop points on a straight line (within 5 mm) and doubled points.
fn simplify(mut v: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut changed = true;
    while changed && v.len() > 3 {
        changed = false;
        let n = v.len();
        for i in 0..n {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let ab = (b[0] - a[0]).hypot(b[1] - a[1]);
            let ac = (c[0] - a[0]).hypot(c[1] - a[1]);
            let cross = ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs();
            if ab < 0.005 || (ac > 1e-9 && cross / ac < 0.005) {
                v.remove(i);
                changed = true;
                break;
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample models from the bimifc checkout (third-party / test files,
    /// not in this repository): skipped when it is not beside eulumdat-rs.
    fn sample(name: &str) -> Option<String> {
        let dir = std::env::var("IFC_SAMPLES").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../../bimifc/ifc").to_string()
        });
        std::fs::read_to_string(format!("{dir}/{name}")).ok()
    }

    #[test]
    fn step_escapes() {
        assert_eq!(decode_step("K\\X2\\00FC\\X0\\che"), "Küche");
        assert_eq!(decode_step("Gr\\X\\F6\\X\\DFe"), "Größe");
        assert_eq!(decode_step("a\\\\b"), "a\\b");
        assert_eq!(decode_step("plain"), "plain");
    }

    #[test]
    fn a_square_floor_is_found() {
        // a 4 × 3 m box 2.5 m high, as two triangles per face (bottom +
        // top only: that is all the footprint looks at)
        let p: Vec<f32> = vec![
            0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 3.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 2.5, 4.0, 0.0,
            2.5, 4.0, 3.0, 2.5, 0.0, 3.0, 2.5,
        ];
        let idx = vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7];
        let f = footprint(&p, &idx).unwrap();
        assert_eq!(f.outline.len(), 4);
        assert!((f.area - 12.0).abs() < 1e-6);
        assert!((f.z1 - f.z0 - 2.5).abs() < 1e-6);
    }

    #[test]
    fn relux_room_with_its_luminaires() {
        let Some(t) = sample("relux-test.ifc") else {
            return;
        };
        let b = Building::parse(&t).unwrap();
        eprintln!(
            "rooms {:?}",
            b.rooms
                .iter()
                .map(|r| (&r.name, r.area, r.height, r.outline.len()))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "luminaires {:?}",
            b.luminaires
                .iter()
                .map(|l| (l.position, l.rotation_deg, l.room, l.photometry))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "photometries {:?} warnings {:?}",
            b.photometries
                .iter()
                .map(|p| (&p.name, p.count))
                .collect::<Vec<_>>(),
            b.warnings
        );
        assert_eq!(b.rooms.len(), 1);
        assert_eq!(b.luminaires.len(), 8);
        assert!(b.luminaires.iter().all(|l| l.room == Some(0)));
        // 5 Slimline with a goniometric source; the 3 Belviso's
        // distributions are empty in the file
        assert_eq!(
            b.luminaires
                .iter()
                .filter(|l| l.photometry.is_some())
                .count(),
            5
        );
        assert_eq!(b.photometries.len(), 1);
        assert!(b.warnings.iter().any(|w| w.starts_with("3 luminaire")));
        let plan = b.plan(0);
        assert_eq!(plan.luminaires.len(), 8);
        for l in &plan.luminaires {
            assert!(l.x >= -0.01 && l.x <= plan.length + 0.01, "{l:?}");
            assert!(l.y >= -0.01 && l.y <= plan.width + 0.01, "{l:?}");
            assert!(l.z > 0.5 && l.z <= plan.height + 0.5, "{l:?}");
        }
    }

    #[test]
    fn fzk_house_rooms_doors_and_exits() {
        let Some(t) = sample("AC20-FZK-Haus.ifc") else {
            return;
        };
        let b = Building::parse(&t).unwrap();
        for r in &b.rooms {
            eprintln!(
                "{} · storey {:?} · {:.1} m² · h {:.2} · {} corners",
                r.name,
                r.storey,
                r.area,
                r.height,
                r.outline.len()
            );
        }
        eprintln!(
            "doors {:?}",
            b.doors
                .iter()
                .map(|d| (&d.name, d.width, d.fire_exit, &d.rooms))
                .collect::<Vec<_>>()
        );
        eprintln!("windows {} warnings {:?}", b.windows.len(), b.warnings);
        assert_eq!(b.rooms.len(), 7);
        assert!(b.rooms.iter().all(|r| r.area > 2.0 && r.height > 2.0));
        assert_eq!(b.doors.len(), 5);
        assert_eq!(b.windows.len(), 11);
        assert!(b.doors.iter().all(|d| !d.rooms.is_empty()));
        // its FireExit flag is on the stair, not a door: the exits are the
        // two doors to the outside (front door, terrace door)
        assert!(!b.doors.iter().any(|d| d.fire_exit));
        let exits: Vec<&str> = b
            .doors
            .iter()
            .filter(|d| d.exit())
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(exits, ["Haustuer", "Terrassentuer"]);
        assert!(
            b.rooms.iter().any(|r| r.name.starts_with("Küche")),
            "STEP escapes decoded"
        );
        assert!(
            b.storeys.iter().all(|s| s.elevation.abs() < 20.0),
            "storeys in m"
        );
    }

    #[test]
    fn bimcollab_office_rooms() {
        let Some(t) = sample("01_BIMcollab_Example_ARC.ifc") else {
            return;
        };
        let b = Building::parse(&t).unwrap();
        eprintln!(
            "{} rooms, {} doors ({} exits), {} windows, storeys {:?}",
            b.rooms.len(),
            b.doors.len(),
            b.doors.iter().filter(|d| d.fire_exit).count(),
            b.windows.len(),
            b.storeys
                .iter()
                .map(|s| (&s.name, s.elevation))
                .collect::<Vec<_>>()
        );
        for r in b.rooms.iter().take(8) {
            eprintln!(
                "  {} · {:.1} m² · h {:.2} · {} corners",
                r.name,
                r.area,
                r.height,
                r.outline.len()
            );
        }
        eprintln!(
            "warnings {:?}",
            b.warnings.iter().take(5).collect::<Vec<_>>()
        );
        assert_eq!(b.rooms.len(), 51);
        assert_eq!(
            b.storeys.iter().map(|s| s.elevation).collect::<Vec<_>>(),
            [-0.8, 0.0, 3.5, 7.0, 10.5]
        );
        // all but one: "empty (02.02)" is a 0.77 m void under the roof
        let odd: Vec<&str> = b
            .rooms
            .iter()
            .filter(|r| r.area <= 1.0 || r.height <= 2.0)
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(odd, ["empty (02.02)"]);
    }
}
