use anyhow::{bail, Context, Result};

/// Decode a KTX2 file (possibly zstd-supercompressed, BC7-encoded) into RGBA8 pixels.
/// Returns (width, height, rgba_bytes).
pub fn decode_ktx2(data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let reader = ktx2::Reader::new(data).context("Failed to parse KTX2 container")?;
    let header = reader.header();

    let width = header.pixel_width;
    let height = header.pixel_height;

    if width == 0 || height == 0 {
        bail!("KTX2 has zero-sized dimensions: {width}x{height}");
    }

    // Get the first (highest-resolution) mip level
    let level0 = reader
        .levels()
        .next()
        .context("KTX2 file contains no mip levels")?;
    let level0_data = level0.data;

    // Handle zstd supercompression
    let raw_blocks = match header.supercompression_scheme {
        Some(ktx2::SupercompressionScheme::Zstandard) => decompress_zstd(level0_data)?,
        None => level0_data.to_vec(),
        Some(other) => bail!("Unsupported KTX2 supercompression scheme: {other:?}"),
    };

    // Decode based on the VkFormat
    decode_blocks(header.format, width, height, &raw_blocks)
}

fn decompress_zstd(compressed: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(compressed)
        .context("Failed to create zstd decoder")?;
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .context("Failed to decompress zstd data")?;
    Ok(decompressed)
}

fn decode_blocks(
    format: Option<ktx2::Format>,
    width: u32,
    height: u32,
    raw: &[u8],
) -> Result<(u32, u32, Vec<u8>)> {
    let pixel_count = (width * height) as usize;

    match format {
        // BC7 (SRGB and UNORM)
        Some(ktx2::Format::BC7_SRGB_BLOCK) | Some(ktx2::Format::BC7_UNORM_BLOCK) => {
            let mut decoded_u32 = vec![0u32; pixel_count];
            texture2ddecoder::decode_bc7(raw, width as usize, height as usize, &mut decoded_u32)
                .map_err(|e| anyhow::anyhow!("BC7 decode failed: {e}"))?;
            Ok((width, height, u32_pixels_to_rgba8(&decoded_u32)))
        }

        // BC1 (DXT1)
        Some(ktx2::Format::BC1_RGB_SRGB_BLOCK)
        | Some(ktx2::Format::BC1_RGB_UNORM_BLOCK)
        | Some(ktx2::Format::BC1_RGBA_SRGB_BLOCK)
        | Some(ktx2::Format::BC1_RGBA_UNORM_BLOCK) => {
            let mut decoded_u32 = vec![0u32; pixel_count];
            texture2ddecoder::decode_bc1(raw, width as usize, height as usize, &mut decoded_u32)
                .map_err(|e| anyhow::anyhow!("BC1 decode failed: {e}"))?;
            Ok((width, height, u32_pixels_to_rgba8(&decoded_u32)))
        }

        // BC3 (DXT5)
        Some(ktx2::Format::BC3_SRGB_BLOCK) | Some(ktx2::Format::BC3_UNORM_BLOCK) => {
            let mut decoded_u32 = vec![0u32; pixel_count];
            texture2ddecoder::decode_bc3(raw, width as usize, height as usize, &mut decoded_u32)
                .map_err(|e| anyhow::anyhow!("BC3 decode failed: {e}"))?;
            Ok((width, height, u32_pixels_to_rgba8(&decoded_u32)))
        }

        // BC5 (two-channel, used for normal maps)
        Some(ktx2::Format::BC5_UNORM_BLOCK) | Some(ktx2::Format::BC5_SNORM_BLOCK) => {
            let mut decoded_u32 = vec![0u32; pixel_count];
            texture2ddecoder::decode_bc5(raw, width as usize, height as usize, &mut decoded_u32)
                .map_err(|e| anyhow::anyhow!("BC5 decode failed: {e}"))?;
            Ok((width, height, u32_pixels_to_rgba8(&decoded_u32)))
        }

        // R8G8B8A8 uncompressed
        Some(ktx2::Format::R8G8B8A8_SRGB) | Some(ktx2::Format::R8G8B8A8_UNORM) => {
            let expected = pixel_count * 4;
            if raw.len() < expected {
                bail!(
                    "RGBA8 data too short: got {} bytes, expected {expected}",
                    raw.len()
                );
            }
            Ok((width, height, raw[..expected].to_vec()))
        }

        // R8G8B8 uncompressed
        Some(ktx2::Format::R8G8B8_SRGB) | Some(ktx2::Format::R8G8B8_UNORM) => {
            let expected = pixel_count * 3;
            if raw.len() < expected {
                bail!(
                    "RGB8 data too short: got {} bytes, expected {expected}",
                    raw.len()
                );
            }
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for chunk in raw[..expected].chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            Ok((width, height, rgba))
        }

        Some(other) => bail!("Unsupported KTX2 format: {other:?}"),
        None => bail!("KTX2 file has no format specified"),
    }
}

/// Convert u32 pixels from texture2ddecoder to RGBA8 bytes.
/// texture2ddecoder packs as `u32::from_le_bytes([B, G, R, A])`,
/// so on LE systems: byte0=B, byte1=G, byte2=R, byte3=A.
fn u32_pixels_to_rgba8(pixels: &[u32]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for &p in pixels {
        let [b, g, r, a] = p.to_le_bytes();
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(a);
    }
    rgba
}
