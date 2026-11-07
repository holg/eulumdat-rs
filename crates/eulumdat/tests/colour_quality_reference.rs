//! Our TM-30 (CIE 224) and CIE 13.3 CRI against an independent
//! implementation.
//!
//! `tests/fixtures/colour_quality/reference.json` holds public spectra (CIE
//! FL / HP illuminants, A, D65 and NIST CQS model lamps) already resampled to
//! 380–780 nm at 5 nm, with the results of colour-science 0.4.7 for exactly
//! those values. colour-science is only the yardstick: nothing in the crate
//! uses it.
#![cfg(feature = "json")]

use eulumdat::atla::types::{SpectralDistribution, SpectralUnits};
use eulumdat::atla::{calculate_cri, calculate_tm30};

struct Worst {
    what: &'static str,
    diff: f64,
    case: String,
}

impl Worst {
    fn new(what: &'static str) -> Self {
        Self {
            what,
            diff: 0.0,
            case: String::new(),
        }
    }
    fn see(&mut self, case: &str, ours: f64, theirs: f64) {
        let d = (ours - theirs).abs();
        if d > self.diff {
            self.diff = d;
            self.case = case.to_string();
        }
    }
    fn check(&self, tol: f64) {
        println!("{:>8}: max |Δ| {:.5} ({})", self.what, self.diff, self.case);
        assert!(
            self.diff <= tol,
            "{}: {:.5} > {tol} ({})",
            self.what,
            self.diff,
            self.case
        );
    }
}

fn nums(v: &serde_json::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn tm30_and_cri_match_the_independent_reference() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/colour_quality/reference.json"
    ))
    .unwrap();
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();

    let mut cct = Worst::new("CCT (K)");
    let mut duv = Worst::new("Duv");
    let mut rf = Worst::new("Rf");
    let mut rg = Worst::new("Rg");
    let mut rf_h = Worst::new("Rf,h");
    let mut rcs_h = Worst::new("Rcs,h %");
    let mut rhs_h = Worst::new("Rhs,h");
    let mut ra = Worst::new("Ra");
    let mut ri = Worst::new("R1-R15");
    let mut cases = 0;

    for case in doc["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let values = nums(&case["spd"]);
        let spd = SpectralDistribution {
            wavelengths: (0..values.len()).map(|i| 380.0 + 5.0 * i as f64).collect(),
            values,
            units: SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        };
        cases += 1;

        let tm = calculate_tm30(&spd).unwrap_or_else(|| panic!("{name}: no TM-30"));
        let ref_cct = case["cct"].as_f64().unwrap();
        cct.see(name, tm.cct, ref_cct);
        duv.see(name, tm.duv, case["duv"].as_f64().unwrap());
        rf.see(name, tm.rf, case["rf"].as_f64().unwrap());
        rg.see(name, tm.rg, case["rg"].as_f64().unwrap());
        for (j, v) in nums(&case["rf_h"]).iter().enumerate() {
            rf_h.see(name, tm.rf_hue[j], *v);
        }
        for (j, v) in nums(&case["rcs_h"]).iter().enumerate() {
            rcs_h.see(name, tm.rcs_hue[j], *v);
        }
        for (j, v) in nums(&case["rhs_h"]).iter().enumerate() {
            rhs_h.see(name, tm.rhs_hue[j], *v);
        }

        // The CRI reference jumps from Planckian to daylight at 5000 K; a
        // source within a few kelvin of the switch can land on either side.
        if (ref_cct - 5000.0).abs() < 5.0 {
            continue;
        }
        let c = calculate_cri(&spd).unwrap_or_else(|| panic!("{name}: no CRI"));
        ra.see(name, c.ra, case["ra"].as_f64().unwrap());
        for (i, v) in nums(&case["ri"]).iter().enumerate() {
            ri.see(name, c.ri[i], *v);
        }
    }
    println!("{cases} spectra");
    assert!(cases >= 30);
    // Measured (2026-11-05): CCT 1.8 K (the reference uses Ohno 2013, we
    // search the locus exactly), Duv 1e-5, Rf 0.007, Rg 0.002, Rf,h 0.011,
    // Rcs,h 0.006 %, Rhs,h 8e-5, Ra 0.017, Ri 0.14 (low-pressure sodium).
    cct.check(3.0);
    duv.check(5e-5);
    rf.check(0.05);
    rg.check(0.05);
    rf_h.check(0.05);
    rcs_h.check(0.05);
    rhs_h.check(1e-3);
    ra.check(0.05);
    ri.check(0.3);
}
