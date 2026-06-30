//! Shared response shapes for Plane `/api/v1`.
//!
//! Loose by design (0.1.0): every field is optional and unknown fields are
//! ignored, so upstream additions or omissions do not break the CLI.
#![allow(dead_code)]

use serde::Deserialize;

/// Identifier returned by the Plane API (UUID rendered as a string in 0.1.0).
pub type Id = String;

/// Cursor-paginated list envelope used by Plane `/api/v1` list endpoints.
#[derive(Debug, Deserialize)]
pub struct Paginated<T> {
    #[serde(default = "Vec::new")]
    pub results: Vec<T>,
    #[serde(default)]
    pub next_cursor: Option<String>,
    #[serde(default)]
    pub prev_cursor: Option<String>,
    #[serde(default)]
    pub next_page_results: bool,
    #[serde(default)]
    pub prev_page_results: bool,
    #[serde(default)]
    pub total_results: Option<u64>,
}
