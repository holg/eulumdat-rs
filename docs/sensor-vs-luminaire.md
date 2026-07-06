# Sensors vs. luminaires in EULUMDAT/LDT

**Short version: you cannot tell a sensor's LDT apart from a luminaire's LDT by
file content. Do not try. The answer must come from external context.**

## What happened

Some products (observed: SLV OCULUS CW / Tria 2) describe a **sensor's
detection-sensitivity pattern** in the same EULUMDAT/LDT format used for a
luminaire's photometric light distribution. The detection lobe is stored exactly
where candela values normally live.

The EULUMDAT format has **no field, flag, or marker** that says "this is a
sensor" — neither the parser (`crates/eulumdat/src/parser.rs`) nor the
[`Eulumdat`](../crates/eulumdat/src/eulumdat.rs) struct carries such a notion,
because the spec is for luminaire photometry only.

## Why content-based detection is unsafe

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
