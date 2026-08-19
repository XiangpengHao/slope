//! The focus plate: one selection, unfolded, with one hop of its references.
//!
//! Selecting anything on the map replaces the map with this: the selection
//! engraved on the center plate — fields, variants, methods, or a signature —
//! with what leans on it on the left and what it reaches for on the right,
//! each grouped by the container the reference lives in. One hop only, never
//! global spaghetti, and every row re-centers the plate on itself. Every focus
//! is a URL.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, ItemInfo, ItemKind, ItemMark, Vis, file_detail};
use crate::views::codemap::chrome::{ItemGlyph, dir_of, file_name, kind_words, plural};
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

/// One container's rows in a reference column.
#[derive(Clone, PartialEq)]
pub struct GroupView {
    file: u32,
    path: String,
    total: u32,
    rows: Vec<RowView>,
}

/// One member row of the center plate: a field, a variant, or a method.
#[derive(Clone, PartialEq)]
pub struct MemberView {
    glyph: Option<ItemKind>,
    name: String,
    detail: String,
    quiet: bool,
    to: Option<Route>,
}

/// The plate's own last line: what the survey can honestly say about this
/// selection, in the chart's voice.
fn foot_words(used: &[GroupView], uses: &[GroupView], vis: Vis) -> String {
    let sum = |gs: &[GroupView]| gs.iter().map(|g| g.total).sum::<u32>();
    let (arriving, leaving) = (sum(used), sum(uses));
    match (arriving, leaving) {
        (0, 0) => match vis {
            Vis::Pub => "pub, and nothing charted leans on it — a door no one has opened.".into(),
            Vis::Crate => "visible to its crate, and unreferenced beyond its own file.".into(),
            Vis::Private => {
                "private: it is folded into its container everywhere it is named.".into()
            }
        },
        (0, leaving) => format!(
            "nothing charted leans on it; it reaches out ×{leaving} into {}.",
            plural(uses.len(), "container")
        ),
        (arriving, 0) => format!(
            "×{arriving} references arrive from {}; it reaches for nothing beyond itself.",
            plural(used.len(), "container")
        ),
        (arriving, leaving) => format!(
            "×{arriving} references arrive from {}; ×{leaving} leave for {}.",
            plural(used.len(), "container"),
            plural(uses.len(), "container")
        ),
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
                            name: "private items — folded, lifted here".to_string(),
                            title: "private items never draw as marks; their references lift to \
                                the container that holds them"
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

/// One reference column: grouped by container, capped rows, counted folds.
#[component]
fn EgoColumn(groups: Vec<GroupView>, head: String, sub: String, right: bool) -> Element {
    let mut opened: Signal<HashSet<u32>> = use_signal(HashSet::new);
    let rows: usize = groups.iter().map(|g| g.rows.len()).sum();
    let total: u32 = groups.iter().map(|g| g.total).sum();
    rsx! {
        div { class: if right { "lg:text-right" } else { "" },
            h2 { class: "font-chart text-[13px] font-semibold tracking-[0.26em] uppercase text-ink",
                "{head}"
                span { class: "ml-2 font-data text-[10px] font-normal normal-case tracking-[0.06em] text-ink-soft",
                    "{rows} · ×{total}"
                }
            }
            p { class: "mt-0.5 font-chart text-[12px] italic text-ink-soft", "{sub}" }
            if groups.is_empty() {
                p { class: "mt-3 font-chart text-[12.5px] italic text-ink-soft",
                    if right {
                        "reaches for nothing beyond itself."
                    } else {
                        "nothing charted leans on this."
                    }
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
                                    span { class: "ego-group-total", "×{group.total}" }
                                }
                                div { class: "px-1.5 py-1",
                                    for (i , row) in group.rows.iter().take(shown).enumerate() {
                                        {
                                            let body = rsx! {
                                                if let Some(kind) = row.kind {
                                                    ItemGlyph { kind, box_px: 11.0 }
                                                } else {
                                                    span { class: "w-[11px] shrink-0" }
                                                }
                                                span { class: "ego-row-name", "{row.name}" }
                                                span { class: "ego-row-count", "×{row.count}" }
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
                                            span { class: "w-[11px] shrink-0" }
                                            span { class: "ego-row-name", "+ {left} more · ×{hidden} folded" }
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

/// The center plate: the selection unfolded. One shape for a file and for an
/// item — a plate is a plate.
#[component]
#[allow(clippy::too_many_arguments)]
fn CenterPlate(
    kind: Option<ItemKind>,
    eyebrow_left: String,
    eyebrow_right: String,
    name: String,
    changed: bool,
    where_line: String,
    badges: Vec<String>,
    sig: Option<String>,
    members: Vec<MemberView>,
    folds: Vec<String>,
    foot: String,
) -> Element {
    rsx! {
        section { class: "plate flex flex-col",
            div { class: "px-5 pb-4 pt-4",
                div { class: "flex items-baseline justify-between gap-3 font-data text-[9px] tracking-[0.16em] uppercase text-ink-soft",
                    span { "{eyebrow_left}" }
                    span { "{eyebrow_right}" }
                }
                h1 { class: "mt-1.5 flex items-baseline gap-2 font-data text-[19px] font-bold text-ink",
                    if let Some(kind) = kind {
                        ItemGlyph { kind, box_px: 14.0 }
                    }
                    span { class: "break-all", "{name}" }
                    if changed {
                        span { class: "text-flare", title: "touched in this epoch", "▎" }
                    }
                }
                p { class: "mt-0.5 break-all font-data text-[10px] text-ink-soft", "{where_line}" }
                if !badges.is_empty() {
                    div { class: "mt-2.5 flex flex-wrap gap-1",
                        for badge in badges.iter() {
                            span { key: "{badge}", class: "ego-badge", "{badge}" }
                        }
                    }
                }
                if let Some(sig) = sig {
                    pre { class: "ego-sig", "{sig}" }
                }
                if !members.is_empty() || !folds.is_empty() {
                    div { class: "mt-3 border-t border-ink-line",
                        for (i , member) in members.iter().enumerate() {
                            {
                                let body = rsx! {
                                    if let Some(glyph) = member.glyph {
                                        ItemGlyph { kind: glyph, box_px: 11.0 }
                                    } else {
                                        span { class: "w-[11px] shrink-0 text-center text-ink-line", "·" }
                                    }
                                    span {
                                        class: "truncate font-data text-[11.5px]",
                                        class: if member.quiet { "text-ink-soft" } else { "font-medium text-ink" },
                                        "{member.name}"
                                    }
                                    if !member.detail.is_empty() {
                                        span { class: "ml-auto shrink-0 truncate pl-3 font-data text-[10.5px] text-ink-soft",
                                            "{member.detail}"
                                        }
                                    }
                                };
                                match member.to.clone() {
                                    Some(to) => rsx! {
                                        Link { key: "{i}", class: "ego-member", to, {body} }
                                    },
                                    None => rsx! {
                                        div { key: "{i}", class: "ego-member", {body} }
                                    },
                                }
                            }
                        }
                        for fold in folds.iter() {
                            p { key: "{fold}", class: "px-1 pt-1.5 font-data text-[9.5px] leading-snug text-ink-soft",
                                "{fold}"
                            }
                        }
                    }
                }
            }
            p { class: "ego-foot", "{foot}" }
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
        const s = document.getElementById('code-search');
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

    // Members of the selected type, wherever their impls are written; a
    // method's signature lives with its own source, so fetch those files too.
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

    // The selection's own body, and the local ids of everything inside it:
    // same-file references are the file detail's to report.
    let body: Option<ItemInfo> = match (&mark, detail) {
        (Some(m), Some(d)) => d.items.iter().find(|i| i.mark == Some(m.id)).cloned(),
        _ => None,
    };
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

    let vis = mark.as_ref().map(|m| m.vis).unwrap_or(Vis::Pub);
    let foot = foot_words(&used, &uses, vis);

    // ---- The center plate. ------------------------------------------------
    let mut members: Vec<MemberView> = Vec::new();
    let mut folds: Vec<String> = Vec::new();
    let plate = match &mark {
        Some(mark) => {
            let member_glyph = model::member_kind(mark.kind);
            if let Some(body) = &body {
                members.extend(body.members.iter().map(|m| MemberView {
                    glyph: member_glyph,
                    name: m.name.clone(),
                    detail: m.ty.clone(),
                    quiet: m.vis == Vis::Private,
                    to: None,
                }));
            }
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
                let elsewhere = kid.file != info.id;
                members.push(MemberView {
                    glyph: Some(kid.kind),
                    name: kid.name.clone(),
                    detail: if elsewhere {
                        format!("in {}", file_name(&where_from))
                    } else {
                        format!("L{}", kid.line)
                    },
                    quiet: kid.vis == Vis::Crate,
                    to: Some(item_route(&where_from, &kid.label)),
                });
            }
            if private_kids > 0 {
                folds.push(format!(
                    "+ {} folded into this plate — their outside references lift here",
                    plural(private_kids, "private member")
                ));
            }
            if body.is_none() {
                folds.push("reading the item's body…".to_string());
            }
            (
                Some(mark.kind),
                kind_words(mark.kind).to_string(),
                mark.vis.words().to_string(),
                mark.name.clone(),
                format!("{} · L{} · crate {}", info.path, mark.line, info.krate),
                mark.traits.clone(),
                body.as_ref().and_then(|b| b.sig.clone()),
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
                members.push(MemberView {
                    glyph: Some(m.kind),
                    name: m.name.clone(),
                    detail: if m.fan_in > 0 {
                        format!("×{} in · L{}", m.fan_in, m.line)
                    } else {
                        format!("L{}", m.line)
                    },
                    quiet: m.vis == Vis::Crate,
                    to: Some(item_route(&info.path, &m.label)),
                });
            }
            if private > 0 {
                folds.push(format!(
                    "+ {} folded — their outside references lift to this file",
                    plural(private, "private item")
                ));
            }
            if nested > 0 {
                folds.push(format!(
                    "+ {} on the plates of their own types",
                    plural(nested, "member")
                ));
            }
            (
                None,
                "file".to_string(),
                format!("crate {}", info.krate),
                file_name(&info.path).to_string(),
                format!(
                    "{} · {} lines · {}",
                    info.path,
                    info.lines,
                    plural(info.items as usize, "item")
                ),
                Vec::new(),
                None,
            )
        }
    };
    let (kind, eyebrow_left, eyebrow_right, name, where_line, badges, sig) = plate;

    rsx! {
        div { class: "absolute inset-0 overflow-y-auto",
            div { class: "mx-auto w-full max-w-[1240px] px-6 pb-24 pt-[92px]",
                Crumb {
                    path: info.path.clone(),
                    item: mark.as_ref().map(|m| m.name.clone()).unwrap_or_default(),
                }
                div { class: "mt-6 grid items-start gap-8 lg:grid-cols-[300px_minmax(0,1fr)_300px] lg:gap-12",
                    EgoColumn {
                        groups: used,
                        head: "Used by".to_string(),
                        sub: "who leans on the selection — grouped by container, heaviest first"
                            .to_string(),
                        right: false,
                    }
                    CenterPlate {
                        kind,
                        eyebrow_left,
                        eyebrow_right,
                        name,
                        changed: info.changed,
                        where_line,
                        badges,
                        sig,
                        members,
                        folds,
                        foot,
                    }
                    EgoColumn {
                        groups: uses,
                        head: "Uses".to_string(),
                        sub: "what the selection reaches for".to_string(),
                        right: true,
                    }
                }
            }
        }
    }
}
