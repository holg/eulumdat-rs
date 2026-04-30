# Safari WebGPU — Bloom-induced UI overlay flicker

## Status

Open upstream issue (Bevy / wgpu). **Mostly mitigated** in this
repository as of v0.7.0 by the WebGL2 fallback bundle:

- Safari macOS pre-Tahoe (no WebGPU) → loads WebGL2 bundle → no Bloom →
  no flicker
- Safari iOS / iPadOS without the WebGPU feature flag (the default) →
  loads WebGL2 bundle → no flicker
- **Only Safari Tahoe (macOS 26) + future iOS with WebGPU enabled by
  default** still hit the flicker. That's a narrow and shrinking
  audience, and they at least see the scene render.

The upstream fix is still wanted — at some point WebGPU coverage will
be the norm and the WebGL2 fallback will be retired — but the
production impact is much smaller than it was before the dual-bundle
landed.

## Symptoms

In the Skyglow demo (`crates/eulumdat-bevy/examples/skyglow_demo.rs`),
the dashboard overlay panel (top-left, "Skyglow Analysis" + sliders +
text labels) flickers at high frequency on Safari. The 3D scene
underneath renders correctly. The flicker is independent of which scene
is loaded (Bistro, Sponza, Urban).

Confirmed on:
- Safari 18+ macOS (Sonoma / Sequoia / Tahoe)
- Safari iOS / iPadOS with the WebGPU feature flag enabled

Does **not** reproduce on:
- Chrome/Edge/Brave on any platform
- Firefox on any platform
- Native (`cargo run --example skyglow_demo`)
- The standalone `eulumdat-3d` viewer (which doesn't use Bloom)

## Reproduction

1. Open Skyglow at `https://iesna.eu/?wasm=skyglow_demo` in Safari with
   WebGPU enabled.
2. Click "Launch Demo".
3. Once the scene renders, the top-left dashboard overlay strobes.
4. The 3D view (stars, buildings, road) is stable.

The console shows no errors related to the flicker (only a benign
`pisa_*.ktx2.meta` 404 from Bevy's optional asset metadata lookup,
unrelated). `AdapterInfo` is all-blank — Safari doesn't expose adapter
info to WebGPU, which is per-spec and not the cause.

## What's been ruled out

| Suspect                                | Verdict |
| -------------------------------------- | ------- |
| Translucent UI panel backgrounds       | Made `PANEL_BG` / `SECTION_BG` opaque (`srgb` instead of `srgba`); flicker persisted. |
| Translucent loading-overlay panel      | Hidden via `Visibility::Hidden` after load; not in scene graph during flicker. |
| KTX2 zstd environment maps             | `.ktx2` files load successfully; only `.meta` sidecars 404, which Bevy treats as default. |
| Canvas resize / `fit_canvas_to_parent` | Whole canvas would flicker, not just the UI region. |
| Per-frame state mutation               | `update_ui_from_state` is gated on `state.is_changed()`; it does not run every frame. |
| `update_fog` writing every frame       | Mutates `DistanceFog`, not UI components. |

## Most likely root cause

Bloom + HDR + UI on Safari WebGPU.

The Skyglow camera stack is:

```rust
Camera3d::default(),
Hdr,
Tonemapping::AgX,
Bloom { intensity: 0.08, ..default() },
EnvironmentMapLight { ... },
```

Bevy renders the UI into the HDR target *before* the tonemap pass. The
Bloom pass then samples that target — including UI pixels — through a
chain of `rgba16float` storage-texture downsamples and upsamples. On
Safari WebGPU specifically, the Bloom mip chain appears to produce
slightly different output frame-to-frame in the UI-pixel region,
visible as a fast strobe once the result composites back over the UI.

The Skyglow scene also contains very-high HDR emissives (stars at
`emissive ~300-500` in linear space, moon at `~400`). Whether those
amplify the Safari precision/scheduling drift in the Bloom downsample
is unconfirmed but plausible.

Disabling Bloom **eliminates** the flicker on Safari. We confirmed this
locally with a temporary `#[cfg(target_arch = "wasm32")]` patch that
set `Bloom { intensity: 0.0, .. }`.

## Why we are not patching application code

The flicker is a *rendering* problem, not an *application* problem.
Putting `cfg` switches in `skyglow_demo.rs` to disable Bloom on WASM:

- Loses visual fidelity for Chrome/Firefox users (who render Bloom fine)
- Pushes platform-specific compositor knowledge into the demo
- Leaks an upstream bug into application surface area
- Doesn't help downstream Bevy users with the same browser/feature mix

The right place to fix this is in Bevy's Bloom render-graph node or in
wgpu's Safari/WebKit backend. Likely investigation paths:

1. Are Bloom's downsample/upsample storage-texture binding flags
   (`STORAGE_BINDING | TEXTURE_BINDING`) being negotiated correctly on
   Safari's WebGPU adapter? If Safari falls back to a non-storage path,
   the result may sample stale memory.
2. Does Safari's WebGPU command-buffer scheduling reorder Bloom passes
   relative to the UI pass in a way that introduces frame-N ↔ frame-N-1
   contamination?
3. Is the Bloom downsample shader emitting `NaN`/`Inf` for HDR pixels
   above a certain magnitude on Safari's MSL transpile path?

## Reference points for upstream

- Bevy fork in use: `holg/bevy.git#photometric_proposal` (commit
  `68124408` at time of writing)
- Bevy version base: `0.19.0-dev`
- wgpu backend reported by the runtime: `BrowserWebGpu`
- Affected Bevy modules: `bevy_post_process::bloom`,
  `bevy_render::renderer`, possibly `bevy_ui_render`

## Workarounds

For users who hit this in the meantime:

- Use Chrome, Edge, or Firefox to view the Skyglow demo.
- The rest of the eulumdat editor (file open, diagrams, validation,
  street designer, goniosim) does not use Bloom and renders correctly
  on Safari.

## Will revisit when

- Bevy main lands a fix in `bevy_post_process::bloom`
- wgpu lands Safari-specific compositor tweaks
- `holg/bevy.git#photometric_proposal` rebases onto a fixed Bevy main

At that point, we re-test in Safari and remove this note.
