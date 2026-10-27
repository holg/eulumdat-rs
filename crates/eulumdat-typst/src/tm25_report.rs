//! Typst report for an IES TM-25 ray file: what the viewer computed, as a
//! document. Header and provenance, quality findings, far field (the
//! editor's polar, cartesian and butterfly diagrams from the exported LDT,
//! plus the relative profile and the x/y cross sections), spectrum with its
//! colorimetry, irradiance plane, luminance map, and the exports available.
//!
//! The data arrives as plain values ([`Tm25ReportData`]), filled by the
//! viewer shell from what the Bevy viewer publishes, so this module depends
//! on nothing but `eulumdat` for the diagrams. Charts are SVG strings
//! embedded inline, like the photometric report.

use eulumdat::diagram::{ButterflyDiagram, CartesianDiagram, PolarDiagram, SvgTheme};
use eulumdat::Eulumdat;

/// Far-field numbers of the file.
#[derive(Clone, Debug, Default)]
pub struct FarFieldData {
    /// `cd` or `W/sr`.
    pub unit: String,
    pub peak: f64,
    /// Polar angle of each ring centre, degrees.
    pub gamma: Vec<f64>,
    /// Azimuthal average relative to the peak, one per ring.
    pub relative: Vec<f64>,
    pub half_intensity_deg: Option<f64>,
    /// `(angle, intensity)` through the emission axis, −90° to +90°.
    pub cut_x: Vec<(f64, f64)>,
    pub cut_y: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, Default)]
pub struct SpectrumData {
    pub wavelengths: Vec<f64>,
    pub values: Vec<f64>,
    pub peak_nm: Option<f64>,
    pub ultraviolet: bool,
    /// `(CCT K, Duv, x, y)` when the spectrum is visible light.
    pub colorimetry: Option<(f64, f64, f64, f64)>,
    pub ler_lm_per_w: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct IrradianceData {
    pub z_mm: f64,
    pub unit: String,
    pub peak: f64,
    pub half_mm: f64,
    pub captured: f64,
    /// `(radius mm, value)`.
    pub radial: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, Default)]
pub struct LuminanceData {
    pub unit: String,
    pub pixel_mm: f64,
    pub cone_deg: f64,
    pub width: usize,
    pub height: usize,
    pub peak: f64,
    pub mean_lit: f64,
    pub rays_per_lit_pixel: f64,
    /// Row-major, row 0 at the bottom.
    pub values: Vec<f64>,
    /// View direction, TM-25 frame.
    pub direction: [f64; 3],
}

#[derive(Clone, Debug, Default)]
pub struct QualityData {
    pub verdict: String,
    /// `(severity, code, message)`.
    pub findings: Vec<(String, String, String)>,
    pub far_field_noise: f64,
}

/// Everything the report needs.
#[derive(Clone, Debug, Default)]
pub struct Tm25ReportData {
    pub file_name: String,
    /// The nine TM-25 description strings, in order.
    pub text: Vec<String>,
    pub date: String,
    pub rays: u64,
    pub kept: u64,
    pub columns: Vec<String>,
    pub flux_kind: String,
    pub radiant_flux_w: f64,
    pub luminous_flux_lm: f64,
    pub bounds_min: [f64; 3],
    pub bounds_max: [f64; 3],
    pub far_field: FarFieldData,
    pub spectrum: Option<SpectrumData>,
    pub irradiance: Option<IrradianceData>,
    pub luminance: Option<LuminanceData>,
    pub quality: QualityData,
    /// The exported EULUMDAT, for the editor's diagrams.
    pub ldt: Option<Eulumdat>,
    /// Lines about horticulture / UV, already formatted.
    pub notes: Vec<String>,
    /// Export formats the viewer offers for this file.
    pub exports: Vec<String>,
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('#', "\\#")
        .replace('$', "\\$")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('@', "\\@")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn svg_esc(svg: &str) -> String {
    svg.replace('\\', "\\\\").replace('"', "\\\"")
}

fn image(svg: &str, width_pct: u32) -> String {
    format!(
        "#align(center)[#image(bytes(\"{}\"), width: {width_pct}%)]\n",
        svg_esc(svg)
    )
}

/// A subsection heading kept on the same page as its figure.
fn figure(heading: &str, intro: &str, svg: &str, width_pct: u32) -> String {
    format!(
        "#block(breakable: false)[\n== {heading}\n\n{intro}{}]\n",
        image(svg, width_pct)
    )
}

fn fmt_tick(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let digits = (2 - v.abs().log10().floor() as i32).clamp(0, 6) as usize;
    let s = format!("{v:.digits$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn fmt(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else if v.abs() >= 1e5 || v.abs() < 1e-3 {
        format!("{v:.3e}")
    } else if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.4}")
    }
}

/// One chart series: points, stroke colour, legend label.
type Series<'a> = (&'a [(f64, f64)], &'a str, &'a str);

/// A line chart: `series` of `(x, y)`, axes labelled, light theme.
fn line_chart(
    series: &[Series],
    x_label: &str,
    y_label: &str,
    w: f64,
    h: f64,
    x_ticks: &[f64],
) -> String {
    let (ml, mr, mt, mb) = (52.0, 12.0, 10.0, 34.0);
    let (pw, ph) = (w - ml - mr, h - mt - mb);
    let xs: Vec<f64> = series
        .iter()
        .flat_map(|(s, _, _)| s.iter().map(|p| p.0))
        .collect();
    let ys: Vec<f64> = series
        .iter()
        .flat_map(|(s, _, _)| s.iter().map(|p| p.1))
        .collect();
    let (x0, x1) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    let y1 = ys.iter().cloned().fold(0.0, f64::max).max(1e-12);
    let sx = |x: f64| ml + (x - x0) / (x1 - x0).max(1e-9) * pw;
    let sy = |y: f64| mt + ph - (y / y1) * ph;
    let mut s = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" font-family="Helvetica, Arial, sans-serif" font-size="10">"##
    );
    s.push_str(&format!(r##"<rect x="{ml}" y="{mt}" width="{pw}" height="{ph}" fill="#fafafa" stroke="#888" stroke-width="0.6"/>"##));
    for i in 0..=4 {
        let y = sy(y1 * i as f64 / 4.0);
        s.push_str(&format!(r##"<line x1="{ml}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#ddd" stroke-width="0.5"/><text x="{:.1}" y="{:.1}" text-anchor="end" fill="#444">{}</text>"##, ml + pw, ml - 4.0, y + 3.5, fmt_tick(y1 * i as f64 / 4.0)));
    }
    for t in x_ticks {
        if *t < x0 - 1e-9 || *t > x1 + 1e-9 {
            continue;
        }
        let x = sx(*t);
        s.push_str(&format!(r##"<line x1="{x:.1}" y1="{mt}" x2="{x:.1}" y2="{:.1}" stroke="#ddd" stroke-width="0.5"/><text x="{x:.1}" y="{:.1}" text-anchor="middle" fill="#444">{}</text>"##, mt + ph, mt + ph + 13.0, fmt_tick(*t)));
    }
    for (pts, colour, label) in series {
        let d: Vec<String> = pts
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", sx(*x), sy(*y)))
            .collect();
        s.push_str(&format!(
            r##"<polyline points="{}" fill="none" stroke="{colour}" stroke-width="1.6"/>"##,
            d.join(" ")
        ));
        let _ = label;
    }
    let mut lx = ml + 6.0;
    for (_, colour, label) in series {
        if !label.is_empty() {
            s.push_str(&format!(r##"<line x1="{lx}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{colour}" stroke-width="2"/><text x="{:.1}" y="{:.1}" fill="#333">{label}</text>"##, mt + 10.0, lx + 14.0, mt + 10.0, lx + 18.0, mt + 13.5));
            lx += 18.0 + 6.5 * label.len() as f64 + 12.0;
        }
    }
    s.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" fill="#333">{x_label}</text>"##,
        ml + pw / 2.0,
        h - 4.0
    ));
    s.push_str(&format!(r##"<text transform="translate(11,{:.1}) rotate(-90)" text-anchor="middle" fill="#333">{y_label}</text>"##, mt + ph / 2.0));
    s.push_str("</svg>");
    s
}

fn ramp(t: f64) -> String {
    let stops = [
        [0.0, 0.0, 255.0],
        [0.0, 255.0, 255.0],
        [0.0, 255.0, 0.0],
        [255.0, 255.0, 0.0],
        [255.0, 0.0, 0.0],
    ];
    let t = t.clamp(0.0, 1.0) * 4.0;
    let k = (t.floor() as usize).min(3);
    let f = t - k as f64;
    let c: Vec<u8> = (0..3)
        .map(|i| (stops[k][i] + (stops[k + 1][i] - stops[k][i]) * f) as u8)
        .collect();
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn heatmap(l: &LuminanceData) -> String {
    let (w, h) = (l.width.max(1), l.height.max(1));
    let cell = (360.0 / w.max(h) as f64).min(12.0);
    let (pw, ph) = (cell * w as f64, cell * h as f64);
    let (ml, mt) = (40.0, 8.0);
    let total_w = ml + pw + 70.0;
    let total_h = mt + ph + 30.0;
    let mut s = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total_w:.0} {total_h:.0}" font-family="Helvetica, Arial, sans-serif" font-size="9" shape-rendering="crispEdges">"##
    );
    s.push_str(&format!(
        r##"<rect x="{ml}" y="{mt}" width="{pw:.1}" height="{ph:.1}" fill="#000"/>"##
    ));
    for y in 0..h {
        for x in 0..w {
            let v = l.values.get(y * w + x).copied().unwrap_or(0.0);
            if v <= 0.0 || l.peak <= 0.0 {
                continue;
            }
            s.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{cell:.2}" height="{cell:.2}" fill="{}"/>"##,
                ml + x as f64 * cell,
                mt + (h - 1 - y) as f64 * cell,
                ramp(v / l.peak)
            ));
        }
    }
    // Colour bar.
    let bx = ml + pw + 14.0;
    for i in 0..40 {
        let t = 1.0 - i as f64 / 39.0;
        s.push_str(&format!(
            r##"<rect x="{bx:.1}" y="{:.1}" width="10" height="{:.2}" fill="{}"/>"##,
            mt + i as f64 * ph / 40.0,
            ph / 40.0 + 0.5,
            ramp(t)
        ));
    }
    s.push_str(&format!(r##"<text x="{:.1}" y="{:.1}" fill="#333">{}</text><text x="{:.1}" y="{:.1}" fill="#333">0</text>"##, bx + 14.0, mt + 8.0, fmt(l.peak), bx + 14.0, mt + ph));
    s.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" fill="#333">{:.2} mm</text>"##,
        ml + pw / 2.0,
        mt + ph + 14.0,
        w as f64 * l.pixel_mm
    ));
    s.push_str(&format!(r##"<text transform="translate(12,{:.1}) rotate(-90)" text-anchor="middle" fill="#333">{:.2} mm</text>"##, mt + ph / 2.0, h as f64 * l.pixel_mm));
    s.push_str("</svg>");
    s
}

/// The whole Typst source.
pub fn generate_tm25_report(d: &Tm25ReportData) -> String {
    let t = |i: usize| esc(d.text.get(i).map(|s| s.trim()).unwrap_or(""));
    let name = if t(0).is_empty() {
        esc(&d.file_name)
    } else {
        t(0)
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let flux_line = if d.luminous_flux_lm > 0.0 && d.radiant_flux_w > 0.0 {
        format!(
            "{:.3} lm / {:.4} W ({:.1} lm/W)",
            d.luminous_flux_lm,
            d.radiant_flux_w,
            d.luminous_flux_lm / d.radiant_flux_w
        )
    } else if d.luminous_flux_lm > 0.0 {
        format!("{:.3} lm", d.luminous_flux_lm)
    } else {
        format!("{:.4} W", d.radiant_flux_w)
    };
    let mut s = String::with_capacity(64 * 1024);
    s.push_str(&format!(r##"// TM-25 ray file report, generated by eulumdat-typst
#set document(title: "Ray file report: {name}", author: "tm25.eu")
#set page(paper: "a4", margin: (x: 2cm, y: 2.2cm),
  header: context {{ if counter(page).get().first() > 1 [#set text(size: 9pt, fill: gray); Ray file report · {name} #h(1fr) #counter(page).display()] }},
  footer: context {{ if counter(page).get().first() > 1 [#set text(size: 8pt, fill: gray); #h(1fr) Generated with tm25ray and eulumdat-rs · tm25.eu] }})
#set text(size: 10pt)
#set heading(numbering: "1.")
#show heading.where(level: 1): it => {{ v(0.6em); text(size: 15pt, weight: "bold")[#it]; v(0.3em) }}
#show heading.where(level: 2): it => {{ v(0.4em); text(size: 12pt, weight: "bold")[#it]; v(0.2em) }}

#align(center)[
  #v(2.5cm)
  #text(size: 11pt, fill: gray, tracking: 0.2em)[IES TM-25 RAY FILE]
  #v(0.4cm)
  #text(size: 26pt, weight: "bold")[{name}]
  #v(0.5cm)
  #text(size: 14pt, fill: gray)[{manufacturer}]
  #v(0.3cm)
  #text(size: 11pt)[{operation}]
  #v(2cm)
  #line(length: 60%, stroke: 0.5pt + gray)
  #v(0.8cm)
  #text(size: 11pt)[
    File: {file} \
    {rays} rays · {flux} \
    Quality: *{verdict}* \
    Report date: {today}
  ]
  #v(1fr)
  #text(size: 9pt, fill: gray)[Generated with tm25ray and eulumdat-rs \ https://tm25.eu]
]
#pagebreak()

= Header

#table(columns: (auto, 1fr), stroke: 0.4pt + gray, inset: 5pt,
  [*Name of light source*], [{n0}],
  [*Manufacturer*], [{n1}],
  [*Creator of optical model*], [{n2}],
  [*Creator of the ray file*], [{n3}],
  [*Equipment / software*], [{n4}],
  [*Camera*], [{n5}],
  [*Operation*], [{n6}],
  [*Additional information*], [{n7}],
  [*Geometry reference*], [{n8}],
  [*Creation date*], [{date}],
  [*Rays*], [{rays} in the file, {kept} kept for the analysis],
  [*Columns*], [{columns}],
  [*Flux*], [{flux} ({kind})],
  [*Start points*], [x {bx0} to {bx1} mm, y {by0} to {by1} mm, z {bz0} to {bz1} mm],
)

"##,
        name = name, manufacturer = t(1), operation = t(6), file = esc(&d.file_name), rays = d.rays, flux = esc(&flux_line), verdict = esc(&d.quality.verdict), today = today,
        n0 = t(0), n1 = t(1), n2 = t(2), n3 = t(3), n4 = t(4), n5 = t(5), n6 = t(6), n7 = t(7), n8 = t(8), date = esc(&d.date), kept = d.kept,
        columns = esc(&d.columns.join(", ")), kind = esc(&d.flux_kind),
        bx0 = fmt(d.bounds_min[0]), bx1 = fmt(d.bounds_max[0]), by0 = fmt(d.bounds_min[1]), by1 = fmt(d.bounds_max[1]), bz0 = fmt(d.bounds_min[2]), bz1 = fmt(d.bounds_max[2]),
    ));

    // Quality.
    s.push_str("= Quality\n\n");
    s.push_str(&format!("Verdict: *{}*. Expected Monte-Carlo noise of a 10° × 5° far-field bin at γ = 30°: {:.1} %.\n\n", esc(&d.quality.verdict), d.quality.far_field_noise * 100.0));
    if d.quality.findings.is_empty() {
        s.push_str("No findings.\n\n");
    } else {
        s.push_str("#table(columns: (auto, auto, 1fr), stroke: 0.4pt + gray, inset: 5pt, [*Severity*], [*Code*], [*Finding*],\n");
        for (sev, code, msg) in &d.quality.findings {
            let colour = match sev.as_str() {
                "error" => "red",
                "warning" => "orange",
                _ => "gray",
            };
            s.push_str(&format!(
                "  [#text(fill: {colour})[{}]], [`{}`], [{}],\n",
                esc(sev),
                code,
                esc(msg)
            ));
        }
        s.push_str(")\n\n");
    }

    // Far field.
    let ff = &d.far_field;
    s.push_str("#pagebreak()\n= Far field\n\n");
    s.push_str(&format!("Every ray binned into a 10° × 5° C/γ grid with solid-angle normalisation. Peak {} {}{}.\n\n",
        fmt(ff.peak), esc(&ff.unit), ff.half_intensity_deg.map(|h| format!(", half intensity at {h:.0}°")).unwrap_or_default()));
    if let Some(ldt) = &d.ldt {
        let theme = SvgTheme::light();
        s.push_str(&figure(
            "Polar diagram",
            "",
            &PolarDiagram::from_eulumdat(ldt).to_svg(400.0, 400.0, &theme),
            62,
        ));
        s.push_str(&figure(
            "Cartesian diagram",
            "",
            &CartesianDiagram::from_eulumdat(ldt, 500.0, 300.0, 4).to_svg(500.0, 300.0, &theme),
            85,
        ));
        s.push_str(&figure(
            "Butterfly diagram",
            "",
            &ButterflyDiagram::from_eulumdat(ldt, 450.0, 350.0, 60.0).to_svg(450.0, 350.0, &theme),
            70,
        ));
    }
    if !ff.gamma.is_empty() {
        let prof: Vec<(f64, f64)> = ff
            .gamma
            .iter()
            .zip(&ff.relative)
            .map(|(g, r)| (*g, *r))
            .collect();
        s.push_str(&figure(
            "Azimuthal average",
            "",
            &line_chart(
                &[(&prof, "#1f6feb", "")],
                "γ (°)",
                "relative intensity",
                520.0,
                220.0,
                &[0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0],
            ),
            85,
        ));
    }
    if !ff.cut_x.is_empty() {
        s.push_str(&figure(
            "Cross sections",
            &format!(
                "Absolute intensity through the emission axis, {}.\n\n",
                esc(&ff.unit)
            ),
            &line_chart(
                &[
                    (&ff.cut_x, "#e8590c", "x cut"),
                    (&ff.cut_y, "#1098ad", "y cut"),
                ],
                "angle (°)",
                &esc(&ff.unit),
                520.0,
                240.0,
                &[-90.0, -60.0, -30.0, 0.0, 30.0, 60.0, 90.0],
            ),
            85,
        ));
    }

    // Spectrum.
    if let Some(sp) = &d.spectrum {
        s.push_str("\n#pagebreak()\n#block(breakable: false)[\n= Spectrum\n\n");
        let mut line = String::new();
        if let Some(p) = sp.peak_nm {
            line.push_str(&format!("Peak at {p:.0} nm. "));
        }
        if let Some((cct, duv, x, y)) = sp.colorimetry {
            line.push_str(&format!(
                "CCT {cct:.0} K, Duv {duv:+.4}, chromaticity x {x:.4} y {y:.4}. "
            ));
        }
        if let Some(ler) = sp.ler_lm_per_w {
            line.push_str(&format!("Luminous efficacy of radiation {ler:.1} lm/W. "));
        }
        if sp.ultraviolet {
            line.push_str("Ultraviolet source: shown in false colour in the viewer. ");
        }
        s.push_str(&format!("{}\n\n", esc(&line)));
        let pts: Vec<(f64, f64)> = sp
            .wavelengths
            .iter()
            .zip(&sp.values)
            .map(|(w, v)| (*w, *v))
            .collect();
        let lo = sp.wavelengths.first().copied().unwrap_or(380.0);
        let hi = sp.wavelengths.last().copied().unwrap_or(780.0);
        let ticks: Vec<f64> = (0..=8)
            .map(|i| ((lo + (hi - lo) * i as f64 / 8.0) / 10.0).round() * 10.0)
            .collect();
        s.push_str(&image(
            &line_chart(
                &[(&pts, "#7048e8", "")],
                "wavelength (nm)",
                "relative",
                520.0,
                230.0,
                &ticks,
            ),
            85,
        ));
        s.push_str("]\n");
    }

    // Irradiance.
    if let Some(ir) = &d.irradiance {
        s.push_str(&format!("\n#block(breakable: false)[\n= Irradiance plane\n\nTarget plane at z = {} mm, {}: peak {} {}, {:.0} % of the flux inside ±{:.1} mm.\n\n",
            fmt(ir.z_mm), esc(&ir.unit), fmt(ir.peak), esc(&ir.unit), ir.captured * 100.0, ir.half_mm));
        if !ir.radial.is_empty() {
            let rmax = ir.radial.last().map(|p| p.0).unwrap_or(1.0);
            let ticks: Vec<f64> = (0..=5).map(|i| rmax * i as f64 / 5.0).collect();
            s.push_str(&image(
                &line_chart(
                    &[(&ir.radial, "#f59f00", "")],
                    "radius (mm)",
                    &esc(&ir.unit),
                    520.0,
                    220.0,
                    &ticks,
                ),
                85,
            ));
        }
        s.push_str("]\n");
    }

    // Luminance.
    if let Some(l) = &d.luminance {
        s.push_str(&format!("\n#block(breakable: false)[\n= Luminance map\n\nView direction ({:.3}, {:.3}, {:.3}) in the TM-25 frame, acceptance cone ±{:.1}°, pixels of {:.3} mm: {} × {} px, {:.2} × {:.2} mm. Peak {} {}, mean over lit pixels {} {}, {:.0} rays per lit pixel (about {:.0} % noise per pixel).\n\n",
            l.direction[0], l.direction[1], l.direction[2], l.cone_deg, l.pixel_mm, l.width, l.height, l.width as f64 * l.pixel_mm, l.height as f64 * l.pixel_mm,
            fmt(l.peak), esc(&l.unit), fmt(l.mean_lit), esc(&l.unit), l.rays_per_lit_pixel, 100.0 / l.rays_per_lit_pixel.max(1e-9).sqrt()));
        s.push_str(&image(&heatmap(l), 70));
        s.push_str("]\n");
    }

    if !d.notes.is_empty() {
        s.push_str("\n= Application notes\n\n");
        for n in &d.notes {
            s.push_str(&format!("- {}\n", esc(n)));
        }
    }
    if !d.exports.is_empty() {
        s.push_str("\n= Exports\n\nAvailable from the viewer for this file: ");
        s.push_str(&esc(&d.exports.join(", ")));
        s.push_str(".\n");
    }
    s.push_str("\n#v(1em)\n#text(size: 8pt, fill: gray)[Method: tm25ray (https://crates.io/crates/tm25ray); far field by solid-angle binning, luminance per docs/luminance.md, quality checks per tm25ray::quality.]\n");
    s
}

/// Assemble the report data from what the TM-25 viewer publishes: the
/// status JSON (header, far field, spectrum, irradiance, quality), the
/// exports (for the LDT the diagrams are drawn from) and the luminance map.
#[cfg(feature = "json")]
pub fn from_json(
    st: &serde_json::Value,
    ex: Option<&serde_json::Value>,
    lum: Option<&serde_json::Value>,
    file_name: &str,
) -> Tm25ReportData {
    let f = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
    let arr = |v: &serde_json::Value, k: &str| -> Vec<f64> {
        v.get(k)
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
            .unwrap_or_default()
    };
    let pairs = |v: &serde_json::Value, k: &str| -> Vec<(f64, f64)> {
        v.get(k)
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
                    .collect()
            })
            .unwrap_or_default()
    };
    let ff = st.get("far_field").cloned().unwrap_or_default();
    let unit = ff
        .get("unit")
        .and_then(|v| v.as_str())
        .map(|u| if u == "Candela" { "cd" } else { "W/sr" })
        .unwrap_or("")
        .to_string();
    let spectrum = st.get("spectrum").filter(|v| !v.is_null()).map(|sp| {
        let w = arr(sp, "wavelengths");
        let v = arr(sp, "values");
        let uv = sp.get("uv").and_then(|x| x.as_bool()).unwrap_or(false);
        let spd = eulumdat_spectrum::Spd::new(&w, &v);
        let c = eulumdat_spectrum::colorimetry::analyze(&spd);
        let m = eulumdat_spectrum::SpectralMetrics::from_spd(&spd);
        SpectrumData {
            peak_nm: sp
                .get("peak")
                .and_then(|p| p.get(0))
                .and_then(|x| x.as_f64()),
            ultraviolet: uv,
            colorimetry: (!uv && w.len() > 1).then_some((c.cct_k, c.duv, c.x, c.y)),
            ler_lm_per_w: (w.len() > 1).then_some(m.ler_lm_per_w),
            wavelengths: w,
            values: v,
        }
    });
    let irradiance = st
        .get("irradiance")
        .filter(|v| !v.is_null())
        .map(|ir| IrradianceData {
            z_mm: f(ir, "z_mm"),
            unit: ir
                .get("unit")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            peak: f(ir, "peak"),
            half_mm: f(ir, "half_mm"),
            captured: f(ir, "captured"),
            radial: pairs(ir, "radial"),
        });
    let luminance = lum.map(|m| LuminanceData {
        unit: m
            .get("unit")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        pixel_mm: f(m, "pixel_mm"),
        cone_deg: f(m, "cone_deg"),
        width: f(m, "width") as usize,
        height: f(m, "height") as usize,
        peak: f(m, "peak"),
        mean_lit: f(m, "mean_lit"),
        rays_per_lit_pixel: f(m, "rays_per_lit_pixel"),
        values: arr(m, "values"),
        direction: {
            let d = arr(m, "direction");
            [
                d.first().copied().unwrap_or(0.0),
                d.get(1).copied().unwrap_or(0.0),
                d.get(2).copied().unwrap_or(1.0),
            ]
        },
    });
    let q = st.get("quality").cloned().unwrap_or_default();
    let mut notes = Vec::new();
    if let Some(h) = st.get("horticulture").filter(|v| !v.is_null()) {
        notes.push(format!(
            "Horticulture: PPF {:.2} µmol/s from the file flux, {:.2} µmol/J of this spectrum{}",
            f(h, "ppf_umol_s"),
            f(h, "ppfd_per_watt"),
            h.get("ppfd_peak_umol_m2_s")
                .and_then(|v| v.as_f64())
                .map(|p| format!(", PPFD peak on the plane {p:.1} µmol/(m²·s)"))
                .unwrap_or_default()
        ));
    }
    if let Some(u) = st.get("uv").filter(|v| !v.is_null()) {
        notes.push(format!(
            "UV exposure, {}: effective fraction {:.3} of the radiometric value{}",
            u.get("action_spectrum")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            f(u, "effective_fraction"),
            u.get("seconds_to_limit_at_peak")
                .and_then(|v| v.as_f64())
                .map(|s| format!(
                    ", the {:.0} J/m² limit is reached after {s:.0} s at the plane peak",
                    f(u, "limit_j_per_m2")
                ))
                .unwrap_or_default()
        ));
    }
    let mut exports = vec![
        "EULUMDAT .ldt".to_string(),
        "IES .ies".into(),
        "ATLA .xml".into(),
    ];
    if let Some(e) = ex {
        if e.get("spd").and_then(|v| v.as_str()).is_some() {
            exports.push("spectrum .spd".into());
        }
        if e.get("lighttools")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            exports.push("LightTools .ray".into());
        }
        if e.get("sre").and_then(|v| v.as_str()).is_some() {
            exports.push("LightTools .sre".into());
        }
        if e.get("lighttools_sampled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            exports.push("LightTools .ray with a wavelength per ray".into());
        }
    }
    exports.push("quality report .txt".into());
    let b = st.get("bounds_mm").cloned().unwrap_or_default();
    let v3 = |k: &str| {
        let a = arr(&b, k);
        [
            a.first().copied().unwrap_or(0.0),
            a.get(1).copied().unwrap_or(0.0),
            a.get(2).copied().unwrap_or(0.0),
        ]
    };
    Tm25ReportData {
        file_name: file_name.to_string(),
        text: st
            .get("text")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .map(|x| x.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default(),
        date: st
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        rays: st.get("rays").and_then(|v| v.as_u64()).unwrap_or(0),
        kept: st.get("kept").and_then(|v| v.as_u64()).unwrap_or(0),
        columns: st
            .get("columns")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .map(|x| x.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default(),
        flux_kind: st
            .get("flux_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        radiant_flux_w: f(st, "radiant_flux_w"),
        luminous_flux_lm: f(st, "luminous_flux_lm"),
        bounds_min: v3("min"),
        bounds_max: v3("max"),
        far_field: FarFieldData {
            unit,
            peak: f(&ff, "peak"),
            gamma: arr(&ff, "gamma"),
            relative: arr(&ff, "relative"),
            half_intensity_deg: ff.get("half_intensity_deg").and_then(|v| v.as_f64()),
            cut_x: pairs(&ff, "cut_x"),
            cut_y: pairs(&ff, "cut_y"),
        },
        spectrum,
        irradiance,
        luminance,
        quality: QualityData {
            verdict: q
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            findings: q
                .get("findings")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .map(|x| {
                            (
                                x.get("severity")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                x.get("code")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                x.get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            far_field_noise: f(&q, "noise"),
        },
        ldt: ex
            .and_then(|e| e.get("ldt"))
            .and_then(|v| v.as_str())
            .and_then(|t| eulumdat::Eulumdat::parse(t).ok()),
        notes,
        exports,
    }
}

/// `from_json` and the Typst source in one call, for the wasm bindings.
#[cfg(feature = "json")]
pub fn typst_from_json(
    status: &str,
    exports: &str,
    luminance: &str,
    file_name: &str,
) -> Result<String, String> {
    let st: serde_json::Value = serde_json::from_str(status).map_err(|e| format!("status: {e}"))?;
    let ex: Option<serde_json::Value> = if exports.trim().is_empty() {
        None
    } else {
        Some(serde_json::from_str(exports).map_err(|e| format!("exports: {e}"))?)
    };
    let lum: Option<serde_json::Value> = if luminance.trim().is_empty() {
        None
    } else {
        Some(serde_json::from_str(luminance).map_err(|e| format!("luminance: {e}"))?)
    };
    Ok(generate_tm25_report(&from_json(
        &st,
        ex.as_ref(),
        lum.as_ref(),
        file_name,
    )))
}
