//! Eulumdat Cross-Platform GUI Application
//!
//! A native application for viewing and editing EULUMDAT/LDT and IES photometric files.

use eframe::egui;

mod app;
mod diagram;
mod render;
mod templates;
mod ui;

/// Load the application icon from embedded SVG
fn load_icon() -> Option<egui::IconData> {
    let svg_data = include_str!(concat!(env!("OUT_DIR"), "/icon.svg"));

    // Parse SVG and render to pixels
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_data, &options).ok()?;

    let size = 64u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;

    // Fill with transparent background
    pixmap.fill(resvg::tiny_skia::Color::TRANSPARENT);

    // Calculate scale to fit
    let tree_size = tree.size();
    let scale = (size as f32 / tree_size.width()).min(size as f32 / tree_size.height());

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(egui::IconData {
        rgba: pixmap.take(),
        width: size,
        height: size,
    })
}

fn main() -> eframe::Result<()> {
    let icon = load_icon();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Eulumdat Viewer");

    if let Some(icon_data) = icon {
        viewport = viewport.with_icon(std::sync::Arc::new(icon_data));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Eulumdat Viewer",
        options,
        Box::new(|cc| Ok(Box::new(app::EulumdatApp::new(cc)))),
    )
}
