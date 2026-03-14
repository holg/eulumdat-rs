# Zonal Lighting Designer — Design Plan

## Overview

A self-contained SVG-based interior lighting design tool in the "Designer" tab.
Users define a rectangular room, select a luminaire, and the tool calculates
the required number of fixtures, optimal layout, and estimated illuminance
using the IES Zonal Cavity Method — the standard hand-calculation method for
uniform interior lighting design.

Additionally provides point-by-point illuminance visualization on the workplane
for a more accurate picture than the zonal-cavity average alone.

This is the interior counterpart to the Area Lighting Designer (exterior).
Together they replicate the full Luxiflux suite (Zonal + Area) as open-source
WASM tools powered by eulumdat-rs.

No external dependencies. Pure Rust/SVG computation.

---

## Background: Zonal Cavity Method

The zonal cavity method (also called the lumen method) is the IES-standard
simplified method for calculating average illuminance in rectangular rooms
with uniform luminaire layouts. It divides the room into three cavities:

```
┌─────────────────────────┐ ─┐
│     Ceiling Cavity       │  │ hcc (ceiling to luminaire plane)
├─────────────────────────┤ ─┤
│                         │  │
│     Room Cavity          │  │ hrc (luminaire plane to workplane)
│                         │  │
├─────────────────────────┤ ─┤
│     Floor Cavity         │  │ hfc (workplane to floor)
└─────────────────────────┘ ─┘
```

### Core Formulas

**Cavity Ratio:**
```
CR = 5 × h × (L + W) / (L × W)
```
where h = cavity height, L = room length, W = room width.

This yields RCR (room cavity ratio), CCR (ceiling cavity ratio), FCR (floor cavity ratio).

**Effective Cavity Reflectances:**
Convert multi-surface cavities into single equivalent reflectances:
- ρcc_eff = f(ceiling reflectance ρcc, wall reflectance ρw, CCR)
- ρfc_eff = f(floor reflectance ρfc, wall reflectance ρw, FCR)

If luminaire is surface-mounted/recessed: CCR = 0, ρcc_eff = ρcc (actual).
If workplane is on the floor: FCR = 0, ρfc_eff = ρfc (actual).

**Coefficient of Utilization (CU):**
The fraction of lamp lumens reaching the workplane. Depends on:
- Luminaire photometric distribution (from LDT/IES data)
- Room Cavity Ratio (RCR)
- Effective ceiling reflectance (ρcc_eff)
- Wall reflectance (ρw)

CU is the key bridge between photometric data and room-level performance.
Computing it from raw candela data is the core technical challenge.

**Average Illuminance:**
```
E_avg = (N × n × Φ_lamp × CU × LLF) / A
```
where:
- N = number of luminaires
- n = lamps per luminaire
- Φ_lamp = lumens per lamp (or luminaire lumens for absolute photometry)
- CU = coefficient of utilization
- LLF = light loss factor (maintenance factor)
- A = room area (L × W)

**Number of Luminaires (inverse):**
```
N = (E_target × A) / (n × Φ_lamp × CU × LLF)
```

**Lighting Power Density:**
```
LPD = (N × W_luminaire) / A    [W/m² or W/ft²]
```

---

## Data Model

### Core Types (in `eulumdat` crate)

```rust
/// Room definition for interior zonal cavity calculation.
struct Room {
    length: f64,              // room length in meters
    width: f64,               // room width in meters
    height: f64,              // floor to ceiling height in meters
    workplane_height: f64,    // workplane above floor (typ. 0.8m office, 0m corridor)
    suspension_length: f64,   // luminaire suspension below ceiling (0 = recessed/surface)
}

/// Room surface reflectances (0.0–1.0).
struct Reflectances {
    ceiling: f64,     // ρcc — typical: 0.70–0.80 (white ceiling)
    walls: f64,       // ρw  — typical: 0.50 (light colored walls)
    floor: f64,       // ρfc — typical: 0.20 (dark floor/carpet)
}

/// Luminaire electrical and optical properties extracted from LDT/IES
/// plus user overrides.
struct LuminaireSpec {
    luminaire_flux: f64,     // total luminaire lumens (from LDT/IES)
    lamp_flux: f64,          // rated lamp lumens (for relative photometry)
    wattage: f64,            // input watts
    is_absolute: bool,       // absolute vs relative photometry
    spacing_criterion: f64,  // max spacing-to-mounting-height ratio (S/MH)
                             // computed from photometric distribution
}

/// Light Loss Factor components.
struct LightLossFactor {
    lld: f64,    // Lamp Lumen Depreciation (typ. 0.85–0.95 for LED)
    ldd: f64,    // Luminaire Dirt Depreciation (typ. 0.90–0.95)
    bf: f64,     // Ballast/Driver Factor (typ. 1.0 for LED)
    rsdd: f64,   // Room Surface Dirt Depreciation (typ. 0.95–0.98)
}

impl LightLossFactor {
    fn total(&self) -> f64 {
        self.lld * self.ldd * self.bf * self.rsdd
    }
}

/// Cavity calculation intermediate results.
struct CavityResults {
    rcr: f64,                // Room Cavity Ratio
    ccr: f64,                // Ceiling Cavity Ratio
    fcr: f64,                // Floor Cavity Ratio
    rho_cc_eff: f64,         // Effective ceiling cavity reflectance
    rho_fc_eff: f64,         // Effective floor cavity reflectance
}

/// CU table: CU values indexed by [RCR][reflectance_combo].
/// Generated from photometric data.
struct CuTable {
    /// RCR values (rows): 0, 1, 2, ..., 10
    rcr_values: Vec<f64>,
    /// Reflectance combinations (columns):
    /// [(ρcc_eff, ρw)]: e.g. (80,70), (80,50), (80,30), (70,50), (70,30), (50,30), ...
    reflectance_combos: Vec<(u8, u8)>,
    /// CU values: cu_values[rcr_index][reflectance_index]
    cu_values: Vec<Vec<f64>>,
    /// Assumed floor reflectance for this table (standard: 20%)
    base_floor_reflectance: f64,
}

/// Zonal lumen summary — flux by angular zone.
/// Used for display/report (IES photometric report format).
/// NOT used for CU computation — that uses existing ZonalLumens30 (6 × 30° zones).
struct ZonalLumenSummary {
    /// Lumens in each 10° zone: 0-10, 10-20, ..., 170-180 (18 zones)
    zone_lumens: Vec<f64>,
    /// Cumulative lumens: 0-10, 0-20, ..., 0-180
    cumulative_lumens: Vec<f64>,
    /// Percent lamp/luminaire for each zone
    zone_percent: Vec<f64>,
    /// Total downward lumens (0°–90°)
    downward_lumens: f64,
    /// Total upward lumens (90°–180°)
    upward_lumens: f64,
    /// Downward/upward ratio
    dff: f64,  // Downward Flux Fraction
}

/// Result of zonal cavity calculation.
struct ZonalResult {
    // Cavity analysis
    cavity: CavityResults,
    cu: f64,                     // interpolated CU for this room

    // Zonal cavity (average) results
    num_luminaires: usize,       // required luminaire count (rounded up)
    avg_illuminance: f64,        // achieved average illuminance (lux or fc)
    target_illuminance: f64,     // user-specified target
    lpd: f64,                    // lighting power density (W/m² or W/ft²)
    llf: f64,                    // total light loss factor used

    // Layout
    layout: LuminaireLayout,     // rows × cols arrangement
    spacing_x: f64,              // luminaire spacing along length
    spacing_y: f64,              // luminaire spacing along width
    spacing_ok: bool,            // spacing ≤ S/MH criterion × mounting_height

    // Point-by-point overlay (optional, computed separately)
    ppb_grid: Option<Vec<Vec<f64>>>,  // illuminance at each workplane point
    ppb_min: Option<f64>,
    ppb_max: Option<f64>,
    ppb_uniformity: Option<f64>,      // min/avg from point-by-point
}

/// Grid layout of luminaires in the room.
struct LuminaireLayout {
    rows: usize,           // number of rows (along width)
    cols: usize,           // number of columns (along length)
    offset_x: f64,         // first luminaire offset from wall (length direction)
    offset_y: f64,         // first luminaire offset from wall (width direction)
}
```

### CU Table Computation from Photometric Data

This is the core technical challenge. The CU table is NOT typically shipped
with EULUMDAT/IES files — it must be computed from the candela distribution.

```rust
/// Compute CU table from photometric data.
///
/// Algorithm (IES method):
/// 1. Compute zonal lumens: integrate candela over each 10° gamma zone
///    Φ_zone = ∫∫ I(C,γ) sin(γ) dγ dC  for each zone
/// 2. For each room configuration (RCR, ρcc_eff, ρw):
///    a. Compute direct ratio (DR): fraction of downward lumens
///       reaching workplane directly (from zonal lumens + geometry)
///    b. Compute room surface transfer functions using flux transfer
///       method (inter-reflections between ceiling, walls, floor)
///    c. CU = (direct component + inter-reflected component) / total lamp lumens
///
/// The flux transfer method models the room as a three-surface enclosure
/// (ceiling cavity, walls, floor cavity) and solves for the equilibrium
/// flux distribution using form factors derived from the cavity ratios.
fn compute_cu_table(
    ldt: &Eulumdat,
    rcr_values: &[f64],                  // e.g. [0, 1, 2, ..., 10]
    reflectance_combos: &[(u8, u8)],     // (ρcc_eff, ρw) pairs
    base_floor_reflectance: f64,         // standard: 0.20
) -> CuTable
```

#### Zonal Lumen Computation

```rust
/// Compute lumens in each 10° gamma zone from candela distribution.
///
/// For each zone [γ1, γ2]:
///   Φ = ∫₀²π ∫_γ1^γ2 I(C,γ) × sin(γ) dγ dC
///
/// For axially symmetric luminaires (single C-plane):
///   Φ = 2π × ∫_γ1^γ2 I(γ) × sin(γ) dγ
///
/// For multi-plane luminaires, average I across all C-planes first,
/// or integrate over C using the trapezoidal rule on available planes.
fn compute_zonal_lumens(ldt: &Eulumdat) -> ZonalLumenSummary
```

#### Direct Ratio Computation

```rust
/// Compute the direct ratio for a given RCR.
///
/// The direct ratio is the fraction of downward luminaire flux
/// that reaches the workplane on first bounce (no inter-reflections).
///
/// Computed from zonal lumens using the relationship between
/// gamma zones and the solid angle subtended by the workplane
/// as seen from the luminaire, which varies with RCR.
///
/// Uses the IES standard direct ratio coefficients:
/// DR = Σ (zone_fraction × K_zone(RCR))
/// where K_zone values relate each gamma zone's contribution
/// to the fraction reaching the workplane for a given RCR.
fn compute_direct_ratio(
    zonal_summary: &ZonalLumenSummary,
    rcr: f64,
) -> f64
```

#### Flux Transfer (Inter-Reflection) Method

```rust
/// Solve the inter-reflection equilibrium using flux transfer.
///
/// Models the room as a three-surface enclosure:
///   - Ceiling (or ceiling cavity with ρcc_eff)
///   - Walls (with ρw)
///   - Floor/workplane (with ρfc_eff, typically 20%)
///
/// The transfer matrix relates flux leaving each surface to flux
/// arriving at each other surface, using form factors derived from
/// the cavity ratios.
///
/// Solves: Φ_received = [I - ρ×F]⁻¹ × Φ_direct
/// where F is the form factor matrix and ρ is the reflectance diagonal.
///
/// CU = Φ_workplane / Φ_lamp_total
fn compute_cu_with_interreflections(
    direct_ratio: f64,
    upward_fraction: f64,
    rcr: f64,
    rho_cc_eff: f64,
    rho_w: f64,
    rho_fc_eff: f64,
) -> f64
```

### Main Calculation

```rust
/// Full zonal cavity calculation for a room.
fn compute_zonal(
    ldt: &Eulumdat,
    room: &Room,
    reflectances: &Reflectances,
    llf: &LightLossFactor,
    target_illuminance: f64,   // desired average lux (or fc)
    cu_table: &CuTable,        // pre-computed, or computed on-the-fly
) -> ZonalResult
```

Steps:
1. Compute cavity heights: hrc = height - workplane_height - suspension_length,
   hcc = suspension_length, hfc = workplane_height
2. Compute cavity ratios: RCR, CCR, FCR
3. Compute effective cavity reflectances: ρcc_eff, ρfc_eff
4. Look up CU from table (interpolate RCR and reflectance values)
5. Correct CU for floor reflectance ≠ 20% if needed:
   CU_corrected = CU × multiplier(ρfc_eff, RCR, ρcc_eff)
   The IES Lighting Handbook Table 9.5 provides correction multipliers.
   For ρfc_eff > 20%, CU increases; for ρfc_eff < 20%, CU decreases.
   The correction is typically small (±2–5%) but matters for accuracy.
6. For absolute photometry: use luminaire lumens and RUF instead of
   lamp lumens and CU. RUF = CU × (lamp_lumens / luminaire_lumens).
   Since absolute photometry has lamp_lumens = -1, compute
   RUF directly from the inter-reflection model using luminaire flux.
7. Compute N = (E_target × A) / (Φ × CU_corrected × LLF)
8. Round N up to integer, compute achievable layout (rows × cols)
9. Check spacing criterion: spacing ≤ S/MH × mounting_height
10. Recalculate achieved E_avg with actual N
11. Compute LPD = (N × watts) / A

### Point-by-Point Overlay

The zonal cavity result gives only an average. For visualization and
uniformity assessment, compute point-by-point illuminance on the workplane
using the same engine as the Area tool (inverse-square-law + cosine):

```rust
/// Point-by-point illuminance on the workplane for the computed layout.
/// Uses direct illuminance only (no inter-reflections) for speed,
/// or optionally adds a diffuse inter-reflection estimate.
fn compute_ppb_overlay(
    ldt: &Eulumdat,
    layout: &LuminaireLayout,
    room: &Room,
    grid_resolution: usize,
    llf: f64,
    include_reflected: bool,  // add estimated reflected component
) -> (Vec<Vec<f64>>, f64, f64, f64)  // (grid, min, max, uniformity)
```

When `include_reflected` is true, add a uniform "ambient" term to each
point based on the inter-reflected flux estimate from the CU calculation:
```
E_reflected ≈ E_avg_zonal - E_avg_direct_only
```
This is an approximation but gives a reasonable total illuminance map
without full radiosity computation.

---

## Preset Room Types

| Room Type        | Target Lux | Workplane | Typical Size  |
|-----------------|-----------|-----------|---------------|
| Open Office      | 500       | 0.80m     | 20m × 15m     |
| Private Office   | 500       | 0.80m     | 4m × 3m       |
| Classroom        | 500       | 0.80m     | 10m × 8m      |
| Corridor         | 100       | 0.00m     | 30m × 2m      |
| Warehouse        | 200       | 0.00m     | 40m × 30m     |
| Retail           | 500       | 0.00m     | 20m × 15m     |
| Workshop         | 750       | 0.85m     | 15m × 10m     |
| Conference Room  | 500       | 0.80m     | 8m × 5m       |
| Restroom         | 200       | 0.00m     | 4m × 3m       |
| Parking Garage   | 75        | 0.00m     | 30m × 15m     |

Target lux values based on EN 12464-1 (Europe) / IES RP-1 (North America).
Presets auto-fill room dimensions, workplane height, and target illuminance.
User can always override.

---

## Reflectance Presets

| Preset           | Ceiling | Walls | Floor |
|-----------------|---------|-------|-------|
| Standard (LCC)   | 0.70    | 0.50  | 0.20  |
| Bright Room      | 0.80    | 0.70  | 0.30  |
| Dark Room        | 0.50    | 0.30  | 0.10  |
| Industrial       | 0.50    | 0.30  | 0.20  |
| Custom           | —       | —     | —     |

"LCC" = Light Ceiling, Colored walls — the IES default assumption.

---

## UI Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Toolbar                                                          │
│  Room Type: [Open Office ▼]   Target: [500 lx ──●──]           │
│  Length: [20m]  Width: [15m]  Height: [3.0m]                    │
│  Workplane: [0.80m ──●──]  Suspension: [0.0m ──●──]            │
│  Reflectances: [Standard ▼]  C: 70%  W: 50%  F: 20%            │
├───────────┬─────────────────────────────────────────────────────┤
│ Left      │  Room View (top-down SVG)                            │
│ Panel     │                                                      │
│           │  ┌──────────────────────────────┐                    │
│ Luminaire │  │ ·  ·  ·  ·  ·  ·  ·  ·  · │  ← luminaire grid  │
│ Info      │  │                              │                    │
│           │  │ ·  ·  ·  ·  ·  ·  ·  ·  · │                    │
│ ● 4000 lm │  │                              │                    │
│ ● 36 W    │  │ ·  ·  ·  ·  ·  ·  ·  ·  · │                    │
│ ● S/MH:1.4│  │                              │                    │
│           │  └──────────────────────────────┘                    │
│           │  20.0m × 15.0m  |  3×9 = 27 luminaires              │
│───────────│                                                      │
│ Results   │─────────────────────────────────────────────────────│
│           │  Illuminance View (point-by-point heatmap)           │
│ Zonal     │                                                      │
│ Cavity:   │  ┌──────────────────────────────┐                    │
│           │  │                              │                    │
│ RCR: 2.6  │  │  Point-by-point illuminance  │                    │
│ CU: 0.63  │  │  on workplane. Colored bands │                    │
│ N: 27     │  │  or grayscale heatmap with   │                    │
│           │  │  contour lines.              │                    │
│ E_avg:    │  │                              │                    │
│  537 lx   │  └──────────────────────────────┘                    │
│ LPD:      │                                                      │
│  3.2 W/m² │  Point-by-Point: Min: 412 lx  Max: 623 lx          │
│           │                  Uniformity (min/avg): 0.77          │
│ Spacing:  │                                                      │
│  2.2m × 5m│                                                      │
│  S/MH OK ✓│                                                      │
│           │                                                      │
│───────────│                                                      │
│ LLF       │                                                      │
│           │                                                      │
│ LLD: 0.90 │                                                      │
│ LDD: 0.95 │                                                      │
│ BF:  1.00 │                                                      │
│ RSDD:0.98 │                                                      │
│ Total:0.84│                                                      │
│           │                                                      │
│───────────│                                                      │
│ CU Table  │                                                      │
│ (toggle)  │                                                      │
│           │                                                      │
│ RCR 80/70 │                                                      │
│  0  .82   │                                                      │
│  1  .76   │                                                      │
│  2  .69   │                                                      │
│  3  .63 ← │                                                      │
│  4  .57   │                                                      │
│  ...      │                                                      │
└───────────┴─────────────────────────────────────────────────────┘
```

---

## Features

### Room Configuration
- Room type presets (auto-fill dimensions + target + workplane)
- Manual dimension entry (length, width, height)
- Workplane height slider (0m floor to full room height)
- Suspension length slider (0 = recessed/surface, up to room height)
- Reflectance presets + custom sliders per surface

### Luminaire Information
- Auto-extracted from loaded LDT/IES: lumens, wattage, lamp count
- Spacing criterion (S/MH) computed from photometric distribution
- Downward flux fraction displayed
- User overrides for wattage and driver/ballast factor
- Zonal lumen summary (collapsible, shows flux distribution by zone)

### Zonal Cavity Calculation
- Real-time: recomputes instantly on any input change
- Shows all intermediate values: cavity ratios, effective reflectances, CU
- Luminaire count rounded up to nearest feasible grid (rows × cols)
- Achieved average illuminance (not just target)
- LPD computation
- Spacing criterion check (green ✓ / red ✗)

### CU Table Display
- Toggleable panel showing the full CU table computed from photometry
- Current operating point highlighted (interpolated RCR + reflectances)
- Standard format matching IES photometric reports
- Exportable as part of the indoor photometric report

### Layout View (top-down)
- SVG room outline with dimension labels
- Luminaire grid positions shown as dots
- Spacing dimensions labeled
- Wall offset distances shown
- Door/window indicators (cosmetic, future phase)

### Illuminance View (point-by-point)
- Heatmap of workplane illuminance (direct + estimated reflected)
- Colored bands (AEC style) or grayscale
- Contour lines with lux labels
- Min/max/avg from point-by-point (more accurate than zonal average)
- Uniformity ratio (min/avg) — the metric that zonal cavity alone cannot provide
- Toggle between plan (flat) and isometric view

### 3D Room Scene View
Interactive axonometric scene showing the physical room — the visual
that sells the tool to non-technical users. Renders as SVG (no WebGL).
Uses the shared `scene3d` module from the Area designer.

**Scene elements (interior):**
- Room box: floor, 4 walls, ceiling rendered as separate faces.
  Back walls semi-transparent so interior is visible.
  Wall reflectance visualized as fill brightness (ρw=0.7 → light gray,
  ρw=0.3 → dark gray). Helps users intuit reflectance impact.
- Ceiling plane: shows luminaire grid as rectangles/circles matching
  LDT luminous opening dimensions. Luminaire shapes inset into ceiling
  if recessed (suspension=0), hanging below if suspended.
  Ceiling grid tiles (optional toggle) — shows T-bar grid at configured
  tile size (600×600mm or 2'×2') as dashed lines on ceiling face.
- Suspension rods: thin vertical lines from ceiling to luminaire if
  suspension_length > 0.
- Workplane: semi-transparent colored plane at workplane_height showing
  illuminance heatmap (colored bands from PPB calculation).
  Partially transparent so floor is visible below.
  Contour lines on the workplane surface.
- Floor: base plane, darker fill matching floor reflectance.
- Cavity visualization (optional toggle): dashed horizontal lines at
  luminaire plane and workplane height, with labels "hcc", "hrc", "hfc"
  on the side wall — directly matches the cavity diagram from the
  Background section. Educational feature for students/presentations.
- Dimension labels: room length/width/height, workplane height,
  suspension length as leader lines on room edges.

**Interaction:**
- Camera preset buttons: [Front-Right] [Front-Left] [Top-Down] [Section]
  "Section" preset: azimuth=0°, elevation=0° — side cutaway view showing
  the three cavities clearly. Hides front wall for visibility.
- Mouse wheel: zoom
- Click luminaire in 3D view: highlight it (cosmetic, no per-luminaire
  editing in zonal — uniform grid only)
- Toggle switches: [Walls ✓] [Ceiling Grid ☐] [Cavities ☐] [Workplane ✓]
  [Dimensions ☐]
- Wall transparency slider: 0% (opaque) to 80% (nearly invisible)

**Rendering pipeline:**
Same as exterior (shared scene3d module):
1. Build face list: floor (1), walls (4, back walls flagged for transparency),
   ceiling (1), luminaire faces (N), workplane (1), optional cavity lines
2. Project all vertices through SceneCamera
3. Z-sort back-to-front
4. Emit SVG polygons with fill/stroke/opacity
5. Overlay dimension labels at projected positions

### Solve Modes
Three calculation modes, switchable:

| Mode               | Input              | Output                    |
|-------------------|--------------------|---------------------------|
| Target → Count    | Target illuminance  | Number of luminaires      |
| Count → Illuminance| Number of luminaires| Achieved illuminance      |
| Target → LPD      | Target LPD (W/m²)  | Achievable illuminance    |

### Light Loss Factor
- Individual component sliders with tooltips explaining each factor
- LED presets (LLD=0.90, LDD=0.95, BF=1.0, RSDD=0.98 → LLF=0.84)
- Fluorescent presets (LLD=0.85, LDD=0.90, BF=0.95, RSDD=0.96)
- Custom per-component override
- Combined LLF displayed and used in calculation

### Export
- CU table as CSV (matches IES indoor report format)
- Zonal lumen summary as CSV
- Room layout as SVG
- Illuminance heatmap as SVG
- Full indoor photometric report as PDF (via Typst):
  - Candela distribution table
  - Polar curve
  - Zonal lumen summary
  - CU table
  - Room calculation results
  - Point-by-point visualization

---

## File Structure

```
crates/
  eulumdat/src/
    scene3d/
      mod.rs              — SceneCamera, CameraPreset, SceneFace, project()
      render.rs           — render_scene_svg(), z-sort, SVG emission
      exterior.rs         — build exterior scene: ground, poles, arms, luminaires, cones
      interior.rs         — build interior scene: room box, ceiling, luminaires, workplane

    zonal/
      mod.rs              — pub mod, Room, Reflectances, ZonalResult, etc.
      cavity.rs           — cavity ratio computation, effective reflectances
      cu_table.rs         — CU table generation from photometric data
      zonal_lumens.rs     — zonal lumen summary computation
      direct_ratio.rs     — direct ratio calculation
      interreflection.rs  — flux transfer / inter-reflection solver
      compute.rs          — compute_zonal() main entry point
      layout.rs           — luminaire grid layout generator
      ppb_overlay.rs      — point-by-point overlay on workplane
      spacing.rs          — spacing criterion computation from photometry
      presets.rs          — room type presets, reflectance presets
      svg.rs              — room layout SVG, illuminance heatmap SVG

  eulumdat-wasm/src/components/
    zonal_designer.rs     — main Leptos component
    zonal_room_view.rs    — room plan view (SVG)
    zonal_illum_view.rs   — point-by-point heatmap view
    zonal_scene_view.rs   — 3D room scene component (camera controls, toggles)
    zonal_controls.rs     — toolbar, room config, LLF panel
    zonal_cu_display.rs   — CU table display panel
    zonal_results.rs      — results panel, zonal lumen summary
```

---

## Existing Infrastructure (Already Implemented)

The following components already exist in `crates/eulumdat/src/calculations.rs`
and can be reused directly — no need to rebuild:

- [x] `CuTable::calculate(ldt)` — full 11×18 CU table from photometry (line 2619)
- [x] `CuTable::calculate_cu_ies()` — IES inter-reflection model (line 2658)
- [x] `CuTable::calculate_direct_ratio_ies()` — direct ratio from zonal lumens (line 2726)
- [x] `ZonalLumens30` — 6-zone (30°) flux breakdown with `downward_total()` / `upward_total()` (line 2454)
- [x] `PhotometricCalculations::zonal_lumens_30deg()` — zonal integration (line 1013)
- [x] `PhotometricCalculations::spacing_criterion_ies()` — S/MH ratio (line 885)
- [x] `PhotometricCalculations::downward_flux()` — flux integration to any angle (line 27)
- [x] `CU_REFLECTANCES` (18 combos), `CU_RCR_VALUES` (0–10) — standard constants
- [x] `compute_area_illuminance()` — point-by-point grid engine (reuse for workplane PPB)
- [x] `heatmap_color()`, `marching_squares()` — heatmap + contour rendering

Partially created module structure:
- [x] `crates/eulumdat/src/zonal/mod.rs` — module declarations
- [x] `crates/eulumdat/src/zonal/presets.rs` — RoomPreset (10 types), ReflectancePreset (4), LlfPreset (2)

---

## Phases

### Phase 1 — Core Types + Cavity Ratios (Step 1–2)
- [x] `Room` struct (length, width, height, workplane_height, suspension_length)
      with `hrc()`, `hcc()`, `hfc()`, `area()` helpers
- [x] `Reflectances` struct (ceiling, walls, floor as 0.0–1.0)
- [x] `LightLossFactor` struct (lld, ldd, bf, rsdd) with `total()`
- [ ] `LuminaireSpec` struct — extract from parsed LDT/IES on load:
      luminaire_flux, lamp_flux, wattage, is_absolute, spacing_criterion.
      For EULUMDAT: luminaire_flux from header, is_absolute always false.
      For IES: check lamp_lumens == -1 for absolute photometry.
      spacing_criterion from existing `spacing_criterion_ies()`.
- [x] `CavityResults` struct (rcr, ccr, fcr, rho_cc_eff, rho_fc_eff)
- [x] `LuminaireLayout` struct (rows, cols, offset_x, offset_y, spacing, S/MH)
- [x] `SolveMode` enum (TargetToCount, CountToIlluminance, TargetToLpd)
- [x] `ZonalResult` struct (cavity, cu, layout, avg/target illuminance,
      lpd, llf, spacing_criterion, ppb)
- [x] `compute_cavity_ratios()` — CR = 5·h·(L+W)/(L·W) for each cavity
- [x] `effective_cavity_reflectance()` — IES approximation formula
      Special case: CR = 0 → ρ_eff = ρ_base (passthrough, no cavity).
- [x] `interpolate_cu()` — bilinear interpolation across pre-computed CuTable
- [x] `find_best_layout()` — factor pair search with aspect ratio scoring
- [x] Unit tests for cavity ratios, effective reflectance, CU interpolation, layout

File: `crates/eulumdat/src/zonal/compute.rs`
Wire: `pub mod zonal;` in `lib.rs`

### Phase 2 — Main Calculation + Minimal UI (Step 3–4)
- [x] `compute_zonal()` main entry point — all 3 solve modes
- [x] `ZonalSvg::room_plan()` — top-down room with luminaire grid, spacing labels
- [x] `ZonalSvg::section_view()` — side section showing 3 cavities with dimensions
- [x] `ZonalDesigner` Leptos component (room presets, dimensions, results, solve modes)
- [x] Wire into `app.rs` as "Interior" tab (after "Exterior" area designer)
- [x] End-to-end test with sample LDT file

Files: `zonal/compute.rs`, `zonal/svg.rs`, `zonal_designer.rs`, `app.rs`, `mod.rs`

### Phase 3 — Point-by-Point Overlay + Visualization (Step 5)
- [x] `compute_ppb_overlay()` — construct LuminairePlace instances from layout,
      call `compute_area_illuminance()` for direct component on workplane grid,
      add uniform reflected estimate: E_reflected = E_avg_zonal − E_avg_direct,
      clamp to 0 if negative.
- [x] `ZonalSvg::illuminance_view()` — PPB heatmap with contours
      (reuses heatmap_color + marching_squares from area iso_view)
- [x] Min/max/avg/uniformity from point-by-point
- [x] PPB toggle in component, render heatmap panel
- [x] **3D Room Scene View (interior):**
  - [x] `scene3d` module (shared with Area): SceneCamera, CameraPreset,
        SceneFace, project(), render_scene_svg(), fit_scale()
  - [x] `build_interior_scene()` — room box (floor + 4 walls + ceiling),
        luminaire grid on ceiling, workplane with illuminance heatmap texture,
        suspension rods if suspension > 0
  - [x] `build_exterior_scene()` — ground heatmap, poles, arms, luminaire
        heads, optional light cones (for Area designer)
  - [x] Back walls semi-transparent (0.35), front walls very transparent (0.15)
  - [x] Wall fill brightness mapped to reflectance values (reflectance_to_rgb)
  - [x] Camera presets: Front-Right, Front-Left, Top-Down, Low-Angle, Section
  - [x] Cavity visualization mode: dashed lines at luminaire plane + workplane
        with hcc/hrc/hfc labels — educational/presentation feature
  - [x] Toggle: 3D Room view checkbox, Cavity lines checkbox, camera dropdown
  - [x] View toggle: [Heatmap] [3D Room] [Section] [CU Table] (unified tab toggle)

### Phase 4 — Solve Modes + LLF + Polish (Step 6–8)
- [x] CountToIlluminance mode: E = N·Φ·CU·LLF / A
- [x] TargetToLpd mode: N = target_lpd·A / wattage
- [x] Solve mode dropdown with conditional inputs
- [x] LLF component sliders with presets (LED, fluorescent)
- [x] Reflectance custom sliders per surface
- [x] CU table display panel (toggleable, highlighted operating point)
- [x] `ZonalSvg::cu_table_svg()` — tabular SVG with highlighted cell
- [x] Luminaire info sidebar (auto from LDT: lumens, watts, S/MH, DFF)
- [x] Zonal lumen summary display
- [x] URL hash serialization for shareable links
- [x] PDF export via Typst (same pattern as area designer):
  - Typst source generated inline with embedded SVG images
  - Report includes: luminaire info, room parameters, reflectances, LLF,
    cavity ratios, CU, results table (count, achieved lux, LPD, spacing),
    room plan SVG, section view SVG, PPB heatmap SVG (if enabled),
    CU table SVG (if enabled)
  - `compileTypstToPdf()` JS bridge (shared with area designer)
  - Fallback: download .typ source if PDF compilation fails
  - "Export PDF" button in toolbar, disabled while exporting

### Phase 5 — Export + Photometric Report (deferred)
- [ ] CU table CSV export (IES indoor report format)
- [ ] Zonal lumen summary CSV export
- [ ] Room layout SVG export
- [ ] Illuminance heatmap SVG export
- [ ] Full indoor photometric report PDF via Typst:
  candela table, polar curve, zonal lumens, CU table, room results

### Phase 6 — Advanced (deferred)
- [ ] UGR (Unified Glare Rating) computation and display
- [ ] Multiple room comparison (side-by-side rooms with same luminaire)
- [ ] Multiple luminaire comparison (same room with different luminaires)
- [ ] Non-rectangular rooms (L-shapes, T-shapes) via room subdivision
- [ ] Daylight integration factor
- [ ] Emergency lighting calculation (reduced lumen output)
- [ ] Save/load room configurations
- [ ] EN 12464-1 / IES RP-1 compliance checker (target met? LPD within limit?)

---

## Technical Notes

### CU Table Validation Strategy

The CU computation is the credibility-critical piece. Validation approach:

1. **Reference luminaires:** Select 5–10 luminaires with published CU tables
   (from IES Lighting Handbook, manufacturer catalogs, or AGi32 Photometric
   Toolbox output). Ensure mix of:
   - Direct (downlight/troffer)
   - Semi-direct (pendant with uplight component)
   - Indirect (uplight)
   - Symmetric vs asymmetric distributions

2. **Tolerance:** CU values should match published tables within ±0.02
   (i.e., 2 percentage points). IES handbook notes that "approximate, eyeball
   interpolation" is acceptable — the method itself is approximate.

3. **Cross-reference tools:** AGi32 Photometric Toolbox computes CU tables
   from IES/EULUMDAT files and is considered the reference implementation.
   Compare output against it.

### Absolute vs Relative Photometry

- **Relative photometry:** IES file specifies lamp lumens separately from
  candela values. CU = luminaire lumens reaching workplane / lamp lumens.
  Formula uses lamp lumens.

- **Absolute photometry:** IES file uses -1 for lamp lumens. Candela values
  represent actual luminaire output. Use Room Utilization Factor (RUF, also
  called utilance) instead of CU, with luminaire lumens in the formula.

  For absolute photometry:
  ```
  E_avg = (N × Φ_luminaire × RUF × LLF) / A
  ```
  where RUF = Φ_workplane / Φ_luminaire (not Φ_lamp).

- **EULUMDAT (.ldt):** Always relative photometry. The file header contains
  both total luminaire lumens and rated lamp lumens. The existing
  `CuTable::calculate()` already handles this correctly since it works
  from the candela distribution directly.

The eulumdat-rs parser already distinguishes absolute from relative.
The zonal calculation must handle both correctly. In practice, almost all
modern LED luminaires use absolute photometry in IES files. EULUMDAT files
are always relative by format definition.

### Spacing Criterion Computation

The spacing criterion (S/MH or SC) defines the maximum spacing-to-mounting-
height ratio for acceptable uniformity. Computed from the photometric
distribution:

1. Find the angle at which intensity drops to 50% of nadir intensity
   (for each principal C-plane)
2. SC = 2 × tan(θ_50%) for each plane
3. Report the minimum SC across planes (most restrictive)

Alternatively, for more sophisticated computation:
- Compute point-by-point illuminance for a single luminaire
- Find the distance at which illuminance drops to 50% of directly below
- SC = 2 × distance / mounting_height

### Integration with Area Designer

Both tools share:
- The `ldt.sample(C, γ)` candela interpolation
- Point-by-point illuminance computation engine
- SVG rendering (heatmaps, contour lines, isometric view)
- **`scene3d` module** — shared axonometric 3D projection engine:
  SceneCamera, CameraPreset, project(), render_scene_svg(), z-sort.
  Area designer uses `build_exterior_scene()` (ground + poles),
  Zonal designer uses `build_interior_scene()` (room box + ceiling).
- LLF/proration factor model
- Unit system toggle

The zonal tool adds the room-cavity abstraction layer on top.
Both live in the same "Designer" tab with a toggle:
`[Interior (Zonal)] [Exterior (Area)]`

---

## Design Decisions

1. **CU computation on-the-fly vs pre-computed:** Compute the full CU table
   once when a luminaire is loaded, cache it. The zonal cavity calculation
   then just interpolates from the cached table. CU table generation involves
   numerical integration and inter-reflection solving — too slow for
   real-time recomputation on every room dimension change. Room parameter
   changes only trigger the cheap interpolation + layout calculation.

2. **Point-by-point accuracy:** The PPB overlay is direct illuminance only
   (no radiosity). To approximate total illuminance, add a uniform reflected
   component estimated from the CU-based calculation. This is pragmatic —
   full radiosity is out of scope for a quick-estimation tool and would
   require a fundamentally different computation engine.

3. **Zonal cavity limitations:** The method is for rectangular rooms with
   uniform layouts only. For non-rectangular rooms (Phase 6), subdivide into
   rectangular zones and calculate each separately. For non-uniform layouts,
   the point-by-point overlay is the primary result.

4. **Mounting height for zonal:** hrc = ceiling_height - workplane_height -
   suspension_length. This is the room cavity height, NOT the total
   mounting height from floor. The mounting height for spacing criterion
   is also hrc. This is a common source of confusion.

5. **UGR computation (Phase 6):** UGR requires observer position and viewing
   direction, plus luminance values of each luminaire as seen from that
   position. This needs the candela distribution sampled at specific angles —
   the same ldt.sample() infrastructure — plus luminaire area from the
   LDT/IES geometry. Non-trivial but well-defined by CIE 117.

6. **30° vs 10° zonal lumens:** The existing `ZonalLumens30` uses 6 zones
   at 30° intervals (0–30, 30–60, 60–90, 90–120, 120–150, 150–180).
   The IES CU computation uses these 6 zones for the direct ratio
   calculation, which is the standard approach. The `ZonalLumenSummary`
   type in the data model specifies 10° zones for the display/report
   (matching the IES photometric report format). These are two different
   things: 30° zones for CU computation (existing, reuse), 10° zones for
   the zonal lumen summary report display (compute separately if needed
   for the Phase 5 photometric report, not required for core calculation).

7. **URL hash serialization (Phase 4):** This is the Luxiflux-killer feature
   for manufacturer embedding. A shareable URL like
   `eulumdat.icu/#zonal/room=20x15x3/wp=0.8/target=500/ref=70-50-20`
   lets manufacturers link directly to a pre-configured calculation from
   their product pages. Combined with the LDT file already loaded, this
   replicates Luxiflux's "launch from product page" workflow — but open,
   free, and embeddable without annual license fees.

8. **3D room view: SVG axonometric, shared with Area designer.** The
   `scene3d` module is shared code — Area builds an exterior scene (ground +
   poles), Zonal builds an interior scene (room box + ceiling luminaires).
   Same projection math, same z-sort renderer, same camera presets.
   Interior-specific choices:
   - Back walls rendered semi-transparent (50–80% opacity) so the room
     interior is visible. Front walls can be hidden entirely in Section preset.
   - Wall fill brightness tracks reflectance values — users immediately see
     that dark walls (ρw=0.3) look different from bright walls (ρw=0.7),
     building intuition for how reflectance affects CU.
   - Cavity visualization toggle draws the three-cavity diagram directly
     onto the room walls — a teaching tool for presentations and the
     Wuhan lectures. No other web tool does this.
   - Section camera preset (azimuth=0°, elevation=0°) produces a side
     cutaway matching standard lighting textbook diagrams.
