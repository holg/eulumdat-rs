# Generates crates/eulumdat-wasm/landing/index.html with data-i18n keys and
# an embedded dictionary for de, fr, es, it, ru, pt-BR, zh.
import json, html, collections, re
# Canonical site URL from scripts/build-config.toml ([site] url).
SITE_URL = "https://iesna.eu/"
try:
    _cfg = open("scripts/build-config.toml", encoding="utf-8").read()
    _m = re.search(r'^\[site\][^\[]*?^url\s*=\s*"([^"]+)"', _cfg, re.M | re.S)
    if _m:
        SITE_URL = _m.group(1)
except OSError:
    pass
EN = collections.OrderedDict([
("title","Candelabra – see any luminaire at work, in your browser"),
("meta","Candelabra is a free, open-source photometric workbench. Drop an LDT, IES, ATLA or GLDF file: every diagram, standards checks, a path-traced office, street or greenhouse lit by the real intensity table, spectral and horticultural analysis, area and road design. Nothing leaves your browser."),
("nav_views","Views"),("nav_sensors","Sensors"),("nav_colour","Colour quality"),("nav_formats","Formats"),("nav_for","Who it is for"),("nav_source","Source"),("nav_open","Open the workbench"),
("h1","See any luminaire at work. In your browser."),
("lead","Drop an LDT, IES, ATLA or GLDF file. Candelabra draws every diagram, checks the standards, and lights an office, a street or a greenhouse with a path tracer that samples the real intensity table. Free and open source. Nothing leaves your machine."),
("cta_open","Open the workbench"),("cta_street","Try the street with the road sample"),
("fine","Runs on WebGPU in Chrome, Edge, Safari and Firefox. Eight languages. No account, no upload."),
("cap_office","Office · photo view"),("cap_street","Street · photo view"),("cap_grid","Office · measurement grid"),("cap_iso","Street · illuminance and isolux"),("cap_gh","Greenhouse · PPFD on the canopy"),
("views_h","Six views, one tracer"),
("views_sub","The same accumulation shows the photo the eye would see, the illuminance in false colour with isolux lines, and the measurement grids of EN 12464-1 and EN 13201-3 with Em, Emin and uniformity per grid. In the greenhouse the values are PPFD and daily light integral. Mounting height, tilt, spacing and the number of luminaires are sliders."),
("samples","Open a sample directly: <a href=\"app/#tab=light-scene&amp;sample=road-luminaire&amp;scene=street\">street</a> · <a href=\"app/#tab=light-scene&amp;sample=road-luminaire&amp;scene=parking&amp;mode=isolux\">parking lot</a> · <a href=\"app/#tab=light-scene&amp;sample=fluorescent-luminaire&amp;scene=office&amp;mode=points\">office</a> · <a href=\"app/#tab=light-scene&amp;sample=projector&amp;scene=facade\">facade</a> · <a href=\"app/#tab=light-scene&amp;sample=tm-33-23-horticultural-led&amp;scene=greenhouse&amp;mode=isolux\">greenhouse</a>. Links can carry scene, view, height, tilt, count, spacing and more."),
("v_photo","Photo"),("v_photo_s","Filmic exposure, haze and veiling glare; facade, cycle lane, parked cars for scale."),
("v_iso","Illuminance"),("v_iso_s","Lux as colour on every surface, with isolux lines every few lux."),
("v_grid","Grid"),("v_grid_s","Standard measurement points; workplane 132 lx average, Uo 0.34 for this sample."),
("v_gh","Greenhouse"),("v_gh_s","A glasshouse with crop benches under horticultural LEDs; the body of the luminaire comes from its GLDF when there is one."),
("v_ppfd","PPFD on the canopy"),("v_ppfd_s","The same scene in µmol/m²/s with isolines, from the luminaire's own spectrum, plus the daily light integral per bench."),
("v_facade","Facade"),("v_facade_s","A projector grazing a building front: window reveals, spill light and a grid on the wall."),
("fmt_h","Three things no other browser tool does"),
("fmt_t","Any lighting format in, any other out"),
("fmt_p","Every file is parsed into one lossless model, so what you open in one format you can save in another. Convert a manufacturer's IES to EULUMDAT, pack photometry with a spectrum into ATLA, or pull the LDT out of a GLDF container, all without leaving the page. To go the other way, our sister site <a href=\"https://gldf.icu\">gldf.icu</a> builds a complete GLDF container around your photometry, interactively."),
("fmt_in","Opens"),("fmt_out","Saves"),("fmt_images","chart images"),("fmt_spd","SPD, CSV spectra"),("fmt_ray","TM-25 ray files"),("fmt_gldf_out","GLDF, built interactively at gldf.icu"),("fmt_bim","TM-32 BIM parameters"),("fmt_sensor","sensor LDT, SensXML"),
("street_t","Street design with automatic standard checks"),
("street_p","Place several luminaires along a road and let Candelabra check the result against the European EN 13201, the American ANSI/IES RP-8, the Chinese CJJ 45 and the dark-sky Model Lighting Ordinance, with the required classes and the pass or fail per criterion."),
("street_link","Open the street designer"),
("cmp_link","Open Compare"),
("know_t","Did you know? The C0 plane is rotated between IES and LDT"),
("know_p","The two AEC MAXWELL samples are the same luminaire, once as a CIE-style file and once as an IESNA file. They look identical until you compare them: the C0 plane sits 90° apart, because the two standards define the reference plane differently. The isocandela view shows it at a glance. <a href=\"app/#tab=compare&amp;sample=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-cie&amp;compare=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-iesna&amp;diagram=isocandela\">Compare the pair in the isocandela view</a> · <a href=\"app/#tab=compare&amp;sample=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-cie&amp;compare=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-iesna&amp;diagram=polar\">as polar curves</a> · <a href=\"app/#tab=compare&amp;sample=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-cie&amp;compare=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-iesna&amp;diagram=heatmap\">as heatmaps</a>."),
("know_cie","Isocandela plot of the CIE file: C0 along the horizontal axis"),
("know_iesna","Isocandela plot of the IESNA file: the same luminaire, turned by 90°"),
("cmp_t","Compare two luminaires"),
("cmp_p","Drop a second file next to the first: diagrams side by side with linked C-plane sliders, a similarity score, and a table of every metric with its delta in percent. Export the comparison as a PDF."),
("lab_h","Spectrum Lab: what if every photon carried its colour?"),
("lab_sub","Photometry is already weighted for the daytime eye: lumens and lux tell you nothing about the night, the body clock or the sky. The Spectrum Lab traces the luminaire photon by photon, each one with a wavelength drawn from a measured spectrum, and does the weighting at the detector instead. One trace answers four questions at once."),
("lab_1","What the daytime eye sees"),("lab_1s","Illuminance, colour temperature, and how the colour shifts with the viewing angle, straight from the traced photons."),
("lab_2","What the night eye sees"),("lab_2s","S/P ratio and mesopic road luminance: the same lux can look bright or dim at night depending on the spectrum."),
("lab_3","What the body clock sees"),("lab_3s","Melanopic DER, the circadian stimulus of the light, from sleep-friendly to alerting."),
("lab_4","What the sky sees"),("lab_4s","Upward light, its colour temperature and blue share, and a dark-sky check against the 3000 K limit."),
("lab_note","Sources: measured vendor spectra from Yuji, Luxeon and Signify, or the editor's own luminaire. The sibling Daylight & Sky tab runs the same engine for sun, moon and sky at any place and time."),
("lab_link","Open the Spectrum Lab"),
("cq_h","Colour quality: what the averages hide"),
("cq_sub","A 3000 K LED with Ra 82 and Rg 98 sounds neutral. The colour vector graphic shows what the averages hide: reds come out 11 % duller, and R9 is only 10. Candelabra computes TM-30 (Rf, Rg and all 16 hue bins) and the CRI (Ra, R1–R15) from any spectrum and draws this card."),
("cq_1","From any spectrum"),("cq_1s","ATLA / TM-33 files, SPD and CSV spectra, or a spectrum digitized from a datasheet chart."),
("cq_2","Standard methods"),("cq_2s","ANSI/IES TM-30-20 (CIE 224:2017) and CIE 13.3, checked against an independent implementation and lab measurements."),
("cq_3","Read the shape"),("cq_3s","Dashed circle = reference. Outside = more saturated, inside = duller; the labels name the largest shifts."),
("cq_4","In the report"),("cq_4s","The card, R1–R15 and all hue bins become a chapter of the PDF report."),
("cq_cap","Demo LED 3000 K (synthetic spectrum): the card exactly as the app draws it."),
("cq_link","Open this card in the app"),("cq_link2","All 16 hue bins"),
("sens_h","Presence and motion sensors: where does the detector see you?"),
("sens_sub","Lighting tools show where light goes; hardly any show where a presence detector can see you. Open a GLDF with a sensor, a sensor LDT or a SensXML file, and Candelabra draws the detection areas for walking across, walking towards and sitting still, at any mounting height. Like everything else here, it runs in your browser and nothing is uploaded."),
("sens_1","Detection areas"),("sens_1s","Plan view for tangential walking, radial walking and seated presence, with range, area and diameter."),
("sens_2","Any mounting height"),("sens_2s","Set the height above the detection plane. Between measured heights the values are interpolated and marked as such; below the end of the volume it says so."),
("sens_3","The volume itself"),("sens_3s","Polar, cartesian, heatmap and 3D views of the detection volume, in metres instead of candela."),
("sens_4","In the PDF report"),("sens_4s","A sensor report on its own, or a sensor chapter after the luminaire's photometric report."),
("sens_note","A sensor LDT is the same EULUMDAT format as a luminaire file, but its table holds distances in metres. The demo sensors are synthetic examples, not products."),
("sens_link","Open the demo sensors"),("sens_link3d","Detection volume in 3D"),
("sens_c1","Ceiling detector"),("sens_c1s","Three detection areas, 2.0 m below the sensor."),
("sens_c2","Corridor detector"),("sens_c2s","A 24 m × 2.5 m strip along the corridor."),
("sens_c3","Wall detector (SensXML)"),("sens_c3s","Sees only in front of the wall; mounted at 2.5 m."),
("sens_ph","Plan them in a room"),
("sens_psub","In the Interior tab, pick a sensor: Candelabra places the fewest detectors that see the room, shows what they miss and estimates what presence control saves."),
("sens_p1","Fewest sensors"),("sens_p1s","An automatic grid that detects at least 95 % of the room; corridor sensors are turned along the corridor. Or set the grid yourself, or one per luminaire."),
("sens_p2","Gaps on the plan"),("sens_p2s","Detection areas on the floor (walking) and on the workplane (seated), spots nobody detects in red, coverage and overlap per motion type."),
("sens_p3","Energy estimate"),("sens_p3s","EN 15193-1 occupancy factor for the control and space type, kWh per year with and without presence control, and the conditions for the credit."),
("sens_p4","Report and link"),("sens_p4s","A sensor section in the interior PDF report; the share link keeps every setting."),
("sens_pcap","Demo classroom, 10 × 8 m: 9 demo ceiling detectors (3 × 3) see 98 % of the workplane; presence control saves about 11 % (EN 15193-1)."),
("sens_plink","Plan sensors in a classroom"),
("nav_emergency","Emergency lighting"),
("em_h","Emergency lighting, checked against EN 1838"),
("em_sub","Draw the escape routes on the plan, mark the safety equipment and the exits: Candelabra places the emergency luminaires and checks every EN 1838 requirement the way the standard asks, with direct light only, the emergency flux at the end of the rated duration and a maintenance factor."),
("em_1","Escape routes around corners"),
("em_1s","Centre line at least 1 lx, central band 0.5 lx, Emax / Emin at most 40, sampled every 10 cm along the route you draw."),
("em_2","Open areas and safety equipment"),
("em_2s","Anti-panic lighting above 60 m² (0.5 lx without the 0.5 m border); 5 lx vertical at fire extinguishers, first aid and call points."),
("em_3","Automatic layout"),
("em_3s","Luminaires at the route ends and evenly between, an open-area grid tightened until the room passes, one aimed at each piece of equipment, all from the luminaire's own spacing table. Drag any of them."),
("em_4","Glare, colour, report"),
("em_4s","Intensity limits between 60° and 90° by mounting height, Ra at least 40, a section in the interior PDF report. GLDF emergency emitters (combined or emergency-only) arrive with their emergency flux."),
("em_cap","Demo L-shaped office, 225 m², 3 m high: 11 demo emergency luminaires (180 lm each) light the 25 m escape route around the corner to at least 3.6 lx, the open area to at least 1 lx and the fire extinguisher to 8.5 lx vertical. Every EN 1838 requirement is met."),
("em_link","Open the L-shaped office"),
("em_link2","Start with an empty plan"),
("em_note","The demo emergency luminaires are synthetic examples, not products. Spacing tables and plan checks also run on the command line (eulumdat emergency)."),
("sky_h","Skyglow at Burg Vischering"),
("sky_sub","A moated castle in Lüdinghausen, rebuilt from the open survey data of Geobasis NRW: the 3D mesh and laser scan for the park and the moat, the building model for the walls, with façades projected from photographs. Floodlights wash the walls, lanterns stand on the bridges. Press Space to switch between lighting that brightens the night sky and lighting that keeps it dark, Z to raise the drawbridge; fly with WASD and the mouse."),
("sky_cap","Dark-sky mode, seen from the east bank. Survey data © Geobasis NRW (dl-de/zero-2.0); façades from 67 Wikimedia Commons photographs, CC BY-SA. <a href=\"vischering-credits.html\">Photo credits</a>"),
("sky_link","Open Burg Vischering"),
("for_h","Who it is for"),
("for_sub","Candelabra is not a replacement for a full project calculation. It is the fastest place to look at, check, understand and show a luminaire, and a growing set of tools around it."),
("a1_h","Lighting designers and architects"),("a1_w","Check a luminaire before it goes into a project, or show a client what it does."),
("a1_1","Every diagram"),("a1_1s","Polar, cartesian, 3D butterfly, heatmap, cone, isolux, isocandela, beam angles."),
("a1_2","Light scene"),("a1_2s","Path-traced office, street, parking lot or facade with lux grids."),
("a1_3","Area, interior and street design"),("a1_3s","Several luminaires, uniformity, EN 12464-1, EN 13201, RP-8 checks."),
("a1_4","Compare and report"),("a1_4s","Two luminaires side by side; a datasheet-style PDF in one click."),
("a2_h","Manufacturers and product data teams"),("a2_w","Validate, convert and publish photometric files without installing anything."),
("a2_1","Validation"),("a2_1s","EN 13032 and IES LM-63 rules with an explanation for every warning."),
("a2_2","Conversion and export"),("a2_2s","LDT, IES, ATLA XML / JSON, SVG, PNG, BIM geometry."),
("a2_3","GLDF with the real body"),("a2_3s","Drop a .gldf and the Light scene renders the manufacturer's L3D model."),
("a2_4","BUG and LCS rating"),("a2_4s","IES TM-15 backlight, uplight and glare classes."),
("a3_h","Horticulture, daylight and dark-sky people"),("a3_w","Spectra, PPFD and the sky: the parts of light that photometry alone does not show."),
("a3_1","Spectral lab"),("a3_1s","CCT, CRI, TM-30, melanopic metrics and PPFD from SPD data; digitize a chart image."),
("a3_2","Greenhouse"),("a3_2s","A traced glasshouse with crop benches: PPFD and daily light integral on the canopy from the luminaire's own spectrum, plus the daylight and glazing demo."),
("a3_3","Daylight sky"),("a3_3s","Sun position, clear and overcast sky, daylight factors anywhere, any date."),
("a3_4","Skyglow"),("a3_4s","Night-sky brightness and light pollution with real star data."),
("a4_h","Teachers, students and developers"),("a4_w","Understand photometry by playing with it, or build on the same Rust crates."),
("a4_1","Lighting quiz"),("a4_1s","175 questions on photometry and design in eight languages."),
("a4_2","Virtual goniophotometer"),("a4_2s","Monte Carlo measurement of a lamp with covers and reflectors; an LDT from a design."),
("a4_3","TM-25 ray files"),("a4_3s","Near-field LED ray sets to far-field solids, spectra and LDT; also at <a href=\"https://tm25.eu\">tm25.eu</a>, with the format handling confirmed by Julius Muschaweck."),
("a4_4","Open source"),("a4_4s","eulumdat-rs on crates.io: parser, diagrams, calculations, FFI for Swift, Kotlin, Python and C#."),
("how_h","How it works"),
("how_1","In the browser"),("how_1s","Rust compiled to WebAssembly; the tracer runs on your GPU through WebGPU, with a CPU fallback."),
("how_2","Your files stay yours"),("how_2s","Nothing is uploaded. A luminaire library lives in your browser's storage only."),
("how_3","Real photometry"),("how_3s","Scenes sample the intensity table itself; grid values match a CPU reference within noise."),
("how_4","Open and free"),("how_4s","AGPL-licensed source on GitHub, library crates on crates.io, GLDF tooling at gldf.icu, ray files at tm25.eu."),
("foot_l","Candelabra · built on <a href=\"https://github.com/holg/eulumdat-rs\">eulumdat-rs</a> · sister sites <a href=\"https://gldf.icu\">gldf.icu</a> and <a href=\"https://tm25.eu\">tm25.eu</a>"),
("foot_open","Open the workbench"),("foot_quiz","Quiz"),("foot_feedback","Feedback"),
])
HTML_KEYS = {"samples","fmt_p","a4_3s","foot_l","know_p","sky_cap"}
TR = json.load(open("crates/eulumdat-wasm/landing/src/landing_tr.json", encoding="utf-8"))
for code, d in TR.items():
    missing = [k for k in EN if k not in d]
    assert not missing, (code, missing)
# Newer sections must be translated, not copies of the English text, and the
# colour-quality intro quotes the demo LED (pinned in
# crates/eulumdat/tests/colour_quality_demo.rs) in every language.
MUST_TRANSLATE = [k for k in EN if k.startswith(("cq_", "sens_", "em_")) or k in ("nav_colour", "nav_sensors", "fmt_sensor", "nav_emergency")]
for code, d in TR.items():
    same = [k for k in MUST_TRANSLATE if d[k] == EN[k]]
    assert not same, (code, "untranslated", same)
    lost = [n for n in ("3000", "82", "98", "11", "10") if n not in d["cq_sub"]]
    assert not lost, (code, "cq_sub lost the demo numbers", lost)
    # sens_pcap quotes tests/sensor_demo.rs::landing_page_classroom_plan.
    lost = [n for n in ("10 × 8", "9", "3 × 3", "98", "11", "15193") if n not in d["sens_pcap"]]
    assert not lost, (code, "sens_pcap lost the demo numbers", lost)
    # em_cap quotes crates/eulumdat/tests/emergency.rs::landing_page_l_office.
    lost = [n for n in ("225", "11", "180", "25", "1838") if n not in d["em_cap"]]
    assert not lost, (code, "em_cap lost the demo numbers", lost)
def t(k): return EN[k]
page = f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title data-i18n="title">{html.escape(t("title"))}</title>
<meta name="description" data-i18n-content="meta" content="{html.escape(t("meta"))}">
<meta property="og:title" content="Candelabra – see any luminaire at work">
<meta property="og:description" content="Free photometric workbench in the browser: LDT / IES editor, diagrams, path-traced light scenes, spectral lab, greenhouse PPFD, goniophotometer, area and street design.">
<meta property="og:type" content="website">
<meta property="og:url" content="{SITE_URL}">
<meta property="og:image" content="{SITE_URL}img/street_render.jpg">
<link rel="canonical" href="{SITE_URL}">
<script>
  // Links from before the landing page existed point at the root with a
  // query or a fragment (?wasm=tm25, ?url=…, #tab=…, #interior?…). Hand them
  // to the app. A plain anchor (#sensors, #sensor-planning) stays here when
  // the page has a section with that id.
  (function () {{
    var q = location.search, h = location.hash;
    var plain = /^#[A-Za-z][\\w-]*$/.test(h);
    if ((q && q.length > 1) || (h && h.length > 1 && !plain)) {{
      location.replace('app/' + q + h);
    }} else if (plain) {{
      document.addEventListener('DOMContentLoaded', function () {{
        if (!document.getElementById(h.slice(1))) location.replace('app/' + h);
      }});
    }}
  }})();
</script>
<style>
  :root {{
    --bg: #f6f7f9; --surface: #ffffff; --text: #16202b; --muted: #5b6673;
    --line: #e3e7ec; --accent: #2563eb; --accent-ink: #ffffff; --tint: #eaf0ff;
    --shadow: 0 10px 30px rgba(16, 24, 40, .10);
  }}
  @media (prefers-color-scheme: dark) {{
    :root:not([data-theme="light"]) {{
      --bg: #0e1116; --surface: #161b22; --text: #e6edf3; --muted: #9aa6b2;
      --line: #262d37; --accent: #5b8def; --accent-ink: #0b1020; --tint: #16213a;
      --shadow: 0 10px 30px rgba(0, 0, 0, .45);
    }}
  }}
  * {{ box-sizing: border-box; }}
  html {{ scroll-behavior: smooth; }}
  body {{ margin: 0; background: var(--bg); color: var(--text); font: 16px/1.55 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }}
  a {{ color: var(--accent); text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  .wrap {{ max-width: 1120px; margin: 0 auto; padding: 0 16px; }}
  header.top {{ display: flex; align-items: center; justify-content: space-between; padding: 14px 0; gap: 10px; }}
  .brand {{ display: flex; align-items: center; gap: 10px; font-weight: 700; font-size: 1.15rem; color: var(--text); }}
  .brand svg {{ width: 28px; height: 28px; }}
  nav.top {{ display: flex; align-items: center; gap: 18px; }}
  nav.top a {{ color: var(--muted); font-size: .95rem; }}
  nav.top select {{ background: var(--surface); color: var(--text); border: 1px solid var(--line); border-radius: 8px; padding: 6px 8px; font-size: .9rem; }}
  .btn {{ display: inline-block; padding: 11px 20px; border-radius: 10px; font-weight: 600; border: 1px solid var(--line); background: var(--surface); color: var(--text); }}
  .btn.primary {{ background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }}
  nav.top a.btn {{ color: var(--accent-ink); }}
  .btn:hover {{ text-decoration: none; filter: brightness(1.05); }}
  .hero {{ display: grid; grid-template-columns: 1.05fr 1fr; gap: 36px; align-items: center; padding: 36px 0 28px; }}
  .hero h1 {{ font-size: clamp(1.9rem, 4vw, 2.9rem); line-height: 1.12; margin: 0 0 14px; letter-spacing: -.01em; }}
  .hero p.lead {{ font-size: 1.1rem; color: var(--muted); margin: 0 0 22px; }}
  .hero .cta {{ display: flex; gap: 10px; flex-wrap: wrap; }}
  .hero .fine {{ margin-top: 12px; font-size: .85rem; color: var(--muted); }}
  .shot {{ position: relative; border-radius: 14px; overflow: hidden; box-shadow: var(--shadow); background: #07080a; aspect-ratio: 16 / 10; }}
  .shot img {{ position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; opacity: 0; transition: opacity 1.2s ease; }}
  .shot img.on {{ opacity: 1; }}
  .shot .cap {{ position: absolute; left: 12px; bottom: 10px; background: rgba(0,0,0,.55); color: #fff; font-size: .78rem; padding: 4px 9px; border-radius: 6px; }}
  section {{ padding: 34px 0; border-top: 1px solid var(--line); }}
  section h2 {{ font-size: 1.5rem; margin: 0 0 6px; }}
  section p.sub {{ color: var(--muted); margin: 0 0 20px; max-width: 760px; }}
  .views {{ display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; }}
  .views figure {{ margin: 0; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; overflow: hidden; }}
  .views img {{ display: block; width: 100%; aspect-ratio: 16 / 10; object-fit: cover; }}
  .views figcaption {{ padding: 10px 12px; font-size: .9rem; }}
  .views figcaption b {{ display: block; }}
  .views figcaption span {{ color: var(--muted); font-size: .82rem; }}
  .cq {{ display: grid; grid-template-columns: 1.15fr 1fr; gap: 24px; align-items: start; }}
  .cq figure {{ margin: 0; }}
  .cq img {{ display: block; width: 100%; border-radius: 12px; border: 1px solid var(--line); background: #fff; }}
  .cq figcaption {{ color: var(--muted); font-size: .84rem; margin-top: 8px; }}
  .cq .how {{ grid-template-columns: 1fr; }}
  .views.plans img {{ aspect-ratio: 1; object-fit: contain; background: #fff; padding: 6px; box-sizing: border-box; }}
  .hl {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 18px; }}
  .know {{ margin-top: 18px; background: var(--tint); border-radius: 12px; padding: 14px 18px; }}
  .know b {{ display: block; margin-bottom: 4px; }}
  .know p {{ margin: 0; color: var(--muted); font-size: .92rem; }}
  .know-pics {{ display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 12px; }}
  .know-pics figure {{ margin: 0; }}
  .know-pics img {{ width: 100%; border-radius: 8px; border: 1px solid var(--line); background: #fff; display: block; }}
  .know-pics figcaption {{ font-size: .8rem; color: var(--muted); margin-top: 4px; text-align: center; }}
  @media (max-width: 600px) {{ .know-pics {{ grid-template-columns: 1fr; }} }}
  .hl article {{ background: var(--surface); border: 1px solid var(--line); border-radius: 14px; padding: 18px 20px; }}
  .hl h3 {{ margin: 0 0 6px; font-size: 1.1rem; }}
  .hl p {{ color: var(--muted); font-size: .92rem; margin: 0 0 12px; }}
  .fmt {{ display: grid; grid-template-columns: 1fr auto 1fr; gap: 10px; align-items: center; }}
  .fmt ul {{ list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px; }}
  .fmt li {{ background: var(--tint); border-radius: 999px; padding: 4px 10px; font-size: .8rem; white-space: nowrap; }}
  .fmt .arrow {{ font-size: 1.6rem; color: var(--accent); text-align: center; }}
  .fmt h4 {{ margin: 0 0 6px; font-size: .8rem; letter-spacing: .08em; text-transform: uppercase; color: var(--muted); }}
  .cmp {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 4px; }}
  .cmp img {{ width: 100%; aspect-ratio: 1; object-fit: cover; border-radius: 8px; background: #fff; border: 1px solid var(--line); }}
  .aud {{ display: grid; grid-template-columns: 1fr 1fr; gap: 28px; }}
  .aud article {{ background: var(--surface); border: 1px solid var(--line); border-radius: 14px; padding: 18px 18px 14px; }}
  .aud h3 {{ margin: 0 0 4px; font-size: 1.1rem; }}
  .aud p.who {{ color: var(--muted); font-size: .88rem; margin: 0 0 12px; }}
  .aud ul {{ list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }}
  .aud li {{ display: grid; grid-template-columns: 24px 1fr; gap: 8px; font-size: .92rem; }}
  .aud li b {{ display: block; }}
  .aud li b a, .views figcaption b a {{ color: var(--text); }}
  .aud li b a:hover, .views figcaption b a:hover {{ color: var(--accent); }}
  .aud li span {{ color: var(--muted); font-size: .84rem; }}
  .aud .pics {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; margin: 12px 0 2px; }}
  .aud .pics img {{ width: 100%; aspect-ratio: 1; object-fit: cover; border-radius: 8px; background: #fff; border: 1px solid var(--line); }}
  .how {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px; }}
  .how div {{ background: var(--tint); border-radius: 12px; padding: 14px 16px; }}
  .how b {{ display: block; margin-bottom: 4px; }}
  .how span {{ color: var(--muted); font-size: .88rem; }}
  .wide {{ margin: 0 0 16px; }}
  .wide img {{ display: block; width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 14px; box-shadow: var(--shadow); background: #07080a; }}
  .wide figcaption {{ color: var(--muted); font-size: .84rem; margin-top: 8px; }}
  footer {{ padding: 26px 0 40px; border-top: 1px solid var(--line); color: var(--muted); font-size: .88rem; display: flex; justify-content: space-between; gap: 16px; flex-wrap: wrap; }}
  @media (max-width: 860px) {{
    .hero {{ grid-template-columns: 1fr; }}
    .aud {{ grid-template-columns: 1fr; }}
    .how {{ grid-template-columns: 1fr 1fr; }}
    .hl {{ grid-template-columns: 1fr; }}
    .fmt {{ grid-template-columns: 1fr; }}
    .fmt .arrow {{ transform: rotate(90deg); }}
    .views {{ grid-template-columns: 1fr; }}
    .cq {{ grid-template-columns: 1fr; }}
    nav.top a.hide-sm {{ display: none; }}
  }}
</style>
</head>
<body>
<div class="wrap">
  <header class="top">
    <a class="brand" href="./">
      <svg viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="12" r="7" fill="#f6c343"/><path d="M6 30 L16 14 L26 30 Z" fill="#2563eb" opacity=".85"/></svg>
      Candelabra
    </a>
    <nav class="top">
      <a class="hide-sm" href="#views" data-i18n="nav_views">{t("nav_views")}</a>
      <a class="hide-sm" href="#formats" data-i18n="nav_formats">{t("nav_formats")}</a>
      <a class="hide-sm" href="#lab">Spectrum Lab</a>
      <a class="hide-sm" href="#colour" data-i18n="nav_colour">{t("nav_colour")}</a>
      <a class="hide-sm" href="#sensors" data-i18n="nav_sensors">{t("nav_sensors")}</a>
      <a class="hide-sm" href="#emergency" data-i18n="nav_emergency">{t("nav_emergency")}</a>
      <a class="hide-sm" href="#for" data-i18n="nav_for">{t("nav_for")}</a>
      <a class="hide-sm" href="https://github.com/holg/eulumdat-rs" data-i18n="nav_source">{t("nav_source")}</a>
      <select id="lang" aria-label="Language">
        <option value="en">English</option><option value="de">Deutsch</option><option value="fr">Français</option><option value="es">Español</option><option value="it">Italiano</option><option value="pt-BR">Português</option><option value="ru">Русский</option><option value="zh">中文</option>
      </select>
      <a class="btn primary" href="app/" data-i18n="nav_open">{t("nav_open")}</a>
    </nav>
  </header>

  <div class="hero">
    <div>
      <h1 data-i18n="h1">{t("h1")}</h1>
      <p class="lead" data-i18n="lead">{t("lead")}</p>
      <div class="cta">
        <a class="btn primary" href="app/" data-i18n="cta_open">{t("cta_open")}</a>
        <a class="btn" href="app/#tab=light-scene&amp;sample=road-luminaire&amp;scene=street" data-i18n="cta_street">{t("cta_street")}</a>
      </div>
      <p class="fine" data-i18n="fine">{t("fine")}</p>
    </div>
    <div class="shot" id="hero">
      <img class="on" src="img/office_render.jpg" alt="">
      <img src="img/street_render.jpg" alt="">
      <img src="img/greenhouse_ppfd.jpg" alt="">
      <img src="img/office_points.jpg" alt="">
      <img src="img/street_isolux.jpg" alt="">
      <div class="cap" id="cap">{t("cap_office")}</div>
    </div>
  </div>

  <section id="views">
    <h2 data-i18n="views_h">{t("views_h")}</h2>
    <p class="sub" data-i18n="views_sub">{t("views_sub")}</p>
    <p class="sub" style="margin-top:-8px" data-i18n="samples">{t("samples")}</p>
    <div class="views">
      <figure><img src="img/street_render.jpg" alt=""><figcaption><b><a href="app/#tab=light-scene&amp;sample=road-luminaire&amp;scene=street" data-i18n="v_photo">{t("v_photo")}</a></b><span data-i18n="v_photo_s">{t("v_photo_s")}</span></figcaption></figure>
      <figure><img src="img/street_isolux.jpg" alt=""><figcaption><b><a href="app/#tab=light-scene&amp;sample=road-luminaire&amp;scene=street&amp;mode=isolux" data-i18n="v_iso">{t("v_iso")}</a></b><span data-i18n="v_iso_s">{t("v_iso_s")}</span></figcaption></figure>
      <figure><img src="img/office_points.jpg" alt=""><figcaption><b><a href="app/#tab=light-scene&amp;sample=fluorescent-luminaire&amp;scene=office&amp;mode=points" data-i18n="v_grid">{t("v_grid")}</a></b><span data-i18n="v_grid_s">{t("v_grid_s")}</span></figcaption></figure>
      <figure><img src="img/greenhouse_render.jpg" alt=""><figcaption><b><a href="app/#tab=light-scene&amp;sample=tm-33-23-horticultural-led&amp;scene=greenhouse" data-i18n="v_gh">{t("v_gh")}</a></b><span data-i18n="v_gh_s">{t("v_gh_s")}</span></figcaption></figure>
      <figure><img src="img/greenhouse_ppfd.jpg" alt=""><figcaption><b><a href="app/#tab=light-scene&amp;sample=tm-33-23-horticultural-led&amp;scene=greenhouse&amp;mode=isolux" data-i18n="v_ppfd">{t("v_ppfd")}</a></b><span data-i18n="v_ppfd_s">{t("v_ppfd_s")}</span></figcaption></figure>
      <figure><img src="img/facade_render.jpg" alt=""><figcaption><b><a href="app/#tab=light-scene&amp;sample=projector&amp;scene=facade" data-i18n="v_facade">{t("v_facade")}</a></b><span data-i18n="v_facade_s">{t("v_facade_s")}</span></figcaption></figure>
    </div>
  </section>

  <section id="formats">
    <h2 data-i18n="fmt_h">{t("fmt_h")}</h2>
    <div class="hl">
      <article>
        <h3 data-i18n="fmt_t">{t("fmt_t")}</h3>
        <p data-i18n="fmt_p">{t("fmt_p")}</p>
        <div class="fmt">
          <div>
            <h4 data-i18n="fmt_in">{t("fmt_in")}</h4>
            <ul><li>EULUMDAT .ldt</li><li>IES LM-63</li><li>ATLA / TM-33 XML, JSON</li><li>GLDF</li><li>OxyTech OXL / OXC</li><li>SPDX TM-27</li><li data-i18n="fmt_spd">{t("fmt_spd")}</li><li data-i18n="fmt_ray">{t("fmt_ray")}</li><li data-i18n="fmt_sensor">{t("fmt_sensor")}</li><li data-i18n="fmt_images">{t("fmt_images")}</li></ul>
          </div>
          <div class="arrow">→</div>
          <div>
            <h4 data-i18n="fmt_out">{t("fmt_out")}</h4>
            <ul><li>EULUMDAT .ldt</li><li>IES LM-63</li><li>ATLA XML, JSON</li><li>SVG, PNG, JPEG</li><li>PDF, Typst</li><li data-i18n="fmt_bim">{t("fmt_bim")}</li><li><a href="https://gldf.icu" data-i18n="fmt_gldf_out">{t("fmt_gldf_out")}</a></li></ul>
          </div>
        </div>
      </article>
      <article>
        <h3 data-i18n="cmp_t">{t("cmp_t")}</h3>
        <p data-i18n="cmp_p">{t("cmp_p")}</p>
        <div class="cmp"><img src="img/polar.svg" alt=""><img src="img/cartesian.svg" alt=""></div>
        <p style="margin:10px 0 0"><a class="btn" href="app/#tab=compare" data-i18n="cmp_link">{t("cmp_link")}</a></p>
      </article>
      <article>
        <h3 data-i18n="street_t">{t("street_t")}</h3>
        <p data-i18n="street_p">{t("street_p")}</p>
        <p style="margin:0"><a class="btn" href="app/#tab=street" data-i18n="street_link">{t("street_link")}</a></p>
      </article>
    </div>
    <div class="know">
      <b data-i18n="know_t">{t("know_t")}</b>
      <p data-i18n="know_p">{t("know_p")}</p>
      <div class="know-pics">
        <figure><a href="app/#tab=compare&amp;sample=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-cie&amp;compare=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-iesna&amp;diagram=isocandela"><img src="img/maxwell_cie.jpg" alt=""></a><figcaption data-i18n="know_cie">{t("know_cie")}</figcaption></figure>
        <figure><a href="app/#tab=compare&amp;sample=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-cie&amp;compare=aec-maxwell-8-t4-luxeon-5050-square-with-glass-measured-iesna&amp;diagram=isocandela"><img src="img/maxwell_iesna.jpg" alt=""></a><figcaption data-i18n="know_iesna">{t("know_iesna")}</figcaption></figure>
      </div>
    </div>
  </section>

  <section id="lab">
    <h2 data-i18n="lab_h">{t("lab_h")}</h2>
    <p class="sub" data-i18n="lab_sub">{t("lab_sub")}</p>
    <div class="how">
      <div><b data-i18n="lab_1">{t("lab_1")}</b><span data-i18n="lab_1s">{t("lab_1s")}</span></div>
      <div><b data-i18n="lab_2">{t("lab_2")}</b><span data-i18n="lab_2s">{t("lab_2s")}</span></div>
      <div><b data-i18n="lab_3">{t("lab_3")}</b><span data-i18n="lab_3s">{t("lab_3s")}</span></div>
      <div><b data-i18n="lab_4">{t("lab_4")}</b><span data-i18n="lab_4s">{t("lab_4s")}</span></div>
    </div>
    <p class="sub" style="margin-top:14px" data-i18n="lab_note">{t("lab_note")}</p>
    <p style="margin:0"><a class="btn primary" href="app/#tab=spectrum-lab" data-i18n="lab_link">{t("lab_link")}</a> <a class="btn" href="app/#tab=daylight">Daylight &amp; Sky</a></p>
  </section>

  <section id="colour">
    <h2 data-i18n="cq_h">{t("cq_h")}</h2>
    <p class="sub" data-i18n="cq_sub">{t("cq_sub")}</p>
    <div class="cq">
      <figure><a href="app/#tab=spectral&amp;sample=demo-led-3000k&amp;spectralview=colour-quality"><img src="img/colour_quality.svg" alt="TM-30 colour vector graphic, Duv and CRI of the demo LED" loading="lazy"></a><figcaption data-i18n="cq_cap">{t("cq_cap")}</figcaption></figure>
      <div>
        <div class="how">
          <div><b data-i18n="cq_1">{t("cq_1")}</b><span data-i18n="cq_1s">{t("cq_1s")}</span></div>
          <div><b data-i18n="cq_2">{t("cq_2")}</b><span data-i18n="cq_2s">{t("cq_2s")}</span></div>
          <div><b data-i18n="cq_3">{t("cq_3")}</b><span data-i18n="cq_3s">{t("cq_3s")}</span></div>
          <div><b data-i18n="cq_4">{t("cq_4")}</b><span data-i18n="cq_4s">{t("cq_4s")}</span></div>
        </div>
        <p style="margin:16px 0 0"><a class="btn primary" href="app/#tab=spectral&amp;sample=demo-led-3000k&amp;spectralview=colour-quality" data-i18n="cq_link">{t("cq_link")}</a> <a class="btn" href="app/#tab=spectral&amp;sample=demo-led-3000k&amp;spectralview=tm30-hue" data-i18n="cq_link2">{t("cq_link2")}</a></p>
      </div>
    </div>
  </section>

  <section id="sensors">
    <h2 data-i18n="sens_h">{t("sens_h")}</h2>
    <p class="sub" data-i18n="sens_sub">{t("sens_sub")}</p>
    <div class="views plans">
      <figure><a href="app/#tab=sensor&amp;sensor=demo"><img src="img/sensor_ceiling.svg" alt="" loading="lazy"></a><figcaption><b data-i18n="sens_c1">{t("sens_c1")}</b><span data-i18n="sens_c1s">{t("sens_c1s")}</span></figcaption></figure>
      <figure><a href="app/#tab=sensor&amp;sensor=demo"><img src="img/sensor_corridor.svg" alt="" loading="lazy"></a><figcaption><b data-i18n="sens_c2">{t("sens_c2")}</b><span data-i18n="sens_c2s">{t("sens_c2s")}</span></figcaption></figure>
      <figure><a href="app/#tab=sensor&amp;sensor=demo"><img src="img/sensor_wall.svg" alt="" loading="lazy"></a><figcaption><b data-i18n="sens_c3">{t("sens_c3")}</b><span data-i18n="sens_c3s">{t("sens_c3s")}</span></figcaption></figure>
    </div>
    <div class="how" style="margin-top:14px">
      <div><b data-i18n="sens_1">{t("sens_1")}</b><span data-i18n="sens_1s">{t("sens_1s")}</span></div>
      <div><b data-i18n="sens_2">{t("sens_2")}</b><span data-i18n="sens_2s">{t("sens_2s")}</span></div>
      <div><b data-i18n="sens_3">{t("sens_3")}</b><span data-i18n="sens_3s">{t("sens_3s")}</span></div>
      <div><b data-i18n="sens_4">{t("sens_4")}</b><span data-i18n="sens_4s">{t("sens_4s")}</span></div>
    </div>
    <p class="sub" style="margin-top:14px" data-i18n="sens_note">{t("sens_note")}</p>
    <p style="margin:0"><a class="btn primary" href="app/#tab=sensor&amp;sensor=demo" data-i18n="sens_link">{t("sens_link")}</a> <a class="btn" href="app/#tab=sensor&amp;sensor=demo&amp;sensorview=3d" data-i18n="sens_link3d">{t("sens_link3d")}</a></p>
    <h3 id="sensor-planning" style="margin:30px 0 6px" data-i18n="sens_ph">{t("sens_ph")}</h3>
    <p class="sub" data-i18n="sens_psub">{t("sens_psub")}</p>
    <div class="cq">
      <figure><a href="app/#interior?preset=2&amp;sample=demo-led-3000k&amp;sensor=demo0"><img src="img/sensor_planning.svg" alt="Classroom plan with nine demo ceiling detectors, their detection areas and the uncovered spots" loading="lazy"></a><figcaption data-i18n="sens_pcap">{t("sens_pcap")}</figcaption></figure>
      <div>
        <div class="how">
          <div><b data-i18n="sens_p1">{t("sens_p1")}</b><span data-i18n="sens_p1s">{t("sens_p1s")}</span></div>
          <div><b data-i18n="sens_p2">{t("sens_p2")}</b><span data-i18n="sens_p2s">{t("sens_p2s")}</span></div>
          <div><b data-i18n="sens_p3">{t("sens_p3")}</b><span data-i18n="sens_p3s">{t("sens_p3s")}</span></div>
          <div><b data-i18n="sens_p4">{t("sens_p4")}</b><span data-i18n="sens_p4s">{t("sens_p4s")}</span></div>
        </div>
        <p style="margin:16px 0 0"><a class="btn primary" href="app/#interior?preset=2&amp;sample=demo-led-3000k&amp;sensor=demo0" data-i18n="sens_plink">{t("sens_plink")}</a></p>
      </div>
    </div>
  </section>

  <section id="emergency">
    <h2 data-i18n="em_h">{t("em_h")}</h2>
    <p class="sub" data-i18n="em_sub">{t("em_sub")}</p>
    <div class="cq">
      <figure><a href="app/#interior?preset=0&amp;sample=demo-led-3000k&amp;sensor=none&amp;shape=0,0;1,0;1,0.5;0.5,0.5;0.5,1;0,1&amp;em=1&amp;eml=route&amp;emr=19.5,3.75;5,3.75;5,14.5~2&amp;emp=fe:15,7.4,270_fa:0.1,10,0_ex:20,3.75,180_ex:5,15,270"><img src="img/emergency_plan.svg" alt="L-shaped office plan with an escape route around the corner, emergency luminaires, a fire extinguisher, first aid and two exits" loading="lazy"></a><figcaption data-i18n="em_cap">{t("em_cap")}</figcaption></figure>
      <div>
        <div class="how">
          <div><b data-i18n="em_1">{t("em_1")}</b><span data-i18n="em_1s">{t("em_1s")}</span></div>
          <div><b data-i18n="em_2">{t("em_2")}</b><span data-i18n="em_2s">{t("em_2s")}</span></div>
          <div><b data-i18n="em_3">{t("em_3")}</b><span data-i18n="em_3s">{t("em_3s")}</span></div>
          <div><b data-i18n="em_4">{t("em_4")}</b><span data-i18n="em_4s">{t("em_4s")}</span></div>
        </div>
        <p style="margin:16px 0 0"><a class="btn primary" href="app/#interior?preset=0&amp;sample=demo-led-3000k&amp;sensor=none&amp;shape=0,0;1,0;1,0.5;0.5,0.5;0.5,1;0,1&amp;em=1&amp;eml=route&amp;emr=19.5,3.75;5,3.75;5,14.5~2&amp;emp=fe:15,7.4,270_fa:0.1,10,0_ex:20,3.75,180_ex:5,15,270" data-i18n="em_link">{t("em_link")}</a> <a class="btn" href="app/#interior?preset=2&amp;sample=demo-led-3000k&amp;sensor=none&amp;em=1" data-i18n="em_link2">{t("em_link2")}</a></p>
        <p class="sub" style="margin-top:14px" data-i18n="em_note">{t("em_note")}</p>
      </div>
    </div>
  </section>

  <section id="skyglow">
    <h2 data-i18n="sky_h">{t("sky_h")}</h2>
    <p class="sub" data-i18n="sky_sub">{t("sky_sub")}</p>
    <figure class="wide"><a href="app/?wasm=skyglow_demo&amp;scene=vischering"><img src="img/skyglow_vischering.jpg" alt="Burg Vischering at night in the Skyglow demo" loading="lazy"></a><figcaption data-i18n="sky_cap">{t("sky_cap")}</figcaption></figure>
    <p style="margin:0"><a class="btn primary" href="app/?wasm=skyglow_demo&amp;scene=vischering" data-i18n="sky_link">{t("sky_link")}</a></p>
  </section>

  <section id="for">
    <h2 data-i18n="for_h">{t("for_h")}</h2>
    <p class="sub" data-i18n="for_sub">{t("for_sub")}</p>
    <div class="aud">
      <article>
        <h3 data-i18n="a1_h">{t("a1_h")}</h3>
        <p class="who" data-i18n="a1_w">{t("a1_w")}</p>
        <div class="pics"><img src="img/polar.svg" alt=""><img src="img/cartesian.svg" alt=""><img src="img/heatmap.svg" alt=""><img src="img/butterfly.svg" alt=""></div>
        <ul>
          <li><span>📈</span><div><b><a href="app/#tab=polar" data-i18n="a1_1">{t("a1_1")}</a></b><span data-i18n="a1_1s">{t("a1_1s")}</span></div></li>
          <li><span>💡</span><div><b><a href="app/#tab=light-scene" data-i18n="a1_2">{t("a1_2")}</a></b><span data-i18n="a1_2s">{t("a1_2s")}</span></div></li>
          <li><span>📐</span><div><b><a href="app/#tab=street" data-i18n="a1_3">{t("a1_3")}</a></b><span data-i18n="a1_3s">{t("a1_3s")}</span></div></li>
          <li><span>🖨️</span><div><b><a href="app/#tab=compare" data-i18n="a1_4">{t("a1_4")}</a></b><span data-i18n="a1_4s">{t("a1_4s")}</span></div></li>
        </ul>
      </article>
      <article>
        <h3 data-i18n="a2_h">{t("a2_h")}</h3>
        <p class="who" data-i18n="a2_w">{t("a2_w")}</p>
        <div class="pics"><img src="img/lcs.svg" alt=""><img src="img/cone.svg" alt=""><img src="img/parking_render.jpg" alt=""><img src="img/facade_render.jpg" alt=""></div>
        <ul>
          <li><span>✅</span><div><b><a href="app/#tab=validation" data-i18n="a2_1">{t("a2_1")}</a></b><span data-i18n="a2_1s">{t("a2_1s")}</span></div></li>
          <li><span>🔁</span><div><b><a href="app/#tab=bim" data-i18n="a2_2">{t("a2_2")}</a></b><span data-i18n="a2_2s">{t("a2_2s")}</span></div></li>
          <li><span>🗃️</span><div><b><a href="app/#tab=light-scene" data-i18n="a2_3">{t("a2_3")}</a></b><span data-i18n="a2_3s">{t("a2_3s")}</span></div></li>
          <li><span>🐞</span><div><b><a href="app/#tab=bug" data-i18n="a2_4">{t("a2_4")}</a></b><span data-i18n="a2_4s">{t("a2_4s")}</span></div></li>
        </ul>
      </article>
      <article>
        <h3 data-i18n="a3_h">{t("a3_h")}</h3>
        <p class="who" data-i18n="a3_w">{t("a3_w")}</p>
        <div class="pics"><img src="img/spectral.svg" alt=""><img src="img/greenhouse.svg" alt=""><img src="img/greenhouse_render.jpg" alt=""><img src="img/greenhouse_ppfd.jpg" alt=""></div>
        <ul>
          <li><span>🌈</span><div><b><a href="app/#tab=spectral" data-i18n="a3_1">{t("a3_1")}</a></b><span data-i18n="a3_1s">{t("a3_1s")}</span></div></li>
          <li><span>🌱</span><div><b><a href="app/#tab=light-scene&amp;sample=tm-33-23-horticultural-led&amp;scene=greenhouse&amp;mode=isolux" data-i18n="a3_2">{t("a3_2")}</a></b><span data-i18n="a3_2s">{t("a3_2s")}</span></div></li>
          <li><span>☀️</span><div><b><a href="app/#tab=daylight" data-i18n="a3_3">{t("a3_3")}</a></b><span data-i18n="a3_3s">{t("a3_3s")}</span></div></li>
          <li><span>🌌</span><div><b><a href="app/?wasm=skyglow_demo" data-i18n="a3_4">{t("a3_4")}</a></b><span data-i18n="a3_4s">{t("a3_4s")}</span></div></li>
        </ul>
      </article>
      <article>
        <h3 data-i18n="a4_h">{t("a4_h")}</h3>
        <p class="who" data-i18n="a4_w">{t("a4_w")}</p>
        <ul>
          <li><span>🎓</span><div><b><a href="app/quiz.html" data-i18n="a4_1">{t("a4_1")}</a></b><span data-i18n="a4_1s">{t("a4_1s")}</span></div></li>
          <li><span>🔬</span><div><b><a href="app/#tab=goniosim" data-i18n="a4_2">{t("a4_2")}</a></b><span data-i18n="a4_2s">{t("a4_2s")}</span></div></li>
          <li><span>✨</span><div><b><a href="app/?wasm=tm25" data-i18n="a4_3">{t("a4_3")}</a></b><span data-i18n="a4_3s">{t("a4_3s")}</span></div></li>
          <li><span>🦀</span><div><b><a href="https://github.com/holg/eulumdat-rs" data-i18n="a4_4">{t("a4_4")}</a></b><span data-i18n="a4_4s">{t("a4_4s")}</span></div></li>
        </ul>
      </article>
    </div>
  </section>

  <section>
    <h2 data-i18n="how_h">{t("how_h")}</h2>
    <div class="how">
      <div><b data-i18n="how_1">{t("how_1")}</b><span data-i18n="how_1s">{t("how_1s")}</span></div>
      <div><b data-i18n="how_2">{t("how_2")}</b><span data-i18n="how_2s">{t("how_2s")}</span></div>
      <div><b data-i18n="how_3">{t("how_3")}</b><span data-i18n="how_3s">{t("how_3s")}</span></div>
      <div><b data-i18n="how_4">{t("how_4")}</b><span data-i18n="how_4s">{t("how_4s")}</span></div>
    </div>
  </section>

  <footer>
    <div data-i18n="foot_l">{t("foot_l")}</div>
    <div><a href="app/" data-i18n="foot_open">{t("foot_open")}</a> · <a href="app/quiz.html" data-i18n="foot_quiz">{t("foot_quiz")}</a> · <a href="https://github.com/holg/eulumdat-rs/issues" data-i18n="foot_feedback">{t("foot_feedback")}</a></div>
  </footer>
</div>
<script>
  // Translations for the other seven languages; English is in the markup.
  var I18N = {json.dumps(TR, ensure_ascii=False)};
  var HTML_KEYS = {json.dumps(sorted(HTML_KEYS))};
  var CAPS = {json.dumps([EN["cap_office"],EN["cap_street"],EN["cap_gh"],EN["cap_grid"],EN["cap_iso"]], ensure_ascii=False)};
  var CAP_KEYS = ["cap_office","cap_street","cap_gh","cap_grid","cap_iso"];
  var CAPS_EN = CAPS.slice();
  var EN_TEXT = {{}};
  document.querySelectorAll('[data-i18n]').forEach(function (el) {{ EN_TEXT[el.getAttribute('data-i18n')] = el.innerHTML; }});
  var META = document.querySelector('[data-i18n-content]');
  var EN_META = META ? META.getAttribute('content') : '';
  function pick() {{
    var stored = null;
    try {{ stored = localStorage.getItem('eulumdat_language'); }} catch (e) {{}}
    if (stored && (stored === 'en' || I18N[stored])) return stored;
    var nav = (navigator.language || 'en');
    if (I18N[nav]) return nav;
    var short = nav.slice(0, 2).toLowerCase();
    if (short === 'pt') return 'pt-BR';
    return I18N[short] ? short : 'en';
  }}
  function apply(code) {{
    var d = I18N[code] || null;
    document.documentElement.lang = code === 'pt-BR' ? 'pt-BR' : code;
    document.querySelectorAll('[data-i18n]').forEach(function (el) {{
      var k = el.getAttribute('data-i18n');
      var v = d ? d[k] : EN_TEXT[k];
      if (v === undefined) return;
      if (HTML_KEYS.indexOf(k) >= 0 || !d) el.innerHTML = v; else el.textContent = v;
    }});
    if (META) META.setAttribute('content', d ? d.meta : EN_META);
    document.title = d ? d.title : EN_TEXT.title;
    var sel = document.getElementById('lang'); if (sel) sel.value = code;
    // English captions are not in the markup (only the first one is), so
    // fall back to the English list, never to the key.
    CAPS = CAP_KEYS.map(function (k, i) {{ return (d && d[k]) || CAPS_EN[i]; }});
    document.getElementById('cap').textContent = CAPS[capIndex];
  }}
  var capIndex = 0;
  document.getElementById('lang').addEventListener('change', function (e) {{
    try {{ localStorage.setItem('eulumdat_language', e.target.value); }} catch (err) {{}}
    apply(e.target.value);
  }});
  apply(pick());
  // Hero slideshow.
  (function () {{
    var imgs = document.querySelectorAll('#hero img');
    setInterval(function () {{
      imgs[capIndex].classList.remove('on'); capIndex = (capIndex + 1) % imgs.length; imgs[capIndex].classList.add('on');
      document.getElementById('cap').textContent = CAPS[capIndex];
    }}, 3600);
  }})();
</script>
</body>
</html>
'''
open("crates/eulumdat-wasm/landing/index.html","w",encoding="utf-8").write(page)
print("landing generated", len(EN), "keys")
