//! The code survey: every workspace file, every item it declares, and every
//! relation between them the analysis could resolve — what holds what, what
//! names what, what implements what, and how each declaration differs from
//! the diff base.
//!
//! The shape only. Which items nest inside which on a plate, what a block
//! measures, and which rows a fold shows are all readings of this graph, and
//! every one of them belongs to the chart doing the reading.

use serde::{Deserialize, Serialize};

/// What kind of thing one item in a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ItemKind {
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

impl ItemKind {
    /// An item's kind, as rust writes it. The keyword is the representation
    /// every rust reader already has; there is nothing to learn.
    pub(crate) fn words(self) -> &'static str {
        match self {
            ItemKind::Fn => "fn",
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Union => "union",
            ItemKind::Trait => "trait",
            ItemKind::TypeAlias => "type",
            ItemKind::Const => "const",
            ItemKind::Static => "static",
            ItemKind::Macro => "macro",
            ItemKind::Mod => "mod",
            ItemKind::Impl => "impl",
        }
    }

    /// `pub fn`, `struct`, `pub(crate) mod` — what rust writes in front of a
    /// name. A private item declares no visibility, so neither does its row.
    pub(crate) fn decl_words(self, vis: &Vis) -> String {
        match vis.keyword() {
            Some(vis) => format!("{vis} {}", self.words()),
            None => self.words().to_string(),
        }
    }
}

/// How widely a declaration is written visible, rung by rung. `pub(crate)` is
/// not `pub` and `pub(super)` is not `pub(crate)`, and the survey keeps all
/// four apart: the data chart's visibility reading slides along these rungs,
/// and a rung the survey blurred is a rung a chart would have to guess at.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Vis {
    /// `pub` — the declaration leaves its own crate.
    Pub,
    /// `pub(crate)` — the whole crate, and no further.
    Crate,
    /// `pub(super)` — the parent module only.
    Super,
    /// `pub(in path)`, carrying the path exactly as the source writes it. Rust
    /// names one module to stop at, so the keyword is that line and not
    /// another rung's: `pub(in crate::views)` is written, never guessed at.
    In(String),
    /// No `pub` at all, or `pub(self)`, which is no wider than none.
    Private,
}

impl Vis {
    /// The visibility as rust writes it. Private declares nothing, so it has
    /// no keyword: rust writes nothing at all, and so does the interface.
    pub(crate) fn keyword(&self) -> Option<String> {
        match self {
            Vis::Pub => Some("pub".to_string()),
            Vis::Crate => Some("pub(crate)".to_string()),
            Vis::Super => Some("pub(super)".to_string()),
            Vis::In(path) => Some(format!("pub(in {path})")),
            Vis::Private => None,
        }
    }
}

/// One source file in the workspace. Its id is its index into
/// [`CodeGraph::files`] — read one back with [`CodeGraph::file`] rather than
/// storing the index on the file, where it could come to disagree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FileInfo {
    /// Path relative to the workspace root, e.g. `src/views/dep/map.rs`.
    pub(crate) path: String,
    /// The cargo **package** that owns this file — `slope-cli`, not the
    /// `slope` binary target rust-analyzer resolved it under. The dependency
    /// altitude draws packages, so this is the one name both altitudes can
    /// key a crate on, and every cross-altitude link depends on it.
    pub(crate) krate: String,
}

/// How an item's own declaration differs from the diff base — the structural
/// diff. Computed syntactically: the base edition of each changed file is
/// parsed (never type-resolved) and declarations are matched by kind and
/// name, so it is exact about added, removed, and rewritten declarations and
/// says nothing about what it cannot see. Items the base already had, in
/// files the diff never touched, are `Same` by construction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Delta {
    /// Written at the base exactly as it is now.
    #[default]
    Same,
    /// Not at the base: this epoch added it.
    Added,
    /// At the base with different text: this epoch rewrote it.
    Changed,
}

/// A holding relation's own diff event. `Added` edges are live edges the base
/// did not have; `Removed` edges are re-drawn from the base edition and exist
/// only as diff ink — the working copy no longer has the field that drew them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum HoldEvent {
    Added,
    Removed,
}

/// A type, static, or free function the base had that the working copy
/// dropped. The data chart draws it as a ghost — dashed frame, rows quoted
/// from the base edition — so a removed declaration leaves a mark instead of
/// vanishing. Its `id` continues after [`CodeGraph::items`], so a
/// [`HoldEdge`] can land on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GhostMark {
    /// `items.len() + index into ghosts` — one id space with the live marks.
    ///
    /// Private, and assigned by [`CodeGraph::push_ghost`]: the id encodes a
    /// position in a list the ghost cannot see, so it is not a caller's to
    /// choose. That method and [`CodeGraph::ghost`] are the only code in the
    /// crate that knows how the two id spaces meet.
    id: u32,
    /// Where it stood at the base.
    pub(crate) at: GhostAt,
    /// What its declaration said there.
    pub(crate) head: DeclHead,
    /// The rows the base wrote under it.
    pub(crate) body: BaseBody,
}

impl GhostMark {
    /// Where a [`HoldEdge`] lands on it. Stamped by
    /// [`CodeGraph::push_ghost`], which is the only thing that may set it.
    pub(crate) fn id(&self) -> u32 {
        self.id
    }
}

/// Where a removed declaration stood. Both may be gone from the working copy
/// with it, so neither is a [`FileInfo`] the survey still holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GhostAt {
    /// The file that declared it at the base, relative to the workspace root.
    pub(crate) path: String,
    /// The cargo package that owned that file, as [`FileInfo::krate`].
    pub(crate) krate: String,
}

/// The rows a declaration wrote at the base edition. Not [`DeclBody`]: the
/// diff reads the base syntactically, so a base method is a quoted
/// (name, signature) and never a [`MethodRow`] with a mark of its own.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct BaseBody {
    /// Fields — or a function's parameters — as the base wrote them.
    pub(crate) field_rows: Vec<DeclRow>,
    /// An enum's variants as the base wrote them.
    pub(crate) variants: Vec<String>,
    /// A static's declared type, or a function's return type, at the base.
    pub(crate) ty: String,
    /// The methods the base wrote for it, quoted as (name, signature). A
    /// ghost's band is drawn whole: the base edition is all there is of it,
    /// so nothing here is gated on a door.
    pub(crate) method_rows: Vec<(String, String)>,
}

/// One declaration as the base edition wrote it: the head, and every row of
/// the body, quoted. A ghost *is* this, plus where it stood — so this and a
/// [`GhostAt`] are all [`CodeGraph::push_ghost`] asks for, and the id is not
/// part of either. The structural diff builds these and nothing else does, so
/// it stays off the client with the diff.
#[cfg(any(feature = "server", test))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GhostDecl {
    pub(crate) head: DeclHead,
    pub(crate) body: BaseBody,
}

/// One quoted row of a declaration: a struct or union field, or a free
/// function's parameter, exactly as the source writes it. Nothing here is
/// reconstructed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DeclRow {
    /// The name as written — a tuple field's is its index, a parameter's is
    /// its pattern.
    pub(crate) name: String,
    /// The declared type as written.
    pub(crate) ty: String,
    /// What the row declares for *itself*, which is not what its item
    /// declares: a `pub(crate)` struct can publish some fields and keep
    /// others, and a reader deciding what may touch this state has to see
    /// which. A parameter declares nothing, ever, so it is always private.
    pub(crate) vis: Vis,
}

/// The structural diff's own reading of a row, and the only one: the client
/// draws from the data chart's own `FieldRow`, which carries the diff state
/// as well.
#[cfg(feature = "server")]
impl DeclRow {
    /// The row as rust writes it, visibility included — what the diff
    /// compares, so dropping a field's `pub` reads as the declaration change
    /// it is.
    pub(crate) fn written(&self) -> String {
        match self.vis.keyword() {
            Some(keyword) => format!("{keyword} {}", self.ty),
            None => self.ty.clone(),
        }
    }
}

/// One method of a type, quoted from its own source. Methods are never marks
/// — a method belongs to its type the way a field does — and no chart draws
/// them as rows: what they are needed for is attribution, and the data
/// sheet's own list of what the selected type offers. A reference resolved to
/// a method is filed under its type, a method naming a type is a naming
/// rather than a holding, and the structural diff compares these rows to tell
/// a rewritten API from a rewritten shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MethodRow {
    /// The method's own name, which is what its edges and its callers file
    /// under.
    pub(crate) name: String,
    /// The signature exactly as written, from the `fn` keyword's line through
    /// the return type, whitespace collapsed — visibility included, body and
    /// doc comment left where they are. What the structural diff compares.
    pub(crate) sig: String,
    /// Its own [`ItemMark::id`] — so a reference the survey resolved to the
    /// method itself can be filed under this row rather than blurred into
    /// its type.
    pub(crate) mark: u32,
    /// The impl block that writes it, as the survey headers one: `impl Vis`
    /// for an inherent method, `impl Clone for Vis` for a trait's, `trait
    /// Held` for a trait's own clause. What separates a type's own API from
    /// the contracts it promises, without asking the file for its sections.
    pub(crate) section: String,
}

/// One landmark a chart may engrave: an item, seated in the containment tree
/// (crate → directory → file → type → method), with the weight of what leans
/// on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ItemMark {
    /// Index into [`CodeGraph::items`].
    pub(crate) id: u32,
    /// The file whose source defines it.
    pub(crate) file: u32,
    /// Semantic container: the type a method or associated item belongs to,
    /// resolved through the impl's self type even when the impl sits in
    /// another file. `None` for items the file itself contains.
    pub(crate) parent: Option<u32>,
    /// What its declaration's own head says.
    pub(crate) head: DeclHead,
    /// The rows it writes under that head.
    pub(crate) body: DeclBody,
    /// What the rest of the workspace does with it.
    pub(crate) reach: Reach,
    /// How this declaration differs from the diff base.
    pub(crate) diff: DeclDiff,
}

/// What a declaration's own head says — everything rust writes before the
/// brace, and where it writes it. A live mark and a [`GhostMark`] carry the
/// same one: a removed declaration had a head too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DeclHead {
    /// Display name, without any section prefix.
    pub(crate) name: String,
    /// The label this item selects by in a URL: `Type::method` inside a
    /// section, the plain name otherwise. A ghost has none to select by.
    pub(crate) label: String,
    pub(crate) kind: ItemKind,
    pub(crate) vis: Vis,
    /// 1-based line in the edition this head was read from.
    pub(crate) line: u32,
}

/// The rows a declaration writes under its head, quoted from source. Every
/// one of them is empty for the kinds that write none.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct DeclBody {
    /// A struct's or union's fields — or a free function's parameters —
    /// quoted from source in declaration order, each carrying the visibility
    /// it declares for itself. The data chart quotes them all — what a field
    /// or a parameter reaches is on the holds edges.
    pub(crate) field_rows: Vec<DeclRow>,
    /// An enum's variants as written, in source order — name, payload types,
    /// and discriminant included.
    pub(crate) variants: Vec<String>,
    /// A static's declared type or a free function's return type, as written.
    /// Empty for a function that returns nothing.
    pub(crate) ty: String,
    /// The methods declared for this type anywhere in the workspace, in the
    /// survey's order — the second band of its block. Every one of them is
    /// here, whatever its visibility: which ones are rows is a door, and a
    /// door is the client's to set.
    pub(crate) method_rows: Vec<MethodRow>,
}

/// What the rest of the workspace does with one declaration — neither of
/// these is written where the declaration is, and neither can be read off it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Reach {
    /// Item-level references reaching it from other files: how much of the
    /// workspace leans on it.
    pub(crate) fan_in: u32,
    /// Hand-written trait impls of this type, as their headers are written
    /// (`impl Clone for Vis`), gathered from every impl anywhere in the
    /// workspace. Derives are not here: they stand in the type's own source,
    /// and a derive is not code anyone wrote.
    pub(crate) impls: Vec<String>,
}

/// How one declaration differs from the diff base: the verdict, and which
/// rows moved under it.
///
/// One field on [`ItemMark`] rather than seven, because the structural diff
/// writes all of it at once and long after the survey stands. A freshly
/// surveyed mark takes the default — the base's shape, nothing moved — so no
/// caller has to spell out seven empties to say "the diff has not run yet",
/// and no caller can spell out three of them and forget the rest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct DeclDiff {
    /// How this declaration differs from the diff base.
    pub(crate) delta: Delta,
    /// Fields added since the base: indexes into [`ItemMark::field_rows`].
    pub(crate) fields_added: Vec<u32>,
    /// Fields the base had that the working copy dropped, quoted from the
    /// base: (insert before this index of `field_rows`, the row).
    pub(crate) fields_removed: Vec<(u32, DeclRow)>,
    /// Variants added since the base: indexes into [`ItemMark::variants`].
    pub(crate) variants_added: Vec<u32>,
    /// Variants the base had that the working copy dropped, quoted from the
    /// base: (insert before this index of `variants`, the variant as written).
    pub(crate) variants_removed: Vec<(u32, String)>,
    /// Methods added since the base: indexes into [`ItemMark::method_rows`].
    pub(crate) methods_added: Vec<u32>,
    /// Methods the base had that the working copy dropped, quoted from the
    /// base: (insert before this index of `method_rows`, name, signature).
    pub(crate) methods_removed: Vec<(u32, String, String)>,
}

/// What a row says about what it reaches: whether the dependent owns the
/// value outright, shares a handle to it, only views it, names a trait
/// instead of a type — or promises the trait's whole contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum HoldKind {
    /// No shared handle anywhere on the walk: the holder owns the value.
    /// Interior mutability alone (`Mutex`, `RefCell`) is still ownership.
    Owns,
    /// A shared handle — `Arc`, `Rc`, `Weak`, or a dioxus signal — so other
    /// code can reach the same state.
    Shares,
    /// A reference: the holder views state something else owns.
    Borrows,
    /// `dyn Trait`: the edge lands on a trait, not on a type.
    Dyn,
    /// `impl Trait for Type`, resolved semantically: the type promises the
    /// trait's contract. No row draws it — the impl block does — and no
    /// wrapper word rides on it.
    Implements,
}

/// One holding relation: `from` has one or more fields whose type walk
/// reaches `to`. Aggregated per (from, to, kind, wrapper), so every field
/// that says the same thing arrives on one edge. A free function's signature
/// draws the same edges from the same walk: a parameter or a return type
/// names a workspace type the way a field does. Private types are here too
/// — privacy folds the chart, it does not hide a fact from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct HoldEdge {
    /// The holder's [`ItemMark::id`]: a struct, an enum, a union, a static,
    /// or a free function whose signature names the held type.
    pub(crate) from: u32,
    /// The held type's [`ItemMark::id`]. Equal to `from` when a type holds
    /// itself, which linked structures really do.
    pub(crate) to: u32,
    pub(crate) kind: HoldKind,
    /// The strongest wrapper met on the walk, in its own word (`Arc`, `Rc`,
    /// `Weak`, `Signal`, `&`, `&mut`, `dyn`); empty for a plain hold.
    pub(crate) via: String,
    /// Every field that draws this edge, quoted from source in declaration
    /// order: (name as written, declared type as written). A tuple field's
    /// name is its index; an enum payload's is its variant's name; a static's
    /// is the static's own name, and so is a free function's return type's.
    /// A method row's is the method's name, whichever part of its signature
    /// reached the target.
    pub(crate) fields: Vec<(String, String)>,
    /// The rows drawing this edge are `from`'s **methods**, not its fields:
    /// its API names the held type rather than keeping one. Aggregation
    /// splits on this, so a pair reached both ways draws both edges and
    /// neither reading has to be guessed at.
    pub(crate) from_method: bool,
    /// This relation against the diff base: `None` = the base held it too.
    /// A `Removed` edge is not structure — the working copy no longer has the
    /// field that drew it — and either end may name a ghost.
    pub(crate) event: Option<HoldEvent>,
}

/// One `impl Trait for Type` between two marks the chart draws. The
/// arrowhead rests on the type, as every family's does: the trait is the
/// contract, and a change to it travels to everything that promised it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ImplEdge {
    /// The trait's [`ItemMark::id`] — the tail.
    pub(crate) trait_mark: u32,
    /// The implementing type's [`ItemMark::id`] — the dependent.
    pub(crate) ty: u32,
    /// This impl against the diff base. A workspace type promising a new
    /// contract, or dropping one, is the kind of change a reviewer came for.
    pub(crate) event: Option<HoldEvent>,
}

/// A reference between two marks, summed. Both ends are marks by
/// construction, and neither carries its file: which file a reference was
/// written in says nothing about whether one type's code leans on another,
/// and the file is on the mark for anyone who wants it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MarkRef {
    /// The [`ItemMark::id`] whose body names the other. A reference written
    /// in an impl block belongs to the type the impl names.
    pub(crate) from: u32,
    pub(crate) to: u32,
    pub(crate) count: u32,
}

/// The code-structure survey: every workspace source file, every resolved
/// reference at item precision, and every item a chart can engrave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CodeGraph {
    pub(crate) files: Vec<FileInfo>,
    /// Every chartable item, in (file, source) order. Impl blocks are not
    /// here: they are attribution, not geometry.
    pub(crate) items: Vec<ItemMark>,
    /// Which workspace types implement which workspace traits, resolved
    /// through the impl's own self type and trait — never from the header
    /// text. An impl of a foreign trait, or for a foreign type, is not here:
    /// it stays a string on [`ItemMark::impls`], because it has no second end
    /// to land on.
    pub(crate) implements: Vec<ImplEdge>,
    /// Every resolved reference the survey placed at item precision, summed
    /// per pair — across files and inside one file alike. A reference the
    /// survey could only place on a file as a whole (a `use` of its module)
    /// has no second end to land on and is not here; it is counted into
    /// [`ItemMark::fan_in`] and nowhere else.
    pub(crate) refs: Vec<MarkRef>,
    /// Which type holds which, and through what wrapper — the data
    /// altitude's structure. Every surveyed type is here, private ones
    /// included. Edges carrying a [`HoldEvent`] are the structural diff's:
    /// `Removed` ones are re-drawn from the base edition.
    pub(crate) holds: Vec<HoldEdge>,
    /// Types, statics, and free functions the base had that the working copy
    /// dropped. Their ids continue after `items`, so `holds` can land on them.
    pub(crate) ghosts: Vec<GhostMark>,
    /// What this survey could not read, in its own words.
    pub(crate) limits: Limits,
}

/// What a survey could not read — the words every cartouche's "what the
/// survey cannot read" fold is built from, so no chrome has to paraphrase the
/// survey in prose of its own.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Limits {
    /// Names the survey could not resolve (type-inference limits). They are
    /// not on the chart; the words on the plate must say so.
    pub(crate) unresolved: u32,
    /// What the survey could not read about **references** — the limits of
    /// the dashed uses ink.
    pub(crate) notes: Vec<String>,
    /// What the survey could not read about the **holds walk** — the data
    /// chart's limits. Kept apart from `notes` so each fold states the limits
    /// of the ink its own chart draws, and no more.
    pub(crate) walk_notes: Vec<String>,
}

impl CodeGraph {
    /// The live mark one id names.
    pub(crate) fn item(&self, id: u32) -> Option<&ItemMark> {
        self.items.get(id as usize)
    }

    /// The file one [`ItemMark::file`] names.
    pub(crate) fn file(&self, id: u32) -> Option<&FileInfo> {
        self.files.get(id as usize)
    }

    /// The path of the file a mark is written in, relative to the workspace
    /// root — the two-step every locator and every route needs.
    pub(crate) fn path_of(&self, mark: &ItemMark) -> Option<&str> {
        Some(self.file(mark.file)?.path.as_str())
    }

    /// The ghost one id names, or `None` when the id is a live mark's.
    ///
    /// Ghosts share the marks' id space, continuing after `items`, so that a
    /// [`HoldEdge`] can land on either without knowing which it has. This is
    /// the only place that decodes that, and [`CodeGraph::push_ghost`] the
    /// only place that encodes it.
    pub(crate) fn ghost(&self, id: u32) -> Option<&GhostMark> {
        (id as usize)
            .checked_sub(self.items.len())
            .and_then(|at| self.ghosts.get(at))
    }

    /// Seat a declaration the base had and the working copy dropped, and hand
    /// back the id a [`HoldEdge`] can land on. The id is stamped here, not
    /// passed in: it is a position in these two lists, which is the graph's
    /// business and no caller's.
    #[cfg(any(feature = "server", test))]
    pub(crate) fn push_ghost(&mut self, at: GhostAt, decl: GhostDecl) -> u32 {
        let id = (self.items.len() + self.ghosts.len()) as u32;
        self.ghosts.push(GhostMark {
            id,
            at,
            head: decl.head,
            body: decl.body,
        });
        id
    }
}
