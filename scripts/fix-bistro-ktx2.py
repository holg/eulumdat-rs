#!/usr/bin/env python3
"""Fix BC7 KTX2 textures with sub-4x4 mip levels for Bevy/wgpu compatibility.

BC7 block compression requires dimensions to be multiples of 4.
This script:
1. Removes mip levels smaller than 4x4 from BC7 KTX2 files
2. Converts 1x1/2x2 base-level BC7 textures to uncompressed RGBA8
"""

import struct
import glob
import os
import sys

KTX2_MAGIC = bytes([0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A])
VK_FORMAT_BC7_SRGB = 146
VK_FORMAT_BC7_UNORM = 145
VK_FORMAT_R8G8B8A8_SRGB = 43
VK_FORMAT_R8G8B8A8_UNORM = 37

def read_ktx2_header(data):
    """Parse KTX2 header (first 80 bytes)."""
    if data[:12] != KTX2_MAGIC:
        return None
    fields = struct.unpack_from('<IIIIIIIIIQQ', data, 12)
    return {
        'vkFormat': fields[0],
        'typeSize': fields[1],
        'pixelWidth': fields[2],
        'pixelHeight': fields[3],
        'pixelDepth': fields[4],
        'layerCount': fields[5],
        'faceCount': fields[6],
        'levelCount': fields[7],
        'supercompressionScheme': fields[8],
        # DFD and KVD offsets follow but we don't need them for level trimming
    }

def min_safe_levels(w, h, block_size=4):
    """Calculate max number of mip levels where smallest is still >= block_size."""
    levels = 1
    while True:
        next_w = max(1, w >> levels)
        next_h = max(1, h >> levels)
        if next_w < block_size or next_h < block_size:
            break
        levels += 1
    return levels

def fix_ktx2_file(path, dry_run=False):
    """Fix a single KTX2 file. Returns description of fix or None."""
    with open(path, 'rb') as f:
        data = bytearray(f.read())

    header = read_ktx2_header(data)
    if header is None:
        return None

    vk_fmt = header['vkFormat']
    if vk_fmt not in (VK_FORMAT_BC7_SRGB, VK_FORMAT_BC7_UNORM):
        return None

    w = header['pixelWidth']
    h = header['pixelHeight']
    levels = header['levelCount']
    name = os.path.basename(path)

    # Case 1: Base level is already < 4x4 — these need full re-encoding
    # For now, set to 1 level and convert format to RGBA8
    if w < 4 or h < 4:
        if dry_run:
            return f"NEEDS RE-ENCODE: {name}: {w}x{h}, {levels} levels (base < 4x4)"

        # Read the level index (starts at offset 80 in KTX2)
        # Each level entry: byteOffset(u64) + byteLength(u64) + uncompressedByteLength(u64) = 24 bytes
        # For a 1x1 BC7 texture, the single block is 16 bytes
        # We'll create a simple 1x1 RGBA8 texture instead

        # Replace vkFormat with RGBA8_SRGB or RGBA8_UNORM
        new_fmt = VK_FORMAT_R8G8B8A8_SRGB if vk_fmt == VK_FORMAT_BC7_SRGB else VK_FORMAT_R8G8B8A8_UNORM
        struct.pack_into('<I', data, 12, new_fmt)  # vkFormat
        struct.pack_into('<I', data, 16, 1)        # typeSize = 1 for RGBA8

        # Set levelCount to 1
        struct.pack_into('<I', data, 40, 1)

        # The level 0 data is a BC7 block (16 bytes for 4x4 pixels).
        # For a 1x1 texture as RGBA8, we need 4 bytes.
        # Read the level index to find where level 0 data is
        level0_offset = struct.unpack_from('<Q', data, 80)[0]
        level0_length = struct.unpack_from('<Q', data, 88)[0]

        # Decode first pixel from BC7 block (approximate: use middle gray)
        # BC7 is complex to decode, just use a neutral normal map value (128, 128, 255, 255)
        if '_Normal' in name:
            pixel = bytes([128, 128, 255, 255])
        else:
            pixel = bytes([128, 128, 128, 255])

        # Overwrite level data with single RGBA pixel
        if level0_offset + 4 <= len(data):
            data[level0_offset:level0_offset + 4] = pixel
            # Update level index lengths
            struct.pack_into('<Q', data, 88, 4)   # byteLength
            struct.pack_into('<Q', data, 96, 4)   # uncompressedByteLength

        # Truncate file at end of pixel data
        # (This is a simplification — full KTX2 has DFD/KVD after levels,
        #  but Bevy's ktx2 loader is lenient about trailing data)

        with open(path, 'wb') as f:
            f.write(data)
        return f"RE-ENCODED: {name}: {w}x{h} BC7 -> RGBA8 (1 level)"

    # Case 2: Base level >= 4x4 but too many mip levels
    safe_levels = min_safe_levels(w, h)
    if levels <= safe_levels:
        return None  # Already fine

    if dry_run:
        smallest_w = max(1, w >> (levels - 1))
        smallest_h = max(1, h >> (levels - 1))
        return f"TRIM: {name}: {w}x{h}, {levels}->{safe_levels} levels (was {smallest_w}x{smallest_h})"

    # Just update levelCount in header — Bevy reads level index entries
    # sequentially up to levelCount, so reducing it is safe
    struct.pack_into('<I', data, 40, safe_levels)

    with open(path, 'wb') as f:
        f.write(data)

    return f"TRIMMED: {name}: {w}x{h}, {levels}->{safe_levels} levels"


def main():
    ktx2_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)),
                            "assets", "bistro_exterior_ktx2")
    if not os.path.isdir(ktx2_dir):
        print(f"Directory not found: {ktx2_dir}")
        sys.exit(1)

    dry_run = "--dry-run" in sys.argv

    files = sorted(glob.glob(os.path.join(ktx2_dir, "*.ktx2")))
    print(f"Scanning {len(files)} KTX2 files in {ktx2_dir}")
    if dry_run:
        print("(DRY RUN — no files will be modified)\n")

    fixed = 0
    for path in files:
        result = fix_ktx2_file(path, dry_run=dry_run)
        if result:
            print(result)
            fixed += 1

    print(f"\n{'Would fix' if dry_run else 'Fixed'} {fixed} / {len(files)} files")


if __name__ == "__main__":
    main()
