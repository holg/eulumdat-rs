//! Typst report of a TM-25 ray file, as the viewer's "PDF report" button
//! builds it, written as `.typ` for the Typst CLI.
//!
//!     cargo run -p eulumdat-tm25 --example tm25_report -- file.TM25RAY out.typ
//!     typst compile out.typ
//!
//! Far field, exports, spectrum colorimetry, quality and a luminance map
//! along the emission axis are computed here from the file; in the viewer
//! the same numbers come from the Bevy bundle's status.

use eulumdat_tm25::{luminance_map, FarFieldBuilder, FluxKind, Reservoir, Tm25File};
use eulumdat_typst::tm25_report::{
    FarFieldData, LuminanceData, QualityData, SpectrumData, Tm25ReportData,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        eprintln!("usage: tm25_report <file.TM25RAY> <out.typ>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(input).expect("read");
    let f = Tm25File::parse(&bytes).expect("parse");
    let h = &f.header;
    let kind = h.flux_kind();
    let mut ffb = FarFieldBuilder::new(10.0, 5.0, kind);
    let mut res = Reservoir::new(200_000, 1);
    let mut quality = eulumdat_tm25::QualityAccumulator::new(h);
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for r in f.rays.iter() {
        ffb.add(&r);
        quality.add(&r);
        for (i, v) in r.position().iter().enumerate() {
            min[i] = min[i].min(*v);
            max[i] = max[i].max(*v);
        }
        res.push(r);
    }
    let ff = ffb.finish();
    let rays = res.finish();
    let sm = ff.smoothed(1, 1, 12.0);
    let exports = eulumdat_tm25::exports(&sm, h, Some((min, max)));
    let ldt = eulumdat::Eulumdat::parse(&exports.ldt).ok();
    let prof = ff.azimuthal_average();
    let pmax = prof.iter().cloned().fold(0.0, f64::max).max(1e-12);
    let cut = |c_pos: f64, c_neg: f64| -> Vec<(f64, f64)> {
        let s = ff.smoothed(1, 0, 6.0);
        (0..=72)
            .map(|i| {
                let a = -90.0 + 2.5 * i as f64;
                (
                    a,
                    if a < 0.0 {
                        s.sample(c_neg, -a)
                    } else {
                        s.sample(c_pos, a)
                    },
                )
            })
            .collect()
    };
    let spectrum = h.shared_spectrum().map(|t| {
        let spd = eulumdat_spectrum::Spd::new(
            &t.wavelengths_nm
                .iter()
                .map(|w| *w as f64)
                .collect::<Vec<_>>(),
            &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        );
        let c = eulumdat_spectrum::colorimetry::analyze(&spd);
        let m = eulumdat_spectrum::SpectralMetrics::from_spd(&spd);
        SpectrumData {
            wavelengths: t.wavelengths_nm.iter().map(|w| *w as f64).collect(),
            values: t.values.iter().map(|v| *v as f64).collect(),
            peak_nm: t.peak().map(|p| p.0 as f64),
            ultraviolet: t.is_ultraviolet(),
            colorimetry: (!t.is_ultraviolet()).then_some((c.cct_k, c.duv, c.x, c.y)),
            ler_lm_per_w: Some(m.ler_lm_per_w),
        }
    });
    let luminance =
        luminance_map(&rays, kind, [0.0, 0.0, 1.0], 8.0, 0.1, 96).map(|m| LuminanceData {
            unit: m.unit().to_string(),
            pixel_mm: m.pixel_mm as f64,
            cone_deg: m.cone_deg as f64,
            width: m.width,
            height: m.height,
            peak: m.peak,
            mean_lit: m.mean_lit,
            rays_per_lit_pixel: m.rays_per_lit_pixel,
            values: m.values.clone(),
            direction: [
                m.direction[0] as f64,
                m.direction[1] as f64,
                m.direction[2] as f64,
            ],
        });
    let q = quality.finish();
    let data = Tm25ReportData {
        file_name: std::path::Path::new(input)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        text: h.text.raw.to_vec(),
        date: h.date_time.clone(),
        rays: h.n_rays,
        kept: rays.len() as u64,
        columns: h.columns.iter().map(|c| format!("{c:?}")).collect(),
        flux_kind: format!("{kind:?}"),
        radiant_flux_w: h.radiant_flux_w as f64,
        luminous_flux_lm: h.luminous_flux_lm as f64,
        bounds_min: [min[0] as f64, min[1] as f64, min[2] as f64],
        bounds_max: [max[0] as f64, max[1] as f64, max[2] as f64],
        far_field: FarFieldData {
            unit: if kind == FluxKind::Photometric {
                "cd".into()
            } else {
                "W/sr".into()
            },
            peak: pmax,
            gamma: ff.g_angles(),
            relative: prof.iter().map(|v| v / pmax).collect(),
            half_intensity_deg: ff.half_intensity_gamma(),
            cut_x: cut(0.0, 180.0),
            cut_y: cut(90.0, 270.0),
        },
        spectrum,
        irradiance: None,
        luminance,
        quality: QualityData {
            verdict: q.verdict(),
            findings: q
                .findings
                .iter()
                .map(|f| {
                    (
                        f.severity.as_str().to_string(),
                        f.code.to_string(),
                        f.message.clone(),
                    )
                })
                .collect(),
            far_field_noise: q.far_field_noise,
        },
        ldt,
        notes: Vec::new(),
        exports: vec![
            "EULUMDAT .ldt".into(),
            "IES .ies".into(),
            "ATLA .xml".into(),
            "LightTools .ray + .sre".into(),
            "quality report .txt".into(),
        ],
    };
    let typ = eulumdat_typst::generate_tm25_report(&data);
    std::fs::write(output, &typ).expect("write");
    // The exported LDT beside it, for anyone who wants the diagrams' input.
    let _ = std::fs::write(format!("{output}.ldt"), &exports.ldt);
    println!(
        "wrote {output}: {} bytes of Typst; compile with: typst compile {output}",
        typ.len()
    );
}
