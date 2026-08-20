//! The data chart's reading of the survey: types as marks, holding as edges.
//!
//! Pure functions over the wire model — no layout and no rendering. The survey
//! ships every type it found, private ones included; this module decides which
//! of them the chart draws as marks, which frame each one sits in, where the
//! rest fold to, and which anchor every edge lands on. Nothing is dropped
//! without a count: a private type folds to its frame's counted row and the
//! edges touching it land there, the way the code map's ties land on a gate.
//!
//! Seating is decided here too, but not measured: each frame gets an ownership
//! forest, every type under its one heaviest same-frame `Owns` holder, and the
//! layout turns that into geometry. The same-frame rule is the whole point —
//! a type never leaves the module that declares it, so ownership reaching
//! across a module stays a drawn line instead of moving a block.
//!
//! Free functions are marks of their own, because a pub fn is a contract the
//! way a pub struct is: its signature's holds edges are what keep a type only
//! functions name from reading as a root nothing reaches. A function is a leaf
//! at both ends of the seating — nothing holds one, and a signature is not
//! containment — so the frames read their contracts after the shapes.
//!
//! Reference ties are computed here too. The survey records references between
//! items; this altitude reads them at type precision, so each endpoint climbs
//! its containment chain to the outermost mark and a tie is kept only when both
//! ends land on a drawn type. References written in a function *body* never
//! reach a type, so they are not on this chart — the legend says so.

use std::collections::{HashMap, HashSet};

use crate::api::{CodeGraph, Delta, GhostMark, HoldEvent, HoldKind, ItemKind, ItemMark, Vis};
use crate::views::codemap::model::Containment;
use crate::views::codemap::{Doors, RefDir};

/// Marks the first paint budgets for. Past it, each frame folds its quietest
/// types into a counted row rather than drawing a wall of blocks.
pub const MARK_BUDGET: usize = 200;
/// Incoming holds edges a type draws before folding them to a count on its own
/// mark. A type four other types reach is a hub, and its fan-in drawn in full
/// is a star burst nobody can read.
const HELD_CAP: usize = 3;
/// Holding fields a resting mark quotes before it defers to a counted line.
/// The mark carries every field either way: selecting it draws them all, which
/// is the only way to read a wide type without leaving the chart.
pub const FIELD_CAP: usize = 8;
/// Resting reference ties whose counts are engraved. Past this the labels are
/// the chart's texture instead of its data.
pub const TIE_LABELS: usize = 12;
/// Reference ties one type rests in an anchored reading.
const TIES_PER_MARK: usize = 2;

/// Where an edge can land: a drawn mark, or one of a frame's counted fold rows.
/// Privacy folds a type for good and the budget folds the quietest ones; either
/// way the edge lands on the row that counts it instead of being cut.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Anchor {
    /// A type or static with a block of its own.
    Mark(u32),
    /// A frame's `+ n private types` row.
    Private(u32),
    /// A frame's `+ n more types` row, where the budget folded the quietest.
    More(u32),
}

/// One seat in a frame's ownership forest: a block, and the blocks that sit
/// under it because it owns them. A counted fold row can seat children too —
/// what only private code owns hangs under the row that counts the private
/// code, because that row is the only holder the chart draws.
#[derive(Clone, PartialEq, Debug)]
pub struct Seat {
    pub anchor: Anchor,
    /// Seated one layer beneath, in the survey's order.
    pub children: Vec<Seat>,
}

impl Seat {
    /// A seat with nothing under it.
    pub fn leaf(anchor: Anchor) -> Self {
        Self {
            anchor,
            children: Vec::new(),
        }
    }
}

/// One frame on the paper: a workspace crate, or one top-level module inside a
/// crate. One level of module frames only — a deeper module path stays in the
/// mark's locator, where rust already writes it.
#[derive(Clone, PartialEq, Debug)]
pub struct Frame {
    pub id: u32,
    pub krate: String,
    /// The top-level module, as rust names it. `None` is the crate's own frame,
    /// which holds the types its crate root declares.
    pub module: Option<String>,
    /// The crate frame a module frame sits in.
    pub parent: Option<u32>,
    /// Drawn marks seated here, in the survey's (file, source) order. The
    /// roster of what the frame draws; `forest` says where each one sits.
    pub marks: Vec<u32>,
    /// Private types, never drawn, counted here.
    pub private: u32,
    /// Types the budget folded away, counted here.
    pub more: u32,
    /// How they seat: the frame's ownership forest, in reading order —
    /// statics, then trees biggest first, then the free functions, then the
    /// vocabulary leaves, then the counted fold rows. Every mark in `marks`
    /// sits somewhere in here exactly once, and a fold row the frame counts is
    /// a seat of its own.
    pub forest: Vec<Seat>,
}

impl Frame {
    /// The label engraved on the frame's border, in rust's own words. A crate
    /// frame names its crate only where the survey has more than one to tell
    /// apart; in a single-crate workspace that name is already the cartouche's.
    pub fn label(&self, multi_crate: bool) -> Option<String> {
        match &self.module {
            Some(module) => Some(format!("mod {module}")),
            None => multi_crate.then(|| self.krate.clone()),
        }
    }
}

/// One quoted row's own diff state, in the diff's own idiom: an added row
/// wears `+`, a dropped one is quoted from the base and struck.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RowState {
    #[default]
    Same,
    Added,
    Removed,
}

impl RowState {
    /// The diff's own marker for the row.
    pub fn marker(self) -> Option<&'static str> {
        match self {
            RowState::Same => None,
            RowState::Added => Some("+"),
            RowState::Removed => Some("−"),
        }
    }

    /// The row's CSS class, empty for an untouched row.
    pub fn class(self) -> &'static str {
        match self {
            RowState::Same => "",
            RowState::Added => "is-add",
            RowState::Removed => "is-del",
        }
    }
}

/// One holding field, quoted from the source: the name as written and the
/// declared type as written. Nothing here is reconstructed.
#[derive(Clone, PartialEq, Debug)]
pub struct FieldRow {
    pub name: String,
    pub decl: String,
    /// The held type's name — the one run of the declaration drawn in full ink,
    /// so `Vec<FileDetail>` reads as the wrapper it is around the type it holds.
    pub target: String,
    /// The row against the diff base. A `Removed` row is the base's, seated
    /// where it stood.
    pub state: RowState,
}

/// One type, static, or free function with a block on the paper.
#[derive(Clone, PartialEq, Debug)]
pub struct DataMark {
    pub id: u32,
    pub frame: u32,
    pub kind: ItemKind,
    pub vis: Vis,
    pub name: String,
    /// The label its definition plate selects by, for the URL.
    pub label: String,
    /// The defining file, relative to the workspace root.
    pub path: String,
    pub line: u32,
    /// How its own declaration differs from the diff base.
    pub delta: Delta,
    /// The base had it, the working copy does not: a ghost, drawn dashed from
    /// the base edition.
    pub ghost: bool,
    /// Fields — a function's parameters — quoted as written in declaration
    /// order, every one of them. A resting block draws [`FIELD_CAP`] and
    /// counts the rest on its foot.
    pub fields: Vec<FieldRow>,
    /// An enum's variants as written — payloads and discriminants included —
    /// quoted as rows (the row text in `decl`, `name` empty), all of them.
    pub variants: Vec<FieldRow>,
    /// A static's declared type or a function's return type, as written.
    pub ty: String,
    /// The workspace type that type reaches, if it reaches one — the run of
    /// `ty` drawn in full ink, as a field row's `target` is. Empty where the
    /// walk found nothing on this chart to hold, which is exactly when the
    /// line draws no holds edge: `GlobalSignal<Option<Viewport>>` names a
    /// type from a dependency, and a dependency has no mark to point at.
    pub ty_target: String,
    /// Incoming holds edges folded to a count: how many types hold this one.
    /// Zero when they are all drawn.
    pub held_by: u32,
    /// The other half of the same fold: how many signatures name this type.
    /// A function keeps nothing, so it is counted apart from the holders.
    pub named_by: u32,
}

impl DataMark {
    /// A static is state no type holds — the chart's other kind of mark.
    pub fn is_static(&self) -> bool {
        self.kind == ItemKind::Static
    }

    /// A free function: a contract rather than a shape. Nothing holds it, its
    /// rows are its parameters, and its `ty` is what it hands back.
    pub fn is_fn(&self) -> bool {
        self.kind == ItemKind::Fn
    }

    /// Where it is written: `src/views/codemap/model.rs:278`. A ghost's line
    /// is the base edition's, and says so.
    pub fn locator(&self) -> String {
        if self.ghost {
            format!("{}:{} (base)", self.path, self.line)
        } else {
            format!("{}:{}", self.path, self.line)
        }
    }

    /// The letter the mark wears, in git's own alphabet: `A`dded since the
    /// base, `D` for a ghost, `M` for a rewritten declaration. `None` where
    /// the base wrote it exactly as it stands — whatever its file did.
    pub fn letter(&self) -> Option<&'static str> {
        if self.ghost {
            return Some("D");
        }
        match self.delta {
            Delta::Added => Some("A"),
            Delta::Changed => Some("M"),
            Delta::Same => None,
        }
    }
}

/// One holding relation, placed on the chart's anchors. The edge is drawn from
/// the held type to its holder, so the arrowhead rests on the holder — the way
/// a shape change travels.
#[derive(Clone, PartialEq, Debug)]
pub struct Hold {
    pub held: Anchor,
    pub holder: Anchor,
    pub kind: HoldKind,
    /// The strongest wrapper on the walk, in its own word. Empty for a plain
    /// hold, which needs none.
    pub via: String,
    /// Fields drawing this edge.
    pub fields: u32,
    /// Drawn at rest. A folded edge stays in the set and inks in the moment the
    /// reader hovers either of its ends.
    pub rest: bool,
    /// The relation against the diff base. Diff ink never folds: an edge with
    /// an event always rests.
    pub event: Option<HoldEvent>,
}

impl Hold {
    pub fn key(&self) -> String {
        format!(
            "{:?}>{:?}:{:?}:{}:{:?}",
            self.held, self.holder, self.kind, self.via, self.event
        )
    }
}

/// Every anchor a shape change to `from` could reach, walking holds edges
/// holder-ward: the transitive holders, and the contracts that name them —
/// a signature has to change with the shape it quotes. `pairs` are (held,
/// holder). A counted fold row can join the set — the edge landing on it is
/// drawn — but the walk ends there: a row is a count, not a type with holders
/// of its own. So does a function: nothing holds one, so nothing is upstream
/// of it.
pub fn upstream(pairs: &[(Anchor, Anchor)], from: Anchor) -> HashSet<Anchor> {
    let mut seen: HashSet<Anchor> = HashSet::new();
    let mut queue: Vec<Anchor> = vec![from];
    while let Some(at) = queue.pop() {
        for (held, holder) in pairs {
            if *held == at
                && *holder != from
                && seen.insert(*holder)
                && matches!(holder, Anchor::Mark(_))
            {
                queue.push(*holder);
            }
        }
    }
    seen
}

/// One reference tie between two types, summed. The arrowhead rests on the
/// user, as it does at every other altitude.
#[derive(Clone, PartialEq, Debug)]
pub struct Tie {
    pub def: Anchor,
    pub user: Anchor,
    pub count: u32,
    /// Drawn at rest under the current reading.
    pub rest: bool,
    /// Heavy enough among the resting ties to carry its count on the paper.
    pub labeled: bool,
}

impl Tie {
    pub fn key(&self) -> String {
        format!("{:?}~{:?}", self.def, self.user)
    }
}

/// Everything one build of the data chart reads out of the survey.
#[derive(Clone, PartialEq, Debug)]
pub struct DataModel {
    pub frames: Vec<Frame>,
    /// Drawn marks, in the survey's (file, source) order.
    pub marks: Vec<DataMark>,
    pub holds: Vec<Hold>,
    pub ties: Vec<Tie>,
    /// More than one crate in the survey: crate frames earn their names.
    pub multi_crate: bool,
    /// The reading this model was built at, so the chart and the sheet word
    /// their visibility fold rows in the same breath the fold was decided.
    pub doors: Doors,
    // ---- Facts for the cartouche. ----
    pub structs: usize,
    pub enums: usize,
    /// Drawn free functions: the surface the chart reads as contracts.
    pub fns: usize,
    /// Statics, plus every drawn type no other type holds. A function is not
    /// one: nothing can hold a function, so counting it would say nothing.
    pub roots: usize,
    /// The structural diff's counts over the drawn marks.
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    /// Top-level modules holding a diff-touched type, in name order.
    pub changed_modules: Vec<String>,
    /// Holds edges whose held end is a workspace trait. Traits get no mark of
    /// their own in v1, so these have nowhere to land.
    pub trait_holds: usize,
}

/// What the cartouche and the legend state about the survey. Small enough to
/// hand the furniture without carrying the whole chart along with it.
#[derive(Clone, PartialEq, Debug)]
pub struct DataFacts {
    pub structs: usize,
    pub enums: usize,
    pub fns: usize,
    pub roots: usize,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub changed_modules: Vec<String>,
    pub trait_holds: usize,
    /// Names the survey could not resolve, straight from the wire model.
    pub unresolved: u32,
}

impl DataModel {
    /// The facts, lifted off the model for the furniture that states them.
    pub fn facts(&self, unresolved: u32) -> DataFacts {
        DataFacts {
            structs: self.structs,
            enums: self.enums,
            fns: self.fns,
            roots: self.roots,
            added: self.added,
            removed: self.removed,
            changed: self.changed,
            changed_modules: self.changed_modules.clone(),
            trait_holds: self.trait_holds,
            unresolved,
        }
    }
}

/// A mark the chart can draw as a block. A static is always drawn, whatever its
/// visibility: it is state no type holds, and the process has no other root.
/// Everything else must clear the door the reviewer set — visibility is a
/// fold, not a mark, and which visibility folds is [`Doors`]. A free function
/// clears it the same way a type does: a pub fn is a contract, and a type only
/// its functions name would otherwise read as a root nothing reaches.
fn drawable(mark: &ItemMark, doors: Doors) -> bool {
    match mark.kind {
        ItemKind::Static => true,
        ItemKind::Struct | ItemKind::Enum | ItemKind::Union => doors.admits(mark.vis),
        ItemKind::Fn => is_free_fn(mark) && doors.admits(mark.vis),
        _ => false,
    }
}

/// A mark the chart could draw or fold: everything the data walk starts from.
fn charted(mark: &ItemMark) -> bool {
    match mark.kind {
        ItemKind::Static | ItemKind::Struct | ItemKind::Enum | ItemKind::Union => true,
        ItemKind::Fn => is_free_fn(mark),
        _ => false,
    }
}

/// A function the file itself declares. A method or an associated function
/// carries the type its impl names as its parent, and stays attributed to it:
/// this altitude charts contracts, and a method's contract is its type's.
fn is_free_fn(mark: &ItemMark) -> bool {
    mark.kind == ItemKind::Fn && mark.parent.is_none()
}

/// The part of a path below the crate's source root — `src/views/star.rs`
/// becomes `views/star.rs`, wherever in the workspace the crate itself sits.
/// A path with no source root of its own keeps its last segment.
fn source_rest(path: &str) -> &str {
    let segments: Vec<&str> = path.split('/').collect();
    let root = segments
        .iter()
        .enumerate()
        .rev()
        .find(|(i, seg)| {
            *i + 1 < segments.len() && matches!(**seg, "src" | "tests" | "benches" | "examples")
        })
        .map(|(i, _)| i);
    match root {
        Some(i) => {
            let cut: usize = segments[..=i].iter().map(|s| s.len() + 1).sum();
            &path[cut..]
        }
        None => path.rsplit('/').next().unwrap_or(path),
    }
}

/// The top-level module a file's types are framed in — the first path segment
/// under the crate's source root, which is exactly how rust names the module.
/// `src/views/codemap/map.rs` is `mod views`; `src/api.rs` is `mod api`; the
/// crate root itself (`main.rs`, `lib.rs`) has no module and frames in the
/// crate.
fn module_of(path: &str) -> Option<&str> {
    let rest = source_rest(path);
    match rest.split_once('/') {
        Some((first, _)) => Some(first),
        None => {
            let stem = rest.strip_suffix(".rs").unwrap_or(rest);
            (!matches!(stem, "main" | "lib" | "mod" | "build")).then_some(stem)
        }
    }
}

/// How loudly a type asks for a block of its own: how many holding edges touch
/// it, how much of the workspace names it, and whether this epoch touched its
/// file. Only used once the chart is over budget.
fn interest(mark: &ItemMark, degree: u32, changed: bool) -> u32 {
    degree + mark.fan_in + if changed { 2 } else { 0 }
}

/// How many marks a seat carries, itself included. A frame reads biggest tree
/// first, so the state with the most shape under it opens the frame.
fn subtree_size(anchor: Anchor, seated: &HashMap<Anchor, Vec<u32>>) -> usize {
    1 + seated.get(&anchor).map_or(0, |kids| {
        kids.iter()
            .map(|&kid| subtree_size(Anchor::Mark(kid), seated))
            .sum::<usize>()
    })
}

/// Grow one seat and everything seated under it.
fn seat_of(anchor: Anchor, seated: &HashMap<Anchor, Vec<u32>>) -> Seat {
    Seat {
        anchor,
        children: seated.get(&anchor).map_or_else(Vec::new, |kids| {
            kids.iter()
                .map(|&kid| seat_of(Anchor::Mark(kid), seated))
                .collect()
        }),
    }
}

/// Whether seating `child` under `candidate` would close a loop. Two types that
/// own each other cannot both sit above the other: the first seat taken stands,
/// and the edge that would have closed the ring stays drawn as a line.
fn would_cycle(child: u32, candidate: Anchor, parents: &HashMap<u32, Anchor>) -> bool {
    let mut at = candidate;
    while let Anchor::Mark(id) = at {
        if id == child {
            return true;
        }
        match parents.get(&id) {
            Some(&up) => at = up,
            None => return false,
        }
    }
    false
}

impl DataModel {
    pub fn build(graph: &CodeGraph, ref_dir: RefDir, doors: Doors) -> Self {
        let changed_file: Vec<bool> = graph.files.iter().map(|f| f.changed).collect();
        let file_changed = |file: u32| changed_file.get(file as usize).copied().unwrap_or(false);

        // Ghosts share the marks' id space, continuing after `items`.
        let ghost_of = |id: u32| -> Option<&GhostMark> {
            (id as usize)
                .checked_sub(graph.items.len())
                .and_then(|at| graph.ghosts.get(at))
        };
        let kind_of = |id: u32| -> Option<ItemKind> {
            graph
                .items
                .get(id as usize)
                .map(|m| m.kind)
                .or_else(|| ghost_of(id).map(|g| g.kind))
        };
        let name_of = |id: u32| -> String {
            graph
                .items
                .get(id as usize)
                .map(|m| m.name.clone())
                .or_else(|| ghost_of(id).map(|g| g.name.clone()))
                .unwrap_or_default()
        };
        let ghost_key = |g: &GhostMark| (g.krate.clone(), module_of(&g.path).map(str::to_string));
        let is_fn = |id: u32| kind_of(id) == Some(ItemKind::Fn);

        // ---- Which marks are drawn, and which fold. ------------------------
        let mut degree = vec![0u32; graph.items.len() + graph.ghosts.len()];
        for edge in &graph.holds {
            if let Some(d) = degree.get_mut(edge.from as usize) {
                *d += 1;
            }
            if let Some(d) = degree.get_mut(edge.to as usize) {
                *d += 1;
            }
        }

        let mut drawn: Vec<u32> = Vec::new();
        let mut private: Vec<u32> = Vec::new();
        for (i, mark) in graph.items.iter().enumerate() {
            if !charted(mark) {
                continue;
            }
            if drawable(mark, doors) {
                drawn.push(i as u32);
            } else {
                private.push(i as u32);
            }
        }

        // Over budget, the quietest types fold to their frame's counted row.
        // Statics never fold: eleven of them are the whole session's state.
        // Neither does anything the diff touched — the diff is what the
        // reviewer came for.
        let mut folded: HashSet<u32> = HashSet::new();
        if drawn.len() > MARK_BUDGET {
            let mut ranked: Vec<u32> = drawn
                .iter()
                .copied()
                .filter(|&m| {
                    graph.items[m as usize].kind != ItemKind::Static
                        && graph.items[m as usize].delta == Delta::Same
                })
                .collect();
            ranked.sort_by_key(|&m| {
                let mark = &graph.items[m as usize];
                (
                    std::cmp::Reverse(interest(mark, degree[m as usize], file_changed(mark.file))),
                    mark.file,
                    mark.line,
                )
            });
            let statics = drawn.len() - ranked.len();
            folded = ranked
                .into_iter()
                .skip(MARK_BUDGET.saturating_sub(statics))
                .collect();
            drawn.retain(|m| !folded.contains(m));
        }

        // ---- Frames: one per crate, one per top-level module inside it. ----
        let file_key: Vec<(String, Option<String>)> = graph
            .files
            .iter()
            .map(|f| (f.krate.clone(), module_of(&f.path).map(str::to_string)))
            .collect();
        let key_of = |mark: u32| -> Option<&(String, Option<String>)> {
            file_key.get(graph.items[mark as usize].file as usize)
        };

        let mut keys: Vec<(String, Option<String>)> = drawn
            .iter()
            .chain(folded.iter())
            .chain(private.iter())
            .filter_map(|&m| key_of(m).cloned())
            .chain(graph.ghosts.iter().map(&ghost_key))
            .collect();
        keys.sort();
        keys.dedup();
        let mut crates: Vec<String> = keys.iter().map(|(krate, _)| krate.clone()).collect();
        crates.dedup();

        let mut frames: Vec<Frame> = Vec::new();
        let mut frame_index: HashMap<(String, Option<String>), u32> = HashMap::new();
        // Crate frames first, so a module frame always has a parent to sit in.
        for krate in &crates {
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: krate.clone(),
                module: None,
                parent: None,
                marks: Vec::new(),
                private: 0,
                more: 0,
                forest: Vec::new(),
            });
            frame_index.insert((krate.clone(), None), id);
        }
        for key in &keys {
            if key.1.is_none() {
                continue;
            }
            let parent = frame_index.get(&(key.0.clone(), None)).copied();
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: key.0.clone(),
                module: key.1.clone(),
                parent,
                marks: Vec::new(),
                private: 0,
                more: 0,
                forest: Vec::new(),
            });
            frame_index.insert(key.clone(), id);
        }
        let frame_of = |mark: u32| -> Option<u32> {
            key_of(mark).and_then(|key| frame_index.get(key).copied())
        };

        let mut anchor_of: Vec<Option<Anchor>> = vec![None; graph.items.len() + graph.ghosts.len()];
        for &m in &drawn {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].marks.push(m);
                anchor_of[m as usize] = Some(Anchor::Mark(m));
            }
        }
        // Ghosts are always drawn: a removed type is diff ink, never folded.
        for ghost in &graph.ghosts {
            if let Some(&frame) = frame_index.get(&ghost_key(ghost)) {
                frames[frame as usize].marks.push(ghost.id);
                anchor_of[ghost.id as usize] = Some(Anchor::Mark(ghost.id));
            }
        }
        for &m in &private {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].private += 1;
                anchor_of[m as usize] = Some(Anchor::Private(frame));
            }
        }
        let mut folded_sorted: Vec<u32> = folded.iter().copied().collect();
        folded_sorted.sort_unstable();
        for m in folded_sorted {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].more += 1;
                anchor_of[m as usize] = Some(Anchor::More(frame));
            }
        }

        // ---- Holds: every edge, landed on an anchor. -----------------------
        let mut trait_holds = 0usize;
        let mut acc: HashMap<(Anchor, Anchor, HoldKind, String, Option<HoldEvent>), u32> =
            HashMap::new();
        for edge in &graph.holds {
            let (holder, held) = (
                anchor_of.get(edge.from as usize).copied().flatten(),
                anchor_of.get(edge.to as usize).copied().flatten(),
            );
            if held.is_none()
                && graph
                    .items
                    .get(edge.to as usize)
                    .is_some_and(|m| m.kind == ItemKind::Trait)
            {
                trait_holds += 1;
            }
            let (Some(holder), Some(held)) = (holder, held) else {
                continue;
            };
            // A type holding itself, or two folded types holding each other,
            // draws nothing: the fold row already counts them both.
            if holder == held {
                continue;
            }
            *acc.entry((held, holder, edge.kind, edge.via.clone(), edge.event))
                .or_default() += edge.fields.len() as u32;
        }
        let mut holds: Vec<Hold> = acc
            .into_iter()
            .map(|((held, holder, kind, via, event), fields)| Hold {
                held,
                holder,
                kind,
                via,
                fields,
                rest: true,
                event,
            })
            .collect();
        let event_ord = |e: Option<HoldEvent>| match e {
            None => 0u8,
            Some(HoldEvent::Added) => 1,
            Some(HoldEvent::Removed) => 2,
        };
        holds.sort_by(|a, b| {
            (a.held, a.holder, a.kind as u8, &a.via, event_ord(a.event)).cmp(&(
                b.held,
                b.holder,
                b.kind as u8,
                &b.via,
                event_ord(b.event),
            ))
        });

        // Who holds what. The arrowhead rests on the holder, so a type's fan-in
        // is the set of edges leaving it — and a type more than three drawn
        // types hold folds them all to a count on its own mark, where hovering
        // either end inks them back in. A removed edge is diff ink, not
        // structure: it neither counts toward the fold nor ever joins it.
        let mut fan_in: HashMap<Anchor, HashSet<Anchor>> = HashMap::new();
        for hold in &holds {
            if hold.event == Some(HoldEvent::Removed) {
                continue;
            }
            fan_in.entry(hold.held).or_default().insert(hold.holder);
        }
        // Only a drawn mark may fold its fan-in: it has a foot to say `held by
        // n types` on. A counted fold row has no room for a second count, so
        // the edges landing on it all stay drawn. What folds is ink, so a
        // signature edge counts toward the fold; what the foot then says
        // keeps the two apart, because a function holds nothing.
        let folded_fan: HashMap<Anchor, (u32, u32)> = fan_in
            .iter()
            .filter(|(anchor, holders)| {
                matches!(anchor, Anchor::Mark(_)) && holders.len() > HELD_CAP
            })
            .map(|(anchor, holders)| {
                let named = holders
                    .iter()
                    .filter(|h| matches!(h, Anchor::Mark(id) if is_fn(*id)))
                    .count() as u32;
                (*anchor, (holders.len() as u32 - named, named))
            })
            .collect();
        for hold in &mut holds {
            hold.rest = hold.event.is_some() || !folded_fan.contains_key(&hold.held);
        }

        // ---- Seating: the ownership forest inside each frame. ---------------
        // Every drawn type sits under its one heaviest same-frame `Owns`
        // holder, so an owns edge is usually a short line to the block right
        // above it. Ownership that crosses a module seats nothing: a type
        // never leaves the frame that declares it, so that edge stays drawn
        // ink and the coupling stays visible instead of being arranged away.
        // Statics are roots, because nothing holds a static. A vocabulary type
        // — one more than [`HELD_CAP`] types hold — is neither child nor
        // parent: seating it would drag half the frame under one block, and
        // its fan-in is already folded to a count on its own mark. A free
        // function is a leaf at both ends: nothing holds a function, and a
        // signature is not containment — a type named by value in a parameter
        // list is not kept there.
        let mut seat_parent: HashMap<u32, Anchor> = HashMap::new();
        for &id in &drawn {
            if matches!(
                graph.items[id as usize].kind,
                ItemKind::Static | ItemKind::Fn
            ) {
                continue;
            }
            let seat = Anchor::Mark(id);
            if folded_fan.contains_key(&seat) {
                continue;
            }
            let Some(home) = frame_of(id) else { continue };
            let mut weight: HashMap<Anchor, u32> = HashMap::new();
            for hold in &holds {
                if hold.held != seat
                    || hold.kind != HoldKind::Owns
                    || hold.holder == seat
                    // A removed edge is not structure; it seats nothing.
                    || hold.event == Some(HoldEvent::Removed)
                {
                    continue;
                }
                if folded_fan.contains_key(&hold.holder) {
                    continue;
                }
                // A signature naming a type says nothing about where the type
                // lives, so a function seats nobody under it.
                if matches!(hold.holder, Anchor::Mark(holder) if is_fn(holder)) {
                    continue;
                }
                let same_frame = match hold.holder {
                    Anchor::Mark(holder) => frame_of(holder) == Some(home),
                    // A frame's private fold row is the drawn stand-in for its
                    // private code, so it can seat what only private code
                    // owns. The `+ n more types` row cannot: it stands for
                    // types the budget took away, not for a place in the
                    // module's shape.
                    Anchor::Private(frame) => frame == home,
                    Anchor::More(_) => false,
                };
                if !same_frame {
                    continue;
                }
                *weight.entry(hold.holder).or_default() += hold.fields;
            }
            // A drawn owner always outranks the private fold row; then the
            // heaviest holder by field count; then the survey's own order.
            let mut candidates: Vec<(Anchor, u32)> = weight.into_iter().collect();
            candidates.sort_by_key(|&(holder, fields)| {
                (
                    matches!(holder, Anchor::Private(_)),
                    std::cmp::Reverse(fields),
                    holder,
                )
            });
            let chosen = candidates
                .iter()
                .map(|&(holder, _)| holder)
                .find(|&holder| !would_cycle(id, holder, &seat_parent));
            if let Some(holder) = chosen {
                seat_parent.insert(id, holder);
            }
        }

        // `drawn` is in the survey's order, so every seat's children are too.
        let mut seated: HashMap<Anchor, Vec<u32>> = HashMap::new();
        for &id in &drawn {
            if let Some(&parent) = seat_parent.get(&id) {
                seated.entry(parent).or_default().push(id);
            }
        }
        for frame in &mut frames {
            // Nothing holds a function, so a function is never vocabulary; the
            // bands stay disjoint and every mark seats exactly once.
            let vocabulary = |m: u32| folded_fan.contains_key(&Anchor::Mark(m)) && !is_fn(m);
            let mut roots: Vec<u32> = frame
                .marks
                .iter()
                .copied()
                .filter(|&m| !seat_parent.contains_key(&m) && !vocabulary(m) && !is_fn(m))
                .collect();
            roots.sort_by_key(|&m| {
                (
                    kind_of(m) != Some(ItemKind::Static),
                    std::cmp::Reverse(subtree_size(Anchor::Mark(m), &seated)),
                    m,
                )
            });
            let mut forest: Vec<Seat> = roots
                .iter()
                .map(|&m| seat_of(Anchor::Mark(m), &seated))
                .collect();
            // Then the contracts, in the survey's order: a function is drawn
            // after the shapes its signature names, because it is a reading of
            // them rather than a place any of them sits.
            forest.extend(
                frame
                    .marks
                    .iter()
                    .copied()
                    .filter(|&m| is_fn(m))
                    .map(|m| Seat::leaf(Anchor::Mark(m))),
            );
            // Then the vocabulary leaves, then the counted rows: what a frame
            // holds back reads last, under everything it draws in full.
            forest.extend(
                frame
                    .marks
                    .iter()
                    .copied()
                    .filter(|&m| vocabulary(m))
                    .map(|m| Seat::leaf(Anchor::Mark(m))),
            );
            if frame.private > 0 {
                forest.push(seat_of(Anchor::Private(frame.id), &seated));
            }
            if frame.more > 0 {
                forest.push(seat_of(Anchor::More(frame.id), &seated));
            }
            frame.forest = forest;
        }

        // ---- The marks themselves. -----------------------------------------
        // Every field and variant is quoted as written; the holds edges say
        // which run of a row names a workspace type, and that run alone is
        // drawn bold. `graph.holds` arrives sorted, so the same survey always
        // writes the same block.
        let drawn_set: HashSet<u32> = drawn
            .iter()
            .copied()
            .chain(graph.ghosts.iter().map(|g| g.id))
            .collect();
        let mut target_of: HashMap<(u32, String), String> = HashMap::new();
        for edge in &graph.holds {
            if !drawn_set.contains(&edge.from) {
                continue;
            }
            let target = name_of(edge.to);
            for (name, _) in &edge.fields {
                // One field can reach two workspace types (`Arc<(A, B)>`); its
                // row bolds the first, and the edges still say the rest.
                target_of
                    .entry((edge.from, name.clone()))
                    .or_insert_with(|| target.clone());
            }
        }
        let target = |id: u32, name: &str| -> String {
            target_of
                .get(&(id, name.to_string()))
                .cloned()
                .unwrap_or_default()
        };
        let vname = |written: &str| -> String {
            written
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        };
        // The base's dropped rows seat back where they stood, struck, so the
        // diff reads in place. Insertions run back to front, so every
        // recorded index still means what it meant.
        let weave = |rows: &mut Vec<FieldRow>, removed: &mut Vec<(usize, FieldRow)>| {
            removed.sort_by_key(|(before, _)| *before);
            for (before, row) in removed.drain(..).rev() {
                let at = before.min(rows.len());
                rows.insert(at, row);
            }
        };

        let mut marks: Vec<DataMark> = drawn
            .iter()
            .filter_map(|&id| {
                let mark = &graph.items[id as usize];
                let frame = frame_of(id)?;
                let file = graph.files.get(mark.file as usize)?;
                let mut fields: Vec<FieldRow> = mark
                    .field_rows
                    .iter()
                    .enumerate()
                    .map(|(at, (name, decl))| FieldRow {
                        name: name.clone(),
                        decl: decl.clone(),
                        target: target(id, name),
                        state: if mark.fields_added.contains(&(at as u32)) {
                            RowState::Added
                        } else {
                            RowState::Same
                        },
                    })
                    .collect();
                let mut dropped: Vec<(usize, FieldRow)> = mark
                    .fields_removed
                    .iter()
                    .map(|(before, name, decl)| {
                        (
                            *before as usize,
                            FieldRow {
                                name: name.clone(),
                                decl: decl.clone(),
                                target: target(id, name),
                                state: RowState::Removed,
                            },
                        )
                    })
                    .collect();
                weave(&mut fields, &mut dropped);
                // A variant's row is its whole written form; the edge that
                // knows its target is filed under the variant's bare name.
                let mut variants: Vec<FieldRow> = mark
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(at, written)| FieldRow {
                        name: String::new(),
                        decl: written.clone(),
                        target: target(id, &vname(written)),
                        state: if mark.variants_added.contains(&(at as u32)) {
                            RowState::Added
                        } else {
                            RowState::Same
                        },
                    })
                    .collect();
                let mut dropped: Vec<(usize, FieldRow)> = mark
                    .variants_removed
                    .iter()
                    .map(|(before, written)| {
                        (
                            *before as usize,
                            FieldRow {
                                name: String::new(),
                                decl: written.clone(),
                                target: target(id, &vname(written)),
                                state: RowState::Removed,
                            },
                        )
                    })
                    .collect();
                weave(&mut variants, &mut dropped);
                Some(DataMark {
                    id,
                    frame,
                    kind: mark.kind,
                    vis: mark.vis,
                    name: mark.name.clone(),
                    label: mark.label.clone(),
                    path: file.path.clone(),
                    line: mark.line,
                    delta: mark.delta,
                    ghost: false,
                    fields,
                    variants,
                    ty: mark.ty.clone(),
                    // A static's one edge is filed under the static's own
                    // name, the way a field's is under the field's — and so is
                    // a function's return type.
                    ty_target: target(id, &mark.name),
                    held_by: folded_fan
                        .get(&Anchor::Mark(id))
                        .map_or(0, |&(held, _)| held),
                    named_by: folded_fan
                        .get(&Anchor::Mark(id))
                        .map_or(0, |&(_, named)| named),
                })
            })
            .collect();
        // Ghosts: whole blocks quoted from the base edition. Their rows are
        // the base's own — the block's dashed frame and `D` say the rest.
        for ghost in &graph.ghosts {
            let Some(&frame) = frame_index.get(&ghost_key(ghost)) else {
                continue;
            };
            marks.push(DataMark {
                id: ghost.id,
                frame,
                kind: ghost.kind,
                vis: ghost.vis,
                name: ghost.name.clone(),
                label: ghost.name.clone(),
                path: ghost.path.clone(),
                line: ghost.line,
                delta: Delta::Same,
                ghost: true,
                fields: ghost
                    .field_rows
                    .iter()
                    .map(|(name, decl)| FieldRow {
                        name: name.clone(),
                        decl: decl.clone(),
                        target: target(ghost.id, name),
                        state: RowState::Same,
                    })
                    .collect(),
                variants: ghost
                    .variants
                    .iter()
                    .map(|written| FieldRow {
                        name: String::new(),
                        decl: written.clone(),
                        target: target(ghost.id, &vname(written)),
                        state: RowState::Same,
                    })
                    .collect(),
                ty: ghost.ty.clone(),
                ty_target: target(ghost.id, &ghost.name),
                held_by: 0,
                named_by: 0,
            });
        }

        // ---- Reference ties, at type precision. ----------------------------
        let containment = Containment::build(graph);
        let is_type = |m: u32| -> bool {
            drawn_set.contains(&m)
                && matches!(
                    graph.items[m as usize].kind,
                    ItemKind::Struct | ItemKind::Enum | ItemKind::Union
                )
        };
        let mut tie_acc: HashMap<(u32, u32), u32> = HashMap::new();
        for edge in &graph.item_edges {
            let (Some(from), Some(to)) = (edge.from, edge.to) else {
                continue;
            };
            let (user, def) = (containment.root(from), containment.root(to));
            if user == def || !is_type(user) || !is_type(def) {
                continue;
            }
            *tie_acc.entry((def, user)).or_default() += edge.count;
        }
        let mut ties: Vec<Tie> = tie_acc
            .into_iter()
            .map(|((def, user), count)| Tie {
                def: Anchor::Mark(def),
                user: Anchor::Mark(user),
                count,
                rest: true,
                labeled: false,
            })
            .collect();
        ties.sort_by(|a, b| {
            (a.def, a.user)
                .cmp(&(b.def, b.user))
                .then(b.count.cmp(&a.count))
        });

        // Which ties rest on the paper. Direction alone cannot thin the chart —
        // every tie is one type's use and another's users — so each reading
        // anchors on the types themselves and hands the rest back on hover.
        if let Some(cap) = ref_dir.per_territory().map(|c| c.min(TIES_PER_MARK)) {
            let mut by_anchor: HashMap<Anchor, Vec<usize>> = HashMap::new();
            for (i, tie) in ties.iter().enumerate() {
                let anchor = match ref_dir {
                    RefDir::UsedBy => tie.def,
                    _ => tie.user,
                };
                by_anchor.entry(anchor).or_default().push(i);
            }
            let resting: HashSet<usize> = by_anchor
                .into_values()
                .flat_map(|mut idx| {
                    idx.sort_unstable_by_key(|&i| (std::cmp::Reverse(ties[i].count), i));
                    idx.into_iter().take(cap)
                })
                .collect();
            for (i, tie) in ties.iter_mut().enumerate() {
                tie.rest = resting.contains(&i);
            }
        }
        let label_bar = {
            let mut counts: Vec<u32> = ties.iter().filter(|t| t.rest).map(|t| t.count).collect();
            counts.sort_unstable_by(|a, b| b.cmp(a));
            counts.get(TIE_LABELS).copied().unwrap_or(0).max(2)
        };
        for tie in &mut ties {
            tie.labeled = tie.rest && tie.count > label_bar;
        }

        // ---- Facts. ---------------------------------------------------------
        // The workspace's counts are the working copy's: a ghost is drawn,
        // never counted as current code.
        let structs = marks
            .iter()
            .filter(|m| !m.ghost && matches!(m.kind, ItemKind::Struct | ItemKind::Union))
            .count();
        let enums = marks
            .iter()
            .filter(|m| !m.ghost && m.kind == ItemKind::Enum)
            .count();
        let fns = marks
            .iter()
            .filter(|m| !m.ghost && m.kind == ItemKind::Fn)
            .count();
        // A root is state nothing else holds: every static, and every type no
        // other type has a field of. A function is counted as a contract
        // instead — nothing can hold one, so "root" would be true of every
        // function and mean nothing.
        let roots = marks
            .iter()
            .filter(|m| {
                !m.ghost && m.kind != ItemKind::Fn && !fan_in.contains_key(&Anchor::Mark(m.id))
            })
            .count();
        let added = marks.iter().filter(|m| m.delta == Delta::Added).count();
        let removed = marks.iter().filter(|m| m.ghost).count();
        let changed = marks.iter().filter(|m| m.delta == Delta::Changed).count();
        let mut changed_modules: Vec<String> = marks
            .iter()
            .filter(|m| m.letter().is_some())
            .map(|m| {
                let frame = &frames[m.frame as usize];
                frame.module.clone().unwrap_or_else(|| frame.krate.clone())
            })
            .collect();
        changed_modules.sort();
        changed_modules.dedup();

        let multi_crate = crates.len() > 1;
        Self {
            frames,
            marks,
            holds,
            ties,
            multi_crate,
            doors,
            structs,
            enums,
            fns,
            roots,
            added,
            removed,
            changed,
            changed_modules,
            trait_holds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Delta, FileInfo, HoldEdge, HoldEvent, ItemEdge};

    fn file(id: u32, path: &str, changed: bool) -> FileInfo {
        FileInfo {
            id,
            path: path.to_string(),
            krate: "slope".to_string(),
            changed,
            lines: 100,
            items: 2,
            refs_in_files: 0,
        }
    }

    fn mark(id: u32, file: u32, name: &str, kind: ItemKind, vis: Vis) -> ItemMark {
        ItemMark {
            id,
            file,
            local: id,
            name: name.to_string(),
            label: name.to_string(),
            kind,
            vis,
            line: id + 1,
            parent: None,
            fan_in: 0,
            impls: Vec::new(),
            field_rows: Vec::new(),
            variants: Vec::new(),
            ty: String::new(),
            delta: Delta::Same,
            fields_added: Vec::new(),
            fields_removed: Vec::new(),
            variants_added: Vec::new(),
            variants_removed: Vec::new(),
        }
    }

    #[test]
    fn a_module_frame_is_the_first_segment_under_src() {
        assert_eq!(module_of("src/views/codemap/map.rs"), Some("views"));
        assert_eq!(module_of("src/api.rs"), Some("api"));
        assert_eq!(module_of("src/main.rs"), None);
        assert_eq!(module_of("crates/engine/src/lib.rs"), None);
        assert_eq!(module_of("crates/engine/src/parse/lex.rs"), Some("parse"));
    }

    /// `Wire` (pub, in `mod api`) is held by `Index` (pub, in `mod analyze`)
    /// and by `Hidden` (private, same module).
    fn graph() -> CodeGraph {
        let mut index = mark(1, 1, "Index", ItemKind::Struct, Vis::Pub);
        index.field_rows = vec![("wire".into(), "Wire".into())];
        let mut cache = mark(3, 1, "CACHE", ItemKind::Static, Vis::Private);
        cache.ty = "OnceCell<Arc<Index>>".to_string();
        CodeGraph {
            files: vec![
                file(0, "src/api.rs", false),
                file(1, "src/analyze/code.rs", true),
            ],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub),
                index,
                mark(2, 1, "Hidden", ItemKind::Struct, Vis::Private),
                cache,
            ],
            item_edges: vec![ItemEdge {
                from_file: 1,
                from: Some(1),
                to_file: 0,
                to: Some(0),
                count: 4,
            }],
            holds: vec![
                HoldEdge {
                    from: 1,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("wire".into(), "Wire".into())],
                    event: None,
                },
                HoldEdge {
                    from: 2,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("wire".into(), "Wire".into())],
                    event: None,
                },
                HoldEdge {
                    from: 3,
                    to: 1,
                    kind: HoldKind::Shares,
                    via: "Arc".into(),
                    fields: vec![("CACHE".into(), "OnceCell<Arc<Index>>".into())],
                    event: None,
                },
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    #[test]
    fn privacy_folds_a_type_and_keeps_its_edge() {
        let model = DataModel::build(&graph(), RefDir::Uses, Doors::Crate);
        // Two module frames under one crate frame; the crate frame is empty
        // but holds them.
        assert_eq!(model.frames.len(), 3);
        assert!(!model.multi_crate);
        let api = model
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some("api"))
            .unwrap();
        assert_eq!(api.marks.len(), 1);
        let analyze = model
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some("analyze"))
            .unwrap();
        // The static is drawn although it is private; the struct is not.
        assert_eq!(analyze.marks.len(), 2);
        assert_eq!(analyze.private, 1);
        // The private holder's edge lands on the frame's counted row.
        assert!(
            model
                .holds
                .iter()
                .any(|h| h.holder == Anchor::Private(analyze.id) && h.held == Anchor::Mark(0))
        );
        // Only the static is a root here: nothing can hold a static, and both
        // types are held — `Wire` by `Index`, `Index` by the static itself.
        assert_eq!(model.roots, 1);
        assert_eq!(model.structs, 2);
    }

    /// The doors setting is a floor, not a filter on one visibility: raising
    /// it to `pub` folds the crate-visible types beside the private ones, and
    /// dropping it to `All` folds nothing at all.
    #[test]
    fn the_doors_setting_moves_the_visibility_fold() {
        let mut g = graph();
        // `Index` (pub, in `mod analyze`) becomes crate-visible.
        g.items[1].vis = Vis::Crate;

        // At `pub` it folds in with the private type: one drawn static left,
        // and the frame counts two behind its row.
        let shut = DataModel::build(&g, RefDir::Uses, Doors::Pub);
        let analyze = shut
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some("analyze"))
            .unwrap();
        assert_eq!(analyze.marks.len(), 1);
        assert_eq!(analyze.private, 2);
        assert!(!shut.marks.iter().any(|m| m.name == "Index"));
        // The static still holds `Index`, so that edge lands on the row.
        assert!(
            shut.holds
                .iter()
                .any(|h| h.held == Anchor::Private(analyze.id) && h.holder == Anchor::Mark(3))
        );
        assert_eq!(shut.doors.fold_word(), "internal type");

        // At `pub(crate)` it is drawn again, and only the private type folds.
        let open = DataModel::build(&g, RefDir::Uses, Doors::Crate);
        let analyze = open
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some("analyze"))
            .unwrap();
        assert_eq!(analyze.marks.len(), 2);
        assert_eq!(analyze.private, 1);
        assert_eq!(open.doors.fold_word(), "private type");

        // At `private` nothing folds for visibility: every charted type is a
        // mark, and no frame carries a counted row.
        let all = DataModel::build(&g, RefDir::Uses, Doors::All);
        assert_eq!(all.marks.len(), 4);
        assert!(all.frames.iter().all(|f| f.private == 0));
        assert!(
            all.holds
                .iter()
                .all(|h| !matches!(h.held, Anchor::Private(_)))
                && all
                    .holds
                    .iter()
                    .all(|h| !matches!(h.holder, Anchor::Private(_)))
        );
        // `Hidden` holds `Wire` and nothing holds `Hidden`, so it joins the
        // roots the moment it is drawn.
        assert!(all.marks.iter().any(|m| m.name == "Hidden"));
        assert_eq!(all.structs, 3);
    }

    #[test]
    fn a_static_quotes_its_type_instead_of_a_field_row() {
        let model = DataModel::build(&graph(), RefDir::Uses, Doors::Crate);
        let cache = model.marks.iter().find(|m| m.name == "CACHE").unwrap();
        assert!(cache.is_static());
        assert!(cache.fields.is_empty());
        // Its edge is filed under its own name, so the quoted line knows which
        // run to draw in full ink — the same run the drawn edge lands on.
        assert_eq!(cache.ty, "OnceCell<Arc<Index>>");
        assert_eq!(cache.ty_target, "Index");
        let index = model.marks.iter().find(|m| m.name == "Index").unwrap();
        assert_eq!(index.fields.len(), 1);
        assert_eq!(index.fields[0].target, "Wire");
        // A changed file no longer marks a type by itself: the letter is the
        // declaration's own delta.
        assert_eq!(index.letter(), None);
        assert!(model.changed_modules.is_empty());
    }

    #[test]
    fn the_diff_draws_ghosts_and_interleaves_base_rows() {
        let mut g = graph();
        let ghost_id = g.items.len() as u32;
        // The working copy dropped `refs: Vec<FileRef>` from `Index`, and
        // `FileRef` itself.
        g.items[1].delta = Delta::Changed;
        g.items[1].fields_removed = vec![(1, "refs".into(), "Vec<FileRef>".into())];
        g.ghosts.push(crate::api::GhostMark {
            id: ghost_id,
            path: "src/api.rs".into(),
            krate: "slope".into(),
            name: "FileRef".into(),
            kind: ItemKind::Struct,
            vis: Vis::Pub,
            line: 9,
            field_rows: vec![("from".into(), "u32".into())],
            variants: Vec::new(),
            ty: String::new(),
        });
        g.holds.push(HoldEdge {
            from: 1,
            to: ghost_id,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: vec![("refs".into(), "Vec<FileRef>".into())],
            event: Some(HoldEvent::Removed),
        });
        let model = DataModel::build(&g, RefDir::Uses, Doors::Crate);
        let ghost = model.marks.iter().find(|m| m.name == "FileRef").unwrap();
        assert!(ghost.ghost);
        assert_eq!(ghost.letter(), Some("D"));
        assert_eq!(ghost.locator(), "src/api.rs:9 (base)");
        // The removed edge is drawn, resting, from the ghost to its holder.
        assert!(model.holds.iter().any(|h| h.held == Anchor::Mark(ghost_id)
            && h.holder == Anchor::Mark(1)
            && h.event == Some(HoldEvent::Removed)
            && h.rest));
        // The base's row seats back where it stood, struck.
        let index = model.marks.iter().find(|m| m.name == "Index").unwrap();
        assert_eq!(index.letter(), Some("M"));
        assert_eq!(index.fields.len(), 2);
        assert_eq!(index.fields[1].name, "refs");
        assert_eq!(index.fields[1].state, RowState::Removed);
        assert_eq!((model.removed, model.changed), (1, 1));
        assert_eq!(
            model.changed_modules,
            vec!["analyze".to_string(), "api".to_string()]
        );
        // A ghost is drawn, never counted as current code.
        assert_eq!(model.structs, 2);
    }

    #[test]
    fn ties_land_on_types_only() {
        let model = DataModel::build(&graph(), RefDir::Both, Doors::Crate);
        assert_eq!(model.ties.len(), 1);
        assert_eq!(model.ties[0].def, Anchor::Mark(0));
        assert_eq!(model.ties[0].user, Anchor::Mark(1));
        assert_eq!(model.ties[0].count, 4);
    }

    // ---- Seating: the ownership forest inside a frame. ---------------------

    fn holds(from: u32, to: u32, fields: &[&str]) -> HoldEdge {
        HoldEdge {
            from,
            to,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: fields
                .iter()
                .map(|name| ((*name).to_string(), "T".to_string()))
                .collect(),
            event: None,
        }
    }

    /// A frame with something to seat. In `mod api`: `Wire` owns `Leaf` and
    /// `Node`, `Node` also owns `Leaf` (the same weight, later in the survey),
    /// the `CACHE` static owns `Node` with one field where `Wire` owns it with
    /// two, four types hold `Id`, and the private `Hidden` is all that owns
    /// `Orphan`. A module away, `Index` owns `Wire`.
    fn seating_graph() -> CodeGraph {
        CodeGraph {
            files: vec![
                file(0, "src/api.rs", false),
                file(1, "src/analyze/code.rs", false),
            ],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub),
                mark(1, 0, "Leaf", ItemKind::Struct, Vis::Pub),
                mark(2, 0, "Node", ItemKind::Struct, Vis::Pub),
                mark(3, 0, "Orphan", ItemKind::Struct, Vis::Pub),
                mark(4, 0, "Id", ItemKind::Struct, Vis::Pub),
                mark(5, 0, "Hidden", ItemKind::Struct, Vis::Private),
                mark(6, 0, "CACHE", ItemKind::Static, Vis::Private),
                mark(7, 1, "Index", ItemKind::Struct, Vis::Pub),
            ],
            item_edges: Vec::new(),
            // Sorted by (from, to), the way the survey ships them.
            holds: vec![
                holds(0, 1, &["leaf"]),
                holds(0, 2, &["head", "tail"]),
                holds(0, 4, &["id"]),
                // Nothing can have a field of a static, so the survey never
                // writes this edge. The rule that a static is a root is the
                // chart's own, and it is worth a guard.
                holds(0, 6, &["cache"]),
                holds(1, 4, &["id"]),
                holds(2, 1, &["leaf"]),
                holds(2, 4, &["id"]),
                holds(3, 4, &["id"]),
                holds(5, 3, &["orphan"]),
                holds(6, 2, &["node"]),
                holds(7, 0, &["wire"]),
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    fn frame_named<'a>(model: &'a DataModel, module: &str) -> &'a Frame {
        model
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some(module))
            .unwrap()
    }

    /// Every seat in a forest, with the ownership depth it sits at.
    fn walk(seats: &[Seat], depth: usize, out: &mut Vec<(Anchor, usize)>) {
        for seat in seats {
            out.push((seat.anchor, depth));
            walk(&seat.children, depth + 1, out);
        }
    }

    fn roots(frame: &Frame) -> Vec<Anchor> {
        frame.forest.iter().map(|s| s.anchor).collect()
    }

    #[test]
    fn a_type_seats_under_the_same_frame_owner_that_holds_it_hardest() {
        let model = DataModel::build(&seating_graph(), RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        let wire = api
            .forest
            .iter()
            .find(|s| s.anchor == Anchor::Mark(0))
            .unwrap();
        // `Leaf` is owned by `Wire` and by `Node` with one field each; the
        // survey's order breaks the tie. `Node` is owned by `Wire` with two
        // fields and by `CACHE` with one, so weight decides.
        assert_eq!(
            wire.children.iter().map(|s| s.anchor).collect::<Vec<_>>(),
            vec![Anchor::Mark(1), Anchor::Mark(2)]
        );
        // Ownership depth is the layer: `Leaf` sits one under `Wire`.
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert!(seats.contains(&(Anchor::Mark(1), 1)));
    }

    #[test]
    fn a_type_owned_from_another_module_is_a_root_and_keeps_its_edge() {
        let model = DataModel::build(&seating_graph(), RefDir::Uses, Doors::Crate);
        // `Index` owns `Wire`, but it is a module away: `Wire` stays a root of
        // its own frame rather than moving into `mod analyze`.
        assert!(roots(frame_named(&model, "api")).contains(&Anchor::Mark(0)));
        assert_eq!(roots(frame_named(&model, "analyze")), vec![Anchor::Mark(7)]);
        // And the ownership is still ink on the paper.
        assert!(model.holds.iter().any(|h| h.held == Anchor::Mark(0)
            && h.holder == Anchor::Mark(7)
            && h.kind == HoldKind::Owns));
    }

    #[test]
    fn a_static_never_seats_under_a_type() {
        let model = DataModel::build(&seating_graph(), RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert!(seats.contains(&(Anchor::Mark(6), 0)));
        assert!(!seats.iter().any(|&(a, d)| a == Anchor::Mark(6) && d > 0));
    }

    #[test]
    fn a_frame_seats_statics_then_trees_then_vocabulary_then_its_fold_rows() {
        let model = DataModel::build(&seating_graph(), RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        assert_eq!(
            roots(api),
            vec![
                // The static register first,
                Anchor::Mark(6),
                // then the trees, biggest first — `Wire` carries two,
                Anchor::Mark(0),
                // then `Id`, which four types hold: never seated, never a seat,
                Anchor::Mark(4),
                // then what the frame does not draw.
                Anchor::Private(api.id),
            ]
        );
        assert_eq!(
            model.marks.iter().find(|m| m.name == "Id").unwrap().held_by,
            4
        );
        // `Orphan` is owned by private code alone, so it hangs under the row
        // that counts the private code.
        assert_eq!(api.forest[3].children.len(), 1);
        assert_eq!(api.forest[3].children[0].anchor, Anchor::Mark(3));
    }

    #[test]
    fn every_drawn_mark_sits_in_its_frame_exactly_once() {
        let model = DataModel::build(&seating_graph(), RefDir::Uses, Doors::Crate);
        for frame in &model.frames {
            let mut seats = Vec::new();
            walk(&frame.forest, 0, &mut seats);
            let mut seated: Vec<u32> = seats
                .iter()
                .filter_map(|(a, _)| match a {
                    Anchor::Mark(id) => Some(*id),
                    _ => None,
                })
                .collect();
            seated.sort_unstable();
            let mut roster = frame.marks.clone();
            roster.sort_unstable();
            assert_eq!(seated, roster, "frame {:?}", frame.module);
        }
    }

    #[test]
    fn two_types_that_own_each_other_seat_once() {
        let graph = CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "A", ItemKind::Struct, Vis::Pub),
                mark(1, 0, "B", ItemKind::Struct, Vis::Pub),
            ],
            item_edges: Vec::new(),
            holds: vec![holds(0, 1, &["b"]), holds(1, 0, &["a"])],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        };
        let model = DataModel::build(&graph, RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        // One seat takes the other; the ring is not seated twice.
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert_eq!(seats.len(), 2);
        assert_eq!(api.forest.len(), 1);
        // Both edges are still drawn — the closing one included.
        assert_eq!(model.holds.len(), 2);
    }

    #[test]
    fn the_same_survey_always_seats_the_same_forest() {
        let graph = seating_graph();
        let a = DataModel::build(&graph, RefDir::Uses, Doors::Crate);
        let b = DataModel::build(&graph, RefDir::Uses, Doors::Crate);
        assert_eq!(a, b);
    }

    // ---- Contracts: free functions as marks. -------------------------------

    /// A free function's mark as the survey writes one: its parameters in the
    /// rows a struct uses for fields, its return type in the line a static
    /// uses for its declared type.
    fn func(
        id: u32,
        file: u32,
        name: &str,
        vis: Vis,
        params: &[(&str, &str)],
        ret: &str,
    ) -> ItemMark {
        let mut f = mark(id, file, name, ItemKind::Fn, vis);
        f.field_rows = params
            .iter()
            .map(|(n, d)| ((*n).to_string(), (*d).to_string()))
            .collect();
        f.ty = ret.to_string();
        f
    }

    fn sig(from: u32, to: u32, kind: HoldKind, via: &str, fields: &[(&str, &str)]) -> HoldEdge {
        HoldEdge {
            from,
            to,
            kind,
            via: via.to_string(),
            fields: fields
                .iter()
                .map(|(n, d)| ((*n).to_string(), (*d).to_string()))
                .collect(),
            event: None,
        }
    }

    /// One module's contracts. In `mod api`: `pub fn survey(graph: &Wire) ->
    /// Nut` borrows `Wire` and hands back `Nut`, `sweep` is private, `Wire::id`
    /// is a method, and the `CACHE` static holds nothing this chart draws.
    fn contract_graph() -> CodeGraph {
        let mut method = mark(4, 0, "id", ItemKind::Fn, Vis::Pub);
        method.parent = Some(0);
        let mut cache = mark(5, 0, "CACHE", ItemKind::Static, Vis::Private);
        cache.ty = "u8".to_string();
        CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub),
                mark(1, 0, "Nut", ItemKind::Struct, Vis::Pub),
                func(2, 0, "survey", Vis::Pub, &[("graph", "&Wire")], "Nut"),
                func(3, 0, "sweep", Vis::Private, &[], ""),
                method,
                cache,
            ],
            item_edges: Vec::new(),
            holds: vec![
                sig(2, 0, HoldKind::Borrows, "&", &[("graph", "&Wire")]),
                // The return type's edge is filed under the function's own
                // name, the way a static's is under the static's.
                sig(2, 1, HoldKind::Owns, "", &[("survey", "Nut")]),
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    #[test]
    fn a_free_function_is_a_mark_and_the_door_folds_it_like_a_type() {
        let model = DataModel::build(&contract_graph(), RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        // `Wire`, `Nut`, `survey`, `CACHE` — and the private function counted
        // behind the same row a private type would be.
        assert_eq!(api.marks.len(), 4);
        assert_eq!(api.private, 1);
        assert!(model.marks.iter().any(|m| m.name == "survey" && m.is_fn()));
        assert!(!model.marks.iter().any(|m| m.name == "sweep"));
        // A method is its type's contract, not the file's: no mark, and not
        // counted in the fold either.
        assert!(!model.marks.iter().any(|m| m.name == "id"));
        // A function is counted as a contract, never as a root: nothing can
        // hold one, so `root` would be true of every function and mean
        // nothing. Both types are named by the signature, so neither is one.
        assert_eq!((model.structs, model.fns, model.roots), (2, 1, 1));

        // At `private` the door opens on the quiet function too.
        let all = DataModel::build(&contract_graph(), RefDir::Uses, Doors::All);
        assert!(all.marks.iter().any(|m| m.name == "sweep"));
        assert_eq!(all.fns, 2);
    }

    #[test]
    fn a_signature_quotes_its_parameters_and_carries_its_wrapper() {
        let model = DataModel::build(&contract_graph(), RefDir::Uses, Doors::Crate);
        let survey = model.marks.iter().find(|m| m.name == "survey").unwrap();
        assert_eq!(survey.fields.len(), 1);
        assert_eq!(survey.fields[0].name, "graph");
        assert_eq!(survey.fields[0].target, "Wire");
        // The return type stands in the static's slot and bolds the same way.
        assert_eq!(
            (survey.ty.as_str(), survey.ty_target.as_str()),
            ("Nut", "Nut")
        );
        // A parameter taken by reference borrows, and the edge carries the
        // wrapper's own word — the field walk's table, unchanged.
        assert!(model.holds.iter().any(|h| h.held == Anchor::Mark(0)
            && h.holder == Anchor::Mark(2)
            && h.kind == HoldKind::Borrows
            && h.via == "&"));
    }

    #[test]
    fn a_function_seats_nothing_and_sits_under_nothing() {
        let model = DataModel::build(&contract_graph(), RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        // `survey` hands back `Nut` by value, and `Nut` still stands on its
        // own ground: a signature is not containment.
        assert!(seats.iter().all(|&(_, depth)| depth == 0));
        assert!(seats.contains(&(Anchor::Mark(1), 0)));
        // And the ownership is still ink on the paper.
        assert!(model.holds.iter().any(|h| h.held == Anchor::Mark(1)
            && h.holder == Anchor::Mark(2)
            && h.kind == HoldKind::Owns));
    }

    #[test]
    fn a_frame_reads_statics_then_trees_then_contracts_then_vocabulary() {
        let mut g = contract_graph();
        // `Id` is reached by four marks — two types, a static, and one
        // signature — so it is vocabulary: never seated, never a seat.
        g.items.push(mark(6, 0, "Id", ItemKind::Struct, Vis::Pub));
        g.items[2].field_rows.push(("id".into(), "Id".into()));
        g.holds.push(holds(0, 6, &["id"]));
        g.holds.push(holds(1, 6, &["id"]));
        g.holds.push(sig(2, 6, HoldKind::Owns, "", &[("id", "Id")]));
        g.holds.push(holds(5, 6, &["CACHE"]));
        let model = DataModel::build(&g, RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        assert_eq!(
            roots(api),
            vec![
                // The static register first,
                Anchor::Mark(5),
                // then the trees, in the survey's order at equal size,
                Anchor::Mark(0),
                Anchor::Mark(1),
                // then the contracts,
                Anchor::Mark(2),
                // then the vocabulary leaf, then what the frame does not draw.
                Anchor::Mark(6),
                Anchor::Private(api.id),
            ]
        );
        let id = model.marks.iter().find(|m| m.name == "Id").unwrap();
        // The fold counts every arrow it folded, and says the two kinds of
        // arrow apart: a signature names a type without holding it.
        assert_eq!((id.held_by, id.named_by), (3, 1));
    }

    #[test]
    fn the_budget_folds_contracts_and_never_statics() {
        let mut items = vec![mark(0, 0, "CACHE", ItemKind::Static, Vis::Private)];
        for id in 1..=(MARK_BUDGET as u32 + 1) {
            items.push(func(id, 0, &format!("f{id}"), Vis::Pub, &[], ""));
        }
        let graph = CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items,
            item_edges: Vec::new(),
            holds: Vec::new(),
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        };
        let model = DataModel::build(&graph, RefDir::Uses, Doors::Crate);
        let api = frame_named(&model, "api");
        // The two quietest functions fold to the counted row; the static
        // stands, because it is the one mark with nowhere else to be counted.
        assert_eq!(api.marks.len(), MARK_BUDGET);
        assert_eq!(api.more, 2);
        assert!(model.marks.iter().any(|m| m.name == "CACHE"));
        assert_eq!(roots(api).last(), Some(&Anchor::More(api.id)));
    }

    #[test]
    fn a_dropped_function_ghosts_and_a_dropped_parameter_weaves_back() {
        let mut g = contract_graph();
        let ghost_id = g.items.len() as u32;
        // The working copy dropped `quiet: bool` from `survey`, and the whole
        // of `pub fn sweep_all(wire: &Wire)`.
        g.items[2].delta = Delta::Changed;
        g.items[2].fields_removed = vec![(1, "quiet".into(), "bool".into())];
        g.ghosts.push(crate::api::GhostMark {
            id: ghost_id,
            path: "src/api.rs".into(),
            krate: "slope".into(),
            name: "sweep_all".into(),
            kind: ItemKind::Fn,
            vis: Vis::Pub,
            line: 40,
            field_rows: vec![("wire".into(), "&Wire".into())],
            variants: Vec::new(),
            ty: String::new(),
        });
        g.holds.push(HoldEdge {
            from: ghost_id,
            to: 0,
            kind: HoldKind::Borrows,
            via: "&".into(),
            fields: vec![("wire".into(), "&Wire".into())],
            event: Some(HoldEvent::Removed),
        });
        let model = DataModel::build(&g, RefDir::Uses, Doors::Crate);

        let ghost = model.marks.iter().find(|m| m.name == "sweep_all").unwrap();
        assert!(ghost.ghost && ghost.is_fn());
        assert_eq!(ghost.letter(), Some("D"));
        assert_eq!(ghost.fields[0].target, "Wire");
        // The removed signature edge is drawn, resting, from the type the base
        // named to the contract that named it.
        assert!(model.holds.iter().any(|h| h.held == Anchor::Mark(0)
            && h.holder == Anchor::Mark(ghost_id)
            && h.event == Some(HoldEvent::Removed)
            && h.rest));
        // A ghost contract seats in the frame its path names, in the fn band.
        assert!(roots(frame_named(&model, "api")).contains(&Anchor::Mark(ghost_id)));

        // The dropped parameter seats back where it stood, struck.
        let survey = model.marks.iter().find(|m| m.name == "survey").unwrap();
        assert_eq!(survey.letter(), Some("M"));
        assert_eq!(survey.fields.len(), 2);
        assert_eq!(survey.fields[1].name, "quiet");
        assert_eq!(survey.fields[1].state, RowState::Removed);
    }
}
