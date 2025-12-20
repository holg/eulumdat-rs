//! SVG rendering utilities

use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Tree};
use std::sync::OnceLock;

/// Get a shared font database with system fonts loaded
fn get_fontdb() -> &'static fontdb::Database {
    static FONTDB: OnceLock<fontdb::Database> = OnceLock::new();
    FONTDB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        db
    })
}

/// Render an SVG string to RGBA pixels
pub fn render_svg_to_rgba(
    svg: &str,
    max_width: u32,
    max_height: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    // Parse SVG with font database for text rendering
    let options = Options {
        fontdb: std::sync::Arc::new(get_fontdb().clone()),
        ..Default::default()
    };
    let tree = Tree::from_str(svg, &options).map_err(|e| e.to_string())?;

    // Get original size
    let size = tree.size();
    let orig_width = size.width();
    let orig_height = size.height();

    // Calculate scale to fit within max dimensions while preserving aspect ratio
    // Use 2x resolution for crisp text rendering (like Retina displays)
    let scale_x = max_width as f32 / orig_width;
    let scale_y = max_height as f32 / orig_height;
    let base_scale = scale_x.min(scale_y);
    let scale = (base_scale * 2.0).min(4.0); // Render at 2x for crisp text, max 4x

    let final_width = (orig_width * scale).ceil() as u32;
    let final_height = (orig_height * scale).ceil() as u32;

    // Create pixmap
    let mut pixmap = Pixmap::new(final_width, final_height)
        .ok_or_else(|| "Failed to create pixmap".to_string())?;

    // Fill with white background
    pixmap.fill(resvg::tiny_skia::Color::WHITE);

    // Render SVG
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Return RGBA pixels
    Ok((pixmap.take(), final_width, final_height))
}

/// Convert RGBA to egui ColorImage
pub fn rgba_to_color_image(pixels: Vec<u8>, width: u32, height: u32) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels)
}
