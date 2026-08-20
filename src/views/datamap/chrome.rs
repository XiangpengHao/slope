//! Data-altitude furniture: the cartouche, the reading toggle, and the legend.
//! The same engraved plates the other two altitudes wear.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, HoldKind};
use crate::views::codemap::chrome::{Altitude, AltitudeSwitch, decl_words, plural};
use crate::views::codemap::{RefDir, item_route, use_code};
use crate::views::datamap::data_type_route;
use crate::views::datamap::model::{Anchor, DataFacts, DataModel, upstream};

/// Which top-level modules a change landed in, in plain words. The chart shows
/// a reviewer where the amber is; the cartouche says it out loud, because that
/// one sentence is the answer to why they climbed to this altitude.
fn insight(modules: &[String]) -> Option<String> {
    match modules {
        [] => None,
        [one] => Some(format!("changed types sit in {one} alone")),
        [a, b] => Some(format!("changed types sit in {a} and {b}")),
        rest => {
            let (last, first) = rest.split_last()?;
            Some(format!(
                "changed types sit in {} and {last}",
                first.join(", ")
            ))
        }
    }
}

/// The data chart's title block: what the workspace holds, what the diff moved,
/// and the reading control for the chart's reference ties.
#[component]
pub fn DataCartouche(facts: DataFacts, workspace: String, diff_line: String) -> Element {
    let insight = insight(&facts.changed_modules);
    rsx! {
        section { class: "plate pointer-events-auto",
            div { class: "px-4 pt-3 pb-2",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{workspace}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "{plural(facts.structs, \"struct\")} · {plural(facts.enums, \"enum\")} · {facts.roots} roots"
                }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Data }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if facts.changed > 0 {
                        p { class: "text-flare",
                            "{plural(facts.changed, \"type\")} in changed files"
                        }
                        if let Some(insight) = insight {
                            p { class: "text-ink-soft", "{insight}" }
                        }
                    } else {
                        p { class: "text-ink-soft", "no files changed" }
                    }
                }
            }
            DataRefToggle {}
        }
    }
}

/// Which reading of the chart's reference ties is drawn. It rides on the
/// cartouche because it acts on the whole plate, and it is the same reading the
/// code map is set to — one reviewer, one question, at either altitude.
#[component]
pub fn DataRefToggle() -> Element {
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
            "aria-label": "which reading of the chart's references is drawn",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "references"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("uses", "each type's heaviest references out — what it reaches for", RefDir::Uses)}
                {seg("used by", "each type's heaviest references in — who names it", RefDir::UsedBy)}
                {seg("both", "every reference between two types, unthinned", RefDir::Both)}
            }
        }
    }
}

/// A hold's kind, in its own lowercase word, for a sheet row's far column.
/// The wrapper's word wins where the walk met one — `Arc` says more than
/// `shares` — and the kind speaks only for a plain hold.
fn hold_word(kind: HoldKind, via: &str) -> String {
    if !via.is_empty() {
        return via.to_string();
    }
    match kind {
        HoldKind::Owns => "owns",
        HoldKind::Shares => "shares",
        HoldKind::Borrows => "borrows",
        HoldKind::Dyn => "dyn",
    }
    .to_string()
}

/// One row of the sheet's holds lists: a drawn type (a link that re-centers
/// the selection on it), or a frame's counted fold row, which is words.
#[derive(Clone, PartialEq)]
struct HoldRow {
    to: Option<Route>,
    decl: String,
    name: String,
    changed: bool,
    word: String,
}

/// The rows one side of the selection draws, from each hold's far end. A
/// fold-row end names its count and its module instead of a type — the
/// chart's own words for what it does not draw.
fn hold_rows(model: &DataModel, holds: Vec<(&Anchor, HoldKind, &str)>) -> Vec<HoldRow> {
    let by_id: std::collections::HashMap<u32, &crate::views::datamap::model::DataMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();
    holds
        .into_iter()
        .map(|(anchor, kind, via)| match anchor {
            Anchor::Mark(id) => {
                let mark = by_id.get(id);
                HoldRow {
                    to: mark.map(|m| data_type_route(&m.path, &m.label)),
                    decl: mark.map(|m| decl_words(m.vis, m.kind)).unwrap_or_default(),
                    name: mark.map(|m| m.name.clone()).unwrap_or_default(),
                    changed: mark.is_some_and(|m| m.changed),
                    word: hold_word(kind, via),
                }
            }
            Anchor::Private(frame) | Anchor::More(frame) => {
                let frame = &model.frames[*frame as usize];
                let count = if matches!(anchor, Anchor::Private(_)) {
                    format!("+ {}", plural(frame.private as usize, "private type"))
                } else {
                    format!("+ {}", plural(frame.more as usize, "more type"))
                };
                let place = frame
                    .label(model.multi_crate)
                    .unwrap_or_else(|| frame.krate.clone());
                HoldRow {
                    to: None,
                    decl: String::new(),
                    name: format!("{count} · {place}"),
                    changed: false,
                    word: hold_word(kind, via),
                }
            }
        })
        .collect()
}

/// One chunked list of hold rows: the first eight, then a typographic
/// "show all n".
#[component]
fn HoldList(rows: Vec<HoldRow>) -> Element {
    let mut all = use_signal(|| false);
    let total = rows.len();
    let shown = if all() || total <= 8 { total } else { 8 };
    rsx! {
        ul { class: "mt-1",
            for (i , row) in rows.iter().take(shown).enumerate() {
                li { key: "{i}",
                    if let Some(to) = row.to.clone() {
                        Link {
                            class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] hover:bg-ink/5",
                            to,
                            if !row.decl.is_empty() {
                                span { class: "shrink-0 text-ink-soft", "{row.decl}" }
                            }
                            span { class: "truncate font-medium text-ink", "{row.name}" }
                            if row.changed {
                                span { class: "shrink-0 font-bold text-flare", "M" }
                            }
                            span { class: "ml-auto shrink-0 text-[9px] text-ink-soft", "{row.word}" }
                        }
                    } else {
                        span { class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] text-ink-soft",
                            span { class: "truncate", "{row.name}" }
                            span { class: "ml-auto shrink-0 text-[9px]", "{row.word}" }
                        }
                    }
                }
            }
        }
        if shown < total {
            button {
                class: "mt-1 px-1 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                onclick: move |_| all.set(true),
                "show all {total}"
            }
        }
    }
}

/// One selected type's sheet: who holds it, what it holds, and the one step
/// further to its definition. The chart keeps the selection's blast radius
/// inked; this plate says the same thing in rows a reader can follow.
#[component]
pub fn DataSheet(graph: CodeGraph, path: String, item: String) -> Element {
    let code = use_code();
    // The sheet reads holding structure, never the tie reading, so the
    // toggle is peeked: it moves nothing on this plate.
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *code.ref_dir.peek())
    }));
    let model = model.read();

    let Some(mark) = model
        .marks
        .iter()
        .find(|m| m.path == path && m.label == item)
    else {
        return rsx! {
            section { class: "plate pointer-events-auto w-full px-4 py-3 sm:w-72",
                p { class: "font-data text-[11px] text-ink",
                    "No type “{item}” in {path} on this survey."
                }
                Link {
                    class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::DataOverview {},
                    "← whole chart"
                }
            }
        };
    };

    let at = Anchor::Mark(mark.id);
    let decl = decl_words(mark.vis, mark.kind);
    let held_by: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.held == at)
            .map(|h| (&h.holder, h.kind, h.via.as_str()))
            .collect(),
    );
    let holds: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.holder == at)
            .map(|h| (&h.held, h.kind, h.via.as_str()))
            .collect(),
    );
    // The blast radius in one line: how much further than its direct holders
    // a change to this shape travels.
    let pairs: Vec<(Anchor, Anchor)> = model.holds.iter().map(|h| (h.held, h.holder)).collect();
    let direct: std::collections::HashSet<Anchor> = model
        .holds
        .iter()
        .filter(|h| h.held == at)
        .map(|h| h.holder)
        .collect();
    let beyond = upstream(&pairs, at)
        .iter()
        .filter(|a| matches!(a, Anchor::Mark(_)) && !direct.contains(a))
        .count();

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3 pb-2",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::DataOverview {},
                    "← whole chart"
                }
                h2 { class: "mt-1.5 flex items-baseline gap-1.5 font-data text-[15px]",
                    span { class: "shrink-0 text-[11px] text-ink-soft", "{decl}" }
                    span { class: "truncate font-semibold text-ink", "{mark.name}" }
                    if mark.changed {
                        span {
                            class: "shrink-0 font-bold text-flare",
                            title: "changed since the diff base",
                            "M"
                        }
                    }
                }
                p { class: "mt-0.5 font-data text-[9.5px] text-ink-soft", "{mark.locator()}" }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                h3 { class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Held by ({held_by.len()})"
                }
                if held_by.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        "no type holds it — a root."
                    }
                } else {
                    HoldList { rows: held_by }
                }
                if beyond > 0 {
                    p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                        "a shape change here reaches {plural(beyond, \"more type\")} upstream."
                    }
                }
                h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Holds ({holds.len()})"
                }
                if holds.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        "holds no workspace types."
                    }
                } else {
                    HoldList { rows: holds }
                }
            }
            div { class: "border-t border-ink-line px-4 py-2",
                Link {
                    class: "font-data text-[9.5px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                    to: item_route(&mark.path, &mark.label),
                    "open its definition →"
                }
            }
        }
    }
}

/// One drawn edge sample for the legend, in the chart's own grammar — the same
/// classes the chart itself draws with, so the key cannot drift from the map.
#[component]
fn WireSample(
    dash: &'static str,
    #[props(default = 1.1)] width: f64,
    #[props(default = "")] label: &'static str,
) -> Element {
    rsx! {
        svg {
            class: "mt-0.5 shrink-0",
            width: "46",
            height: "14",
            view_box: "0 0 46 14",
            "aria-hidden": "true",
            g { class: "data-wire {dash}",
                path {
                    class: "wire-path",
                    d: "M1,10 Q22,5 40,9",
                    fill: "none",
                    style: "stroke-width: {width}px;",
                }
                path { class: "wire-head", d: "M45,9.2 L38.4,6.2 L38.8,11.6 Z" }
                if !label.is_empty() {
                    text {
                        class: "wire-label",
                        x: "21",
                        y: "5",
                        text_anchor: "middle",
                        "{label}"
                    }
                }
            }
        }
    }
}

/// The key: every mark and line the chart can draw that it cannot state for
/// itself, then the walk's own honesty notes. What the drawing already says —
/// a block is a type, the frame around it is its module — is not repeated here.
#[component]
pub fn DataLegend(facts: DataFacts, #[props(default = true)] start_open: bool) -> Element {
    rsx! {
        details {
            class: "plate fold pointer-events-auto w-full open:pb-3 sm:w-64",
            open: start_open,
            summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this chart"
            }
            div { class: "legend-scroll space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink max-h-[42dvh] sm:max-h-[calc(100dvh_-_300px)]",
                div { class: "space-y-1.5",
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-owns" }
                        span {
                            span { class: "text-ink", "owns" }
                            span { class: "text-ink-soft",
                                " — a field of this type. the arrowhead rests on the holder: a shape change travels along the arrow. a block sits under the same-module block that owns it hardest; ownership from another module stays a drawn line."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-shares", width: 1.3, label: "Arc" }
                        span {
                            span { class: "text-ink", "shares" }
                            span { class: "text-ink-soft",
                                " — held through a shared handle, the wrapper's own word on the line. more than one holder can reach the same value."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-borrows", label: "&" }
                        span {
                            span { class: "text-ink", "borrows" }
                            span { class: "text-ink-soft",
                                " — a reference: the holder views state something else owns. "
                            }
                            span { class: "text-ink", "dyn" }
                            span { class: "text-ink-soft", " names a trait instead of a type." }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-ref", width: 1.6 }
                        span {
                            span { class: "text-ink", "references" }
                            span { class: "text-ink-soft",
                                " — lighter, and a reading rather than structure: how often one type names another, summed, with the arrow on the user."
                            }
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    div { class: "flex items-start gap-2",
                        svg {
                            class: "mt-0.5 shrink-0",
                            width: "46",
                            height: "14",
                            view_box: "0 0 46 14",
                            "aria-hidden": "true",
                            rect {
                                x: "1",
                                y: "1",
                                width: "44",
                                height: "12",
                                fill: "var(--color-paper)",
                                stroke: "var(--color-ink-line)",
                            }
                            rect {
                                x: "1",
                                y: "1",
                                width: "2.5",
                                height: "12",
                                fill: "var(--color-ink)",
                            }
                        }
                        span {
                            span { class: "text-ink", "static" }
                            span { class: "text-ink-soft",
                                " — a root: state no type holds, drawn whether or not it is pub, with its declared type quoted under its name."
                            }
                        }
                    }
                    p {
                        span { class: "text-flare", "M" }
                        span { class: "text-ink-soft", " — defined in a file the diff touched" }
                    }
                    p {
                        span { class: "dm-nm", "Wire" }
                        span { class: "text-ink-soft", " · " }
                        span { class: "dm-nm is-sum", "HoldKind" }
                        span { class: "text-ink-soft",
                            " — a product type\u{2019}s name and a sum type\u{2019}s take different type colors. the keyword in front of each says the same thing in rust\u{2019}s own words."
                        }
                    }
                    p { class: "text-ink-soft",
                        "a block quotes every field and variant as written, colored by token class the way the definition plate colors its source. the bold run names the workspace type a field holds; a plain type name comes from outside the workspace, has no mark of its own, and so has no line drawn to it."
                    }
                    p {
                        span { class: "font-medium", "+ 4 more fields" }
                        span { class: "text-ink-soft",
                            " — rows past the block's eight quoted ones, held back only while it rests: selecting the block draws every field and variant it has. variants past theirs count the same way."
                        }
                    }
                    p {
                        span { class: "font-medium", "held by 6 types" }
                        span { class: "text-ink-soft",
                            " — more than three types hold this one, so its incoming edges rest folded. hover either end to ink them in."
                        }
                    }
                    p {
                        span { class: "font-medium", "+ 5 private types" }
                        span { class: "text-ink-soft",
                            " — a private type is never a mark, and every holds edge touching one lands on its module's counted row."
                        }
                    }
                    p { class: "text-ink-soft",
                        "the references toggle sets the reading: "
                        span { class: "text-ink", "uses" }
                        " and "
                        span { class: "text-ink", "used by" }
                        " rest each type\u{2019}s two heaviest ties, "
                        span { class: "text-ink", "both" }
                        " rests every one."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5 text-ink-soft",
                    p {
                        "the walk reads declared field types. "
                        span { class: "text-ink", "Arc" }
                        ", "
                        span { class: "text-ink", "Rc" }
                        ", "
                        span { class: "text-ink", "Weak" }
                        " and the dioxus signals — "
                        span { class: "text-ink", "Signal" }
                        ", "
                        span { class: "text-ink", "GlobalSignal" }
                        ", "
                        span { class: "text-ink", "ReadSignal" }
                        ", "
                        span { class: "text-ink", "Memo" }
                        ", "
                        span { class: "text-ink", "Resource" }
                        " — read as sharing; a reference as borrowing; "
                        span { class: "text-ink", "dyn Trait" }
                        " as its trait. every other generic type — Vec, Box, Option, HashMap, Mutex, an unknown external — is transparent, and the walk recurses into it."
                    }
                    p {
                        "references from free functions and trait items are not on this chart: a tie is kept only where both ends land on a drawn type."
                    }
                    p {
                        "type parameters are holes: their fields quote as written, and the walk reads nothing through them."
                    }
                    if facts.trait_holds > 0 {
                        p {
                            "{plural(facts.trait_holds, \"dyn hold\")} land on a trait, and a trait has no mark of its own yet."
                        }
                    }
                    if facts.unresolved > 0 {
                        p {
                            "{facts.unresolved} names could not be resolved (type-inference limits) and are not on the chart."
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    UsageRow {
                        gesture: "click a type",
                        effect: "select it: the block opens to every field and variant it quoted a count for, everything a shape change could reach keeps its ink, the rest recedes, and its sheet opens. its definition is one step further, on the sheet.",
                    }
                    UsageRow { gesture: "esc · bare paper", effect: "deselect" }
                    UsageRow { gesture: "hover a type", effect: "all of its edges, at full ink" }
                    UsageRow { gesture: "f · ← · →", effect: "refit the chart · back · forward" }
                }
            }
        }
    }
}

/// One row of the legend's gesture section.
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
