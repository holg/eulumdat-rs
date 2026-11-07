//! "Colour quality" chapter for a luminaire with a spectrum: the colour
//! quality card (TM-30 vector graphic, Duv, Rf / Rg, Ra, R9, Re), the CIE 13.3
//! indices R1–R15 and the 16 TM-30 hue bins.

use crate::sensor_report::{esc, image, table};
use eulumdat::atla::{
    ColourQuality, ColourQualityLabels, SpectralDistribution, Tm30Reference, Tm30Theme,
};

/// The chapter as Typst source (level-1 heading "Colour quality"), or `None`
/// when the spectrum does not cover 400–700 nm.
pub fn colour_quality_chapter(spd: &SpectralDistribution, name: &str) -> Option<String> {
    let q = ColourQuality::from_spd(spd)?;
    let labels = ColourQualityLabels {
        title: String::new(),
        ..Default::default()
    };
    let hues = labels.hues.clone();
    let card = q.card_svg(&labels, &Tm30Theme::light());
    let tm = &q.tm30;

    let mut s = String::from("= Colour quality\n");
    if !name.trim().is_empty() {
        s.push_str(&format!(
            "#text(size: 9pt, fill: gray)[{}]\n",
            esc(name.trim())
        ));
    }
    s.push_str(&image(&card, 84));

    let reference = match tm.reference {
        Tm30Reference::Planckian => format!("Planckian radiator at {:.0} K", tm.cct),
        Tm30Reference::Daylight => format!("CIE daylight at {:.0} K", tm.cct),
        Tm30Reference::Blend { daylight_share } => format!(
            "blend of Planckian radiator and CIE daylight at {:.0} K ({:.0} % daylight)",
            tm.cct,
            daylight_share * 100.0
        ),
    };

    if let Some(c) = &q.cri {
        s.push_str("\n== Colour rendering index (CIE 13.3)\n");
        let first: Vec<String> = c.ri[..8].iter().map(|r| format!("{r:.1}")).collect();
        let mut row1 = first.clone();
        row1.push(format!("{:.1}", c.ra));
        s.push_str(&table(
            "9",
            &["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "Ra"],
            &[row1],
        ));
        let mut row2: Vec<String> = c.ri[8..].iter().map(|r| format!("{r:.1}")).collect();
        row2.push(format!("{:.1}", c.re));
        s.push_str("#v(4pt)\n");
        s.push_str(&table(
            "8",
            &["R9", "R10", "R11", "R12", "R13", "R14", "R15", "Re"],
            &[row2],
        ));
        let reference_cri = if c.daylight_reference {
            format!("CIE daylight at {:.0} K", c.cct)
        } else {
            format!("Planckian radiator at {:.0} K", c.cct)
        };
        s.push_str(&format!(
            "#text(size: 9pt)[Reference: {}. Ra = mean of R1–R8, Re = mean of R1–R15. R9 = strong red, R13 / R15 = skin tones.]\n",
            esc(&reference_cri)
        ));
        if !c.within_duv_limit {
            s.push_str(&format!(
                "#text(size: 9pt, fill: rgb(\"#b45309\"))[|Duv| = {:.4} is above the CIE 13.3 limit of 0.0054: the CRI is not defined for this source and is shown for information only.]\n",
                c.duv.abs()
            ));
        }
    }

    s.push_str("\n== TM-30 hue bins\n");
    let rows: Vec<Vec<String>> = (0..16)
        .map(|j| {
            vec![
                format!("{}", j + 1),
                hues[j].clone(),
                format!("{:.1}", tm.rf_hue[j]),
                format!("{:+.1} %", tm.rcs_hue[j]),
                format!("{:+.3}", tm.rhs_hue[j]),
            ]
        })
        .collect();
    s.push_str(&table(
        "5",
        &["Bin", "Hue", "Rf,h", "Rcs,h", "Rhs,h (rad)"],
        &rows,
    ));
    s.push_str(&format!(
        "#text(size: 9pt)[Rf = {:.1}, Rg = {:.1}, CCT = {:.0} K, Duv = {:+.4}. Reference: {}.]\n",
        tm.rf,
        tm.rg,
        tm.cct,
        tm.duv,
        esc(&reference)
    ));

    s.push_str(
        "\n== How to read this\n\
- *Rf* (fidelity, 0–100): how closely 99 colour samples look like under the reference light.\n\
- *Rg* (gamut): average saturation against the reference; 100 = same, above = more saturated.\n\
- *Vector graphic*: dashed circle = reference. Where the line runs outside, colours of that hue look more saturated; inside, duller. Rg is an average, the shape shows which hues are pushed and pulled.\n\
- *Rcs,h* / *Rhs,h*: chroma and hue shift of each of the 16 hue bins.\n\
- *Duv*: distance of the light colour from the black-body line; negative = towards magenta, positive = towards green.\n\
- *Ra*, *R9*: CIE 13.3 colour rendering; R9 (strong red) is not part of Ra and is often the weak point of white LEDs.\n\
- Calculated from the spectrum: ANSI/IES TM-30-20 (fidelity = CIE 224:2017) and CIE 13.3.\n",
    );
    Some(s)
}

/// Appends the chapter to a report (after a weak page break); returns the
/// report unchanged when the spectrum cannot be evaluated.
pub fn append_colour_quality_chapter(
    report: &str,
    spd: &SpectralDistribution,
    name: &str,
) -> String {
    match colour_quality_chapter(spd, name) {
        Some(ch) => format!("{report}\n#pagebreak(weak: true)\n{ch}"),
        None => report.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eulumdat::atla::SpectralUnits;

    fn warm_led() -> SpectralDistribution {
        let wavelengths: Vec<f64> = (380..=780).step_by(5).map(f64::from).collect();
        let values = wavelengths
            .iter()
            .map(|&wl| {
                0.55 * (-((wl - 450.0) / 11.0).powi(2)).exp()
                    + (-((wl - 600.0) / 70.0).powi(2)).exp()
            })
            .collect();
        SpectralDistribution {
            wavelengths,
            values,
            units: SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        }
    }

    #[test]
    fn chapter_has_card_tables_and_notes() {
        let ch = colour_quality_chapter(&warm_led(), "Warm LED").unwrap();
        for needle in [
            "= Colour quality",
            "#image(bytes(",
            "== Colour rendering index (CIE 13.3)",
            "== TM-30 hue bins",
            "Rhs,h (rad)",
            "== How to read this",
        ] {
            assert!(ch.contains(needle), "{needle}");
        }
        let combined = append_colour_quality_chapter("= Photometric report\n", &warm_led(), "x");
        assert!(combined.contains("#pagebreak(weak: true)\n= Colour quality"));
    }

    /// `COLOUR_CHAPTER_OUT=x.typ cargo test -p eulumdat-typst write_chapter`
    /// writes a standalone file for `typst compile`.
    #[test]
    fn write_chapter() {
        let Ok(out) = std::env::var("COLOUR_CHAPTER_OUT") else {
            return;
        };
        let ch = colour_quality_chapter(&warm_led(), "Warm LED (synthetic)").unwrap();
        std::fs::write(
            out,
            format!("#set page(paper: \"a4\")\n#set text(size: 10pt)\n{ch}"),
        )
        .unwrap();
    }
}
