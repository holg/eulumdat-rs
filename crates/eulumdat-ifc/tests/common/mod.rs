//! The Interior designer's sample room, shared by `tests/roundtrip.rs` and
//! `examples/make_sample_ifc.rs` (`#[path]`-included there): the landing
//! page's L-shaped open office (link `#interior?preset=0&sample=
//! demo-led-3000k&shape=…&em=1&eml=route…`) with its zonal layout, its
//! EN 1838 emergency luminaires (as `tests/emergency.rs::
//! landing_page_l_office` lays them out) and its two exits.
#![allow(dead_code)]

use eulumdat::emergency::{self, EmphasisKind, EmphasisPoint, EscapeRoute, Limits, Settings};
use eulumdat::zonal::{compute_zonal, LlfPreset, ReflectancePreset, RoomPreset, SolveMode};
use eulumdat::{CuTable, Eulumdat};
use eulumdat_ifc::export::ExportPhotometry;
use eulumdat_ifc::{PlanLuminaire, PlanOpening, RoomPlan};

pub const EM_LDT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../eulumdat/tests/fixtures/emergency/demo_em_route.ldt"
));

pub const SAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/interior_sample.ifc"
);

pub const DEMO_LDT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../eulumdat/tests/fixtures/colour_quality/demo_led_downlight.ldt"
));

/// The landing page's Interior sample: the L-shaped open office as the
/// designer lays it out (Open Office preset, standard reflectances, LED
/// light loss factor, 500 lx with the demo LED) and its two exits.
pub fn interior_sample() -> (RoomPlan, Vec<ExportPhotometry>) {
    let ldt = Eulumdat::parse(DEMO_LDT).unwrap();
    let outline = vec![
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 7.5),
        (10.0, 7.5),
        (10.0, 15.0),
        (0.0, 15.0),
    ];
    let room = RoomPreset::OpenOffice
        .to_room()
        .with_outline(Some(outline.clone()));
    let r = compute_zonal(
        &ldt,
        &room,
        &ReflectancePreset::Standard.to_reflectances(),
        &LlfPreset::Led.to_llf(),
        RoomPreset::OpenOffice.target_lux(),
        &CuTable::calculate(&ldt),
        SolveMode::TargetToCount,
        None,
        None,
    );
    let z = room.height - room.suspension_length;
    // the emergency luminaires, as the designer's emergency section places
    // them (demo escape-route optic at 3 m, its flux × MF 0.8)
    let em = Eulumdat::parse(EM_LDT).unwrap();
    let (limits, set) = (Limits::en1838(), Settings::default());
    let routes = vec![EscapeRoute {
        name: "1".into(),
        path: vec![(19.5, 3.75), (5.0, 3.75), (5.0, 14.5)],
        width: 2.0,
        place: Default::default(),
    }];
    let pt = |kind, x, y, f| EmphasisPoint {
        kind,
        x,
        y,
        z: None,
        facing: Some(f),
    };
    let points = vec![
        pt(EmphasisKind::FireEquipment, 15.0, 7.4, 270.0),
        pt(EmphasisKind::FirstAid, 0.1, 10.0, 0.0),
        pt(EmphasisKind::Exit, 20.0, 3.75, 180.0),
        pt(EmphasisKind::Exit, 5.0, 15.0, 270.0),
    ];
    let flux = emergency::luminaire_flux_lm(&em);
    let spacing = emergency::spacing_table(&em, flux, &[3.0], 2.0, &limits, &set).remove(0);
    let input = emergency::AutoInput {
        ldt: &em,
        flux_lm: flux,
        height: 3.0,
        ra: Some(80.0),
        spacing: &spacing,
        routes: &routes,
        room: Some(&outline),
        open_area: Some(&outline),
        open_area_place: emergency::PlaceCategory::CrowdedPlace,
        points: &points,
    };
    let em_name = "Demo escape-route optic (emergency)";
    let em_luminaires: Vec<PlanLuminaire> = emergency::auto_layout(&input, &limits, &set)
        .into_iter()
        .map(|(x, y, rot)| PlanLuminaire {
            x,
            y,
            z: 3.0,
            rotation_deg: rot,
            photometry: Some(1),
            name: em_name.into(),
            emergency: true,
        })
        .collect();
    let em_ldt = emergency::at_flux(&em, set.maintenance_factor * flux).to_ldt();

    let luminaires = r
        .layout
        .positions()
        .into_iter()
        .map(|(x, y)| PlanLuminaire {
            x,
            y,
            z,
            rotation_deg: 0.0,
            photometry: Some(0),
            name: "Demo LED downlight".into(),
            emergency: false,
        })
        .chain(em_luminaires)
        .collect();

    let exit = |x: f64, y: f64, n: usize| PlanOpening {
        x,
        y,
        width: 1.0,
        height: 2.1,
        sill: 0.0,
        fire_exit: true,
        exit: true,
        name: format!("Exit {n}"),
    };
    let plan = RoomPlan {
        // the web export names the room after its preset
        name: RoomPreset::OpenOffice.label().to_string(),
        length: room.length,
        width: room.width,
        height: room.height,
        area: room.area(),
        outline,
        luminaires,
        doors: vec![exit(20.0, 3.75, 1), exit(5.0, 15.0, 2)],
        windows: Vec::new(),
        origin: [0.0; 3],
    };
    let photometries = vec![
        ExportPhotometry {
            name: "Demo LED downlight".into(),
            ldt: DEMO_LDT.to_string(),
        },
        ExportPhotometry {
            name: em_name.into(),
            ldt: em_ldt,
        },
    ];
    (plan, photometries)
}
