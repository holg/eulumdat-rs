//! IES TM-33-23 export (XML + JSON, `atla::tm33`): units, structure and the
//! round trip back to the data model / EULUMDAT.

use eulumdat::atla::{self, tm33, LuminaireOpticalData, SpectralUnits};
use eulumdat::Eulumdat;
use std::path::PathBuf;

fn templates() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("eulumdat-wasm/templates")
}

fn opts() -> tm33::WriteOptions {
    tm33::WriteOptions {
        today: Some("2026-11-07".into()),
        creator: None,
    }
}

fn ldt(name: &str) -> Eulumdat {
    Eulumdat::from_file(templates().join(name)).unwrap()
}

fn assert_close(a: f64, b: f64, rel: f64, what: &str) {
    let tol = rel * a.abs().max(b.abs()).max(1e-9);
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b}");
}

/// The intensity tables of two LDTs agree (relative 1e-5).
fn same_intensities(a: &Eulumdat, b: &Eulumdat) {
    assert_eq!(a.intensities.len(), b.intensities.len(), "planes");
    for (ra, rb) in a.intensities.iter().zip(&b.intensities) {
        assert_eq!(ra.len(), rb.len());
        for (x, y) in ra.iter().zip(rb) {
            assert!((x - y).abs() <= 1e-5 * x.abs().max(1.0), "{x} vs {y}");
        }
    }
}

#[test]
fn ldt_becomes_conformant_tm33_xml() {
    let src = ldt("fluorescent_luminaire.ldt");
    let doc = LuminaireOpticalData::from_eulumdat(&src);
    let xml = tm33::write_xml(&doc, &opts()).unwrap();

    assert!(xml.contains("<IESTM33-22>"));
    assert!(xml.contains("<Version>1.1</Version>"));
    // dimensions in metres, number of emitters
    assert!(xml.contains("<Length>1.17</Length>"), "{xml}");
    assert!(xml.contains("<NumEmitter>1</NumEmitter>"));
    // underscores, not the PDF's " _ "
    assert!(
        xml.contains("<Symm>Symm_Quad</Symm>"),
        "C0-C90 symmetry (Isym 4)"
    );
    assert!(!xml.contains(" _ "));
    // required header fields are present
    for tag in [
        "<Description>",
        "<Laboratory>",
        "<ReportNumber>",
        "<ReportDate>",
    ] {
        assert!(xml.contains(tag), "{tag}");
    }
    // intensities in candela: cd/klm × lamp klm
    let flux = src.lamp_sets[0].total_luminous_flux;
    let first = src.intensities[0][0] * flux / 1000.0;
    let expect = format!(">{}</IntData>", tm33_sig(first));
    assert!(xml.contains(&expect), "first value {expect}");
    assert!(xml.contains("<LuminousFlux>"));
    // luminous area as the bottom emission area
    assert!(xml.contains("<EmissionAreas>") && xml.contains("<BottomArea>"));

    // back: same table, lamp flux, dimensions, luminous area
    let back = atla::xml::parse(&xml).unwrap();
    assert_eq!(back.schema_version, atla::SchemaVersion::Tm3323);
    let ldt_back = back.to_eulumdat();
    same_intensities(&src, &ldt_back);
    assert_close(ldt_back.length, src.length, 1e-9, "length");
    assert_close(ldt_back.width, src.width, 1e-9, "width");
    assert_close(
        ldt_back.luminous_area_length,
        src.luminous_area_length,
        1e-9,
        "lum. length",
    );
    assert_close(
        ldt_back.luminous_area_width,
        src.luminous_area_width,
        1e-9,
        "lum. width",
    );
    assert_close(
        ldt_back.lamp_sets[0].total_luminous_flux,
        flux,
        1e-9,
        "lamp flux",
    );
}

/// Same formatting as the writer (6 significant digits).
fn tm33_sig(v: f64) -> String {
    let mag = v.abs().log10().floor() as i32;
    let d = (5 - mag).max(0) as usize;
    let s = format!("{v:.d$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[test]
fn luminous_flux_is_the_flux_of_the_table() {
    for name in [
        "road_luminaire.ldt",
        "projector.ldt",
        "fluorescent_luminaire.ldt",
    ] {
        let src = ldt(name);
        let doc = LuminaireOpticalData::from_eulumdat(&src);
        let xml = tm33::write_xml(&doc, &opts()).unwrap();
        let flux: f64 = xml
            .split("<LuminousFlux>")
            .nth(1)
            .and_then(|s| s.split('<').next())
            .and_then(|s| s.parse().ok())
            .expect("LuminousFlux");
        // the crate's own (symmetry-aware) integral of the cd/klm table
        let mut unit = src.clone();
        unit.conversion_factor = 1.0;
        let per_klm = eulumdat::PhotometricCalculations::calculated_luminous_flux(&unit);
        let lamp = src.lamp_sets[0].total_luminous_flux;
        assert_close(flux, per_klm * lamp / 1000.0, 0.05, name);
        let ldt_back = atla::xml::parse(&xml).unwrap().to_eulumdat();
        same_intensities(&src, &ldt_back);
    }
}

#[test]
fn spectrum_is_emitter_spectral_and_comes_back() {
    let text = std::fs::read_to_string(templates().join("demo_led_3000k.xml")).unwrap();
    let doc = atla::parse(&text).unwrap();
    let spd = doc.emitters[0]
        .spectral_distribution
        .clone()
        .expect("demo has an SPD");
    let xml = tm33::write_xml(&doc, &opts()).unwrap();
    assert!(xml.contains("<SpectralData>") && xml.contains("<EmitterSpectral>"));
    assert!(xml.contains("<Normalized>true</Normalized>"));
    assert!(xml.contains("<PwrData w=\""));
    assert!(!xml.contains("<SpectralDistribution>"));
    assert!(tm33::contents(&doc).spectrum);

    let back = atla::xml::parse(&xml).unwrap();
    let s2 = back.emitters[0]
        .spectral_distribution
        .clone()
        .expect("spectrum back");
    assert_eq!(s2.units, SpectralUnits::Relative);
    assert_eq!(s2.values.len(), spd.values.len());
    let peak = spd.values.iter().cloned().fold(0.0, f64::max);
    for (a, b) in spd.values.iter().zip(&s2.values) {
        assert!((a / peak - b).abs() < 1e-5, "{a} vs {b}");
    }
    // colour values: Ra in CIE_CRI
    if doc.emitters[0]
        .color_rendering
        .as_ref()
        .and_then(|c| c.ra)
        .is_some()
    {
        assert!(xml.contains("<CIE_CRI>"));
    }
}

#[test]
fn json_carries_the_same_data_as_xml() {
    for name in ["fluorescent_luminaire.ldt", "road_luminaire.ldt"] {
        let doc = LuminaireOpticalData::from_eulumdat(&ldt(name));
        let xml = tm33::write_xml(&doc, &opts()).unwrap();
        let json = tm33::write_json(&doc, &opts()).unwrap();
        assert!(json.contains("\"FileType\": \"IESTM33-22\""));
        assert!(json.contains("\"IntDataSymm\""), "{name}: planes as a grid");
        assert!(json.contains("\"Emitter\": ["), "Emitter is an array");
        serde_json::from_str::<serde_json::Value>(&json).expect("valid JSON");

        let a = atla::xml::parse(&xml).unwrap();
        let b = tm33::parse_json(&json).unwrap();
        // also through the generic JSON entry point
        let c = atla::json::parse(&json).unwrap();
        for d in [&b, &c] {
            assert_eq!(d.header.description, a.header.description);
            assert_eq!(d.header.report_date, a.header.report_date);
            assert_eq!(d.header.comments_list, a.header.comments_list);
            same_intensities(&a.to_eulumdat(), &d.to_eulumdat());
            assert_eq!(
                d.luminaire.as_ref().unwrap().luminous_openings.len(),
                a.luminaire.as_ref().unwrap().luminous_openings.len()
            );
        }
    }
    // spectrum through JSON
    let text = std::fs::read_to_string(templates().join("demo_led_3000k.xml")).unwrap();
    let doc = atla::parse(&text).unwrap();
    let json = tm33::write_json(&doc, &opts()).unwrap();
    assert!(json.contains("\"PwrDataArray\""));
    let back = tm33::parse_json(&json).unwrap();
    assert_eq!(
        back.emitters[0]
            .spectral_distribution
            .as_ref()
            .unwrap()
            .values
            .len(),
        doc.emitters[0]
            .spectral_distribution
            .as_ref()
            .unwrap()
            .values
            .len()
    );
}

#[test]
fn round_luminaire_is_an_align_z_cylinder() {
    let text = std::fs::read_to_string(templates().join("demo_led_3000k.xml")).unwrap();
    let doc = atla::parse(&text).unwrap();
    let src = doc.to_eulumdat();
    assert_eq!(src.width, 0.0, "the demo downlight is round");
    let xml = tm33::write_xml(&doc, &opts()).unwrap();
    assert!(xml.contains("<Shape>Align_Z</Shape>"));
    let back = atla::xml::parse(&xml).unwrap().to_eulumdat();
    assert_eq!(back.width, 0.0);
    assert_close(back.length, src.length, 1e-9, "diameter");
}

#[test]
fn bim_custom_data_survives_xml_and_json() {
    let text =
        std::fs::read_to_string(templates().join("tm-32-24_office_downlight_bim.xml")).unwrap();
    let doc = atla::parse(&text).unwrap();
    let c = tm33::contents(&doc);
    assert!(c.bim, "TM-32-24 blocks recognised");
    let before = atla::BimParameters::from_atla(&doc);

    let xml = tm33::write_xml(&doc, &opts()).unwrap();
    let json = tm33::write_json(&doc, &opts()).unwrap();
    assert!(json.contains("\"@guid\""), "attributes inside CustomData");
    for back in [
        atla::xml::parse(&xml).unwrap(),
        tm33::parse_json(&json).unwrap(),
    ] {
        assert_eq!(back.custom_data_items.len(), doc.custom_data_items.len());
        let after = atla::BimParameters::from_atla(&back);
        assert_eq!(after.populated_count(), before.populated_count());
        assert_eq!(after.bug_rating, before.bug_rating);
        assert_eq!(after.housing_color, before.housing_color);
        assert_eq!(after.weight, before.weight);
    }
}

#[test]
fn candela_files_are_read_as_candela() {
    // 1 plane, 2 000 lm rated, values in candela (flux ≈ 1 600 lm)
    let mut pts = String::new();
    for k in 0..=18 {
        let g = k as f64 * 5.0;
        let i = if g <= 90.0 {
            500.0 * g.to_radians().cos()
        } else {
            0.0
        };
        pts.push_str(&format!("<IntData h=\"0\" v=\"{g}\">{i}</IntData>"));
    }
    let xml = format!(
        "<IESTM33-22><Version>1.1</Version><Header><Description>t</Description>\
         <Laboratory>l</Laboratory><ReportNumber>r</ReportNumber><ReportDate>2024-01-01</ReportDate>\
         </Header><Luminaire><Dimensions><Length>0.6</Length><Width>0.6</Width><Height>0.05</Height>\
         </Dimensions><NumEmitter>1</NumEmitter></Luminaire><Emitter><Quantity>1</Quantity>\
         <Description>LED</Description><RatedLumens>2000</RatedLumens><InputWattage>20</InputWattage>\
         <LuminousData><LuminousIntensity><AbsolutePhotometry>False</AbsolutePhotometry>\
         <NumberMeasured>19</NumberMeasured><NumberHorz>1</NumberHorz><NumberVert>19</NumberVert>{pts}\
         </LuminousIntensity></LuminousData></Emitter></IESTM33-22>"
    );
    let doc = atla::xml::parse(&xml).unwrap();
    let ldt = doc.to_eulumdat();
    // 500 cd at nadir for 2 klm = 250 cd/klm
    assert_close(ldt.intensities[0][0], 250.0, 1e-9, "cd/klm");
    assert_close(ldt.length, 600.0, 1e-9, "metres → mm");
    assert_eq!(
        doc.emitters[0]
            .intensity_distribution
            .as_ref()
            .unwrap()
            .absolute_photometry,
        Some(false)
    );
}

#[test]
fn annex_a_json_form_is_read() {
    // h / v next to a flat IntDataSymm (one plane), as in the TM-33-23 annex
    let json = r#"{
        "FileType": "IESTM33-22", "Version": 1.0,
        "Header": {"Manufacturer": "M", "Description": "D", "Laboratory": "L",
                   "ReportNumber": "R", "ReportDate": "2024-01-01", "Comment": ["a", "b"]},
        "Luminaire": {"Dimensions": {"Length": 1.2, "Width": 0.6, "Height": 0.1}, "NumEmitter": 1},
        "Emitter": [{
            "Quantity": 1, "Description": "E", "InputWattage": 30,
            "ColorTemperature": {"FixedCCT": 4000},
            "ColorRendering": {"CIE_CRI": {"Ra": 92, "R9": 85}, "IES_TM30": {"Rf": 90, "Rg": 101}},
            "LuminousData": {
                "LuminousIntensity": {
                    "AbsolutePhotometry": true, "Symm": "Symm_Full",
                    "NumberMeasured": 3, "NumberHorz": 1, "NumberVert": 3,
                    "h": [0.0], "v": [0.0, 45.0, 90.0],
                    "IntDataSymm": [300, 200, 0]
                },
                "LuminousFlux": 1000.0
            }
        }]
    }"#;
    let doc = atla::json::parse(json).unwrap();
    assert_eq!(doc.header.comments_list, vec!["a", "b"]);
    let e = &doc.emitters[0];
    assert_eq!(e.cct, Some(4000.0));
    let cr = e.color_rendering.as_ref().unwrap();
    assert_eq!(
        (cr.ra, cr.r9, cr.rf, cr.rg),
        (Some(92.0), Some(85.0), Some(90.0), Some(101.0))
    );
    let d = e.intensity_distribution.as_ref().unwrap();
    assert_eq!(d.vertical_angles, vec![0.0, 45.0, 90.0]);
    // absolute photometry, 1 000 lm: candela = cd/klm
    assert_close(d.intensities[0][0], 300.0, 1e-9, "nadir");
    assert_close(
        doc.luminaire
            .as_ref()
            .unwrap()
            .dimensions
            .as_ref()
            .unwrap()
            .length,
        1200.0,
        1e-9,
        "mm",
    );
}

// ─── IES TM-27 SPDX (spectrum only) ──────────────────────────────────

fn spdx_opts() -> atla::spdx::WriteOptions {
    atla::spdx::WriteOptions {
        today: Some("2026-11-07".into()),
    }
}

#[test]
fn spectrum_becomes_a_tm27_spdx_document() {
    let text = std::fs::read_to_string(templates().join("demo_led_3000k.xml")).unwrap();
    let doc = atla::parse(&text).unwrap();
    let spd = doc.emitters[0].spectral_distribution.clone().unwrap();
    let x = atla::spdx::write(&doc, 0, &spdx_opts()).expect("the demo has a spectrum");
    assert!(x.contains("<IESTM2714 xmlns=\"http://www.ies.org/iestm2714\" version=\"1.0\">"));
    assert!(x.contains("<SpectralQuantity>relative</SpectralQuantity>"));
    for tag in [
        "<Description>",
        "<DocumentCreator>",
        "<DocumentCreationDate>2026-11-07<",
    ] {
        assert!(x.contains(tag), "{tag}");
    }
    assert!(atla::spdx::is_spdx(&x));

    let back = atla::spdx::parse(&x).unwrap();
    let s2 = spectrum_values(&atla::spdx::to_atla(&back));
    let peak = spd.values.iter().cloned().fold(0.0, f64::max);
    assert_eq!(s2.len(), spd.values.len());
    for (a, b) in spd.values.iter().zip(&s2) {
        assert!((a / peak - b).abs() < 1e-5, "{a} vs {b}");
    }
    assert!(
        s2.iter().all(|v| (0.0..=1.0).contains(v)),
        "relative data in 0..1"
    );
    // writing what was read gives the same document
    let again = atla::spdx::write(&atla::spdx::to_atla(&back), 0, &spdx_opts()).unwrap();
    assert_eq!(again, x);
    // no spectrum, no document
    let plain = LuminaireOpticalData::from_eulumdat(&ldt("road_luminaire.ldt"));
    assert!(atla::spdx::write(&plain, 0, &spdx_opts()).is_none());
}

/// The first emitter's spectral values of a document.
fn spectrum_values(doc: &LuminaireOpticalData) -> Vec<f64> {
    doc.emitters[0]
        .spectral_distribution
        .as_ref()
        .map(|s| s.values.clone())
        .unwrap_or_default()
}

#[test]
fn spdx_header_and_bandwidth_survive() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/files/Fluorescent.spdx");
    let src = atla::spdx::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let doc = atla::spdx::to_atla(&src);
    let x = atla::spdx::write(&doc, 0, &spdx_opts()).unwrap();
    let back = atla::spdx::parse(&x).unwrap();
    assert_eq!(back.header.description, src.header.description);
    assert_eq!(back.header.comments, src.header.comments);
    assert_eq!(back.header.unique_identifier, src.header.unique_identifier);
    assert_eq!(back.bandwidth_fwhm, src.bandwidth_fwhm);
    assert_eq!(back.wavelengths, src.wavelengths);
    let peak = src.values.iter().cloned().fold(0.0, f64::max);
    for (a, b) in src.values.iter().zip(&back.values) {
        assert!((a / peak - b).abs() < 1e-6);
    }
}

#[test]
fn absolute_spectrum_is_flux() {
    let mut doc = LuminaireOpticalData::from_eulumdat(&ldt("road_luminaire.ldt"));
    doc.emitters[0].spectral_distribution = Some(atla::SpectralDistribution {
        wavelengths: vec![400.0, 500.0, 600.0],
        values: vec![0.002, 0.004, 0.003],
        units: SpectralUnits::WattsPerNanometer,
        start_wavelength: None,
        wavelength_interval: None,
    });
    let x = atla::spdx::write(&doc, 0, &spdx_opts()).unwrap();
    assert!(x.contains("<SpectralQuantity>flux</SpectralQuantity>"));
    assert!(x.contains("<SpectralData wavelength=\"500\">0.004</SpectralData>"));
}
