use eulumdat::diagram::HeatmapDiagram;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

pub fn render_heatmap(area: Rect, buf: &mut Buffer, heatmap: &HeatmapDiagram, focused: bool) {
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Heatmap ");
    let inner = block.inner(area);
    block.render(area, buf);

    if heatmap.is_empty() || inner.width < 4 || inner.height < 4 {
        return;
    }

    let num_c = heatmap.c_angles.len();
    let num_g = heatmap.g_angles.len();
    if num_c == 0 || num_g == 0 {
        return;
    }

    // Reserve space for labels
    let y_label_width = 5u16;
    let x_label_height = 1u16;
    let legend_width = 6u16;

    let plot_x = inner.x + y_label_width;
    let plot_y = inner.y;
    let plot_w = inner.width.saturating_sub(y_label_width + legend_width);
    let plot_h = inner.height.saturating_sub(x_label_height);

    if plot_w < 2 || plot_h < 2 {
        return;
    }

    // Draw cells
    for cell in &heatmap.cells {
        let cx = plot_x + (cell.c_index as u16 * plot_w) / num_c as u16;
        let cy = plot_y + (cell.g_index as u16 * plot_h) / num_g as u16;

        if cx < plot_x + plot_w && cy < plot_y + plot_h {
            let color = Color::Rgb(cell.color.r, cell.color.g, cell.color.b);
            buf[(cx, cy)].set_char('\u{2588}').set_fg(color);
        }
    }

    // Y-axis labels (gamma angles)
    let y_step = (num_g / (plot_h as usize).max(1)).max(1);
    for (i, &g) in heatmap.g_angles.iter().enumerate() {
        if i % y_step != 0 {
            continue;
        }
        let cy = plot_y + (i as u16 * plot_h) / num_g as u16;
        if cy < plot_y + plot_h {
            let label = format!("{:>4.0}\u{00b0}", g);
            let lx = inner.x;
            for (j, ch) in label.chars().enumerate() {
                let px = lx + j as u16;
                if px < plot_x {
                    buf[(px, cy)].set_char(ch).set_fg(Color::DarkGray);
                }
            }
        }
    }

    // X-axis labels (C-plane angles)
    let label_y = plot_y + plot_h;
    if label_y < inner.y + inner.height {
        let x_step = (num_c / ((plot_w as usize) / 5).max(1)).max(1);
        for (i, &c) in heatmap.c_angles.iter().enumerate() {
            if i % x_step != 0 {
                continue;
            }
            let cx = plot_x + (i as u16 * plot_w) / num_c as u16;
            let label = format!("{:.0}\u{00b0}", c);
            for (j, ch) in label.chars().enumerate() {
                let px = cx + j as u16;
                if px < plot_x + plot_w {
                    buf[(px, label_y)].set_char(ch).set_fg(Color::DarkGray);
                }
            }
        }
    }

    // Color legend bar
    let legend_x = plot_x + plot_w + 1;
    if legend_x + 1 < inner.x + inner.width {
        for row in 0..plot_h {
            let normalized = 1.0 - (row as f64 / plot_h as f64);
            let color = eulumdat::diagram::heatmap_color(normalized);
            let cy = plot_y + row;
            buf[(legend_x, cy)]
                .set_char('\u{2588}')
                .set_fg(Color::Rgb(color.r, color.g, color.b));
        }
        // Legend labels
        let max_label = format!("{:.0}", heatmap.max_candela);
        for (j, ch) in max_label.chars().enumerate() {
            let px = legend_x + 1 + j as u16;
            if px < inner.x + inner.width {
                buf[(px, plot_y)].set_char(ch).set_fg(Color::DarkGray);
            }
        }
        let bottom = plot_y + plot_h.saturating_sub(1);
        buf[(legend_x + 1, bottom)]
            .set_char('0')
            .set_fg(Color::DarkGray);
    }
}
