//! EN 1838 emergency lighting (`eulumdat::emergency`) and GLDF luminaire
//! configurations (`eulumdat::gldf_product`).

use eulumdat::emergency::{
    self, EmergencyLuminaire, EmergencyPlan, EmphasisKind, EmphasisPoint, EscapeRoute,
    HighRiskArea, Limits, OpenArea, Settings,
};
use eulumdat::gldf_product::{EmergencyBehaviour, FluxSource, GldfProduct};
use eulumdat::{Eulumdat, LampSet, PhotometricCalculations};
use std::path::PathBuf;

/// A downlight with the same intensity everywhere below the horizon.
fn hemisphere() -> Eulumdat {
    let g: Vec<f64> = (0..=36).map(|k| k as f64 * 5.0).collect();
    let row: Vec<f64> = g
        .iter()
        .map(|&a| if a <= 90.0 { 159.155 } else { 0.0 })
        .collect();
    Eulumdat {
        num_c_planes: 4,
        c_plane_distance: 90.0,
        num_g_planes: 37,
        g_plane_distance: 5.0,
        c_angles: vec![0.0, 90.0, 180.0, 270.0],
        g_angles: g,
        intensities: vec![row.clone(), row.clone(), row.clone(), row],
        lamp_sets: vec![LampSet {
            num_lamps: 1,
            total_luminous_flux: 1000.0,
            ..Default::default()
        }],
        conversion_factor: 1.0,
        light_output_ratio: 100.0,
        ..Default::default()
    }
}

fn lum(ldt: &Eulumdat, flux: f64, x: f64, y: f64, h: f64) -> EmergencyLuminaire<'_> {
    EmergencyLuminaire {
        ldt,
        flux_lm: flux,
        x,
        y,
        height: h,
        rotation: 0.0,
        ra: Some(80.0),
    }
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-9)
}

#[test]
fn shipped_limits_match_the_json_table() {
    let from_json = Limits::from_json(emergency::LIMITS_JSON).unwrap();
    assert_eq!(from_json, Limits::en1838());
    let l = Limits::en1838();
    assert_eq!(l.glare_limit(2.4, false), 500.0);
    assert_eq!(l.glare_limit(2.5, false), 900.0);
    assert_eq!(l.glare_limit(3.99, true), 5000.0);
    assert_eq!(l.glare_limit(4.5, false), 5000.0);
    assert_eq!(l.glare_limit(12.0, true), 10000.0);
    assert_eq!(l.high_risk_min_lx(500.0), 50.0);
    assert_eq!(l.high_risk_min_lx(100.0), 15.0);
}

#[test]
fn illuminance_follows_inverse_square_and_cosine() {
    let ldt = hemisphere();
    let per_klm = PhotometricCalculations::calculated_luminous_flux(&ldt);
    // 159.155 cd/klm × 2π ≈ 1000, plus the trapezoid over the 90°–95° step
    assert!(
        (per_klm - 1000.0).abs() < 60.0,
        "flux of the synthetic table: {per_klm}"
    );
    let flux = 100.0;
    let i = 159.155 * flux / per_klm; // candela of the scaled luminaire
    let l = [lum(&ldt, flux, 0.0, 0.0, 3.0)];
    let up = (0.0, 0.0, 1.0);
    let nadir = emergency::illuminance(&l, (0.0, 0.0, 0.0), up, 1.0);
    assert!(close(nadir, i / 9.0, 1e-6), "{nadir} vs {}", i / 9.0);
    // 3 m sideways: γ = 45°, E = I cos³γ / h²
    let off = emergency::illuminance(&l, (3.0, 0.0, 0.0), up, 1.0);
    let expect = i * (45f64.to_radians().cos()).powi(3) / 9.0;
    assert!(close(off, expect, 1e-3), "{off} vs {expect}");
    // the maintenance factor scales linearly
    let mf = emergency::illuminance(&l, (0.0, 0.0, 0.0), up, 0.8);
    assert!(close(mf, 0.8 * nadir, 1e-9));
    // the luminaire's flux is the requested one
    assert!(close(emergency::luminaire_flux_lm(&ldt), per_klm, 1e-9));
}

fn corridor(ldt: &Eulumdat, flux: f64, spacing: f64) -> EmergencyPlan<'_> {
    let n = (20.0 / spacing).round() as usize;
    EmergencyPlan {
        luminaires: (0..=n)
            .map(|k| lum(ldt, flux, k as f64 * spacing, 1.0, 3.0))
            .collect(),
        routes: vec![EscapeRoute {
            name: "corridor".into(),
            path: vec![(0.0, 1.0), (20.0, 1.0)],
            width: 2.0,
            place: Default::default(),
        }],
        ..Default::default()
    }
}

#[test]
fn escape_route_passes_with_close_spacing_and_fails_with_wide() {
    let ldt = hemisphere();
    let limits = Limits::en1838();
    let s = Settings::default();
    let ok = emergency::evaluate(&corridor(&ldt, 100.0, 4.0), &limits, &s);
    let r = &ok.routes[0];
    assert!(r.pass(), "{r:?}");
    assert!(close(r.length_m, 20.0, 1e-9));
    assert!(r.centre.min_lx >= 1.0 && r.band.min_lx >= 0.5);
    assert!(!r.too_wide);
    assert_eq!(r.samples.len(), 201, "every 0.1 m");
    // twice the flux of a 12 m spacing still leaves a dark middle
    let bad = emergency::evaluate(&corridor(&ldt, 100.0, 10.0), &limits, &s);
    let r = &bad.routes[0];
    assert!(!r.pass_centre, "{:?}", r.centre);
    // the darkest point lies between two luminaires
    let x = r.centre.at_min.0;
    assert!((x - 5.0).abs() < 0.2 || (x - 15.0).abs() < 0.2, "{x}");
    assert!(!bad.pass());
}

#[test]
fn open_areas_border_and_size_rule() {
    let ldt = hemisphere();
    let limits = Limits::en1838();
    let s = Settings::default();
    let grid: Vec<EmergencyLuminaire> = (0..4)
        .flat_map(|i| (0..4).map(move |j| (i, j)))
        .map(|(i, j)| {
            lum(
                &ldt,
                100.0,
                1.25 + i as f64 * 2.5,
                1.25 + j as f64 * 2.5,
                3.0,
            )
        })
        .collect();
    let square = |a: f64| vec![(0.0, 0.0), (a, 0.0), (a, a), (0.0, a)];
    let plan = EmergencyPlan {
        luminaires: grid,
        open_areas: vec![
            OpenArea {
                name: "hall".into(),
                polygon: square(10.0),
                place: emergency::PlaceCategory::CrowdedPlace,
            },
            OpenArea {
                name: "store".into(),
                polygon: square(5.0),
                place: emergency::PlaceCategory::CrowdedPlace,
            },
        ],
        ..Default::default()
    };
    let rep = emergency::evaluate(&plan, &limits, &s);
    let hall = &rep.open_areas[0];
    assert!(hall.required && hall.pass(), "{:?}", hall.stats);
    assert!(close(hall.area_m2, 100.0, 1e-9));
    // no grid point within 0.5 m of the walls
    assert!(hall
        .samples
        .iter()
        .all(|&(x, y, _)| (0.5..=9.5).contains(&x) && (0.5..=9.5).contains(&y)));
    assert!(
        !rep.open_areas[1].required,
        "25 m² needs no anti-panic lighting"
    );
}

#[test]
fn high_risk_uses_the_task_plane_and_its_own_minimum() {
    let ldt = hemisphere();
    let limits = Limits::en1838();
    let plan = |flux: f64| EmergencyPlan {
        luminaires: vec![lum(&ldt, flux, 1.0, 1.0, 3.0)],
        high_risk: vec![HighRiskArea {
            name: "saw".into(),
            polygon: vec![(0.5, 0.5), (1.5, 0.5), (1.5, 1.5), (0.5, 1.5)],
            task_height: 0.85,
            normal_maintained_lx: 500.0,
        }],
        ..Default::default()
    };
    let weak = emergency::evaluate(&plan(100.0), &limits, &Settings::default());
    assert_eq!(weak.high_risk[0].required_lx, 50.0);
    assert!(!weak.high_risk[0].pass_min);
    let strong = emergency::evaluate(&plan(3000.0), &limits, &Settings::default());
    assert!(
        strong.high_risk[0].pass(),
        "{:?}",
        strong.high_risk[0].stats
    );
    // the luminaire stands in the area: high-risk glare column
    assert!(strong.glare[0].high_risk);
}

#[test]
fn points_of_emphasis_need_vertical_illuminance() {
    let ldt = hemisphere();
    let limits = Limits::en1838();
    let plan = |flux: f64, facing: Option<f64>| EmergencyPlan {
        luminaires: vec![lum(&ldt, flux, 0.0, 0.0, 3.0)],
        points: vec![
            EmphasisPoint {
                kind: EmphasisKind::FireEquipment,
                x: 1.0,
                y: 0.0,
                z: Some(1.2),
                facing,
            },
            EmphasisPoint {
                kind: EmphasisKind::Exit,
                x: 1.5,
                y: 0.0,
                z: None,
                facing: None,
            },
        ],
        ..Default::default()
    };
    let s = Settings::default();
    // facing the luminaire (towards -x)
    let weak = emergency::evaluate(&plan(100.0, Some(180.0)), &limits, &s);
    assert_eq!(weak.points[0].pass, Some(false));
    let strong = emergency::evaluate(&plan(1000.0, Some(180.0)), &limits, &s);
    assert_eq!(strong.points[0].pass, Some(true), "{:?}", strong.points[0]);
    // facing away from it: no light on the face
    let away = emergency::evaluate(&plan(1000.0, Some(0.0)), &limits, &s);
    assert_eq!(away.points[0].vertical_lx, Some(0.0));
    // without a facing: best of eight directions, said so
    let any = emergency::evaluate(&plan(1000.0, None), &limits, &s);
    assert!(any.points[0].assumed_facing);
    // exits only report the distance to the nearest luminaire
    assert_eq!(strong.points[1].pass, None);
    assert!(close(
        strong.points[1].nearest_luminaire_m.unwrap(),
        1.5,
        1e-9
    ));
}

#[test]
fn glare_limit_by_mounting_height_and_route_direction() {
    let ldt = hemisphere();
    let limits = Limits::en1838();
    // 10 000 lm spread evenly over the lower hemisphere: ~1590 cd at 60°–90°
    let plan = |h: f64| EmergencyPlan {
        luminaires: vec![lum(&ldt, 10_000.0, 5.0, 1.0, h)],
        routes: vec![EscapeRoute {
            name: "r".into(),
            path: vec![(0.0, 1.0), (20.0, 1.0)],
            width: 2.0,
            place: Default::default(),
        }],
        ..Default::default()
    };
    let low = emergency::evaluate(&plan(2.4), &limits, &Settings::default());
    let g = &low.glare[0];
    assert!(g.route_directions, "over the route: directions of travel");
    assert!(!g.pass && g.limit_cd == 500.0, "{g:?}");
    assert!(g.i_max_cd > 1500.0 && g.i_max_cd < 1700.0);
    let high = emergency::evaluate(&plan(4.6), &limits, &Settings::default());
    assert!(high.glare[0].pass && high.glare[0].limit_cd == 5000.0);
}

#[test]
fn colour_rendering_rule() {
    let ldt = hemisphere();
    let mut plan = corridor(&ldt, 100.0, 4.0);
    plan.luminaires[0].ra = Some(30.0);
    plan.luminaires[1].ra = None;
    let rep = emergency::evaluate(&plan, &Limits::en1838(), &Settings::default());
    assert_eq!(rep.ra_below_limit, vec![0]);
    assert_eq!(rep.ra_unknown, vec![1]);
    assert!(!rep.pass());
}

#[test]
fn spacing_table_grows_with_flux_and_height() {
    let ldt = hemisphere();
    let s = Settings {
        maintenance_factor: 1.0,
        ..Default::default()
    };
    let rows = emergency::spacing_table(&ldt, 100.0, &[2.5, 4.0], 2.0, &Limits::en1838(), &s);
    let more = emergency::spacing_table(&ldt, 200.0, &[2.5], 2.0, &Limits::en1838(), &s);
    assert!(rows[0].route_spacing_c0_m > 2.0);
    assert!(more[0].route_spacing_c0_m > rows[0].route_spacing_c0_m);
    assert!(more[0].open_area_grid_m > rows[0].open_area_grid_m);
    // rotationally symmetric: both directions the same
    assert_eq!(rows[0].route_spacing_c0_m, rows[0].route_spacing_c90_m);
    let i = 159.155 * 100.0 / PhotometricCalculations::calculated_luminous_flux(&ldt);
    assert!(close(rows[0].e_nadir_lx, i / 6.25, 1e-6));
    assert!(close(rows[0].glare_i_max_cd, i, 1e-3));
}

// ─── GLDF ────────────────────────────────────────────────────────────

/// Our own small product.xml with every way GLDF gives emergency flux.
const PRODUCT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Root>
  <Header><Manufacturer>Demo</Manufacturer></Header>
  <GeneralDefinitions>
    <Files>
      <File id="f1" contentType="ldc/eulumdat" type="localFileName">down.ldt</File>
      <File id="f2" contentType="ldc/eulumdat" type="localFileName">wide.ldt</File>
      <File id="fs" contentType="sensor/sensxml" type="localFileName">pir.xml</File>
    </Files>
    <Sensors><Sensor id="S1"><SensorFileReference fileId="fs"/></Sensor></Sensors>
    <Photometries>
      <Photometry id="P1"><PhotometryFileReference fileId="f1"/></Photometry>
      <Photometry id="P2"><PhotometryFileReference fileId="f2"/></Photometry>
    </Photometries>
    <LightSources>
      <FixedLightSource id="LS1"><ColorInformation><ColorRenderingIndex>82</ColorRenderingIndex>
        <CorrelatedColorTemperature>3000</CorrelatedColorTemperature></ColorInformation></FixedLightSource>
    </LightSources>
    <ControlGears><ControlGear id="CG1"><EmergencyBallastLumenFactor>0.1</EmergencyBallastLumenFactor></ControlGear></ControlGears>
    <Emitters>
      <Emitter id="normal"><FixedLightEmitter emergencyBehaviour="None"><PhotometryReference photometryId="P1"/>
        <LightSourceReference fixedLightSourceId="LS1"/><RatedLuminousFlux>2000</RatedLuminousFlux></FixedLightEmitter></Emitter>
      <Emitter id="comb-blf"><FixedLightEmitter emergencyBehaviour="Combined"><PhotometryReference photometryId="P2"/>
        <ControlGearReference controlGearId="CG1"/><RatedLuminousFlux>3000</RatedLuminousFlux></FixedLightEmitter></Emitter>
      <Emitter id="only"><FixedLightEmitter emergencyBehaviour="EmergencyOnly"><PhotometryReference photometryId="P2"/>
        <RatedLuminousFlux>150</RatedLuminousFlux></FixedLightEmitter></Emitter>
      <Emitter id="comb-rated"><FixedLightEmitter emergencyBehaviour="Combined"><PhotometryReference photometryId="P1"/>
        <RatedLuminousFlux>2000</RatedLuminousFlux><EmergencyRatedLuminousFlux>120</EmergencyRatedLuminousFlux></FixedLightEmitter></Emitter>
      <Emitter id="pir"><SensorEmitter><SensorReference sensorId="S1"/></SensorEmitter></Emitter>
    </Emitters>
  </GeneralDefinitions>
  <ProductDefinitions>
    <ProductMetaData><Name><Locale language="de">Demo DE</Locale><Locale language="en">Demo</Locale></Name></ProductMetaData>
    <Variants>
      <Variant id="v1"><Name><Locale language="en">normal + combined</Locale></Name>
        <Mountings><Ceiling><Recessed recessedDepth="80"/></Ceiling></Mountings>
        <Geometry><ModelGeometryReference geometryId="g">
          <EmitterReference emitterId="normal"><EmitterObjectExternalName>LEO0</EmitterObjectExternalName></EmitterReference>
          <EmitterReference emitterId="comb-blf"><EmitterObjectExternalName>LEO1</EmitterObjectExternalName></EmitterReference>
        </ModelGeometryReference></Geometry></Variant>
      <Variant id="v2"><Name><Locale language="en">normal + emergency only + sensor</Locale></Name>
        <Geometry><ModelGeometryReference geometryId="g">
          <EmitterReference emitterId="normal"><EmitterObjectExternalName>LEO0</EmitterObjectExternalName></EmitterReference>
          <EmitterReference emitterId="only"><EmitterObjectExternalName>LEO1</EmitterObjectExternalName></EmitterReference>
          <EmitterReference emitterId="pir"><EmitterObjectExternalName>Sensor</EmitterObjectExternalName></EmitterReference>
        </ModelGeometryReference></Geometry></Variant>
      <Variant id="v3"><Geometry><ModelGeometryReference geometryId="g">
          <EmitterReference emitterId="comb-rated"/></ModelGeometryReference></Geometry>
        <DescriptiveAttributes><Emergency><DurationTimeAndFlux>
          <Flux hours="3">90</Flux><Flux hours="1">200</Flux></DurationTimeAndFlux></Emergency></DescriptiveAttributes></Variant>
    </Variants>
  </ProductDefinitions>
</Root>"#;

#[test]
fn gldf_configurations_resolve_the_emergency_flux() {
    let p = GldfProduct::parse(PRODUCT_XML).unwrap();
    assert_eq!(p.manufacturer.as_deref(), Some("Demo"));
    assert_eq!(
        p.product_name.as_deref(),
        Some("Demo"),
        "English locale first"
    );
    let c = p.configs(None);
    assert_eq!(c.len(), 3);

    // v1: ballast lumen factor 0.1 × 3000 lm
    assert_eq!(c[0].mounting.as_deref(), Some("Ceiling/Recessed"));
    let comb = &c[0].lamps[1];
    assert_eq!(comb.behaviour, EmergencyBehaviour::Combined);
    assert_eq!(comb.slot.as_deref(), Some("LEO1"));
    assert_eq!(comb.photometry_file.as_deref(), Some("wide.ldt"));
    assert_eq!(comb.normal_lm, Some(3000.0));
    assert_eq!(comb.emergency_lm, Some(300.0));
    assert_eq!(comb.emergency_flux_source, FluxSource::BallastLumenFactor);
    assert_eq!(c[0].lamps[0].ra, Some(82.0));
    assert!(c[0].has_emergency() && !c[0].has_sensor());

    // v2: emergency-only emitter = its rated flux, off in normal mode; sensor
    let only = &c[1].lamps[1];
    assert_eq!(only.normal_lm, None);
    assert_eq!(only.emergency_lm, Some(150.0));
    assert_eq!(only.emergency_flux_source, FluxSource::Rated);
    assert_eq!(c[1].normal_lamps().count(), 1);
    assert_eq!(c[1].emergency_lamps().count(), 1);
    assert_eq!(c[1].sensors[0].sensor_file.as_deref(), Some("pir.xml"));
    assert_eq!(c[1].sensors[0].slot.as_deref(), Some("Sensor"));

    // v3: the duration table wins over EmergencyRatedLuminousFlux
    let v3 = |d| p.config(&p.variants[2], d).lamps[0].clone();
    assert_eq!(v3(None).emergency_lm, Some(200.0), "shortest duration");
    assert_eq!(v3(Some(3)).emergency_lm, Some(90.0));
    assert_eq!(v3(Some(3)).emergency_flux_source, FluxSource::Duration(3));
    assert_eq!(v3(Some(2)).emergency_lm, Some(90.0), "next longer entry");
}

fn tria2() -> Option<PathBuf> {
    let dir = std::env::var_os("TRIA2_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../gldf-rs/tests/data/SLV - Tria 2_extracted")
        });
    dir.join("product.xml").exists().then_some(dir)
}

#[test]
fn slv_tria_2_one_luminaire_with_light_emergency_and_sensor() {
    let Some(dir) = tria2() else {
        eprintln!("SLV Tria 2 not found next to this repository: skipped");
        return;
    };
    let p = GldfProduct::parse(&std::fs::read_to_string(dir.join("product.xml")).unwrap()).unwrap();
    let c = p.configs(None);
    assert_eq!(c.len(), 3);
    let v = c
        .iter()
        .find(|c| c.name == "no Em / only Em / sens")
        .unwrap();
    let only = v.emergency_lamps().next().unwrap();
    assert_eq!(only.photometry_file.as_deref(), Some("wide.ldt"));
    assert_eq!(only.emergency_lm, Some(100.0));
    assert_eq!(only.slot.as_deref(), Some("LEO1"));
    assert_eq!(v.normal_lamps().next().unwrap().normal_lm, Some(7800.0));
    assert_eq!(v.sensors[0].sensor_file.as_deref(), Some("sensor.ldt"));
    let comb = c[0].emergency_lamps().next().unwrap();
    assert_eq!(comb.behaviour, EmergencyBehaviour::Combined);
    assert_eq!(
        (comb.normal_lm, comb.emergency_lm),
        (Some(7800.0), Some(100.0))
    );
    assert_eq!(comb.emergency_flux_source, FluxSource::EmergencyRated);
    assert_eq!(comb.ra, Some(80.0));

    // emergency mode of the wide optic at 100 lm, and the real EM pack
    let s = Settings {
        maintenance_factor: 1.0,
        ..Default::default()
    };
    let wide = Eulumdat::from_file(dir.join("ldc/wide.ldt")).unwrap();
    let rows = emergency::spacing_table(&wide, 100.0, &[3.0], 2.0, &Limits::en1838(), &s);
    eprintln!("wide @100 lm, 3 m: {:?}", rows[0]);
    assert!(close(rows[0].e_nadir_lx, 5.98, 0.02));
    assert!((rows[0].route_spacing_c0_m - 7.3).abs() <= 0.3);
    assert!((rows[0].open_area_grid_m - 6.9).abs() <= 0.3);
    assert!(close(rows[0].glare_i_max_cd, 6.43, 0.03));
    let em = Eulumdat::from_file(dir.join("ldc/emergency.ldt")).unwrap();
    let rows = emergency::spacing_table(&em, 45.0, &[3.0], 2.0, &Limits::en1838(), &s);
    eprintln!("EM pack @45 lm, 3 m: {:?}", rows[0]);
    assert!((rows[0].route_spacing_c0_m - 5.0).abs() <= 0.3);
    assert!((rows[0].open_area_grid_m - 4.2).abs() <= 0.3);
}

#[test]
fn demo_emergency_luminaires_are_pinned() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/emergency");
    let s = Settings::default();
    let route = Eulumdat::from_file(dir.join("demo_em_route.ldt")).unwrap();
    let open = Eulumdat::from_file(dir.join("demo_em_open.ldt")).unwrap();
    assert!(close(emergency::luminaire_flux_lm(&route), 180.0, 0.01));
    assert!(close(emergency::luminaire_flux_lm(&open), 220.0, 0.01));
    let r = emergency::spacing_table(&route, 180.0, &[3.0], 2.0, &Limits::en1838(), &s);
    // a directional optic: far along C0, short across
    assert!((r[0].route_spacing_c0_m - 15.2).abs() <= 0.2, "{:?}", r[0]);
    assert!((r[0].route_spacing_c90_m - 6.8).abs() <= 0.2, "{:?}", r[0]);
    let o = emergency::spacing_table(&open, 220.0, &[3.0], 2.0, &Limits::en1838(), &s);
    assert!((o[0].open_area_grid_m - 13.9).abs() <= 0.2, "{:?}", o[0]);
}

#[test]
fn a_luminaire_at_its_emergency_flux() {
    let ldt = hemisphere();
    let em = emergency::at_flux(&ldt, 150.0);
    assert!(close(emergency::luminaire_flux_lm(&em), 150.0, 1e-9));
    // same shape, other flux: the intensity table is untouched
    assert_eq!(em.intensities, ldt.intensities);
    let l = [lum(&em, emergency::luminaire_flux_lm(&em), 0.0, 0.0, 3.0)];
    let m = [lum(&ldt, 150.0, 0.0, 0.0, 3.0)];
    let up = (0.0, 0.0, 1.0);
    assert!(close(
        emergency::illuminance(&l, (1.0, 0.0, 0.0), up, 1.0),
        emergency::illuminance(&m, (1.0, 0.0, 0.0), up, 1.0),
        1e-9
    ));
}

/// The landing page's emergency demo: the open-office preset (3 m high)
/// drawn as an L, an escape route around the corner, safety equipment and
/// two exits, demo escape-route optic, automatic layout. The caption
/// (`em_cap` in the landing generator) quotes these numbers.
#[test]
fn landing_page_l_office() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/emergency");
    let ldt = Eulumdat::from_file(dir.join("demo_em_route.ldt")).unwrap();
    let limits = Limits::en1838();
    let s = Settings::default();
    // link: preset=0 (20 × 15 m) shape=0,0;1,0;1,0.5;0.5,0.5;0.5,1;0,1
    let room = vec![
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 7.5),
        (10.0, 7.5),
        (10.0, 15.0),
        (0.0, 15.0),
    ];
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
    let flux = emergency::luminaire_flux_lm(&ldt);
    let spacing = emergency::spacing_table(&ldt, flux, &[3.0], 2.0, &limits, &s).remove(0);
    let input = emergency::AutoInput {
        ldt: &ldt,
        flux_lm: flux,
        height: 3.0,
        ra: Some(80.0),
        spacing: &spacing,
        routes: &routes,
        room: Some(&room),
        open_area: Some(&room),
        open_area_place: emergency::PlaceCategory::CrowdedPlace,
        points: &points,
    };
    let pos = emergency::auto_layout(&input, &limits, &s);
    let rep = emergency::evaluate(&input.plan(&pos), &limits, &s);
    let r = &rep.routes[0];
    let a = &rep.open_areas[0];
    let fe = rep.points[0].vertical_lx.unwrap();
    let fa = rep.points[1].vertical_lx.unwrap();
    eprintln!(
        "luminaires {} | route {:.1} m: min {:.2} lx band {:.2} ratio {:.1} | area {:.0} m² min {:.2} ratio {:.1} | fire {:.2} first aid {:.2} | glare {:.0} cd",
        pos.len(), r.length_m, r.centre.min_lx, r.band.min_lx, r.centre.ratio,
        a.area_m2, a.stats.min_lx, a.stats.ratio, fe, fa,
        rep.glare.iter().map(|g| g.i_max_cd).fold(0.0, f64::max)
    );
    assert!(rep.pass(), "the demo meets every requirement");
    // quoted by the landing page (em_cap)
    assert_eq!(pos.len(), 11);
    assert!((a.area_m2 - 225.0).abs() < 1e-9);
    assert!((25.0..25.5).contains(&r.length_m));
    assert!(
        (3.6..3.75).contains(&r.centre.min_lx),
        "{}",
        r.centre.min_lx
    );
    assert!((1.0..1.1).contains(&a.stats.min_lx), "{}", a.stats.min_lx);
    assert!((8.4..8.7).contains(&fe), "{fe}");
}

#[test]
fn the_floor_map_agrees_with_the_point_values() {
    let ldt = hemisphere();
    let lums = vec![emergency::EmergencyLuminaire {
        ldt: &ldt,
        flux_lm: 200.0,
        x: 2.0,
        y: 2.0,
        height: 3.0,
        rotation: 0.0,
        ra: None,
    }];
    // an L: the notch's cells are outside
    let room = [
        (0.0, 0.0),
        (4.0, 0.0),
        (4.0, 2.0),
        (2.0, 2.0),
        (2.0, 4.0),
        (0.0, 4.0),
    ];
    let g = emergency::floor_grid(&lums, &room, 0.5, 0.8);
    assert_eq!((g.nx, g.ny), (8, 8));
    assert!(g.at(7, 7).is_none(), "the notch is not floor");
    let e = g.at(3, 3).expect("inside");
    let direct = emergency::illuminance(&lums, (1.75, 1.75, 0.0), (0.0, 0.0, 1.0), 0.8);
    assert!(close(e, direct, 1e-9), "{e} vs {direct}");
    let (lo, hi) = g.min_max().unwrap();
    assert!(lo > 0.0 && hi >= e && lo <= e);
}

// ─── China: GB 55037-2022 ────────────────────────────────────────────

#[test]
fn chinese_limits_match_their_data_file() {
    let v: serde_json::Value = serde_json::from_str(emergency::LIMITS_CN_JSON).unwrap();
    let l = Limits::gb55037();
    let p = l.places.as_ref().expect("a per-place profile");
    let f = &v["floor_min_illuminance_lx"];
    assert_eq!(
        f["stairs_front_rooms_refuge"].as_f64(),
        Some(p.stairs_or_refuge_lx)
    );
    assert_eq!(f["evacuation_corridor"].as_f64(), Some(p.corridor_lx));
    assert_eq!(f["crowded_place"].as_f64(), Some(p.crowded_lx));
    assert_eq!(f["other_place"].as_f64(), Some(p.other_lx));
    let m = &v["measurement"];
    assert_eq!(
        m["route_band_share_of_width"].as_f64(),
        Some(l.escape_route.band_share)
    );
    assert_eq!(m["area_border_m"].as_f64(), Some(l.anti_panic.border_m));
    let g = &v["GB_51309_2018"];
    let d = &g["duration_h"];
    let hours: Vec<f64> = l.timing.durations.iter().map(|d| d.hours).collect();
    assert_eq!(
        vec![
            d["buildings_over_100_m"].as_f64().unwrap(),
            d["hospitals_elderly_care_public_over_100000_m2_underground_over_20000_m2"]
                .as_f64()
                .unwrap(),
            d["other"].as_f64().unwrap()
        ],
        hours
    );
    assert_eq!(g["switch_over_s"].as_f64(), Some(l.timing.full_s));
    assert_eq!(
        g["switch_over_high_risk_s"].as_f64(),
        Some(l.timing.high_risk_full_s)
    );
    assert_eq!(v["profile"]["id"].as_str(), Some(l.id.as_str()));
    assert_eq!(v["profile"]["name"].as_str(), Some(l.name.as_str()));
    // the values per place
    use emergency::PlaceCategory as P;
    assert_eq!(p.required(P::StairsOrRefuge), 10.0);
    assert_eq!(p.required(P::EscapeCorridor), 3.0);
    assert_eq!(p.required(P::CrowdedPlace), 3.0);
    assert_eq!(p.required(P::OtherPlace), 1.0);
    assert_eq!(Limits::by_id("gb55037"), Some(l));
    assert_eq!(Limits::by_id("en1838"), Some(Limits::en1838()));
}

#[test]
fn one_corridor_two_profiles() {
    use emergency::PlaceCategory as P;
    let ldt = hemisphere();
    // a 20 m corridor, 2 m wide, a luminaire every 4 m at 2.8 m
    let lums: Vec<_> = (0..=5)
        .map(|k| lum(&ldt, 300.0, k as f64 * 4.0, 0.0, 2.8))
        .collect();
    let route = |place| EscapeRoute {
        name: "1".into(),
        path: vec![(0.0, 0.0), (20.0, 0.0)],
        width: 2.0,
        place,
    };
    let plan = |place| EmergencyPlan {
        luminaires: lums.clone(),
        routes: vec![route(place)],
        points: vec![EmphasisPoint {
            kind: EmphasisKind::FireEquipment,
            x: 10.0,
            y: 1.0,
            z: None,
            facing: Some(270.0),
        }],
        ..Default::default()
    };
    let s = Settings::default();
    let en = emergency::evaluate(&plan(P::EscapeCorridor), &Limits::en1838(), &s);
    let cn = emergency::evaluate(&plan(P::EscapeCorridor), &Limits::gb55037(), &s);
    let stairs = emergency::evaluate(&plan(P::StairsOrRefuge), &Limits::gb55037(), &s);
    let (e, c, st) = (&en.routes[0], &cn.routes[0], &stairs.routes[0]);
    // the same light, other requirements
    assert_eq!(e.centre, c.centre);
    assert_eq!((e.required_centre_lx, e.required_band_lx), (1.0, 0.5));
    // China checks the central half only (it holds the centre line)
    assert_eq!((c.required_centre_lx, c.required_band_lx), (0.0, 3.0));
    assert_eq!((st.required_centre_lx, st.required_band_lx), (0.0, 10.0));
    assert!(
        c.max_ratio.is_infinite() && c.pass_ratio,
        "no uniformity in China"
    );
    assert_eq!(cn.profile, "GB 55037-2022");
    assert_eq!(en.profile, "EN 1838:2013");
    // the corridor has ~3–6 lx: fine for EN and China, too dark for stairs
    assert!(e.pass(), "{e:?}");
    assert!(c.band.min_lx > 3.0 && c.pass(), "{:?}", c.band);
    assert!(!st.pass(), "stairs need 10 lx: {:?}", st.band);
    // equipment: EN wants 5 lx vertical, China has no such rule
    assert!(en.points[0].pass.is_some());
    assert_eq!(cn.points[0].pass, None);
    assert!(cn.points[0].vertical_lx.is_some(), "still reported");
    assert!(cn.glare.iter().all(|g| g.pass && g.limit_cd.is_infinite()));
}

#[test]
fn chinese_areas_have_a_minimum_whatever_their_size() {
    use emergency::PlaceCategory as P;
    let ldt = hemisphere();
    let lums = vec![lum(&ldt, 400.0, 2.5, 2.5, 3.0)];
    let small = vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)];
    let plan = |place| EmergencyPlan {
        luminaires: lums.clone(),
        open_areas: vec![OpenArea {
            name: "room".into(),
            polygon: small.clone(),
            place,
        }],
        ..Default::default()
    };
    let s = Settings::default();
    let en = emergency::evaluate(&plan(P::CrowdedPlace), &Limits::en1838(), &s);
    assert!(!en.open_areas[0].required, "25 m² is below EN's 60 m²");
    let other = emergency::evaluate(&plan(P::OtherPlace), &Limits::gb55037(), &s);
    let crowded = emergency::evaluate(&plan(P::CrowdedPlace), &Limits::gb55037(), &s);
    let (o, c) = (&other.open_areas[0], &crowded.open_areas[0]);
    assert!(o.required && c.required);
    assert_eq!((o.required_lx, c.required_lx), (1.0, 3.0));
    // the whole floor (no border): more points than EN's core area
    assert!(o.stats.points > en.open_areas[0].stats.points);
    assert_eq!(o.stats, c.stats);
}

#[test]
fn chinese_stairs_get_closer_luminaires() {
    use emergency::PlaceCategory as P;
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/emergency");
    let ldt = Eulumdat::from_file(dir.join("demo_em_route.ldt")).unwrap();
    let limits = Limits::gb55037();
    let s = Settings::default();
    // a brighter setting of the demo optic: 10 lx on stairs is a lot
    let flux = 900.0;
    let spacing = emergency::spacing_table(&ldt, flux, &[3.0], 2.0, &limits, &s).remove(0);
    let layout = |place| {
        let routes = vec![EscapeRoute {
            name: "1".into(),
            path: vec![(0.0, 1.0), (24.0, 1.0)],
            width: 2.0,
            place,
        }];
        let input = emergency::AutoInput {
            ldt: &ldt,
            flux_lm: flux,
            height: 3.0,
            ra: None,
            spacing: &spacing,
            routes: &routes,
            room: None,
            open_area: None,
            open_area_place: P::CrowdedPlace,
            points: &[],
        };
        let pos = emergency::auto_layout(&input, &limits, &s);
        let rep = emergency::evaluate(&input.plan(&pos), &limits, &s);
        (pos.len(), rep.routes[0].clone())
    };
    let (n_corridor, corridor) = layout(P::EscapeCorridor);
    let (n_stairs, stairs) = layout(P::StairsOrRefuge);
    eprintln!(
        "corridor: {n_corridor} luminaires, band min {:.2} lx | stairs: {n_stairs}, band min {:.2} lx",
        corridor.band.min_lx, stairs.band.min_lx
    );
    assert!(corridor.pass(), "{corridor:?}");
    assert!(stairs.pass(), "{:?}", stairs.band);
    assert!(n_stairs > n_corridor);
}

// ─── report rows per profile ─────────────────────────────────────────

/// The corridor of `one_corridor_two_profiles`, with an extinguisher.
fn corridor_plan(ldt: &Eulumdat, place: emergency::PlaceCategory) -> EmergencyPlan<'_> {
    EmergencyPlan {
        luminaires: (0..=5)
            .map(|k| lum(ldt, 300.0, k as f64 * 4.0, 0.0, 2.8))
            .collect(),
        routes: vec![EscapeRoute {
            name: "1".into(),
            path: vec![(0.0, 0.0), (20.0, 0.0)],
            width: 2.0,
            place,
        }],
        points: vec![EmphasisPoint {
            kind: EmphasisKind::FireEquipment,
            x: 10.0,
            y: 1.0,
            z: None,
            facing: Some(270.0),
        }],
        ..Default::default()
    }
}

#[test]
fn report_rows_follow_the_profile() {
    use emergency::{CheckKind as K, PlaceCategory as P, Subject};
    let ldt = hemisphere();
    let s = Settings::default();
    let kinds =
        |r: &emergency::EmergencyReport| -> Vec<K> { r.checks.iter().map(|c| c.kind).collect() };
    let en = emergency::evaluate(
        &corridor_plan(&ldt, P::EscapeCorridor),
        &Limits::en1838(),
        &s,
    );
    assert_eq!(
        kinds(&en),
        vec![
            K::CentreMin,
            K::BandMin,
            K::RouteRatio,
            K::Vertical,
            K::Glare,
            K::Ra
        ]
    );
    assert!(en.checks.iter().all(|c| c.pass.is_some()));
    let cn = emergency::evaluate(
        &corridor_plan(&ldt, P::EscapeCorridor),
        &Limits::gb55037(),
        &s,
    );
    // China: the floor minimum over the central half; equipment reported only
    assert_eq!(kinds(&cn), vec![K::BandMin, K::Vertical]);
    assert_eq!(cn.checks[0].limit, 3.0);
    assert_eq!(cn.checks[1].pass, None);
    assert_eq!(cn.checks[0].subject, Subject::Route(0));
    // the English rows name what each profile measures
    let [what, value, req] = en.describe(&en.checks[1], &Limits::en1838());
    assert_eq!(what, "Route 1: floor min, central half");
    assert!(value.ends_with(" lx") && req == "≥ 0.5 lx", "{value} {req}");
    let [what, _, req] = cn.describe(&cn.checks[0], &Limits::gb55037());
    assert_eq!(
        what,
        "Route 1 (evacuation corridor): floor min, central half"
    );
    assert_eq!(req, "≥ 3 lx");
    let [_, _, req] = cn.describe(&cn.checks[1], &Limits::gb55037());
    assert_eq!(req, "no requirement");
    // pass = every row with a limit
    assert_eq!(en.pass(), en.checks.iter().all(|c| c.pass != Some(false)));
    assert!(en.has_checks() && cn.has_checks());
}

#[test]
fn every_profile_has_an_id_name_and_region() {
    let all = Limits::all();
    assert!(all.len() >= 2);
    for l in &all {
        assert!(
            !l.id.is_empty() && !l.name.is_empty() && !l.region.is_empty(),
            "{l:?}"
        );
        assert_eq!(Limits::by_id(&l.id).as_ref(), Some(l));
        // something to check on a route
        let r = &l.escape_route;
        let lux = r.centre_min_lx > 0.0 || r.band_min_lx > 0.0 || r.avg_min_lx > 0.0;
        let layout = r.min_luminaires > 0 || r.max_spacing_m.is_finite();
        assert!(lux || layout, "{}", l.id);
        assert_eq!(
            lux, !l.lux_informative,
            "{}: lux rules or lux for information",
            l.id
        );
        assert!(l.escape_route.band_share > 0.0 && l.escape_route.band_share <= 1.0);
        if let Some(p) = &l.places {
            assert!(
                !p.route_places.is_empty() && !p.area_places.is_empty(),
                "{}",
                l.id
            );
        }
    }
    let mut ids: Vec<&str> = all.iter().map(|l| l.id.as_str()).collect();
    ids.dedup();
    assert_eq!(ids.len(), all.len(), "ids are unique");
    assert_eq!(
        Limits::by_id("CN").map(|l| l.id),
        Some("gb55037".to_string())
    );
}

#[test]
fn en1838_2024_lights_the_route_width_less_the_edge_strips() {
    use emergency::{CheckKind as K, PlaceCategory as P};
    let l = Limits::en1838_2024();
    let r = &l.escape_route;
    // up to 2 m: the central half (W / 4 on each side); wider: 0.5 m strips
    assert!(close(r.band_half_width(2.0), 0.5, 1e-12));
    assert!(close(r.band_half_width(1.2), 0.3, 1e-12));
    assert!(close(r.band_half_width(4.0), 1.5, 1e-12));
    assert!(close(
        Limits::en1838().escape_route.band_half_width(4.0),
        1.0,
        1e-12
    ));
    let ldt = hemisphere();
    let s = Settings::default();
    let rep = emergency::evaluate(&corridor_plan(&ldt, P::EscapeCorridor), &l, &s);
    let kinds: Vec<K> = rep.checks.iter().map(|c| c.kind).collect();
    assert_eq!(kinds[..3], [K::BandMin, K::RouteRatio, K::RouteCount]);
    assert_eq!(rep.checks[0].limit, 1.0, "1 lx over the band, not 0.5");
    assert_eq!(
        rep.checks[2].value,
        Some(6.0),
        "six luminaires on the corridor"
    );
    // one luminaire is not enough (§5.1.8), however bright
    let one = EmergencyPlan {
        luminaires: vec![lum(&ldt, 5000.0, 10.0, 0.0, 2.8)],
        ..corridor_plan(&ldt, P::EscapeCorridor)
    };
    let rep = emergency::evaluate(&one, &l, &s);
    let count = rep.checks.iter().find(|c| c.kind == K::RouteCount).unwrap();
    assert_eq!(count.pass, Some(false));
    assert!(!rep.routes[0].pass() && !rep.pass());
    // the UK code takes the same values, with its own durations
    let uk = Limits::bs5266();
    assert_eq!(uk.escape_route, l.escape_route);
    assert_eq!(uk.timing.durations[0].hours, 3.0);
}

#[test]
fn russia_wants_5_lx_in_front_of_exits() {
    use emergency::{CheckKind as K, PlaceCategory as P};
    let ldt = hemisphere();
    let mut plan = corridor_plan(&ldt, P::EscapeCorridor);
    plan.points.push(EmphasisPoint {
        kind: EmphasisKind::Exit,
        x: 20.0,
        y: 0.0,
        z: None,
        facing: Some(180.0),
    });
    let s = Settings::default();
    let ru = emergency::evaluate(&plan, &Limits::sp52(), &s);
    let exit = ru.checks.iter().find(|c| c.kind == K::ExitFloor).unwrap();
    // 0.5 m in front of the door, on the floor, from the luminaire above
    let e = emergency::illuminance(&plan.luminaires, (19.5, 0.0, 0.0), (0.0, 0.0, 1.0), 0.8);
    assert!(close(exit.value.unwrap(), e, 1e-9));
    assert_eq!(exit.limit, 5.0);
    assert_eq!(exit.pass, Some(e >= 5.0));
    // EN 1838 has no such rule
    let en = emergency::evaluate(&plan, &Limits::en1838(), &s);
    assert!(en.checks.iter().all(|c| c.kind != K::ExitFloor));
    assert_eq!(Limits::sp52().timing.full_s, 10.0);
}

#[test]
fn spain_checks_equipment_horizontally_and_rooms_up_to_1_m() {
    use emergency::{CheckKind as K, PlaceCategory as P};
    let ldt = hemisphere();
    let s = Settings::default();
    let plan = corridor_plan(&ldt, P::EscapeCorridor);
    let es = emergency::evaluate(&plan, &Limits::cte_sua4(), &s);
    let en = emergency::evaluate(&plan, &Limits::en1838(), &s);
    // horizontal at the extinguisher's height, not vertical towards the room
    let h = emergency::illuminance(&plan.luminaires, (10.0, 1.0, 1.2), (0.0, 0.0, 1.0), 0.8);
    assert!(close(es.points[0].vertical_lx.unwrap(), h, 1e-9));
    assert!(!close(en.points[0].vertical_lx.unwrap(), h, 1e-3));
    assert!(
        es.checks.iter().all(|c| c.kind != K::Glare),
        "no glare limit"
    );
    // anti-panic: any size, no border, the floor and the 1 m plane
    let small = vec![(0.0, 2.0), (5.0, 2.0), (5.0, 7.0), (0.0, 7.0)];
    let room = EmergencyPlan {
        open_areas: vec![OpenArea {
            name: "room".into(),
            polygon: small,
            place: P::CrowdedPlace,
        }],
        ..corridor_plan(&ldt, P::EscapeCorridor)
    };
    let es = emergency::evaluate(&room, &Limits::cte_sua4(), &s);
    let en = emergency::evaluate(&room, &Limits::en1838(), &s);
    assert!(es.open_areas[0].required && !en.open_areas[0].required);
    assert_eq!(
        es.open_areas[0].stats.points,
        2 * 20 * 20,
        "0.25 m grid, two planes"
    );
}

#[test]
fn france_counts_lumens_and_spacings_not_lux() {
    use emergency::{CheckKind as K, PlaceCategory as P};
    let ldt = hemisphere();
    let s = Settings::default();
    let l = Limits::fr_erp();
    let route = |spacing: f64, flux: f64| EmergencyPlan {
        luminaires: (0..)
            .map(|k| k as f64 * spacing)
            .take_while(|x| *x <= 20.0 + 1e-9)
            .map(|x| lum(&ldt, flux, x, 0.0, 2.8))
            .collect(),
        ..corridor_plan(&ldt, P::EscapeCorridor)
    };
    let ok = emergency::evaluate(&route(10.0, 60.0), &l, &s);
    let find = |r: &emergency::EmergencyReport, k: K| {
        r.checks.iter().find(|c| c.kind == k).cloned().unwrap()
    };
    assert_eq!(find(&ok, K::RouteSpacing).value, Some(10.0));
    assert_eq!(find(&ok, K::RouteCount).value, Some(3.0));
    assert_eq!(find(&ok, K::LuminaireFlux).pass, Some(true));
    // lux only for information
    let band = find(&ok, K::BandMin);
    assert_eq!((band.pass, band.limit), (None, 0.0));
    assert!(ok.pass(), "{:?}", ok.checks);
    // 16 m apart: too far; 40 lm: too little
    let far = emergency::evaluate(&route(16.0, 60.0), &l, &s);
    assert_eq!(find(&far, K::RouteSpacing).pass, Some(false));
    let dim = emergency::evaluate(&route(10.0, 40.0), &l, &s);
    assert_eq!(find(&dim, K::LuminaireFlux).pass, Some(false));
    // ambient lighting: 5 lm per m², neighbours within 4 mounting heights
    let hall = vec![(0.0, 2.0), (20.0, 2.0), (20.0, 12.0), (0.0, 12.0)];
    let plan = |flux: f64| EmergencyPlan {
        luminaires: [(5.0, 7.0), (15.0, 7.0)]
            .iter()
            .map(|&(x, y)| lum(&ldt, flux, x, y, 3.0))
            .collect(),
        open_areas: vec![OpenArea {
            name: "hall".into(),
            polygon: hall.clone(),
            place: P::CrowdedPlace,
        }],
        ..Default::default()
    };
    let lit = emergency::evaluate(&plan(500.0), &l, &s);
    assert!(close(
        find(&lit, K::AreaFlux).value.unwrap(),
        1000.0 / 200.0,
        1e-12
    ));
    assert_eq!(find(&lit, K::AreaFlux).pass, Some(true));
    assert!(close(
        find(&lit, K::AreaSpacing).value.unwrap(),
        10.0 / 3.0,
        1e-12
    ));
    assert_eq!(find(&lit, K::AreaCount).pass, Some(true));
    let dim = emergency::evaluate(&plan(400.0), &l, &s);
    assert_eq!(find(&dim, K::AreaFlux).pass, Some(false));
    // the spacing table follows the spacing rules
    let rows = emergency::spacing_table(&ldt, 100.0, &[3.0], 2.0, &l, &s);
    assert_eq!(rows[0].route_spacing_c0_m, 15.0);
    assert_eq!(rows[0].open_area_grid_m, 12.0);
}

#[test]
fn us_codes_check_average_minimum_and_the_end_of_the_duration() {
    use emergency::{CheckKind as K, PlaceCategory as P};
    let ldt = hemisphere();
    let l = Limits::nfpa101();
    let plan = corridor_plan(&ldt, P::EscapeCorridor);
    let s = Settings::default();
    let rep = emergency::evaluate(&plan, &l, &s);
    let kinds: Vec<K> = rep.checks.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        vec![
            K::BandMin,
            K::RouteAverage,
            K::RouteRatio,
            K::EndAverage,
            K::EndMin,
            K::Vertical
        ]
    );
    // over the full width: the band edges are the walls
    assert!(close(l.escape_route.band_half_width(2.0), 1.0, 1e-12));
    let r = &rep.routes[0];
    assert_eq!(rep.checks[1].value, Some(r.band.avg_lx));
    assert_eq!((rep.checks[0].limit, rep.checks[1].limit), (1.1, 10.8));
    // the extinguisher has no rule in the US (signs only)
    assert_eq!(rep.checks[5].pass, None);
    // the 1.5 h values follow the output at the end of the duration
    let half = emergency::evaluate(
        &plan,
        &l,
        &Settings {
            end_output_share: 0.5,
            ..Default::default()
        },
    );
    let end = |r: &emergency::EmergencyReport| {
        r.checks
            .iter()
            .find(|c| c.kind == K::EndAverage)
            .unwrap()
            .value
            .unwrap()
    };
    assert!(close(end(&half), 0.5 * end(&rep), 1e-12));
    assert!(close(end(&rep), r.band.avg_lx, 1e-12));
    // ~3–6 lx: far below the 10.8 lx average the US codes ask for
    assert!(!rep.pass());
    // a brighter corridor passes, and the decline to 60 % is allowed
    let bright = EmergencyPlan {
        luminaires: (0..=10)
            .map(|k| lum(&ldt, 900.0, k as f64 * 2.0, 0.0, 2.8))
            .collect(),
        ..corridor_plan(&ldt, P::EscapeCorridor)
    };
    let at = |share: f64| {
        emergency::evaluate(
            &bright,
            &l,
            &Settings {
                end_output_share: share,
                ..Default::default()
            },
        )
    };
    assert!(at(1.0).pass(), "{:?}", at(1.0).checks);
    assert!(at(0.6).pass());
    // far below 60 % the 1.5 h values fail
    let avg = at(1.0).routes[0].band.avg_lx;
    assert!(!at(6.0 / avg).pass() && at(6.6 / avg).routes[0].pass_end);
}

#[test]
fn place_profiles_of_the_americas_and_asia() {
    use emergency::{CheckKind as K, PlaceCategory as P};
    let ldt = hemisphere();
    let s = Settings::default();
    let plan = corridor_plan(&ldt, P::EscapeCorridor);
    let first = |l: &Limits, place| {
        let r = emergency::evaluate(&corridor_plan(&ldt, place), l, &s);
        (r.checks[0].kind, r.checks[0].limit)
    };
    // Brazil: 3 lx on flat ground, 5 lx on stairs, ≤ 20 : 1
    let br = Limits::nbr10898();
    assert_eq!(first(&br, P::EscapeCorridor), (K::BandMin, 3.0));
    assert_eq!(first(&br, P::StairsOrRefuge), (K::BandMin, 5.0));
    let rep = emergency::evaluate(&plan, &br, &s);
    let ratio = rep.checks.iter().find(|c| c.kind == K::RouteRatio).unwrap();
    assert_eq!(
        (ratio.limit, ratio.value),
        (20.0, Some(rep.routes[0].band.ratio))
    );
    // New Zealand: 1 lx in exitways, 0.2 lx elsewhere
    let nz = Limits::nz_f6();
    assert_eq!(first(&nz, P::EscapeCorridor), (K::BandMin, 1.0));
    assert_eq!(first(&nz, P::OtherPlace), (K::BandMin, 0.2));
    // Australia: 0.2 lx minimum and 0.5 lx average, stairs 1 lx
    let au = Limits::as2293();
    let rep = emergency::evaluate(&plan, &au, &s);
    assert_eq!(
        rep.checks
            .iter()
            .map(|c| (c.kind, c.limit))
            .take(2)
            .collect::<Vec<_>>(),
        vec![(K::BandMin, 0.2), (K::RouteAverage, 0.5)]
    );
    assert_eq!(first(&au, P::StairsOrRefuge), (K::BandMin, 1.0));
    // Japan: 2 lx everywhere on the floor, the full width
    let jp = Limits::jp_bsl();
    assert_eq!(first(&jp, P::EscapeCorridor), (K::BandMin, 2.0));
    assert!(close(jp.escape_route.band_half_width(2.0), 1.0, 1e-12));
    // India: 10 lx on the centre line, 5 lx over the central half
    let rep = emergency::evaluate(&plan, &Limits::nbc_in(), &s);
    assert_eq!(
        rep.checks
            .iter()
            .map(|c| (c.kind, c.limit))
            .take(2)
            .collect::<Vec<_>>(),
        vec![(K::CentreMin, 10.0), (K::BandMin, 5.0)]
    );
    // Canada: average 10 lx, minimum 1 lx, no ratio
    let rep = emergency::evaluate(&plan, &Limits::nbcc(), &s);
    assert_eq!(
        rep.checks
            .iter()
            .map(|c| (c.kind, c.limit))
            .collect::<Vec<_>>(),
        vec![
            (K::BandMin, 1.0),
            (K::RouteAverage, 10.0),
            (K::Vertical, 0.0)
        ]
    );
    // every profile's English rows read
    for l in Limits::all() {
        let rep = emergency::evaluate(&plan, &l, &s);
        for c in &rep.checks {
            let [what, value, req] = rep.describe(c, &l);
            assert!(
                !what.is_empty() && !value.is_empty() && !req.is_empty(),
                "{} {c:?}",
                l.id
            );
        }
        assert!(!l.not_calculated().is_empty());
    }
}

#[test]
fn profiles_match_their_data_file() {
    let v: serde_json::Value = serde_json::from_str(emergency::PROFILES_JSON).unwrap();
    let list = v["profiles"].as_array().unwrap();
    let get = |id: &str| -> &serde_json::Value {
        list.iter()
            .find(|p| p["id"] == id)
            .unwrap_or_else(|| panic!("{id} missing"))
    };
    // every profile is either here or in its own file
    for l in Limits::all() {
        if l.id != "en1838" && l.id != "gb55037" {
            let p = get(&l.id);
            assert_eq!(p["name"].as_str(), Some(l.name.as_str()), "{}", l.id);
            assert!(!p["sources"].as_array().unwrap().is_empty(), "{}", l.id);
        }
    }
    let n = |id: &str, k: &str| get(id)["values"][k].as_f64().unwrap();
    let e = Limits::en1838_2024();
    assert_eq!(
        n("en1838-2024", "route_min_lx_over_band"),
        e.escape_route.band_min_lx
    );
    assert_eq!(
        n("en1838-2024", "route_edge_max_m"),
        e.escape_route.edge_max_m
    );
    assert_eq!(
        n("en1838-2024", "route_max_ratio"),
        e.escape_route.max_ratio
    );
    assert_eq!(n("en1838-2024", "open_area_min_lx"), e.anti_panic.min_lx);
    let es = Limits::cte_sua4();
    assert_eq!(
        n("cte-sua4", "anti_panic_planes_up_to_m"),
        es.anti_panic.planes_up_to_m
    );
    assert_eq!(
        n("cte-sua4", "equipment_horizontal_lx"),
        es.emphasis_min_vertical_lx
    );
    let fr = Limits::fr_erp();
    assert_eq!(
        n("fr-erp", "route_max_spacing_m"),
        fr.escape_route.max_spacing_m
    );
    assert_eq!(n("fr-erp", "luminaire_min_lm"), fr.min_luminaire_lm);
    assert_eq!(
        n("fr-erp", "ambient_min_lm_per_m2"),
        fr.anti_panic.min_flux_per_m2
    );
    assert_eq!(
        n("fr-erp", "ambient_max_spacing_per_height"),
        fr.anti_panic.max_spacing_per_height
    );
    let ru = Limits::sp52();
    assert_eq!(n("sp52", "exit_floor_lx"), ru.exit_floor_min_lx);
    assert_eq!(
        get("sp52")["values"]["response_s"][1].as_f64(),
        Some(ru.timing.full_s)
    );
    let us = Limits::nfpa101();
    let end = us.end_of_duration.clone().unwrap();
    assert_eq!(n("nfpa101", "route_avg_lx"), us.escape_route.avg_min_lx);
    assert_eq!(n("nfpa101", "route_min_lx"), us.escape_route.band_min_lx);
    assert_eq!(n("nfpa101", "end_avg_lx"), end.avg_min_lx);
    assert_eq!(n("nfpa101", "end_min_lx"), end.min_lx);
    assert_eq!(
        n("nfpa101", "external_sign_face_lx"),
        us.emphasis_min_vertical_lx
    );
    let ca = Limits::nbcc();
    assert_eq!(
        (n("nbcc", "avg_lx"), n("nbcc", "min_lx")),
        (ca.escape_route.avg_min_lx, ca.escape_route.band_min_lx)
    );
    let br = Limits::nbr10898().places.unwrap();
    assert_eq!(
        (n("nbr10898", "flat_lx"), n("nbr10898", "level_change_lx")),
        (br.corridor_lx, br.stairs_or_refuge_lx)
    );
    assert_eq!(
        n("nbr10898", "max_ratio"),
        Limits::nbr10898().escape_route.max_ratio
    );
    assert_eq!(
        n("jp-bsl", "floor_min_lx_fluorescent_led"),
        Limits::jp_bsl().escape_route.band_min_lx
    );
    let ind = Limits::nbc_in();
    assert_eq!(
        (n("nbc-in", "route_centre_lx"), n("nbc-in", "route_band_lx")),
        (ind.escape_route.centre_min_lx, ind.escape_route.band_min_lx)
    );
    let au = Limits::as2293();
    assert_eq!(
        (n("as2293", "route_min_lx"), n("as2293", "route_avg_lx")),
        (au.escape_route.band_min_lx, au.escape_route.avg_min_lx)
    );
    assert_eq!(
        n("as2293", "stairs_lx"),
        au.places.unwrap().stairs_or_refuge_lx
    );
    let nz = Limits::nz_f6().places.unwrap();
    assert_eq!(
        (n("nz-f6", "exitway_lx"), n("nz-f6", "elsewhere_lx")),
        (nz.corridor_lx, nz.other_lx)
    );
}

/// The landing page's L-shaped office laid out automatically for `limits`.
fn l_office_auto(limits: &Limits) -> (usize, emergency::EmergencyReport) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/emergency");
    let ldt = Eulumdat::from_file(dir.join("demo_em_route.ldt")).unwrap();
    let s = Settings::default();
    let room = vec![
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 7.5),
        (10.0, 7.5),
        (10.0, 15.0),
        (0.0, 15.0),
    ];
    let routes = vec![EscapeRoute {
        name: "1".into(),
        path: vec![(19.5, 3.75), (5.0, 3.75), (5.0, 14.5)],
        width: 2.0,
        place: limits
            .places
            .as_ref()
            .and_then(|p| p.route_places.first().copied())
            .unwrap_or_default(),
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
    let flux = emergency::luminaire_flux_lm(&ldt);
    let spacing = emergency::spacing_table(&ldt, flux, &[3.0], 2.0, limits, &s).remove(0);
    let input = emergency::AutoInput {
        ldt: &ldt,
        flux_lm: flux,
        height: 3.0,
        ra: Some(80.0),
        spacing: &spacing,
        routes: &routes,
        room: Some(&room),
        open_area: Some(&room),
        open_area_place: limits
            .places
            .as_ref()
            .and_then(|p| p.area_places.first().copied())
            .unwrap_or(emergency::PlaceCategory::CrowdedPlace),
        points: &points,
    };
    let pos = emergency::auto_layout(&input, limits, &s);
    let rep = emergency::evaluate(&input.plan(&pos), limits, &s);
    (pos.len(), rep)
}

#[test]
fn the_demo_office_passes_every_profile_automatically() {
    let mut failed = Vec::new();
    for l in Limits::all() {
        let (n, rep) = l_office_auto(&l);
        let bad: Vec<String> = rep
            .checks
            .iter()
            .filter(|c| c.pass == Some(false))
            .map(|c| {
                let [w, v, r] = rep.describe(c, &l);
                format!("{w}: {v} ({r})")
            })
            .collect();
        eprintln!(
            "{:<12} {n:>3} luminaires {}",
            l.id,
            if bad.is_empty() {
                "✓".to_string()
            } else {
                bad.join("; ")
            }
        );
        if !bad.is_empty() {
            failed.push(l.id.clone());
        }
    }
    assert!(failed.is_empty(), "automatic layout fails {failed:?}");
}
