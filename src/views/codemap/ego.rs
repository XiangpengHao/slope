//! The focus plate: one selection, quoted, with one hop of its references.
//!
//! Selecting an item on the map replaces the map with this: the item's own
//! source text on the center plate — the definition, syntax-highlighted, with
//! a line-number gutter counting from its first line in the real file — with
//! what leans on it on the left and what it reaches for on the right, each
//! grouped by the file the reference is written in. One hop only, never global
//! spaghetti, and every row re-centers the plate on itself. Every focus is a
//! URL.
//!
//! Selecting a whole file gives the same plate with the file's items as an
//! outline instead of a quotation: a file has no single definition to quote.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::Route;
use crate::api::{
    CodeGraph, ItemKind, ItemMark, ItemSource, Tok, Vis, file_detail, item_source,
};
use crate::views::codemap::chrome::{decl_words, dir_of, file_name, kind_words, plural};
use crate::views::codemap::model::{self, Center, Containment, Dir, Group};
use crate::views::codemap::{file_route, item_route, use_code};

/// Rows a group shows before it defers to its own counted fold.
const ROW_CAP: usize = 3;

// TODO: the columns are joined to the center plate by the reading order alone.
// Drawn wires would need every row's box measured in the browser (refs plus a
// resize observer); descoped as decoration until the layout is settled.

/// One row of a reference column, ready to draw.
#[derive(Clone, PartialEq)]
pub struct RowView {
    /// `None` for the lifted-private line: it names nothing, by design.
    kind: Option<ItemKind>,
    name: String,
    title: String,
    to: Option<Route>,
    count: u32,
}

/// One file's rows in a reference column.
#[derive(Clone, PartialEq)]
pub struct GroupView {
    file: u32,
    path: String,
    total: u32,
    rows: Vec<RowView>,
}

/// One row of a whole file's outline: an item the file defines.
#[derive(Clone, PartialEq)]
pub struct OutlineRow {
    kind: ItemKind,
    vis: Vis,
    name: String,
    refs: u32,
    line: u32,
    to: Route,
}

/// One `impl` block's methods, gathered wherever the impl is written.
#[derive(Clone, PartialEq)]
pub struct ImplGroup {
    /// The impl header exactly as it is written: `impl Vis`, `impl Clone for Vis`.
    head: String,
    rows: Vec<ImplRow>,
}

/// One associated item under an impl header.
#[derive(Clone, PartialEq)]
pub struct ImplRow {
    kind: ItemKind,
    vis: Vis,
    name: String,
    /// `src/api.rs:165` — where this one is written.
    locator: String,
    to: Route,
}

/// The class that inks one run of quoted source. Colour lives inside the code
/// pane and says one thing only: what kind of token this is.
fn tok_class(tok: Tok) -> &'static str {
    match tok {
        Tok::Kw => "tok-kw",
        Tok::Comment => "tok-comment",
        Tok::Doc => "tok-doc",
        Tok::Str => "tok-str",
        Tok::Num => "tok-num",
        Tok::Lifetime => "tok-lifetime",
        Tok::Attr => "tok-attr",
        Tok::Type => "tok-type",
        Tok::Fn => "tok-fn",
        Tok::Macro => "tok-macro",
        Tok::Punct => "tok-punct",
        Tok::Ident | Tok::Space => "tok-ident",
    }
}

/// Turn one hop of grouped references into rows the column can draw.
fn column_views(graph: &CodeGraph, groups: Vec<Group>) -> Vec<GroupView> {
    groups
        .into_iter()
        .map(|group| {
            let path = graph
                .files
                .get(group.file as usize)
                .map(|f| f.path.clone())
                .unwrap_or_default();
            let rows = group
                .rows
                .iter()
                .map(
                    |row| match row.mark.and_then(|m| graph.items.get(m as usize)) {
                        Some(mark) => {
                            let file = graph
                                .files
                                .get(mark.file as usize)
                                .map(|f| f.path.clone())
                                .unwrap_or_default();
                            RowView {
                                kind: Some(mark.kind),
                                name: mark.name.clone(),
                                title: format!(
                                    "{} · {} · {}",
                                    mark.label,
                                    kind_words(mark.kind),
                                    mark.vis.words()
                                ),
                                to: Some(item_route(&file, &mark.label)),
                                count: row.count,
                            }
                        }
                        None => RowView {
                            kind: None,
                            name: "private items".to_string(),
                            title: "private items are never drawn; their references count \
                                against the file that holds them"
                                .to_string(),
                            to: None,
                            count: row.count,
                        },
                    },
                )
                .collect();
            GroupView {
                file: group.file,
                path,
                total: group.total,
                rows,
            }
        })
        .collect()
}

/// One breadcrumb: whole map ▸ directory ▸ file ▸ item.
#[component]
fn Crumb(path: String, item: String) -> Element {
    let dir = dir_of(&path).to_string();
    let name = file_name(&path).to_string();
    rsx! {
        nav { class: "flex flex-wrap items-baseline gap-1.5 font-data text-[10px] text-ink-soft",
            Link {
                class: "text-[9.5px] tracking-[0.14em] uppercase underline-offset-4 hover:text-ink hover:underline",
                to: Route::CodeOverview {},
                "← whole map"
            }
            if !dir.is_empty() {
                span { class: "text-ink-faint", "▸" }
                span { "{dir}" }
            }
            span { class: "text-ink-faint", "▸" }
            if item.is_empty() {
                span { class: "font-medium text-ink", "{name}" }
            } else {
                Link {
                    class: "underline-offset-4 hover:text-ink hover:underline",
                    to: file_route(&path),
                    "{name}"
                }
                span { class: "text-ink-faint", "▸" }
                span { class: "font-medium text-ink", "{item}" }
            }
        }
    }
}

/// One reference column: grouped by file, capped rows, counted folds.
#[component]
fn EgoColumn(groups: Vec<GroupView>, head: String, outgoing: bool) -> Element {
    let mut opened: Signal<HashSet<u32>> = use_signal(HashSet::new);
    let files: usize = groups.len();
    let total: u32 = groups.iter().map(|g| g.total).sum();
    rsx! {
        div { class: if outgoing { "lg:text-right" } else { "" },
            h2 { class: "font-chart text-[13px] font-semibold tracking-[0.26em] uppercase text-ink",
                "{head}"
            }
            if groups.is_empty() {
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    if outgoing {
                        "No outgoing references."
                    } else {
                        "No references."
                    }
                }
            } else {
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "{plural(total as usize, \"reference\")} in {plural(files, \"file\")}"
                }
            }
            div { class: "mt-3 space-y-2 text-left",
                for group in groups.iter() {
                    {
                        let open = opened.read().contains(&group.file);
                        let shown = if open {
                            group.rows.len()
                        } else {
                            ROW_CAP.min(group.rows.len())
                        };
                        let hidden: u32 = group.rows[shown..].iter().map(|r| r.count).sum();
                        let left = group.rows.len() - shown;
                        let file = group.file;
                        rsx! {
                            section { key: "{group.file}", class: "ego-group",
                                header { class: "ego-group-head",
                                    Link {
                                        class: "truncate underline-offset-4 hover:text-ink hover:underline",
                                        to: file_route(&group.path),
                                        "{group.path}"
                                    }
                                    span { class: "ego-group-total",
                                        "{plural(group.total as usize, \"ref\")}"
                                    }
                                }
                                div { class: "px-1.5 py-1",
                                    for (i , row) in group.rows.iter().take(shown).enumerate() {
                                        {
                                            let body = rsx! {
                                                if let Some(kind) = row.kind {
                                                    span { class: "ego-row-kw", "{kind_words(kind)}" }
                                                }
                                                span { class: "ego-row-name", "{row.name}" }
                                                span { class: "ego-row-count", "{row.count}" }
                                            };
                                            match row.to.clone() {
                                                Some(to) => rsx! {
                                                    Link {
                                                        key: "{i}",
                                                        class: "ego-row",
                                                        to,
                                                        title: "{row.title}",
                                                        {body}
                                                    }
                                                },
                                                None => rsx! {
                                                    div {
                                                        key: "{i}",
                                                        class: "ego-row is-lifted",
                                                        title: "{row.title}",
                                                        {body}
                                                    }
                                                },
                                            }
                                        }
                                    }
                                    if left > 0 {
                                        button {
                                            class: "ego-row is-more",
                                            onclick: move |_| {
                                                let mut set = opened.peek().clone();
                                                set.insert(file);
                                                opened.set(set);
                                            },
                                            span { class: "ego-row-name",
                                                "+{left} more ({plural(hidden as usize, \"ref\")})"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The item's own source, as the reviewer's editor would show it: a line
/// gutter counting from the item's real first line, no wrapping, and the text
/// itself selectable so it can be copied straight out of the plate.
#[component]
fn CodePane(source: ItemSource) -> Element {
    rsx! {
        div { class: "ego-code",
            div { class: "ego-lines",
                for (i , line) in source.lines.iter().enumerate() {
                    div { key: "{i}", class: "ego-line",
                        span { class: "ego-ln", "{source.first_line as usize + i}" }
                        span { class: "ego-src",
                            for (n , (text , tok)) in line.iter().enumerate() {
                                span { key: "{n}", class: tok_class(*tok), "{text}" }
                            }
                        }
                    }
                }
            }
        }
        if source.elided > 0 {
            p { class: "ego-elided", "+ {source.elided} more lines" }
        }
    }
}

/// The center plate: the selection quoted, then everything the source itself
/// cannot show — the methods written for it elsewhere, and the file's outline
/// when the whole file is in focus.
#[component]
fn CenterPlate(
    locator: String,
    facts: String,
    changed: bool,
    source: Option<ItemSource>,
    loading: bool,
    outline: Vec<OutlineRow>,
    impls: Vec<ImplGroup>,
    folds: Vec<String>,
) -> Element {
    rsx! {
        section { class: "plate flex flex-col",
            div { class: "px-5 pb-4 pt-4",
                p { class: "flex items-baseline gap-2 font-data text-[11px] text-ink",
                    span { class: "break-all", "{locator}" }
                    if changed {
                        span {
                            class: "text-flare",
                            title: "this file changed since the diff base",
                            "M"
                        }
                    }
                }
                if !facts.is_empty() {
                    p { class: "mt-0.5 font-data text-[10px] text-ink-soft", "{facts}" }
                }
                if let Some(source) = source {
                    div { class: "mt-3", CodePane { source } }
                } else if loading {
                    p { class: "mt-3 font-data text-[10.5px] text-ink-soft", "loading…" }
                }
                if !outline.is_empty() {
                    div { class: "mt-3 border-t border-ink-line",
                        for row in outline.iter() {
                            Link {
                                key: "{row.line}-{row.name}",
                                class: "ego-member",
                                to: row.to.clone(),
                                title: "{row.name} · {row.vis.words()}",
                                span { class: "ego-member-decl", "{decl_words(row.vis, row.kind)}" }
                                span { class: "ego-member-name", "{row.name}" }
                                if row.refs > 0 {
                                    span { class: "ego-member-refs", "{row.refs} refs" }
                                }
                                span { class: "ego-member-line", "{row.line}" }
                            }
                        }
                    }
                }
                for group in impls.iter() {
                    div { key: "{group.head}", class: "mt-3 border-t border-ink-line pt-2",
                        p { class: "ego-impl-head", "{group.head}" }
                        for row in group.rows.iter() {
                            Link {
                                key: "{row.locator}",
                                class: "ego-member",
                                to: row.to.clone(),
                                title: "{row.name} · {row.vis.words()}",
                                span { class: "ego-member-decl", "{decl_words(row.vis, row.kind)}" }
                                span { class: "ego-member-name", "{row.name}" }
                                span { class: "ego-member-line", "{row.locator}" }
                            }
                        }
                    }
                }
                if !folds.is_empty() {
                    div { class: "mt-2 space-y-0.5",
                        for fold in folds.iter() {
                            p { key: "{fold}", class: "font-data text-[9.5px] leading-snug text-ink-soft",
                                "{fold}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Keyboard on the focus plate: `/` finds a file, Escape steps back up the
/// ladder. Only one altitude is mounted at a time, so each installs its own
/// handler over the other's.
const EGO_KEYS_JS: &str = r#"
if (window.__slopifyKeys) {
    document.removeEventListener('keydown', window.__slopifyKeys);
}
window.__slopifyKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') {
        e.preventDefault();
        const s = [...document.querySelectorAll('#code-search')]
            .find((el) => el.offsetParent !== null);
        if (s) s.focus();
        return;
    }
    if (e.key === 'Escape') dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopifyKeys);
"#;

/// The focus plate. `item` is empty for a whole-file focus.
#[component]
pub fn EgoPlate(graph: CodeGraph, path: String, item: String) -> Element {
    let code = use_code();
    let nav = use_navigator();

    // Escape climbs one rung: an item hands the plate back to its file, a file
    // hands it back to the whole map.
    let up = {
        let (path, item) = (path.clone(), item.clone());
        move || {
            if item.is_empty() {
                nav.push(Route::CodeOverview {});
            } else {
                nav.push(file_route(&path));
            }
        }
    };
    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(EGO_KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                if key == "Escape" {
                    up();
                }
            }
        });
    });

    let containment = use_memo({
        let graph = graph.clone();
        move || Containment::build(&graph)
    });

    // The selection: an item named by the route, or the file itself. Every
    // hook runs before the survey is questioned, so a route naming a file this
    // survey never saw cannot change the hook order under us.
    let found = graph.files.iter().find(|f| f.path == path).cloned();
    let mark: Option<ItemMark> = found.as_ref().and_then(|info| {
        (!item.is_empty())
            .then(|| {
                graph
                    .items
                    .iter()
                    .find(|m| m.file == info.id && m.label == item)
                    .cloned()
            })
            .flatten()
    });

    // Members of the selected type, wherever their impls are written; the impl
    // header a method sits under lives with its own source, so fetch those
    // files too.
    let kids: Vec<u32> = mark
        .as_ref()
        .map(|m| containment.read().kids(m.id).to_vec())
        .unwrap_or_default();
    let mut wanted: Vec<u32> = found.iter().map(|f| f.id).collect();
    wanted.extend(
        kids.iter()
            .filter_map(|m| graph.items.get(*m as usize).map(|k| k.file)),
    );
    wanted.sort_unstable();
    wanted.dedup();

    use_effect(use_reactive((&wanted,), move |(wanted,)| {
        for id in wanted {
            if code.details.peek().contains_key(&id) {
                continue;
            }
            spawn(async move {
                if let Ok(detail) = file_detail(id).await {
                    let mut details = code.details;
                    details.write().insert(id, detail);
                }
            });
        }
    }));

    // The selection's own source text — the definition the plate quotes.
    let quoted: Option<(u32, u32)> = mark.as_ref().map(|m| (m.file, m.local));
    use_effect(use_reactive((&quoted,), move |(quoted,)| {
        let Some(key) = quoted else { return };
        if code.sources.peek().contains_key(&key) {
            return;
        }
        spawn(async move {
            if let Ok(source) = item_source(key.0, key.1).await {
                let mut sources = code.sources;
                sources.write().insert(key, source);
            }
        });
    }));

    let Some(info) = found else {
        return rsx! {
            div { class: "absolute inset-0 grid place-items-center p-4",
                section { class: "plate max-w-md px-5 py-4",
                    p { class: "font-data text-[11px] text-ink", "No file at “{path}” in this survey." }
                    Link {
                        class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                        to: Route::CodeOverview {},
                        "← whole map"
                    }
                }
            }
        };
    };
    let center = match &mark {
        Some(m) => Center::Item(m.id),
        None => Center::File(info.id),
    };

    let details = code.details.read();
    let detail = details.get(&info.id);
    let source = quoted.and_then(|key| code.sources.read().get(&key).cloned());

    // The local ids of everything inside the selection: same-file references
    // are the file detail's to report.
    let inside: HashSet<u32> = match &mark {
        Some(m) => {
            let mut marks = Vec::new();
            containment.read().inside(m.id, &mut marks);
            marks
                .iter()
                .filter_map(|&x| graph.items.get(x as usize))
                .filter(|k| k.file == info.id)
                .map(|k| k.local)
                .collect()
        }
        None => HashSet::new(),
    };
    let local_mark: HashMap<u32, Option<u32>> = detail
        .map(|d| d.items.iter().map(|i| (i.id, i.mark)).collect())
        .unwrap_or_default();

    // Same-file references, at item precision, for an item focus. A whole-file
    // focus has none to show: a file's use of itself is not coupling.
    let within = |dir: Dir| -> Vec<(Option<u32>, u32)> {
        let Some(d) = detail else {
            return Vec::new();
        };
        if mark.is_none() {
            return Vec::new();
        }
        d.item_refs
            .iter()
            .filter_map(|r| {
                let (near, far) = match dir {
                    Dir::UsedBy => (r.to, r.from),
                    Dir::Uses => (r.from, r.to),
                };
                if !inside.contains(&near) || inside.contains(&far) {
                    return None;
                }
                Some((local_mark.get(&far).copied().flatten(), r.count))
            })
            .collect()
    };

    let (used, uses) = {
        let containment = containment.read();
        (
            column_views(
                &graph,
                model::groups(
                    &graph,
                    &containment,
                    center,
                    Dir::UsedBy,
                    within(Dir::UsedBy).into_iter(),
                ),
            ),
            column_views(
                &graph,
                model::groups(
                    &graph,
                    &containment,
                    center,
                    Dir::Uses,
                    within(Dir::Uses).into_iter(),
                ),
            ),
        )
    };

    // ---- The center plate. ------------------------------------------------
    let mut outline: Vec<OutlineRow> = Vec::new();
    let mut impls: Vec<ImplGroup> = Vec::new();
    let mut folds: Vec<String> = Vec::new();
    let (locator, facts) = match &mark {
        Some(mark) => {
            // A type's associated items, grouped under the impl header they
            // are written under — wherever in the workspace that is.
            let mut private_kids = 0usize;
            for kid in kids.iter().filter_map(|k| graph.items.get(*k as usize)) {
                if kid.vis == Vis::Private {
                    private_kids += 1;
                    continue;
                }
                let where_from = graph
                    .files
                    .get(kid.file as usize)
                    .map(|f| f.path.clone())
                    .unwrap_or_default();
                let head = details
                    .get(&kid.file)
                    .and_then(|d| d.items.get(kid.local as usize))
                    .map(|i| i.section.clone())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("impl {}", mark.name));
                let row = ImplRow {
                    kind: kid.kind,
                    vis: kid.vis,
                    name: kid.name.clone(),
                    locator: format!("{where_from}:{}", kid.line),
                    to: item_route(&where_from, &kid.label),
                };
                match impls.iter_mut().find(|g| g.head == head) {
                    Some(group) => group.rows.push(row),
                    None => impls.push(ImplGroup {
                        head,
                        rows: vec![row],
                    }),
                }
            }
            // A trait impl with no items of its own is still code someone
            // wrote, and rustdoc lists it; so does this plate.
            for header in &mark.impls {
                if !impls.iter().any(|g| &g.head == header) {
                    impls.push(ImplGroup {
                        head: header.clone(),
                        rows: Vec::new(),
                    });
                }
            }
            if private_kids > 0 {
                folds.push(format!("+ {private_kids} private"));
            }
            (
                format!("{}:{}", info.path, mark.line),
                format!("crate {}", info.krate),
            )
        }
        None => {
            let mut private = 0usize;
            let mut nested = 0usize;
            for m in graph.items.iter().filter(|m| m.file == info.id) {
                if m.parent.is_some() {
                    nested += 1;
                    continue;
                }
                if m.vis == Vis::Private {
                    private += 1;
                    continue;
                }
                outline.push(OutlineRow {
                    kind: m.kind,
                    vis: m.vis,
                    name: m.name.clone(),
                    refs: m.fan_in,
                    line: m.line,
                    to: item_route(&info.path, &m.label),
                });
            }
            if private > 0 {
                folds.push(format!("+ {private} private"));
            }
            if nested > 0 {
                folds.push(format!("+ {nested} on their own types"));
            }
            (
                info.path.clone(),
                format!(
                    "{} lines · {} · crate {}",
                    info.lines,
                    plural(info.items as usize, "item"),
                    info.krate
                ),
            )
        }
    };
    let loading = mark.is_some() && source.is_none();

    rsx! {
        div { class: "absolute inset-0 overflow-y-auto",
            div { class: "mx-auto w-full max-w-[1360px] px-6 pb-24 pt-[92px]",
                Crumb {
                    path: info.path.clone(),
                    item: mark.as_ref().map(|m| m.name.clone()).unwrap_or_default(),
                }
                div { class: "mt-6 grid items-start gap-8 lg:grid-cols-[264px_minmax(0,1fr)_264px] lg:gap-10",
                    EgoColumn {
                        groups: used,
                        head: "Used by".to_string(),
                        outgoing: false,
                    }
                    CenterPlate {
                        locator,
                        facts,
                        changed: info.changed,
                        source,
                        loading,
                        outline,
                        impls,
                        folds,
                    }
                    EgoColumn {
                        groups: uses,
                        head: "Uses".to_string(),
                        outgoing: true,
                    }
                }
            }
        }
    }
}
