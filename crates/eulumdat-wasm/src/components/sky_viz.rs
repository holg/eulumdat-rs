//! Sky visualisations for the Spectrum Lab: pure SVG generators that turn a
//! location + date + time into pictures a lighting designer actually reads —
//! a sky-dome colour preview, a sun-path arc, a 24-hour day/night timeline
//! strip, and a stylised sky-lit ground scene.
//!
//! Everything here is a pure function `(inputs) -> String` (an SVG document),
//! so it is trivially unit-testable and reactive: the Leptos component just
//! re-invokes these on any signal change and drops the SVG into the DOM.

use eulumdat_daylight::availability::daylight_cct;
use eulumdat_daylight::location::{LocalDateTime, NamedLocation};
use eulumdat_daylight::DaylightAvailability;

/// Correlated-colour-temperature → sRGB hex, over the ~1500–15000 K range.
///
/// A compact fit (Neil Bartlett's approximation) good enough for on-screen sky
/// and lamp colour swatches. Warm CCTs are amber, ~6500 K is white, high CCTs
/// are blue.
pub fn kelvin_to_hex(kelvin: f64) -> String {
    let t = (kelvin / 100.0).clamp(10.0, 400.0);

    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2)
    };
    let g = if t <= 66.0 {
        99.470_802_586_1 * t.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };

    format!(
        "#{:02x}{:02x}{:02x}",
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8
    )
}

/// Blend two `#rrggbb` hex colours by `t` ∈ [0,1] (0 = a, 1 = b).
fn blend_hex(a: &str, b: &str, t: f64) -> String {
    let parse = |s: &str| {
        let s = s.trim_start_matches('#');
        (
            u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f64,
            u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f64,
            u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f64,
        )
    };
    let (ar, ag, ab) = parse(a);
    let (br, bg, bb) = parse(b);
    let t = t.clamp(0.0, 1.0);
    format!(
        "#{:02x}{:02x}{:02x}",
        (ar + (br - ar) * t) as u8,
        (ag + (bg - ag) * t) as u8,
        (ab + (bb - ab) * t) as u8,
    )
}

/// Brightness factor 0..1 from sun altitude, used to darken sky colours toward
/// night. Smooth through civil twilight (−6°) so the timeline fades naturally.
fn brightness(altitude_deg: f64) -> f64 {
    if altitude_deg >= 6.0 {
        1.0
    } else if altitude_deg <= -12.0 {
        0.02 // starlit floor, not pure black
    } else {
        // Map [−12°, 6°] → [0.02, 1.0] smoothly.
        let t = (altitude_deg + 12.0) / 18.0;
        0.02 + 0.98 * t * t
    }
}

/// Darken a hex colour by a 0..1 factor (1 = unchanged).
fn darken(hex: &str, factor: f64) -> String {
    blend_hex("#05070f", hex, factor.clamp(0.0, 1.0))
}

/// The zenith and horizon sky colours for a sun altitude + turbidity.
/// Returns `(zenith_hex, horizon_hex)`.
fn sky_colours(altitude_deg: f64, turbidity: f64) -> (String, String) {
    let f = brightness(altitude_deg);
    if altitude_deg <= -6.0 {
        // Deep night: dark navy zenith, faint warm glow at the horizon.
        return (darken("#0a1430", f), darken("#241830", f));
    }
    // Daylight/twilight: zenith uses the (bluer) daylight CCT, the horizon is
    // ~1500 K warmer (longer air path → more scattering out of blue).
    let cct = daylight_cct(altitude_deg, turbidity);
    let zenith = kelvin_to_hex(cct.min(12000.0));
    // A believable sky is bluer than a blackbody of the same CCT; nudge zenith
    // toward sky-blue, horizon toward warm haze.
    let zenith = blend_hex(&zenith, "#4a90d9", 0.45);
    let horizon_cct = (cct - 1800.0).max(3200.0);
    let horizon = blend_hex(&kelvin_to_hex(horizon_cct), "#e8c9a0", 0.35);
    (darken(&zenith, f), darken(&horizon, f))
}

/// Render a phase-correct moon disc at `(cx, cy)` of radius `r`.
///
/// The lit face is drawn as the moon disc; the shadow is a dark ellipse whose
/// width encodes the phase (full = no shadow, new = full shadow, crescent =
/// wide shadow). `illuminated` is the lit fraction 0..1; `waxing` puts the lit
/// limb on the correct side (Northern-hemisphere convention: waxing lit on the
/// right).
pub fn moon_disc_svg(cx: f64, cy: f64, r: f64, illuminated: f64, waxing: bool) -> String {
    let f = illuminated.clamp(0.0, 1.0);
    // The terminator is an ellipse; its horizontal semi-axis goes from +r (new,
    // full shadow across) to 0 (quarter) to −r (full, shadow flipped away).
    // Map lit fraction to the terminator offset: k = cos(phase) where lit=(1+k)/2.
    let k = 2.0 * f - 1.0; // −1 new … +1 full
    let term_rx = (r * k).abs();
    // Base disc (lit colour).
    let disc = format!(
        r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="#eef1f6" opacity="0.95"/>"##
    );
    if f > 0.97 {
        // Full moon: just the disc with a soft glow.
        return format!(
            r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{gr:.1}" fill="#eef1f6" opacity="0.18"/>{disc}"##,
            gr = r * 1.9
        );
    }
    if f < 0.03 {
        // New moon: a faint outline only (earthshine), essentially dark.
        return format!(
            r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="#20242c" stroke="#3a4048" stroke-width="0.5" opacity="0.6"/>"##
        );
    }
    // Shadow: composed of a half-disc plus a terminator ellipse. We approximate
    // with a clipped shadow: draw the full disc, then overlay a shadow shape.
    // The shadow covers the un-lit side. For a waxing moon the RIGHT is lit, so
    // the shadow is on the left; for waning, mirror.
    let shadow_on_left = waxing; // waxing → lit right → shadow left
    // Build the shadow as a path: outer semicircle (shadow side) + inner
    // terminator ellipse arc. Sweep flags flip with phase (gibbous vs crescent).
    let gibbous = f > 0.5; // >half lit → shadow is a crescent sliver
    let sx = if shadow_on_left { -1.0 } else { 1.0 };
    let top = format!("{:.1},{:.1}", cx, cy - r);
    let bot = format!("{:.1},{:.1}", cx, cy + r);
    // Outer arc goes around the shadow side (semicircle).
    // sweep of outer semicircle: pick so it bulges to the shadow side.
    let outer_sweep = if shadow_on_left { 0 } else { 1 };
    // Inner terminator: an elliptical arc of horizontal radius term_rx.
    // For a crescent (f<0.5) the terminator bulges toward the lit side (away
    // from shadow); for gibbous it bulges toward the shadow side.
    let inner_sweep = if gibbous == shadow_on_left { 1 } else { 0 };
    let _ = sx;
    let shadow = format!(
        r##"<path d="M {top} A {r:.1} {r:.1} 0 0 {outer_sweep} {bot} A {term_rx:.1} {r:.1} 0 0 {inner_sweep} {top} Z" fill="#12151b" opacity="0.9"/>"##
    );
    format!("{disc}{shadow}")
}

// ── 1. Sky-dome colour preview ───────────────────────────────────────────

/// An SVG half-dome showing the sky gradient (zenith → horizon) with the sun
/// disc placed at its real screen position (azimuth across, altitude up).
///
/// `w`/`h` are the SVG viewport size. The dome fills the width; the sun sits on
/// a hemisphere projection so its height tracks altitude and its horizontal
/// position tracks azimuth (South = centre).
pub fn sky_dome_svg(
    dt: &LocalDateTime,
    loc: &NamedLocation,
    turbidity: f64,
    w: f64,
    h: f64,
) -> String {
    let sun = dt.solar_position(loc);
    let alt = sun.altitude_deg();
    let az = sun.azimuth_deg();
    let (zenith, horizon) = sky_colours(alt, turbidity);

    // Sun screen position: azimuth 90..270 (E..W through S) maps left..right;
    // altitude 0..90 maps bottom..top of the dome.
    let az_frac = ((az - 90.0) / 180.0).clamp(-0.15, 1.15); // allow slight overhang
    let sun_x = w * az_frac;
    let ground_y = h * 0.86;
    let sun_y = ground_y - (alt.clamp(-5.0, 90.0) / 90.0) * (ground_y - h * 0.08);

    let sun_visible = alt > -2.0;
    let sun_color = if alt > 8.0 {
        "#fff6d8"
    } else if alt > 0.0 {
        "#ffcf6b" // low sun: warm
    } else {
        "#ff9d5c"
    };
    let glow_r = if alt > 8.0 { 26.0 } else { 34.0 };

    // Real moon at its computed position + phase, drawn when the sky is dark
    // enough to see it and it is above the horizon.
    let moon = {
        let m = dt.moon_position(loc);
        if alt < 2.0 && m.is_up() {
            let m_az_frac = ((m.azimuth_deg() - 90.0) / 180.0).clamp(-0.1, 1.1);
            let mx = w * m_az_frac;
            let my = ground_y - (m.altitude_deg().clamp(0.0, 90.0) / 90.0) * (ground_y - h * 0.08);
            moon_disc_svg(mx, my, 12.0, m.illuminated_fraction, m.waxing)
        } else {
            String::new()
        }
    };

    let sun_svg = if sun_visible {
        format!(
            r##"<circle cx="{sun_x:.1}" cy="{sun_y:.1}" r="{glow_r:.0}" fill="{sun_color}" opacity="0.30"/>
               <circle cx="{sun_x:.1}" cy="{sun_y:.1}" r="14" fill="{sun_color}"/>"##
        )
    } else {
        String::new()
    };

    format!(
        r##"<svg viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto;border-radius:8px;display:block;">
  <defs>
    <linearGradient id="skyGrad" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="{zenith}"/>
      <stop offset="100%" stop-color="{horizon}"/>
    </linearGradient>
  </defs>
  <rect x="0" y="0" width="{w}" height="{ground_y:.1}" fill="url(#skyGrad)"/>
  {moon}
  {sun_svg}
  <rect x="0" y="{ground_y:.1}" width="{w}" height="{gh:.1}" fill="#20262e"/>
  <line x1="0" y1="{ground_y:.1}" x2="{w}" y2="{ground_y:.1}" stroke="#39404a" stroke-width="1"/>
</svg>"##,
        gh = h - ground_y,
    )
}

// ── 2. Sun-path arc (altitude vs hour) ───────────────────────────────────

/// An SVG chart of the sun's altitude across the 24 h of the chosen date at the
/// location, with the horizon line, the current-time marker, and a shaded
/// daytime region. The classic architectural sun-path chart.
pub fn sun_path_svg(dt: &LocalDateTime, loc: &NamedLocation, w: f64, h: f64) -> String {
    let pad_l = 34.0;
    let pad_b = 22.0;
    let pad_t = 10.0;
    let plot_w = w - pad_l - 8.0;
    let plot_h = h - pad_b - pad_t;

    // altitude −90..90 → y (top = 90). We show −10..90 for a tighter plot.
    let alt_to_y = |alt: f64| pad_t + (90.0 - alt.clamp(-10.0, 90.0)) / 100.0 * plot_h;
    let hour_to_x = |hh: f64| pad_l + (hh / 24.0) * plot_w;
    let horizon_y = alt_to_y(0.0);

    // Sample altitude every 15 min.
    let mut pts = Vec::new();
    let steps = 96;
    for i in 0..=steps {
        let hh = i as f64 / steps as f64 * 24.0;
        let s = LocalDateTime::new(dt.year, dt.month, dt.day, hh).solar_position(loc);
        pts.push((hour_to_x(hh), alt_to_y(s.altitude_deg())));
    }
    let path: String = pts
        .iter()
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x:.1},{y:.1}", if i == 0 { "M" } else { "L" }))
        .collect::<Vec<_>>()
        .join(" ");

    // Current-time marker.
    let now_x = hour_to_x(dt.local_hours);
    let now_sun = dt.solar_position(loc);
    let now_y = alt_to_y(now_sun.altitude_deg());
    let marker_color = if now_sun.is_daytime() { "#f0883e" } else { "#58a6ff" };

    // Hour ticks at 0/6/12/18/24. Axis lines/text use `currentColor` (inherited
    // from the container's theme colour) so the chart is legible in light AND
    // dark mode; only the semantic colours (amber daytime, gold arc) are fixed.
    let ticks: String = [0.0, 6.0, 12.0, 18.0, 24.0]
        .iter()
        .map(|&hh| {
            let x = hour_to_x(hh);
            format!(
                r##"<line x1="{x:.1}" y1="{pad_t:.1}" x2="{x:.1}" y2="{by:.1}" stroke="currentColor" stroke-width="1" opacity="0.18"/>
                   <text x="{x:.1}" y="{ty:.1}" fill="currentColor" opacity="0.6" font-size="9" text-anchor="middle">{hh:.0}h</text>"##,
                by = pad_t + plot_h,
                ty = h - 8.0,
            )
        })
        .collect();

    format!(
        r##"<svg viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto;display:block;color:var(--text-muted,#8b949e);">
  <rect x="{pad_l}" y="{pad_t}" width="{plot_w:.1}" height="{daylight_h:.1}" fill="#f0883e" opacity="0.10"/>
  {ticks}
  <line x1="{pad_l}" y1="{horizon_y:.1}" x2="{rx:.1}" y2="{horizon_y:.1}" stroke="currentColor" stroke-width="1" stroke-dasharray="3,3" opacity="0.5"/>
  <text x="4" y="{horizon_y:.1}" fill="currentColor" opacity="0.6" font-size="9">0°</text>
  <text x="4" y="{topy:.1}" fill="currentColor" opacity="0.6" font-size="9">90°</text>
  <path d="{path}" fill="none" stroke="#e0a92e" stroke-width="2"/>
  <line x1="{now_x:.1}" y1="{pad_t}" x2="{now_x:.1}" y2="{by:.1}" stroke="{marker_color}" stroke-width="1" opacity="0.6"/>
  <circle cx="{now_x:.1}" cy="{now_y:.1}" r="4.5" fill="{marker_color}" stroke="var(--surface-elevated,#0d1117)" stroke-width="1.5"/>
</svg>"##,
        rx = pad_l + plot_w,
        by = pad_t + plot_h,
        topy = alt_to_y(90.0) + 3.0,
        daylight_h = horizon_y - pad_t,
    )
}

// ── 3. 24-hour day→night timeline strip ──────────────────────────────────

/// A horizontal strip of `n` cells across the chosen day, each coloured by the
/// real sky colour + brightness at that hour, with the current time marked.
/// The "watch the day happen" visual.
pub fn day_timeline_svg(
    dt: &LocalDateTime,
    loc: &NamedLocation,
    turbidity: f64,
    w: f64,
    h: f64,
) -> String {
    let n = 48; // half-hour cells
    let cell_w = w / n as f64;
    let strip_h = h - 16.0;

    let mut cells = String::new();
    for i in 0..n {
        let hh = (i as f64 + 0.5) / n as f64 * 24.0;
        let s = LocalDateTime::new(dt.year, dt.month, dt.day, hh).solar_position(loc);
        let (zenith, horizon) = sky_colours(s.altitude_deg(), turbidity);
        // Use the mid sky colour for the cell.
        let color = blend_hex(&zenith, &horizon, 0.5);
        cells.push_str(&format!(
            r##"<rect x="{:.2}" y="0" width="{:.2}" height="{strip_h:.1}" fill="{color}"/>"##,
            i as f64 * cell_w,
            cell_w + 0.5,
        ));
    }

    let now_x = dt.local_hours / 24.0 * w;
    let labels: String = [(0.0, "00"), (6.0, "06"), (12.0, "12"), (18.0, "18"), (24.0, "24")]
        .iter()
        .map(|(hh, lbl)| {
            let x = hh / 24.0 * w;
            let anchor = if *hh == 0.0 {
                "start"
            } else if *hh == 24.0 {
                "end"
            } else {
                "middle"
            };
            format!(
                r##"<text x="{x:.1}" y="{ly:.1}" fill="currentColor" opacity="0.65" font-size="9" text-anchor="{anchor}">{lbl}h</text>"##,
                ly = h - 3.0,
            )
        })
        .collect();

    format!(
        r##"<svg viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto;display:block;border-radius:6px;overflow:hidden;color:var(--text-muted,#8b949e);">
  {cells}
  <rect x="0" y="0" width="{w}" height="{strip_h:.1}" fill="none" stroke="currentColor" stroke-width="1" opacity="0.25"/>
  <line x1="{now_x:.1}" y1="-1" x2="{now_x:.1}" y2="{marker_bottom:.1}" stroke="#ffffff" stroke-width="2" style="mix-blend-mode:difference"/>
  <polygon points="{now_x:.1},{marker_bottom:.1} {ml:.1},{mtip:.1} {mr:.1},{mtip:.1}" fill="currentColor"/>
  {labels}
</svg>"##,
        marker_bottom = strip_h,
        mtip = strip_h + 5.0,
        ml = now_x - 4.0,
        mr = now_x + 4.0,
    )
}

// ── 4. Sky-lit ground scene (stylised render) ────────────────────────────

/// A stylised SVG render of a road scene lit by the sky at the chosen time: the
/// sky gradient behind, a ground plane whose brightness tracks the horizontal
/// illuminance, a sun/moon in the sky, and cast shading. Not a ray trace — a
/// fast, honest visual of "what does this light look like on the ground".
/// The kind of scene to render under the sky. Each ties to a real use case:
/// **Road** → mesopic road lighting, **Room** → interior daylight factor,
/// **Plaza** → an open exterior space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneKind {
    Road,
    Room,
    Plaza,
}

pub fn scene_render_svg(
    dt: &LocalDateTime,
    loc: &NamedLocation,
    kind: SceneKind,
    turbidity: f64,
    w: f64,
    h: f64,
) -> String {
    let sun = dt.solar_position(loc);
    let alt = sun.altitude_deg();
    let (zenith, horizon) = sky_colours(alt, turbidity);

    // Ground brightness from the actual horizontal illuminance (log-mapped over
    // the ~0.001..100000 lx daylight/night span).
    let avail = if sun.is_daytime() {
        DaylightAvailability::clear_sky(&sun, turbidity).ghi_lux
    } else if alt > -6.0 {
        5.0
    } else {
        0.002
    };
    let lit = ((avail.max(1e-4).log10() + 4.0) / 9.0).clamp(0.03, 1.0); // 1e-4→0, 1e5→1
    let night = alt < 2.0;

    let horizon_y = h * 0.52;
    let sun_x = w * ((sun.azimuth_deg() - 90.0) / 180.0).clamp(0.05, 0.95);
    let sun_y = horizon_y - (alt.clamp(-3.0, 90.0) / 90.0) * (horizon_y - h * 0.08);
    // Sun disc by day, real phase-correct moon by night.
    let disc = if alt > -3.0 {
        let c = if alt > 8.0 { "#fff6d8" } else { "#ffcf6b" };
        format!(
            r##"<circle cx="{sun_x:.1}" cy="{sun_y:.1}" r="30" fill="{c}" opacity="0.25"/><circle cx="{sun_x:.1}" cy="{sun_y:.1}" r="13" fill="{c}"/>"##
        )
    } else {
        let m = dt.moon_position(loc);
        if m.is_up() {
            let mx = w * ((m.azimuth_deg() - 90.0) / 180.0).clamp(0.05, 0.95);
            let my = horizon_y - (m.altitude_deg().clamp(0.0, 90.0) / 90.0) * (horizon_y - h * 0.06);
            moon_disc_svg(mx, my, 11.0, m.illuminated_fraction, m.waxing)
        } else {
            String::new()
        }
    };

    let ground_base = match kind {
        SceneKind::Road => "#3a3f36",   // asphalt
        SceneKind::Room => "#6b5f4e",   // wood/parquet floor
        SceneKind::Plaza => "#5a5348",  // paving
    };
    let ground = darken(ground_base, lit);
    let ground_far = darken(ground_base, (lit * 0.7).max(0.02));

    let foreground = match kind {
        SceneKind::Road => road_foreground(w, h, horizon_y, lit, night),
        SceneKind::Room => room_foreground(w, h, horizon_y, &zenith, &horizon, lit),
        SceneKind::Plaza => plaza_foreground(w, h, horizon_y, lit, night),
    };

    format!(
        r##"<svg viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg" style="width:100%;height:auto;display:block;border-radius:8px;">
  <defs>
    <linearGradient id="scSky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="{zenith}"/><stop offset="100%" stop-color="{horizon}"/>
    </linearGradient>
    <linearGradient id="scGround" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="{ground_far}"/><stop offset="100%" stop-color="{ground}"/>
    </linearGradient>
  </defs>
  <rect x="0" y="0" width="{w}" height="{horizon_y:.1}" fill="url(#scSky)"/>
  {disc}
  <rect x="0" y="{horizon_y:.1}" width="{w}" height="{gh:.1}" fill="url(#scGround)"/>
  {foreground}
</svg>"##,
        gh = h - horizon_y,
    )
}

/// Road scene: perspective lane + centre line + a street lamp that glows at night.
fn road_foreground(w: f64, h: f64, horizon_y: f64, lit: f64, night: bool) -> String {
    format!(
        r##"<polygon points="{cx1:.1},{hy:.1} {cx2:.1},{hy:.1} {rr:.1},{h:.1} {rl:.1},{h:.1}" fill="#2b2f28" opacity="0.7"/>
  <line x1="{mid:.1}" y1="{hy:.1}" x2="{mid:.1}" y2="{h:.1}" stroke="#c9c48a" stroke-width="2" stroke-dasharray="8,10" opacity="{lane:.2}"/>
  <line x1="{px:.1}" y1="{pt:.1}" x2="{px:.1}" y2="{hy:.1}" stroke="#555" stroke-width="3"/>
  <circle cx="{px:.1}" cy="{pt:.1}" r="4" fill="#ffe08a" opacity="{lamp:.2}"/>"##,
        cx1 = w * 0.44, cx2 = w * 0.56, rl = w * 0.2, rr = w * 0.8, mid = w * 0.5,
        hy = horizon_y, lane = lit.max(0.15),
        px = w * 0.82, pt = horizon_y - h * 0.28,
        lamp = if night { 1.0 } else { 0.0 },
    )
}

/// Room scene: an interior with a window showing the sky, a floor and a back
/// wall. The window is bright with daylight (daylight-factor use case).
fn room_foreground(w: f64, h: f64, _horizon_y: f64, zenith: &str, horizon: &str, lit: f64) -> String {
    // Interior overlay: darken everything a little (indoors), then a bright
    // window cut into the back wall showing the sky gradient.
    let wall_op = 0.55;
    format!(
        r##"<rect x="0" y="0" width="{w:.1}" height="{h:.1}" fill="#1a1712" opacity="{wall_op}"/>
  <rect x="{wx:.1}" y="{wy:.1}" width="{ww:.1}" height="{wh:.1}" fill="{zenith}"/>
  <rect x="{wx:.1}" y="{wmid:.1}" width="{ww:.1}" height="{wh2:.1}" fill="{horizon}"/>
  <rect x="{wx:.1}" y="{wy:.1}" width="{ww:.1}" height="{whf:.1}" fill="none" stroke="#3a332a" stroke-width="3"/>
  <line x1="{wcx:.1}" y1="{wy:.1}" x2="{wcx:.1}" y2="{wby:.1}" stroke="#3a332a" stroke-width="2"/>
  <polygon points="0,{h:.1} {fl:.1},{fly:.1} {fr:.1},{fly:.1} {w:.1},{h:.1}" fill="#c9b48a" opacity="{floor:.2}"/>"##,
        wall_op = wall_op,
        wx = w * 0.30, wy = h * 0.16, ww = w * 0.40, wh = h * 0.20,
        wmid = h * 0.36, wh2 = h * 0.16, whf = h * 0.36,
        wcx = w * 0.50, wby = h * 0.52,
        fl = w * 0.30, fr = w * 0.70, fly = h * 0.60,
        floor = (lit * 0.9 + 0.1).min(1.0),
    )
}

/// Plaza scene: an open paved space with a couple of building blocks on the
/// horizon and long shadows — a generic exterior.
fn plaza_foreground(w: f64, h: f64, horizon_y: f64, lit: f64, night: bool) -> String {
    let win = if night { 0.9 } else { 0.0 }; // lit windows at night
    format!(
        r##"<rect x="{b1x:.1}" y="{b1y:.1}" width="{b1w:.1}" height="{b1h:.1}" fill="#2c2f36" opacity="0.85"/>
  <rect x="{b2x:.1}" y="{b2y:.1}" width="{b2w:.1}" height="{b2h:.1}" fill="#23262c" opacity="0.85"/>
  <rect x="{b1wx:.1}" y="{b1wy:.1}" width="6" height="6" fill="#ffe08a" opacity="{win:.2}"/>
  <rect x="{b1wx2:.1}" y="{b1wy:.1}" width="6" height="6" fill="#ffe08a" opacity="{win:.2}"/>
  <line x1="0" y1="{ph:.1}" x2="{w:.1}" y2="{ph:.1}" stroke="#00000022" stroke-width="1"/>
  <line x1="{lx:.1}" y1="{hy:.1}" x2="{lx:.1}" y2="{h:.1}" stroke="#4a4a52" stroke-width="2" opacity="{lit:.2}"/>"##,
        b1x = w * 0.08, b1y = horizon_y - h * 0.20, b1w = w * 0.16, b1h = h * 0.20,
        b2x = w * 0.70, b2y = horizon_y - h * 0.14, b2w = w * 0.20, b2h = h * 0.14,
        b1wx = w * 0.12, b1wx2 = w * 0.18, b1wy = horizon_y - h * 0.15,
        ph = horizon_y + h * 0.02, lx = w * 0.5, hy = horizon_y,
        lit = lit.max(0.15),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(h: f64) -> LocalDateTime {
        LocalDateTime::new(2026, 6, 21, h)
    }
    fn berlin() -> NamedLocation {
        eulumdat_daylight::location::location_by_name("Berlin").unwrap()
    }

    #[test]
    fn kelvin_hex_is_valid_and_ordered() {
        for k in [2000.0, 3000.0, 6500.0, 10000.0] {
            let hex = kelvin_to_hex(k);
            assert_eq!(hex.len(), 7, "hex {hex} malformed");
            assert!(hex.starts_with('#'));
        }
        // Warm has more red-vs-blue than cool.
        let warm = kelvin_to_hex(2500.0);
        let cool = kelvin_to_hex(9000.0);
        let red = |h: &str| u8::from_str_radix(&h[1..3], 16).unwrap();
        let blue = |h: &str| u8::from_str_radix(&h[5..7], 16).unwrap();
        assert!(red(&warm) >= blue(&warm), "warm should be reddish");
        assert!(blue(&cool) >= red(&cool), "cool should be bluish");
    }

    #[test]
    fn all_svgs_are_wellformed_across_the_day() {
        let loc = berlin();
        for h in [0.0, 6.0, 12.0, 18.0, 23.0] {
            let d = dt(h);
            for svg in [
                sky_dome_svg(&d, &loc, 2.5, 400.0, 200.0),
                sun_path_svg(&d, &loc, 400.0, 160.0),
                day_timeline_svg(&d, &loc, 2.5, 600.0, 40.0),
                scene_render_svg(&d, &loc, SceneKind::Road, 2.5, 400.0, 240.0),
                scene_render_svg(&d, &loc, SceneKind::Room, 2.5, 400.0, 240.0),
                scene_render_svg(&d, &loc, SceneKind::Plaza, 2.5, 400.0, 240.0),
            ] {
                assert!(svg.starts_with("<svg"), "svg must start with <svg at h={h}");
                assert!(svg.trim_end().ends_with("</svg>"), "svg must close at h={h}");
                assert!(!svg.contains("NaN"), "svg has NaN at h={h}");
                assert!(!svg.contains("inf"), "svg has inf at h={h}");
            }
        }
    }

    #[test]
    fn night_sky_is_darker_than_noon() {
        // Compare the zenith colour brightness at noon vs midnight.
        let (znoon, _) = sky_colours(50.0, 2.5);
        let (znight, _) = sky_colours(-30.0, 2.5);
        let lum = |h: &str| {
            let r = u8::from_str_radix(&h[1..3], 16).unwrap() as u32;
            let g = u8::from_str_radix(&h[3..5], 16).unwrap() as u32;
            let b = u8::from_str_radix(&h[5..7], 16).unwrap() as u32;
            r + g + b
        };
        assert!(lum(&znoon) > lum(&znight), "noon sky brighter than night");
    }

    #[test]
    fn moon_disc_phase_shapes() {
        // Full moon: a bright disc, no shadow path.
        let full = moon_disc_svg(50.0, 50.0, 12.0, 0.99, true);
        assert!(full.contains("circle"), "full moon has a disc");
        assert!(!full.contains("<path"), "full moon has no terminator shadow");

        // New moon: dark, essentially no lit disc fill.
        let new = moon_disc_svg(50.0, 50.0, 12.0, 0.01, true);
        assert!(new.contains("#20242c") || new.contains("#12151b"), "new moon is dark");

        // Crescent/gibbous: a lit disc plus a shadow terminator path.
        let cres = moon_disc_svg(50.0, 50.0, 12.0, 0.25, true);
        assert!(cres.contains("<path"), "crescent has a terminator shadow");
        assert!(cres.contains("#eef1f6"), "crescent has a lit face");

        // Waxing vs waning crescents differ (lit limb on opposite sides), so
        // their SVG paths must not be identical.
        let waning = moon_disc_svg(50.0, 50.0, 12.0, 0.25, false);
        assert_ne!(cres, waning, "waxing and waning crescents must render differently");

        // All well-formed, no NaN.
        for s in [full, new, cres, waning] {
            assert!(!s.contains("NaN"));
        }
    }
}
