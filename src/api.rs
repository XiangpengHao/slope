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
    /// The manifest's own words about itself, when it has any.
    pub description: Option<String>,
    /// SPDX license expression from the manifest.
    pub license: Option<String>,
    /// Declared source repository, the link a reviewer wants most.
    pub repository: Option<String>,
    pub homepage: Option<String>,
    /// Declared docs URL; externals from crates.io get a docs.rs link even
    /// without one.
    pub documentation: Option<String>,
    /// Resolved from crates.io, so the registry and docs.rs both have a page
    /// for this exact version.
    pub crates_io: bool,
    /// A member's directory, relative to the workspace root — where to look
    /// for the change on disk. `None` for externals.
    pub rel_path: Option<String>,
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

// ---------------------------------------------------------------------------
// The code altitude: files, items, and semantically resolved references.
// ---------------------------------------------------------------------------

/// What kind of thing one item in a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Union,
    Trait,
    TypeAlias,
    Const,
    Static,
    Macro,
    /// A module: inline (`mod x { .. }`) or an out-of-line declaration.
    Mod,
    /// An impl block. Its associated functions are their own items, carrying
    /// the impl's name as their `section`.
    Impl,
}

/// One source file in the workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    /// Stable within one analysis: index into [`CodeGraph::files`].
    pub id: u32,
    /// Path relative to the workspace root, e.g. `src/views/atlas.rs`.
    pub path: String,
    /// Name of the crate this file belongs to.
    pub krate: String,
    pub lines: u32,
    /// How many items the file defines (functions, types, traits, …).
    pub items: u32,
    pub fns: u32,
    pub types: u32,
    pub traits: u32,
    /// How many other files reference this one. Drives the mark's magnitude:
    /// the more of the workspace leans on a file, the bigger its star.
    pub refs_in_files: u32,
    /// How many other files this one references.
    pub refs_out_files: u32,
}

/// A file-level reference edge: `from` uses something defined in `to`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRef {
    pub from: u32,
    pub to: u32,
    /// Resolved references aggregated over the whole file pair.
    pub count: u32,
}

/// The code-structure survey: every workspace source file and every resolved
/// file-to-file reference. Item detail ships separately, per file, on unfold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeGraph {
    pub files: Vec<FileInfo>,
    pub refs: Vec<FileRef>,
    /// Names the survey could not resolve (type-inference limits). They are
    /// not on the chart; the words on the plate must say so.
    pub unresolved: u32,
    /// Fidelity notes, in plain words, for the legend.
    pub notes: Vec<String>,
}

/// One item inside a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemInfo {
    /// Index into [`FileDetail::items`].
    pub id: u32,
    /// Display name; inline-module items carry their path (`tests::sample`).
    pub name: String,
    /// The impl or trait header this item sits under, e.g. `impl Trail`;
    /// empty for top-level items.
    pub section: String,
    pub kind: ItemKind,
    /// 1-based lines in the source file.
    pub line: u32,
    pub end_line: u32,
    /// Declared with some form of `pub`.
    pub public: bool,
}

/// A reference between two items of the same file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemRef {
    pub from: u32,
    pub to: u32,
    pub count: u32,
}

/// A reference crossing a file boundary, kept at item precision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemXRef {
    /// Item id in the detail's own file.
    pub item: u32,
    /// The other file.
    pub file: u32,
    /// Name of the item on the other end; empty when the reference lands
    /// between items (e.g. a `use` of the whole file's module).
    pub other: String,
    pub count: u32,
}

/// Everything the cutaway needs for one file: its items in source order and
/// its references at item precision, both directions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileDetail {
    pub file: u32,
    pub items: Vec<ItemInfo>,
    /// References between this file's own items.
    pub item_refs: Vec<ItemRef>,
    /// From this file's items out to other files.
    pub refs_out: Vec<ItemXRef>,
    /// From other files into this file's items.
    pub refs_in: Vec<ItemXRef>,
}

/// Survey the workspace's code structure with rust-analyzer: every workspace
/// source file, its items, and semantically resolved references. The first
/// call runs the survey (tens of seconds on a large workspace); later calls
/// answer from the cache.
#[server]
pub async fn code_graph() -> Result<CodeGraph, ServerFnError> {
    crate::analyze::code::index()
        .await
        .map(|idx| idx.graph.clone())
        .map_err(ServerFnError::new)
}

/// One file's cutaway: items and item-level references. `file` is the id the
/// last [`code_graph`] call handed out.
#[server]
pub async fn file_detail(file: u32) -> Result<FileDetail, ServerFnError> {
    let idx = crate::analyze::code::index()
        .await
        .map_err(ServerFnError::new)?;
    idx.details
        .get(file as usize)
        .cloned()
        .ok_or_else(|| ServerFnError::new(format!("no file with id {file} in this survey")))
}
