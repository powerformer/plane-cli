#![allow(dead_code)]

use crate::core::model::common::Id;
use serde::Deserialize;

/// A Plane project. Loose: only the fields the CLI renders, all optional.
#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub id: Option<Id>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}
