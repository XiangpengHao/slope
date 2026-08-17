//! The dependency graph.
//!
//! Everything here crosses the server-function boundary, so the types stay
//! plain data. Resolution and ranking run server-side, once; the client is
//! handed a graph and works out for itself which part of it to draw.
//!
//! Nothing in this module has a coordinate. Where a crate sits on screen is a
//! function of what the reader has opened, and that answer belongs next to the
//! pane that draws it — not baked into the payload.

use serde::{Deserialize, Serialize};

// Resolution and ranking are server-only; the wasm client never shells out.
#[cfg(not(target_arch = "wasm32"))]
pub mod build;
#[cfg(not(target_arch = "wasm32"))]
pub mod metadata;

// Distances and the why-path are recomputed on every selection, so they run
// client-side: a round trip per click is latency the interaction cannot afford.
pub mod focus;

/// One crate, as a card on the pane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crate {
    pub id: usize,
    pub name: String,
    pub version: String,
    /// Longest-path distance from the workspace members, and the law of the
    /// graph: everything a crate depends on has a strictly greater rank, so
    /// dependencies are always to the right and dependents always to the left,
    /// at every zoom, without exception.
    pub rank: usize,
    pub deps: Vec<usize>,
    pub dependents: Vec<usize>,
    /// A crate this workspace builds, rather than one it pulls in.
    pub is_root: bool,
    /// This crate name resolves at more than one version in the workspace.
    pub duplicate: bool,
    /// What the crate says about where its source lives, when it says anything.
    pub repository: Option<String>,
    /// Resolved from a registry rather than a path or a git checkout. Only these
    /// have a crates.io page — linking one for a path dependency sends the
    /// reader to whatever stranger happens to own that name.
    pub registry: bool,
}

impl Crate {
    /// The card's second line. The version disambiguates a duplicate, and there
    /// is nothing else about a crate that belongs at this size.
    pub fn subtitle(&self) -> String {
        self.version.clone()
    }
}

/// One crate name resolving at several versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub name: String,
    pub versions: Vec<String>,
    pub ids: Vec<usize>,
}

/// Everything the client needs to draw any part of the graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub manifest_dir: String,
    /// Resolved packages, including duplicate versions of the same name.
    pub package_count: usize,
    /// Distinct crate names.
    pub distinct_count: usize,
    /// Declared dependencies across the whole graph.
    pub dependency_count: usize,
    pub crates: Vec<Crate>,
    pub duplicates: Vec<DuplicateGroup>,
}

impl Workspace {
    /// The crates this workspace actually builds, in the order cargo resolved
    /// them. This is where every reading of the graph starts.
    pub fn members(&self) -> impl Iterator<Item = &Crate> {
        self.crates.iter().filter(|entry| entry.is_root)
    }
}

/// A resolved package before ranking. Produced by `metadata`, consumed by
/// `build`. Never crosses the wire.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct RawPackage {
    pub name: String,
    pub version: String,
    /// Indices into the same `Vec<RawPackage>`.
    pub deps: Vec<usize>,
    pub is_root: bool,
    pub repository: Option<String>,
    /// Resolved from a registry, and so has a crates.io page.
    pub registry: bool,
}
