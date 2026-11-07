# Sensor comparison — plan and status (2026-11-06)

**Status:** built (all of v1 + v2 below, in the Compare tab as chosen):
core `eulumdat::sensor::compare`, report, CLI `sensor-compare`, web
`components/sensor_compare.rs`. Still open: v3 ranking of N sensors.

Goal: put two (later: several) presence / motion detectors side by side under
the **same conditions** and answer the planner's real questions: which one
sees more, where, at my mounting height, and how many of each does my room
need. Sources are everything the app already reads: sensor LDT (uploaded,
dropped and recognised, or from a GLDF), SensXML, GLDF sensor files, the demo
sensors. LDT and SensXML can be mixed.

## What exists to build on

| Piece | Where |
|---|---|
| One API for both formats: `SensorFile::area_at(motion, mount, plane, planning)`, `areas`, `measured_heights`, `warnings` | `eulumdat::sensor::file` |
| Rebuilt detection volume for SensXML (angular views) | `SensXml::to_detection_volume` |
| C-plane views (polar / cartesian pair, cut on the plan) | `sensor::diagrams::render_at_plane`, `SensorPlanSvg::render_with_cut` |
| Coverage, automatic grid with orientation, EN 15193-1 | `sensor::coverage`, `sensor::en15193` |
| Luminaire comparison pattern: metrics with Δ % and significance, overlay polar / cartesian, side-by-side views, comparison PDF | `eulumdat::compare`, `components/compare_panel.rs` |
| Sensor list, upload, demos, Sensors tab state | `components/sensor_view.rs` (`SensorFiles`, `SensorUi`) |

## Comparison conditions (one set for both sensors)

Mounting height H (ceiling), walking plane (floor, 0 m), presence plane
(workplane, default 0.8 m), planning shapes on/off (SensXML), C-plane. A
sensor LDT is cut at H − plane, a SensXML file is read at H (its measured
plane), exactly as in the Interior designer, so both sides mean the same.
Each side keeps its data quality (measured / interpolated / extrapolated,
"volume ends at …") and its warnings.

## Building blocks

1. **Plan overlay** — footprints of A (solid) and B (dashed) per motion type
   on one scale, legend with max range / area; the shared C-plane cut with
   the reach of both marked on it.
2. **Polar and cartesian overlay** — both volumes in metres in one diagram
   at the shared C-plane (slider), like the luminaire compare overlay.
3. **Numbers table** — per motion type: max range, area, Ø, quality, depth
   limit, A | B | Δ % with the significance colours of `eulumdat::compare`.
4. **Range over mounting height** — Ø (or area) of each motion type from
   2 m to 15 m for A and B in one chart; measured heights marked (SensXML),
   the height where a volume ends shown. Answers "which detector for an 8 m
   hall".
5. **Room check** — for one room (L × W × H, workplane): sensors needed for
   ≥ 95 % (automatic grid incl. orientation), coverage and overlap, m² per
   sensor, EN 15193-1 credit, for A and B; optional price per sensor →
   cost. "B needs 4 where A needs 9."
6. **Comparison PDF** (Typst, like the luminaire comparison report) and a
   share link (demo sensors only; uploads stay in the browser).
7. **Ranking (N sensors)** — all sensors of the list (e.g. every sensor of a
   manufacturer GLDF) ranked for one room: sensors needed, coverage, range at
   H. The step from "compare two" to "choose the right detector".

## Core API (proposal)

`eulumdat::sensor::compare`:
- `ComparisonConditions { mount, walk_plane, presence_plane, planning, c_plane }`
- `SensorComparison::new(&SensorFile, &SensorFile, &conditions, labels)` →
  per-motion rows (A, B, Δ %, significance) + `to_text()` (CLI);
- `plan_overlay_svg`, `polar_overlay_svg`, `cartesian_overlay_svg`;
- `height_sweep(file, motion, heights, planes) -> Vec<(h, Option<DetectionArea>)>`
  and `height_chart_svg(a, b, …)`;
- `room_check(files, room) -> Vec<RoomCheck>` (reuses `auto_grid_oriented`,
  `coverage_with`, `en15193::assess`);
- Typst: `eulumdat_typst::generate_sensor_comparison_report`.
- CLI: `eulumdat sensor-compare a.ldt b.xml --height 3 [--svg …] [--report x.pdf]`.

## UI (proposal)

Sensors tab, "Compare with…" picker next to the sensor picker (empty =
single view as today). With B chosen, the view buttons show the overlay
versions (plan, polar, cartesian) plus "Range over height" and "Room
check"; height, planning and C-plane controls drive both. The luminaire
Compare tab stays for luminaires. Strings in `ui.sensor` (8 languages).
Deep link `#tab=sensor&sensor=demo&compare=<i>` (demo sensors).

## Phases

- **v1**: core `compare` (conditions, rows, overlays), Sensors tab picker,
  plan + polar + cartesian overlay with the shared C-plane, numbers table,
  tests on the demo sensors, landing note.
- **v2**: range over height chart, room check (+ cost), comparison PDF,
  share link, CLI.
- **v3**: ranking of N sensors for a room.

## Open points

- How far to trust LDT vs SensXML side by side: an LDT volume follows the
  Relux convention, SensXML areas are measured (IEC 63180 test conditions);
  show the source and quality on every row, never hide it.
- Wall sensors: compare in their own frame (C0 into the room); a wall
  mounting in the room check needs wall positions (Interior designer limit
  as well).
