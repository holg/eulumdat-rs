//! Persistent in-browser library of opened photometric files.
//!
//! Every file that lands in the editor — drag-dropped, picked via the
//! file dialog, opened by a template button, or fetched through the
//! `?url=` query parameter — is appended to this library so the
//! dashboard can show the user's entire collection. Persisted in
//! `localStorage` under [`STORAGE_KEY`] as a JSON array. Capped at
//! [`MAX_ENTRIES`] to avoid blowing past the ~5 MiB localStorage
//! budget; the oldest entry is dropped when the cap is hit.
//!
//! Entries are deduplicated by content hash so reloading the same URL
//! or re-dropping the same file just refreshes the timestamp instead
//! of creating duplicates.

use serde::{Deserialize, Serialize};

/// Where the entry came from. Drives the source icon in the library
/// list and lets us round-trip a `?url=` link by re-fetching.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value")]
pub enum LibrarySource {
    /// Loaded via `?url=` — keep the original URL so the user can
    /// share the link back out or refresh.
    Url(String),
    /// Drag-dropped onto the page or opened with the file picker.
    LocalFile,
    /// Loaded from one of the bundled templates.
    Template,
    /// Fetched from the `eulumdat-wasm-templates` lazy WASM module.
    Bundle,
}

impl LibrarySource {
    /// Single-character icon for compact list rendering.
    pub fn icon(&self) -> &'static str {
        match self {
            LibrarySource::Url(_) => "🔗",
            LibrarySource::LocalFile => "📄",
            LibrarySource::Template => "📦",
            LibrarySource::Bundle => "📚",
        }
    }
}

/// One opened file in the library. `content` holds the raw text so we
/// can re-parse on selection without going back over the network.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LibraryEntry {
    /// Stable identifier — derived from `content` so reopening the
    /// same file replaces the existing entry.
    pub id: String,
    /// Display filename — what the user sees in the list.
    pub name: String,
    /// Raw file body. Routed back through `load_file_content`'s
    /// extension dispatcher on activation.
    pub content: String,
    /// Provenance.
    pub source: LibrarySource,
    /// `Date.now()` (ms) when the entry was added or last refreshed.
    pub added_at: f64,
}

/// localStorage key holding the JSON-serialized [`Library`].
pub const STORAGE_KEY: &str = "eulumdat_library";

/// Hard cap on entries. Browser localStorage is ~5 MiB per origin and
/// LDT files can be up to a couple hundred KiB each, so 50 is a safe
/// ceiling. Older entries are evicted FIFO when the cap is hit.
pub const MAX_ENTRIES: usize = 50;

/// Ordered collection of [`LibraryEntry`] — newest first.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Library {
    pub entries: Vec<LibraryEntry>,
}

impl Library {
    /// Load the persisted library from `localStorage`. Returns an empty
    /// library on any failure (key missing, JSON parse error, browser
    /// without localStorage); never panics.
    pub fn load() -> Self {
        let Some(window) = web_sys::window() else {
            return Self::default();
        };
        let Ok(Some(storage)) = window.local_storage() else {
            return Self::default();
        };
        let Ok(Some(raw)) = storage.get_item(STORAGE_KEY) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    /// Persist to `localStorage`. Errors are logged to the browser
    /// console but otherwise swallowed — losing a write is recoverable
    /// (the in-memory signal keeps the data for this session).
    pub fn save(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(Some(storage)) = window.local_storage() else {
            return;
        };
        match serde_json::to_string(self) {
            Ok(json) => {
                if let Err(e) = storage.set_item(STORAGE_KEY, &json) {
                    web_sys::console::warn_1(
                        &format!("library: localStorage write failed: {:?}", e).into(),
                    );
                }
            }
            Err(e) => web_sys::console::warn_1(&format!("library: serialize failed: {}", e).into()),
        }
    }

    /// Insert or refresh an entry. New entries land at the front so
    /// the list reads newest-first. If an entry with the same `id`
    /// already exists, it's removed and the new one takes its place
    /// (re-fronted with refreshed timestamp). Evicts the oldest entry
    /// once [`MAX_ENTRIES`] is exceeded.
    pub fn push(&mut self, entry: LibraryEntry) {
        self.entries.retain(|e| e.id != entry.id);
        self.entries.insert(0, entry);
        while self.entries.len() > MAX_ENTRIES {
            self.entries.pop();
        }
    }

    /// Remove a single entry by id. No-op if not present.
    pub fn remove(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
    }

    /// Drop everything.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Stable id derived from file content. Same content → same id, so
/// re-opening a file refreshes its entry rather than duplicating it.
/// Uses a simple FNV-1a 64-bit hash to avoid pulling in a real crypto
/// dependency for what's effectively a dedup key.
pub fn content_id(content: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in content.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{:016x}", h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_id_stable_and_unique() {
        let a = content_id("hello world");
        let b = content_id("hello world");
        let c = content_id("HELLO WORLD");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn push_dedups_and_refronts() {
        let mut lib = Library::default();
        lib.push(LibraryEntry {
            id: "x".into(),
            name: "a.ldt".into(),
            content: "first".into(),
            source: LibrarySource::LocalFile,
            added_at: 1.0,
        });
        lib.push(LibraryEntry {
            id: "y".into(),
            name: "b.ldt".into(),
            content: "second".into(),
            source: LibrarySource::LocalFile,
            added_at: 2.0,
        });
        // Re-push x with a fresher timestamp
        lib.push(LibraryEntry {
            id: "x".into(),
            name: "a.ldt".into(),
            content: "first".into(),
            source: LibrarySource::LocalFile,
            added_at: 3.0,
        });
        assert_eq!(lib.len(), 2);
        assert_eq!(lib.entries[0].id, "x");
        assert_eq!(lib.entries[0].added_at, 3.0);
        assert_eq!(lib.entries[1].id, "y");
    }

    #[test]
    fn push_evicts_oldest_at_cap() {
        let mut lib = Library::default();
        for i in 0..(MAX_ENTRIES + 5) {
            lib.push(LibraryEntry {
                id: format!("e{i}"),
                name: format!("e{i}.ldt"),
                content: format!("body {i}"),
                source: LibrarySource::LocalFile,
                added_at: i as f64,
            });
        }
        assert_eq!(lib.len(), MAX_ENTRIES);
        // Newest first
        assert_eq!(lib.entries[0].id, format!("e{}", MAX_ENTRIES + 4));
    }
}
