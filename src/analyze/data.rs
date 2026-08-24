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
//! A free function is walked the same way, because a pub fn is a contract
//! just as a pub struct is: its parameters and its return type are declared
//! types, so they are walked exactly as a field declaration is. Only the
//! signature — a body names things at the code altitude, and that is where
//! those names stay.
//!
//! A method is walked as one row of its *type's* contract: the edges leave the
//! type, filed under the method's own name, and the row is the signature as
//! written. A method never becomes a holder of its own, because it is not a
//! landmark — it is a clause of the block its impl names.
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

use crate::api::{HoldEdge, HoldKind, ItemKind, MethodRow, Vis};

/// One item the walk starts from: a struct, enum, union, static, or free
/// function the survey has already given a mark — or one method, which starts
/// from the type its impl names.
pub(super) struct Holder {
    /// The [`crate::api::ItemMark::id`] the rows and the edges belong to. For
    /// a method that is its *type's* mark: a method is a clause of the type's
    /// contract, never a landmark of its own.
    pub(crate) mark: u32,
    pub(crate) kind: ItemKind,
    /// Index into the survey's file list. An impl block can sit in a file the
    /// type does not, so this is the method's own file, not the type's.
    pub(crate) file: u32,
    /// The item's own source range — how the survey names a syntax node.
    pub(crate) range: TextRange,
    /// Set when this holder is one method of `mark`.
    pub(crate) method: Option<MethodOf>,
}

/// What the survey knows about a method that its own source does not say: who
/// declares it, and whether it arrived through a trait.
pub(super) struct MethodOf {
    /// The method's own mark.
    pub(crate) mark: u32,
    pub(crate) vis: Vis,
    /// Declared inside `impl Trait for Type`.
    pub(crate) via_trait: bool,
}

/// What the walk found. Everything but `holds` is indexed by mark id, so the
/// survey can lift it straight onto the [`crate::api::ItemMark`]s.
pub(super) struct DataWalk {
    /// Holding edges, aggregated per (from, to, kind, via, rows) and sorted.
    pub(crate) holds: Vec<HoldEdge>,
    /// A struct's or union's fields — or a free function's parameters —
    /// quoted in declaration order: (name as written, declared type as
    /// written).
    pub(crate) field_rows: Vec<Vec<(String, String)>>,
    /// An enum's variants as written — name, payload, discriminant.
    pub(crate) variants: Vec<Vec<String>>,
    /// A static's declared type or a free function's return type, as written.
    pub(crate) ty: Vec<String>,
    /// A type's methods, in the survey's order, quoted as written.
    pub(crate) method_rows: Vec<Vec<MethodRow>>,
}

/// Where a wrapper has to be defined to count as one. A type the reviewer
/// wrote is never a wrapper, whatever it happens to be called.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WrapperHome {
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
/// needs a shared handle. The legend on `/surface` quotes this table, so the
/// two must agree.
const WRAPPERS: &[(&str, WrapperHome, HoldKind)] = &[
    // A shared handle: the state behind it has more than one possible reader.
    ("Arc", WrapperHome::Std, HoldKind::Shares),
    ("Rc", WrapperHome::Std, HoldKind::Shares),
    ("Weak", WrapperHome::Std, HoldKind::Shares),
    // A dioxus signal is Copy-shared runtime storage; holding one is holding
    // a handle to state the runtime owns.
    ("Signal", WrapperHome::Dioxus, HoldKind::Shares),
    ("GlobalSignal", WrapperHome::Dioxus, HoldKind::Shares),
    ("ReadSignal", WrapperHome::Dioxus, HoldKind::Shares),
    ("Memo", WrapperHome::Dioxus, HoldKind::Shares),
    ("Resource", WrapperHome::Dioxus, HoldKind::Shares),
];

impl WrapperHome {
    fn matches(self, krate: &str) -> bool {
        match self {
            WrapperHome::Std => matches!(krate, "std" | "core" | "alloc"),
            WrapperHome::Dioxus => krate.starts_with("dioxus"),
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
        // Never met on a type walk: an impl block draws it, not a row.
        HoldKind::Implements => 4,
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

/// The tail of an edge under construction: the mark it leaves, and whether a
/// method row is what draws it.
type Tail = (u32, bool);

/// The edges under construction: (from, to, kind, via, drawn by a method) →
/// the rows that draw it, in source order. The last key is what keeps "this
/// type keeps one of those" and "this type's API names one" from aggregating
/// into a single line neither reading can be recovered from.
pub(super) type Edges = HashMap<(u32, u32, HoldKind, String, bool), Vec<(String, String)>>;

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
    let mut method_rows: Vec<Vec<MethodRow>> = vec![Vec::new(); marks];
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
                    SyntaxKind::STRUCT
                        | SyntaxKind::ENUM
                        | SyntaxKind::UNION
                        | SyntaxKind::STATIC
                        | SyntaxKind::TRAIT
                        | SyntaxKind::FN
                        | SyntaxKind::TYPE_ALIAS
                        | SyntaxKind::CONST
                )
            })
            .map(|n| (n.text_range(), n))
            .collect();

        for holder in &holders[at..end] {
            let Some(node) = nodes.get(&holder.range) else {
                continue;
            };
            let mark = holder.mark as usize;
            let tail: Tail = (holder.mark, holder.method.is_some());
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
                            tail,
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
                                tail,
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
                    field_edges(db, mark_of_def, tail, name, decl, &def.ty(db), &mut acc);
                }
                // A trait's associated type or const is a clause of its
                // contract with no signature to walk: quoted as a row, and an
                // associated const's declared type walked the way a static's
                // is. What an associated type's bounds name is not read.
                //
                // A *free* one is a mark instead — a contract of one line —
                // and quotes what it names in the slot a static's declared
                // type uses: a const its type, an alias its target.
                ItemKind::TypeAlias | ItemKind::Const if holder.method.is_none() => {
                    let name = ast::AnyHasName::cast(node.clone())
                        .and_then(|n| n.name())
                        .map(|n| n.text().to_string())
                        .unwrap_or_default();
                    if let Some(c) = ast::Const::cast(node.clone()) {
                        ty[mark] = type_text(c.ty());
                        let Some(def) = sema.to_def(&c) else {
                            continue;
                        };
                        let decl = ty[mark].clone();
                        field_edges(db, mark_of_def, tail, name, decl, &def.ty(db), &mut acc);
                    } else if let Some(a) = ast::TypeAlias::cast(node.clone()) {
                        // The target as written, and the walk run over what it
                        // resolves to: an alias is one name standing in front
                        // of another, and the edge points at the other.
                        ty[mark] = type_text(a.ty());
                        let Some(def) = sema.to_def(&a) else {
                            continue;
                        };
                        let decl = ty[mark].clone();
                        field_edges(db, mark_of_def, tail, name, decl, &def.ty(db), &mut acc);
                    }
                }
                ItemKind::TypeAlias | ItemKind::Const => {
                    let Some(method) = &holder.method else {
                        continue;
                    };
                    let name = ast::AnyHasName::cast(node.clone())
                        .and_then(|n| n.name())
                        .map(|n| n.text().to_string())
                        .unwrap_or_default();
                    let sig = decl_text(node);
                    if let Some(c) = ast::Const::cast(node.clone())
                        && let Some(def) = sema.to_def(&c)
                    {
                        field_edges(
                            db,
                            mark_of_def,
                            tail,
                            name.clone(),
                            sig.clone(),
                            &def.ty(db),
                            &mut acc,
                        );
                    }
                    method_rows[mark].push(MethodRow {
                        name,
                        sig,
                        vis: method.vis,
                        via_trait: method.via_trait,
                        mark: method.mark,
                    });
                }
                ItemKind::Fn => {
                    let Some(f) = ast::Fn::cast(node.clone()) else {
                        continue;
                    };
                    let quoted: Vec<ast::Param> = f
                        .param_list()
                        .into_iter()
                        .flat_map(|l| l.params())
                        .collect();
                    // The rows are a quotation and never wait on inference. A
                    // function an attribute macro rewrote — `#[server]`,
                    // `#[component]` — resolves to the expansion or to
                    // nothing at all, and the reader's own file is what the
                    // block quotes either way. Where the resolved signature
                    // does not line up with the written one, it is about
                    // another function: quote the rows and draw no edges from
                    // it, and let the written types answer instead.
                    let def = sema
                        .to_def(&f)
                        .filter(|def| def.params_without_self(db).len() == quoted.len());
                    // An `async fn` returns its body's type wrapped in a
                    // future rust-analyzer synthesized; the reader wrote the
                    // inner one, so that is the one the walk follows.
                    let returns = |db: &'db RootDatabase| {
                        def.map(|def| def.async_ret_type(db).unwrap_or_else(|| def.ret_type(db)))
                            // No def to ask: the written type still resolves
                            // in the file's own scope, which is enough to land
                            // an edge on a mark.
                            .or_else(|| sema.resolve_type(&f.ret_type().and_then(|r| r.ty())?))
                    };
                    let name = f.name().map(|n| n.text().to_string()).unwrap_or_default();
                    // A method is one row of its type's contract, so the whole
                    // signature is the quotation and the method's own name is
                    // what every type it names is filed under: the row is what
                    // a reader points at, not the parameter inside it.
                    if let Some(method) = &holder.method {
                        let sig = signature_text(&f);
                        let walk_row = |ty: &Type<'_>, acc: &mut Edges| {
                            field_edges(db, mark_of_def, tail, name.clone(), sig.clone(), ty, acc);
                        };
                        for param in def.iter().flat_map(|def| def.params_without_self(db)) {
                            walk_row(param.ty(), &mut acc);
                        }
                        for param in quoted.iter().filter(|_| def.is_none()) {
                            if let Some(ty) = param.ty().and_then(|ty| sema.resolve_type(&ty)) {
                                walk_row(&ty, &mut acc);
                            }
                        }
                        if let Some(ret) = returns(db).filter(|ret| !ret.is_unit()) {
                            walk_row(&ret, &mut acc);
                        }
                        method_rows[mark].push(MethodRow {
                            name,
                            sig,
                            vis: method.vis,
                            via_trait: method.via_trait,
                            mark: method.mark,
                        });
                        continue;
                    }
                    // A free function's block is its signature laid out: the
                    // written parameters and the resolved ones stand in the
                    // same order, and a free function has no `self` to throw
                    // the count off — so each row keeps the words the source
                    // wrote while the edge follows the resolved type.
                    let resolved = def.map(|def| def.params_without_self(db));
                    for (at, source) in quoted.iter().enumerate() {
                        let name = pat_text(source.pat());
                        let decl = type_text(source.ty());
                        let ty = match resolved.as_ref().and_then(|params| params.get(at)) {
                            Some(param) => Some(param.ty().clone()),
                            None => source.ty().and_then(|ty| sema.resolve_type(&ty)),
                        };
                        if let Some(ty) = ty {
                            field_edges(
                                db,
                                mark_of_def,
                                tail,
                                name.clone(),
                                decl.clone(),
                                &ty,
                                &mut acc,
                            );
                        }
                        field_rows[mark].push((name, decl));
                    }
                    // The return type is the signature's own row, filed under
                    // the function's name the way a static's type is under the
                    // static's. A function that returns nothing says nothing.
                    let ret_text = type_text(f.ret_type().and_then(|r| r.ty()));
                    if ret_text.is_empty() || ret_text == "()" {
                        continue;
                    }
                    ty[mark] = ret_text.clone();
                    if let Some(ret) = returns(db) {
                        field_edges(db, mark_of_def, tail, name, ret_text, &ret, &mut acc);
                    }
                }
                _ => {}
            }
        }
        at = end;
    }

    let mut holds: Vec<HoldEdge> = acc
        .into_iter()
        .map(|((from, to, kind, via, from_method), fields)| HoldEdge {
            from,
            to,
            kind,
            via,
            fields,
            from_method,
            event: None,
        })
        .collect();
    holds.sort_by(|a, b| {
        (a.from, a.to, rank(a.kind), &a.via, a.from_method).cmp(&(
            b.from,
            b.to,
            rank(b.kind),
            &b.via,
            b.from_method,
        ))
    });

    DataWalk {
        holds,
        field_rows,
        variants,
        ty,
        method_rows,
    }
}

/// One variant as its source writes it: the name, its payload types, and its
/// discriminant, with whitespace runs collapsed so a record variant broken
/// across lines still reads as one row. Nothing is reconstructed.
pub(super) fn variant_text(variant: &ast::Variant, name: &str) -> String {
    // A payload written across lines can hold doc comments between its
    // fields; they are prose about the declaration, not the declaration, and
    // a row that inlines one reads as a corrupted quotation. Comments drop,
    // whitespace runs collapse to one space.
    let collapse = |node: &SyntaxNode| {
        let mut out = String::new();
        for token in node
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
        {
            match token.kind() {
                SyntaxKind::COMMENT => {}
                SyntaxKind::WHITESPACE => {
                    if !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                _ => out.push_str(token.text()),
            }
        }
        out.trim().to_string()
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

/// Walk one row's type and file every edge it draws.
fn field_edges(
    db: &RootDatabase,
    mark_of_def: &dyn Fn(ModuleDef) -> Option<u32>,
    from: Tail,
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
        let row = acc
            .entry((from.0, to, kind, via.to_string(), from.1))
            .or_default();
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
pub(super) fn type_text(ty: Option<ast::Type>) -> String {
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

/// One row's declaration as its source writes it: from the first keyword
/// through the return type and any where clause, with the doc comment, the
/// attributes, the body and the closing semicolon left where they are, and
/// runs of whitespace collapsed. A row is a quotation — the body is the
/// implementation, and this altitude is about the promise. A trait's
/// associated type or const quotes the same way, default and all.
pub(super) fn decl_text(node: &SyntaxNode) -> String {
    let mut out = String::new();
    for element in node.children_with_tokens() {
        match element.kind() {
            SyntaxKind::ATTR | SyntaxKind::COMMENT | SyntaxKind::SEMICOLON => continue,
            SyntaxKind::BLOCK_EXPR => break,
            _ => out.push_str(&element.to_string()),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One method's signature, the way [`decl_text`] writes any row.
pub(super) fn signature_text(f: &ast::Fn) -> String {
    decl_text(f.syntax())
}

/// A parameter's binding as the source writes it — `graph`, `mut at`, `_`.
/// A pattern is the name the reader knows the parameter by, so it is quoted
/// like every other row, never reduced to a position.
pub(super) fn pat_text(pat: Option<ast::Pat>) -> String {
    let Some(pat) = pat else {
        return String::new();
    };
    pat.syntax()
        .text()
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
