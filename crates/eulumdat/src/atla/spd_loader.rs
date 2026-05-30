//! Universal loader for spectral power distribution files.
//!
//! Auto-detects four formats produced by real spectrometers / vendor datasheets:
//!
//! 1. **Luxeon `.spd`** — tab-separated, `#` comments, columns `wavelength[nm]
//!    relative_power`. Typically datasheet-digitised, uniform 5 nm grid.
//! 2. **Generic `wavelength_nm,intensity` CSV** — used by Yuji "WB Day/Nite"
//!    captures and Luxeon 95-CRI exports. Header row `wavelength_nm,intensity`,
//!    irregular 1 nm-ish grid (can have repeated or non-integer wavelengths).
//! 3. **Signify lab CSV** — a compound format from Signify's lab spectrometer.
//!    The first ~67 rows are a numbered metric prelude (CIE values, CCT, CRI
//!    R1–R15, Duv, PPFD, EML, S/P, blue-light hazard…); the SPD body follows
//!    after a `wavelength` header line. Both halves are parsed: the prelude
//!    surfaces as [`ReferenceMetrics`] for cross-checking our own colorimetry,
//!    and the body becomes the [`SpectralDistribution`].
//! 4. **Anything that looks like two whitespace-separated numeric columns** —
//!    fallback, for ad-hoc spectrometer exports.
//!
//! Irregular grids with duplicate wavelengths (a quirk of some Yuji exports)
//! are collapsed — the duplicate's value is averaged — but spacing is NOT
//! resampled; downstream colorimetry uses trapezoidal integration that handles
//! non-uniform grids correctly.

use crate::atla::types::{
    ColorRendering, Emitter, Header, LuminaireOpticalData, SpectralDistribution, SpectralUnits,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Pre-computed metrics that some vendor files (notably Signify) ship inline.
/// Used as a *ground truth* to validate our own CIE colorimetry against.
///
/// Only fields actually observed in the Signify corpus are modelled; everything
/// is `Option` because no other vendor ships these inline.
#[derive(Debug, Clone, Default)]
pub struct ReferenceMetrics {
    // CIE 1931
    pub cie_x: Option<f64>, // tristimulus X
    pub cie_y: Option<f64>,
    pub cie_z: Option<f64>,
    pub chromaticity_x: Option<f64>,
    pub chromaticity_y: Option<f64>,
    // CIE 1960
    pub cie1960_u: Option<f64>,
    pub cie1960_v: Option<f64>,
    // CIE 1976
    pub cie1976_u_prime: Option<f64>,
    pub cie1976_v_prime: Option<f64>,
    // Color appearance
    pub cct_k: Option<f64>,
    pub duv: Option<f64>,
    pub dominant_wavelength_nm: Option<f64>,
    pub peak_wavelength_nm: Option<f64>,
    pub half_peak_width_nm: Option<f64>,
    pub purity_pct: Option<f64>,
    // Color rendering
    pub ra: Option<f64>,
    pub r_special: [Option<f64>; 15], // R1..R15
    pub cqs: Option<f64>,
    pub gai_ees: Option<f64>,
    pub gai_bb_8: Option<f64>,
    pub gai_bb_15: Option<f64>,
    // Photopic / vision
    pub illuminance_lx: Option<f64>,
    pub irradiance_w_m2: Option<f64>,
    pub sp_ratio: Option<f64>,
    // Circadian
    pub eml: Option<f64>,
    pub m_edi: Option<f64>,
    // Horticulture
    pub par_w_m2: Option<f64>,
    pub ppfd_umol_m2s: Option<f64>,
    pub ypfd_umol_m2s: Option<f64>,
    // Safety
    pub blue_light_hazard_w_m2: Option<f64>,
    pub blue_light_risk_group: Option<String>,
    // Tolerance
    pub sdcm: Option<f64>,
}

/// What `load` returns: the SPD and (if present) reference metrics from the file.
#[derive(Debug, Clone, Default)]
pub struct LoadedSpd {
    pub spd: SpectralDistribution,
    pub reference: Option<ReferenceMetrics>,
    /// Free-text identifier (filename stem, manufacturer header line, etc.).
    pub label: String,
}

/// Errors that can come out of SPD loading.
#[derive(Debug)]
pub enum SpdLoadError {
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for SpdLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpdLoadError::Io(e) => write!(f, "io: {e}"),
            SpdLoadError::Parse(s) => write!(f, "parse: {s}"),
        }
    }
}

impl std::error::Error for SpdLoadError {}

impl From<std::io::Error> for SpdLoadError {
    fn from(e: std::io::Error) -> Self {
        SpdLoadError::Io(e)
    }
}

/// Load an SPD file, auto-detecting the format.
pub fn load<P: AsRef<Path>>(path: P) -> Result<LoadedSpd, SpdLoadError> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    let label = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut loaded = parse(&content)?;
    if loaded.label.is_empty() {
        loaded.label = label;
    }
    Ok(loaded)
}

/// Parse SPD content from a string. Format is auto-detected.
pub fn parse(content: &str) -> Result<LoadedSpd, SpdLoadError> {
    // Signify prelude rows start with `1,CIE1931 colorspace tristimulus values X,`
    // and the metric block ends with a literal `wavelength` line before the body.
    if content.starts_with("1,CIE1931") || content.contains("\n1,CIE1931") {
        return parse_signify(content);
    }
    // Generic `wavelength_nm,intensity` header.
    let first_line = content.lines().next().unwrap_or("").trim().to_ascii_lowercase();
    if first_line.starts_with("wavelength_nm") || first_line == "wavelength,intensity" {
        return parse_named_csv(content);
    }
    // Luxeon `.spd`: hash comments, tab/whitespace-separated two-column body.
    if content.lines().any(|l| l.trim_start().starts_with('#')) {
        return parse_luxeon_spd(content);
    }
    // Fallback: best-effort two-column whitespace/comma split.
    parse_two_column(content)
}

// ────────────────────────────────────────────────────────────────────────────
// Format parsers
// ────────────────────────────────────────────────────────────────────────────

fn parse_luxeon_spd(content: &str) -> Result<LoadedSpd, SpdLoadError> {
    let mut label = String::new();
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            // Pick the first comment line that looks like a title (not column hdr).
            let r = rest.trim();
            if label.is_empty() && !r.starts_with("Wavelength") && !r.is_empty() {
                label = r.to_string();
            }
            continue;
        }
        let mut it = line.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty());
        if let (Some(w), Some(v)) = (it.next(), it.next()) {
            let (Ok(w), Ok(v)) = (w.parse::<f64>(), v.parse::<f64>()) else { continue };
            pairs.push((w, v));
        }
    }
    if pairs.is_empty() {
        return Err(SpdLoadError::Parse("Luxeon .spd: no wavelength/value rows".into()));
    }
    Ok(LoadedSpd { spd: build_spd(pairs), reference: None, label })
}

fn parse_named_csv(content: &str) -> Result<LoadedSpd, SpdLoadError> {
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if i == 0 {
            continue; // header
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        if let (Some(w), Some(v)) = (it.next(), it.next()) {
            let (Ok(w), Ok(v)) = (w.trim().parse::<f64>(), v.trim().parse::<f64>()) else { continue };
            pairs.push((w, v));
        }
    }
    if pairs.is_empty() {
        return Err(SpdLoadError::Parse("named CSV: no rows after header".into()));
    }
    Ok(LoadedSpd { spd: build_spd(pairs), reference: None, label: String::new() })
}

fn parse_two_column(content: &str) -> Result<LoadedSpd, SpdLoadError> {
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty());
        if let (Some(w), Some(v)) = (it.next(), it.next()) {
            if let (Ok(w), Ok(v)) = (w.parse::<f64>(), v.parse::<f64>()) {
                pairs.push((w, v));
            }
        }
    }
    if pairs.is_empty() {
        return Err(SpdLoadError::Parse("fallback two-column: no numeric rows".into()));
    }
    Ok(LoadedSpd { spd: build_spd(pairs), reference: None, label: String::new() })
}

fn parse_signify(content: &str) -> Result<LoadedSpd, SpdLoadError> {
    let mut metrics = ReferenceMetrics::default();
    let mut in_body = false;
    let mut pairs: Vec<(f64, f64)> = Vec::new();

    for line in content.lines() {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            continue;
        }
        if in_body {
            let mut it = line.split(',');
            if let (Some(w), Some(v)) = (it.next(), it.next()) {
                if let (Ok(w), Ok(v)) = (w.trim().parse::<f64>(), v.trim().parse::<f64>()) {
                    pairs.push((w, v));
                }
            }
            continue;
        }
        // The SPD body is preceded by a literal `wavelength` line (one token).
        if line.eq_ignore_ascii_case("wavelength") {
            in_body = true;
            continue;
        }
        // Prelude row: `N,Label,value`. Some lines (Exposure Time) lack the
        // leading number; handle both shapes.
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 2 {
            continue;
        }
        // Skip the index column if present.
        let (label, raw_val) = if parts.len() >= 3 && parts[0].trim().parse::<u32>().is_ok() {
            (parts[1].trim(), parts[2..].join(","))
        } else {
            (parts[0].trim(), parts[1..].join(","))
        };
        assign_signify_metric(&mut metrics, label, raw_val.trim());
    }
    if pairs.is_empty() {
        return Err(SpdLoadError::Parse(
            "Signify: prelude parsed but no SPD body after `wavelength` line".into(),
        ));
    }
    Ok(LoadedSpd { spd: build_spd(pairs), reference: Some(metrics), label: String::new() })
}

/// Map a Signify prelude label/value pair into the typed [`ReferenceMetrics`].
/// Match prefix-loose: "Color render index R1" must beat "...Ra".
fn assign_signify_metric(m: &mut ReferenceMetrics, label: &str, raw: &str) {
    // Some values come as "0.7040/RG0" (number + risk group). Split off the slash.
    let (numeric_str, after_slash) = match raw.split_once('/') {
        Some((a, b)) => (a.trim(), Some(b.trim())),
        None => (raw, None),
    };
    let num = numeric_str.parse::<f64>().ok();
    let l = label;
    if l.starts_with("CIE1931 colorspace tristimulus values X") { m.cie_x = num; }
    else if l.starts_with("CIE1931 colorspace tristimulus values Y") { m.cie_y = num; }
    else if l.starts_with("CIE1931 colorspace tristimulus values Z") { m.cie_z = num; }
    else if l.starts_with("CIE1931 colorspace chromaticity coordinates x") { m.chromaticity_x = num; }
    else if l.starts_with("CIE1931 colorspace chromaticity coordinates y") { m.chromaticity_y = num; }
    else if l.starts_with("CIE1960 colorspace chromaticity coordinates u") && !l.contains('\'') { m.cie1960_u = num; }
    else if l.starts_with("CIE1960 colorspace chromaticity coordinates v") && !l.contains('\'') { m.cie1960_v = num; }
    else if l.starts_with("CIE1976 colorspace chromaticity coordinates u'") { m.cie1976_u_prime = num; }
    else if l.starts_with("CIE1976 colorspace chromaticity coordinates v'") { m.cie1976_v_prime = num; }
    else if l.starts_with("Color temperature CCT") { m.cct_k = num; }
    else if l.starts_with("Color shift Duv") { m.duv = num; }
    else if l.starts_with("Color render index Ra") { m.ra = num; }
    else if let Some(idx) = l.strip_prefix("Color render index R") {
        if let Ok(i) = idx.parse::<usize>() {
            if (1..=15).contains(&i) { m.r_special[i - 1] = num; }
        }
    }
    else if l.starts_with("Peak wavelength Lp") { m.peak_wavelength_nm = num; }
    else if l.starts_with("Half-peak width HW") { m.half_peak_width_nm = num; }
    else if l.starts_with("Dominant wavelength Ld") { m.dominant_wavelength_nm = num; }
    else if l.starts_with("Color purity Purity") { m.purity_pct = num; }
    else if l.starts_with("Light-dark vision ratio S/P") { m.sp_ratio = num; }
    else if l.starts_with("Illuminance Lux") { m.illuminance_lx = num; }
    else if l.starts_with("Irradiance Ee") { m.irradiance_w_m2 = num; }
    else if l.starts_with("Color quality scale CQS") { m.cqs = num; }
    else if l.starts_with("Gamut area index GAI_EES") { m.gai_ees = num; }
    else if l.starts_with("Gamut area index GAI_BB_8") { m.gai_bb_8 = num; }
    else if l.starts_with("Gamut area index GAI_BB_15") { m.gai_bb_15 = num; }
    else if l.starts_with("Equivalen melanopic lux EML") { m.eml = num; }
    else if l.starts_with("Melanopic equivalent daylight illuminance M_EDI") { m.m_edi = num; }
    else if l.starts_with("Photosynthetically active radiation PAR") { m.par_w_m2 = num; }
    else if l.starts_with("Photosynthetic photon flux density PPFD") { m.ppfd_umol_m2s = num; }
    else if l.starts_with("Yield photon flux density YPFD") { m.ypfd_umol_m2s = num; }
    else if l.starts_with("Blue light hazard weighted irradiance Eb") {
        m.blue_light_hazard_w_m2 = num;
        m.blue_light_risk_group = after_slash.map(|s| s.to_string());
    }
    else if l.starts_with("Color tolerance SDCM") {
        // "2.48/3000" → SDCM value is the first part.
        m.sdcm = num;
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Common: build a SpectralDistribution from collected (λ, value) pairs.
// Handles duplicate wavelengths (average), sorts ascending, and detects uniform
// spacing to populate start_wavelength / wavelength_interval.
// ────────────────────────────────────────────────────────────────────────────

fn build_spd(pairs: Vec<(f64, f64)>) -> SpectralDistribution {
    // Collapse duplicates by averaging — Yuji exports occasionally repeat λ.
    // BTreeMap key by integer-microns to dedupe without f64 hashing issues.
    let mut acc: BTreeMap<i64, (f64, f64, usize)> = BTreeMap::new(); // key=λ*1000, (λ, Σv, n)
    for (w, v) in pairs {
        if !w.is_finite() || !v.is_finite() {
            continue;
        }
        let key = (w * 1000.0).round() as i64;
        let e = acc.entry(key).or_insert((w, 0.0, 0));
        e.1 += v;
        e.2 += 1;
    }
    let mut wavelengths = Vec::with_capacity(acc.len());
    let mut values = Vec::with_capacity(acc.len());
    for (_, (w, sum, n)) in acc {
        wavelengths.push(w);
        values.push(sum / n as f64);
    }

    // Detect uniform spacing: max(|step − mean step|) / mean step < 1e-3.
    let (start_wavelength, wavelength_interval) = if wavelengths.len() >= 2 {
        let n = wavelengths.len() - 1;
        let mean_step = (wavelengths[n] - wavelengths[0]) / n as f64;
        let uniform = wavelengths.windows(2).all(|w| {
            let s = w[1] - w[0];
            (s - mean_step).abs() / mean_step.abs().max(1e-9) < 1e-3
        });
        if uniform {
            (Some(wavelengths[0]), Some(mean_step))
        } else {
            (Some(wavelengths[0]), None)
        }
    } else {
        (wavelengths.first().copied(), None)
    };

    SpectralDistribution {
        wavelengths,
        values,
        units: SpectralUnits::Relative,
        start_wavelength,
        wavelength_interval,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ATLA bridge: turn a LoadedSpd into a LuminaireOpticalData so the rest of
// the editor (UI, diagrams, exporters) sees an SPD-only file the same way it
// already sees an SPDX (IES TM-27-14) file.
// ────────────────────────────────────────────────────────────────────────────

/// Build a minimal ATLA `LuminaireOpticalData` from a loaded SPD: one emitter
/// carrying the SPD and **no intensity distribution**. When the file is a
/// Signify lab export, the prelude's CCT / Ra / Duv / S/P are also populated on
/// the emitter so the dashboard's info panel shows the lab-measured values.
///
/// Polar/cartesian diagrams will be empty for these files — the same caveat
/// as SPDX (see [`crate::atla::spdx::to_atla`]).
pub fn to_atla(loaded: &LoadedSpd) -> LuminaireOpticalData {
    let label = if loaded.label.is_empty() { None } else { Some(loaded.label.clone()) };
    let mut doc = LuminaireOpticalData {
        header: Header {
            description: label.clone(),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut emitter = Emitter {
        description: label,
        spectral_distribution: Some(loaded.spd.clone()),
        quantity: 1,
        ..Default::default()
    };

    if let Some(r) = &loaded.reference {
        emitter.cct = r.cct_k;
        emitter.duv = r.duv;
        emitter.sp_ratio = r.sp_ratio;
        // Build a partial ColorRendering only if at least one CRI value is known.
        if r.ra.is_some() || r.r_special[8].is_some() {
            emitter.color_rendering = Some(ColorRendering {
                ra: r.ra,
                r9: r.r_special[8],
                rf: None,
                rg: None,
            });
        }
    }

    doc.emitters = vec![emitter];
    doc
}

/// User-facing warnings for an SPD-only load: same shape as `spdx::get_warnings`.
pub fn get_warnings(loaded: &LoadedSpd) -> Vec<String> {
    let mut w = vec![
        "SPD file contains spectral data only — no photometric (intensity) distribution.".to_string(),
        "Polar/cartesian diagrams will be empty. Only the spectral diagram is available.".to_string(),
    ];
    if loaded.spd.wavelengths.len() < 20 {
        w.push(format!(
            "Limited spectral resolution: only {} data points.",
            loaded.spd.wavelengths.len()
        ));
    }
    if loaded.reference.is_some() {
        w.push("Vendor reference metrics (CCT, Duv, CRI) were parsed from the file prelude.".into());
    }
    w
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_luxeon_spd() {
        let s = "# Luxeon test\n# Wavelength[nm]\tRelative_Power\n400\t0.5\n410\t0.8\n420\t1.0\n";
        let r = parse(s).unwrap();
        assert_eq!(r.spd.wavelengths, vec![400.0, 410.0, 420.0]);
        assert_eq!(r.spd.values, vec![0.5, 0.8, 1.0]);
        assert_eq!(r.spd.wavelength_interval, Some(10.0));
        assert!(r.reference.is_none());
        assert_eq!(r.label, "Luxeon test");
    }

    #[test]
    fn detects_named_csv() {
        let s = "wavelength_nm,intensity\n400.0,0.5\n401.0,0.7\n402.0,0.9\n";
        let r = parse(s).unwrap();
        assert_eq!(r.spd.wavelengths.len(), 3);
        assert!((r.spd.wavelength_interval.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn duplicate_wavelengths_are_averaged() {
        let s = "wavelength_nm,intensity\n400.0,0.4\n400.0,0.6\n401.0,1.0\n";
        let r = parse(s).unwrap();
        assert_eq!(r.spd.wavelengths, vec![400.0, 401.0]);
        assert!((r.spd.values[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn detects_signify_and_parses_metrics() {
        let s = "1,CIE1931 colorspace tristimulus values X,107.88\n\
                 4,CIE1931 colorspace chromaticity coordinates x,0.4386\n\
                 5,CIE1931 colorspace chromaticity coordinates y,0.4066\n\
                 10,Color temperature CCT(K),2997\n\
                 14,Color shift Duv,0.0010\n\
                 15,Color render index Ra,85.9\n\
                 16,Color render index R1,87.0\n\
                 24,Color render index R9,47.9\n\
                 30,Color render index R15,84.5\n\
                 45,Blue light hazard weighted irradiance Eb(W/m²),0.7040/RG0\n\
                 wavelength\n\
                 340,0\n\
                 341,0.1\n\
                 342,0.2\n";
        let r = parse(s).unwrap();
        let ref_m = r.reference.expect("Signify reference metrics");
        assert_eq!(ref_m.cie_x, Some(107.88));
        assert_eq!(ref_m.chromaticity_x, Some(0.4386));
        assert_eq!(ref_m.chromaticity_y, Some(0.4066));
        assert_eq!(ref_m.cct_k, Some(2997.0));
        assert_eq!(ref_m.duv, Some(0.0010));
        assert_eq!(ref_m.ra, Some(85.9));
        assert_eq!(ref_m.r_special[0], Some(87.0)); // R1
        assert_eq!(ref_m.r_special[8], Some(47.9)); // R9
        assert_eq!(ref_m.r_special[14], Some(84.5)); // R15
        assert_eq!(ref_m.blue_light_hazard_w_m2, Some(0.7040));
        assert_eq!(ref_m.blue_light_risk_group.as_deref(), Some("RG0"));
        assert_eq!(r.spd.wavelengths, vec![340.0, 341.0, 342.0]);
        assert_eq!(r.spd.values, vec![0.0, 0.1, 0.2]);
    }

    #[test]
    fn to_atla_carries_spd_and_signify_metadata() {
        // Compose a Signify-shaped string with the metrics we care about.
        let s = "1,CIE1931 colorspace tristimulus values X,107.88\n\
                 10,Color temperature CCT(K),2997\n\
                 14,Color shift Duv,0.0010\n\
                 15,Color render index Ra,85.9\n\
                 24,Color render index R9,47.9\n\
                 35,Light-dark vision ratio S/P,4.860\n\
                 wavelength\n\
                 400,0.1\n\
                 500,0.5\n\
                 600,1.0\n\
                 700,0.2\n";
        let loaded = parse(s).unwrap();
        let doc = to_atla(&loaded);
        assert_eq!(doc.emitters.len(), 1);
        let e = &doc.emitters[0];
        assert!(e.spectral_distribution.is_some(), "SPD must be attached to emitter");
        assert_eq!(e.cct, Some(2997.0));
        assert_eq!(e.duv, Some(0.001));
        assert_eq!(e.sp_ratio, Some(4.86));
        let cr = e.color_rendering.as_ref().expect("ColorRendering populated");
        assert_eq!(cr.ra, Some(85.9));
        assert_eq!(cr.r9, Some(47.9));
        // Intensity distribution must NOT be present — this is spectral-only.
        assert!(e.intensity_distribution.is_none());
        // Warnings are non-empty and mention SPD-only.
        let w = get_warnings(&loaded);
        assert!(w.iter().any(|m| m.contains("spectral data only")));
    }

    /// Round-trip every file in docs/SPDs/ that we have. Confirms the loader
    /// handles every real-world file we ship without producing an empty SPD.
    #[test]
    fn loads_every_real_corpus_file() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/SPDs");
        if !std::path::Path::new(root).exists() {
            return; // corpus may not be present in all checkouts
        }
        let mut total = 0;
        for sub in ["Luxeon_SPD_fixed", "luxeon_95CRI", "Youji-Nite", "Signify"] {
            let dir = format!("{root}/{sub}");
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                let ok_ext = matches!(
                    p.extension().and_then(|s| s.to_str()),
                    Some("spd") | Some("csv")
                );
                if !ok_ext { continue; }
                let r = load(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                assert!(
                    !r.spd.wavelengths.is_empty() && r.spd.values.len() == r.spd.wavelengths.len(),
                    "{}: empty or mismatched SPD",
                    p.display()
                );
                if sub == "Signify" {
                    let m = r.reference.as_ref().expect("Signify file must yield metrics");
                    assert!(m.cct_k.is_some(), "{}: missing CCT in prelude", p.display());
                    assert!(m.ra.is_some(), "{}: missing Ra in prelude", p.display());
                }
                total += 1;
            }
        }
        assert!(total >= 20, "expected >=20 corpus files, found {total}");
    }
}
