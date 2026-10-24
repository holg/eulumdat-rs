//! Digitize a spectrum chart image (as found on datasheets and product pages)
//! into a relative spectral power distribution.
//!
//! Nothing about the chart style is assumed: axis, background and curve
//! colours are detected, the curve may be a solid fill or a plain line, tick
//! marks may be missing (then the label positions are used), and light grid
//! lines are tolerated. The only thing the caller must supply is what the
//! x-axis ticks mean — [`Options::x_range`] (first and last tick in nm) or an
//! explicit [`Options::x_ticks`] list — because reading printed numbers is out
//! of scope.
//!
//! Pipeline
//!
//! 1. **Background** — most common colour along the image border. Every pixel
//!    farther than [`Options::ink_tolerance`] from it is *ink*.
//! 2. **Axes** — the x-axis is the bottom-most band of rows whose longest ink
//!    run is nearly the widest in the image; the y-axis is the left-most such
//!    band of columns. Band thickness is refined by colour so a fill sitting on
//!    the axis is not swallowed.
//! 3. **X calibration** — tick marks are ink runs touching the axis from
//!    below (or above); without any, the label glyph blocks below the axis are
//!    used. The y-axis line itself counts as a tick when a label is centred
//!    under it. Positions are paired left→right with the tick values and fitted
//!    by least squares.
//! 4. **Curve** — the fill colour is the median ink colour inside the plot
//!    area; per-pixel coverage is the projection onto background→fill. In fill
//!    mode each column's coverage is integrated upward from the axis
//!    (sub-pixel accurate thanks to anti-aliasing); in line mode the thickest
//!    ink run's centre is the curve height. The mode is chosen automatically
//!    unless [`Options::mode`] forces it. Height in pixels is the relative
//!    value — the x-axis line is taken as zero.
//! 5. **Resampling** — binned to a regular grid, peak-normalised, floored.
//!    [`Digitized::to_spd`] writes the tab-separated text that
//!    `eulumdat::atla::spd_loader` reads.
//!
//! The crate takes raw RGB / RGBA pixels ([`ImageRef`]) and has no
//! dependencies, so the same code runs in the CLI (decoded with `image`) and in
//! the browser (decoded by `<canvas>`).

use std::fmt;

/// Borrowed 8-bit pixel buffer, row-major, `channels` = 3 (RGB) or 4 (RGBA).
#[derive(Clone, Copy, Debug)]
pub struct ImageRef<'a> {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub data: &'a [u8],
}

impl<'a> ImageRef<'a> {
    pub fn new(
        width: usize,
        height: usize,
        channels: usize,
        data: &'a [u8],
    ) -> Result<Self, Error> {
        if channels != 3 && channels != 4 {
            return Err(Error::BadInput(format!(
                "channels must be 3 or 4, got {channels}"
            )));
        }
        if data.len() < width * height * channels {
            return Err(Error::BadInput(format!(
                "pixel buffer too short: {} < {}×{}×{}",
                data.len(),
                width,
                height,
                channels
            )));
        }
        if width < 32 || height < 32 {
            return Err(Error::BadInput(format!(
                "image too small: {width}×{height}"
            )));
        }
        Ok(Self {
            width,
            height,
            channels,
            data,
        })
    }

    #[inline]
    fn rgb(&self, x: usize, y: usize) -> [f64; 3] {
        let i = (y * self.width + x) * self.channels;
        [
            self.data[i] as f64,
            self.data[i + 1] as f64,
            self.data[i + 2] as f64,
        ]
    }
}

/// How the curve is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CurveMode {
    /// Fill if most ink columns touch the x-axis, otherwise line.
    #[default]
    Auto,
    /// Solid area under the curve.
    Fill,
    /// A stroked line; the thickest ink run per column is the curve.
    Line,
}

/// Everything that steers the digitizer.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Wavelength of the first and last x-axis tick (left → right). Detected
    /// tick positions are spread evenly between them. Ignored when
    /// `x_ticks` or `x_calibration` is set.
    pub x_range: Option<(f64, f64)>,
    /// Explicit wavelength of every x-axis tick, left → right.
    pub x_ticks: Option<Vec<f64>>,
    /// Manual `(pixel column, wavelength)` pairs; skips tick detection.
    pub x_calibration: Option<Vec<(f64, f64)>>,
    /// Manual `(pixel row, value)` pairs for charts whose x-axis line is not
    /// at zero. Default: height above the axis line is the value.
    pub y_calibration: Option<Vec<(f64, f64)>>,
    pub mode: CurveMode,
    /// RGB distance from the background above which a pixel is ink.
    pub ink_tolerance: f64,
    /// Coverage below this fraction is treated as background (kills light
    /// grid lines and JPEG noise).
    pub coverage_floor: f64,
    /// Curve colour; `None` = median ink colour inside the plot area.
    pub fill_color: Option<[u8; 3]>,
    /// Background colour; `None` = most common colour along the image border.
    pub bg_color: Option<[u8; 3]>,
    /// First wavelength written; `None` = first tick.
    pub start_nm: Option<f64>,
    /// Last wavelength written; `None` = last tick.
    pub end_nm: Option<f64>,
    /// Output grid step in nm.
    pub step_nm: f64,
    /// Values below this fraction of the peak are written as 0.
    pub floor: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            x_range: None,
            x_ticks: None,
            x_calibration: None,
            y_calibration: None,
            mode: CurveMode::Auto,
            ink_tolerance: 60.0,
            coverage_floor: 0.2,
            fill_color: None,
            bg_color: None,
            start_nm: None,
            end_nm: None,
            step_nm: 1.0,
            floor: 0.002,
        }
    }
}

impl Options {
    /// Parse a `"#rrggbb"` / `"rrggbb"` colour.
    pub fn parse_hex(s: &str) -> Result<[u8; 3], Error> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            return Err(Error::BadInput(format!(
                "colour must be 6 hex digits: {s:?}"
            )));
        }
        let mut out = [0u8; 3];
        for (i, o) in out.iter_mut().enumerate() {
            *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| Error::BadInput(format!("bad hex colour {s:?}")))?;
        }
        Ok(out)
    }

    /// Parse `"a,b,c"` into tick values.
    pub fn parse_ticks(s: &str) -> Result<Vec<f64>, Error> {
        s.split(',')
            .map(|t| {
                t.trim()
                    .parse::<f64>()
                    .map_err(|_| Error::BadInput(format!("bad tick value {t:?}")))
            })
            .collect()
    }

    /// Parse `"first:last"` (or `"first-last"`, `"first..last"`) into a range.
    pub fn parse_range(s: &str) -> Result<(f64, f64), Error> {
        let (a, b) = s
            .split_once(':')
            .or_else(|| s.split_once(".."))
            .or_else(|| s.trim().split_once('-'))
            .ok_or_else(|| Error::BadInput(format!("range {s:?} needs first:last")))?;
        let f = |t: &str| {
            t.trim()
                .parse::<f64>()
                .map_err(|_| Error::BadInput(format!("bad range value {t:?}")))
        };
        Ok((f(a)?, f(b)?))
    }

    /// Parse `"px:val,px:val"` into calibration pairs.
    pub fn parse_calibration(s: &str) -> Result<Vec<(f64, f64)>, Error> {
        s.split(',')
            .map(|pair| {
                let (a, b) = pair.split_once(':').ok_or_else(|| {
                    Error::BadInput(format!("calibration pair {pair:?} needs px:value"))
                })?;
                Ok((
                    a.trim()
                        .parse()
                        .map_err(|_| Error::BadInput(format!("bad pixel {a:?}")))?,
                    b.trim()
                        .parse()
                        .map_err(|_| Error::BadInput(format!("bad value {b:?}")))?,
                ))
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    BadInput(String),
    /// Neither `x_range`, `x_ticks` nor `x_calibration` given.
    MissingXScale,
    /// No long horizontal / vertical ink line found.
    NoAxes,
    /// Fewer than two calibration points on the x-axis.
    Calibration {
        found: usize,
    },
    /// No curve inside the plot area.
    NoCurve,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadInput(s) => write!(f, "{s}"),
            Error::MissingXScale => write!(
                f,
                "the x-axis scale is needed: first and last tick wavelength (x_range) or the tick list"
            ),
            Error::NoAxes => write!(f, "no axis lines found — is this a chart with a drawn x- and y-axis?"),
            Error::Calibration { found } => write!(
                f,
                "need at least two x-axis ticks or labels to calibrate, found {found} — pass a manual calibration"
            ),
            Error::NoCurve => write!(f, "no curve found inside the plot area"),
        }
    }
}

impl std::error::Error for Error {}

/// A local maximum of the digitized curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peak {
    pub wavelength_nm: f64,
    /// Peak-normalised height (1.0 for the global maximum).
    pub relative: f64,
    /// Full width at half maximum in nm.
    pub fwhm_nm: f64,
}

/// What the detector found — for a debug overlay and sanity checks.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    /// Inclusive row range of the x-axis line.
    pub x_axis_rows: (usize, usize),
    /// Inclusive column range of the y-axis line.
    pub y_axis_cols: (usize, usize),
    /// `(pixel column, wavelength)` pairs used for the fit.
    pub x_points: Vec<(f64, f64)>,
    /// Largest |fit − label| in nm.
    pub x_residual: f64,
    /// True when the x positions came from label glyphs instead of tick marks.
    pub x_from_labels: bool,
    /// Mode actually used.
    pub mode: CurveMode,
    pub background: [u8; 3],
    pub axis: [u8; 3],
    pub fill: [u8; 3],
    /// nm per pixel column.
    pub nm_per_px: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Digitized {
    /// Regular grid, `step_nm` apart.
    pub wavelengths: Vec<f64>,
    /// Peak-normalised relative power, 0..=1.
    pub values: Vec<f64>,
    pub peaks: Vec<Peak>,
    pub calibration: Calibration,
    pub warnings: Vec<String>,
    /// Raw `(pixel column, curve height in px above the axis)` trace, one
    /// entry per column right of the y-axis. Overlay it to eyeball the result.
    pub trace: Vec<(f64, f64)>,
}

impl Digitized {
    /// Luxeon-style `.spd` text: `#` comment header, tab-separated
    /// `wavelength<TAB>relative` rows — what `eulumdat::atla::spd_loader`
    /// and the web drop-zone parse.
    pub fn to_spd(&self, label: &str, image_name: &str, source: Option<&str>) -> String {
        let mut s = String::with_capacity(self.wavelengths.len() * 16 + 256);
        s.push_str(&format!("# {label}\n"));
        s.push_str(&format!(
            "# Digitized from chart image {image_name} with eulumdat-digitize\n"
        ));
        if let Some(src) = source {
            s.push_str(&format!("# Source: {src}\n"));
        }
        s.push_str(
            "# Relative spectral power, peak normalised to 1.0 — no absolute radiometric scale.\n",
        );
        s.push_str("# Wavelength[nm]\tRelative_Power\n");
        for (wl, v) in self.wavelengths.iter().zip(&self.values) {
            s.push_str(&format!("{}\t{:.6}\n", fmt_g(*wl), v));
        }
        s
    }

    /// One-line summary of the detected peaks, e.g. `452 nm (0.69, FWHM 25 nm) · 648 nm (1.00, FWHM 85 nm)`.
    pub fn peak_summary(&self) -> String {
        self.peaks
            .iter()
            .map(|p| {
                format!(
                    "{:.0} nm ({:.2}, FWHM {:.0} nm)",
                    p.wavelength_nm, p.relative, p.fwhm_nm
                )
            })
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// Shortest plain decimal for a wavelength (`650`, `650.5`).
fn fmt_g(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Run the whole pipeline on one image.
pub fn digitize(img: ImageRef<'_>, opts: &Options) -> Result<Digitized, Error> {
    if opts.x_range.is_none() && opts.x_ticks.is_none() && opts.x_calibration.is_none() {
        return Err(Error::MissingXScale);
    }
    let (w, h) = (img.width, img.height);
    let mut warnings = Vec::new();

    // Background and ink ----------------------------------------------------
    let bg = match opts.bg_color {
        Some(c) => [c[0] as f64, c[1] as f64, c[2] as f64],
        None => estimate_background(&img),
    };
    let ink_tol2 = opts.ink_tolerance * opts.ink_tolerance;
    let mut ink = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            ink[y * w + x] = dist2(img.rgb(x, y), bg) > ink_tol2;
        }
    }

    // Axes ------------------------------------------------------------------
    let (x_axis, y_axis, axis_rgb) = find_axes(&img, &ink)?;
    let y_axis_col = (y_axis.0 + y_axis.1) as f64 / 2.0;
    let plot_left = y_axis.1 + 1;
    let plot_bottom = x_axis.0; // exclusive: rows < plot_bottom are inside
                                // The plot area ends where the axis lines end: titles above and legends
                                // to the right are ignored.
    let plot_top = axis_line_start(&ink, w, y_axis, x_axis).saturating_sub(6);
    let plot_right = axis_line_end(&ink, w, x_axis, y_axis);

    // Fill colour and coverage ---------------------------------------------------
    let fill = match opts.fill_color {
        Some(c) => [c[0] as f64, c[1] as f64, c[2] as f64],
        None => estimate_fill(&img, &ink, plot_left, plot_right, plot_top, plot_bottom)
            .unwrap_or(axis_rgb),
    };
    let cov = coverage(
        &img,
        &ink,
        bg,
        fill,
        axis_rgb,
        x_axis,
        y_axis,
        opts.coverage_floor,
    );

    // X calibration ----------------------------------------------------------------
    let mut x_from_labels = false;
    let x_points: Vec<(f64, f64)> = match &opts.x_calibration {
        Some(p) => p.clone(),
        None => {
            let mut px = find_x_ticks(&ink, w, h, x_axis, plot_left);
            if px.len() < 2 {
                x_from_labels = true;
                px = find_x_labels(&ink, w, h, x_axis, plot_left);
            }
            let inserted = repair_spacing(&mut px);
            if inserted > 0 {
                warnings.push(format!(
                    "{inserted} tick position(s) interpolated into gaps of the detected tick pattern"
                ));
            }
            // The y-axis line is a tick when a label sits centred under it
            // (closer to the axis than to any detected tick).
            let spacing = median_spacing(&px).unwrap_or(w as f64 / 20.0);
            if let Some(c) = label_centre_near(&ink, w, h, x_axis, y_axis_col, spacing * 0.15) {
                let nearest_tick = px
                    .iter()
                    .map(|p| (p - c).abs())
                    .fold(f64::INFINITY, f64::min);
                if (c - y_axis_col).abs() < nearest_tick {
                    px.insert(0, y_axis_col);
                }
            }
            px.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let vals: Vec<f64> = match &opts.x_ticks {
                Some(t) => t.clone(),
                None => {
                    let (first, last) = opts.x_range.unwrap();
                    linspace(first, last, px.len())
                }
            };
            if px.len() != vals.len() {
                warnings.push(format!(
                    "found {} x-axis positions at {:?} but {} tick values given — paired {} in order",
                    px.len(),
                    px.iter().map(|p| p.round() as i64).collect::<Vec<_>>(),
                    vals.len(),
                    px.len().min(vals.len())
                ));
            }
            px.into_iter().zip(vals).collect()
        }
    };
    let (ax, bx, x_res) = fit_linear(&x_points)?;

    // Y scale ---------------------------------------------------------------------------
    // value = ay * row + by ; default: height in px above the axis top edge.
    let (ay, by) = match &opts.y_calibration {
        Some(p) if p.len() >= 2 => {
            let (a, b, _) = fit_linear(p)
                .map_err(|_| Error::BadInput("y calibration needs two distinct rows".into()))?;
            (a, b)
        }
        Some(_) => return Err(Error::BadInput("y calibration needs two points".into())),
        None => (-1.0, plot_bottom as f64),
    };

    // Curve extraction ------------------------------------------------------------------
    let (heights, mode) = extract_columns(
        &cov,
        w,
        plot_top,
        plot_bottom,
        plot_left,
        plot_right,
        opts.mode,
    );
    let trace: Vec<(f64, f64)> = heights
        .iter()
        .enumerate()
        .map(|(i, &hpx)| ((plot_left + i) as f64, hpx))
        .collect();
    let wl_cols: Vec<f64> = trace.iter().map(|(c, _)| ax * c + bx).collect();
    // Height hpx above the axis top edge is row (plot_bottom - hpx).
    let val_cols: Vec<f64> = trace
        .iter()
        .map(|(_, hpx)| ay * (plot_bottom as f64 - hpx) + by)
        .collect();

    // Resample -----------------------------------------------------------------------------
    let start = opts
        .start_nm
        .unwrap_or_else(|| x_points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min));
    let end = opts.end_nm.unwrap_or_else(|| {
        x_points
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let step = if opts.step_nm > 0.0 {
        opts.step_nm
    } else {
        1.0
    };
    let n = ((end - start) / step + 0.5).floor() as usize + 1;
    let wavelengths: Vec<f64> = (0..n).map(|i| start + i as f64 * step).collect();
    let mut values = vec![0.0; n];
    for (i, &wl) in wavelengths.iter().enumerate() {
        let (lo, hi) = (wl - step / 2.0, wl + step / 2.0);
        let mut sum = 0.0;
        let mut cnt = 0usize;
        for (j, &c) in wl_cols.iter().enumerate() {
            if c >= lo && c < hi {
                sum += val_cols[j];
                cnt += 1;
            }
        }
        values[i] = if cnt > 0 {
            sum / cnt as f64
        } else {
            interp(wl, &wl_cols, &val_cols)
        };
    }

    let peak = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if peak <= 0.0 || peak.is_nan() {
        return Err(Error::NoCurve);
    }
    for v in values.iter_mut() {
        *v /= peak;
        if *v < opts.floor {
            *v = 0.0;
        }
        *v = v.clamp(0.0, 1.0);
    }

    let peaks = find_peaks(&wavelengths, &values);
    let to_u8 = |c: [f64; 3]| [c[0].round() as u8, c[1].round() as u8, c[2].round() as u8];

    Ok(Digitized {
        wavelengths,
        values,
        peaks,
        calibration: Calibration {
            x_axis_rows: x_axis,
            y_axis_cols: y_axis,
            x_points,
            x_residual: x_res,
            x_from_labels,
            mode,
            background: to_u8(bg),
            axis: to_u8(axis_rgb),
            fill: to_u8(fill),
            nm_per_px: ax,
        },
        warnings,
        trace,
    })
}

#[inline]
fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn linspace(first: f64, last: f64, n: usize) -> Vec<f64> {
    match n {
        0 => vec![],
        1 => vec![first],
        _ => (0..n)
            .map(|i| first + (last - first) * i as f64 / (n - 1) as f64)
            .collect(),
    }
}

fn median_spacing(px: &[f64]) -> Option<f64> {
    if px.len() < 2 {
        return None;
    }
    let mut sorted = px.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut d: Vec<f64> = sorted.windows(2).map(|w| w[1] - w[0]).collect();
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(d[d.len() / 2])
}

/// Most common colour (4-bit quantised) along the image border.
fn estimate_background(img: &ImageRef<'_>) -> [f64; 3] {
    let (w, h) = (img.width, img.height);
    let mut counts = vec![0u32; 4096];
    let mut sums = vec![[0f64; 3]; 4096];
    let mut add = |p: [f64; 3]| {
        let k = ((p[0] as usize) >> 4) * 256 + ((p[1] as usize) >> 4) * 16 + ((p[2] as usize) >> 4);
        counts[k] += 1;
        sums[k][0] += p[0];
        sums[k][1] += p[1];
        sums[k][2] += p[2];
    };
    for x in 0..w {
        add(img.rgb(x, 0));
        add(img.rgb(x, h - 1));
    }
    for y in 0..h {
        add(img.rgb(0, y));
        add(img.rgb(w - 1, y));
    }
    let best = (0..4096).max_by_key(|&k| counts[k]).unwrap();
    let n = counts[best].max(1) as f64;
    [sums[best][0] / n, sums[best][1] / n, sums[best][2] / n]
}

/// Per-channel median of a set of pixels, via 256-bin histograms.
struct MedianColour {
    hist: [[u32; 256]; 3],
    n: u32,
}

impl MedianColour {
    fn new() -> Self {
        Self {
            hist: [[0; 256]; 3],
            n: 0,
        }
    }
    fn push(&mut self, p: [f64; 3]) {
        for (hist, &v) in self.hist.iter_mut().zip(p.iter()) {
            hist[(v.clamp(0.0, 255.0)) as usize] += 1;
        }
        self.n += 1;
    }
    fn median(&self) -> Option<[f64; 3]> {
        if self.n == 0 {
            return None;
        }
        let mut out = [0f64; 3];
        for (o, hist) in out.iter_mut().zip(self.hist.iter()) {
            let mut acc = 0u32;
            for (v, &cnt) in hist.iter().enumerate() {
                acc += cnt;
                if acc * 2 >= self.n {
                    *o = v as f64;
                    break;
                }
            }
        }
        Some(out)
    }
}

/// Median ink colour inside the plot area.
fn estimate_fill(
    img: &ImageRef<'_>,
    ink: &[bool],
    plot_left: usize,
    plot_right: usize,
    plot_top: usize,
    plot_bottom: usize,
) -> Option<[f64; 3]> {
    let w = img.width;
    let mut m = MedianColour::new();
    for y in plot_top..plot_bottom.saturating_sub(1) {
        for x in (plot_left + 1)..plot_right.min(w) {
            if ink[y * w + x] {
                m.push(img.rgb(x, y));
            }
        }
    }
    m.median()
}

/// First row of the y-axis line: the start of the longest ink run in its
/// left-most column, which must reach the x-axis.
fn axis_line_start(
    ink: &[bool],
    w: usize,
    y_axis: (usize, usize),
    x_axis: (usize, usize),
) -> usize {
    let x = y_axis.0;
    let mut y = x_axis.0;
    while y > 0 && ink[(y - 1) * w + x] {
        y -= 1;
    }
    y
}

/// One past the last column of the x-axis line, scanning its bottom row rightward from the y-axis.
fn axis_line_end(ink: &[bool], w: usize, x_axis: (usize, usize), y_axis: (usize, usize)) -> usize {
    let y = x_axis.1;
    let mut x = y_axis.1;
    while x + 1 < w && ink[y * w + x + 1] {
        x += 1;
    }
    x + 1
}

/// Longest contiguous `true` run in a slice.
fn longest_run(bits: impl Iterator<Item = bool>) -> usize {
    let mut best = 0;
    let mut cur = 0;
    for b in bits {
        if b {
            cur += 1;
            best = best.max(cur);
        } else {
            cur = 0;
        }
    }
    best
}

/// `[(start, end_exclusive)]` of `true` runs.
fn runs(bits: &[bool]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &b) in bits.iter().enumerate() {
        match (b, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, bits.len()));
    }
    out
}

/// Locate the x-axis (row band) and y-axis (column band) and the axis colour.
///
/// Candidate lines are rows / columns whose longest ink run is ≥ 80 % of the
/// best in the image, ignoring a 2 px margin at the image edge. The x-axis is
/// the bottom-most candidate group, the y-axis the left-most. A group is then
/// trimmed to pixels matching the colour of its outer row / column so that a
/// curve fill resting on the axis is not counted as axis, capped at 12 px.
/// Inclusive pixel range of an axis line's thickness.
type Band = (usize, usize);

fn find_axes(img: &ImageRef<'_>, ink: &[bool]) -> Result<(Band, Band, [f64; 3]), Error> {
    let (w, h) = (img.width, img.height);
    let margin = 2usize;

    let row_run: Vec<usize> = (0..h)
        .map(|y| longest_run((0..w).map(|x| ink[y * w + x])))
        .collect();
    let col_run: Vec<usize> = (0..w)
        .map(|x| longest_run((0..h).map(|y| ink[y * w + x])))
        .collect();
    let max_row = *row_run[margin..h - margin].iter().max().unwrap_or(&0);
    let max_col = *col_run[margin..w - margin].iter().max().unwrap_or(&0);
    if max_row < w / 4 || max_col < h / 4 {
        return Err(Error::NoAxes);
    }

    let row_cand: Vec<bool> = (0..h)
        .map(|y| y >= margin && y < h - margin && row_run[y] as f64 >= max_row as f64 * 0.8)
        .collect();
    let col_cand: Vec<bool> = (0..w)
        .map(|x| x >= margin && x < w - margin && col_run[x] as f64 >= max_col as f64 * 0.8)
        .collect();
    let x_group = runs(&row_cand).into_iter().last().ok_or(Error::NoAxes)?;
    let y_group = runs(&col_cand).into_iter().next().ok_or(Error::NoAxes)?;

    // Axis colour: median ink colour of the outermost row of the x-axis group.
    let mut m = MedianColour::new();
    let y_outer = x_group.1 - 1;
    for x in 0..w {
        if ink[y_outer * w + x] {
            m.push(img.rgb(x, y_outer));
        }
    }
    let axis_rgb = m.median().ok_or(Error::NoAxes)?;

    // Trim the x-axis group from its bottom row upward while rows match the axis colour.
    let row_matches = |y: usize| {
        let mut mm = MedianColour::new();
        for x in 0..w {
            if ink[y * w + x] {
                mm.push(img.rgb(x, y));
            }
        }
        mm.median()
            .is_some_and(|c| dist2(c, axis_rgb) < 60.0 * 60.0)
    };
    let mut x_top = x_group.1 - 1;
    while x_top > x_group.0 && x_group.1 - x_top < 12 && row_matches(x_top - 1) {
        x_top -= 1;
    }
    let x_axis = (x_top, x_group.1 - 1);

    // Trim the y-axis group from its left column rightward likewise.
    let col_matches = |x: usize| {
        let mut mm = MedianColour::new();
        for y in 0..h {
            if ink[y * w + x] {
                mm.push(img.rgb(x, y));
            }
        }
        mm.median()
            .is_some_and(|c| dist2(c, axis_rgb) < 60.0 * 60.0)
    };
    let mut y_right = y_group.0;
    while y_right + 1 < y_group.1 && y_right - y_group.0 < 12 && col_matches(y_right + 1) {
        y_right += 1;
    }
    let y_axis = (y_group.0, y_right);

    if y_axis.1 + 8 >= w || x_axis.0 < 8 {
        return Err(Error::NoAxes);
    }
    Ok((x_axis, y_axis, axis_rgb))
}

/// Centres of tick marks: ink runs touching the x-axis from below (fallback:
/// above), at most 20 px wide, 3..=40 px long, allowing ≤ 2 px of
/// anti-aliasing gap between axis and tick. A run only counts when no column
/// within 6 px on either side carries touching ink of comparable length —
/// that rejects the foot of a filled curve resting on the axis while still
/// accepting a tick next to a thin stroked line.
fn find_x_ticks(
    ink: &[bool],
    w: usize,
    h: usize,
    x_axis: (usize, usize),
    plot_left: usize,
) -> Vec<f64> {
    let touch_len_below = |x: usize| {
        let mut y = x_axis.1 + 1;
        let mut skipped = 0;
        while y < h && !ink[y * w + x] && skipped < 2 {
            y += 1;
            skipped += 1;
        }
        let mut n = 0;
        while y < h && ink[y * w + x] && n < 40 {
            n += 1;
            y += 1;
        }
        n
    };
    let touch_len_above = |x: usize| {
        let mut y = x_axis.0; // exclusive: rows < y are above the axis
        let mut skipped = 0;
        while y > 0 && !ink[(y - 1) * w + x] && skipped < 2 {
            y -= 1;
            skipped += 1;
        }
        let mut n = 0;
        while y > 0 && ink[(y - 1) * w + x] && n < 40 {
            n += 1;
            y -= 1;
        }
        n
    };
    let collect = |f: &dyn Fn(usize) -> usize| {
        let x0 = plot_left + 2;
        let lens: Vec<usize> = (x0..w).map(f).collect();
        let bits: Vec<bool> = lens.iter().map(|&n| n >= 3).collect();
        runs(&bits)
            .into_iter()
            .filter(|&(s, e)| {
                let len = lens[s..e].iter().copied().max().unwrap_or(0);
                let limit = ((len as f64 * 0.6).round() as usize).max(2);
                let isolated = (s.saturating_sub(6)..s).all(|i| lens[i] < limit)
                    && (e..(e + 6).min(lens.len())).all(|i| lens[i] < limit);
                e - s <= 20 && isolated
            })
            .map(|(s, e)| x0 as f64 + (s + e - 1) as f64 / 2.0)
            .collect::<Vec<f64>>()
    };
    let below = collect(&touch_len_below);
    if below.len() >= 2 {
        return below;
    }
    collect(&touch_len_above)
}

/// Fill gaps in an otherwise evenly spaced tick list: a gap close to `k`
/// times the smallest spacing (k ≥ 2) gets `k − 1` interpolated positions.
/// Returns the number of inserted positions.
fn repair_spacing(px: &mut Vec<f64>) -> usize {
    if px.len() < 3 {
        return 0;
    }
    px.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min_gap = px
        .windows(2)
        .map(|p| p[1] - p[0])
        .fold(f64::INFINITY, f64::min);
    if min_gap <= 0.0 {
        return 0;
    }
    let mut out = Vec::with_capacity(px.len());
    let mut inserted = 0;
    for i in 0..px.len() {
        if i > 0 {
            let gap = px[i] - px[i - 1];
            let k = (gap / min_gap).round() as usize;
            if k >= 2 && ((gap / min_gap) - k as f64).abs() < 0.15 {
                for j in 1..k {
                    out.push(px[i - 1] + gap * j as f64 / k as f64);
                    inserted += 1;
                }
            }
        }
        out.push(px[i]);
    }
    *px = out;
    inserted
}

/// Column-wise ink profile of the label band below the x-axis, merged into
/// glyph blocks; returns each block's centre column.
fn x_label_blocks(ink: &[bool], w: usize, h: usize, x_axis: (usize, usize)) -> Vec<(usize, usize)> {
    let r0 = (x_axis.1 + 3).min(h);
    let r1 = (x_axis.1 + 3 + (h / 8).max(20)).min(h);
    let bits: Vec<bool> = (0..w)
        .map(|x| (r0..r1).filter(|&y| ink[y * w + x]).count() >= 2)
        .collect();
    let gap = (w / 150).max(4);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in runs(&bits) {
        match merged.last_mut() {
            Some(last) if s - last.1 <= gap => last.1 = e,
            _ => merged.push((s, e)),
        }
    }
    merged.into_iter().filter(|(s, e)| e - s >= 4).collect()
}

/// Centres of the label glyph blocks below the x-axis, right of the plot's left edge.
fn find_x_labels(
    ink: &[bool],
    w: usize,
    h: usize,
    x_axis: (usize, usize),
    plot_left: usize,
) -> Vec<f64> {
    x_label_blocks(ink, w, h, x_axis)
        .into_iter()
        .map(|(s, e)| (s + e - 1) as f64 / 2.0)
        .filter(|&c| c > plot_left as f64 + 3.0)
        .collect()
}

/// Centre of the label block (below the x-axis) closest to column `col`, if
/// it lies within `tol` px.
fn label_centre_near(
    ink: &[bool],
    w: usize,
    h: usize,
    x_axis: (usize, usize),
    col: f64,
    tol: f64,
) -> Option<f64> {
    x_label_blocks(ink, w, h, x_axis)
        .into_iter()
        .map(|(s, e)| (s + e - 1) as f64 / 2.0)
        .filter(|c| (c - col).abs() <= tol)
        .min_by(|a, b| (a - col).abs().partial_cmp(&(b - col).abs()).unwrap())
}

/// Least-squares `value = a·px + b`; returns `(a, b, max residual)`.
fn fit_linear(points: &[(f64, f64)]) -> Result<(f64, f64, f64), Error> {
    let n = points.len();
    if n < 2 {
        return Err(Error::Calibration { found: n });
    }
    let nf = n as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), p| (a + p.0, b + p.1));
    let (mx, my) = (sx / nf, sy / nf);
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    if sxx.abs() < 1e-12 {
        return Err(Error::Calibration { found: 1 });
    }
    let a = sxy / sxx;
    let b = my - a * mx;
    let resid = points
        .iter()
        .map(|p| (a * p.0 + b - p.1).abs())
        .fold(0.0, f64::max);
    Ok((a, b, resid))
}

/// Per-pixel coverage by the fill colour in `[0, 1]`: projection onto the
/// bg → fill line (anti-aliased edges land in between), zero below `floor`
/// for non-ink pixels. In the 3 px fringe next to a coloured axis, pixels
/// that look more like the axis than the fill are its anti-aliased edge and
/// count as background.
#[allow(clippy::too_many_arguments)]
fn coverage(
    img: &ImageRef<'_>,
    ink: &[bool],
    bg: [f64; 3],
    fill: [f64; 3],
    axis: [f64; 3],
    x_axis: (usize, usize),
    y_axis: (usize, usize),
    floor: f64,
) -> Vec<f64> {
    let (w, h) = (img.width, img.height);
    let d = [fill[0] - bg[0], fill[1] - bg[1], fill[2] - bg[2]];
    let denom = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let mut cov = vec![0.0; w * h];
    if denom < 1e-9 {
        return cov;
    }
    let axis_differs = dist2(axis, fill) > 60.0 * 60.0;
    for y in 0..h {
        let near_x_axis = y + 3 >= x_axis.0 && y <= x_axis.1 + 3;
        for x in 0..w {
            let i = y * w + x;
            let p = img.rgb(x, y);
            let near_axis = near_x_axis || (x + 3 >= y_axis.0 && x <= y_axis.1 + 3);
            if axis_differs && near_axis && ink[i] && dist2(p, axis) < dist2(p, fill) {
                continue;
            }
            let dot = (p[0] - bg[0]) * d[0] + (p[1] - bg[1]) * d[1] + (p[2] - bg[2]) * d[2];
            let c = (dot / denom).clamp(0.0, 1.0);
            // Keep anti-aliased edge pixels (partial coverage) even if they are
            // within the ink tolerance of the background; drop faint grid lines.
            cov[i] = if c >= floor || ink[i] { c } else { 0.0 };
        }
    }
    cov
}

/// One vertical ink run in a column.
#[derive(Clone, Copy, Debug)]
struct Run {
    start: usize,
    /// Exclusive.
    end: usize,
    /// Integrated coverage (= height in px for a solid fill).
    sum: f64,
    /// Coverage-weighted centre row (sub-pixel), in pixel-edge coordinates.
    centroid: f64,
}

/// Ink runs in one column from the axis upward.
fn column_runs(cov: &[f64], w: usize, x: usize, plot_top: usize, plot_bottom: usize) -> Vec<Run> {
    let mut out = Vec::new();
    let mut y = plot_bottom;
    while y > plot_top {
        y -= 1;
        if cov[y * w + x] <= 0.0 {
            continue;
        }
        // Run downward-to-upward: collect until a gap of ≥ 3 background rows.
        let end = y + 1;
        let mut sum = 0.0;
        let mut moment = 0.0;
        let mut gap = 0;
        let mut start = y;
        let mut yy = y as isize;
        while yy >= plot_top as isize {
            let v = cov[yy as usize * w + x];
            if v <= 0.0 {
                gap += 1;
                if gap >= 3 {
                    break;
                }
            } else {
                gap = 0;
                start = yy as usize;
                sum += v;
                moment += v * (yy as f64 + 0.5);
            }
            yy -= 1;
        }
        let centroid = if sum > 0.0 {
            moment / sum
        } else {
            (start + end) as f64 / 2.0
        };
        out.push(Run {
            start,
            end,
            sum,
            centroid,
        });
        if start <= plot_top {
            break;
        }
        y = start;
    }
    out
}

/// Curve height above the axis top edge for every plot column, plus the mode used.
fn extract_columns(
    cov: &[f64],
    w: usize,
    plot_top: usize,
    plot_bottom: usize,
    plot_left: usize,
    plot_right: usize,
    mode: CurveMode,
) -> (Vec<f64>, CurveMode) {
    let cols: Vec<Vec<Run>> = (plot_left..w)
        .map(|x| {
            if x < plot_right {
                column_runs(cov, w, x, plot_top, plot_bottom)
            } else {
                Vec::new()
            }
        })
        .collect();

    let mode = match mode {
        CurveMode::Auto => {
            let with_ink = cols.iter().filter(|r| !r.is_empty()).count();
            let touching = cols
                .iter()
                .filter(|r| {
                    r.first()
                        .is_some_and(|run| run.end + 2 >= plot_bottom && run.end - run.start >= 3)
                })
                .count();
            if with_ink > 0 && touching as f64 >= with_ink as f64 * 0.3 {
                CurveMode::Fill
            } else {
                CurveMode::Line
            }
        }
        m => m,
    };

    let heights = cols
        .iter()
        .map(|r| match mode {
            CurveMode::Fill | CurveMode::Auto => r
                .first()
                .filter(|run| run.end + 2 >= plot_bottom)
                .map_or(0.0, |run| run.sum),
            // Short runs touching the axis are tick marks drawn inside the
            // plot, not the curve; a curve at zero height is zero anyway.
            CurveMode::Line => r
                .iter()
                .filter(|run| !(run.end + 2 >= plot_bottom && run.end - run.start <= 40))
                .max_by(|a, b| a.sum.partial_cmp(&b.sum).unwrap())
                .map_or(0.0, |run| plot_bottom as f64 - run.centroid),
        })
        .collect();
    (heights, mode)
}

/// Linear interpolation on ascending `xs`.
fn interp(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[xs.len() - 1] {
        return ys[ys.len() - 1];
    }
    let i = xs.partition_point(|&v| v < x);
    let (x0, x1) = (xs[i - 1], xs[i]);
    let t = if x1 > x0 { (x - x0) / (x1 - x0) } else { 0.0 };
    ys[i - 1] + t * (ys[i] - ys[i - 1])
}

/// Local maxima whose prominence (height above the higher of the two
/// neighbouring valleys) exceeds 5 % of the peak, with their FWHM.
fn find_peaks(wl: &[f64], rel: &[f64]) -> Vec<Peak> {
    let mut peaks = Vec::new();
    let n = rel.len();
    for i in 1..n.saturating_sub(1) {
        if !(rel[i] >= rel[i - 1] && rel[i] > rel[i + 1] && rel[i] > 0.05) {
            continue;
        }
        // Valley on each side: the minimum before the curve climbs above rel[i] again.
        let mut left = rel[i];
        let mut j = i;
        while j > 0 {
            j -= 1;
            if rel[j] > rel[i] {
                break;
            }
            left = left.min(rel[j]);
        }
        let mut right = rel[i];
        let mut j = i;
        while j + 1 < n {
            j += 1;
            if rel[j] > rel[i] {
                break;
            }
            right = right.min(rel[j]);
        }
        if rel[i] - left.max(right) <= 0.05 {
            continue;
        }
        let half = rel[i] / 2.0;
        let mut l = i;
        while l > 0 && rel[l] > half {
            l -= 1;
        }
        let mut r = i;
        while r < n - 1 && rel[r] > half {
            r += 1;
        }
        peaks.push(Peak {
            wavelength_nm: wl[i],
            relative: rel[i],
            fwhm_nm: wl[r] - wl[l],
        });
    }
    peaks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic chart styles, all drawing 0.7·G(450, σ 10) + G(650, σ 35).
    struct Style {
        bg: [u8; 3],
        axis: [u8; 3],
        fill: [u8; 3],
        /// Draw tick marks below the x-axis.
        ticks: bool,
        /// Draw label glyph blocks below the x-axis (and one under the y-axis
        /// when `label_under_axis`).
        labels: bool,
        label_under_axis: bool,
        /// Light grid lines through the plot.
        grid: Option<[u8; 3]>,
        /// Stroke the curve instead of filling it.
        line: bool,
        /// First/last tick nm; the y-axis sits at `axis_nm`.
        first_nm: f64,
        last_nm: f64,
        axis_nm: f64,
    }

    const W: usize = 900;
    const H: usize = 500;
    const PX_PER_NM: f64 = 1.2;

    fn curve(nm: f64) -> f64 {
        0.7 * (-((nm - 450.0) / 10.0).powi(2) / 2.0).exp()
            + (-((nm - 650.0) / 35.0).powi(2) / 2.0).exp()
    }

    fn synth(st: &Style) -> Vec<u8> {
        let mut px = vec![0u8; W * H * 3];
        let set = |px: &mut Vec<u8>, x: usize, y: usize, c: [u8; 3]| {
            let i = (y * W + x) * 3;
            px[i..i + 3].copy_from_slice(&c);
        };
        for y in 0..H {
            for x in 0..W {
                set(&mut px, x, y, st.bg);
            }
        }
        let col_of = |nm: f64| 102.0 + (nm - st.axis_nm) * PX_PER_NM;
        let top_of = |v: f64| 450.0 - v * 400.0;
        // Grid first so the curve covers it.
        if let Some(g) = st.grid {
            for k in 1..5 {
                let y = 450 - k * 80;
                for x in 105..=850 {
                    set(&mut px, x, y, g);
                }
            }
            let mut nm = st.first_nm;
            while nm <= st.last_nm + 1e-9 {
                let x = col_of(nm).round() as usize;
                for y in 50..450 {
                    set(&mut px, x, y, g);
                }
                nm += 100.0;
            }
        }
        let mix = |a: u8, b: u8, t: f64| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        for x in 105..=850usize {
            let nm = (x as f64 - 102.0) / PX_PER_NM + st.axis_nm;
            let top = top_of(curve(nm));
            for y in 0..450usize {
                let cov = if st.line {
                    // 3 px stroke centred on `top`.
                    let d = (y as f64 + 0.5 - top).abs();
                    (2.0 - d).clamp(0.0, 1.0)
                } else {
                    ((y as f64 + 1.0) - top).clamp(0.0, 1.0)
                };
                if cov > 0.0 {
                    set(
                        &mut px,
                        x,
                        y,
                        [
                            mix(st.bg[0], st.fill[0], cov),
                            mix(st.bg[1], st.fill[1], cov),
                            mix(st.bg[2], st.fill[2], cov),
                        ],
                    );
                }
            }
        }
        for x in 100..=850usize {
            for y in 450..455usize {
                set(&mut px, x, y, st.axis);
            }
        }
        for y in 50..455usize {
            for x in 100..105usize {
                set(&mut px, x, y, st.axis);
            }
        }
        let mut nm = st.first_nm;
        while nm <= st.last_nm + 1e-9 {
            let c = col_of(nm).round() as usize;
            let is_axis = (nm - st.axis_nm).abs() < 1e-9;
            if st.ticks && !is_axis {
                for x in c - 2..=c + 2 {
                    for y in 457..471usize {
                        set(&mut px, x, y, st.axis);
                    }
                }
            }
            if st.labels && (!is_axis || st.label_under_axis) {
                // Three "digits" 8 px wide with 3 px gaps, 14 px tall.
                for d in 0..3usize {
                    let x0 = c - 15 + d * 11;
                    for x in x0..x0 + 8 {
                        for y in 476..490usize {
                            set(&mut px, x, y, st.fill);
                        }
                    }
                }
            }
            nm += 100.0;
        }
        // A y-axis label block left of the axis and a title above the plot,
        // neither of which must disturb detection.
        for x in 40..70usize {
            for y in 45..59usize {
                set(&mut px, x, y, st.fill);
            }
        }
        for x in 300..600usize {
            for y in 10..24usize {
                set(&mut px, x, y, st.fill);
            }
        }
        px
    }

    fn tinted() -> Style {
        Style {
            bg: [200, 216, 214],
            axis: [255, 82, 39],
            fill: [77, 95, 106],
            ticks: true,
            labels: true,
            label_under_axis: true,
            grid: None,
            line: false,
            first_nm: 200.0,
            last_nm: 800.0,
            axis_nm: 200.0,
        }
    }

    fn mono() -> Style {
        Style {
            bg: [255, 255, 255],
            axis: [0, 0, 0],
            fill: [0, 0, 0],
            ticks: false,
            labels: true,
            label_under_axis: true,
            grid: Some([221, 221, 221]),
            line: false,
            first_nm: 200.0,
            last_nm: 800.0,
            axis_nm: 200.0,
        }
    }

    fn run(st: &Style, opts: &Options) -> Digitized {
        let px = synth(st);
        digitize(ImageRef::new(W, H, 3, &px).unwrap(), opts).unwrap()
    }

    fn check_curve(d: &Digitized, tol_nm: f64) {
        assert!(
            d.calibration.x_residual < 0.5,
            "x residual {}",
            d.calibration.x_residual
        );
        assert_eq!(d.calibration.x_axis_rows, (450, 454));
        assert_eq!(d.calibration.y_axis_cols, (100, 104));
        assert_eq!(d.peaks.len(), 2, "peaks: {:?}", d.peaks);
        let (blue, red) = (d.peaks[0], d.peaks[1]);
        assert!((blue.wavelength_nm - 450.0).abs() <= tol_nm, "{blue:?}");
        assert!((red.wavelength_nm - 650.0).abs() <= tol_nm, "{red:?}");
        assert!((blue.relative - 0.7).abs() < 0.03, "{blue:?}");
        assert_eq!(red.relative, 1.0);
        assert!((blue.fwhm_nm - 23.5).abs() < 3.0, "{blue:?}");
        assert!((red.fwhm_nm - 82.4).abs() < 4.0, "{red:?}");
    }

    #[test]
    fn tinted_fill_chart_with_ticks() {
        let d = run(
            &tinted(),
            &Options {
                x_range: Some((200.0, 800.0)),
                ..Options::default()
            },
        );
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.calibration.mode, CurveMode::Fill);
        assert!(!d.calibration.x_from_labels);
        assert_eq!(
            d.calibration.x_points.len(),
            7,
            "{:?}",
            d.calibration.x_points
        );
        assert_eq!(d.calibration.axis, [255, 82, 39]);
        assert_eq!(d.calibration.fill, [77, 95, 106]);
        assert_eq!(d.wavelengths.len(), 601);
        check_curve(&d, 1.0);
        let i550 = d.wavelengths.iter().position(|&w| w == 550.0).unwrap();
        assert!(
            (d.values[i550] - 0.017).abs() < 0.01,
            "v(550) = {}",
            d.values[i550]
        );
        assert_eq!(d.values[0], 0.0);
    }

    #[test]
    fn black_on_white_labels_only_with_grid() {
        let d = run(
            &mono(),
            &Options {
                x_range: Some((200.0, 800.0)),
                ..Options::default()
            },
        );
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.calibration.mode, CurveMode::Fill);
        assert!(d.calibration.x_from_labels);
        assert_eq!(
            d.calibration.x_points.len(),
            7,
            "{:?}",
            d.calibration.x_points
        );
        check_curve(&d, 1.5);
        // Grid columns must not leak into the curve.
        let i300 = d.wavelengths.iter().position(|&w| w == 300.0).unwrap();
        assert_eq!(d.values[i300], 0.0);
    }

    #[test]
    fn line_chart_axis_not_a_tick() {
        let st = Style {
            line: true,
            ticks: true,
            labels: true,
            label_under_axis: false,
            first_nm: 400.0,
            last_nm: 700.0,
            axis_nm: 380.0,
            grid: Some([230, 230, 230]),
            ..mono()
        };
        let d = run(
            &st,
            &Options {
                x_range: Some((400.0, 700.0)),
                ..Options::default()
            },
        );
        assert!(d.warnings.is_empty(), "{:?}", d.warnings);
        assert_eq!(d.calibration.mode, CurveMode::Line);
        assert_eq!(
            d.calibration.x_points.len(),
            4,
            "{:?}",
            d.calibration.x_points
        );
        assert!(d.calibration.x_residual < 0.5);
        // Output spans the ticks, 400–700.
        assert_eq!(d.wavelengths.first(), Some(&400.0));
        assert_eq!(d.wavelengths.last(), Some(&700.0));
        // A 3 px stroke centre is pixel-quantised, so a flat peak wanders a little.
        assert_eq!(d.peaks.len(), 2, "{:?}", d.peaks);
        assert!(
            (d.peaks[0].wavelength_nm - 450.0).abs() <= 2.5,
            "{:?}",
            d.peaks
        );
        assert!(
            (d.peaks[1].wavelength_nm - 650.0).abs() <= 2.5,
            "{:?}",
            d.peaks
        );
        assert!((d.peaks[0].relative - 0.7).abs() < 0.03, "{:?}", d.peaks);
    }

    #[test]
    fn explicit_ticks_rgba_and_step() {
        let px = synth(&tinted());
        let rgba: Vec<u8> = px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect();
        let opts = Options {
            x_ticks: Some(vec![200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0]),
            step_nm: 5.0,
            ..Options::default()
        };
        let d = digitize(ImageRef::new(W, H, 4, &rgba).unwrap(), &opts).unwrap();
        assert_eq!(d.wavelengths.len(), 121);
        assert_eq!(d.peaks.len(), 2);
        assert!((d.peaks[1].wavelength_nm - 650.0).abs() <= 2.5);
    }

    #[test]
    fn manual_calibration_and_window() {
        let opts = Options {
            x_calibration: Some(vec![(102.0, 200.0), (822.0, 800.0)]),
            y_calibration: Some(vec![(450.0, 0.0), (50.0, 100.0)]),
            start_nm: Some(400.0),
            end_nm: Some(700.0),
            ..Options::default()
        };
        let d = run(&tinted(), &opts);
        assert_eq!(d.wavelengths.len(), 301);
        assert_eq!(d.peaks.len(), 2);
        check_curve(&d, 1.0);
    }

    #[test]
    fn tick_count_mismatch_warns() {
        // Six values for seven detected positions: paired in order, warned.
        let opts = Options {
            x_ticks: Some(vec![200.0, 300.0, 400.0, 500.0, 600.0, 700.0]),
            ..Options::default()
        };
        let d = run(&tinted(), &opts);
        assert_eq!(d.warnings.len(), 1, "{:?}", d.warnings);
        assert_eq!(d.calibration.x_points.len(), 6);
    }

    #[test]
    fn spd_text() {
        let d = run(
            &tinted(),
            &Options {
                x_range: Some((200.0, 800.0)),
                ..Options::default()
            },
        );
        let s = d.to_spd("Test lamp", "chart.png", Some("unit test"));
        assert!(s.starts_with("# Test lamp\n"));
        assert!(s.contains("# Source: unit test\n"));
        assert!(s.contains("# Wavelength[nm]\tRelative_Power\n"));
        let rows: Vec<&str> = s.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(rows.len(), 601);
        assert_eq!(rows[0], "200\t0.000000");
        assert!(rows.iter().any(|r| r.ends_with("\t1.000000")));
        assert!(d.peak_summary().contains("650 nm (1.00"));
    }

    #[test]
    fn errors() {
        let px = synth(&tinted());
        let img = ImageRef::new(W, H, 3, &px).unwrap();
        assert_eq!(
            digitize(img, &Options::default()),
            Err(Error::MissingXScale)
        );
        let blank = vec![255u8; W * H * 3];
        let opts = Options {
            x_range: Some((0.0, 1.0)),
            ..Options::default()
        };
        assert_eq!(
            digitize(ImageRef::new(W, H, 3, &blank).unwrap(), &opts),
            Err(Error::NoAxes)
        );
        assert!(ImageRef::new(10, 10, 3, &blank).is_err());
        assert!(ImageRef::new(W, H, 2, &blank).is_err());
    }

    #[test]
    fn option_parsers() {
        assert_eq!(Options::parse_hex("#E8501e").unwrap(), [0xe8, 0x50, 0x1e]);
        assert!(Options::parse_hex("#12345").is_err());
        assert_eq!(
            Options::parse_ticks("200, 300,400").unwrap(),
            vec![200.0, 300.0, 400.0]
        );
        assert_eq!(Options::parse_range("380:780").unwrap(), (380.0, 780.0));
        assert_eq!(Options::parse_range("380..780").unwrap(), (380.0, 780.0));
        assert_eq!(Options::parse_range("380-780").unwrap(), (380.0, 780.0));
        assert_eq!(
            Options::parse_calibration("10:200,20.5:300").unwrap(),
            vec![(10.0, 200.0), (20.5, 300.0)]
        );
        assert!(Options::parse_calibration("10-200").is_err());
    }
}
