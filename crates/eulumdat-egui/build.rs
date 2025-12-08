//! Build script for eulumdat-egui
//!
//! Copies template LDT files and icon to OUT_DIR for include_str! macros.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_root = Path::new(&manifest_dir);

    // === Templates ===
    let templates_dest = Path::new(&out_dir).join("templates");
    fs::create_dir_all(&templates_dest).expect("Failed to create templates directory");

    // Try local assets first (for crates.io builds), then fall back to workspace location
    let local_templates = crate_root.join("assets/templates");
    let workspace_root = crate_root.parent().unwrap().parent().unwrap();
    let workspace_templates = workspace_root.join("EulumdatApp/EulumdatApp/Resources/Templates");

    let templates_src = if local_templates.exists() {
        local_templates
    } else if workspace_templates.exists() {
        workspace_templates
    } else {
        panic!(
            "Templates directory not found! Checked:\n  - {}\n  - {}",
            local_templates.display(),
            workspace_templates.display()
        );
    };

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
    println!("cargo:rerun-if-changed={}", templates_src.display());

    // === Icon ===
    let local_icon = crate_root.join("assets/icon.svg");
    let workspace_icon = workspace_root.join("assets/icon.svg");

    let icon_src = if local_icon.exists() {
        local_icon
    } else if workspace_icon.exists() {
        workspace_icon
    } else {
        panic!(
            "Icon not found! Checked:\n  - {}\n  - {}",
            local_icon.display(),
            workspace_icon.display()
        );
    };

    let icon_dest = Path::new(&out_dir).join("icon.svg");
    fs::copy(&icon_src, &icon_dest).expect("Failed to copy icon");
    println!("cargo:rerun-if-changed={}", icon_src.display());
}
