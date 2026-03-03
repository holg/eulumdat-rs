//! Usage statistics and analytics tracking.
//!
//! Tracks server-side events like page views, WASM downloads, and client events.
//! Statistics are kept in memory and can be retrieved via the /api/stats endpoint.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Events that can be tracked
#[derive(Debug, Clone)]
pub enum StatsEvent {
    /// Page view
    PageView { path: String },
    /// WASM file download
    WasmDownload { size: u64 },
    /// Client-side event (from JavaScript)
    ClientEvent {
        name: String,
        data: Option<serde_json::Value>,
    },
}

/// Summary of statistics for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    /// Server start time
    pub started_at: DateTime<Utc>,
    /// Total page views
    pub page_views: u64,
    /// Page views by path
    pub page_views_by_path: HashMap<String, u64>,
    /// WASM downloads
    pub wasm_downloads: u64,
    /// Bytes transferred (compressed)
    pub bytes_transferred: u64,
    /// Client events by name
    pub client_events: HashMap<String, u64>,
    /// Feature usage (tracked from client events)
    pub feature_usage: FeatureUsage,
    /// Uptime in seconds
    pub uptime_seconds: i64,
}

/// Feature usage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureUsage {
    /// Number of LDT files opened
    pub ldt_files_opened: u64,
    /// Number of IES files opened
    pub ies_files_opened: u64,
    /// Number of ATLA/XML files opened
    pub atla_files_opened: u64,
    /// Number of PDF exports
    pub pdf_exports: u64,
    /// Number of Typst exports
    pub typst_exports: u64,
    /// Number of LDT exports
    pub ldt_exports: u64,
    /// Number of IES exports
    pub ies_exports: u64,
    /// 3D viewer loads
    pub viewer_3d_loads: u64,
    /// Diagram views by type
    pub diagram_views: HashMap<String, u64>,
}

/// Statistics tracker
pub struct Stats {
    started_at: DateTime<Utc>,
    inner: RwLock<StatsInner>,
}

struct StatsInner {
    page_views: u64,
    page_views_by_path: HashMap<String, u64>,
    wasm_downloads: u64,
    bytes_transferred: u64,
    client_events: HashMap<String, u64>,
    feature_usage: FeatureUsage,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            started_at: Utc::now(),
            inner: RwLock::new(StatsInner {
                page_views: 0,
                page_views_by_path: HashMap::new(),
                wasm_downloads: 0,
                bytes_transferred: 0,
                client_events: HashMap::new(),
                feature_usage: FeatureUsage::default(),
            }),
        }
    }

    /// Record an event
    pub fn record(&self, event: StatsEvent) {
        let mut inner = self.inner.write();

        match event {
            StatsEvent::PageView { path } => {
                inner.page_views += 1;
                *inner.page_views_by_path.entry(path).or_insert(0) += 1;
            }
            StatsEvent::WasmDownload { size } => {
                inner.wasm_downloads += 1;
                inner.bytes_transferred += size;
            }
            StatsEvent::ClientEvent { name, data } => {
                *inner.client_events.entry(name.clone()).or_insert(0) += 1;

                // Track feature usage from known events
                match name.as_str() {
                    "file_open" => {
                        if let Some(data) = data {
                            if let Some(ext) = data.get("extension").and_then(|v| v.as_str()) {
                                match ext.to_lowercase().as_str() {
                                    "ldt" => inner.feature_usage.ldt_files_opened += 1,
                                    "ies" => inner.feature_usage.ies_files_opened += 1,
                                    "xml" => inner.feature_usage.atla_files_opened += 1,
                                    _ => {}
                                }
                            }
                        }
                    }
                    "export_pdf" => inner.feature_usage.pdf_exports += 1,
                    "export_typst" => inner.feature_usage.typst_exports += 1,
                    "export_ldt" => inner.feature_usage.ldt_exports += 1,
                    "export_ies" => inner.feature_usage.ies_exports += 1,
                    "load_3d_viewer" => inner.feature_usage.viewer_3d_loads += 1,
                    "view_diagram" => {
                        if let Some(data) = data {
                            if let Some(diagram_type) = data.get("type").and_then(|v| v.as_str()) {
                                *inner
                                    .feature_usage
                                    .diagram_views
                                    .entry(diagram_type.to_string())
                                    .or_insert(0) += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Get summary of statistics
    pub fn summary(&self) -> StatsSummary {
        let inner = self.inner.read();
        let now = Utc::now();

        StatsSummary {
            started_at: self.started_at,
            page_views: inner.page_views,
            page_views_by_path: inner.page_views_by_path.clone(),
            wasm_downloads: inner.wasm_downloads,
            bytes_transferred: inner.bytes_transferred,
            client_events: inner.client_events.clone(),
            feature_usage: inner.feature_usage.clone(),
            uptime_seconds: (now - self.started_at).num_seconds(),
        }
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}
