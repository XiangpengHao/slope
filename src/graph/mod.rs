//! The dependency graph, and the board geometry derived from it.
//!
//! Everything in this module crosses the server-function boundary, so the types
//! stay plain data. Resolution (`metadata`) is server-only; layout is pure and
//! compiles everywhere, but only ever runs server-side — the client receives
//! finished coordinates and never computes any of its own.
//!
//! The board is laid out **once**. Selecting a crate moves the camera and
//! changes which copper is lit; it never re-seats a pad. A world that rebuilds
//! itself under you is a world you cannot learn.

use serde::{Deserialize, Serialize};

// Resolution and layout run server-side; the client is handed finished geometry
// for the whole board.
#[cfg(not(target_arch = "wasm32"))]
pub mod layout;

#[cfg(not(target_arch = "wasm32"))]
pub mod metadata;

// Hop distances and the why-path are recomputed on every selection, so they run
// client-side: a round trip per click is latency the interaction cannot afford.
// None of it moves a pad.
pub mod focus;
// Hit-testing only has a job where there is a canvas to point at, so off the
// wasm target this is exercised by its tests and nothing else.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod quadtree;
// Copper is routed once, server-side, at layout time. The client receives the
// finished polylines and never routes anything.
#[cfg(not(target_arch = "wasm32"))]
pub mod route;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// One crate, as a pad on the board.
///
/// Every pad is the same object. Nothing about a crate — how many depend on it,
/// how deep it sits, whether you own it — changes the pad's size or shape. What
/// a crate *is* lives entirely in the copper running into and out of it, which
/// is the one thing a dependency actually is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    pub id: usize,
    pub name: String,
    pub version: String,
    pub x: f32,
    pub y: f32,
    /// Column index: longest-path distance from the workspace members. Also the
    /// law of the board — everything a crate depends on has a strictly greater
    /// rank, so dependencies are always to the right and dependents always to
    /// the left, at every zoom, without exception.
    pub rank: usize,
    pub deps: Vec<usize>,
    pub dependents: Vec<usize>,
    /// A crate this workspace actually builds, rather than one it pulls in.
    pub is_root: bool,
    /// This crate name resolves at more than one version in the workspace.
    pub duplicate: bool,
}

impl Pad {
    /// A pad the whole board routes through. Decides when the legend names it,
    /// never how the pad is drawn: every pad is the same object.
    pub fn major(&self) -> bool {
        self.dependents.len() >= 8
    }

    /// Silkscreen legend. Version rides along only where it disambiguates.
    pub fn label(&self) -> String {
        if self.duplicate {
            format!("{} {}", self.name, self.version)
        } else {
            self.name.clone()
        }
    }

    /// Reference designator: the stable short code a part carries on a board.
    /// Gives the title block something exact to report and survives renaming.
    pub fn designator(&self) -> String {
        format!("P{:03}", self.id)
    }

    /// Zoom tier at which this pad's legend is silkscreened. The board names
    /// what it routes through at the widest view and fills in the rest as you
    /// move in, because 346 names at once is not a legend, it is a texture.
    pub fn legend_tier(&self) -> u8 {
        if self.is_root || self.major() {
            0
        } else if self.dependents.len() >= 3 || self.duplicate {
            1
        } else {
            2
        }
    }
}

/// A routed dependency: copper from the dependent's pad to the dependency's,
/// as a polyline of horizontal, vertical, and 45-degree segments. Board traces
/// chamfer their corners rather than turning square, and that constraint is
/// also what keeps a run traceable by eye.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    /// The crate that declares the dependency.
    pub from: usize,
    /// The crate it depends on.
    pub to: usize,
    pub points: Vec<Point>,
}

/// One crate name resolving at several versions — the board's fab note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub name: String,
    pub versions: Vec<String>,
    pub pad_ids: Vec<usize>,
}

/// Everything the client needs to draw the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub workspace: String,
    pub manifest_dir: String,
    pub package_count: usize,
    pub distinct_count: usize,
    pub pads: Vec<Pad>,
    pub traces: Vec<Trace>,
    pub duplicates: Vec<DuplicateGroup>,
    pub width: f32,
    pub height: f32,
    /// The lattice the board is laid out on. Pads sit on exact multiples of it,
    /// so the renderer can draw the drill grid the parts are actually seated in
    /// rather than a decorative background rule.
    pub pitch: f32,
    pub col_pitch: f32,
}

/// A resolved package before layout. Produced by `metadata`, consumed by
/// `layout`. Never crosses the wire.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq)]
pub struct RawPackage {
    pub name: String,
    pub version: String,
    /// Indices into the same `Vec<RawPackage>`.
    pub deps: Vec<usize>,
    pub is_root: bool,
}
