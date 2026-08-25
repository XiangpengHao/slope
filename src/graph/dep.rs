//! The dependency graph: every crate `cargo metadata` resolved, the edges
//! between them, and what the epoch changed about each — the shape only. How
//! far a crate sits from a chart's center, and on what ring, is that chart's
//! business and is not recorded here.

use serde::{Deserialize, Serialize};

/// Which cargo dependency table an edge comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum DepKind {
    Normal,
    Dev,
    Build,
}

/// A manifest edit to a dependency, detected between the epoch base and the
/// working copy. These are first-class review events: an LLM adding or
/// bumping a dependency must be impossible to miss.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum DepEvent {
    Added,
    Removed,
    /// Version requirement changed: (old, new).
    Bumped(String, String),
}

/// One crate in the resolved graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CrateInfo {
    /// Stable node id: `name@version` (`name` alone for ghosts).
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    /// Member of the workspace (vs an external dependency).
    pub(crate) is_member: bool,
    /// Files under this member's directory changed in the epoch.
    pub(crate) changed: bool,
    pub(crate) changed_files: u32,
    /// This member's Cargo.toml changed in the epoch.
    pub(crate) manifest_changed: bool,
    /// Minimum hops downstream from a changed crate. `Some(0)` = changed
    /// itself; `None` = untouched by the epoch.
    pub(crate) affected_dist: Option<u32>,
    /// How many crates in the resolved graph depend on this one. Drives the
    /// star's magnitude.
    pub(crate) dependents: u32,
    /// Direct dependencies.
    pub(crate) direct_deps: u32,
    /// Direct dependencies that are external (non-member) crates.
    pub(crate) external_deps: u32,
    /// Only present as the target of a removed dependency; no longer in the
    /// resolved graph.
    pub(crate) ghost: bool,
    /// The manifest's own words about itself, when it has any.
    pub(crate) description: Option<String>,
    /// SPDX license expression from the manifest.
    pub(crate) license: Option<String>,
    /// Declared source repository, the link a reviewer wants most.
    pub(crate) repository: Option<String>,
    pub(crate) homepage: Option<String>,
    /// Declared docs URL; externals from crates.io get a docs.rs link even
    /// without one.
    pub(crate) documentation: Option<String>,
    /// Resolved from crates.io, so the registry and docs.rs both have a page
    /// for this exact version.
    pub(crate) crates_io: bool,
    /// A member's directory, relative to the workspace root — where to look
    /// for the change on disk. `None` for externals.
    pub(crate) rel_path: Option<String>,
}

/// A dependency edge: `from` depends on `to`. Both reference [`CrateInfo::id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DepLink {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: DepKind,
    pub(crate) event: Option<DepEvent>,
}

/// The diff window the chart reads against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Epoch {
    /// Human-readable base, e.g. `main @ 1a2b3c4`.
    pub(crate) base: String,
    /// Human-readable target, normally `working copy`.
    pub(crate) target: String,
    /// Why change tracking is off or degraded, in plain words.
    pub(crate) note: Option<String>,
}

/// The full analysis result: every resolved crate and edge, plus the diff.
/// The client decides what to draw; it never receives less than the truth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DepGraph {
    /// Workspace display name (root directory name).
    pub(crate) name: String,
    /// Absolute workspace root path.
    pub(crate) root: String,
    /// Node id of the workspace's root package, when it has one. A virtual
    /// workspace (members only, no root package) has none.
    pub(crate) root_crate: Option<String>,
    pub(crate) epoch: Epoch,
    pub(crate) crates: Vec<CrateInfo>,
    pub(crate) links: Vec<DepLink>,
}
