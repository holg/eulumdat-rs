# Area Lighting Designer — Design Plan

## Overview

A self-contained SVG-based area lighting design tool in the "Designer" tab.
Users place 1–6+ luminaires on a plan view, configure heights and spacing,
and see combined illuminance results (isometric ISO view + statistics).

Includes a spacing optimizer that sweeps mounting heights and pole spacings
to find the most cost-effective configuration meeting a target illuminance
criterion — the core value proposition for manufacturers and specifiers.

No external dependencies (no Google Maps). Pure Rust/SVG computation.

---

## Data Model

### Core Types (in `eulumdat` crate)

```rust
/// A single luminaire placement on the ground plane.
struct LuminairePlace {
    id: usize,
    x: f64,              // ground X position in meters
    y: f64,              // ground Y position in meters
    mounting_height: f64, // pole height in meters
    tilt_angle: f64,     // 0° = straight down, 90° = horizontal
    rotation: f64,       // C0 direction in degrees (0° = front/+Y)
    arm_length: f64,     // lateral offset from pole center in meters
    arm_direction: f64,  // direction of arm overhang in degrees
    ldt_index: usize,    // index into loaded LDT files (Phase 5: mixed fixtures)
}

/// Physical pole configuration — generates LuminairePlaces from pole positions.
enum ArrangementType {
    Single,          // one luminaire per pole
    BackToBack,      // two luminaires, opposite directions (180°)
    TwinArm,         // two luminaires, same side, slight offset
    Quad,            // four luminaires at 90° each
    WallMounted,     // mounted on vertical surface, not pole
}

struct PoleConfig {
    arrangement: ArrangementType,
    arm_length: f64,    // overhang from pole center, meters
    arm_droop: f64,     // arm droop angle from horizontal, degrees (0° = level, +° = drooping)
                        // Droop tilts the luminaire forward geometrically.
                        // LuminairePlace.tilt_angle is an *additional* adjustment on top.
                        // Effective tilt = arm_droop + tilt_angle.
}

/// A single repeating bay between poles.
/// The optimizer works on one bay but includes contributions from
/// neighboring poles for edge accuracy.
struct CalculationBay {
    width: f64,             // pole spacing X
    depth: f64,             // pole spacing Y
    neighbor_depth: usize,  // how many bay repetitions to include
                            // Auto-detected from beam angle if not set:
                            //   narrow beam (< 60°): 1 bay
                            //   medium beam (60°–120°): 2 bays
                            //   wide beam (> 120°): 3 bays
                            // Can be overridden manually.
}

/// Result of multi-luminaire illuminance calculation.
struct AreaResult {
    lux_grid: Vec<Vec<f64>>,  // combined illuminance at each ground point
    min_lux: f64,
    avg_lux: f64,
    max_lux: f64,
    uniformity_min_avg: f64,  // min / avg  (U₀)
    uniformity_avg_min: f64,  // avg / min
    uniformity_min_max: f64,  // min / max  (Ud)
    area_width: f64,
    area_depth: f64,
}
```

### Optimization Types

```rust
/// Optimization target criteria.
struct OptimizationCriteria {
    target_min_lux: f64,            // e.g. "at least 20 lx everywhere"
    target_uniformity: Option<f64>, // optional: min/avg ≥ 0.25
    height_range: (f64, f64),       // e.g. 8m–14m, step 1m
    height_step: f64,               // e.g. 1.0m or 2.0m
    spacing_range: (f64, f64),      // search bounds for pole spacing
    arrangement: ArrangementType,
}

/// One row in the optimization result matrix.
struct OptimizationResult {
    mounting_height: f64,
    optimal_spacing_x: f64,
    optimal_spacing_y: f64,
    min_lux: f64,
    avg_lux: f64,
    max_lux: f64,
    uniformity_min_avg: f64,
    meets_criteria: bool,
    poles_needed: usize,   // total poles for the user's area at this spacing
}
```

### Computation

```rust
fn compute_area_illuminance(
    ldt: &Eulumdat,
    placements: &[LuminairePlace],
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,       // LLF / maintenance factor (0.0–1.0)
) -> AreaResult
```

For each grid point `(gx, gy)`:
1. Loop over all luminaire placements
2. For each placement, compute vector from luminaire to ground point
3. Convert vector to polar: azimuth `φ` and distance `d`
4. Map to C-plane: `C_sample = φ - rotation` (mod 360°).
   The `rotation` field defines where the luminaire's C0 direction points
   in world space (0° = +Y). Subtracting it transforms the world-space
   azimuth back into the luminaire's local C-plane coordinate system.
5. Compute gamma from geometry: `γ = atan(horizontal_dist / mounting_height)`
   Then apply effective tilt: `γ_eff = γ - (arm_droop + tilt_angle)`.
   Tilt shifts the gamma lookup — a forward-tilted luminaire's peak
   intensity moves to higher gamma angles in world space.
6. Sample intensity: `I = ldt.sample(C_sample, γ_eff)`
7. Compute illuminance: `E = I × cos(θ) / d² × (flux / 1000) × proration_factor`
   where `θ` is the angle of incidence on the horizontal plane and
   `d` is the 3D distance from luminaire to ground point.
8. Sum contributions from all luminaires

This reuses the existing isolux physics — just with multiple source positions.

### Optimization Algorithm

```rust
fn optimize_spacing(
    ldt: &Eulumdat,
    criteria: &OptimizationCriteria,
    pole_config: &PoleConfig,
    area_width: f64,
    area_depth: f64,
    grid_resolution: usize,
    proration_factor: f64,
) -> Vec<OptimizationResult>
```

For each mounting height in `height_range` (stepped by `height_step`):
1. Generate pole positions for the arrangement type
2. Search for the **widest** spacing that still meets `target_min_lux`
   (and optionally `target_uniformity`):

   **Search strategy:** Golden-section search on spacing is preferred over
   binary search. Binary search assumes a monotonic relationship (wider
   spacing → lower min lux), which holds for symmetric fixtures but can
   break for heavily asymmetric luminaires (e.g. road optics with strong
   C0/C180 throw). In those cases the min-lux point can shift between
   grid positions as spacing changes, creating local non-monotonicity.
   Golden-section is more robust to this.

   Fallback: if golden-section converges to a suspicious result, run a
   coarse sweep (e.g. 10 evenly spaced candidates) first to bracket the
   region, then refine with golden-section within the best bracket.

3. Compute within a single bay but include contributions from neighboring
   poles (auto-detected depth based on beam angle, see `CalculationBay`)
4. Record the optimal spacing + statistics for that height
5. Compute `poles_needed = ceil(area_width / spacing_x) × ceil(area_depth / spacing_y)`

Result: a matrix of height → optimal spacing → statistics → pole count,
letting users pick the most cost-effective configuration.

---

## Layout Presets

### Pole Arrangement Types

| Arrangement   | Luminaires/Pole | Description                                      |
|--------------|-----------------|--------------------------------------------------|
| Single        | 1               | One luminaire per pole                            |
| Back-to-Back  | 2               | Two luminaires, opposite directions (180°)        |
| Twin Arm      | 2               | Two luminaires, same side, slight lateral offset  |
| Quad          | 4               | Four luminaires at 90° each                       |
| Wall Mounted  | 1               | Mounted on vertical surface, no pole              |

For Back-to-Back, auto-generate two `LuminairePlace` entries per pole with
180° rotation offset. For Quad, four entries at 90° increments. Arm length
shifts luminaire position laterally from pole center.

### Grid Presets (Pole Count)

| Preset       | Poles | Pattern             |
|-------------|-------|---------------------|
| Single       | 1     | `●`                 |
| Pair         | 2     | `● ●`               |
| Row of 3     | 3     | `● ● ●`             |
| 2×2 Grid     | 4     | `●● / ●●`           |
| 2×3 Grid     | 6     | `●●● / ●●●`         |
| 3×3 Grid     | 9     | `●●● / ●●● / ●●●`   |
| Custom       | N     | Free placement       |

Note: Pole count × luminaires-per-pole = total luminaire placements.
A 2×3 grid of Back-to-Back poles = 12 luminaire calculations.

### Quick Position Presets

Shift selected pole(s) to predefined positions within the area:

- **Vertical:** Top / Middle / Bottom (sets Y to 80% / 50% / 20% of area depth)
- **Horizontal:** Left / Center / Right (sets X to 20% / 50% / 80% of area width)
- **Perimeter:** Distribute selected poles evenly around the area edges

### Per-Luminaire Overrides

Each luminaire can have individual:
- Position (x, y) — draggable on plan view or typed in
- Mounting height (may differ from global default)
- Tilt angle
- Rotation (C0 orientation)

---

## UI Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ Toolbar                                                          │
│  Layout: [Grid ▼]  Poles: [6]  Rows: [2] Cols: [3]             │
│  Arrangement: [Back-to-Back ▼]  Arm Length: [1.5m ──●──]       │
│  Arm Droop: [5° ──●──]                                        │
│  Spacing X: [30m ──●──]   Spacing Y: [20m ──●──]               │
│  Height: [10m ──●──]   Proration: [1.0 ──●──]                  │
├───────────┬─────────────────────────────────────────────────────┤
│ Left      │  Plan View (top-down SVG)                            │
│ Panel     │                                                      │
│           │  ┌──────────────────────────────┐                    │
│ Presets:  │  │                              │                    │
│ [▲ Top]   │  │    ●─────────●─────────●     │  ← draggable      │
│ [━ Mid]   │  │    │         │         │     │    poles           │
│ [▼ Bot]   │  │    ●─────────●─────────●     │                    │
│           │  │                              │                    │
│ [◄][●][►] │  └──────────────────────────────┘                    │
│           │  Area: 60m × 40m                                     │
│ Selected: │                                                      │
│ Pole #3   │─────────────────────────────────────────────────────│
│  x: 15m  │  ISO View (combined illuminance, isometric or flat)   │
│  y: 10m  │                                                      │
│  h: 10m  │  ┌──────────────────────────────┐                    │
│  tilt: 0° │  │  Combined contours from all  │                    │
│  rot: 0°  │  │  luminaires. Grayscale or    │                    │
│  arm: 1.5m│  │  AEC colored bands.          │                    │
│           │  │                              │                    │
│───────────│  └──────────────────────────────┘                    │
│ Optimizer │                                                      │
│           │  Summary: Min: 5 lx  Avg: 22 lx  Max: 85 lx        │
│ Target:   │           Uniformity (min/avg): 0.23                 │
│ [20 lx]   │           Uniformity (avg/min): 4.4                  │
│ U₀ ≥:     │                                                      │
│ [0.25]    │                                                      │
│ [▶ Optimize]                                                     │
│           │                                                      │
│ Results:  │                                                      │
│ Ht   Spc  Min  Avg  U₀  Poles                                   │
│ 8m   22m  21   48  .44  12                                       │
│ 10m  28m  20   35  .57   8 ✓                                     │
│ 12m  32m  20   29  .69   6 ✓                                     │
│           │                                                      │
│───────────│                                                      │
│ Height    │                                                      │
│ Compare   │                                                      │
│           │                                                      │
│ ☑ 10m    │                                                      │
│  Min:  5  │                                                      │
│  Avg: 22  │                                                      │
│  Max: 85  │                                                      │
│  U₀: 0.23│                                                      │
│           │                                                      │
│ ☐ 8m     │                                                      │
│  Min:  8  │                                                      │
│  Avg: 35  │                                                      │
│  Max: 120 │                                                      │
│  U₀: 0.23│                                                      │
│           │                                                      │
│ ☐ 12m    │                                                      │
│  ...      │                                                      │
└───────────┴─────────────────────────────────────────────────────┘
```

---

## Features

### Plan View (top-down)
- SVG rendered, responsive
- Pole positions shown as dots/icons with ID labels
- Luminaire arm directions visible per arrangement type
- Area outline (rectangle) with dimension labels
- Spacing grid lines (dashed)
- **Click** a pole to select it → shows properties in left panel
- **Drag** a pole to reposition (updates x, y in real-time)
- **Click empty area** to deselect

### ISO / Illuminance View
- Renders combined illuminance from all luminaire contributions
- Can toggle between:
  - Isometric 3D view (grayscale, like current ISO View)
  - Flat top-down view (AEC colored bands)
- Contour lines with lux labels
- All pole/luminaire positions marked

### Spacing Optimizer
- User sets target minimum illuminance (lux) and optionally target uniformity
- Selects arrangement type and arm length
- Enters mounting height range (min, max, step)
- Clicks "Optimize" → runs sweep across heights
- For each height, golden-section search finds widest spacing meeting criteria
- Results matrix shows: height, optimal spacing, min/avg/max lux, uniformity, pole count
- Rows marked ✓ that meet all criteria
- Clicking a result row applies that configuration to the plan view
- Pole count = total poles needed for the user's area at that spacing

### Height Compare Cards
- Left panel shows cards for different mounting heights
- Each card shows min/avg/max and uniformity for that height
- Check a card to overlay its contours on the ISO view
- Useful for comparing "10m poles vs 12m poles" at a glance
- Heights can be preset (e.g. 8m, 10m, 12m) or user-defined
- Optimizer results auto-populate these cards

### Calculation Bay Mode
- Instead of computing the entire area, compute one repeating bay
- Bay = area between adjacent poles (spacing_x × spacing_y)
- Include contributions from neighboring poles for edge accuracy
- Neighbor depth auto-detected from luminaire beam angle:
  narrow beam (< 60°) → 1 bay, medium (60°–120°) → 2, wide (> 120°) → 3
- Can be overridden manually for edge cases
- Bay mode is used by the optimizer for speed
- Full-area mode used for manual/custom placement display
- Toggle between "Bay view" and "Full area view"

### Proration Factor
- Multiplier applied to total flux before calculation
- Accounts for:
  - Lamp Lumen Depreciation (LLD)
  - Luminaire Dirt Depreciation (LDD)
  - Lamp Burnout Factor (LBF)
  - Temperature factor
- Default: 1.0 (no depreciation)
- Typical range: 0.5–1.0
- Can be typed or slider-adjusted

### Export
- Export illuminance grid as CSV
- Export plan view as SVG
- Export ISO view as SVG
- Export combined report as PDF (via Typst)
- Export optimizer results matrix as CSV

---

## File Structure

```
crates/
  eulumdat/src/
    area/
      mod.rs              — pub mod, AreaResult, LuminairePlace, PoleConfig, etc.
      compute.rs          — compute_area_illuminance()
      optimize.rs         — optimize_spacing(), OptimizationCriteria/Result
      layout.rs           — grid/row/perimeter preset generators, ArrangementType
      bay.rs              — CalculationBay, neighbor contribution logic
      svg.rs              — plan view SVG, combined ISO view SVG

  eulumdat-wasm/src/components/
    area_designer.rs      — main Leptos component
    area_plan_view.rs     — interactive plan view (drag, select)
    area_iso_view.rs      — combined illuminance visualization
    area_controls.rs      — toolbar, sliders, left panel
    area_optimizer.rs     — optimizer panel + results matrix
```

---

## Phases

### Phase 1 — Core Calculation + Static UI ✅
- [x] `LuminairePlace`, `AreaResult`, `PoleConfig` types
- [x] `ArrangementType` enum with luminaire generation from pole positions
- [x] `compute_area_illuminance()` with superposition
- [x] Grid layout presets (1, 2, 3, 4, 6, 9 poles)
- [x] Static plan view SVG
- [x] Combined ISO view with contours
- [x] Statistics panel (min/avg/max/uniformity)
- [x] Wire into Designer tab

### Phase 2 — Interactive Plan View + Arrangements ✅
- [x] Draggable pole positions (mousedown/mousemove/mouseup + screen-to-world transform)
- [x] Click to select, show properties in left panel
- [x] Per-luminaire height/tilt/rotation overrides
- [x] Quick position presets (9-button grid: corners, edges, center)
- [x] Pole arrangement types in toolbar (Single/BackToBack/TwinArm/Quad/WallMounted)
- [x] Arm length, droop controls
- [x] Smart defaults from LDT photometric data (flux, beam spread, DFF)
- [x] Imperial unit support throughout (m/lx ↔ ft/fc)
- [x] URL hash-based state sharing (shareable links)

### Phase 3 — Spacing Optimizer + Height Comparison ✅
- [x] `OptimizationCriteria` and `OptimizationRow` types
- [x] `optimize_spacing()` with height sweep + golden-section search on spacing
- [x] Bay-based computation with 3×3 neighbor contributions for optimizer speed
- [x] Optimizer panel UI (target inputs, results matrix)
- [x] Click result row → apply configuration to plan view
- [x] Poles-needed column (cost-effectiveness metric)
- [x] Multi-height comparison cards in left panel (clickable, selected state)
- [x] Side-by-side statistics per height (min/avg/max/U₀/spacing/poles)
- [x] Overlay toggle for contours at different heights (checkbox per card, colored dashed contours)

### Phase 4 — Proration & Export ✅
- [x] Proration factor input (slider 0.3–1.0)
- [x] CSV export of illuminance grid (with unit-aware headers)
- [x] CSV export of optimizer results matrix
- [x] SVG export of plan + ISO views
- [x] Typst report export (.typ with embedded SVGs, stats, optimizer table)

### Phase 5 — Advanced ✅
- [x] Bay view vs full-area view toggle (checkbox, computes single bay with 3×3 neighbors)
- [x] Custom area shapes (polygon support — draw polygon, point-in-polygon masking, polygon SVG rendering)
- [x] Perimeter placement preset (distributes poles evenly around rectangle edges)
- [x] Mixed luminaire types (load extra LDT/IES files, per-pole LDT selector, `compute_area_illuminance_mixed`)
- [x] Undo/redo for placement changes (history stack, Undo/Redo buttons in toolbar)
- [x] Save/load design configurations (localStorage-based, Save/Load buttons in toolbar)
- [x] Wall-mounted luminaire support (auto 90° tilt, no arm offset, info banner)

---

## Design Decisions

1. **Area shape:** Rectangles only through Phase 4. Polygons in Phase 5.
   95% of parking lots and area lighting projects are rectangular bays.
   The bay-based optimizer approach makes rectangles the natural unit.

2. **Max luminaires:** No artificial cap. Users think in "poles" not
   "luminaires." A 3×3 grid of Quad poles = 36 luminaire calculations,
   which is fine. Grid presets define poles; arrangement type multiplies
   luminaires per pole.

3. **Mixed fixtures:** Same LDT for all luminaires in Phases 1–4.
   Different LDT per pole in Phase 5. Most area lighting uses identical
   fixtures — mixed types are rare and mostly for architectural projects.

4. **Units:** Follow the global unit system toggle (SI/Imperial).
   No local override — area lighting is deeply regional (footcandles +
   feet for US, lux + meters for Europe) and the global toggle handles it.

5. **Calculation grid density:** Adaptive.
   - 20×20 per bay for optimizer sweep (fast convergence)
   - 40×40 for interactive drag feedback (responsive)
   - 80×80 for final "Calculate" render (publication quality)

6. **Real-time vs compute button:** Hybrid.
   - Manual placement mode: recompute on every change at 20×20 for instant
     feedback, then auto-refine to 80×80 after 300ms debounce
   - Optimizer sweep: explicit "Optimize" button — runs dozens of
     calculations across the height/spacing matrix
