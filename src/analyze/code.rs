//! The code survey: rust-analyzer as a library over the workspace sources.
//!
//! `cargo metadata` answers the crate altitude; this module answers the two
//! below it — files and items. It loads the workspace into a rust-analyzer
//! database, walks every workspace-member source file, collects its items
//! (functions, types, traits, impls), and resolves every reference it can:
//! paths, method calls, and field accesses. References that reach outside the
//! workspace are dropped — this altitude charts the reviewer's own code, not
//! its dependencies.
//!
//! Every name is read where rust-analyzer really resolved it, which for a
//! dioxus app is usually not where it is written: `rsx!` bodies are unparsed
//! token trees, and a `#[component]` or `#[server]` function's body is
//! type-checked as the macro's expansion, not as the text on disk. So each
//! name token is descended into the macros it reaches and resolved there,
//! keeping its own range in the real file for the plate to link. Reading only
//! the real tree would leave most of a dioxus workspace's references
//! uncounted, and a review tool that undercounts references invites deleting
//! live code.
//!
//! Resolution is semantic (types, traits, and methods are resolved the way
//! rustc sees them), but not omniscient: names that type inference cannot
//! settle are counted and reported in words, never silently invented.

use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use ra_ap_hir::{Adt, Crate, HasSource, InFile, ModuleDef, PathResolution, Semantics, Variant};
use ra_ap_ide_db::RootDatabase;
use ra_ap_ide_db::base_db::EditionedFileId;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_syntax::ast::{HasName, HasVisibility, VisibilityKind};
use ra_ap_syntax::{
    AstNode, AstToken, Edition, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, ast,
};
use ra_ap_vfs::{FileId, Vfs};
use tokio::sync::OnceCell;

use super::data;
use crate::api::{
    CodeGraph, FileDetail, FileInfo, FileRef, ImplEdge, ItemEdge, ItemInfo, ItemKind, ItemMark,
    ItemRef, ItemSource, MarkRef, SrcLink, SrcRun, Tok, Vis,
};

/// The whole survey, precomputed once: the shipped graph plus every file's
/// cutaway detail, ready to answer per-file queries from memory.
pub(crate) struct CodeIndex {
    pub(crate) graph: CodeGraph,
    /// Indexed by [`FileInfo::id`].
    pub(crate) details: Vec<FileDetail>,
    /// Every surveyed file's source text, indexed by [`FileInfo::id`]. The
    /// focus plate quotes it; it never crosses the wire whole.
    pub(crate) sources: Vec<String>,
    /// Clickable reference spans per file: (start byte, end byte, target file,
    /// target item label), sorted by start. The focus plate turns the ones
    /// inside a quoted item into links.
    pub(crate) ref_spans: Vec<Vec<RefSpan>>,
}

/// One clickable reference in a file's real source: the byte range of the
/// reference's name token, and where it resolves. Server-side only — the
/// focus plate translates the ones a quoted item contains into [`SrcLink`]s.
pub(crate) struct RefSpan {
    pub(crate) start: u32,
    pub(crate) end: u32,
    /// Target file, a [`FileInfo::id`].
    pub(crate) file: u32,
    /// Target item's URL label; empty for a whole-file target.
    pub(crate) label: String,
}

static INDEX: OnceCell<Result<Arc<CodeIndex>, String>> = OnceCell::const_new();

/// The cached survey. The first caller pays for it (tens of seconds on a
/// large workspace — rust-analyzer loads the whole workspace); everyone
/// after answers from memory.
pub(crate) async fn index() -> Result<Arc<CodeIndex>, String> {
    INDEX
        .get_or_init(|| async {
            tokio::task::spawn_blocking(|| {
                let dir = super::workspace_dir();
                std::panic::catch_unwind(AssertUnwindSafe(|| survey(&dir)))
                    .unwrap_or_else(|_| {
                        Err("the code survey crashed inside rust-analyzer".to_string())
                    })
                    .map(Arc::new)
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
        })
        .await
        .clone()
}

/// Offsets where each line starts, for offset → line mapping.
struct LineStarts(Vec<u32>);

impl LineStarts {
    fn new(text: &str) -> Self {
        let mut starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i as u32 + 1);
            }
        }
        Self(starts)
    }

    /// 1-based line for a byte offset.
    fn line(&self, offset: TextSize) -> u32 {
        let off = u32::from(offset);
        self.0.partition_point(|&s| s <= off) as u32
    }

    /// Byte offset where a 1-based line starts.
    fn start_of(&self, line: u32) -> Option<u32> {
        self.0.get(line.checked_sub(1)? as usize).copied()
    }

    fn count(&self) -> u32 {
        self.0.len() as u32
    }
}

/// One item as collected from the syntax tree, before ids are assigned.
struct RawItem {
    name: String,
    section: String,
    kind: ItemKind,
    range: TextRange,
    vis: Vis,
    /// The impl or trait block this item sits inside, by source range. An
    /// impl block is attribution, not geometry: its items belong to the
    /// impl's self type, wherever that type is written.
    owner: Option<TextRange>,
}

/// One file being surveyed.
struct RawFile {
    path: String,
    krate: String,
    efid: EditionedFileId,
    items: Vec<RawItem>,
    lines: u32,
    /// Source ranges the walk left out as test-only. Nothing written inside
    /// one is a declaration or a reference this survey knows about.
    test_ranges: Vec<TextRange>,
}

/// Where a resolved reference lands.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RefTarget {
    file: u32,
    /// Index into the target file's items; `None` for a whole-file target
    /// (a `use` of the module itself).
    item: Option<u32>,
}

/// Where a reference comes from: (source file, source item, target).
type RefSource = (u32, Option<u32>, RefTarget);

pub(crate) fn survey(dir: &std::path::Path) -> Result<CodeIndex, String> {
    let manifest = dir.join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "No Cargo.toml found in {}. Point slope at a cargo workspace: \
             SLOPE_WORKSPACE=/path/to/workspace",
            dir.display()
        ));
    }

    // All features on: a file behind `#[cfg(feature = …)]` is still the
    // reviewer's code, and a survey that silently skips it lies by omission.
    let cargo_config = CargoConfig {
        sysroot: Some(RustLibSource::Discover),
        all_targets: true,
        features: ra_ap_project_model::CargoFeatures::All,
        ..CargoConfig::default()
    };
    // Stay a polite guest on the reviewer's machine: the survey shares it
    // with their editor and whatever else is running.
    let threads = std::thread::available_parallelism()
        .map(|n| (n.get() / 4).max(2))
        .unwrap_or(2)
        .min(4);
    let load_config = LoadCargoConfig {
        load_out_dirs_from_check: true,
        with_proc_macro_server: ProcMacroServerChoice::Sysroot,
        prefill_caches: false,
        num_worker_threads: threads,
        proc_macro_processes: 1,
    };
    let (db, vfs, proc_macro) = load_workspace_at(dir, &cargo_config, &load_config, &|_| {})
        .map_err(|e| format!("rust-analyzer could not load the workspace: {e}"))?;

    // The new trait solver reads the database through a thread-local; every
    // Semantics call must run with it attached or inference panics.
    ra_ap_hir::attach_db(&db, || {
        survey_attached(dir, &db, &vfs, proc_macro.is_some())
    })
}

fn survey_attached(
    dir: &std::path::Path,
    db: &RootDatabase,
    vfs: &Vfs,
    proc_macros: bool,
) -> Result<CodeIndex, String> {
    let root = dunce_canonical(dir);
    let sema = Semantics::new(db);

    // ---- Pass A: enumerate workspace files and collect their items. ------

    // Walk every local crate's module tree; each module names its file. A
    // file reached from several crates (a lib and its bin) keeps the first,
    // in sorted crate order, so ids never depend on hash order.
    let mut crates: Vec<(String, Crate)> = Crate::all(db)
        .into_iter()
        .filter(|k| k.origin(db).is_local())
        .map(|k| {
            let name = k
                .display_name(db)
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".to_string());
            (name, k)
        })
        .collect();
    crates.sort_by(|a, b| a.0.cmp(&b.0));

    let mut seen: HashMap<FileId, ()> = HashMap::new();
    let mut raw: Vec<RawFile> = Vec::new();
    for (krate_name, krate) in &crates {
        for module in krate.modules(db) {
            let Some(efid) = module.as_source_file_id(db) else {
                continue;
            };
            let fid = efid.file_id(db);
            if seen.insert(fid, ()).is_some() {
                continue;
            }
            let Some(rel) = workspace_rel(vfs, fid, &root) else {
                continue;
            };
            raw.push(RawFile {
                path: rel,
                krate: krate_name.clone(),
                efid,
                items: Vec::new(),
                lines: 0,
                test_ranges: Vec::new(),
            });
        }
    }
    raw.sort_by(|a, b| a.path.cmp(&b.path));

    let file_of: HashMap<FileId, u32> = raw
        .iter()
        .enumerate()
        .map(|(i, f)| (f.efid.file_id(db), i as u32))
        .collect();

    let mut starts: Vec<LineStarts> = Vec::with_capacity(raw.len());
    // The text is read once here and kept: the focus plate quotes an item's
    // own source, and re-reading the file later would risk quoting something
    // the survey never saw.
    let mut sources: Vec<String> = Vec::with_capacity(raw.len());
    for file in raw.iter_mut() {
        let source = sema.parse(file.efid);
        let text = source.syntax().text().to_string();
        let lines = LineStarts::new(&text);
        file.lines = lines.count();
        starts.push(lines);
        sources.push(text);
        let mut items = Vec::new();
        let mut skipped = Vec::new();
        collect_items(
            source.syntax(),
            &ItemScope::root(),
            &mut items,
            &mut skipped,
        );
        items.sort_by_key(|i| (i.range.start(), std::cmp::Reverse(i.range.end())));
        file.items = items;
        file.test_ranges = skipped;
    }

    // ---- Pass A½: attribution. -------------------------------------------

    // Global mark ids: every item but the impl blocks, in (file, source)
    // order. An impl block is attribution, not geometry — it never gets a
    // mark of its own.
    let mut mark_of: Vec<Vec<Option<u32>>> = Vec::with_capacity(raw.len());
    let mut mark_at: Vec<(u32, u32)> = Vec::new();
    for (fi, file) in raw.iter().enumerate() {
        let mut per_file = Vec::with_capacity(file.items.len());
        for (li, item) in file.items.iter().enumerate() {
            if item.kind == ItemKind::Impl {
                per_file.push(None);
                continue;
            }
            per_file.push(Some(mark_at.len() as u32));
            mark_at.push((fi as u32, li as u32));
        }
        mark_of.push(per_file);
    }

    // Every impl block's self type, resolved semantically: the methods under
    // it belong to that type even when the impl sits in another file. The
    // trait it implements rides along as a lens on the type, never as
    // nesting.
    let mut impl_self: HashMap<(u32, u32), u32> = HashMap::new();
    let mut impl_traits: Vec<(u32, String)> = Vec::new();
    let mut implements: Vec<ImplEdge> = Vec::new();
    for (fi, file) in raw.iter().enumerate() {
        if !file.items.iter().any(|i| i.kind == ItemKind::Impl) {
            continue;
        }
        let source = sema.parse(file.efid);
        let impls: HashMap<TextRange, ast::Impl> = source
            .syntax()
            .descendants()
            .filter_map(ast::Impl::cast)
            .map(|i| (i.syntax().text_range(), i))
            .collect();
        for (li, item) in file.items.iter().enumerate() {
            if item.kind != ItemKind::Impl {
                continue;
            }
            let Some(node) = impls.get(&item.range) else {
                continue;
            };
            let Some(imp) = sema.to_impl_def(node) else {
                continue;
            };
            // Only a workspace type can hold territory on this map; an impl
            // for a foreign or unnameable type keeps its items on the file's
            // own shelf.
            let target = imp
                .self_ty(db)
                .as_adt()
                .and_then(|adt| def_target(&sema, db, vfs, &root, &file_of, &raw, adt.into()));
            let Some(RefTarget {
                file: ty_file,
                item: Some(ty_item),
            }) = target
            else {
                continue;
            };
            let Some(Some(mark)) = mark_of[ty_file as usize].get(ty_item as usize).copied() else {
                continue;
            };
            impl_self.insert((fi as u32, li as u32), mark);
            if node.trait_().is_some() {
                impl_traits.push((mark, impl_header(node)));
                // The trait it promises, resolved the way the self type was:
                // through the impl itself, never off the header's words. A
                // foreign trait has no mark to land on and stays a string.
                if let Some(trait_mark) = imp
                    .trait_(db)
                    .and_then(|t| def_target(&sema, db, vfs, &root, &file_of, &raw, t.into()))
                    .and_then(|target| {
                        let local = target.item?;
                        mark_of[target.file as usize].get(local as usize).copied()?
                    })
                {
                    implements.push(ImplEdge {
                        trait_mark,
                        ty: mark,
                        event: None,
                    });
                }
            }
        }
    }

    // Containment: an item's parent is the type its impl names, or the trait
    // that declares it. Inline modules are not containers at this altitude.
    // The data walk below reads it: a function with no parent is free, and a
    // free function's signature is a contract the data altitude charts.
    let mut parent_of: Vec<Option<u32>> = vec![None; mark_at.len()];
    for (fi, file) in raw.iter().enumerate() {
        let by_range: HashMap<TextRange, u32> = file
            .items
            .iter()
            .enumerate()
            .map(|(li, it)| (it.range, li as u32))
            .collect();
        for (li, item) in file.items.iter().enumerate() {
            let (Some(mark), Some(owner)) = (
                mark_of[fi][li],
                item.owner.and_then(|r| by_range.get(&r).copied()),
            ) else {
                continue;
            };
            let parent = match file.items[owner as usize].kind {
                ItemKind::Impl => impl_self.get(&(fi as u32, owner)).copied(),
                // A trait declares its own items; nothing else nests.
                _ => mark_of[fi][owner as usize],
            };
            if parent != Some(mark) {
                parent_of[mark as usize] = parent;
            }
        }
    }

    // ---- Pass B: resolve references. --------------------------------------

    // (source file, source item, target) → count.
    let mut acc: HashMap<(u32, Option<u32>, RefTarget), u32> = HashMap::new();
    let mut unresolved: u32 = 0;
    // Each recorded reference's name token, as a byte range in the real
    // source file, with its target — the raw material for clickable spans.
    let mut raw_spans: Vec<Vec<(u32, u32, RefTarget)>> = vec![Vec::new(); raw.len()];

    for (src_file, file) in raw.iter().enumerate() {
        let source = sema.parse(file.efid);
        scan_refs(
            &sema,
            db,
            vfs,
            &root,
            &file_of,
            &raw,
            src_file as u32,
            source.syntax(),
            &mut acc,
            &mut unresolved,
            &mut raw_spans[src_file],
        );
    }

    // Order the spans and keep at most one per range: two references cannot
    // share one token on screen, so the first (leftmost, then longest kept
    // first by the sort) wins and anything overlapping it is dropped.
    let ref_spans: Vec<Vec<RefSpan>> = raw_spans
        .into_iter()
        .map(|mut spans| {
            spans.sort_by_key(|&(s, e, _)| (s, e));
            let mut out: Vec<RefSpan> = Vec::new();
            for (start, end, target) in spans {
                if out.last().is_some_and(|prev| start < prev.end) {
                    continue;
                }
                let label = target
                    .item
                    .map(|it| &raw[target.file as usize].items[it as usize])
                    // An impl block has no mark to land on; fall back to the
                    // file as a whole.
                    .filter(|item| item.kind != ItemKind::Impl)
                    .map(item_label)
                    .unwrap_or_default();
                out.push(RefSpan {
                    start,
                    end,
                    file: target.file,
                    label,
                });
            }
            out
        })
        .collect();

    // ---- Pass C: the data walk. -------------------------------------------

    // Which types hold which, and through what. A field's type resolves to a
    // mark through the very same def → mark path a reference does, so the
    // data altitude never invents a landmark the code altitude does not have.
    let data_mark = |def: ModuleDef| -> Option<u32> {
        let want: &[ItemKind] = match def {
            ModuleDef::Adt(_) => &[ItemKind::Struct, ItemKind::Enum, ItemKind::Union],
            ModuleDef::Trait(_) => &[ItemKind::Trait],
            _ => return None,
        };
        let target = def_target(&sema, db, vfs, &root, &file_of, &raw, def)?;
        let local = target.item?;
        // A type declared inside a function body has no mark of its own and
        // resolves to the function around it, which is not a data mark.
        let item = raw[target.file as usize].items.get(local as usize)?;
        if !want.contains(&item.kind) {
            return None;
        }
        mark_of[target.file as usize][local as usize]
    };
    // A type's methods are rows of the type, so they are walked from the
    // type's mark: the method's own file and range say where to read the
    // signature, and the edges leave the block that draws it.
    let charted_type = |mark: u32| {
        let (fi, li) = mark_at[mark as usize];
        matches!(
            raw[fi as usize].items[li as usize].kind,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Trait
        )
    };
    let holders: Vec<data::Holder> = mark_at
        .iter()
        .enumerate()
        .filter_map(|(id, &(fi, li))| {
            let item = &raw[fi as usize].items[li as usize];
            let method = match (item.kind, parent_of[id]) {
                // A row of the block its owner draws: a method of a type, or
                // one of the clauses a trait declares — a method signature, an
                // associated type, an associated const. Anything under an impl
                // for a type this chart never draws keeps to itself.
                (ItemKind::Fn | ItemKind::TypeAlias | ItemKind::Const, Some(owner))
                    if charted_type(owner) =>
                {
                    Some(data::MethodOf { mark: id as u32 })
                }
                _ => None,
            };
            let walked = match item.kind {
                // A trait has no rows of its own to walk: its clauses are
                // items in their own right, and they arrive as their own
                // holders, pointed at the trait's mark.
                ItemKind::Struct | ItemKind::Enum | ItemKind::Union | ItemKind::Static => true,
                // A free function is its own contract; a method is its type's.
                ItemKind::Fn => parent_of[id].is_none() || method.is_some(),
                // An associated type or const is a row of the block that
                // declares it; a free one is a contract of its own, one line
                // long.
                ItemKind::TypeAlias | ItemKind::Const => true,
                _ => false,
            };
            walked.then_some(data::Holder {
                mark: match &method {
                    Some(of) => parent_of[of.mark as usize].unwrap_or(id as u32),
                    None => id as u32,
                },
                kind: item.kind,
                file: fi,
                range: item.range,
                method,
            })
        })
        .collect();
    let efids: Vec<EditionedFileId> = raw.iter().map(|f| f.efid).collect();
    let mut walk = data::walk(&sema, db, &efids, &holders, mark_at.len(), &data_mark);

    // ---- Assemble the wire model. -----------------------------------------

    let mut file_pair: HashMap<(u32, u32), u32> = HashMap::new();
    let mut item_refs: Vec<Vec<ItemRef>> = vec![Vec::new(); raw.len()];
    // Cross-file references at item precision, aggregated per pair. Several
    // impl blocks for one type collapse onto the same endpoint, so the pairs
    // must be summed, not pushed.
    let mut edge_acc: HashMap<(u32, Option<u32>, u32, Option<u32>), u32> = HashMap::new();
    // The same thing for two items of one file. The cutaway reads those from
    // the file's own detail, but a chart that draws dependence needs them on
    // the wire: which file a reference was written in says nothing about
    // whether one contract leans on another.
    let mut local_acc: HashMap<(u32, u32), u32> = HashMap::new();

    let mut ordered: Vec<(RefSource, u32)> = acc.into_iter().collect();
    ordered.sort_by_key(|((f, i, t), _)| (*f, *i, t.file, t.item));

    for ((src_file, src_item, target), count) in ordered {
        if target.file == src_file {
            // Within one file: an item-level edge for the cutaway. A file
            // referencing itself as a whole says nothing; drop that.
            if let (Some(from), Some(to)) = (src_item, target.item)
                && from != to
            {
                item_refs[src_file as usize].push(ItemRef { from, to, count });
                // The same reference at mark precision. An impl block is
                // attribution, so a reference written inside one comes from
                // the type it names; a reference *to* an impl block is a
                // reference to nothing the chart can land on.
                let from = mark_of[src_file as usize][from as usize]
                    .or_else(|| impl_self.get(&(src_file, from)).copied());
                let to = mark_of[src_file as usize][to as usize];
                if let (Some(from), Some(to)) = (from, to)
                    && from != to
                {
                    *local_acc.entry((from, to)).or_default() += count;
                }
            }
            continue;
        }
        *file_pair.entry((src_file, target.file)).or_default() += count;

        // The same edge at item precision, for the map's lifting. A reference
        // written inside an impl block belongs to the type that impl names —
        // which is how `impl Trait for Type` becomes a type → trait tie.
        let from = src_item.and_then(|l| {
            mark_of[src_file as usize][l as usize]
                .or_else(|| impl_self.get(&(src_file, l)).copied())
        });
        let to = target
            .item
            .and_then(|l| mark_of[target.file as usize][l as usize]);
        // An impl's mention of its own self type is not a reference to it.
        if from.is_some() && from == to {
            continue;
        }
        *edge_acc
            .entry((src_file, from, target.file, to))
            .or_default() += count;
    }

    let mut fan_in: Vec<u32> = vec![0; mark_at.len()];
    let mut item_edges: Vec<ItemEdge> = edge_acc
        .into_iter()
        .map(|((from_file, from, to_file, to), count)| {
            if let Some(to) = to {
                fan_in[to as usize] += count;
            }
            ItemEdge {
                from_file,
                from,
                to_file,
                to,
                count,
            }
        })
        .collect();
    item_edges.sort_by_key(|e| (e.from_file, e.from, e.to_file, e.to));

    let mut local_refs: Vec<MarkRef> = local_acc
        .into_iter()
        .map(|((from, to), count)| MarkRef { from, to, count })
        .collect();
    local_refs.sort_by_key(|r| (r.from, r.to));

    // The landmarks themselves, with the trait impls written for them
    // anywhere in the workspace.
    let mut impls_of: Vec<Vec<String>> = vec![Vec::new(); mark_at.len()];
    for (mark, header) in impl_traits {
        impls_of[mark as usize].push(header);
    }
    let mut items: Vec<ItemMark> = Vec::with_capacity(mark_at.len());
    for (id, &(fi, li)) in mark_at.iter().enumerate() {
        let item = &raw[fi as usize].items[li as usize];
        let mut impls = std::mem::take(&mut impls_of[id]);
        impls.sort();
        impls.dedup();
        items.push(ItemMark {
            id: id as u32,
            file: fi,
            local: li,
            name: item.name.clone(),
            label: item_label(item),
            kind: item.kind,
            vis: item.vis,
            line: starts[fi as usize].line(item.range.start()),
            parent: parent_of[id],
            fan_in: fan_in[id],
            impls,
            field_rows: std::mem::take(&mut walk.field_rows[id]),
            variants: std::mem::take(&mut walk.variants[id]),
            ty: std::mem::take(&mut walk.ty[id]),
            method_rows: std::mem::take(&mut walk.method_rows[id]),
            // The structural diff writes these once the graph stands.
            delta: crate::api::Delta::Same,
            fields_added: Vec::new(),
            fields_removed: Vec::new(),
            variants_added: Vec::new(),
            variants_removed: Vec::new(),
            methods_added: Vec::new(),
            methods_removed: Vec::new(),
        });
    }

    let mut in_files: HashMap<u32, u32> = HashMap::new();
    for &(_, to) in file_pair.keys() {
        *in_files.entry(to).or_default() += 1;
    }

    // The epoch's touch — the same diff the crate altitude reads. The full
    // diff stays around: the structural pass below reads base editions of the
    // changed files through it.
    let diff = super::vcs::Diff::detect(dir);
    let changed: HashSet<String> = diff.changed_files.iter().cloned().collect();

    let files: Vec<FileInfo> = raw
        .iter()
        .enumerate()
        .map(|(i, f)| FileInfo {
            id: i as u32,
            path: f.path.clone(),
            krate: f.krate.clone(),
            changed: changed.contains(&f.path),
            lines: f.lines,
            items: f
                .items
                .iter()
                .filter(|it| it.kind != ItemKind::Impl)
                .count() as u32,
            refs_in_files: in_files.get(&(i as u32)).copied().unwrap_or(0),
        })
        .collect();

    let mut refs: Vec<FileRef> = file_pair
        .into_iter()
        .map(|((from, to), count)| FileRef { from, to, count })
        .collect();
    refs.sort_by_key(|r| (r.from, r.to));

    let details: Vec<FileDetail> = raw
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let lines = &starts[i];
            FileDetail {
                file: i as u32,
                items: f
                    .items
                    .iter()
                    .enumerate()
                    .map(|(id, it)| ItemInfo {
                        id: id as u32,
                        name: it.name.clone(),
                        section: it.section.clone(),
                        kind: it.kind,
                        line: lines.line(it.range.start()),
                        vis: it.vis,
                        mark: mark_of[i][id],
                        start: u32::from(it.range.start()),
                        end: u32::from(it.range.end()),
                    })
                    .collect(),
                item_refs: std::mem::take(&mut item_refs[i]),
            }
        })
        .collect();

    // Two lists, because a legend should state the limits of the ink its own
    // chart draws: references are the code map's whole subject and the dashed
    // ink at the two altitudes above it, while the holds walk is theirs alone.
    // Said once here, in the survey's own words, so no legend paraphrases it.
    let mut notes = vec![
        "rust-analyzer resolves every reference; only references between \
         workspace files are charted"
            .to_string(),
        "a derive macro writes no reference the survey can count — a type's \
         derives stand in its own source, on its plate"
            .to_string(),
        "a name inside a string a macro rewrites — an rsx! text node's \
         `\"{words(x)}\"` — keeps no trail back into the literal, so it \
         cannot be placed and is not counted; a format string's own captures \
         (`\"{LIMIT}\"`) are"
            .to_string(),
        "`impl Trait for Type` counts as a reference from the type to the \
         trait; the impl block holds no ground of its own"
            .to_string(),
    ];
    if !proc_macros {
        notes.push(
            "proc macros were not expanded — references inside macro output \
             are not counted"
                .to_string(),
        );
    }
    if unresolved > 0 {
        notes.push(format!(
            "{unresolved} names could not be resolved (type-inference limits) \
             and are not on the chart"
        ));
    }
    // Never a silent cut: what the survey declines to read, it says.
    let test_decls: usize = raw.iter().map(|file| file.test_ranges.len()).sum();
    if test_decls > 0 {
        notes.push(format!(
            "{test_decls} test-only declarations, and everything written \
             inside them, are not surveyed — set SLOPE_TESTS=1 to chart them"
        ));
    }

    let walk_notes = vec![
        "the walk reads declared types: `Arc`, `Rc`, `Weak` and the dioxus \
         signals as sharing, a reference as borrowing, `dyn Trait` as its \
         trait; every other generic — `Vec`, `Box`, `Mutex`, an unknown \
         external — is transparent, and the walk recurses through it"
            .to_string(),
        "a type parameter, a trait bound and an `impl Trait` are holes: the \
         row quotes them, the walk reads nothing through them. a field whose \
         walk reaches no workspace type is a plain field, and draws no line"
            .to_string(),
        "a signature is walked like a field — parameters and return type \
         both. a method's belongs to the type its impl names, and no function \
         body is on either chart"
            .to_string(),
        "what a macro declares, the survey cannot read: a type's derived \
         impls stand on its definition plate, and nothing here counts them"
            .to_string(),
    ];

    let mut graph = CodeGraph {
        files,
        refs,
        items,
        implements,
        item_edges,
        local_refs,
        holds: walk.holds,
        ghosts: Vec::new(),
        unresolved,
        notes,
        walk_notes,
    };
    // The structural diff: per-declaration deltas, ghosts for what the base
    // had, and added/removed hold events, read syntactically from the base.
    super::basediff::apply(dir, &diff, &mut graph, &sources, &details);

    Ok(CodeIndex {
        graph,
        details,
        sources,
        ref_spans,
    })
}

/// Canonicalize for prefix-stripping; fall back to the path as given.
fn dunce_canonical(dir: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

/// A vfs file's path relative to the workspace root — `None` for anything
/// outside it (sysroot, registry deps, `target/` build output).
fn workspace_rel(vfs: &Vfs, fid: FileId, root: &std::path::Path) -> Option<String> {
    let vpath = vfs.file_path(fid);
    let abs = vpath.as_path()?;
    let rel = std::path::Path::new(abs.as_str()).strip_prefix(root).ok()?;
    let rel = rel.to_string_lossy().into_owned();
    if rel.starts_with("target/") || !rel.ends_with(".rs") {
        return None;
    }
    Some(rel)
}

/// `"Trail::note"` for items in a section, plain name otherwise.
fn item_label(item: &RawItem) -> String {
    if item.section.is_empty() {
        item.name.clone()
    } else {
        format!("{}::{}", section_type(&item.section), item.name)
    }
}

/// The self type of a section header: `impl Clone for Trail` → `Trail`.
fn section_type(section: &str) -> &str {
    let s = section
        .rsplit_once(" for ")
        .map(|(_, ty)| ty)
        .unwrap_or(section);
    s.strip_prefix("impl ")
        .or_else(|| s.strip_prefix("trait "))
        .unwrap_or(s)
}

/// Collapse whitespace and cap length, for impl headers used as names. The
/// only place the survey rewrites source text: an impl header is a label,
/// and a label is one line. Everything a reader reads as code — an item's
/// definition — is quoted whole instead, by [`item_source`].
fn compact(text: impl ToString) -> String {
    const CAP: usize = 48;
    let text = text.to_string();
    let mut out = String::new();
    let mut last_ws = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !last_ws && !out.is_empty() {
                out.push(' ');
            }
            last_ws = true;
        } else {
            out.push(c);
            last_ws = false;
        }
    }
    if out.len() > CAP {
        let mut cut = CAP;
        while !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
        out.push('…');
    }
    out
}

/// Visibility as declared. `pub(crate)`, `pub(super)`, and `pub(in path)`
/// stop at the crate boundary and must not read as `pub`; `pub(self)` is no
/// wider than no `pub` at all.
pub(super) fn vis_kind(vis: Option<ast::Visibility>) -> Vis {
    match vis.map(|v| v.kind()) {
        None | Some(VisibilityKind::PubSelf) => Vis::Private,
        Some(VisibilityKind::Pub) => Vis::Pub,
        Some(_) => Vis::Crate,
    }
}

fn impl_header(i: &ast::Impl) -> String {
    let self_ty = i
        .self_ty()
        .map(|t| compact(t.syntax().text()))
        .unwrap_or_default();
    match i.trait_() {
        Some(t) => format!("impl {} for {}", compact(t.syntax().text()), self_ty),
        None => format!("impl {self_ty}"),
    }
}

/// What the item walk carries down the tree.
struct ItemScope {
    /// Inline-module path (`tests::`).
    prefix: String,
    /// The enclosing impl or trait header, for display.
    section: String,
    /// Source range of the enclosing impl or trait item.
    owner: Option<TextRange>,
    /// Visibility items inherit when they declare none: a trait's items are
    /// as visible as the trait. An impl's items keep what they declare —
    /// a trait impl's methods read as private and fold into their type,
    /// which is exactly where the reader looks for them.
    inherited: Option<Vis>,
}

impl ItemScope {
    fn root() -> Self {
        Self {
            prefix: String::new(),
            section: String::new(),
            owner: None,
            inherited: None,
        }
    }

    fn item(&self, name: &str, kind: ItemKind, range: TextRange, vis: Vis) -> RawItem {
        RawItem {
            name: format!("{}{name}", self.prefix),
            section: self.section.clone(),
            kind,
            range,
            vis,
            owner: self.owner,
        }
    }

    fn vis(&self, vis: Option<ast::Visibility>) -> Vis {
        match vis_kind(vis) {
            Vis::Private => self.inherited.unwrap_or(Vis::Private),
            declared => declared,
        }
    }
}

/// Whether a declaration exists only for the test build: `#[cfg(test)]` in
/// any of its shapes, or a `#[test]` function. `#[cfg(any(test, …))]` is not
/// here — that code ships under the other predicate too.
pub(super) fn test_only(node: &SyntaxNode) -> bool {
    node.children()
        .filter(|child| child.kind() == SyntaxKind::ATTR)
        .any(|attr| {
            let text: String = attr
                .text()
                .to_string()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            text == "#[test]"
                || text.ends_with("::test]")
                || text.starts_with("#[cfg(test")
                || text.starts_with("#[cfg(all(test")
        })
}

/// Collect the file's items in tree order. Inline modules contribute their
/// path to item names but are not containers at this altitude: their items
/// stay on the file's own shelf.
///
/// Test-only declarations are left out unless [`super::charts_tests`] says
/// otherwise, and their ranges go to `skipped` so the reference pass can drop
/// what they wrote too — a fixture calling a function is not a use of it.
fn collect_items(
    node: &SyntaxNode,
    ctx: &ItemScope,
    out: &mut Vec<RawItem>,
    skipped: &mut Vec<TextRange>,
) {
    for child in node.children() {
        if !super::charts_tests() && test_only(&child) {
            skipped.push(child.text_range());
            continue;
        }
        let name_of = |n: Option<ast::Name>| n.map(|n| n.text().to_string());
        let range = child.text_range();

        if let Some(f) = ast::Fn::cast(child.clone()) {
            if let Some(name) = name_of(f.name()) {
                out.push(ctx.item(&name, ItemKind::Fn, range, ctx.vis(f.visibility())));
            }
        } else if let Some(s) = ast::Struct::cast(child.clone()) {
            if let Some(name) = name_of(s.name()) {
                out.push(ctx.item(&name, ItemKind::Struct, range, ctx.vis(s.visibility())));
            }
        } else if let Some(e) = ast::Enum::cast(child.clone()) {
            if let Some(name) = name_of(e.name()) {
                out.push(ctx.item(&name, ItemKind::Enum, range, ctx.vis(e.visibility())));
            }
        } else if let Some(u) = ast::Union::cast(child.clone()) {
            if let Some(name) = name_of(u.name()) {
                out.push(ctx.item(&name, ItemKind::Union, range, ctx.vis(u.visibility())));
            }
        } else if let Some(t) = ast::Trait::cast(child.clone()) {
            if let Some(name) = name_of(t.name()) {
                let vis = ctx.vis(t.visibility());
                out.push(ctx.item(&name, ItemKind::Trait, range, vis));
                if let Some(list) = t.assoc_item_list() {
                    let inner = ItemScope {
                        prefix: ctx.prefix.clone(),
                        section: format!("trait {}{name}", ctx.prefix),
                        owner: Some(range),
                        inherited: Some(vis),
                    };
                    collect_items(list.syntax(), &inner, out, skipped);
                }
            }
        } else if let Some(a) = ast::TypeAlias::cast(child.clone()) {
            if let Some(name) = name_of(a.name()) {
                let vis = ctx.vis(a.visibility());
                out.push(ctx.item(&name, ItemKind::TypeAlias, range, vis));
            }
        } else if let Some(c) = ast::Const::cast(child.clone()) {
            let name = name_of(c.name()).unwrap_or_else(|| "_".to_string());
            let vis = ctx.vis(c.visibility());
            out.push(ctx.item(&name, ItemKind::Const, range, vis));
        } else if let Some(s) = ast::Static::cast(child.clone()) {
            if let Some(name) = name_of(s.name()) {
                let vis = ctx.vis(s.visibility());
                out.push(ctx.item(&name, ItemKind::Static, range, vis));
            }
        } else if let Some(m) = ast::MacroRules::cast(child.clone()) {
            if let Some(name) = name_of(m.name()) {
                // `macro_rules!` carries no `pub`; `#[macro_export]` is what
                // opens it, and that is an attribute, not a visibility.
                let exported = child
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::ATTR)
                    .any(|a| a.text().to_string().contains("macro_export"));
                let vis = if exported { Vis::Pub } else { Vis::Private };
                out.push(ctx.item(&name, ItemKind::Macro, range, vis));
            }
        } else if let Some(m) = ast::Module::cast(child.clone()) {
            // Only an inline module is a landmark. `mod x;` declares the file
            // beside it, which the chart already draws as its own block; a
            // reference to that module lands on the file, so a mark here could
            // only ever read "no references".
            if let Some(name) = name_of(m.name())
                && let Some(list) = m.item_list()
            {
                let vis = ctx.vis(m.visibility());
                out.push(ctx.item(&name, ItemKind::Mod, range, vis));
                let inner = ItemScope {
                    prefix: format!("{}{name}::", ctx.prefix),
                    section: ctx.section.clone(),
                    owner: ctx.owner,
                    inherited: ctx.inherited,
                };
                collect_items(list.syntax(), &inner, out, skipped);
            }
        } else if let Some(i) = ast::Impl::cast(child.clone()) {
            let header = impl_header(&i);
            let mut item = ctx.item(&header, ItemKind::Impl, range, Vis::Private);
            // The header names the impl whole; the inline-module prefix on it
            // would say nothing a reader wants.
            item.name = header.clone();
            out.push(item);
            if let Some(list) = i.assoc_item_list() {
                let inner = ItemScope {
                    prefix: ctx.prefix.clone(),
                    section: header,
                    owner: Some(range),
                    inherited: None,
                };
                collect_items(list.syntax(), &inner, out, skipped);
            }
        } else if let Some(x) = ast::ExternBlock::cast(child.clone())
            && let Some(list) = x.extern_item_list()
        {
            collect_items(list.syntax(), ctx, out, skipped);
        }
    }
}

/// The innermost item whose range contains `offset`. Items are sorted by
/// (start asc, end desc), so scanning backwards from the last item starting
/// at or before the offset finds the innermost container first.
fn item_at(items: &[RawItem], offset: TextSize) -> Option<u32> {
    let idx = items.partition_point(|i| i.range.start() <= offset);
    items[..idx]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, i)| i.range.end() >= offset)
        .map(|(i, _)| i as u32)
}

/// What one name in the source turned out to be.
enum Resolved {
    /// It resolved to something this survey charts.
    Target(RefTarget),
    /// It resolved, but not to anything the chart holds: a local, a `Vec`, a
    /// dependency's type, or a path qualifier the segment it qualifies already
    /// speaks for.
    Elsewhere,
    /// A name that should have resolved and did not.
    Missed,
}

/// Walk one file's syntax tree and record every reference that resolves to a
/// workspace file. Every recorded reference lands in `spans` too, keyed by its
/// own name token in the real file, so the focus plate can turn it into a link.
#[allow(clippy::too_many_arguments)]
fn scan_refs(
    sema: &Semantics<'_, RootDatabase>,
    db: &RootDatabase,
    vfs: &Vfs,
    root: &std::path::Path,
    file_of: &HashMap<FileId, u32>,
    raw: &[RawFile],
    src_file: u32,
    node: &SyntaxNode,
    acc: &mut HashMap<RefSource, u32>,
    unresolved: &mut u32,
    spans: &mut Vec<(u32, u32, RefTarget)>,
) {
    let src_items = &raw[src_file as usize].items;
    let test_ranges = &raw[src_file as usize].test_ranges;
    fn record(
        acc: &mut HashMap<RefSource, u32>,
        spans: &mut Vec<(u32, u32, RefTarget)>,
        src_items: &[RawItem],
        test_ranges: &[TextRange],
        src_file: u32,
        at: TextRange,
        target: RefTarget,
    ) {
        // A name written inside a declaration the walk left out is not a
        // reference the chart has anywhere to draw from.
        if test_ranges.iter().any(|range| range.contains(at.start())) {
            return;
        }
        let src_item = item_at(src_items, at.start());
        // A reference from an item to itself (recursion, `Self` in an impl)
        // says nothing at any zoom level.
        if target.file == src_file && src_item.is_some() && src_item == target.item {
            return;
        }
        *acc.entry((src_file, src_item, target)).or_default() += 1;
        spans.push((at.start().into(), at.end().into(), target));
    }

    for tok in node
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == SyntaxKind::IDENT)
    {
        // A declaration's own name is not a reference to anything.
        if tok.parent().is_some_and(|p| p.kind() == SyntaxKind::NAME) {
            continue;
        }
        match resolve_name(sema, db, vfs, root, file_of, raw, &tok) {
            Resolved::Target(target) => {
                record(
                    acc,
                    spans,
                    src_items,
                    test_ranges,
                    src_file,
                    tok.text_range(),
                    target,
                );
            }
            Resolved::Elsewhere => {}
            Resolved::Missed => {
                if countable(&tok) {
                    *unresolved += 1;
                }
            }
        }
    }

    // A name a format string captures — `format!("{LIMIT}")` — sits inside a
    // string literal, so no name token on the tree stands for it. The template's
    // own parts carry the resolution, already ranged in this file.
    for tok in node
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| t.kind() == SyntaxKind::STRING)
    {
        let Some(string) = ast::String::cast(tok) else {
            continue;
        };
        let Some(parts) = sema.as_format_args_parts(&string) else {
            continue;
        };
        for (at, res) in parts {
            let Some(PathResolution::Def(def)) = res.and_then(|part| part.left()) else {
                continue;
            };
            if let Some(target) = def_target(sema, db, vfs, root, file_of, raw, def) {
                record(acc, spans, src_items, test_ranges, src_file, at, target);
            }
        }
    }
}

/// Where one name resolves, read wherever rust-analyzer really resolved it.
///
/// A name written in plain code resolves where it stands. Two whole classes of
/// name never do: a name inside a macro's arguments is not parsed as a path at
/// all — the arguments are one unread token tree, and `rsx!` bodies are where
/// most of this app's references live — and the body of a function an attribute
/// macro rewrote (`#[component]`, `#[server]`) carries no inference on the real
/// tree, because the body that was type-checked is the expansion's. Both only
/// resolve inside the expansion, so a name the real tree cannot answer for is
/// descended into the macros it reaches and read there. Either way the token
/// keeps its own range in the real file, so every span the plate links is one a
/// reader can point at.
fn resolve_name(
    sema: &Semantics<'_, RootDatabase>,
    db: &RootDatabase,
    vfs: &Vfs,
    root: &std::path::Path,
    file_of: &HashMap<FileId, u32>,
    raw: &[RawFile],
    tok: &SyntaxToken,
) -> Resolved {
    // In place first: it costs nothing and answers for most of the code.
    if let Some(named) = resolve_at(sema, db, vfs, root, file_of, raw, tok) {
        return named;
    }
    for down in sema.descend_into_macros_exact(tok.clone()) {
        if let Some(named) = resolve_at(sema, db, vfs, root, file_of, raw, &down) {
            return named;
        }
    }
    Resolved::Missed
}

/// Read one name token exactly where it sits. `None` means this position has no
/// answer — the token is not a reference here, or resolution failed and an
/// expansion may still know.
#[allow(clippy::too_many_arguments)]
fn resolve_at(
    sema: &Semantics<'_, RootDatabase>,
    db: &RootDatabase,
    vfs: &Vfs,
    root: &std::path::Path,
    file_of: &HashMap<FileId, u32>,
    raw: &[RawFile],
    tok: &SyntaxToken,
) -> Option<Resolved> {
    let landed = |def: ModuleDef| match def_target(sema, db, vfs, root, file_of, raw, def) {
        Some(target) => Resolved::Target(target),
        None => Resolved::Elsewhere,
    };
    let name_ref = tok.parent().and_then(ast::NameRef::cast)?;
    let parent = name_ref.syntax().parent()?;
    match parent.kind() {
        SyntaxKind::METHOD_CALL_EXPR => {
            let call = ast::MethodCallExpr::cast(parent)?;
            // `a.b(c)` also holds `c`; only the method's own name is the call.
            if !same_node(call.name_ref(), &name_ref) {
                return Some(Resolved::Elsewhere);
            }
            let f = sema.resolve_method_call(&call)?;
            Some(landed(f.into()))
        }
        SyntaxKind::FIELD_EXPR => {
            let field = ast::FieldExpr::cast(parent)?;
            if !same_node(field.name_ref(), &name_ref) {
                return Some(Resolved::Elsewhere);
            }
            // Tuple-field access (`pair.0`) has no named target; only a real
            // field names its parent type.
            let Some(f) = sema.resolve_field(&field)?.left() else {
                return Some(Resolved::Elsewhere);
            };
            let adt: Adt = match f.parent_def(db) {
                Variant::Struct(s) => Adt::Struct(s),
                Variant::Union(u) => Adt::Union(u),
                Variant::EnumVariant(v) => Adt::Enum(v.parent_enum(db)),
            };
            Some(landed(adt.into()))
        }
        SyntaxKind::PATH_SEGMENT => {
            let seg = ast::PathSegment::cast(parent)?;
            // Only the last segment of `a::b::c` names the target; the
            // qualifiers are the same path's own children.
            let mut path = seg.parent_path();
            while let Some(up) = path.parent_path() {
                path = up;
            }
            if !same_node(path.segment(), &seg) {
                return Some(Resolved::Elsewhere);
            }
            match sema.resolve_path(&path)? {
                PathResolution::Def(def) => Some(landed(def)),
                PathResolution::SelfType(imp) => Some(match imp.self_ty(db).as_adt() {
                    Some(adt) => landed(adt.into()),
                    None => Resolved::Elsewhere,
                }),
                _ => Some(Resolved::Elsewhere),
            }
        }
        // A name in any other position — a struct literal's field, a record
        // pattern's — is charted by the path beside it, not twice.
        _ => Some(Resolved::Elsewhere),
    }
}

/// Whether two optional nodes are the same node.
fn same_node<A: AstNode, B: AstNode>(a: Option<A>, b: &B) -> bool {
    a.is_some_and(|a| a.syntax() == b.syntax())
}

/// Whether a name that failed to resolve is one the survey should own up to.
/// Expressions, types, and use trees are names that should have resolved;
/// attribute paths and macro fragments miss for reasons that are not type
/// inference's fault.
fn countable(tok: &SyntaxToken) -> bool {
    let Some(node) = tok.parent() else {
        return false;
    };
    node.ancestors().any(|a| {
        matches!(
            a.kind(),
            SyntaxKind::PATH_EXPR
                | SyntaxKind::PATH_TYPE
                | SyntaxKind::USE_TREE
                | SyntaxKind::RECORD_EXPR
                | SyntaxKind::PATH_PAT
        )
    }) && !node.ancestors().any(|a| a.kind() == SyntaxKind::ATTR)
}

/// Where a definition lives, if it lives in a workspace file.
fn def_target(
    sema: &Semantics<'_, RootDatabase>,
    db: &RootDatabase,
    vfs: &Vfs,
    root: &std::path::Path,
    file_of: &HashMap<FileId, u32>,
    raw: &[RawFile],
    def: ModuleDef,
) -> Option<RefTarget> {
    // A module reference targets the module's own file, whole.
    if let ModuleDef::Module(m) = def {
        return match m.as_source_file_id(db) {
            Some(efid) => {
                let file = *file_of.get(&efid.file_id(db))?;
                Some(RefTarget { file, item: None })
            }
            // An inline module targets the `mod` item in its parent file.
            None => {
                let range = m.definition_source_range(db);
                target_at(db, vfs, root, file_of, raw, range)
            }
        };
    }
    // An enum variant is charted as a reference to its enum.
    let def = match def {
        ModuleDef::EnumVariant(v) => ModuleDef::Adt(Adt::Enum(v.parent_enum(db))),
        other => other,
    };
    let range: InFile<TextRange> = match def {
        ModuleDef::Function(f) => src_range(sema, f)?,
        ModuleDef::Adt(Adt::Struct(s)) => src_range(sema, s)?,
        ModuleDef::Adt(Adt::Union(u)) => src_range(sema, u)?,
        ModuleDef::Adt(Adt::Enum(e)) => src_range(sema, e)?,
        ModuleDef::Const(c) => src_range(sema, c)?,
        ModuleDef::Static(s) => src_range(sema, s)?,
        ModuleDef::Trait(t) => src_range(sema, t)?,
        ModuleDef::TypeAlias(t) => src_range(sema, t)?,
        ModuleDef::Macro(m) => src_range(sema, m)?,
        ModuleDef::EnumVariant(_) | ModuleDef::Module(_) | ModuleDef::BuiltinType(_) => {
            return None;
        }
    };
    target_at(db, vfs, root, file_of, raw, range)
}

fn src_range<D>(sema: &Semantics<'_, RootDatabase>, def: D) -> Option<InFile<TextRange>>
where
    D: HasSource,
    D::Ast: AstNode,
{
    let src = sema.source(def)?;
    Some(src.map(|ast| ast.syntax().text_range()))
}

/// Map a definition's source range back to a real file and the item that
/// contains it. Definitions inside macro expansions map to the expansion's
/// call site; when the range cannot be mapped, the file still counts with no
/// item attribution.
fn target_at(
    db: &RootDatabase,
    vfs: &Vfs,
    root: &std::path::Path,
    file_of: &HashMap<FileId, u32>,
    raw: &[RawFile],
    range: InFile<TextRange>,
) -> Option<RefTarget> {
    let (fid, at) = match range.file_id.file_id() {
        Some(efid) => (efid.file_id(db), Some(range.value.start())),
        None => {
            // A definition inside a macro expansion maps back to the call
            // site in the real file.
            let (rooted, _ctx) =
                InFile::new(range.file_id, range.value).original_node_file_range(db);
            (rooted.file_id.file_id(db), Some(rooted.range.start()))
        }
    };
    let _ = (vfs, root);
    let file = *file_of.get(&fid)?;
    let item = at.and_then(|off| item_at(&raw[file as usize].items, off));
    Some(RefTarget { file, item })
}

// ---------------------------------------------------------------------------
// The definition plate: an item's own source, quoted.
// ---------------------------------------------------------------------------

impl CodeIndex {
    /// One item's source text, dedented and lexed into coloured runs — what Go to
    /// Definition lands on. The plate quotes the file whole: nothing is rewritten
    /// or cut. Every run whose name resolved to something in the workspace
    /// carries a link, so the quoted code navigates like the code it quotes.
    pub(crate) fn item_source(&self, file: u32, item: u32) -> Option<ItemSource> {
        let info = self.details.get(file as usize)?.items.get(item as usize)?;
        let text = self.sources.get(file as usize)?;
        let path = self.graph.files.get(file as usize)?.path.clone();

        let (start, end) = (info.start as usize, info.end as usize);
        if start > end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return None;
        }

        // A method inside an impl starts mid-line: give it back the indent its own
        // line begins with before stripping what every line shares, or its first
        // line hangs out to the left of its body.
        let bol = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let indent = &text[bol..start];
        let restored = indent.is_empty() || indent.chars().all(char::is_whitespace);
        let mut snippet = String::with_capacity(end - start + indent.len());
        if restored {
            snippet.push_str(indent);
        }
        snippet.push_str(&text[start..end]);
        let common = common_indent(&snippet);
        let mut lines: Vec<Vec<SrcRun>> = lex_lines(&dedent(&snippet, common))
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|(text, tok)| SrcRun {
                        text,
                        tok,
                        link: None,
                    })
                    .collect()
            })
            .collect();

        // Attach the file's clickable spans to the runs they name. A span's
        // position translates from file bytes to plate bytes by the line it is on
        // and the indent the plate stripped; a span that does not land cleanly on
        // one run is silently left unlinked rather than guessed at.
        let own_label = if info.section.is_empty() {
            info.name.clone()
        } else {
            format!("{}::{}", section_type(&info.section), info.name)
        };
        let starts = LineStarts::new(text);
        let mut links: Vec<SrcLink> = Vec::new();
        let mut link_of: HashMap<(u32, String), u32> = HashMap::new();
        for span in &self.ref_spans[file as usize] {
            if (span.start as usize) < start || (span.end as usize) > end {
                continue;
            }
            // The quoted item linking to itself would navigate nowhere.
            if span.file == file && span.label == own_label {
                continue;
            }
            let file_line = starts.line(TextSize::new(span.start));
            let Some(li) = file_line.checked_sub(info.line) else {
                continue;
            };
            let Some(line) = lines.get_mut(li as usize) else {
                continue;
            };
            // When the first line's indent was not restored (non-whitespace
            // before the item on its own line), its columns are shifted; skip
            // rather than mislink.
            if li == 0 && !restored {
                continue;
            }
            let Some(col) = starts
                .start_of(file_line)
                .and_then(|ls| span.start.checked_sub(ls))
                .and_then(|c| c.checked_sub(common as u32))
            else {
                continue;
            };
            let (col, len) = (col as usize, (span.end - span.start) as usize);
            // The name token lexes as exactly one run, so full containment is the
            // normal case; runs are never split.
            let mut at = 0usize;
            for run in line.iter_mut() {
                if at >= col && at + run.text.len() <= col + len {
                    let key = (span.file, span.label.clone());
                    let id = *link_of.entry(key).or_insert_with(|| {
                        links.push(SrcLink {
                            path: self.graph.files[span.file as usize].path.clone(),
                            label: span.label.clone(),
                        });
                        links.len() as u32 - 1
                    });
                    run.link = Some(id);
                }
                at += run.text.len();
            }
        }

        Some(ItemSource {
            path,
            first_line: info.line,
            lines,
            links,
        })
    }
}

/// The indent every non-blank line shares, in bytes — what [`dedent`] strips.
fn common_indent(text: &str) -> usize {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0)
}

/// Strip the indent every line shares, so a method quoted out of its impl
/// block starts at the plate's left edge instead of four spaces into it.
fn dedent(text: &str, common: usize) -> String {
    if common == 0 {
        return text.to_string();
    }
    text.lines()
        .map(|line| line.get(common..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Lex a snippet into per-line runs. A fragment that does not parse as a whole
/// file is fine: the parser always produces a tree, and the token stream is
/// the source text either way.
fn lex_lines(snippet: &str) -> Vec<Vec<(String, Tok)>> {
    let parsed = ra_ap_syntax::SourceFile::parse(snippet, Edition::CURRENT);
    let tokens: Vec<SyntaxToken> = parsed
        .syntax_node()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .collect();

    let mut lines: Vec<Vec<(String, Tok)>> = vec![Vec::new()];
    for (i, token) in tokens.iter().enumerate() {
        let class = classify(&tokens, i);
        for (n, part) in token.text().split('\n').enumerate() {
            if n > 0 {
                lines.push(Vec::new());
            }
            let part = part.trim_end_matches('\r');
            if !part.is_empty() {
                lines
                    .last_mut()
                    .expect("a line is always open")
                    .push((part.to_string(), class));
            }
        }
    }
    if lines.len() > 1 && lines.last().is_some_and(Vec::is_empty) {
        lines.pop();
    }
    lines
}

/// The nearest token before `i` that is neither whitespace nor a comment.
fn before(tokens: &[SyntaxToken], i: usize) -> Option<SyntaxKind> {
    tokens[..i]
        .iter()
        .rev()
        .map(SyntaxToken::kind)
        .find(|kind| !kind.is_trivia())
}

/// The nearest token after `i` that is neither whitespace nor a comment.
fn after(tokens: &[SyntaxToken], i: usize) -> Option<SyntaxKind> {
    tokens[i + 1..]
        .iter()
        .map(SyntaxToken::kind)
        .find(|kind| !kind.is_trivia())
}

/// What one token is, for colouring. Rust's own categories: the palette that
/// draws them is the client's business.
fn classify(tokens: &[SyntaxToken], i: usize) -> Tok {
    let token = &tokens[i];
    let kind = token.kind();
    if kind == SyntaxKind::WHITESPACE {
        return Tok::Space;
    }
    if kind == SyntaxKind::COMMENT {
        let text = token.text();
        let doc = ["///", "//!", "/**", "/*!"]
            .iter()
            .any(|p| text.starts_with(p));
        return if doc { Tok::Doc } else { Tok::Comment };
    }
    // An attribute is one thing to a reader — `#[derive(Clone, Copy)]` reads
    // as a unit — so everything inside it takes one class.
    if token
        .parent_ancestors()
        .any(|n| n.kind() == SyntaxKind::ATTR)
    {
        return Tok::Attr;
    }
    if kind.is_literal() {
        return match kind {
            SyntaxKind::INT_NUMBER | SyntaxKind::FLOAT_NUMBER => Tok::Num,
            _ => Tok::Str,
        };
    }
    if kind == SyntaxKind::LIFETIME_IDENT {
        return Tok::Lifetime;
    }
    if kind.is_keyword(Edition::CURRENT) {
        return Tok::Kw;
    }
    if kind == SyntaxKind::IDENT {
        if after(tokens, i) == Some(SyntaxKind::BANG) {
            return Tok::Macro;
        }
        if before(tokens, i) == Some(SyntaxKind::FN_KW) {
            return Tok::Fn;
        }
        if token.text().starts_with(char::is_uppercase) {
            return Tok::Type;
        }
        return Tok::Ident;
    }
    if kind.is_punct() {
        return Tok::Punct;
    }
    Tok::Ident
}
