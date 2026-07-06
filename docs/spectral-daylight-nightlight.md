# Spectral Monte Carlo, Daylight & Nightlight — Deep Plan

> **STATUS (implemented on the `spectrography` branch).** The full S/D/N stack
> below is built, tested, and green:
> - **`eulumdat-spectrum`** — new zero-dep leaf crate: `Spd` + importance
>   sampler, CIE 1931/scotopic/melanopic/PAR LUTs, Robertson colorimetry
>   (`analyze`, `from_chromaticity`), CIE D-series reconstruction, CIE 191
>   mesopic (`mesopic_luminance`), Sellmeier dispersion, CCT synthesis. 30 tests.
> - **S1** goniosim: `SourceSpectrum` (per-source, `None`=555 nm bit-compatible),
>   `DetectorMode::WeightedChannels` (7-channel), radiometric transport, colour /
>   scotopic / melanopic / S-P over angle. Round-trip CCT ±40 K through the tracer.
> - **S2** spectral materials: `SpectralOverride` (Sellmeier IOR + τ(λ) curve) on
>   the transmitter path; amber-filter CCT shift and dispersion validated.
> - **D1/D2** daylight: `Source::SunDisc`, `Source::SkyDome` (2-D CDF from
>   `SkyRadiance`), `PlaneDetector`, `trace_illuminance`, `daylight_factor_grid`.
>   Open-field plane recovers DHI (±6 %), sun beam recovers DNI·sinα (±5 %).
> - **N1/N2** nightlight: `Source::Moon`/`night_sky_source`, `DarkSkyReport`
>   (ULOR, Rayleigh spectral-ULOR, blue %, upward CCT, melanopic spill, S/P),
>   and CIE 191 mesopic road correction (`road_luminance::mesopic`, `spectrum`
>   feature on the core crate).
> - **S3** GPU (`eulumdat-rt`): per-photon wavelength sampled on-GPU from an
>   uploaded SPD CDF, exact 5 nm weighting LUT uploaded as a buffer, 4-channel
>   (X,Y,Z,scotopic) atomics. GPU CCT matches direct colorimetry within ~6 K
>   (3000→2994 K, 6500→6497 K); S/P ordering matches CPU.
> - **E2E**: 37 real vendor SPDs from `docs/SPDs/` traced through the full MC
>   pipeline, worst ΔCCT **24 K**. Energy invariance (spectral vs monochromatic)
>   holds to 1e-9.
>
> The prose below is the original design rationale; a few naming details
> (`SunDisc`, the 4-channel GPU subset vs the 7-channel CPU set) differ from the
> as-built code, which is authoritative.



Extends `eulumdat-goniosim.md` (CPU reference tracer) and `eulumdat-rt.md`
(GPU tracer). Those documents describe *monochromatic* photon transport at
555 nm. This one takes the same engine from "one wavelength, electric light
only" to the full day–night cycle:

> One tracer, full spectrum, sun to stars.
> The same photons that give you an LDT give you CCT-over-angle,
> daylight factor, mesopic road luminance, and a dark-sky report.

## Why now — the pieces already exist

| Piece | Where | Status |
|---|---|---|
| `Photon.wavelength` field | `eulumdat-goniosim/src/ray.rs` | Present, hardcoded 555 nm, unused |
| SPD loader (4 vendor formats) | `eulumdat/src/atla/spd_loader.rs` | Done, lab-grade |
| CIE colorimetry (Robertson CCT, Duv, u′v′) | `eulumdat/src/atla/colorimetry.rs` | Done, Signify parity ±15 K / ±0.0003 Duv |
| Spectrum synthesis from CCT+CRI | `spectral.rs::synthesize_spectrum` | Done |
| TM-27 SPDX export | `eulumdat/src/atla/spdx.rs` | Done |
| SPD test corpus | `docs/SPDs/` (Luxeon ×21, Yuji Day/Nite ×4, Signify ×6) | Done |
| Solar position, Perez + CIE skies | `eulumdat-daylight` | Done (photometric) |
| Frozen coordinate adapter (horizon ↔ Bevy ↔ Type-C) | `eulumdat-daylight/src/coords.rs` | Done |
| CIE 171:2006 TC 5.1–5.8 | `goniosim/tests/cie_171.rs` | Passing |
| CIE 171:2006 TC 5.9–5.14 (daylighting) | — | **Missing — becomes the daylight validation target** |
| Patch-radiosity CU engine (Stockmar grid) | `eulumdat/src/radiosity.rs` | Done, DIAL parity |
| Star catalog (HYG v3) | `eulumdat-bevy/data/` | Done (visual) |
| Skyglow demo | `?wasm=skyglow_demo` | Done (visual) |

Nothing below requires new science infrastructure — it requires *wiring the
existing colorimetry into the existing tracer* and adding sky/night sources.

---

# Part I — Spectral Monte Carlo

## Transport model: one wavelength per photon

Each photon carries a single wavelength, importance-sampled from the source
SPD. This is the decision that makes everything else simple:

- **Unbiased** — the ensemble of photons *is* the spectrum.
- **Zero per-photon memory growth** — the field already exists (f64 CPU,
  one extra f32 on GPU).
- **Trivially parallel** — no correlation between photons, same as today.
- **Dispersion for free** — refraction just reads n(λ) for *this* photon.

Rejected alternative: hero-wavelength spectral sampling (HWSS, 4 correlated
wavelengths per path). HWSS reduces color noise for *camera images* at low
sample counts. Our detector integrates 10⁶–10⁹ photons per run — spectral
noise averages out exactly like directional noise does. Revisit only if the
camera output mode (`eulumdat-rt` Phase 3) shows visible chroma noise.

### Sampling

```rust
/// Built once per source from its SPD (1 nm grid, 380–780 default,
/// extensible to 280–1400 for UV/NIR when the SPD covers it).
pub struct SpectralSampler {
    cdf: Vec<f64>,        // cumulative radiant power
    lambda_min: f64,
    step: f64,
}
impl SpectralSampler {
    pub fn from_spd(spd: &SpectralDistribution) -> Self;
    pub fn sample(&self, xi: f64) -> f64;   // invert CDF → wavelength [nm]
}
```

Every `Source` variant gains an optional SPD:

```rust
pub struct SourceSpectrum {
    pub spd: SpectralDistribution,
    sampler: SpectralSampler,
}
// Source::Led { .., spectrum: Option<SourceSpectrum> }
// None → monochromatic 555 nm, bit-identical to today's results.
```

### The critical convention: photons are radiometric, detectors are photometric

Photon `energy` stays a **radiant** (power-proportional) weight. All
photometric weighting — V(λ), V′(λ), melanopic s_mel(λ), PAR — happens **at
the detector**, not at emission.

This is what makes one trace yield every metric: a 730 nm far-red photon
carries real watts but ~0 lumens. Weight at emission and you can never
recover scotopic, melanopic, or PPFD from the same run. Weight at detection
and they are seven multiply-adds per photon.

Normalization: the source's luminous flux (lm) and its SPD jointly define
the radiant flux via `Φ_e = Φ_v / (683 · ∫SPD·V dλ / ∫SPD dλ)`. The
`SourceSpectrum` constructor computes this once; `detector.to_candela()`
is unchanged for the photopic path — existing tests keep passing.

## Spectral materials

`MaterialParams` fields become wavelength-addressable, defaulting to today's
scalar behavior:

```rust
pub enum SpectralValue {
    Constant(f64),                       // today's behavior, default
    Curve(SpectralDistribution),         // measured datasheet curve
    Sellmeier { b: [f64; 3], c: [f64; 3] }, // for IOR dispersion only
}

pub struct MaterialParams {
    pub reflectance: SpectralValue,      // was reflectance_pct: f64
    pub ior: SpectralValue,              // Sellmeier → real dispersion
    pub transmittance: SpectralValue,    // tinted/colored covers
    // thickness_mm, diffusion_pct unchanged (geometric, not spectral)
}
```

Evaluation is one lookup: `material.reflectance.at(photon.wavelength)`.
The internal `Material` enum evaluates lazily per interaction — no
precomputation needed on CPU. Catalog additions with real dispersion data:

| Material | Source of n(λ) | Notes |
|---|---|---|
| PMMA | Sellmeier (Sultanova 2009) | n: 1.505 @ 400 nm → 1.485 @ 700 nm |
| BK7 / soda-lime glass | Sellmeier (Schott) | classic |
| Polycarbonate | Sellmeier (Sultanova) | strongest dispersion of the three |
| Aluminum | tabulated R(λ) | slightly blue-deficient |
| Gold anodized | tabulated R(λ) | strongly yellow — good demo material |
| White paint (TiO₂) | flat 85%, dips < 420 nm | UV edge |

### Phase-2 material: phosphor / fluorescence

White LEDs *are* a spectral transport problem: blue pump + Stokes-shifted
re-emission. A `Phosphor` material absorbs at λ < λ_edge with probability
`a(λ)`, re-emits isotropically at a wavelength sampled from the emission
SPD, weighted by quantum yield. This unlocks simulating **remote-phosphor
optics** — a real design task no mainstream lighting tool covers, and a
flagship demo: trace a royal-blue die through a phosphor plate and watch
the detector's colorimetry converge to 3000 K.

## Spectral detector — two modes, one trace loop

Full spectral binning is memory-hungry: 360×181 direction bins × 401
wavelengths × f64 ≈ 209 MB. Almost no use case needs it. So:

```rust
pub enum DetectorMode {
    /// Today's single f64 per bin. Default.
    Photopic,
    /// 7 accumulators per direction bin: X, Y, Z, V′ (scotopic),
    /// melanopic, PAR(400–700 quanta), radiant. ~10 MB at 1°×1°.
    /// Y is photopic — LDT export unchanged.
    WeightedChannels,
    /// Full SPD per direction bin at `step_nm` (5 nm default → 81 bands,
    /// ~42 MB at 1°×1°). For TM-27 export and CRI-over-angle.
    SpectralBins { step_nm: f64 },
}
```

`WeightedChannels` is the workhorse: per detected photon it costs seven
multiply-adds against 1 nm lookup tables (CIE 1931 CMFs, V′, s_mel from
CIE S 026, PAR). From X,Y,Z per angle you get chromaticity, CCT, and Duv
*per exit direction* via the existing `Colorimetry` — i.e. **color-over-angle**,
the classic LED luminaire defect (yellow ring from phosphor path length),
now measurable in simulation.

### New outputs

| Output | Mode needed | Consumer |
|---|---|---|
| LDT / IES (photopic) | any | unchanged |
| Scotopic LDT + per-angle S/P ratio | WeightedChannels | night/road track (Part III) |
| CCT / Duv over angle (polar plot) | WeightedChannels | LED design, color uniformity |
| Melanopic DER, α-opic ELR | WeightedChannels | HCL / WELL compliance |
| PPFD distribution | WeightedChannels | horticulture (`atla/greenhouse.rs`) |
| TM-27 SPDX (angular-integrated or per zone) | SpectralBins | `spdx.rs`, exists |
| CRI / TM-30 over angle | SpectralBins | premium analysis |

## Spectral validation (all analytic or corpus-anchored)

1. **Colorimetric round trip** — `synthesize_spectrum(3000 K, CRI 80)` →
   trace through vacuum → detector SPD → `analyze_spd` → CCT within ±15 K,
   Duv within ±0.0003. Same bar as the Signify parity suite.
2. **Energy invariance** — spectral run and monochromatic run of the same
   scene agree on total flux and photopic LVK within MC noise.
3. **Fresnel dispersion** — R(λ) of a glass slab at 60° incidence matches
   the closed-form Fresnel value from Sellmeier n(λ) at 400/550/700 nm.
4. **Beer–Lambert tint** — Luxeon 3000 K SPD through an absorbing filter
   with known τ(λ): transmitted CCT shift matches the analytic integral.
5. **Corpus sweep** — every SPD in `docs/SPDs/` traced through free space
   reproduces its own colorimetry (extends `spd_colorimetry_corpus` example
   into the MC domain).

## GPU port (eulumdat-rt)

- Photon payload: +1 f32 (`wavelength`). RNG unchanged.
- Source CDF: one extra storage buffer (401 f32), binary-search sample —
  same pattern as the existing LVK CDF sampling.
- LUTs: CMFs / V′ / s_mel / PAR as a single 401×7 f32 buffer.
- Detector: `WeightedChannels` = 7 atomic adds instead of 1. Fixed-point
  u32 as today. `SpectralBins` stays CPU-only in v1 (atomics × 81 bands
  is memory-bandwidth-hostile; do it when a real need appears).
- Materials: `SpectralValue::Curve` uploaded as 81-band f32 texture per
  material; `Sellmeier` evaluated inline in WGSL (cheap).
- Validation: identical scenes CPU vs GPU, per-channel similarity > 0.99 —
  same harness as the monochromatic comparison in `eulumdat-rt.md`.

---

# Part II — Daylight

`eulumdat-daylight` supplies the physics (sun position, Perez/CIE sky
luminance, lux availability). The tracer gains two source types that consume
it, plus aperture geometry. The crate stays a pure leaf — goniosim depends
on it, not the reverse.

## Sources

```rust
/// The sun: quasi-parallel disc, 0.533° angular diameter.
Source::Sun {
    position: SolarPosition,          // from eulumdat_daylight::solar
    dni_lux: f64,                     // direct normal illuminance
    spectrum: Option<SourceSpectrum>, // v1: CIE D-illuminant from air-mass CCT
},

/// The sky: luminance-weighted hemisphere dome.
Source::SkyDome {
    radiance: SkyRadiance,            // Perez or CIE sky, already built
    dome_radius: f64,                 // >> scene extent
    spectrum: SkySpectralModel,
},
```

**SkyDome sampling.** Build a 2D CDF over the dome from
`radiance.relative_luminance(θ, φ) · cosθ · dΩ` on a fine grid (1–2°;
Tregenza/Reinhart patches only for *reporting*, not sampling — no reason to
quantize the sampler). Sample a direction, emit a photon *inward* from the
dome surface. The CDF rebuilds only when sun position or sky params change —
same machinery as the `FromLvk` CDF.

**Sky spectrum.** Three fidelity levels, in order of implementation:

1. `Uniform(D65)` — one SPD for the whole dome. Enough for daylight factor
   and any photopic-only result (spectrum then cancels entirely).
2. `CctField` — per-direction CCT (clear zenith ~10 000–25 000 K, horizon
   ~5 500 K, overcast ~6 500 K flat), SPD reconstructed on the fly from the
   **CIE D-series eigenvectors** (S₀ + M₁S₁ + M₂S₂). ~40 lines; the D-series
   math belongs in `eulumdat-spectrum` (see crate layout below).
3. Sun: Bird/ASTM G-173-shaped SPD attenuated by air mass. Phase 3 polish.

## Geometry: rooms with holes

Daylight enters scenes through openings. Two additions to `geometry.rs`:

- `Sheet` gains an optional `Vec<Aperture>` (rectangular cutouts) — a wall
  with a window hole is one primitive, not CSG.
- Glazing = existing `ClearTransmitter` with `SpectralValue::Curve` τ(λ) —
  low-E, solar-control, and tinted glass become catalog entries from
  manufacturer curves. No new physics: Fresnel + Beer–Lambert already
  handle it once materials are spectral.

## Detectors: illuminance planes

The spherical goniophotometer detector is wrong for daylighting — the
question is "how much light lands on the desk," not "what escapes." Add:

```rust
pub struct PlaneDetector {
    pub sheet: Sheet,                 // position/orientation/extent
    pub grid: (usize, usize),         // Stockmar-compatible resolution
    pub bins: DetectorMode,           // same three modes as the sphere
}
```

Photons crossing the plane record `energy · |cosθ|` into the cell — this is
also exactly what CIE 171 TC 5.x tables specify, so the existing direct-
illumination tests can migrate to it.

## Validation: CIE 171:2006 TC 5.9–5.14 — the missing six

These are *the* daylighting benchmarks and we currently skip them:

| TC | What it tests | Notes |
|---|---|---|
| 5.9 | Sky component, unglazed opening, CIE general skies | core |
| 5.10 | Sky component, glazed opening | needs glazing τ |
| 5.11 | Sky component with external obstruction | shadowing |
| 5.12 | Internally reflected component | MC interreflection |
| 5.13, 5.14 | External reflection cases | **known-questioned reference values** — implement, compare, but don't gate CI on them (same policy as the TC 5.7 errata note in `cie-171-2006-tests.md`) |

Second validation axis: **daylight factor cross-check against the radiosity
CU engine** on the same room — two independent solvers, one geometry. Where
they agree we trust both; where they diverge (specular, glazing angles) MC
is the reference.

## Deliverables

- **Daylight factor report**: DF grid under CIE overcast sky, min/avg/max,
  uniformity — the number building codes ask for.
- **Combined electric + daylight**: two traces (sources are independent —
  MC is linear), summed on the illuminance plane. Slider in the UI: dim the
  luminaires as the sun comes up. This is the daylight-autonomy scaffold.
- **Annual loop (phase 3)**: EPW weather reader → Perez per hour →
  sDA/ASE. Embarrassingly parallel (8 760 independent runs); a natural GPU
  batch job and a headline feature. Not in scope until the single-moment
  path is validated.
- **Bevy**: sky dome from the same `SkyRadiance` (evaluate luminance +
  D-series color per vertex), sun disc, date/time/location scrubber.
  Shares `coords.rs` so the sun in the visual and the sun in the trace are
  provably the same sun.

---

# Part III — Nightlight

Night is not "daylight off." It has its own sources, its own photometry
(the eye switches sensors), and its own compliance regime (dark-sky). All
three build directly on Parts I–II.

## Sources

```rust
/// Moonlight: sun machinery reused, scaled by phase.
Source::Moon {
    position: SolarPosition,          // same horizon coords
    phase: f64,                       // 0 = new, 1 = full
    // illuminance from phase curve: full ≈ 0.25 lx, quarter ≈ 0.025 lx
    // SPD: solar SPD × lunar-regolith albedo ≈ sunlight at CCT ~4 100 K
},

/// Starlight + airglow: uniform dome, ~0.001 lx. The HYG star field in
/// eulumdat-bevy stays a *visual* layer — photometrically the aggregate
/// uniform dome is correct and 10⁶× cheaper.
Source::NightSky { horizontal_lux: f64 },   // default 0.001

/// Skyglow: the SkyDome source with a Garstang/Cinzano-shaped
/// luminance field (bright toward the city azimuth, falls off with
/// altitude). Phase 2; the skyglow demo already has the visual language.
Source::Skyglow { city_azimuth_deg: f64, zenith_luminance: f64, .. },
```

## Mesopic photometry — CIE 191:2010 (the road-lighting payoff)

Between ~0.005 and ~5 cd/m² the eye is neither photopic nor scotopic.
Road luminance lives *exactly* in this range (0.3–2 cd/m²), which means
every photopic-only road calculation misstates what drivers actually see —
and the error depends on the lamp's spectrum. This is where the
`road_luminance` module (in progress) and the spectral tracer meet:

```rust
// in eulumdat-spectrum:
/// S/P ratio from an SPD: (∫SPD·V′dλ / ∫SPD·V dλ) · (1700/683)
pub fn sp_ratio(spd: &SpectralDistribution) -> f64;

/// CIE 191 iterative mesopic system: photopic luminance + S/P → L_mes.
pub fn mesopic_luminance(l_photopic: f64, sp: f64) -> f64;
```

- The `WeightedChannels` detector already accumulates V and V′ — S/P per
  angle falls out of a completed trace for free.
- `road_luminance` gains a mesopic mode: same geometry and r-tables,
  luminances corrected through CIE 191 before Lavg/Uo/Ul are reported.
- **Test vectors**: the Yuji corpus is purpose-built for this — WB *Day*
  4000 K/5000 K (S/P ≈ 1.6–2.0) vs WB *Nite* 2200 K/2700 K (S/P ≈ 0.4–1.1).
  Published S/P values for these lamp classes anchor the integrals.

## Dark-sky / light-pollution report

All computable from one `WeightedChannels` trace of the luminaire:

| Metric | Definition | Basis |
|---|---|---|
| ULR / ULOR | flux at γ > 90° / total | photopic channel (exists today) |
| **Spectral ULOR** | upward flux weighted λ⁻⁴ (Rayleigh) | skyglow scatters blue disproportionately — an amber and a white luminaire with identical ULR differ ~4× in sky-glow contribution |
| Blue content % | ∫SPD 400–500 nm / ∫SPD 380–780 | DarkSky guidance |
| CCT limit check | ≤ 2 200 K (DarkSky), ≤ 3 000 K (many ordinances) | existing colorimetry |
| Melanopic content of spill | melanopic channel at γ > 90° | ecological impact |
| S/P ratio | above | context for mesopic claims |

Package as a **"night compliance" panel** next to the existing validation
panel: one LDT + one SPD in, a dark-sky report card out. No competitor
does this; it is a natural headline for the `spectrography` branch.

## Scene presets

- `street_at_night(luminaire, spd, road_class)` — road + luminaire +
  `NightSky`, mesopic road report out.
- `bedroom_window_streetlight(..)` — light-trespass: illuminance +
  melanopic lux on a vertical plane behind a window. Increasingly litigated;
  trivially computable here.
- `observatory_horizon(..)` — Skyglow source + spectral ULOR integration.

---

# Crate layout

One new leaf crate, everything else is extension:

```
eulumdat-spectrum (NEW — leaf, zero deps, like eulumdat-daylight)
├── SpectralDistribution, SpectralSampler          (moved/re-exported from atla)
├── LUTs: CIE 1931 CMFs, V(λ), V′(λ), s_mel (CIE S 026), PAR   @ 1 nm
├── Colorimetry (Robertson CCT/Duv)                (moved from atla)
├── CIE D-series (S₀,S₁,S₂ eigenvector reconstruction)
├── sp_ratio, mesopic_luminance (CIE 191)
└── Sellmeier evaluation + dispersion catalog

dependency flow (leaves at top):
    eulumdat-spectrum      eulumdat-daylight
            ▲   ▲               ▲   ▲
            │   └───────┐       │   │
        eulumdat        eulumdat-goniosim ──▶ eulumdat-rt
            ▲                   │
            └── atla/* re-exports keep the existing public API stable
```

`atla/spectral.rs`, `colorimetry.rs`, `spd_loader.rs` keep their paths and
re-export from `eulumdat-spectrum` — no downstream breakage (wasm, ffi, py).
`eulumdat-daylight` stays dependency-free; its D-series *coefficients* (per-
patch CCT model) live there as pure numbers, the SPD *reconstruction* lives
in `eulumdat-spectrum`, and goniosim composes the two.

# Phasing

Three tracks; S is the trunk, D and N branch off it after S1.

| Phase | Content | Gate |
|---|---|---|
| **S1** | `eulumdat-spectrum` crate; wavelength sampling in goniosim; `WeightedChannels` detector; radiometric/photometric convention | colorimetric round trip ±15 K; monochromatic results bit-compatible when `spectrum: None` |
| **S2** | `SpectralValue` materials, Sellmeier catalog, spectral glazing; CCT-over-angle + scotopic LDT export | Fresnel-dispersion + Beer–Lambert analytic tests; corpus sweep |
| **S3** | GPU port of S1+S2 (payload f32, LUT buffer, 7-channel atomics) | CPU/GPU similarity > 0.99 per channel |
| **S4** | Phosphor material; `SpectralBins` mode; TM-27/CRI-over-angle | blue-pump → 3000 K white round trip |
| **D1** | `Sun` + `SkyDome` sources (Uniform D65); apertures; `PlaneDetector` | TC 5.9 sky component vs analytic |
| **D2** | Glazing τ(λ); TC 5.10–5.12; DF report; radiosity cross-check | CIE 171 daylight suite green (5.13/5.14 informational) |
| **D3** | Per-patch sky CCT (D-series); Bird sun SPD; Bevy sky dome + time scrubber | visual sun == traced sun via `coords.rs` |
| **D4** | EPW reader; annual loop; sDA/ASE (GPU batch) | spot-check vs published Radiance results |
| **N1** | `sp_ratio` + CIE 191 mesopic; `Moon`/`NightSky` sources; mesopic mode in `road_luminance` | Yuji Day/Nite S/P vs published values; CIE 191 table reproduction |
| **N2** | Dark-sky report (spectral ULOR, blue %, melanopic spill); night presets | amber-vs-white skyglow ratio matches Rayleigh analytic |
| **N3** | Garstang skyglow source; Bevy night mode (star field + glow) | qualitative vs skyglow demo |

# Cost estimates

- **Wavelength sampling**: one CDF binary search per photon — < 2% on
  emission, invisible overall.
- **WeightedChannels**: 7 multiply-adds per *detected* photon (detection is
  ~1% of trace time) — negligible. Memory 7× detector: ~10 MB at 1°.
- **Spectral material lookup**: one interpolated read per interaction — a
  few % on material-heavy scenes; Sellmeier is 6 mults.
- **SpectralBins**: memory-bound (42 MB at 5 nm/1°), CPU-only, opt-in.
- **SkyDome CDF**: ~16k-cell CDF, rebuilt only on sun/sky change — free.

The performance targets in `eulumdat-rt.md` survive intact; spectral is a
payload change, not an architecture change.

# Non-goals (unchanged from the base docs, plus)

- No polarization (matters for sky at large scattering angles — noted, deferred).
- No participating atmosphere inside the scene (fog/aerosol between
  luminaire and road) — Skyglow models the *result*, not the volume.
- No thermal/IR radiometry beyond carrying NIR wavelengths through.
- Star field remains visual; photometric night sky is the uniform dome.
