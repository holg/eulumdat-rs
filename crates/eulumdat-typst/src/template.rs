//! Typst template generation for photometric reports.

use eulumdat::{
    bug_rating::BugDiagram,
    diagram::{ButterflyDiagram, CartesianDiagram, HeatmapDiagram, PolarDiagram, SvgTheme},
    Eulumdat, PhotometricCalculations, PhotometricComparison, PhotometricSummary, Significance,
};

use crate::generator::ReportSection;

/// Generate Typst source with inline embedded SVGs for PDF compilation.
/// Returns the complete Typst source with SVGs embedded as bytes.
/// The second element (svg_files) is kept for backwards compatibility but will be empty.
pub fn generate_typst_with_files(
    ldt: &Eulumdat,
    sections: &[ReportSection],
) -> (String, Vec<(String, String)>) {
    let mut source = String::new();
    let theme = SvgTheme::light();

    // Document setup
    source.push_str(&generate_preamble());

    // Title page
    source.push_str(&generate_title_page(ldt));

    // Table of contents
    source.push_str("\n#outline(title: \"Contents\", indent: auto)\n#pagebreak()\n\n");

    // Generate each section
    for section in sections {
        match section {
            ReportSection::Summary => {
                source.push_str(&generate_summary_section(ldt));
            }
            ReportSection::LuminaireInfo => {
                source.push_str(&generate_luminaire_info_section(ldt));
            }
            ReportSection::LampData => {
                source.push_str(&generate_lamp_data_section(ldt));
            }
            ReportSection::Dimensions => {
                source.push_str(&generate_dimensions_section(ldt));
            }
            ReportSection::PhotometricData => {
                source.push_str(&generate_photometric_data_section(ldt));
            }
            ReportSection::PolarDiagram => {
                let diagram = PolarDiagram::from_eulumdat(ldt);
                let svg = diagram.to_svg(400.0, 400.0, &theme);
                source.push_str(&generate_polar_diagram_section_inline(&svg));
            }
            ReportSection::CartesianDiagram => {
                let diagram = CartesianDiagram::from_eulumdat(ldt, 500.0, 300.0, 4);
                let svg = diagram.to_svg(500.0, 300.0, &theme);
                source.push_str(&generate_cartesian_diagram_section_inline(&svg));
            }
            ReportSection::ButterflyDiagram => {
                let diagram = ButterflyDiagram::from_eulumdat(ldt, 450.0, 350.0, 60.0);
                let svg = diagram.to_svg(450.0, 350.0, &theme);
                source.push_str(&generate_butterfly_diagram_section_inline(&svg));
            }
            ReportSection::HeatmapDiagram => {
                let diagram = HeatmapDiagram::from_eulumdat(ldt, 500.0, 300.0);
                let svg = diagram.to_svg(500.0, 300.0, &theme);
                source.push_str(&generate_heatmap_diagram_section_inline(&svg));
            }
            ReportSection::BugRating => {
                source.push_str(&generate_bug_rating_section(ldt));
            }
            ReportSection::IntensityTable => {
                source.push_str(&generate_intensity_table_section(ldt));
            }
            ReportSection::ZonalLumens => {
                source.push_str(&generate_zonal_lumens_section(ldt));
            }
            ReportSection::DirectRatios => {
                source.push_str(&generate_direct_ratios_section(ldt));
            }
            ReportSection::CuTable => {
                source.push_str(&generate_cu_table_section(ldt));
            }
            ReportSection::UgrTable => {
                source.push_str(&generate_ugr_table_section(ldt));
            }
            ReportSection::CandelaTable => {
                source.push_str(&generate_candela_table_section(ldt));
            }
        }
    }

    // Return empty svg_files since we embed inline now
    (source, Vec::new())
}

fn generate_polar_diagram_section_inline(svg_content: &str) -> String {
    // Escape the SVG for embedding in Typst
    let escaped_svg = escape_svg_for_typst(svg_content);
    format!(
        r##"
= Polar Diagram

The polar diagram shows the luminous intensity distribution in the C0-C180 and C90-C270 planes.

#align(center)[
  #image(bytes("{}"), width: 80%)
]

#v(1em)

*Legend:*
- Red curve: C0-C180 plane
- Blue curve: C90-C270 plane
- Concentric circles: Intensity levels (cd/klm)

#pagebreak()
"##,
        escaped_svg
    )
}

fn generate_cartesian_diagram_section_inline(svg_content: &str) -> String {
    let escaped_svg = escape_svg_for_typst(svg_content);
    format!(
        r##"
= Cartesian Diagram

The Cartesian diagram shows luminous intensity (cd/klm) versus gamma angle (°) for multiple C-planes.

#align(center)[
  #image(bytes("{}"), width: 90%)
]

#pagebreak()
"##,
        escaped_svg
    )
}

fn generate_butterfly_diagram_section_inline(svg_content: &str) -> String {
    let escaped_svg = escape_svg_for_typst(svg_content);
    format!(
        r##"
= 3D Butterfly Diagram

The butterfly diagram provides a three-dimensional isometric view of the light distribution across all C-planes.

#align(center)[
  #image(bytes("{}"), width: 85%)
]

#pagebreak()
"##,
        escaped_svg
    )
}

fn generate_heatmap_diagram_section_inline(svg_content: &str) -> String {
    let escaped_svg = escape_svg_for_typst(svg_content);
    format!(
        r##"
= Intensity Heatmap

The heatmap shows intensity distribution across all C-planes (horizontal axis) and gamma angles (vertical axis). Warmer colors indicate higher intensity.

#align(center)[
  #image(bytes("{}"), width: 90%)
]

#pagebreak()
"##,
        escaped_svg
    )
}

/// Escape SVG content for embedding in Typst string.
/// Handles quotes and backslashes.
fn escape_svg_for_typst(svg: &str) -> String {
    svg.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Generate the complete Typst source for a photometric report.
/// (For .typ file export - uses inline SVG placeholders, not suitable for PDF)
pub fn generate_typst_source(
    ldt: &Eulumdat,
    sections: &[ReportSection],
    include_dark_theme: bool,
) -> String {
    let mut source = String::new();

    // Document setup
    source.push_str(&generate_preamble());

    // Title page
    source.push_str(&generate_title_page(ldt));

    // Table of contents
    source.push_str("\n#outline(title: \"Contents\", indent: auto)\n#pagebreak()\n\n");

    // Generate each section
    for section in sections {
        match section {
            ReportSection::Summary => {
                source.push_str(&generate_summary_section(ldt));
            }
            ReportSection::LuminaireInfo => {
                source.push_str(&generate_luminaire_info_section(ldt));
            }
            ReportSection::LampData => {
                source.push_str(&generate_lamp_data_section(ldt));
            }
            ReportSection::Dimensions => {
                source.push_str(&generate_dimensions_section(ldt));
            }
            ReportSection::PhotometricData => {
                source.push_str(&generate_photometric_data_section(ldt));
            }
            ReportSection::PolarDiagram => {
                source.push_str(&generate_polar_diagram_section(ldt, include_dark_theme));
            }
            ReportSection::CartesianDiagram => {
                source.push_str(&generate_cartesian_diagram_section(ldt, include_dark_theme));
            }
            ReportSection::ButterflyDiagram => {
                source.push_str(&generate_butterfly_diagram_section(ldt, include_dark_theme));
            }
            ReportSection::HeatmapDiagram => {
                source.push_str(&generate_heatmap_diagram_section(ldt, include_dark_theme));
            }
            ReportSection::BugRating => {
                source.push_str(&generate_bug_rating_section(ldt));
            }
            ReportSection::IntensityTable => {
                source.push_str(&generate_intensity_table_section(ldt));
            }
            ReportSection::ZonalLumens => {
                source.push_str(&generate_zonal_lumens_section(ldt));
            }
            ReportSection::DirectRatios => {
                source.push_str(&generate_direct_ratios_section(ldt));
            }
            ReportSection::CuTable => {
                source.push_str(&generate_cu_table_section(ldt));
            }
            ReportSection::UgrTable => {
                source.push_str(&generate_ugr_table_section(ldt));
            }
            ReportSection::CandelaTable => {
                source.push_str(&generate_candela_table_section(ldt));
            }
        }
    }

    source
}

fn generate_preamble() -> String {
    r#"// Eulumdat Photometric Report
// Generated by eulumdat-typst

#set document(
  title: "Photometric Report",
  author: "Eulumdat Report Generator",
)

#set page(
  paper: "a4",
  margin: (x: 2cm, y: 2.5cm),
  header: context {
    if counter(page).get().first() > 1 [
      #set text(size: 9pt, fill: gray)
      Photometric Report
      #h(1fr)
      #counter(page).display()
    ]
  },
  footer: context {
    if counter(page).get().first() > 1 [
      #set text(size: 8pt, fill: gray)
      #h(1fr)
      Generated with Eulumdat-RS
    ]
  },
)

#set text(size: 10pt)
#set heading(numbering: "1.1")
#set par(justify: true)

// Custom styles
#let info-box(title, content) = {
  block(
    fill: luma(245),
    stroke: luma(224),
    inset: 10pt,
    radius: 4pt,
    width: 100%,
  )[
    #text(weight: "bold", size: 11pt)[#title]
    #v(4pt)
    #content
  ]
}

#let data-table(..cells) = {
  table(
    columns: 2,
    stroke: 0.5pt + luma(204),
    inset: 8pt,
    fill: (col, row) => if row == 0 { luma(240) } else { none },
    ..cells
  )
}

#let metric(label, value, unit: none) = {
  [*#label:* #value #if unit != none [#unit]]
}

"#
    .to_string()
}

fn generate_title_page(ldt: &Eulumdat) -> String {
    let luminaire_name = escape_typst(&ldt.luminaire_name);
    let manufacturer = escape_typst(&ldt.identification);
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();

    format!(
        r#"
#align(center)[
  #v(3cm)
  #text(size: 28pt, weight: "bold")[Photometric Report]
  #v(1cm)
  #text(size: 18pt)[{luminaire_name}]
  #v(0.5cm)
  #text(size: 14pt, fill: gray)[{manufacturer}]
  #v(3cm)
  #line(length: 60%, stroke: 0.5pt + gray)
  #v(1cm)
  #text(size: 11pt)[
    Report Date: {date} \
    Format: EULUMDAT (LDT)
  ]
  #v(1fr)
  #text(size: 9pt, fill: gray)[
    Generated with Eulumdat-RS \
    https://github.com/holg/eulumdat-rs
  ]
]

#pagebreak()
"#
    )
}

fn generate_summary_section(ldt: &Eulumdat) -> String {
    let summary = PhotometricSummary::from_eulumdat(ldt);
    let total_flux = summary.calculated_flux;

    format!(
        r#"
= Executive Summary

#info-box("Key Metrics")[
  #columns(2)[
    #metric("Total Luminous Flux", "{:.0}", unit: "lm")
    #metric("Light Output Ratio", "{:.1}", unit: "%")
    #metric("Luminaire Efficacy", "{:.1}", unit: "lm/W")
    #colbreak()
    #metric("Max Intensity", "{:.0}", unit: "cd/klm")
    #metric("Beam Angle (50%)", "{:.1}", unit: "°")
    #metric("Field Angle (10%)", "{:.1}", unit: "°")
  ]
]

#v(1em)

#info-box("CIE Flux Codes")[
  N1={:.0} / N2={:.0} / N3={:.0} / N4={:.0} / N5={:.0}
]

#v(1em)

#info-box("Spacing Criteria")[
  #columns(2)[
    #metric("S/H (C0-C180)", "{:.2}")
    #colbreak()
    #metric("S/H (C90-C270)", "{:.2}")
  ]
]

#pagebreak()
"#,
        total_flux,
        summary.lor,
        summary.luminaire_efficacy,
        summary.max_intensity,
        summary.beam_angle,
        summary.field_angle,
        summary.cie_flux_codes.n1,
        summary.cie_flux_codes.n2,
        summary.cie_flux_codes.n3,
        summary.cie_flux_codes.n4,
        summary.cie_flux_codes.n5,
        summary.spacing_c0,
        summary.spacing_c90,
    )
}

fn generate_luminaire_info_section(ldt: &Eulumdat) -> String {
    let symmetry_str = match ldt.symmetry {
        eulumdat::Symmetry::None => "No symmetry (full data)",
        eulumdat::Symmetry::VerticalAxis => "Vertical axis symmetry",
        eulumdat::Symmetry::PlaneC0C180 => "Symmetry about C0-C180 plane",
        eulumdat::Symmetry::PlaneC90C270 => "Symmetry about C90-C270 plane",
        eulumdat::Symmetry::BothPlanes => "Symmetry about both planes",
    };

    let type_str = match ldt.type_indicator {
        eulumdat::TypeIndicator::PointSourceSymmetric => "Point source with vertical symmetry",
        eulumdat::TypeIndicator::Linear => "Linear luminaire",
        eulumdat::TypeIndicator::PointSourceOther => "Point source with other symmetry",
    };

    format!(
        r#"
= Luminaire Information

#data-table(
  [*Property*], [*Value*],
  [Luminaire Name], [{}],
  [Identification], [{}],
  [Luminaire Number], [{}],
  [File Name], [{}],
  [Type Indicator], [{}],
  [Symmetry], [{}],
  [Number of C-Planes], [{}],
  [C-Plane Distance], [{} °],
  [Number of Gamma Angles], [{}],
  [Gamma Angle Distance], [{} °],
)

"#,
        escape_typst(&ldt.luminaire_name),
        escape_typst(&ldt.identification),
        escape_typst(&ldt.luminaire_number),
        escape_typst(&ldt.file_name),
        type_str,
        symmetry_str,
        ldt.num_c_planes,
        ldt.c_plane_distance,
        ldt.num_g_planes,
        ldt.g_plane_distance,
    )
}

fn generate_lamp_data_section(ldt: &Eulumdat) -> String {
    let mut content = String::from(
        r#"
= Lamp Data

"#,
    );

    for (i, lamp) in ldt.lamp_sets.iter().enumerate() {
        content.push_str(&format!(
            r#"
== Lamp Set {}

#data-table(
  [*Property*], [*Value*],
  [Number of Lamps], [{}],
  [Lamp Type], [{}],
  [Total Luminous Flux], [{:.0} lm],
  [Color Appearance], [{}],
  [Color Rendering Group], [{}],
  [Wattage (incl. Ballast)], [{:.1} W],
)

"#,
            i + 1,
            lamp.num_lamps,
            escape_typst(&lamp.lamp_type),
            lamp.total_luminous_flux,
            escape_typst(&lamp.color_appearance),
            escape_typst(&lamp.color_rendering_group),
            lamp.wattage_with_ballast,
        ));
    }

    content
}

fn generate_dimensions_section(ldt: &Eulumdat) -> String {
    format!(
        r#"
= Dimensions

== Luminaire Dimensions

#data-table(
  [*Dimension*], [*Value (mm)*],
  [Length], [{:.1}],
  [Width], [{:.1}],
  [Height], [{:.1}],
)

== Luminous Area

#data-table(
  [*Dimension*], [*Value (mm)*],
  [Length], [{:.1}],
  [Width], [{:.1}],
)

== Luminous Heights at C-Planes

#data-table(
  [*C-Plane*], [*Height (mm)*],
  [C0], [{:.1}],
  [C90], [{:.1}],
  [C180], [{:.1}],
  [C270], [{:.1}],
)

"#,
        ldt.length,
        ldt.width,
        ldt.height,
        ldt.luminous_area_length,
        ldt.luminous_area_width,
        ldt.height_c0,
        ldt.height_c90,
        ldt.height_c180,
        ldt.height_c270,
    )
}

fn generate_photometric_data_section(ldt: &Eulumdat) -> String {
    let summary = PhotometricSummary::from_eulumdat(ldt);

    format!(
        r#"
= Photometric Properties

== Optical Characteristics

#data-table(
  [*Property*], [*Value*],
  [Light Output Ratio (LOR)], [{:.1} %],
  [Downward LOR (DLOR)], [{:.1} %],
  [Upward LOR (ULOR)], [{:.1} %],
  [Downward Flux Fraction], [{:.1} %],
  [Conversion Factor], [{:.4}],
  [Tilt Angle], [{:.1} °],
)

== Beam Characteristics (IES Definition)

#data-table(
  [*Property*], [*Value*],
  [Maximum Intensity], [{:.0} cd/klm],
  [Beam Angle (50%)], [{:.1} °],
  [Field Angle (10%)], [{:.1} °],
)

== Efficacy

#data-table(
  [*Property*], [*Value*],
  [Total Lamp Flux], [{:.0} lm],
  [Luminaire Luminous Flux], [{:.0} lm],
  [Total Input Power], [{:.1} W],
  [Lamp Efficacy], [{:.1} lm/W],
  [Luminaire Efficacy], [{:.1} lm/W],
)

#pagebreak()
"#,
        summary.lor,
        summary.dlor,
        summary.ulor,
        ldt.downward_flux_fraction,
        ldt.conversion_factor,
        ldt.tilt_angle,
        summary.max_intensity,
        summary.beam_angle,
        summary.field_angle,
        summary.total_lamp_flux,
        summary.calculated_flux,
        summary.total_wattage,
        summary.lamp_efficacy,
        summary.luminaire_efficacy,
    )
}

fn generate_polar_diagram_section(ldt: &Eulumdat, _dark_theme: bool) -> String {
    let theme = SvgTheme::light();
    let diagram = PolarDiagram::from_eulumdat(ldt);
    let _svg = diagram.to_svg(400.0, 400.0, &theme);

    // Note: For standalone .typ export, diagrams can't be embedded without external files
    r##"
= Polar Diagram

The polar diagram shows the luminous intensity distribution in the C0-C180 and C90-C270 planes.

_Note: For PDF generation, use the CLI command `eulumdat report` which properly embeds diagrams._

#pagebreak()
"##
    .to_string()
}

fn generate_cartesian_diagram_section(_ldt: &Eulumdat, _dark_theme: bool) -> String {
    // Note: For standalone .typ export, diagrams can't be embedded without external files
    r##"
= Cartesian Diagram

The Cartesian diagram shows luminous intensity (cd/klm) versus gamma angle (°) for multiple C-planes.

_Note: For PDF generation, use the CLI command `eulumdat report` which properly embeds diagrams._

#pagebreak()
"##.to_string()
}

fn generate_butterfly_diagram_section(_ldt: &Eulumdat, _dark_theme: bool) -> String {
    // Note: For standalone .typ export, diagrams can't be embedded without external files
    r##"
= 3D Butterfly Diagram

The butterfly diagram provides a three-dimensional isometric view of the light distribution across all C-planes.

_Note: For PDF generation, use the CLI command `eulumdat report` which properly embeds diagrams._

#pagebreak()
"##.to_string()
}

fn generate_heatmap_diagram_section(_ldt: &Eulumdat, _dark_theme: bool) -> String {
    // Note: For standalone .typ export, diagrams can't be embedded without external files
    r##"
= Intensity Heatmap

The heatmap shows intensity distribution across all C-planes (horizontal axis) and gamma angles (vertical axis). Warmer colors indicate higher intensity.

_Note: For PDF generation, use the CLI command `eulumdat report` which properly embeds diagrams._

#pagebreak()
"##.to_string()
}

fn generate_bug_rating_section(ldt: &Eulumdat) -> String {
    let bug = BugDiagram::from_eulumdat(ldt);
    let total = bug.total_lumens;

    format!(
        r#"
= BUG Rating (IESNA TM-15-11)

The Backlight-Uplight-Glare (BUG) rating system classifies outdoor luminaires according to their light distribution characteristics.

#info-box("BUG Classification")[
  #align(center)[
    #text(size: 24pt, weight: "bold")[B{} U{} G{}]
  ]
]

#v(1em)

== Zone Lumens

#table(
  columns: 4,
  stroke: 0.5pt + luma(204),
  inset: 8pt,
  fill: (col, row) => if row == 0 {{ luma(240) }} else {{ none }},
  [*Zone*], [*Category*], [*Lumens*], [*Percentage*],
  [BL], [Backlight Low], [{:.1}], [{:.2}%],
  [BM], [Backlight Medium], [{:.1}], [{:.2}%],
  [BH], [Backlight High], [{:.1}], [{:.2}%],
  [BVH], [Backlight Very High], [{:.1}], [{:.2}%],
  [UL], [Uplight Low], [{:.1}], [{:.2}%],
  [UH], [Uplight High], [{:.1}], [{:.2}%],
  [FVH], [Forward Very High], [{:.1}], [{:.2}%],
  [FH], [Forward High], [{:.1}], [{:.2}%],
  [FM], [Forward Medium], [{:.1}], [{:.2}%],
  [FL], [Forward Low], [{:.1}], [{:.2}%],
)

#pagebreak()
"#,
        bug.rating.b,
        bug.rating.u,
        bug.rating.g,
        bug.zones.bl,
        if total > 0.0 {
            bug.zones.bl / total * 100.0
        } else {
            0.0
        },
        bug.zones.bm,
        if total > 0.0 {
            bug.zones.bm / total * 100.0
        } else {
            0.0
        },
        bug.zones.bh,
        if total > 0.0 {
            bug.zones.bh / total * 100.0
        } else {
            0.0
        },
        bug.zones.bvh,
        if total > 0.0 {
            bug.zones.bvh / total * 100.0
        } else {
            0.0
        },
        bug.zones.ul,
        if total > 0.0 {
            bug.zones.ul / total * 100.0
        } else {
            0.0
        },
        bug.zones.uh,
        if total > 0.0 {
            bug.zones.uh / total * 100.0
        } else {
            0.0
        },
        bug.zones.fvh,
        if total > 0.0 {
            bug.zones.fvh / total * 100.0
        } else {
            0.0
        },
        bug.zones.fh,
        if total > 0.0 {
            bug.zones.fh / total * 100.0
        } else {
            0.0
        },
        bug.zones.fm,
        if total > 0.0 {
            bug.zones.fm / total * 100.0
        } else {
            0.0
        },
        bug.zones.fl,
        if total > 0.0 {
            bug.zones.fl / total * 100.0
        } else {
            0.0
        },
    )
}

fn generate_intensity_table_section(ldt: &Eulumdat) -> String {
    let mut content = String::from(
        r#"
= Intensity Data Table

Luminous intensity values in cd/klm (candelas per kilolumen).

"#,
    );

    // Generate a compact table (show first few C-planes and gamma angles)
    let max_c = ldt.c_angles.len().min(8);
    let max_g = ldt.g_angles.len().min(10);

    content.push_str(&format!(
        "#table(\n  columns: {},\n  stroke: 0.5pt + luma(204),\n  inset: 4pt,\n  align: right,\n  fill: (col, row) => if row == 0 or col == 0 {{ luma(240) }} else {{ none }},\n",
        max_c + 1
    ));

    // Header row
    content.push_str("  [γ \\\\ C],");
    for c_idx in 0..max_c {
        content.push_str(&format!(" [{}°],", ldt.c_angles[c_idx] as i32));
    }
    content.push('\n');

    // Data rows
    for g_idx in 0..max_g {
        content.push_str(&format!("  [{}°],", ldt.g_angles[g_idx] as i32));
        for c_idx in 0..max_c {
            if c_idx < ldt.intensities.len() && g_idx < ldt.intensities[c_idx].len() {
                content.push_str(&format!(" [{:.0}],", ldt.intensities[c_idx][g_idx]));
            } else {
                content.push_str(" [-],");
            }
        }
        content.push('\n');
    }

    content.push_str(")\n\n");

    if ldt.c_angles.len() > max_c || ldt.g_angles.len() > max_g {
        content.push_str(&format!(
            "_Table truncated. Full data: {} C-planes × {} gamma angles._\n\n",
            ldt.c_angles.len(),
            ldt.g_angles.len()
        ));
    }

    content.push_str("#pagebreak()\n");
    content
}

fn generate_zonal_lumens_section(ldt: &Eulumdat) -> String {
    let summary = PhotometricSummary::from_eulumdat(ldt);
    let total = summary.zonal_lumens.downward_total() + summary.zonal_lumens.upward_total();

    format!(
        r#"
= Zonal Lumens Distribution

Distribution of luminous flux across angular zones (30° intervals).

#table(
  columns: 3,
  stroke: 0.5pt + luma(204),
  inset: 8pt,
  fill: (col, row) => if row == 0 {{ luma(240) }} else {{ none }},
  [*Zone*], [*Lumens*], [*Percentage*],
  [0° - 30°], [{:.1}], [{:.1}%],
  [30° - 60°], [{:.1}], [{:.1}%],
  [60° - 90°], [{:.1}], [{:.1}%],
  [90° - 120°], [{:.1}], [{:.1}%],
  [120° - 150°], [{:.1}], [{:.1}%],
  [150° - 180°], [{:.1}], [{:.1}%],
  [*Total*], [*{:.1}*], [*100%*],
)

"#,
        summary.zonal_lumens.zone_0_30,
        if total > 0.0 {
            summary.zonal_lumens.zone_0_30 / total * 100.0
        } else {
            0.0
        },
        summary.zonal_lumens.zone_30_60,
        if total > 0.0 {
            summary.zonal_lumens.zone_30_60 / total * 100.0
        } else {
            0.0
        },
        summary.zonal_lumens.zone_60_90,
        if total > 0.0 {
            summary.zonal_lumens.zone_60_90 / total * 100.0
        } else {
            0.0
        },
        summary.zonal_lumens.zone_90_120,
        if total > 0.0 {
            summary.zonal_lumens.zone_90_120 / total * 100.0
        } else {
            0.0
        },
        summary.zonal_lumens.zone_120_150,
        if total > 0.0 {
            summary.zonal_lumens.zone_120_150 / total * 100.0
        } else {
            0.0
        },
        summary.zonal_lumens.zone_150_180,
        if total > 0.0 {
            summary.zonal_lumens.zone_150_180 / total * 100.0
        } else {
            0.0
        },
        total,
    )
}

fn generate_direct_ratios_section(ldt: &Eulumdat) -> String {
    format!(
        r#"
= Direct Ratios (Utilization Factors)

Direct ratios for various room indices (k values).

#table(
  columns: 11,
  stroke: 0.5pt + luma(204),
  inset: 6pt,
  align: center,
  fill: (col, row) => if row == 0 {{ luma(240) }} else {{ none }},
  [*k*], [0.6], [0.8], [1.0], [1.25], [1.5], [2.0], [2.5], [3.0], [4.0], [5.0],
  [*η*], [{:.3}], [{:.3}], [{:.3}], [{:.3}], [{:.3}], [{:.3}], [{:.3}], [{:.3}], [{:.3}], [{:.3}],
)

_Note: These are direct ratios from the LDT file, representing the fraction of luminaire flux reaching the work plane for different room configurations._

"#,
        ldt.direct_ratios[0],
        ldt.direct_ratios[1],
        ldt.direct_ratios[2],
        ldt.direct_ratios[3],
        ldt.direct_ratios[4],
        ldt.direct_ratios[5],
        ldt.direct_ratios[6],
        ldt.direct_ratios[7],
        ldt.direct_ratios[8],
        ldt.direct_ratios[9],
    )
}

/// Escape special Typst characters in text.
fn escape_typst(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('$', "\\$")
        .replace('%', "\\%")
        .replace('&', "\\&")
        .replace('_', "\\_")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('*', "\\*")
        .replace('`', "\\`")
        .replace('"', "\\\"")
}

/// Generate CU table section
fn generate_cu_table_section(ldt: &Eulumdat) -> String {
    let cu = PhotometricCalculations::cu_table(ldt);

    let mut rows = String::new();

    // Generate data rows
    for (i, &rcr) in cu.rcr_values.iter().enumerate() {
        if i >= cu.values.len() {
            continue;
        }
        let row = &cu.values[i];

        // Format values grouped by ceiling reflectance
        let mut vals = String::new();
        for val in row.iter().take(18) {
            vals.push_str(&format!("[{:.0}], ", val));
        }

        // Keep trailing comma for table row
        rows.push_str(&format!("  [{}], {}\n", rcr, vals));
    }

    format!(
        r##"
= Coefficients of Utilization

The CU table shows the Coefficient of Utilization for various room cavity ratios (RCR) and surface reflectances, using the Zonal Cavity Method.

*Effective Floor Cavity Reflectance: {:.0}%*

#set text(size: 6pt)
#table(
  columns: 19,
  stroke: 0.5pt,
  inset: 2pt,
  align: center,
  fill: (col, row) => if row == 0 {{ luma(240) }} else if row == 1 {{ luma(250) }} else {{ none }},
  table.cell(rowspan: 2)[*RCR*],
  table.cell(colspan: 4)[*RC=80*],
  table.cell(colspan: 4)[*RC=70*],
  table.cell(colspan: 3)[*RC=50*],
  table.cell(colspan: 3)[*RC=30*],
  table.cell(colspan: 3)[*RC=10*],
  table.cell(colspan: 1)[*0*],
  [70], [50], [30], [10], [70], [50], [30], [10], [50], [30], [10], [50], [30], [10], [50], [30], [10], [0],
{})
#set text(size: 10pt)

_Note: Values are percentages (×100). RC = ceiling reflectance, RW (column headers) = wall reflectance._

#pagebreak()
"##,
        cu.floor_reflectance * 100.0,
        rows
    )
}

/// Generate UGR table section
fn generate_ugr_table_section(ldt: &Eulumdat) -> String {
    let ugr = PhotometricCalculations::ugr_table(ldt);

    let mut rows = String::new();

    for (i, &(x, y)) in ugr.room_sizes.iter().enumerate() {
        if i >= ugr.crosswise.len() {
            continue;
        }

        let x_str = if x == x.floor() {
            format!("{}H", x as i32)
        } else {
            format!("{:.1}H", x)
        };
        let y_str = if y == y.floor() {
            format!("{}H", y as i32)
        } else {
            format!("{:.1}H", y)
        };

        let mut row_vals = format!("  [{} × {}], ", x_str, y_str);

        // Crosswise values
        for j in 0..5.min(ugr.crosswise[i].len()) {
            row_vals.push_str(&format!("[{:.1}], ", ugr.crosswise[i][j]));
        }

        // Endwise values
        for j in 0..5.min(ugr.endwise[i].len()) {
            row_vals.push_str(&format!("[{:.1}], ", ugr.endwise[i][j]));
        }

        // Keep trailing comma for table row
        rows.push_str(&format!("{}\n", row_vals));
    }

    format!(
        r##"
= Unified Glare Rating (UGR) Table

The UGR table shows glare ratings for various room sizes and surface reflectances, following CIE 117:1995.

#set text(size: 7pt)
#table(
  columns: (auto, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr),
  stroke: 0.5pt,
  inset: 3pt,
  align: center,
  fill: (col, row) => if row == 0 {{ luma(240) }} else if row == 1 {{ luma(250) }} else {{ none }},
  table.cell(rowspan: 2)[*Room*],
  table.cell(colspan: 5)[*Crosswise*],
  table.cell(colspan: 5)[*Endwise*],
  [70/50], [70/30], [50/50], [50/30], [30/30], [70/50], [70/30], [50/50], [50/30], [30/30],
{})
#set text(size: 10pt)

*Maximum UGR = {:.1}*

_Note: Room dimensions in multiples of mounting height (H). Column headers show ceiling/wall reflectances (%)._

#pagebreak()
"##,
        rows, ugr.max_ugr
    )
}

/// Generate candela tabulation section
fn generate_candela_table_section(ldt: &Eulumdat) -> String {
    let tab = PhotometricCalculations::candela_tabulation(ldt);

    // For large tables, we'll paginate
    let entries_per_page = 60;
    let mut pages = String::new();
    let mut page_num = 0;

    let num_pages = tab.estimated_pages(entries_per_page);

    for chunk in tab.entries.chunks(entries_per_page) {
        page_num += 1;

        let mut rows = String::new();
        for entry in chunk {
            rows.push_str(&format!(
                "  [{:.1}], [{:.3}],\n",
                entry.gamma, entry.candela
            ));
        }

        let continuation = if num_pages > 1 {
            format!(" (Page {} of {})", page_num, num_pages)
        } else {
            String::new()
        };

        pages.push_str(&format!(
            r##"
= Candela Tabulation{}

#set text(size: 8pt)
#table(
  columns: (1fr, 2fr),
  stroke: 0.5pt,
  inset: 4pt,
  align: (left, right),
  fill: (col, row) => if row == 0 {{ luma(240) }} else {{ none }},
  [*Angle (°)*], [*Candela (cd)*],
{})
#set text(size: 10pt)

"##,
            continuation, rows
        ));
    }

    // Add summary at the end
    pages.push_str(&format!(
        r##"
*Maximum Candela = {:.3} cd* at Horizontal Angle = {}°, Vertical Angle = {}°

_Note: Candela values are absolute (total luminous flux = {:.0} lm)._

#pagebreak()
"##,
        tab.max_candela, tab.max_angle.0 as i32, tab.max_angle.1 as i32, tab.total_flux
    ));

    pages
}

/// Generate a Typst comparison report for two photometric files with inline SVGs.
/// Returns the complete Typst source ready for compilation to PDF.
pub fn generate_comparison_report(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    label_a: &str,
    label_b: &str,
) -> String {
    let mut source = String::new();
    let theme = SvgTheme::light();

    // Preamble + custom comparison styles
    source.push_str(&generate_comparison_preamble());

    // Title page
    source.push_str(&generate_comparison_title_page(label_a, label_b));

    // Compute comparison
    let cmp = PhotometricComparison::from_eulumdat(ldt_a, ldt_b, label_a, label_b);

    // Similarity score section
    source.push_str(&generate_similarity_section(&cmp));

    // Overlay diagrams (Polar + Cartesian)
    source.push_str(&generate_overlay_diagrams_section(
        ldt_a, ldt_b, label_a, label_b, &theme,
    ));

    // Metrics comparison table
    source.push_str(&generate_comparison_metrics_section(&cmp));

    // Side-by-side luminaire info
    source.push_str(&generate_side_by_side_info_section(
        ldt_a, ldt_b, label_a, label_b,
    ));

    // Side-by-side diagrams (Heatmap + Butterfly)
    source.push_str(&generate_side_by_side_diagrams_section(
        ldt_a, ldt_b, label_a, label_b, &theme,
    ));

    source
}

fn generate_comparison_preamble() -> String {
    let mut preamble = generate_preamble();
    // Add comparison-specific custom functions
    preamble.push_str(
        r##"
// Comparison report custom styles
#let score-badge(score) = {
  let fill = if score > 90 { rgb("#16a34a") }
    else if score > 70 { rgb("#d97706") }
    else { rgb("#dc2626") }
  align(center)[
    #block(
      fill: fill,
      stroke: none,
      inset: 12pt,
      radius: 8pt,
    )[
      #text(size: 14pt, fill: white, weight: "bold")[Similarity Score]
      #v(4pt)
      #text(size: 36pt, fill: white, weight: "bold")[#calc.round(score, digits: 1)%]
    ]
  ]
}

"##,
    );
    preamble
}

fn generate_comparison_title_page(label_a: &str, label_b: &str) -> String {
    let la = escape_typst(label_a);
    let lb = escape_typst(label_b);
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();

    format!(
        r#"
#align(center)[
  #v(3cm)
  #text(size: 28pt, weight: "bold")[Photometric Comparison Report]
  #v(1.5cm)
  #text(size: 16pt)[{la}]
  #v(0.3cm)
  #text(size: 14pt, fill: gray)[vs]
  #v(0.3cm)
  #text(size: 16pt)[{lb}]
  #v(3cm)
  #line(length: 60%, stroke: 0.5pt + gray)
  #v(1cm)
  #text(size: 11pt)[
    Report Date: {date} \
    Format: EULUMDAT (LDT)
  ]
  #v(1fr)
  #text(size: 9pt, fill: gray)[
    Generated with Eulumdat-RS \
    https://github.com/holg/eulumdat-rs
  ]
]

#pagebreak()
"#
    )
}

fn generate_similarity_section(cmp: &PhotometricComparison) -> String {
    let score = cmp.similarity_score * 100.0;
    let major_count = cmp.significant_metrics(Significance::Major).len();
    let moderate_count = cmp.significant_metrics(Significance::Moderate).len();

    format!(
        r#"
= Overall Similarity

#score-badge({score:.1})

#v(1em)

#info-box("Summary")[
  #columns(2)[
    #metric("Major Differences", "{major_count}")
    #colbreak()
    #metric("Moderate Differences", "{moderate_count}")
  ]
]

#pagebreak()
"#
    )
}

fn generate_overlay_diagrams_section(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    label_a: &str,
    label_b: &str,
    theme: &SvgTheme,
) -> String {
    let mut section = String::new();
    section.push_str("\n= Overlay Diagrams\n\n");

    // Polar overlay
    let pa = PolarDiagram::from_eulumdat(ldt_a);
    let pb = PolarDiagram::from_eulumdat(ldt_b);
    let polar_svg = PolarDiagram::to_overlay_svg(&pa, &pb, 400.0, 400.0, theme, label_a, label_b);
    let escaped_polar = escape_svg_for_typst(&polar_svg);
    section.push_str(&format!(
        r##"== Polar Diagram Overlay

Red: {} | Blue: {}

#align(center)[
  #image(bytes("{}"), width: 70%)
]

#v(1em)

"##,
        escape_typst(label_a),
        escape_typst(label_b),
        escaped_polar,
    ));

    // Cartesian overlay
    let ca = CartesianDiagram::from_eulumdat(ldt_a, 500.0, 300.0, 4);
    let cb = CartesianDiagram::from_eulumdat(ldt_b, 500.0, 300.0, 4);
    let cart_svg =
        CartesianDiagram::to_overlay_svg(&ca, &cb, 500.0, 300.0, theme, label_a, label_b);
    let escaped_cart = escape_svg_for_typst(&cart_svg);
    section.push_str(&format!(
        r##"== Cartesian Diagram Overlay

#align(center)[
  #image(bytes("{}"), width: 85%)
]

#pagebreak()
"##,
        escaped_cart,
    ));

    section
}

fn significance_fill_color(sig: &Significance) -> &'static str {
    match sig {
        Significance::Negligible => "dcfce7",
        Significance::Minor => "fef9c3",
        Significance::Moderate => "fed7aa",
        Significance::Major => "fecaca",
    }
}

fn generate_comparison_metrics_section(cmp: &PhotometricComparison) -> String {
    let mut section = String::new();
    section.push_str(
        r#"
= Metrics Comparison

#table(
  columns: (auto, 1fr, 1fr, 1fr, 1fr),
  stroke: 0.5pt + luma(204),
  inset: 6pt,
  fill: (col, row) => if row == 0 { luma(240) } else { none },
  [*Metric*], [*File A*], [*File B*], [*Delta*], [*Delta %*],
"#,
    );

    for m in &cmp.metrics {
        let unit = if m.unit.is_empty() {
            String::new()
        } else {
            format!(" {}", m.unit)
        };
        let fill_hex = significance_fill_color(&m.significance);
        // Color only the Delta cell using table.cell with fill
        section.push_str(&format!(
            "  [{}], [{:.1}{}], [{:.1}{}], table.cell(fill: rgb(\"#{}\"))[{:+.1}], [{:+.1}%],\n",
            escape_typst(&m.name),
            m.value_a,
            unit,
            m.value_b,
            unit,
            fill_hex,
            m.delta,
            m.delta_percent,
        ));
    }

    section.push_str(")\n\n#pagebreak()\n");
    section
}

fn generate_side_by_side_info_section(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    label_a: &str,
    label_b: &str,
) -> String {
    let sym_str = |ldt: &Eulumdat| -> &'static str {
        match ldt.symmetry {
            eulumdat::Symmetry::None => "No symmetry",
            eulumdat::Symmetry::VerticalAxis => "Vertical axis",
            eulumdat::Symmetry::PlaneC0C180 => "C0-C180 plane",
            eulumdat::Symmetry::PlaneC90C270 => "C90-C270 plane",
            eulumdat::Symmetry::BothPlanes => "Both planes",
        }
    };

    let lamp_type_a = ldt_a
        .lamp_sets
        .first()
        .map(|l| l.lamp_type.as_str())
        .unwrap_or("-");
    let lamp_type_b = ldt_b
        .lamp_sets
        .first()
        .map(|l| l.lamp_type.as_str())
        .unwrap_or("-");
    let color_a = ldt_a
        .lamp_sets
        .first()
        .map(|l| l.color_appearance.as_str())
        .unwrap_or("-");
    let color_b = ldt_b
        .lamp_sets
        .first()
        .map(|l| l.color_appearance.as_str())
        .unwrap_or("-");

    format!(
        r#"
= Side-by-Side Luminaire Info

#table(
  columns: (auto, 1fr, 1fr),
  stroke: 0.5pt + luma(204),
  inset: 8pt,
  fill: (col, row) => if row == 0 {{ luma(240) }} else {{ none }},
  [*Property*], [*{}*], [*{}*],
  [Luminaire Name], [{}], [{}],
  [Manufacturer], [{}], [{}],
  [Symmetry], [{}], [{}],
  [Dimensions (L×W×H)], [{:.0}×{:.0}×{:.0} mm], [{:.0}×{:.0}×{:.0} mm],
  [Lamp Type], [{}], [{}],
  [Color Temperature], [{}], [{}],
  [Number of Lamps], [{}], [{}],
)

#pagebreak()
"#,
        escape_typst(label_a),
        escape_typst(label_b),
        escape_typst(&ldt_a.luminaire_name),
        escape_typst(&ldt_b.luminaire_name),
        escape_typst(&ldt_a.identification),
        escape_typst(&ldt_b.identification),
        sym_str(ldt_a),
        sym_str(ldt_b),
        ldt_a.length,
        ldt_a.width,
        ldt_a.height,
        ldt_b.length,
        ldt_b.width,
        ldt_b.height,
        escape_typst(lamp_type_a),
        escape_typst(lamp_type_b),
        escape_typst(color_a),
        escape_typst(color_b),
        ldt_a.lamp_sets.first().map(|l| l.num_lamps).unwrap_or(0),
        ldt_b.lamp_sets.first().map(|l| l.num_lamps).unwrap_or(0),
    )
}

fn generate_side_by_side_diagrams_section(
    ldt_a: &Eulumdat,
    ldt_b: &Eulumdat,
    label_a: &str,
    label_b: &str,
    theme: &SvgTheme,
) -> String {
    let mut section = String::new();
    section.push_str("\n= Side-by-Side Diagrams\n\n");

    // Heatmaps
    let ha = HeatmapDiagram::from_eulumdat(ldt_a, 400.0, 250.0);
    let hb = HeatmapDiagram::from_eulumdat(ldt_b, 400.0, 250.0);
    let svg_ha = escape_svg_for_typst(&ha.to_svg(400.0, 250.0, theme));
    let svg_hb = escape_svg_for_typst(&hb.to_svg(400.0, 250.0, theme));

    section.push_str(&format!(
        r##"== Intensity Heatmaps

#grid(
  columns: (1fr, 1fr),
  gutter: 8pt,
  [#align(center)[*{}*] #image(bytes("{}"), width: 100%)],
  [#align(center)[*{}*] #image(bytes("{}"), width: 100%)],
)

#v(1em)

"##,
        escape_typst(label_a),
        svg_ha,
        escape_typst(label_b),
        svg_hb,
    ));

    // Butterfly diagrams
    let ba = ButterflyDiagram::from_eulumdat(ldt_a, 350.0, 280.0, 60.0);
    let bb = ButterflyDiagram::from_eulumdat(ldt_b, 350.0, 280.0, 60.0);
    let svg_ba = escape_svg_for_typst(&ba.to_svg(350.0, 280.0, theme));
    let svg_bb = escape_svg_for_typst(&bb.to_svg(350.0, 280.0, theme));

    section.push_str(&format!(
        r##"== 3D Butterfly Diagrams

#grid(
  columns: (1fr, 1fr),
  gutter: 8pt,
  [#align(center)[*{}*] #image(bytes("{}"), width: 100%)],
  [#align(center)[*{}*] #image(bytes("{}"), width: 100%)],
)
"##,
        escape_typst(label_a),
        svg_ba,
        escape_typst(label_b),
        svg_bb,
    ));

    section
}
