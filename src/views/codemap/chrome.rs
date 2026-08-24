//! Code-altitude furniture: the cartouche, search, the legend, the crate
//! sheet, and the drawn vocabulary the map and the focus plate share. All of
//! it the same engraved ink.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, FileInfo, ItemKind, ItemMark, Vis};
use crate::views::codemap::{RefDir, file_route, item_route, use_code};

pub(crate) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        return format!("{n} {word}");
    }
    // English, not `+ "s"`: `alias` takes `es`, and the cartouche was printing
    // `2 aliass`. Only the endings rust's own vocabulary actually hands us.
    let suffix = match word.chars().last() {
        Some('s') | Some('x') | Some('z') => "es",
        Some('h') if word.ends_with("ch") || word.ends_with("sh") => "es",
        _ => "s",
    };
    format!("{n} {word}{suffix}")
}

pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map(|(_, n)| n).unwrap_or(path)
}

pub(super) fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// An item's kind, as rust writes it. The keyword is the representation every
/// rust reader already has; there is nothing to learn.
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

/// `pub fn`, `struct`, `pub(crate) mod` — what rust writes in front of a name.
/// A private item declares no visibility, so neither does its row.
pub(crate) fn decl_words(vis: Vis, kind: ItemKind) -> String {
    match vis.keyword() {
        Some(vis) => format!("{vis} {}", kind_words(kind)),
        None => kind_words(kind).to_string(),
    }
}

/// Which rung of the ladder a cartouche stands on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Altitude {
    /// `/dep` — crates on rings of hops.
    Deps,
    /// `/code` — files and items as nested territory.
    Code,
    /// `/data` — the workspace's state, tiered into roots and what they hold.
    Data,
}

/// The altitude line: the ladder between the charts, and the only navigation
/// between them. The current rung is engraved solid; the others are links.
#[component]
pub(crate) fn AltitudeSwitch(at: Altitude) -> Element {
    let rung = |label: &'static str, to: Route, mine: Altitude| {
        rsx! {
            if at == mine {
                span { class: "text-ink underline underline-offset-4", "{label}" }
            } else {
                Link {
                    class: "text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to,
                    "{label}"
                }
            }
        }
    };
    rsx! {
        p { class: "flex gap-3 font-data text-[9.5px] tracking-[0.14em] uppercase",
            {rung("dependencies", Route::DepOverview {}, Altitude::Deps)}
            {rung("code", Route::CodeOverview {}, Altitude::Code)}
            {rung("data", Route::DataOverview {}, Altitude::Data)}
        }
    }
}

/// The code map's title block. It holds the survey's totals — the map itself no
/// longer repeats them on every frame and every block — and the reading control
/// for the map's ties, which acts on the whole plate and so belongs here.
#[component]
pub(super) fn CodeCartouche(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let files = graph.files.len();
    let crates: std::collections::HashSet<&str> =
        graph.files.iter().map(|f| f.krate.as_str()).collect();
    let lines: u64 = graph.files.iter().map(|f| f.lines as u64).sum();
    let items = graph.items.len();
    let changed = graph.files.iter().filter(|f| f.changed).count();

    rsx! {
        section { class: "plate pointer-events-auto",
            div { class: "px-4 pt-3 pb-2",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{workspace}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "{plural(files, \"file\")} · {items} items · {lines} lines"
                    if crates.len() > 1 {
                        " · {plural(crates.len(), \"crate\")}"
                    }
                }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Code }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if changed > 0 {
                        p { class: "text-flare", "{plural(changed, \"file\")} changed" }
                    } else {
                        p { class: "text-ink-soft", "no files changed" }
                    }
                }
            }
            RefDirToggle {}
        }
    }
}

/// Find a file by any part of its path, or an item by name. One list: the
/// reviewer is looking for a place in the code, and a file and an item are
/// both places.
#[derive(Clone, PartialEq)]
enum SearchHit {
    File(FileInfo),
    /// The item, and the path of the file that defines it. Boxed because
    /// an `ItemMark` is 296 bytes against `FileInfo`'s 88, and every hit
    /// in the list would otherwise pay for the larger of the two.
    Item(Box<ItemMark>, String),
}

/// How one hit ranks: a prefix match is what the reviewer meant, files come
/// before items, and the rest ranks by how much of the workspace leans on it.
type Rank = (bool, u8, std::cmp::Reverse<u32>, String);

#[component]
pub(super) fn CodeSearch(graph: CodeGraph) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let nav = use_navigator();

    let results = use_memo(move || {
        let q = query().trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(Rank, SearchHit)> = Vec::new();
        for file in graph
            .files
            .iter()
            .filter(|f| f.path.to_lowercase().contains(&q))
        {
            let name = file_name(&file.path).to_lowercase();
            hits.push((
                (
                    !name.starts_with(&q),
                    0,
                    std::cmp::Reverse(file.refs_in_files),
                    file.path.clone(),
                ),
                SearchHit::File(file.clone()),
            ));
        }
        for item in graph
            .items
            .iter()
            .filter(|m| m.label.to_lowercase().contains(&q))
        {
            let path = graph
                .files
                .get(item.file as usize)
                .map(|f| f.path.clone())
                .unwrap_or_default();
            hits.push((
                (
                    !item.name.to_lowercase().starts_with(&q),
                    1,
                    std::cmp::Reverse(item.fan_in),
                    item.label.clone(),
                ),
                SearchHit::Item(Box::new(item.clone()), path),
            ));
        }
        hits.sort_by(|a, b| a.0.cmp(&b.0));
        hits.truncate(10);
        hits.into_iter().map(|(_, hit)| hit).collect()
    });

    let route_of = |hit: &SearchHit| match hit {
        SearchHit::File(f) => file_route(&f.path),
        SearchHit::Item(m, path) => item_route(path, &m.label),
    };

    rsx! {
        div { class: "pointer-events-auto relative w-full",
            input {
                id: "code-search",
                class: "plate w-full px-3 py-1.5 font-data text-[11px] text-ink placeholder:text-ink-soft focus:outline-none",
                r#type: "search",
                placeholder: "find a file or item…   /",
                autocomplete: "off",
                spellcheck: "false",
                "aria-label": "Find a file or item",
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
                                nav.push(route_of(hit));
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
                        p { class: "font-data text-[10px] text-ink-soft", "no matches" }
                    }
                } else {
                    ul { class: "plate absolute left-0 right-0 top-full z-20 mt-1 max-h-72 overflow-auto py-1",
                        for (i , hit) in results().into_iter().enumerate() {
                            li {
                                Link {
                                    to: route_of(&hit),
                                    class: if i == active() { "flex w-full items-baseline gap-1.5 px-2.5 py-1 bg-ink/5" } else { "flex w-full items-baseline gap-1.5 px-2.5 py-1 hover:bg-ink/5" },
                                    onclick: move |_| query.set(String::new()),
                                    match &hit {
                                        SearchHit::File(f) => rsx! {
                                            span { class: "truncate font-data text-[11px] text-ink", "{file_name(&f.path)}" }
                                            if f.changed {
                                                span { class: "shrink-0 font-data text-[9.5px] text-flare", "M" }
                                            }
                                            span { class: "ml-auto shrink-0 truncate font-data text-[9px] text-ink-soft",
                                                "{dir_of(&f.path)}"
                                            }
                                        },
                                        SearchHit::Item(m, path) => rsx! {
                                            span { class: "shrink-0 font-data text-[9.5px] text-ink-soft", "{kind_words(m.kind)}" }
                                            span { class: "truncate font-data text-[11px] text-ink", "{m.name}" }
                                            span { class: "ml-auto shrink-0 truncate font-data text-[9px] text-ink-soft",
                                                "{path}:{m.line}"
                                            }
                                        },
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

/// One row of a legend's gesture section: the gesture in tracked caps, what it
/// does beside it. Shared by all three chart legends — one shape for the whole
/// ladder, so a reader who learned one legend has learned the others.
///
/// It renders two cells, not a row, so the caller can seat them in [`Gestures`]'
/// grid: at 224px of plate every gesture used to set its own label column, and
/// the effects came out ragged.
#[component]
pub(crate) fn UsageRow(gesture: &'static str, effect: &'static str) -> Element {
    rsx! {
        span { class: "font-data text-[9.5px] tracking-[0.1em] uppercase text-ink", "{gesture}" }
        span { class: "text-ink-soft", "{effect}" }
    }
}

/// The gesture rows, on one grid so every label sets in the same column and
/// every effect starts at the same rule.
#[component]
pub(crate) fn Gestures(children: Element) -> Element {
    rsx! {
        div { class: "grid grid-cols-[auto_1fr] items-baseline gap-x-2 gap-y-1 border-t border-ink-line pt-2.5",
            {children}
        }
    }
}

/// The key: every mark the map draws that the map cannot state for itself, the
/// gestures, and the survey's own limits behind a fold.
///
/// It explains no control that carries its own label and tooltip (2026-08-21,
/// distill): the references toggle had a paragraph here restating its three
/// button titles word for word. It repeats nothing the drawing already says — a
/// block is a file, a frame is its directory, a row is written as rust. And it
/// paraphrases no note: the survey's limits print in the survey's own words,
/// and only the ones about references, this altitude's whole subject.
///
/// The plate takes the column's remainder rather than a hand-set height: the
/// key is short enough to stand open, and the cartouche above it grows and
/// shrinks with the diff.
#[component]
pub(super) fn CodeLegend(
    notes: Vec<String>,
    /// Whether the diff touched anything, so the `M` key is drawn only where
    /// there is an `M` on the paper.
    changed: bool,
    #[props(default = true)] start_open: bool,
) -> Element {
    rsx! {
        details {
            class: "plate fold legend-plate pointer-events-auto flex min-h-0 w-full flex-col open:pb-3 sm:w-64",
            open: start_open,
            summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this map"
            }
            div { class: "legend-scroll min-h-0 flex-1 space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink max-h-[42dvh] sm:max-h-none",
                div { class: "space-y-1.5",
                    div { class: "flex items-center gap-2",
                        TieSample { width: 1.9 }
                        span { "n references, summed" }
                    }
                    p { class: "text-ink-soft",
                        "the arrow rests on the user — the way change travels."
                    }
                    p {
                        span { class: "font-medium", "+ 4 pub · 5 private" }
                        span { class: "text-ink-soft",
                            " — what a block holds back. a folded directory\u{2019}s gate counts the same way, and every reference into what it holds lands on it."
                        }
                    }
                    // Only where there is a diff: a key for a mark the map is
                    // not drawing is dead weight (2026-08-21, distill).
                    if changed {
                        p {
                            span { class: "text-flare", "M" }
                            span { class: "text-ink-soft", " — changed since the diff base" }
                        }
                    }
                }
                Gestures {
                    UsageRow { gesture: "click", effect: "a directory folds to a counted gate" }
                    UsageRow { gesture: "hover", effect: "every tie of one block, with its count" }
                    UsageRow { gesture: "/ · f", effect: "find a file or item · refit" }
                    UsageRow { gesture: "← · → · esc", effect: "back · forward · step up" }
                }
                SurveyLimits { notes }
            }
        }
    }
}

/// The survey\u{2019}s own limits, folded. They are read once, to trust the chart —
/// not consulted while reading it — so they rest behind one line instead of
/// spending a third of the plate at every altitude. Every altitude\u{2019}s legend
/// closes with this, and the words are the survey\u{2019}s, never a paraphrase.
#[component]
pub(crate) fn SurveyLimits(notes: Vec<String>) -> Element {
    if notes.is_empty() {
        return rsx! {};
    }
    rsx! {
        details { class: "fold border-t border-ink-line pt-2.5",
            summary { class: "cursor-pointer select-none font-data text-[9.5px] tracking-[0.1em] uppercase text-ink",
                "what the survey cannot read"
            }
            div { class: "mt-1.5 space-y-1 text-ink-soft",
                for (i , note) in notes.iter().enumerate() {
                    p { key: "{i}", "{note}" }
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
                                "{count} refs"
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

/// Which reading of the map's ties is drawn. It rides on the cartouche because
/// it acts on the whole plate: each block draws its heaviest ties in the chosen
/// direction, and hovering a block reveals the rest. `both` is the unthinned
/// picture. Active segment wears a 1px ink border — no fills on this plate.
#[component]
fn RefDirToggle() -> Element {
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
                    let mut dir = code.ref_dir;
                    dir.set(val);
                },
                "{label}"
            }
        }
    };
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "which reading of the map's references is drawn",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "references"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("uses", "each file's heaviest references out — what it leans on", RefDir::Uses)}
                {seg("used by", "each file's heaviest references in — who leans on it", RefDir::UsedBy)}
                {seg("both", "every reference between files, unthinned", RefDir::Both)}
            }
        }
    }
}

/// One crate's sheet: its files, and what crosses its boundary.
#[component]
pub(super) fn CratePanel(graph: CodeGraph, name: String) -> Element {
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
            div { class: "px-4 pt-3 pb-2",
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
                        "{plural(changed, \"file\")} changed"
                    }
                }
                Link {
                    class: "mt-1 inline-block font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                    to: Route::DepFocus { name: name.clone() },
                    "its dependencies ↑"
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                // A crate no workspace crate touches has one thing to say, not
                // two empty headings and a toggle with nothing to act on.
                if used_rows.is_empty() && uses_rows.is_empty() {
                    p { class: "mt-3 border-t border-ink-line pt-3 font-data text-[10px] leading-relaxed text-ink-soft",
                        "no references cross this crate's boundary"
                    }
                } else {
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
                                    span { class: "shrink-0 font-data text-[9px] text-flare", "M" }
                                }
                                span { class: "ml-auto shrink-0 font-data text-[9px] text-ink-soft",
                                    "{f.lines} lines"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
