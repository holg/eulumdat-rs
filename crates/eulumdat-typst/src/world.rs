//! Typst World implementation for compiling documents.
//!
//! This module is only available when the `compile` feature is enabled.

#![cfg(feature = "compile")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::Library;

/// A minimal Typst world for compiling documents from memory.
pub struct MemoryWorld {
    /// The main source file.
    main: Source,
    /// Additional source files.
    sources: HashMap<FileId, Source>,
    /// The standard library.
    library: LazyHash<Library>,
    /// Font book.
    book: LazyHash<FontBook>,
    /// Available fonts.
    fonts: Vec<Font>,
}

impl MemoryWorld {
    /// Create a new world with the given main source content.
    pub fn new(content: &str) -> Self {
        let main_id = FileId::new(None, typst::syntax::VirtualPath::new("main.typ"));
        let main = Source::new(main_id, content.to_string());

        // Load system fonts
        let (book, fonts) = load_fonts();

        Self {
            main,
            sources: HashMap::new(),
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
        }
    }
}

impl typst::World for MemoryWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else if let Some(source) = self.sources.get(&id) {
            Ok(source.clone())
        } else {
            Err(FileError::NotFound(PathBuf::new()))
        }
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        // For SVG images embedded directly in source, we don't need file access
        Err(FileError::NotFound(PathBuf::new()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        let now = chrono::Local::now();
        Datetime::from_ymd(
            now.format("%Y").to_string().parse().ok()?,
            now.format("%m").to_string().parse().ok()?,
            now.format("%d").to_string().parse().ok()?,
        )
    }
}

/// Load fonts from the system.
fn load_fonts() -> (FontBook, Vec<Font>) {
    static FONTS: OnceLock<(FontBook, Vec<Font>)> = OnceLock::new();

    FONTS
        .get_or_init(|| {
            let mut book = FontBook::new();
            let mut fonts = Vec::new();

            // Try common system font paths
            let font_paths = get_system_font_paths();
            for path in font_paths {
                if path.exists() {
                    load_fonts_from_dir(&path, &mut book, &mut fonts);
                }
            }

            (book, fonts)
        })
        .clone()
}

fn get_system_font_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/System/Library/Fonts"));
        paths.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(home).join("Library/Fonts"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/share/fonts"));
        paths.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(home).join(".fonts"));
            paths.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(windir) = std::env::var_os("WINDIR") {
            paths.push(PathBuf::from(windir).join("Fonts"));
        }
    }

    paths
}

fn load_fonts_from_dir(dir: &Path, book: &mut FontBook, fonts: &mut Vec<Font>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if ext == "ttf" || ext == "otf" || ext == "ttc" {
                        if let Ok(data) = std::fs::read(&path) {
                            let buffer = Bytes::from(data);
                            for font in Font::iter(buffer) {
                                book.push(font.info().clone());
                                fonts.push(font);
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                load_fonts_from_dir(&path, book, fonts);
            }
        }
    }
}

/// Compile Typst source to PDF bytes.
pub fn compile_to_pdf(source: &str) -> Result<Vec<u8>, crate::error::ReportError> {
    let world = MemoryWorld::new(source);

    // Compile the document
    let result = typst::compile(&world);

    let document = result
        .output
        .map_err(|errors| crate::error::ReportError::Compile(format!("{:?}", errors)))?;

    // Export to PDF
    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errors| crate::error::ReportError::Compile(format!("{:?}", errors)))?;

    Ok(pdf_bytes)
}
