//! What the data chart reads out of the survey: the workspace's state, tiered.
//!
//! The rung above asks which crates lean on which; this one asks what the
//! code *keeps*. Its marks are the shapes state can take — structs, enums,
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
//! a root, with the borrow drawn as a line.
//!
//! One reading narrows which blocks are on the paper at all: the visibility
//! floor (2026-08-25, user). It slides along the rungs rust writes — `pub`,
//! `pub(crate)`, `pub(super)`, all — so a reviewer auditing what a crate
//! publishes can read that surface alone, and widening is one move back. It
//! acts on blocks and never on rows: a block is a quotation of a declaration,
//! and a quotation with its private fields dropped would misquote it, so every
//! row of a drawn block stays, wearing its own `pub` as it always did. A type
//! whose every holder is off the reading does not become a root — nothing on
//! the paper holds it, but something in the workspace does — so it stands with
//! [`Stand::Narrower`] and the sheet says why.

use std::collections::{HashMap, HashSet};

use crate::Route;
use crate::graph::data::{
    CodeGraph, Delta, GhostMark, HoldEvent, HoldKind, ItemKind, ItemMark, MarkRef, Vis,
};
use crate::views::data::{DataReading, RefDir, mark_route};

/// Parent chains are shallow by construction (file → type → method); the
/// bound only keeps a malformed link from spinning.
const MAX_DEPTH: usize = 8;

/// Which item of the survey's containment tree (file → type → method) each
/// mark belongs to, at the outermost turn — the item the file itself holds.
/// A reference resolved to a method is the type's reference: this is what
/// climbs it there.
struct Containment {
    root: Vec<u32>,
}

impl Containment {
    fn build(graph: &CodeGraph) -> Self {
        let marks = &graph.items;
        let root = (0..marks.len() as u32)
            .map(|i| {
                let mut cur = i;
                for _ in 0..MAX_DEPTH {
                    match marks[cur as usize].parent {
                        Some(p) if (p as usize) < marks.len() && p != cur => cur = p,
                        _ => break,
                    }
                }
                cur
            })
            .collect();
        Self { root }
    }

    /// The item the file holds directly — a method's type, a type itself.
    fn root(&self, mark: u32) -> u32 {
        self.root.get(mark as usize).copied().unwrap_or(mark)
    }
}

/// Structural holders a standing mark draws before folding them to a count on
/// its own foot. Past this the type is vocabulary: seating it under one holder
/// would misread the rest, and its fan-in drawn in full is a star burst.
const HELD_CAP: usize = 3;
/// Resting uses edges whose counts are engraved. Past this the labels are the
/// chart's texture instead of its data.
pub(super) const TIE_LABELS: usize = 12;
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
pub(super) enum Anchor {
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

/// A module frame's name in a [`Folds`] set, and in the URL that selects it:
/// the crate first, then the module path. The crate's own frame is that name
/// alone — and the name is the cargo package (`slope-cli`), the same word the
/// dependency chart's star wears, which is what lets its focus panel descend
/// onto this frame.
fn mod_key(krate: &str, module: &[String]) -> Vec<String> {
    let mut key = vec![krate.to_string()];
    key.extend(module.iter().cloned());
    key
}

impl Anchor {
    /// The frame a counted row stands in. `None` on a mark, which stands for
    /// itself wherever it was seated.
    pub(super) fn frame(self) -> Option<u32> {
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
    pub(super) anchor: Anchor,
    /// Seated one layer beneath, in the survey's order.
    pub(super) children: Vec<Seat>,
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
    pub(super) id: u32,
    pub(super) krate: String,
    /// The module path, segment by segment, as rust names it: `["views",
    /// "data"]` is `mod views::data`. Empty is the crate's own frame, which
    /// holds the types its crate root declares.
    pub(super) module: Vec<String>,
    /// The frame this one sits inside: the module one segment up, or the crate
    /// frame for a top-level module. `None` only on a crate frame.
    pub(super) parent: Option<u32>,
    /// Drawn marks seated here, in the survey's (file, source) order. The
    /// roster of what the frame draws; `forest` says where each one sits.
    pub(super) marks: Vec<u32>,
    /// Whether the reviewer folded this module by hand, and what the row says
    /// if they did.
    pub(super) fold: Fold,
    /// How they seat: the frame's ownership forest, in reading order —
    /// statics, then roots by how much state stands under them, then the
    /// vocabulary leaves. Every mark in `marks` sits somewhere in here exactly
    /// once, and a folded frame's own row is a seat of its own.
    pub(super) forest: Vec<Seat>,
}

/// A module folded by hand: the frame draws its border, its label and one
/// row, and nothing inside it is on the paper. The modules nested in it earn
/// no frame of their own — a fold is one boundary, not a stack of empty ones.
///
/// The two travel together because they must agree: an open frame packs
/// nothing, and a folded one always has a row to write a count on.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(super) struct Fold {
    pub(super) folded: bool,
    /// What the folded row counts: every datum inside this module and inside
    /// the modules nested in it. Zero on an open frame.
    pub(super) packed: u32,
}

impl Frame {
    /// The label engraved on the frame's border, in rust's own words. A module
    /// frame wears its last segment alone — `mod data`, drawn inside `mod
    /// views` — because that is how rust writes it in the file, and the paper's
    /// own nesting says the rest of the path. A crate frame names its crate
    /// only where the survey has more than one to tell apart; in a single-crate
    /// workspace that name is already the cartouche's.
    pub(super) fn label(&self, multi_crate: bool) -> Option<String> {
        match self.module.last() {
            Some(segment) => Some(format!("mod {segment}")),
            None => multi_crate.then(|| self.krate.clone()),
        }
    }

    /// This frame's name in a [`Folds`] set, and in the URL that selects it.
    pub(super) fn key(&self) -> Vec<String> {
        mod_key(&self.krate, &self.module)
    }

    /// The frame in prose, where no paper around it says which one it is: the
    /// whole path as rust would write it in a `use` line (`views::data`), or
    /// the crate's own name where the frame is the crate's. The border's chip
    /// says `mod map` and more than one module in this workspace answers to
    /// that, so a line the reader meets away from the chart spells the path
    /// out.
    pub(super) fn words(&self) -> String {
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
    pub(super) fn marker(self) -> Option<&'static str> {
        match self {
            RowState::Same => None,
            RowState::Added => Some("+"),
            RowState::Removed => Some("−"),
        }
    }

    /// The row's CSS class, empty for an untouched row.
    pub(super) fn class(self) -> &'static str {
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
    pub(super) name: String,
    /// The route that selects the block that run names, at this altitude.
    /// `Some` makes the run a link of its own — click the type's name inside
    /// a row and the chart goes to that type (2026-08-24, user), which is the
    /// same focus the block's own click is, so the camera glides only where
    /// the block is not already legible. `None` where there is no block to go
    /// to — a folded module's state — or where the run names the block it is
    /// written in, which is where the reader already stands.
    pub(super) at: Option<Route>,
}

#[cfg(test)]
impl Held {
    /// A held name with no block to go to.
    pub(super) fn named(name: &str) -> Self {
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
    pub(super) name: String,
    pub(super) decl: String,
    /// What the row declares for itself, drawn in front of its name. A field
    /// can be narrower than the type holding it — a `pub(crate)` struct may
    /// publish some fields and keep the rest — and a reader deciding what may
    /// touch this state has to see which (2026-08-24, user). A variant declares
    /// nothing of its own: it is as visible as the enum it belongs to.
    pub(super) vis: Vis,
    /// The workspace type this row reaches, and where its block stands.
    pub(super) target: Held,
    /// The row against the diff base. A `Removed` row is the base's, seated
    /// where it stood.
    pub(super) state: RowState,
}

impl ItemKind {
    /// What a block's head opens with and what closes it, so a quotation reads
    /// the way rust writes it (2026-08-24, user): braces around a body, and
    /// nothing at all around a declaration with no rows to bracket — a unit
    /// struct, a static with no type the survey could read. The closer is a
    /// line of its own, which is what makes a long block's end findable.
    pub(super) fn brackets(self, rows: usize) -> (&'static str, &'static str) {
        match self {
            _ if rows == 0 => ("", ""),
            ItemKind::Struct | ItemKind::Union | ItemKind::Enum => ("{", "}"),
            // No body to bracket: the line under the name is the declared type,
            // and rust writes a colon in front of it — an alias, an equals.
            ItemKind::Static | ItemKind::Const => (":", ""),
            ItemKind::TypeAlias => ("=", ""),
            _ => ("", ""),
        }
    }
}

impl FieldRow {
    /// The row as the block draws it: what it declares, its name, its type.
    /// One string, so measuring a row and drawing it can never disagree.
    pub(super) fn written(&self) -> String {
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
/// it frames as the module it is — `src/load.rs` is `mod load` — and the crate
/// root itself (`main.rs`, `lib.rs`) names no module at all and frames in the
/// crate.
///
/// A leaf file's own module is not a frame: a frame per file would draw the
/// directory tree twice, and this chart is about what the code keeps, not
/// where it is filed.
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
    /// Every type that holds it is narrower than the visibility reading draws.
    /// There is no block on the paper to nest inside and no line to draw, so
    /// it stands — and it is not a root: the holding is real, the reading is
    /// what left it off.
    Narrower,
}

/// The far end of a body reference this chart draws no block for: a free
/// function, a trait, a const, an alias. Undrawn is not unnameable — each is
/// a real item with a definition — so the sheet gives every one of them a row
/// and a link to its code (2026-08-23, user); only the paper keeps them to a
/// count.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Unseen {
    /// Its [`ItemMark`] id: the row's words, and where the link lands.
    pub(super) item: u32,
    /// References across this pair, summed.
    pub(super) count: u32,
}

/// One shape or static with a block on the paper.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct DataMark {
    pub(super) id: u32,
    pub(super) frame: u32,
    /// What the block's head says, and where its source is.
    pub(super) head: MarkHead,
    /// The rows drawn inside it.
    pub(super) rows: MarkRows,
    /// Where the epoch left it.
    pub(super) state: MarkState,
    /// Where it seats among the other blocks.
    pub(super) seat: MarkSeat,
    /// The ink the chart will not draw, counted and listed.
    pub(super) undrawn: Undrawn,
}

/// What a block's head says, and where a reader goes to read the source.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct MarkHead {
    pub(super) kind: ItemKind,
    pub(super) vis: crate::graph::data::Vis,
    pub(super) name: String,
    /// The label its selection sheet selects by, for the URL.
    pub(super) label: String,
    pub(super) path: String,
    pub(super) line: u32,
}

/// The rows drawn inside a block. Methods are not among them: a block is
/// state only.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct MarkRows {
    /// Fields quoted as written, every one of them — this chart's whole
    /// quotation.
    pub(super) fields: Vec<FieldRow>,
    /// An enum's variants as written, all of them.
    pub(super) variants: Vec<FieldRow>,
    /// A static's declared type, as written.
    pub(super) ty: String,
    /// The workspace type that type reaches, drawn in full ink — and where
    /// its own block stands, so the run is a link to it.
    pub(super) ty_target: Held,
}

/// Where the epoch left one block. `ghost` and a `delta` of `Added` are the
/// two ends of the same axis and never both true.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(super) struct MarkState {
    pub(super) delta: Delta,
    /// The base had it, the working copy does not: drawn dashed from the base.
    pub(super) ghost: bool,
}

/// Where a block seats among the others.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct MarkSeat {
    pub(super) tier: Tier,
    /// The marks nested inside this block, in the survey's order.
    pub(super) kids: Vec<u32>,
    /// Structural holders folded to a count: nonzero only on vocabulary
    /// marks, whose incoming holds rest folded.
    pub(super) held_by: u32,
}

/// What leans on a block that this chart draws no mark for. All of it is the
/// chart's own limit stated in numbers and rows, never silence.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct Undrawn {
    /// Distinct declarations whose own signature names it — free functions,
    /// method rows, consts, aliases, trait clauses. None of them has a block
    /// here, so the count is the ink the chart will not draw.
    pub(super) named_by: u32,
    /// The bodies with no block here that reach it — function bodies,
    /// mostly. The other half of the same undrawn ink, kept as ends and not
    /// as a number, because the sheet lists every one of them; heaviest
    /// first. The paper says only how many.
    pub(super) used_by: Vec<Unseen>,
    /// Where its own impls reach code the chart draws no mark for, the same
    /// way round. Said on the sheet, never on the paper.
    pub(super) unseen_uses: Vec<Unseen>,
    /// The types that hold it and the visibility reading left off the paper.
    /// A holder with no block is still a holder, so it is an end and not a
    /// count: the sheet gives each one a row that quotes its source, and
    /// [`Stand::Narrower`] is the tier this list explains.
    pub(super) holders_off: Vec<u32>,
}

impl DataMark {
    pub(super) fn is_static(&self) -> bool {
        self.head.kind == ItemKind::Static
    }

    /// A root wears the gate's 2.5px ink left edge — the static's own mark,
    /// widened to every block a chain of holding begins at.
    pub(super) fn is_root(&self) -> bool {
        matches!(self.seat.tier, Tier::Root)
    }

    /// Where it is written; a ghost's line is the base edition's.
    pub(super) fn locator(&self) -> String {
        if self.state.ghost {
            format!("{}:{} (base)", self.head.path, self.head.line)
        } else {
            format!("{}:{}", self.head.path, self.head.line)
        }
    }

    /// The letter the mark wears, in git's own alphabet.
    pub(super) fn letter(&self) -> Option<&'static str> {
        if self.state.ghost {
            return Some("D");
        }
        match self.state.delta {
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
pub(super) struct Hold {
    pub(super) held: Anchor,
    pub(super) holder: Anchor,
    pub(super) kind: HoldKind,
    /// The strongest wrapper on the walk, in its own word.
    pub(super) via: String,
    /// Rows drawing this edge.
    pub(super) fields: u32,
    /// Drawn at rest. A folded edge inks in when either end is hovered.
    pub(super) rest: bool,
    pub(super) event: Option<HoldEvent>,
}

impl Hold {
    pub(super) fn key(&self) -> String {
        format!(
            "{:?}>{:?}:{:?}:{}:{:?}",
            self.held, self.holder, self.kind, self.via, self.event
        )
    }
}

/// One implementation dependence between two drawn marks: one type's impls
/// lean on another type. The dashed family, always drawn the same direction —
/// the arrowhead rests on the user.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Tie {
    pub(super) def: Anchor,
    pub(super) user: Anchor,
    pub(super) count: u32,
    /// Which of the def's methods the references name, heaviest first, for
    /// the sheet. The rows are not drawn here — methods are not this
    /// chart's — but which clause a body leans on is still the answer.
    pub(super) rows: Vec<(String, u32)>,
    pub(super) rest: bool,
    pub(super) labeled: bool,
}

impl Tie {
    pub(super) fn key(&self) -> String {
        format!("{:?}~{:?}", self.def, self.user)
    }
}

/// One undrawn naming: a declaration whose own signature names a drawn type.
/// The sheet's rows and the foot's `named by n signatures` both read this.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Naming {
    /// The named type's mark.
    pub(super) ty: u32,
    /// The naming contract's mark in the survey — a free fn, const, alias,
    /// or, for a method row, the type whose API says the word.
    pub(super) namer: u32,
    /// The namer is a method row of a type rather than a free contract.
    pub(super) from_method: bool,
    pub(super) event: Option<HoldEvent>,
}

/// What the cartouche states about the survey.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct DataFacts {
    /// How many of each shape of state the workspace keeps.
    pub(super) kinds: KindCount,
    /// What the epoch did to them.
    pub(super) diff: DiffCount,
    /// The modules the diff lands in, named.
    pub(super) changed_modules: Vec<String>,
    /// Names the survey could not resolve, as [`Limits::unresolved`].
    pub(super) unresolved: u32,
    /// Data declarations the visibility reading leaves off the paper, as
    /// [`VisFloor::off_paper`] counts them.
    pub(super) off_paper: usize,
}

/// How many of each shape of state the survey found, ghosts excluded.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(super) struct KindCount {
    pub(super) structs: usize,
    pub(super) enums: usize,
    pub(super) unions: usize,
    pub(super) statics: usize,
}

/// What the epoch did to the state on the chart.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(super) struct DiffCount {
    pub(super) added: usize,
    pub(super) removed: usize,
    pub(super) changed: usize,
}

/// Everything one build of the data chart reads out of the survey.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct DataModel {
    pub(super) frames: Vec<Frame>,
    /// Drawn marks, in the survey's (file, source) order.
    pub(super) marks: Vec<DataMark>,
    /// The drawn holding edges — everything but the nesting.
    pub(super) holds: Vec<Hold>,
    /// Every current structural relation as (held, holder), the nesting
    /// included: the blast radius walks all of it, drawn or seated.
    pub(super) pairs: Vec<(Anchor, Anchor)>,
    pub(super) ties: Vec<Tie>,
    /// The undrawn naming ink, for the sheet's rows.
    pub(super) naming: Vec<Naming>,
    pub(super) multi_crate: bool,
}

impl DataModel {
    /// The facts the cartouche states, counted off the marks when it asks.
    ///
    /// These were eleven fields on the model, each its own pass over `marks`
    /// at build time. Eight of them had one reader — this method, copying
    /// them across one for one — and three (`roots`, `nested`, `standing`)
    /// had none at all: the cartouche stopped stating tier counts on
    /// 2026-08-21 and the counting stayed behind. A census is a reading of
    /// the marks, so it is taken from them, once, by whoever wants it.
    pub(super) fn facts(&self, unresolved: u32, off_paper: usize) -> DataFacts {
        let current = |m: &&DataMark| !m.state.ghost;
        let of_kind = |kind: ItemKind| {
            self.marks
                .iter()
                .filter(current)
                .filter(|m| m.head.kind == kind)
                .count()
        };
        let mut changed_modules: Vec<String> = self
            .marks
            .iter()
            .filter(|m| m.letter().is_some())
            .map(|m| self.frames[m.frame as usize].words())
            .collect();
        changed_modules.sort();
        changed_modules.dedup();
        let delta = |d: Delta| self.marks.iter().filter(|m| m.state.delta == d).count();
        DataFacts {
            kinds: KindCount {
                structs: of_kind(ItemKind::Struct),
                enums: of_kind(ItemKind::Enum),
                unions: of_kind(ItemKind::Union),
                statics: of_kind(ItemKind::Static),
            },
            diff: DiffCount {
                added: delta(Delta::Added),
                removed: self.marks.iter().filter(|m| m.state.ghost).count(),
                changed: delta(Delta::Changed),
            },
            changed_modules,
            unresolved,
            off_paper,
        }
    }

    /// Every drawn mark by its own id. The sheet and the chart both need it,
    /// and both used to build it themselves — four identical `HashMap`s over
    /// the same list.
    pub(super) fn by_id(&self) -> HashMap<u32, &DataMark> {
        self.marks.iter().map(|m| (m.id, m)).collect()
    }

    /// The frame one id names. Frames are indexed by their own id.
    pub(super) fn frame(&self, id: u32) -> Option<&Frame> {
        self.frames.get(id as usize)
    }
}

impl ItemKind {
    /// A mark this chart draws: the shapes state takes, and the statics that
    /// anchor it. Everything else names state without keeping any.
    pub(super) fn is_data(self) -> bool {
        matches!(
            self,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Static
        )
    }
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
    pub(in crate::views::data) fn build(graph: &CodeGraph, reading: &DataReading) -> Self {
        let &DataReading {
            ref_dir,
            vis_floor,
            ref folds,
        } = reading;
        let ghost_of = |id: u32| -> Option<&GhostMark> { graph.ghost(id) };
        let kind_of = |id: u32| -> Option<ItemKind> {
            graph
                .item(id)
                .map(|m| m.head.kind)
                .or_else(|| ghost_of(id).map(|g| g.head.kind))
        };
        let name_of = |id: u32| -> String {
            graph
                .items
                .get(id as usize)
                .map(|m| m.head.name.clone())
                .or_else(|| ghost_of(id).map(|g| g.head.name.clone()))
                .unwrap_or_default()
        };
        let ghost_key = |g: &GhostMark| frame_key(&g.at.krate, &g.at.path);

        // ---- Which marks are drawn. -----------------------------------------
        // Every shape and every static the visibility reading admits. Two
        // things take state off the paper and they are both the reviewer's own
        // move: a module folded by hand, which leaves its state on that
        // boundary's one counted row, and the visibility floor, which leaves a
        // narrower declaration off the reading altogether — no row, no count
        // in a frame, only the one number the cartouche states.
        let file_key: Vec<FrameKey> = graph
            .files
            .iter()
            .map(|f| frame_key(&f.krate, &f.path))
            .collect();
        let key_of =
            |mark: u32| -> Option<&FrameKey> { file_key.get(graph.item(mark)?.file as usize) };
        let fold_key = |key: &FrameKey| -> Option<FrameKey> {
            (0..=key.1.len())
                .map(|cut| (key.0.clone(), key.1[..cut].to_vec()))
                .find(|(krate, path)| folds.contains(&mod_key(krate, path)))
        };

        let mut drawn: Vec<u32> = Vec::new();
        let mut packed: Vec<u32> = Vec::new();
        let mut narrower: HashSet<u32> = HashSet::new();
        for (i, mark) in graph.items.iter().enumerate() {
            if !mark.head.kind.is_data() || mark.parent.is_some() {
                continue;
            }
            let i = i as u32;
            if !vis_floor.admits(&mark.head.vis) {
                narrower.insert(i);
            } else if key_of(i).and_then(&fold_key).is_some() {
                packed.push(i);
            } else {
                drawn.push(i);
            }
        }

        // ---- Frames: one per crate, then the module tree inside it. ---------
        let framed_key = |key: FrameKey| -> FrameKey { fold_key(&key).unwrap_or(key) };
        // A removed declaration was written as visible as its base edition
        // wrote it, so the reading reads its head like any other.
        let data_ghosts: Vec<&GhostMark> = graph
            .ghosts
            .iter()
            .filter(|g| g.head.kind.is_data() && vis_floor.admits(&g.head.vis))
            .collect();
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
                fold: Fold {
                    folded: folds.contains(&mod_key(krate, &[])),
                    packed: 0,
                },
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
                fold: Fold {
                    folded: folds.contains(&mod_key(&key.0, &key.1)),
                    packed: 0,
                },
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
                frames[frame as usize].fold.packed += 1;
                anchor_of[m as usize] = Some(Anchor::Mod(frame));
            }
        }
        for ghost in &data_ghosts {
            let key = ghost_key(ghost);
            if let Some(frame) = fold_frame(&key) {
                frames[frame as usize].fold.packed += 1;
                anchor_of[ghost.id() as usize] = Some(Anchor::Mod(frame));
            } else if let Some(&frame) = frame_index.get(&key) {
                frames[frame as usize].marks.push(ghost.id());
                anchor_of[ghost.id() as usize] = Some(Anchor::Mark(ghost.id()));
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
            !from_method && kind_of(from).is_some_and(ItemKind::is_data)
        };
        // Current structural holders per drawn type, Owns/Shares only: a
        // borrow is a view, not a hold, so it never decides the tier.
        let mut holders: HashMap<u32, HashSet<u32>> = HashMap::new();
        let mut shared: HashSet<u32> = HashSet::new();
        // Drawn types held by declarations this reading left off the paper,
        // and by which: held state with nothing on the paper to say so. Ends,
        // not a flag — the sheet gives every one of them a row.
        let mut held_narrower: HashMap<u32, Vec<u32>> = HashMap::new();
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
                if edge.event == Some(HoldEvent::Removed) {
                    continue;
                }
                if !placed {
                    // A holder the visibility reading left off the paper holds
                    // it all the same. There is no end to draw the line to, so
                    // the tier says it in words instead of reading `a root`.
                    if narrower.contains(&from)
                        && matches!(edge.kind, HoldKind::Owns | HoldKind::Shares)
                    {
                        held_narrower.entry(to).or_default().push(from);
                    }
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
            let Some(mark) = graph.item(id) else {
                continue;
            };
            if mark.head.kind == ItemKind::Static || vocab.contains(&id) {
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
            let Some(mark) = graph.item(id) else {
                return Tier::Root;
            };
            if mark.head.kind == ItemKind::Static {
                return Tier::Root;
            }
            if let Some(&parent) = nest.get(&id) {
                return Tier::Nested(parent);
            }
            let held = holders.get(&id).map_or(0, |set| set.len());
            if held == 0 {
                match held_narrower.contains_key(&id) {
                    true => Tier::Standing(Stand::Narrower),
                    false => Tier::Root,
                }
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
            if frame.fold.folded {
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
                    .map(|g| g.id()),
            )
            .collect();
        // Where a held type's block stands, by the key its URL uses, so the
        // bold run naming it is a link to it. A run the chart draws no block
        // for is bold text and nothing more, and so is a self-hold: the
        // reader is already standing in that block.
        let seat_of = |id: u32| -> Option<(String, String)> {
            match graph.item(id) {
                Some(mark) => Some((graph.path_of(mark)?.to_string(), mark.head.label.clone())),
                None => ghost_of(id).map(|g| (g.at.path.clone(), g.head.name.clone())),
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
                .body
                .field_rows
                .iter()
                .enumerate()
                .map(|(at, row)| FieldRow {
                    name: row.name.clone(),
                    decl: row.ty.clone(),
                    vis: row.vis.clone(),
                    target: target(id, &row.name),
                    state: if mark.diff.fields_added.contains(&(at as u32)) {
                        RowState::Added
                    } else {
                        RowState::Same
                    },
                })
                .collect();
            let mut dropped: Vec<(usize, FieldRow)> = mark
                .diff
                .fields_removed
                .iter()
                .map(|(before, row)| {
                    (
                        *before as usize,
                        FieldRow {
                            name: row.name.clone(),
                            decl: row.ty.clone(),
                            vis: row.vis.clone(),
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
                .body
                .variants
                .iter()
                .enumerate()
                .map(|(at, written)| FieldRow {
                    name: String::new(),
                    decl: written.clone(),
                    vis: Vis::Private,
                    target: target(id, &vname(written)),
                    state: if mark.diff.variants_added.contains(&(at as u32)) {
                        RowState::Added
                    } else {
                        RowState::Same
                    },
                })
                .collect();
            let mut dropped: Vec<(usize, FieldRow)> = mark
                .diff
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
                let mark = graph.item(id)?;
                let frame = frame_of(id)?;
                let file = graph.file(mark.file)?;
                let tier = tier_of(id);
                Some(DataMark {
                    id,
                    frame,
                    head: MarkHead {
                        kind: mark.head.kind,
                        vis: mark.head.vis.clone(),
                        name: mark.head.name.clone(),
                        label: mark.head.label.clone(),
                        path: file.path.clone(),
                        line: mark.head.line,
                    },
                    rows: MarkRows {
                        fields: field_rows(id, mark),
                        variants: variant_rows(id, mark),
                        ty: mark.body.ty.clone(),
                        ty_target: target(id, &mark.head.name),
                    },
                    state: MarkState {
                        delta: mark.diff.delta,
                        ghost: false,
                    },
                    seat: MarkSeat {
                        tier,
                        kids: kids.get(&id).cloned().unwrap_or_default(),
                        held_by: if vocab.contains(&id) {
                            holders.get(&id).map_or(0, |set| set.len() as u32)
                        } else {
                            0
                        },
                    },
                    // The ties pass fills in the two lists.
                    undrawn: Undrawn {
                        named_by: named_set.get(&id).map_or(0, |set| set.len() as u32),
                        ..Undrawn::default()
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
                id: ghost.id(),
                frame,
                head: MarkHead {
                    kind: ghost.head.kind,
                    vis: ghost.head.vis.clone(),
                    name: ghost.head.name.clone(),
                    label: ghost.head.name.clone(),
                    path: ghost.at.path.clone(),
                    line: ghost.head.line,
                },
                rows: MarkRows {
                    fields: ghost
                        .body
                        .field_rows
                        .iter()
                        .map(|row| FieldRow {
                            name: row.name.clone(),
                            decl: row.ty.clone(),
                            vis: row.vis.clone(),
                            target: target(ghost.id(), &row.name),
                            state: RowState::Same,
                        })
                        .collect(),
                    variants: ghost
                        .body
                        .variants
                        .iter()
                        .map(|written| FieldRow {
                            name: String::new(),
                            decl: written.clone(),
                            vis: Vis::Private,
                            target: target(ghost.id(), &vname(written)),
                            state: RowState::Same,
                        })
                        .collect(),
                    ty: ghost.body.ty.clone(),
                    ty_target: target(ghost.id(), &ghost.head.name),
                },
                state: MarkState {
                    delta: Delta::Same,
                    ghost: true,
                },
                seat: MarkSeat {
                    tier: Tier::Standing(Stand::Afar),
                    kids: Vec::new(),
                    held_by: 0,
                },
                undrawn: Undrawn::default(),
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
            .filter_map(|&id| Some((id, graph.item(id)?)))
            .flat_map(|(id, mark)| {
                mark.body
                    .method_rows
                    .iter()
                    .map(move |row| (row.mark, (id, row.name.clone())))
            })
            .collect();
        let mut rows_acc: HashMap<(u32, u32), HashMap<String, u32>> = HashMap::new();
        for &MarkRef { from, to, count } in &graph.refs {
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
            mark.undrawn.used_by = unseen_ends(unseen_in.get(&mark.id));
            mark.undrawn.unseen_uses = unseen_ends(unseen_out.get(&mark.id));
            if let Some(holders) = held_narrower.get(&mark.id) {
                let mut holders = holders.clone();
                holders.sort_by_key(|&id| (name_of(id), id));
                holders.dedup();
                mark.undrawn.holders_off = holders;
            }
        }
        let multi_crate = crates.len() > 1;
        Self {
            frames,
            marks,
            holds,
            pairs,
            ties,
            naming,
            multi_crate,
        }
    }
}

#[cfg(test)]
/// The survey builders this altitude's tests share: `/data`'s model and
/// its sheet both read one `CodeGraph`, so both build theirs the same way.
pub(in crate::views::data) mod tests {
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
        assert_eq!(module_path("src/views/data/map.rs"), ["views", "data"]);
        // A module's own file and a file beside it frame in that module itself.
        assert_eq!(module_path("src/views/mod.rs"), ["views"]);
        assert_eq!(module_path("src/views/shell.rs"), ["views"]);
        // A file under the source root is the module it declares.
        assert_eq!(module_path("src/load.rs"), ["load"]);
        assert!(module_path("src/main.rs").is_empty());
        assert!(module_path("crates/engine/src/lib.rs").is_empty());
        assert_eq!(module_path("crates/engine/src/parse/lex.rs"), ["parse"]);
    }
    use crate::graph::data::{
        DeclBody, DeclDiff, DeclHead, DeclRow, FileInfo, HoldEdge, Limits, Reach, Vis,
    };
    use crate::views::data::VisFloor;

    pub(in crate::views::data) fn file(path: &str) -> FileInfo {
        FileInfo {
            path: path.to_string(),
            krate: "slope".to_string(),
        }
    }

    pub(in crate::views::data) fn mark(id: u32, file: u32, name: &str, kind: ItemKind) -> ItemMark {
        ItemMark {
            id,
            file,
            parent: None,
            head: DeclHead {
                name: name.to_string(),
                label: name.to_string(),
                kind,
                vis: Vis::Private,
                line: id + 1,
            },
            body: DeclBody::default(),
            reach: Reach::default(),
            diff: DeclDiff::default(),
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

    pub(in crate::views::data) fn graph(items: Vec<ItemMark>, holds: Vec<HoldEdge>) -> CodeGraph {
        CodeGraph {
            files: vec![file("src/graph/data.rs"), file("src/views/dep/map.rs")],
            items,
            implements: Vec::new(),
            refs: Vec::new(),
            holds,
            ghosts: Vec::new(),
            limits: Limits::default(),
        }
    }

    pub(in crate::views::data) fn build(graph: &CodeGraph) -> DataModel {
        DataModel::build(graph, &DataReading::default())
    }

    pub(in crate::views::data) fn by_name<'a>(model: &'a DataModel, name: &str) -> &'a DataMark {
        model.marks.iter().find(|m| m.head.name == name).unwrap()
    }

    /// The visibility reading is a floor on what the paper draws: each stop
    /// keeps every rung above it and adds the next one down, and nothing else
    /// about a drawn block changes.
    #[test]
    fn the_visibility_reading_draws_only_what_is_written_that_wide() {
        let mut g = graph(
            vec![
                mark(0, 0, "Open", ItemKind::Struct),
                mark(1, 0, "Crated", ItemKind::Struct),
                mark(2, 0, "Scoped", ItemKind::Struct),
                mark(3, 0, "Shut", ItemKind::Struct),
            ],
            Vec::new(),
        );
        g.items[0].head.vis = Vis::Pub;
        g.items[1].head.vis = Vis::Crate;
        g.items[2].head.vis = Vis::In("crate::views".to_string());
        let drawn = |vis_floor: VisFloor| -> Vec<String> {
            let reading = DataReading {
                vis_floor,
                ..DataReading::default()
            };
            DataModel::build(&g, &reading)
                .marks
                .iter()
                .map(|m| m.head.name.clone())
                .collect()
        };
        assert_eq!(drawn(VisFloor::Pub), ["Open"]);
        assert_eq!(drawn(VisFloor::Crate), ["Open", "Crated"]);
        assert_eq!(drawn(VisFloor::Super), ["Open", "Crated", "Scoped"]);
        assert_eq!(drawn(VisFloor::All), ["Open", "Crated", "Scoped", "Shut"]);
        // What a reading leaves off is a number it states, so a narrow reading
        // never reads as an empty workspace.
        assert_eq!(VisFloor::Pub.off_paper(&g), 3);
        assert_eq!(VisFloor::All.off_paper(&g), 0);
    }

    /// A type whose every holder is narrower than the reading draws is not a
    /// root: the holding is real, and the paper simply has no block to draw the
    /// line from. It stands, and says so.
    #[test]
    fn a_type_the_reading_leaves_no_holder_for_stands_rather_than_reads_as_a_root() {
        let mut g = graph(
            vec![
                mark(0, 0, "Keeper", ItemKind::Struct),
                mark(1, 0, "Held", ItemKind::Struct),
            ],
            vec![owns(0, 1)],
        );
        // The holder stays private; what it holds is published.
        g.items[1].head.vis = Vis::Pub;
        let reading = DataReading {
            vis_floor: VisFloor::Pub,
            ..DataReading::default()
        };
        let model = DataModel::build(&g, &reading);
        let held = by_name(&model, "Held");
        assert_eq!(held.seat.tier, Tier::Standing(Stand::Narrower));
        assert!(!held.is_root());
        // And no ink is drawn to a block that is not on the paper — but the
        // holder is still an end the sheet can name and quote, so it is
        // carried as one rather than left to the tier sentence alone.
        assert!(model.holds.is_empty());
        assert_eq!(held.undrawn.holders_off, vec![0]);
        // Widened, the nesting is back exactly as it was.
        assert_eq!(by_name(&build(&g), "Held").seat.tier, Tier::Nested(0));
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
        assert_eq!(by_name(&model, "Wire").seat.tier, Tier::Root);
        assert_eq!(by_name(&model, "Nut").seat.tier, Tier::Nested(0));
        assert_eq!(by_name(&model, "Wire").seat.kids, vec![1]);
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
        g.items[0].body.field_rows = vec![DeclRow {
            name: "field".into(),
            ty: "Vec<Nut>".into(),
            vis: Vis::Private,
        }];
        let model = build(&g);
        let held = by_name(&model, "Wire").rows.fields[0].target.clone();
        assert_eq!(held.name, "Nut");
        assert_eq!(held.at, Some(mark_route("src/views/dep/map.rs", "Nut")));

        // A shape that holds itself: the link would go where the reader is.
        let mut g = graph(vec![mark(0, 0, "Node", ItemKind::Struct)], vec![owns(0, 0)]);
        g.items[0].body.field_rows = vec![DeclRow {
            name: "field".into(),
            ty: "Option<Box<Node>>".into(),
            vis: Vis::Private,
        }];
        let model = build(&g);
        let held = by_name(&model, "Node").rows.fields[0].target.clone();
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
        g.items[0].head.kind = ItemKind::Fn;
        let model = build(&g);
        // The fn has no block; the struct is a root, its naming counted.
        assert_eq!(model.marks.len(), 1);
        let wire = by_name(&model, "Wire");
        assert_eq!(wire.seat.tier, Tier::Root);
        assert_eq!(wire.undrawn.named_by, 1);
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
            by_name(&model, "Config").seat.tier,
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
        assert_eq!(
            by_name(&model, "Wire").seat.tier,
            Tier::Standing(Stand::Afar)
        );
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
        assert_eq!(by_name(&model, "Wire").seat.tier, Tier::Root);
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
            .filter(|m| matches!(m.seat.tier, Tier::Nested(_)))
            .count();
        assert_eq!(nested, 1);
        assert_eq!(
            model
                .marks
                .iter()
                .filter(|m| m.seat.tier == Tier::Standing(Stand::Ring))
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
        assert_eq!(placed.seat.tier, Tier::Standing(Stand::Vocab));
        assert_eq!(placed.seat.held_by, 5);
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
        items[0].body.ty = "Mutex<Trail>".into();
        let g = graph(items, vec![owns(0, 1)]);
        let model = build(&g);
        assert_eq!(by_name(&model, "CACHE").seat.tier, Tier::Root);
        assert_eq!(by_name(&model, "Trail").seat.tier, Tier::Nested(0));
        assert_eq!(by_name(&model, "CACHE").seat.kids, vec![1]);
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
        g.refs = vec![
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
        assert_eq!(wire.undrawn.used_by.len(), 1);
        assert_eq!(
            (wire.undrawn.used_by[0].item, wire.undrawn.used_by[0].count),
            (0, 4)
        );
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
        assert_eq!(nut.seat.tier, Tier::Root);
        assert_eq!(nut.undrawn.named_by, 1);
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
        assert_eq!(by_name(&model, "Nut").seat.tier, Tier::Nested(0));
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
