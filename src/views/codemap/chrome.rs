//! Code-altitude furniture: cartouche, search, the refs toggle, legend,
//! and the file / crate panels. All of it the same engraved ink.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, FileInfo, ItemKind};
use crate::views::codemap::map::{DirMarkSvg, FileMarkSvg, ItemGlyph, item_sel_label};
use crate::views::codemap::{RefDir, file_route, item_route, use_code};

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map(|(_, n)| n).unwrap_or(path)
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// The altitude line: the ladder between the two charts. The current rung
/// is engraved solid; the other is a link.
#[component]
pub fn AltitudeSwitch(code: bool) -> Element {
    rsx! {
        p { class: "flex gap-3 font-data text-[9.5px] tracking-[0.14em] uppercase",
            if code {
                Link {
                    class: "text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::Overview {},
                    "dependencies"
                }
                span { class: "text-ink underline underline-offset-4", "code" }
            } else {
                span { class: "text-ink underline underline-offset-4", "dependencies" }
                Link {
                    class: "text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::CodeOverview {},
                    "code"
                }
            }
        }
    }
}

/// The code chart's title block.
#[component]
pub fn CodeCartouche(graph: CodeGraph, workspace: String, epoch_line: String) -> Element {
    let files = graph.files.len();
    let crates: std::collections::HashSet<&str> =
        graph.files.iter().map(|f| f.krate.as_str()).collect();
    let lines: u64 = graph.files.iter().map(|f| f.lines as u64).sum();

    rsx! {
        section { class: "plate pointer-events-auto px-4 py-3",
            h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                "{workspace}"
            }
            p { class: "mt-0.5 font-chart text-[12px] italic text-ink-soft",
                "code structure · {plural(files, \"file\")} · {plural(crates.len(), \"crate\")}"
            }
            div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                AltitudeSwitch { code: true }
                p {
                    span { class: "text-ink-soft", "surveyed " }
                    "{lines} lines · {graph.refs.len()} reference edges"
                }
                p { class: "text-ink-soft", "{epoch_line}" }
                p { class: "text-ink-soft",
                    "changes are not yet drawn at this altitude — structure only"
                }
            }
        }
    }
}

/// Find a file by any part of its path.
#[component]
pub fn CodeSearch(graph: CodeGraph) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let nav = use_navigator();

    let results = use_memo(move || {
        let q = query().trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<FileInfo> = graph
            .files
            .iter()
            .filter(|f| f.path.to_lowercase().contains(&q))
            .cloned()
            .collect();
        hits.sort_by_key(|f| {
            (
                !file_name(&f.path).to_lowercase().starts_with(&q),
                std::cmp::Reverse(f.refs_in_files),
                f.path.clone(),
            )
        });
        hits.truncate(9);
        hits
    });

    rsx! {
        div { class: "pointer-events-auto relative w-full",
            input {
                id: "code-search",
                class: "plate w-full px-3 py-1.5 font-data text-[11px] text-ink placeholder:text-ink-soft focus:outline-none",
                r#type: "search",
                placeholder: "find a file…   /",
                autocomplete: "off",
                spellcheck: "false",
                "aria-label": "Find a file",
                value: "{query}",
                oninput: move |e| {
                    query.set(e.value());
                    active.set(0);
                },
                onkeydown: move |e| {
                    let n = results().len();
                    match e.key() {
                        Key::ArrowDown if n > 0 => {
                            e.prevent_default();
                            active.set((active() + 1) % n);
                        }
                        Key::ArrowUp if n > 0 => {
                            e.prevent_default();
                            active.set((active() + n - 1) % n);
                        }
                        Key::Enter => {
                            if let Some(hit) = results().get(active().min(n.saturating_sub(1))) {
                                nav.push(file_route(&hit.path));
                                query.set(String::new());
                            }
                        }
                        Key::Escape => query.set(String::new()),
                        _ => {}
                    }
                },
            }
            if !query().trim().is_empty() {
                if results().is_empty() {
                    div { class: "plate absolute left-0 right-0 top-full z-20 mt-1 px-3 py-2",
                        p { class: "font-data text-[10px] tracking-[0.1em] uppercase text-ink-soft",
                            "no matches"
                        }
                    }
                } else {
                    ul { class: "plate absolute left-0 right-0 top-full z-20 mt-1 max-h-72 overflow-auto py-1",
                        for (i , hit) in results().into_iter().enumerate() {
                            li {
                                Link {
                                    to: file_route(&hit.path),
                                    class: if i == active() { "flex w-full items-center gap-1.5 px-2.5 py-1 bg-ink/5" } else { "flex w-full items-center gap-1.5 px-2.5 py-1 hover:bg-ink/5" },
                                    onclick: move |_| query.set(String::new()),
                                    FileMarkSvg {
                                        refs_in: hit.refs_in_files,
                                        focal: false,
                                        box_px: 18.0,
                                    }
                                    span { class: "truncate font-data text-[11px] text-ink",
                                        "{file_name(&hit.path)}"
                                    }
                                    span { class: "ml-auto truncate font-data text-[9px] text-ink-soft",
                                        "{dir_of(&hit.path)}"
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

/// Which of the selection's references the chart draws.
#[component]
pub fn RefsToggle() -> Element {
    let code = use_code();
    let current = *code.ref_dir.read();
    let seg = |label: &'static str, hint: &'static str, val: RefDir| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                title: hint,
                onclick: move |_| {
                    let mut d = code.ref_dir;
                    d.set(val);
                },
                "{label}"
            }
        }
    };
    rsx! {
        div { class: "plate pointer-events-auto px-2 py-1",
            span { class: "block font-chart text-[10px] tracking-[0.18em] uppercase text-ink",
                "Refs"
            }
            div { class: "mt-0.5 flex items-stretch gap-0.5",
                {seg("uses", "what the selection references", RefDir::Uses)}
                {seg("used by", "what references the selection", RefDir::UsedBy)}
            }
        }
    }
}

/// One line sample for the legend's chord key.
#[component]
fn ChordSample(stroke: &'static str, #[props(default = 1.2)] width: f64) -> Element {
    rsx! {
        svg {
            width: "34",
            height: "8",
            view_box: "0 0 34 8",
            class: "shrink-0",
            "aria-hidden": "true",
            line {
                x1: "0",
                y1: "4",
                x2: "32",
                y2: "4",
                stroke,
                stroke_width: "{width}",
            }
            path {
                d: "M27.5 1.4 L32.5 4 L27.5 6.6",
                fill: "none",
                stroke,
                stroke_width: "1.1",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

/// One row of the legend's "using this map" section.
#[component]
fn UsageRow(gesture: &'static str, effect: &'static str) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "shrink-0 font-data text-[9.5px] tracking-[0.1em] uppercase text-ink",
                "{gesture}"
            }
            span { class: "text-ink-soft", "{effect}" }
        }
    }
}

/// The key: every mark the map can draw, named in words, plus the survey's
/// own honesty notes.
#[component]
pub fn CodeLegend(graph: CodeGraph, #[props(default = true)] start_open: bool) -> Element {
    rsx! {
        details {
            class: "plate fold pointer-events-auto w-full open:pb-3 sm:w-64",
            open: start_open,
            summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this map"
            }
            // The key reads first — every mark and line named in words —
            // then the gestures, then the survey's own honesty notes. The
            // fold is a drawn, always-visible scrollbar, never a hidden cut.
            div { class: "legend-scroll space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink sm:max-h-[42dvh]",
                p {
                    "every directory is a street, lettered on its line; its files sit as lots above it and its subdirectories branch below"
                }
                div { class: "space-y-2 border-t border-ink-line pt-2.5",
                    div { class: "flex items-center gap-2",
                        div { class: "flex items-end",
                            FileMarkSvg { refs_in: 0, focal: false, box_px: 18.0 }
                            FileMarkSvg { refs_in: 9, focal: false, box_px: 22.0 }
                            FileMarkSvg { refs_in: 60, focal: false, box_px: 26.0 }
                        }
                        span { "files are sized by how many files reference them" }
                    }
                    div { class: "flex items-center gap-2",
                        DirMarkSvg {
                            open: true,
                            focal: false,
                            root: false,
                            box_px: 18.0,
                        }
                        span { "open directory" }
                    }
                    div { class: "flex items-center gap-2",
                        DirMarkSvg {
                            open: false,
                            focal: false,
                            root: false,
                            box_px: 18.0,
                        }
                        span {
                            span { class: "font-medium", "GATE" }
                            " — a folded directory; its count is written beside it"
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    div { class: "flex items-center gap-2",
                        ChordSample { stroke: "var(--color-ink)", width: 1.25 }
                        span { "ink, arrow in: the selection uses this" }
                    }
                    div { class: "flex items-center gap-2",
                        ChordSample { stroke: "var(--color-ink-line)", width: 1.1 }
                        span { "hairline, arrow out: this uses the selection" }
                    }
                    p { class: "text-ink-soft",
                        "arrows point the way change travels — into the file that uses the definition; ×n counts repeated references; references are drawn for the selection only"
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5",
                    div { class: "grid grid-cols-2 gap-x-3 gap-y-1",
                        for (kind , words) in [
                            (ItemKind::Fn, "function"),
                            (ItemKind::Struct, "struct"),
                            (ItemKind::Enum, "enum"),
                            (ItemKind::Trait, "trait"),
                            (ItemKind::TypeAlias, "type alias"),
                            (ItemKind::Const, "const · static"),
                            (ItemKind::Macro, "macro"),
                            (ItemKind::Mod, "module"),
                        ]
                        {
                            div { class: "flex items-center gap-1.5",
                                ItemGlyph { kind, box_px: 12.0 }
                                span { "{words}" }
                            }
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    UsageRow { gesture: "click a file", effect: "select it — its references are drawn" }
                    UsageRow { gesture: "click again", effect: "the file cuts away: its items, in source order" }
                    UsageRow { gesture: "click an item", effect: "draw that item's references" }
                    UsageRow { gesture: "click a square", effect: "fold or open a directory" }
                    UsageRow { gesture: "refs toggle", effect: "draw what it uses, or what uses it" }
                    UsageRow { gesture: "back / esc", effect: "step back up" }
                    UsageRow { gesture: "/ · f", effect: "find a file · refit the map" }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5 text-ink-soft",
                    for note in graph.notes.iter() {
                        p { "{note}" }
                    }
                }
            }
        }
    }
}

/// A chunked list of reference rows.
#[component]
fn RefList(rows: Vec<(Route, String, String, u32)>) -> Element {
    const CHUNK: usize = 8;
    let mut all = use_signal(|| false);
    let total = rows.len();
    let shown = if all() { total } else { CHUNK.min(total) };
    rsx! {
        ul { class: "mt-1",
            for (i , (to , name , detail , count)) in rows.into_iter().take(shown).enumerate() {
                li { key: "{i}",
                    Link {
                        to,
                        class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 hover:bg-ink/5",
                        span { class: "truncate font-data text-[11px] text-ink", "{name}" }
                        span { class: "truncate font-data text-[9px] text-ink-soft", "{detail}" }
                        if count > 1 {
                            span { class: "ml-auto shrink-0 font-data text-[9px] text-ink-soft",
                                "×{count}"
                            }
                        }
                    }
                }
            }
        }
        if total > shown {
            button {
                class: "mt-1 px-1 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                onclick: move |_| all.set(true),
                "show all {total}"
            }
        }
    }
}

fn kind_words(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Fn => "FN",
        ItemKind::Struct => "STRUCT",
        ItemKind::Enum => "ENUM",
        ItemKind::Union => "UNION",
        ItemKind::Trait => "TRAIT",
        ItemKind::TypeAlias => "TYPE",
        ItemKind::Const => "CONST",
        ItemKind::Static => "STATIC",
        ItemKind::Macro => "MACRO",
        ItemKind::Mod => "MOD",
        ItemKind::Impl => "IMPL",
    }
}

/// The focused file's fact sheet: identity, then both directions of its
/// references. With an item selected, the item's own sheet.
#[component]
pub fn FilePanel(graph: CodeGraph, path: String, item: String) -> Element {
    let code = use_code();
    let Some(info) = graph.files.iter().find(|f| f.path == path).cloned() else {
        return rsx! {
            section { class: "plate pointer-events-auto w-72 px-4 py-3",
                p { class: "font-data text-[11px] text-ink", "No file at “{path}” in this survey." }
                Link {
                    class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::CodeOverview {},
                    "← whole map"
                }
            }
        };
    };
    let details = code.details.read();
    let detail = details.get(&info.id);
    let by_id: HashMap<u32, &FileInfo> = graph.files.iter().map(|f| (f.id, f)).collect();

    let sel_item = detail.and_then(|d| {
        if item.is_empty() {
            None
        } else {
            d.items.iter().find(|i| item_sel_label(i) == item).cloned()
        }
    });

    // File-level reference rows, both directions.
    let uses_rows: Vec<(Route, String, String, u32)> = graph
        .refs
        .iter()
        .filter(|r| r.from == info.id)
        .filter_map(|r| {
            let f = by_id.get(&r.to)?;
            Some((
                file_route(&f.path),
                file_name(&f.path).to_string(),
                dir_of(&f.path).to_string(),
                r.count,
            ))
        })
        .collect();
    let used_rows: Vec<(Route, String, String, u32)> = graph
        .refs
        .iter()
        .filter(|r| r.to == info.id)
        .filter_map(|r| {
            let f = by_id.get(&r.from)?;
            Some((
                file_route(&f.path),
                file_name(&f.path).to_string(),
                dir_of(&f.path).to_string(),
                r.count,
            ))
        })
        .collect();

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::CodeOverview {},
                    "← whole map"
                }
                h2 { class: "mt-1.5 break-all font-data text-[15px] font-semibold text-ink",
                    "{file_name(&info.path)}"
                }
                p { class: "break-all font-data text-[10px] text-ink-soft", "{info.path}" }
                p { class: "mt-0.5 font-data text-[10.5px] text-ink-soft",
                    "crate {info.krate} · {info.lines} lines"
                }
                p { class: "mt-0.5 font-data text-[10px] text-ink-soft",
                    "{plural(info.fns as usize, \"fn\")} · {plural(info.types as usize, \"type\")} · {plural(info.traits as usize, \"trait\")}"
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                if let Some(sel) = sel_item {
                    // ---- One item's sheet. ----
                    div { class: "mt-2 border-t border-ink-line pt-2",
                        div { class: "flex items-center gap-1.5",
                            ItemGlyph { kind: sel.kind, box_px: 13.0 }
                            h3 { class: "break-all font-data text-[12px] font-medium text-ink",
                                "{sel.name}"
                            }
                        }
                        p { class: "mt-0.5 font-data text-[9.5px] tracking-[0.1em] uppercase text-ink-soft",
                            "{kind_words(sel.kind)} · L{sel.line}–{sel.end_line} · "
                            if sel.public { "PUB" } else { "PRIVATE" }
                        }
                        if !sel.section.is_empty() {
                            p { class: "mt-0.5 font-data text-[10px] text-ink-soft", "{sel.section}" }
                        }
                        Link {
                            class: "mt-1 inline-block font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                            to: file_route(&info.path),
                            "← whole file"
                        }
                    }
                    if let Some(d) = detail {
                        {
                            let uses: Vec<(Route, String, String, u32)> = d
                                .refs_out
                                .iter()
                                .filter(|r| r.item == sel.id)
                                .filter_map(|r| {
                                    let f = by_id.get(&r.file)?;
                                    let to = if r.other.is_empty() {
                                        file_route(&f.path)
                                    } else {
                                        item_route(&f.path, &r.other)
                                    };
                                    let name = if r.other.is_empty() {
                                        file_name(&f.path).to_string()
                                    } else {
                                        r.other.clone()
                                    };
                                    Some((to, name, file_name(&f.path).to_string(), r.count))
                                })
                                .collect();
                            let used: Vec<(Route, String, String, u32)> = d
                                .refs_in
                                .iter()
                                .filter(|r| r.item == sel.id)
                                .filter_map(|r| {
                                    let f = by_id.get(&r.file)?;
                                    let to = if r.other.is_empty() {
                                        file_route(&f.path)
                                    } else {
                                        item_route(&f.path, &r.other)
                                    };
                                    let name = if r.other.is_empty() {
                                        file_name(&f.path).to_string()
                                    } else {
                                        r.other.clone()
                                    };
                                    Some((to, name, file_name(&f.path).to_string(), r.count))
                                })
                                .collect();
                            let within: Vec<(Route, String, String, u32)> = d
                                .item_refs
                                .iter()
                                .filter(|r| r.from == sel.id)
                                .filter_map(|r| {
                                    let target = d.items.iter().find(|i| i.id == r.to)?;
                                    Some((
                                        item_route(&info.path, &item_sel_label(target)),
                                        target.name.clone(),
                                        "same file".to_string(),
                                        r.count,
                                    ))
                                })
                                .collect();
                            rsx! {
                                h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                                    "Used by ({used.len()})"
                                }
                                if used.is_empty() {
                                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                                        "nothing outside this file"
                                    }
                                } else {
                                    RefList { rows: used }
                                }
                                h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                                    "Uses ({uses.len() + within.len()})"
                                }
                                if uses.is_empty() && within.is_empty() {
                                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                                        "nothing in the workspace"
                                    }
                                } else {
                                    RefList { rows: uses }
                                    if !within.is_empty() {
                                        p { class: "mt-1 font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                                            "within this file"
                                        }
                                        RefList { rows: within }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // ---- The file's sheet. ----
                    h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Used by ({used_rows.len()})"
                    }
                    if used_rows.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "no other workspace file references it"
                        }
                    } else {
                        RefList { rows: used_rows }
                    }
                    h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Uses ({uses_rows.len()})"
                    }
                    if uses_rows.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "nothing in the workspace — self-contained"
                        }
                    } else {
                        RefList { rows: uses_rows }
                    }
                    h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        if let Some(d) = detail {
                            "Items ({d.items.iter().filter(|i| i.kind != ItemKind::Impl).count()})"
                        } else {
                            "Items"
                        }
                    }
                    if let Some(d) = detail {
                        ul { class: "mt-1",
                            for it in d.items.iter().cloned() {
                                li {
                                    if it.kind == ItemKind::Impl {
                                        p { class: "mt-1 truncate px-1 font-data text-[9.5px] text-ink-soft",
                                            "{it.name}"
                                        }
                                    } else {
                                        Link {
                                            to: item_route(&info.path, &item_sel_label(&it)),
                                            class: if it.section.is_empty() {
                                                "flex w-full items-center gap-1.5 px-1 py-0.5 hover:bg-ink/5"
                                            } else {
                                                "flex w-full items-center gap-1.5 py-0.5 pl-3 pr-1 hover:bg-ink/5"
                                            },
                                            ItemGlyph { kind: it.kind, box_px: 11.0 }
                                            span {
                                                class: "truncate font-data text-[10.5px]",
                                                class: if it.public { "font-medium text-ink" } else { "text-ink-soft" },
                                                "{it.name}"
                                            }
                                            span { class: "ml-auto shrink-0 font-data text-[9px] text-ink-soft",
                                                "L{it.line}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        p { class: "mt-1 font-data text-[9px] leading-snug text-ink-soft",
                            "ink names are pub; quiet names are private to the crate"
                        }
                    } else {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "reading the file’s items…"
                        }
                    }
                }
            }
        }
    }
}

/// One crate's district sheet: its files, and what crosses its boundary.
#[component]
pub fn CratePanel(graph: CodeGraph, name: String) -> Element {
    let mut files: Vec<FileInfo> = graph
        .files
        .iter()
        .filter(|f| f.krate == name)
        .cloned()
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let ids: std::collections::HashSet<u32> = files.iter().map(|f| f.id).collect();
    let by_id: HashMap<u32, &FileInfo> = graph.files.iter().map(|f| (f.id, f)).collect();

    if files.is_empty() {
        return rsx! {
            section { class: "plate pointer-events-auto w-72 px-4 py-3",
                p { class: "font-data text-[11px] text-ink", "No crate named “{name}” in this survey." }
                Link {
                    class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::CodeOverview {},
                    "← whole map"
                }
            }
        };
    }

    // Boundary references, aggregated per far file.
    let mut out: HashMap<u32, u32> = HashMap::new();
    let mut into: HashMap<u32, u32> = HashMap::new();
    for r in &graph.refs {
        match (ids.contains(&r.from), ids.contains(&r.to)) {
            (true, false) => *out.entry(r.to).or_default() += r.count,
            (false, true) => *into.entry(r.from).or_default() += r.count,
            _ => {}
        }
    }
    let rows = |m: HashMap<u32, u32>| -> Vec<(Route, String, String, u32)> {
        let mut v: Vec<(Route, String, String, u32)> = m
            .into_iter()
            .filter_map(|(id, count)| {
                let f = by_id.get(&id)?;
                Some((
                    file_route(&f.path),
                    file_name(&f.path).to_string(),
                    f.krate.clone(),
                    count,
                ))
            })
            .collect();
        v.sort_by_key(|(_, name, _, c)| (std::cmp::Reverse(*c), name.clone()));
        v
    };
    let uses_rows = rows(out);
    let used_rows = rows(into);
    let lines: u64 = files.iter().map(|f| f.lines as u64).sum();

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::CodeOverview {},
                    "← whole map"
                }
                h2 { class: "mt-1.5 break-all font-data text-[15px] font-semibold text-ink", "{name}" }
                p { class: "mt-0.5 font-data text-[10.5px] text-ink-soft",
                    "crate · {plural(files.len(), \"file\")} · {lines} lines"
                }
                Link {
                    class: "mt-1 inline-block font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                    to: Route::Focus { name: name.clone() },
                    "its dependencies ↑"
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Used outside ({used_rows.len()})"
                }
                if used_rows.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        "no other workspace crate references its code"
                    }
                } else {
                    RefList { rows: used_rows }
                }
                h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Uses outside ({uses_rows.len()})"
                }
                if uses_rows.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        "its code references no other workspace crate"
                    }
                } else {
                    RefList { rows: uses_rows }
                }
                h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Files ({files.len()})"
                }
                ul { class: "mt-1",
                    for f in files {
                        li {
                            Link {
                                to: file_route(&f.path),
                                class: "flex w-full items-center gap-1.5 px-1 py-0.5 hover:bg-ink/5",
                                FileMarkSvg {
                                    refs_in: f.refs_in_files,
                                    focal: false,
                                    box_px: 16.0,
                                }
                                span { class: "truncate font-data text-[10.5px] text-ink", "{f.path}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
