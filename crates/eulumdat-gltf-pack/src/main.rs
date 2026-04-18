mod ktx2_decode;
mod ktx2_encode;
mod pack;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "eulumdat-gltf-pack")]
#[command(about = "Pack a glTF with external KTX2 textures into a self-contained GLB")]
struct Cli {
    /// Input .gltf file path
    input: PathBuf,

    /// Output .glb file path
    #[arg(short, long)]
    output: PathBuf,

    /// Maximum texture dimension (textures larger than this are downscaled)
    #[arg(long, default_value_t = 512)]
    max_texture_size: u32,

    /// JPEG quality (1-100, used when --gpu-compress is off)
    #[arg(long, default_value_t = 75)]
    jpeg_quality: u8,

    /// Encode textures as KTX2 with Basis Universal UASTC supercompression.
    /// Produces GPU-compressed textures that stay compressed in VRAM (BC7/ASTC/ETC2).
    /// Requires KHR_texture_basisu extension support (WebGPU, modern native).
    #[arg(long)]
    gpu_compress: bool,

    /// UASTC quality level (1=fastest, 4=highest). Default: 2
    #[arg(long, default_value_t = 2)]
    uastc_quality: u32,

    /// Enable UASTC RDO (Rate-Distortion Optimization) for smaller files.
    /// Quality scalar: lower = smaller but more lossy. Default: 1.0
    #[arg(long)]
    rdo_quality: Option<f32>,

    /// Generate mipmaps for GPU-compressed textures
    #[arg(long)]
    mipmaps: bool,

    /// Show what would be done without writing output
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let opts = pack::PackOptions {
        max_texture_size: cli.max_texture_size,
        jpeg_quality: cli.jpeg_quality,
        gpu_compress: cli.gpu_compress,
        uastc_quality: cli.uastc_quality,
        rdo_quality: cli.rdo_quality,
        mipmaps: cli.mipmaps,
        dry_run: cli.dry_run,
    };

    pack::pack_gltf(&cli.input, &cli.output, &opts)
}
