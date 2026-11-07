//! IES TM-33-23 output (XML and JSON) and the TM-33-23 JSON reader.
//!
//! The document is built once as an element tree in the order of the
//! TM-33-23 schema; that tree is written as XML ([`write_xml`]) or, by the
//! "XML to JSON mapping" of TM-33-23 §4.8, as JSON ([`write_json`]).
//! [`parse_json`] maps a TM-33-23 JSON document back to XML and reads it
//! with the XML reader, so both forms carry the same data.
//!
//! The data model keeps the EULUMDAT conventions; on the way out they are
//! converted to TM-33-23 units: dimensions in metres (model: mm),
//! intensities in candela (model: cd/klm of the emitter's rated lumens),
//! relative spectra normalised to a peak of 1.0 (`Normalized`), absolute
//! spectra in W/nm. The XML reader converts back.
//!
//! TM-32-24 BIM parameters travel as `CustomData` blocks of the TM-33-23
//! document ([`contents`] reports whether a document has them).

use crate::atla::error::Result;
use crate::atla::types::*;

/// Placeholder for required header fields the source does not give.
pub const NOT_SPECIFIED: &str = "Not specified";

/// What a TM-33-23 file written from a document carries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tm33Contents {
    /// An intensity distribution.
    pub photometry: bool,
    /// An emitter spectrum.
    pub spectrum: bool,
    /// Colour values (CCT, CRI, TM-30).
    pub colour: bool,
    /// TM-32-24 BIM parameters (`CustomData`).
    pub bim: bool,
    /// Other `CustomData` blocks.
    pub custom: bool,
}

/// What [`write_xml`] / [`write_json`] would carry for `doc`.
pub fn contents(doc: &LuminaireOpticalData) -> Tm33Contents {
    let bim = doc.custom_data_items.iter().any(is_bim_item);
    Tm33Contents {
        photometry: doc.emitters.iter().any(|e| {
            e.intensity_distribution
                .as_ref()
                .is_some_and(|d| !d.intensities.is_empty())
        }),
        spectrum: doc.emitters.iter().any(|e| {
            e.spectral_distribution
                .as_ref()
                .is_some_and(|s| !s.values.is_empty())
        }),
        colour: doc
            .emitters
            .iter()
            .any(|e| e.cct.is_some() || e.color_rendering.is_some()),
        bim,
        custom: doc.custom_data_items.iter().any(|i| !is_bim_item(i))
            || doc
                .custom_data
                .as_ref()
                .is_some_and(|c| !c.data.trim().is_empty()),
    }
}

/// A `CustomData` block with TM-32-24 BIM parameters.
pub fn is_bim_item(item: &CustomDataItem) -> bool {
    let name = item.name.to_ascii_lowercase().replace(['-', '_', ' '], "");
    name.starts_with("tm32")
        || item.raw_content.contains("ies.org/tm-32")
        || item.raw_content.contains("<BIM")
}

/// Options for writing.
#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    /// Today's date (`YYYY-MM-DD`). TM-33-23 requires a report date: when
    /// the source has none, or one that is not a date, this date is used
    /// and a header comment says so. Also the `DocumentCreationDate`.
    pub today: Option<String>,
    /// `DocumentCreator` when the source has none.
    pub creator: Option<String>,
}

/// TM-33-23 XML (root `IESTM33-22`, version 1.1).
pub fn write_xml(doc: &LuminaireOpticalData, opts: &WriteOptions) -> Result<String> {
    Ok(xml_document(&document(doc, opts), Some(2)))
}

/// TM-33-23 XML without line breaks.
pub fn write_xml_compact(doc: &LuminaireOpticalData, opts: &WriteOptions) -> Result<String> {
    Ok(xml_document(&document(doc, opts), None))
}

// ─── element tree ────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub(crate) struct El {
    name: String,
    attrs: Vec<(String, String)>,
    text: Option<String>,
    children: Vec<El>,
    /// XML kept as it is (the content of a `CustomData` block).
    raw: Option<String>,
}

impl El {
    fn new(name: &str) -> Self {
        El {
            name: name.to_string(),
            ..Default::default()
        }
    }
    fn with_text(name: &str, text: impl Into<String>) -> Self {
        El {
            name: name.to_string(),
            text: Some(text.into()),
            ..Default::default()
        }
    }
    fn add(&mut self, c: El) {
        self.children.push(c);
    }
    fn leaf(&mut self, name: &str, text: impl Into<String>) {
        self.children.push(El::with_text(name, text));
    }
    fn opt_str(&mut self, name: &str, v: &Option<String>) {
        if let Some(v) = nonempty(v) {
            self.leaf(name, v);
        }
    }
    fn opt_num(&mut self, name: &str, v: Option<f64>) {
        if let Some(v) = v.filter(|v| v.is_finite()) {
            self.leaf(name, sig(v, 9));
        }
    }
    fn opt_int(&mut self, name: &str, v: Option<f64>) {
        if let Some(v) = v.filter(|v| v.is_finite()) {
            self.leaf(name, (v.round() as i64).to_string());
        }
    }
    fn child(&self, name: &str) -> Option<&El> {
        self.children.iter().find(|c| c.name == name)
    }
}

fn nonempty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// `v` with at most `digits` significant digits, no exponent, no trailing
/// zeros (`397.85`, `0.0012`, `42000`).
pub(crate) fn sig(v: f64, digits: i32) -> String {
    if !v.is_finite() || v == 0.0 {
        return "0".to_string();
    }
    let mag = v.abs().log10().floor() as i32;
    let decimals = digits - 1 - mag;
    let s = if decimals > 0 {
        format!("{:.*}", decimals as usize, v)
    } else {
        let p = 10f64.powi(-decimals);
        format!("{:.0}", (v / p).round() * p)
    };
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    if s == "-0" {
        "0".to_string()
    } else {
        s
    }
}

/// `v` rounded to `digits` significant digits.
pub(crate) fn round_sig(v: f64, digits: i32) -> f64 {
    sig(v, digits).parse().unwrap_or(v)
}

fn bool_text(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

/// `YYYY-MM-DD` from the date forms found in luminaire files
/// (`2024-06-15`, `2024-06-15T10:00`, `15.06.2024`, `15/06/2024`).
pub fn iso_date(s: &str) -> Option<String> {
    let s = s.trim();
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| {
        b.get(r.clone())
            .is_some_and(|x| x.iter().all(u8::is_ascii_digit))
    };
    if b.len() >= 10
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
    {
        let (m, d): (u32, u32) = (s[5..7].parse().ok()?, s[8..10].parse().ok()?);
        return ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| s[..10].to_string());
    }
    let parts: Vec<&str> = s.split(['.', '/', '-']).map(str::trim).collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        let (a, b2, y): (u32, u32, u32) = (
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        );
        if parts[2].len() != 4 {
            return None;
        }
        // day first (European) unless that cannot be a month / day
        let (d, m) = if b2 > 12 && a <= 12 { (b2, a) } else { (a, b2) };
        if (1..=12).contains(&m) && (1..=31).contains(&d) {
            return Some(format!("{y:04}-{m:02}-{d:02}"));
        }
    }
    None
}

/// The document as a TM-33-23 element tree.
pub(crate) fn document(doc: &LuminaireOpticalData, opts: &WriteOptions) -> El {
    let mut root = El::new("IESTM33-22");
    root.leaf("Version", "1.1");
    root.add(header(&doc.header, opts));
    if let Some(l) = luminaire(doc) {
        root.add(l);
    }
    if let Some(e) = doc.equipment.as_ref().and_then(equipment) {
        root.add(e);
    }
    let opening = doc
        .luminaire
        .as_ref()
        .and_then(|l| l.luminous_openings.first());
    for (i, e) in doc.emitters.iter().enumerate() {
        root.add(emitter(e, i, if i == 0 { opening } else { None }));
    }
    for item in &doc.custom_data_items {
        root.add(custom_item(item));
    }
    if let Some(cd) = doc
        .custom_data
        .as_ref()
        .filter(|c| !c.data.trim().is_empty())
    {
        let mut e = El::new("CustomData");
        let ns = cd.namespace.clone().unwrap_or_default();
        e.leaf(
            "Name",
            if ns.is_empty() {
                "ATLA S001 custom data".to_string()
            } else {
                ns.clone()
            },
        );
        e.leaf(
            "UniqueIdentifier",
            if ns.is_empty() {
                "atla-s001-custom-data".to_string()
            } else {
                ns
            },
        );
        let data = cd.data.trim();
        if data.starts_with('<') {
            e.raw = Some(data.to_string());
        } else {
            e.leaf("Data", data);
        }
        root.add(e);
    }
    root
}

fn header(h: &Header, opts: &WriteOptions) -> El {
    let mut e = El::new("Header");
    let mut notes: Vec<String> = Vec::new();
    e.opt_str("Manufacturer", &h.manufacturer);
    e.opt_str("CatalogNumber", &h.catalog_number);
    let gtin = h
        .gtin_int
        .map(|g| g.to_string())
        .or_else(|| nonempty(&h.gtin).map(str::to_string));
    match gtin {
        Some(g) if g.chars().all(|c| c.is_ascii_digit()) => e.leaf("GTIN", g),
        Some(g) => notes.push(format!("GTIN: {g}")),
        None => {}
    }
    let description = nonempty(&h.description)
        .or(nonempty(&h.catalog_number))
        .or(nonempty(&h.manufacturer))
        .unwrap_or("Luminaire");
    e.leaf("Description", description);
    e.leaf(
        "Laboratory",
        nonempty(&h.laboratory).unwrap_or(NOT_SPECIFIED),
    );
    e.leaf(
        "ReportNumber",
        nonempty(&h.report_number).unwrap_or(NOT_SPECIFIED),
    );
    let source_date = nonempty(&h.report_date)
        .or(nonempty(&h.test_date))
        .or(nonempty(&h.issue_date));
    match source_date.and_then(iso_date) {
        Some(d) => e.leaf("ReportDate", d),
        None => {
            if let Some(today) = opts.today.as_deref() {
                e.leaf("ReportDate", today);
                notes.push(match source_date {
                    Some(orig) => format!("Report date in the source file: {orig}"),
                    None => "The source file gives no report date; ReportDate is the export date."
                        .to_string(),
                });
            }
        }
    }
    let creator = nonempty(&h.document_creator)
        .map(str::to_string)
        .or_else(|| opts.creator.clone());
    e.opt_str("DocumentCreator", &creator);
    let created = nonempty(&h.document_creation_date)
        .and_then(iso_date)
        .or_else(|| opts.today.clone());
    e.opt_str("DocumentCreationDate", &created);
    let uid = nonempty(&h.unique_identifier)
        .or(nonempty(&h.uuid))
        .map(str::to_string);
    e.opt_str("UniqueIdentifier", &uid);
    let mut comments: Vec<String> = h
        .comments_list
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    if comments.is_empty() {
        if let Some(c) = nonempty(&h.comments) {
            comments.push(c.to_string());
        }
    }
    comments.extend(notes);
    for c in comments {
        e.leaf("Comment", c);
    }
    let mut refs: Vec<String> = h.references.clone();
    if let Some(r) = nonempty(&h.reference) {
        if !refs.iter().any(|x| x == r) {
            refs.push(r.to_string());
        }
    }
    for r in refs.into_iter().filter(|r| !r.trim().is_empty()) {
        e.leaf("Reference", r);
    }
    e.opt_str("MoreInfoURI", &h.more_info_uri);
    e
}

fn metres(mm: f64) -> String {
    sig(mm / 1000.0, 9)
}

fn luminaire(doc: &LuminaireOpticalData) -> Option<El> {
    let d = doc.luminaire.as_ref()?.dimensions.as_ref()?;
    let mut e = El::new("Luminaire");
    let mut dims = El::new("Dimensions");
    // EULUMDAT: width 0 = round, the length is the diameter
    let round = d.width <= 0.0 && d.length > 0.0;
    dims.leaf("Length", metres(d.length));
    dims.leaf("Width", metres(if round { d.length } else { d.width }));
    dims.leaf("Height", metres(d.height));
    e.add(dims);
    if round {
        // circular in the x-y plane
        e.leaf("Shape", "Align_Z");
    }
    e.leaf("NumEmitter", doc.emitters.len().to_string());
    Some(e)
}

fn equipment(eq: &Equipment) -> Option<El> {
    let mut e = El::new("Equipment");
    let name = |m: &Option<String>, model: &Option<String>| -> Option<String> {
        nonempty(model).or(nonempty(m)).map(str::to_string)
    };
    if let Some(g) = &eq.goniometer {
        let mut gon = El::new("Gonioradiometer");
        let t = g
            .goniometer_type
            .as_deref()
            .map(GoniometerTypeEnum::parse)
            .unwrap_or(GoniometerTypeEnum::CieC);
        gon.leaf("Type", t.to_tm33_str());
        gon.opt_str("MeasurementEquipment", &name(&g.manufacturer, &g.model));
        e.add(gon);
    }
    if let Some(s) = &eq.integrating_sphere {
        let mut sp = El::new("IntegratingSphere");
        sp.opt_str("MeasurementEquipment", &name(&s.manufacturer, &s.model));
        e.add(sp);
    }
    if let Some(s) = &eq.spectroradiometer {
        let mut sp = El::new("Spectroradiometer");
        sp.leaf(
            "MeasurementEquipment",
            name(&s.manufacturer, &s.model).unwrap_or_else(|| NOT_SPECIFIED.to_string()),
        );
        sp.opt_num("BandwidthFWHM", s.resolution);
        e.add(sp);
    }
    (!e.children.is_empty()).then_some(e)
}

/// TM-33-23 symmetry from the horizontal angles of the measured planes.
fn infer_symmetry(h: &[f64]) -> SymmetryType {
    let (lo, hi) = h
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
    if h.len() <= 1 {
        SymmetryType::Full
    } else if lo.abs() < 1e-6 && (hi - 90.0).abs() < 1e-6 {
        SymmetryType::Quad
    } else if lo.abs() < 1e-6 && (hi - 180.0).abs() < 1e-6 {
        SymmetryType::Bi0
    } else if (lo - 90.0).abs() < 1e-6 && (hi - 270.0).abs() < 1e-6 {
        SymmetryType::Bi90
    } else {
        SymmetryType::None
    }
}

/// Luminous flux of a distribution (lm per unit of the values): the mean
/// over the measured planes of each zone (planes weighted by the angle they
/// stand for; symmetric data covers its sector), times the zone's solid
/// angle.
pub(crate) fn distribution_flux(d: &IntensityDistribution) -> f64 {
    let h = &d.horizontal_angles;
    let v = &d.vertical_angles;
    if v.len() < 2 || d.intensities.is_empty() {
        return 0.0;
    }
    let weights: Vec<f64> = if h.len() <= 1 {
        vec![1.0]
    } else {
        let n = h.len();
        (0..n)
            .map(|j| {
                let a = if j == 0 { h[0] } else { h[j - 1] };
                let b = if j + 1 == n { h[n - 1] } else { h[j + 1] };
                ((b - a) / 2.0).abs()
            })
            .collect()
    };
    let total_w: f64 = weights.iter().sum::<f64>().max(1e-12);
    let mean_at = |k: usize| -> f64 {
        d.intensities
            .iter()
            .zip(&weights)
            .map(|(row, w)| row.get(k).copied().unwrap_or(0.0) * w)
            .sum::<f64>()
            / total_w
    };
    let mut flux = 0.0;
    for k in 0..v.len() - 1 {
        let (a, b) = (v[k].to_radians(), v[k + 1].to_radians());
        flux +=
            2.0 * std::f64::consts::PI * 0.5 * (mean_at(k) + mean_at(k + 1)) * (a.cos() - b.cos());
    }
    flux.abs()
}

fn emitter(em: &Emitter, idx: usize, opening: Option<&LuminousOpening>) -> El {
    let mut e = El::new("Emitter");
    e.leaf(
        "Quantity",
        em.quantity.clamp(1, i32::MAX as u32).to_string(),
    );
    let fallback = format!("Emitter {}", idx + 1);
    e.leaf(
        "Description",
        nonempty(&em.description).unwrap_or(&fallback),
    );
    e.opt_str("CatalogNumber", &em.catalog_number);
    let rated = em.rated_lumens.filter(|f| *f > 0.0);
    e.opt_num("RatedLumens", rated);
    e.leaf("InputWattage", sig(em.input_watts.unwrap_or(0.0), 9));
    e.opt_num("PowerFactor", em.power_factor);
    e.opt_num("BallastFactor", em.ballast_factor);
    if let Some(cct) = em.cct.filter(|c| *c > 0.0) {
        let mut c = El::new("ColorTemperature");
        c.opt_int("FixedCCT", Some(cct));
        e.add(c);
    }
    if let Some(cr) = &em.color_rendering {
        let mut c = El::new("ColorRendering");
        if let Some(ra) = cr.ra {
            let mut cie = El::new("CIE_CRI");
            cie.opt_int("Ra", Some(ra));
            cie.opt_int("R9", cr.r9);
            c.add(cie);
        }
        if let (Some(rf), Some(rg)) = (cr.rf, cr.rg) {
            let mut tm30 = El::new("IES_TM30");
            tm30.opt_int("Rf", Some(rf));
            tm30.opt_int("Rg", Some(rg));
            c.add(tm30);
        }
        if !c.children.is_empty() {
            e.add(c);
        }
    }
    e.opt_num("Duv", em.duv);
    e.opt_num("SPRatio", em.sp_ratio);
    if let Some(g) = &em.data_generation {
        let mut dg = El::new("DataGeneration");
        dg.leaf("Simulation", bool_text(g.source == DataSource::Simulated));
        dg.leaf("IntensityScaling", bool_text(g.scaled));
        dg.leaf("AngleInterpolation", bool_text(g.interpolated));
        e.add(dg);
    }
    if let Some(d) = em
        .intensity_distribution
        .as_ref()
        .filter(|d| !d.intensities.is_empty())
    {
        e.add(luminous_data(em, d));
    }
    if let Some(s) = &em.spectral_distribution {
        if let Some(sd) = spectral_data(s) {
            e.add(sd);
        }
    }
    if let Some(o) = opening {
        if let Some(a) = emission_areas(o) {
            e.add(a);
        }
    }
    e
}

/// The factor from the model's intensity values to candela.
fn candela_scale(em: &Emitter, d: &IntensityDistribution) -> f64 {
    let mult = d.multiplier.map(f64::from).unwrap_or(1.0);
    match d.units {
        IntensityUnits::CandelaPerKilolumen => {
            let flux = em
                .rated_lumens
                .filter(|f| *f > 0.0)
                .or(em.measured_lumens.filter(|f| *f > 0.0));
            mult * flux.map(|f| f / 1000.0).unwrap_or(1.0)
        }
        _ => mult,
    }
}

fn luminous_data(em: &Emitter, d: &IntensityDistribution) -> El {
    let scale = candela_scale(em, d);
    let mut li = El::new("LuminousIntensity");
    li.leaf(
        "AbsolutePhotometry",
        bool_text(d.absolute_photometry.unwrap_or(false)),
    );
    li.leaf(
        "Symm",
        d.symmetry
            .unwrap_or_else(|| infer_symmetry(&d.horizontal_angles))
            .to_tm33_str(),
    );
    let mut points = Vec::new();
    for (hi, h) in d.horizontal_angles.iter().enumerate() {
        let Some(row) = d.intensities.get(hi) else {
            continue;
        };
        for (vi, v) in d.vertical_angles.iter().enumerate() {
            if let Some(x) = row.get(vi) {
                let mut p = El::with_text("IntData", sig(x * scale, 6));
                p.attrs.push(("h".into(), sig(*h, 9)));
                p.attrs.push(("v".into(), sig(*v, 9)));
                points.push(p);
            }
        }
    }
    li.leaf("NumberMeasured", points.len().to_string());
    li.leaf("NumberHorz", d.horizontal_angles.len().to_string());
    li.leaf("NumberVert", d.vertical_angles.len().to_string());
    li.children.extend(points);
    let mut ld = El::new("LuminousData");
    ld.add(li);
    let flux = distribution_flux(d) * scale;
    if flux > 0.0 {
        ld.leaf("LuminousFlux", sig(flux, 6));
    }
    ld
}

/// Wavelengths and values of a spectrum (an evenly spaced one may carry
/// only its start and step).
pub(crate) fn spectrum_points(s: &SpectralDistribution) -> (Vec<f64>, Vec<f64>) {
    let w = if s.wavelengths.len() == s.values.len() {
        s.wavelengths.clone()
    } else if let (Some(start), Some(step)) = (s.start_wavelength, s.wavelength_interval) {
        (0..s.values.len())
            .map(|i| start + step * i as f64)
            .collect()
    } else {
        return (Vec::new(), Vec::new());
    };
    (w, s.values.clone())
}

fn spectral_data(s: &SpectralDistribution) -> Option<El> {
    let (w, v) = spectrum_points(s);
    if w.is_empty() {
        return None;
    }
    let relative = s.units == SpectralUnits::Relative;
    let peak = v.iter().cloned().fold(0.0f64, f64::max);
    let mut es = El::new("EmitterSpectral");
    es.leaf("NumberWavelength", w.len().to_string());
    if relative {
        es.leaf("Normalized", "true");
    }
    for (wl, x) in w.iter().zip(&v) {
        let val = if relative && peak > 0.0 { x / peak } else { *x };
        let mut p = El::with_text("PwrData", sig(val, 6));
        p.attrs.push(("w".into(), sig(*wl, 9)));
        es.add(p);
    }
    let mut sd = El::new("SpectralData");
    sd.add(es);
    Some(sd)
}

fn emission_areas(o: &LuminousOpening) -> Option<El> {
    let circular = o.shape == LuminousOpeningShape::Circular;
    let len = o.dimensions.length;
    let width = o
        .dimensions
        .width
        .filter(|w| *w > 0.0)
        .unwrap_or(if circular { len } else { 0.0 });
    if len <= 0.0 || width <= 0.0 {
        return None;
    }
    let mut area = El::new("BottomArea");
    area.leaf("Length", metres(len));
    area.leaf("Width", metres(width));
    let (dx, dy) = o
        .position
        .as_ref()
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    area.leaf("LengthOffset", metres(dx));
    area.leaf("WidthOffset", metres(dy));
    if circular {
        area.leaf("Circular", "true");
    }
    let mut face = El::new("BottomFace");
    face.leaf("NumberBottom", "1");
    face.add(area);
    let mut areas = El::new("EmissionAreas");
    areas.add(face);
    Some(areas)
}

fn custom_item(item: &CustomDataItem) -> El {
    let mut e = El::new("CustomData");
    e.leaf(
        "Name",
        if item.name.trim().is_empty() {
            "Custom data"
        } else {
            item.name.trim()
        },
    );
    e.leaf(
        "UniqueIdentifier",
        if item.unique_identifier.trim().is_empty() {
            "custom-data"
        } else {
            item.unique_identifier.trim()
        },
    );
    let raw = item.raw_content.trim();
    if !raw.is_empty() {
        e.raw = Some(raw.to_string());
    }
    e
}

// ─── XML ─────────────────────────────────────────────────────────────

pub(crate) fn esc(s: &str, attr: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn xml_document(root: &El, indent: Option<usize>) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    if indent.is_some() {
        out.push('\n');
    }
    xml_el(&mut out, root, 0, indent);
    out
}

/// Re-indent a block of XML lines to `pad`, keeping their relative indent.
fn reindent(raw: &str, pad: &str) -> String {
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    let common = lines
        .iter()
        .skip(1)
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = String::new();
    for (k, l) in lines.iter().enumerate() {
        let body = if k == 0 {
            l.trim_start()
        } else {
            &l[common.min(l.len() - l.trim_start().len())..]
        };
        out.push_str(pad);
        out.push_str(body.trim_end());
        out.push('\n');
    }
    out
}

fn xml_el(out: &mut String, e: &El, depth: usize, indent: Option<usize>) {
    let pad = indent.map(|n| " ".repeat(n * depth)).unwrap_or_default();
    let nl = if indent.is_some() { "\n" } else { "" };
    out.push_str(&pad);
    out.push('<');
    out.push_str(&e.name);
    for (k, v) in &e.attrs {
        out.push_str(&format!(" {k}=\"{}\"", esc(v, true)));
    }
    if e.children.is_empty() && e.raw.is_none() {
        match &e.text {
            Some(t) => {
                out.push('>');
                out.push_str(&esc(t, false));
                out.push_str(&format!("</{}>", e.name));
            }
            None => out.push_str("/>"),
        }
        out.push_str(nl);
        return;
    }
    out.push('>');
    if let Some(t) = e.text.as_deref().filter(|t| !t.trim().is_empty()) {
        out.push_str(&esc(t, false));
    }
    out.push_str(nl);
    for c in &e.children {
        xml_el(out, c, depth + 1, indent);
    }
    if let Some(raw) = &e.raw {
        match indent {
            Some(n) => out.push_str(&reindent(raw, &" ".repeat(n * (depth + 1)))),
            None => out.push_str(raw.trim()),
        }
    }
    out.push_str(&pad);
    out.push_str(&format!("</{}>", e.name));
    out.push_str(nl);
}

// ─── JSON (TM-33-23 §4.8) ────────────────────────────────────────────

#[cfg(feature = "json")]
pub use json_io::{is_tm33_json, parse_json, write_json};

#[cfg(feature = "json")]
mod json_io {
    use super::*;
    use crate::atla::error::AtlaError;
    use serde_json::Value;

    /// Elements that are always JSON arrays (§4.8.1.1, Table 61).
    const ARRAYS: &[&str] = &[
        "Comment",
        "Reference",
        "MeasurementEquipment",
        "Emitter",
        "MeasUncertainty",
        "EmitterSpectral",
        "PlaneData",
        "TopArea",
        "BottomArea",
        "C0Area",
        "C90Area",
        "C180Area",
        "C270Area",
        "CustomData",
    ];

    /// Elements of type string (kept as JSON strings even when they look
    /// like numbers, e.g. a catalogue number).
    const STRINGS: &[&str] = &[
        "Manufacturer",
        "CatalogNumber",
        "GTIN",
        "Description",
        "Laboratory",
        "ReportNumber",
        "ReportDate",
        "DocumentCreator",
        "DocumentCreationDate",
        "UniqueIdentifier",
        "Comment",
        "Reference",
        "MoreInfoURI",
        "Shape",
        "Type",
        "MeasurementEquipment",
        "BandwidthMethod",
        "Certification",
        "ApprovalBody",
        "ApprovalScope",
        "MeasurementType",
        "Symm",
        "EmitterName",
        "Name",
    ];

    /// Containers of `IntData h= v=` points.
    const INTENSITY: &[&str] = &["LuminousIntensity", "RadiantIntensity", "PhotonIntensity"];

    /// An ordered JSON value (serde_json's map is sorted without the
    /// `preserve_order` feature; the element order is part of the format).
    enum J {
        Obj(Vec<(String, J)>),
        Arr(Vec<J>),
        Str(String),
        Num(String),
        Bool(bool),
    }

    /// The text of a JSON number that reads back to the same text, so the
    /// XML ↔ JSON round trip keeps every value as written.
    fn canonical_number(t: &str) -> Option<String> {
        if let Ok(i) = t.parse::<i64>() {
            return (i.to_string() == t).then(|| t.to_string());
        }
        let v: f64 = t.parse().ok()?;
        (v.is_finite() && format!("{v}") == t && t.contains('.')).then(|| t.to_string())
    }

    fn leaf(name: &str, parent: &str, text: &str, custom: bool) -> J {
        if !custom && (STRINGS.contains(&name) || parent == "Regulatory") {
            return J::Str(text.to_string());
        }
        let t = text.trim();
        if t.eq_ignore_ascii_case("true") {
            return J::Bool(true);
        }
        if t.eq_ignore_ascii_case("false") {
            return J::Bool(false);
        }
        match canonical_number(t) {
            Some(n) => J::Num(n),
            None => J::Str(text.to_string()),
        }
    }

    fn num(t: &str) -> J {
        match canonical_number(t.trim()) {
            Some(n) => J::Num(n),
            None => match t.trim().parse::<f64>() {
                Ok(v) if v.is_finite() => J::Num(format!("{v}")),
                _ => J::Str(t.to_string()),
            },
        }
    }

    fn attr<'a>(e: &'a El, k: &str) -> &'a str {
        e.attrs
            .iter()
            .find(|(a, _)| a == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("0")
    }

    /// `IntData` points of an intensity container: one grid
    /// (`IntDataSymm`: h, v, rows of values) when they form the
    /// NumberHorz × NumberVert planes in order, else `IntDataNoSymm`
    /// triples.
    fn pack_intensity(e: &El, pts: &[&El]) -> (String, J) {
        let count = |n: &str| -> usize {
            e.child(n)
                .and_then(|c| c.text.as_deref())
                .and_then(|t| t.trim().parse().ok())
                .unwrap_or(0)
        };
        let (nh, nv) = (count("NumberHorz"), count("NumberVert"));
        let mut hs: Vec<&str> = Vec::new();
        let mut vs: Vec<&str> = Vec::new();
        for p in pts {
            let (h, v) = (attr(p, "h"), attr(p, "v"));
            if !hs.contains(&h) {
                hs.push(h);
            }
            if !vs.contains(&v) {
                vs.push(v);
            }
        }
        let grid = nh > 0
            && nv > 0
            && hs.len() == nh
            && vs.len() == nv
            && pts.len() == nh * nv
            && pts
                .iter()
                .enumerate()
                .all(|(k, p)| attr(p, "h") == hs[k / nv] && attr(p, "v") == vs[k % nv]);
        let value = |p: &El| num(p.text.as_deref().unwrap_or("0"));
        if grid {
            let rows = (0..nh)
                .map(|i| J::Arr(pts[i * nv..(i + 1) * nv].iter().map(|p| value(p)).collect()))
                .collect();
            (
                "IntDataSymm".into(),
                J::Obj(vec![
                    ("h".into(), J::Arr(hs.iter().map(|h| num(h)).collect())),
                    ("v".into(), J::Arr(vs.iter().map(|v| num(v)).collect())),
                    ("IntData".into(), J::Arr(rows)),
                ]),
            )
        } else {
            (
                "IntDataNoSymm".into(),
                J::Arr(
                    pts.iter()
                        .map(|p| J::Arr(vec![num(attr(p, "h")), num(attr(p, "v")), value(p)]))
                        .collect(),
                ),
            )
        }
    }

    fn pack_pairs(pts: &[&El], key: &str, a: &str, b: &str, block: &str) -> (String, J) {
        (
            block.into(),
            J::Obj(vec![
                (
                    a.into(),
                    J::Arr(pts.iter().map(|p| num(attr(p, key))).collect()),
                ),
                (
                    b.into(),
                    J::Arr(
                        pts.iter()
                            .map(|p| num(p.text.as_deref().unwrap_or("0")))
                            .collect(),
                    ),
                ),
            ]),
        )
    }

    /// Elements of an XML fragment (the content of a `CustomData` block).
    fn fragment(xml: &str) -> Vec<El> {
        use quick_xml::events::Event;
        use quick_xml::Reader;
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut stack: Vec<El> = vec![El::new("r")];
        let start = |e: &quick_xml::events::BytesStart| -> El {
            let mut el = El::new(&String::from_utf8_lossy(e.name().as_ref()));
            for a in e.attributes().flatten() {
                el.attrs.push((
                    String::from_utf8_lossy(a.key.as_ref()).to_string(),
                    a.unescape_value()
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                ));
            }
            el
        };
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => stack.push(start(&e)),
                Ok(Event::Empty(e)) => {
                    let el = start(&e);
                    if let Some(top) = stack.last_mut() {
                        top.children.push(el);
                    }
                }
                Ok(Event::Text(t)) => {
                    if let Some(top) = stack.last_mut() {
                        let s = t.unescape().map(|s| s.to_string()).unwrap_or_default();
                        top.text = Some(top.text.take().unwrap_or_default() + &s);
                    }
                }
                Ok(Event::CData(t)) => {
                    if let Some(top) = stack.last_mut() {
                        let s = String::from_utf8_lossy(&t).to_string();
                        top.text = Some(top.text.take().unwrap_or_default() + &s);
                    }
                }
                Ok(Event::End(_)) => {
                    if stack.len() > 1 {
                        let el = stack.pop().expect("stack");
                        if let Some(top) = stack.last_mut() {
                            top.children.push(el);
                        }
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }
        stack
            .into_iter()
            .next()
            .map(|r| r.children)
            .unwrap_or_default()
    }

    fn el_json(e: &El, parent: &str, custom: bool) -> J {
        if e.children.is_empty() && e.raw.is_none() && e.attrs.is_empty() {
            return leaf(&e.name, parent, e.text.as_deref().unwrap_or(""), custom);
        }
        let mut out: Vec<(String, J)> = Vec::new();
        for (k, v) in &e.attrs {
            out.push((format!("@{k}"), J::Str(v.clone())));
        }
        if let Some(t) = e.text.as_deref().filter(|t| !t.trim().is_empty()) {
            out.push(("#text".into(), leaf(&e.name, parent, t, custom)));
        }
        let packed: Option<&str> = match e.name.as_str() {
            n if !custom && INTENSITY.contains(&n) => Some("IntData"),
            "EmitterSpectral" if !custom => Some("PwrData"),
            "TiltAngles" if !custom => Some("Tilt"),
            _ => None,
        };
        let mut children: Vec<El> = e.children.clone();
        if let Some(raw) = &e.raw {
            children.extend(fragment(raw));
        }
        // group by name, in order of first appearance
        let mut groups: Vec<(String, Vec<&El>)> = Vec::new();
        for c in &children {
            match groups.iter_mut().find(|(n, _)| *n == c.name) {
                Some((_, g)) => g.push(c),
                None => groups.push((c.name.clone(), vec![c])),
            }
        }
        for (name, group) in groups {
            if Some(name.as_str()) == packed {
                let (k, v) = match name.as_str() {
                    "IntData" => pack_intensity(e, &group),
                    "PwrData" => pack_pairs(&group, "w", "w", "PwrData", "PwrDataArray"),
                    _ => pack_pairs(&group, "angle", "Angle", "Mult", "TiltArray"),
                };
                out.push((k, v));
                continue;
            }
            let child_custom =
                custom || (e.name == "CustomData" && name != "Name" && name != "UniqueIdentifier");
            let vals: Vec<J> = group
                .iter()
                .map(|c| el_json(c, &e.name, child_custom))
                .collect();
            let as_array = group.len() > 1 || (!child_custom && ARRAYS.contains(&name.as_str()));
            out.push((
                name,
                if as_array {
                    J::Arr(vals)
                } else {
                    vals.into_iter().next().expect("one")
                },
            ));
        }
        J::Obj(out)
    }

    fn json_str(s: &str) -> String {
        serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
    }

    fn pretty(j: &J, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth + 1);
        let end = "  ".repeat(depth);
        match j {
            J::Str(s) => out.push_str(&json_str(s)),
            J::Num(n) => out.push_str(n),
            J::Bool(b) => out.push_str(bool_text(*b)),
            J::Arr(items) if items.iter().all(|i| matches!(i, J::Num(_) | J::Bool(_))) => {
                out.push('[');
                for (k, i) in items.iter().enumerate() {
                    if k > 0 {
                        out.push_str(", ");
                    }
                    pretty(i, depth, out);
                }
                out.push(']');
            }
            J::Arr(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (k, i) in items.iter().enumerate() {
                    out.push_str(&pad);
                    pretty(i, depth + 1, out);
                    out.push_str(if k + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&end);
                out.push(']');
            }
            J::Obj(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (k, (key, v)) in entries.iter().enumerate() {
                    out.push_str(&pad);
                    out.push_str(&json_str(key));
                    out.push_str(": ");
                    pretty(v, depth + 1, out);
                    out.push_str(if k + 1 < entries.len() { ",\n" } else { "\n" });
                }
                out.push_str(&end);
                out.push('}');
            }
        }
    }

    /// TM-33-23 JSON (§4.8): `"FileType": "IESTM33-22"`, element names as
    /// keys, repeated elements as arrays, intensities as `IntDataSymm`
    /// grids, spectra as `PwrDataArray`. Attributes inside `CustomData`
    /// (TM-32-24 `guid`, `unit`) are `"@name"` keys, their text `"#text"`.
    pub fn write_json(doc: &LuminaireOpticalData, opts: &WriteOptions) -> Result<String> {
        let root = document(doc, opts);
        let mut entries = vec![("FileType".to_string(), J::Str("IESTM33-22".into()))];
        if let J::Obj(rest) = el_json(&root, "", false) {
            entries.extend(rest);
        }
        let mut out = String::new();
        pretty(&J::Obj(entries), 0, &mut out);
        out.push('\n');
        Ok(out)
    }

    /// A JSON document in the TM-33-23 form (not ATLA S001-A JSON).
    pub fn is_tm33_json(text: &str) -> bool {
        let head: String = text.chars().take(4096).collect();
        head.contains("\"IESTM33") || head.contains("\"Header\"") || head.contains("\"Emitter\"")
    }

    fn scalar_text(v: &Value) -> Option<String> {
        match v {
            Value::Bool(b) => Some(bool_text(*b).to_string()),
            Value::Number(n) => Some(if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                format!("{}", n.as_f64().unwrap_or(0.0))
            }),
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn numbers(v: Option<&Value>) -> Vec<String> {
        match v {
            Some(Value::Array(a)) => a.iter().filter_map(scalar_text).collect(),
            Some(x) => scalar_text(x).into_iter().collect(),
            None => Vec::new(),
        }
    }

    fn point(name: &str, attrs: &[(&str, &String)], value: String) -> El {
        let mut p = El::with_text(name, value);
        for (k, v) in attrs {
            p.attrs.push((k.to_string(), (*v).clone()));
        }
        p
    }

    /// `IntData` elements from `IntDataSymm` (the grid of §4.8.1.2.2, or
    /// the Annex A form with `h` / `v` next to it and a flat list for one
    /// plane) or `IntDataNoSymm` triples.
    fn unpack_intensity(o: &serde_json::Map<String, Value>, el: &mut El) {
        if let Some(sym) = o.get("IntDataSymm") {
            let (hs, vs, data) = match sym {
                Value::Object(g) => (numbers(g.get("h")), numbers(g.get("v")), g.get("IntData")),
                other => (numbers(o.get("h")), numbers(o.get("v")), Some(other)),
            };
            let rows: Vec<Vec<String>> = match data {
                Some(Value::Array(a)) if a.iter().all(|x| x.is_array()) => {
                    a.iter().map(|r| numbers(Some(r))).collect()
                }
                Some(Value::Array(a)) => vec![a.iter().filter_map(scalar_text).collect()],
                _ => Vec::new(),
            };
            for (i, row) in rows.iter().enumerate() {
                let Some(h) = hs.get(i) else { break };
                for (j, x) in row.iter().enumerate() {
                    if let Some(v) = vs.get(j) {
                        el.add(point("IntData", &[("h", h), ("v", v)], x.clone()));
                    }
                }
            }
        }
        if let Some(Value::Array(pts)) = o.get("IntDataNoSymm") {
            for p in pts {
                let t = numbers(Some(p));
                if t.len() >= 3 {
                    let x = t[t.len() - 1].clone();
                    let mut attrs: Vec<(&str, &String)> = vec![("h", &t[0]), ("v", &t[1])];
                    if t.len() >= 4 {
                        attrs.push(("w", &t[2]));
                    }
                    el.add(point("IntData", &attrs, x));
                }
            }
        }
    }

    fn unpack_pairs(v: &Value, a: &str, b: &str, name: &str, attr_name: &str, el: &mut El) {
        let (xs, ys) = (numbers(v.get(a)), numbers(v.get(b)));
        for (x, y) in xs.iter().zip(&ys) {
            el.add(point(name, &[(attr_name, x)], y.clone()));
        }
    }

    fn json_into(parent: &mut El, key: &str, v: &Value) {
        let key = if key == "Comments" && parent.name == "Header" {
            "Comment"
        } else {
            key
        };
        match v {
            Value::Null => {}
            Value::Array(items) => {
                for it in items {
                    json_into(parent, key, it);
                }
            }
            Value::Object(o) => {
                let mut el = El::new(key);
                let intensity = INTENSITY.contains(&key)
                    && (o.contains_key("IntDataSymm") || o.contains_key("IntDataNoSymm"));
                for (k, val) in o {
                    if let Some(a) = k.strip_prefix('@') {
                        if let Some(t) = scalar_text(val) {
                            el.attrs.push((a.to_string(), t));
                        }
                        continue;
                    }
                    if k == "#text" {
                        el.text = scalar_text(val);
                        continue;
                    }
                    match k.as_str() {
                        "IntDataSymm" | "IntDataNoSymm" if intensity => {}
                        "h" | "v"
                            if intensity
                                && o.get("IntDataSymm").is_some_and(|x| !x.is_object()) => {}
                        "PwrDataArray" => {
                            unpack_pairs(val, "w", "PwrData", "PwrData", "w", &mut el)
                        }
                        "TiltArray" => unpack_pairs(val, "Angle", "Mult", "Tilt", "angle", &mut el),
                        _ => json_into(&mut el, k, val),
                    }
                }
                if intensity {
                    unpack_intensity(o, &mut el);
                }
                parent.add(el);
            }
            scalar => {
                if let Some(t) = scalar_text(scalar) {
                    parent.leaf(key, t);
                }
            }
        }
    }

    /// Read a TM-33-23 JSON document.
    pub fn parse_json(text: &str) -> Result<LuminaireOpticalData> {
        let v: Value = serde_json::from_str(text)?;
        let obj = v
            .as_object()
            .ok_or_else(|| AtlaError::JsonParse("TM-33-23 JSON: not an object".into()))?;
        let body = obj
            .get("IESTM33-22")
            .or_else(|| obj.get("IESTM33"))
            .and_then(Value::as_object)
            .unwrap_or(obj);
        let mut root = El::new("IESTM33-22");
        // schema order (the map is sorted by key)
        const ORDER: [&str; 6] = [
            "Version",
            "Header",
            "Luminaire",
            "Equipment",
            "Emitter",
            "CustomData",
        ];
        let rank = |k: &str| ORDER.iter().position(|o| *o == k).unwrap_or(ORDER.len());
        let mut keys: Vec<&String> = body.keys().filter(|k| *k != "FileType").collect();
        keys.sort_by_key(|k| rank(k));
        for k in keys {
            json_into(&mut root, k, &body[k.as_str()]);
        }
        if root.child("Version").is_none() {
            root.children.insert(0, El::with_text("Version", "1.1"));
        }
        crate::atla::xml::parse(&xml_document(&root, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn significant_digits() {
        assert_eq!(sig(397.849_999_999_999_97, 6), "397.85");
        assert_eq!(sig(42000.0, 6), "42000");
        assert_eq!(sig(0.001_234_567, 6), "0.00123457");
        assert_eq!(sig(1.2, 9), "1.2");
        assert_eq!(sig(-0.000_000_01, 3), "-0.00000001");
        assert_eq!(sig(0.0, 6), "0");
        assert_eq!(sig(123_456_789.0, 6), "123457000");
    }

    #[test]
    fn dates() {
        assert_eq!(iso_date("2024-06-15").as_deref(), Some("2024-06-15"));
        assert_eq!(
            iso_date("2024-06-15T10:00:00Z").as_deref(),
            Some("2024-06-15")
        );
        assert_eq!(iso_date("15.06.2024").as_deref(), Some("2024-06-15"));
        assert_eq!(iso_date("6/15/2024").as_deref(), Some("2024-06-15"));
        assert_eq!(iso_date("07.2019 / JD"), None);
        assert_eq!(iso_date(""), None);
    }

    #[test]
    fn symmetry_from_planes() {
        assert_eq!(infer_symmetry(&[0.0]), SymmetryType::Full);
        assert_eq!(infer_symmetry(&[0.0, 45.0, 90.0]), SymmetryType::Quad);
        assert_eq!(infer_symmetry(&[0.0, 90.0, 180.0]), SymmetryType::Bi0);
        assert_eq!(
            infer_symmetry(&[0.0, 90.0, 180.0, 270.0, 345.0]),
            SymmetryType::None
        );
    }
}
