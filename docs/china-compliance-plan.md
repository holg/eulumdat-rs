# China: what Candelabra must comply with (plan, 2026-11-08)

China is the special market. Two kinds of compliance: the **planning
checks** must follow the Chinese national standards (GB), and the **site
itself** must follow Chinese rules where it shows maps or uses services
that are not available there. Values below come from public summaries
(web, 2026-11-08), not from the standard texts: every number is to be
checked against the text (or a values sheet from the user, as for EN 1838)
before a profile goes live.

## 1. Which standards, per tool

Since 2022 China has mandatory "general codes" (通用规范, GB 55xxx) that
take precedence; the older standards became recommended (GB/T) or keep
only what the general codes do not contradict.

| Tool in Candelabra | China | Candelabra today |
|---|---|---|
| Street / road (EN 13201, RP-8) | CJJ 45-2015 urban road lighting | **built** (`standards::cjj45`, Street designer) |
| Interior (EN 12464-1 targets) | GB/T 50034-2024 lighting design of buildings (issued 2024-03-12, in force 2024-08-01, replaces GB 50034-2013, now recommended); mandatory: GB 55015-2021 (energy: lighting power density limits), GB 55016-2021 (building environment: lighting quality, daylight; GB/T 50034-2024 refers to it for photobiological safety) | missing: room types with Em, UGR, U0, Ra; LPD check |
| Emergency (EN 1838) | GB 55037-2022 general code for fire protection of buildings (mandatory, in force 2023-06-01) + GB 51309-2018 fire emergency lighting and evacuation indicating systems (in force 2019-03-01); products GB 17945-2024 | **built** (profile `gb55037`, one of 14 standards in the emergency picker, see `docs/standards-en1838-en17037-plan.md`) |
| Daylight (EN 17037, planned) | GB/T 50033-2013 daylighting design of buildings (daylight factor by room type, five light-climate zones with a factor K); mandatory parts in GB 55016-2021 | not started; the globe picker gives the location → light-climate zone |
| Sensors / controls (EN 15193-1) | GB 55015-2021 (controls, energy) | EN 15193-1 only |

## 2. Emergency lighting: decided and built (2026-11-08)

**Decision.** The check for escape routes in China follows **GB 55037-2022
§10.1.10** (mandatory general code, in force 2023-06-01): minimum
horizontal illuminance on the floor, by place:

| Place | Minimum on the floor |
|---|---|
| Evacuation stairwells and their front rooms / combined front rooms, refuge corridors and their front rooms, refuge floors, refuge rooms, fire-fighting-only passages | 10 lx |
| Evacuation corridors, crowded places | 3 lx |
| All other places | 1 lx |

- Where GB 51309-2018 gives lower floor values for the same places
  (corridors 1 lx, stairs / front rooms 5 lx), the general code governs.
  GB 51309 stays for what the general code does not say: durations
  (0.5 / 1.0 / 1.5 h), switch-over (5 s, 0.25 s high-risk), A-type
  luminaires, signs. Reported, not calculated.
- GB/T 50034-2024 §5.5.5 (averages, centre-line 40:1) is for evacuation
  lighting outside fire emergencies; its §5.5.2 refers fire emergency
  lighting to the fire codes. Not used.
- Measured: a minimum at any point (not an average), no uniformity ratio.
  Routes over the central half of their width (Chinese design papers
  compute it that way), rooms over the whole floor (no border is defined:
  0 m, the strict reading). No vertical minimum at equipment, no glare or
  Ra limit (values still shown).
- Sources: several independent reproductions of §10.1.10 (design-review
  and training material, a 2024 forum thread quoting the clause and
  GB/T 50034-2024 §5.5.2 / §5.5.5); the code text itself could not be
  fetched (the standards sites block automated access). Values and
  sources: `crates/eulumdat/data/cn_gb55037_limits.json`. Check against the
  published text when you have it.

**Built.** Core `eulumdat::emergency`: `Limits::gb55037()`,
`Limits::by_id`, `PlaceCategory` on routes and areas, `PlaceLimits`,
`route_limits` / `area_limits` (EN 1838 unchanged), report rows per
profile, automatic layout tightens a failing route (stairs at 10 lx).
CLI `eulumdat emergency … --standard gb55037` (alias `cn`), plan JSON
`"place": "corridor" | "stairs" | "crowded" | "other"`. Interior designer:
standard select (default GB 55037 in the Chinese UI, else EN 1838, link
`emstd=`), place per route (`emr=…~2~stairs`) and for the room
(`emoplace=`), results and PDF per profile, floor bands 1 / 2 / 3 / 5 /
7.5 / 10 lx. Light scene: the plan carries the standard
(`PlanEmergency::standard`, `route_stairs`), band edges measured, false
colours up to 10 lx, the panel checks per standard.

## 2a. Background: the Chinese rule shape (research notes)

Different from EN 1838: no centre line / central band, no Emax/Emin ratio
(to check), the **minimum horizontal illuminance on the floor depends on
the kind of place**:

| Place (summary) | GB 51309-2018 | GB 55037-2022 (to check) |
|---|---|---|
| Evacuation corridors and passages | ≥ 1.0 lx | ≥ 1.0 lx |
| Crowded places (assembly halls, large sales / office halls …), refuge floors / rooms | ≥ 3.0 lx | ≥ 3.0 lx |
| Stairs, front rooms, refuge corridors (general) | ≥ 5.0 lx | reportedly ≥ 10.0 lx for all of them |
| The same in crowded places, care homes for the elderly, wards / operating departments; refuge rooms there | ≥ 10.0 lx | ≥ 10.0 lx |

Also in GB 51309 (to check): duration by building (0.5 h / 1.0 h / 1.5 h
for buildings over 100 m), switch-over time (5 s; 0.25 s in high-risk
places), A-type (≤ DC 36 V) luminaires up to 8 m mounting height, spacing
of evacuation signs. Design practice in Chinese papers checks the minimum
across the central half of the corridor width (2024 papers in 建筑电气 /
现代建筑电气).

Consequence for the core: `Limits` → profiles with a rule per **place
category** (corridor, crowded place, stair / front room / refuge corridor,
special care), the Interior emergency section lets the user give each
route / area its category (defaults: routes = corridor, the room as open
area = crowded place when it qualifies), the report names the profile and
the clause. The Light scene check uses the same profile.

## 3. The site itself

- **Maps.** China requires maps shown to users there to go through map
  review (审图号, Ministry of Natural Resources) and to show the national
  territory as China defines it; non-compliant maps have been seized and
  apps blocked. Two places show maps:
  - the new globe location picker (geodb-rs; dr5hn data lists countries
    with flags, including ones China does not recognise as countries),
  - the Outdoor designer's site map: OpenStreetMap tiles (since
    2026-11-08; it replaced the Google Maps designer, which was never
    reachable from mainland China and failed without an API key), found
    on the same globe in street mode. tile.openstreetmap.org is reachable
    from China only unreliably, and OSM shows borders as its community
    maps them.
  Options: for Chinese users (zh locale or a China build) a text city
  search instead of the globe and country names following Chinese
  conventions; a reviewed map service (Tianditu 天地图, the state map,
  or AMap 高德, both with keys, AMap in GCJ-02 coordinates) as the site
  map's tile source (`eulumdat::area::sitemap` takes one URL pattern;
  GCJ-02 needs the offset applied to the site); or no map background in
  China. Deployed with the globe on 2026-11-08 (the user's decision; the
  map question stays open).
- **Reachability.** The app is hosted in Germany (candelabra.eu): reachable
  but slow from China; Google services are blocked. Hosting in mainland
  China needs an ICP filing (备案); Hong Kong or a CDN with China edge
  nodes are the usual middle ways.
- **Language.** zh is already one of the 8 locales; the reports (PDF) are
  English only.

## 4. Order (proposal)

1. Emergency profiles in the core (EN 1838:2013 as the first, the Chinese
   one next) — values from the user's sheet for GB 55037 / GB 51309.
2. Interior: GB/T 50034-2024 room types (Em, UGR, U0, Ra) + GB 55015 LPD.
3. Decide the map handling for China before the globe picker is deployed.
4. Daylight: GB/T 50033 next to EN 17037 (light-climate zone from the
   location).
