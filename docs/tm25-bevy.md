# Prompt: TM-25 ray file 3D viewer in Bevy (for iesna.eu)

> Status (2026-09-18): phase 1 implemented — `crates/eulumdat-bevy/src/tm25/`
> (`Tm25ViewerPlugin`, feature `tm25`), binary `tm25-viewer`
> (`examples/tm25_viewer.rs`), lazy bundle `bundles.tm25` in
> `scripts/build-config.toml` + steps 1.9 / 3.9 / 6.88 in
> `scripts/build-wasm-split.sh`, `src/static/tm25-loader.js` (IndexedDB file
> hand-off), Leptos host `components/tm25_viewer.rs` at `?wasm=tm25`, editor
> drop-zone hook for `.tm25ray`. All six views on the CPU path: rays, point
> crosses, far-field solid (smoothed, alpha-to-coverage), camera-view filter
> over the 200k analysis set, target-plane irradiance (jet texture, radial
> profile, datasheet-verified at z = 0.1 mm), UV exposure with the ICNIRP
> actinic S(λ) from `eulumdat_spectrum::uv` (a germicidal table can be
> dropped in through `ActionSpectrum::from_table`). Orbit camera, colour
> modes, DOM control/info panels incl. datasheet-style x/y cross sections.
> Open: GPU compute paths (phase 2), screenshot export, README screenshots.

## Role and goal

You are building an interactive 3D viewer for IES TM-25 ray files (`.TM25RAY`) using Bevy.
It ships on iesna.eu as a separate, lazily loaded WASM bundle (`?wasm=tm25`), exactly like the greenhouse demo, and nothing TM-25 related may be compiled into the always-loaded Leptos editor bundle.
iesna.eu is Bevy 0.19 (`crates/eulumdat-bevy`, WebGPU only); match its version, plugins, UI approach and WASM build setup.
Do not upgrade Bevy or introduce a second UI framework without asking.
The viewer must run natively and in the browser (WASM).
See "Integration into eulumdat-rs / iesna.eu" below for the concrete files to touch.

## Rendering backend: WebGPU

The browser build targets WebGPU, not WebGL2.
`eulumdat-bevy` selects the backend through its own `webgpu` / `webgl2` features (default `webgpu`); the greenhouse bundle is built with `--features bevy-ui,post-process,wasm-bindgen,js-sys` and its loader probes `navigator.gpu.requestAdapter()` before importing the module. Reuse that loader and its missing-WebGPU screen.
WebGPU gives us compute shaders and storage buffers, which this viewer relies on for live filtering, binning and splatting.
If the browser has no WebGPU support, show a clear message (browser and version requirements) instead of a blank canvas.
A WebGL2 fallback is out of scope; do not design features around WebGL2 limits.

WebGPU limits to respect:

- Points and lines are always 1 px wide in WebGPU, so thick lines and sized points must be built from instanced quads.
- The default `maxStorageBufferBindingSize` is 128 MiB, which holds about 4.7M rays at 28 bytes each.
  Query the adapter limits at startup and size the GPU ray budget from them.
- Storage buffer atomics are integer only (`atomic<u32>` / `atomic<i32>`), so flux accumulation in compute shaders uses fixed-point integers with a documented scale factor.
- Raising `maxStorageBufferBindingSize` above the default requires requesting it in Bevy's `RenderPlugin` / `WgpuSettings` limits at startup; the adapter probe in the loader only checks that WebGPU exists.
- Bevy 0.19 has no ready-made instancing helper. Stage the GPU work: phase 1 does binning, camera-view filtering and irradiance on the CPU over the analysis set (1M rays is tens of milliseconds single-threaded in WASM, recompute on slider release, `rayon` natively), phase 2 moves them to compute passes via `AsBindGroup` storage buffers and a render-graph node. The performance targets below are phase 2 targets.

The purpose is to make ray files understandable at a glance.
An LDT/IES file only shows the far field.
A ray file also shows where light leaves the source, and the viewer should make that visible.

## Dependencies and data source

Parsing is done by the `tm25ray` crate (github.com/holg/tm25ray, crates.io `tm25ray`); `crates/eulumdat-tm25` is the bridge that re-exports it and adds the EULUMDAT/ATLA conversions.
The viewer must not contain its own binary parser.
It uses the crate for: header, text fields, spectrum, ray iteration, and flux-preserving subsampling.
The crate must stay WASM-clean: `memmap2` and `rayon` behind cargo features that the WASM build does not enable, no `std::time::Instant` or `SystemTime` (they panic on wasm32).
Only the Bevy viewer bundle depends on it. The Leptos editor never does; it only recognises the `.tm25ray` extension and hands the browser `File` object over.
If `tm25-rs` is not ready yet, stub it behind a trait `RaySource` so the viewer can be developed in parallel.

## Integration into eulumdat-rs / iesna.eu

Copy the greenhouse pattern, not skyglow (the skyglow example no longer compiles on Bevy 0.19 and its bundle is disabled in `build-config.toml`).

| Piece | Greenhouse today | TM-25 viewer |
|---|---|---|
| Bevy binary | `[[bin]] greenhouse-demo` → `examples/greenhouse_demo.rs` | `[[bin]] tm25-viewer` → `examples/tm25_viewer.rs`, plugin code in `src/tm25/` behind a `tm25` feature so the default `eulumdat-3d` bundle does not grow |
| Build | `bundles.greenhouse = true` in `scripts/build-config.toml`; step 1.x in `scripts/build-wasm-split.sh`: `bevy build`/`wasm-bindgen --target web`, `wasm-opt -Oz`, content hash, copy to `dist/greenhouse/` | `bundles.tm25 = true`; clone the step to `dist/tm25/`; add `tm25` to `FORCE_MODULE_NAMES` |
| Loader | `src/static/greenhouse-loader.js` (dev copy) and a hashed generated copy: WebGPU probe, `window.loadGreenhouseDemo`, `isGreenhouseLoaded` | `tm25-loader.js` with `window.loadTm25Viewer`, `isTm25Loaded`, `isTm25Loading`; same "exceptions for control flow" guard |
| Leptos host | `components/greenhouse_demo.rs`: full-screen `<canvas id="greenhouse-canvas">`, click-to-load overlay, Unsupported/Error states, `?wasm=greenhouse_demo` branch in `app.rs` | `components/tm25_viewer.rs`, `<canvas id="tm25-canvas">`, `?wasm=tm25` branch; the Bevy `WindowPlugin` uses `canvas: Some("#tm25-canvas")` |
| Data handoff | LDT text via `localStorage` (`eulumdat_current_ldt` + timestamp), polled each frame by `viewer/wasm_sync.rs` | not possible for ray files (localStorage is a few MB, files reach 2.8 GB). See "Loading and memory strategy" |
| Settings | `ViewerSettings` JSON in `localStorage`, Leptos writes, Bevy polls (`load_viewer_settings_from_local_storage`) | same mechanism for sliders and toggles on the web; keyboard shortcuts natively |

Bundle size will be similar to the greenhouse bundle (~25 MB compressed) because Bevy's renderer dominates; the parser adds almost nothing.

Reuse from `crates/eulumdat-bevy`:

- `photometric::mesh::photometric_solid_mesh` and `photometric_solid_material` take anything implementing `PhotometricData`; implement that trait for the C/γ histogram built from the rays and view 3 is the existing solid, so a ray-file solid and an LDT solid overlay in the same frame and the same coordinate convention. Adopt the TM-25 → Bevy axis mapping from that module's convention rather than inventing a new one, then keep the unit test.
- `photometric::color` for CCT colours; for spectral colour use `eulumdat_spectrum::colorimetry::analyze` on the header spectrum (SPD → xy → sRGB), which is more robust than a dominant-wavelength lookup. UV/IR sources get the labelled false colour.
- `eulumdat-spectrum` is the place for the germicidal action-spectrum table (as a data file next to the photopic/melanopic curves), so the UV-C dose overlay and the CLI share it.
- The existing `viewer::camera` is a first-person WASD controller; write a small orbit controller for millimetre scale instead of bending it.
- Screenshot: the greenhouse binary already has a native `--screenshot out.png` path; on the web use `canvas.toBlob` from the loader.

Entry points for users:

- Drop or open a `.tm25ray` in the editor: the drop handler in `app.rs` dispatches by extension today, so add the extension, keep the `File` in a JS global (`window.tm25PendingFile`) and switch the view to the viewer by signal, not by navigating to a new URL (a reload would drop the `File`). `?wasm=tm25` alone shows the viewer with its own file picker and drop target.
- `?wasm=tm25&url=...` fetches a hosted file with streaming `fetch` for demos. Do not host the ams OSRAM files; their disclaimer covers redistribution. Host a synthetic ray set generated by the writer milestone instead.

## Facts from the reference file (use for defaults and acceptance tests)

Fixture: `rayfile_SU_CULCP1_VC_100k_20260512_IES_TM25.TM25RAY`, ams OSRAM SU CULCP1.VC, UV-C LED, 150 mA.

- 100,000 rays, each 5.1e-7 W, total 51 mW radiant flux (radiometric, no lumens).
- Spectrum 220 to 320 nm, peak 266 nm, FWHM about 262 to 272 nm.
- Start points: x in [-1.75, 1.61] mm, y in [-1.61, 1.64] mm, z in [-0.815, 0.0] mm.
- Geometry visible in the point cloud: a flat chip plane at z = -0.755 mm, and angled reflector walls widening from about ±0.85 mm at the cavity floor to about ±1.35 mm at the top (z = 0).
- Directions: all in the upper hemisphere, kz in [0.16, 1.0], max polar angle about 81°.
- Far field: flat to about 25°, half intensity near 57°, peak about 19 mW/sr.
- Larger versions of the same source exist (more rays, same geometry), up to 100M rays or about 2.8 GB.

## Coordinate system

TM-25 positions are in millimetres with z as the optical axis (emission direction).
Bevy is y-up, and the existing photometric solid (`photometric::mesh`) puts γ = 0 (the emission axis, LDT nadir) on Bevy −Y, C = 0 on +X and C = 90 on +Z, i.e. luminaires point down.
To overlay a ray-derived solid with an LDT solid without any extra rotation, convert once at load time with the same convention: Bevy `(x, y, z) = (x_tm, −z_tm, y_tm)`, and the same for directions.
This is a proper rotation (−90° about X, determinant +1), so handedness is preserved; a ray with k = (0, 0, 1) points along Bevy −Y, and TM-25 +x is C = 0.
`tm25ray` exposes `direction_to_c_gamma` and `to_bevy` helpers plus the unit test (re-exported by `eulumdat-tm25`), so the viewer does not redo this.
Work in millimetres as world units; the source is only about 3.5 mm wide, so set camera near/far planes accordingly (for example near 0.01, far 1000).

## Loading and memory strategy

- Keep two budgets: an analysis budget of rays uploaded to a GPU storage buffer (default 1M, capped by adapter limits) and a display budget of rays actually drawn as geometry (default 50,000).
  Compute passes (far field, camera view, irradiance) run on the analysis set; drawing uses the display subset.
- For small files (up to about 1M rays), load fully in memory, then subsample.
- For large files, stream through the file once and use reservoir sampling to keep N rays.
  Rescale each kept ray's flux so the kept set sums to the header total.
- WASM: the Leptos shell (or the viewer's own drop target) stores the browser `File` in `window.tm25PendingFile`; the Bevy bundle picks it up through `web_sys`, reads it with `Blob.slice` + `arrayBuffer()` in chunks of a few MB inside a `wasm_bindgen_futures::spawn_local` task, and feeds the streaming reader. Results go to the Bevy world through a channel polled by a system (the async task pool is single-threaded on wasm32, so never block a system on it).
  A 2.8 GB file must never be materialised in WASM memory. The WASM link settings allow 4 GB max memory with 256 MB initial; stay far below that.
- Native: `FileDragAndDrop` window events, the command-line path, and `memmap2` behind the `mmap` feature.
- Show a progress indicator while streaming, and run parsing off the main frame loop (async task) so the UI stays responsive.
- Keep the full-resolution far-field histogram accumulated on the CPU during the streaming pass, so the far-field view uses all rays, not only the subsample.

## Views (each toggleable)

### 1. Ray view (main feature)

Draw each sampled ray as a line segment from its start point along its direction.
Use one static mesh with `LineList` topology, built once, not immediate-mode gizmos per frame.
Rebuild the mesh only when sample count, ray length or colour mode changes.

- Ray length slider, default 3 mm, range 0.5 to 50 mm.
- Vertex alpha fades from start to end so dense bundles stay readable.
- Additive blending, so dense regions glow.

### 2. Emission point cloud

Render only the start points.
This view reveals the package: chip plane, cavity walls, hot centre.
WebGPU only draws 1 px points, so render each point as a small camera-facing quad.
Phase 1: one static mesh with four vertices per point (50k points is 200k vertices, trivial) and a custom `Material` whose vertex shader billboards the quad toward the camera; no instancing machinery needed.
Phase 2: read positions straight from the ray storage buffer in the vertex shader so the point cloud covers the 1M analysis set.
Default quad size 0.01 mm, adjustable.

### 3. Far-field intensity solid

Bin all ray directions into a C/γ grid (for example 5° × 2.5°).
Intensity per bin = summed flux / solid angle, with `ΔΩ = Δφ · (cos γ1 − cos γ2)`.
Build a mesh whose radius in each direction is proportional to intensity, placed at the source origin and scaled to a user-controlled size.
Show units: W/sr for radiometric files, cd for photometric files.
Optionally also draw the classic C0/C90 polar curves as line meshes.

### 4. Camera view (near-field luminance image)

This is the most important unique feature.
Use the current orbit camera's view direction v.
Select rays whose direction satisfies `k · (-v_toward_camera) >= cos(cone)`, meaning rays that would reach the camera.
Highlight those rays' start points, dim everything else.
The result shows what a near-field goniophotometer camera sees from that angle, for example the chip plus bright reflector wall patches at 45°.
Cone half-angle slider, default 8°.
Update live while orbiting.
Do the filter on the GPU: a compute pass (or the vertex shader of the point-cloud pass) tests each ray against the camera direction every frame, over the full analysis set, so the image sharpens with 1M rays instead of 50k.

### 5. Target plane irradiance

A plane perpendicular to the optical axis at distance d above the source, slider from 1 mm to 500 mm, logarithmic.
Intersect all rays with the plane and splat their flux into a 2D grid (for example 256 × 256) in a compute shader using fixed-point `atomic<u32>` accumulation, then resolve into a texture.
Recompute on every slider change; it should feel instant.
Show it as a false-colour heatmap on the plane with a scale bar in mW/cm² (radiometric) or lx (photometric).
Show a small 2D radial profile next to it.
This view makes the near-field vs far-field transition obvious: at 2 mm the profile is wide and flat-topped, at 50 mm it matches the far-field shape.

### 6. UV-C dose overlay (radiometric UV files only)

Enabled when the spectrum lies mostly below 400 nm.
Weight the spectrum with a germicidal action spectrum (make the weighting table a replaceable data file, do not hardcode one source).
Show germicidal-weighted irradiance on the target plane and a dose for a user-set exposure time in mJ/cm².

## Colour modes for rays and points

- Polar angle γ (default): blue near the axis, red near 90°.
- Start height z: separates chip emission from wall emission clearly (chip at z = -0.755 mm in the fixture).
- Flux per ray: uniform in the fixture, but other files may vary.
- Spectral colour: for visible sources, map the header spectrum's dominant wavelength to RGB.
  UV and IR are invisible, so use a clearly labelled false colour (for example violet for UV-C) instead of black.

## UI

On the web the controls live in the Leptos shell (DOM, themed, translatable) and reach Bevy through the `ViewerSettings`-style localStorage sync already used by the 3D viewer; the Bevy side only renders. Natively use the `bevy-ui` feature (as the greenhouse demo does) or keyboard shortcuts; the `egui-ui` feature in `eulumdat-bevy` is an empty placeholder and must not be relied on.

- File open (native dialog, browser file picker), plus drag and drop on both.
- Info panel from the header: product name, manufacturer, operating condition, ray count, total flux with unit, date, spectrum mini-plot.
- Display sample count selector: 5k, 20k, 50k, 200k; analysis set size shown next to it.
- View toggles, colour mode, sliders listed above.
- Orbit camera with sensible framing: auto-fit to the start-point bounding box on load.
- Axis gizmo and a millimetre scale bar, because the source is tiny and users lose sense of scale.
- Screenshot export (PNG).

## Performance targets

- 50,000 drawn rays at 60 fps in a mid-range laptop browser with WebGPU.
- Camera-view filter and irradiance splat over 1M rays in under 5 ms per update on the same machine.
- Loading the 100k fixture and first frame in under 1 second natively.
- Streaming a 10M ray file (about 280 MB) in the browser without exceeding about 200 MB of WASM memory.

## Acceptance tests

Automated (no GPU needed):

- Coordinate conversion test (k = (0, 0, 1) maps to Bevy +Y).
- Subsampling keeps total flux at 0.051 W within f32 tolerance.
- Reservoir sampling over the fixture returns exactly N rays and is deterministic for a fixed seed.
- Far-field binning of the fixture gives half intensity between 52° and 62° and zero intensity above 82°.
- Camera-view filter at 45° with an 8° cone selects roughly 1.5% of rays (about 1,550 of 100,000; allow ±20%).
- Target plane intersection: every ray reaches a plane at d > 0 because all kz > 0.
- GPU and CPU reference implementations agree: irradiance grid and camera-view selection from the compute shaders match a CPU implementation on the fixture within fixed-point rounding.

Manual, with the fixture:

- Point cloud side view shows a flat line at z = -0.755 mm and two slanted walls.
- Point cloud top view shows a square cavity floor with a bright centre.
- Camera view at 45° shows the chip and separate bright patches from the reflector walls.
- Irradiance at 2 mm is visibly wider and flatter than at 50 mm.

## Out of scope for this iteration

- Writing TM-25 files.
- Full optical ray tracing through user-supplied lenses (possible later: rays already have start points and directions, so a single refracting surface would be a natural next step).
- Photometric conversion of UV sources (lumens are meaningless there).

## Deliverables

1. `tm25ray` from crates.io (reader, see `tm25-20.md`) via the `crates/eulumdat-tm25` bridge.
2. A Bevy plugin `Tm25ViewerPlugin` in `crates/eulumdat-bevy/src/tm25/` behind a `tm25` feature, plus the `tm25-viewer` binary (`examples/tm25_viewer.rs`) that opens a file passed on the command line.
3. The lazy WASM bundle: `bundles.tm25` in `scripts/build-config.toml`, the build step and hashed loader in `scripts/build-wasm-split.sh`, `src/static/tm25-loader.js`, the Leptos host component and the `?wasm=tm25` route, the `.tm25ray` drop-zone hook. Verified with `./scripts/build-wasm-split.sh servesimple` on port 8042 and the WebGPU-only policy (no WebGL2 fallback).
4. Short README with screenshots of the six views using the fixture.

## Exports and the virtual goniometer (2026-09-21)

- After a load the Bevy plugin publishes `localStorage["tm25_exports"]` with the
  far field (the same smoothed grid the solid shows) as EULUMDAT, IES and ATLA
  XML (spectrum attached) plus the header spectrum as `.spd`, built by
  `eulumdat_tm25::exports`. The host page offers them as downloads, "Open in
  editor" (one-shot `eulumdat_pending_file`) and "Virtual goniometer".
- The flux-rescaled ray subset (≤ 200k rays) is written as a small TM-25 file
  into the loader's IndexedDB stash (`tm25StashFile(bytes, "gonio")`). The
  goniometer page (`?wasm=goniosim&source=tm25`) takes it, builds
  `eulumdat_goniosim::Source::RaySet` (flux-weighted replay from the real start
  points, TM-25 `(x, y, z)` → goniometer `(x, y, −z)`, mm → m) and keeps the
  far-field LDT of the same rays as the reference distribution. Covers and
  reflectors therefore see the emitting area, which is what a ray file is for.
  The goniometer also accepts a `.tm25ray` file directly. CPU tracer only.
