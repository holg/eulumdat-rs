use anyhow::{Context, Result};
use ktx2_rw::{BasisCompressionParams, Ktx2Texture, VkFormat};

/// Options for UASTC encoding.
pub struct UastcOptions {
    pub quality: u32,
    pub rdo_quality: Option<f32>,
    pub mipmaps: bool,
    /// True for sRGB color textures, false for linear data (normal maps).
    pub srgb: bool,
}

impl Default for UastcOptions {
    fn default() -> Self {
        Self {
            quality: 2,
            rdo_quality: None,
            mipmaps: false,
            srgb: true,
        }
    }
}

/// Encode RGBA8 pixels to a KTX2 file with Basis Universal UASTC supercompression.
///
/// The output is a valid KTX2 container that Bevy can load and transcode to
/// the GPU's native format (BC7 on desktop, ASTC on Apple, ETC2 on mobile).
/// Textures stay compressed in VRAM — no 18GB RAM explosion.
pub fn encode_uastc(
    rgba: &[u8],
    width: u32,
    height: u32,
    opts: &UastcOptions,
) -> Result<Vec<u8>> {
    let vk_format = if opts.srgb {
        VkFormat::R8G8B8A8Srgb
    } else {
        VkFormat::R8G8B8A8Unorm
    };

    let levels = if opts.mipmaps {
        // Calculate mip levels: floor(log2(max(w,h))) + 1
        ((width.max(height) as f32).log2().floor() as u32 + 1).max(1)
    } else {
        1
    };

    let mut texture = Ktx2Texture::create(width, height, 1, 1, 1, levels, vk_format)
        .context("Failed to create KTX2 texture")?;

    // Set level 0 image data
    texture
        .set_image_data(0, 0, 0, rgba)
        .context("Failed to set KTX2 image data")?;

    // Map quality 1-4 to ktx2-rw's 0-255 scale
    let quality_255 = match opts.quality {
        0 | 1 => 64,
        2 => 128,
        3 => 192,
        _ => 255,
    };

    let mut params_builder = BasisCompressionParams::builder()
        .uastc(true)
        .quality_level(quality_255)
        .thread_count(4);

    if let Some(rdo) = opts.rdo_quality {
        params_builder = params_builder.uastc_rdo_quality_scalar(rdo);
    }

    let params = params_builder.build();

    texture
        .compress_basis(&params)
        .context("UASTC compression failed")?;

    texture.write_to_memory().context("Failed to write KTX2 to memory")
}
