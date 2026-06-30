#![allow(dead_code)]

use crate::core::model::common::Id;
use serde::Deserialize;

/// A Plane work item (issue). Loose: only the fields the CLI renders, all optional.
#[derive(Debug, Deserialize)]
pub struct WorkItem {
    #[serde(default)]
    pub id: Option<Id>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sequence_id: Option<i64>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub state: Option<Id>,
    #[serde(default)]
    pub project: Option<Id>,
}
