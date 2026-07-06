# Native App Modernization Plan

**Status:** Proposed — awaiting implementation
**Date:** 2026-06-23
**Scope:** Bring the native apps (Android, HarmonyOS) up to the Swift app's
`AtlaDocument` feature level, and replace **all** native 3D renderers with the
existing Bevy WASM viewer hosted in a platform WebView.

---

## Background

The core (`eulumdat`) and FFI (`eulumdat-ffi`) crates were refactored to an
**ATLA-unified data model**. `eulumdat-ffi` already exports the full modern
surface via UniFFI:

- `AtlaDocument` (primary type) — `parseXml/parseJson/fromLdt/fromIes`,
  `toXml/toJson/toLdt/toIes`, `manufacturer/description/catalogNumber`,
  `emitters/primaryEmitter`, `totalLuminousFlux/totalInputWatts/efficacy`,
  `cct/cri`, `hasSpectralData`.
- Supporting types: `Emitter`, `ColorRendering`, `SpectralDistribution`.
- Diagram generators (`generateAtla*Svg`): polar, butterfly, cartesian,
  heatmap, cone, beam-angle, bug, lcs — plus `generateSpectralSvg` and
  `generateGreenhouseSvg` (both ATLA-only), each with `_localized` variants.
- `comparePhotometric` (+ localized), BIM (`getBimParameters`, `hasBimData`),
  validation (`validateLdt`, `getValidationErrors`), `batchConvertContents`.

### Current native app status

| App | Framework | 3D today | Data model | Status |
|-----|-----------|----------|-----------|--------|
| **Swift** (`EulumdatApp/`) | SwiftUI + UniFFI | SceneKit/Metal | `AtlaDocument` (hybrid w/ legacy `Eulumdat`) | **Current** — reference impl |
| **Android** (`EulumdatAndroid/`) | Compose + UniFFI | Compose `Canvas` (painter's algo) | legacy `Eulumdat` only | **Stale** — no ATLA, no spectral/greenhouse |
| **HarmonyOS** (`EulumdatHarmonyOS/`) | Cangjie + **C FFI** | SVG butterfly | legacy `Eulumdat` via `eulumdat-harmonyos-ffi` (C, not UniFFI) | **Very stale** — pre-ATLA C surface, dual source tree |

**Parity target = the Swift app's 13 views:** ContentView (editor), Compare,
Validation, SchemaValidation, BimPanel, BatchConvert, plus all diagram types
including Spectral + Greenhouse.

---

## Key architectural decisions

1. **3D = Bevy WASM in a WebView, on every platform.** Do not attempt to
   embed native Bevy. `eulumdat-bevy` depends on a Bevy *fork*
   (`holg/bevy.git#photometric_proposal`, `publish = false`) and only targets
   desktop + WASM via `bevy_winit`. Cross-compiling the fork (incl. the
   `basis-universal` C++ dep and storage-texture shader path) to
   `aarch64-linux-android` / `aarch64-apple-ios` is high-risk and out of scope.
   Instead, reuse the **already-built Bevy WASM bundle** in:
   - Android: `android.webkit.WebView`
   - iOS/macOS: `WKWebView`
   - HarmonyOS: ArkUI/native `Web` component
   - **Requires on-device WebGPU** (the project is WebGPU-only — no WebGL2
     fallback for the main viewer). This means newer OS versions; document the
     minimum and show a graceful "3D needs WebGPU" fallback to the SVG butterfly.

2. **Data bridge to the WebView = `localStorage` injection.** The Bevy viewer
   reads the current luminaire from `localStorage["eulumdat_current_ldt"]` and
   polls `eulumdat_ldt_timestamp` for changes (`viewer/wasm_sync.rs`). There is
   **no `?ldt=` query-param loader on the native path**, so the host app must
   inject the LDT string into `localStorage` via JS evaluation:
   - Android: `webView.evaluateJavascript("localStorage.setItem(...)", null)`
   - iOS: `wkWebView.evaluateJavaScript("localStorage.setItem(...)")`
   - Inject **after** page load (or via a `WKUserScript` at
     `.atDocumentStart`), then bump the timestamp key so the poller reloads.
   - *Optional core enhancement:* add a `?ldt=<base64>` startup loader to the
     Bevy viewer so the WebView can pass data via URL instead of a JS bridge.
     Cleaner, but a `eulumdat-bevy` change — treat as a stretch goal.

3. **Android & HarmonyOS keep the SVG butterfly as the WebGPU-unavailable
   fallback.** The existing SVG butterfly (from `generateAtlaButterflySvg`)
   stays as the degraded-mode 3D-ish view. The hand-drawn Compose `Canvas`
   renderer (`Butterfly3DView.kt`) and Swift SceneKit renderer are **removed**.

4. **Bevy WASM bundle hosting.** Each app bundles the Bevy WASM build as a
   local asset (Android `assets/`, iOS app bundle, HarmonyOS `resources/`) and
   serves it to the WebView from a local origin (WebGPU + WASM need a secure /
   `file://`-with-flags or local-HTTP context). Determine per-platform whether
   a local HTTP server or asset loader is required (WebGPU may refuse
   `file://`). Reuse `./scripts/build-wasm-split.sh` output (the Bevy
   `eulumdat-3d-{hash}.js` + `_bg.wasm`).

---

## Phase 1 — Android → AtlaDocument parity + Bevy WebView 3D

Files: `EulumdatAndroid/app/src/main/java/eu/trahe/eulumdat/`

**1a. Data layer (`data/LdtData.kt`, `data/LdtRepository.kt`)**
- Make `AtlaDocument` the primary parsed type: `AtlaDocument.fromLdt()/fromIes()/parseXml()/parseJson()`.
- Keep `Eulumdat` only for raw LDT-field editing (mirror Swift's round-trip).
- Expose `emitters()`, `cct()`, `cri()`, `colorRendering`, `efficacy()`,
  `totalLuminousFlux()`, `hasSpectralData()`. (Note: `cri` is already
  referenced 26× in the data layer — the UI expects it; wire it to the ATLA
  source.)

**1b. Diagrams (`ui/EulumdatApp.kt`)**
- Swap basic `generate*Svg` → `generateAtla*Svg`.
- Add **Spectral** (`generateSpectralSvg`) and **Greenhouse**
  (`generateGreenhouseSvg`) tabs, gated on `hasSpectralData()`.

**1c. New screens for Swift parity (new Compose files under `ui/`)**
- `CompareScreen` → `comparePhotometric` / `comparePhotometricLocalized`
- `BimScreen` → `getBimParameters` / `hasBimData`
- `ValidationScreen` → `validateLdt` + `getValidationErrors`
- `BatchConvertScreen` → `batchConvertContents`
- Add nav (drawer or tab row) — current app is a single `EulumdatApp` composable.

**1d. Replace 3D renderer**
- **Delete** `ui/Butterfly3DView.kt` (Compose `Canvas` painter's-algo renderer).
- Add `ui/BevyWebView.kt`: an `AndroidView` wrapping `WebView` with
  `WebView.settings.javaScriptEnabled = true`, WebGPU-capable WebView (Chrome
  WebView ≥ the version that ships WebGPU), loading the bundled Bevy WASM.
- Inject the current LDT into `localStorage` via `evaluateJavascript`; bump
  timestamp on change so the viewer hot-reloads.
- Fallback: if WebGPU unavailable, render `generateAtlaButterflySvg` in an
  SVG/WebView image instead.

**1e. Bindings & build**
- `scripts/build-android.sh` already targets `-p eulumdat-ffi` across 4 ABIs —
  regenerate Kotlin UniFFI bindings to expose `AtlaDocument`.
- Add a build step (or reuse `build-wasm-split.sh`) to copy the Bevy WASM
  bundle into `app/src/main/assets/bevy/`.
- Verify compile on all ABIs.

## Phase 2 — HarmonyOS → AtlaDocument parity + Bevy WebView 3D

Files: `EulumdatHarmonyOS/`, `crates/eulumdat-harmonyos-ffi/`

**2a. Extend the C FFI (`eulumdat-harmonyos-ffi/src/lib.rs`)**
Cangjie cannot consume UniFFI (UniFFI emits Kotlin/Swift/Python only), so the
existing hand-rolled C surface is extended rather than replaced:
- Add an opaque `AtlaHandle` + `atla_parse_xml/json/from_ldt/from_ies`.
- Add `atla_cct/cri/efficacy/total_flux`, `atla_emitters`, `atla_has_spectral`.
- Add `atla_spectral_svg`, `atla_greenhouse_svg`, `atla_beam_angle_svg`,
  `atla_compare`, plus matching `*_free`. (`eulumdat-photweb` is already a dep,
  so spectral/greenhouse logic is reachable.)
- Update `eulumdat_ffi.h`.

**2b. Cangjie bindings + UI (`ffi.cj`, `engine.cj`, `types.cj`, `main.cj`)**
- Mirror the new C functions in `ffi.cj`; typed wrappers in `engine.cj`.
- Add Spectral / Greenhouse / Compare views in `main.cj`.

**2c. 3D via Web component**
- Replace the SVG-only butterfly with an ArkUI `Web` component loading the
  bundled Bevy WASM (same localStorage-injection bridge). Keep SVG butterfly
  as the no-WebGPU fallback (HarmonyOS WebGPU support is the gating risk —
  verify on target device/emulator; if absent, HarmonyOS stays SVG-only and
  this sub-step is deferred).

**2d. De-duplicate the Cangjie source tree**
- Two copies exist: `EulumdatHarmonyOS/src/eulumdat/*.cj` and
  `EulumdatHarmonyOS/Eulumdat/entry/src/main/cangjie/eulumdat/*.cj`.
- Make the DevEco `entry/` tree the source of truth; delete or symlink the
  other so edits don't drift.

## Phase 3 — Swift: swap SceneKit → Bevy WebView (optional, consistency)

Swift is already on `AtlaDocument`, so only the 3D renderer changes:
- **Delete** `Butterfly3DView.swift` (SceneKit/Metal) and `Room3DView.swift`.
- Add `BevyWebView.swift`: a `UIViewRepresentable`/`NSViewRepresentable`
  wrapping `WKWebView` loading the bundled Bevy WASM, with the same
  localStorage bridge.
- Keep SVG butterfly fallback for OS versions without WebGPU in WKWebView.
- *Trade-off:* SceneKit is native, smooth, and already works. Only do this if
  one-renderer-everywhere consistency is worth losing native 3D. **Recommend
  deferring** until Android/HarmonyOS WebView 3D is proven.

## Phase 4 — Docs & consistency
- Rewrite `.claude/CLAUDE.md` (stale: still lists Leptos/Bevy/Swift/Android as
  "✅ Complete", no TUI/ATLA/daylight/street/spectrography). Reflect:
  ATLA-unified core, WASM + TUI as primary showcases, native apps as
  `AtlaDocument` consumers with Bevy-WASM-in-WebView for 3D.
- Reconcile README "What's New in 0.5.0" header vs the 0.7.0 workspace version.

---

## Sequencing & risk

1. **Android first** — no core/FFI changes needed for the data side; fastest
   visible win; establishes the `AtlaDocument` + Bevy-WebView pattern the other
   platforms copy.
2. **HarmonyOS second** — higher risk (hand-written C surface extension +
   Cangjie + dual tree + uncertain WebGPU support).
3. **Swift 3D swap last / optional** — it already works on SceneKit; only for
   consistency.

### Top risks
- **On-device WebGPU availability** gates the Bevy-WebView 3D on every
  platform. Mitigation: SVG butterfly fallback + documented min OS.
- **Serving WASM to a WebView** may require a local HTTP origin (WebGPU/WASM
  often refuse `file://`). Resolve per platform early.
- **HarmonyOS C-FFI** is a hand-maintained parallel surface — every new ATLA
  capability must be re-exported by hand.
- **`eulumdat-bevy` stays fork-pinned & `publish = false`** until the
  `photometric_proposal` branch lands in upstream Bevy; the WebView approach
  sidesteps this for native but the WASM bundle still builds from the fork.

### Out of scope
- Native (on-device) Bevy via NDK/UIKit.
- Bevy on HarmonyOS as a native engine (no `ohos` winit backend exists).
- Any change to the WASM/TUI showcase apps beyond reusing the Bevy bundle.
