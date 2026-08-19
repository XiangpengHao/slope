//! The code survey: rust-analyzer as a library over the workspace sources.
//!
//! `cargo metadata` answers the crate altitude; this module answers the two
//! below it — files and items. It loads the workspace into a rust-analyzer
//! database, walks every workspace-member source file, collects its items
//! (functions, types, traits, impls), and resolves every reference it can:
//! paths, method calls, and field accesses, including inside macro
//! expansions. References that reach outside the workspace are dropped —
//! this altitude charts the reviewer's own code, not its dependencies.
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
use ra_ap_syntax::ast::{HasGenericParams, HasName, HasVisibility, VisibilityKind};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, TextRange, TextSize, ast};
use ra_ap_vfs::{FileId, Vfs};
use tokio::sync::OnceCell;

use crate::api::{
    CodeGraph, FileDetail, FileInfo, FileRef, ItemEdge, ItemInfo, ItemKind, ItemMark, ItemMember,
    ItemRef, ItemXRef, Vis,
};

/// The whole survey, precomputed once: the shipped graph plus every file's
/// cutaway detail, ready to answer per-file queries from memory.
pub struct CodeIndex {
    pub graph: CodeGraph,
    /// Indexed by [`FileInfo::id`].
    pub details: Vec<FileDetail>,
}

static INDEX: OnceCell<Result<Arc<CodeIndex>, String>> = OnceCell::const_new();

/// The cached survey. The first caller pays for it (tens of seconds on a
/// large workspace — rust-analyzer loads the whole workspace); everyone
/// after answers from memory.
pub async fn index() -> Result<Arc<CodeIndex>, String> {
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
    /// Struct fields or enum variants, in source order.
    members: Vec<ItemMember>,
    /// A function's signature, without its body.
    sig: Option<String>,
    derives: Vec<String>,
}

/// One file being surveyed.
struct RawFile {
    path: String,
    krate: String,
    efid: EditionedFileId,
    items: Vec<RawItem>,
    lines: u32,
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

pub fn survey(dir: &std::path::Path) -> Result<CodeIndex, String> {
    let manifest = dir.join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "No Cargo.toml found in {}. Point slopify at a cargo workspace: \
             SLOPIFY_WORKSPACE=/path/to/workspace",
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
    for file in raw.iter_mut() {
        let source = sema.parse(file.efid);
        let text = source.syntax().text().to_string();
        let lines = LineStarts::new(&text);
        file.lines = lines.count();
        starts.push(lines);
        let mut items = Vec::new();
        collect_items(source.syntax(), &ItemCtx::root(), &mut items);
        items.sort_by_key(|i| (i.range.start(), std::cmp::Reverse(i.range.end())));
        file.items = items;
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
            if let Some(t) = node.trait_() {
                impl_traits.push((mark, compact(t.syntax().text())));
            }
        }
    }

    // ---- Pass B: resolve references. --------------------------------------

    // (source file, source item, target) → count.
    let mut acc: HashMap<(u32, Option<u32>, RefTarget), u32> = HashMap::new();
    let mut unresolved: u32 = 0;

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
            None,
            0,
            &mut acc,
            &mut unresolved,
        );
    }

    // ---- Assemble the wire model. -----------------------------------------

    // Containment: an item's parent is the type its impl names, or the trait
    // that declares it. Inline modules are not containers at this altitude.
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

    let mut file_pair: HashMap<(u32, u32), u32> = HashMap::new();
    let mut item_refs: Vec<Vec<ItemRef>> = vec![Vec::new(); raw.len()];
    let mut refs_out: Vec<Vec<ItemXRef>> = vec![Vec::new(); raw.len()];
    let mut refs_in: Vec<Vec<ItemXRef>> = vec![Vec::new(); raw.len()];
    // Cross-file references at item precision, aggregated per pair. Several
    // impl blocks for one type collapse onto the same endpoint, so the pairs
    // must be summed, not pushed.
    let mut edge_acc: HashMap<(u32, Option<u32>, u32, Option<u32>), u32> = HashMap::new();

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
            }
            continue;
        }
        *file_pair.entry((src_file, target.file)).or_default() += count;
        if let Some(from) = src_item {
            let other = target
                .item
                .map(|t| item_label(&raw[target.file as usize].items[t as usize]))
                .unwrap_or_default();
            refs_out[src_file as usize].push(ItemXRef {
                item: from,
                file: target.file,
                other,
                count,
            });
        }
        if let Some(to) = target.item {
            let other = src_item
                .map(|s| item_label(&raw[src_file as usize].items[s as usize]))
                .unwrap_or_default();
            refs_in[target.file as usize].push(ItemXRef {
                item: to,
                file: src_file,
                other,
                count,
            });
        }

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

    // The landmarks themselves, with their traits gathered from every impl.
    let mut traits_of: Vec<Vec<String>> = vec![Vec::new(); mark_at.len()];
    for (mark, name) in impl_traits {
        traits_of[mark as usize].push(name);
    }
    let mut items: Vec<ItemMark> = Vec::with_capacity(mark_at.len());
    for (id, &(fi, li)) in mark_at.iter().enumerate() {
        let item = &raw[fi as usize].items[li as usize];
        let mut traits = std::mem::take(&mut traits_of[id]);
        traits.extend(item.derives.iter().cloned());
        traits.sort();
        traits.dedup();
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
            traits,
        });
    }

    let mut in_files: HashMap<u32, u32> = HashMap::new();
    let mut out_files: HashMap<u32, u32> = HashMap::new();
    for &(from, to) in file_pair.keys() {
        *out_files.entry(from).or_default() += 1;
        *in_files.entry(to).or_default() += 1;
    }

    // The epoch's touch, by path — the same diff the crate altitude reads.
    let changed: HashSet<String> = super::vcs::detect_diff(dir)
        .changed_files
        .into_iter()
        .collect();

    let files: Vec<FileInfo> = raw
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let count = |kinds: &[ItemKind]| {
                f.items.iter().filter(|it| kinds.contains(&it.kind)).count() as u32
            };
            FileInfo {
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
                fns: count(&[ItemKind::Fn]),
                types: count(&[
                    ItemKind::Struct,
                    ItemKind::Enum,
                    ItemKind::Union,
                    ItemKind::TypeAlias,
                ]),
                traits: count(&[ItemKind::Trait]),
                refs_in_files: in_files.get(&(i as u32)).copied().unwrap_or(0),
                refs_out_files: out_files.get(&(i as u32)).copied().unwrap_or(0),
            }
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
                        end_line: lines.line(it.range.end()),
                        vis: it.vis,
                        mark: mark_of[i][id],
                        members: it.members.clone(),
                        sig: it.sig.clone(),
                        derives: it.derives.clone(),
                    })
                    .collect(),
                item_refs: std::mem::take(&mut item_refs[i]),
                refs_out: std::mem::take(&mut refs_out[i]),
                refs_in: std::mem::take(&mut refs_in[i]),
            }
        })
        .collect();

    let mut notes = vec![
        "references are resolved semantically by rust-analyzer; only \
         references between workspace files are charted"
            .to_string(),
        "references produced by derive macros are not counted; the traits a \
         type derives are listed on its plate instead"
            .to_string(),
        "an `impl Trait for Type` counts as a reference from the type to the \
         trait — the impl block itself holds no ground"
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

    Ok(CodeIndex {
        graph: CodeGraph {
            files,
            refs,
            items,
            item_edges,
            unresolved,
            notes,
        },
        details,
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

/// Collapse whitespace and cap length, for impl headers used as names.
fn compact(text: impl ToString) -> String {
    compact_to(text, 48)
}

/// Collapse whitespace and cap length. Source text lands on plates, so it
/// arrives on one line or not at all.
fn compact_to(text: impl ToString, cap: usize) -> String {
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
    if out.len() > cap {
        let mut cut = cap;
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
fn vis_kind(vis: Option<ast::Visibility>) -> Vis {
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
struct ItemCtx {
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

impl ItemCtx {
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
            members: Vec::new(),
            sig: None,
            derives: Vec::new(),
        }
    }

    fn vis(&self, vis: Option<ast::Visibility>) -> Vis {
        match vis_kind(vis) {
            Vis::Private => self.inherited.unwrap_or(Vis::Private),
            declared => declared,
        }
    }
}

/// Struct fields or union fields, in source order. Tuple fields keep their
/// index as their name — `.0` is what the reader writes.
fn fields_of(list: Option<ast::FieldList>) -> Vec<ItemMember> {
    match list {
        Some(ast::FieldList::RecordFieldList(r)) => r
            .fields()
            .map(|f| ItemMember {
                name: f.name().map(|n| n.text().to_string()).unwrap_or_default(),
                ty: f
                    .ty()
                    .map(|t| compact_to(t.syntax().text(), 64))
                    .unwrap_or_default(),
                vis: vis_kind(f.visibility()),
            })
            .collect(),
        Some(ast::FieldList::TupleFieldList(t)) => t
            .fields()
            .enumerate()
            .map(|(i, f)| ItemMember {
                name: format!(".{i}"),
                ty: f
                    .ty()
                    .map(|t| compact_to(t.syntax().text(), 64))
                    .unwrap_or_default(),
                vis: vis_kind(f.visibility()),
            })
            .collect(),
        None => Vec::new(),
    }
}

/// Enum variants with their payloads. A variant is as visible as its enum.
fn variants_of(list: Option<ast::VariantList>) -> Vec<ItemMember> {
    list.map(|l| {
        l.variants()
            .map(|v| ItemMember {
                name: v.name().map(|n| n.text().to_string()).unwrap_or_default(),
                ty: v
                    .field_list()
                    .map(|f| compact_to(f.syntax().text(), 64))
                    .unwrap_or_default(),
                vis: Vis::Pub,
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Derive names, read from the attribute's own text — derive arguments are
/// plain paths, so the words are the badge.
fn derives_of(node: &SyntaxNode) -> Vec<String> {
    let mut out = Vec::new();
    for attr in node.children().filter(|c| c.kind() == SyntaxKind::ATTR) {
        let text = attr.text().to_string();
        let Some(inner) = text
            .trim()
            .strip_prefix("#[derive(")
            .and_then(|t| t.trim_end().strip_suffix(")]"))
        else {
            continue;
        };
        out.extend(
            inner
                .split(',')
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(|n| compact_to(n, 24)),
        );
    }
    out
}

/// A function's signature without its body: what the focus plate engraves in
/// place of fields.
fn fn_signature(f: &ast::Fn, name: &str) -> String {
    let mut sig = String::new();
    if let Some(v) = f.visibility() {
        sig.push_str(&compact(v.syntax().text()));
        sig.push(' ');
    }
    for (token, word) in [
        (f.default_token(), "default "),
        (f.const_token(), "const "),
        (f.async_token(), "async "),
        (f.unsafe_token(), "unsafe "),
    ] {
        if token.is_some() {
            sig.push_str(word);
        }
    }
    if let Some(abi) = f.abi() {
        sig.push_str(&compact(abi.syntax().text()));
        sig.push(' ');
    }
    sig.push_str("fn ");
    sig.push_str(name);
    if let Some(g) = f.generic_param_list() {
        sig.push_str(&compact_to(g.syntax().text(), 64));
    }
    if let Some(p) = f.param_list() {
        sig.push_str(&compact_to(p.syntax().text(), 180));
    }
    if let Some(r) = f.ret_type() {
        sig.push(' ');
        sig.push_str(&compact_to(r.syntax().text(), 96));
    }
    if let Some(w) = f.where_clause() {
        sig.push(' ');
        sig.push_str(&compact_to(w.syntax().text(), 96));
    }
    sig
}

/// Collect the file's items in tree order. Inline modules contribute their
/// path to item names but are not containers at this altitude: their items
/// stay on the file's own shelf.
fn collect_items(node: &SyntaxNode, ctx: &ItemCtx, out: &mut Vec<RawItem>) {
    for child in node.children() {
        let name_of = |n: Option<ast::Name>| n.map(|n| n.text().to_string());
        let range = child.text_range();

        if let Some(f) = ast::Fn::cast(child.clone()) {
            if let Some(name) = name_of(f.name()) {
                let mut item = ctx.item(&name, ItemKind::Fn, range, ctx.vis(f.visibility()));
                item.sig = Some(fn_signature(&f, &name));
                out.push(item);
            }
        } else if let Some(s) = ast::Struct::cast(child.clone()) {
            if let Some(name) = name_of(s.name()) {
                let mut item = ctx.item(&name, ItemKind::Struct, range, ctx.vis(s.visibility()));
                item.members = fields_of(s.field_list());
                item.derives = derives_of(&child);
                out.push(item);
            }
        } else if let Some(e) = ast::Enum::cast(child.clone()) {
            if let Some(name) = name_of(e.name()) {
                let mut item = ctx.item(&name, ItemKind::Enum, range, ctx.vis(e.visibility()));
                item.members = variants_of(e.variant_list());
                item.derives = derives_of(&child);
                out.push(item);
            }
        } else if let Some(u) = ast::Union::cast(child.clone()) {
            if let Some(name) = name_of(u.name()) {
                let mut item = ctx.item(&name, ItemKind::Union, range, ctx.vis(u.visibility()));
                item.members =
                    fields_of(u.record_field_list().map(ast::FieldList::RecordFieldList));
                item.derives = derives_of(&child);
                out.push(item);
            }
        } else if let Some(t) = ast::Trait::cast(child.clone()) {
            if let Some(name) = name_of(t.name()) {
                let vis = ctx.vis(t.visibility());
                out.push(ctx.item(&name, ItemKind::Trait, range, vis));
                if let Some(list) = t.assoc_item_list() {
                    let inner = ItemCtx {
                        prefix: ctx.prefix.clone(),
                        section: format!("trait {}{name}", ctx.prefix),
                        owner: Some(range),
                        inherited: Some(vis),
                    };
                    collect_items(list.syntax(), &inner, out);
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
            if let Some(name) = name_of(m.name()) {
                let vis = ctx.vis(m.visibility());
                out.push(ctx.item(&name, ItemKind::Mod, range, vis));
                if let Some(list) = m.item_list() {
                    let inner = ItemCtx {
                        prefix: format!("{}{name}::", ctx.prefix),
                        section: ctx.section.clone(),
                        owner: ctx.owner,
                        inherited: ctx.inherited,
                    };
                    collect_items(list.syntax(), &inner, out);
                }
            }
        } else if let Some(i) = ast::Impl::cast(child.clone()) {
            let header = impl_header(&i);
            let mut item = ctx.item(&header, ItemKind::Impl, range, Vis::Private);
            // The header names the impl whole; the inline-module prefix on it
            // would say nothing a reader wants.
            item.name = header.clone();
            out.push(item);
            if let Some(list) = i.assoc_item_list() {
                let inner = ItemCtx {
                    prefix: ctx.prefix.clone(),
                    section: header,
                    owner: Some(range),
                    inherited: None,
                };
                collect_items(list.syntax(), &inner, out);
            }
        } else if let Some(x) = ast::ExternBlock::cast(child.clone())
            && let Some(list) = x.extern_item_list()
        {
            collect_items(list.syntax(), ctx, out);
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

/// How deep to chase macro calls inside macro expansions.
const MACRO_DEPTH: usize = 3;

/// Walk one syntax tree and record every reference that resolves to a
/// workspace file. `origin` is the offset in the *real* source file that
/// references get attributed to — the node's own position on the real tree,
/// the macro call site inside expansions.
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
    origin: Option<TextSize>,
    depth: usize,
    acc: &mut HashMap<(u32, Option<u32>, RefTarget), u32>,
    unresolved: &mut u32,
) {
    let src_items = &raw[src_file as usize].items;
    fn record(
        acc: &mut HashMap<(u32, Option<u32>, RefTarget), u32>,
        unresolved: &mut u32,
        src_items: &[RawItem],
        src_file: u32,
        origin: Option<TextSize>,
        at: TextSize,
        target: Option<RefTarget>,
    ) {
        let Some(target) = target else {
            *unresolved += 1;
            return;
        };
        let attributed = origin.unwrap_or(at);
        let src_item = item_at(src_items, attributed);
        // A reference from an item to itself (recursion, `Self` in an impl)
        // says nothing at any zoom level.
        if target.file == src_file && src_item.is_some() && src_item == target.item {
            return;
        }
        *acc.entry((src_file, src_item, target)).or_default() += 1;
    }

    for desc in node.descendants() {
        match desc.kind() {
            SyntaxKind::METHOD_CALL_EXPR => {
                let Some(call) = ast::MethodCallExpr::cast(desc.clone()) else {
                    continue;
                };
                let target = sema
                    .resolve_method_call(&call)
                    .and_then(|f| def_target(sema, db, vfs, root, file_of, raw, f.into()));
                record(
                    acc,
                    unresolved,
                    src_items,
                    src_file,
                    origin,
                    desc.text_range().start(),
                    target,
                );
            }
            SyntaxKind::FIELD_EXPR => {
                let Some(field) = ast::FieldExpr::cast(desc.clone()) else {
                    continue;
                };
                // Tuple-field access (`pair.0`) has no named target; only a
                // real field names its parent type.
                let Some(f) = sema.resolve_field(&field).and_then(|e| e.left()) else {
                    continue;
                };
                let adt: Adt = match f.parent_def(db) {
                    Variant::Struct(s) => Adt::Struct(s),
                    Variant::Union(u) => Adt::Union(u),
                    Variant::EnumVariant(v) => Adt::Enum(v.parent_enum(db)),
                };
                let target = def_target(sema, db, vfs, root, file_of, raw, adt.into());
                if target.is_some() {
                    record(
                        acc,
                        unresolved,
                        src_items,
                        src_file,
                        origin,
                        desc.text_range().start(),
                        target,
                    );
                }
            }
            SyntaxKind::PATH => {
                // Only the outermost path of `a::b::c` resolves; its
                // qualifiers are children of the same PATH node.
                if desc.parent().is_some_and(|p| p.kind() == SyntaxKind::PATH) {
                    continue;
                }
                let Some(path) = ast::Path::cast(desc.clone()) else {
                    continue;
                };
                match sema.resolve_path(&path) {
                    Some(PathResolution::Def(def)) => {
                        let target = def_target(sema, db, vfs, root, file_of, raw, def);
                        if target.is_some() {
                            record(
                                acc,
                                unresolved,
                                src_items,
                                src_file,
                                origin,
                                desc.text_range().start(),
                                target,
                            );
                        }
                    }
                    Some(PathResolution::SelfType(imp)) => {
                        let target = imp.self_ty(db).as_adt().and_then(|adt| {
                            def_target(sema, db, vfs, root, file_of, raw, adt.into())
                        });
                        if target.is_some() {
                            record(
                                acc,
                                unresolved,
                                src_items,
                                src_file,
                                origin,
                                desc.text_range().start(),
                                target,
                            );
                        }
                    }
                    Some(_) => {}
                    None => {
                        // Only count contexts where a name should have
                        // resolved: expressions, types, and use trees.
                        // Attribute paths and macro fragments miss for
                        // reasons that are not type inference's fault.
                        let countable =
                            desc.ancestors().any(|a| {
                                matches!(
                                    a.kind(),
                                    SyntaxKind::PATH_EXPR
                                        | SyntaxKind::PATH_TYPE
                                        | SyntaxKind::USE_TREE
                                        | SyntaxKind::RECORD_EXPR
                                        | SyntaxKind::PATH_PAT
                                )
                            }) && !desc.ancestors().any(|a| a.kind() == SyntaxKind::ATTR);
                        if countable {
                            *unresolved += 1;
                        }
                    }
                }
            }
            SyntaxKind::MACRO_CALL => {
                if depth >= MACRO_DEPTH {
                    continue;
                }
                let Some(call) = ast::MacroCall::cast(desc.clone()) else {
                    continue;
                };
                let call_at = origin.unwrap_or_else(|| desc.text_range().start());
                if let Some(expansion) = sema.expand_macro_call(&call) {
                    scan_refs(
                        sema,
                        db,
                        vfs,
                        root,
                        file_of,
                        raw,
                        src_file,
                        &expansion.value,
                        Some(call_at),
                        depth + 1,
                        acc,
                        unresolved,
                    );
                }
            }
            _ => {}
        }
    }
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
