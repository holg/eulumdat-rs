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
            },
            OpenArea {
                name: "store".into(),
                polygon: square(5.0),
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
