//! Report generator that produces Typst source from photometric data.

use eulumdat::Eulumdat;

use crate::template::{
    generate_comparison_report, generate_typst_source, generate_typst_with_files,
};

/// Sections that can be included in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportSection {
    /// Executive summary with key metrics
    Summary,
    /// Luminaire identification and metadata
    LuminaireInfo,
    /// Lamp sets data
    LampData,
    /// Physical dimensions
    Dimensions,
    /// Photometric properties (LOR, efficacy, etc.)
    PhotometricData,
    /// Polar diagram (C0-C180 / C90-C270)
    PolarDiagram,
    /// Cartesian diagram (intensity vs gamma)
    CartesianDiagram,
    /// 3D Butterfly diagram
    ButterflyDiagram,
    /// Intensity heatmap
    HeatmapDiagram,
    /// BUG rating analysis
    BugRating,
    /// Full intensity data table
    IntensityTable,
    /// Zonal lumens distribution
    ZonalLumens,
    /// Direct ratios (utilization factors)
    DirectRatios,
    /// Coefficient of Utilization (CU) table
    CuTable,
    /// Unified Glare Rating (UGR) table
    UgrTable,
    /// Full candela tabulation (like Photometric Toolbox)
    CandelaTable,
}

impl ReportSection {
    /// Get all available sections in recommended order.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Summary,
            Self::LuminaireInfo,
            Self::LampData,
            Self::Dimensions,
            Self::PhotometricData,
            Self::PolarDiagram,
            Self::CartesianDiagram,
            Self::ButterflyDiagram,
            Self::HeatmapDiagram,
            Self::BugRating,
            Self::ZonalLumens,
            Self::DirectRatios,
            Self::IntensityTable,
        ]
    }

    /// Get sections for a compact report.
    pub fn compact() -> Vec<Self> {
        vec![
            Self::Summary,
            Self::LuminaireInfo,
            Self::PolarDiagram,
            Self::BugRating,
        ]
    }
}

/// Options for report generation.
#[derive(Debug, Clone)]
pub struct ReportOptions {
    /// Sections to include in the report.
    pub sections: Vec<ReportSection>,
    /// Include dark theme diagrams (in addition to light).
    pub include_dark_theme: bool,
    /// Paper size.
    pub paper_size: PaperSize,
    /// Language for labels (ISO 639-1 code).
    pub language: String,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            sections: ReportSection::all(),
            include_dark_theme: false,
            paper_size: PaperSize::A4,
            language: "en".to_string(),
        }
    }
}

/// Paper sizes for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    A4,
    Letter,
    A3,
}

impl PaperSize {
    /// Get the Typst paper name.
    pub fn typst_name(&self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::Letter => "us-letter",
            Self::A3 => "a3",
        }
    }
}

/// Report generator for photometric data.
pub struct ReportGenerator<'a> {
    ldt: &'a Eulumdat,
}

impl<'a> ReportGenerator<'a> {
    /// Create a new report generator.
    pub fn new(ldt: &'a Eulumdat) -> Self {
        Self { ldt }
    }

    /// Generate the Typst source code for the report.
    ///
    /// The returned string can be written to a `.typ` file and compiled
    /// using `typst compile report.typ report.pdf`.
    pub fn generate_typst(&self, options: &ReportOptions) -> String {
        generate_typst_source(self.ldt, &options.sections, options.include_dark_theme)
    }

    /// Generate the Typst source with default options.
    pub fn generate_typst_default(&self) -> String {
        self.generate_typst(&ReportOptions::default())
    }

    /// Write the Typst source to a file.
    pub fn write_typst(
        &self,
        path: &std::path::Path,
        options: &ReportOptions,
    ) -> std::io::Result<()> {
        let source = self.generate_typst(options);
        std::fs::write(path, source)
    }

    /// Generate PDF bytes from the report using the typst CLI.
    ///
    /// This writes the Typst source (with inline SVGs) to a temp file,
    /// invokes `typst compile`, and returns the PDF bytes.
    /// Requires `typst` to be installed and in PATH.
    pub fn generate_pdf(&self, options: &ReportOptions) -> crate::Result<Vec<u8>> {
        use std::process::Command;

        // Generate Typst source with inline embedded SVGs
        let (source, _) = generate_typst_with_files(self.ldt, &options.sections);

        // Create temp files
        let temp_dir = std::env::temp_dir();
        let typ_path = temp_dir.join("eulumdat_report.typ");
        let pdf_path = temp_dir.join("eulumdat_report.pdf");

        // Write Typst source
        std::fs::write(&typ_path, &source)?;

        // Invoke typst CLI
        let output = Command::new("typst")
            .args([
                "compile",
                typ_path.to_str().unwrap(),
                pdf_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| {
                crate::error::ReportError::Compile(format!(
                    "Failed to run typst CLI. Is typst installed? Error: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Clean up temp files
            let _ = std::fs::remove_file(&typ_path);
            return Err(crate::error::ReportError::Compile(format!(
                "typst compile failed:\n{}",
                stderr
            )));
        }

        // Read PDF
        let pdf_bytes = std::fs::read(&pdf_path)?;

        // Clean up temp files
        let _ = std::fs::remove_file(&typ_path);
        let _ = std::fs::remove_file(&pdf_path);

        Ok(pdf_bytes)
    }

    /// Generate a Typst comparison report for two photometric files.
    /// Returns the Typst source with inline SVGs, ready for PDF compilation.
    pub fn generate_comparison(
        ldt_a: &Eulumdat,
        ldt_b: &Eulumdat,
        label_a: &str,
        label_b: &str,
    ) -> String {
        generate_comparison_report(ldt_a, ldt_b, label_a, label_b)
    }

    /// Generate PDF with default options.
    pub fn generate_pdf_default(&self) -> crate::Result<Vec<u8>> {
        self.generate_pdf(&ReportOptions::default())
    }

    /// Write PDF to a file.
    pub fn write_pdf(&self, path: &std::path::Path, options: &ReportOptions) -> crate::Result<()> {
        let pdf = self.generate_pdf(options)?;
        std::fs::write(path, pdf)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_ldt() -> Eulumdat {
        Eulumdat {
            luminaire_name: "Test Luminaire".to_string(),
            identification: "Test Manufacturer".to_string(),
            symmetry: eulumdat::Symmetry::VerticalAxis,
            c_angles: vec![0.0],
            g_angles: vec![0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0],
            intensities: vec![vec![100.0, 95.0, 80.0, 50.0, 20.0, 5.0, 0.0]],
            lamp_sets: vec![eulumdat::LampSet {
                num_lamps: 1,
                lamp_type: "LED".to_string(),
                total_luminous_flux: 1000.0,
                color_appearance: "3000K".to_string(),
                color_rendering_group: "1B".to_string(),
                wattage_with_ballast: 10.0,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_generate_typst_source() {
        let ldt = create_test_ldt();
        let generator = ReportGenerator::new(&ldt);
        let source = generator.generate_typst(&ReportOptions::default());

        assert!(source.contains("Photometric Report"));
        assert!(source.contains("Test Luminaire"));
        assert!(source.contains("Test Manufacturer"));
    }

    #[test]
    fn test_compact_sections() {
        let sections = ReportSection::compact();
        assert!(sections.contains(&ReportSection::Summary));
        assert!(sections.contains(&ReportSection::PolarDiagram));
        assert!(!sections.contains(&ReportSection::IntensityTable));
    }
}
