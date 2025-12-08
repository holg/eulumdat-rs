//! Build script for eulumdat-egui
//!
//! Copies template LDT files and icon from shared directories.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();

    // === Templates ===
    let templates_dest = Path::new(&out_dir).join("templates");
    fs::create_dir_all(&templates_dest).expect("Failed to create templates directory");

    let templates_src = workspace_root.join("EulumdatApp/EulumdatApp/Resources/Templates");

    if templates_src.exists() {
        for entry in fs::read_dir(&templates_src).expect("Failed to read templates directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "ldt") {
                let file_name = path.file_name().unwrap();
                let dest_path = templates_dest.join(file_name);
                fs::copy(&path, &dest_path).expect("Failed to copy template file");
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    } else {
        println!(
            "cargo:warning=Templates directory not found: {}",
            templates_src.display()
        );
    }
    println!("cargo:rerun-if-changed={}", templates_src.display());

    // === Icon ===
    let icon_src = workspace_root.join("assets/icon.svg");
    let icon_dest = Path::new(&out_dir).join("icon.svg");

    if icon_src.exists() {
        fs::copy(&icon_src, &icon_dest).expect("Failed to copy icon");
        println!("cargo:rerun-if-changed={}", icon_src.display());
    } else {
        println!("cargo:warning=Icon not found: {}", icon_src.display());
    }
}
