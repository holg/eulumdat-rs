# Sensors vs. luminaires in EULUMDAT/LDT

**Short version (2026-11-05): a real sensor LDT can be recognised by its
content.** `eulumdat::sensor::is_sensor_ldt` checks three things:

1. every table value is a range in metres: 0 < max ≤ 60 (luminaire values in
   cd/klm are far larger);
2. field 8 of lines 33–42 (a luminaire's "direct ratios") is a mounting code
   1, 2 or 3 (wall, ceiling, both); a direct ratio is a fraction 0 … 1;
3. those ten values do not rise with the room index, as a luminaire's direct
   ratios always do.

Measured: all **1 187** sensor LDTs of the Relux Net corpus (8 manufacturers)
match; **none of 34 916** luminaire LDTs (6 480 GLDF files, 81 manufacturers)
and none of the 100 luminaire LDTs of luminaires with a built-in sensor that
the corpus files under sensors. Field 8 = 2 or 3 alone already matches 93 %
of the sensors and no luminaire. Corpus test:
`SENSOR_LDT_CORPUS=<dir> cargo test -p eulumdat sensor::detect`.

Order of evidence: declared context first (GLDF `contentType="sensor/…"`, the
`sensor/` folder, a "sensor" upload), then the fingerprint. Limits: every
known sensor LDT uses this one (Relux) layout; a sensor LDT in some other
layout is not recognised and stays a luminaire. A luminaire LDT read as a
sensor gets a warning (`SensorWarning::ImplausibleRange` /
`NoSensorParameters`).

Where it is used: `eulumdat-scene::l3d::gldf_extract` (an unmarked `.ldt`
with the fingerprint becomes a sensor), the web editor (an opened or dropped
`.ldt` with the fingerprint opens on the Sensors tab, with "Open as luminaire
instead").

The rest of this note is the history: why the project first concluded that
the two cannot be told apart.

## What happened (first analysis)

> Correction (2026-11-04): the SLV "sensor.ldt" below is not a sensor at all
> but the photometry of a luminaire copied into the container's `sensor/`
> folder (docs/sensors-plan.md §1). The comparison is right; the conclusion
> drawn from it is not.

Some products (observed: SLV OCULUS CW / Tria 2) describe a **sensor's
detection-sensitivity pattern** in the same EULUMDAT/LDT format used for a
luminaire's photometric light distribution. The detection lobe is stored exactly
where candela values normally live.

The EULUMDAT format has **no field, flag, or marker** that says "this is a
sensor" — neither the parser (`crates/eulumdat/src/parser.rs`) nor the
[`Eulumdat`](../crates/eulumdat/src/eulumdat.rs) struct carries such a notion,
because the spec is for luminaire photometry only.

## Why content-based detection looked unsafe

Comparing the real SLV pair (`sensor.ldt` vs. `wide.ldt`, same product), the two
files are **structurally identical and share an identical identity + lamp block**:

| Field | sensor | luminaire |
|---|---|---|
| Ityp / Isym | 1 / 1 | 1 / 1 |
| manufacturer / product / number | SLV / OCULUS CW / 1004664 | *same* |
| num_lamps | -1 | -1 |
| lamp_type | `LED, dim-to-warm 2000K, CRI90 Wide Beam` | *same* |
| flux | 35 lm | 35 lm |
| color temperature | 2000K | 2000K |
| CRI | 90 | 90 |
| wattage | 0.9 W | 0.9 W |

In particular, the intuitive **"a sensor wouldn't carry CRI/colour temperature"**
rule is wrong here: the exporter copied the parent luminaire's lamp block verbatim
into the sensor file, so CRI=90 and CT=2000K are present-but-meaningless. That rule
would be a **false negative** on this exact file.

The only things that differ are the measured values:

- **Intensity profile** — luminaire is a smooth beam (537 → 0.79 cd/klm); sensor is
  a flat plateau (~480) that drops off a cliff. But a genuine narrow-beam luminaire
  can look similar, so this is not separable in general.
- **`direct_ratios`** (EULUMDAT field 27) — luminaire is a monotonic physical curve
  `0.37 … 0.91`; sensor is junk `1, 0.76, 0.38, 0, 1, 1, 0, 1, 0, 0.91`. Suggestive,
  but the values are still in range `[0,1]`, so validation `W031` does not flag it,
  and we do not treat it as a classifier.

None of these is reliable, so eulumdat-rs ships **no** sensor heuristic and the
parser never guesses.

## What to do instead

Sensor-vs-light is **caller/container knowledge**. The authoritative signal lives
outside this crate — for example a GLDF container marks the file with
`contentType="sensor/sensldt"` and references it via `<SensorEmitter>`. Consumers
that need the distinction must carry it from that context; do not infer it from the
parsed `Eulumdat`.

If a future consumer wants to tag a parsed file, the clean approach is a
**caller-set** kind on the consuming side — explicitly never inferred by eulumdat's
parser.

## Locked-in by test

This finding is encoded as an executable regression in
[`crates/eulumdat/tests/sensor_vs_luminaire.rs`](../crates/eulumdat/tests/sensor_vs_luminaire.rs),
which asserts the two real files are indistinguishable by identity/CRI/CT/lamp
block.

> Note: the existing fixture `crates/eulumdat/tests/fixtures/variant2_sensor_wide.ldt`
> is **not** a sensor — it is a normal wide-beam luminaire whose name merely contains
> the word "sensor". The sensor pair lives in `fixtures/{sensor.ldt, wide_luminaire.ldt}`.
