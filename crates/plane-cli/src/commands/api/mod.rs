pub mod me;
pub mod project;
pub mod request;
pub mod work_item;

pub use me::ApiMeOptions;

use crate::core::app::AppState;
use serde_json::Value;

/// Resolve the workspace slug (config default or --workspace) for
/// workspace-scoped commands.
pub(crate) fn require_workspace(state: &AppState) -> Result<String, String> {
    state
        .config
        .workspace_slug
        .as_deref()
        .map(str::trim)
        .filter(|slug| !slug.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            "workspace is required; set --workspace, workspace_slug in plane.toml, or PLANE_WORKSPACE_SLUG"
                .to_string()
        })
}

/// Render any JSON value as the pretty-printed `--json` output.
pub(crate) fn render_json(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|json| format!("{json}\n"))
        .map_err(|error| error.to_string())
}
