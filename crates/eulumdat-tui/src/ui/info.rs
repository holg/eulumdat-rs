use eulumdat::{Eulumdat, PhotometricSummary, ValidationWarning};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

pub fn render_info(
    area: Rect,
    buf: &mut Buffer,
    ldt: &Eulumdat,
    summary: &PhotometricSummary,
    warnings: &[ValidationWarning],
    scroll: u16,
    focused: bool,
) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // LUMINAIRE section
    lines.push(Line::styled(
        " LUMINAIRE ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    add_field(&mut lines, "Name", &ldt.luminaire_name);
    add_field(&mut lines, "Manufacturer", &ldt.identification);
    add_field(&mut lines, "Number", &ldt.luminaire_number);
    add_field(&mut lines, "Symmetry", &format!("{:?}", ldt.symmetry));
    add_field(&mut lines, "Type", &format!("{:?}", ldt.type_indicator));
    add_field(
        &mut lines,
        "Dimensions",
        &format!(
            "{:.0} x {:.0} x {:.0} mm",
            ldt.length, ldt.width, ldt.height
        ),
    );
    add_field(
        &mut lines,
        "Luminous area",
        &format!(
            "{:.0} x {:.0} mm",
            ldt.luminous_area_length, ldt.luminous_area_width
        ),
    );
    add_field(
        &mut lines,
        "C-planes",
        &format!(
            "{} ({:.1}\u{00b0} step)",
            ldt.c_angles.len(),
            ldt.c_plane_distance
        ),
    );
    add_field(
        &mut lines,
        "G-angles",
        &format!(
            "{} ({:.1}\u{00b0} step)",
            ldt.g_angles.len(),
            ldt.g_plane_distance
        ),
    );
    lines.push(Line::raw(""));

    // LAMPS section
    lines.push(Line::styled(
        " LAMPS ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    for (i, lamp) in ldt.lamp_sets.iter().enumerate() {
        if ldt.lamp_sets.len() > 1 {
            lines.push(Line::styled(
                format!("  Set {}", i + 1),
                Style::default().fg(Color::Yellow),
            ));
        }
        add_field(&mut lines, "Type", &lamp.lamp_type);
        add_field(&mut lines, "Count", &format!("{}", lamp.num_lamps));
        add_field(
            &mut lines,
            "Flux",
            &format!("{:.0} lm", lamp.total_luminous_flux),
        );
        add_field(
            &mut lines,
            "Wattage",
            &format!("{:.1} W", lamp.wattage_with_ballast),
        );
        add_field(&mut lines, "CCT", &lamp.color_appearance);
        add_field(&mut lines, "CRI", &lamp.color_rendering_group);
    }
    lines.push(Line::raw(""));

    // PHOTOMETRY section
    lines.push(Line::styled(
        " PHOTOMETRY ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    add_field(&mut lines, "LOR", &format!("{:.1}%", summary.lor));
    add_field(&mut lines, "DLOR", &format!("{:.1}%", summary.dlor));
    add_field(&mut lines, "ULOR", &format!("{:.1}%", summary.ulor));
    add_field(
        &mut lines,
        "Lamp flux",
        &format!("{:.0} lm", summary.total_lamp_flux),
    );
    add_field(
        &mut lines,
        "Efficacy",
        &format!("{:.1} lm/W", summary.luminaire_efficacy),
    );
    add_field(
        &mut lines,
        "Distribution",
        &format!("{:?}", summary.distribution_type),
    );
    add_field(
        &mut lines,
        "Beam (IES)",
        &format!("{:.1}\u{00b0}", summary.beam_angle),
    );
    add_field(
        &mut lines,
        "Field (IES)",
        &format!("{:.1}\u{00b0}", summary.field_angle),
    );
    add_field(
        &mut lines,
        "Beam (CIE)",
        &format!("{:.1}\u{00b0}", summary.beam_angle_cie),
    );
    add_field(
        &mut lines,
        "Field (CIE)",
        &format!("{:.1}\u{00b0}", summary.field_angle_cie),
    );
    add_field(
        &mut lines,
        "Max intensity",
        &format!("{:.1} cd/klm", summary.max_intensity),
    );
    add_field(
        &mut lines,
        "Avg intensity",
        &format!("{:.1} cd/klm", summary.avg_intensity),
    );
    add_field(&mut lines, "S/H C0", &format!("{:.2}", summary.spacing_c0));
    add_field(
        &mut lines,
        "S/H C90",
        &format!("{:.2}", summary.spacing_c90),
    );
    lines.push(Line::raw(""));

    // WARNINGS section
    if !warnings.is_empty() {
        lines.push(Line::styled(
            format!(" WARNINGS ({}) ", warnings.len()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        for w in warnings {
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", w.code), Style::default().fg(Color::Yellow)),
                Span::styled(w.message.clone(), Style::default().fg(Color::Gray)),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" Info "),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    paragraph.render(area, buf);
}

fn add_field(lines: &mut Vec<Line<'static>>, label: &str, value: &str) {
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<14}", label),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ]));
}
