//! Colour-quality card: TM-30 colour vector graphic, Duv bar and the key
//! numbers (CCT, Duv, Rf / Rg, Ra, R9, Re) of one spectrum, as one SVG.
//!
//! Single averages hide hue-specific shifts (an Rg near 100 can still mean
//! duller reds and stronger yellows); the normalised vector graphic shows
//! them, and the card labels the largest gains and losses.

use super::cri::{calculate_cri, CriResult, CRI_DUV_LIMIT};
use super::tm30::{calculate_tm30, cvg_body, Tm30Result, Tm30Theme, NUM_BINS, TM30_BIN_COLOURS};
use crate::atla::types::SpectralDistribution;

/// TM-30 + CRI of one spectrum.
#[derive(Debug, Clone)]
pub struct ColourQuality {
    pub tm30: Tm30Result,
    /// CIE 13.3 CRI (None if it cannot be computed).
    pub cri: Option<CriResult>,
}

/// Texts of the card (English defaults; the web app passes translations).
#[derive(Debug, Clone)]
pub struct ColourQualityLabels {
    /// Heading; empty = none.
    pub title: String,
    /// Second line, e.g. the luminaire or lamp name; empty = none.
    pub subtitle: String,
    pub cvg_title: String,
    pub cvg_text: String,
    /// Note under the explanation; `{rg}` is replaced by Rg.
    pub cvg_note: String,
    pub duv_title: String,
    pub magenta: String,
    pub green: String,
    pub black_body_line: String,
    pub measured: String,
    pub cct: String,
    pub duv: String,
    pub rf_rg: String,
    pub ra: String,
    pub r9: String,
    pub re: String,
    /// Shown when |Duv| exceeds the CIE 13.3 limit (`{limit}` = 0.0054).
    pub cri_limit: String,
    /// Method line at the bottom.
    pub method: String,
    /// Hue names of the 16 TM-30 bins.
    pub hues: [String; NUM_BINS],
}

impl Default for ColourQualityLabels {
    fn default() -> Self {
        let hues = [
            "red",
            "red-orange",
            "orange",
            "yellow-orange",
            "yellow",
            "yellow-green",
            "green",
            "green",
            "cyan",
            "cyan",
            "blue",
            "blue",
            "blue-violet",
            "violet",
            "purple",
            "pink",
        ]
        .map(String::from);
        Self {
            title: "Colour quality".into(),
            subtitle: String::new(),
            cvg_title: "TM-30 colour vector graphic.".into(),
            cvg_text: "Dashed circle = reference. Outside = more saturated, inside = duller.".into(),
            cvg_note: "Rg {rg} is an average over all hues. The shape shows where colours are pushed and pulled.".into(),
            duv_title: "Duv: distance to the black-body line".into(),
            magenta: "magenta".into(),
            green: "green".into(),
            black_body_line: "black-body line".into(),
            measured: "measured".into(),
            cct: "CCT".into(),
            duv: "Duv".into(),
            rf_rg: "TM-30 Rf / Rg".into(),
            ra: "CRI Ra".into(),
            r9: "CRI R9".into(),
            re: "CRI Re (R1–R15)".into(),
            cri_limit: "* CRI is only defined for |Duv| < {limit}.".into(),
            method: "Calculated from the spectrum: ANSI/IES TM-30-20 (CIE 224:2017) and CIE 13.3.".into(),
            hues,
        }
    }
}

impl ColourQuality {
    /// TM-30 and CRI of a spectrum; `None` if TM-30 cannot be computed
    /// (spectrum not covering 400–700 nm).
    pub fn from_spd(spd: &SpectralDistribution) -> Option<Self> {
        let tm30 = calculate_tm30(spd)?;
        let cri = calculate_cri(spd);
        Some(Self { tm30, cri })
    }

    /// Hue bins worth a label: the largest chroma shifts (|Rcs| ≥ 1.5 %),
    /// at most four, never two neighbouring bins, sorted by bin.
    pub fn callout_bins(&self) -> Vec<usize> {
        let rcs = &self.tm30.rcs_hue;
        let mut order: Vec<usize> = (0..NUM_BINS).collect();
        order.sort_by(|&a, &b| rcs[b].abs().total_cmp(&rcs[a].abs()));
        let mut picked: Vec<usize> = Vec::new();
        for j in order {
            if picked.len() == 4 || rcs[j].abs() < 1.5 {
                break;
            }
            let near = picked.iter().any(|&p| {
                let d = (p as i32 - j as i32).rem_euclid(NUM_BINS as i32);
                d <= 1 || d >= NUM_BINS as i32 - 1
            });
            if !near {
                picked.push(j);
            }
        }
        picked.sort_unstable();
        picked
    }

    /// The card as SVG (1000 user units wide, height from the content; it
    /// scales with its viewBox).
    pub fn card_svg(&self, labels: &ColourQualityLabels, theme: &Tm30Theme) -> String {
        let tm = &self.tm30;
        let w = 1000.0;
        let dark = is_dark(&theme.background);
        let muted = &theme.reference_color;
        let font = &theme.font_family;
        let mut s = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} @H@" width="{w}" height="@H@" font-family="{font}">
<rect width="{w}" height="@H@" fill="{bg}"/>
"#,
            bg = theme.background,
        );

        // Title block.
        let mut top = 40.0;
        if !labels.title.is_empty() {
            top += 34.0;
            s.push_str(&format!(
                r#"<text x="40" y="{top}" font-size="36" font-weight="700" fill="{fg}">{t}</text>
"#,
                fg = theme.foreground,
                t = esc(&labels.title),
            ));
        }
        if !labels.subtitle.is_empty() {
            top += 30.0;
            s.push_str(&format!(
                r#"<text x="40" y="{top}" font-size="19" fill="{muted}">{t}</text>
"#,
                t = esc(&labels.subtitle),
            ));
        }

        // Vector graphic with labelled shifts.
        let radius = 158.0;
        let (cx, cy) = (335.0, top + 95.0 + radius * 1.5);
        // Labels stay left of the explanation column.
        let (label_min_x, label_max_x, tx) = (14.0, 660.0, 690.0);
        s.push_str(&cvg_body(tm, cx, cy, radius, theme, false));
        for j in self.callout_bins() {
            let a = (22.5 * j as f64 + 11.25).to_radians();
            let r = radius * 1.5 + 22.0;
            let (mut x, y) = (cx + r * a.cos(), cy - r * a.sin());
            let name = &labels.hues[j];
            let value = signed_percent(tm.rcs_hue[j]);
            // Rough text width: ~0.55 em per character.
            let est_w = (text_em(name) * 18.0).max(text_em(&value) * 24.0 * 1.1);
            let anchor = if a.cos() > 0.25 {
                x = x.min(label_max_x - est_w);
                "start"
            } else if a.cos() < -0.25 {
                x = x.max(label_min_x + est_w);
                "end"
            } else {
                x = x.clamp(label_min_x + est_w / 2.0, label_max_x - est_w / 2.0);
                "middle"
            };
            // Labels above the centre sit higher, below it lower.
            let (y1, y2) = if a.sin() >= 0.0 {
                (y - 22.0, y + 2.0)
            } else {
                (y + 10.0, y + 34.0)
            };
            let colour = shade(TM30_BIN_COLOURS[j], dark);
            s.push_str(&format!(
                r#"<text x="{x:.1}" y="{y1:.1}" font-size="18" fill="{colour}" text-anchor="{anchor}">{n}</text>
<text x="{x:.1}" y="{y2:.1}" font-size="24" font-weight="700" fill="{colour}" text-anchor="{anchor}">{v}</text>
"#,
                n = esc(name),
                v = esc(&value),
            ));
        }

        // Explanation to the right: title (bold), text, note (muted).
        let mut ty = cy - 120.0;
        for (text, weight, colour) in [
            (labels.cvg_title.as_str(), "700", theme.foreground.as_str()),
            (labels.cvg_text.as_str(), "400", theme.foreground.as_str()),
        ] {
            for line in wrap(text, 25) {
                s.push_str(&format!(
                    r#"<text x="{tx}" y="{ty:.1}" font-size="20" font-weight="{weight}" fill="{colour}">{t}</text>
"#,
                    t = esc(&line),
                ));
                ty += 28.0;
            }
        }
        ty += 18.0;
        let note = labels.cvg_note.replace("{rg}", &format!("{:.1}", tm.rg));
        for line in wrap(&note, 25) {
            s.push_str(&format!(
                r#"<text x="{tx}" y="{ty:.1}" font-size="20" fill="{muted}">{t}</text>
"#,
                t = esc(&line),
            ));
            ty += 28.0;
        }

        // Duv bar: ±0.006, wider (in 0.002 steps) when the source lies further out.
        // Room for labels below the graphic.
        let by = cy + radius * 1.5 + 112.0;
        let (bx, bw, bh) = (40.0, 920.0, 22.0);
        let range = if tm.duv.abs() * 1.15 > 0.006 {
            (tm.duv.abs() * 1.15 / 0.002).ceil() * 0.002
        } else {
            0.006
        };
        let (mag, neutral, grn) = if dark {
            ("#b05a96", "#4b4f57", "#6a9c4f")
        } else {
            ("#e7a6d2", "#f4eee4", "#a5cf86")
        };
        s.push_str(&format!(
            r#"<defs><linearGradient id="cq-duv-grad" x1="0" x2="1" y1="0" y2="0"><stop offset="0" stop-color="{mag}"/><stop offset="0.5" stop-color="{neutral}"/><stop offset="1" stop-color="{grn}"/></linearGradient></defs>
<text x="{bx}" y="{t:.1}" font-size="19" font-weight="600" fill="{fg}">{title}</text>
<rect x="{bx}" y="{by:.1}" width="{bw}" height="{bh}" rx="{r}" fill="url(#cq-duv-grad)"/>
<line x1="{mx:.1}" y1="{l0:.1}" x2="{mx:.1}" y2="{l1:.1}" stroke="{muted}" stroke-width="1.5"/>
"#,
            t = by - 12.0,
            fg = theme.foreground,
            title = esc(&labels.duv_title),
            r = bh / 2.0,
            mx = bx + bw / 2.0,
            l0 = by - 4.0,
            l1 = by + bh + 4.0,
        ));
        let duv = tm.duv;
        let pos = bx + bw * (0.5 + (duv / range).clamp(-1.0, 1.0) / 2.0);
        s.push_str(&format!(
            r#"<line x1="{pos:.1}" y1="{l0:.1}" x2="{pos:.1}" y2="{l1:.1}" stroke="{fg}" stroke-width="4"/>
<text x="{bx}" y="{ly:.1}" font-size="17" fill="{muted}">−{r:.3} {m}</text>
<text x="{mx:.1}" y="{ly:.1}" font-size="17" fill="{muted}" text-anchor="middle">{bb} · {meas} {d}</text>
<text x="{rx:.1}" y="{ly:.1}" font-size="17" fill="{muted}" text-anchor="end">{g} +{r:.3}</text>
"#,
            l0 = by - 8.0,
            l1 = by + bh + 8.0,
            fg = theme.foreground,
            ly = by + bh + 32.0,
            m = esc(&labels.magenta),
            mx = bx + bw / 2.0,
            bb = esc(&labels.black_body_line),
            meas = esc(&labels.measured),
            d = signed(duv, 4),
            rx = bx + bw,
            g = esc(&labels.green),
            r = range,
        ));

        // Tiles.
        let cri = self.cri.as_ref();
        let star = if cri.is_some_and(|c| !c.within_duv_limit) {
            "*"
        } else {
            ""
        };
        let cri_val = |f: &dyn Fn(&CriResult) -> f64| {
            cri.map(|c| format!("{:.1}{star}", f(c)))
                .unwrap_or_else(|| "—".into())
        };
        let tiles = [
            (labels.cct.as_str(), format!("{:.0} K", tm.cct)),
            (labels.duv.as_str(), signed(duv, 4)),
            (
                labels.rf_rg.as_str(),
                format!("{:.1} / {:.1}", tm.rf, tm.rg),
            ),
            (labels.ra.as_str(), cri_val(&|c| c.ra)),
            (labels.r9.as_str(), cri_val(&|c| c.r9())),
            (labels.re.as_str(), cri_val(&|c| c.re)),
        ];
        let ty0 = by + bh + 70.0;
        let col_w = 290.0;
        for (k, (label, value)) in tiles.iter().enumerate() {
            let x = 40.0 + (k % 3) as f64 * (col_w + 25.0);
            let y = ty0 + (k / 3) as f64 * 100.0;
            s.push_str(&format!(
                r#"<line x1="{x}" y1="{y:.1}" x2="{x2}" y2="{y:.1}" stroke="{fg}" stroke-width="2.5"/>
<text x="{x}" y="{ly:.1}" font-size="17" fill="{muted}">{l}</text>
<text x="{x}" y="{vy:.1}" font-size="38" font-weight="700" fill="{fg}">{v}</text>
"#,
                x2 = x + col_w,
                fg = theme.foreground,
                ly = y + 26.0,
                vy = y + 70.0,
                l = esc(label),
                v = esc(value),
            ));
        }

        // Footer.
        let mut fy = ty0 + 200.0 + 10.0;
        if !star.is_empty() {
            let t = labels
                .cri_limit
                .replace("{limit}", &format!("{CRI_DUV_LIMIT}"));
            s.push_str(&format!(
                r#"<text x="40" y="{fy:.1}" font-size="14" fill="{muted}">{t}</text>
"#,
                t = esc(&t),
            ));
            fy += 20.0;
        }
        s.push_str(&format!(
            r#"<text x="40" y="{fy:.1}" font-size="14" fill="{muted}">{t}</text>
</svg>"#,
            t = esc(&labels.method),
        ));
        s.replace("@H@", &format!("{:.0}", fy + 26.0))
    }
}

/// "+7 %", "−11 %" (one decimal below 10 %).
fn signed_percent(v: f64) -> String {
    let body = if v.abs() < 9.95 {
        format!("{:.1}", v.abs())
    } else {
        format!("{:.0}", v.abs())
    };
    format!("{}{} %", if v < 0.0 { "−" } else { "+" }, body)
}

/// Signed number with a typographic minus.
fn signed(v: f64, decimals: usize) -> String {
    let body = format!("{:.*}", decimals, v.abs());
    let zero = body.chars().all(|c| c == '0' || c == '.');
    let sign = if zero {
        ""
    } else if v < 0.0 {
        "−"
    } else {
        "+"
    };
    format!("{sign}{body}")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Approximate advance of a character in em (CJK and full-width forms are
/// about twice as wide as Latin text).
fn char_em(c: char) -> f64 {
    if ('\u{2E80}'..='\u{9FFF}').contains(&c)
        || ('\u{AC00}'..='\u{D7AF}').contains(&c)
        || ('\u{F900}'..='\u{FAFF}').contains(&c)
        || ('\u{FF00}'..='\u{FFEF}').contains(&c)
    {
        1.0
    } else {
        0.55
    }
}

fn text_em(s: &str) -> f64 {
    s.chars().map(char_em).sum()
}

/// Greedy wrap to `width` Latin characters (≈ 0.55 em each); words wider
/// than a line, and text without spaces (CJK), break between characters.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let max_em = width as f64 * 0.55;
    let mut lines = Vec::new();
    let mut cur = String::new();
    let push_word = |word: &str, cur: &mut String, lines: &mut Vec<String>| {
        if text_em(word) > max_em {
            // Wider than a line (e.g. CJK text without spaces): continue on
            // the current line and break between characters.
            if !cur.is_empty() {
                cur.push(' ');
            }
            for c in word.chars() {
                // CJK rule: closing punctuation never starts a line.
                let closing = "。，、；：！？）」』】〉》．，".contains(c);
                if !closing && !cur.trim().is_empty() && text_em(cur) + char_em(c) > max_em {
                    lines.push(std::mem::take(cur).trim_end().to_string());
                }
                cur.push(c);
            }
            return;
        }
        let sep = if cur.is_empty() { 0.0 } else { 0.55 };
        if !cur.is_empty() && text_em(cur) + sep + text_em(word) > max_em {
            lines.push(std::mem::take(cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    };
    for word in text.split_whitespace() {
        push_word(word, &mut cur, &mut lines);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn hex_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(f64::from);
    Some((p(0)?, p(2)?, p(4)?))
}

fn is_dark(bg: &str) -> bool {
    hex_rgb(bg).is_some_and(|(r, g, b)| 0.299 * r + 0.587 * g + 0.114 * b < 128.0)
}

/// Bin colour adjusted for text: darker on light backgrounds, lighter on dark.
fn shade(hex: &str, dark: bool) -> String {
    let Some((r, g, b)) = hex_rgb(hex) else {
        return hex.to_string();
    };
    let f = |c: f64| {
        let v = if dark {
            c + (255.0 - c) * 0.25
        } else {
            c * 0.72
        };
        v.round().clamp(0.0, 255.0) as u8
    };
    format!("#{:02x}{:02x}{:02x}", f(r), f(g), f(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atla::cie_tables::N;
    use crate::atla::colour_common::wavelength;

    fn led_3000k() -> SpectralDistribution {
        let wavelengths: Vec<f64> = (0..N).map(wavelength).collect();
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
            units: crate::atla::types::SpectralUnits::Relative,
            start_wavelength: None,
            wavelength_interval: None,
        }
    }

    #[test]
    fn card_has_all_parts() {
        let q = ColourQuality::from_spd(&led_3000k()).unwrap();
        let svg = q.card_svg(&ColourQualityLabels::default(), &Tm30Theme::light());
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        for needle in ["CRI R9", "TM-30 Rf / Rg", "black-body line", "cq-duv-grad"] {
            assert!(svg.contains(needle), "{needle}");
        }
        assert!(q.cri.is_some());
    }

    #[test]
    fn callouts_are_sparse_and_separated() {
        let q = ColourQuality::from_spd(&led_3000k()).unwrap();
        let bins = q.callout_bins();
        assert!(!bins.is_empty() && bins.len() <= 4, "{bins:?}");
        for (i, a) in bins.iter().enumerate() {
            for b in &bins[i + 1..] {
                let d = (*a as i32 - *b as i32).rem_euclid(16);
                assert!(d > 1 && d < 15, "{bins:?}");
            }
        }
    }

    #[test]
    fn duv_bar_widens_for_off_locus_sources() {
        let mut q = ColourQuality::from_spd(&led_3000k()).unwrap();
        q.tm30.duv = -0.008;
        let svg = q.card_svg(&ColourQualityLabels::default(), &Tm30Theme::light());
        assert!(svg.contains("−0.010 magenta") && svg.contains("green +0.010"));
        q.tm30.duv = 0.001;
        let svg = q.card_svg(&ColourQualityLabels::default(), &Tm30Theme::light());
        assert!(svg.contains("−0.006 magenta"));
    }

    #[test]
    fn number_formatting() {
        assert_eq!(signed(-0.00081, 4), "−0.0008");
        assert_eq!(signed(0.00004, 4), "0.0000");
        assert_eq!(signed_percent(7.2), "+7.2 %");
        assert_eq!(signed_percent(-11.4), "−11 %");
        assert_eq!(wrap("a bb ccc dddd", 6), vec!["a bb", "ccc", "dddd"]);
        // Text without spaces (CJK) breaks between characters.
        let zh = wrap("虚线圆为参考光源圆外更饱和圆内更暗淡", 10);
        assert!(
            zh.len() > 1 && zh.iter().all(|l| text_em(l) <= 5.5),
            "{zh:?}"
        );
        // A long CJK run continues the current line instead of starting a new one.
        let mixed = wrap("Rg 105.6 是所有色调的平均值形状显示哪些颜色被增强", 25);
        assert!(mixed[0].starts_with("Rg 105.6 是"), "{mixed:?}");
        // Closing punctuation stays at the end of the previous line.
        let punct = wrap("是所有色调的平均值。形状显示哪些颜色被增强", 18);
        assert!(punct.iter().all(|l| !l.starts_with('。')), "{punct:?}");
    }
}
