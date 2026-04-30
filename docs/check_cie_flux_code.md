# Bug: CIE Flux Code formula in `eulumdat::PhotometricCalculations::cie_flux_codes` is wrong

## Symptom (revised)

The gldf-rs viewer shows the GLDF-stated CIE Flux Code next to the value
computed by `eulumdat::PhotometricCalculations::cie_flux_codes`. For the
LEDVANCE BIOLUX HCL DL DN150 S 20W TW ZB
(`gldf-rs/tmp/ldv_inspect/ldc/4058075724600_biolux_hcl_dl_dn150_s_20w_tw_zb_1.ldt`):

| Code | GLDF | eulumdat-rs | Spec answer |
|------|------|-------------|-------------|
| N1 | 95 | 100 | 95 (downward flux % in 0–41.4° cone) |
| N2 | 100 | 100 | 100 (downward flux % in 0–60° cone) |
| N3 | 100 | 93 | 100 (downward flux % in 0–75° cone) |
| N4 | 100 | 0 | 100 (DLOR — % of total flux that is downward) |
| N5 | 100 | 0 | 100 (LOR — luminaire flux ÷ lamp flux %) |

**Initial diagnosis was wrong.** The GLDF value `95 100 100 100 100` is
correct — it's the canonical signature of a narrow-beam recessed
downlight, which is exactly what the BIOLUX HCL DL is. Our calculation
is the side that's wrong.

## What the CIE Flux Code actually means

Per CIE 52-1982 (and the IES LM-31 / older IES LM-58 convention), the
five-number flux code is **not** "% in 0–90 / 0–60 / 0–40 / 90–180 /
90–120 zones". The actual definitions:

| Code | Definition | Domain |
|------|------------|--------|
| N1 | % of **downward** flux contained in cone 0–41.4° | downward only |
| N2 | % of **downward** flux contained in cone 0–60° | downward only |
| N3 | % of **downward** flux contained in cone 0–75° | downward only |
| N4 | % of **total luminaire** flux that flows downward (= DLOR) | total |
| N5 | LOR — luminaire flux ÷ lamp flux × 100 | total |

Two things matter here:

1. **N1, N2, N3 are cumulative fractions of the *downward* flux**, not
   fractions of total flux. So for any luminaire whose entire output is
   downward, N3 will be 100 by definition (unless flux exists between 75°
   and 90°, which is rare for downlights).
2. **The cone angles are 41.4°, 60°, 75°, not 40°, 60°, 90°.** The angles
   were chosen so each zone subtends π/2 steradians, dividing the lower
   hemisphere into four equal-solid-angle slices.
3. **N4 is DLOR, N5 is LOR.** They're not "upward" zones at all. A pure
   downlight has N4 = 100 and N5 = 100 when the LDT's stated LORL is 100%.

So `95 100 100 100 100` reads:
- 95% of downward flux is concentrated in the innermost 41.4° cone (focused beam)
- the remaining 5% is between 41.4° and 60°
- nothing between 60° and 75°
- 100% of total flux goes downward (pure direct)
- LOR = 100% (luminaire flux equals lamp flux)

Textbook narrow-beam recessed downlight signature.

## Why our `cie_flux_codes` gives wrong values

`crates/eulumdat/src/calculations.rs:740`:

```rust
pub fn cie_flux_codes(ldt: &Eulumdat) -> CieFluxCodes {
    let total = Self::total_output(ldt);
    if total <= 0.0 { return CieFluxCodes::default(); }

    let flux_40  = Self::downward_flux(ldt, 40.0);
    let flux_60  = Self::downward_flux(ldt, 60.0);
    let flux_90  = Self::downward_flux(ldt, 90.0);
    let flux_120 = Self::downward_flux(ldt, 120.0);
    let flux_180 = Self::downward_flux(ldt, 180.0);

    CieFluxCodes {
        n1: flux_90,            // 0-90° (DLOR)        ← WRONG, this is N4
        n2: flux_60,            // 0-60°              ← WRONG arc & basis
        n3: flux_40,            // 0-40°              ← WRONG arc & basis
        n4: flux_180 - flux_90, // 90-180° (ULOR)     ← WRONG, this is not N4
        n5: flux_120 - flux_90, // 90-120°            ← WRONG, this is not N5
    }
}
```

Errors:

1. **Cone angles are wrong.** N1 should be 41.4°, N2 60°, N3 75°.
2. **N1/N2/N3 should be % of downward flux**, not % of total flux. Our
   `downward_flux(arc)` returns % of total. To get the spec quantity we
   need `downward_flux(arc) / downward_flux(90)` × 100 (or compute the
   numerator and divide by the downward total).
3. **N4 should be DLOR**, i.e. `downward_flux(90)`. We're computing
   `flux_180 - flux_90` (the upward fraction) — the opposite quantity.
4. **N5 should be LOR**, available directly as `ldt.light_output_ratio`.
   We're computing flux in the 90–120° zone, which is a meaningful
   metric but not what the CIE flux code N5 means.

The `Display` impl rounds five numbers to integers and joins them, so the
output looks shaped right (`100 100 93 0 0`) but the values are five
different metrics from the ones the spec calls for.

## Why this slipped past tests

The `compare.rs` round-trip code (`a.cie_flux_codes.n1` vs `b.cie_flux_codes.n1`,
etc.) compares two `CieFluxCodes` instances against each other — both
produced by the same wrong formula — so the test confirms internal
consistency, not external correctness. Any test that compared output
against a known-correct reference for a standard luminaire (e.g. a
documented IES file with a published flux code) would have caught this.

## Fix

Replace the body with the correct formula. Sketch:

```rust
pub fn cie_flux_codes(ldt: &Eulumdat) -> CieFluxCodes {
    let total = Self::total_output(ldt);
    if total <= 0.0 { return CieFluxCodes::default(); }

    // % of total flux in each cone — these are the integration outputs.
    let p_41_4 = Self::downward_flux(ldt, 41.4); // 0–41.4° cone, % of total
    let p_60   = Self::downward_flux(ldt, 60.0);
    let p_75   = Self::downward_flux(ldt, 75.0);
    let p_90   = Self::downward_flux(ldt, 90.0); // = DLOR

    // N1, N2, N3 are cumulative fractions of *downward* flux.
    // Guard against division by zero on pure-uplight luminaires.
    let (n1, n2, n3) = if p_90 > 0.0 {
        (
            100.0 * p_41_4 / p_90,
            100.0 * p_60   / p_90,
            100.0 * p_75   / p_90,
        )
    } else {
        (0.0, 0.0, 0.0)
    };

    CieFluxCodes {
        n1,
        n2,
        n3,
        n4: p_90,                      // DLOR
        n5: ldt.light_output_ratio,    // LOR (already a percentage in EULUMDAT)
    }
}
```

Update doc comments on `CieFluxCodes` fields (`n1` is "0–41.4°", etc.;
`n4` is DLOR; `n5` is LOR). Add a regression test against a known
luminaire — the BIOLUX file with expected `95 100 100 100 100` is a
clean fixture for this.

## Display rounding

The `Display` impl uses `{:.0}` (round to integer). For the BIOLUX,
`p_41_4 / p_90 = 0.95...` × 100 = ~95, which rounds to 95 — matching the
GLDF. For luminaires that fall on `.5` boundaries, the rounding rule
matters: CIE 52 specifies round-half-to-even (banker's rounding); Rust's
`f64::round` rounds half-away-from-zero. Worth a one-line comment
acknowledging this; in practice the boundary cases are vanishingly rare.

## Implications for `gldf-rs`

The "GLDF vs Calc" dual display in `crates/gldf-rs-wasm/src/components/photometry_editor.rs`
will become useful again **once the eulumdat fix lands**. Until then,
the calc column is misleading — it shows real numbers that are not the
CIE flux code.

Two interim options for `gldf-rs`:

1. **Hide the calc column for now**, just show the GLDF value.
2. **Label the calc column as "(needs verification)"** with a tooltip
   pointing at this doc, so users know not to trust the calc.

I'd suggest option 1 — a wrong number with a "Calc" label is more
misleading than no calc at all.

## Apology / lesson

The earlier draft of this doc concluded that the GLDF value was wrong and
ours was right. That was wrong: I had misread the CIE flux code spec, and
the eulumdat docstrings reinforced the misreading. Two takeaways:

- Don't trust internal docstrings as the source of truth for an external
  spec — verify against CIE/IES documents.
- A unit test against a published reference luminaire (with the answer
  on the back of the page) would have caught this in five minutes.

## Files

- `crates/eulumdat/src/calculations.rs` — function to fix at line 740, type
  doc comments at line 2424.
- `crates/eulumdat/src/compare.rs:607–648` and `:893–940` — round-trip
  comparator; will keep working but will now compare correct values.
- Reference LDT: `gldf-rs/tmp/ldv_inspect/ldc/4058075724600_biolux_hcl_dl_dn150_s_20w_tw_zb_1.ldt`
  with expected output `95 100 100 100 100`.
