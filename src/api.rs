//! Shared graph types and the server API that produces them.
//!
//! Everything here crosses the wire between the server (which runs the
//! analysis) and the wasm client (which draws the chart), so it stays free
//! of any server-only dependency.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// Which cargo dependency table an edge comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

/// A manifest edit to a dependency, detected between the epoch base and the
/// working copy. These are first-class review events: an LLM adding or
/// bumping a dependency must be impossible to miss.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DepEvent {
    Added,
    Removed,
    /// Version requirement changed: (old, new).
    Bumped(String, String),
}

/// One crate in the resolved graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrateInfo {
    /// Stable node id: `name@version` (`name` alone for ghosts).
    pub id: String,
    pub name: String,
    pub version: String,
    /// Member of the workspace (vs an external dependency).
    pub is_member: bool,
    /// Files under this member's directory changed in the epoch.
    pub changed: bool,
    pub changed_files: u32,
    /// This member's Cargo.toml changed in the epoch.
    pub manifest_changed: bool,
    /// Minimum hops downstream from a changed crate. `Some(0)` = changed
    /// itself; `None` = untouched by the epoch.
    pub affected_dist: Option<u32>,
    /// How many crates in the resolved graph depend on this one. Drives the
    /// star's magnitude.
    pub dependents: u32,
    /// Direct dependencies.
    pub direct_deps: u32,
    /// Direct dependencies that are external (non-member) crates.
    pub external_deps: u32,
    /// Only present as the target of a removed dependency; no longer in the
    /// resolved graph.
    pub ghost: bool,
}

/// A dependency edge: `from` depends on `to`. Both reference [`CrateInfo::id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepLink {
    pub from: String,
    pub to: String,
    pub kind: DepKind,
    pub event: Option<DepEvent>,
}

/// The diff window the chart reads against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Epoch {
    /// `"jj"` or `"git"`; `None` when no VCS was detected.
    pub vcs: Option<String>,
    /// Human-readable base, e.g. `main @ 1a2b3c4`.
    pub base: String,
    /// Human-readable target, normally `working copy`.
    pub target: String,
    /// No changes between base and target.
    pub clean: bool,
    /// Why change tracking is off or degraded, in plain words.
    pub note: Option<String>,
}

/// The full analysis result: every resolved crate and edge, plus the diff.
/// The client decides what to draw; it never receives less than the truth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceGraph {
    /// Workspace display name (root directory name).
    pub name: String,
    /// Absolute workspace root path.
    pub root: String,
    /// Node id of the workspace's root package, when it has one. A virtual
    /// workspace (members only, no root package) has none.
    pub root_crate: Option<String>,
    pub epoch: Epoch,
    pub crates: Vec<CrateInfo>,
    pub links: Vec<DepLink>,
}

/// Analyze the target workspace: resolved dependency graph via
/// `cargo metadata`, diff via the detected VCS. The target is
/// `SLOPIFY_WORKSPACE` (falling back to the server's working directory);
/// `SLOPIFY_BASE` overrides the diff base revision.
#[server]
pub async fn workspace_graph() -> Result<WorkspaceGraph, ServerFnError> {
    tokio::task::spawn_blocking(crate::analyze::analyze)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map_err(ServerFnError::new)
}
