//! The structural diff: what this epoch added, removed, and rewrote,
//! declaration by declaration.
//!
//! The live survey is semantic; the base edition is read syntactically.
//! `file_at_base` fetches each changed file as the base wrote it and
//! `ra_ap_syntax` parses it — no name resolution, no type inference, no
//! second rust-analyzer run. The trade is stated, never hidden: added and
//! removed declarations and rows are exact (declarations match by kind and
//! name), while a *removed* relation's target is matched by name against the
//! types the chart knows, and what stays ambiguous is counted on the legend.
//! A full semantic survey of the base would slot in behind the same wire
//! model; this is the cheap edition, not the final word.
//!
//! Scope: structs, enums, unions, statics, and free functions — everything
//! that earns a block at the data altitude — plus the methods a type declares,
//! which are compared so a rewritten API reads apart from a rewritten shape.
//! Methods are compared only against the impls in the type's own file, because
//! that is the only base edition this pass fetches; a method written in an
//! impl block in another file is quoted and left out of the weave. The other
//! item kinds keep `Delta::Same` until the code altitude takes its pass.

use std::collections::{HashMap, HashSet};

use ra_ap_syntax::ast::{HasName, HasVisibility};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, ast};

use super::vcs::{Diff, file_at_base};
use crate::api::{
    CodeGraph, DeclRow, Delta, FileDetail, GhostMark, HoldEdge, HoldEvent, HoldKind, ImplEdge,
    ItemKind, ItemMark, Vis,
};

/// One declaration as the base edition wrote it.
struct BaseDecl {
    /// Inline-module path included (`tests::Sample`), matching the survey's
    /// own naming.
    name: String,
    kind: ItemKind,
    vis: Vis,
    /// 1-based line in the base edition.
    line: u32,
    /// The whole declaration, whitespace-collapsed, for the did-it-change
    /// comparison. Attributes and doc comments ride along, as they do in the
    /// live survey's byte range.
    text: String,
    field_rows: Vec<DeclRow>,
    variants: Vec<String>,
    ty: String,
    /// The methods the base's own impl blocks in *this file* wrote for it,
    /// quoted as (name, signature). Impls in other files are not read: the
    /// diff only fetches the base edition of the files that changed, so the
    /// live side is held to the same file when the two are compared.
    method_rows: Vec<(String, String)>,
}

/// Whether the base edition's node sits in test-only code — the same reading
/// the live survey makes, so the two editions leave out the same declarations
/// and the diff has nothing spurious to report.
fn skips_tests(node: &SyntaxNode) -> bool {
    !super::charts_tests() && node.ancestors().any(|up| super::code::test_only(&up))
}

/// Runs of whitespace collapsed to one space — the same edit `type_text` and
/// `variant_text` make, so base and live text compare on words, not layout.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 1-based line of a byte offset.
fn line_of(text: &str, offset: usize) -> u32 {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as u32
        + 1
}

/// A field list's rows, syntactically: the name as written (a tuple field's
/// is its index), the declared type as written, and the visibility the field
/// declares for itself — read the same way the live survey reads it, so the
/// two editions compare without a false change.
fn base_fields(list: Option<ast::FieldList>) -> Vec<DeclRow> {
    let mut out = Vec::new();
    match list {
        Some(ast::FieldList::RecordFieldList(fields)) => {
            for field in fields.fields() {
                let name = field
                    .name()
                    .map(|n| n.text().to_string())
                    .unwrap_or_default();
                out.push(DeclRow {
                    name,
                    ty: super::data::type_text(field.ty()),
                    vis: super::code::vis_kind(field.visibility()),
                });
            }
        }
        Some(ast::FieldList::TupleFieldList(fields)) => {
            for (index, field) in fields.fields().enumerate() {
                out.push(DeclRow {
                    name: index.to_string(),
                    ty: super::data::type_text(field.ty()),
                    vis: super::code::vis_kind(field.visibility()),
                });
            }
        }
        None => {}
    }
    out
}

/// The inline-module path standing over a node (`tests::`), or `None` when
/// the node is not on a plain module chain at all — a type declared inside a
/// function body, or a function inside an impl or a trait, has no mark of its
/// own in the live survey, so the base reads it the same way.
fn module_prefix(node: &SyntaxNode) -> Option<String> {
    let mut names: Vec<String> = Vec::new();
    for up in node.ancestors().skip(1) {
        match up.kind() {
            // An extern block holds no ground of its own: the live survey
            // shelves its items on the file, and so must the base.
            SyntaxKind::SOURCE_FILE
            | SyntaxKind::ITEM_LIST
            | SyntaxKind::EXTERN_BLOCK
            | SyntaxKind::EXTERN_ITEM_LIST => {}
            SyntaxKind::MODULE => {
                let name = ast::Module::cast(up)?.name()?.text().to_string();
                names.push(name);
            }
            _ => return None,
        }
    }
    names.reverse();
    Some(names.iter().map(|n| format!("{n}::")).collect())
}

/// A parameter list as the base wrote it: the binding as written and the
/// declared type as written — the rows a function's block quotes.
fn base_params(list: Option<ast::ParamList>) -> Vec<DeclRow> {
    list.into_iter()
        .flat_map(|list| list.params())
        .map(|p| DeclRow {
            name: super::data::pat_text(p.pat()),
            ty: super::data::type_text(p.ty()),
            // A parameter declares no visibility of its own, ever.
            vis: Vis::Private,
        })
        .collect()
}

/// The method bands one base edition writes, keyed the way a declaration is
/// named: the inline-module path of the impl block, then the self type as the
/// impl writes it. `impl Wire` and `impl Clone for Wire` both land on `Wire`,
/// in source order, which is how the live survey files them too.
fn base_methods(file: &ast::SourceFile) -> HashMap<String, Vec<(String, String)>> {
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for imp in file.syntax().descendants().filter_map(ast::Impl::cast) {
        if skips_tests(imp.syntax()) {
            continue;
        }
        let Some(prefix) = module_prefix(imp.syntax()) else {
            continue;
        };
        let Some(self_ty) = imp.self_ty() else {
            continue;
        };
        let key = format!(
            "{prefix}{}",
            collapsed(&self_ty.syntax().text().to_string())
        );
        let rows = out.entry(key).or_default();
        for method in imp
            .assoc_item_list()
            .into_iter()
            .flat_map(|list| list.assoc_items())
        {
            let ast::AssocItem::Fn(f) = method else {
                continue;
            };
            let Some(name) = f.name() else { continue };
            rows.push((name.text().to_string(), super::data::signature_text(&f)));
        }
    }
    out
}

impl BaseDecl {
    /// Every charted declaration in one base edition, in source order.
    fn scan(text: &str) -> Vec<Self> {
        let parse = ra_ap_syntax::SourceFile::parse(text, ra_ap_syntax::Edition::CURRENT);
        let mut methods = base_methods(&parse.tree());
        let mut out = Vec::new();
        for node in parse.tree().syntax().descendants() {
            if skips_tests(&node) {
                continue;
            }
            let kind = match node.kind() {
                SyntaxKind::STRUCT => ItemKind::Struct,
                SyntaxKind::ENUM => ItemKind::Enum,
                SyntaxKind::UNION => ItemKind::Union,
                SyntaxKind::STATIC => ItemKind::Static,
                SyntaxKind::TRAIT => ItemKind::Trait,
                SyntaxKind::FN => ItemKind::Fn,
                SyntaxKind::CONST => ItemKind::Const,
                SyntaxKind::TYPE_ALIAS => ItemKind::TypeAlias,
                _ => continue,
            };
            let Some(prefix) = module_prefix(&node) else {
                continue;
            };
            let range = node.text_range();
            let (name, vis, field_rows, variants, ty, own_rows) = match kind {
                ItemKind::Struct => {
                    let Some(s) = ast::Struct::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = s.name() else { continue };
                    let vis = super::code::vis_kind(s.visibility());
                    (
                        name.text().to_string(),
                        vis,
                        base_fields(s.field_list()),
                        Vec::new(),
                        String::new(),
                        Vec::new(),
                    )
                }
                ItemKind::Union => {
                    let Some(u) = ast::Union::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = u.name() else { continue };
                    let vis = super::code::vis_kind(u.visibility());
                    (
                        name.text().to_string(),
                        vis,
                        base_fields(u.record_field_list().map(ast::FieldList::RecordFieldList)),
                        Vec::new(),
                        String::new(),
                        Vec::new(),
                    )
                }
                ItemKind::Enum => {
                    let Some(e) = ast::Enum::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = e.name() else { continue };
                    let vis = super::code::vis_kind(e.visibility());
                    let variants = e
                        .variant_list()
                        .into_iter()
                        .flat_map(|l| l.variants())
                        .filter_map(|v| {
                            let name = v.name()?.text().to_string();
                            Some(super::data::variant_text(&v, &name))
                        })
                        .collect();
                    (
                        name.text().to_string(),
                        vis,
                        Vec::new(),
                        variants,
                        String::new(),
                        Vec::new(),
                    )
                }
                ItemKind::Static => {
                    let Some(s) = ast::Static::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = s.name() else { continue };
                    let vis = super::code::vis_kind(s.visibility());
                    (
                        name.text().to_string(),
                        vis,
                        Vec::new(),
                        Vec::new(),
                        super::data::type_text(s.ty()),
                        Vec::new(),
                    )
                }
                // A trait's band is written inside its own declaration, so the
                // base reads it where it stands — no impl block to find, and no
                // same-file limit on it.
                ItemKind::Trait => {
                    let Some(t) = ast::Trait::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = t.name() else { continue };
                    let rows = t
                        .assoc_item_list()
                        .into_iter()
                        .flat_map(|list| list.assoc_items())
                        .filter_map(|item| {
                            let (name, node) = match &item {
                                ast::AssocItem::Fn(f) => (f.name()?, f.syntax()),
                                ast::AssocItem::TypeAlias(a) => (a.name()?, a.syntax()),
                                ast::AssocItem::Const(c) => (c.name()?, c.syntax()),
                                _ => return None,
                            };
                            Some((name.text().to_string(), super::data::decl_text(node)))
                        })
                        .collect();
                    (
                        name.text().to_string(),
                        super::code::vis_kind(t.visibility()),
                        Vec::new(),
                        Vec::new(),
                        String::new(),
                        rows,
                    )
                }
                // A contract one line long: what it names stands in the slot a
                // static's declared type uses.
                ItemKind::Const => {
                    let Some(c) = ast::Const::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = c.name() else { continue };
                    (
                        name.text().to_string(),
                        super::code::vis_kind(c.visibility()),
                        Vec::new(),
                        Vec::new(),
                        super::data::type_text(c.ty()),
                        Vec::new(),
                    )
                }
                ItemKind::TypeAlias => {
                    let Some(a) = ast::TypeAlias::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = a.name() else { continue };
                    (
                        name.text().to_string(),
                        super::code::vis_kind(a.visibility()),
                        Vec::new(),
                        Vec::new(),
                        super::data::type_text(a.ty()),
                        Vec::new(),
                    )
                }
                ItemKind::Fn => {
                    let Some(f) = ast::Fn::cast(node.clone()) else {
                        continue;
                    };
                    let Some(name) = f.name() else { continue };
                    let vis = super::code::vis_kind(f.visibility());
                    // A function that hands nothing back quotes no return line,
                    // at the base as in the working copy.
                    let ret = super::data::type_text(f.ret_type().and_then(|r| r.ty()));
                    (
                        name.text().to_string(),
                        vis,
                        base_params(f.param_list()),
                        Vec::new(),
                        if ret == "()" { String::new() } else { ret },
                        Vec::new(),
                    )
                }
                _ => continue,
            };
            let name = format!("{prefix}{name}");
            let method_rows = match kind {
                ItemKind::Struct | ItemKind::Enum | ItemKind::Union => {
                    methods.remove(&name).unwrap_or_default()
                }
                // A trait's clauses stand inside its own declaration.
                ItemKind::Trait => own_rows,
                _ => Vec::new(),
            };
            out.push(BaseDecl {
                name,
                kind,
                vis,
                line: line_of(text, usize::from(range.start())),
                text: collapsed(&node.text().to_string()),
                field_rows,
                variants,
                ty,
                method_rows,
            });
        }
        out
    }
}

/// The last segment of a written path: `fmt::Display` is `Display`, which is
/// how the live survey names the mark it resolved.
fn bare_name(written: &str) -> String {
    let head = written.split(['<', '(', ' ']).next().unwrap_or(written);
    head.rsplit("::").next().unwrap_or(head).to_string()
}

/// The leading identifier of a variant's written form — how the live model
/// files a variant's edges too.
fn variant_name(written: &str) -> String {
    written
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Row diffs for one changed declaration: which live rows are new, and which
/// base rows the working copy dropped. Rows match by name; a same-named row
/// whose declared type changed is both — the base row a ghost, the live row
/// marked, the diff's own idiom for a changed line.
fn diff_rows(
    base: &[(String, String)],
    live: &[(String, String)],
) -> (Vec<u32>, Vec<(u32, String, String)>) {
    let live_of: HashMap<&str, usize> = live
        .iter()
        .enumerate()
        .map(|(i, (n, _))| (n.as_str(), i))
        .collect();
    let base_decl: HashMap<&str, &str> =
        base.iter().map(|(n, d)| (n.as_str(), d.as_str())).collect();
    let added: Vec<u32> = live
        .iter()
        .enumerate()
        .filter(|(_, (n, d))| base_decl.get(n.as_str()).copied() != Some(d.as_str()))
        .map(|(i, _)| i as u32)
        .collect();
    // Where each dropped base row renders: before its own name's live row if
    // the name survives (a changed type), else before the next base row that
    // does, else at the end.
    let mut removed: Vec<(u32, String, String)> = Vec::new();
    let mut before = vec![live.len() as u32; base.len()];
    let mut next = live.len() as u32;
    for (bi, (name, _)) in base.iter().enumerate().rev() {
        match live_of.get(name.as_str()) {
            Some(&li) => {
                next = li as u32;
                before[bi] = li as u32;
            }
            None => before[bi] = next,
        }
    }
    for (bi, (name, decl)) in base.iter().enumerate() {
        let survives = live_of
            .get(name.as_str())
            .is_some_and(|&li| live[li].1 == *decl);
        if !survives {
            removed.push((before[bi], name.clone(), decl.clone()));
        }
    }
    (added, removed)
}

/// The same diff for an enum's variants, matched by the variant's name.
fn diff_variants(base: &[String], live: &[String]) -> (Vec<u32>, Vec<(u32, String)>) {
    let as_rows = |list: &[String]| -> Vec<(String, String)> {
        list.iter()
            .map(|written| (variant_name(written), written.clone()))
            .collect()
    };
    let (added, removed) = diff_rows(&as_rows(base), &as_rows(live));
    (
        added,
        removed
            .into_iter()
            .map(|(before, _, written)| (before, written))
            .collect(),
    )
}

/// The wrapper table, syntactically: the words the live walk reads as
/// something other than plain ownership. A name the workspace itself defines
/// is never a wrapper (the name map wins), mirroring the live walk's rule.
const WRAPPER_WORDS: &[(&str, HoldKind)] = &[
    ("Arc", HoldKind::Shares),
    ("Rc", HoldKind::Shares),
    ("Weak", HoldKind::Shares),
    ("Signal", HoldKind::Shares),
    ("GlobalSignal", HoldKind::Shares),
    ("ReadSignal", HoldKind::Shares),
    ("Memo", HoldKind::Shares),
    ("Resource", HoldKind::Shares),
];

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

/// Read one dropped declaration text the way the live walk reads a semantic
/// type, by name: which known workspace types it names, and the strongest
/// wrapper word standing in the text. `skip_leading` drops the first
/// identifier (a variant's own name). Ambiguous names — more than one known
/// type by that name — are counted, never guessed.
fn name_walk(
    text: &str,
    skip_leading: bool,
    targets: &HashMap<String, Vec<u32>>,
    ambiguous: &mut u32,
) -> (HoldKind, String, Vec<u32>) {
    let mut kind = HoldKind::Owns;
    let mut via = String::new();
    if text.contains('&') {
        kind = HoldKind::Borrows;
        via = "&".to_string();
    }
    let mut found: Vec<u32> = Vec::new();
    let mut chars = text.chars().peekable();
    let mut first = skip_leading;
    while let Some(&c) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            let mut run = String::new();
            while chars
                .peek()
                .is_some_and(|&c| c.is_alphanumeric() || c == '_')
            {
                run.push(chars.next().unwrap());
            }
            if std::mem::take(&mut first) {
                continue;
            }
            match targets.get(&run) {
                Some(ids) if ids.len() == 1 => {
                    if !found.contains(&ids[0]) {
                        found.push(ids[0]);
                    }
                }
                Some(_) => *ambiguous += 1,
                None => {
                    if run == "dyn" && rank(HoldKind::Dyn) > rank(kind) {
                        kind = HoldKind::Dyn;
                        via = "dyn".to_string();
                    }
                    if let Some(&(word, k)) = WRAPPER_WORDS.iter().find(|(word, _)| *word == run)
                        && rank(k) > rank(kind)
                    {
                        kind = k;
                        via = word.to_string();
                    }
                }
            }
        } else {
            chars.next();
        }
    }
    if kind == HoldKind::Owns {
        via = String::new();
    }
    (kind, via, found)
}

/// The crate a base-edition path belonged to: the live file sharing the most
/// leading path segments wins, so a deleted file lands beside its neighbors.
fn krate_for(path: &str, graph: &CodeGraph) -> String {
    let segs: Vec<&str> = path.split('/').collect();
    graph
        .files
        .iter()
        .max_by_key(|f| {
            f.path
                .split('/')
                .zip(&segs)
                .take_while(|(a, b)| a == *b)
                .count()
        })
        .map(|f| f.krate.clone())
        .unwrap_or_default()
}

/// Compute the structural diff and write it onto the graph: per-item deltas
/// and row diffs, ghost marks for removed declarations, and hold events —
/// `Added` on live edges the base could not have drawn, `Removed` edges
/// re-drawn from the base edition.
pub(super) fn apply(
    dir: &std::path::Path,
    diff: &Diff,
    graph: &mut CodeGraph,
    sources: &[String],
    details: &[FileDetail],
) {
    if diff.base_ref.is_none() || diff.changed_files.is_empty() {
        return;
    }
    let file_id: HashMap<&str, u32> = graph
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.id))
        .collect();
    // What the data chart can draw, live side. Only a free function: a method
    // is matched under the type its impl names, never as a declaration of the
    // file's own, and two types can each own a `build` without either being
    // the base's free `build`.
    let charted = |item: &ItemMark| match item.kind {
        ItemKind::Struct
        | ItemKind::Enum
        | ItemKind::Union
        | ItemKind::Static
        | ItemKind::Trait => true,
        // Free only: an associated const or type is its owner's row, and the
        // base reads it the same way — `module_prefix` never leaves an impl
        // or a trait block.
        ItemKind::Fn | ItemKind::Const | ItemKind::TypeAlias => item.parent.is_none(),
        _ => false,
    };

    // ---- Read each changed file's two editions and match by (kind, name). --
    // Candidates cross files before they settle: a declaration that left one
    // changed file and appeared in another is the same declaration moved, not
    // a ghost plus a stranger.
    let mut added: Vec<u32> = Vec::new(); // live item ids with no base match yet
    let mut removed: Vec<(String, BaseDecl)> = Vec::new(); // (path, decl)
    let mut matched: Vec<(u32, BaseDecl)> = Vec::new();

    for path in &diff.changed_files {
        if !path.ends_with(".rs") {
            continue;
        }
        let live_file = file_id.get(path.as_str()).copied();
        if live_file.is_none() && dir.join(path).exists() {
            // On disk but not in the survey (not on any crate's module tree):
            // not on this chart in either edition.
            continue;
        }
        let base = file_at_base(dir, diff, path)
            .map(|text| BaseDecl::scan(&text))
            .unwrap_or_default();
        let mut base_of: HashMap<(ItemKind, &str), &BaseDecl> = base
            .iter()
            .map(|d| ((d.kind, d.name.as_str()), d))
            .collect();
        if let Some(file) = live_file {
            for item in graph.items.iter().filter(|i| i.file == file) {
                if !charted(item) {
                    continue;
                }
                match base_of.remove(&(item.kind, item.name.as_str())) {
                    Some(decl) => matched.push((
                        item.id,
                        BaseDecl {
                            name: decl.name.clone(),
                            kind: decl.kind,
                            vis: decl.vis,
                            line: decl.line,
                            text: decl.text.clone(),
                            field_rows: decl.field_rows.clone(),
                            variants: decl.variants.clone(),
                            ty: decl.ty.clone(),
                            method_rows: decl.method_rows.clone(),
                        },
                    )),
                    None => added.push(item.id),
                }
            }
        }
        let leftover: HashSet<(ItemKind, String)> =
            base_of.keys().map(|(k, n)| (*k, n.to_string())).collect();
        for decl in base {
            if leftover.contains(&(decl.kind, decl.name.clone())) {
                removed.push((path.clone(), decl));
            }
        }
    }

    // ---- Moves: a removed (kind, name) that reappeared as an added one. ----
    let mut still_removed: Vec<(String, BaseDecl)> = Vec::new();
    for (path, decl) in removed {
        let take = added.iter().position(|&id| {
            let item = &graph.items[id as usize];
            item.kind == decl.kind && item.name == decl.name
        });
        match take {
            Some(at) => matched.push((added.swap_remove(at), decl)),
            None => still_removed.push((path, decl)),
        }
    }

    // ---- Deltas and row diffs onto the live marks. --------------------------
    for &id in &added {
        graph.items[id as usize].delta = Delta::Added;
    }
    for (id, decl) in matched {
        let item = &graph.items[id as usize];
        // The type's own method band, held to the file the type is declared
        // in — the only file whose base edition this pass read. `seat` maps
        // each of those rows back to its place in the whole band, so the
        // weave marks the row the reader is looking at.
        let seat: Vec<usize> = item
            .method_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                // A trait's clauses are written inside it, so all of them are
                // in the file the base read; a type's are wherever its impl
                // blocks are, and only its own file's were fetched.
                item.kind == ItemKind::Trait
                    || graph
                        .items
                        .get(row.mark as usize)
                        .is_some_and(|m| m.file == item.file)
            })
            .map(|(at, _)| at)
            .collect();
        let live_methods: Vec<(String, String)> = seat
            .iter()
            .map(|&at| {
                let row = &item.method_rows[at];
                (row.name.clone(), row.sig.clone())
            })
            .collect();
        // A function's block quotes its signature, so its signature is what
        // this altitude reads: a rewritten body is the code altitude's news,
        // and an `M` here would point at rows that did not move. A type's
        // block quotes its methods too, and an impl block lives outside the
        // declaration's own text, so the band is compared beside it.
        let same = if decl.kind == ItemKind::Fn {
            item.vis == decl.vis && item.field_rows == decl.field_rows && item.ty == decl.ty
        } else if decl.kind == ItemKind::Trait {
            // A trait's block is its band, and its declaration's own text
            // carries the default methods' bodies: comparing the text would
            // flare a trait whose promise never moved.
            item.vis == decl.vis && live_methods == decl.method_rows
        } else {
            let live_text = details
                .get(item.file as usize)
                .and_then(|d| d.items.get(item.local as usize))
                .and_then(|info| {
                    sources
                        .get(item.file as usize)
                        .and_then(|src| src.get(info.start as usize..info.end as usize))
                })
                .map(collapsed)
                .unwrap_or_default();
            live_text == decl.text && live_methods == decl.method_rows
        };
        if same {
            continue; // Same, moved or not: the shape is the base's shape.
        }
        let full = item.method_rows.len() as u32;
        let (ma, mr) = diff_rows(&decl.method_rows, &live_methods);
        let item = &mut graph.items[id as usize];
        item.delta = Delta::Changed;
        // Fields compare on the text a reader sees, visibility included, so
        // a field losing its `pub` reads as the change it is.
        let as_pairs = |rows: &[DeclRow]| -> Vec<(String, String)> {
            rows.iter().map(|r| (r.name.clone(), r.written())).collect()
        };
        let (fa, fr) = diff_rows(&as_pairs(&decl.field_rows), &as_pairs(&item.field_rows));
        let base_row: HashMap<&str, &DeclRow> = decl
            .field_rows
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect();
        item.fields_added = fa;
        item.fields_removed = fr
            .into_iter()
            .filter_map(|(at, name, _)| Some((at, (*base_row.get(name.as_str())?).clone())))
            .collect();
        let (va, vr) = diff_variants(&decl.variants, &item.variants);
        item.variants_added = va;
        item.variants_removed = vr;
        // Back into the whole band's own indexes: a row the base dropped
        // seats before the same-file row that took its place, or at the end.
        item.methods_added = ma.into_iter().map(|at| seat[at as usize] as u32).collect();
        item.methods_removed = mr
            .into_iter()
            .map(|(before, name, sig)| {
                let at = seat.get(before as usize).map_or(full, |&at| at as u32);
                (at, name, sig)
            })
            .collect();
    }

    // ---- Ghosts for what did not come back. ---------------------------------
    for (path, decl) in still_removed {
        let id = (graph.items.len() + graph.ghosts.len()) as u32;
        graph.ghosts.push(GhostMark {
            id,
            krate: krate_for(&path, graph),
            path,
            name: decl.name,
            kind: decl.kind,
            vis: decl.vis,
            line: decl.line,
            field_rows: decl.field_rows,
            variants: decl.variants,
            ty: decl.ty,
            method_rows: decl.method_rows,
        });
    }

    // ---- Hold events: added edges. ------------------------------------------
    // Exact where it can be: an edge whose either end is an added type, or
    // whose every drawing field is an added row, cannot have been at the base.
    let added_rows: HashMap<u32, HashSet<String>> = graph
        .items
        .iter()
        .filter(|i| i.delta == Delta::Changed)
        .map(|i| {
            let mut names: HashSet<String> = i
                .fields_added
                .iter()
                .filter_map(|&at| i.field_rows.get(at as usize))
                .map(|row| row.name.clone())
                .collect();
            names.extend(
                i.variants_added
                    .iter()
                    .filter_map(|&at| i.variants.get(at as usize))
                    .map(|w| variant_name(w)),
            );
            names.extend(
                i.methods_added
                    .iter()
                    .filter_map(|&at| i.method_rows.get(at as usize))
                    .map(|row| row.name.clone()),
            );
            (i.id, names)
        })
        .collect();
    let delta_of = |id: u32| {
        graph
            .items
            .get(id as usize)
            .map(|i| i.delta)
            .unwrap_or_default()
    };
    for edge in &mut graph.holds {
        let all_new = added_rows
            .get(&edge.from)
            .is_some_and(|names| edge.fields.iter().all(|(n, _)| names.contains(n)));
        if delta_of(edge.from) == Delta::Added || delta_of(edge.to) == Delta::Added || all_new {
            edge.event = Some(HoldEvent::Added);
        }
    }

    // ---- Hold events: removed edges, re-drawn from the base edition. --------
    // A dropped row's declared type is matched by name against the types this
    // chart knows — live and ghost alike. Where the pair still holds through
    // another live field, the relation stands and no removed edge is drawn.
    let mut targets: HashMap<String, Vec<u32>> = HashMap::new();
    for item in &graph.items {
        if matches!(
            item.kind,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Trait
        ) {
            let bare = item.name.rsplit("::").next().unwrap_or(&item.name);
            targets.entry(bare.to_string()).or_default().push(item.id);
        }
    }
    for ghost in &graph.ghosts {
        if matches!(
            ghost.kind,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Trait
        ) {
            let bare = ghost.name.rsplit("::").next().unwrap_or(&ghost.name);
            targets.entry(bare.to_string()).or_default().push(ghost.id);
        }
    }
    // A pair that still stands through another live row draws no removed
    // edge — and the two readings are kept apart here too: a field the base
    // dropped is still news when the API happens to name the same type.
    let live_pair: HashSet<(u32, u32, bool)> = graph
        .holds
        .iter()
        .filter(|e| e.event.is_none())
        .map(|e| (e.from, e.to, e.from_method))
        .collect();

    let mut ambiguous = 0u32;
    // (from, name, quoted row, skip the row's own leading name, a method row)
    let mut dropped: Vec<(u32, String, String, bool, bool)> = Vec::new();
    for item in &graph.items {
        for (_, row) in &item.fields_removed {
            dropped.push((item.id, row.name.clone(), row.written(), false, false));
        }
        for (_, written) in &item.variants_removed {
            dropped.push((item.id, variant_name(written), written.clone(), true, false));
        }
        for (_, name, sig) in &item.methods_removed {
            dropped.push((item.id, name.clone(), sig.clone(), false, true));
        }
    }
    for ghost in &graph.ghosts {
        for row in &ghost.field_rows {
            dropped.push((ghost.id, row.name.clone(), row.written(), false, false));
        }
        for written in &ghost.variants {
            dropped.push((
                ghost.id,
                variant_name(written),
                written.clone(),
                true,
                false,
            ));
        }
        for (name, sig) in &ghost.method_rows {
            dropped.push((ghost.id, name.clone(), sig.clone(), false, true));
        }
        if !ghost.ty.is_empty() {
            dropped.push((ghost.id, ghost.name.clone(), ghost.ty.clone(), false, false));
        }
    }
    let mut gone: super::data::Edges = HashMap::new();
    for (from, name, decl, skip, from_method) in dropped {
        let (kind, via, found) = name_walk(&decl, skip, &targets, &mut ambiguous);
        for to in found {
            if to == from || live_pair.contains(&(from, to, from_method)) {
                continue;
            }
            let rows = gone
                .entry((from, to, kind, via.clone(), from_method))
                .or_default();
            if !rows.iter().any(|(n, d)| n == &name && d == &decl) {
                rows.push((name.clone(), decl.clone()));
            }
        }
    }
    let mut gone: Vec<HoldEdge> = gone
        .into_iter()
        .map(|((from, to, kind, via, from_method), fields)| HoldEdge {
            from,
            to,
            kind,
            via,
            fields,
            from_method,
            event: Some(HoldEvent::Removed),
        })
        .collect();
    gone.sort_by(|a, b| {
        (a.from, a.to, rank(a.kind), &a.via, a.from_method).cmp(&(
            b.from,
            b.to,
            rank(b.kind),
            &b.via,
            b.from_method,
        ))
    });
    graph.holds.extend(gone);

    // ---- Implements: what the base promised, and what it does not. ----------
    // A type taking on a contract, or dropping one, is the loudest thing this
    // altitude can say about a change, so it is diff ink either way. The base
    // edition is read for the pair of names an impl block writes — the live
    // side resolved them properly, but the base has only syntax, so the two
    // are compared as `(trait, type)` names and an ambiguous name is left
    // alone rather than guessed at.
    let name_of = |mark: u32| -> Option<&str> {
        let name = &graph.items.get(mark as usize)?.name;
        Some(name.rsplit("::").next().unwrap_or(name))
    };
    let mut base_impls: HashSet<(String, String)> = HashSet::new();
    let mut read_files: HashSet<&str> = HashSet::new();
    for path in &diff.changed_files {
        if !path.ends_with(".rs") {
            continue;
        }
        let Some(text) = file_at_base(dir, diff, path) else {
            continue;
        };
        read_files.insert(path.as_str());
        let parse = ra_ap_syntax::SourceFile::parse(&text, ra_ap_syntax::Edition::CURRENT);
        for imp in parse
            .tree()
            .syntax()
            .descendants()
            .filter_map(ast::Impl::cast)
        {
            let (Some(trait_), Some(self_ty)) = (imp.trait_(), imp.self_ty()) else {
                continue;
            };
            base_impls.insert((
                bare_name(&collapsed(&trait_.syntax().text().to_string())),
                bare_name(&collapsed(&self_ty.syntax().text().to_string())),
            ));
        }
    }
    // Only impls the base pass actually read can be compared: an impl block in
    // a file this epoch never touched is unchanged by construction.
    let changed_file: HashSet<u32> = graph
        .files
        .iter()
        .filter(|f| read_files.contains(f.path.as_str()))
        .map(|f| f.id)
        .collect();
    let mut live_impls: HashSet<(String, String)> = HashSet::new();
    for edge in &mut graph.implements {
        let pair = match (name_of(edge.trait_mark), name_of(edge.ty)) {
            (Some(t), Some(ty)) => (t.to_string(), ty.to_string()),
            _ => continue,
        };
        live_impls.insert(pair.clone());
        // Where the impl block itself was not read, nothing can be said.
        let touched = graph
            .items
            .get(edge.ty as usize)
            .is_some_and(|m| changed_file.contains(&m.file))
            || graph
                .items
                .get(edge.trait_mark as usize)
                .is_some_and(|m| changed_file.contains(&m.file));
        if touched && !base_impls.contains(&pair) {
            edge.event = Some(HoldEvent::Added);
        }
    }
    // What the base promised and the working copy does not, re-drawn from the
    // base edition — but only where both ends still have a mark to land on.
    let mark_named = |want: &str, kinds: &[ItemKind]| -> Option<u32> {
        let mut found = None;
        for item in &graph.items {
            let bare = item.name.rsplit("::").next().unwrap_or(&item.name);
            if bare == want && kinds.contains(&item.kind) {
                if found.is_some() {
                    return None; // ambiguous: two marks by that name
                }
                found = Some(item.id);
            }
        }
        found
    };
    let mut dropped_impls: Vec<ImplEdge> = base_impls
        .difference(&live_impls)
        .filter_map(|(trait_name, ty_name)| {
            Some(ImplEdge {
                trait_mark: mark_named(trait_name, &[ItemKind::Trait])?,
                ty: mark_named(
                    ty_name,
                    &[ItemKind::Struct, ItemKind::Enum, ItemKind::Union],
                )?,
                event: Some(HoldEvent::Removed),
            })
        })
        .collect();
    dropped_impls.sort_by_key(|e| (e.trait_mark, e.ty));
    graph.implements.extend(dropped_impls);

    // ---- The method, in words. ----------------------------------------------
    graph.notes.push(
        "the structural diff reads the base edition of each changed file \
         syntactically: declarations match by kind and name, a method band is \
         matched against the impls in the type's own file, and a removed \
         relation's target is matched by name — never type-resolved"
            .to_string(),
    );
    if ambiguous > 0 {
        graph.notes.push(format!(
            "{ambiguous} names in removed declarations matched more than one \
             type and are not drawn"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_decls_read_kinds_names_and_rows() {
        let text = r#"
/// Doc.
pub struct Wire { pub id: u32, pub(crate) name: String, seen: bool }
pub(crate) enum Kind { A, B(String) }
static CACHE: OnceCell<Arc<Index>> = OnceCell::new();
mod tests {
    pub struct Sample(u8);
}
fn body() { struct Local; }
pub fn survey(dir: &Path, quiet: bool) -> Result<Index, String> { todo!() }
fn nothing() -> () {}
impl Wire {
    pub fn id(&self) -> u32 { self.id }
}
"#;
        let decls = BaseDecl::scan(text);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        // A free function is a declaration at this altitude; a method is its
        // type's, and a type declared in a body has no mark in either edition.
        assert_eq!(
            names,
            vec![
                "Wire",
                "Kind",
                "CACHE",
                "tests::Sample",
                "body",
                "survey",
                "nothing"
            ]
        );
        assert_eq!(decls[0].vis, Vis::Pub);
        // Each field's own visibility, not its type's: a `pub` struct can
        // publish some of its state and keep the rest.
        let rows = |at: usize| -> Vec<(&str, &str, Vis)> {
            decls[at]
                .field_rows
                .iter()
                .map(|row| (row.name.as_str(), row.ty.as_str(), row.vis))
                .collect()
        };
        assert_eq!(
            rows(0),
            vec![
                ("id", "u32", Vis::Pub),
                ("name", "String", Vis::Crate),
                ("seen", "bool", Vis::Private),
            ]
        );
        assert_eq!(decls[1].vis, Vis::Crate);
        assert_eq!(decls[1].variants, vec!["A", "B(String)"]);
        assert_eq!(decls[2].ty, "OnceCell<Arc<Index>>");
        assert_eq!(rows(3), vec![("0", "u8", Vis::Private)]);
        // A signature reads as rows and a return line: the parameters as the
        // base wrote them, the return type in the slot a static's type uses.
        assert_eq!((decls[5].kind, decls[5].vis), (ItemKind::Fn, Vis::Pub));
        // A parameter declares no visibility of its own, ever.
        assert_eq!(
            rows(5),
            vec![
                ("dir", "&Path", Vis::Private),
                ("quiet", "bool", Vis::Private),
            ]
        );
        assert_eq!(decls[5].ty, "Result<Index, String>");
        // Handing nothing back is not a return line, written either way.
        assert!(decls[4].ty.is_empty() && decls[6].ty.is_empty());
    }

    /// A type's band is written outside its declaration, so the base has to
    /// read the impl blocks beside it and file them under the same name the
    /// declaration is filed under — inline modules included.
    #[test]
    fn a_base_edition_reads_the_method_band_off_its_impls() {
        let text = r#"
pub struct Wire { pub id: u32 }
impl Wire {
    /// Doc.
    #[inline]
    pub fn build(nut: &Nut) -> Wire { todo!() }
    fn secret(&self) -> u32 { self.id }
}
impl Clone for Wire {
    fn clone(&self) -> Self { todo!() }
}
mod tests {
    pub struct Wire;
    impl Wire {
        pub fn probe(&self) {}
    }
}
"#;
        let decls = BaseDecl::scan(text);
        let rows = |name: &str| {
            decls
                .iter()
                .find(|d| d.name == name)
                .unwrap()
                .method_rows
                .clone()
        };
        // Both impls, in source order, with the doc comment, the attribute
        // and the body left where they are.
        assert_eq!(
            rows("Wire"),
            vec![
                (
                    "build".to_string(),
                    "pub fn build(nut: &Nut) -> Wire".to_string()
                ),
                ("secret".to_string(), "fn secret(&self) -> u32".to_string()),
                ("clone".to_string(), "fn clone(&self) -> Self".to_string()),
            ]
        );
        // A type inside an inline module keeps its own band, not the outer
        // type's: the impl is named the way the declaration is.
        assert_eq!(
            rows("tests::Wire"),
            vec![("probe".to_string(), "pub fn probe(&self)".to_string())]
        );
    }

    #[test]
    fn row_diff_marks_added_removed_and_retyped() {
        let base = vec![
            ("id".to_string(), "u32".to_string()),
            ("changed".to_string(), "bool".to_string()),
            ("lines".to_string(), "u32".to_string()),
        ];
        let live = vec![
            ("id".to_string(), "u32".to_string()),
            ("status".to_string(), "FileStatus".to_string()),
            ("lines".to_string(), "u64".to_string()),
        ];
        let (added, removed) = diff_rows(&base, &live);
        // `status` is new; `lines` changed type, so its live row is marked too.
        assert_eq!(added, vec![1, 2]);
        // `changed` is gone (ghost before the row that replaced its slot);
        // `lines`'s base row ghosts right above its own live row.
        assert_eq!(
            removed,
            vec![
                (1, "changed".to_string(), "bool".to_string()),
                (2, "lines".to_string(), "u32".to_string()),
            ]
        );
    }

    #[test]
    fn name_walk_finds_targets_and_wrappers() {
        let mut targets: HashMap<String, Vec<u32>> = HashMap::new();
        targets.insert("FileRef".to_string(), vec![7]);
        targets.insert("Trail".to_string(), vec![3]);
        let mut ambiguous = 0;
        let (kind, via, found) = name_walk("Vec<FileRef>", false, &targets, &mut ambiguous);
        assert_eq!((kind, via.as_str(), found), (HoldKind::Owns, "", vec![7]));
        let (kind, via, found) = name_walk("GlobalSignal<Trail>", false, &targets, &mut ambiguous);
        assert_eq!(
            (kind, via.as_str(), found),
            (HoldKind::Shares, "GlobalSignal", vec![3])
        );
        // A variant's own leading name never targets itself.
        targets.insert("File".to_string(), vec![9]);
        let (_, _, found) = name_walk("File(String, String)", true, &targets, &mut ambiguous);
        assert!(found.is_empty());
        assert_eq!(ambiguous, 0);
    }
}
