//! What the data chart reads out of the survey: the workspace's state, tiered.
//!
//! The code map asks where the code is written; this altitude asks what it
//! *keeps*. Its marks are the shapes state can take — structs, enums,
//! unions — and the statics that anchor state no type holds. Functions,
//! traits, consts and aliases have no block here: a signature names state, it
//! does not keep any, so naming is counted on the mark it names and read on
//! the sheet.
//!
//! The one organizing move is the tier. **Top-level data is a root**: a
//! static, or a type no other workspace type keeps in a field. Everything
//! else is secondary — state that lives inside other state — and the paper
//! says so by nesting: a held type is drawn *inside* the block of the type
//! that owns it hardest, the way a module frame is drawn inside its parent.
//! Plain ownership inside one module is therefore never a line; it is the
//! nesting itself. What cannot nest without lying stays a standing block with
//! its holding edges drawn: shared state (`Arc` has no single container),
//! state owned from another module (the coupling must stay visible ink), a
//! ring of mutual owners, and vocabulary types so widely held that seating
//! them under one holder would misread the other holders.
//!
//! A borrow is a view, not a hold: a type other types only `&`-reach is still
//! a root, with the borrow drawn as a line. Visibility never folds this
//! chart — private state is still state — so there is no doors toggle and no
//! `+ n private` row; the `pub` on a header is words, as everywhere.

use std::collections::{HashMap, HashSet};

use crate::Route;
use crate::api::{CodeGraph, Delta, GhostMark, HoldEvent, HoldKind, ItemKind, ItemMark, Vis};
use crate::views::codemap::RefDir;
use crate::views::codemap::model::Containment;
use crate::views::data::mark_route;

/// Structural holders a standing mark draws before folding them to a count on
/// its own foot. Past this the type is vocabulary: seating it under one holder
/// would misread the rest, and its fan-in drawn in full is a star burst.
const HELD_CAP: usize = 3;
/// Resting uses edges whose counts are engraved. Past this the labels are the
/// chart's texture instead of its data.
pub(crate) const TIE_LABELS: usize = 12;
/// Uses edges one mark rests in an anchored reading.
const TIES_PER_MARK: usize = 2;

// ---------------------------------------------------------------------------
// The chart's vocabulary: where an edge can land, what a frame is, and how a
// row is quoted. Pure descriptions of the paper, with no reading of the survey
// in them — the reading is `DataModel::build`, below.
// ---------------------------------------------------------------------------

/// Where an edge can land: a drawn mark, or the counted row a folded module
/// leaves behind. A reader can fold a whole module; the edge lands on the row
/// that counts it instead of being cut.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum Anchor {
    /// A type or static with a block of its own.
    Mark(u32),
    /// A whole module, folded by hand: the frame's own row, standing for every
    /// datum inside it and inside the modules nested in it.
    Mod(u32),
}

/// The modules the reviewer folded by hand, each named the way a fold has to
/// survive the next build: the crate, then the module path as rust nests it.
/// A frame id is an index into one build and says nothing across two.
pub(super) type Folds = HashSet<Vec<String>>;

/// A module frame's name in a [`Folds`] set: the crate first, then the module
/// path. The crate's own frame is the crate name alone.
fn mod_key(krate: &str, module: &[String]) -> Vec<String> {
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
            Anchor::Mod(frame) => Some(frame),
        }
    }
}

/// One seat in a frame's ownership forest: a block, and the blocks that sit
/// under it because it owns them. A folded module's counted row is a seat of
/// its own, with nothing under it — the fold is the whole reading.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Seat {
    pub(crate) anchor: Anchor,
    /// Seated one layer beneath, in the survey's order.
    pub(crate) children: Vec<Seat>,
}

impl Seat {
    /// A seat with nothing under it.
    pub(super) fn leaf(anchor: Anchor) -> Self {
        Self {
            anchor,
            children: Vec::new(),
        }
    }
}

/// One frame on the paper: a workspace crate, or one module inside a crate.
/// Module frames nest the way rust's modules do — `mod views` holds `mod data`
/// holds the state `views::data` declares — so the ground reads as the tree the
/// code is written in rather than as one flat row of the crate's first
/// segments.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Frame {
    pub(crate) id: u32,
    pub(crate) krate: String,
    /// The module path, segment by segment, as rust names it: `["views",
    /// "data"]` is `mod views::data`. Empty is the crate's own frame, which
    /// holds the types its crate root declares.
    pub(crate) module: Vec<String>,
    /// The frame this one sits inside: the module one segment up, or the crate
    /// frame for a top-level module. `None` only on a crate frame.
    pub(crate) parent: Option<u32>,
    /// Drawn marks seated here, in the survey's (file, source) order. The
    /// roster of what the frame draws; `forest` says where each one sits.
    pub(crate) marks: Vec<u32>,
    /// The reviewer folded this module by hand: it draws its border, its label
    /// and one row, and nothing inside it is on the paper. The modules nested
    /// in it earn no frame of their own — a fold is one boundary, not a stack
    /// of empty ones.
    pub(crate) folded: bool,
    /// What that row counts: every datum inside this module and inside the
    /// modules nested in it. Zero on an open frame.
    pub(crate) packed: u32,
    /// How they seat: the frame's ownership forest, in reading order —
    /// statics, then roots by how much state stands under them, then the
    /// vocabulary leaves. Every mark in `marks` sits somewhere in here exactly
    /// once, and a folded frame's own row is a seat of its own.
    pub(crate) forest: Vec<Seat>,
}

impl Frame {
    /// The label engraved on the frame's border, in rust's own words. A module
    /// frame wears its last segment alone — `mod data`, drawn inside `mod
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
    /// whole path as rust would write it in a `use` line (`views::data`), or
    /// the crate's own name where the frame is the crate's. The border's chip
    /// says `mod map` and three modules in this workspace answer to that, so
    /// a line the reader meets away from the chart spells the path out.
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
pub(super) enum RowState {
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

/// The workspace type one quoted row reaches: the name to draw in full ink,
/// and — where this chart draws that type's own block — where that block
/// stands, so the run can be its own link.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct Held {
    /// The held type's name — the one run of the declaration drawn in full ink,
    /// so `Vec<FileDetail>` reads as the wrapper it is around the type it
    /// holds. Empty where the row reaches nothing this workspace declares.
    pub(crate) name: String,
    /// The route that selects the block that run names, at this altitude.
    /// `Some` makes the run a link of its own — click the type's name inside
    /// a row and the chart goes to that type (2026-08-24, user), which is the
    /// same focus the block's own click is, so the camera glides only where
    /// the block is not already legible. `None` where there is no block to go
    /// to — a folded module's state — or where the run names the block it is
    /// written in, which is where the reader already stands.
    pub(crate) at: Option<Route>,
}

#[cfg(test)]
impl Held {
    /// A held name with no block to go to.
    pub(crate) fn named(name: &str) -> Self {
        Held {
            name: name.to_string(),
            at: None,
        }
    }
}

/// One quoted row of a block: a field or a variant, as the source writes it.
/// Nothing here is reconstructed.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct FieldRow {
    pub(crate) name: String,
    pub(crate) decl: String,
    /// What the row declares for itself, drawn in front of its name. A field
    /// can be narrower than the type holding it — a `pub(crate)` struct may
    /// publish some fields and keep the rest — and a reader deciding what may
    /// touch this state has to see which (2026-08-24, user). A variant declares
    /// nothing of its own: it is as visible as the enum it belongs to.
    pub(crate) vis: Vis,
    /// The workspace type this row reaches, and where its block stands.
    pub(crate) target: Held,
    /// The row against the diff base. A `Removed` row is the base's, seated
    /// where it stood.
    pub(crate) state: RowState,
}

/// What a block's head opens with and what closes it, so a quotation reads
/// the way rust writes it (2026-08-24, user): braces around a body, and
/// nothing at all around a declaration with no rows to bracket — a unit
/// struct, a static with no type the survey could read. The closer is a line
/// of its own, which is what makes a long block's end findable.
pub(super) fn brackets(kind: ItemKind, rows: usize) -> (&'static str, &'static str) {
    match kind {
        _ if rows == 0 => ("", ""),
        ItemKind::Struct | ItemKind::Union | ItemKind::Enum => ("{", "}"),
        // No body to bracket: the line under the name is the declared type,
        // and rust writes a colon in front of it — an alias, an equals.
        ItemKind::Static | ItemKind::Const => (":", ""),
        ItemKind::TypeAlias => ("=", ""),
        _ => ("", ""),
    }
}

impl FieldRow {
    /// The row as the block draws it: what it declares, its name, its type.
    /// One string, so measuring a row and drawing it can never disagree.
    pub(crate) fn written(&self) -> String {
        let body = match self.name.is_empty() {
            true => self.decl.clone(),
            false => format!("{}: {}", self.name, self.decl),
        };
        match self.vis.keyword() {
            Some(keyword) => format!("{keyword} {body}"),
            None => body,
        }
    }
}

/// Every anchor a shape change to `from` could reach, walking holds edges
/// holder-ward: the transitive holders, and the contracts that name them —
/// a signature has to change with the shape it quotes. `pairs` are (held,
/// holder). A counted fold row can join the set — the edge landing on it is
/// drawn — but the walk ends there: a row is a count, not a type with holders
/// of its own. So does a function: nothing holds one, so nothing is upstream
/// of it.
pub(super) fn upstream(pairs: &[(Anchor, Anchor)], from: Anchor) -> HashSet<Anchor> {
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

/// The part of a path below the crate's source root — `src/views/shell.rs`
/// becomes `views/shell.rs`, wherever in the workspace the crate itself sits.
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
/// reads them as. `src/views/data/map.rs` frames in `views::data`, and so does
/// `src/views/data/mod.rs`; `src/views/shell.rs` frames in `views` beside
/// them. A file directly under the root has no directory to name it, so
/// it frames as the module it is — `src/api.rs` is `mod api` — and the crate
/// root itself (`main.rs`, `lib.rs`) names no module at all and frames in the
/// crate.
///
/// A leaf file's own module is not a frame: the file altitude is two rungs
/// above, and a frame per file would draw the directory tree twice.
fn module_path(path: &str) -> Vec<&str> {
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

/// Where a drawn mark stands in the holding order — the chart's one verdict.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tier {
    /// Top-level data: a static, or a type no other type keeps in a field
    /// (`Owns` or `Shares`). State code reaches directly, where every chain
    /// of holding begins.
    Root,
    /// Secondary data, drawn inside the block of the mark that owns it
    /// hardest. The nesting is the ownership; no line restates it.
    Nested(u32),
    /// Held, but standing at module level with its holding edges drawn,
    /// because nesting it would lie — the reason says how.
    Standing(Stand),
}

/// Why a held type stands instead of nesting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stand {
    /// A shared handle holds it (`Arc`, `Rc`, a signal): sharing has no
    /// single container, so every holder keeps a drawn line.
    Shared,
    /// More than [`HELD_CAP`] types hold it: vocabulary. Its fan-in folds to
    /// `held by n types` on its own foot and inks back in on hover.
    Vocab,
    /// Its owners live in other modules; a type never leaves the frame that
    /// declares it, so the cross-frame ownership stays drawn ink.
    Afar,
    /// Mutual ownership: the seat that would close a loop stays a line.
    Ring,
}

/// The far end of a body reference this chart draws no block for: a free
/// function, a trait, a const, an alias. Undrawn is not unnameable — each is
/// a real item with a definition — so the sheet gives every one of them a row
/// and a link to its code (2026-08-23, user); only the paper keeps them to a
/// count.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Unseen {
    /// Its [`ItemMark`] id: the row's words, and where the link lands.
    pub(crate) item: u32,
    /// References across this pair, summed.
    pub(crate) count: u32,
}

/// One shape or static with a block on the paper.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct DataMark {
    pub(crate) id: u32,
    pub(crate) frame: u32,
    pub(crate) kind: ItemKind,
    pub(crate) vis: crate::api::Vis,
    pub(crate) name: String,
    /// The label its definition plate selects by, for the URL.
    pub(crate) label: String,
    pub(crate) path: String,
    pub(crate) line: u32,
    pub(crate) delta: Delta,
    /// The base had it, the working copy does not: drawn dashed from the base.
    pub(crate) ghost: bool,
    /// Fields quoted as written, every one of them — this chart's whole
    /// quotation. Methods are not rows here: a block is state only.
    pub(crate) fields: Vec<FieldRow>,
    /// An enum's variants as written, all of them.
    pub(crate) variants: Vec<FieldRow>,
    /// A static's declared type, as written.
    pub(crate) ty: String,
    /// The workspace type that type reaches, drawn in full ink — and where
    /// its own block stands, so the run is a link to it.
    pub(crate) ty_target: Held,
    pub(crate) tier: Tier,
    /// The marks nested inside this block, in the survey's order.
    pub(crate) kids: Vec<u32>,
    /// Distinct declarations whose own signature names it — free functions,
    /// method rows, consts, aliases, trait clauses. None of them has a block
    /// here, so the count is the ink the chart will not draw.
    pub(crate) named_by: u32,
    /// The bodies with no block here that reach it — function bodies,
    /// mostly. The other half of the same undrawn ink, kept as ends and not
    /// as a number, because the sheet lists every one of them; heaviest
    /// first. The paper says only how many.
    pub(crate) used_by: Vec<Unseen>,
    /// Where its own impls reach code the chart draws no mark for, the same
    /// way round. Said on the sheet, never on the paper.
    pub(crate) unseen_uses: Vec<Unseen>,
    /// Structural holders folded to a count: nonzero only on vocabulary
    /// marks, whose incoming holds rest folded.
    pub(crate) held_by: u32,
}

impl DataMark {
    pub(super) fn is_static(&self) -> bool {
        self.kind == ItemKind::Static
    }

    /// A root wears the gate's 2.5px ink left edge — the static's own mark,
    /// widened to every block a chain of holding begins at.
    pub(super) fn is_root(&self) -> bool {
        matches!(self.tier, Tier::Root)
    }

    /// Where it is written; a ghost's line is the base edition's.
    pub(crate) fn locator(&self) -> String {
        if self.ghost {
            format!("{}:{} (base)", self.path, self.line)
        } else {
            format!("{}:{}", self.path, self.line)
        }
    }

    /// The letter the mark wears, in git's own alphabet.
    pub(super) fn letter(&self) -> Option<&'static str> {
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

/// One drawn holding relation. The nesting already says plain same-module
/// ownership, so what is here is exactly the ink the paper cannot say:
/// sharing, borrowing, second holders, cross-module ownership, and the
/// diff's added and removed relations. Drawn held → holder; the arrowhead
/// rests on the holder, the way a shape change travels.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Hold {
    pub(crate) held: Anchor,
    pub(crate) holder: Anchor,
    pub(crate) kind: HoldKind,
    /// The strongest wrapper on the walk, in its own word.
    pub(crate) via: String,
    /// Rows drawing this edge.
    pub(crate) fields: u32,
    /// Drawn at rest. A folded edge inks in when either end is hovered.
    pub(crate) rest: bool,
    pub(crate) event: Option<HoldEvent>,
}

impl Hold {
    pub(crate) fn key(&self) -> String {
        format!(
            "{:?}>{:?}:{:?}:{}:{:?}",
            self.held, self.holder, self.kind, self.via, self.event
        )
    }
}

/// One implementation dependence between two drawn marks: one type's impls
/// lean on another type. The dashed family, drawn at every altitude the same
/// direction — the arrowhead rests on the user.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Tie {
    pub(crate) def: Anchor,
    pub(crate) user: Anchor,
    pub(crate) count: u32,
    /// Which of the def's methods the references name, heaviest first, for
    /// the sheet. The rows are not drawn here — methods are not this
    /// chart's — but which clause a body leans on is still the answer.
    pub(crate) rows: Vec<(String, u32)>,
    pub(crate) rest: bool,
    pub(crate) labeled: bool,
}

impl Tie {
    pub(crate) fn key(&self) -> String {
        format!("{:?}~{:?}", self.def, self.user)
    }
}

/// One undrawn naming: a declaration whose own signature names a drawn type.
/// The sheet's rows and the foot's `named by n signatures` both read this.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Naming {
    /// The named type's mark.
    pub(crate) ty: u32,
    /// The naming contract's mark in the survey — a free fn, const, alias,
    /// or, for a method row, the type whose API says the word.
    pub(crate) namer: u32,
    /// The namer is a method row of a type rather than a free contract.
    pub(crate) from_method: bool,
    pub(crate) event: Option<HoldEvent>,
}

/// What the cartouche states about the survey.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct DataFacts {
    pub(crate) structs: usize,
    pub(crate) enums: usize,
    pub(crate) unions: usize,
    pub(crate) statics: usize,
    pub(crate) added: usize,
    pub(crate) removed: usize,
    pub(crate) changed: usize,
    pub(crate) changed_modules: Vec<String>,
    pub(crate) unresolved: u32,
}

/// Everything one build of the data chart reads out of the survey.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct DataModel {
    pub(crate) frames: Vec<Frame>,
    /// Drawn marks, in the survey's (file, source) order.
    pub(crate) marks: Vec<DataMark>,
    /// The drawn holding edges — everything but the nesting.
    pub(crate) holds: Vec<Hold>,
    /// Every current structural relation as (held, holder), the nesting
    /// included: the blast radius walks all of it, drawn or seated.
    pub(crate) pairs: Vec<(Anchor, Anchor)>,
    pub(crate) ties: Vec<Tie>,
    /// The undrawn naming ink, for the sheet's rows.
    pub(crate) naming: Vec<Naming>,
    pub(crate) multi_crate: bool,
    // ---- Facts for the cartouche. ----
    pub(crate) structs: usize,
    pub(crate) enums: usize,
    pub(crate) unions: usize,
    pub(crate) statics: usize,
    pub(crate) roots: usize,
    pub(crate) nested: usize,
    pub(crate) standing: usize,
    pub(crate) added: usize,
    pub(crate) removed: usize,
    pub(crate) changed: usize,
    pub(crate) changed_modules: Vec<String>,
}

impl DataModel {
    /// The facts, lifted off the model for the furniture that states them.
    pub(crate) fn facts(&self, unresolved: u32) -> DataFacts {
        DataFacts {
            structs: self.structs,
            enums: self.enums,
            unions: self.unions,
            statics: self.statics,
            added: self.added,
            removed: self.removed,
            changed: self.changed,
            changed_modules: self.changed_modules.clone(),
            unresolved,
        }
    }
}

/// A mark this chart draws: the shapes state takes, and the statics that
/// anchor it. Everything else names state without keeping any.
fn data_kind(kind: ItemKind) -> bool {
    matches!(
        kind,
        ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Static
    )
}

/// Which frame a file's state belongs to: its crate, and the module path.
type FrameKey = (String, Vec<String>);

fn frame_key(krate: &str, path: &str) -> FrameKey {
    (
        krate.to_string(),
        module_path(path).into_iter().map(str::to_string).collect(),
    )
}

/// Whether nesting `child` under `candidate` would close a loop. Two types
/// that own each other cannot both contain the other: the first seat taken
/// stands, and the edge that would have closed the ring stays drawn.
fn would_ring(child: u32, candidate: u32, parents: &HashMap<u32, u32>) -> bool {
    let mut at = candidate;
    loop {
        if at == child {
            return true;
        }
        match parents.get(&at) {
            Some(&up) => at = up,
            None => return false,
        }
    }
}

/// How many blocks nest inside a mark, itself included: the weight of the
/// state it contains, for reading order inside a frame.
fn contained(id: u32, kids: &HashMap<u32, Vec<u32>>) -> usize {
    1 + kids.get(&id).map_or(0, |list| {
        list.iter().map(|&kid| contained(kid, kids)).sum::<usize>()
    })
}

impl DataModel {
    pub(crate) fn build(graph: &CodeGraph, ref_dir: RefDir, folds: &Folds) -> Self {
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

        // ---- Which marks are drawn. -----------------------------------------
        // Every shape and every static, whatever its visibility: state does
        // not fold at a door. Only a module the reviewer folded by hand takes
        // state off the paper, onto that boundary's one counted row.
        let file_key: Vec<FrameKey> = graph
            .files
            .iter()
            .map(|f| frame_key(&f.krate, &f.path))
            .collect();
        let key_of = |mark: u32| -> Option<&FrameKey> {
            file_key.get(graph.items[mark as usize].file as usize)
        };
        let fold_key = |key: &FrameKey| -> Option<FrameKey> {
            (0..=key.1.len())
                .map(|cut| (key.0.clone(), key.1[..cut].to_vec()))
                .find(|(krate, path)| folds.contains(&mod_key(krate, path)))
        };

        let mut drawn: Vec<u32> = Vec::new();
        let mut packed: Vec<u32> = Vec::new();
        for (i, mark) in graph.items.iter().enumerate() {
            if !data_kind(mark.kind) || mark.parent.is_some() {
                continue;
            }
            let i = i as u32;
            if key_of(i).and_then(&fold_key).is_some() {
                packed.push(i);
            } else {
                drawn.push(i);
            }
        }

        // ---- Frames: one per crate, then the module tree inside it. ---------
        let framed_key = |key: FrameKey| -> FrameKey { fold_key(&key).unwrap_or(key) };
        let data_ghosts: Vec<&GhostMark> =
            graph.ghosts.iter().filter(|g| data_kind(g.kind)).collect();
        let mut keys: Vec<FrameKey> = drawn
            .iter()
            .filter_map(|&m| key_of(m).cloned())
            .chain(packed.iter().filter_map(|&m| key_of(m).and_then(&fold_key)))
            .chain(data_ghosts.iter().map(|g| framed_key(ghost_key(g))))
            .collect();
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
        for krate in &crates {
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: krate.clone(),
                module: Vec::new(),
                parent: None,
                marks: Vec::new(),
                folded: folds.contains(&mod_key(krate, &[])),
                packed: 0,
                forest: Vec::new(),
            });
            frame_index.insert((krate.clone(), Vec::new()), id);
        }
        // Sorted, a path always follows the path it extends.
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
                folded: folds.contains(&mod_key(&key.0, &key.1)),
                packed: 0,
                forest: Vec::new(),
            });
            frame_index.insert(key.clone(), id);
        }
        let frame_of = |mark: u32| -> Option<u32> {
            key_of(mark).and_then(|key| frame_index.get(key).copied())
        };
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
        for &m in &packed {
            if let Some(frame) = key_of(m).and_then(&fold_frame) {
                frames[frame as usize].packed += 1;
                anchor_of[m as usize] = Some(Anchor::Mod(frame));
            }
        }
        for ghost in &data_ghosts {
            let key = ghost_key(ghost);
            if let Some(frame) = fold_frame(&key) {
                frames[frame as usize].packed += 1;
                anchor_of[ghost.id as usize] = Some(Anchor::Mod(frame));
            } else if let Some(&frame) = frame_index.get(&key) {
                frames[frame as usize].marks.push(ghost.id);
                anchor_of[ghost.id as usize] = Some(Anchor::Mark(ghost.id));
            }
        }
        let drawn_mark =
            |id: u32| matches!(anchor_of.get(id as usize), Some(Some(Anchor::Mark(_))));

        // ---- Reading the holds: structure to one side, naming to the other. -
        // A structural hold is a field of a shape or a static reaching a type:
        // that is state living inside state. Everything else that names a
        // drawn type — a free fn's signature, a method row, a const's declared
        // type, an alias, a trait clause — is naming: counted on the mark and
        // listed on the sheet, never drawn, because none of the namers has a
        // block here.
        let structural = |from: u32, from_method: bool| -> bool {
            !from_method && kind_of(from).is_some_and(data_kind)
        };
        // Current structural holders per drawn type, Owns/Shares only: a
        // borrow is a view, not a hold, so it never decides the tier.
        let mut holders: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut shared: HashSet<u32> = HashSet::new();
        let mut naming: Vec<Naming> = Vec::new();
        let mut named_set: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut named_seen: HashSet<(u32, u32)> = HashSet::new();
        for edge in &graph.holds {
            let (from, to) = (edge.from, edge.to);
            if !drawn_mark(to) || from == to {
                continue;
            }
            if structural(from, edge.from_method) {
                // A holder in a folded module still holds: the edge lands on
                // the boundary's row, and the held type must not read as a
                // root while a drawn line says otherwise.
                let placed = anchor_of.get(from as usize).is_some_and(Option::is_some);
                if edge.event == Some(HoldEvent::Removed) || !placed {
                    continue;
                }
                match edge.kind {
                    HoldKind::Owns | HoldKind::Shares => {
                        holders.entry(to).or_default().insert(from);
                        if edge.kind == HoldKind::Shares {
                            shared.insert(to);
                        }
                    }
                    _ => {}
                }
            } else {
                // A removed naming is the diff's to say, not a current namer.
                if edge.event != Some(HoldEvent::Removed) {
                    named_set.entry(to).or_default().insert(from);
                }
                if named_seen.insert((to, from)) {
                    naming.push(Naming {
                        ty: to,
                        namer: from,
                        from_method: edge.from_method,
                        event: edge.event,
                    });
                }
            }
        }

        // ---- The tier, and the nesting forest. -------------------------------
        // Greedy in the survey's order, cycle-checked, so the same survey
        // always nests the same chart. Vocabulary types neither nest nor
        // contain: seating one would drag half the frame under one block, and
        // holding kids under a folded fan-in would bury them.
        let vocab: HashSet<u32> = holders
            .iter()
            .filter(|(_, set)| set.len() > HELD_CAP)
            .map(|(&id, _)| id)
            .collect();
        let mut nest: HashMap<u32, u32> = HashMap::new();
        let mut ringed: HashSet<u32> = HashSet::new();
        for &id in &drawn {
            let mark = &graph.items[id as usize];
            if mark.kind == ItemKind::Static || vocab.contains(&id) {
                continue;
            }
            let Some(home) = frame_of(id) else { continue };
            // Same-frame plain owners, weighed by how many rows draw the
            // relation; the heaviest wins, the survey's order breaking ties.
            let mut weight: HashMap<u32, u32> = HashMap::new();
            for edge in &graph.holds {
                if edge.to != id
                    || edge.from == id
                    || edge.kind != HoldKind::Owns
                    || edge.from_method
                    || edge.event == Some(HoldEvent::Removed)
                    || !structural(edge.from, edge.from_method)
                    || !drawn_mark(edge.from)
                    || vocab.contains(&edge.from)
                    || frame_of(edge.from) != Some(home)
                {
                    continue;
                }
                *weight.entry(edge.from).or_default() += edge.fields.len() as u32;
            }
            let mut candidates: Vec<(u32, u32)> = weight.into_iter().collect();
            candidates.sort_by_key(|&(holder, fields)| (std::cmp::Reverse(fields), holder));
            let mut had_candidate = false;
            for (holder, _) in candidates {
                had_candidate = true;
                if !would_ring(id, holder, &nest) {
                    nest.insert(id, holder);
                    had_candidate = false;
                    break;
                }
            }
            if had_candidate {
                ringed.insert(id);
            }
        }
        let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
        for &id in &drawn {
            if let Some(&parent) = nest.get(&id) {
                kids.entry(parent).or_default().push(id);
            }
        }
        let tier_of = |id: u32| -> Tier {
            let mark = &graph.items[id as usize];
            if mark.kind == ItemKind::Static {
                return Tier::Root;
            }
            if let Some(&parent) = nest.get(&id) {
                return Tier::Nested(parent);
            }
            let held = holders.get(&id).map_or(0, |set| set.len());
            if held == 0 {
                Tier::Root
            } else if held > HELD_CAP {
                Tier::Standing(Stand::Vocab)
            } else if shared.contains(&id) {
                Tier::Standing(Stand::Shared)
            } else if ringed.contains(&id) {
                Tier::Standing(Stand::Ring)
            } else {
                Tier::Standing(Stand::Afar)
            }
        };

        // ---- The drawn holds: everything the nesting does not already say. --
        // Aggregated per (held, holder, kind, via, event) the way the wire
        // chart's, minus the one relation per nested mark the paper says
        // itself: its plain-owns edge from the block it is drawn inside.
        type HoldAgg = (Anchor, Anchor, HoldKind, String, Option<HoldEvent>);
        let mut acc: HashMap<HoldAgg, u32> = HashMap::new();
        let mut pairs: Vec<(Anchor, Anchor)> = Vec::new();
        for edge in &graph.holds {
            if !structural(edge.from, edge.from_method) {
                continue;
            }
            let holder = anchor_of.get(edge.from as usize).copied().flatten();
            let held = anchor_of.get(edge.to as usize).copied().flatten();
            let (Some(holder), Some(held)) = (holder, held) else {
                continue;
            };
            if holder == held {
                continue;
            }
            if edge.event != Some(HoldEvent::Removed) {
                pairs.push((held, holder));
            }
            // The nesting relation: plain ownership, drawn as containment.
            // An Added event elides too — the kid's own letter and the
            // holder's `+` row already say it, and a flare line from a block
            // to the block it is drawn inside would say it twice. A Removed
            // relation never nests, so the ghost ink always draws.
            if edge.kind == HoldKind::Owns
                && edge.via.is_empty()
                && edge.event != Some(HoldEvent::Removed)
                && nest.get(&edge.to) == Some(&edge.from)
            {
                continue;
            }
            *acc.entry((held, holder, edge.kind, edge.via.clone(), edge.event))
                .or_default() += edge.fields.len() as u32;
        }
        pairs.sort_unstable();
        pairs.dedup();
        let mut holds: Vec<Hold> = acc
            .into_iter()
            .map(|((held, holder, kind, via, event), fields)| Hold {
                held,
                holder,
                kind,
                via,
                fields,
                // A vocabulary mark's structural fan-in rests folded, its
                // count on its own foot; diff ink never folds.
                rest: event.is_some() || !matches!(held, Anchor::Mark(id) if vocab.contains(&id)),
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
        naming.sort_by_key(|n| (n.ty, n.namer));

        // ---- Reading order inside each frame. --------------------------------
        // Statics first — the chart's anchors — then roots by how much state
        // they contain, then the standing blocks, then a folded module's row.
        // Every seat is a leaf: the nesting happens inside the blocks, not in
        // the frame's shelves.
        for frame in &mut frames {
            if frame.folded {
                frame.forest = vec![Seat::leaf(Anchor::Mod(frame.id))];
                continue;
            }
            let mut top: Vec<u32> = frame
                .marks
                .iter()
                .copied()
                .filter(|m| ghost_of(*m).is_some() || !nest.contains_key(m))
                .collect();
            top.sort_by_key(|&m| {
                let tier = if ghost_of(m).is_some() {
                    2u8
                } else {
                    match tier_of(m) {
                        Tier::Root => 0,
                        Tier::Standing(_) => 1,
                        Tier::Nested(_) => 2,
                    }
                };
                (
                    tier,
                    kind_of(m) != Some(ItemKind::Static),
                    std::cmp::Reverse(contained(m, &kids)),
                    m,
                )
            });
            frame.forest = top
                .into_iter()
                .map(|m| Seat::leaf(Anchor::Mark(m)))
                .collect();
        }

        // ---- The marks themselves, rows quoted and diff-woven. ---------------
        let drawn_set: HashSet<u32> = drawn
            .iter()
            .copied()
            .chain(
                data_ghosts
                    .iter()
                    .filter(|g| fold_key(&ghost_key(g)).is_none())
                    .map(|g| g.id),
            )
            .collect();
        // Where a held type's block stands, by the key its URL uses, so the
        // bold run naming it is a link to it. A run the chart draws no block
        // for is bold text and nothing more, and so is a self-hold: the
        // reader is already standing in that block.
        let seat_of = |id: u32| -> Option<(String, String)> {
            match graph.items.get(id as usize) {
                Some(mark) => Some((
                    graph.files.get(mark.file as usize)?.path.clone(),
                    mark.label.clone(),
                )),
                None => ghost_of(id).map(|g| (g.path.clone(), g.name.clone())),
            }
        };
        let mut target_of: HashMap<(u32, String), Held> = HashMap::new();
        for edge in &graph.holds {
            if !drawn_set.contains(&edge.from) {
                continue;
            }
            let target = Held {
                name: name_of(edge.to),
                at: (drawn_set.contains(&edge.to) && edge.to != edge.from)
                    .then(|| seat_of(edge.to))
                    .flatten()
                    .map(|(path, label)| mark_route(&path, &label)),
            };
            for (name, _) in &edge.fields {
                target_of
                    .entry((edge.from, name.clone()))
                    .or_insert_with(|| target.clone());
            }
        }
        let target = |id: u32, name: &str| -> Held {
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
        let weave = |rows: &mut Vec<FieldRow>, removed: &mut Vec<(usize, FieldRow)>| {
            removed.sort_by_key(|(before, _)| *before);
            for (before, row) in removed.drain(..).rev() {
                let at = before.min(rows.len());
                rows.insert(at, row);
            }
        };
        let field_rows = |id: u32, mark: &ItemMark| -> Vec<FieldRow> {
            let mut fields: Vec<FieldRow> = mark
                .field_rows
                .iter()
                .enumerate()
                .map(|(at, row)| FieldRow {
                    name: row.name.clone(),
                    decl: row.ty.clone(),
                    vis: row.vis,
                    target: target(id, &row.name),
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
                .map(|(before, row)| {
                    (
                        *before as usize,
                        FieldRow {
                            name: row.name.clone(),
                            decl: row.ty.clone(),
                            vis: row.vis,
                            target: target(id, &row.name),
                            state: RowState::Removed,
                        },
                    )
                })
                .collect();
            weave(&mut fields, &mut dropped);
            fields
        };
        let variant_rows = |id: u32, mark: &ItemMark| -> Vec<FieldRow> {
            let mut variants: Vec<FieldRow> = mark
                .variants
                .iter()
                .enumerate()
                .map(|(at, written)| FieldRow {
                    name: String::new(),
                    decl: written.clone(),
                    vis: Vis::Private,
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
                            vis: Vis::Private,
                            target: target(id, &vname(written)),
                            state: RowState::Removed,
                        },
                    )
                })
                .collect();
            weave(&mut variants, &mut dropped);
            variants
        };

        let mut marks: Vec<DataMark> = drawn
            .iter()
            .filter_map(|&id| {
                let mark = &graph.items[id as usize];
                let frame = frame_of(id)?;
                let file = graph.files.get(mark.file as usize)?;
                let tier = tier_of(id);
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
                    fields: field_rows(id, mark),
                    variants: variant_rows(id, mark),
                    ty: mark.ty.clone(),
                    ty_target: target(id, &mark.name),
                    tier,
                    kids: kids.get(&id).cloned().unwrap_or_default(),
                    named_by: named_set.get(&id).map_or(0, |set| set.len() as u32),
                    // The ties pass fills these in.
                    used_by: Vec::new(),
                    unseen_uses: Vec::new(),
                    held_by: if vocab.contains(&id) {
                        holders.get(&id).map_or(0, |set| set.len() as u32)
                    } else {
                        0
                    },
                })
            })
            .collect();
        // Ghosts: whole blocks quoted from the base edition, standing in the
        // frame their path names. A removed type has no current holders, so
        // it stands whatever held it in the base — the removed edges say who.
        for ghost in &data_ghosts {
            let key = ghost_key(ghost);
            let (Some(&frame), None) = (frame_index.get(&key), fold_key(&key)) else {
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
                    .map(|row| FieldRow {
                        name: row.name.clone(),
                        decl: row.ty.clone(),
                        vis: row.vis,
                        target: target(ghost.id, &row.name),
                        state: RowState::Same,
                    })
                    .collect(),
                variants: ghost
                    .variants
                    .iter()
                    .map(|written| FieldRow {
                        name: String::new(),
                        decl: written.clone(),
                        vis: Vis::Private,
                        target: target(ghost.id, &vname(written)),
                        state: RowState::Same,
                    })
                    .collect(),
                ty: ghost.ty.clone(),
                ty_target: target(ghost.id, &ghost.name),
                tier: Tier::Standing(Stand::Afar),
                kids: Vec::new(),
                named_by: 0,
                used_by: Vec::new(),
                unseen_uses: Vec::new(),
                held_by: 0,
            });
        }

        // ---- The uses family, climbed to the type. ---------------------------
        // Every resolved reference, each end climbing its containment chain,
        // so a method's call is its type's. Both ends drawn keeps the pair;
        // a reference from code with no block here — a function's body, most
        // of the time — is exactly the "directly accessed" ink. It keeps the
        // item it came from, not just a tally: the sheet lists those bodies
        // by name and links each to its code, and only the paper's hover
        // words fold them to `used by n bodies` (2026-08-23, user).
        let containment = Containment::build(graph);
        let mut tie_acc: HashMap<(u32, u32), u32> = HashMap::new();
        let mut unseen_in: HashMap<u32, HashMap<u32, u32>> = HashMap::new();
        let mut unseen_out: HashMap<u32, HashMap<u32, u32>> = HashMap::new();
        let row_of: HashMap<u32, (u32, String)> = drawn
            .iter()
            .filter_map(|&id| Some((id, graph.items.get(id as usize)?)))
            .flat_map(|(id, mark)| {
                mark.method_rows
                    .iter()
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
                (false, true) => {
                    *unseen_in.entry(def).or_default().entry(user).or_default() += count;
                }
                (true, false) => {
                    *unseen_out.entry(user).or_default().entry(def).or_default() += count;
                }
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
        // Which of them rest, under the shared reading: each mark keeps its
        // heaviest few, the rest ink in on hover and for as long as either
        // end is selected — a fold by attention, never a cut.
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
        // Heaviest first, ties broken by name: the sheet reads these rows in
        // the order a reviewer would rank them.
        let unseen_ends = |acc: Option<&HashMap<u32, u32>>| -> Vec<Unseen> {
            let mut ends: Vec<Unseen> = acc
                .into_iter()
                .flatten()
                .map(|(&item, &count)| Unseen { item, count })
                .collect();
            ends.sort_by(|a, b| {
                b.count
                    .cmp(&a.count)
                    .then_with(|| name_of(a.item).cmp(&name_of(b.item)))
            });
            ends
        };
        for mark in &mut marks {
            mark.used_by = unseen_ends(unseen_in.get(&mark.id));
            mark.unseen_uses = unseen_ends(unseen_out.get(&mark.id));
        }
        // ---- Facts. -----------------------------------------------------------
        let current = |m: &&DataMark| !m.ghost;
        let structs = marks
            .iter()
            .filter(current)
            .filter(|m| m.kind == ItemKind::Struct)
            .count();
        let enums = marks
            .iter()
            .filter(current)
            .filter(|m| m.kind == ItemKind::Enum)
            .count();
        let unions = marks
            .iter()
            .filter(current)
            .filter(|m| m.kind == ItemKind::Union)
            .count();
        let statics = marks
            .iter()
            .filter(current)
            .filter(|m| m.kind == ItemKind::Static)
            .count();
        let roots = marks.iter().filter(current).filter(|m| m.is_root()).count();
        let nested = marks
            .iter()
            .filter(current)
            .filter(|m| matches!(m.tier, Tier::Nested(_)))
            .count();
        let standing = marks
            .iter()
            .filter(current)
            .filter(|m| matches!(m.tier, Tier::Standing(_)))
            .count();
        let added = marks.iter().filter(|m| m.delta == Delta::Added).count();
        let removed = marks.iter().filter(|m| m.ghost).count();
        let changed = marks.iter().filter(|m| m.delta == Delta::Changed).count();
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
            pairs,
            ties,
            naming,
            multi_crate,
            structs,
            enums,
            unions,
            statics,
            roots,
            nested,
            standing,
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

    /// The frame a file's marks land in is the directory chain under the
    /// crate's source root — the vocabulary this module inherited from the
    /// retired surface chart, and the one piece of it with a rule worth
    /// restating.
    #[test]
    fn a_module_path_is_the_directory_chain_under_src() {
        // The whole chain, as deep as the code is written: two modules here,
        // not one flat `views`.
        assert_eq!(module_path("src/views/data/map.rs"), ["views", "data"]);
        assert_eq!(
            module_path("src/views/codemap/map.rs"),
            ["views", "codemap"]
        );
        // A module's own file and a file beside it frame in that module itself.
        assert_eq!(module_path("src/views/mod.rs"), ["views"]);
        assert_eq!(module_path("src/views/shell.rs"), ["views"]);
        // A file under the source root is the module it declares.
        assert_eq!(module_path("src/api.rs"), ["api"]);
        assert!(module_path("src/main.rs").is_empty());
        assert!(module_path("crates/engine/src/lib.rs").is_empty());
        assert_eq!(module_path("crates/engine/src/parse/lex.rs"), ["parse"]);
    }
    use crate::api::{DeclRow, FileInfo, HoldEdge, MarkRef, Vis};

    fn file(id: u32, path: &str) -> FileInfo {
        FileInfo {
            id,
            path: path.to_string(),
            krate: "slope".to_string(),
            changed: false,
            lines: 100,
            items: 2,
            refs_in_files: 0,
        }
    }

    fn mark(id: u32, file: u32, name: &str, kind: ItemKind) -> ItemMark {
        ItemMark {
            id,
            file,
            local: id,
            name: name.to_string(),
            label: name.to_string(),
            kind,
            vis: Vis::Private,
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

    fn owns(from: u32, to: u32) -> HoldEdge {
        HoldEdge {
            from,
            to,
            kind: HoldKind::Owns,
            via: String::new(),
            fields: vec![("field".into(), "T".into())],
            from_method: false,
            event: None,
        }
    }

    fn graph(items: Vec<ItemMark>, holds: Vec<HoldEdge>) -> CodeGraph {
        CodeGraph {
            files: vec![file(0, "src/api.rs"), file(1, "src/views/dep/map.rs")],
            refs: Vec::new(),
            items,
            implements: Vec::new(),
            item_edges: Vec::new(),
            local_refs: Vec::new(),
            holds,
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
            walk_notes: Vec::new(),
        }
    }

    fn build(graph: &CodeGraph) -> DataModel {
        DataModel::build(graph, RefDir::default(), &Folds::new())
    }

    fn by_name<'a>(model: &'a DataModel, name: &str) -> &'a DataMark {
        model.marks.iter().find(|m| m.name == name).unwrap()
    }

    #[test]
    fn an_unheld_type_is_a_root_and_a_held_one_nests_in_its_holder() {
        let g = graph(
            vec![
                mark(0, 0, "Wire", ItemKind::Struct),
                mark(1, 0, "Nut", ItemKind::Struct),
            ],
            vec![owns(0, 1)],
        );
        let model = build(&g);
        assert_eq!(by_name(&model, "Wire").tier, Tier::Root);
        assert_eq!(by_name(&model, "Nut").tier, Tier::Nested(0));
        assert_eq!(by_name(&model, "Wire").kids, vec![1]);
        // The nesting is the ownership: no line restates it.
        assert!(model.holds.is_empty());
        // The blast radius still walks it.
        assert_eq!(model.pairs, vec![(Anchor::Mark(1), Anchor::Mark(0))]);
    }

    /// The run naming a held type carries the route to that type's block, so
    /// the run is a link: the reader who meets `field: Vec<Nut>` clicks `Nut`
    /// and the chart selects `Nut`, wherever on the paper it stands. A run
    /// naming the block it is written in carries none.
    #[test]
    fn a_held_run_carries_the_route_to_the_block_it_names() {
        let mut g = graph(
            vec![
                mark(0, 0, "Wire", ItemKind::Struct),
                mark(1, 1, "Nut", ItemKind::Struct),
            ],
            vec![owns(0, 1)],
        );
        g.items[0].field_rows = vec![DeclRow {
            name: "field".into(),
            ty: "Vec<Nut>".into(),
            vis: Vis::Private,
        }];
        let model = build(&g);
        let held = by_name(&model, "Wire").fields[0].target.clone();
        assert_eq!(held.name, "Nut");
        assert_eq!(held.at, Some(mark_route("src/views/dep/map.rs", "Nut")));

        // A shape that holds itself: the link would go where the reader is.
        let mut g = graph(vec![mark(0, 0, "Node", ItemKind::Struct)], vec![owns(0, 0)]);
        g.items[0].field_rows = vec![DeclRow {
            name: "field".into(),
            ty: "Option<Box<Node>>".into(),
            vis: Vis::Private,
        }];
        let model = build(&g);
        let held = by_name(&model, "Node").fields[0].target.clone();
        assert_eq!(held.name, "Node");
        assert_eq!(held.at, None);
    }

    #[test]
    fn only_functions_naming_a_type_leave_it_a_root_and_counted() {
        let mut g = graph(
            vec![
                mark(0, 0, "survey", ItemKind::Fn),
                mark(1, 0, "Wire", ItemKind::Struct),
            ],
            vec![owns(0, 1)],
        );
        g.items[0].kind = ItemKind::Fn;
        let model = build(&g);
        // The fn has no block; the struct is a root, its naming counted.
        assert_eq!(model.marks.len(), 1);
        let wire = by_name(&model, "Wire");
        assert_eq!(wire.tier, Tier::Root);
        assert_eq!(wire.named_by, 1);
        assert!(model.holds.is_empty());
    }

    #[test]
    fn shared_state_stands_beside_its_holders_with_the_edge_drawn() {
        let mut edge = owns(0, 1);
        edge.kind = HoldKind::Shares;
        edge.via = "Arc".into();
        let g = graph(
            vec![
                mark(0, 0, "App", ItemKind::Struct),
                mark(1, 0, "Config", ItemKind::Struct),
            ],
            vec![edge],
        );
        let model = build(&g);
        assert_eq!(
            by_name(&model, "Config").tier,
            Tier::Standing(Stand::Shared)
        );
        assert_eq!(model.holds.len(), 1);
        assert_eq!(model.holds[0].via, "Arc");
    }

    #[test]
    fn a_type_owned_only_from_another_module_stands_with_the_edge_drawn() {
        let g = graph(
            vec![
                mark(0, 1, "Atlas", ItemKind::Struct),
                mark(1, 0, "Wire", ItemKind::Struct),
            ],
            vec![owns(0, 1)],
        );
        let model = build(&g);
        // `Atlas` frames in `views`, `Wire` in `api`: ownership crosses the
        // boundary, so it stays drawn ink and the held type stands.
        assert_eq!(by_name(&model, "Wire").tier, Tier::Standing(Stand::Afar));
        assert_eq!(model.holds.len(), 1);
    }

    #[test]
    fn a_borrow_is_a_view_so_a_type_only_borrowed_is_still_a_root() {
        let mut edge = owns(0, 1);
        edge.kind = HoldKind::Borrows;
        edge.via = "&".into();
        let g = graph(
            vec![
                mark(0, 0, "Reader", ItemKind::Struct),
                mark(1, 0, "Wire", ItemKind::Struct),
            ],
            vec![edge],
        );
        let model = build(&g);
        assert_eq!(by_name(&model, "Wire").tier, Tier::Root);
        // The borrow is still drawn: a view is a line, not a container.
        assert_eq!(model.holds.len(), 1);
        assert_eq!(model.holds[0].via, "&");
    }

    #[test]
    fn mutual_owners_cannot_both_nest_and_the_ring_edge_stays_drawn() {
        let g = graph(
            vec![
                mark(0, 0, "Leaf", ItemKind::Struct),
                mark(1, 0, "Tree", ItemKind::Struct),
            ],
            vec![owns(0, 1), owns(1, 0)],
        );
        let model = build(&g);
        let nested = model
            .marks
            .iter()
            .filter(|m| matches!(m.tier, Tier::Nested(_)))
            .count();
        assert_eq!(nested, 1);
        assert_eq!(
            model
                .marks
                .iter()
                .filter(|m| m.tier == Tier::Standing(Stand::Ring))
                .count(),
            1
        );
        // One relation nests; the one that would close the ring is drawn.
        assert_eq!(model.holds.len(), 1);
    }

    #[test]
    fn a_widely_held_type_is_vocabulary_its_fan_in_folded_to_a_count() {
        let mut items = vec![mark(0, 0, "Placed", ItemKind::Struct)];
        let mut holds = Vec::new();
        for id in 1..=5u32 {
            items.push(mark(id, 0, &format!("H{id}"), ItemKind::Struct));
            holds.push(owns(id, 0));
        }
        let g = graph(items, holds);
        let model = build(&g);
        let placed = by_name(&model, "Placed");
        assert_eq!(placed.tier, Tier::Standing(Stand::Vocab));
        assert_eq!(placed.held_by, 5);
        // The edges exist but rest folded; hover and selection ink them in.
        assert_eq!(model.holds.len(), 5);
        assert!(model.holds.iter().all(|h| !h.rest));
    }

    #[test]
    fn a_static_is_a_root_and_can_contain_the_state_it_anchors() {
        let mut items = vec![
            mark(0, 0, "CACHE", ItemKind::Static),
            mark(1, 0, "Trail", ItemKind::Struct),
        ];
        items[0].ty = "Mutex<Trail>".into();
        let g = graph(items, vec![owns(0, 1)]);
        let model = build(&g);
        assert_eq!(by_name(&model, "CACHE").tier, Tier::Root);
        assert_eq!(by_name(&model, "Trail").tier, Tier::Nested(0));
        assert_eq!(by_name(&model, "CACHE").kids, vec![1]);
    }

    #[test]
    fn body_references_from_functions_are_named_not_drawn() {
        let mut g = graph(
            vec![
                mark(0, 0, "render", ItemKind::Fn),
                mark(1, 0, "Wire", ItemKind::Struct),
                mark(2, 0, "Nut", ItemKind::Struct),
            ],
            vec![],
        );
        g.local_refs = vec![
            MarkRef {
                from: 0,
                to: 1,
                count: 4,
            },
            MarkRef {
                from: 2,
                to: 1,
                count: 2,
            },
        ];
        let model = build(&g);
        let wire = by_name(&model, "Wire");
        // The fn keeps its own row on the mark; the type's draw a tie.
        assert_eq!(wire.used_by.len(), 1);
        assert_eq!((wire.used_by[0].item, wire.used_by[0].count), (0, 4));
        assert_eq!(model.ties.len(), 1);
        assert_eq!(model.ties[0].count, 2);
    }

    #[test]
    fn a_method_row_naming_a_type_is_naming_never_holding() {
        let mut edge = owns(0, 1);
        edge.from_method = true;
        let g = graph(
            vec![
                mark(0, 0, "Wire", ItemKind::Struct),
                mark(1, 0, "Nut", ItemKind::Struct),
            ],
            vec![edge],
        );
        let model = build(&g);
        let nut = by_name(&model, "Nut");
        assert_eq!(nut.tier, Tier::Root);
        assert_eq!(nut.named_by, 1);
        assert!(model.holds.is_empty());
    }

    #[test]
    fn statics_open_the_frame_then_roots_by_contained_state() {
        let g = graph(
            vec![
                mark(0, 0, "Small", ItemKind::Struct),
                mark(1, 0, "Big", ItemKind::Struct),
                mark(2, 0, "Kid", ItemKind::Struct),
                mark(3, 0, "STATE", ItemKind::Static),
            ],
            vec![owns(1, 2)],
        );
        let model = build(&g);
        let frame = &model.frames[model.marks[0].frame as usize];
        let order: Vec<Anchor> = frame.forest.iter().map(|s| s.anchor).collect();
        assert_eq!(
            order,
            vec![Anchor::Mark(3), Anchor::Mark(1), Anchor::Mark(0)]
        );
    }

    #[test]
    fn an_added_owns_relation_that_nests_draws_no_second_line() {
        let mut edge = owns(0, 1);
        edge.event = Some(HoldEvent::Added);
        let g = graph(
            vec![
                mark(0, 0, "Wire", ItemKind::Struct),
                mark(1, 0, "Nut", ItemKind::Struct),
            ],
            vec![edge],
        );
        let model = build(&g);
        // The kid's own letter and the holder's `+` row tell the diff; a
        // flare line into the block it is drawn inside would say it twice.
        assert_eq!(by_name(&model, "Nut").tier, Tier::Nested(0));
        assert!(model.holds.is_empty());
    }

    #[test]
    fn the_same_survey_always_builds_the_same_model() {
        let g = graph(
            vec![
                mark(0, 0, "Wire", ItemKind::Struct),
                mark(1, 0, "Nut", ItemKind::Struct),
                mark(2, 1, "Atlas", ItemKind::Struct),
            ],
            vec![owns(0, 1), owns(2, 1)],
        );
        let a = build(&g);
        let b = build(&g);
        assert_eq!(a, b);
    }
}
