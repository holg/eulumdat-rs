# eulumdat-bevy knowledge base

A talking-points reference for presenting the **eulumdat-bevy** crate and
the upstream **`pbr_photometric_lights`** Bevy proposal. Written to
prepare for technical Q&A — covers what we do, why we do it, where the
seams are, and what the upstream conversation looks like.

---

## 1. What eulumdat-bevy is

`eulumdat-bevy` is a Bevy plugin that drives Bevy's lighting from **real
photometric data files** — the same EULUMDAT (`.ldt`) and IES
(`.ies` / LM-63) files lighting designers use in Dialux / Relux / AGi32.

Two layers:

- **`PhotometricPlugin<T>`** — minimal plugin (`src/photometric/`). Takes
  any type implementing the `PhotometricData` trait. Spawns Bevy
  `PointLight` + `SpotLight` entities driven by the data. No scene, no
  camera, no controls — embeddable in any Bevy app.
- **`eulumdat-3d` viewer** — a complete Bevy app on top of the plugin
  (`src/viewer/`, `src/main.rs`). Adds scenes, fly-camera, controls,
  the photometric solid visualization. This is what users see when
  they open `https://iesna.eu/?wasm=eulumdat-3d` or run the standalone
  desktop binary.
- **Skyglow demo** — a second binary (`examples/skyglow_demo.rs`)
  that uses the plugin to render a Bistro street scene with two LDT
  files swapping at runtime. The light-pollution outreach piece.

Source: `crates/eulumdat-bevy/`.

---

## 2. The `PhotometricData` trait — the seam

`src/photometric/data.rs` defines a single trait that any photometric
source (LDT, IES, GLDF, ATLA, custom) implements:

```rust
pub trait PhotometricData: Send + Sync + Clone + Debug + 'static {
    fn sample(&self, c_angle: f64, g_angle: f64) -> f64; // cd/klm
    fn max_intensity(&self) -> f64;
    fn total_flux(&self) -> f64;                          // lumens
    fn light_output_ratio(&self) -> f64;                  // 0..1
    fn downward_fraction(&self) -> f64;                   // 0..1
    fn dimensions(&self) -> (f32, f32, f32);              // meters
    fn color_temperature(&self) -> Option<f32>;           // Kelvin
    fn cri(&self) -> Option<f32>;                         // 0..100
    fn beam_angle(&self) -> f64;                          // radians
    fn is_cylindrical(&self) -> bool { ... }
    fn upward_fraction(&self) -> f64 { ... }
}
```

We provide one impl out of the box: `impl PhotometricData for Eulumdat`
in `src/eulumdat_impl.rs`. Adding IES support is just another impl. The
trait is intentionally narrow — easy to satisfy from any of the
half-dozen photometric file formats.

**Why this matters for talks:** the plugin doesn't depend on the
`eulumdat` crate — it depends on the trait. The crate name has eulumdat
in it for historical reasons; the architecture is format-agnostic. We
could extract it as `bevy-photometry` tomorrow.

---

## 3. The two rendering paths (the central technical story)

This is the heart of the upstream proposal and the most important thing
to be precise about in technical Q&A.

### Path A: standard Bevy lights driven by photometric metadata (stable)

`src/photometric/systems.rs` reads the `PhotometricLight<T>` component
and spawns plain Bevy `PointLight` + `SpotLight` entities, configured
from the data:

- **Color** = `kelvin_to_color(data.color_temperature())` then
  `apply_cri_adjustment(color, data.cri())` — Tanner Helland's
  Kelvin→RGB approximation, desaturated for low CRI.
- **Intensity** = `data.total_flux() × data.light_output_ratio() ×
  intensity_scale` — derived from real LM-63/LDT flux numbers, not
  arbitrary.
- **Beam angle** = `data.beam_angle()` (the IES 50%-intensity half-angle)
  → maps to `SpotLight::inner_angle` / `outer_angle`.
- **Direction** = `rotation × Vec3::Y` (or `-Y` for downward spots)
  — respects the luminaire's `Transform`.
- **Up-vs-down split**: for asymmetric road luminaires, we spawn one
  main downward spot at 30% of the downward flux plus two side spots
  at ~35% each for wider coverage. For symmetric downlights, one spot
  at 100% of downward flux. Upward fraction becomes either an ambient
  bounce (low) or an upward spot (uplighters).

This path works on **stock Bevy 0.18+** unmodified. It's what gets
shipped in the WebGL2 fallback. Quality: passable — the luminaire's
*overall* flux + color + rough beam angle are correct. What's NOT
captured: the actual angular distribution. Two fixtures with the same
flux and CCT but very different intensity tables (e.g. a road luminaire
vs. a high-bay) render the same.

### Path B: `pbr_photometric_lights` — the proposal

The `holg/bevy.git#photometric_proposal` branch adds a new Bevy shader
path that samples the **full intensity table as a storage texture**
per-pixel in the fragment shader. Each photometric light gets a 2D
texture (C-angle × γ-angle) of intensity values; the shader looks up
the intensity in the direction from surface to light source, scales by
flux, and shades.

What this changes vs path A:

- A road luminaire's distinctive "bat-wing" cross-section is visible
  on the actual ground around it — narrow under, wide across.
- An asymmetric optic that throws 80% of light forward but only 5%
  back actually shades the scene asymmetrically.
- Glare cones for high-mast lights show the real fall-off from
  candelas-per-klm tables, not a generic spotlight cone.
- IBL is unchanged — this is direct lighting only. Indirect light from
  the scene still works through Bevy's normal PBR pipeline.

This is the part being upstreamed. Until it merges, `eulumdat-bevy`
pins to the fork via a git dep in `Cargo.toml` and can't publish to
crates.io (crates.io rejects git deps).

### How the two paths coexist in our code

- Stable plugin code in `src/photometric/systems.rs` always runs —
  spawns Path A lights.
- On the fork branch, additional `PhotometricLight` component data
  (the intensity table) is propagated to Bevy's storage textures by
  Bevy's own infrastructure — no extra crate code needed. Bevy
  handles the per-light texture and the shader sampling.
- In our WebGL2 fallback bundle (`cfg(feature = "webgl2")` in
  `examples/skyglow_demo.rs`), we explicitly skip features that
  require storage textures (Bloom, EnvironmentMapLight via KTX2 zstd,
  the dense cluster config). Standard Bevy lights still work; the
  per-pixel intensity-table sampling doesn't kick in. We make this
  obvious to the user with a "reduced fidelity fallback" banner.

---

## 4. Why this matters (the "so what" for the audience)

If asked **"why bother — Bevy already has lights?"**:

- Architects, lighting designers, dark-sky researchers, road engineers
  all need to **preview real-world luminaire performance** in 3D.
  Standard game engine lights (omnidirectional point lights, generic
  spot cones) are not photometrically meaningful — they don't shade
  the way an Erco downlight or a Bega bollard actually does.
- Existing tools (Dialux, Relux, AGi32) are CAD-shaped: precise but
  slow, paywalled, not embeddable. Bevy is a fast, web-deployable,
  open engine. Bringing real photometric data into Bevy lets the
  same `.ldt`/`.ies` files architects already have drive a
  walkthrough.
- **MLO / IDA dark-sky compliance**, **EN 13201 road lighting**, and
  **WELL Building** all reference photometric properties (uplight %,
  glare angle, U₀ uniformity). A photometric Bevy lets you author
  *and* validate against those standards in one tool. The street
  designer (`crates/eulumdat-wasm-street`) does this against EN 13201,
  RP-8, CJJ 45, and MLO.
- **Light pollution outreach**. The Skyglow demo is meant for IDA
  chapters, observatory partnerships (L'Observatoire de la Nuit),
  and amateur astronomy clubs — show *visually* what a shielded vs
  unshielded fixture does to the night sky. Standard game engine
  spots can't show this; photometric ones can.

---

## 5. The upstream proposal (`pbr_photometric_lights`)

**Branch**: `holg/bevy.git`, `photometric_proposal` branch, currently
based on Bevy `0.19.0-dev`.

### What it adds to Bevy

- New `Photometric` component carrying an intensity-table image
  handle + flux/LOR/CCT metadata.
- Storage-texture upload of the intensity table — one 2D image per
  light, C × γ packed.
- A new fragment-shader code path that samples that table when
  shading a surface lit by the light. Falls through to standard
  Bevy lighting when the feature is disabled or the texture is
  absent.
- A `pbr_photometric_lights` cargo feature flag that turns the path
  on at build time. WebGL2 builds keep it off (no storage textures).
- Companion API: `Bloom`, `EnvironmentMapLight`, `ClusterConfig`
  unchanged — they compose with photometric lights naturally because
  the photometric path only changes the *direct* lighting term.

### Why we forked

Three changes that don't fit a third-party crate:

1. **Fragment shader injection.** Bevy's PBR shaders are inside `bevy_pbr`;
   adding a new lighting term means editing those shaders. Can't do
   that from a plugin.
2. **Storage texture infrastructure.** Bevy's cluster light data is
   already a storage buffer; we extend the layout to carry an
   intensity-texture array reference per cluster light.
3. **Light component fields.** `PointLight` / `SpotLight` gain an
   optional `photometric: Option<Photometric>` field. Adding fields
   to a `Component` requires owning its definition.

### Other small fork divergences (worth knowing if Q&A drills in)

- `PointLight.shadow_maps_enabled` (our fork) vs `PointLight.shadows_enabled`
  (stock 0.18). Stock 0.19-dev is being moved toward the per-shadow-map
  control too, so this should align.
- `bevy/bevy_ui_widgets` (our fork) vs `bevy/experimental_bevy_ui_widgets`
  (stock 0.18). Stock 0.19 is dropping the `experimental_` prefix as
  part of the UI widgets stabilization — also auto-aligns.
- The KTX2 zstd path: stock Bevy 0.18 has `zstd_rust`, our fork has it
  too. No divergence here, just configurability.

### Upstream status (May 2026)

- The proposal is broken into smaller PRs against Bevy main, each
  reviewable in isolation: light-component fields → cluster storage
  → shader path → feature flag.
- Bevy 0.19's release window is mid-2026.
- Once 0.19 ships with the proposal merged, `eulumdat-bevy` switches
  to `bevy = "0.19"` from crates.io (one-line `Cargo.toml` change) and
  becomes publishable. **Until then, `eulumdat-bevy` stays at 0.6.0
  on crates.io (pre-fork), and the current photometric work lives at
  `0.7.0` only in this repo.**

---

## 6. WebGPU + WebGL2 dual-bundle (the deployment story)

Skyglow ships **two Bevy WASM bundles** built from the same source:

| Bundle | Cargo feature | Photometric path | Audience |
|---|---|---|---|
| `dist/skyglow/` | `webgpu` (default) | `pbr_photometric_lights` storage textures | Chrome/Edge/Brave/Safari Tahoe |
| `dist/skyglow-webgl2/` | `webgl2` | Path A only (CCT-driven generic lights) | iPad iOS, Linux Firefox stable, blocklisted Linux Chrome |

A runtime `navigator.gpu.requestAdapter()` probe in
`dist/skyglow-loader-<hash>.js` picks one. The Leptos shell reads
`window.skyglowBackend` to know which loaded and shows a "reduced
fidelity" banner on the WebGL2 path.

URL param `?wasm=skyglow_demo&force=webgl2` (or `force=webgpu`) bypasses
the probe — useful for previewing the fallback or for HN reply links.

**Talking-point version**: "We probe WebGPU at runtime, load the full
fidelity Bevy bundle when it's available, fall back to a reduced-quality
WebGL2 bundle otherwise. Banner tells the user which they got."

---

## 7. The Skyglow demo specifically

If asked about the methodology of the light-pollution piece itself, see
the dedicated note: methodology is **comparative rendering, not a
quantitative atmospheric simulation**.

Three components combine:

1. **Photometric data drives the lights** — two LDT files swap
   (`road_luminaire.ldt` for "pollution" mode, `projector.ldt` for
   "preserved" mode). On WebGPU, the angular distribution drives the
   actual shading. On WebGL2, only the CCT and flux are honored;
   distribution is generic.
2. **Skyglow is rendered, not computed**. The LDT's
   `downward_flux_fraction` (read via `data.downward_fraction()`) is
   plumbed into:
   - `update_fog` (`skyglow_demo.rs:1267`) — fog color shifts toward
     warm-orange as uplight rises.
   - `update_ambient` (`skyglow_demo.rs:1279`) — sky clear color lifts.
   - Star brightness — `bright_stars.json` stars dim as the sky
     brightens (HDR scaling, not real extinction).
3. **A "Sky Glow Score"** (A–F) is computed directly from the LDT's
   uplight fraction. It's a readout, not a model.

What we **don't** simulate (be honest about this):

- No Mie/Rayleigh scattering. Real skyglow has wavelength dependence
  (blue scatters more than red); we render it as a homogeneous color shift.
- No SPD. CCT → RGB only.
- No Bortle scale calibration.
- No atmospheric extinction.

The honest framing: *"This is what the same street looks like with two
different luminaire choices. Real LDT files, comparative render.
Skyglow is rendered visually, not predicted quantitatively."*

If pressed for what would make it quantitative: a follow-up project
implementing the IDA's atmospheric scattering model + SPD propagation
+ Bortle calibration. Probably its own crate (`skyglow-rs` or
`bortle-rs`). Doable, but a real research project.

---

## 8. Anticipated Q&A

### "Why not just use IES/LDT in Three.js / Babylon / Unreal?"

- **Unreal**: has IES profile support, but only as a *texture mask* on
  spotlights — not a full angular sampling. You can fake a road
  luminaire shape but not the asymmetric flux distribution. Also not
  open source, not web-deployable.
- **Three.js**: has community IES loaders (`ies-loader`) that produce
  a spotlight cone with an IES-derived attenuation. Same limitation.
- **Babylon.js**: similar — IES textures as cone modifiers.

What we do that's different: the intensity *table itself* drives the
shader's lighting term. Asymmetric and bat-wing distributions render
correctly, not just intensity drop-off along a cone axis.

### "Why Bevy specifically?"

- **Open source, MIT/Apache** — no licensing friction for partners.
- **WebGPU-first** — modern shaders, compute pipelines, can do storage
  textures (the precondition for the per-light intensity table).
- **WASM-friendly** — the same Bevy app runs natively and in browsers
  via wasm-bindgen. We ship one codebase.
- **ECS** — clean integration: a `PhotometricLight` component slots
  next to standard `PointLight`/`SpotLight` without changing the
  rest of the scene.
- **Active fork-able community** — the photometric proposal is a
  PR-able series, not a vendor-locked rewrite.

### "What's the performance cost of the photometric path?"

- Per-light: one storage-texture binding (intensity table, typically
  72 × 36 floats = ~10 KB). Per-pixel: one texture sample + bilinear.
  Roughly comparable to sampling a normal map for a textured light
  cookie. Adds linearly with light count.
- The Bistro scene in Skyglow runs at 60 FPS on a 2020 M1 MacBook
  with 12 photometric lights + Bloom + IBL on Chrome WebGPU.
- WebGL2 fallback runs at higher FPS because it skips Bloom + IBL,
  but renders fewer features.

### "Does this work with IES files too?"

Yes — IES is just another `PhotometricData` impl. The `eulumdat` crate
already parses IES (`crates/eulumdat/src/ies.rs`); we'd add a
`PhotometricData` impl wrapping IES data. Same trait, same plugin, same
shader. The trait is intentionally format-agnostic for this reason.

### "What about colorimetric accuracy / spectral data?"

Not in this release. The `PhotometricData` trait returns a CCT, not an
SPD, and the renderer uses `kelvin_to_color` (Tanner Helland fit) for
RGB. The infrastructure for spectra exists in `eulumdat::atla::spectral`
(parses IES TM-30 / ANSI/IES TM-30-15 reports + spectral SPDs from
ATLA-S001 files), but it isn't wired into the Bevy renderer.

That's a future feature: SPD → CIE 1931 XYZ → sRGB transformation,
plus optional spectral rendering at higher quality. Probably tied to
the IDA outreach work — astronomers care more about SPD-correct
skyglow than architects do.

### "What's the licensing situation?"

- `eulumdat-bevy` itself: AGPL-3.0-or-later (workspace default).
- Our Bevy fork (`holg/bevy.git#photometric_proposal`): MIT / Apache-2.0
  (same as upstream Bevy — no relicensing).
- Star data (`bright_stars.json`): HYG v3 by David Nash, CC-BY-SA 2.5.
  Attribution in `crates/eulumdat-bevy/data/STARS_README.md` and in
  the file itself.
- Bistro / Sponza scenes: their own licenses (Khronos sample assets,
  Amazon Lumberyard) — referenced from `assets/`, not redistributed.

If asked about a more permissive license: AGPL is for the workspace as
a whole. Individual crates that ship to crates.io as libraries
(`eulumdat`, `eulumdat-ui`, etc.) carry the same workspace license. The
question "can I commercially use this?" answer: yes under AGPL terms;
we can discuss alternative licensing.

### "How big is the WASM bundle?"

- **Skyglow WebGPU bundle**: ~38 MB raw, **6.8 MB Brotli-compressed** on
  the wire.
- **Skyglow WebGL2 fallback**: ~39 MB raw, similar compressed.
- **Initial Leptos load** (editor + viewer UI shell): 1.6 MB compressed.
- The Bevy bundle is lazy-loaded — only fetched when the user opens
  Skyglow.
- One bundle per user — the loader picks WebGPU or WebGL2, not both.

### "Why is the WebGPU bundle so big?"

Bevy's render graph + asset pipelines + glTF parser + KTX2 transcoder
+ shaders + photometric path. A from-scratch shader-only WebGPU app
would be much smaller, but you'd be rewriting half of Bevy. The Bevy
WASM size has been an ongoing community optimization target — current
state is "shippable, not lean."

### "What about mobile / iPad performance?"

- **iPad (WebGL2 fallback)**: Bistro renders at ~30 FPS, lower than
  desktop but interactive. Bloom and IBL are off by default for this
  reason — they're the most expensive features.
- **Android Chrome**: has WebGPU enabled by default on recent versions;
  performance varies wildly with the GPU/Mali driver. We don't have
  comprehensive numbers.
- **Native binary**: `cargo run --example skyglow_demo --release`
  on M1 = solid 60 FPS at 1080p with everything on.

---

## 9. Where the code lives (cheat sheet)

```
crates/eulumdat-bevy/
├── Cargo.toml                          # Bevy fork dep, feature flags
├── src/
│   ├── lib.rs                          # PhotometricPlugin, viewer plugin
│   ├── main.rs                         # eulumdat-3d bin entry point
│   ├── eulumdat_impl.rs                # impl PhotometricData for Eulumdat
│   ├── photometric/
│   │   ├── data.rs                     # The PhotometricData trait
│   │   ├── light.rs                    # PhotometricLight<T> component
│   │   ├── plugin.rs                   # PhotometricPlugin<T>
│   │   ├── systems.rs                  # Spawn-system: PhotometricLight → Bevy lights
│   │   ├── mesh.rs                     # Photometric-solid mesh generation
│   │   └── color.rs                    # CCT/CRI/Helland color helpers
│   └── viewer/                         # Full eulumdat-3d viewer app
└── examples/
    ├── skyglow_demo.rs                 # Light-pollution demo
    └── light_stress.rs                 # Performance benchmark
```

For Bevy fork divergences:

```
holg/bevy.git, branch photometric_proposal
├── crates/bevy_pbr/src/render/photometric.rs    # PhotometricRenderPlugin
├── crates/bevy_pbr/src/light/...                # Extended PointLight/SpotLight
└── (shader edits in bevy_pbr's WGSL files)
```

---

## 10. What to say if something goes wrong on stage

- **Demo flickers in Safari**: "Known issue, Bloom + HDR + UI on
  Safari's WebGPU compositor. The 3D scene is fine; the dashboard
  overlay flickers. Upstream Bevy / wgpu, not our code. iPad and
  pre-Tahoe Safari hit the WebGL2 bundle and don't see it."
- **Linux Firefox blank page**: "Firefox stable on Linux doesn't ship
  the WebGPU runtime. Try Chrome or Firefox Nightly. We have a
  WebGL2 fallback bundle but it requires the page to load past the
  initial probe — works for me on this machine."
- **Crash mid-demo**: "Live web demo, 38 MB WASM, lazy-loaded — refresh
  usually works. The native binary at `cargo run -p eulumdat-bevy
  --bin skyglow-demo --release` is more stable for live presentation."
- **Bistro scene loading slowly**: "Bistro is 181 MB; we serve it
  uncached from the demo server. Native preloads it from disk."
- **Audience asks why eulumdat-bevy isn't on crates.io**: "Because the
  Bevy photometric proposal isn't in stock Bevy yet — we're pinned to
  a git fork until the PR series lands in 0.19. The rest of the
  workspace (eulumdat, eulumdat-cli, eulumdat-i18n, etc., 19 crates
  total) ships to crates.io at 0.7.0."

---

## 11. Slide-friendly summary

> **eulumdat-bevy** is a Bevy plugin that turns real photometric files
> — the same `.ldt` and `.ies` that lighting designers already use —
> into properly-shaded 3D scenes. It works in two modes: a stable path
> that uses Bevy's existing lights driven by photometric metadata, and
> a proposed Bevy core extension (`pbr_photometric_lights`, currently
> a fork branch, being upstreamed into Bevy 0.19) that samples the
> full intensity table per pixel. Demo at iesna.eu — runs in any
> browser via a WebGPU + WebGL2 dual-bundle. Targets dark-sky outreach,
> road-lighting compliance, and architectural lighting preview.

---

*Last updated 2026-05. Refresh when the upstream proposal merges into
Bevy 0.19 stable — at that point this doc's "fork" sections become
historical, and `eulumdat-bevy` becomes a normal crates.io crate.*
