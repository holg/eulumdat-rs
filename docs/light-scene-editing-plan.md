# Light scene: custom scenes and more editing — plan

Status 2026-11-06: path B built (both designers → Light scene, "Planned"
scene with outline, reflectances, poles, sensors; Interior polygon rooms).
Building block 1 built as well (`eulumdat_scene::doc::SceneDoc`, scenes
saved in the project file and localStorage, autosave). Open: building
blocks 2 (several luminaire types), 3–5, paths A, C, D. Follows the sensors /
moving objects / quality work in the Light scene (`eulumdat-scene`,
`components/rt_scene.rs`).

## Where we are

- Five parametric scenes (`SceneKind`: street, parking, facade, office,
  greenhouse), each built in code by `build_scene(&SceneParams)`: analytic
  primitives (box, oriented box, sphere, capsule, cylinder), lamps,
  measurement grids, camera.
- Editing today: move luminaires (with pole / arm / wire), the person, the
  cars and sensors on the ground (`ObjectId`, `SceneParams::moves`);
  placement settings (height, tilt, count, spacing, layout, ceiling, …).
- One photometry per scene: every lamp uses the same IES table
  (`ies_tex`, binding 3, one `texture_2d`).
- Limits: `MAX_PRIMS` 192, tested one by one per ray (no acceleration
  structure for primitives); `MAX_LAMPS` 16; the only triangle geometry is
  the luminaire body from L3D (`LuminaireMesh`, BVH, ≤ 32 768 triangles,
  instanced per lamp).
- Measurement grids are fixed per scene (`MeasureGrid` over a `Receiver`).
- Nothing about a scene is saved, except what a share link carries.

## Building blocks (needed by every path below)

1. **Scene document.** A serialisable `SceneDoc` (JSON): ground / room,
   objects (kind, position, yaw, size, material), luminaires (photometry
   ref, position, rotation, tilt), sensors, measurement planes, camera.
   The five built-in scenes become generators that produce a `SceneDoc`;
   `build_scene` turns any doc into prims + lamps + grids. Moves, added and
   deleted objects are edits of the doc. Saved as a new project item kind
   (`ItemKind::Scene`) in the `*.candelabra.json` project file, so a scene
   travels with its luminaires and sensors.
2. **Several luminaire types per scene.** IES tables as a texture array
   (or one atlas), a table index per lamp (`GpuLamp` has room), the viewer
   holding `Vec<Photometry>`; luminaires picked from the current project.
   Also needed for mixed street / facade / bollard designs.
3. **More geometry.** A BVH over the primitives (built on the CPU, uploaded
   like the mesh BVH, traversed in WGSL and `cpu.rs`), raising `MAX_PRIMS`
   to a few thousand without slowing the tracer; more lamps (`MAX_LAMPS`
   64) with the one-random-lamp sampling already in place.
4. **Editing tools in the picture.** Select by clicking (ray-pick the
   primitive → its object), the move handle we have, a yaw ring (cars,
   luminaires), height drag (luminaires, sensors), size handles (boxes,
   walls), delete / duplicate, undo / redo, snapping (grid 0.1 m, to walls,
   to poles), a properties panel for the selection (size, colour /
   reflectance, luminaire type, tilt).
5. **Measurement planes.** Draw a rectangle (floor, workplane, facade) as a
   calculation grid with spacing per EN 12464-1 / EN 13201; Em / Emin / Uo
   per plane as today.

## Paths (pick an order)

### A. Edit the built-in scenes (S–M)

Palette: person, car (colours), tree, bench, bollard, hedge, wall / box,
desk, extra pole + luminaire, sensor. Rotate, delete, duplicate,
materials. Saved in the project. Needs 1 (doc), 4; 3 once the palette
fills the 192-prim budget.

### B. From the planners into the Light scene (M) — recommended first

"Open in Light scene" in the Interior designer and the Outdoor designer:

- Interior → an office-like room of the planned size (length, width,
  height, reflectances, workplane, suspension), the planned luminaires at
  their positions (moved ones included), the planned sensors, the
  workplane grid.
- Outdoor → the planned area (rectangle or polygon as ground), its poles
  with arrangement, height, tilt, rotation and per-pole luminaire type, the
  motion detectors at their height.

Everything planned in 2D is checked in 3D, path traced, with the same
sensors switching the light. Needs 1, 2 (outdoor uses mixed luminaire
types), partly 5. Reuses `zonal::LuminaireLayout::positions()`,
`area` pole placements and the sensor plans.

### C. Room / area builder (M–L)

Rooms from rectangles or polygons with wall height, doors and windows
(cut-outs), several rooms, a furniture palette; outdoor areas with
kerbs, paths and planting. Grows out of A + B.

### D. Import geometry (L)

OBJ / glTF (and IFC from the BIM tab) as triangle meshes with a BVH
(generalising the luminaire-body BVH from one instanced mesh to scene
meshes), materials mapped to reflectances by name or colour, scale and
up-axis on import. For real buildings and sites.

## Suggested order

1. Scene document + save in the project (building block 1).
2. Several luminaire types (building block 2).
3. Path B (both designers → Light scene).
4. Editing tools (4) and the palette (A); primitive BVH (3) when the
   budget is reached.
5. Measurement planes (5).
6. Room / area builder (C), then import (D).

## Questions

- Which comes first for you: B (designers → 3D), A (edit / palette), or D
  (import real geometry)?
- Should a saved scene keep its own luminaire copies, or always follow the
  project's luminaires (a changed LDT updates the scene)?
- Import formats that matter most (OBJ, glTF, IFC, DWG via conversion)?
