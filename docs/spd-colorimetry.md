# SPD loading + CIE colorimetry

Two modules let you take an arbitrary spectral power distribution — measured by
a spectrometer, digitised from a datasheet, or shipped by a vendor — and
compute the full CIE colorimetric description of the light it represents.

## Loader: `eulumdat::load_spd` / `parse_spd`

`crates/eulumdat/src/atla/spd_loader.rs` auto-detects four real-world formats
found in `docs/SPDs/`:

| Format | Extension | Example vendor | Shape |
|---|---|---|---|
| Luxeon datasheet `.spd` | `.spd` | `Luxeon_70_3000K.spd` | `# comment` header, tab-separated `wavelength_nm relative_power`, uniform 5 nm |
| Generic named CSV | `.csv` | Yuji "WB Day/Nite", luxeon\_95CRI | header `wavelength_nm,intensity`, irregular ~1 nm grid (may contain duplicates) |
| Signify lab CSV | `.csv` | `Signify bread.csv` | numbered metric prelude (CIE/CRI/Duv/PPFD/EML/…), then `wavelength` body |
| Generic two-column fallback | any | — | whitespace/comma split |

```rust
use eulumdat::{load_spd, analyze_spd};

let loaded = load_spd("docs/SPDs/Signify/Signify bread .csv")?;
let cie    = analyze_spd(&loaded.spd);
println!("{} K (Duv {:+.4})", cie.cct_k.round(), cie.duv);

// Lab files (Signify) also surface their pre-computed metrics:
if let Some(r) = &loaded.reference {
    assert!((cie.cct_k - r.cct_k.unwrap()).abs() < 50.0);
}
```

Irregular grids with duplicate wavelengths (a Yuji quirk) are collapsed —
duplicates averaged — but spacing is not resampled; the colorimetry uses
trapezoidal integration that handles non-uniform grids correctly.

## Colorimetry: `eulumdat::analyze_spd`

`crates/eulumdat/src/atla/colorimetry.rs` is self-contained CIE math
(`Colorimetry`):

- **Tristimulus** X, Y, Z (CIE 1931 2° CMF, 5 nm, normalised Y = 100)
- **CIE 1931** chromaticity x, y
- **CIE 1960** UCS u, v
- **CIE 1976** UCS u′, v′
- **CCT** via Robertson's 31-isotemperature-line method (fallback to McCamy
  outside 1666–25000 K)
- **Duv** as the signed perpendicular distance from the Planckian locus in
  CIE 1960 uv (positive = greenish, negative = magenta)
- **Dominant wavelength** + **colour purity %** via ray–locus intersection
  from the equal-energy white point
- **Peak wavelength** + **half-peak width** on the SPD's own grid

### Why a new module (not `tm30::xyz_to_cct`)

The existing `tm30.rs` ships private helpers using McCamy's polynomial
(±3–10 K) and a coarse Duv approximation, sufficient for its TM-30 internals
but not for matching a lab spectrometer. The new module uses Robertson
+ true 1960 uv Duv and gets the Signify corpus to:

| Quantity | Signify ref (6 files) | Ours | Δ |
|---|---|---|---|
| x | 0.4004 – 0.4527 | … | within ±0.0001 |
| y | 0.3856 – 0.4066 | … | within ±0.0001 |
| u′ | 0.2346 – 0.2670 | … | within ±0.0001 |
| v′ | 0.5084 – 0.5226 | … | within ±0.0001 |
| CCT (K) | 2637 – 3580 | … | within ±15 |
| Duv | −0.0079 – +0.0010 | … | within ±0.0003 |

That's the regression target enforced by
`crates/eulumdat/src/atla/colorimetry.rs::matches_signify_reference`.

## Live demo

```
cargo run -q -p eulumdat --example spd_colorimetry_corpus
```

walks every SPD in `docs/SPDs/`, prints `ours` vs the Signify `ref` line by
line, and lets you eyeball the agreement.
