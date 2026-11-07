# Sensors: presence, motion and daylight detectors (plan, 2026-11-04)

Status: research done, nothing implemented yet. This plan replaces the open
ends of [`sensor-vs-luminaire.md`](sensor-vs-luminaire.md).

Tags used below: **[V]** = read in a source, **[I]** = our inference, **VERIFY** =
must be checked before it is relied on in code.

## 0. Summary

- A sensor is "same same, but different" from a luminaire. Placement, mounting
  height, rotation, plan views, grids, polygon masks, the path tracer's
  visibility tests, the area designer and the report are shared. What differs
  is the kernel and the output: instead of `E = I·cos/r²` (lux, added up),
  a sensor answers "is a person at this point detected, for this kind of
  motion?" (yes/no, or count of sensors), and the result is a coverage
  percentage, not lux.
- The real sensor data format is **SensXML** (Relux, sensNORM / IEC 63180):
  detection radii in metres, per motion type (tangential walking, radial
  walking, seated presence), per mounting height. GLDF carries it as
  `contentType="sensor/sensxml"`.
- `sensor/sensldt` is EULUMDAT for sensors, and is now decoded (section
  3.1a): the intensity matrix is the **detection volume in metres** (distance
  from the sensor to the volume boundary per C/γ), the direct-ratio block
  holds the tangential / radial / presence scale factors. A footprint is the
  volume cut at `mounting height − target height`. Implemented in
  `eulumdat::sensor` and `eulumdat sensor`.
- For daylight photosensors the luminaire analogy holds better: the angular
  response R(C, γ) is stored like an intensity table, and the signal is
  `S = ∫ L·R dΩ`. That needs radiance at a point, which our tracers almost
  have.
- Rough share already covered by existing cores: presence/motion coverage
  ≈ 80–90 %, CLI/web display ≈ 70 %, daylight photosensor control ≈ 60 %
  (the tracer exists, but the Light scene has no daylight yet).

## 1. Correction: our "sensor" fixture holds no sensor data

`crates/eulumdat/tests/fixtures/sensor.ldt` comes from the GLDF consortium's
SLV "Tria 2" demo. **[V]** Its header says `OCULUS CW / 1004664 /
1004664_OCULUS CW-259379-1.ldt`: it is the photometry of a different SLV
product (a wide-beam spot), copied into `sensor/` "for testing purpose" (2021
commit in github.com/globallightingdata/examples). In that old demo container
`ldc/wide.ldt` is the same OCULUS CW file, which is why
`sensor_vs_luminaire.rs` found the two "indistinguishable".

**[V]** On 2025-12-16 (commit 974d82d) the consortium replaced it with a real
SensXML 4.5 file (`sensor/sensor.xml`, `contentType="sensor/sensxml"`,
measured by Steinel Analytics, a Theben theRonda S360 ceiling PIR,
manufacturer anonymised as "The sensor company"). The gldf.io page still shows
the old `sensldt` version.

Consequences:
- The rule "never infer sensor vs. luminaire from LDT content" stays right.
  The reason changes: the file was never a sensor pattern.
- `SvgLabels::for_sensor()` ("Relative sensitivity, %") is fine as a display
  mode, but its doc comments and `tests/sensor_display.rs` call the fixture "a
  detection lobe". Fix the wording (Phase 0).

## 2. What exists today

| Piece | Where | Use for sensors |
|---|---|---|
| Sensor labels for diagrams | `eulumdat/src/diagram/svg.rs` `SvgLabels::for_sensor` | relative lobe display of `sensldt` and photosensor R(C, γ) |
| "Never infer" write-up + tests | `docs/sensor-vs-luminaire.md`, `tests/sensor_vs_luminaire.rs`, `tests/sensor_display.rs` | keep, correct wording |
| TM-32 BIM sensor fields | `eulumdat/src/atla/bim.rs` (`with_sensor`, `detector_type`, `detection_method`, `detection_area`, `presence_detection_area`, …) | fill from GLDF/SensXML on export |
| Placement + angle maths | `eulumdat/src/area/compute.rs` `LuminairePlace`, `compute_single_illuminance` (rotation/tilt → C, γ) | identical for sensors; split out the angle part |
| Polygon areas, masks, plan/iso SVG | `eulumdat/src/area/{polygon,svg,layout}.rs` | coverage map, sensor grid layouts |
| Cone diagram (field angle, diameter at height) | `eulumdat/src/diagram/cone.rs` | quick "cone model" sensor input |
| GLDF/L3D reader for the Light scene | `eulumdat-scene/src/l3d.rs` `gldf_extract`, `best_photometry` | **bug**, see Phase 0.1 |
| Path tracer probes + occlusion | `eulumdat-scene/src/cpu.rs` `probe_irradiance`, `occluded`; `trace.wgsl` `probe` | line of sight for PIR; photosensor probe |
| Measurement grids, overlay | `eulumdat-scene/src/measure.rs`, `rt_scene.rs` 2D overlay | footprint overlay |
| Daylight MC (sun, sky dome, glazing, plane detectors) | `eulumdat-goniosim/src/daylight.rs`, `illuminance.rs` | daylight signal at a photosensor |
| Spectra (V(λ), SPDs) | `eulumdat-spectrum` | photosensor spectral mismatch factor |
| GLDF sensor model + editor page | gldf-rs `general_definitions/sensors.rs`, `SensorEmitter`, `gldf-rs-wasm/.../pages/sensors.rs`, enums in `validation/xsd_enums.rs` | the editor side already exists; gldf-rs can depend on `eulumdat` and reuse our parser |
| L3D `SensorObject` | l3d-rs commit 3743ce0 | **local only, not pushed**; crates.io `l3d_rs 0.2.3` (used by `eulumdat-scene`) lacks it |

## 3. Research findings

### 3.1 How sensors are stored

**GLDF (rc.3, the latest public schema) [V]**
- `GeneralDefinitions/Files/File contentType="sensor/sensxml" | "sensor/sensldt"`,
  stored in the `sensor/` folder.
- `Sensors/Sensor id` → `SensorFileReference fileId`, optional
  `DetectorCharacteristics` (Round | Square | Other, ≤ 3),
  `DetectionMethods` (Passive Infrared | High Frequency | Microwave |
  Ultrasonic | Camera | Other, ≤ 6), `DetectorTypes` (Motion Detector |
  Presence Detector | Daylight Detector | Other, ≤ 4).
- `Emitters/Emitter/SensorEmitter` → `SensorReference sensorId`, optional
  `Name`, optional `Rotation X/Y/Z/G0` ("e.g. for wall or floor sensors, if
  not measured in position of use"). An `Emitter` is either light or sensor,
  never both.
- Variant geometry links the emitter to the L3D part by
  `EmitterReference/EmitterObjectExternalName` = L3D `partName`.
- `ControlGear/PowerConsumptionControls` (W, idle detector),
  `Electrical/SwitchingCapacity` (free string).
- rc.1 used `<Emitter><Sensor sensorId>`; accept it when reading.
- No range, height or motion type outside the sensor file.

**L3D `SensorObject` [V]**: `partName`, `Position` (m, relative to the parent
part), `Rotation` (degrees, order ZXY), Z up. No shape, unlike a
`LightEmittingObject`. **[I]** Zero rotation = looking down (−Z), like a LEO.
VERIFY with the consortium.

**SensXML 4.5 [V, from the real file; no public XSD]**

```
SensXML/Version 4.5
  Header: Manufacturer, ProductName, ProductNumber, ProductType, ProductTechnology,
          Laboratory, ReportNumber, ReportDate, UniqueIdentifier, …
  Sensor: TotalSystemPower (W), MountingType, Quantity, Dimensions L/W/H,
          SensorCenter offsets, Measurement (Type=sensNorm, Version, Equipment, Checksum)
  DetectionSource n=1..3
    DetectionType 1|2|3, NumberPolarData, NumberGridData, ActiveGrid,
    ShowRawData, SamplingDirection, RotationAngle
    PolarData n: NumberValues, AngularIncrement, StartAngle, EndAngle,
                 DistanceFromCenter (= mounting height, [I]),
                 RangeData angle="…" → radius in m
                 PlanningData: Shape, ScaleShape, DetectionRate (85), Radius, Start/EndAngle
    GridData n:  NumCellsX/Y, DimCellX/Y (0.5 m), DistanceFromCenter,
                 CellData x y → 0|1, PlanningData
```

Numbers from the sample (computed from the file):

| DetectionType | Height | Values | Radius min / mean / max | Planning radius |
|---|---|---|---|---|
| 1 | 2.5 m | 30 (starts at 60°) | 4.00 / 4.57 / 4.75 m | 1 (!) |
| 1 | 4.0 m | 36 | 4.50 / 6.56 / 7.50 m | 6 |
| 2 | 2.5 m | 30 (starts at 60°) | 2.19 / 3.59 / 4.76 m | 2.5 |
| 2 | 4.0 m | 36 | 4.25 / 5.82 / 7.41 m | 5 |
| 3 (grid) | 2.5 m | 94 of 225 cells on | area 23.5 m², Ø 5.5 m equivalent | 2.25 |

**[I]** Type 1 = tangential walking (always the largest), type 2 = radial
walking, type 3 = presence (seated, minor motion). PlanningData is the
idealised conservative shape (sensNORM idealises to circles/squares, allowing
≤ 15 % of triggers short of the line; `DetectionRate 85`). The parser must
tolerate: declared `NumberValues` ≠ values present, gaps (0–50° missing),
space-padded attributes (`angle=" 0.000"`), ISO-8859-1 encoding, and odd
planning values (radius 1 next to a measured 4.6 m).

**`sensor/sensldt`**: EULUMDAT for sensors, same layout. **Decoded
2026-11-04 from the ReluxNet corpus (section 3.1a):** the intensity matrix
is a **detection volume in metres**, the direct-ratio block is a parameter
block. Earlier ideas in this plan (relative sensitivity with an `S/d²`
threshold, binary masks from an unsourced spec summary) do not match the
files and are dropped. No written definition exists online (gldf.io, XSD,
templates: `<Sensor><!-- To be defined --></Sensor>`) [V]; Relux reads
"sensor data from EULUMDAT files" since ReluxDesktop 2024.1.8 [V].

The same paste sketched a SensXML layout (`<Sensor><Metadata>…<MountingHeight
Min/Max/Nominal>`, `<DetectionData><Tangential|Radial|Presence>` as X/Y
polygons, UTF-8). **It does not match the real 4.5 file** (root `<SensXML>`,
`DetectionSource`/`DetectionType`, polar radii + 0/1 grids, ISO-8859-1): none
of those elements occur in it. Follow the file. What the paste gets right:
three detection types in one file, several heights per type, interpolation
between heights, XSD not public.

**SensXML documentation**: said to be online, but not found by search;
relux.com refuses automated access (HTTP 403), gldf.io and sensNORM only
describe the measuring method. Get the spec / XSD by hand from Relux (or
from whoever has the link) and replace the inferred field meanings below.

### 3.1a ReluxNet sensor corpus (2026-11-04)

Scraped with light-other-rs `reluxnet` (`--section sensors`), stored outside
the repo in `/Volumes/tb4ssd/develeop/gldf-search-data/reluxnet-sensors{,-extracted}`.
9 manufacturers (Steinel 652 articles, B.E.G. 213, ESYLUX 114, Theben 87,
Kopp 16, Signify 9, inventronics 7, Relux demos 4): 1 102 `.rolfz`,
1 288 sensor LDTs, 36 SensXML files (`lvk/*.xml`, e.g. `sensnorm_test.xml`,
`iec_test.xml`). None of the 640 378 GLDF files on the volume carries a sensor.

Per article Relux stores its own **analysis** of each sensor file
(`article.analysis.<file id>`): `maximum_range` (m), `type_of_entry`
(1 = Presence, 2 = Motion, 3 = Tangential), `sensor_type` (1 = PIR, 2 = HF,
3 = LF), `sensor_characteristics` (1 circular 300–360°, 2 semi-circular
145–300°, 3 sector < 145°, 4 stretched). Counts: entries Motion 298,
Presence + Motion 250; PIR 496, HF 52; circular 356, semi-circular 94,
stretched 92, sector 6.

Relux renders every lvk file at
`https://reluxnet.relux.com/assets/cache/product_images/{status_name}/lvk_png/resized/{file}.png`
(220 × 220 px only). 213 renders are plan views in metres (sensor at the
centre, yellow / green / blue curves, e.g. B.E.G. PD4: ≈ 10.7 / 3.6 / 2.6 m);
149 are ordinary polar diagrams in cd/klm (files without analysis, or renders
cached before Relux recognised the file as a sensor). These are the reference
images for our own sensor plan view.

**What a sensor LDT holds (decoded; implemented in `eulumdat::sensor`):**
- **Matrix = detection volume.** The value at (C, γ) is the distance in metres
  from the sensor to the boundary of the volume in which a person is detected
  (γ = 0 down, C0 = +x). Proof: Steinel Dual US `seb_008420` has C0 values
  exactly `4.25/cos γ` up to 19°, then `1.5/sin γ`; C90 `4.25/cos γ` up to a
  10 m limit, i.e. a box 4.25 m deep × 3 m × 20 m; Relux's `maximum_range`
  10.098 m is its corner √(10² + 1.5²) on the cut plane.
- **Footprint = cut of the volume** with the plane `depth = H − z` below the
  sensor: a point at horizontal distance ρ under γ is inside while the
  boundary's vertical depth `r(C, γ)·cos γ ≥ depth`.
- **Relux reference depth ≈ 2.0 m**: the tangential reach at 2.0 m below the
  sensor reproduces Relux's `maximum_range` for most ceiling sensors (B.E.G.
  PD4 10.67 vs 10.71, Dual US 10.10 vs 10.10, IS 3360 19.88 vs 20.16); the
  outer curve of the plan renders fits best at 1.95–2.2 m. Wall sensors fit
  shallower (B.E.G. indoor180 ≈ 1.0–1.2 m).
- **Parameter block (lines 33–42)**: field 2 = tangential scale (mostly 1,
  ESYLUX 0.46–2.512), field 1 = radial scale, field 3 = presence scale (0 = no
  presence detection, exactly when Relux lists no Presence entry), field 6 =
  1 for presence detectors / 0 for motion detectors of a family, field 8 =
  1 wall / 2 ceiling / 3 other, field 9 = 0 or an angle (20…270°).
  Radial / presence area = the volume scaled by its field, cut at the same
  plane (rule A). **Confirmed on 165 renders** (image scale from Relux's
  `maximum_range`, centre fixed at the render centre): outer shape within
  5 % in 99 %, radial in 94 %, presence in 90 % (median errors 1.9 / 2.7 /
  4.2 %); "scale the outer outline" (rule B) fails (≈ 50 %). The depth that
  reproduces `maximum_range`: ceiling 2.00 m (median), mounting type 3
  1.95 m, wall 1.75 m. Only outlier: B.E.G. PD4 (render probably cached
  from an older file; PD2 / PD11 of the same family follow rule A). Field 8
  uses the SensXML `MountingType` coding (1 wall, 2 ceiling, 3 wall/ceiling,
  per comments in Signify files). Still unknown: fields 4, 5, 7, 9, 10.
- Lamp block, flux and LORL carry no meaning for the detection area (flux 1,
  600 or 1000; "LORL" 0.5–11 %).
- Parsing: 1 287 of 1 288 corpus LDTs parse after making empty value fields
  read as 0 (blank lines inside the matrix = missing value; thePrema files
  leave the width empty). The one failure is a Steinel luminaire LDT with an
  extra header line.
- Section 1 needs another look: the old SLV `sensor.ldt` has values ≈ 480,
  which cannot be metres; it is a luminaire file after all (or a different
  convention), so the "placeholder" correction stands.

**SensXML in the corpus**: 22 unique files (36 copies) from Signify (7),
Osram / inventronics (7), Steinel lab files, Kopp, ESYLUX (SensCalc export),
Relux demos; all root `SensXML`, versions 3.9 / 4.0 / 4.4 / 4.5 / "0",
20 × ISO-8859-1, 2 × UTF-8; labs METAS and Steinel Analytics; measurement
types "IEC 63180:2020 (automated)", "sensNorm", "IEC_A1 and sensNorm 3.0".
New nodes beyond the first sample: `PlanningData/Length|Width|Polygon/Vertex`,
`Shape` 1–4, `TestData` (dummy temperatures 28/35 °C, `ScaleDummy` 0.5/1/2/5),
`TestConditionData` (ambient 21/25 °C, humidity 45 %), `NumberPolarData` up
to 6 heights. Inventory script and output: scratch, to be turned into
`eulumdat sensor inventory` + `docs/sensxml-observed.md` (section 5.6).
No SensXML files elsewhere online (GitHub code search, manufacturer sites).
The two GLDFs with sensors on ReluxNet (Signify MPS32xx / SC200B,
"19-GLDF-Sensor_only") need a Relux login to download.

**Implemented (2026-11-04)**: `eulumdat::sensor` (`DetectionVolume`,
`Footprint`, `Motion`, `SensorParams`, `SensorPlanSvg`, `RELUX_REFERENCE_DEPTH`)
and `eulumdat sensor <file.ldt> [--depth D | --height H --target z] [--svg]`,
which prints tangential / radial / presence max range, area and reach and
draws a Relux-style plan view. `eulumdat::sensor::sensxml::SensXml` (feature
`xml`): tolerant reader for all 36 corpus files (decoding, extras,
warnings), footprints per motion type and mounting height from measured
polar / grid data or planning shapes (circle, rectangle, polygon), linear
between heights, ∝ height below the lowest; `eulumdat sensor file.xml
--height H [--planning]`. Relux's SensXML plan view = planning shapes at
2.0 m, reproduced to ±0.02 m (KOOP PD-360-12m, Osram OCI). Corpus test:
`SENSXML_CORPUS=<dir> cargo test -p eulumdat sensxml::tests::corpus`. Scrape tooling in light-other-rs `reluxnet`:
`--section sensors`, examples `extract_rolfz`, `fetch_lvk_png`, `probe_page`,
`fetch_url`.

### 3.2 Detection areas: standards

**IEC 63180:2020 + A1:2025 (ed. 1.1) [V]**: PIR only. Defines major motion
(radial = towards the detector, tangential = across it), minor motion (e.g.
seated arm movements; "presence detector" is a non-standard name for a
minor-motion detector), detection area, non-detection area, delay time, dead
time. Test person 1.7 ± 0.1 m, 70 ± 10 kg, surface 7 ± 2 K above ambient;
room 18–23 °C; dummies at full, 1:2, 1:5 scale; a test arm for minor motion.
Results: polar diagrams of the radial and tangential areas, a minor-motion
area, and a 3D data model (§8.4). A1 adds recommended test mounting heights
and changes the tangential boundary offset from +10° to +5°. The method comes
from **sensNORM** (industry association, 2014, initiated by Relux): detector
rotated in 10° steps (36 directions), automated dummy walks, test arm swung
over a grid, raw results idealised to circles or squares.

**NEMA WD 7-2011 (R2016) [V]**: PIR, ultrasonic, microwave; 3 × 3 ft grid;
walker 67 in at 4 ft/s; minor motion = robotic arm at 36 in (0.91 m) sweeping
90°/s; output = grid map marked "major" / "major + minor". No performance
levels.

**Reference planes [V]**: seated presence ≈ desk height 0.8 m (Theben), Relux
shows presence at 0.75 m, NEMA minor motion at 0.91 m. Walking = floor.

### 3.3 Datasheet numbers and how areas scale with height

Theben theRonda P360, diameters in m [V]:

| Height | tangential | radial | seated |
|---|---|---|---|
| 2.0 m | 22 | 6 | 4.5 |
| 2.5 m | 23 | 7 | 5.5 |
| 3.0 m | 24 | 8 | 6.0 |
| 3.5 m | 24 | 8 | 7.0 |
| 4–6 m | 24 | 8 | – |
| 10 m | 25 | 8 | – |

- Walking areas stop growing above ≈ 3 m: the range limit takes over from the
  lens cone. Seated presence is not published above a cut-off height
  (3.5 m here).
- Niko PXXLR seated: Ø 8 / 5 / 6 m at 2.5 / 3.0 / 3.5 m [V], not even
  monotonic. So: **interpolate measured tables, do not extrapolate a cone.**
- Other examples [V, partly search excerpts]: B.E.G. PD4-M at 2.5 m: seated
  6.4, t 24, r 8 m; ESYLUX PD-C360i/24 at 3 m: t 24, r 11, presence 8 m;
  Theben thePassa corridor: t 16 × 3.5 → 30 × 5 m from 2.0 to ≥ 3.5 m.
- Vendor letters (A/B/C/D) are not consistent across vendors; map to motion
  type by name.

### 3.4 Physics per technology

- **PIR [V]**: Fresnel lenslets form discrete zones; the pyro element reacts to
  change, so a still person fades out. Tangential motion crosses more zones,
  so it reaches farther. Signal ∝ 1/d², temperature difference and speed.
  Needs line of sight: blocked by partitions, shelving, pendant luminaires,
  plants. Range falls as room temperature rises (datasheets quote 17–18 °C).
  Above ≈ 3.5 m the gaps between zones grow.
- **HF / microwave (5–60 GHz) [V]**: passes glass, wood and lightweight
  walls, so it triggers on neighbouring rooms; metal reflects. **[I]** A cone
  is the wrong model: use a range sphere and flag thin walls instead of
  blocking.
- **Ultrasonic (32–40 kHz) [V]**: fills the room, no line of sight needed,
  stops at walls.
- **Camera [V/I]**: frustum, e.g. Steinel HPD2 110° opening, 2.3–6 m.
  Footprint = `2·(h − z)·tan(FOV/2)`.

### 3.5 Planning rules [V]

- One detector per desk island, placed above the workstations; desks must lie
  in the seated (minor-motion) area.
- Overlap footprints, ≈ 20–30 % (sources differ), and from 3.5–4 m mounting
  height always.
- Corridors: rectangular patterns; if people enter from the end, plan with the
  radial range. Sports halls: tangential range.
- Combined presence + light sensors measure daylight under the sensor; one
  light measurement per control group.

### 3.6 Energy and codes

**EN 15193-1:2017+A1:2021, Annex E [V]**, occupancy dependency factor:

```
F_O = 1 − (1 − F_OC)·F_A / 0.2          for 0   ≤ F_A < 0.2
F_O = F_OC + 0.2 − F_A                   for 0.2 ≤ F_A < 0.9
F_O = (7 − 10·F_OC)·(F_A − 1)            for 0.9 ≤ F_A ≤ 1
```

(Equal to the `min` of the three lines for F_OC ∈ [0.8, 1].)

| Control | F_OC |
|---|---|
| manual on/off | 1.00 |
| manual + automatic sweep-off | 0.95 |
| auto on / dimmed | 0.95 |
| auto on / auto off | 0.90 |
| manual on / dimmed | 0.90 |
| manual on / auto off | 0.80 |

| Space | F_A |
|---|---|
| office building, overall | 0.20 |
| cellular office, 1 person | 0.40 |
| cellular office, 2–6 persons | 0.30 |
| open plan, one sensor per 30 m² | 0.00 |
| open plan, one sensor per 10 m² | 0.20 |
| office corridor (dimmed) | 0.40 |
| conference room | 0.50 |
| classroom | 0.25 |
| toilet | 0.90 |
| car park, public | 0.80 |

F_O < 1 only if: off or ≤ 20 % within 15 min of the last presence; the group
switched together is ≤ 30 m² (meeting rooms excepted); the detector's coverage
closely matches the area lit by the luminaires it controls. The last
condition is exactly what our coverage engine can check. Open-plan F_A
depends on m² per sensor, which also comes out of the coverage engine.

LENI [V]: `W = Σ (P_N·F_C)·F_O·(t_D·F_D + t_N) / 1000`,
`F_D = 1 − F_D,S·F_D,C`; quick method adds 1 kWh/m²a (emergency standby)
and 1.5 kWh/m²a (control standby). The F_D,C tables are paywalled (not read).

**ASHRAE 90.1-2022 [V]**: time-off ≤ 20 min (15 min exterior); partial-on ≤
50 %; partial-off ≥ 50 % (30 % in garages); open offices: zones ≤ 600 ft²,
after 20 min vacancy −80 % or off; daylight dimming to ≤ 20 % and off;
sidelit primary zone depth = 1 × window-head height, secondary = 2 ×.

**Title 24 2022 [V]**: off/partial-off ≤ 20 min; offices > 250 ft² multi-zone,
zones ≤ 600 ft²; daylight controls in zones ≥ 120 W; outdoor ≤ 24 ft mounting:
−50…90 % within 15 min. The 2025 code applies from 2026-01-01: VERIFY changes.

### 3.7 DALI-2 (IEC 62386-303 / -304) [V]

Occupancy states 0x00 vacant, 0xAA occupied without movement, 0xFF occupied
with movement. Hold timer default 15 min (10 s steps, max 42 min 20 s),
report timer default 20 s, dead time default 100 ms. Light sensor: percentage
hysteresis plus an absolute minimum. Only the hold time matters for energy
modelling.

### 3.8 Daylight photosensors

- **Signal [V]**: `S = ∫ L(ω)·R(ω) dΩ`. R multiplies radiance; R = cos γ
  makes S the illuminance (Ward, rsensor). The LRC calls spatial response
  "analogous to a luminaire intensity distribution, but describes sensitivity".
  **[I]** So store R like I(C, γ): γ = 0 on the sensor axis (nadir for a
  ceiling sensor), C0 = the housing's reference mark, cosine included, peak =
  1, so a cosine sensor reads lux. γ up to 180° covers protruding lenses.
- **Formats [V]**: no standard. De facto: the SPOT/rsensor tab matrix (first
  row `degrees` + azimuths from 0 to ≥ 270°, then one row per polar angle;
  angles past the last row are 0). Daysim's `.sen` / XML schema not found.
- **Measured shapes [V]**: Watt Stopper LS-201 with lens ≈ 9° half-response;
  without lens ≈ 30°. Rubinstein used cosine, partly shielded, 30° tube,
  15° window-aimed tube. NLPIP: "narrow" and "wide", some asymmetric (wide in
  C0, narrow in C90). Philips SNS400 (luminaire-integrated): 40° field of view
  ([I] at 2.7 m it sees a disc of ≈ 1 m radius, i.e. its own luminaire's
  footprint).
- **Control algorithms [V]** (Rubinstein, Ward & Verderber 1988). Dimming
  level δ ∈ [δ_min, 1]; sensor `S_T = S_D + δ·S_E`; task `E_T = E_D + δ·E_E`
  (S_E, E_E = electric only at δ = 1; M < 0):
  - integral reset: hold `S_T = S_E` → `E_T = E_D + E_E·(1 − S_D/S_E)`;
    meets target only if `E_D/S_D = E_E/S_E`.
  - open-loop proportional: `δ = M·S_D + 1`.
  - closed-loop proportional: `δ = M·(S_T − S_E) + 1`
    → `δ = [1 + M·(S_D − S_E)] / (1 − M·S_E)`.
  - daytime calibration at t_cal: `δ_cal = (E_target − E_D)/E_E`,
    `S_cal = S_D + δ_cal·S_E`, `M = (1 − δ_cal)/(S_E − S_cal)`.
  - reference ("ideal") control as in EnergyPlus:
    `f_L = max(0, (E_set − E_D)/E_set)`, with min-power / stepped variants.
  - **[I]** Transport is linear, so trace S_D and E_D per sky state, S_E and
    E_E once per zone; every algorithm is then closed form, no feedback loop.
- **Why it matters [V]**: task/sensor ratio differs between daylight and
  electric light (Oakland Federal Building: 244 vs 585 lx/V, factor 2.4;
  simulation within 6.7 %). Night-calibrated integral reset over-dims badly
  ([I] ≈ 42 % of target in that case). SPOT example: product-specific sensor
  30 % vs generic cosine 58 % annual light output in the same zone.
- **Spectral [V]**: f1′ and mismatch factor
  `F = [∫S_C·s·∫S_Z·V] / [∫S_C·V·∫S_Z·s]`; measured differences of 36–118 %
  over 5 sensors × 16 glazings (Doulos 2008). **[I]** Cheap to add with
  `eulumdat-spectrum`: one scalar per source (sun, sky, each LED SPD, each
  glazing τ(λ)). Relevant for tinted glass and the greenhouse shade net.
- **Outputs studies show [V]**: scatter of optimum dimming vs sensor signal;
  workplane illuminance over time (optimum vs achieved); fraction of time
  below target; monthly energy (base, optimum, algorithm); over-dimming %.

### 3.9 What other tools do

- **Relux [V]**: reads SensXML (also from GLDF since 2024.1.6) and sensors from
  LDT; draws presence / tangential / radial areas in 2D/3D, calculation on a
  reference plane, false colours, grouped sensors. **SensCalc** (2025, free
  online, IEC 63180 + sensNORM): sensor count, positions, payback;
  manufacturers asked to supply GLDF.
- **DIALux evo [V]**: energy per EN 15193 with presence-control type as a
  setting; evo 11 adds one daylight "sensor measuring point" per room and
  control groups. No presence-detector objects or GLDF sensor import found
  ([I] no detection-area planning).
- **Daysim / DAYSIMps, SPOT, LBNL psens [V]**: photosensor angular response,
  closed-loop/integral-reset/open-loop algorithms, calibration point, energy.
  DAYSIMps is no longer developed. **Honeybee, EnergyPlus [V]**: sensors are
  cosine points only.
- **[I]** Open niche for us: a browser tool that reads GLDF/SensXML, shows
  detection areas, checks coverage and desks, gives F_O, and later simulates a
  real photosensor response, all client-side.

## 4. Same same, but different

| | Luminaire | Presence / motion sensor | Daylight photosensor |
|---|---|---|---|
| Data | I(C, γ) in cd/klm | SensXML: radius(azimuth) in m per motion type per height; sensldt: slant range r(C, γ) in m (the detection volume) | R(C, γ), relative, peak 1 |
| Kernel at a point | `E = I·cos/r²` | inside footprint? SensXML: interpolate radius; sensldt: `√(ρ²+D²) ≤ k·r(C, γ)` (+ line of sight for PIR) | `S = ∫ L·R dΩ` at the sensor |
| Combine several | sum of lux | count / any | per control zone |
| Reference plane | workplane / road | floor (walking), ≈ 0.8 m (seated) | none, sensor sees the room |
| Height dependence | inverse square, exact | measured table, interpolate | exact (tracer) |
| Result | Em, Emin, Uo | % covered, gaps, overlaps, desks covered, F_O | dimming over time, energy, under/over-dimming |
| Diagrams | polar, cartesian, isolux | footprint plan, height table | polar of R (sensor labels) |

## 5. Design

### 5.1 Data types (core crate, feature `sensor` = quick-xml, no new deps)

```rust
// crates/eulumdat/src/sensor/mod.rs
pub enum Motion { Tangential, Radial, Presence }            // IEC 63180 terms
pub enum Technology { Pir, HighFrequency, Ultrasonic, Camera, Light, Other } // GLDF DetectionMethod
pub enum DataSource { SensXml, Datasheet, Cone, SensLdt }    // shown as a badge

pub struct Sensor {
    pub header: SensorHeader,          // manufacturer, product, number, lab, report, date, uuid
    pub technology: Technology,
    pub system_power_w: Option<f64>,
    pub patterns: Vec<Pattern>,        // one per Motion present in the data
    pub source: DataSource,
    pub warnings: Vec<String>,         // gaps, odd planning radius, extrapolation …
}
pub struct Pattern { pub motion: Motion, pub samples: Vec<HeightSample> } // sorted by height
pub struct HeightSample {
    pub mounting_height: f64,          // SensXML DistanceFromCenter (VERIFY meaning)
    pub shape: Shape,
    pub planning: Option<Planning>,    // idealised shape + DetectionRate %
}
pub enum Shape {
    Polar { start_deg: f64, step_deg: f64, radius_m: Vec<Option<f64>> }, // gaps = None
    Grid { nx: usize, ny: usize, cell: (f64, f64), origin: (f64, f64), on: Vec<bool> },
    Circle { radius_m: f64 },
    Rect { length_m: f64, width_m: f64 },
}
pub struct Footprint {
    pub outline: Vec<(f64, f64)>,      // sensor-local metres, C0 = +X
    pub area_m2: f64,
    pub equivalent_diameter_m: f64,
    pub quality: Quality,              // Measured | Interpolated | Extrapolated
}
impl Sensor {
    pub fn footprint(&self, m: Motion, mounting_height: f64, use_: Use /* Raw | Planning */) -> Option<Footprint>;
    pub fn detects(&self, m: Motion, mounting_height: f64, dx: f64, dy: f64) -> bool;
}
```

Inputs, all producing the same `Sensor`:
1. `sensxml::parse(bytes)` (ISO-8859-1 → UTF-8 by byte mapping, tolerant reader).
2. `Sensor::from_datasheet(table)`: diameters (or L × W) per height per motion,
   the format every datasheet prints; serde JSON/TOML so users and tests can
   type them in.
3. `Sensor::cone(half_angle_deg, max_range_m, motion)`: quick what-if.
4. `sensldt`: parsed by the existing LDT parser into an `Eulumdat` with a
   caller-set kind (from the GLDF `contentType` or the Relux article type,
   never guessed) and wrapped as `DetectionVolume` (done): values = slant
   range in metres, footprint = cut at `depth = H − z`, motion types =
   volume × parameter-block field 2 / 1 / 3. Angular diagrams can still use
   `SvgLabels::for_sensor()` (label should become "range, m").

### 5.2 Footprint at a given mounting height

- Grids are converted to a radius per 10° sector (farthest "on" cell) so grid
  and polar data interpolate the same way; the raw grid is still drawn exactly
  at its measured height.
- Between two measured heights: linear in height, per azimuth.
- Below the lowest measured height: scale radius ∝ height (lens cone), mark
  `Extrapolated`. Above the highest: hold the last radius (range limit), mark
  `Extrapolated`. Above a presence cut-off nothing is extrapolated upward
  without a warning.
- Gaps in polar data: circular linear interpolation between neighbours,
  warning listed.
- `Use::Planning` (default for coverage: conservative, what planners expect)
  vs `Use::Raw` (measured outline, drawn on top). If the planning radius is
  < 50 % of the smallest raw radius (sample: 1 m vs 4 m), warn and fall back
  to raw.
- SensXML azimuth 0 and direction: VERIFY (`SamplingDirection`,
  `RotationAngle`). Until then 0° = sensor C0 = +X, counter-clockwise, like LDT
  C-planes. Most PIR patterns are near-round, so the error is small.

### 5.3 Coverage engine (`sensor/coverage.rs`)

- Sensor placement reuses `LuminairePlace` (x, y, mounting height, rotation).
  A luminaire-integrated sensor = luminaire place + L3D `SensorObject` offset
  rotated with the luminaire (+ `SensorEmitter/Rotation`).
- Split `compute_single_illuminance` into a shared `local_angles(dx, dy, h,
  tilt, rot) → (c, γ, r)` plus the lux kernel; the sensor kernel uses the same
  transform (wall sensors: tilt).
- `coverage(sensors, &AreaPolygon, motion, resolution) → CoverageResult`:
  count grid (u8), covered fraction, overlap fraction, uncovered cells (and
  their outlines via the existing contour code), m² per sensor.
- `desk_coverage(rects)`: each desk fully inside a presence footprint?
- `en15193::f_o(f_oc, f_a)`, the F_OC and F_A tables, and a conditions check
  (hold time ≤ 15 min, group ≤ 30 m², coverage vs lit area).
- SVG: footprint plan (three nested areas, colours per motion type, height
  table beside it), coverage map (count colours, gaps hatched). Reuses
  `AreaSvg` plan view.

### 5.4 Line of sight in the Light scene

- Sensor position from the L3D body (`SensorObject`, Z up → scene Y up, the
  same transform as the body mesh) or free placement.
- For each grid point inside the footprint: `CpuTracer::occluded(sensor,
  target)` with target height ≈ 1.0 m for walking (torso of the 1.7 m test
  person, [I], setting) and ≈ 0.8 m for seated. A few thousand shadow rays,
  CPU is enough, no WGSL change.
- PIR / camera: occlusion blocks. Ultrasonic: room walls block, furniture
  does not. HF: nothing blocks, walls are only flagged.
- Draw on the existing 2D overlay canvas: footprint outline, shadowed
  (blocked) cells, the mannequin (`scene::figure`) marked detected / not.

### 5.5 Daylight photosensor

- R(C, γ) as an `Eulumdat` (caller-set kind `Photosensor`) or imported from
  the SPOT/rsensor matrix (small text parser). Bilinear sampling, like
  `ldt.sample`.
- Signal by path tracing: a new probe mode next to `probe_irradiance`: sample
  directions with pdf ∝ R (CDF over the table; goniosim already samples LDT
  tables), next-event estimation weighted by R(direction) instead of cos.
  Returns S in "lux-equivalent" units.
- Daylight: the Light scene has no sky in rooms (its `sky()` is an urban night
  sky, the office window band is dark). Two routes:
  - **A (first):** goniosim already has `Sun`, `SkyDome`, glazing sheets and
    plane detectors. Add a small one-sided disc `PointSensor` detector that
    weights each photon crossing by `R(dir)/cos θ` (the Daysim trick turned
    round) → S_D per sky state, and E_D on the workplane grid.
  - **B (later):** sky + sun + glass in the Light scene tracer, so the office
    shows daylight visually as well.
- Control: linear decomposition (S_D, E_D per sky state; S_E, E_E per zone at
  full output), then the closed forms from 3.8 for integral reset,
  open-loop and closed-loop proportional, plus the ideal reference. Night or
  daytime calibration selectable.
- Year: hourly sky states from `eulumdat-daylight` (solar position, Perez
  clear / CIE overcast; no weather files yet) → achieved workplane
  illuminance, % of hours below target, over-dimming %, energy, a simulated
  F_D to put next to EN 15193-1.
- Spectral mismatch (optional): one factor per source from
  `eulumdat-spectrum` (s(λ) of the sensor, SPD of sun/sky/LED, τ(λ) of the
  glazing).

### 5.6 Reading formats without a public schema (SensXML, sensldt)

The SensXML XSD is not public, so the reader is built by observation: collect
files that the proprietary tools (Relux, manufacturers' exports) produce and
accept, record exactly which nodes, enums and value types occur, write the
structs to match, and widen them when a file that works in Relux breaks our
reader.

- **Corpus, outside the repo.** Vendor files carry no licence, so they live in
  a local folder (e.g. `~/Lichtdaten/sensors/`), never in git. Sources: GLDF
  files with sensors (the consortium's Tria 2 example), Relux database /
  ReluxNet sensor downloads, manufacturer download pages (Theben, Steinel,
  B.E.G., ESYLUX, Niko, Swisslux). Also check Relux `.rolfz` containers
  (ESYLUX offers them): they may contain SensXML.
- **Inventory tool.** `eulumdat sensor inventory <dir>` walks every `.xml`,
  `.gldf`, `.rolfz` and sensor `.ldt` and prints per element path: count of
  files, occurrences, value type (int / float / text / empty), min / max,
  distinct values when there are few (these are the enums), attributes, and
  encoding / declaration variants. For sensldt: binary mask vs graded, value
  range, symmetry, what the lamp block holds.
- **Observed schema in the repo.** The inventory output (structure and
  statistics only, no vendor data) goes into `docs/sensxml-observed.md`. It is
  our de-facto schema and records which corpus file first showed each node.
- **Tolerant reader, two layers.**
  1. Generic: XML → element tree with paths. Handles ISO-8859-1, Windows-1252,
     UTF-8 with or without BOM, space-padded numbers, decimal commas.
  2. Typed mapping: every field `Option`; enums as integers with an
     `Unknown(i32)` variant; declared counts (`NumberValues`,
     `NumberPolarData`) are hints, never trusted; unknown elements are kept in
     `extras` and listed as warnings, never errors. Parsing never panics.
- **Every edge case becomes a test.** When a corpus file breaks the reader,
  write a small synthetic fixture with the same shape (our own numbers),
  commit that, widen the structs. The vendor file stays outside the repo.
- **Corpus test.** `SENSXML_CORPUS=<dir> cargo test -p eulumdat --features
  sensor sensxml_corpus` parses every file and checks sanity: no panic,
  heights 1.5–20 m, radii between 0 and 50 m, at least one detection type.
  Skipped when the variable is not set (as `TM25RAY_FIXTURE`).
- **Cross-check against the real tool.** For a few products, compare our
  footprint diameters with the datasheet and with what Relux shows for the
  same file; keep the table in `docs/sensxml-observed.md`.
- **Read-only first.** gldf-rs passes sensor files through as bytes. Writing
  SensXML only once the observed schema is stable, and then only with the
  `extras` written back, so nothing is lost.

## 6. Work plan

Sizes: S ≈ 1 day, M ≈ 2–4 days, L ≈ 1–2 weeks.

### Phase 0: fixes and facts (S), the low-hanging fruit

1. **Done (2026-11-04): GLDF loader honours `contentType`.** `gldf_file_types`
   reads the `<File>` list, `GldfExtract::sensors` holds the sensor files; the
   real Tria 2 container now loads `ldc/narrow.ldt` and sets
   `sensor/sensor.ldt` aside. Original task: `eulumdat-scene/src/l3d.rs`:
   read `product.xml` `<File id contentType>` (small quick-xml scan, no gldf-rs
   dependency); only `ldc/*` files become photometries; `sensor/*` files go to
   a new `GldfExtract::sensors`. Today `best_photometry` can pick
   `sensor/sensor.ldt` as the luminaire. Test: synthetic GLDF where the sensor
   LDT has the highest flux.
2. Correct `docs/sensor-vs-luminaire.md`, the comments in
   `tests/sensor_vs_luminaire.rs`, `tests/sensor_display.rs` and
   `SvgLabels::for_sensor` (fixture = placeholder luminaire, section 1).
3. l3d-rs: push 3743ce0 (`SensorObject`), publish 0.2.4, bump `eulumdat-scene`.
4. Start the corpus (section 5.6): local folder outside the repo with the
   Tria 2 SensXML, the Esylux sensor LDT and whatever else can be downloaded.
   Write a synthetic SensXML fixture (our own numbers, the structure of the
   sample). The real files come without a licence: never commit them; test
   against them via `SENSXML_CORPUS` (as `TM25RAY_FIXTURE`).

### Phase 1: sensor core (M)

`crates/eulumdat/src/sensor/{mod,xmltree,sensxml,datasheet,footprint,coverage,en15193,svg}.rs`,
plus `eulumdat sensor inventory` and the first `docs/sensxml-observed.md`
(section 5.6) before the typed structs are fixed.
Tests: parser (gaps, padding, Latin-1, count mismatch), interpolation and
extrapolation flags, theRonda P360 datasheet round trip, sensldt footprint
(box volume → exact rectangle: done; corpus files vs Relux `maximum_range`
and plan renders via an env-var corpus path), coverage of a 6 × 4 m
office with 1 vs 2 sensors, F_O against the table values (e.g. F_OC 0.9,
F_A 0.5 → 0.6), SVG snapshot.

### Phase 2: CLI and web display (M)

**Done (2026-11-04):** `eulumdat diagram --sensor` / `-t sensor-plan`;
shared `sensor::diagrams` (plan, polar, cartesian, heatmap, 3D with range
wording, no flux scaling); web "Sensors" tab with GLDF sensors, dropped
SensXML and "loaded LDT as sensor" (lossless, not the ATLA round trip),
deep links, dashboard card, 8 locales. Checked in a headless browser
(ESYLUX `ep10427459`: 10.65 / 4.91 / 3.91 m as in the CLI; Steinel Dual US:
the 3 × 20 m rectangle; Steinel IR Quattro HD SensXML). Remaining: the
landing page entry, sensor layer in the area designer (Phase 3).
**Done (2026-11-05):** SensXML → detection volume (all diagram types for
SensXML), depth limits + "not detected" instead of 0.00 m, Typst sensor
report + PDF export (CLI and web), neutral wording (reference height).

Demo sensors for links and posts (synthetic, committed):
`tests/fixtures/sensors/` + "Load demo sensors" / `#tab=sensor&sensor=demo`:
2 sensor LDTs + 4 SensXML (ceiling, wall 180°, high-bay 6–14 m, corridor).

- CLI: `eulumdat sensor <file.xml|file.gldf|datasheet.toml> [--height 3]
  [--motion all|tangential|radial|presence] [--svg out.svg] [--json]` prints
  Ø per motion per height, source badge, warnings.
- Web: GLDF open keeps sensors in an app context (like the L3D body); `.xml`
  dispatch sniffs the root element (`SensXML` vs ATLA). New "Sensor" tab: plan
  of the footprints with a height slider (2–6 m), motion toggles, raw vs
  planning, the height table, header (lab, report, power). `sensldt` files:
  diagrams in sensor mode plus the "not defined" note.
- Per project rules: tab bar + dashboard `FeatureGrid` card + guide, `#tab=sensor`
  and `height=` deep links, i18n `ui.sensor` in 8 locales (`serde(default)`),
  landing page entry via `gen_landing.py`.

### Phase 3: planning (M–L)

- Area designer: sensor layer (grid / one per luminaire / manual), coverage
  map per motion type, % covered, gaps, overlaps, desks, m² per sensor, F_O
  with the conditions check; ASHRAE / Title 24 checklist (time delay, zone
  size).
  **Done (2026-11-06):** Interior designer (automatic / grid / one per
  luminaire, coverage, gaps, overlaps, m² per sensor, EN 15193-1 F_O with
  the conditions, PDF section) and the outdoor area designer ("Outdoor"
  tab: one per pole / grid / automatic, coverage on the ground at one
  sensor height, polygon areas through `coverage_in`, no energy estimate).
  Manual placement = dragging luminaires, poles and sensors on the plan
  (`components/plan_drag.rs`, link keys `lmove` / `smove`). Open: desks,
  ASHRAE / Title 24 checklist.
- Light scene: sensor markers, footprint overlay, line-of-sight shadowing,
  mannequin detected / not. Street / parking: outdoor sensors (Title 24
  outdoor rule).
- Report: a sensor page in the PDF.
- BIM export: fill the TM-32 sensor fields from the sensor.

### Phase 4: daylight photosensor (L)

R(C, γ) input (LDT + rsensor matrix), goniosim `PointSensor`, linear
decomposition, the four control algorithms, calibration, hourly year,
outputs from 3.8, spectral factor. Later route B (daylight in the Light
scene).

### Phase 5: other platforms and gldf-rs (M)

FFI (`parse_sensxml`, `footprint`, `coverage`), Apple / Android sensor view,
gldf-rs-wasm sensors page draws footprints through `eulumdat` (feature
`sensor`).

## 7. Open questions

1. `sensor/sensldt` remaining unknowns: parameter fields 4, 5, 7, 9, 10;
   the exact plane Relux uses for wall sensors (≈ 1.75 m vs 2.0 m ceiling);
   84 renders with the sensor off-centre (wall sensors) not yet compared.
2. More SensXML samples: generate / check the reader against several real
   files (Relux database, GLDF files with sensors) rather than one. Also the
   SensXML documentation link (relux.com is not reachable for automated
   fetches; XSD reported as restricted). Until then semantics are inferred:
   `DetectionType` 1/2/3 (we infer
   tangential / radial / presence), `DistanceFromCenter` (we infer mounting
   height), `MountingType`, `Shape`, `ScaleShape`, `SamplingDirection`,
   `RotationAngle`, `ActiveGrid`, azimuth origin. Ask Relux for the XSD.
3. L3D `SensorObject` viewing axis at zero rotation; meaning of
   `SensorEmitter/Rotation G0`.
4. IEC 63180 details behind the paywall: test-arm height, grid pitch, walking
   speed, the recommended mounting heights from A1.
5. EN 15193-1 F_D,C tables (paywalled), needed if we report the standard F_D
   next to the simulated one.
6. Title 24 2025 changes to sensor rules.
7. Daysim `.sen` / XML photosensor schema (only the rsensor matrix is known).

## 8. Sources

- GLDF: github.com/globallightingdata/examples (commit 974d82d, "SLV - Tria 2
  with sensor"), github.com/globallightingdata/templates/tree/master/006_sensor,
  github.com/globallightingdata/website/blob/master/docs/structure/sensors.md,
  examples issue #8; L3D XSD github.com/globallightingdata/l3d/blob/master/xsd/l3d.xsd
- Relux: activatereluxdesktop.relux.com/en/page/release-notes.html,
  activatereluxdesktop.relux.com/en/sensnorm.html; SensCalc:
  led-professional.com/all/new-unique-online-tool-for-planning-pir-sensors-with-energy-and-amortisation-calculation
- IEC 63180 samples: cdn.standards.iteh.ai/samples/101399/6e494c4306994c8aa87359b149d0b374/IEC-63180-2020.pdf,
  assets.vde-verlag.de/iec-normen/preview-pdf/info_iec63180{ed1.1}en.pdf;
  sensNORM: esylux.com (interview with Olaf Riebenstein)
- NEMA WD 7 / PIR physics: energy.gov/sites/default/files/2023-07/ssl-rti_pir-sensors_mar2022.pdf;
  Theben planning guide: tga-fachplaner.de/sites/default/files/2020-07/theben-leitfaden_bewegungs-_und_praesenzmelder_richtig_planen_2020-07-02.pdf
- Datasheets: Theben theRonda P360 / S360 / thePassa manuals (theben.de/ocsmedia),
  Niko guide.niko.eu (detection area of long-range presence detectors)
- EN 15193-1: normsplash.com DIN sample (DIN-EN-15193-1-2021), ENEA slides
  sue.enea.it/wp-content/uploads/2018/06/Blaso-UNI-EN-15193-1_2017_-LENICALC.pdf
- DIALux: evo.support-en.dial.de (energy consumption), dial.de news on evo 11
- Photosensors: Radiance rsensor man page (floyd.lbl.gov/radiance/man_html/rsensor.1.html)
  and discourse threads; Rubinstein, Ward & Verderber 1988
  (digital.library.unt.edu/ark:/67531/metadc1190150); escholarship.org/uc/item/5tt2k9gv;
  escholarship.org/uc/item/70h8x3wt; osti.gov/servlets/purl/890988;
  aceee.org/files/proceedings/2008/data/papers/3_124.pdf;
  radiance-online.org 2009 SPOT and 2013 Daysim workshop slides;
  mdpi.com/1996-1073/14/21/7056; EnergyPlus engineering reference (daylighting);
  Signify SNS400 spec sheet; Krüger et al. 2023 (f1′, acris.aalto.fi)
