//! A designer room as an IFC4 model, written with bimifc-writer (feature
//! `export`): the space (outline extruded to the room height), walls along
//! the outline, the doors in them (`Pset_DoorCommon.FireExit` for exits),
//! windows, and the luminaires as `IfcLightFixture` with their LDT in
//! `Pset_GLDF_LDTRawContent` (the gldf.icu layout the importer reads);
//! emergency luminaires carry `Pset_LightFixtureTypeSecurityLighting`.
//!
//! It is the inverse of [`Building::plan`](crate::Building::plan): a
//! [`RoomPlan`] exported and read back gives the same plan (outline,
//! height, luminaire positions, heights and directions, photometry, exits);
//! `tests/roundtrip.rs` checks that.

use base64::Engine;
use bimifc_writer::{
    ElementKind, ElementPset, IfcProperty, IfcValue, LightFixtureData, ProjectInfo, Rect, RoomData,
    RoomDimensions, RoomElement, SpaceData,
};

use crate::{PlanOpening, RoomPlan};

/// What else goes into the file.
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// IfcProject name.
    pub project: String,
    /// Wall thickness (m); walls stand outside the room outline.
    pub wall_thickness: f64,
    /// Exporting application (IfcApplication, STEP header).
    pub application: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            project: "Candelabra room".into(),
            wall_thickness: 0.2,
            application: "Candelabra".into(),
        }
    }
}

/// A photometry to embed: name and LDT text (index = the luminaires'
/// `photometry`).
#[derive(Clone, Debug)]
pub struct ExportPhotometry {
    pub name: String,
    pub ldt: String,
}

/// bimifc-writer hangs a 0.08 m housing below a luminaire's mount point;
/// the importer takes the housing's underside as the light plane, so the
/// mount point goes this much above the plan's luminaire height.
const HOUSING_H: f64 = 0.08;

/// The room as an IFC4 STEP file.
pub fn room_to_ifc(
    plan: &RoomPlan,
    photometries: &[ExportPhotometry],
    opts: &ExportOptions,
) -> String {
    let t = opts.wall_thickness.max(0.01);
    // plan (x east, y south) → world (x east, y north)
    let w = plan.width;
    let world = |x: f64, y: f64| (x, w - y);
    let mut poly: Vec<(f64, f64)> = plan.outline.iter().map(|&(x, y)| world(x, y)).collect();
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    let n = poly.len();

    let mut elements = Vec::new();
    // Walls on the outside of each edge; at a convex corner the wall runs
    // on by the thickness so the corner is closed.
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len < 1e-6 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (uy, -ux); // outward for a counter-clockwise outline
        let convex = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
            (q.0 - p.0) * (r.1 - q.1) - (q.1 - p.1) * (r.0 - q.0) > 0.0
        };
        let start = if convex(poly[(i + n - 1) % n], a, b) {
            t
        } else {
            0.0
        };
        let end = if convex(a, b, poly[(i + 2) % n]) {
            t
        } else {
            0.0
        };
        let length = len + start + end;
        // middle of the wall: along the edge shifted by the run-ons,
        // outwards by half the thickness
        let along = (end - start) / 2.0;
        let c = (
            (a.0 + b.0) / 2.0 + ux * along + nx * t / 2.0,
            (a.1 + b.1) / 2.0 + uy * along + ny * t / 2.0,
        );
        elements.push(element(
            ElementKind::Wall,
            c,
            length,
            t,
            uy.atan2(ux),
            plan.height,
            0.0,
            format!("Wall {}", i + 1),
            format!("wall-{i}"),
            Vec::new(),
        ));
    }

    // Doors and windows in the wall nearest to them.
    let mut opening = |o: &PlanOpening, kind: ElementKind, k: usize, door: bool| {
        let p = world(o.x, o.y);
        let (q, angle, normal) = nearest_edge(&poly, p);
        let c = (q.0 + normal.0 * t / 2.0, q.1 + normal.1 * t / 2.0);
        let psets = if door {
            vec![ElementPset {
                name: "Pset_DoorCommon".into(),
                properties: vec![
                    IfcProperty::new("FireExit", IfcValue::Boolean(o.fire_exit)),
                    IfcProperty::new("IsExternal", IfcValue::Boolean(o.exit)),
                ],
            }]
        } else {
            Vec::new()
        };
        elements.push(element(
            kind,
            c,
            if o.width > 0.1 { o.width } else { 1.0 },
            t,
            angle,
            if o.height > 0.1 {
                o.height
            } else if door {
                2.1
            } else {
                1.2
            },
            o.sill.max(0.0),
            o.name.clone(),
            format!("{}-{k}", if door { "door" } else { "window" }),
            psets,
        ));
    };
    for (k, d) in plan.doors.iter().enumerate() {
        opening(d, ElementKind::Door, k, true);
    }
    for (k, win) in plan.windows.iter().enumerate() {
        opening(win, ElementKind::Window, k, false);
    }

    let b64 = base64::engine::general_purpose::STANDARD;
    let light_fixtures = plan
        .luminaires
        .iter()
        .map(|l| {
            let ph = l.photometry.and_then(|i| photometries.get(i));
            let ldt = ph.and_then(|p| eulumdat::Eulumdat::parse(&p.ldt).ok());
            let (x, y) = world(l.x, l.y);
            LightFixtureData {
                name: if l.name.is_empty() {
                    ph.map_or_else(|| "Luminaire".into(), |p| p.name.clone())
                } else {
                    l.name.clone()
                },
                x,
                y,
                z: l.z + HOUSING_H,
                // plan angles run the other way round (y flipped)
                rotation: (-l.rotation_deg).to_radians(),
                flux_lm: ldt.as_ref().map_or(0.0, |d| d.luminous_flux()),
                cct_k: None,
                ldt_base64: ph.map(|p| b64.encode(p.ldt.as_bytes())),
                ldt_file_name: ph.map(|p| format!("{}.ldt", file_stem(&p.name))),
                // the standard set for emergency (security) luminaires
                psets: if l.emergency {
                    vec![ElementPset {
                        name: "Pset_LightFixtureTypeSecurityLighting".into(),
                        properties: vec![IfcProperty::new(
                            "FixtureHeight",
                            IfcValue::PositiveLengthMeasure(l.z.max(0.01)),
                        )],
                    }]
                } else {
                    Vec::new()
                },
                storey: String::new(),
                control: None,
                horticulture: None,
            }
        })
        .collect();

    let (x1, y1) = poly
        .iter()
        .fold((f64::MIN, f64::MIN), |m, p| (m.0.max(p.0), m.1.max(p.1)));
    let mut data = RoomData::new(
        elements,
        // world coordinates as they are (origin 0, 0)
        Rect {
            x: 0.0,
            y: 0.0,
            width: x1,
            height: y1,
        },
        RoomDimensions {
            width: plan.length,
            height: plan.height,
            depth: plan.width,
        },
    );
    data.project = ProjectInfo {
        project_name: opts.project.clone(),
        building_name: opts.project.clone(),
        application_name: opts.application.clone(),
        organization: opts.application.clone(),
        author: opts.application.clone(),
        ..ProjectInfo::default()
    };
    data.light_fixtures = light_fixtures;
    data.spaces = vec![SpaceData {
        // name = long name: the importer shows "long (name)" when they
        // differ, so this comes back as it went out
        name: plan.name.clone(),
        long_name: Some(plan.name.clone()),
        polygon: poly,
        height: plan.height,
        z_offset: 0.0,
        storey: String::new(),
        psets: Vec::new(),
        identifier: Some("space-1".into()),
        slab_thickness: None,
        slab_only: false,
    }];
    bimifc_writer::write_ifc(&data)
}

#[allow(clippy::too_many_arguments)]
fn element(
    kind: ElementKind,
    centre: (f64, f64),
    length: f64,
    depth: f64,
    rotation: f64,
    height: f64,
    z: f64,
    label: String,
    id: String,
    psets: Vec<ElementPset>,
) -> RoomElement {
    RoomElement {
        kind,
        // CGRect: corner + size; the writer rotates about the middle
        rect: Rect {
            x: centre.0 - length / 2.0,
            y: centre.1 - depth / 2.0,
            width: length,
            height: depth,
        },
        rotation,
        label: Some(label),
        height,
        z_offset: z,
        storey: String::new(),
        color: None,
        psets,
        material: None,
        identifier: Some(id),
        mesh: None,
    }
}

/// The point of the outline nearest to `p`, the direction of that edge
/// and its outward normal (counter-clockwise outline).
fn nearest_edge(poly: &[(f64, f64)], p: (f64, f64)) -> ((f64, f64), f64, (f64, f64)) {
    let n = poly.len();
    let mut best = (f64::INFINITY, p, 0.0, (0.0, -1.0));
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dy * dy).max(1e-12);
        let s = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
        let q = (a.0 + s * dx, a.1 + s * dy);
        let d = (q.0 - p.0).hypot(q.1 - p.1);
        if d < best.0 {
            let len = len2.sqrt();
            best = (d, q, dy.atan2(dx), (dy / len, -dx / len));
        }
    }
    (best.1, best.2, best.3)
}

fn signed_area(p: &[(f64, f64)]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

fn file_stem(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() {
        "luminaire".into()
    } else {
        s
    }
}
