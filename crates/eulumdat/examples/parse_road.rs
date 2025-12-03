use eulumdat::{Eulumdat, Symmetry};
use std::fs;

fn main() {
    let content = fs::read_to_string("../eulumdat-wasm/templates/road_luminaire.ldt").unwrap();
    let ldt = Eulumdat::parse(&content).unwrap();

    println!("=== Road Luminaire Parsing Test ===");
    println!("Symmetry: {:?} ({})", ldt.symmetry, ldt.symmetry as i32);
    println!("Nc (num_c_planes): {}", ldt.num_c_planes);
    println!("Ng (num_g_planes): {}", ldt.num_g_planes);
    println!();
    println!("c_angles count: {}", ldt.c_angles.len());
    println!("g_angles count: {}", ldt.g_angles.len());
    println!("intensities count (Mc): {}", ldt.intensities.len());
    println!();

    // For symmetry 3, find the start index at C90
    let c_start = match ldt.symmetry {
        Symmetry::PlaneC90C270 => ldt.c_angles.iter().position(|&c| c >= 90.0).unwrap_or(0),
        _ => 0,
    };

    println!(
        "c_start index for symmetry {}: {}",
        ldt.symmetry as i32, c_start
    );
    println!();

    // Show C-angles that correspond to intensity data
    println!(
        "C-angles for intensity data (starting at index {}):",
        c_start
    );
    for i in 0..ldt.intensities.len().min(10) {
        print!("{:>6.0} ", ldt.c_angles[c_start + i]);
    }
    println!("...");

    println!();
    println!("Intensity data for G=0 (first row):");
    print!("Values:   ");
    for i in 0..ldt.intensities.len().min(10) {
        print!("{:>6.0} ", ldt.intensities[i][0]);
    }
    println!("...");

    println!();
    println!(
        "First 5 G-angles: {:?}",
        &ldt.g_angles[..5.min(ldt.g_angles.len())]
    );
    println!();

    // Show matching your expected data format
    println!("=== Table format (matching C++ app) ===");
    print!("       ");
    for i in 0..ldt.intensities.len().min(10) {
        print!("{:>6.0} ", ldt.c_angles[c_start + i]);
    }
    println!();

    for (g_idx, &g_angle) in ldt.g_angles.iter().take(5).enumerate() {
        print!("{:>5.1}  ", g_angle);
        for i in 0..ldt.intensities.len().min(10) {
            print!("{:>6.0} ", ldt.intensities[i][g_idx]);
        }
        println!();
    }
}
