//! Bridge between [`tm25ray`] (IES TM-25 ray files) and the eulumdat
//! ecosystem: turn a binned far field into an EULUMDAT document and a
//! TM-25 spectral table into an ATLA spectral distribution.
//!
//! Everything from `tm25ray` is re-exported, so `eulumdat_tm25::Tm25File`
//! and friends work as before; the eulumdat-specific additions are
//! [`FarFieldExt`], [`luminaire_from_far_field`] (header-aware document:
//! name, date, emitting area, lamp lumens), [`atla_from_luminaire`] (with the
//! header spectrum attached), [`spectral_table_to_atla`],
//! [`spectral_table_to_spd_text`] and the one-call [`exports`] used by the CLI
//! and the web viewer's download buttons.

pub use tm25ray::*;

use eulumdat::atla::{LuminaireOpticalData, SpectralDistribution, SpectralUnits};
use eulumdat::{Eulumdat, IesExporter, LampSet, Symmetry, TypeIndicator};

/// EULUMDAT conversion for [`FarField`].
pub trait FarFieldExt {
    /// Convert to an EULUMDAT document. Intensities are stored per kilo-unit
    /// of the header flux (cd/klm for photometric files; for radiometric
    /// files the same normalisation is applied to W and the result is
    /// relative, which the report-number field says explicitly).
    ///
    /// The LDT grid uses the conventional angles `C = 0, c_step, …` and
    /// `γ = 0, g_step, …, 180` (so C0/C90/C180/C270 exist), sampled with
    /// bilinear interpolation between the bin centres.
    fn to_eulumdat(&self, name: &str, manufacturer: &str) -> Eulumdat;
}

impl FarFieldExt for FarField {
    fn to_eulumdat(&self, name: &str, manufacturer: &str) -> Eulumdat {
        let per_klm = if self.total_flux > 0.0 {
            self.total_flux / 1000.0
        } else {
            1.0
        };
        let c_angles: Vec<f64> = (0..self.c_count())
            .map(|i| i as f64 * self.c_step())
            .collect();
        let g_angles: Vec<f64> = (0..=self.g_count())
            .map(|i| i as f64 * self.g_step())
            .collect();
        let intensities: Vec<Vec<f64>> = c_angles
            .iter()
            .map(|&c| {
                g_angles
                    .iter()
                    .map(|&g| self.sample(c, g) / per_klm)
                    .collect()
            })
            .collect();
        let unit_note = match self.unit {
            IntensityUnit::Candela => "cd/klm from TM-25 ray file",
            IntensityUnit::WattPerSteradian => "RADIOMETRIC: values are W/sr per kW, not cd/klm",
        };
        let mut ldt = Eulumdat::new();
        ldt.identification = manufacturer.to_string();
        ldt.luminaire_name = name.to_string();
        ldt.measurement_report_number = unit_note.to_string();
        ldt.type_indicator = TypeIndicator::PointSourceOther;
        ldt.symmetry = Symmetry::None;
        ldt.num_c_planes = self.c_count();
        ldt.c_plane_distance = self.c_step();
        ldt.num_g_planes = self.g_count() + 1;
        ldt.g_plane_distance = self.g_step();
        ldt.light_output_ratio = 100.0;
        ldt.downward_flux_fraction = self.forward_fraction() * 100.0;
        ldt.conversion_factor = 1.0;
        ldt.lamp_sets = vec![LampSet {
            num_lamps: 1,
            lamp_type: "TM-25 ray set".to_string(),
            total_luminous_flux: self.total_flux,
            ..Default::default()
        }];
        ldt.c_angles = c_angles;
        ldt.g_angles = g_angles;
        ldt.intensities = intensities;
        ldt
    }
}

/// Spectral table → ATLA `SpectralDistribution` (relative units, peak = 1).
pub fn spectral_table_to_atla(t: &SpectralTable) -> SpectralDistribution {
    let peak = t.values.iter().cloned().fold(0.0f32, f32::max);
    let scale = if peak > 0.0 { 1.0 / peak as f64 } else { 1.0 };
    SpectralDistribution {
        wavelengths: t.wavelengths_nm.iter().map(|w| *w as f64).collect(),
        values: t.values.iter().map(|v| *v as f64 * scale).collect(),
        units: SpectralUnits::Relative,
        start_wavelength: t.wavelengths_nm.first().map(|w| *w as f64),
        wavelength_interval: if t.wavelengths_nm.len() >= 2 {
            Some((t.wavelengths_nm[1] - t.wavelengths_nm[0]) as f64)
        } else {
            None
        },
    }
}

/// Start-point bounds of a ray set in TM-25 millimetres: `(min, max)`.
pub type Bounds = ([f32; 3], [f32; 3]);

/// Lamp lumens of a ray file: the header lumens when the file is photometric,
/// otherwise the radiant watts times the luminous efficacy of the header
/// spectrum (0 when neither is available).
pub fn lamp_lumens(h: &Header) -> f64 {
    match h.flux_kind() {
        FluxKind::Photometric | FluxKind::Both if h.luminous_flux_lm > 0.0 => {
            h.luminous_flux_lm as f64
        }
        _ => h
            .shared_spectrum()
            .map(|t| spectral_metrics(t).ler_lm_per_w * h.radiant_flux_w as f64)
            .unwrap_or(0.0),
    }
}

/// `eulumdat_spectrum` metrics (LER, PPFD/W, …) of a TM-25 spectral table.
pub fn spectral_metrics(t: &SpectralTable) -> eulumdat_spectrum::SpectralMetrics {
    let spd = eulumdat_spectrum::Spd::new(
        &t.wavelengths_nm
            .iter()
            .map(|w| *w as f64)
            .collect::<Vec<_>>(),
        &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
    );
    eulumdat_spectrum::SpectralMetrics::from_spd(&spd)
}

/// EULUMDAT document of a binned far field, filled in from the file header:
/// name and manufacturer, measurement date, the emitting area from the ray
/// start-point `bounds` (a bare LED's luminous dimensions), and the lamp flux
/// in lumens ([`lamp_lumens`]). For a radiometric file the intensities are
/// per kW of the file; with lamp lumens = LER × W that is cd/klm exactly, so
/// no rescale is needed.
pub fn luminaire_from_far_field(ff: &FarField, h: &Header, bounds: Option<Bounds>) -> Eulumdat {
    let mut ldt = ff.to_eulumdat(h.text.name(), h.text.manufacturer());
    if let Some((lo, hi)) = bounds {
        let dim = |i: usize| ((hi[i] - lo[i]) as f64).max(0.1);
        ldt.length = dim(0);
        ldt.width = dim(1);
        ldt.height = dim(2);
        ldt.luminous_area_length = dim(0);
        ldt.luminous_area_width = dim(1);
    }
    ldt.date_user = h.date_time.clone();
    // TM-25 has no part-number field: field 3 is the creator of the optical
    // model (a person or company), so the catalogue number stays empty.
    let lumens = lamp_lumens(h);
    if lumens > 0.0 {
        if let Some(ls) = ldt.lamp_sets.first_mut() {
            ls.total_luminous_flux = lumens;
        }
        // Say where the lumens came from: the file's own photometric total, or
        // the radiant watts times the efficacy of the header spectrum.
        let source = match h.flux_kind() {
            FluxKind::Photometric | FluxKind::Both if h.luminous_flux_lm > 0.0 => {
                "file header".to_string()
            }
            _ => format!("{} W radiant × LER", h.radiant_flux_w),
        };
        ldt.measurement_report_number =
            format!("from TM-25 ray file; lamp flux {lumens:.2} lm ({source})");
    }
    ldt
}

/// ATLA document of `ldt` with the header spectrum attached to the emitter,
/// the report date taken from the file, the ray-file creator as laboratory
/// and the model creator, equipment and operating condition as comments.
pub fn atla_from_luminaire(ldt: &Eulumdat, h: &Header) -> LuminaireOpticalData {
    let mut doc = LuminaireOpticalData::from_eulumdat(ldt);
    if let Some(t) = h.shared_spectrum() {
        let spd = spectral_table_to_atla(t);
        eulumdat::atla::spd_loader::attach_spd(&mut doc, &spd, None);
    }
    if h.date_time.len() >= 10 {
        doc.header.report_date = Some(h.date_time[..10].to_string());
    }
    let text = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
    if let Some(lab) = text(h.text.rayfile_creator()) {
        doc.header.laboratory = Some(lab);
    }
    let comments: Vec<String> = [
        ("Optical model by", h.text.model_creator()),
        ("Equipment", h.text.equipment()),
        ("Operating condition", h.text.operating_condition()),
    ]
    .into_iter()
    .filter_map(|(label, v)| text(v).map(|v| format!("{label}: {v}")))
    .collect();
    if !comments.is_empty() {
        doc.header.comments = Some(comments.join("; "));
    }
    doc
}

/// Spectral table as a relative `.spd` text file (peak = 1, tab separated,
/// `#` comments), the format `eulumdat::atla::spd_loader` reads back.
pub fn spectral_table_to_spd_text(t: &SpectralTable, label: &str) -> String {
    let peak = t.values.iter().cloned().fold(0.0f32, f32::max);
    let scale = if peak > 0.0 { 1.0 / peak } else { 1.0 };
    let mut s = String::with_capacity(t.len() * 16 + 200);
    s.push_str(&format!("# {label}\n"));
    s.push_str("# Spectral table of an IES TM-25 ray file (tm25ray)\n");
    s.push_str("# Relative spectral power, peak normalised to 1.0\n");
    s.push_str("# Wavelength[nm]\tRelative_Power\n");
    for (w, v) in t.wavelengths_nm.iter().zip(&t.values) {
        s.push_str(&format!("{w}\t{:.6}\n", v * scale));
    }
    s
}

/// Every file the viewer and the CLI can hand out for a ray file.
#[derive(Debug, Clone, Default)]
pub struct Exports {
    /// EULUMDAT text.
    pub ldt: String,
    /// IESNA LM-63 text.
    pub ies: String,
    /// ATLA S001 XML with the header spectrum attached.
    pub atla_xml: String,
    /// Relative `.spd` of the header spectrum, when the file has one.
    pub spd: Option<String>,
    /// Lamp lumens used for the documents (0 when unknown).
    pub lamp_lumens: f64,
}

/// Check that a file of `total_size` bytes holds exactly the header's ray
/// block.
///
/// Streaming readers parse the header from a prefix and cannot see a file
/// that is a few bytes off; they then read every ray misaligned. Files
/// written by tm25ray 0.1.x are 4 to 20 bytes off (its spectral-block
/// layout differed from the standard), so check before streaming.
pub fn check_file_size(h: &Header, total_size: u64) -> core::result::Result<(), String> {
    let expected = h.ray_start as u64 + h.ray_block_size();
    if expected == total_size {
        Ok(())
    } else {
        Err(format!(
            "file size {total_size} B does not match the header ({expected} B: ray block at {} + {} rays × {} B); written by tm25ray 0.1.x, or truncated",
            h.ray_start,
            h.n_rays,
            h.record_size()
        ))
    }
}

/// Build all export texts at once. `bounds` gives the emitting area.
pub fn exports(ff: &FarField, h: &Header, bounds: Option<Bounds>) -> Exports {
    let ldt = luminaire_from_far_field(ff, h, bounds);
    let doc = atla_from_luminaire(&ldt, h);
    let label = if h.text.name().is_empty() {
        "TM-25 spectrum".to_string()
    } else {
        h.text.name().to_string()
    };
    Exports {
        ldt: ldt.to_ldt(),
        ies: IesExporter::export(&ldt),
        atla_xml: eulumdat::atla::xml::write(&doc).unwrap_or_default(),
        spd: h
            .shared_spectrum()
            .map(|t| spectral_table_to_spd_text(t, &label)),
        lamp_lumens: lamp_lumens(h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lambertian(n: usize) -> Vec<Ray> {
        let mut rng = Rng64::new(9);
        (0..n)
            .map(|_| {
                let u = rng.next_f64();
                let phi = 2.0 * std::f64::consts::PI * rng.next_f64();
                let (s, c) = (u.sqrt(), (1.0 - u).sqrt());
                Ray::new(
                    [0.0, 0.0, 0.0],
                    [(s * phi.cos()) as f32, (s * phi.sin()) as f32, c as f32],
                )
                .with_radiant_flux(1e-3)
            })
            .collect()
    }

    #[test]
    fn far_field_to_eulumdat_uses_conventional_grid() {
        let rays = lambertian(50_000);
        let ff = FarField::from_rays(&rays, 30.0, 10.0, FluxKind::Radiometric);
        let ldt = ff.to_eulumdat("Synthetic", "eulumdat-rs");
        assert_eq!(ldt.c_angles.len(), 12);
        assert_eq!(ldt.c_angles[3], 90.0);
        assert_eq!(ldt.g_angles.len(), 19);
        assert_eq!(ldt.g_angles[18], 180.0);
        assert!(ldt.intensities[0][0] > ldt.intensities[0][8]);
        assert!(ldt.measurement_report_number.contains("RADIOMETRIC"));
        assert!(ldt.to_ldt().contains("Synthetic"));
    }

    #[test]
    fn spectrum_to_atla_is_peak_normalised() {
        let t = SpectralTable {
            wavelengths_nm: (0..51).map(|i| 220.0 + 2.0 * i as f32).collect(),
            values: (0..51).map(|i| if i == 23 { 100.0 } else { 1.0 }).collect(),
        };
        let atla = spectral_table_to_atla(&t);
        assert_eq!(atla.values.iter().cloned().fold(0.0, f64::max), 1.0);
        assert_eq!(atla.wavelength_interval, Some(2.0));
        assert_eq!(atla.units, SpectralUnits::Relative);
    }

    fn uv_header() -> Header {
        let mut h = Header::new(KnownDataFlags::radiometric());
        h.radiant_flux_w = 0.05;
        h.date_time = "2026-05-12 13:57:44".into();
        h.spectral_id = SpectralId::SharedTable;
        h.spectra.push(SpectralTable {
            wavelengths_nm: (0..51).map(|i| 220.0 + 2.0 * i as f32).collect(),
            values: (0..51).map(|i| if i == 23 { 100.0 } else { 1.0 }).collect(),
        });
        h
    }

    #[test]
    fn exports_cover_every_format() {
        let rays = lambertian(20_000);
        let ff = FarField::from_rays(&rays, 30.0, 10.0, FluxKind::Radiometric);
        let h = uv_header();
        let e = exports(&ff, &h, Some(([-1.0, -1.0, 0.0], [1.0, 1.0, 0.5])));
        // UV spectrum: no lumens, so the radiometric note stays.
        assert_eq!(e.lamp_lumens, 0.0);
        assert!(e.ldt.contains("RADIOMETRIC"));
        assert!(e.ies.contains("LM-63"));
        assert!(e.atla_xml.contains("SpectralDistribution"));
        let spd = e.spd.expect("shared spectrum");
        assert!(spd.contains("266\t1.000000"));
        assert!(eulumdat::atla::spd_loader::parse(&spd).is_ok());
        let ldt = Eulumdat::parse(&e.ldt).unwrap();
        assert_eq!(ldt.date_user, "2026-05-12 13:57:44");
        assert!((ldt.length - 2.0).abs() < 1e-9);
    }

    #[test]
    fn text_fields_map_to_their_meaning() {
        let rays = lambertian(5_000);
        let ff = FarField::from_rays(&rays, 30.0, 10.0, FluxKind::Radiometric);
        let mut h = uv_header();
        h.text.raw[0] = "GW CSSRMU".into();
        h.text.raw[1] = "ams OSRAM".into();
        h.text.raw[2] = "BZ".into(); // creator of the optical model (initials)
        h.text.raw[3] = "Test Lab".into(); // creator of the ray file
        h.text.raw[6] = "350 mA, 25 °C".into();
        let ldt = luminaire_from_far_field(&ff, &h, None);
        assert_eq!(ldt.luminaire_name, "GW CSSRMU");
        assert_eq!(ldt.identification, "ams OSRAM");
        assert!(ldt.luminaire_number.is_empty(), "no part number in TM-25");
        let doc = atla_from_luminaire(&ldt, &h);
        assert_eq!(doc.header.laboratory.as_deref(), Some("Test Lab"));
        let comments = doc.header.comments.unwrap();
        assert!(comments.contains("Optical model by: BZ"), "{comments}");
        assert!(comments.contains("Operating condition: 350 mA, 25 °C"));
    }

    #[test]
    fn file_size_check_catches_a_shifted_file() {
        let rays = lambertian(100);
        let mut h = Header::new(KnownDataFlags::radiometric());
        h.radiant_flux_w = rays.iter().map(|r| r.flux(FluxKind::Radiometric)).sum();
        let mut buf = Vec::new();
        write_tm25(&mut buf, &h, &rays).unwrap();
        let f = Tm25File::parse(&buf).unwrap();
        assert!(check_file_size(&f.header, buf.len() as u64).is_ok());
        // A 0.1.x file: 4 extra bytes before the ray block
        let err = check_file_size(&f.header, buf.len() as u64 + 4).unwrap_err();
        assert!(err.contains("0.1.x"), "{err}");
    }

    #[test]
    fn photometric_header_sets_lamp_lumens() {
        let mut h = Header::new(KnownDataFlags::photometric());
        h.luminous_flux_lm = 123.5;
        assert_eq!(lamp_lumens(&h), 123.5);
        let rays: Vec<Ray> = lambertian(5_000)
            .into_iter()
            .map(|r| r.with_luminous_flux(1e-3))
            .collect();
        let ff = FarField::from_rays(&rays, 30.0, 10.0, FluxKind::Photometric);
        let ldt = luminaire_from_far_field(&ff, &h, None);
        assert_eq!(ldt.lamp_sets[0].total_luminous_flux, 123.5);
        assert!(ldt.measurement_report_number.contains("123.50 lm"));
        assert!(ldt.measurement_report_number.contains("file header"));
    }
}
