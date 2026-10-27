//! Channel mixer for the TM-25 viewer: the blue and yellow ray files a
//! vendor ships per white LED (ams OSRAM) combined into one TM-25 set with
//! the flux split of a colour bin, or solved for a target CCT, and opened
//! in the viewer or downloaded. The mixing itself is `tm25ray::mix`; the
//! colour comes from `eulumdat_spectrum::colorimetry`.
//!
//! Each file is read into memory and reduced to a flux-preserving
//! reservoir of up to [`KEEP_PER_CHANNEL`] rays, so 100k to 1M-ray files
//! are the intended inputs.

use std::rc::Rc;

use eulumdat_tm25::{
    mix, mixed_spectrum, mixed_totals, to_bytes, weights_for_flux_fractions, Channel, Header,
    MixMode, Ray, Reservoir, SpectralTable, Tm25Reader,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Rays kept per channel (flux rescaled to the file total).
pub const KEEP_PER_CHANNEL: usize = 300_000;

/// One loaded channel file.
pub struct Loaded {
    pub name: String,
    pub label: String,
    pub header: Header,
    pub rays: Vec<Ray>,
    /// Bin spectrum replacing the file's table, with its file name.
    pub spectrum: Option<(String, SpectralTable)>,
}

impl Loaded {
    fn lumens(&self) -> f64 {
        self.rays
            .iter()
            .map(|r| r.luminous_flux_lm.unwrap_or(0.0) as f64)
            .sum()
    }
    fn watts(&self) -> f64 {
        self.rays
            .iter()
            .map(|r| r.radiant_flux_w.unwrap_or(0.0) as f64)
            .sum()
    }
}

fn colour_label(name: &str) -> String {
    const COLOURS: [&str; 10] = [
        "blue", "yellow", "red", "green", "amber", "white", "cyan", "lime", "phosphor", "uv",
    ];
    name.split(['_', '-', ' ', '.'])
        .find(|t| COLOURS.contains(&t.to_lowercase().as_str()))
        .map(|t| t.to_lowercase())
        .unwrap_or_else(|| name.chars().take(12).collect())
}

async fn file_bytes(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let buf = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

async fn load_channel(file: web_sys::File) -> Result<Loaded, String> {
    let name = file.name();
    let bytes = file_bytes(&file).await?;
    let mut reader = Tm25Reader::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let header = reader.header().clone();
    let mut res = Reservoir::new(KEEP_PER_CHANNEL, 1);
    loop {
        let chunk = reader.read_chunk(65_536).map_err(|e| e.to_string())?;
        if chunk.is_empty() {
            break;
        }
        for r in chunk {
            res.push(r);
        }
    }
    let rays = res.finish();
    if rays.is_empty() {
        return Err(format!("{name}: no rays"));
    }
    Ok(Loaded {
        label: colour_label(&name),
        name,
        header,
        rays,
        spectrum: None,
    })
}

/// `nm value` lines, as the vendors' spectrum .txt files.
fn parse_spectrum_txt(text: &str) -> Option<SpectralTable> {
    let mut t = SpectralTable::default();
    for line in text.lines() {
        let mut it = line.split(['\t', ' ', ';', ',']).filter(|s| !s.is_empty());
        if let (Some(a), Some(b)) = (it.next(), it.next()) {
            if let (Ok(nm), Ok(v)) = (a.parse::<f32>(), b.parse::<f32>()) {
                t.wavelengths_nm.push(nm);
                t.values.push(v);
            }
        }
    }
    (t.len() >= 2).then_some(t)
}

/// Channels with weights from luminous-flux fractions (normalised).
fn with_weights<'a>(loaded: &'a [Rc<Loaded>], fractions: &[f64]) -> Vec<Channel<'a>> {
    let lm: Vec<f64> = loaded.iter().map(|l| l.lumens().max(1e-12)).collect();
    let fr: Vec<f64> = (0..loaded.len())
        .map(|i| fractions.get(i).copied().unwrap_or(0.0).max(0.0))
        .collect();
    let w = weights_for_flux_fractions(&lm, &fr);
    loaded
        .iter()
        .zip(w)
        .map(|(l, weight)| Channel {
            header: &l.header,
            rays: &l.rays,
            weight,
            label: l.label.clone(),
            spectrum: l.spectrum.as_ref().map(|(_, t)| t.clone()),
        })
        .collect()
}

/// CCT and Duv of the mixed spectrum.
fn cct_of(loaded: &[Rc<Loaded>], fractions: &[f64]) -> Option<(f64, f64, f64, f64)> {
    let ch = with_weights(loaded, fractions);
    let t = mixed_spectrum(&ch)?;
    let spd = eulumdat_spectrum::Spd::new(
        &t.wavelengths_nm
            .iter()
            .map(|w| *w as f64)
            .collect::<Vec<_>>(),
        &t.values.iter().map(|v| *v as f64).collect::<Vec<_>>(),
    );
    let c = eulumdat_spectrum::colorimetry::analyze(&spd);
    Some((c.cct_k, c.duv, c.x, c.y))
}

#[component]
pub fn Tm25Mixer(
    /// Receives the mixed file to open in the viewer.
    on_mixed: Callback<web_sys::File>,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let (loaded, set_loaded) = signal_local::<Vec<Rc<Loaded>>>(Vec::new());
    let (fractions, set_fractions) = signal::<Vec<f64>>(Vec::new());
    let (indexed, set_indexed) = signal(false);
    let (target_cct, set_target_cct) = signal(3000.0f64);
    let (status, set_status) = signal(String::new());
    let (busy, set_busy) = signal(false);

    // Fractions default to the files' own lumen shares (weights of 1).
    let reset_fractions = move || {
        let l = loaded.get_untracked();
        let total: f64 = l.iter().map(|c| c.lumens()).sum::<f64>().max(1e-12);
        set_fractions.set(l.iter().map(|c| c.lumens() / total).collect());
    };

    let add_file = move |file: web_sys::File| {
        set_busy.set(true);
        set_status.set(format!("reading {}…", file.name()));
        wasm_bindgen_futures::spawn_local(async move {
            match load_channel(file).await {
                Ok(c) => {
                    set_loaded.update(|v| v.push(Rc::new(c)));
                    reset_fractions();
                    set_status.set(String::new());
                }
                Err(e) => set_status.set(e),
            }
            set_busy.set(false);
        });
    };
    let on_channel_input = move |ev: leptos::ev::Event| {
        let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(files) = input.files() {
            for i in 0..files.length() {
                if let Some(f) = files.get(i) {
                    add_file(f);
                }
            }
        }
        input.set_value("");
    };
    let on_spectrum_input = move |i: usize, ev: leptos::ev::Event| {
        let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
        let Some(file) = input.files().and_then(|f| f.get(0)) else {
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            let text = match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                Ok(t) => t.as_string().unwrap_or_default(),
                Err(_) => String::new(),
            };
            match parse_spectrum_txt(&text) {
                Some(t) => set_loaded.update(|v| {
                    if let Some(c) = v.get_mut(i) {
                        *c = Rc::new(Loaded {
                            name: c.name.clone(),
                            label: c.label.clone(),
                            header: c.header.clone(),
                            rays: c.rays.clone(),
                            spectrum: Some((file.name(), t)),
                        });
                    }
                }),
                None => set_status.set(format!("{}: no 'nm value' pairs found", file.name())),
            }
        });
    };
    let remove = move |i: usize| {
        set_loaded.update(|v| {
            if i < v.len() {
                v.remove(i);
            }
        });
        reset_fractions();
    };

    let preview = Memo::new(move |_| {
        let l = loaded.get();
        let fr = fractions.get();
        if l.len() < 2 {
            return None;
        }
        let ch = with_weights(&l, &fr);
        let (lm, w) = mixed_totals(&ch);
        Some((cct_of(&l, &fr), lm, w))
    });

    // Two channels: bisection on the first channel's fraction, CCT rising with it.
    let solve = move |_| {
        let l = loaded.get_untracked();
        if l.len() != 2 {
            set_status.set("the CCT solver works with exactly two channels".into());
            return;
        }
        let target = target_cct.get_untracked();
        let cct = |f: f64| cct_of(&l, &[f, 1.0 - f]).map(|c| c.0).unwrap_or(0.0);
        let (mut lo, mut hi) = (0.002f64, 0.6f64);
        if !(cct(lo) < target && target < cct(hi)) {
            set_status.set(format!(
                "{target:.0} K is outside what these two channels can mix ({:.0}–{:.0} K)",
                cct(lo),
                cct(hi)
            ));
            return;
        }
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if cct(mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let f = 0.5 * (lo + hi);
        set_fractions.set(vec![f, 1.0 - f]);
        set_status.set(String::new());
    };

    let build = move || -> Result<(String, Vec<u8>), String> {
        let l = loaded.get_untracked();
        let ch = with_weights(&l, &fractions.get_untracked());
        let mode = if indexed.get_untracked() {
            MixMode::Indexed
        } else {
            MixMode::Shared
        };
        let (h, rays) = mix(&ch, mode).map_err(|e| e.to_string())?;
        let bytes = to_bytes(&h, &rays).map_err(|e| e.to_string())?;
        let stem = l[0]
            .name
            .split("_IES_TM25")
            .next()
            .unwrap_or(&l[0].name)
            .trim_end_matches(".TM25RAY")
            .trim_end_matches(".tm25ray")
            .to_string();
        Ok((format!("{stem}_mix.TM25RAY"), bytes))
    };
    let mix_open = move |_| match build() {
        Ok((name, bytes)) => {
            let arr = js_sys::Uint8Array::from(bytes.as_slice());
            let parts = js_sys::Array::of1(&arr.buffer());
            match web_sys::File::new_with_buffer_source_sequence(&parts, &name) {
                Ok(file) => on_mixed.run(file),
                Err(e) => set_status.set(format!("{e:?}")),
            }
        }
        Err(e) => set_status.set(e),
    };
    let mix_download = move |_| match build() {
        Ok((name, bytes)) => {
            super::file_handler::download_bytes(&name, &bytes, "application/octet-stream")
        }
        Err(e) => set_status.set(e),
    };

    let small = "color: #9aa4b2; font-size: 11px;";
    let btn = "padding: 5px 9px; background: #2a2f3a; color: #e8ecf1; border: 0; border-radius: 6px; cursor: pointer; font-size: 12px;";
    view! {
        <hr style="border: 0; border-top: 1px solid #2a2f3a; margin: 10px 0;" />
        <label><input type="checkbox" prop:checked=move || open.get() on:change=move |ev| set_open.set(event_target_checked(&ev)) /> " Mix channels (blue + yellow)"</label>
        {move || open.get().then(|| view! {
            <div style="margin-top: 6px;">
                <div style=small>"Vendors ship one ray file per colour channel of a white LED. Add them, set the flux split of the colour bin, or solve for a CCT, then open the mix."</div>
                <label style="display: block; margin: 8px 0;">
                    <span style="display: inline-block; padding: 5px 9px; background: #1f6feb; border-radius: 6px; cursor: pointer; font-size: 12px;">"Add channel files"</span>
                    <input type="file" accept=".tm25ray,.TM25RAY" multiple style="display: none;" on:change=on_channel_input />
                </label>
                {move || loaded.get().iter().enumerate().map(|(i, c)| {
                    let lm = c.lumens();
                    let w = c.watts();
                    let spec = c.spectrum.as_ref().map(|(n, t)| format!("bin spectrum {n} ({} pts)", t.len())).or_else(|| c.header.shared_spectrum().map(|t| format!("file spectrum {:.0}–{:.0} nm", t.wavelengths_nm[0], t.wavelengths_nm.last().unwrap()))).unwrap_or_else(|| "no spectrum".into());
                    let frac = fractions.get().get(i).copied().unwrap_or(0.0);
                    let label = c.label.clone();
                    view! {
                        <div style="border: 1px solid #2a2f3a; border-radius: 6px; padding: 6px 8px; margin: 6px 0; font-size: 12px;">
                            <div style="display: flex; justify-content: space-between; gap: 6px;">
                                <strong>{label}</strong>
                                <button style=btn on:click=move |_| remove(i)>"×"</button>
                            </div>
                            <div style=small>{format!("{} rays kept · {}", c.rays.len(), if c.header.flags.luminous_flux { format!("{lm:.2} lm") } else { format!("{w:.4} W") })}</div>
                            <div style=small>{spec}</div>
                            <label style="display: flex; justify-content: space-between; gap: 8px; margin-top: 4px; align-items: center;">
                                <span>"Share of luminous flux %"</span>
                                <input type="number" min="0" max="100" step="0.1" style="width: 70px;" prop:value=move || format!("{:.2}", frac * 100.0)
                                    on:change=move |ev| { let v: f64 = event_target_value(&ev).parse().unwrap_or(0.0); set_fractions.update(|f| { if i < f.len() { f[i] = (v / 100.0).max(0.0); } }); } />
                            </label>
                            <label style="display: block; margin-top: 4px;">
                                <span style="display: inline-block; padding: 3px 7px; background: #2a2f3a; border-radius: 5px; cursor: pointer; font-size: 11px;">"Bin spectrum .txt (optional)"</span>
                                <input type="file" accept=".txt,.sre,.csv" style="display: none;" on:change=move |ev| on_spectrum_input(i, ev) />
                            </label>
                        </div>
                    }
                }).collect::<Vec<_>>()}
                {move || (loaded.get().len() >= 2).then(|| view! {
                    <div style="display: flex; gap: 6px; align-items: center; margin: 6px 0; flex-wrap: wrap;">
                        <span>"Target CCT"</span>
                        <input type="number" min="1500" max="20000" step="50" style="width: 80px;" prop:value=move || format!("{:.0}", target_cct.get())
                            on:change=move |ev| set_target_cct.set(event_target_value(&ev).parse().unwrap_or(3000.0)) />
                        <span>"K"</span>
                        <button style=btn on:click=solve>"Solve split"</button>
                        <button style=btn on:click=move |_| reset_fractions()>"Files' own split"</button>
                    </div>
                    {move || preview.get().map(|(c, lm, w)| view! {
                        <div style="font-size: 12px; margin: 4px 0;">
                            {c.map(|(cct, duv, x, y)| { let duv_style = if duv.abs() > 0.006 { "color: #ff6b6b;" } else { "" }; view! {
                                <span>{format!("mix: CCT {cct:.0} K, ")}</span>
                                <span style=duv_style title="ANSI C78.377 white bins lie within about ±0.006 Duv of the Planckian locus; beyond that the ratio alone cannot reach the colour and the bin's own channel spectra are needed">{format!("Duv {duv:+.4}")}</span>
                                <span>{format!(", x {x:.4} y {y:.4} · ")}</span>
                            }.into_any() }).unwrap_or_else(|| view! { <span>"mix: no common spectrum · "</span> }.into_any())}
                            {format!("{lm:.2} lm / {w:.4} W")}
                            {if lm > 0.0 && w > 0.0 { format!(" = {:.1} lm/W", lm / w) } else { String::new() }}
                        </div>
                    })}
                    <label style="display: block; margin: 4px 0;"><input type="checkbox" prop:checked=move || indexed.get() on:change=move |ev| set_indexed.set(event_target_checked(&ev)) /> " Keep the channel spectra as tables (TM-25 identifier 4, radiometric)"</label>
                    <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-top: 6px;">
                        <button style="padding: 6px 10px; background: #1f6feb; color: #fff; border: 0; border-radius: 6px; cursor: pointer; font-size: 12px;" on:click=mix_open>"Mix and open"</button>
                        <button style=btn on:click=mix_download>"Download mixed TM-25"</button>
                    </div>
                })}
                {move || (!status.get().is_empty() || busy.get()).then(|| view! { <div style="color: #ffd166; font-size: 12px; margin-top: 4px;">{move || status.get()}</div> })}
            </div>
        })}
    }
}
