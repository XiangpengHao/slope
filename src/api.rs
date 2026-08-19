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

/// How widely an item is declared visible. `pub(crate)`, `pub(super)`, and
/// `pub(in path)` are not `pub`: the altitude's interest bar reads them
/// apart, and privacy is a permanent fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Vis {
    /// `pub` — visible outside its own crate.
    Pub,
    /// `pub(crate)`, `pub(super)`, `pub(in path)`.
    Crate,
    /// No `pub` at all. Never a mark on the map; its references lift.
    Private,
}

impl Vis {
    /// Weight the interest bar adds for visibility: the wider the door, the
    /// more the map owes the reader a name.
    pub fn weight(self) -> u32 {
        match self {
            Vis::Pub => 2,
            Vis::Crate => 1,
            Vis::Private => 0,
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Vis::Pub => "pub",
            Vis::Crate => "pub(crate)",
            Vis::Private => "private",
        }
    }
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
    /// Touched between the epoch base and the working copy.
    pub changed: bool,
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

/// One landmark the map may engrave: an item, seated in the containment tree
/// (crate → directory → file → type → method), with the weight that decides
/// whether it clears the altitude's bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemMark {
    /// Index into [`CodeGraph::items`].
    pub id: u32,
    /// The file whose source defines it.
    pub file: u32,
    /// Index into that file's [`FileDetail::items`].
    pub local: u32,
    /// Display name, without any section prefix.
    pub name: String,
    /// The label this item selects by in a URL: `Type::method` inside a
    /// section, the plain name otherwise.
    pub label: String,
    pub kind: ItemKind,
    pub vis: Vis,
    pub line: u32,
    /// Semantic container: the type a method or associated item belongs to,
    /// resolved through the impl's self type even when the impl sits in
    /// another file. `None` for items the file itself contains.
    pub parent: Option<u32>,
    /// Item-level references reaching it from other files. Drives the
    /// engraved weight of its mark.
    pub fan_in: u32,
    /// Traits this type derives or implements, from every impl anywhere in
    /// the workspace. A lens on the type, never nesting.
    pub traits: Vec<String>,
}

/// A reference between two items, aggregated per pair. Endpoints carry their
/// file, so the client can lift an edge to whatever is visible at the current
/// fold state without fetching item detail; a `None` item is a reference to a
/// file as a whole (a `use` of its module).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemEdge {
    pub from_file: u32,
    pub from: Option<u32>,
    pub to_file: u32,
    pub to: Option<u32>,
    pub count: u32,
}

/// The code-structure survey: every workspace source file, every resolved
/// reference at both file and item precision, and every item the map can
/// engrave. Item *bodies* — fields, variants, signatures — ship separately,
/// per file, when a focus asks for them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeGraph {
    pub files: Vec<FileInfo>,
    pub refs: Vec<FileRef>,
    /// Every chartable item, in (file, source) order. Impl blocks are not
    /// here: they are attribution, not geometry.
    pub items: Vec<ItemMark>,
    /// Cross-file references at item precision, aggregated per pair.
    pub item_edges: Vec<ItemEdge>,
    /// Names the survey could not resolve (type-inference limits). They are
    /// not on the chart; the words on the plate must say so.
    pub unresolved: u32,
    /// Fidelity notes, in plain words, for the legend.
    pub notes: Vec<String>,
}

/// One field of a struct, or one variant of an enum: the body the focus plate
/// unfolds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemMember {
    /// Field or variant name; the index for a tuple field.
    pub name: String,
    /// Field type or variant payload, as written. Empty when there is none.
    pub ty: String,
    pub vis: Vis,
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
    pub vis: Vis,
    /// Index into [`CodeGraph::items`]; `None` for impl blocks.
    pub mark: Option<u32>,
    /// Struct fields or enum variants, in source order.
    pub members: Vec<ItemMember>,
    /// A function's signature, without its body.
    pub sig: Option<String>,
    /// Derive names, for the plate's badges.
    pub derives: Vec<String>,
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
