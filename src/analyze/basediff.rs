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
//! Scope (the data altitude): structs, enums, unions, and statics. The other
//! item kinds keep `Delta::Same` until the code altitude takes its pass.

use std::collections::{HashMap, HashSet};

use ra_ap_syntax::ast::{HasName, HasVisibility};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, ast};

use super::vcs::{Diff, file_at_base};
use crate::api::{
    CodeGraph, Delta, FileDetail, GhostMark, HoldEdge, HoldEvent, HoldKind, ItemKind, Vis,
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
    field_rows: Vec<(String, String)>,
    variants: Vec<String>,
    ty: String,
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

/// A field list's rows, syntactically: (name as written — a tuple field's
/// index — and the declared type as written).
fn base_fields(list: Option<ast::FieldList>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    match list {
        Some(ast::FieldList::RecordFieldList(fields)) => {
            for field in fields.fields() {
                let name = field
                    .name()
                    .map(|n| n.text().to_string())
                    .unwrap_or_default();
                out.push((name, super::data::type_text(field.ty())));
            }
        }
        Some(ast::FieldList::TupleFieldList(fields)) => {
            for (index, field) in fields.fields().enumerate() {
                out.push((index.to_string(), super::data::type_text(field.ty())));
            }
        }
        None => {}
    }
    out
}

/// The inline-module path standing over a node (`tests::`), or `None` when
/// the node is not on a plain module chain at all — a type declared inside a
/// function body has no mark in the live survey, so the base reads it the
/// same way.
fn module_prefix(node: &SyntaxNode) -> Option<String> {
    let mut names: Vec<String> = Vec::new();
    for up in node.ancestors().skip(1) {
        match up.kind() {
            SyntaxKind::SOURCE_FILE | SyntaxKind::ITEM_LIST => {}
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

/// Every charted declaration in one base edition, in source order.
fn base_decls(text: &str) -> Vec<BaseDecl> {
    let parse = ra_ap_syntax::SourceFile::parse(text, ra_ap_syntax::Edition::CURRENT);
    let mut out = Vec::new();
    for node in parse.tree().syntax().descendants() {
        let kind = match node.kind() {
            SyntaxKind::STRUCT => ItemKind::Struct,
            SyntaxKind::ENUM => ItemKind::Enum,
            SyntaxKind::UNION => ItemKind::Union,
            SyntaxKind::STATIC => ItemKind::Static,
            _ => continue,
        };
        let Some(prefix) = module_prefix(&node) else {
            continue;
        };
        let range = node.text_range();
        let (name, vis, field_rows, variants, ty) = match kind {
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
                )
            }
            _ => continue,
        };
        out.push(BaseDecl {
            name: format!("{prefix}{name}"),
            kind,
            vis,
            line: line_of(text, usize::from(range.start())),
            text: collapsed(&node.text().to_string()),
            field_rows,
            variants,
            ty,
        });
    }
    out
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
    let charted = |kind: ItemKind| {
        matches!(
            kind,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Static
        )
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
            .map(|text| base_decls(&text))
            .unwrap_or_default();
        let mut base_of: HashMap<(ItemKind, &str), &BaseDecl> = base
            .iter()
            .map(|d| ((d.kind, d.name.as_str()), d))
            .collect();
        if let Some(file) = live_file {
            for item in graph.items.iter().filter(|i| i.file == file) {
                if !charted(item.kind) {
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
        if live_text == decl.text {
            continue; // Same, moved or not: the shape is the base's shape.
        }
        let item = &mut graph.items[id as usize];
        item.delta = Delta::Changed;
        let (fa, fr) = diff_rows(&decl.field_rows, &item.field_rows);
        item.fields_added = fa;
        item.fields_removed = fr;
        let (va, vr) = diff_variants(&decl.variants, &item.variants);
        item.variants_added = va;
        item.variants_removed = vr;
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
                .map(|(n, _)| n.clone())
                .collect();
            names.extend(
                i.variants_added
                    .iter()
                    .filter_map(|&at| i.variants.get(at as usize))
                    .map(|w| variant_name(w)),
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
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union
        ) {
            let bare = item.name.rsplit("::").next().unwrap_or(&item.name);
            targets.entry(bare.to_string()).or_default().push(item.id);
        }
    }
    for ghost in &graph.ghosts {
        if matches!(
            ghost.kind,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union
        ) {
            let bare = ghost.name.rsplit("::").next().unwrap_or(&ghost.name);
            targets.entry(bare.to_string()).or_default().push(ghost.id);
        }
    }
    let live_pair: HashSet<(u32, u32)> = graph
        .holds
        .iter()
        .filter(|e| e.event.is_none())
        .map(|e| (e.from, e.to))
        .collect();

    let mut ambiguous = 0u32;
    let mut dropped: Vec<(u32, String, String, bool)> = Vec::new(); // (from, name, decl, skip_leading)
    for item in &graph.items {
        for (_, name, decl) in &item.fields_removed {
            dropped.push((item.id, name.clone(), decl.clone(), false));
        }
        for (_, written) in &item.variants_removed {
            dropped.push((item.id, variant_name(written), written.clone(), true));
        }
    }
    for ghost in &graph.ghosts {
        for (name, decl) in &ghost.field_rows {
            dropped.push((ghost.id, name.clone(), decl.clone(), false));
        }
        for written in &ghost.variants {
            dropped.push((ghost.id, variant_name(written), written.clone(), true));
        }
        if !ghost.ty.is_empty() {
            dropped.push((ghost.id, ghost.name.clone(), ghost.ty.clone(), false));
        }
    }
    let mut gone: super::data::Edges = HashMap::new();
    for (from, name, decl, skip) in dropped {
        let (kind, via, found) = name_walk(&decl, skip, &targets, &mut ambiguous);
        if kind == HoldKind::Dyn {
            continue; // traits draw no marks yet, at the base or now
        }
        for to in found {
            if to == from || live_pair.contains(&(from, to)) {
                continue;
            }
            let rows = gone.entry((from, to, kind, via.clone())).or_default();
            if !rows.iter().any(|(n, d)| n == &name && d == &decl) {
                rows.push((name.clone(), decl.clone()));
            }
        }
    }
    let mut gone: Vec<HoldEdge> = gone
        .into_iter()
        .map(|((from, to, kind, via), fields)| HoldEdge {
            from,
            to,
            kind,
            via,
            fields,
            event: Some(HoldEvent::Removed),
        })
        .collect();
    gone.sort_by(|a, b| {
        (a.from, a.to, rank(a.kind), &a.via).cmp(&(b.from, b.to, rank(b.kind), &b.via))
    });
    graph.holds.extend(gone);

    // ---- The method, in words. ----------------------------------------------
    graph.notes.push(
        "the structural diff reads the base edition of each changed file \
         syntactically: declarations match by kind and name, and a removed \
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
pub struct Wire { pub id: u32, pub name: String }
pub(crate) enum Kind { A, B(String) }
static CACHE: OnceCell<Arc<Index>> = OnceCell::new();
mod tests {
    pub struct Sample(u8);
}
fn body() { struct Local; }
"#;
        let decls = base_decls(text);
        let names: Vec<&str> = decls.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["Wire", "Kind", "CACHE", "tests::Sample"]);
        assert_eq!(decls[0].vis, Vis::Pub);
        assert_eq!(
            decls[0].field_rows,
            vec![
                ("id".to_string(), "u32".to_string()),
                ("name".to_string(), "String".to_string())
            ]
        );
        assert_eq!(decls[1].vis, Vis::Crate);
        assert_eq!(decls[1].variants, vec!["A", "B(String)"]);
        assert_eq!(decls[2].ty, "OnceCell<Arc<Index>>");
        assert_eq!(
            decls[3].field_rows,
            vec![("0".to_string(), "u8".to_string())]
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
