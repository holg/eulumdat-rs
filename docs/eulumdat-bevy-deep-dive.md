# eulumdat-bevy deep dive — the after-talk-beer edition

Companion to `docs/eulumdat-bevy-knowledge-base.md`. The polished doc is
for the talk; this is for the conversations afterwards. Code details,
design decisions, the things that went wrong, the stuff we'd do
differently. Written assuming the other person knows Rust + has used
Bevy + can read shaders.

Topics:

1. Architectural decisions and the alternatives we didn't pick
2. The `PhotometricData` trait — what's in it, what isn't, what we'd
   change
3. The path-A spawn system, line by line — including the parts that
   are physically nonsense
4. The photometric_proposal shader path — what actually changes in Bevy
5. The fork-vs-stock divergences in painful detail
6. WASM build pipeline — why it looks the way it does
7. Skyglow specifically — the embarrassing parts
8. What we'd do differently from scratch
9. Open problems / scratchpad
10. Stories worth telling

---

## 1. Architectural decisions

### Plugin-vs-application split

`PhotometricPlugin<T>` (`src/photometric/plugin.rs`) is intentionally
the smallest possible thing — three systems, one component, one
resource:

```rust
app.init_resource::<PhotometricPluginState<T>>()
    .add_systems(Update, (
        spawn_photometric_lights::<T>,
        update_photometric_lights::<T>,
        cleanup_photometric_lights::<T>,
    ));
```

That's it. No scene, no camera, no controls. The viewer code
(`src/viewer/`, `src/main.rs`) is a separate layer that adds all of
that on top. We deliberately wanted "drop into existing Bevy app" to
be a small ask.

The alternative we rejected: one fat `PhotometricViewerPlugin` that
brings everything. Looked tempting in early prototypes but coupled
every user to our opinionated camera + UI + control scheme. The split
fell out naturally once we tried to embed it in `eulumdat-wasm` (the
Leptos editor) — the editor wants the plugin without our camera.

### Trait-based data, not concrete LDT

`PhotometricData` is generic over `T`. Each plugin instance is
`PhotometricPlugin::<Eulumdat>`. We could have hard-coded `Eulumdat`
and saved a lot of `<T: PhotometricData>` bounds.

We didn't because:
- IES support is coming. Same trait, second impl. The generic-over-T
  cost is paid once, then it's free forever.
- GLDF when somebody wants it.
- The trait doc-comment says "When extracting to a standalone
  `bevy_photometry` crate, this file can be copied as-is — it has no
  dependencies on `eulumdat`." That's the long game.

The cost: every system signature has `T: PhotometricData` plus you
write `PhotometricPlugin::<Eulumdat>::default()` every time. Worth it.

### Why store the data in the component, not behind a handle

`PhotometricLight<T>` holds `data: T` directly, not
`data: Handle<PhotometricAsset>`. We discussed this for a while.

Reasons we put it in the component:

- LDT files are tiny (typically 5–30 KB). Storing them in components
  doesn't bloat anything.
- Bevy's `Changed<PhotometricLight<T>>` change detection works directly
  — flip a field, the spawn system re-fires.
- No asset-loading dance. Adding a luminaire is `commands.spawn((
  PhotometricLight::new(ldt_data), Transform::...))`. No `.load()`,
  no waiting for `AssetEvent::LoadedWithDependencies`, no fragile
  scene-vs-asset timing.

Trade-off: if you have 10,000 instances of the same fixture, you pay
the storage 10,000 times. We have ~12 luminaires in the heaviest
scene, so it doesn't matter. If somebody adds a stadium scene with
500 identical floodlights, we'd switch to `Handle<Photometric>`.

### Why we built `eulumdat-bevy` at all (and not "just contribute to Bevy")

Honest answer: we needed the demo running for a partner conversation,
and Bevy's release cadence is months. Building outside Bevy gave us
the freedom to iterate fast on the shader-side proposal *and* ship
the visible thing (Skyglow, the viewer, the integration with the
editor) on our own schedule.

The upstream PR series came after we knew what worked. That's much
easier to land than "here's a 40-file design doc, please review."

---

## 2. The `PhotometricData` trait — what's actually in it

### What's there (`src/photometric/data.rs`)

```rust
trait PhotometricData: Send + Sync + Clone + Debug + 'static {
    fn sample(&self, c_angle: f64, g_angle: f64) -> f64;    // cd/klm
    fn max_intensity(&self) -> f64;
    fn total_flux(&self) -> f64;                            // lumens
    fn light_output_ratio(&self) -> f64;                    // 0..1
    fn downward_fraction(&self) -> f64;                     // 0..1
    fn dimensions(&self) -> (f32, f32, f32);                // m
    fn color_temperature(&self) -> Option<f32>;             // Kelvin
    fn cri(&self) -> Option<f32>;                           // 0..100
    fn beam_angle(&self) -> f64;                            // radians
    fn is_cylindrical(&self) -> bool { … }
    fn upward_fraction(&self) -> f64 { … }
}
```

### What's not there (and probably should be)

- **No SPD.** Just CCT. The whole spectral story (TM-30, CIE 1931 XYZ
  transform, blue-rich-LED skyglow argument) bypasses this trait. The
  `eulumdat::atla::spectral` infrastructure exists but isn't plumbed
  to the renderer.
- **No symmetry information.** LDT files declare symmetry (none /
  one-plane / two-plane / rotational) — `sample(c, g)` has to do the
  right thing internally. We currently rely on the `Eulumdat::sample`
  impl in the core crate to honor symmetry. An IES implementation
  would need to do the equivalent. The trait should probably surface
  symmetry explicitly so the renderer can avoid sampling redundantly.
- **No `glare_class()`** or other compliance-derived metadata. The
  street designer reads those from the underlying `Eulumdat` directly.
  Should probably move into the trait if we ever want street designer
  to work with non-LDT data.
- **No `is_uplight()`** convenience. We compute it from
  `downward_fraction() < 0.5` in the dashboard. Fine for now.

### What's slightly wrong

- `beam_angle()` returns "the IES 50%-intensity half-angle." That's the
  classical definition, but the LDT file format doesn't have a beam
  angle field — we compute it from the intensity table by finding the
  angle at half max. That works for nice symmetric distributions and
  is wrong for asymmetric ones (a road luminaire doesn't have *a*
  beam angle).
- `dimensions()` returns `(width, length, height)` in meters. The LDT
  has separate fields for the luminous emitting surface and the
  physical housing; we collapse those into one. For the viewer's
  luminaire mesh that's fine. For collision / physics it'd be wrong.

### The trait we'd build with hindsight

```rust
trait PhotometricData {
    // What it already has.
    fn sample(&self, c: f64, g: f64) -> f64;
    fn total_flux(&self) -> f64;
    fn light_output_ratio(&self) -> f64;
    fn dimensions(&self) -> (f32, f32, f32);

    // What's missing.
    fn symmetry(&self) -> Symmetry;          // explicit, not implicit
    fn intensity_table(&self) -> &IntensityTable;  // raw access for renderer
    fn color(&self) -> ColorSpec;            // CCT or SPD or RGB
    fn compliance_metadata(&self) -> Option<&ComplianceData>;
}
```

The current trait is good enough for shipping; the better one is a 0.8
or 0.9 thing.

---

## 3. The path-A spawn system, line by line

`src/photometric/systems.rs::spawn_lights_for_entity` is the function
that turns one `PhotometricLight<T>` into N Bevy lights. It's where
all the heuristics live, and it's the place that's most "this is what
shipped on Friday afternoon" code in the crate.

Walkthrough of what it does for a typical road luminaire:

```rust
let total_flux = data.total_flux() as f32;           // e.g. 33,200 lm
let lor = data.light_output_ratio() as f32;          // e.g. 0.85
let luminaire_flux = total_flux * lor;                // 28,220 lm
let color_temp = data.color_temperature().unwrap_or(4000.0);
let cri = data.cri().unwrap_or(80.0);
let light_color = apply_cri_adjustment(kelvin_to_color(color_temp), cri);

let downward_fraction = data.downward_fraction() as f32;  // 0.95
let beam_angle = data.beam_angle() as f32;                // ~1.2 rad

let intensity_scale = 50.0 * light.intensity_scale;       // ★ the magic number
```

The `* 50.0` is the first warning sign. Bevy's `PointLight.intensity`
is roughly luminous power but the engine's lighting curve doesn't
quite match real-world photometry. We tuned `* 50.0` empirically until
the Bistro scene looked "right" against reference photos. There is no
physical derivation for it. **If somebody asks "but is the luminance
in cd/m² calibrated?" — no.** This is one of the things that goes
away when path B (`pbr_photometric_lights`) takes over: that path
samples real cd/klm values and Bevy's PBR consumes them properly.

Then the spawn dance for a road luminaire:

```rust
// One ambient PointLight at 30% intensity
commands.spawn((PointLight { color, intensity: F*0.3*S, ... }));

// Main downward SpotLight at 30% × downward_fraction
commands.spawn((SpotLight { intensity: F*0.3*D*S,
    inner_angle: beam*0.2, outer_angle: beam*0.6, ... }));

// Two side spots at 35% each
commands.spawn((SpotLight { intensity: F*0.35*D*S,
    inner_angle: 0.3, outer_angle: 0.8, ... }));
commands.spawn((SpotLight { intensity: F*0.35*D*S, ... }));
```

That's **four Bevy lights per photometric luminaire** for an asymmetric
fixture. The percentages (30/35/35) come from "what makes the Bistro
look right." Total isn't even 100% — we lose 5% in the ambient
double-counting, gain it back via the LOR factor. It's wrong by an
amount that would horrify a lighting designer.

The reason this is acceptable in path A: it's a stand-in. Path B,
when active, ignores all this and uses the real intensity table. Path
A only ships in the WebGL2 fallback where the user already knows
they're getting reduced fidelity.

If somebody asks "why don't you just spawn ONE spot per luminaire" —
because asymmetric road lights produce a *bat-wing* distribution that
a single spot cone can't approximate. Three spots at different angles
+ one ambient is the cheapest way to fake the shape. Five would be
slightly better. Twelve would be nearly right. We picked four.

### `BevyLightMarker<T>`

Each spawned Bevy light gets a `BevyLightMarker<T>::new(parent_entity)`
component pointing at the `PhotometricLight` entity. The `cleanup`
and `update` systems use this to find "all Bevy lights belonging to
this photometric luminaire" when the photometric component changes
or is removed.

Without the marker, a `Changed<PhotometricLight<T>>` event would have
no way to find which Bevy lights to update. Bevy's relationship API
(`Parent`/`Children`) doesn't quite fit because the lights aren't
children of the photometric entity — they're at the world transform.

---

## 4. The photometric_proposal shader path

What actually changes in Bevy when you check out the
`photometric_proposal` branch:

### `bevy_pbr/src/light/`

`PointLight` and `SpotLight` gain an optional field:

```rust
pub struct PointLight {
    // existing fields…
    pub photometric: Option<Photometric>,
}

pub struct Photometric {
    pub intensity_table: Handle<Image>,  // 2D image, C × γ
    pub max_intensity: f32,              // cd, peak for normalization
    pub flux: f32,                       // total flux, lumens
    pub lor: f32,                        // light output ratio
}
```

When `photometric.is_some()`, the PBR shader path samples the
intensity table instead of using the standard
`distance × cos_theta` falloff for the directional term.

### `bevy_pbr/src/render/photometric.rs`

A new `PhotometricRenderPlugin` that:
- Holds a GPU storage texture array (one slice per active photometric
  light) sized for the worst case (we cap at 256 lights with
  intensity tables to bound GPU memory).
- Uploads the LDT-derived intensity table to its assigned slice when
  a light's `Photometric.intensity_table` handle resolves.
- Extends the cluster light data so each cluster entry carries its
  slice index alongside its position / range / etc.

### Shader edits (`crates/bevy_pbr/src/render/pbr_lighting.wgsl`)

The `point_light` and `spot_light` shader functions get a new branch:

```wgsl
fn photometric_intensity(
    light_to_surface: vec3<f32>,
    light_axis: vec3<f32>,
    light_basis_x: vec3<f32>,
    light_basis_y: vec3<f32>,
    intensity_table_slice: u32,
) -> f32 {
    // Convert light-to-surface direction into the luminaire's local
    // C-plane / γ-angle space.
    let local_dir = normalize(vec3<f32>(
        dot(light_to_surface, light_basis_x),
        dot(light_to_surface, light_basis_y),
        dot(light_to_surface, light_axis),
    ));

    let gamma = acos(-local_dir.z);            // 0 = nadir, π = zenith
    let c = atan2(local_dir.y, local_dir.x);   // 0..2π around vertical

    let uv = vec2<f32>(c / TAU, gamma / PI);
    let cd_per_klm = textureSampleLevel(
        photometric_intensity_tables,
        photometric_sampler,
        uv,
        intensity_table_slice,
        0.0,
    ).r;

    // Scale by total flux × LOR / max_intensity to get true cd, then
    // apply standard photometric falloff (1/r²).
    return cd_per_klm * lights.data[i].flux * lights.data[i].lor
         / (1000.0 * length_squared(light_to_surface_world));
}
```

If `intensity_table_slice == NO_PHOTOMETRIC`, the existing
non-photometric path runs.

### Why per-pixel sampling and not pre-computed cookie textures

Two options for "drive Bevy lighting from real intensity data":

- **Light cookies** (the Unreal / Three.js approach). Project the
  intensity table onto a spotlight cone as a 2D texture mask. Fast,
  works in any renderer, fits the `Spotlight` model.
- **Per-pixel intensity sampling** (what we did). The fragment shader
  looks up the intensity in the direction from surface → light for
  *every* surface pixel.

We picked per-pixel because:

- Cookies work for symmetric "spotlight-shaped" distributions. They
  fail for *omnidirectional* sources with asymmetric distributions —
  e.g. a streetlight that throws 70% of its light forward, 20%
  sideways, 10% back. There's no single cone the cookie projects
  through.
- Cookies require a fixed cone direction and angle. Real luminaires
  have data in 360° × 180° of solid angle.
- The storage-texture cost is comparable: a cookie is a 2D texture,
  a per-light intensity table is also a 2D texture. Same memory.

Cookies are easier to retrofit into existing renderers — that's why
Unreal does them. We didn't have that constraint; building from
scratch on Bevy let us pick the better approach.

---

## 5. The fork-vs-stock divergences in painful detail

If you ever need to migrate `eulumdat-bevy` to a different Bevy
version (or someone asks "why a fork?"), these are the actual
differences:

### Field renames

- `PointLight::shadows_enabled` (stock 0.18) →
  `PointLight::shadow_maps_enabled` (our fork). Our reasoning: per-
  light-source shadow control is more granular than "global shadows
  on this light"; the rename signals the per-shadow-map nature. Stock
  Bevy 0.19-dev is moving toward similar naming, so this re-aligns.
- `SpotLight` — same change.

### Feature renames

- `experimental_bevy_ui_widgets` (stock 0.18) → `bevy_ui_widgets`
  (our fork). The `experimental_` prefix is being dropped in stock
  0.19 as the widgets stabilize. Auto-aligns.

### New components / fields

- `Photometric` component (our fork only). Carries the intensity
  table handle + flux + LOR + max intensity. Optional field on
  `PointLight` and `SpotLight`.
- `PhotometricRenderPlugin` (our fork only). Wires up the GPU storage
  texture array, the cluster-data extension, the shader injection.

### Cargo features

- `pbr_photometric_lights` — feature in our fork's `bevy_pbr`.
  Enables the new shader path. When off, the new component fields
  exist but are ignored. When on, photometric lights use the new
  sampling path; non-photometric lights still use the standard path.

### Shader edits

- `crates/bevy_pbr/src/render/pbr_lighting.wgsl` — branch added in
  `apply_point_light` and `apply_spot_light` to choose photometric
  vs standard path.
- `crates/bevy_pbr/src/render/clustered_forward.wgsl` — cluster
  light data layout extended.

### Why not feature-gate everything

Bevy's WGSL doesn't have meaningful `#ifdef`-equivalent for cargo
features at the shader level. We could pre-process at the Rust side
to swap in different shader bodies, but that means maintaining two
shader versions. The fork approach lets us edit the actual shaders
in place.

### Migration cost when upstream merges

Best case: 1-line `Cargo.toml` change. Stock Bevy 0.19 ships with the
proposal merged, `eulumdat-bevy` just changes `git = "…"` to
`version = "0.19"`.

Worst case: the upstream review changes the API (renames `Photometric`
to `PhotometricLight`, moves fields around, splits the shader into a
separate plugin). Then we update `src/photometric/light.rs` and
`systems.rs` to match. Maybe 50 lines of changes. Real cost is mostly
in re-testing the demo.

---

## 6. WASM build pipeline

`scripts/build-wasm-split.sh` is the source of truth. Why it looks
the way it does:

### Why we don't use `trunk` for the Bevy bundle

`trunk` is great for the Leptos editor — it handles Sass, asset
copying, WASM optimization. But:

- Bevy needs `wasm-bindgen` invoked with specific flags. Trunk's
  wasm-opt invocations broke our Bevy build at one point (stripped
  reference-types).
- Bevy's render pipeline needs the WASM exported with `--target web`,
  not `--target bundler` (trunk's default).
- We need fine-grained control over which features build (WebGPU vs
  WebGL2). Trunk's feature handling is per-crate, not per-bin.

So the Bevy bundles use raw `cargo build` + raw `wasm-bindgen`,
copy into `dist/`, content-hash the filenames manually. Ugly but works.

### Why two skyglow bundles, separate target-dirs

The dual-bundle (WebGPU + WebGL2) is built with `--target-dir`
swapping:

- WebGPU: `target/wasm32-unknown-unknown/release/`
- WebGL2: `target/skyglow-webgl2/wasm32-unknown-unknown/release/`

Without separate target dirs, the two builds clobber each other's
`release/skyglow-demo.wasm`. With separate dirs, each cargo build
is fully cached. Cost: ~2× disk usage. Worth it.

### Why content hashes in filenames

Cache busting. We don't want users hitting stale Bevy bundles after
a deploy. Each `.wasm` and its `.js` glue get an MD5-derived hash
in the filename; the loader has the hash baked in. Stale files in
the user's cache are simply not requested again.

### Why `--no-verify` for WASM publishes

Crates.io's verify step runs `cargo build` on the published source.
For `eulumdat-wasm`, that means building Leptos+Bevy on Linux x86_64
— which fails because `getrandom = { features = ["wasm_js"] }` is
WASM-only. `--no-verify` skips the build check; the crate publishes
as source, and downstream users pull it for their WASM project where
it actually builds.

---

## 7. Skyglow specifically — the embarrassing parts

The Skyglow demo (`examples/skyglow_demo.rs`) is the most-visible
thing in the repo and also has the most "Friday afternoon" code.

### The star catalog is a one-off snapshot, not real positions

`bright_stars.json` is precomputed alt/az from HYG v3 for one
location at one moment (Lüdinghausen, 51.77°N 7.44°E, 2025-12-29
19:25 UTC). Anyone using the demo at any other time/place is seeing
"a sky that looked like this once in Westphalia."

We documented this honestly in `crates/eulumdat-bevy/data/STARS_README.md`
and there's a `regenerate_stars.py` script that uses `skyfield` to
rebuild for any location/time. Nothing ever calls it at runtime
because it'd add a Python dependency to a Rust + WASM repo.

The right fix: embed raw HYG (mag ≤ 5, ~9k stars, ~500 KB JSON) and
compute alt/az client-side at load time from geolocation + UTC. Real
work, doable, not done. **If an astronomer asks**, lead with this:
"the demo uses a fixed snapshot, real-time positioning is a follow-up
feature, here's the regen script as a hint of where it's going."

### The "skyglow score" is just `uplight_pct`

The A–F grade in the dashboard is computed in `update_ui_from_state`:

```rust
let score = state.uplight_pct / 100.0;
let (grade, _) = if score < 0.05 { ("A", green) }
              else if score < 0.15 { ("B", green) }
              else if score < 0.30 { ("C", amber) }
              else if score < 0.50 { ("D", orange) }
              else { ("F", red) };
```

That's it. Not Bortle. Not IDA's actual scale. Not calibrated
against anything. We chose thresholds that looked reasonable and
gave a wide spread between "road luminaire" (D/F) and "projector"
(A). **If somebody asks "but how do you derive an A grade?"** —
honest answer: it's the IDA's uplight target (< 0% U-component for
LZ0 / dark-sky areas). A and B map roughly to LZ0/LZ1; C to LZ2;
D to LZ3; F to LZ4. We could cite IES TM-15 BUG ratings if we
wanted to be more rigorous — but BUG is BUG, not "skyglow score."

### Star dimming is hand-tuned

`setup_stars`:

```rust
let brightness = 50.0 + (4.5 - mag).max(0.0) / 6.0 * 450.0;
```

Where `mag` is apparent magnitude. So magnitude 0 (Vega) gets
brightness 50 + 4.5/6 × 450 ≈ 387. Magnitude 4.5 gets brightness
50. Linearly between. Real atmospheric extinction is exponential
(`exp(-τ × airmass(zenith_angle))`), wavelength-dependent, and
varies with site altitude. We faked it as a linear ramp because
it looked right.

For the IDA audience this is the place we'd most want to fix —
proper extinction + wavelength-dependent skyglow would land. Add
it to the "follow-up SPD project" list.

### The "preserved darkness" mode looks fake

When you switch to PreservedDarkness mode, the scene gets *very*
dark — too dark. The reason: real preserved-darkness streets aren't
black, they have moonlight, distant skyglow from neighbors, etc.
Our scene has the LDT lights + ambient at 0.

A more honest preserved mode would have:
- A lower ambient lift representing moonlight
- A subtle warm glow representing the *neighbor's* skyglow you can't
  fix
- Stars at full magnitude visibility

We didn't do that because the side-by-side comparison reads stronger
with the contrast cranked. It's a demo, not a measurement.

### The Bistro asset is 181 MB

`BistroExterior_web.glb`, 181 MB. Loaded on every Skyglow demo open
when the user switches to Bistro scene. Compressed it'd be ~50 MB
but we ship uncompressed because the WebGPU bundle's wgpu can read
KTX2 textures directly from disk faster than from a decompressed
glb-in-zip.

This will bite us if traffic ever scales. The first 1000 HN visitors
caused a noticeable bandwidth blip. Real fix: switch Bistro to KTX2
+ Meshopt geometry compression, get to ~20 MB. Not done.

---

## 8. What we'd do differently from scratch

If we restarted on a clean repo today:

### The `PhotometricData` trait

Add explicit `symmetry()` and `intensity_table()` accessors so the
renderer (path B) doesn't have to call `sample()` once per pixel —
it can grab the table directly and upload to GPU. The current
`sample()`-only approach forced us to compute the table *outside*
the trait (in `eulumdat::Eulumdat`), then convert to a Bevy `Image`,
then upload. Bypassing one of those steps would clean up
`src/photometric/light.rs` significantly.

### Path A vs Path B split

We currently switch on `cfg(feature = "webgl2")` in the demo code
itself. Cleaner would be: detect WebGPU support at runtime in the
plugin, fall back to path A internally. Bevy's render graph already
has feature-based branching; we should use it.

This requires path A to live in the plugin code rather than just in
the cfg-gated demo. Mostly already true — `spawn_lights_for_entity`
runs unconditionally in `update_photometric_lights`. We'd just need
to short-circuit path B (skip the storage texture upload) when WebGPU
isn't there.

### Crate name

`eulumdat-bevy` is wrong. The crate has nothing to do with the LDT
file format — it's a Bevy plugin for photometric lighting. Should
be `bevy_photometry` (matching Bevy's naming convention for plugins)
or `bevy-photometric` (matching ours). We can't rename now without
breaking the existing 0.6.0 on crates.io.

When the upstream proposal merges and we restart on `bevy = "0.19"`:
- Publish `bevy_photometry` as the new name
- Make `eulumdat-bevy` a thin alias crate that re-exports from
  `bevy_photometry` and adds the `eulumdat`-specific impl

### The viewer code

`src/viewer/` is a separate concern from the plugin. We discussed
splitting it out into `eulumdat-viewer` or `bevy_photometry_viewer`
many times. Didn't, because the editor (`eulumdat-wasm`) imports
plugin + viewer together. Should still do it.

### Don't store data in the component

We mentioned this earlier as a deliberate choice. With 1+ years of
hindsight, the asset-handle approach would be cleaner. Bevy's
`Assets<T>` + `Handle<T>` infrastructure is solid; we just bypassed
it.

### The Bevy fork

In retrospect we should have published the proposal as a series of
small PRs against upstream main from day one, then *built against
upstream Bevy nightly* with the unmerged PRs cherry-picked. Less
divergence, faster upstream pickup. We started with a fork because
prototyping was faster; the cost is the publishability problem.

---

## 9. Open problems / scratchpad

Things we'd want to dig into but haven't:

### Photometric IES textures vs ID textures (cookies) — perf comparison

We claimed cookies are roughly comparable in cost. We haven't actually
benchmarked path B vs a cookie-only implementation. Per-light
storage texture might be more expensive than we think if the GPU's
texture cache thrashes. A `light_stress.rs`-style benchmark with
100+ photometric lights would settle it.

### SPD rendering

The whole spectral story sits in `eulumdat::atla::spectral`. Bevy
renders in sRGB. To go spectral we'd need:
- An SPD → XYZ → sRGB transform at material level
- A spectral lighting term in the shader (4 or 9 wavelength bins,
  Hero-wavelength tracing, or full SPD)
- Reference data — at minimum SPDs for the LDT files we ship

The IDA outreach use case really wants this for "blue-rich LED
makes skyglow worse" — currently we can't show that. Real project.

### Why does Safari WebGPU flicker on Bloom

`docs/safari_bloom_flicker.md` documents the symptom. The root cause
is still unclear. Things we haven't tried:
- Capture the WebGPU command stream from Safari (it'd require WebKit
  developer tools we don't have)
- Reduce the Bloom mip count to 1 — see if the flicker correlates
  with downsample passes
- Run the Bevy `bloom_3d` example in Safari Tahoe — is it just
  *our* combination of Bloom + UI + HDR that flickers, or is it
  Bloom in any Safari WebGPU app?

Suspect upstream wgpu storage-texture barrier issue on Safari's
WebKit backend. Filing an upstream wgpu issue with a minimal
reproducer is the right next step.

### Cluster light limit

Bevy's clustered forward+ has a hard limit (default 256 lights / cluster
or so). We've never hit it with the Bistro scene's 12 luminaires. A
stadium scene with 200 floodlights would. Path B's photometric
texture array also has a 256 limit. Both are bumpable but increase
GPU memory linearly.

### IES file support

`eulumdat::IesParser` exists. We've never wired up `impl
PhotometricData for IesData`. Half a day's work, then we can demo
"any IES file from this manufacturer's website drops in." Should
ship before the next talk.

---

## 10. Stories worth telling

The bits that don't fit a polished doc but are good after-beer
material:

### How we got the photometric proposal to compile against Bevy main

The first attempt was a fork with massive divergence — we'd added the
photometric path against a Bevy snapshot from six months ago and
ignored Bevy main's evolution. Three weeks of rebasing later, the
fork compiled but the shader path was subtly broken because Bevy had
refactored its cluster light data layout in the meantime. We
rewrote the cluster extension to match.

Lesson: track main, rebase weekly, file PRs early. The current
proposal branch has been rebasing onto Bevy main every couple of
weeks since.

### The CIE flux code bug

Unrelated to Bevy, related to the broader eulumdat work. The CIE
52-1982 flux-code computation was wrong in our `eulumdat` core for
**years** — cones at 40°/60°/90° instead of 41.4°/60°/75°, plus N1/N2/N3
on total flux instead of downward, plus N4 = upward fraction instead
of DLOR. Found it by comparing against a published BIOLUX HCL DL DN150
reference (`95 100 100 100 100`). We were off by enough that the wrong
output would have flagged compliant fixtures as non-compliant.

Lesson: regression-test against published reference data, even
boring metadata fields nobody looks at.

### The CJJ 45 misclassification

We had `MajorArterial` → Class II in our Chinese road class mapping
when it should be Class I (Major Arterial shares Class I with
Expressway per CJJ 45-2015 Table 3.3.2). Caught by Richard during a
review. Three other Chinese road grades were also one class off.

Lesson: when adapting a standard from another language/jurisdiction,
have a native speaker review the class mappings. Don't trust
auto-translation of "次干路" → "minor arterial" → Class III.

### The atla → eulumdat collapse

Originally `atla` was a separate workspace crate (the ATLA-S001
unified photometric data model). We had `eulumdat`, `atla`, and the
two crates calling each other in a circular-ish way. After publishing
once we realized: atla isn't useful without eulumdat (and vice versa
for our use case). Spent a day folding atla back into
`crates/eulumdat/src/atla/` as a module. Cleaner architecture,
fewer published crates, one fewer thing to release.

Lesson: don't pre-split crates. Wait until you have an actual second
user before extracting.

### The Safari Bloom diagnosis

Took an hour of diagnosis to figure out that the Safari overlay
flicker was specifically the Bloom + HDR + UI interaction. The
original report was "the dashboard panel flickers" — we initially
thought it was a Leptos/HTML issue, then a Bevy UI alpha-blend
issue, then finally narrowed it to the Bloom shader. Documented
the journey in `docs/safari_bloom_flicker.md`.

Lesson: when a bug only happens on one specific browser, add a
WebGL2 fallback build and the bug is mitigated for 80% of the
affected audience. Don't fix what you can route around.

### The obscura → skyglow rename

Originally called "Obscura: Darkness Preservation Simulator." Late
in the project we realized: (a) "obscura" was tied to a specific
partner conversation (L'Observatoire de la Nuit), (b) we wanted
multiple dark-sky partners, (c) the name had branding overlap with
existing photography terms. Renamed everything to "Skyglow" (the
IES/IDA technical term for upward-scattered pollution). Kept the
legacy `?wasm=obscura_demo` URL working so old links don't break.

Lesson: vendor-neutral technical terms beat clever names when
you're doing outreach across multiple partner audiences.

---

## 11. Things to NOT say at the bar

- **"It just works."** No, the demo has caveats. Be honest about
  them; the technical audience will respect you more for it.
- **"WebGPU is ready everywhere."** It's not. Linux Firefox stable
  literally doesn't ship the runtime. We have a fallback for a
  reason.
- **"Our skyglow simulation is accurate."** It's a comparative render,
  not a quantitative simulation. Misrepresenting this to an
  astronomer audience would be embarrassing.
- **"Why didn't you contribute upstream from day one?"** You can say
  "we wanted to ship something visible to a partner conversation
  fast" without sounding defensive. That's the truth.

---

*Refresh this doc when stories accumulate or when the upstream PR
series lands. The polished knowledge base (`eulumdat-bevy-knowledge-base.md`)
is for the talk; this one is for the conversations afterwards.*
