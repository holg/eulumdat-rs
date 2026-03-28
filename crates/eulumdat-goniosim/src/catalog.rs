//! Material catalog with common materials and their datasheet values.
//!
//! All values based on typical manufacturer datasheets.

use crate::material::MaterialParams;

/// Return all catalog materials.
pub fn material_catalog() -> Vec<MaterialParams> {
    vec![
        clear_pmma_3mm(),
        satin_pmma_3mm(),
        opal_light_pmma_3mm(),
        opal_pmma_3mm(),
        clear_glass_4mm(),
        satin_glass_4mm(),
        clear_polycarbonate_3mm(),
        opal_polycarbonate_3mm(),
        anodized_aluminum(),
        mirror_aluminum(),
        white_paint(),
        matte_black(),
    ]
}

// ---- Transparent materials (PMMA) ----

pub fn clear_pmma_3mm() -> MaterialParams {
    MaterialParams {
        name: "PMMA klar 3mm".into(),
        reflectance_pct: 4.0, // Fresnel at normal incidence for IOR 1.49
        ior: 1.49,
        transmittance_pct: 92.0,
        thickness_mm: 3.0,
        diffusion_pct: 0.0,
    }
}

pub fn satin_pmma_3mm() -> MaterialParams {
    MaterialParams {
        name: "PMMA satin 3mm".into(),
        reflectance_pct: 4.0,
        ior: 1.49,
        transmittance_pct: 85.0,
        thickness_mm: 3.0,
        diffusion_pct: 25.0,
    }
}

pub fn opal_light_pmma_3mm() -> MaterialParams {
    MaterialParams {
        name: "PMMA opal leicht 3mm".into(),
        reflectance_pct: 4.0,
        ior: 1.49,
        transmittance_pct: 75.0,
        thickness_mm: 3.0,
        diffusion_pct: 60.0,
    }
}

pub fn opal_pmma_3mm() -> MaterialParams {
    MaterialParams {
        name: "PMMA opal 3mm".into(),
        reflectance_pct: 4.0,
        ior: 1.49,
        transmittance_pct: 50.0,
        thickness_mm: 3.0,
        diffusion_pct: 95.0,
    }
}

// ---- Transparent materials (Glass) ----

pub fn clear_glass_4mm() -> MaterialParams {
    MaterialParams {
        name: "Glas klar 4mm".into(),
        reflectance_pct: 4.0,
        ior: 1.52,
        transmittance_pct: 90.0,
        thickness_mm: 4.0,
        diffusion_pct: 0.0,
    }
}

pub fn satin_glass_4mm() -> MaterialParams {
    MaterialParams {
        name: "Glas satiniert 4mm".into(),
        reflectance_pct: 4.0,
        ior: 1.52,
        transmittance_pct: 75.0,
        thickness_mm: 4.0,
        diffusion_pct: 30.0,
    }
}

// ---- Transparent materials (Polycarbonate) ----

pub fn clear_polycarbonate_3mm() -> MaterialParams {
    MaterialParams {
        name: "Polycarbonat klar 3mm".into(),
        reflectance_pct: 5.0, // Fresnel for IOR 1.585
        ior: 1.585,
        transmittance_pct: 88.0,
        thickness_mm: 3.0,
        diffusion_pct: 0.0,
    }
}

pub fn opal_polycarbonate_3mm() -> MaterialParams {
    MaterialParams {
        name: "Polycarbonat opal 3mm".into(),
        reflectance_pct: 5.0,
        ior: 1.585,
        transmittance_pct: 55.0,
        thickness_mm: 3.0,
        diffusion_pct: 90.0,
    }
}

// ---- Opaque materials (metals) ----

pub fn anodized_aluminum() -> MaterialParams {
    MaterialParams {
        name: "Aluminium eloxiert".into(),
        reflectance_pct: 80.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 70.0,
    }
}

pub fn mirror_aluminum() -> MaterialParams {
    MaterialParams {
        name: "Aluminium Spiegel".into(),
        reflectance_pct: 95.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 0.0,
    }
}

// ---- Opaque materials (paint) ----

pub fn white_paint() -> MaterialParams {
    MaterialParams {
        name: "Weisslack".into(),
        reflectance_pct: 85.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    }
}

pub fn matte_black() -> MaterialParams {
    MaterialParams {
        name: "Schwarz matt".into(),
        reflectance_pct: 5.0,
        ior: 0.0,
        transmittance_pct: 0.0,
        thickness_mm: 0.0,
        diffusion_pct: 100.0,
    }
}
