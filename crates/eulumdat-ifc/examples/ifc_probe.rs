//! Prints what bimifc's lighting extraction finds per fixture:
//! `cargo run -p eulumdat-ifc --example ifc_probe -- model.ifc`.
fn main() {
    let path = std::env::args().nth(1).expect("model.ifc");
    let text = std::fs::read_to_string(&path).expect("read");
    let model = bimifc_parser::parse_auto(&text).expect("parse");
    let l = bimifc_parser::extract_lighting_data(model.resolver());
    for f in &l.light_fixtures {
        println!(
            "fixture #{} {:?}: {} source(s)",
            f.id,
            f.name,
            f.light_sources.len()
        );
        for s in &f.light_sources {
            let d = s.distribution.as_ref();
            println!(
                "  source #{} {} flux {:?} planes {:?} ldt {}",
                s.id,
                s.source_type,
                s.luminous_flux,
                d.map(|d| d.planes.len()),
                bimifc_parser::light_source_to_ldt(s)
                    .map(|t| t.lines().count())
                    .unwrap_or(0)
            );
        }
    }
}
