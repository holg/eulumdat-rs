# Patch-radiosity coefficient of utilization

`eulumdat::radiosity` is a physically-based indoor illuminance / coefficient-of-
utilization engine. It computes the inter-reflected light field the way DIALux
Classic does — subdivide the room surfaces into Lambertian patches, compute form
factors between them, and solve the radiosity system

```
B_i = E_i + ρ_i · Σ_j B_j · F_ij
```

to convergence (Gauss–Seidel). It is deterministic, CPU-only, std-only, and adds
**no dependencies** to the crate.

It exists because the empirical zonal-cavity model (`CuTable::calculate_cu_ies`,
`src/calculations.rs`) runs ~25–27 % low in high-cavity-ratio and non-square
rooms: `calculate_direct_ratio_ies` assumes a square room, and its inter-
reflection decay constants were fitted near RCR≈0 and extrapolate poorly. The
radiosity engine uses the real L×W×H aspect ratio and solves the actual transfer
system, so it does not have those biases.

## Status: additive, opt-in

The radiosity module was **added alongside** the existing zonal/`CuTable` path,
which is **unchanged**. `compute_zonal` and `interpolate_cu` still use the
empirical `CuTable`. Nothing in the shipping calc switches automatically — code
must call `eulumdat::radiosity::*` explicitly to use it.

Migration of the indoor calc path (`compute_zonal` / the rapidcalc UI) to drive
radiosity with real room geometry is a **separate, not-yet-done step**. See
"Integration strategies" below.

## Public API

Re-exported at the crate root:

| Symbol | Purpose |
|---|---|
| `RoomMesh::new(l, w, h, refl, divisions)` | subdivide a box room into patches |
| `SurfaceReflectances { ceiling, wall, floor }` | diffuse reflectances 0..1 |
| `Luminaire { pos: Vec3, flux }` | a downlight at a position, total lumens |
| `compute_form_factors(&mesh) -> FormFactors` | analytic point-to-patch FFs, reciprocity-enforced |
| `direct_illuminance(&mesh, &lums, &ldt)` | direct term from LDT photometry (via `SymmetryHandler`) |
| `solve_radiosity(&mesh, &ff, &direct, max_iter, tol)` | Gauss–Seidel interreflection solve |
| `radiosity_cu(l, w, h, refl, &lums, &ldt, divisions) -> (cu, e_avg_floor, RadiosityResult)` | end-to-end CU |
| `illuminance_at_point(p, &mesh, &result, &lums, &ldt)` | E at any point (for a floating work plane) |
| `workplane_stats_at_height(&mesh, &result, &lums, &ldt, plane, standard, grid)` | Ē / U₀ on a free `grid × grid` mesh (ad-hoc) |
| `workplane_stats_normative(&mesh, &result, &lums, &ldt, plane, standard)` | Ē / U₀ on the **binding** Stockmar/CIE grid (no free `grid` arg) |
| `WorkPlane` | evaluation height: `Office` 0.75 m, `GeneralDefault` 0.85 m, `Corridor` 0.2 m, `Warehouse` 0.0 m, `Custom(h)` |
| `EvaluationStandard` | `En12464_2021` (default), `En12464Pre2021`, `IesRp1`, `None` |
| `WorkplaneStats` | `e_avg`, `e_min`, `e_max`, `u0` (min/avg), `avg_min_ratio`, `max_min_ratio`, `border`, `patches` |

Photometry is sampled through `SymmetryHandler::get_intensity_at(&Eulumdat,
c_deg, g_deg)` (candela per 1000 lm), scaled by `luminaire_flux / 1000`.

## Two standards, layered

The engine separates **physics** from **evaluation**:

- **Physics** (the radiosity solve) is validated against **CIE 171:2006**
  (`tests/radiosity_cie171.rs`) — see [cie-171-2006-tests.md](cie-171-2006-tests.md).
- **Evaluation** (`EvaluationStandard`) is the regional reporting layer:
  - `En12464_2021` — excludes a wall border (Randzone) `min(0.5 m, 0.15 × shortest
    side)` from the task area; reports `U₀ = E_min/E_avg`. The European default.
  - `En12464Pre2021` — fixed 0.5 m border.
  - `IesRp1` — ANSI/IES RP-1 (US office), full work plane, ratio-based
    uniformity (`avg_min_ratio`, `max_min_ratio`).
  - `None` — whole work plane (matches DIALux "ohne Randzonen").

`EvaluationStandard::for_unit_system(metric)` pairs with `UnitSystem`: Metric ⇒
EN 12464-1, Imperial ⇒ IES RP-1.

## The evaluation grid (Raster) is binding, not free

A subtle but decisive point for cross-program agreement: the standard does **not**
let you read Ē / E_min / E_max / U₀ at the *true numerical extremum* of the
continuous field. It mandates a specific **measurement/calculation grid** and you
report on **those points only**. Searching a finer mesh for the actual peak is
exactly what historically made DIAL/DIALux report higher maxima (and worse U₀)
than the norm intends — "in bester Absicht", but non-conformant. The grid is the
verbindliche Vorgabe; the rechnerische Minima/Maxima are not.

So the grid follows the standard the same way the border and the uniformity figure
already do — and therefore follows the unit system via `for_unit_system`:

| Unit system | Standard | Grid rule (`EvaluationStandard::grid_points`) |
|---|---|---|
| Metric | EN 12464-1 | **Stockmar**: max cell `p = 0.2·5^(log₁₀ d)`, `d` = longer task-area side, **capped at 1.0 m**; points/axis = `ceil(side / p)`, same `p` both axes |
| Imperial | IES RP-1 / CIE | fixed **max spacing ≈ 0.6 m (2 ft)**; points/axis = `ceil(side / spacing)` |
| — | None | `(0, 0)` — no normative grid; caller picks its own resolution |

`grid_points` grids the **task area** (room minus the wall border), so the same
room + same standard ⇒ identical evaluation points in any conforming program.
Use `workplane_stats_normative` (no `grid` argument) for conformant numbers;
`workplane_stats_at_height(…, grid)` remains for ad-hoc exploration where the
free-mesh max deliberately *does* drift with the grid argument.

## Maintenance factor

The radiosity solve computes **initial** illuminance from the luminaire flux you
pass in. To get a **maintained** value, apply the maintenance factor — either by
passing maintained flux, or by multiplying afterwards. DIALux's default
Wartungsfaktor is **0.80**; DIALux's reported "Average" is the *maintained*
value. (The `eulumdat::zonal` LED `LlfPreset` totals 0.838 — that is a
four-component LLF, **not** the DIALux-parity value; use 0.80 to match DIALux.)

## Validation

**CIE 171:2006 (physics).** `tests/radiosity_cie171.rs` checks the converged
surface average against the exact integrating-cavity (Sumpner) relation
`E_avg = (Φ/A)/(1−ρ)` and the indirect/direct ratio `ρ/(1−ρ)` — within 2 %.
Plus inline unit tests: form-factor reciprocity & row-sum, the analytic
parallel-plate form factor (Howell C-11), energy conservation, reflectance
monotonicity, EN 12464-1 border widths, and work-plane height behaviour.

**DIALux (end-to-end).** Loading the *same* LDT into DIALux and into this engine,
at MF 0.80 and a 0.80 m work plane, the maintained Ē matches to ~1 % and U₀ to
~0.02:

| Case (same LDT) | ours | DIALux | ratio | U₀ ours/DIALux |
|---|---|---|---|---|
| Acrux Büro 4×3, 22 lum | ~513 lx | 508 lx | 1.01 | 0.80 / 0.81 |
| Altais Büro 4×3, 38 lum | ~524 lx | 519 lx | 1.01 | 0.82 / 0.84 |

The remaining ~1 % is the point-source direct term very slightly sharpening the
field versus DIALux's luminaire model; optionally tightened later with an area-
source emitter. (These figures use `workplane_stats_normative` so the grid is the
Stockmar grid for both sides — the grid is no longer a free variable in the
comparison; any residual gap is physics, not Rasterwahl.)

**DIAL discrepancy table (regression test).** `tests/dial_parity.rs` reproduces
the original iesna.eu-vs-DIAL comparison (`tests/fixtures/dial_discrepancy.csv`,
with `alya_359lm.ldt` / `acrux_452lm.ldt`). The OLD iesna.eu deploy ran Ē
*systematically high* (65–96 % of DIAL on the same layout) by averaging a free
grid; the radiosity engine on the Stockmar grid lands on the **DIAL "Prüfung"**
column:

| Type | ours / DIAL Ē | U₀ |
|---|---|---|
| Büro (office, 0.75 m plane) | 99–102 % | within ±0.03 |
| Flur (corridor, **floor 0.0 m** plane) | 100–114 % | within ~0.1 |

Two corridor lessons baked into the engine and test:
- **`WorkPlane::Corridor` is the floor (0.0 m)**, per EN 12464-1 for traffic
  routes — a raised plane (the old 0.2 m) sits closer to the luminaires and
  reads several percent high. (Raising the plane only *increases* Ē, so it cannot
  explain DIAL reading lower; floor is both correct and our lowest reading.)
- Dim, interreflection-heavy corridors keep a +7…+13 % overshoot vs **current**
  DIALux. This was decomposed (`examples/divergence_probe`, since removed) and is
  **not** a solver artifact: resolution is converged (div 8→32 stable), the plane
  is already at the floor, and treating each luminaire as an area source instead
  of a point changes nothing. The gap is a *fraction of the indirect term* and
  grows with room height (Alya h=2.5 → +7 %, Acrux h=5 → +13 %), which is the
  signature of a **method difference**: we are ideal diffuse radiosity (all
  interreflected light conserved and Lambertian-re-emitted), whereas modern
  DIALux uses a Monte-Carlo raytracer ("photon" mode, much slower) that loses a
  little more indirect light per bounce. Our results match DIAL's *radiosity-era*
  numbers — which is what the old iesna.eu engine was historically tuned to. The
  test bounds corridors at 18 % and asserts every row is *at least as close to
  DIAL as the old engine was*.

The `22×22` row's `4×163` table entry is DIAL grid/point notation, not a physical
4-column layout, so its U₀ is excluded from the strict assertion (Ē still
matches). Re-run the whole thing live with
`cargo run -p eulumdat --example radiosity_grid_demo`.

## Integration strategies (for replacing the empirical CU)

When the shipping indoor calc is to use radiosity instead of `CuTable`:

1. **Drive radiosity from `compute_zonal`** with the actual room + computed
   luminaire layout (physically correct; makes `TargetToCount` mode iterative —
   solve CU at a nominal layout, then refine). Recommended for the real calc.
2. **Fill `CuTable` cells from radiosity** at a canonical room reproducing each
   RCR (drop-in for existing callers, but reintroduces the square-room
   abstraction).
3. **Expose radiosity as a parallel public API** and migrate callers
   individually (current state — zero regression).

Keep `calculate_cu_ies` available during any transition for A/B comparison.

## Provenance

Prototyped and validated in the separate `light-other-rs/crates/light-calc`
crate (DIALux parity harness, the 9-case comparison, the `radiosity_compare`
example), then landed here as the single source of truth. The port rationale and
the strategy trade-offs are written up in
`light-other-rs/crates/light-calc/PORT_TO_EULUMDAT.md`.
