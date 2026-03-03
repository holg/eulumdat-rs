//! Accessible form components
//!
//! Provides reusable form components with proper ARIA attributes and label associations
//! for WCAG compliance.
//!
//! Note: These components are designed for future use. The current implementation
//! adds accessibility directly to existing components in tabs.rs, bevy_scene.rs, etc.

use std::sync::atomic::{AtomicU32, Ordering};

/// Global counter for generating unique IDs
static ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generate a unique ID for form elements
pub fn generate_id(prefix: &str) -> String {
    let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", prefix, id)
}

/// Format a number for grid display: show as integer if whole, otherwise with decimals
pub fn format_grid_value(v: f64) -> String {
    if (v - v.round()).abs() < 0.001 {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}
