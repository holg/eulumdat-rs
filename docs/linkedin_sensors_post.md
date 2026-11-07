# Posts: presence & motion sensors in Candelabra (drafts, 2026-11-04)

Demo link (works after the next deploy, no upload, synthetic data):
`https://iesna.eu/#tab=sensor&sensor=demo`
(the landing page forwards `#…` links to `/app/`; direct: `https://iesna.eu/app/#tab=sensor&sensor=demo`)

Images (synthetic demo sensors only, made with the CLI, in `tmp/sensor-post/`):
`overview.png` (ceiling detector · corridor detector · SensXML), `overview_sensxml.png`
(SensXML: wall 180° · high-bay at 10 m · corridor), `plan_ceiling.png`,
`volume_ceiling.png`, `polar_ceiling.png`.

The demo list in the app: two sensor LDTs (ceiling 360°, corridor) and four SensXML
(ceiling, wall 180°, high-bay 6–14 m with interpolation, corridor).

## Post 1 — announcement (EN)

```
Lighting tools have plenty of ways to show where light goes.
Hardly any show where a presence detector can see you.

Candelabra (iesna.eu) now reads sensors:
• GLDF files with a sensor (sensor/sensldt and sensor/sensxml)
• sensor LDTs: the same EULUMDAT format, but the "intensity" table is the
  detection volume in metres
• SensXML: detection areas measured per mounting height (IEC 63180 / sensNORM)

For each sensor you get the detection areas for tangential walking, radial
walking and seated presence at any mounting height, plus the volume itself as
polar, heatmap and 3D views. Cone, beam angle and isolux are switched off for
sensors: they describe light, not detection.

Everything runs in your browser. Nothing is uploaded.
The core is open source (Rust, AGPL): eulumdat-rs.

Try the demo sensors: https://iesna.eu/#tab=sensor&sensor=demo

#lighting #lightingdesign #controls #presencedetection #GLDF #DALI #opensource
```

## Post 1 — Ankündigung (DE)

```
Lichtplanungs-Tools zeigen auf viele Arten, wohin Licht fällt.
Kaum eines zeigt, wo ein Präsenzmelder Sie sieht.

Candelabra (iesna.eu) liest jetzt Sensoren:
• GLDF-Dateien mit Sensor (sensor/sensldt und sensor/sensxml)
• Sensor-LDTs: dasselbe EULUMDAT-Format, aber die „Lichtstärke“-Tabelle ist
  das Erfassungsvolumen in Metern
• SensXML: Erfassungsbereiche je Montagehöhe gemessen (IEC 63180 / sensNORM)

Für jeden Sensor gibt es die Erfassungsbereiche für tangentiales Gehen,
radiales Gehen und sitzende Präsenz in jeder Montagehöhe, dazu das Volumen
selbst als Polar-, Heatmap- und 3D-Ansicht. Kegel, Halbwertswinkel und Isolux
sind für Sensoren abgeschaltet: sie beschreiben Licht, nicht Erfassung.

Alles läuft im Browser. Nichts wird hochgeladen.
Der Kern ist Open Source (Rust, AGPL): eulumdat-rs.

Demo-Sensoren: https://iesna.eu/#tab=sensor&sensor=demo
```

## Post 2 — "What is inside a sensor LDT?" (technical, EN)

```
A sensor LDT looks exactly like a luminaire LDT. The numbers mean something else.

1. The intensity table is a detection volume: the value at (C, γ) is the
   distance in metres from the sensor to the edge of the zone in which a person
   is detected. A corridor sensor's file literally contains 4.25/cos γ and
   1.5/sin γ — a slab 4.25 m deep and 3 m wide.
2. A detection area is a cut through that volume at
   mounting height − person height.
3. The ten "direct ratios" are a parameter block: scale factors for
   tangential walking, radial walking and seated presence, plus the mounting
   type (wall / ceiling).
4. The lamp block (flux, LOR) means nothing for a sensor. A heatmap that
   multiplies by lamp flux gets every range wrong.

Nothing in the file says "I am a sensor". The context does (GLDF content type),
so software must never guess.

We wrote this down openly and implemented it in eulumdat-rs (Rust), including
the plan view, a SensXML reader and a CLI:
  eulumdat sensor demo.ldt --height 2.8 --svg areas.svg

#lighting #GLDF #EULUMDAT #opensource #rustlang
```

## Facts behind the claims (keep posts inside these)

| Claim | Basis |
|---|---|
| "Hardly any tools show detection areas" | Found: Relux SensCalc (browser, login, server-side per our understanding), ReluxDesktop (desktop), manufacturer PDF guides. DIALux evo: energy factors and a daylight sensor point, no presence detection areas found. Say "hardly any", not "first" or "only". |
| "Runs in the browser, nothing uploaded" | WASM client; sensor files are parsed in the page. |
| Formats read | GLDF `sensor/sensldt` + `sensor/sensxml`, sensor LDT, SensXML 3.9 – 4.5 (tested on 22 real files). |
| Motion types | Tangential / radial / presence = IEC 63180 major motion (across / towards) and minor motion. |
| Accuracy | Our plan views match reference plan views of real sensor files within 5 % for 90–99 % of 165 files; SensXML planning shapes to ±0.02 m. Quote the numbers without naming data sources. |
| Open source | eulumdat-rs core + CLI (AGPL-3.0-or-later). |

## Leave out of public posts

- How the reference data was collected (the ReluxNet sensor scrape and the
  `.rolfz` decryption with the light-other-rs tooling). The corpus stays
  private; posts show only the synthetic demo sensors.
- Vendor product names next to our numbers, unless the data is the vendor's
  own published datasheet.
