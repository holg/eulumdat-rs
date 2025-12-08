//! SVG rendering to RGBA pixels using resvg

use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Tree};

/// Render an SVG string to RGBA pixels
///
/// Returns (pixels, width, height) on success
pub fn render_ldt_to_rgba(
    svg: &str,
    max_width: u32,
    max_height: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    // Parse SVG
    let options = Options::default();
    let tree = Tree::from_str(svg, &options).map_err(|e| e.to_string())?;

    // Get original size
    let size = tree.size();
    let orig_width = size.width();
    let orig_height = size.height();

    // Calculate scale to fit within max dimensions while preserving aspect ratio
    let scale_x = max_width as f32 / orig_width;
    let scale_y = max_height as f32 / orig_height;
    let scale = scale_x.min(scale_y).min(1.0); // Don't upscale

    let final_width = (orig_width * scale).ceil() as u32;
    let final_height = (orig_height * scale).ceil() as u32;

    // Create pixmap
    let mut pixmap =
        Pixmap::new(final_width, final_height).ok_or_else(|| "Failed to create pixmap")?;

    // Fill with white background
    pixmap.fill(resvg::tiny_skia::Color::WHITE);

    // Render SVG
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Return RGBA pixels
    Ok((pixmap.take(), final_width, final_height))
}

/// Render an SVG to BGRA pixels (for Windows DIB)
///
/// Windows bitmaps use BGRA format, so we need to swap channels
pub fn render_ldt_to_bgra(
    svg: &str,
    max_width: u32,
    max_height: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    let (mut pixels, width, height) = render_ldt_to_rgba(svg, max_width, max_height)?;

    // Convert RGBA to BGRA (swap R and B)
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    Ok((pixels, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_svg() {
        let svg = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <circle cx="50" cy="50" r="40" fill="blue"/>
        </svg>"#;

        let result = render_ldt_to_rgba(svg, 200, 200);
        assert!(result.is_ok());

        let (pixels, width, height) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
        assert_eq!(pixels.len(), (100 * 100 * 4) as usize);
    }
}
