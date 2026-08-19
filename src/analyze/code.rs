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

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use ra_ap_hir::{Adt, Crate, HasSource, InFile, ModuleDef, PathResolution, Semantics, Variant};
use ra_ap_ide_db::RootDatabase;
use ra_ap_ide_db::base_db::EditionedFileId;
use ra_ap_load_cargo::{LoadCargoConfig, ProcMacroServerChoice, load_workspace_at};
use ra_ap_project_model::{CargoConfig, RustLibSource};
use ra_ap_syntax::ast::{HasName, HasVisibility};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, TextRange, TextSize, ast};
use ra_ap_vfs::{FileId, Vfs};
use tokio::sync::OnceCell;

use crate::api::{
    CodeGraph, FileDetail, FileInfo, FileRef, ItemInfo, ItemKind, ItemRef, ItemXRef,
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
    public: bool,
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
    ra_ap_hir::attach_db(&db, || survey_attached(dir, &db, &vfs, proc_macro.is_some()))
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

    for file in &mut raw {
        let source = sema.parse(file.efid);
        let text = source.syntax().text().to_string();
        let lines = LineStarts::new(&text);
        file.lines = lines.count();
        let mut items = Vec::new();
        collect_items(source.syntax(), "", "", &mut items);
        items.sort_by_key(|i| (i.range.start(), std::cmp::Reverse(i.range.end())));
        file.items = items;
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

    let mut file_pair: HashMap<(u32, u32), u32> = HashMap::new();
    let mut item_refs: Vec<Vec<ItemRef>> = vec![Vec::new(); raw.len()];
    let mut refs_out: Vec<Vec<ItemXRef>> = vec![Vec::new(); raw.len()];
    let mut refs_in: Vec<Vec<ItemXRef>> = vec![Vec::new(); raw.len()];

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
    }

    let mut in_files: HashMap<u32, u32> = HashMap::new();
    let mut out_files: HashMap<u32, u32> = HashMap::new();
    for &(from, to) in file_pair.keys() {
        *out_files.entry(from).or_default() += 1;
        *in_files.entry(to).or_default() += 1;
    }

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
                lines: f.lines,
                items: f.items.iter().filter(|it| it.kind != ItemKind::Impl).count() as u32,
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
            let lines = LineStarts::new(&sema.parse(f.efid).syntax().text().to_string());
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
                        public: it.public,
                    })
                    .collect(),
                item_refs: std::mem::take(&mut item_refs[i].clone()),
                refs_out: refs_out[i].clone(),
                refs_in: refs_in[i].clone(),
            }
        })
        .collect();

    let mut notes = vec![
        "references are resolved semantically by rust-analyzer; only \
         references between workspace files are charted"
            .to_string(),
        "references produced by derive macros are not counted".to_string(),
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
    if out.len() > 48 {
        let mut cut = 48;
        while !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
        out.push('…');
    }
    out
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

/// Collect the file's items in tree order. `prefix` carries inline-module
/// paths (`tests::`); `section` carries the enclosing impl/trait header.
fn collect_items(node: &SyntaxNode, prefix: &str, section: &str, out: &mut Vec<RawItem>) {
    for child in node.children() {
        let public = |vis: Option<ast::Visibility>| vis.is_some();
        let name_of = |n: Option<ast::Name>| n.map(|n| n.text().to_string());

        if let Some(f) = ast::Fn::cast(child.clone()) {
            if let Some(name) = name_of(f.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Fn,
                    range: child.text_range(),
                    public: public(f.visibility()),
                });
            }
        } else if let Some(s) = ast::Struct::cast(child.clone()) {
            if let Some(name) = name_of(s.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Struct,
                    range: child.text_range(),
                    public: public(s.visibility()),
                });
            }
        } else if let Some(e) = ast::Enum::cast(child.clone()) {
            if let Some(name) = name_of(e.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Enum,
                    range: child.text_range(),
                    public: public(e.visibility()),
                });
            }
        } else if let Some(u) = ast::Union::cast(child.clone()) {
            if let Some(name) = name_of(u.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Union,
                    range: child.text_range(),
                    public: public(u.visibility()),
                });
            }
        } else if let Some(t) = ast::Trait::cast(child.clone()) {
            if let Some(name) = name_of(t.name()) {
                let header = format!("trait {prefix}{name}");
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Trait,
                    range: child.text_range(),
                    public: public(t.visibility()),
                });
                if let Some(list) = t.assoc_item_list() {
                    collect_items(list.syntax(), prefix, &header, out);
                }
            }
        } else if let Some(a) = ast::TypeAlias::cast(child.clone()) {
            if let Some(name) = name_of(a.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::TypeAlias,
                    range: child.text_range(),
                    public: public(a.visibility()),
                });
            }
        } else if let Some(c) = ast::Const::cast(child.clone()) {
            let name = name_of(c.name()).unwrap_or_else(|| "_".to_string());
            out.push(RawItem {
                name: format!("{prefix}{name}"),
                section: section.to_string(),
                kind: ItemKind::Const,
                range: child.text_range(),
                public: public(c.visibility()),
            });
        } else if let Some(s) = ast::Static::cast(child.clone()) {
            if let Some(name) = name_of(s.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Static,
                    range: child.text_range(),
                    public: public(s.visibility()),
                });
            }
        } else if let Some(m) = ast::MacroRules::cast(child.clone()) {
            if let Some(name) = name_of(m.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Macro,
                    range: child.text_range(),
                    public: false,
                });
            }
        } else if let Some(m) = ast::Module::cast(child.clone()) {
            if let Some(name) = name_of(m.name()) {
                out.push(RawItem {
                    name: format!("{prefix}{name}"),
                    section: section.to_string(),
                    kind: ItemKind::Mod,
                    range: child.text_range(),
                    public: public(m.visibility()),
                });
                if let Some(list) = m.item_list() {
                    collect_items(list.syntax(), &format!("{prefix}{name}::"), section, out);
                }
            }
        } else if let Some(i) = ast::Impl::cast(child.clone()) {
            let header = impl_header(&i);
            out.push(RawItem {
                name: header.clone(),
                section: section.to_string(),
                kind: ItemKind::Impl,
                range: child.text_range(),
                public: false,
            });
            if let Some(list) = i.assoc_item_list() {
                collect_items(list.syntax(), prefix, &header, out);
            }
        } else if let Some(x) = ast::ExternBlock::cast(child.clone())
            && let Some(list) = x.extern_item_list()
        {
            collect_items(list.syntax(), prefix, section, out);
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
                if desc
                    .parent()
                    .is_some_and(|p| p.kind() == SyntaxKind::PATH)
                {
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
                        let countable = desc.ancestors().any(|a| {
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
