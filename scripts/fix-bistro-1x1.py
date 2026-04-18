#!/usr/bin/env python3
"""Replace 1x1 BC7 KTX2 files with valid 4x4 RGBA8 KTX2 files.

These are flat normal maps (all pixels = (128,128,255,255)) or solid colors.
We generate a complete, valid KTX2 file from scratch rather than patching.
"""

import struct
import glob
import os
import sys

KTX2_MAGIC = bytes([0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A])
VK_FORMAT_R8G8B8A8_SRGB = 43


def make_ktx2_rgba8(width, height, pixel_data):
    """Create a minimal valid KTX2 file with RGBA8_SRGB format, 1 mip level."""
    # KTX2 header: 80 bytes
    # Level index: 24 bytes (1 level)
    # DFD: we need a valid Data Format Descriptor
    # KVD: empty (0 bytes)
    # Pixel data: width * height * 4 bytes

    pixel_size = width * height * 4
    assert len(pixel_data) == pixel_size

    # Build DFD (Data Format Descriptor)
    # Minimal DFD for RGBA8: totalSize(4) + descriptorBlock
    # Descriptor block: 24 bytes header + 16 bytes per sample (4 samples for RGBA)
    num_samples = 4
    descriptor_block_size = 24 + 16 * num_samples  # 88 bytes
    dfd_total_size = 4 + descriptor_block_size  # 92 bytes

    dfd = bytearray(dfd_total_size)
    # DFD total size
    struct.pack_into('<I', dfd, 0, dfd_total_size)
    # Descriptor block header (at offset 4)
    struct.pack_into('<I', dfd, 4, 0)  # vendorId=0, descriptorType=0
    struct.pack_into('<H', dfd, 8, 0)  # versionNumber=0 (KHR_DF_VERSION)
    struct.pack_into('<H', dfd, 10, descriptor_block_size)  # descriptorBlockSize
    # Color model = KHR_DF_MODEL_RGBSDA (1), primaries = BT709 (1), transfer = sRGB (2)
    dfd[12] = 1   # colorModel = RGBSDA
    dfd[13] = 1   # colorPrimaries = BT709
    dfd[14] = 2   # transferFunction = sRGB
    dfd[15] = 0   # flags
    # texelBlockDimension: 0,0,0,0 means 1x1x1x1
    dfd[16] = 0; dfd[17] = 0; dfd[18] = 0; dfd[19] = 0
    # bytesPlane0-7: bytesPlane0 = 4 (4 bytes per pixel)
    dfd[20] = 4; dfd[21] = 0; dfd[22] = 0; dfd[23] = 0
    dfd[24] = 0; dfd[25] = 0; dfd[26] = 0; dfd[27] = 0

    # Sample descriptions (16 bytes each): R, G, B, A
    channels = [0, 1, 2, 15]  # KHR_DF_CHANNEL_RGBSDA_RED=0, GREEN=1, BLUE=2, ALPHA=15
    for i, ch in enumerate(channels):
        off = 28 + i * 16
        struct.pack_into('<H', dfd, off, i * 8)        # bitOffset
        struct.pack_into('<B', dfd, off + 2, 7)         # bitLength (8-1=7)
        dfd[off + 3] = ch                               # channelType
        # samplePosition: 0,0,0,0
        dfd[off + 4] = 0; dfd[off + 5] = 0; dfd[off + 6] = 0; dfd[off + 7] = 0
        # sampleLower: 0
        struct.pack_into('<I', dfd, off + 8, 0)
        # sampleUpper: 255
        struct.pack_into('<I', dfd, off + 12, 255)

    # Layout:
    # [0..80)    header
    # [80..104)  level index (1 entry = 24 bytes)
    # [104..104+dfd_total_size) DFD
    # pixel data follows, aligned to 4 bytes
    level_index_end = 104
    dfd_offset = level_index_end
    dfd_end = dfd_offset + dfd_total_size
    # Align pixel data to 4 bytes (already aligned since dfd_total_size=92, 104+92=196)
    pixel_offset = (dfd_end + 3) & ~3
    total_size = pixel_offset + pixel_size

    buf = bytearray(total_size)

    # KTX2 header (80 bytes)
    buf[0:12] = KTX2_MAGIC
    struct.pack_into('<I', buf, 12, VK_FORMAT_R8G8B8A8_SRGB)  # vkFormat
    struct.pack_into('<I', buf, 16, 1)                          # typeSize
    struct.pack_into('<I', buf, 20, width)                      # pixelWidth
    struct.pack_into('<I', buf, 24, height)                     # pixelHeight
    struct.pack_into('<I', buf, 28, 0)                          # pixelDepth
    struct.pack_into('<I', buf, 32, 0)                          # layerCount
    struct.pack_into('<I', buf, 36, 1)                          # faceCount
    struct.pack_into('<I', buf, 40, 1)                          # levelCount
    struct.pack_into('<I', buf, 44, 0)                          # supercompressionScheme (none)

    # DFD byte offset and byte length
    struct.pack_into('<I', buf, 48, dfd_offset)                 # dfdByteOffset
    struct.pack_into('<I', buf, 52, dfd_total_size)             # dfdByteLength

    # KVD (key/value data) - empty
    struct.pack_into('<I', buf, 56, 0)                          # kvdByteOffset
    struct.pack_into('<I', buf, 60, 0)                          # kvdByteLength

    # SGD (supercompression global data) - none
    struct.pack_into('<Q', buf, 64, 0)                          # sgdByteOffset
    struct.pack_into('<Q', buf, 72, 0)                          # sgdByteLength

    # Level index (1 entry: byteOffset, byteLength, uncompressedByteLength)
    struct.pack_into('<Q', buf, 80, pixel_offset)               # byteOffset
    struct.pack_into('<Q', buf, 88, pixel_size)                 # byteLength
    struct.pack_into('<Q', buf, 96, pixel_size)                 # uncompressedByteLength

    # DFD
    buf[dfd_offset:dfd_offset + dfd_total_size] = dfd

    # Pixel data
    buf[pixel_offset:pixel_offset + pixel_size] = pixel_data

    return bytes(buf)


def main():
    ktx2_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)),
                            "assets", "bistro_exterior_ktx2")
    if not os.path.isdir(ktx2_dir):
        print(f"Directory not found: {ktx2_dir}")
        sys.exit(1)

    # Find files that are currently broken (our failed re-encode made them ~few hundred bytes)
    # or still BC7 with base < 4x4
    fixed = 0
    for path in sorted(glob.glob(os.path.join(ktx2_dir, "*.ktx2"))):
        with open(path, 'rb') as f:
            data = f.read(44)
        if len(data) < 44:
            continue

        # Check if magic is valid
        has_valid_magic = data[:12] == KTX2_MAGIC

        vk_format = struct.unpack_from('<I', data, 12)[0] if has_valid_magic else 0
        pw = struct.unpack_from('<I', data, 20)[0] if has_valid_magic else 0
        ph = struct.unpack_from('<I', data, 24)[0] if has_valid_magic else 0

        name = os.path.basename(path)

        # Fix if: broken magic, or base dimensions < 4 with BC7, or already "fixed" RGBA8 with < 4
        needs_fix = False
        if not has_valid_magic:
            needs_fix = True
        elif (pw < 4 or ph < 4) and vk_format in (145, 146, VK_FORMAT_R8G8B8A8_SRGB, 37):
            needs_fix = True

        if not needs_fix:
            continue

        # Generate a valid 4x4 RGBA8_SRGB KTX2
        # Use flat normal map color for Normal textures, gray for others
        if '_Normal' in name:
            pixel = bytes([128, 128, 255, 255])  # flat normal
        else:
            pixel = bytes([128, 128, 128, 255])  # neutral gray

        pixel_data = pixel * (4 * 4)  # 4x4 = 16 pixels
        ktx2_data = make_ktx2_rgba8(4, 4, pixel_data)

        with open(path, 'wb') as f:
            f.write(ktx2_data)

        print(f"REPLACED: {name} -> 4x4 RGBA8_SRGB ({len(ktx2_data)} bytes)")
        fixed += 1

    print(f"\nFixed {fixed} files")


if __name__ == "__main__":
    main()
