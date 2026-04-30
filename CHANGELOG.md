# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-04-30

This release consolidates the work landed across 0.5.x / 0.6.x development
into a single coordinated cut. Detailed historical entries for those
intermediate versions are not reconstructed here; the highlights below
cover everything new since 0.4.0.

### Fixed

- **CIE flux code formula** in
  `PhotometricCalculations::cie_flux_codes` now follows CIE 52-1982:
  cone half-angles **41.4° / 60° / 75°** (was 40 / 60 / 90),
  N1 / N2 / N3 cumulative on **downward** flux (was on total flux),
  N4 = **DLOR** (was upward fraction), N5 = **LOR** (was 90–120° zone).
  Regression test (`crates/eulumdat/tests/cie_flux_code_regression.rs`)
  asserts `95 100 100 100 100` for the published BIOLUX HCL DL DN150
  reference. Display switched to `f64::round_ties_even` per spec
  banker's rounding. Full analysis: `docs/check_cie_flux_code.md`.
- **DLOR / ULOR derivation** in
  `GldfPhotometricData::from_eulumdat` was using the previous
  (now-renamed) `cie_codes.n1` / `n4` fields with their old meanings;
  fixed to compute DLOR = `n4 × LOR / 100` and ULOR = `LOR − DLOR`.
- **CJJ 45 class definitions corrected** per CJJ 45-2015 Table 3.3.2:
  - `主干路` (Major Arterial) → **Class I** (was incorrectly Class II)
  - `次干路` (Minor Arterial) → **Class II** (was Class III)
  - `支路` (Branch) → **Class III** (was IV)
  - `居住区道路` (Residential) → **Class IV** (was conflated with Branch)
  - Class III illuminance target corrected to 8 lux (was 10)
  - Class IV illuminance target corrected to 5 lux (was 8)

  The decision tree now offers five road types (was four), and locale
  labels in all 8 languages embed the Chinese characters inline so the
  meaning is unambiguous.
- **Greenhouse PPFD** previously reported 0 across the area for
  asymmetric distributions (e.g. the OxyTech Pinco 36 fixture with peak
  at γ=20°). The beam-angle estimator now anchors on the actual peak
  index and walks outward to the half-peak boundary, clamped to
  [5°, 170°]. Regression tests cover both narrow-beam and wide-beam
  distributions.
- **Trade-off chart Pareto frontier** previously drawn in green, which
  visually conflicted with the high-uniformity green markers on the
  same chart. Now dashed, neutral-color, with a legend entry.

### Added

- **OxyTech `.OXL` / `.OXC` read support** —
  new `eulumdat::atla::oxl` module with `OxlPackage`, multi-luminaire
  catalog support. Parses LITESTAR's `<LitePack>` XML schema (header
  metadata + per-luminaire photometric data + `<MeasurementMatrix>`)
  into the existing atla model. Spec: `docs/OXL-UK.pdf`. Regression
  test against the published Pinco 36 LED RA 4000K sample. WASM editor
  drag-drop, file picker, and CLI all route `.oxl` / `.oxc` through it.
- **Street designer** — full layout-design workflow for outdoor
  roadway lighting:
  - **Compliance standards**: ANSI/IES RP-8 (US), DIN EN 13201 (EU),
    CJJ 45 (CN), MLO dark-sky compliance.
  - **Per-standard decision trees** with the official selection
    inputs for each region.
  - **EN 13201-1 Annex A selector** — full-matrix `Vws` calculation
    for C-family and P-family lighting classes.
  - **Layout optimizer** — brute-force search across pole spacing ×
    mounting height × arrangement, optimizing pole count, total flux
    per km, or safety margin.
  - **Plan-view SVG renderer** with optional sidewalk band, isolux
    contours, threshold-tint overlay, and theme support.
  - **i18n** — decision-tree + optimizer strings translated across
    8 locales.
- **Companion WASM apps**: `eulumdat-wasm-street` (lazy-loaded street
  designer), `eulumdat-wasm-quiz` + `eulumdat-quiz-py` (photometric
  knowledge quiz with 8 locales).
- **Street designer exports** — every diagram is downloadable. Inline
  `⬇ SVG` buttons next to the plan view, the new PV/PC principal-planes
  diagram, and the layout trade-off chart, plus a master Exports panel
  that adds optimizer results as CSV / JSON and a full **PDF report**
  compiled in-browser via Typst. The PV/PC section in the PDF follows
  the on-screen visibility toggle.
- **Trade-off chart UX**: hover tooltips on every bubble (spacing ×
  height × arrangement, poles/km, U₀, lux, flux/km, "click to apply"),
  click-to-apply via event delegation, neutral-dashed Pareto frontier
  with a legend entry.
- **Advisor mode** (`eulumdat::street::advisor`) — one-line
  recommendations on each failed compliance criterion, keyed by
  parameter category (glare → reduce γ > 80°, uniformity → tighter
  spacing, BUG B/U → shielding, etc.). Surfaced below the compliance
  table in the WASM editor.
- **Plan-view calculation grid points** overlay — toggle in the plan
  toolbar to verify the EN 13201 / RP-8 prescribed grid is what's
  being measured.
- **Principal-planes (PV/PC) diagram** — `eulumdat::diagram::principal_planes`
  produces a side-by-side polar plot of the C0–C180 (along-road) and
  C90–C270 (across-road) intensity cuts at a shared scale.
- **Workflow round-trip integration test** —
  `crates/eulumdat/tests/street_workflow_roundtrip.rs` exercises the
  edit → recalculate → optimize → apply → re-edit → recalculate → advisor
  path against the road template fixture.
- **Skyglow demo star data documented** —
  `crates/eulumdat-bevy/data/STARS_README.md` with HYG Database v3
  attribution (CC-BY-SA 2.5), generation parameters, and
  `regenerate_stars.py` for regenerating the snapshot for arbitrary
  observer locations and times.
- **Build script** — `scripts/build-wasm-split.sh` gained selective
  `force <module>` mode and a `force -h` listing of available bundle
  names; workspace-wide source-hash check fixes prior false
  "unchanged" skips.

### Changed

- **Atla module integration** — the historical `crates/atla` workspace
  crate has been folded into `crates/eulumdat/src/atla/`. Public path
  changes from `atla::Foo` → `eulumdat::atla::Foo`. Old `atla` features
  (`xml`, `json`, `i18n`, `eulumdat`) collapse into the eulumdat crate's
  feature set: `xml` and `json` are eulumdat features (default-on),
  `i18n` was already an eulumdat feature, and the previous
  feature-gated `eulumdat` bridge is now unconditional core
  (`atla::convert` always compiles).
- **Application-layer rename**: misleading `ldt` / `Ldt` / `set_ldt`
  identifiers renamed to `ldc` / `Ldc` / `set_ldc` where the value is
  a generic light distribution rather than a literal `.ldt` file. The
  storage key `eulumdat_current_ldt` is retained for cross-tool
  compatibility (Bevy 3D viewer, Street designer all read it).
- **CLI `commands.rs`** routes `.oxl` / `.oxc` extensions through
  `eulumdat::atla::parse_file`, unblocking OXL imports from the
  command line.
- **Bevy `obscura-demo` renamed to `skyglow-demo`** across the cargo
  bin, example, file paths, JS loader, DOM canvas id, build script,
  and 5 video-scene scripts. "Skyglow" is the IES/IDA technical term
  for upward-scattered light pollution, making the demo vendor-neutral
  for IDA chapters and amateur-astronomy outreach. The legacy
  `?wasm=obscura_demo` URL still routes through, and the legacy
  `window.loadObscuraDemo` JS globals are aliased — existing
  links / videos don't break.

### Known issues

- **Skyglow dashboard overlay flickers in Safari** (macOS + iPadOS,
  with WebGPU enabled). The 3D scene underneath renders correctly;
  only the top-left "Skyglow Analysis" panel strobes. The flicker is
  caused by the Bloom + HDR + UI interaction on Safari's WebGPU
  compositor and reproduces with both translucent and opaque UI
  backgrounds. Disabling Bloom locally eliminates it. This is upstream
  territory (Bevy `bevy_post_process::bloom` / wgpu Safari/WebKit
  backend) and is intentionally **not** patched at the application
  level — see `docs/safari_bloom_flicker.md` for the full diagnostic.
  Workaround: view Skyglow in Chrome, Edge, or Firefox until upstream
  ships a fix.

### Build

- Three publishable crates gained `README.md` files (symlinks to the
  workspace README) so `cargo publish` packages them cleanly:
  `eulumdat-i18n`, `eulumdat-typst`, `eulumdat-ffi`.
- Workspace version bump (`0.6.0 → 0.7.0`) covers all internal crates;
  WASM bundles, Python packages (eulumdat-py, eulumdat-quiz-py), and
  Android `versionName` default all sync to 0.7.0.

## [0.4.0] - 2025-12-20

### Added

#### atla (NEW - Unified Photometric Data Model)
- **ATLA-S001 Implementation** - Advanced Transfer Language for photometric Applications:
  - `Atla` struct - Unified data model for all photometric formats
  - XML parsing/writing with full TM-33-23 compatibility
  - JSON serialization for web and API use
  - Lossless conversion between LDT, IES, and ATLA formats

- **Spectral Data Support**:
  - `SpectralData` struct - Wavelength-based intensity distributions (380-780nm)
  - `SpectralPoint` - Individual wavelength/intensity pairs
  - CCT (Correlated Color Temperature) calculation
  - CRI (Color Rendering Index) support
  - Spectral power distribution visualization

- **Horticultural Lighting Metrics** (IES TM-33-23):
  - PPF (Photosynthetic Photon Flux) calculation
  - PPFD (Photosynthetic Photon Flux Density)
  - YPF (Yield Photon Flux) with McCree curve weighting
  - Phytochrome photoequilibrium ratios (Pfr/Ptotal)
  - PAR (Photosynthetically Active Radiation) 400-700nm
  - Far-red (700-800nm) and UV (280-400nm) regions

- **Greenhouse Metrics**:
  - `GreenhouseMetrics` struct with all horticultural calculations
  - DLI (Daily Light Integral) estimation
  - Spectral quality ratios (Red:Far-red, Blue:Red)

#### eulumdat (core library)
- **Full ANSI/IES LM-63-2019 Support** - Comprehensive parsing and export for the latest IES standard:
  - `IesVersion` enum - Support for LM-63-1991, 1995, 2002, and 2019 formats
  - `FileGenerationType` enum - Accredited lab, simulation, scaled, interpolated variants (Section 5.13, Table 2)
  - `LuminousShape` enum - 15 luminous opening shapes from negative dimensions (Section 5.11, Table 1)
  - `TiltData` struct - TILT=INCLUDE parsing with lamp geometry and angle/factor pairs (Annex F)
  - `LampPosition` struct - Lamp position angles from [LAMPPOSITION] keyword (Annex E)
  - `IesData` struct - Complete parsed IES data with all keywords and photometric parameters
  - `IesParser::parse_to_ies_data()` - Parse IES to structured data before Eulumdat conversion
  - `[MORE]` continuation support - Multi-line keyword values (Annex A)
  - Version-specific header parsing (`IES:LM-63-2019`, `IESNA:LM-63-2002`, `IESNA91`)

- **IES-Specific Validation** - 71 validation rules for IES files:
  - Required keywords: [TEST], [TESTLAB], [ISSUEDATE], [MANUFAC]
  - File generation type validation
  - Photometric data consistency checks
  - `validate_ies()` and `validate_ies_strict()` functions
  - `IesValidationWarning` with severity levels

- **IES Export Options**:
  - `IesExportOptions` struct for customizable export
  - `IesExporter::export()` - Default LM-63-2019 format
  - `IesExporter::export_2002()` - Legacy LM-63-2002 format
  - `IesExporter::export_with_options()` - Custom export settings

- **IesMetadata for GLDF Integration** - New struct capturing IES-specific data for GLDF:
  - File provenance: version, test_report, test_lab, issue_date, manufacturer
  - Generation type flags: is_accredited, is_simulation, is_scaled, is_interpolated
  - Luminous shape: dimensions, is_rectangular, is_circular
  - TILT data: has_tilt_data, lamp_geometry, angle_count
  - `to_gldf_properties()` - Export as key-value pairs
  - `to_gldf_emitter_geometry()` - GLDF SimpleGeometry compatible output

- **BeamFieldAnalysis** - Comprehensive beam/field angle analysis:
  - IES definition (based on maximum intensity)
  - CIE/NEMA definition (based on center-beam intensity)
  - Batwing distribution detection
  - `distribution_type()` - Classification: Standard, Mild/Moderate/Strong batwing

#### eulumdat-py (Python bindings)
- `IesVersion`, `FileGenerationType`, `LuminousShape` enums exposed
- `IesMetadata` class with all LM-63-2019 specific fields
- `parse_ies_to_data()` function for structured IES parsing

#### gldf-rs Integration
- `LdtMetadata` enhanced with `ies_metadata` and `photometric_data` fields
- `from_ies_data()` method for IES-specific metadata extraction
- `ldt_to_gldf()` now populates `DescriptivePhotometry` with calculated values:
  - CIE flux code, light output ratio, luminous efficacy
  - Downward/upward flux fractions, DLOR/ULOR
  - Half-peak and tenth-peak divergence angles
  - Cut-off angle, photometric code, BUG rating
  - UGR table values (4H×8H, 70/50/20)

### Changed
- Default IES export format changed from LM-63-2002 to LM-63-2019
- IES header now uses `IES:LM-63-2019` instead of `IESNA:LM-63-2002`
- `[ISSUEDATE]` keyword now required for LM-63-2019 exports

### Fixed
- serde derives added to `PhotometricType` and `UnitType` enums for GLDF integration

---

## [0.3.0] - 2025-12-16

### Added

#### eulumdat (core library)
- **Photometric Calculations Module** (`calculations.rs`) - Comprehensive lighting industry calculations:
  - `PhotometricCalculations` struct with all standard photometric calculations
  - `PhotometricSummary` - Complete photometric summary with all calculated values
  - `GldfPhotometricData` - GLDF (Global Lighting Data Format) compatible export
  - `CieFluxCodes` - CIE flux codes (N1-N5) calculation
  - `ZonalLumens30` - Flux distribution in 30° zones
  - `UgrParams` / `UgrTableValues` - UGR glare rating parameters and results
- **New Calculations**:
  - `downward_flux(ldt, arc)` - Downward flux fraction up to any angle
  - `beam_angle(ldt)` / `field_angle(ldt)` - 50%/10% intensity angles
  - `beam_angle_for_plane(ldt, c_plane)` - Beam angle for specific C-plane
  - `field_angle_for_plane(ldt, c_plane)` - Field angle for specific C-plane
  - `spacing_criterion(ldt, c_plane)` - S/H ratio for uniform illumination
  - `spacing_criteria(ldt)` - S/H for both principal planes
  - `cie_flux_codes(ldt)` - CIE flux code calculation (N1-N5)
  - `luminaire_efficacy(ldt)` - Luminaire efficacy in lm/W
  - `luminaire_efficiency(ldt)` - Efficiency as percentage
  - `zonal_lumens_10deg(ldt)` / `zonal_lumens_30deg(ldt)` - Zonal lumens
  - `k_factor(ldt, room_index, reflectances)` - K-factor (utilance)
  - `ugr(ldt, params)` - Full UGR calculation
  - `luminaire_luminance(ldt, viewing_angle)` - Luminance at angle
  - `cut_off_angle(ldt)` - Cut-off angle (2.5% threshold)
  - `photometric_code(ldt)` - Photometric classification code (e.g., "D-M")
- **Enhanced BUG Diagram**:
  - `BugDiagram::to_svg_with_details()` - SVG with detailed zone lumens table

#### eulumdat-py (Python bindings)
- **New Classes**:
  - `PhotometricSummary` - Complete photometric summary with `to_text()`, `to_compact()`, `to_dict()`
  - `GldfPhotometricData` - GLDF-compatible data with `to_text()`, `to_dict()`
  - `CieFluxCodes` - CIE flux codes (N1-N5) with string formatting
  - `ZonalLumens30` - Zonal lumens with `downward_total()`, `upward_total()`
  - `UgrParams` - UGR calculation parameters with `standard_office()` preset
- **New Methods on `Eulumdat`**:
  - `photometric_summary()` - Returns `PhotometricSummary`
  - `gldf_data()` - Returns `GldfPhotometricData`
  - `cie_flux_codes()` - Returns `CieFluxCodes`
  - `beam_angle()` / `field_angle()` - Beam/field angles in degrees
  - `spacing_criteria()` - Returns (S/H C0, S/H C90) tuple
  - `zonal_lumens_30()` - Returns `ZonalLumens30`
  - `downward_flux(arc)` - Flux percentage to given angle
  - `cut_off_angle()` - Cut-off angle in degrees
  - `photometric_code()` - Classification code string
  - `luminaire_efficacy_lor()` - Efficacy with LOR
  - `calculate_ugr(params)` - UGR value for room config
  - `bug_svg_with_details()` - BUG diagram with zone lumens table

#### eulumdat-cli
- **`summary` command** - Display photometric summary:
  ```bash
  eulumdat summary file.ldt [-f text|compact|json] [-o output.txt]
  ```
- **`gldf` command** - Export GLDF-compatible data as JSON:
  ```bash
  eulumdat gldf file.ldt [-o output.json] [--pretty]
  ```
- **`calc` command** - Calculate specific photometric values:
  ```bash
  eulumdat calc file.ldt -t cie-codes|beam-angles|spacing|zonal-lumens|all
  ```

---

## [0.2.2] - 2025-12-11

### Added

#### eulumdat (core library)
- **`sample(c_angle, g_angle)`** - Bilinear interpolated intensity sampling at any C/G angle
- **Symmetry-aware sampling** - Automatic expansion for all symmetry types

#### eulumdat-photweb (NEW)
- **PhotometricWeb** struct for advanced intensity sampling
- `sample()` and `sample_normalized()` with bilinear interpolation
- **LdcMesh** for 3D photometric solid mesh generation
- Symmetry handling (None, VerticalAxis, PlaneC0C180, PlaneC90C270, BothPlanes)
- Graphics-ready output (positions, normals, indices)

#### eulumdat-wasm (Web App)
- **IES file import support** - Upload and convert IES files directly in the browser

#### EulumdatHarmonyOS (NEW)
- **HarmonyOS/OpenHarmony port** with Cangjie language bindings
- N-API FFI bridge for native Rust integration
- Full ArkUI interface with all diagram types
- eulumdat-harmonyos-ffi crate for C ABI exports

#### macOS/iOS App
- **Room3DView** - Photometric visualization in 3D room environment

### Fixed
- HarmonyOS app icon (was incorrectly using GeoDB icon)

---

## [0.2.1] - 2025-12-08

### Added

#### eulumdat (core library)
- **Batch conversion API** - New `batch` module for efficient bulk processing of multiple LDT/IES files
- Modular code organization for better maintainability

#### eulumdat-cli
- **`batch` command** - Convert multiple files at once: `eulumdat batch input/ -o output/ -f ies`
- Refactored into modular structure (`cli.rs`, `commands.rs`)

#### eulumdat-egui (NEW)
- **Cross-platform desktop GUI application** for Windows, macOS, and Linux
- All diagram types: Polar, Cartesian, Butterfly, 3D, Heatmap, BUG, LCS
- 7 information tabs: Diagram, General, Dimensions, Lamps, Optical, Intensity, Validation
- Built-in template library for quick testing
- Export to SVG, IES, LDT formats
- Intensity table with **Copy CSV** button and **color toggle**
- Validation tab showing both errors and warnings

#### eulumdat-py (Python bindings)
- **Batch conversion** - `batch_convert()` function for efficient bulk processing
- `BatchInput`, `BatchOutput`, `BatchStats` classes
- `InputFormat` and `ConversionFormat` enums

#### eulumdat-ffi (Swift/Kotlin bindings)
- Batch conversion support
- Modular code organization matching core library

#### eulumdat-windows-preview (NEW)
- **Windows Shell Preview Handler** for File Explorer
- Preview LDT files in Explorer's preview pane (Alt+P)
- Renders polar diagram SVG

#### macOS/iOS App (EulumdatApp)
- **QuickLook Extension** - Preview .ldt files directly in Finder (press Space)
- **Intensity tab improvements**:
  - Copy CSV button for exporting data to spreadsheets
  - Color toggle to show/hide heatmap coloring
- Updated app icon with "LDT/IES" branding
- Fixed app icon transparency for App Store compliance

#### Android App (EulumdatAndroid)
- Interactive 3D butterfly diagram with touch rotation
- Template library matching iOS app
- Updated launcher icon with "LDT/IES" text

#### CI/CD
- All crates published to crates.io (eulumdat, cli, egui, ffi, wasm, py, windows-preview)
- Python wheels published to PyPI for all platforms
- GitHub Actions workflow for building GUI binaries (Windows, macOS, Linux)
- Android APK builds included in releases
- Swift XCFramework builds and releases
- Optional Google Play Store deployment

### Changed
- Improved intensity table layout with smaller, more compact cells
- Validation tab now shows both errors (red) and warnings (yellow)
- Release artifacts categorized by type (cli-*, gui-*, android-*)

### Fixed
- Python bindings error handling compatibility with nightly Rust
- App icons now use solid background (no alpha channel) for App Store compliance
- Swift workflow patching for UniFFI generated bindings
- crates.io publishing with proper version specifications

## [0.2.0] - 2024-11-15

### Added
- Initial release with core parsing, validation, and diagram generation
- CLI tool with info, validate, convert, diagram, and bug commands
- Python bindings via PyO3
- Swift/Kotlin bindings via UniFFI
- WebAssembly editor
- BUG rating calculation (IESNA TM-15-11)
- 6 diagram types: Polar, Cartesian, Butterfly, Heatmap, BUG, LCS
