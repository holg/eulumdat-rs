use eulumdat::{
    diagram::{PolarDiagram, WatchFaceStyle},
    Eulumdat, Symmetry,
};
use std::fs;

fn main() {
    let ldt = Eulumdat {
        symmetry: Symmetry::BothPlanes,
        c_angles: vec![0.0, 30.0, 60.0, 90.0],
        g_angles: vec![
            0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0, 150.0, 165.0, 180.0,
        ],
        intensities: vec![
            vec![
                280.0, 275.0, 260.0, 230.0, 180.0, 120.0, 60.0, 25.0, 8.0, 2.0, 0.0, 0.0, 0.0,
            ],
            vec![
                270.0, 265.0, 250.0, 220.0, 170.0, 110.0, 55.0, 22.0, 7.0, 2.0, 0.0, 0.0, 0.0,
            ],
            vec![
                250.0, 245.0, 230.0, 200.0, 150.0, 95.0, 45.0, 18.0, 5.0, 1.0, 0.0, 0.0, 0.0,
            ],
            vec![
                220.0, 215.0, 200.0, 170.0, 125.0, 75.0, 35.0, 12.0, 3.0, 0.0, 0.0, 0.0, 0.0,
            ],
        ],
        ..Default::default()
    };

    let polar = PolarDiagram::from_eulumdat(&ldt);

    // Generate different styles
    let dark = polar.to_watch_face_svg(396, 396, &WatchFaceStyle::dark());
    fs::write("/tmp/watchface_dark.svg", &dark).unwrap();

    let california = polar.to_watch_face_svg(396, 396, &WatchFaceStyle::california());
    fs::write("/tmp/watchface_california.svg", &california).unwrap();

    let complication = polar.to_complication_svg(120);
    fs::write("/tmp/watchface_complication.svg", &complication).unwrap();

    println!("Generated watch face SVGs in /tmp/");
    println!("  - /tmp/watchface_dark.svg (396x396)");
    println!("  - /tmp/watchface_california.svg (396x396)");
    println!("  - /tmp/watchface_complication.svg (120x120)");
}
