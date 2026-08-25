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

/// Which crate this is and where it sits: everything true of it before the
/// epoch or the resolve graph says a word.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CrateAt {
    /// Stable node id: `name@version` (`name` alone for a ghost, which has no
    /// resolved version to key on).
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    /// Member of the workspace (vs an external dependency).
    pub(crate) is_member: bool,
    /// A member's directory, relative to the workspace root — where to look
    /// for the change on disk. `None` for externals, and for a member cargo
    /// resolved from outside the workspace root.
    pub(crate) rel_path: Option<String>,
}

/// What one crate's manifest says about itself: the words and links a fact
/// sheet quotes, none of which the resolve graph knows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Words {
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
}

/// Where one crate stands in the graph and in the epoch — everything the
/// analysis worked out about it that its own manifest cannot say.
///
/// Whether a crate *is* changed is not in here: that is `changed_files > 0`,
/// read through [`CrateInfo::is_changed`]. It was a stored `changed` flag
/// until it could be set to disagree with the count beside it, and a recorded
/// answer that can contradict its own evidence is worse than no answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Standing {
    /// Files under this member's directory changed in the epoch.
    pub(crate) changed_files: u32,
    /// This member's Cargo.toml changed in the epoch.
    pub(crate) manifest_changed: bool,
    /// Minimum hops downstream from a changed crate. `Some(0)` = changed
    /// itself; `None` = untouched by the epoch. Read it through
    /// [`CrateInfo::downstream_hops`], which drops the zero.
    pub(crate) affected_dist: Option<u32>,
    /// How many crates in the resolved graph depend on this one. Drives the
    /// star's magnitude.
    pub(crate) dependents: u32,
    /// Direct dependencies.
    pub(crate) direct_deps: u32,
    /// Direct dependencies that are external (non-member) crates.
    pub(crate) external_deps: u32,
}

/// One crate in the resolved graph: which crate it is, what its manifest
/// says, and where it stands once the graph and the epoch have spoken.
///
/// Built through [`CrateInfo::resolved`] or [`CrateInfo::ghost`] and never by
/// literal. Which of the two it is decides fifteen values at once — a ghost
/// has no manifest left to read and nothing in the graph to count — and the
/// two states have nothing in common but a name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CrateInfo {
    pub(crate) at: CrateAt,
    pub(crate) words: Words,
    pub(crate) standing: Standing,
    /// Only present as the target of a removed dependency; no longer in the
    /// resolved graph.
    pub(crate) ghost: bool,
}

/// The two ways a crate enters the graph. Both are the analysis's, so both
/// stay off the client, where a `CrateInfo` only ever arrives deserialized.
#[cfg(any(feature = "server", test))]
impl CrateInfo {
    /// One crate cargo resolved, with what its manifest says and where the
    /// graph and the epoch put it.
    pub(crate) fn resolved(at: CrateAt, words: Words, standing: Standing) -> Self {
        Self {
            at,
            words,
            standing,
            ghost: false,
        }
    }

    /// A crate only a removed dependency names. It is not in the resolved
    /// graph, so there is nothing to count and no manifest left to read: the
    /// name, and whatever version the old manifest asked for, is all there is.
    pub(crate) fn ghost(name: String, version: String) -> Self {
        Self {
            at: CrateAt {
                id: name.clone(),
                name,
                version,
                is_member: false,
                rel_path: None,
            },
            words: Words::default(),
            standing: Standing::default(),
            ghost: true,
        }
    }
}

impl CrateInfo {
    /// The epoch changed files under this crate.
    pub(crate) fn is_changed(&self) -> bool {
        self.standing.changed_files > 0
    }

    /// How far downstream of a change this crate sits — when it is not the
    /// change itself. A changed crate's own `affected_dist` is `Some(0)`, and
    /// no reading wants to call that "zero hops downstream".
    pub(crate) fn downstream_hops(&self) -> Option<u32> {
        self.standing.affected_dist.filter(|&d| d > 0)
    }

    /// A workspace member still in the resolved graph.
    pub(crate) fn is_live_member(&self) -> bool {
        self.at.is_member && !self.ghost
    }

    /// An external dependency still in the resolved graph.
    pub(crate) fn is_external(&self) -> bool {
        !self.at.is_member && !self.ghost
    }
}

/// A dependency edge: `from` depends on `to`. Both reference [`CrateAt::id`].
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
    /// Node id of the workspace's root package, when it has one. A virtual
    /// workspace (members only, no root package) has none.
    pub(crate) root_crate: Option<String>,
    pub(crate) epoch: Epoch,
    pub(crate) crates: Vec<CrateInfo>,
    pub(crate) links: Vec<DepLink>,
}

impl DepGraph {
    /// The crate one node id names, live or ghost.
    pub(crate) fn crate_at(&self, id: &str) -> Option<&CrateInfo> {
        self.crates.iter().find(|c| c.at.id == id)
    }
}
