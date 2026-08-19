//! Code-altitude furniture: the cartouche, search, the legend, the crate
//! sheet, and the drawn vocabulary the map and the focus plate share. All of
//! it the same engraved ink.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, FileInfo, ItemKind};
use crate::views::codemap::file_route;

pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map(|(_, n)| n).unwrap_or(path)
}

pub(crate) fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

pub(crate) fn kind_words(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Fn => "fn",
        ItemKind::Struct => "struct",
        ItemKind::Enum => "enum",
        ItemKind::Union => "union",
        ItemKind::Trait => "trait",
        ItemKind::TypeAlias => "type",
        ItemKind::Const => "const",
        ItemKind::Static => "static",
        ItemKind::Macro => "macro",
        ItemKind::Mod => "mod",
        ItemKind::Impl => "impl",
    }
}

/// The tiny glyph vocabulary for items, shared by the map's landmark rows, the
/// focus plate, and the legend. Every glyph is ink; kind is shape, never
/// color.
#[component]
pub fn ItemGlyph(kind: ItemKind, #[props(default = 12.0)] box_px: f64) -> Element {
    let c = box_px / 2.0;
    rsx! {
        svg {
            class: "shrink-0",
            width: "{box_px}",
            height: "{box_px}",
            view_box: "0 0 {box_px} {box_px}",
            "aria-hidden": "true",
            match kind {
                ItemKind::Fn => rsx! {
                    circle { cx: "{c}", cy: "{c}", r: "2.6", fill: "var(--color-ink)" }
                },
                ItemKind::Struct => rsx! {
                    rect {
                        x: "{c - 2.6}",
                        y: "{c - 2.6}",
                        width: "5.2",
                        height: "5.2",
                        fill: "var(--color-ink)",
                    }
                },
                ItemKind::Enum => rsx! {
                    rect {
                        x: "{c - 2.7}",
                        y: "{c - 2.7}",
                        width: "5.4",
                        height: "5.4",
                        fill: "var(--color-ink)",
                        transform: "rotate(45 {c} {c})",
                    }
                },
                ItemKind::Union => rsx! {
                    rect {
                        x: "{c - 2.7}",
                        y: "{c - 2.7}",
                        width: "5.4",
                        height: "5.4",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                        transform: "rotate(45 {c} {c})",
                    }
                    circle { cx: "{c}", cy: "{c}", r: "1.1", fill: "var(--color-ink)" }
                },
                ItemKind::Trait => rsx! {
                    path {
                        d: "M{c} {c - 3.1} L{c + 3.1} {c + 2.6} L{c - 3.1} {c + 2.6} Z",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                    }
                },
                ItemKind::TypeAlias => rsx! {
                    rect {
                        x: "{c - 2.7}",
                        y: "{c - 2.7}",
                        width: "5.4",
                        height: "5.4",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                    }
                },
                ItemKind::Const | ItemKind::Static => rsx! {
                    rect {
                        x: "{c - 3.2}",
                        y: "{c - 0.9}",
                        width: "6.4",
                        height: "1.8",
                        fill: "var(--color-ink)",
                    }
                },
                ItemKind::Macro => rsx! {
                    g {
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                        stroke_linecap: "round",
                        line { x1: "{c}", y1: "{c - 3.2}", x2: "{c}", y2: "{c + 3.2}" }
                        line {
                            x1: "{c - 2.8}",
                            y1: "{c - 1.6}",
                            x2: "{c + 2.8}",
                            y2: "{c + 1.6}",
                        }
                        line {
                            x1: "{c + 2.8}",
                            y1: "{c - 1.6}",
                            x2: "{c - 2.8}",
                            y2: "{c + 1.6}",
                        }
                    }
                },
                ItemKind::Mod => rsx! {
                    rect {
                        x: "{c - 2.8}",
                        y: "{c - 2.8}",
                        width: "5.6",
                        height: "5.6",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                    }
                    circle { cx: "{c}", cy: "{c}", r: "1.2", fill: "var(--color-ink)" }
                },
                ItemKind::Impl => rsx! {
                    g { stroke: "var(--color-ink)", stroke_width: "1.1",
                        line { x1: "{c - 3.0}", y1: "{c - 3.0}", x2: "{c - 3.0}", y2: "{c + 3.0}" }
                        line { x1: "{c - 3.0}", y1: "{c - 3.0}", x2: "{c + 1.0}", y2: "{c - 3.0}" }
                        line { x1: "{c - 3.0}", y1: "{c + 3.0}", x2: "{c + 1.0}", y2: "{c + 3.0}" }
                    }
                },
            }
        }
    }
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

/// The code map's title block.
#[component]
pub fn CodeCartouche(graph: CodeGraph, workspace: String, epoch_line: String) -> Element {
    let files = graph.files.len();
    let crates: std::collections::HashSet<&str> =
        graph.files.iter().map(|f| f.krate.as_str()).collect();
    let lines: u64 = graph.files.iter().map(|f| f.lines as u64).sum();
    let items = graph.items.len();
    let pubs = graph
        .items
        .iter()
        .filter(|i| i.vis == crate::api::Vis::Pub)
        .count();
    let changed = graph.files.iter().filter(|f| f.changed).count();

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
                    "{lines} lines · {items} items, {pubs} pub"
                }
                p { class: "text-ink-soft", "{epoch_line}" }
                if changed > 0 {
                    p { class: "text-flare",
                        "▎{plural(changed, \"file\")} touched this epoch"
                    }
                } else {
                    p { class: "text-ink-soft", "nothing touched in this epoch" }
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
                                    class: if i == active() { "flex w-full items-baseline gap-1.5 px-2.5 py-1 bg-ink/5" } else { "flex w-full items-baseline gap-1.5 px-2.5 py-1 hover:bg-ink/5" },
                                    onclick: move |_| query.set(String::new()),
                                    span { class: "truncate font-data text-[11px] text-ink",
                                        "{file_name(&hit.path)}"
                                    }
                                    if hit.changed {
                                        span { class: "shrink-0 font-data text-[9px] text-flare", "▎" }
                                    }
                                    span { class: "ml-auto shrink-0 truncate font-data text-[9px] text-ink-soft",
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

/// One tie sample for the legend: a hairline with the arrowhead resting on
/// the user's end.
#[component]
fn TieSample(#[props(default = 1.3)] width: f64) -> Element {
    rsx! {
        svg {
            width: "36",
            height: "8",
            view_box: "0 0 36 8",
            class: "shrink-0",
            "aria-hidden": "true",
            line {
                x1: "0",
                y1: "4",
                x2: "33",
                y2: "4",
                stroke: "var(--color-ink-line)",
                stroke_width: "{width}",
            }
            path {
                d: "M28.5 1.4 L33.5 4 L28.5 6.6",
                fill: "none",
                stroke: "var(--color-ink-line)",
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
            // The key reads first — every territory, mark and line named in
            // words — then the gestures, then the survey's own honesty notes.
            div { class: "legend-scroll space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink sm:max-h-[42dvh]",
                p {
                    "every directory holds territory; a file is a block inside it. nesting means one thing only: "
                    span { class: "text-ink", "belongs to" }
                    "."
                }
                div { class: "space-y-2 border-t border-ink-line pt-2.5",
                    div { class: "flex items-baseline gap-2",
                        div { class: "flex shrink-0 items-baseline gap-1",
                            ItemGlyph { kind: ItemKind::Struct, box_px: 12.0 }
                            ItemGlyph { kind: ItemKind::Enum, box_px: 12.0 }
                            ItemGlyph { kind: ItemKind::Fn, box_px: 12.0 }
                            ItemGlyph { kind: ItemKind::Trait, box_px: 12.0 }
                        }
                        span { "struct · enum · fn · trait — engraved size follows fan-in" }
                    }
                    div { class: "grid grid-cols-2 gap-x-3 gap-y-1",
                        for (kind , words) in [
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
                    div { class: "flex items-center gap-2",
                        TieSample { width: 1.9 }
                        span { "×n — every reference between two territories, summed" }
                    }
                    p { class: "text-ink-soft",
                        "the arrow rests on the user — the way change travels. fold a district and the ties into everything inside it gather onto its gate; open it and they redistribute."
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    p {
                        span { class: "font-medium", "+ n folded" }
                        " — a block's last line counts what it hides: items too quiet for this altitude, and every private one."
                    }
                    p { class: "text-ink-soft",
                        "private items are never drawn — that fold is permanent — but their references to other blocks lift to the block that holds them and stay counted, which is why a block can be tied to a module none of its named items mention."
                    }
                    p {
                        span { class: "text-flare", "▎" }
                        span { class: "text-ink-soft", " touched in this epoch" }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    UsageRow { gesture: "click a file", effect: "focus it — its plate, and both directions of its references" }
                    UsageRow { gesture: "click an item", effect: "focus that item instead" }
                    UsageRow { gesture: "click a district", effect: "fold it to a counted gate; click the gate to open" }
                    UsageRow { gesture: "hover a block", effect: "its ties come up to full ink, lighter ties show their counts" }
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
    let changed = files.iter().filter(|f| f.changed).count();

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
                if changed > 0 {
                    p { class: "font-data text-[10px] text-flare",
                        "▎{plural(changed, \"file\")} touched this epoch"
                    }
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
                                class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 hover:bg-ink/5",
                                span { class: "truncate font-data text-[10.5px] text-ink", "{f.path}" }
                                if f.changed {
                                    span { class: "shrink-0 font-data text-[9px] text-flare", "▎" }
                                }
                                span { class: "ml-auto shrink-0 font-data text-[9px] text-ink-soft",
                                    "{f.lines} L"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
