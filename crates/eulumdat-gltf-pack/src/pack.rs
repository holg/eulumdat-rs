use anyhow::{bail, Context, Result};
use gltf_json as json;
use std::fs;
use std::path::Path;

use crate::ktx2_decode;
use crate::ktx2_encode;

pub struct PackOptions {
    pub max_texture_size: u32,
    pub jpeg_quality: u8,
    pub gpu_compress: bool,
    pub uastc_quality: u32,
    pub rdo_quality: Option<f32>,
    pub mipmaps: bool,
    pub dry_run: bool,
}

struct ImageBlob {
    data: Vec<u8>,
    mime: &'static str, // "image/jpeg", "image/png", or "image/ktx2"
}

/// GLB magic, version 2, header size 12 bytes
const GLB_HEADER_SIZE: usize = 12;
const GLB_CHUNK_HEADER_SIZE: usize = 8;
const GLB_MAGIC: u32 = 0x46546C67; // "glTF"
const GLB_VERSION: u32 = 2;
const GLB_JSON_CHUNK: u32 = 0x4E4F534A; // "JSON"
const GLB_BIN_CHUNK: u32 = 0x004E4942; // "BIN\0"

pub fn pack_gltf(input: &Path, output: &Path, opts: &PackOptions) -> Result<()> {
    let base_dir = input
        .parent()
        .context("Input path has no parent directory")?;

    if opts.gpu_compress {
        println!("Mode: GPU-compressed (UASTC quality={}, mipmaps={})",
            opts.uastc_quality,
            if opts.mipmaps { "on" } else { "off" });
    } else {
        println!("Mode: CPU textures (JPEG q={}, PNG for alpha)", opts.jpeg_quality);
    }

    // Parse the source glTF
    let gltf_text = fs::read_to_string(input).context("Failed to read input .gltf")?;
    let source: json::Root =
        serde_json::from_str(&gltf_text).context("Failed to parse glTF JSON")?;

    // Read the geometry .bin buffer
    let geometry_bytes = read_bin_buffer(&source, base_dir)?;
    let geometry_len = geometry_bytes.len();
    let geometry_padded = pad_to_4(geometry_len);

    println!(
        "Geometry buffer: {:.1} MB",
        geometry_len as f64 / 1_048_576.0
    );

    // Detect which texture indices are used as normal maps
    let normal_map_indices = find_normal_map_textures(&source);

    // Process images
    let image_count = source.images.len();
    println!("Processing {image_count} images...");

    let mut image_blobs: Vec<ImageBlob> = Vec::with_capacity(image_count);
    let mut total_original: u64 = 0;
    let mut total_packed: u64 = 0;

    for (i, img) in source.images.iter().enumerate() {
        let uri = img
            .uri
            .as_ref()
            .with_context(|| format!("Image {i} has no URI"))?;
        let img_path = base_dir.join(uri);
        let img_name = img.name.as_deref().unwrap_or(uri);

        let raw = fs::read(&img_path)
            .with_context(|| format!("Failed to read image: {}", img_path.display()))?;
        total_original += raw.len() as u64;

        if opts.dry_run {
            let ext = detect_format(&raw, uri);
            println!(
                "  [{}/{}] {} ({:.0} KB, {ext}) — would process",
                i + 1,
                image_count,
                img_name,
                raw.len() as f64 / 1024.0
            );
            image_blobs.push(ImageBlob {
                data: Vec::new(),
                mime: if opts.gpu_compress { "image/ktx2" } else { "image/jpeg" },
            });
            continue;
        }

        let is_normal = is_normal_map_image(i, &source, &normal_map_indices);
        let blob = convert_image(&raw, uri, img_name, i, image_count, is_normal, opts)?;
        total_packed += blob.data.len() as u64;
        image_blobs.push(blob);
    }

    println!(
        "Textures: {:.1} MB -> {:.1} MB ({:.0}% reduction)",
        total_original as f64 / 1_048_576.0,
        total_packed as f64 / 1_048_576.0,
        if total_original > 0 {
            (1.0 - total_packed as f64 / total_original as f64) * 100.0
        } else {
            0.0
        }
    );

    if opts.dry_run {
        println!("Dry run complete — no output written.");
        return Ok(());
    }

    // Build the combined binary chunk: geometry + image blobs
    let mut bin_chunk = Vec::with_capacity(
        geometry_padded
            + image_blobs
                .iter()
                .map(|b| pad_to_4(b.data.len()))
                .sum::<usize>(),
    );

    // Geometry first (padded to 4-byte boundary)
    bin_chunk.extend_from_slice(&geometry_bytes);
    while bin_chunk.len() < geometry_padded {
        bin_chunk.push(0);
    }

    // Build the output glTF JSON with embedded images
    let (json_root, bin_chunk) = build_output_json(source, image_blobs, bin_chunk, opts.gpu_compress)?;

    // Serialize JSON
    let json_str = serde_json::to_string(&json_root).context("Failed to serialize glTF JSON")?;
    let json_bytes = json_str.into_bytes();
    let json_padded = pad_to_4(json_bytes.len());

    let total_size = GLB_HEADER_SIZE
        + GLB_CHUNK_HEADER_SIZE
        + json_padded
        + GLB_CHUNK_HEADER_SIZE
        + bin_chunk.len();

    println!(
        "Writing GLB: {:.1} MB (json: {:.0} KB, bin: {:.1} MB)",
        total_size as f64 / 1_048_576.0,
        json_padded as f64 / 1024.0,
        bin_chunk.len() as f64 / 1_048_576.0,
    );

    // Write GLB
    let mut glb = Vec::with_capacity(total_size);

    // GLB header
    glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    glb.extend_from_slice(&GLB_VERSION.to_le_bytes());
    glb.extend_from_slice(&(total_size as u32).to_le_bytes());

    // JSON chunk
    glb.extend_from_slice(&(json_padded as u32).to_le_bytes());
    glb.extend_from_slice(&GLB_JSON_CHUNK.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    while glb.len() < GLB_HEADER_SIZE + GLB_CHUNK_HEADER_SIZE + json_padded {
        glb.push(0x20); // Space padding for JSON
    }

    // BIN chunk
    glb.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
    glb.extend_from_slice(&GLB_BIN_CHUNK.to_le_bytes());
    glb.extend_from_slice(&bin_chunk);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(output, &glb).with_context(|| format!("Failed to write {}", output.display()))?;

    println!("Done: {}", output.display());
    Ok(())
}

fn read_bin_buffer(root: &json::Root, base_dir: &Path) -> Result<Vec<u8>> {
    if root.buffers.is_empty() {
        bail!("glTF has no buffers");
    }

    let buf = &root.buffers[0];
    let uri = buf
        .uri
        .as_ref()
        .context("First buffer has no URI (already a GLB?)")?;

    let bin_path = base_dir.join(uri);
    fs::read(&bin_path).with_context(|| format!("Failed to read buffer: {}", bin_path.display()))
}

/// Detect image format from magic bytes or file extension.
fn detect_format(data: &[u8], uri: &str) -> &'static str {
    // Check magic bytes first
    if data.len() >= 12 && &data[0..4] == b"\xabKTX" {
        return "ktx2";
    }
    if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        return "png";
    }
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        return "jpeg";
    }
    // Fallback to extension
    let lower = uri.to_lowercase();
    if lower.ends_with(".ktx2") {
        "ktx2"
    } else if lower.ends_with(".png") {
        "png"
    } else {
        "jpeg"
    }
}

/// Check if an RGBA image has meaningful transparency.
/// Uses a threshold to ignore BC7 compression artifacts (alpha 254-255).
fn has_alpha(rgba: &[u8]) -> bool {
    // Count pixels with alpha significantly below opaque
    let transparent_pixels = rgba.chunks_exact(4).filter(|px| px[3] < 250).count();
    let total_pixels = rgba.len() / 4;
    // Need at least 0.1% of pixels to be transparent to consider this an alpha image
    transparent_pixels > 0 && transparent_pixels * 1000 > total_pixels
}

/// Find which texture indices are used as normal maps in any material.
fn find_normal_map_textures(root: &json::Root) -> Vec<u32> {
    let mut indices = Vec::new();
    for mat in &root.materials {
        if let Some(ref normal) = mat.normal_texture {
            indices.push(normal.index.value() as u32);
        }
    }
    indices
}

/// Check if image index `i` is used as a normal map texture.
fn is_normal_map_image(image_index: usize, root: &json::Root, normal_tex_indices: &[u32]) -> bool {
    // Map: texture index -> image index via texture.source
    for &tex_idx in normal_tex_indices {
        if let Some(tex) = root.textures.get(tex_idx as usize) {
            if tex.source.value() == image_index {
                return true;
            }
        }
    }
    false
}

fn convert_image(
    file_data: &[u8],
    uri: &str,
    name: &str,
    index: usize,
    total: usize,
    is_normal: bool,
    opts: &PackOptions,
) -> Result<ImageBlob> {
    let format = detect_format(file_data, uri);

    // Decode to RGBA first
    let (w, h, rgba) = match format {
        "ktx2" => ktx2_decode::decode_ktx2(file_data)
            .with_context(|| format!("Failed to decode KTX2 for image '{name}'"))?,
        _ => {
            let img = image::load_from_memory(file_data)
                .with_context(|| format!("Failed to decode image '{name}'"))?;
            (img.width(), img.height(), img.to_rgba8().into_raw())
        }
    };

    let (final_w, final_h, final_rgba) = resize_if_needed(w, h, rgba, opts.max_texture_size)?;

    if opts.gpu_compress {
        encode_uastc_and_log(name, index, total, w, h, final_w, final_h, &final_rgba, is_normal, opts)
    } else {
        encode_cpu_and_log(name, index, total, w, h, final_w, final_h, &final_rgba, opts)
    }
}

/// Encode as UASTC (GPU-compressed, stays compressed in VRAM).
fn encode_uastc_and_log(
    name: &str,
    index: usize,
    total: usize,
    orig_w: u32,
    orig_h: u32,
    w: u32,
    h: u32,
    rgba: &[u8],
    is_normal: bool,
    opts: &PackOptions,
) -> Result<ImageBlob> {
    let uastc_opts = ktx2_encode::UastcOptions {
        quality: opts.uastc_quality,
        rdo_quality: opts.rdo_quality,
        mipmaps: opts.mipmaps,
        srgb: !is_normal,
    };

    let data = ktx2_encode::encode_uastc(rgba, w, h, &uastc_opts)
        .with_context(|| format!("UASTC encode failed for '{name}'"))?;

    let normal_tag = if is_normal { " [normal]" } else { "" };
    println!(
        "  [{}/{}] {} {}x{} -> {}x{} UASTC{} ({:.0} KB)",
        index + 1,
        total,
        name,
        orig_w,
        orig_h,
        w,
        h,
        normal_tag,
        data.len() as f64 / 1024.0
    );

    Ok(ImageBlob {
        data,
        mime: "image/ktx2",
    })
}

/// Encode as JPEG (opaque) or PNG (has alpha) — CPU format.
fn encode_cpu_and_log(
    name: &str,
    index: usize,
    total: usize,
    orig_w: u32,
    orig_h: u32,
    w: u32,
    h: u32,
    rgba: &[u8],
    opts: &PackOptions,
) -> Result<ImageBlob> {
    let blob = if has_alpha(rgba) {
        // Encode as PNG to preserve alpha
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            rgba,
            w,
            h,
            image::ExtendedColorType::Rgba8,
        )
        .with_context(|| format!("PNG encode failed for '{name}'"))?;
        ImageBlob {
            data: buf,
            mime: "image/png",
        }
    } else {
        // Encode as JPEG (smaller, no alpha needed)
        let rgb = rgba_to_rgb(rgba, w, h)?;
        let mut buf = Vec::new();
        let mut encoder =
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, opts.jpeg_quality);
        encoder
            .encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
            .with_context(|| format!("JPEG encode failed for '{name}'"))?;
        ImageBlob {
            data: buf,
            mime: "image/jpeg",
        }
    };

    let fmt_tag = if blob.mime == "image/png" {
        "PNG"
    } else {
        "JPG"
    };
    println!(
        "  [{}/{}] {} {}x{} -> {}x{} {fmt_tag} ({:.0} KB)",
        index + 1,
        total,
        name,
        orig_w,
        orig_h,
        w,
        h,
        blob.data.len() as f64 / 1024.0
    );

    Ok(blob)
}

fn resize_if_needed(
    w: u32,
    h: u32,
    rgba: Vec<u8>,
    max_size: u32,
) -> Result<(u32, u32, Vec<u8>)> {
    if w > max_size || h > max_size {
        let scale = max_size as f64 / w.max(h) as f64;
        let new_w = ((w as f64 * scale).round() as u32).max(1);
        let new_h = ((h as f64 * scale).round() as u32).max(1);

        let img = image::RgbaImage::from_raw(w, h, rgba)
            .context("Failed to create image from decoded pixels")?;
        let resized =
            image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3);
        Ok((new_w, new_h, resized.into_raw()))
    } else {
        Ok((w, h, rgba))
    }
}

fn rgba_to_rgb(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
    let pixel_count = (w * h) as usize;
    let expected = pixel_count * 4;
    if rgba.len() < expected {
        bail!(
            "RGBA buffer too small: got {} bytes, expected {expected}",
            rgba.len()
        );
    }
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for chunk in rgba[..expected].chunks_exact(4) {
        rgb.push(chunk[0]);
        rgb.push(chunk[1]);
        rgb.push(chunk[2]);
    }
    Ok(rgb)
}

fn build_output_json(
    source: json::Root,
    image_blobs: Vec<ImageBlob>,
    mut bin_chunk: Vec<u8>,
    _gpu_compress: bool,
) -> Result<(json::Root, Vec<u8>)> {
    let mut root = source.clone();

    // Replace buffers: single GLB buffer (uri: None)
    root.buffers = vec![json::Buffer {
        byte_length: json::validation::USize64(0), // placeholder
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: None,
    }];

    // Existing buffer views reference buffer 0 and point into geometry.
    // Add new buffer views for each image blob.
    let existing_bv_count = root.buffer_views.len();
    let mut image_bv_indices: Vec<json::Index<json::buffer::View>> = Vec::new();

    for blob in &image_blobs {
        let offset = bin_chunk.len();
        bin_chunk.extend_from_slice(&blob.data);

        // Pad to 4-byte boundary
        while bin_chunk.len() % 4 != 0 {
            bin_chunk.push(0);
        }

        let bv = json::buffer::View {
            buffer: json::Index::new(0),
            byte_offset: Some(json::validation::USize64(offset as u64)),
            byte_length: json::validation::USize64(blob.data.len() as u64),
            byte_stride: None,
            target: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
        };

        let bv_index = json::Index::new((existing_bv_count + image_bv_indices.len()) as u32);
        image_bv_indices.push(bv_index);
        root.buffer_views.push(bv);
    }

    // Update images: uri -> bufferView + per-image mimeType
    for (i, img) in root.images.iter_mut().enumerate() {
        img.uri = None;
        img.buffer_view = Some(image_bv_indices[i]);
        img.mime_type = Some(json::image::MimeType(image_blobs[i].mime.into()));
    }

    // Set final buffer byte_length
    root.buffers[0].byte_length = json::validation::USize64(bin_chunk.len() as u64);

    // Bevy 0.18 doesn't support KHR_texture_basisu in its glTF loader yet
    // (see https://github.com/bevyengine/bevy/issues/19104).
    // Instead, we embed KTX2/UASTC images directly with mimeType "image/ktx2".
    // Bevy's KTX2 loader handles basis-universal transcoding natively.
    root.extensions_used
        .retain(|e| e != "KHR_texture_basisu");
    root.extensions_required
        .retain(|e| e != "KHR_texture_basisu");

    // Clear texture extensions (KHR_texture_basisu source pointers)
    for tex in root.textures.iter_mut() {
        tex.extensions = None;
    }

    Ok((root, bin_chunk))
}

fn pad_to_4(len: usize) -> usize {
    (len + 3) & !3
}
