//! The data walk: which types hold which, and through what.
//!
//! The code survey answers where the items are and who names whom; this
//! answers the altitude below it — what state exists, and what the boundary
//! of sharing is. In rust that answer is statically readable, because
//! ownership is written in the types: `Arc<Mutex<T>>` says shared mutable
//! state in the signature, `&'a T` says view. Every surveyed struct, enum,
//! union, and static has its fields' semantic types walked, and every
//! workspace type the walk reaches becomes a holding edge.
//!
//! Nothing here is guessed. A wrapper is a wrapper because of its name *and*
//! the crate that defines it, so a workspace type called `Signal` stays a
//! plain type; a field whose walk reaches no workspace type is counted, not
//! invented. The walk runs on the survey's already-loaded database and
//! reuses its def → mark resolution, so an edge lands on exactly the mark the
//! code altitude engraved.

use std::collections::HashMap;

use ra_ap_hir::{Adt, ModuleDef, Mutability, Semantics, Type};
use ra_ap_ide_db::RootDatabase;
use ra_ap_ide_db::base_db::EditionedFileId;
use ra_ap_syntax::ast::HasName;
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, TextRange, ast};

use crate::api::{HoldEdge, HoldKind, ItemKind};

/// One item the walk starts from: a struct, enum, union, or static the
/// survey has already given a mark.
pub(super) struct Holder {
    /// Its [`crate::api::ItemMark::id`].
    pub mark: u32,
    pub kind: ItemKind,
    /// Index into the survey's file list.
    pub file: u32,
    /// The item's own source range — how the survey names a syntax node.
    pub range: TextRange,
}

/// What the walk found. Everything but `holds` is indexed by mark id, so the
/// survey can lift it straight onto the [`crate::api::ItemMark`]s.
pub(super) struct DataWalk {
    /// Holding edges, aggregated per (from, to, kind, via) and sorted.
    pub holds: Vec<HoldEdge>,
    /// A struct's or union's fields, quoted in declaration order:
    /// (name as written, declared type as written).
    pub field_rows: Vec<Vec<(String, String)>>,
    /// An enum's variants as written — name, payload, discriminant.
    pub variants: Vec<Vec<String>>,
    /// A static's declared type, as written.
    pub ty: Vec<String>,
}

/// Where a wrapper has to be defined to count as one. A type the reviewer
/// wrote is never a wrapper, whatever it happens to be called.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Home {
    /// `std`, `core`, or `alloc`.
    Std,
    /// Any crate whose name starts with `dioxus`.
    Dioxus,
}

/// The wrapper table: the generic types the walk reads as something other
/// than plain ownership, matched on the type's own name and on the crate
/// that defines it. The `via` word an edge carries is the name in the first
/// column.
///
/// References and `dyn Trait` are the two wrappers not on this table,
/// because they are not types: the walk reads a reference as `Borrows` via
/// `&` or `&mut`, and `dyn Trait` as `Dyn` via `dyn`, landing on the trait.
///
/// Everything absent from all of that — `Vec`, `Option`, `Box`, `HashMap`,
/// `Result`, `Mutex`, `RwLock`, `RefCell`, `Cell`, `OnceCell`, `LazyLock`,
/// tuples, arrays, slices, and every unknown external type — is transparent:
/// the walk passes through into its arguments and the hold stays plain.
/// Interior mutability without a shared handle is still ownership; sharing
/// needs a shared handle. The legend on `/data` quotes this table, so the
/// two must agree.
const WRAPPERS: &[(&str, Home, HoldKind)] = &[
    // A shared handle: the state behind it has more than one possible reader.
    ("Arc", Home::Std, HoldKind::Shares),
    ("Rc", Home::Std, HoldKind::Shares),
    ("Weak", Home::Std, HoldKind::Shares),
    // A dioxus signal is Copy-shared runtime storage; holding one is holding
    // a handle to state the runtime owns.
    ("Signal", Home::Dioxus, HoldKind::Shares),
    ("GlobalSignal", Home::Dioxus, HoldKind::Shares),
    ("ReadSignal", Home::Dioxus, HoldKind::Shares),
    ("Memo", Home::Dioxus, HoldKind::Shares),
    ("Resource", Home::Dioxus, HoldKind::Shares),
];

impl Home {
    fn matches(self, krate: &str) -> bool {
        match self {
            Home::Std => matches!(krate, "std" | "core" | "alloc"),
            Home::Dioxus => krate.starts_with("dioxus"),
        }
    }
}

/// How loudly a wrapper speaks. An edge takes the strongest wrapper met
/// anywhere on the path: `Arc<Vec<&dyn Trait>>` shares, whatever else it
/// passed through on the way.
fn rank(kind: HoldKind) -> u8 {
    match kind {
        HoldKind::Owns => 0,
        HoldKind::Borrows => 1,
        HoldKind::Dyn => 2,
        HoldKind::Shares => 3,
    }
}

/// The louder of two wrappers. A tie keeps the one already on the path, so
/// the word written on the edge is the outermost of its rank.
fn stronger(
    have: (HoldKind, &'static str),
    met: (HoldKind, &'static str),
) -> (HoldKind, &'static str) {
    if rank(met.0) > rank(have.0) {
        met
    } else {
        have
    }
}

/// How deep the walk follows type arguments. A field type nested past this
/// is pathological rather than data.
const MAX_DEPTH: usize = 16;

/// The edges under construction: (from, to, kind, via) → the fields that draw
/// it, in source order.
type Edges = HashMap<(u32, u32, HoldKind, String), Vec<(String, String)>>;

/// Walk every holder's fields and aggregate what they hold. `mark_of_def`
/// is the survey's own def → mark resolution, already filtered to the marks
/// a data edge may land on; `marks` is how many marks the survey engraved.
pub(super) fn walk<'db>(
    sema: &Semantics<'db, RootDatabase>,
    db: &'db RootDatabase,
    files: &[EditionedFileId],
    holders: &[Holder],
    marks: usize,
    mark_of_def: &dyn Fn(ModuleDef) -> Option<u32>,
) -> DataWalk {
    let mut field_rows: Vec<Vec<(String, String)>> = vec![Vec::new(); marks];
    let mut variants: Vec<Vec<String>> = vec![Vec::new(); marks];
    let mut ty: Vec<String> = vec![String::new(); marks];
    let mut acc: Edges = HashMap::new();

    // Holders arrive in (file, source) order, so each file is parsed once.
    let mut at = 0usize;
    while at < holders.len() {
        let file = holders[at].file;
        let end = at + holders[at..].partition_point(|h| h.file == file);
        let Some(&efid) = files.get(file as usize) else {
            at = end;
            continue;
        };
        let source = sema.parse(efid);
        // The survey knows its items by source range, not by node; index the
        // file's declarations so each holder finds its own syntax back.
        let nodes: HashMap<TextRange, SyntaxNode> = source
            .syntax()
            .descendants()
            .filter(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::STRUCT | SyntaxKind::ENUM | SyntaxKind::UNION | SyntaxKind::STATIC
                )
            })
            .map(|n| (n.text_range(), n))
            .collect();

        for holder in &holders[at..end] {
            let Some(node) = nodes.get(&holder.range) else {
                continue;
            };
            let mark = holder.mark as usize;
            match holder.kind {
                ItemKind::Struct | ItemKind::Union => {
                    let list = ast::Struct::cast(node.clone())
                        .and_then(|s| s.field_list())
                        .or_else(|| {
                            ast::Union::cast(node.clone())
                                .and_then(|u| u.record_field_list())
                                .map(ast::FieldList::RecordFieldList)
                        });
                    for (name, decl, field_ty) in fields_of(sema, db, list) {
                        field_edges(
                            db,
                            mark_of_def,
                            holder.mark,
                            name.clone(),
                            decl.clone(),
                            &field_ty,
                            &mut acc,
                        );
                        field_rows[mark].push((name, decl));
                    }
                }
                ItemKind::Enum => {
                    let Some(e) = ast::Enum::cast(node.clone()) else {
                        continue;
                    };
                    for variant in e.variant_list().into_iter().flat_map(|l| l.variants()) {
                        let Some(name) = variant.name().map(|n| n.text().to_string()) else {
                            continue;
                        };
                        variants[mark].push(variant_text(&variant, &name));
                        // A variant's payload is held by the enum, and a
                        // payload field has no name a reader would know: the
                        // variant is what they read it by.
                        for (_, decl, field_ty) in fields_of(sema, db, variant.field_list()) {
                            field_edges(
                                db,
                                mark_of_def,
                                holder.mark,
                                name.clone(),
                                decl,
                                &field_ty,
                                &mut acc,
                            );
                        }
                    }
                }
                ItemKind::Static => {
                    let Some(s) = ast::Static::cast(node.clone()) else {
                        continue;
                    };
                    ty[mark] = type_text(s.ty());
                    let name = s.name().map(|n| n.text().to_string()).unwrap_or_default();
                    let Some(def) = sema.to_def(&s) else {
                        continue;
                    };
                    let decl = ty[mark].clone();
                    field_edges(
                        db,
                        mark_of_def,
                        holder.mark,
                        name,
                        decl,
                        &def.ty(db),
                        &mut acc,
                    );
                }
                _ => {}
            }
        }
        at = end;
    }

    let mut holds: Vec<HoldEdge> = acc
        .into_iter()
        .map(|((from, to, kind, via), fields)| HoldEdge {
            from,
            to,
            kind,
            via,
            fields,
        })
        .collect();
    holds.sort_by(|a, b| {
        (a.from, a.to, rank(a.kind), &a.via).cmp(&(b.from, b.to, rank(b.kind), &b.via))
    });

    DataWalk {
        holds,
        field_rows,
        variants,
        ty,
    }
}

/// One variant as its source writes it: the name, its payload types, and its
/// discriminant, with whitespace runs collapsed so a record variant broken
/// across lines still reads as one row. Nothing is reconstructed.
fn variant_text(variant: &ast::Variant, name: &str) -> String {
    let collapse = |node: &SyntaxNode| {
        node.text()
            .to_string()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let payload = match variant.field_list() {
        Some(ast::FieldList::TupleFieldList(list)) => collapse(list.syntax()),
        Some(ast::FieldList::RecordFieldList(list)) => format!(" {}", collapse(list.syntax())),
        None => String::new(),
    };
    match variant.const_arg() {
        Some(arg) => format!("{name}{payload} = {}", collapse(arg.syntax())),
        None => format!("{name}{payload}"),
    }
}

/// Walk one field's type and file every edge it draws.
fn field_edges(
    db: &RootDatabase,
    mark_of_def: &dyn Fn(ModuleDef) -> Option<u32>,
    from: u32,
    name: String,
    decl: String,
    ty: &Type<'_>,
    acc: &mut Edges,
) {
    let mut found: Vec<(u32, HoldKind, &'static str)> = Vec::new();
    walk_ty(db, mark_of_def, ty, (HoldKind::Owns, ""), 0, &mut found);
    if found.is_empty() {
        return;
    }
    // One field can reach the same type by more than one route — a
    // `GlobalSignal<T>` resolves to `Global<Signal<T>, T>` and so meets `T`
    // both behind the signal and beside it. One field says one thing about
    // one type: the strongest reading it found.
    let mut strongest: Vec<(u32, HoldKind, &'static str)> = Vec::new();
    for (to, kind, via) in found {
        match strongest.iter_mut().find(|(seen, ..)| *seen == to) {
            Some(row) if rank(kind) > rank(row.1) => *row = (to, kind, via),
            Some(_) => {}
            None => strongest.push((to, kind, via)),
        }
    }
    for (to, kind, via) in strongest {
        let row = acc.entry((from, to, kind, via.to_string())).or_default();
        // One field says a thing once, however many times the walk met it:
        // `(Foo, Foo)` is one row, not two.
        if !row.iter().any(|(n, d)| n == &name && d == &decl) {
            row.push((name.clone(), decl.clone()));
        }
    }
}

/// Walk one semantic type, collecting every workspace mark it reaches and the
/// strongest wrapper standing between the holder and each one. See
/// [`WRAPPERS`] for what counts as a wrapper and what is transparent.
fn walk_ty(
    db: &RootDatabase,
    mark_of_def: &dyn Fn(ModuleDef) -> Option<u32>,
    ty: &Type<'_>,
    strongest: (HoldKind, &'static str),
    depth: usize,
    out: &mut Vec<(u32, HoldKind, &'static str)>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    // A reference is a view on state something else owns.
    if let Some((inner, mutability)) = ty.as_reference() {
        let word = match mutability {
            Mutability::Mut => "&mut",
            Mutability::Shared => "&",
        };
        let s = stronger(strongest, (HoldKind::Borrows, word));
        walk_ty(db, mark_of_def, &inner, s, depth + 1, out);
        return;
    }
    // A raw pointer names its pointee and says nothing about ownership.
    if let Some((inner, _)) = ty.as_raw_ptr() {
        walk_ty(db, mark_of_def, &inner, strongest, depth + 1, out);
        return;
    }
    // `dyn Trait` holds a trait, not a type: the trait is the target, and it
    // has no arguments of its own to follow.
    if let Some(trait_) = ty.as_dyn_trait() {
        let (kind, via) = stronger(strongest, (HoldKind::Dyn, "dyn"));
        if let Some(mark) = mark_of_def(ModuleDef::Trait(trait_)) {
            out.push((mark, kind, via));
        }
        return;
    }
    let mut strongest = strongest;
    if let Some(adt) = ty.as_adt() {
        match wrapper(db, adt) {
            Some(met) => strongest = stronger(strongest, met),
            // Only a workspace type is a mark; an external type is a wrapper
            // or an atom, and either way the walk passes through it.
            None => {
                if let Some(mark) = mark_of_def(ModuleDef::Adt(adt)) {
                    out.push((mark, strongest.0, strongest.1));
                }
            }
        }
    }
    // A slice or an array holds its element; a tuple and every generic type
    // hand over their arguments.
    if let Some(element) = ty.as_slice() {
        walk_ty(db, mark_of_def, &element, strongest, depth + 1, out);
    } else if let Some((element, _)) = ty.as_array(db) {
        walk_ty(db, mark_of_def, &element, strongest, depth + 1, out);
    } else {
        for arg in ty.type_arguments() {
            walk_ty(db, mark_of_def, &arg, strongest, depth + 1, out);
        }
    }
}

/// The wrapper this type is, if [`WRAPPERS`] knows it. A type defined in the
/// workspace is never a wrapper: a local `Signal` is the reviewer's own type
/// and reading it as dioxus storage would be a lie about their code.
fn wrapper(db: &RootDatabase, adt: Adt) -> Option<(HoldKind, &'static str)> {
    let krate = adt.module(db).krate(db);
    if krate.origin(db).is_local() {
        return None;
    }
    let home = krate.display_name(db)?.to_string();
    let name = adt.name(db);
    let name = name.as_str();
    WRAPPERS
        .iter()
        .find(|&&(word, wanted, _)| word == name && wanted.matches(&home))
        .map(|&(word, _, kind)| (kind, word))
}

/// One field list's fields in source order: the name as written (a tuple
/// field's index for a tuple), the declared type as written, and the type
/// rust-analyzer resolved for it — which is where type aliases resolve and
/// generic parameters stay holes.
fn fields_of<'db>(
    sema: &Semantics<'db, RootDatabase>,
    db: &'db RootDatabase,
    list: Option<ast::FieldList>,
) -> Vec<(String, String, Type<'db>)> {
    let mut out = Vec::new();
    match list {
        Some(ast::FieldList::RecordFieldList(fields)) => {
            for field in fields.fields() {
                let Some(def) = sema.to_def(&field) else {
                    continue;
                };
                let name = field
                    .name()
                    .map(|n| n.text().to_string())
                    .unwrap_or_default();
                out.push((name, type_text(field.ty()), def.ty(db)));
            }
        }
        Some(ast::FieldList::TupleFieldList(fields)) => {
            for (index, field) in fields.fields().enumerate() {
                let Some(def) = sema.to_def(&field) else {
                    continue;
                };
                out.push((index.to_string(), type_text(field.ty()), def.ty(db)));
            }
        }
        None => {}
    }
    out
}

/// A declared type as the source writes it, with runs of whitespace collapsed
/// to one space — the only edit, so a type broken across lines still reads as
/// one row. Nothing is reconstructed, abbreviated, or resolved: the reader
/// sees the words that are in the file.
fn type_text(ty: Option<ast::Type>) -> String {
    let Some(ty) = ty else {
        return String::new();
    };
    ty.syntax()
        .text()
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
