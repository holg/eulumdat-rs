#!/usr/bin/env python3
"""
Automated feature video for iesna.eu — Playwright + Edge TTS + ffmpeg.

Drives Chrome through every feature, records video, generates voiceover,
and composites into a single MP4. No manual editing needed.

Usage:
    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate
    python scripts/feature_video.py
    python scripts/feature_video.py --url http://localhost:8042 --scene 3
    python scripts/feature_video.py --dry-run
"""

import argparse
import asyncio
import os
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import edge_tts
from playwright.async_api import async_playwright, Page

# ============================================================================
# Scene definitions
# ============================================================================

@dataclass
class Scene:
    name: str
    narrator: str
    min_duration: float = 5.0  # seconds
    actions: list = field(default_factory=list)  # list of (action_fn_name, *args)


SCENES = [
    # Verified selectors on live iesna.eu:
    #   .luminaire-row            — dashboard row (click=select+expand, ctrl+click=compare)
    #   select.template-select    — dashboard template (default/aec/alternative)
    #   button.sidebar-icon-btn   — sidebar icons (edit/zoom/compare/designer/export)
    #   nav.main-tabs button      — editor main tabs
    #   .sub-tabs button          — editor sub-tabs (diagrams, etc.)
    #   .zonal-view-tab           — interior designer view tabs
    #   button.theme-toggle       — dark/light toggle
    #   select.language-selector-compact — language dropdown
    #   button:has-text('SI')     — unit toggle (changes to ft after click)
    #   button:has-text('←')      — back to dashboard

    # ═══ Act 1: Dashboard ═══
    Scene(
        name="01_dashboard",
        narrator="IESNA dot E-U — professional photometric analysis, in your browser. "
                 "Click any luminaire to see its detail panel.",
        min_duration=6,
        actions=[
            ("navigate", "/"),
            ("wait", 2),
            ("click", ".luminaire-row"),
            ("wait", 3),
        ],
    ),
    Scene(
        name="02_templates",
        narrator="Switch dashboard templates — Standard, AEC, Alternative. "
                 "Each layout highlights different photometric aspects.",
        min_duration=8,
        actions=[
            ("select", "select.template-select", "aec"),
            ("wait", 3),
            ("select", "select.template-select", "alternative"),
            ("wait", 3),
            ("select", "select.template-select", "default"),
            ("wait", 1),
        ],
    ),

    # ═══ Act 2: Compare (from dashboard) ═══
    Scene(
        name="03_compare",
        narrator="Command-click a second luminaire to compare. "
                 "Side-by-side polar overlays, cartesian curves, heatmaps.",
        min_duration=8,
        actions=[
            ("ctrl_click", ".luminaire-row >> nth=1"),
            ("wait", 2),
            ("click", "button.sidebar-icon-btn[title='Open compare view']"),
            ("wait", 3),
            ("scroll_down", 200),
            ("wait", 2),
        ],
    ),

    # ═══ Act 3: Editor (stay here for all tabs) ═══
    Scene(
        name="04_general",
        narrator="Open the editor. The General tab shows I-E-S distribution type, "
                 "B-U-G rating, cutoff classification, dark-sky zone.",
        min_duration=10,
        actions=[
            ("click", "button:has-text('←')"),
            ("wait", 1),
            ("click", "button.sidebar-icon-btn[title='Edit luminaire data']"),
            ("wait", 2),
            ("scroll_down", 300),
            ("wait", 2),
            ("scroll_down", 300),
            ("wait", 2),
            ("scroll_to_top",),
            ("wait", 1),
        ],
    ),
    Scene(
        name="05_diagrams",
        narrator="Diagrams — polar, cartesian, heatmap, three-D butterfly, cone, "
                 "and floodlight ISO views.",
        min_duration=14,
        actions=[
            ("click", "nav.main-tabs button:has-text('Diagrams')"),
            ("wait", 2),
            ("click", ".sub-tabs button:has-text('Heatmap')"),
            ("wait", 2),
            ("click", ".sub-tabs button:has-text('3D')"),
            ("wait", 2),
            ("click", ".sub-tabs button:has-text('Cone')"),
            ("wait", 2),
            ("click", ".sub-tabs button:has-text('ISO View')"),
            ("wait", 2),
            ("click", ".sub-tabs button:has-text('2D')"),
            ("wait", 2),
        ],
    ),
    Scene(
        name="06_analysis",
        narrator="Analysis — spectral distribution, B-U-G ratings, "
                 "luminaire classification.",
        min_duration=6,
        actions=[
            ("click", "nav.main-tabs button:has-text('Analysis')"),
            ("wait", 3),
            ("scroll_down", 300),
            ("wait", 2),
        ],
    ),
    Scene(
        name="07_language",
        narrator="Switch languages — watch diagrams adapt. "
                 "Toggle metric and imperial. Dark mode and back.",
        min_duration=12,
        actions=[
            ("select_by_text", "select.language-selector-compact", "Deutsch"),
            ("wait", 2),
            ("select_by_text", "select.language-selector-compact", "Français"),
            ("wait", 2),
            ("select_by_text", "select.language-selector-compact", "English"),
            ("wait", 1),
            ("click", "button:has-text('SI')"),
            ("wait", 2),
            ("click", "button:has-text('SI'), button:has-text('ft')"),
            ("wait", 1),
            ("click", "button.theme-toggle"),
            ("wait", 2),
            ("click", "button.theme-toggle"),
            ("wait", 1),
        ],
    ),
    Scene(
        name="08_validation",
        narrator="Validation against EULUMDAT, ATLA, T-M thirty-three, T-M thirty-two.",
        min_duration=6,
        actions=[
            ("click", "nav.main-tabs button:has-text('Validation')"),
            ("wait", 3),
            ("scroll_down", 200),
            ("wait", 2),
        ],
    ),

    # ═══ Act 4: Designers (stay in editor) ═══
    Scene(
        name="09_interior",
        narrator="Interior Lighting Designer — zonal cavity illuminance. "
                 "Toggle Show Values for point-by-point numbers. "
                 "Schedule tab for U-S standards.",
        min_duration=14,
        actions=[
            ("click", "nav.main-tabs button:has-text('Interior')"),
            ("wait", 3),
            ("click", ".zonal-view-tab:has-text('Heatmap')"),
            ("wait", 2),
            ("click", "label:has-text('Show values')"),
            ("wait", 3),
            ("click", ".zonal-view-tab:has-text('Schedule')"),
            ("wait", 2),
            ("click", ".zonal-view-tab:has-text('CU Table')"),
            ("wait", 2),
        ],
    ),
    Scene(
        name="10_area",
        narrator="Area Lighting Designer — outdoor spaces, pole grids, "
                 "illuminance contours.",
        min_duration=8,
        actions=[
            ("click", "nav.main-tabs button:has-text('Designer')"),
            ("wait", 3),
            ("scroll_down", 300),
            ("wait", 3),
        ],
    ),
    Scene(
        name="11_maps",
        narrator="Maps Designer — Google Maps integration, polygon areas, "
                 "luminaire placement on satellite view.",
        min_duration=6,
        actions=[
            ("click", "nav.main-tabs button:has-text('Maps')"),
            ("wait", 4),
        ],
    ),

    # ═══ Act 5: 3D & Advanced ═══
    Scene(
        name="12_3d",
        narrator="Three-D Scene viewer — Bevy engine, walk around with W-A-S-D.",
        min_duration=7,
        actions=[
            ("click", "nav.main-tabs button:has-text('3D')"),
            ("wait", 5),
        ],
    ),
    Scene(
        name="13_goniosim",
        narrator="Virtual goniophotometer — trace photons through optical covers. "
                 "Compare original and simulated distributions. "
                 "Validated against C-I-E one-seventy-one.",
        min_duration=10,
        actions=[
            ("navigate", "/?wasm=goniosim"),
            ("wait", 3),
            ("click", "button:has-text('Trace')"),
            ("wait", 5),
        ],
    ),

    # ═══ Act 6: Export + ATLA ═══
    Scene(
        name="14_export",
        narrator="Export as P-D-F, L-D-T, I-E-S, or ATLA format. "
                 "Internally, the app uses ATLA S-0-0-1 — "
                 "backwards compatible with all L-D-T and I-E-S versions, "
                 "carries B-I-M parameters, NEMA GUIDs, spectral data. "
                 "A true open standard bridging all formats.",
        min_duration=10,
        actions=[
            ("navigate", "/"),
            ("wait", 2),
            ("click", "button.file-menu-toggle >> nth=0"),
            ("wait", 3),
            ("key_press", "Escape"),
            ("wait", 1),
        ],
    ),

    # ═══ Act 7: Outro ═══
    Scene(
        name="15_outro",
        narrator="IESNA dot E-U — open source, in your browser, no install needed. "
                 "But stay tuned.",
        min_duration=4,
        actions=[("wait", 3)],
    ),

    # ═══ Act 8: Obscura ═══
    Scene(
        name="16_obscura_launch",
        narrator="The Obscura darkness preservation simulator — "
                 "Bevy engine, WebGPU, right in the browser.",
        min_duration=20,
        actions=[
            ("navigate", "/?wasm=obscura_demo"),
            ("wait", 3),
            ("click", "button:has-text('Launch')"),
            ("wait", 15),
        ],
    ),
    Scene(
        name="17_obscura_sliders",
        narrator="Uplight percentage controls sky glow. "
                 "Let's increase it with the right bracket key.",
        min_duration=10,
        actions=[
            ("click", "#obscura-canvas"),
            ("wait", 0.5),
            ("key_press", "]"),
            ("wait", 1.5),
            ("key_press", "]"),
            ("wait", 1.5),
            ("key_press", "]"),
            ("wait", 1.5),
            ("mouse_drag", "#obscura-canvas", 960, 540, 700, 400, 2.0),
            ("wait", 1),
        ],
    ),
    Scene(
        name="18_obscura_haze",
        narrator="Increase haze — atmospheric scattering amplifies sky glow.",
        min_duration=8,
        actions=[
            ("click", "#obscura-canvas"),
            ("key_press", "0"),
            ("wait", 1.5),
            ("key_press", "0"),
            ("wait", 1.5),
            ("key_press", "0"),
            ("wait", 1.5),
            ("mouse_drag", "#obscura-canvas", 960, 540, 1100, 500, 2.0),
            ("wait", 1),
        ],
    ),
    Scene(
        name="19_obscura_sponza",
        narrator="Press one — Sponza Atrium, a classic test scene.",
        min_duration=10,
        actions=[
            ("click", "#obscura-canvas"),
            ("key_press", "1"),
            ("wait", 6),
            ("mouse_drag", "#obscura-canvas", 960, 540, 600, 380, 2.0),
            ("key_hold", "w", 1.5),
            ("wait", 1),
        ],
    ),
    Scene(
        name="20_obscura_bistro",
        narrator="Press three — Bistro Exterior. Twenty-five megabytes, "
                 "cobblestone streets, building facades, all in the browser.",
        min_duration=18,
        actions=[
            ("click", "#obscura-canvas"),
            ("key_press", "3"),
            ("wait", 12),
            ("key_hold", "w", 2.0),
            ("mouse_drag", "#obscura-canvas", 960, 540, 700, 540, 1.5),
            ("key_hold", "w", 2.0),
            ("wait", 1),
        ],
    ),
    Scene(
        name="21_obscura_walk",
        narrator="Photometric lights use actual L-D-T road luminaire data.",
        min_duration=12,
        actions=[
            ("click", "#obscura-canvas"),
            ("mouse_drag", "#obscura-canvas", 960, 540, 600, 540, 2.0),
            ("key_hold", "w", 3.0),
            ("mouse_drag", "#obscura-canvas", 600, 540, 1100, 480, 2.0),
            ("key_hold", "w", 2.0),
            ("wait", 1),
        ],
    ),
    Scene(
        name="22_obscura_sky",
        narrator="Look up. Rise with Q above the rooftops.",
        min_duration=8,
        actions=[
            ("click", "#obscura-canvas"),
            ("mouse_drag", "#obscura-canvas", 960, 540, 960, 250, 2.5),
            ("key_hold", "q", 3.0),
            ("wait", 1),
        ],
    ),
    Scene(
        name="23_obscura_reduce",
        narrator="Lower uplight percentage — less light going into the sky.",
        min_duration=10,
        actions=[
            ("click", "#obscura-canvas"),
            ("key_press", "["),
            ("wait", 1.2),
            ("key_press", "["),
            ("wait", 1.2),
            ("key_press", "["),
            ("wait", 1.2),
            ("key_press", "["),
            ("wait", 1.2),
            ("key_press", "["),
            ("wait", 1.2),
        ],
    ),
    Scene(
        name="24_obscura_clear",
        narrator="Reduce haze and intensity.",
        min_duration=8,
        actions=[
            ("click", "#obscura-canvas"),
            ("key_press", "9"),
            ("wait", 1),
            ("key_press", "9"),
            ("wait", 1),
            ("key_press", "9"),
            ("wait", 1),
            ("key_press", "-"),
            ("wait", 1),
            ("key_press", "-"),
            ("wait", 1),
        ],
    ),
    Scene(
        name="25_obscura_stars",
        narrator="The stars are emerging. "
                 "Each star uses multiple lights for color temperature and spectral accuracy. "
                 "We apologize for the oversized stars — "
                 "that's the trade-off for rendering color and brightness faithfully. "
                 "This is what our night sky could look like.",
        min_duration=15,
        actions=[
            ("click", "#obscura-canvas"),
            ("key_press", "["),
            ("wait", 2),
            ("mouse_drag", "#obscura-canvas", 960, 300, 700, 250, 3.0),
            ("wait", 2),
            ("mouse_drag", "#obscura-canvas", 700, 250, 1100, 300, 3.0),
            ("wait", 3),
        ],
    ),

    # ═══ Act 9: One more thing — Quiz ═══
    Scene(
        name="26_one_more_thing",
        narrator="And... one more thing.",
        min_duration=3,
        actions=[("wait", 2)],
    ),
    Scene(
        name="27_quiz",
        narrator="Test your lighting knowledge. One hundred seventy-five questions "
                 "across sixteen categories, in eight languages.",
        min_duration=12,
        actions=[
            ("navigate", "/quiz.html"),
            ("wait", 3),
            ("click", "button:has-text('Start')"),
            ("wait", 2),
            ("click", ".option-btn >> nth=0"),
            ("wait", 2),
            ("click", "button:has-text('Next')"),
            ("wait", 2),
        ],
    ),
]

# ============================================================================
# Browser action helpers
# ============================================================================

async def execute_action(page: Page, action: tuple, base_url: str):
    """Execute a single browser action."""
    cmd = action[0]

    if cmd == "navigate":
        path = action[1]
        url = base_url.rstrip("/") + path
        await page.goto(url, wait_until="networkidle", timeout=30000)
    elif cmd == "wait":
        await page.wait_for_timeout(int(action[1] * 1000))
    elif cmd == "click":
        selector = action[1]
        try:
            await page.click(selector, timeout=5000)
        except Exception as e:
            print(f"  [warn] click '{selector}' failed: {e}")
    elif cmd == "click_tab":
        tab_name = action[1]
        try:
            await page.click(f".tab:has-text('{tab_name}')", timeout=5000)
        except Exception:
            # Try main tab buttons
            try:
                await page.click(f"button:has-text('{tab_name}')", timeout=3000)
            except Exception as e:
                print(f"  [warn] tab '{tab_name}' not found: {e}")
    elif cmd == "click_subtab":
        name = action[1]
        try:
            await page.click(f".tab:has-text('{name}')", timeout=3000)
        except Exception:
            try:
                await page.click(f"button:has-text('{name}')", timeout=3000)
            except Exception as e:
                print(f"  [warn] subtab '{name}' not found: {e}")
    elif cmd == "select":
        selector, value = action[1], action[2]
        try:
            await page.select_option(selector, value=value, timeout=5000)
        except Exception as e:
            print(f"  [warn] select '{selector}' failed: {e}")
    elif cmd == "scroll_down":
        pixels = action[1]
        await page.evaluate(f"window.scrollBy(0, {pixels})")
    elif cmd == "scroll_to_top":
        await page.evaluate("window.scrollTo(0, 0)")
    elif cmd == "type":
        selector, text = action[1], action[2]
        await page.fill(selector, text)
    elif cmd == "select_by_text":
        # Try multiple selectors, select by visible text
        selectors = action[1].split(", ")
        text = action[2]
        for sel in selectors:
            try:
                await page.select_option(sel.strip(), label=text, timeout=2000)
                break
            except Exception:
                continue
    elif cmd == "mouse_drag":
        # Drag from (x1,y1) to (x2,y2) over duration seconds on a selector.
        # Coordinates are in 1920x1080 space — scaled to actual element size.
        selector = action[1]
        x1, y1, x2, y2 = action[2], action[3], action[4], action[5]
        duration = action[6] if len(action) > 6 else 1.0
        steps = int(duration * 30)
        try:
            el = await page.query_selector(selector)
            if el:
                box = await el.bounding_box()
                if box:
                    bx, by, bw, bh = box["x"], box["y"], box["width"], box["height"]
                    # Scale from 1920x1080 reference to actual size
                    sx = bw / 1920.0
                    sy = bh / 1080.0
                    ax1, ay1 = bx + x1 * sx, by + y1 * sy
                    ax2, ay2 = bx + x2 * sx, by + y2 * sy
                    await page.mouse.move(ax1, ay1)
                    await page.mouse.down(button="right")
                    for step in range(steps):
                        t = (step + 1) / steps
                        mx = ax1 + (ax2 - ax1) * t
                        my = ay1 + (ay2 - ay1) * t
                        await page.mouse.move(mx, my)
                        await page.wait_for_timeout(int(1000 / 30))
                    await page.mouse.up(button="right")
        except Exception as e:
            print(f"  [warn] mouse_drag failed: {e}")
    elif cmd == "key_hold":
        # Hold a key for duration seconds
        key = action[1]
        duration = action[2] if len(action) > 2 else 1.0
        await page.keyboard.down(key)
        await page.wait_for_timeout(int(duration * 1000))
        await page.keyboard.up(key)
    elif cmd == "key_press":
        key = action[1]
        await page.keyboard.press(key)
    elif cmd == "mouse_wheel":
        # Scroll mouse wheel on a selector
        selector = action[1]
        delta_y = action[2]  # negative = zoom in, positive = zoom out
        try:
            el = await page.query_selector(selector)
            if el:
                box = await el.bounding_box()
                if box:
                    cx = box["x"] + box["width"] / 2
                    cy = box["y"] + box["height"] / 2
                    await page.mouse.move(cx, cy)
                    await page.mouse.wheel(0, delta_y)
        except Exception as e:
            print(f"  [warn] mouse_wheel failed: {e}")
    elif cmd == "ctrl_click":
        selector = action[1]
        try:
            await page.click(selector, modifiers=["Meta"], timeout=5000)  # Meta = Cmd on Mac
        except Exception as e:
            print(f"  [warn] ctrl_click '{selector}' failed: {e}")
    elif cmd == "click_at":
        # Click at specific coordinates within a selector
        selector = action[1]
        x, y = action[2], action[3]
        try:
            el = await page.query_selector(selector)
            if el:
                box = await el.bounding_box()
                if box:
                    await page.mouse.click(box["x"] + x, box["y"] + y)
        except Exception as e:
            print(f"  [warn] click_at '{selector}' ({x},{y}) failed: {e}")
    elif cmd == "wait_for_hidden":
        # Wait for an element to disappear (e.g. loading overlay)
        selector = action[1]
        timeout = action[2] if len(action) > 2 else 30000
        try:
            await page.wait_for_selector(selector, state="hidden", timeout=timeout)
        except Exception as e:
            print(f"  [warn] wait_for_hidden '{selector}' timed out: {e}")
    else:
        print(f"  [warn] unknown action: {cmd}")


# ============================================================================
# TTS generation
# ============================================================================

async def generate_tts(text: str, output_path: str, voice: str = "en-US-GuyNeural"):
    """Generate TTS audio using edge-tts."""
    communicate = edge_tts.Communicate(text, voice)
    await communicate.save(output_path)
    return output_path


def get_audio_duration(path: str) -> float:
    """Get duration of an audio file in seconds via ffprobe."""
    result = subprocess.run(
        ["ffprobe", "-v", "quiet", "-show_entries", "format=duration",
         "-of", "csv=p=0", path],
        capture_output=True, text=True
    )
    try:
        return float(result.stdout.strip())
    except ValueError:
        return 5.0


# ============================================================================
# Video assembly
# ============================================================================

def combine_scene_video_audio(video_path: str, audio_path: str, output_path: str):
    """Combine a video and audio file, matching durations."""
    audio_dur = get_audio_duration(audio_path)
    video_dur = get_video_duration(video_path)

    # Pad or trim video to match audio
    subprocess.run([
        "ffmpeg", "-y",
        "-i", video_path,
        "-i", audio_path,
        "-c:v", "libx264", "-preset", "fast", "-crf", "23",
        "-c:a", "aac", "-b:a", "128k",
        "-shortest",
        "-movflags", "+faststart",
        output_path,
    ], capture_output=True)


def get_video_duration(path: str) -> float:
    """Get duration of a video file."""
    result = subprocess.run(
        ["ffprobe", "-v", "quiet", "-show_entries", "format=duration",
         "-of", "csv=p=0", path],
        capture_output=True, text=True
    )
    try:
        return float(result.stdout.strip())
    except ValueError:
        return 5.0


def concatenate_videos(scene_videos: list[str], output_path: str):
    """Concatenate multiple MP4 files into one."""
    concat_file = output_path + ".concat.txt"
    with open(concat_file, "w") as f:
        for v in scene_videos:
            f.write(f"file '{os.path.abspath(v)}'\n")

    subprocess.run([
        "ffmpeg", "-y",
        "-f", "concat", "-safe", "0",
        "-i", concat_file,
        "-c", "copy",
        "-movflags", "+faststart",
        output_path,
    ], capture_output=True)

    os.unlink(concat_file)


# ============================================================================
# Main orchestration
# ============================================================================

async def record_scene(
    page: Page,
    scene: Scene,
    work_dir: str,
    base_url: str,
    voice: str,
    dry_run: bool = False,
) -> str | None:
    """Record one scene: execute actions + generate TTS. Returns combined MP4 path."""
    print(f"\n{'='*60}")
    print(f"Scene: {scene.name}")
    print(f"Narrator: {scene.narrator[:80]}...")

    # Generate TTS audio
    audio_path = os.path.join(work_dir, f"{scene.name}.mp3")
    if not dry_run:
        await generate_tts(scene.narrator, audio_path, voice)
        audio_dur = get_audio_duration(audio_path)
        print(f"  Audio: {audio_dur:.1f}s")
    else:
        audio_dur = len(scene.narrator) / 15.0  # ~15 chars/sec estimate
        print(f"  Audio (est): {audio_dur:.1f}s")

    if dry_run:
        return None

    # Calculate how long to record
    record_dur = max(audio_dur + 1.0, scene.min_duration)

    # Execute browser actions (video is already recording via context)
    total_action_time = 0
    for action in scene.actions:
        await execute_action(page, action, base_url)
        if action[0] == "wait":
            total_action_time += action[1]

    # If actions were shorter than audio, pad with idle time
    remaining = record_dur - total_action_time
    if remaining > 0:
        await page.wait_for_timeout(int(remaining * 1000))

    return audio_path


async def main():
    parser = argparse.ArgumentParser(description="Generate iesna.eu feature video")
    parser.add_argument("--url", default="https://iesna.eu", help="Base URL")
    parser.add_argument("--voice", default="en-US-GuyNeural", help="Edge TTS voice")
    parser.add_argument("--output", default="feature_video.mp4", help="Output file")
    parser.add_argument("--resolution", default="2560x1440", help="WxH")
    parser.add_argument("--scene", type=int, help="Record only this scene (1-based)")
    parser.add_argument("--dry-run", action="store_true", help="Print scenes, no recording")
    parser.add_argument("--headed", action="store_true", default=True, help="Show browser")
    args = parser.parse_args()

    width, height = map(int, args.resolution.split("x"))
    # Use ./tmp/ for work files (not system temp)
    script_dir = Path(__file__).resolve().parent.parent
    work_dir = str(script_dir / "tmp" / "iesna_video")
    os.makedirs(work_dir, exist_ok=True)
    print(f"Work dir: {work_dir}")

    scenes = SCENES
    if args.scene:
        idx = args.scene - 1
        if 0 <= idx < len(scenes):
            scenes = [scenes[idx]]
        else:
            print(f"Scene {args.scene} out of range (1-{len(SCENES)})")
            return

    if args.dry_run:
        print(f"\n{'='*60}")
        print(f"DRY RUN — {len(scenes)} scenes")
        print(f"URL: {args.url}")
        print(f"Voice: {args.voice}")
        total_est = 0
        for i, s in enumerate(scenes, 1):
            est = max(len(s.narrator) / 15.0, s.min_duration)
            total_est += est
            print(f"\n  {i:2d}. {s.name} (~{est:.0f}s)")
            print(f"      \"{s.narrator[:100]}...\"" if len(s.narrator) > 100 else f"      \"{s.narrator}\"")
        print(f"\nEstimated total: {total_est:.0f}s ({total_est/60:.1f}min)")
        return

    # --- Step 1: Pre-generate ALL TTS audio files ---
    print(f"\n{'='*60}")
    print("Step 1: Generating TTS audio for all scenes...")
    audio_paths = []
    for scene in scenes:
        audio_path = os.path.join(work_dir, f"{scene.name}.mp3")
        await generate_tts(scene.narrator, audio_path, args.voice)
        dur = get_audio_duration(audio_path)
        audio_paths.append(audio_path)
        print(f"  {scene.name}: {dur:.1f}s")

    # --- Step 2: Record ONE continuous browser session ---
    print(f"\n{'='*60}")
    print("Step 2: Recording browser session (single window)...")

    video_dir = os.path.join(work_dir, "video")
    os.makedirs(video_dir, exist_ok=True)

    async with async_playwright() as p:
        context = await p.chromium.launch_persistent_context(
            user_data_dir="",
            headless=False,
            viewport={"width": width, "height": height},
            record_video_dir=video_dir,
            record_video_size={"width": width, "height": height},
            args=[
                "--disable-blink-features=AutomationControlled",
                "--autoplay-policy=no-user-gesture-required",
            ],
        )
        page = context.pages[0] if context.pages else await context.new_page()

        # Navigate to start
        await page.goto(args.url, wait_until="networkidle", timeout=30000)
        await page.wait_for_timeout(1000)

        # Track timestamps for each scene (for audio sync)
        scene_timestamps = []  # (start_sec, audio_path)
        recording_start = asyncio.get_event_loop().time()

        for i, scene in enumerate(scenes):
            scene_start = asyncio.get_event_loop().time() - recording_start
            scene_timestamps.append((scene_start, audio_paths[i]))

            print(f"\n  [{i+1}/{len(scenes)}] {scene.name} (t={scene_start:.1f}s)")

            # Calculate target duration from TTS audio
            audio_dur = get_audio_duration(audio_paths[i])
            target_dur = max(audio_dur + 0.5, scene.min_duration)

            # Execute actions
            action_start = asyncio.get_event_loop().time()
            for action in scene.actions:
                await execute_action(page, action, args.url)

            # Pad to match audio duration
            elapsed = asyncio.get_event_loop().time() - action_start
            remaining = target_dur - elapsed
            if remaining > 0:
                await page.wait_for_timeout(int(remaining * 1000))

        # Small pause at the end
        await page.wait_for_timeout(2000)

        total_recording = asyncio.get_event_loop().time() - recording_start
        print(f"\n  Total recording time: {total_recording:.1f}s")

        # Close to finalize video
        await context.close()

    # --- Step 3: Find the recorded video ---
    video_files = list(Path(video_dir).glob("*.webm"))
    if not video_files:
        print("[error] No video file found!")
        return
    raw_video = str(video_files[0])
    print(f"\nRaw video: {raw_video} ({get_video_duration(raw_video):.1f}s)")

    # --- Step 4: Mix all TTS audio into one track, timed to scene starts ---
    print(f"\n{'='*60}")
    print("Step 3: Mixing audio track...")

    # Build ffmpeg filter to place each audio at its scene timestamp
    mixed_audio = os.path.join(work_dir, "mixed_audio.mp3")
    inputs = []
    filter_parts = []
    for i, (start_sec, apath) in enumerate(scene_timestamps):
        inputs.extend(["-i", apath])
        # Delay each audio clip to its scene start time
        delay_ms = int(start_sec * 1000)
        filter_parts.append(f"[{i}:a]adelay={delay_ms}|{delay_ms}[a{i}]")

    # Mix all delayed audio streams
    mix_inputs = "".join(f"[a{i}]" for i in range(len(scene_timestamps)))
    filter_parts.append(f"{mix_inputs}amix=inputs={len(scene_timestamps)}:duration=longest[aout]")
    filter_str = ";".join(filter_parts)

    cmd = ["ffmpeg", "-y"] + inputs + [
        "-filter_complex", filter_str,
        "-map", "[aout]",
        "-c:a", "libmp3lame", "-b:a", "192k",
        mixed_audio,
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  [warn] Audio mix failed: {result.stderr[:200]}")
        # Fallback: just use first audio
        shutil.copy(audio_paths[0], mixed_audio)

    print(f"  Mixed audio: {get_audio_duration(mixed_audio):.1f}s")

    # --- Step 5: Combine video + mixed audio → final MP4 ---
    print(f"\n{'='*60}")
    print("Step 4: Final composite...")

    output = args.output
    subprocess.run([
        "ffmpeg", "-y",
        "-i", raw_video,
        "-i", mixed_audio,
        "-c:v", "libx264", "-preset", "fast", "-crf", "22",
        "-c:a", "aac", "-b:a", "192k",
        "-shortest",
        "-movflags", "+faststart",
        output,
    ], capture_output=True)

    if os.path.exists(output) and os.path.getsize(output) > 0:
        print(f"\nDone! Output: {output}")
        print(f"Duration: {get_video_duration(output):.1f}s")
        print(f"Size: {os.path.getsize(output) / 1024 / 1024:.1f} MB")
    else:
        print(f"\n[error] Failed to create {output}")

    print(f"\nWork dir (for debugging): {work_dir}")


if __name__ == "__main__":
    asyncio.run(main())
