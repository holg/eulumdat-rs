# Plan: EN 1838 (emergency lighting) and EN 17037 (daylight in buildings)

Status (2026-11-07): limit values in; P0a core (GLDF configurations), P1
(`eulumdat::emergency` + CLI) and the interior part of P2 built: Interior
designer emergency section (routes drawn on the plan, open area, points of
emphasis, automatic placement, results, PDF section, link keys), synthetic
demo emergency luminaires, GLDF emergency emitters as project luminaires,
and the Light scene emergency mode (several luminaire types per scene,
normal lighting off / emergency on, route band and equipment marks with an
on / off switch, walking a route at eye height, EN 1838 check of the
traced points).
Next: GLDF configurations as one placed luminaire, high-risk areas in the UI,
exterior escape routes (Outdoor designer). IFC import comes after this
(it will feed the same room / window / escape-route model; the sibling repo
`gldf-rs` already has `gldf-ifc-viewer-wasm`).

Why: standards are where Candelabra is strong (EN 12464-1 grids and UGR,
EN 13201, EN 15193-1 Annex E for sensors, TM-30 / CIE 13.3, TM-33-23,
TM-32-24, TM-27). Emergency lighting and daylight are the two standards
every interior lighting project is checked against next.

## 0. Sources

- Limit values (2026-11-07): `crates/eulumdat/data/en1838_en17037_limits.json` (the rule
  data, from the user's limit sheet `docs/EN_1838_17037_Limit_Values.pdf`,
  git-ignored like all PDFs there). The core embeds this table; a national
  profile can override it later.
- The sheet adds, outside the JSON: criteria hold at the end of the rated
  duration with aged / dirty luminaires (maintenance factor), direct light
  only; escape routes are judged on the centre line and the central band
  (≥ 50 % of the width) for routes up to 2 m wide; anti-panic lighting
  applies to areas > 60 m² (or named high-risk zones); rated duration ≥ 1 h
  (3 h for high-risk / sleeping occupancies); the high-risk measurement
  plane is the task plane; points of emphasis = fire alarm call points,
  first-aid and fire-fighting equipment, externally lit escape signs.
- Not in the sheet, our defaults (settings, stated in the report): point
  spacing along the centre line (0.1 m), grid in open / high-risk areas
  (≤ 0.5 m, like an EN 12464-1 grid), maintenance factor (user input,
  default 0.8), ageing at the end of the duration (from GLDF
  `DurationTimeAndFlux` when present).
- Full standard texts, if they come later, go into `docs/` too (never into
  git, never copied into code, tests or docs).
- National rules on top (Germany: DIN VDE V 0108-100-1, ASR A3.4/7) are a
  later profile. EN 50172 / EN 50171 only appear as inputs (duration) and
  report notes.

## 0a. Test product: SLV Tria 2 (GLDF example) — one luminaire, light + emergency + sensor

The GLDF example in gldf-rs (`tests/data/SLV - Tria 2.gldf`, extracted
next to it; also served by gldf.icu) is a recessed ceiling luminaire with
two light-emitting objects and a sensor slot in its L3D (`LEO0`, `LEO1`
0.8 m apart, `Sensor`). Its emitters cover every GLDF emergency case:

| Emitter | GLDF | Normal mode | Emergency mode |
|---|---|---|---|
| em-n-n / em-m-n / em-w-n | `FixedLightEmitter emergencyBehaviour="None"` | narrow 1600 lm / middle 7800 lm / wide 7800 lm | off |
| em-w-c | `emergencyBehaviour="Combined"` + `EmergencyRatedLuminousFlux` 100 | wide 7800 lm | wide at 100 lm |
| em-w-o | `emergencyBehaviour="EmergencyOnly"` | off | wide at 100 lm |
| em-s | `SensorEmitter` → `sensor/sensor.ldt` | detection | detection |

Variants pick the emitters per LEO (`EmitterReference` +
`EmitterObjectExternalName`), e.g. "no Em / only Em / sens" = normal light
on LEO0, emergency-only light on LEO1, the sensor in the slot. That is the
"one configuration" to plan with: the same placed luminaires give EN 12464-1
(normal emitters), EN 1838 (emergency emitters) and the sensor coverage
(sensor emitter), and the Light scene can switch them between normal and
emergency mode while the sensor reacts.

Notes:
- Its `sensor.ldt` is a luminaire LDT (values up to 486 "m"), already
  flagged as `SensorWarning::ImplausibleRange`; for the demo it is shown
  with that warning or paired with a demo sensor.
- The extracted folder also has `ldc/emergency.ldt` (a real emergency
  downlight: 45 lm, 38°, 3 h, absolute photometry), not referenced by
  `product.xml`: a second test for emergency-only photometry and the
  glare check.
- Git: it stays in gldf-rs. Tests here read `../gldf-rs/tests/data/
  SLV - Tria 2.gldf` (or an env var) and skip when it is missing; the
  committed tests use a synthetic GLDF with the same structure
  (extend `crates/eulumdat/tests/fixtures/sensors/make_demo_gldf.py`:
  normal + combined + emergency-only emitters, a sensor emitter, variants).

### Data analysis (2026-11-07): is it enough?

What the Tria 2 really contains (read with our parser):

| File | LDT says | `product.xml` says |
|---|---|---|
| `ldc/narrow.ldt` | "NEW TRIA", 2200 lm, 27 W, absolute, rotationally symmetric, beam 23° | em-n-n 1600 lm |
| `ldc/middle.ldt` | "NEW TRIA 155", 562 lm, 8.2 W, absolute, beam 36° | em-m-n 7800 lm |
| `ldc/wide.ldt` | "OCULUS CW" (another SLV product), 35 lm, 0.9 W, beam 87° | em-w-* 7800 lm normal, 100 lm emergency |
| `ldc/emergency.ldt` (unreferenced) | "NEW TRIA 155 EM 3h", 45 lm, same optic as middle (peak 1814.5 cd/klm, 36°) | – |
| `sensor/sensor.ldt` | "OCULUS CW" again: a luminaire LDT, no detection data | sensor1 |

- The light distributions are real, the GLDF fluxes are invented (wide:
  35 lm in the LDT vs 7800 lm, × 223). The import must scale each
  photometry to its emitter's flux and warn about such mismatches.
- No duration, battery, ballast lumen factor or dedicated emergency type
  in the Tria 2; the "3 h" is only in the EM LDT's name. Ra 80 (light
  sources) meets the Ra ≥ 40 rule.
- L3D: two heads at x = ±0.4 m (LEO0 / LEO1, round, Ø 57.5 mm), the sensor
  object at the centre.
- Emergency mode, direct light only, no maintenance factor (our quick
  calculation, to be reproduced by the core as a regression check):

| Emitter in emergency mode | h | E nadir | escape route ≥ 1 lx (Emax/Emin ≤ 40): max spacing | open area ≥ 0.5 lx: max grid | I max 60°–90° |
|---|---|---|---|---|---|
| wide @ 100 lm | 2.5 m | 8.6 lx | 6.6 m | 6.2 m | 6.4 cd |
| wide @ 100 lm | 3 m | 6.0 lx | 7.3 m | 6.9 m | 6.4 cd |
| wide @ 100 lm | 4 m | 3.4 lx | 8.2 m | 8.3 m | 6.4 cd |
| EM pack 45 lm, 36° | 2.5 m | 13.1 lx | 4.4 m | 3.6 m | 0.5 cd |
| EM pack 45 lm, 36° | 3 m | 9.1 lx | 5.0 m | 4.2 m | 0.5 cd |
| EM pack 45 lm, 36° | 4 m | 5.1 lx | 5.5 m | 5.3 m | 0.5 cd |

  (All Tria LDTs are absolute photometry with LOR 100 %, so "emergency lm
  = lamp lm" or "= luminaire lm" gives the same numbers here; for relative
  LDTs the import has to scale by the luminaire flux, see below.)

GLDF can carry more than the Tria 2 uses (gldf-rs-lib): `emergencyBehaviour`
per emitter, `EmergencyRatedLuminousFlux` and `EmergencyBallastLumenFactor`
(emitter / light source / control gear), product `Emergency` with
`DurationTimeAndFlux` (flux over time) and `DedicatedEmergencyLightingType`.

Corpus (`gldf-search-data/gldfs`, 450 591 files, vendor data, not in git):
a 5 000 sample has none of these structured elements, because almost all
of these GLDFs were generated by light-other-rs from scraped data
(295 of 300). About 10 % mention emergency lighting in free text
("emergency luminous flux BLF 8 %", "1h or 3h", "EN 60598-2-22"; Lightnet,
3F Filippi, ERCO, RZB, Trilux, Glamox …): usable as hints, not as data.

Verdict:

| Need | Enough? | Source / gap |
|---|---|---|
| Emergency photometry + flux, glare check | yes | Tria 2 structure, real EM LDT, corpus EM luminaires (BLF from text, user confirms) |
| GLDF emergency structure for tests | yes, for import | Tria 2 (None / Combined / EmergencyOnly + variants + LEOs); synthetic demo GLDF for committed tests |
| Duration / flux at end of duration | partly | `DurationTimeAndFlux` when present, else rated emergency flux + user-set ageing factor |
| Sensor in the same configuration | no (Tria 2) | its sensor file is a luminaire LDT; use the demo sensors / sensor corpus in the synthetic GLDF |
| Safety signs (luminance, size) | no | user input (internally / externally lit, height); luminance values from the sign maker |
| EN 1838 limit values | no | standard text (P0) |
| Reference results for EN 1838 | partly | analytic point-source cases + the table above; an established tool's results for one floor plan (user) |
| EN 17037 geometry, sky, sun | yes for sky / sun | `eulumdat-daylight` (CIE overcast / clear / uniform, Perez, locations); windows are new |
| EN 17037 site climate (E_v,d,med, hourly skies) | no | weather files (EPW / TMY), download only with the user's OK; none in the repo |
| EN 17037 limit values / targets | no | standard text (P0) |

Consequences for the import (P0a):

1. Flux of an emitter in normal mode = `RatedLuminousFlux`; the LDT is
   scaled to it (its own luminaire flux = lamp flux × LOR; absolute LDTs:
   the flux itself), warning when they differ by more than 10 %.
2. Emergency flux, first that exists: the lowest flux of
   `DurationTimeAndFlux` within the rated duration → `EmergencyRated
   LuminousFlux` → `EmergencyBallastLumenFactor` × rated flux → user input
   (the description's "BLF x %" offered as a suggestion).
3. `Combined`: same photometry, emergency flux; `EmergencyOnly`: off in
   normal mode; `None`: off in emergency mode.
4. Sensor emitters use the sensor file (warnings as today); the L3D
   `SensorObject` gives its offset, `LightEmittingObject` the lamp offsets.

### Step: read GLDF emitters and variants (no standard text needed)

Today the GLDF import keeps photometries, sensors and the L3D only
(`eulumdat_scene::l3d::GldfExtract`). Add, without a gldf-rs dependency
(a small reader like `gldf_file_types`):

```text
GldfEmitter { id, kind: Light { photometry file, rated_lm, emergency:
              None | Combined { emergency_lm } | EmergencyOnly } | Sensor { file } }
GldfVariant { id, name, mounting, slots: [(emitter id, LEO / object name)] }
LuminaireConfig = a variant resolved: normal lamps, emergency lamps (LDT +
              emergency flux), sensor (file + offset), LEO offsets from the L3D
```

The web app offers the variants when a GLDF is opened (default: the first
one with emergency or sensor parts) and keeps the choice in the project
item. This step can start now.

## 1. EN 1838: limit values (from `crates/eulumdat/data/en1838_en17037_limits.json`)

| Topic | Requirement |
|---|---|
| Escape routes ≤ 2 m wide | E on the floor along the centre line ≥ 1.0 lx; central band (≥ 50 % of the width) ≥ 0.5 lx; Emax / Emin ≤ 40 along the centre line |
| Open (anti-panic) areas | > 60 m² (or named zones): E ≥ 0.5 lx on the floor, 0.5 m border excluded; Emax / Emin ≤ 40 |
| High-risk task areas | ≥ 10 % of the normal maintained E and ≥ 15 lx, on the task plane; Emax / Emin ≤ 10 |
| Points of emphasis | ≥ 5.0 lx vertical at fire alarm call points, first-aid and fire-fighting equipment, externally lit escape signs |
| Disability glare, I max between 60° and 90° from the downward vertical | h < 2.5 m: 500 cd (escape route / anti-panic), 1000 cd (high-risk); < 3.0: 900 / 1800; < 3.5: 1600 / 3200; < 4.0: 2500 / 5000; < 4.5: 3500 / 7000; ≥ 4.5: 5000 / 10000 |
| Colour rendering | Ra ≥ 40 (all three) |
| Response | 50 % within 5 s, 100 % within 60 s; high-risk 100 % within 0.5 s |
| Duration | ≥ 1 h, 3 h for high-risk / sleeping occupancies (input + report) |
| Calculation | direct light only, end of rated duration, maintenance factor |

Glare direction: escape routes in the directions of travel along the
route, open areas at all azimuths (60°–90° = the γ band of the LDT).

## 2. EN 1838: building on what we have

- Direct-only point-by-point is what the Interior designer already computes
  (`zonal` PPB, polygon rooms, `PlanFrame`, `plan_drag`): emergency
  luminaires are just another layout with their emergency flux.
- Emergency flux sources, in order: TM-32-24 BIM `emergency_luminous_flux`
  (+ `EmergencyType` maintained / non-maintained), GLDF
  `EmergencyBallastLumenFactor` / emergency-only emitters (gldf-rs-lib),
  else user input (lm or factor of the normal flux).
- Glare check: `ldt.sample(c, γ)` over 60°–90° in the route direction (or
  all C-planes for open areas), scaled to the emergency flux, against the
  mounting-height table.

### Core: `eulumdat::emergency`

```text
EmergencyLuminaire { ldt, flux_lm (emergency mode), position, rotation, height }
EscapeRoute        { polyline (m), width }           // centre line + band
OpenArea           { polygon }                        // 0.5 m border off
HighRiskArea       { polygon, task_lux }
PointOfEmphasis    { kind, position }
SafetySign         { lit: Internal|External, height, position, facing }
evaluate(&Plan, &Rules) -> EmergencyReport            // per item: Emin, Emax,
                                                      // ratio, pass / fail,
                                                      // limiting point
Rules = data table per edition / national profile
```

Escape routes sampled along the centre line (spacing ≤ 0.1 m or per text)
plus the central band; open areas on a grid inside the inset polygon.

### Web

- Interior designer, new section "🚨 Emergency lighting": pick emergency
  luminaires (project items; flagged by BIM / GLDF data or by hand), layout
  (grid / by hand / at points of emphasis), draw escape routes on the plan
  (polyline tool next to "✏️ Draw Polygon"), mark open / high-risk areas,
  place points of emphasis and signs; plan overlay: route centre lines
  coloured by lux, failing spots red, glare warnings on luminaires; table
  per route / area; share-link keys; PDF chapter "Emergency lighting
  (EN 1838)".
- Light scene (built 2026-11-07): "emergency mode" (normal luminaires
  off, emergency ones at their emergency flux × MF, combined ones at their
  share; exposure key 2 lx, false colour 0–5 lx), escape routes as a
  safety-green band on the floor, open-area border, equipment with its
  measured vertical lux; the markings switch off to see the room as a
  person escaping sees it; "walk the route" puts the camera at 1.6 m on a
  route (slider, ▶ at 1 m/s). In emergency mode the grids are the routes
  (a point every 0.5 m on the centre line), the open area (0.5 m grid,
  0.5 m border left out) and one vertical point per piece of equipment;
  points take direct light only, as EN 1838 counts it.
- Outdoor designer: not needed (EN 1838 is about buildings; exterior
  escape routes later if asked).

### Validation

- Analytic point sources (inverse-square × cos³) on routes and areas.
- Same layout and LDT in an established planning tool: Emin / Emax on the
  route within rounding (done that way for EN 12464-1 before).
- Demo data: synthetic emergency LDTs (corridor optic, wide anti-panic
  optic, our numbers) + a demo floor plan, pinned by tests, used on the
  landing page.

### Other regions (open, 2026-11-07; to check against the documents)

Emergency lighting differs by region like road lighting does. What we
know so far, from general knowledge, not yet checked against the texts:
- EU / EFTA (incl. CH as SN EN 1838): EN 1838 is the lighting part;
  around it: EN 50172 (systems, durations, tests), EN 60598-2-22
  (luminaires), EN ISO 7010 (signs), and national rules on where emergency
  lighting is required and for how long (DE workplace rule ASR A3.4/7,
  UK BS 5266-1, FR rules for buildings open to the public, which count
  lumens per m² rather than lux, AT TRVB E 102, CH the cantonal fire
  protection directives (VKF / AEAI)).
- International: ISO 30061 / CIE S 020 (the base EN 1838 follows).
- US: NFPA 101 (Life Safety Code, emergency lighting section) and the IBC
  (means of egress illumination): an *average* along the path of egress
  (about 1 fc ≈ 10.8 lx initially, at least 0.1 fc at any point, max / min
  40, allowed to fall over the 1.5 h duration), measured at the walking
  surface; equipment UL 924. That is a different rule shape (average +
  minimum along the path, a decline over the duration) from EN 1838's
  centre-line minimum.
- Others: AS/NZS 2293, China GB 51309, Canada NBC.

Design consequence: `Limits` (already loadable with `from_json`) would
become a profile with a rule shape per check (minimum / average / ratio,
end of duration), a profile choice in the Interior section and the report
naming the profile. Needs the user's limit tables per region (as for EN
1838), not copied standard text.

## 3. EN 17037: limit values (from `crates/eulumdat/data/en1838_en17037_limits.json`)

| Aspect | Minimum | Medium | High |
|---|---|---|---|
| Daylight provision: target E over 50 % of the reference plane / minimum target E over 95 %, for ≥ 50 % of the daylight hours | 300 / 100 lx | 500 / 300 lx | 750 / 500 lx |
| View out: horizontal sight angle, outside viewing distance, layers (sky, landscape, ground) | ≥ 14°, ≥ 10 m, 1 layer (landscape) | ≥ 28°, ≥ 20 m, 2 layers | ≥ 54°, ≥ 50 m, 3 layers |
| Exposure to sunlight on the reference day | ≥ 1.5 h | ≥ 3.0 h | ≥ 4.0 h |
| Protection from glare: DGP exceeded in < 5 % of the usage time | < 0.45 | < 0.40 | < 0.35 |

Method: climate-based modelling or daylight factor D relative to the median
diffuse external illuminance of the site; reference plane 0.85 m, 0.5 m
border off (as before).

To check against the standard text before the view-out code ships:
- Minimum outside viewing distance: the sheet says 10 m; I remember 6 m.
  The JSON value (10 m) is used until checked.
- Sunlight reference day: the sheet says "usually 21 March"; I remember a
  freely chosen day between 1 February and 21 March. The UI lets the user
  pick the day, default 21 March.

## 4. EN 17037: building on what we have

- `eulumdat-daylight`: solar position, CIE standard skies (overcast …),
  Perez all-weather sky, daylight availability, locations.
- `eulumdat::radiosity`: patch radiosity for rectangular rooms, already
  validated against the CIE 171:2006 interreflection cases.
- Light scene path tracer (GPU + CPU): any geometry, inter-reflections;
  needs a sky emitter and window openings.
- Interior designer rooms (rectangle / polygon). Missing: windows.

### New: windows and obstructions (shared foundation)

```text
Window { wall (edge index), offset, width, sill, head, tau (glazing),
         frame_factor, maintenance }
Obstruction = horizon profile per azimuth (simple: angle of the building
         opposite), or boxes outside
Site { latitude, longitude, north (room rotation), location name }
```

The same model will later come from IFC (IfcWindow, IfcSpace).

### Daylight factor (method 2) first

- Sky component: numeric integration of the CIE overcast sky luminance
  over the window solid angle from each grid point (× τ × frame × MF),
  externally reflected component from the obstruction (its luminance as a
  fraction of the sky), internally reflected component with the radiosity
  solver (window patches as diffuse emitters).
- Target D from E_v,d,med of the site: computed from an hourly weather
  file (EPW upload; or a PVGIS TMY fetch, which needs the user's OK per
  site) as the standard defines it; the standard's own location table only
  as a check.
- Output: DF map on the plan, fraction of the plane above the targets,
  level reached; PDF chapter.

### Then

1. Exposure to sunlight: solar positions for the chosen date, reference
   point per window, window azimuth range + obstruction horizon → hours,
   level. Cheap, deterministic, good first deliverable together with view
   out.
2. View out: geometry from room / window / eye positions + user inputs
   (outside distance, layers) → level.
3. Climate-based (method 1): daylight coefficients (contribution of each
   sky patch to each grid point, from radiosity or the tracer) × hourly
   Perez skies from the weather file → hours above target per point.
4. Glare: the simplified route first; DGP from the path tracer (fisheye
   at the eye position) later.

### Validation

- CIE 171:2006 natural-lighting cases derived analytically (sky component
  of an opening under uniform / CIE overcast sky: closed-form configuration
  factors), no table values copied.
- Average DF against the well-known split-flux formula as a plausibility
  band.
- An established daylight simulator only as a yardstick (like
  colour-science for TM-30; ask before downloading anything).

## 5. Order of work

| Phase | Content | Depends on |
|---|---|---|
| P0 | limit values: done (`crates/eulumdat/data/en1838_en17037_limits.json`); open: view-out distance, sunlight day | user |
| P0a | GLDF emitters / variants / emergency behaviour: core reader `eulumdat::gldf_product` done (tests: synthetic product.xml + Tria 2); synthetic demo GLDF and configurations in web projects open | – |
| P1 | done: `eulumdat::emergency` (rule table = the JSON, checked by a test), 11 tests, CLI `eulumdat emergency` (spacing table / GLDF variants / `--plan`) | P0a |
| P2 | interior done (section, plan tools, report, demo data, GLDF emitters, Light scene emergency mode + walk); open: high-risk areas in the UI, exterior routes | P1 |
| P3 | windows / obstructions / site in rooms; sunlight exposure + view out | P0 |
| P4 | daylight factor (method 2) + validation + report | P3 |
| P5 | climate-based (method 1), weather files, glare | P4 |
| P6 | IFC import feeding rooms, windows, doors / exits | P3 |

Strings: new `zonal_designer.emergency` / `.daylight` structs, 8 locales,
English defaults. Landing: one section per standard once P2 / P4 are done
(neutral wording, demo data only).

## 6. Open questions

- Editions: which EN 1838 edition? EN 17037 with A1? (The limit table is
  the same for the values above.)
- View-out minimum distance (10 m vs 6 m) and the sunlight reference day.
- National profile first (Germany: DIN VDE V 0108-100-1)?
- Weather data: EPW upload only, or also fetching a TMY (PVGIS) per site?
- Real emergency LDT / GLDF examples beyond the Tria 2 (kept out of git)?
