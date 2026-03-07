use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::ViewMode;

pub fn render_status(
    area: Rect,
    buf: &mut Buffer,
    view_mode: ViewMode,
    file_name: &str,
    c_plane_label: Option<&str>,
) {
    let mut spans = vec![
        Span::styled(
            format!(" [{}] ", view_mode.label()),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", file_name),
            Style::default().fg(Color::White),
        ),
    ];

    if let Some(label) = c_plane_label {
        spans.push(Span::styled(
            format!(" {} ", label),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(80, 140, 255)),
        ));
    }

    spans.extend_from_slice(&[
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled("[V]", Style::default().fg(Color::Yellow)),
        Span::styled("view ", Style::default().fg(Color::Gray)),
        Span::styled("[+-]", Style::default().fg(Color::Yellow)),
        Span::styled("zoom ", Style::default().fg(Color::Gray)),
        Span::styled(
            "[\u{2191}\u{2193}\u{2190}\u{2192}]",
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("pan ", Style::default().fg(Color::Gray)),
    ]);

    if view_mode == ViewMode::Polar {
        spans.extend_from_slice(&[
            Span::styled("[JK]", Style::default().fg(Color::Yellow)),
            Span::styled("C-plane ", Style::default().fg(Color::Gray)),
        ]);
    }

    spans.extend_from_slice(&[
        Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
        Span::styled("focus ", Style::default().fg(Color::Gray)),
        Span::styled("[R]", Style::default().fg(Color::Yellow)),
        Span::styled("reset ", Style::default().fg(Color::Gray)),
        Span::styled("[Q]", Style::default().fg(Color::Yellow)),
        Span::styled("quit", Style::default().fg(Color::Gray)),
    ]);

    let paragraph = Paragraph::new(Line::from(spans));
    paragraph.render(area, buf);
}
