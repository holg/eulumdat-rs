//! Colorimetry and metrics of a TM-25 file's shared spectral table.
//!
//!     cargo run -p eulumdat-tm25 --example tm25_spectrum -- file.TM25RAY ...
//!
//! Prints CCT, Duv, chromaticity, luminous efficacy of radiation and the
//! file's own lm/W from its header totals, so a mixed file (tm25ray's
//! `tm25_mix`) can be checked against the colour bin it was mixed for.

use eulumdat_tm25::Tm25File;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        let f = match Tm25File::parse(&bytes) {
            Ok(f) => f,
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        let h = &f.header;
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(path.clone());
        let Some(t) = h.shared_spectrum() else {
            println!("{name}: no shared spectral table");
            continue;
        };
        let spd = eulumdat_spectrum::Spd::new(
            &t.wavelengths_nm
                .iter()
                .map(|w| *w as f64)
                .collect::<Vec<_>>(),
            &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
        );
        let c = eulumdat_spectrum::colorimetry::analyze(&spd);
        let m = eulumdat_spectrum::SpectralMetrics::from_spd(&spd);
        let file_lm_per_w = if h.radiant_flux_w > 0.0 {
            h.luminous_flux_lm / h.radiant_flux_w
        } else {
            0.0
        };
        println!(
            "{name}\n  CCT {:.0} K, Duv {:+.4}, x {:.4} y {:.4} · LER of the spectrum {:.1} lm/W · header {:.3} lm / {:.4} W = {:.1} lm/W\n  {} points {:.0}–{:.0} nm, peak at {:.0} nm, blue fraction {:.3}",
            c.cct_k, c.duv, c.x, c.y, m.ler_lm_per_w, h.luminous_flux_lm, h.radiant_flux_w, file_lm_per_w,
            t.len(), t.wavelengths_nm[0], t.wavelengths_nm.last().unwrap(), t.peak().map(|p| p.0).unwrap_or(0.0), m.blue_fraction
        );
    }
}
