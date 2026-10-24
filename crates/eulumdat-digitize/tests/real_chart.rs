//! Regression test against a real product-page chart of a blue/red LED grow
//! lamp (tinted background, coloured axes, no y tick marks, solid fill),
//! downscaled to half size and colour-snapped to keep the fixture small.
//! Expected values were cross-checked against a full-resolution digitization.

use eulumdat_digitize::{digitize, CurveMode, ImageRef, Options};

#[test]
fn grow_lamp_chart() {
    let bytes = include_bytes!("fixtures/grow_lamp_chart_half.png");
    let img = image::load_from_memory(bytes)
        .expect("decode fixture")
        .to_rgb8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let opts = Options {
        x_range: Some((200.0, 800.0)),
        ..Options::default()
    };
    let d = digitize(ImageRef::new(w, h, 3, img.as_raw()).unwrap(), &opts).unwrap();

    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    assert_eq!(d.calibration.mode, CurveMode::Fill);
    assert!(
        !d.calibration.x_from_labels,
        "chart has tick marks below the axis"
    );
    assert_eq!(
        d.calibration.x_points.len(),
        7,
        "{:?}",
        d.calibration.x_points
    );
    assert!(
        d.calibration.x_residual < 0.5,
        "x residual {}",
        d.calibration.x_residual
    );
    assert_eq!(d.wavelengths.len(), 601);

    assert_eq!(d.peaks.len(), 2, "{:?}", d.peaks);
    let (blue, red) = (d.peaks[0], d.peaks[1]);
    assert!((blue.wavelength_nm - 452.0).abs() <= 1.5, "{blue:?}");
    assert!((blue.relative - 0.688).abs() < 0.03, "{blue:?}");
    assert!((blue.fwhm_nm - 25.0).abs() <= 4.0, "{blue:?}");
    assert!((red.wavelength_nm - 648.0).abs() <= 1.5, "{red:?}");
    assert_eq!(red.relative, 1.0);
    assert!((red.fwhm_nm - 85.0).abs() <= 5.0, "{red:?}");

    // Nothing below 400 nm, a near-empty green gap, a far-red tail at 800 nm.
    let at = |nm: f64| d.values[d.wavelengths.iter().position(|&w| w == nm).unwrap()];
    assert_eq!(at(300.0), 0.0);
    assert!(at(530.0) < 0.02, "v(530) = {}", at(530.0));
    assert!(
        at(795.0) > 0.02 && at(795.0) < 0.06,
        "v(795) = {}",
        at(795.0)
    );

    let spd = d.to_spd("Blue/red grow lamp", "grow_lamp_chart_half.png", None);
    assert_eq!(spd.lines().filter(|l| !l.starts_with('#')).count(), 601);
}
