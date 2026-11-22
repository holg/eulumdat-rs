//! Writes `tests/fixtures/interior_sample.ifc`: the Interior designer's
//! sample room (the landing page's L-shaped open office with the demo LED
//! and its two exits) exported to IFC4.
//! `cargo run -p eulumdat-ifc --features export --example make_sample_ifc`

#[path = "../tests/common/mod.rs"]
mod common;

use eulumdat_ifc::export::{room_to_ifc, ExportOptions};

fn main() {
    let (plan, photometries) = common::interior_sample();
    let ifc = room_to_ifc(
        &plan,
        &photometries,
        &ExportOptions {
            project: "Candelabra Interior sample".into(),
            ..ExportOptions::default()
        },
    );
    std::fs::write(common::SAMPLE, &ifc).unwrap();
    println!(
        "{}: {} luminaires, {} exits, {} bytes",
        common::SAMPLE,
        plan.luminaires.len(),
        plan.doors.len(),
        ifc.len()
    );
}
