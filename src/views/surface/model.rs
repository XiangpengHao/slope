//! The chart's reading of the survey: contracts as marks, dependence as edges.
//!
//! A block is a contract, and every row of it is a clause: a struct's fields,
//! an enum's variants, a trait's declared methods and associated items, a
//! function's parameters and return, the one type a static, const or alias
//! names — and, under a rule of its own, the methods a type publishes. A
//! method is never a mark of its own: it belongs to its type the way a field
//! does, and giving each one a block would bury the shapes under their own
//! API.
//!
//! Two inks run between the blocks, and never mix. **Solid** is interface
//! coupling: the dependent's own published surface names the other end, so a
//! change there forces a change here — including the one solid line no row
//! writes, `implements`, from a trait to the type that promised it.
//! **Dashed** is implementation coupling: the dependent's body leans on the
//! other end, a call or a name written inside a function, which a rewrite can
//! take back without anyone else noticing. Both run the same way round,
//! arrowhead on the dependent.
//!
//! Pure functions over the wire model — no layout and no rendering. The survey
//! ships everything it found, private items included; this module decides
//! which of them the chart draws as marks, which frame each one sits in, where
//! the rest fold to, and which anchor every edge lands on. Nothing is dropped
//! without a count: what the door folds goes to its frame's counted row and
//! the edges touching it land there, the way the code map's ties land on a
//! gate.
//!
//! Seating is decided here too, but not measured: each frame gets an ownership
//! forest, every type under its one heaviest same-frame `Owns` holder, and the
//! layout turns that into geometry. The same-frame rule is the whole point —
//! a type never leaves the module that declares it, so ownership reaching
//! across a module stays a drawn line instead of moving a block. A contract is
//! a leaf at both ends of that forest: nothing holds a function, a trait, a
//! const or an alias, and naming a type is not containment — so they seat
//! nothing and sit under nothing, falling in *beside* whatever they are most
//! about, because proximity should follow dependence.
//!
//! The uses family is computed here too. The survey records every reference it
//! resolved at item precision, inside a file and across; this altitude reads
//! them at mark precision, so each endpoint climbs its containment chain to
//! the block that draws it — a method's call is its type's — and a pair is
//! kept when both ends land on a drawn mark. What lands anywhere else is
//! counted on the mark it did reach, so a quiet contract can be read as quiet
//! rather than mistaken for dead.

use std::collections::{HashMap, HashSet};

use crate::api::{CodeGraph, Delta, GhostMark, HoldEvent, HoldKind, ItemKind, ItemMark, Vis};
use crate::views::codemap::model::Containment;
use crate::views::codemap::{Doors, RefDir};

/// Incoming holds edges a type draws before folding them to a count on its own
/// mark. A type four other types reach is a hub, and its fan-in drawn in full
/// is a star burst nobody can read.
const HELD_CAP: usize = 3;
// No row cap of any kind lives here any more (user decision, 2026-08-20): a
// block quotes every field, every variant, every method row and every
// parameter it has, always. A declaration read eight rows deep is a
// declaration half read, and a reader who has to select a block to see the
// rest of its shape is reading the chart twice. What a block still counts at
// its foot is the chart's own ink — a folded fan-in — never its own words.
/// Resting uses edges whose counts are engraved. Past this the labels are
/// the chart's texture instead of its data.
pub(crate) const TIE_LABELS: usize = 12;
/// Uses edges one mark rests in an anchored reading.
const TIES_PER_MARK: usize = 2;

/// Where an edge can land: a drawn mark, or one of a frame's counted fold rows.
/// Privacy folds a type for good and a reader can fold a whole module; either
/// way the edge lands on the row that counts it instead of being cut.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum Anchor {
    /// A type or static with a block of its own.
    Mark(u32),
    /// A frame's `+ n private items` row.
    Private(u32),
    /// A whole module, folded by hand: the frame's own row, standing for every
    /// contract inside it and inside the modules nested in it.
    Mod(u32),
}

/// The modules the reviewer folded by hand, each named the way a fold has to
/// survive the next build: the crate, then the module path as rust nests it.
/// A frame id is an index into one build and says nothing across two.
pub(crate) type Folds = HashSet<Vec<String>>;

/// A module frame's name in a [`Folds`] set: the crate first, then the module
/// path. The crate's own frame is the crate name alone.
pub(crate) fn mod_key(krate: &str, module: &[String]) -> Vec<String> {
    let mut key = vec![krate.to_string()];
    key.extend(module.iter().cloned());
    key
}

impl Anchor {
    /// The frame a counted row stands in. `None` on a mark, which stands for
    /// itself wherever it was seated.
    pub(crate) fn frame(self) -> Option<u32> {
        match self {
            Anchor::Mark(_) => None,
            Anchor::Private(frame) | Anchor::Mod(frame) => Some(frame),
        }
    }
}

/// One seat in a frame's ownership forest: a block, and the blocks that sit
/// under it because it owns them. A counted fold row can seat children too —
/// what only private code owns hangs under the row that counts the private
/// code, because that row is the only holder the chart draws.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Seat {
    pub(crate) anchor: Anchor,
    /// Seated one layer beneath, in the survey's order.
    pub(crate) children: Vec<Seat>,
}

impl Seat {
    /// A seat with nothing under it.
    pub(crate) fn leaf(anchor: Anchor) -> Self {
        Self {
            anchor,
            children: Vec::new(),
        }
    }
}

/// One frame on the paper: a workspace crate, or one module inside a crate.
/// Module frames nest the way rust's modules do — `mod views` holds `mod
/// surface` holds the contracts `views::surface` declares — so the ground reads
/// as the tree the code is written in rather than as one flat row of the
/// crate's first segments.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Frame {
    pub(crate) id: u32,
    pub(crate) krate: String,
    /// The module path, segment by segment, as rust names it: `["views",
    /// "surface"]` is `mod views::surface`. Empty is the crate's own frame,
    /// which holds the types its crate root declares.
    pub(crate) module: Vec<String>,
    /// The frame this one sits inside: the module one segment up, or the crate
    /// frame for a top-level module. `None` only on a crate frame.
    pub(crate) parent: Option<u32>,
    /// Drawn marks seated here, in the survey's (file, source) order. The
    /// roster of what the frame draws; `forest` says where each one sits.
    pub(crate) marks: Vec<u32>,
    /// Private types, never drawn, counted here.
    pub(crate) private: u32,
    /// The reviewer folded this module by hand: it draws its border, its label
    /// and one row, and nothing inside it is on the paper. The modules nested
    /// in it earn no frame of their own — a fold is one boundary, not a stack
    /// of empty ones.
    pub(crate) folded: bool,
    /// What that row counts: every contract inside this module and inside the
    /// modules nested in it, whatever door it stood at. Zero on an open frame.
    pub(crate) packed: u32,
    /// How they seat: the frame's ownership forest, in reading order —
    /// statics, then trees biggest first, then the free functions, then the
    /// vocabulary leaves, then the counted fold rows. Every mark in `marks`
    /// sits somewhere in here exactly once, and a fold row the frame counts is
    /// a seat of its own.
    pub(crate) forest: Vec<Seat>,
}

impl Frame {
    /// The label engraved on the frame's border, in rust's own words. A module
    /// frame wears its last segment alone — `mod surface`, drawn inside `mod
    /// views` — because that is how rust writes it in the file, and the paper's
    /// own nesting says the rest of the path. A crate frame names its crate
    /// only where the survey has more than one to tell apart; in a single-crate
    /// workspace that name is already the cartouche's.
    pub(crate) fn label(&self, multi_crate: bool) -> Option<String> {
        match self.module.last() {
            Some(segment) => Some(format!("mod {segment}")),
            None => multi_crate.then(|| self.krate.clone()),
        }
    }

    /// This frame's name in a [`Folds`] set, and in the URL that selects it.
    pub(crate) fn key(&self) -> Vec<String> {
        mod_key(&self.krate, &self.module)
    }

    /// The frame in prose, where no paper around it says which one it is: the
    /// whole path as rust would write it in a `use` line (`views::surface`), or
    /// the crate's own name where the frame is the crate's. The border's chip
    /// says `mod surface` and three modules in this workspace answer to that,
    /// so a line the reader meets away from the chart spells the path out.
    pub(crate) fn words(&self) -> String {
        match self.module.is_empty() {
            true => self.krate.clone(),
            false => self.module.join("::"),
        }
    }
}

/// One quoted row's own diff state, in the diff's own idiom: an added row
/// wears `+`, a dropped one is quoted from the base and struck.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum RowState {
    #[default]
    Same,
    Added,
    Removed,
}

impl RowState {
    /// The diff's own marker for the row.
    pub(crate) fn marker(self) -> Option<&'static str> {
        match self {
            RowState::Same => None,
            RowState::Added => Some("+"),
            RowState::Removed => Some("−"),
        }
    }

    /// The row's CSS class, empty for an untouched row.
    pub(crate) fn class(self) -> &'static str {
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
pub(crate) struct FieldRow {
    pub(crate) name: String,
    pub(crate) decl: String,
    /// The held type's name — the one run of the declaration drawn in full ink,
    /// so `Vec<FileDetail>` reads as the wrapper it is around the type it holds.
    pub(crate) target: String,
    /// The row against the diff base. A `Removed` row is the base's, seated
    /// where it stood.
    pub(crate) state: RowState,
}

/// One type, static, or free function with a block on the paper.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct SurfaceMark {
    pub(crate) id: u32,
    pub(crate) frame: u32,
    pub(crate) kind: ItemKind,
    pub(crate) vis: Vis,
    pub(crate) name: String,
    /// The label its definition plate selects by, for the URL.
    pub(crate) label: String,
    /// The defining file, relative to the workspace root.
    pub(crate) path: String,
    pub(crate) line: u32,
    /// How its own declaration differs from the diff base.
    pub(crate) delta: Delta,
    /// The base had it, the working copy does not: a ghost, drawn dashed from
    /// the base edition.
    pub(crate) ghost: bool,
    /// Fields — a function's parameters — quoted as written in declaration
    /// order, every one of them, and every one of them drawn.
    pub(crate) fields: Vec<FieldRow>,
    /// An enum's variants as written — payloads and discriminants included —
    /// quoted as rows (the row text in `decl`, `name` empty), all of them, and
    /// all of them drawn: a sum type is its variant list.
    pub(crate) variants: Vec<FieldRow>,
    /// The second band: the methods that clear the door, quoted as written
    /// signatures in the survey's order. The row text is in `decl` and the
    /// method's own name in `name`, which is what its edges and its callers
    /// are filed under. The band draws the whole list; only the door decides
    /// which rows are in it.
    pub(crate) methods: Vec<FieldRow>,
    /// A static's declared type or a function's return type, as written.
    pub(crate) ty: String,
    /// The workspace type that type reaches, if it reaches one — the run of
    /// `ty` drawn in full ink, as a field row's `target` is. Empty where the
    /// walk found nothing on this chart to hold, which is exactly when the
    /// line draws no holds edge: `GlobalSignal<Option<Viewport>>` names a
    /// type from a dependency, and a dependency has no mark to point at.
    pub(crate) ty_target: String,
    /// References into it the chart cannot draw a line for, summed: the ones
    /// leaving a mark the visibility setting or the budget folded, or an item
    /// with no mark of its own.
    /// The uses family draws everything else, so this is exactly the residue
    /// — and the difference between "nothing uses it" and "nothing the chart
    /// draws uses it", which a reviewer deciding whether code is dead must
    /// never have to guess at.
    pub(crate) unseen_users: u32,
    /// The same residue on the way out: references from its own body that
    /// land where the chart draws no mark.
    pub(crate) unseen_uses: u32,
    /// Incoming holds edges folded to a count: how many types hold this one.
    /// Zero when they are all drawn.
    pub(crate) held_by: u32,
    /// The other half of the same fold: how many signatures name this type.
    /// A function keeps nothing, so it is counted apart from the holders.
    pub(crate) named_by: u32,
}

impl SurfaceMark {
    /// A static is state no type holds — the chart's other kind of mark.
    pub(crate) fn is_static(&self) -> bool {
        self.kind == ItemKind::Static
    }

    /// A free function: a contract rather than a shape. Nothing holds it, its
    /// rows are its parameters, and its `ty` is what it hands back.
    pub(crate) fn is_fn(&self) -> bool {
        self.kind == ItemKind::Fn
    }

    /// Where it is written: `src/views/codemap/model.rs:278`. A ghost's line
    /// is the base edition's, and says so.
    pub(crate) fn locator(&self) -> String {
        if self.ghost {
            format!("{}:{} (base)", self.path, self.line)
        } else {
            format!("{}:{}", self.path, self.line)
        }
    }

    /// The letter the mark wears, in git's own alphabet: `A`dded since the
    /// base, `D` for a ghost, `M` for a rewritten declaration. `None` where
    /// the base wrote it exactly as it stands — whatever its file did.
    pub(crate) fn letter(&self) -> Option<&'static str> {
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
pub(crate) struct Hold {
    pub(crate) held: Anchor,
    pub(crate) holder: Anchor,
    pub(crate) kind: HoldKind,
    /// The strongest wrapper on the walk, in its own word. Empty for a plain
    /// hold, which needs none.
    pub(crate) via: String,
    /// Rows drawing this edge.
    pub(crate) fields: u32,
    /// The rows are the holder's *methods*: its API names the held mark
    /// rather than keeping one of it. Never true of a function mark, whose
    /// whole block is a signature already.
    pub(crate) from_method: bool,
    /// Drawn at rest. A folded edge stays in the set and inks in the moment the
    /// reader hovers either of its ends.
    pub(crate) rest: bool,
    /// The relation against the diff base. Diff ink never folds: an edge with
    /// an event always rests.
    pub(crate) event: Option<HoldEvent>,
}

impl Hold {
    pub(crate) fn key(&self) -> String {
        format!(
            "{:?}>{:?}:{:?}:{}:{}:{:?}",
            self.held, self.holder, self.kind, self.via, self.from_method, self.event
        )
    }
}

/// One drawn interface line under construction: where it lands, what wrapper
/// it met, whether a method row draws it, and its own diff event. Everything
/// that must not aggregate away is in the key.
type HoldKey = (Anchor, Anchor, HoldKind, String, bool, Option<HoldEvent>);

/// Every anchor a shape change to `from` could reach, walking holds edges
/// holder-ward: the transitive holders, and the contracts that name them —
/// a signature has to change with the shape it quotes. `pairs` are (held,
/// holder). A counted fold row can join the set — the edge landing on it is
/// drawn — but the walk ends there: a row is a count, not a type with holders
/// of its own. So does a function: nothing holds one, so nothing is upstream
/// of it.
pub(crate) fn upstream(pairs: &[(Anchor, Anchor)], from: Anchor) -> HashSet<Anchor> {
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

/// One implementation dependence between two drawn marks, summed: every
/// reference the user's body writes to the def, whatever file it was written
/// in. Drawn dashed, and the arrowhead rests on the user — the dependent — as
/// it does in every family at every altitude.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Tie {
    pub(crate) def: Anchor,
    pub(crate) user: Anchor,
    pub(crate) count: u32,
    /// Which of the def's drawn method rows this leans on, heaviest first —
    /// the part of the API being used, where the survey resolved the call to
    /// a method rather than to the block as a whole. Empty when it reaches
    /// the mark itself, or a row the door folded.
    pub(crate) rows: Vec<(String, u32)>,
    /// Drawn at rest under the current reading.
    pub(crate) rest: bool,
    /// Heavy enough among the resting ties to carry its count on the paper.
    pub(crate) labeled: bool,
}

impl Tie {
    pub(crate) fn key(&self) -> String {
        format!("{:?}~{:?}", self.def, self.user)
    }
}

/// Everything one build of the surface chart reads out of the survey.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct SurfaceModel {
    pub(crate) frames: Vec<Frame>,
    /// Drawn marks, in the survey's (file, source) order.
    pub(crate) marks: Vec<SurfaceMark>,
    pub(crate) holds: Vec<Hold>,
    pub(crate) ties: Vec<Tie>,
    /// More than one crate in the survey: crate frames earn their names.
    pub(crate) multi_crate: bool,
    /// The reading this model was built at, so the chart and the sheet word
    /// their visibility fold rows in the same breath the fold was decided.
    pub(crate) doors: Doors,
    // ---- Facts for the cartouche. ----
    pub(crate) structs: usize,
    pub(crate) enums: usize,
    /// Drawn free functions: the surface the chart reads as contracts.
    pub(crate) fns: usize,
    /// Drawn traits: contracts with nothing but clauses.
    pub(crate) traits: usize,
    /// Drawn consts and type aliases: contracts one line long.
    pub(crate) consts: usize,
    pub(crate) aliases: usize,
    /// Method rows drawn on type blocks — the rest of the published surface,
    /// which is not marks and would otherwise go uncounted.
    pub(crate) methods: usize,
    /// Drawn uses edges: how much of the workspace's coupling is one body
    /// leaning on another contract rather than a published surface naming it.
    pub(crate) uses: usize,
    /// Statics, plus every drawn type no other type holds. A function is not
    /// one: nothing can hold a function, so counting it would say nothing.
    pub(crate) roots: usize,
    /// The structural diff's counts over the drawn marks.
    pub(crate) added: usize,
    pub(crate) removed: usize,
    pub(crate) changed: usize,
    /// The modules holding a diff-touched contract, each named by its whole
    /// path (`views::surface`), in name order.
    pub(crate) changed_modules: Vec<String>,
}

/// What the cartouche and the legend state about the survey. Small enough to
/// hand the furniture without carrying the whole chart along with it.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct SurfaceFacts {
    pub(crate) structs: usize,
    pub(crate) enums: usize,
    pub(crate) fns: usize,
    pub(crate) traits: usize,
    pub(crate) consts: usize,
    pub(crate) aliases: usize,
    pub(crate) added: usize,
    pub(crate) removed: usize,
    pub(crate) changed: usize,
    pub(crate) changed_modules: Vec<String>,
    /// Names the survey could not resolve, straight from the wire model.
    pub(crate) unresolved: u32,
}

impl SurfaceModel {
    /// The facts, lifted off the model for the furniture that states them.
    pub(crate) fn facts(&self, unresolved: u32) -> SurfaceFacts {
        SurfaceFacts {
            structs: self.structs,
            enums: self.enums,
            fns: self.fns,
            traits: self.traits,
            consts: self.consts,
            aliases: self.aliases,
            added: self.added,
            removed: self.removed,
            changed: self.changed,
            changed_modules: self.changed_modules.clone(),
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
        // A trait is a contract with nothing but clauses: it clears the door
        // the same way a shape does.
        ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Trait => {
            doors.admits(mark.vis)
        }
        // A free function, const, or type alias is a contract of its own; the
        // same names inside a type or a trait are that block's rows.
        ItemKind::Fn | ItemKind::Const | ItemKind::TypeAlias => {
            free(mark) && doors.admits(mark.vis)
        }
        _ => false,
    }
}

/// A mark the chart could draw or fold: every contract the survey found.
fn charted(mark: &ItemMark) -> bool {
    match mark.kind {
        ItemKind::Static
        | ItemKind::Struct
        | ItemKind::Enum
        | ItemKind::Union
        | ItemKind::Trait => true,
        ItemKind::Fn | ItemKind::Const | ItemKind::TypeAlias => free(mark),
        _ => false,
    }
}

/// An item the file itself declares. A method, an associated const, an
/// associated type carries the block its impl or trait names as its parent,
/// and stays attributed to it: this altitude charts contracts, and theirs is
/// their owner's.
fn free(mark: &ItemMark) -> bool {
    mark.parent.is_none()
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

/// The module path a file's contracts are framed in, segment by segment: the
/// directories under the crate's source root, which is exactly the path rust
/// reads them as. `src/views/surface/map.rs` frames in `views::surface`, and so
/// does `src/views/surface/mod.rs`; `src/views/atlas.rs` frames in `views`
/// beside them. A file directly under the root has no directory to name it, so
/// it frames as the module it is — `src/api.rs` is `mod api` — and the crate
/// root itself (`main.rs`, `lib.rs`) names no module at all and frames in the
/// crate.
///
/// A leaf file's own module is not a frame: the file altitude is the rung above,
/// and a frame per file would draw the directory tree twice.
pub(crate) fn module_path(path: &str) -> Vec<&str> {
    let rest = source_rest(path);
    let mut dirs: Vec<&str> = rest.split('/').collect();
    let file = dirs.pop().unwrap_or_default();
    if dirs.is_empty() {
        let stem = file.strip_suffix(".rs").unwrap_or(file);
        if !matches!(stem, "main" | "lib" | "mod" | "build") {
            dirs.push(stem);
        }
    }
    dirs
}

/// Which frame a file's contracts belong to: its crate, and the module path
/// inside it.
type FrameKey = (String, Vec<String>);

/// A file's frame key, owned.
fn frame_key(krate: &str, path: &str) -> FrameKey {
    (
        krate.to_string(),
        module_path(path).into_iter().map(str::to_string).collect(),
    )
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

impl Seat {
    /// Grow one seat and everything seated under it.
    fn of(anchor: Anchor, seated: &HashMap<Anchor, Vec<u32>>) -> Self {
        Seat {
            anchor,
            children: seated.get(&anchor).map_or_else(Vec::new, |kids| {
                kids.iter()
                    .map(|&kid| Seat::of(Anchor::Mark(kid), seated))
                    .collect()
            }),
        }
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

impl SurfaceModel {
    pub(crate) fn build(graph: &CodeGraph, ref_dir: RefDir, doors: Doors, folds: &Folds) -> Self {
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
        let ghost_key = |g: &GhostMark| frame_key(&g.krate, &g.path);
        let is_fn = |id: u32| kind_of(id) == Some(ItemKind::Fn);
        // Everything that is a contract rather than a shape: never seated
        // under anything, never a seat, and placed beside what it names.
        let is_contract = |id: u32| {
            matches!(
                kind_of(id),
                Some(ItemKind::Fn | ItemKind::Trait | ItemKind::Const | ItemKind::TypeAlias)
            )
        };

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

        // Which method rows the reading draws. A method below the door is
        // implementation, not surface — its body's references still climb to
        // its type in the uses family, so nothing is lost by leaving it out of
        // the contract. A trait impl's method carries no `pub` and is
        // published all the same: it is callable wherever the trait is.
        let door = |row: &crate::api::MethodRow| {
            doors.admits(if row.via_trait { Vis::Pub } else { row.vis })
        };

        // Which frame every file's contracts belong to, and which of those
        // frames the reviewer folded. Read before anything is drawn: a folded
        // module's contracts are off the paper, so they never stand in the
        // budget's way either.
        let file_key: Vec<FrameKey> = graph
            .files
            .iter()
            .map(|f| frame_key(&f.krate, &f.path))
            .collect();
        let key_of = |mark: u32| -> Option<&FrameKey> {
            file_key.get(graph.items[mark as usize].file as usize)
        };
        // The fold a key sits in, if any: the key itself where the reviewer
        // folded this very module, or the outermost folded module above it.
        // Read from the crate down, so folding the module above a folded one
        // swallows it — a fold is one boundary, never a stack of them.
        let fold_key = |key: &FrameKey| -> Option<FrameKey> {
            (0..=key.1.len())
                .map(|cut| (key.0.clone(), key.1[..cut].to_vec()))
                .find(|(krate, path)| folds.contains(&mod_key(krate, path)))
        };
        let folds_away = |mark: u32| -> bool { key_of(mark).and_then(fold_key).is_some() };

        let mut drawn: Vec<u32> = Vec::new();
        let mut private: Vec<u32> = Vec::new();
        // Contracts inside a folded module: not drawn, and not counted at
        // their own frame's door either — the fold's one row counts them all.
        let mut packed: Vec<u32> = Vec::new();
        for (i, mark) in graph.items.iter().enumerate() {
            if !charted(mark) {
                continue;
            }
            let i = i as u32;
            if folds_away(i) {
                packed.push(i);
            } else if drawable(mark, doors) {
                drawn.push(i);
            } else {
                private.push(i);
            }
        }

        // Nothing folds by count here. A global budget fold hid marks by a
        // number nobody set, reflowed the chart when the threshold moved, and
        // left a row a URL could still point at; the folds that remain are the
        // ones a reader asks for — the visibility door, and a module folded by
        // hand (user decision, 2026-08-21).

        // ---- Frames: one per crate, then the module tree inside it. ---------
        // A folded module earns its own frame and its nested modules earn
        // none: everything inside the boundary the reviewer drew reports to
        // that boundary.
        let framed_key = |key: FrameKey| -> FrameKey { fold_key(&key).unwrap_or(key) };
        let mut keys: Vec<FrameKey> = drawn
            .iter()
            .chain(private.iter())
            .filter_map(|&m| key_of(m).cloned())
            .chain(packed.iter().filter_map(|&m| key_of(m).and_then(fold_key)))
            .chain(graph.ghosts.iter().map(|g| framed_key(ghost_key(g))))
            .collect();
        // Every module on the way down earns a frame, whether or not it
        // declares a contract of its own: `mod views::surface` has to be drawn
        // inside `mod views`, so the module between them is on the paper even
        // when every file it holds is a `mod` line.
        let ancestors: Vec<FrameKey> = keys
            .iter()
            .flat_map(|(krate, path)| {
                (1..path.len()).map(|cut| (krate.clone(), path[..cut].to_vec()))
            })
            .collect();
        keys.extend(ancestors);
        keys.sort();
        keys.dedup();
        let mut crates: Vec<String> = keys.iter().map(|(krate, _)| krate.clone()).collect();
        crates.dedup();

        let mut frames: Vec<Frame> = Vec::new();
        let mut frame_index: HashMap<FrameKey, u32> = HashMap::new();
        // Crate frames first, so a top-level module always has one to sit in.
        for krate in &crates {
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: krate.clone(),
                module: Vec::new(),
                parent: None,
                marks: Vec::new(),
                private: 0,
                folded: folds.contains(&mod_key(krate, &[])),
                packed: 0,
                forest: Vec::new(),
            });
            frame_index.insert((krate.clone(), Vec::new()), id);
        }
        // Sorted, a path always follows the path it extends, so the frame a
        // module nests in is built before the module itself.
        for key in &keys {
            if key.1.is_empty() {
                continue;
            }
            let up = (key.0.clone(), key.1[..key.1.len() - 1].to_vec());
            let parent = frame_index.get(&up).copied();
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: key.0.clone(),
                module: key.1.clone(),
                parent,
                marks: Vec::new(),
                private: 0,
                folded: folds.contains(&mod_key(&key.0, &key.1)),
                packed: 0,
                forest: Vec::new(),
            });
            frame_index.insert(key.clone(), id);
        }
        let frame_of = |mark: u32| -> Option<u32> {
            key_of(mark).and_then(|key| frame_index.get(key).copied())
        };

        // The frame a folded module's contracts report to: the boundary the
        // reviewer folded, wherever inside it they were written.
        let fold_frame = |key: &FrameKey| -> Option<u32> {
            fold_key(key).and_then(|fold| frame_index.get(&fold).copied())
        };

        let mut anchor_of: Vec<Option<Anchor>> = vec![None; graph.items.len() + graph.ghosts.len()];
        for &m in &drawn {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].marks.push(m);
                anchor_of[m as usize] = Some(Anchor::Mark(m));
            }
        }
        // A folded module's whole roster, drawn types and private ones alike,
        // lands on the one row its frame draws.
        for &m in &packed {
            if let Some(frame) = key_of(m).and_then(fold_frame) {
                frames[frame as usize].packed += 1;
                anchor_of[m as usize] = Some(Anchor::Mod(frame));
            }
        }
        // Ghosts are drawn wherever the paper has room for them: a removed
        // type is diff ink, and the budget never folds one. A module the
        // reviewer folded by hand does — the fold is a reading, and it holds
        // for everything inside the boundary.
        for ghost in &graph.ghosts {
            let key = ghost_key(ghost);
            if let Some(frame) = fold_frame(&key) {
                frames[frame as usize].packed += 1;
                anchor_of[ghost.id as usize] = Some(Anchor::Mod(frame));
            } else if let Some(&frame) = frame_index.get(&key) {
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

        // ---- The interface family: every edge, landed on an anchor. --------
        // A `dyn Trait` lands on the trait's own block now, and an
        // `impl Trait for Type` joins the same ink: both are one contract
        // naming another, and both point at the dependent.
        let mut acc: HashMap<HoldKey, u32> = HashMap::new();
        let implements = graph.implements.iter().map(|edge| {
            (
                edge.trait_mark,
                edge.ty,
                HoldKind::Implements,
                // The word rust writes for it, engraved on the line: no
                // wrapper stands between a type and a contract it promises.
                "implements".to_string(),
                false,
                edge.event,
                1u32,
            )
        });
        let walked = graph.holds.iter().map(|edge| {
            (
                edge.from,
                edge.to,
                edge.kind,
                edge.via.clone(),
                edge.from_method,
                edge.event,
                edge.fields.len() as u32,
            )
        });
        // An implements edge runs trait → type, which is already tail → head;
        // every other edge is written holder-ward and is turned here.
        for (from, to, kind, via, from_method, event, rows) in walked.chain(implements) {
            let (holder, held) = if kind == HoldKind::Implements {
                (
                    anchor_of.get(to as usize).copied().flatten(),
                    anchor_of.get(from as usize).copied().flatten(),
                )
            } else {
                (
                    anchor_of.get(from as usize).copied().flatten(),
                    anchor_of.get(to as usize).copied().flatten(),
                )
            };
            let (Some(holder), Some(held)) = (holder, held) else {
                continue;
            };
            // A type holding itself, or two folded types holding each other,
            // draws nothing: the fold row already counts them both.
            if holder == held {
                continue;
            }
            // An alias standing in front of a plain type has no wrapper to
            // name, and the line would go wordless where rust has a word for
            // exactly this. The walk keeps the wrapper where it met one.
            let via = match holder {
                Anchor::Mark(m) if via.is_empty() && kind_of(m) == Some(ItemKind::TypeAlias) => {
                    "aliases".to_string()
                }
                _ => via,
            };
            *acc.entry((held, holder, kind, via, from_method, event))
                .or_default() += rows;
        }
        let mut holds: Vec<Hold> = acc
            .into_iter()
            .map(
                |((held, holder, kind, via, from_method, event), fields)| Hold {
                    held,
                    holder,
                    kind,
                    via,
                    fields,
                    from_method,
                    rest: true,
                    event,
                },
            )
            .collect();
        let event_ord = |e: Option<HoldEvent>| match e {
            None => 0u8,
            Some(HoldEvent::Added) => 1,
            Some(HoldEvent::Removed) => 2,
        };
        holds.sort_by(|a, b| {
            (
                a.held,
                a.holder,
                a.kind as u8,
                &a.via,
                a.from_method,
                event_ord(a.event),
            )
                .cmp(&(
                    b.held,
                    b.holder,
                    b.kind as u8,
                    &b.via,
                    b.from_method,
                    event_ord(b.event),
                ))
        });

        // Who holds what. The arrowhead rests on the holder, so a type's fan-in
        // is the set of edges leaving it — and a type more than three drawn
        // types hold folds them all to a count on its own mark, where hovering
        // either end inks them back in. A removed edge is diff ink, not
        // structure: it neither counts toward the fold nor ever joins it.
        let mut fan_in: HashMap<Anchor, HashSet<Anchor>> = HashMap::new();
        // Which of those holders keep the thing, and which only name it in a
        // signature — a function's whole block, or one method row of a type.
        let mut names_it: HashMap<Anchor, HashSet<Anchor>> = HashMap::new();
        for hold in &holds {
            if hold.event == Some(HoldEvent::Removed) {
                continue;
            }
            // Implementing a trait is not the trait holding the type, and
            // not the type holding the trait: it is a promise, and it belongs
            // in neither count.
            if hold.kind == HoldKind::Implements {
                continue;
            }
            fan_in.entry(hold.held).or_default().insert(hold.holder);
            if hold.from_method || matches!(hold.holder, Anchor::Mark(m) if is_fn(m)) {
                names_it.entry(hold.held).or_default().insert(hold.holder);
            }
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
                let named = names_it.get(anchor).map_or(0, |set| {
                    holders.iter().filter(|h| set.contains(h)).count() as u32
                });
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
                ItemKind::Static
                    | ItemKind::Fn
                    | ItemKind::Trait
                    | ItemKind::Const
                    | ItemKind::TypeAlias
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
                // Naming a type says nothing about where the type lives, so
                // no contract seats anybody: not a method row that hands one
                // back, not a function's signature, not a const's declared
                // type, not an alias standing in front of it.
                if hold.from_method
                    || matches!(hold.holder, Anchor::Mark(holder) if is_contract(holder))
                {
                    continue;
                }
                let same_frame = match hold.holder {
                    Anchor::Mark(holder) => frame_of(holder) == Some(home),
                    // A frame's private fold row is the drawn stand-in for its
                    // private code, so it can seat what only private code
                    // owns. A folded module's row cannot: it stands in another
                    // frame entirely.
                    Anchor::Private(frame) => frame == home,
                    Anchor::Mod(_) => false,
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
        // Which top-level seat a mark ends up under, so a contract can be
        // placed beside the shape it names instead of in a band of its own.
        // `None` where the trail runs into a counted fold row, which stands
        // for code the chart does not draw and cannot seat a contract beside.
        let tree_of = |mark: u32| -> Option<u32> {
            let mut at = mark;
            // The seat parents are acyclic by construction; the bound is only
            // there so a future rule cannot hang the chart.
            for _ in 0..64 {
                match seat_parent.get(&at) {
                    Some(Anchor::Mark(up)) => at = *up,
                    Some(_) => return None,
                    None => return Some(at),
                }
            }
            None
        };
        // The mark a contract stands nearest: the same-frame mark it is most
        // about. For a function that is what its signature names hardest; for
        // a trait it is the type that implements it most, and its signature
        // targets after that. Proximity follows dependence — and a contract
        // about nothing in its own frame reads after the shapes, in the band.
        let signature_home = |fn_id: u32, frame: u32| -> Option<u32> {
            let mut weight: HashMap<u32, u32> = HashMap::new();
            let same_frame = |target: u32| {
                target != fn_id
                    && (target as usize) < graph.items.len()
                    && frame_of(target) == Some(frame)
            };
            for hold in &holds {
                // A removed edge is diff ink, not structure: it places
                // nothing, the way it seats nothing.
                if hold.event == Some(HoldEvent::Removed) {
                    continue;
                }
                // An implementor is what a trait is most about, and it stands
                // at the head of that edge rather than the tail.
                if hold.kind == HoldKind::Implements {
                    if hold.held != Anchor::Mark(fn_id) {
                        continue;
                    }
                    if let Anchor::Mark(target) = hold.holder
                        && same_frame(target)
                    {
                        *weight.entry(target).or_default() += 2;
                    }
                    continue;
                }
                if hold.holder != Anchor::Mark(fn_id) {
                    continue;
                }
                let Anchor::Mark(target) = hold.held else {
                    continue;
                };
                if hold.from_method || !same_frame(target) {
                    continue;
                }
                *weight.entry(target).or_default() += hold.fields;
            }
            // The heaviest by field count, the survey's own order breaking a
            // tie — the rule the ownership seating already follows.
            weight
                .into_iter()
                .max_by_key(|&(target, fields)| (fields, std::cmp::Reverse(target)))
                .map(|(target, _)| target)
                .and_then(tree_of)
        };
        for frame in &mut frames {
            // A folded module draws its boundary, its label, and one row that
            // counts what is inside it. Nothing else about its shape is on the
            // paper, so there is no forest to grow.
            if frame.folded {
                frame.forest = vec![Seat::leaf(Anchor::Mod(frame.id))];
                continue;
            }
            // Nothing holds a function, so a function is never vocabulary; the
            // bands stay disjoint and every mark seats exactly once.
            let vocabulary = |m: u32| folded_fan.contains_key(&Anchor::Mark(m)) && !is_contract(m);
            let mut roots: Vec<u32> = frame
                .marks
                .iter()
                .copied()
                .filter(|&m| !seat_parent.contains_key(&m) && !vocabulary(m) && !is_contract(m))
                .collect();
            roots.sort_by_key(|&m| {
                (
                    kind_of(m) != Some(ItemKind::Static),
                    std::cmp::Reverse(subtree_size(Anchor::Mark(m), &seated)),
                    m,
                )
            });
            // Each contract falls in beside the tree holding the shape its
            // signature names hardest; the rest keep the band at the end. A
            // function still seats nothing and sits under nothing — this is
            // reading order inside the frame, not parenthood.
            let mut beside: HashMap<u32, Vec<u32>> = HashMap::new();
            let mut band: Vec<u32> = Vec::new();
            for m in frame.marks.iter().copied().filter(|&m| is_contract(m)) {
                match signature_home(m, frame.id) {
                    Some(tree) if tree != m => beside.entry(tree).or_default().push(m),
                    _ => band.push(m),
                }
            }
            let mut forest: Vec<Seat> = Vec::new();
            let place = |forest: &mut Vec<Seat>, m: u32| {
                forest.push(Seat::of(Anchor::Mark(m), &seated));
                for &f in beside.get(&m).into_iter().flatten() {
                    forest.push(Seat::leaf(Anchor::Mark(f)));
                }
            };
            for &m in &roots {
                place(&mut forest, m);
            }
            // Then the vocabulary leaves, then the contracts about nothing
            // here, then the counted rows: what a frame holds back reads
            // last, under everything it draws in full.
            for m in frame.marks.iter().copied().filter(|&m| vocabulary(m)) {
                place(&mut forest, m);
            }
            forest.extend(band.into_iter().map(|m| Seat::leaf(Anchor::Mark(m))));
            if frame.private > 0 {
                forest.push(Seat::of(Anchor::Private(frame.id), &seated));
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
            .chain(
                graph
                    .ghosts
                    .iter()
                    .filter(|g| fold_key(&ghost_key(g)).is_none())
                    .map(|g| g.id),
            )
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

        let mut marks: Vec<SurfaceMark> = drawn
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
                // The second band: the methods that clear the door.
                let seat: Vec<usize> = mark
                    .method_rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| door(row))
                    .map(|(at, _)| at)
                    .collect();
                let mut methods: Vec<FieldRow> = seat
                    .iter()
                    .map(|&at| {
                        let row = &mark.method_rows[at];
                        FieldRow {
                            name: row.name.clone(),
                            decl: row.sig.clone(),
                            target: target(id, &row.name),
                            state: if mark.methods_added.contains(&(at as u32)) {
                                RowState::Added
                            } else {
                                RowState::Same
                            },
                        }
                    })
                    .collect();
                // A dropped method seats before the drawn row that took its
                // place: its recorded index is into the whole band, and the
                // door may have folded rows out from under it.
                let mut dropped: Vec<(usize, FieldRow)> = mark
                    .methods_removed
                    .iter()
                    .map(|(before, name, sig)| {
                        let at = seat.partition_point(|&row| row < *before as usize);
                        (
                            at,
                            FieldRow {
                                name: name.clone(),
                                decl: sig.clone(),
                                target: target(id, name),
                                state: RowState::Removed,
                            },
                        )
                    })
                    .collect();
                weave(&mut methods, &mut dropped);
                Some(SurfaceMark {
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
                    methods,
                    ty: mark.ty.clone(),
                    // A static's one edge is filed under the static's own
                    // name, the way a field's is under the field's — and so is
                    // a function's return type.
                    ty_target: target(id, &mark.name),
                    // The uses family fills these in once it knows which
                    // marks it could not land on.
                    unseen_users: 0,
                    unseen_uses: 0,
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
            let key = ghost_key(ghost);
            let (Some(&frame), None) = (frame_index.get(&key), fold_key(&key)) else {
                continue;
            };
            marks.push(SurfaceMark {
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
                // A ghost's band is the base's whole band: no door can fold
                // what is not there any more.
                methods: ghost
                    .method_rows
                    .iter()
                    .map(|(name, sig)| FieldRow {
                        name: name.clone(),
                        decl: sig.clone(),
                        target: target(ghost.id, name),
                        state: RowState::Same,
                    })
                    .collect(),
                ty: ghost.ty.clone(),
                ty_target: target(ghost.id, &ghost.name),
                // A ghost has no users to count either way: the survey read
                // the working copy, and the working copy no longer declares
                // it, so nothing in it can name it.
                unseen_users: 0,
                unseen_uses: 0,
                held_by: 0,
                named_by: 0,
            });
        }

        // ---- The uses family, at mark precision. ---------------------------
        // Implementation coupling: one mark's body leans on another. Every
        // resolved reference the survey placed at item precision counts —
        // across files and inside one — with each end climbing its containment
        // chain to the mark that draws it, so a method's call is its type's
        // and a free function's is its own. A pair is kept when both ends land
        // on a drawn mark; what lands anywhere else is counted, not cut.
        let containment = Containment::build(graph);
        let mut tie_acc: HashMap<(u32, u32), u32> = HashMap::new();
        let mut unseen_in: HashMap<u32, u32> = HashMap::new();
        let mut unseen_out: HashMap<u32, u32> = HashMap::new();
        // Which method row a reference landed on before it climbed. The
        // survey resolves a call to the method itself, and that is the part
        // of the API being leaned on: keeping it means the sheet can say
        // which clause, not merely which block. Only drawn rows — a call to a
        // method the door folded is the type's, and says nothing more.
        let row_of: HashMap<u32, (u32, String)> = drawn
            .iter()
            .filter_map(|&id| Some((id, graph.items.get(id as usize)?)))
            .flat_map(|(id, mark)| {
                mark.method_rows
                    .iter()
                    .filter(|row| door(row))
                    .map(move |row| (row.mark, (id, row.name.clone())))
            })
            .collect();
        let mut rows_acc: HashMap<(u32, u32), HashMap<String, u32>> = HashMap::new();
        let cross = graph
            .item_edges
            .iter()
            .map(|e| (e.from, e.to, e.count))
            .chain(
                graph
                    .local_refs
                    .iter()
                    .map(|r| (Some(r.from), Some(r.to), r.count)),
            );
        for (from, to, count) in cross {
            // A reference written at file scope — a `use` line — has no mark
            // to leave from. What it enables is counted where it is written,
            // so counting the import too would count one dependence twice.
            let (Some(from), Some(to)) = (from, to) else {
                continue;
            };
            let (user, def) = (containment.root(from), containment.root(to));
            if user == def {
                continue;
            }
            match (drawn_set.contains(&user), drawn_set.contains(&def)) {
                (true, true) => {
                    *tie_acc.entry((def, user)).or_default() += count;
                    // A call naming one row of the def's API files under it.
                    if let Some((owner, row)) = row_of.get(&to)
                        && *owner == def
                    {
                        *rows_acc
                            .entry((def, user))
                            .or_default()
                            .entry(row.clone())
                            .or_default() += count;
                    }
                }
                (false, true) => *unseen_in.entry(def).or_default() += count,
                (true, false) => *unseen_out.entry(user).or_default() += count,
                (false, false) => {}
            }
        }
        let mut ties: Vec<Tie> = tie_acc
            .into_iter()
            .map(|((def, user), count)| {
                let mut rows: Vec<(String, u32)> = rows_acc
                    .get(&(def, user))
                    .map(|rows| rows.iter().map(|(r, n)| (r.clone(), *n)).collect())
                    .unwrap_or_default();
                rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                Tie {
                    def: Anchor::Mark(def),
                    user: Anchor::Mark(user),
                    count,
                    rows,
                    rest: true,
                    labeled: false,
                }
            })
            .collect();
        ties.sort_by(|a, b| {
            (a.def, a.user)
                .cmp(&(b.def, b.user))
                .then(b.count.cmp(&a.count))
        });

        // Which of them rest on the paper. Direction alone cannot thin the
        // chart — every edge here is one mark's use and another's users — so
        // each reading anchors on the marks themselves and hands the rest
        // back on hover.
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
        // What the family could not land, kept on the mark it did reach. The
        // sheet says it out loud, because the difference between a mark
        // nothing uses and one whose users the doors folded is the whole
        // question a reviewer asks of a quiet contract.
        for mark in &mut marks {
            mark.unseen_users = unseen_in.get(&mark.id).copied().unwrap_or(0);
            mark.unseen_uses = unseen_out.get(&mark.id).copied().unwrap_or(0);
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
        let traits = marks
            .iter()
            .filter(|m| !m.ghost && m.kind == ItemKind::Trait)
            .count();
        let kinds = |want: ItemKind| marks.iter().filter(|m| !m.ghost && m.kind == want).count();
        let consts = kinds(ItemKind::Const);
        let aliases = kinds(ItemKind::TypeAlias);
        let uses = ties.len();
        let methods = marks
            .iter()
            .filter(|m| !m.ghost)
            .map(|m| m.methods.len())
            .sum();
        // A root is state nothing else holds: every static, and every type no
        // other type has a field of. A function is counted as a contract
        // instead — nothing can hold one, so "root" would be true of every
        // function and mean nothing.
        let roots = marks
            .iter()
            .filter(|m| {
                !m.ghost
                    && !matches!(
                        m.kind,
                        ItemKind::Fn | ItemKind::Trait | ItemKind::Const | ItemKind::TypeAlias
                    )
                    && !fan_in.contains_key(&Anchor::Mark(m.id))
            })
            .count();
        let added = marks.iter().filter(|m| m.delta == Delta::Added).count();
        let removed = marks.iter().filter(|m| m.ghost).count();
        let changed = marks.iter().filter(|m| m.delta == Delta::Changed).count();
        // The insight line reads away from the paper, so each module is named
        // by its whole path — `views::surface`, not the `mod surface` its
        // border wears.
        let mut changed_modules: Vec<String> = marks
            .iter()
            .filter(|m| m.letter().is_some())
            .map(|m| frames[m.frame as usize].words())
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
            traits,
            consts,
            aliases,
            methods,
            uses,
            roots,
            added,
            removed,
            changed,
            changed_modules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Delta, FileInfo, HoldEdge, HoldEvent, ItemEdge, MarkRef};

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
            method_rows: Vec::new(),
            methods_added: Vec::new(),
            methods_removed: Vec::new(),
        }
    }

    #[test]
    fn a_module_path_is_the_directory_chain_under_src() {
        // The whole chain, as deep as the code is written: two modules here,
        // not one flat `views`.
        assert_eq!(
            module_path("src/views/surface/map.rs"),
            ["views", "surface"]
        );
        assert_eq!(
            module_path("src/views/codemap/map.rs"),
            ["views", "codemap"]
        );
        // A module's own file and a file beside it frame in that module itself.
        assert_eq!(module_path("src/views/mod.rs"), ["views"]);
        assert_eq!(module_path("src/views/atlas.rs"), ["views"]);
        // A file under the source root is the module it declares.
        assert_eq!(module_path("src/api.rs"), ["api"]);
        assert!(module_path("src/main.rs").is_empty());
        assert!(module_path("crates/engine/src/lib.rs").is_empty());
        assert_eq!(module_path("crates/engine/src/parse/lex.rs"), ["parse"]);
    }

    /// One pub struct per file, named after the file it is declared in: enough
    /// to ask which frame a path lands in and nothing else.
    fn framing_graph(paths: &[&str]) -> CodeGraph {
        CodeGraph {
            files: paths
                .iter()
                .enumerate()
                .map(|(i, path)| file(i as u32, path, false))
                .collect(),
            refs: Vec::new(),
            items: paths
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    let stem = path.rsplit('/').next().unwrap_or(path);
                    let name = stem.strip_suffix(".rs").unwrap_or(stem).to_uppercase();
                    mark(i as u32, i as u32, &name, ItemKind::Struct, Vis::Pub)
                })
                .collect(),
            implements: Vec::new(),
            item_edges: Vec::new(),
            local_refs: Vec::new(),
            holds: Vec::new(),
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    /// The ground is the module tree, as deep as rust writes it: `mod surface`
    /// is drawn inside `mod views`, and a contract sits in the module that
    /// declares it rather than in that module's first segment.
    #[test]
    fn module_frames_nest_the_way_the_modules_do() {
        let model = SurfaceModel::build(
            &framing_graph(&[
                "src/main.rs",
                "src/views/mod.rs",
                "src/views/atlas.rs",
                "src/views/surface/map.rs",
                "src/views/codemap/map.rs",
            ]),
            RefDir::Uses,
            Doors::Crate,
            &Folds::new(),
        );
        // The crate, `views`, and the two modules inside it.
        assert_eq!(model.frames.len(), 4);
        let views = frame_named(&model, "views");
        let surface = frame_named(&model, "views::surface");
        let codemap = frame_named(&model, "views::codemap");
        assert_eq!(surface.parent, Some(views.id));
        assert_eq!(codemap.parent, Some(views.id));
        assert_eq!(views.parent, Some(model.frames[0].id));
        assert!(model.frames[0].module.is_empty());

        // The border wears rust's own word for the module, and the nesting
        // says the rest of the path; prose spells the path out.
        assert_eq!(surface.label(false).as_deref(), Some("mod surface"));
        assert_eq!(views.label(false).as_deref(), Some("mod views"));
        assert_eq!(surface.words(), "views::surface");
        assert_eq!(model.frames[0].label(false), None);

        // `views/mod.rs` and `views/atlas.rs` frame in `views` itself; the file
        // a module away frames a module away.
        let named = |frame: &Frame| -> Vec<String> {
            frame
                .marks
                .iter()
                .map(|&m| model.marks.iter().find(|k| k.id == m).unwrap().name.clone())
                .collect()
        };
        assert_eq!(named(views), vec!["MOD", "ATLAS"]);
        assert_eq!(named(surface), vec!["MAP"]);
        assert_eq!(named(codemap), vec!["MAP"]);
        assert_eq!(named(&model.frames[0]), vec!["MAIN"]);
    }

    /// Folding a module by hand takes the whole boundary off the paper and
    /// leaves one counted row: every contract inside it, the nested modules'
    /// contracts included, and every edge that touched one lands on the row.
    /// The nested modules earn no frame — a fold is one boundary, not a stack
    /// of empty ones.
    #[test]
    fn folding_a_module_counts_everything_inside_it_on_one_row() {
        let mut g = framing_graph(&[
            "src/main.rs",
            "src/views/mod.rs",
            "src/views/surface/map.rs",
            "src/views/codemap/map.rs",
        ]);
        // `MAIN`, in the crate frame, holds the type `views::surface` declares.
        g.holds.push(HoldEdge {
            from: 0,
            to: 2,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: vec![("map".into(), "MAP".into())],
            from_method: false,
            event: None,
        });
        // A private type inside the fold is counted there too, not at its own
        // module's door: the fold is the only boundary left to count at.
        g.items
            .push(mark(4, 3, "HIDDEN", ItemKind::Struct, Vis::Private));

        let open = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        assert_eq!(open.frames.len(), 4);
        assert_eq!(open.marks.len(), 4);

        let folds: Folds = [mod_key("slope", &["views".to_string()])]
            .into_iter()
            .collect();
        let shut = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &folds);

        // The crate frame and `views`. The two modules inside it are gone.
        assert_eq!(shut.frames.len(), 2);
        let views = frame_named(&shut, "views");
        assert!(views.folded);
        assert!(views.marks.is_empty());
        // `MOD`, both `MAP`s, and the private one.
        assert_eq!(views.packed, 4);
        assert_eq!(views.private, 0);
        assert_eq!(views.forest, vec![Seat::leaf(Anchor::Mod(views.id))]);
        // Only what is left outside the boundary is drawn.
        let drawn: Vec<&str> = shut.marks.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(drawn, vec!["MAIN"]);
        // The edge is not cut, it lands: the row stands for what it counts.
        let edge = shut.holds.iter().find(|h| h.holder == Anchor::Mark(0));
        assert_eq!(edge.map(|h| h.held), Some(Anchor::Mod(views.id)));
    }

    /// Folding the module above a folded one swallows it: the outermost fold is
    /// the boundary the reader drew, and a fold inside it has nothing left to
    /// say. Unfolding the outer one hands the inner fold back untouched.
    #[test]
    fn the_outermost_fold_is_the_one_the_chart_draws() {
        let g = framing_graph(&["src/main.rs", "src/views/surface/map.rs"]);
        let folds: Folds = [
            mod_key("slope", &["views".to_string()]),
            mod_key("slope", &["views".to_string(), "surface".to_string()]),
        ]
        .into_iter()
        .collect();
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &folds);
        assert_eq!(model.frames.len(), 2);
        let views = frame_named(&model, "views");
        assert!(views.folded);
        assert_eq!(views.packed, 1);
    }

    /// A module whose own files declare nothing still holds the modules under
    /// it: without its frame, `mod surface` would have nowhere to sit.
    #[test]
    fn a_module_between_two_frames_is_drawn_with_nothing_of_its_own() {
        let model = SurfaceModel::build(
            &framing_graph(&["src/views/surface/map.rs"]),
            RefDir::Uses,
            Doors::Crate,
            &Folds::new(),
        );
        assert_eq!(model.frames.len(), 3);
        let views = frame_named(&model, "views");
        assert!(views.marks.is_empty());
        assert_eq!(views.private, 0);
        assert_eq!(frame_named(&model, "views::surface").parent, Some(views.id));
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
            implements: Vec::new(),
            item_edges: vec![ItemEdge {
                from_file: 1,
                from: Some(1),
                to_file: 0,
                to: Some(0),
                count: 4,
            }],
            local_refs: Vec::new(),
            holds: vec![
                HoldEdge {
                    from: 1,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("wire".into(), "Wire".into())],
                    from_method: false,
                    event: None,
                },
                HoldEdge {
                    from: 2,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("wire".into(), "Wire".into())],
                    from_method: false,
                    event: None,
                },
                HoldEdge {
                    from: 3,
                    to: 1,
                    kind: HoldKind::Shares,
                    via: "Arc".into(),
                    fields: vec![("CACHE".into(), "OnceCell<Arc<Index>>".into())],
                    from_method: false,
                    event: None,
                },
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    #[test]
    fn privacy_folds_a_type_and_keeps_its_edge() {
        let model = SurfaceModel::build(&graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        // Two module frames under one crate frame; the crate frame is empty
        // but holds them.
        assert_eq!(model.frames.len(), 3);
        assert!(!model.multi_crate);
        let api = frame_named(&model, "api");
        assert_eq!(api.marks.len(), 1);
        let analyze = frame_named(&model, "analyze");
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
        let shut = SurfaceModel::build(&g, RefDir::Uses, Doors::Pub, &Folds::new());
        let analyze = frame_named(&shut, "analyze");
        assert_eq!(analyze.marks.len(), 1);
        assert_eq!(analyze.private, 2);
        assert!(!shut.marks.iter().any(|m| m.name == "Index"));
        // The static still holds `Index`, so that edge lands on the row.
        assert!(
            shut.holds
                .iter()
                .any(|h| h.held == Anchor::Private(analyze.id) && h.holder == Anchor::Mark(3))
        );
        assert_eq!(shut.doors.fold_word(), "internal item");

        // At `pub(crate)` it is drawn again, and only the private type folds.
        let open = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        let analyze = frame_named(&open, "analyze");
        assert_eq!(analyze.marks.len(), 2);
        assert_eq!(analyze.private, 1);
        assert_eq!(open.doors.fold_word(), "private item");

        // At `private` nothing folds for visibility: every charted type is a
        // mark, and no frame carries a counted row.
        let all = SurfaceModel::build(&g, RefDir::Uses, Doors::All, &Folds::new());
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
        let model = SurfaceModel::build(&graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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
            method_rows: Vec::new(),
        });
        g.holds.push(HoldEdge {
            from: 1,
            to: ghost_id,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: vec![("refs".into(), "Vec<FileRef>".into())],
            from_method: false,
            event: Some(HoldEvent::Removed),
        });
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
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

    /// A uses edge needs a block at both ends. `Hidden` is folded here, so the
    /// one drawn pair is `Index` reaching `Wire`.
    #[test]
    fn a_uses_edge_needs_a_drawn_mark_at_both_ends() {
        let model = SurfaceModel::build(&graph(), RefDir::Both, Doors::Crate, &Folds::new());
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
            from_method: false,
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
            implements: Vec::new(),
            item_edges: Vec::new(),
            local_refs: Vec::new(),
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
            walk_notes: Vec::new(),
        }
    }

    /// The frame one module path names, `views::surface` and all.
    fn frame_named<'a>(model: &'a SurfaceModel, module: &str) -> &'a Frame {
        model
            .frames
            .iter()
            .find(|f| !f.module.is_empty() && f.words() == module)
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
        let model =
            SurfaceModel::build(&seating_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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
        let model =
            SurfaceModel::build(&seating_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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
        let model =
            SurfaceModel::build(&seating_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let api = frame_named(&model, "api");
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert!(seats.contains(&(Anchor::Mark(6), 0)));
        assert!(!seats.iter().any(|&(a, d)| a == Anchor::Mark(6) && d > 0));
    }

    #[test]
    fn a_frame_seats_statics_then_trees_then_vocabulary_then_its_fold_rows() {
        let model =
            SurfaceModel::build(&seating_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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
        // Both fixtures: the one with only shapes to seat, and the one whose
        // contracts are placed beside them.
        for graph in [
            seating_graph(),
            contract_graph(),
            api_graph(),
            trait_graph(),
        ] {
            seats_once(&SurfaceModel::build(
                &graph,
                RefDir::Uses,
                Doors::Crate,
                &Folds::new(),
            ));
            seats_once(&SurfaceModel::build(
                &graph,
                RefDir::Uses,
                Doors::All,
                &Folds::new(),
            ));
        }
    }

    fn seats_once(model: &SurfaceModel) {
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
            assert_eq!(seated, roster, "frame {}", frame.words());
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
            implements: Vec::new(),
            item_edges: Vec::new(),
            local_refs: Vec::new(),
            holds: vec![holds(0, 1, &["b"]), holds(1, 0, &["a"])],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        };
        let model = SurfaceModel::build(&graph, RefDir::Uses, Doors::Crate, &Folds::new());
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
        let a = SurfaceModel::build(&graph, RefDir::Uses, Doors::Crate, &Folds::new());
        let b = SurfaceModel::build(&graph, RefDir::Uses, Doors::Crate, &Folds::new());
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
            from_method: false,
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
            files: vec![
                file(0, "src/api.rs", false),
                file(1, "src/analyze/code.rs", false),
            ],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub),
                mark(1, 0, "Nut", ItemKind::Struct, Vis::Pub),
                func(2, 0, "survey", Vis::Pub, &[("graph", "&Wire")], "Nut"),
                func(3, 0, "sweep", Vis::Private, &[], ""),
                method,
                cache,
                func(6, 1, "index", Vis::Pub, &[], ""),
            ],
            // A module away, `index` calls `survey` five times.
            implements: Vec::new(),
            item_edges: vec![ItemEdge {
                from_file: 1,
                from: Some(6),
                to_file: 0,
                to: Some(2),
                count: 5,
            }],
            // And inside `src/api.rs`: the method `Wire::id` calls `survey`,
            // `survey` calls the private `sweep`, and `sweep` names `Wire`.
            local_refs: vec![
                MarkRef {
                    from: 2,
                    to: 3,
                    count: 2,
                },
                MarkRef {
                    from: 3,
                    to: 0,
                    count: 1,
                },
                MarkRef {
                    from: 4,
                    to: 2,
                    count: 3,
                },
            ],
            holds: vec![
                sig(2, 0, HoldKind::Borrows, "&", &[("graph", "&Wire")]),
                // The return type's edge is filed under the function's own
                // name, the way a static's is under the static's.
                sig(2, 1, HoldKind::Owns, "", &[("survey", "Nut")]),
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    #[test]
    fn a_free_function_is_a_mark_and_the_door_folds_it_like_a_type() {
        let model =
            SurfaceModel::build(&contract_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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
        assert_eq!((model.structs, model.fns, model.roots), (2, 2, 1));

        // At `private` the door opens on the quiet function too.
        let all = SurfaceModel::build(&contract_graph(), RefDir::Uses, Doors::All, &Folds::new());
        assert!(all.marks.iter().any(|m| m.name == "sweep"));
        assert_eq!(all.fns, 3);
    }

    #[test]
    fn a_signature_quotes_its_parameters_and_carries_its_wrapper() {
        let model =
            SurfaceModel::build(&contract_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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

    /// The other ink: bodies. Every reference the survey resolved is a
    /// candidate, wherever it was written — the file a call sits in says
    /// nothing about whether one contract leans on another — and each end
    /// climbs to the mark that draws it, so a method's call is its type's.
    #[test]
    fn a_body_reference_is_drawn_whichever_file_it_was_written_in() {
        let model =
            SurfaceModel::build(&contract_graph(), RefDir::Both, Doors::Crate, &Folds::new());
        let tie = |def: u32, user: u32| {
            model
                .ties
                .iter()
                .find(|t| t.def == Anchor::Mark(def) && t.user == Anchor::Mark(user))
        };
        // Across a module: `index` calls `survey`, and the arrowhead rests on
        // the dependent, as it does in every family.
        assert_eq!(tie(2, 6).map(|t| t.count), Some(5));
        // Inside one file: `Wire::id` calls `survey`, and the method climbs to
        // the type whose block draws it.
        assert_eq!(tie(2, 0).map(|t| t.count), Some(3));
        // A function's body is on the chart now: `survey` reaches nothing
        // drawn, because the only thing it calls is folded — and that is
        // counted, not cut.
        assert!(tie(3, 2).is_none());
        assert_eq!(model.ties.len(), 2);
    }

    /// What the family cannot land, it counts. The difference between a mark
    /// nothing uses and one whose users the doors folded is the whole question
    /// a reviewer asks of a quiet contract, so the sheet must never have to
    /// guess at it.
    #[test]
    fn the_uses_family_counts_what_it_cannot_draw() {
        let model =
            SurfaceModel::build(&contract_graph(), RefDir::Both, Doors::Crate, &Folds::new());
        let of = |name: &str| {
            model
                .marks
                .iter()
                .find(|m| m.name == name)
                .map(|m| (m.unseen_users, m.unseen_uses))
                .unwrap()
        };
        // The private `sweep` names `Wire` once and is called twice by
        // `survey`: both ends of both references are real, and neither can be
        // drawn, because one end has no block.
        assert_eq!(of("Wire"), (1, 0));
        assert_eq!(of("survey"), (0, 2));
        // Nothing reaches `Nut` at all — the verdict a reviewer deletes on.
        assert_eq!(of("Nut"), (0, 0));

        // Open the door and the same two references are drawn instead of
        // counted: the residue is a fold, not a fact about the code.
        let all = SurfaceModel::build(&contract_graph(), RefDir::Both, Doors::All, &Folds::new());
        assert!(
            all.marks
                .iter()
                .all(|m| m.unseen_users == 0 && m.unseen_uses == 0)
        );
        assert_eq!(all.ties.len(), 4);
    }

    #[test]
    fn a_function_seats_nothing_and_sits_under_nothing() {
        let model =
            SurfaceModel::build(&contract_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
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
    fn a_frame_reads_statics_then_trees_each_with_its_contracts_then_vocabulary() {
        let mut g = contract_graph();
        // `Id` is reached by four marks — two types, a static, and one
        // signature — so it is vocabulary: never seated, never a seat.
        g.items.push(mark(7, 0, "Id", ItemKind::Struct, Vis::Pub));
        g.items[2].field_rows.push(("id".into(), "Id".into()));
        g.holds.push(holds(0, 7, &["id"]));
        g.holds.push(holds(1, 7, &["id"]));
        g.holds.push(sig(2, 7, HoldKind::Owns, "", &[("id", "Id")]));
        g.holds.push(holds(5, 7, &["CACHE"]));
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        let api = frame_named(&model, "api");
        assert_eq!(
            roots(api),
            vec![
                // The static register first,
                Anchor::Mark(5),
                // then the trees, in the survey's order at equal size — and
                // each contract falls in beside the shape it names hardest,
                // `survey` after `Wire`, which its parameter borrows.
                Anchor::Mark(0),
                Anchor::Mark(2),
                Anchor::Mark(1),
                // then the vocabulary leaf, then what the frame does not draw.
                Anchor::Mark(7),
                Anchor::Private(api.id),
            ]
        );
        let id = model.marks.iter().find(|m| m.name == "Id").unwrap();
        // The fold counts every arrow it folded, and says the two kinds of
        // arrow apart: a signature names a type without holding it.
        assert_eq!((id.held_by, id.named_by), (3, 1));
    }

    #[test]
    fn no_count_ever_folds_a_contract_away() {
        // The chart draws every contract that clears the door, however many
        // there are: what folds is what a reader folded, never a number.
        let mut items = vec![mark(0, 0, "CACHE", ItemKind::Static, Vis::Private)];
        for id in 1..=301u32 {
            items.push(func(id, 0, &format!("f{id}"), Vis::Pub, &[], ""));
        }
        let graph = CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items,
            implements: Vec::new(),
            item_edges: Vec::new(),
            local_refs: Vec::new(),
            holds: Vec::new(),
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        };
        let model = SurfaceModel::build(&graph, RefDir::Uses, Doors::Crate, &Folds::new());
        let api = frame_named(&model, "api");
        assert_eq!(api.marks.len(), 302);
        assert!(model.marks.iter().any(|m| m.name == "CACHE"));
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
            method_rows: Vec::new(),
        });
        g.holds.push(HoldEdge {
            from: ghost_id,
            to: 0,
            kind: HoldKind::Borrows,
            via: "&".into(),
            fields: vec![("wire".into(), "&Wire".into())],
            from_method: false,
            event: Some(HoldEvent::Removed),
        });
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());

        let ghost = model.marks.iter().find(|m| m.name == "sweep_all").unwrap();
        assert!(ghost.ghost && ghost.is_fn());
        assert_eq!(ghost.letter(), Some("D"));
        assert_eq!(ghost.fields[0].target, "Wire");
        // A ghost has no callers to count: the survey read the working copy,
        // and the working copy no longer declares it.
        assert_eq!((ghost.unseen_users, ghost.unseen_uses), (0, 0));
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

    // ---- The method band: a type's API as rows of its own block. -----------

    fn method(
        name: &str,
        sig: &str,
        vis: Vis,
        via_trait: bool,
        mark: u32,
    ) -> crate::api::MethodRow {
        crate::api::MethodRow {
            name: name.to_string(),
            sig: sig.to_string(),
            vis,
            via_trait,
            mark,
        }
    }

    /// A type wearing its API. `Wire` publishes `build`, keeps `hidden` to
    /// itself, reaches `inner` only inside its crate, and answers `read`
    /// through a trait — which carries no `pub` and is published all the same.
    /// `build` names `Nut` by value, and the free `survey` calls it four times.
    fn api_graph() -> CodeGraph {
        let mut wire = mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub);
        wire.field_rows = vec![("id".into(), "u32".into())];
        wire.method_rows = vec![
            method(
                "build",
                "pub fn build(nut: Nut) -> Wire",
                Vis::Pub,
                false,
                3,
            ),
            method("hidden", "fn hidden(&self) -> u32", Vis::Private, false, 4),
            method("read", "fn read(&self) -> u32", Vis::Private, true, 5),
            method(
                "inner",
                "pub(crate) fn inner(&self) -> u32",
                Vis::Crate,
                false,
                6,
            ),
        ];
        let mut methods: Vec<ItemMark> = ["build", "hidden", "read", "inner"]
            .iter()
            .enumerate()
            .map(|(at, name)| mark(3 + at as u32, 0, name, ItemKind::Fn, Vis::Pub))
            .collect();
        for m in &mut methods {
            m.parent = Some(0);
        }
        let mut items = vec![
            wire,
            mark(1, 0, "Nut", ItemKind::Struct, Vis::Pub),
            func(2, 0, "survey", Vis::Pub, &[], ""),
        ];
        items.append(&mut methods);
        CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items,
            implements: Vec::new(),
            item_edges: Vec::new(),
            // `survey` calls `Wire::build` — the survey resolves that to the
            // method itself, which is the row a reader wants named.
            local_refs: vec![MarkRef {
                from: 2,
                to: 3,
                count: 4,
            }],
            holds: vec![HoldEdge {
                from: 0,
                to: 1,
                kind: HoldKind::Owns,
                via: String::new(),
                fields: vec![("build".into(), "pub fn build(nut: Nut) -> Wire".into())],
                from_method: true,
                event: None,
            }],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    fn band(model: &SurfaceModel, name: &str) -> Vec<String> {
        model
            .marks
            .iter()
            .find(|m| m.name == name)
            .unwrap()
            .methods
            .iter()
            .map(|row| row.name.clone())
            .collect()
    }

    #[test]
    fn the_method_band_is_the_door_the_type_is_read_at() {
        // At `pub(crate)`: everything but the private helper. A trait impl's
        // method is published whatever it declares — it is callable wherever
        // the trait is — so it stands beside the `pub` ones.
        let open = SurfaceModel::build(&api_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        assert_eq!(band(&open, "Wire"), vec!["build", "read", "inner"]);
        // At `pub`: only what leaves the crate, the trait's answer included.
        let shut = SurfaceModel::build(&api_graph(), RefDir::Uses, Doors::Pub, &Folds::new());
        assert_eq!(band(&shut, "Wire"), vec!["build", "read"]);
        // At `private`: the whole band, in the survey's order.
        let all = SurfaceModel::build(&api_graph(), RefDir::Uses, Doors::All, &Folds::new());
        assert_eq!(band(&all, "Wire"), vec!["build", "hidden", "read", "inner"]);
        // A method is never a mark of its own, at any door.
        assert!(!all.marks.iter().any(|m| m.name == "build"));
        // The row quotes the signature as written, and the type it names is
        // the bold run of it.
        let row = &open
            .marks
            .iter()
            .find(|m| m.name == "Wire")
            .unwrap()
            .methods[0];
        assert_eq!(row.decl, "pub fn build(nut: Nut) -> Wire");
        assert_eq!(row.target, "Nut");
    }

    /// A method's signature edge is the type's, filed under the method's row —
    /// and it is not a field. The chart must never read "Wire holds a Nut"
    /// when all Wire does is hand one back.
    #[test]
    fn a_method_edge_is_the_types_api_and_not_a_field_of_it() {
        let model = SurfaceModel::build(&api_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let edge = model
            .holds
            .iter()
            .find(|h| h.held == Anchor::Mark(1) && h.holder == Anchor::Mark(0))
            .unwrap();
        assert!(edge.from_method);
        assert_eq!(edge.kind, HoldKind::Owns);
        // And it seats nothing: a signature is not containment, whether it is
        // a whole function's or one row of a type's.
        let api = frame_named(&model, "api");
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert!(seats.contains(&(Anchor::Mark(1), 0)));
        assert!(seats.iter().all(|&(_, depth)| depth == 0));
        // The fold counts it as a signature naming the type, never a holder.
        let nut = model.marks.iter().find(|m| m.name == "Nut").unwrap();
        assert_eq!((nut.held_by, nut.named_by), (0, 0));
    }

    #[test]
    fn a_call_to_a_method_lands_on_the_row_it_names() {
        let model = SurfaceModel::build(&api_graph(), RefDir::Both, Doors::Crate, &Folds::new());
        let tie = model
            .ties
            .iter()
            .find(|t| t.def == Anchor::Mark(0) && t.user == Anchor::Mark(2))
            .unwrap();
        assert_eq!(tie.count, 4);
        // The call climbed to the type, and the sheet can still say which
        // clause of the contract is leaned on.
        assert_eq!(tie.rows, vec![("build".to_string(), 4)]);

        // A call to a row the door folded is the type's, and says no more:
        // filing it under a row nobody can see would point at nothing.
        let mut g = api_graph();
        g.local_refs[0].to = 4; // `hidden`
        let folded = SurfaceModel::build(&g, RefDir::Both, Doors::Crate, &Folds::new());
        let tie = folded
            .ties
            .iter()
            .find(|t| t.def == Anchor::Mark(0) && t.user == Anchor::Mark(2))
            .unwrap();
        assert_eq!(tie.count, 4);
        assert!(tie.rows.is_empty());
    }

    #[test]
    fn the_band_weaves_the_diff_the_way_the_fields_do() {
        let mut g = api_graph();
        // The working copy added `inner` and dropped `pub fn drain(self)`,
        // which stood where `read` now does.
        g.items[0].delta = Delta::Changed;
        g.items[0].methods_added = vec![3];
        g.items[0].methods_removed = vec![(2, "drain".into(), "pub fn drain(self)".into())];
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        let wire = model.marks.iter().find(|m| m.name == "Wire").unwrap();
        assert_eq!(wire.letter(), Some("M"));
        // The door folded `hidden` out from under the recorded index, and the
        // struck row still seats where it stood — before `read`.
        let rows: Vec<(&str, RowState)> = wire
            .methods
            .iter()
            .map(|row| (row.name.as_str(), row.state))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("build", RowState::Same),
                ("drain", RowState::Removed),
                ("read", RowState::Same),
                ("inner", RowState::Added),
            ]
        );
    }

    #[test]
    fn a_ghost_type_quotes_the_band_the_base_wrote() {
        let mut g = api_graph();
        let ghost_id = g.items.len() as u32;
        g.ghosts.push(crate::api::GhostMark {
            id: ghost_id,
            path: "src/api.rs".into(),
            krate: "slope".into(),
            name: "Coil".into(),
            kind: ItemKind::Struct,
            vis: Vis::Pub,
            line: 12,
            field_rows: vec![("turns".into(), "u32".into())],
            variants: Vec::new(),
            ty: String::new(),
            method_rows: vec![("wind".into(), "pub fn wind(&self) -> Nut".into())],
        });
        g.holds.push(HoldEdge {
            from: ghost_id,
            to: 1,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: vec![("wind".into(), "pub fn wind(&self) -> Nut".into())],
            from_method: true,
            event: Some(HoldEvent::Removed),
        });
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        let ghost = model.marks.iter().find(|m| m.name == "Coil").unwrap();
        assert!(ghost.ghost);
        // A ghost's band is the base's whole band: no door can fold what is
        // not there any more, and the row still bolds what it named.
        assert_eq!(ghost.methods.len(), 1);
        assert_eq!(ghost.methods[0].decl, "pub fn wind(&self) -> Nut");
        assert_eq!(ghost.methods[0].target, "Nut");
        assert!(model.holds.iter().any(|h| h.held == Anchor::Mark(1)
            && h.holder == Anchor::Mark(ghost_id)
            && h.from_method
            && h.event == Some(HoldEvent::Removed)));
    }

    // ---- Traits: contracts with nothing but clauses. -----------------------

    fn implement(trait_mark: u32, ty: u32, event: Option<HoldEvent>) -> crate::api::ImplEdge {
        crate::api::ImplEdge {
            trait_mark,
            ty,
            header: String::new(),
            event,
        }
    }

    /// `Reads` is a pub trait declaring `read` and `CAP`; `Quiet` is private.
    /// `Wire` implements `Reads`, `Board` took it on this epoch, and `Board`
    /// keeps a `dyn Reads` while `Wire` keeps a `dyn Quiet`.
    fn trait_graph() -> CodeGraph {
        let mut reads = mark(1, 0, "Reads", ItemKind::Trait, Vis::Pub);
        reads.method_rows = vec![
            method("read", "fn read(&self) -> Wire", Vis::Pub, false, 4),
            method("CAP", "const CAP: usize", Vis::Pub, false, 5),
        ];
        let mut quiet = mark(2, 0, "Quiet", ItemKind::Trait, Vis::Private);
        quiet.method_rows = vec![method("hush", "fn hush(&self)", Vis::Private, false, 6)];
        let mut board = mark(3, 0, "Board", ItemKind::Struct, Vis::Pub);
        board.field_rows = vec![("reader".into(), "Box<dyn Reads>".into())];
        let mut wire = mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub);
        wire.field_rows = vec![("hush".into(), "Box<dyn Quiet>".into())];
        let mut clauses: Vec<ItemMark> = ["read", "CAP", "hush"]
            .iter()
            .enumerate()
            .map(|(at, name)| mark(4 + at as u32, 0, name, ItemKind::Fn, Vis::Pub))
            .collect();
        clauses[0].parent = Some(1);
        clauses[1].parent = Some(1);
        clauses[2].parent = Some(2);
        let mut items = vec![wire, reads, quiet, board];
        items.append(&mut clauses);
        CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items,
            implements: vec![
                implement(1, 0, None),
                // `Board` took the contract on this epoch: review gold.
                implement(1, 3, Some(HoldEvent::Added)),
                implement(2, 0, None),
            ],
            item_edges: Vec::new(),
            local_refs: Vec::new(),
            holds: vec![
                // What a trait's own row names is the trait's edge, filed
                // under the row.
                HoldEdge {
                    from: 1,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("read".into(), "fn read(&self) -> Wire".into())],
                    from_method: true,
                    event: None,
                },
                // Two `dyn` rows: one onto a drawn trait, one onto a folded.
                HoldEdge {
                    from: 3,
                    to: 1,
                    kind: HoldKind::Dyn,
                    via: "dyn".into(),
                    fields: vec![("reader".into(), "Box<dyn Reads>".into())],
                    from_method: false,
                    event: None,
                },
                HoldEdge {
                    from: 0,
                    to: 2,
                    kind: HoldKind::Dyn,
                    via: "dyn".into(),
                    fields: vec![("hush".into(), "Box<dyn Quiet>".into())],
                    from_method: false,
                    event: None,
                },
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    #[test]
    fn a_trait_is_a_mark_the_door_admits_and_a_band_of_clauses() {
        let model = SurfaceModel::build(&trait_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        assert_eq!(model.facts(0).traits, 1);
        let reads = model.marks.iter().find(|m| m.name == "Reads").unwrap();
        // Nearly all band: a trait is its clauses, methods and associated
        // items alike, quoted as written.
        assert!(reads.fields.is_empty());
        assert_eq!(
            reads
                .methods
                .iter()
                .map(|r| r.decl.as_str())
                .collect::<Vec<_>>(),
            vec!["fn read(&self) -> Wire", "const CAP: usize"]
        );
        // The row's own edge bolds what it names.
        assert_eq!(reads.methods[0].target, "Wire");
        // A trait is never a root: nothing can hold a contract.
        assert!(!model.marks.iter().any(|m| m.name == "Quiet"));
        let api = frame_named(&model, "api");
        assert_eq!(api.private, 1);
        // At `private` the quiet one is drawn beside it.
        let all = SurfaceModel::build(&trait_graph(), RefDir::Uses, Doors::All, &Folds::new());
        assert_eq!(all.facts(0).traits, 2);
    }

    #[test]
    fn a_trait_rows_edge_is_the_traits_and_files_under_the_row() {
        let model = SurfaceModel::build(&trait_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let edge = model
            .holds
            .iter()
            .find(|h| h.held == Anchor::Mark(0) && h.holder == Anchor::Mark(1))
            .unwrap();
        assert!(edge.from_method);
        // And it is a signature naming the type, never the trait holding it.
        let wire = model.marks.iter().find(|m| m.name == "Wire").unwrap();
        assert_eq!((wire.held_by, wire.named_by), (0, 0));
    }

    #[test]
    fn implementing_runs_from_the_contract_to_the_type_that_promised_it() {
        let model = SurfaceModel::build(&trait_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let edge = |trait_: Anchor, ty: Anchor| {
            model
                .holds
                .iter()
                .find(|h| h.kind == HoldKind::Implements && h.held == trait_ && h.holder == ty)
        };
        // Tail is the contract, head is the promise: a change to `Reads`
        // travels to everything that implements it.
        assert!(edge(Anchor::Mark(1), Anchor::Mark(0)).is_some());
        // An impl this epoch added is diff ink.
        assert_eq!(
            edge(Anchor::Mark(1), Anchor::Mark(3)).and_then(|h| h.event),
            Some(HoldEvent::Added)
        );
        // A folded trait's promise lands on its module's counted row, the way
        // every other edge touching folded code does.
        let api = frame_named(&model, "api");
        assert!(edge(Anchor::Private(api.id), Anchor::Mark(0)).is_some());
        // Promising a contract is not being held by one: `Wire` is still a
        // root, and the word for the line is rust's own.
        assert!(model.roots >= 1);
        assert_eq!(
            edge(Anchor::Mark(1), Anchor::Mark(0)).map(|h| h.via.as_str()),
            Some("implements")
        );
    }

    #[test]
    fn a_dyn_row_lands_on_the_trait_it_names() {
        let model = SurfaceModel::build(&trait_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let dyn_edge = |held: Anchor| {
            model
                .holds
                .iter()
                .find(|h| h.kind == HoldKind::Dyn && h.held == held)
        };
        // A drawn trait takes the edge on its own block.
        assert_eq!(
            dyn_edge(Anchor::Mark(1)).map(|h| h.holder),
            Some(Anchor::Mark(3))
        );
        // A folded one takes it on the counted row — it is not dropped, and
        // there is no honesty counter left to explain it away.
        let api = frame_named(&model, "api");
        assert_eq!(
            dyn_edge(Anchor::Private(api.id)).map(|h| h.holder),
            Some(Anchor::Mark(0))
        );
        // At `private` the same edge lands on the trait's own block.
        let all = SurfaceModel::build(&trait_graph(), RefDir::Uses, Doors::All, &Folds::new());
        assert!(
            all.holds
                .iter()
                .any(|h| h.kind == HoldKind::Dyn && h.held == Anchor::Mark(2))
        );
    }

    #[test]
    fn a_ghost_trait_quotes_the_band_the_base_wrote() {
        let mut g = trait_graph();
        let ghost_id = g.items.len() as u32;
        g.ghosts.push(crate::api::GhostMark {
            id: ghost_id,
            path: "src/api.rs".into(),
            krate: "slope".into(),
            name: "Winds".into(),
            kind: ItemKind::Trait,
            vis: Vis::Pub,
            line: 30,
            field_rows: Vec::new(),
            variants: Vec::new(),
            ty: String::new(),
            method_rows: vec![("wind".into(), "fn wind(&self) -> Wire".into())],
        });
        g.implements.push(crate::api::ImplEdge {
            trait_mark: ghost_id,
            ty: 0,
            header: "impl Winds for Wire".into(),
            event: Some(HoldEvent::Removed),
        });
        // Its band named `Wire`, the way the base diff re-draws a dropped
        // row's edge from the edition that had it.
        g.holds.push(HoldEdge {
            from: ghost_id,
            to: 0,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: vec![("wind".into(), "fn wind(&self) -> Wire".into())],
            from_method: true,
            event: Some(HoldEvent::Removed),
        });
        let model = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        let ghost = model.marks.iter().find(|m| m.name == "Winds").unwrap();
        assert!(ghost.ghost && ghost.kind == ItemKind::Trait);
        assert_eq!(ghost.letter(), Some("D"));
        assert_eq!(ghost.methods.len(), 1);
        assert_eq!(ghost.methods[0].target, "Wire");
        // The promise it took away is drawn from the base edition.
        assert!(model.holds.iter().any(|h| h.kind == HoldKind::Implements
            && h.held == Anchor::Mark(ghost_id)
            && h.holder == Anchor::Mark(0)
            && h.event == Some(HoldEvent::Removed)));
        // A trait seats like the other contracts: never under anything.
        let api = frame_named(&model, "api");
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert!(seats.contains(&(Anchor::Mark(ghost_id), 0)));
    }

    // ---- Contracts one line long: consts and type aliases. -----------------

    /// `CAP` and `Wire`'s alias are pub; `SEED` is private. The alias stands
    /// in front of `Wire`, and the const's declared type names it too.
    fn one_line_graph() -> CodeGraph {
        let mut cap = mark(1, 0, "CAP", ItemKind::Const, Vis::Pub);
        cap.ty = "Wire".to_string();
        let mut seed = mark(2, 0, "SEED", ItemKind::Const, Vis::Private);
        seed.ty = "u32".to_string();
        let mut alias = mark(3, 0, "Spool", ItemKind::TypeAlias, Vis::Pub);
        alias.ty = "Vec<Wire>".to_string();
        // An associated const of `Wire` is a row of its block, never a mark.
        let mut assoc = mark(4, 0, "LIMIT", ItemKind::Const, Vis::Pub);
        assoc.parent = Some(0);
        let mut wire = mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub);
        wire.method_rows = vec![method("LIMIT", "const LIMIT: usize", Vis::Pub, false, 4)];
        CodeGraph {
            files: vec![file(0, "src/api.rs", false)],
            refs: Vec::new(),
            items: vec![wire, cap, seed, alias, assoc],
            implements: Vec::new(),
            item_edges: Vec::new(),
            local_refs: Vec::new(),
            holds: vec![
                HoldEdge {
                    from: 1,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("CAP".into(), "Wire".into())],
                    from_method: false,
                    event: None,
                },
                HoldEdge {
                    from: 3,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("Spool".into(), "Vec<Wire>".into())],
                    from_method: false,
                    event: None,
                },
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    #[test]
    fn a_const_and_an_alias_are_contracts_one_line_long() {
        let model =
            SurfaceModel::build(&one_line_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let facts = model.facts(0);
        assert_eq!((facts.consts, facts.aliases), (1, 1));
        let cap = model.marks.iter().find(|m| m.name == "CAP").unwrap();
        // The line a static uses for its declared type is the line they use
        // for what they name, and it bolds the same way.
        assert_eq!((cap.ty.as_str(), cap.ty_target.as_str()), ("Wire", "Wire"));
        assert!(cap.fields.is_empty() && cap.methods.is_empty());
        // A private one folds behind the module's counted row.
        assert!(!model.marks.iter().any(|m| m.name == "SEED"));
        assert_eq!(frame_named(&model, "api").private, 1);
        // An associated const is its block's row, and never drawn twice.
        assert!(!model.marks.iter().any(|m| m.name == "LIMIT"));
        let wire = model.marks.iter().find(|m| m.name == "Wire").unwrap();
        assert_eq!(wire.methods.len(), 1);
        // Neither is a root by declaration — only a static is.
        assert_eq!(model.roots, 0);

        let all = SurfaceModel::build(&one_line_graph(), RefDir::Uses, Doors::All, &Folds::new());
        assert_eq!(all.facts(0).consts, 2);
    }

    #[test]
    fn an_alias_points_at_what_it_stands_in_front_of() {
        let model =
            SurfaceModel::build(&one_line_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let edge = model
            .holds
            .iter()
            .find(|h| h.held == Anchor::Mark(0) && h.holder == Anchor::Mark(3))
            .unwrap();
        // A change to the target travels to the alias: the arrowhead rests on
        // the alias, and the line carries rust's word for what it is.
        assert_eq!(edge.via, "aliases");
        assert!(!edge.from_method);
        // A wrapper the walk met still wins the word — the alias is not the
        // only thing the line has to say.
        let mut g = one_line_graph();
        g.holds[1].via = "Arc".to_string();
        let shared = SurfaceModel::build(&g, RefDir::Uses, Doors::Crate, &Folds::new());
        assert!(
            shared
                .holds
                .iter()
                .any(|h| h.holder == Anchor::Mark(3) && h.via == "Arc")
        );
    }

    #[test]
    fn a_one_line_contract_seats_beside_what_it_names() {
        let model =
            SurfaceModel::build(&one_line_graph(), RefDir::Uses, Doors::Crate, &Folds::new());
        let api = frame_named(&model, "api");
        // `Wire` first, then the two contracts about it, then the fold row.
        assert_eq!(
            roots(api),
            vec![
                Anchor::Mark(0),
                Anchor::Mark(1),
                Anchor::Mark(3),
                Anchor::Private(api.id),
            ]
        );
        let mut seats = Vec::new();
        walk(&api.forest, 0, &mut seats);
        assert!(seats.iter().all(|&(_, depth)| depth == 0));
    }
}
