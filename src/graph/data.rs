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
    pub(crate) fn decl_words(self, vis: Vis) -> String {
        match vis.keyword() {
            Some(vis) => format!("{vis} {}", self.words()),
            None => self.words().to_string(),
        }
    }
}

/// How widely an item is declared visible. `pub(crate)`, `pub(super)`, and
/// `pub(in path)` are not `pub`, and the survey keeps them apart: a chart may
/// read visibility, and none of them may guess it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Vis {
    /// `pub` — visible outside its own crate.
    Pub,
    /// `pub(crate)`, `pub(super)`, `pub(in path)`.
    Crate,
    /// No `pub` at all.
    Private,
}

impl Vis {
    /// The visibility as rust writes it. Private declares nothing, so it has
    /// no keyword: rust writes nothing at all, and so does the interface.
    pub(crate) fn keyword(self) -> Option<&'static str> {
        match self {
            Vis::Pub => Some("pub"),
            Vis::Crate => Some("pub(crate)"),
            Vis::Private => None,
        }
    }
}

/// One source file in the workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FileInfo {
    /// Stable within one analysis: index into [`CodeGraph::files`].
    pub(crate) id: u32,
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
    pub(crate) id: u32,
    /// The file that declared it at the base, relative to the workspace root.
    /// The file itself may be gone too.
    pub(crate) path: String,
    /// The cargo package that owns that file, as [`FileInfo::krate`].
    pub(crate) krate: String,
    pub(crate) name: String,
    pub(crate) kind: ItemKind,
    pub(crate) vis: Vis,
    /// 1-based line in the base edition of the file.
    pub(crate) line: u32,
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
    /// Index into that file's own items, in source order.
    pub(crate) local: u32,
    /// Display name, without any section prefix.
    pub(crate) name: String,
    /// The label this item selects by in a URL: `Type::method` inside a
    /// section, the plain name otherwise.
    pub(crate) label: String,
    pub(crate) kind: ItemKind,
    pub(crate) vis: Vis,
    pub(crate) line: u32,
    /// Semantic container: the type a method or associated item belongs to,
    /// resolved through the impl's self type even when the impl sits in
    /// another file. `None` for items the file itself contains.
    pub(crate) parent: Option<u32>,
    /// Item-level references reaching it from other files: how much of the
    /// workspace leans on it.
    pub(crate) fan_in: u32,
    /// Hand-written trait impls of this type, as their headers are written
    /// (`impl Clone for Vis`), gathered from every impl anywhere in the
    /// workspace. Derives are not here: they stand in the type's own source,
    /// and a derive is not code anyone wrote.
    pub(crate) impls: Vec<String>,
    /// A struct's or union's fields — or a free function's parameters —
    /// quoted from source in declaration order, each carrying the visibility
    /// it declares for itself. The data chart quotes them all — what a field
    /// or a parameter reaches is on the holds edges. Empty for everything
    /// else.
    pub(crate) field_rows: Vec<DeclRow>,
    /// An enum's variants as written, in source order — name, payload types,
    /// and discriminant included. Empty for everything that is not an enum.
    pub(crate) variants: Vec<String>,
    /// A static's declared type or a free function's return type, as written.
    /// Empty for everything else, and for a function that returns nothing.
    pub(crate) ty: String,
    /// The methods declared for this type anywhere in the workspace, in the
    /// survey's order — the second band of its block. Every one of them is
    /// here, whatever its visibility: which ones are rows is a door, and a
    /// door is the client's to set. Empty for everything that is not a type.
    pub(crate) method_rows: Vec<MethodRow>,
    /// How this declaration differs from the diff base.
    pub(crate) delta: Delta,
    /// Fields added since the base: indexes into `field_rows`.
    pub(crate) fields_added: Vec<u32>,
    /// Fields the base had that the working copy dropped, quoted from the
    /// base: (insert before this index of `field_rows`, the row).
    pub(crate) fields_removed: Vec<(u32, DeclRow)>,
    /// Variants added since the base: indexes into `variants`.
    pub(crate) variants_added: Vec<u32>,
    /// Variants the base had that the working copy dropped, quoted from the
    /// base: (insert before this index of `variants`, the variant as written).
    pub(crate) variants_removed: Vec<(u32, String)>,
    /// Methods added since the base: indexes into `method_rows`.
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

/// A reference between two items, aggregated per pair. Endpoints carry their
/// file, so the client can lift an edge to whatever is visible at the current
/// fold state without fetching item detail; a `None` item is a reference to a
/// file as a whole (a `use` of its module).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ItemEdge {
    pub(crate) from_file: u32,
    pub(crate) from: Option<u32>,
    pub(crate) to_file: u32,
    pub(crate) to: Option<u32>,
    pub(crate) count: u32,
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

/// A reference between two items of one file, at mark precision, summed. The
/// cross-file [`ItemEdge`]s carry their endpoints' files because either end
/// may be a whole file; both ends of one of these is a mark by construction,
/// so it carries nothing else. The chart needs them apart from the cross-file
/// list because which file a reference was written in says nothing about
/// whether one type's code leans on another.
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
    /// Cross-file references at item precision, aggregated per pair.
    pub(crate) item_edges: Vec<ItemEdge>,
    /// The references the pair above cannot carry: two items of one file,
    /// both ends a mark. Together the two lists are every resolved reference
    /// the survey placed at item precision.
    pub(crate) local_refs: Vec<MarkRef>,
    /// Which type holds which, and through what wrapper — the data
    /// altitude's structure. Every surveyed type is here, private ones
    /// included. Edges carrying a [`HoldEvent`] are the structural diff's:
    /// `Removed` ones are re-drawn from the base edition.
    pub(crate) holds: Vec<HoldEdge>,
    /// Types, statics, and free functions the base had that the working copy
    /// dropped. Their ids continue after `items`, so `holds` can land on them.
    pub(crate) ghosts: Vec<GhostMark>,
    /// Names the survey could not resolve (type-inference limits). They are
    /// not on the chart; the words on the plate must say so.
    pub(crate) unresolved: u32,
    /// What the survey could not read about **references**, in plain words —
    /// the limits of the dashed uses ink.
    pub(crate) notes: Vec<String>,
    /// What the survey could not read about the **holds walk**, in plain words
    /// — the data chart's limits. Kept apart from `notes` so each cartouche's
    /// "what the survey cannot read" fold states the limits of the ink its own
    /// chart draws, and never paraphrases the survey in prose of its own.
    pub(crate) walk_notes: Vec<String>,
}
