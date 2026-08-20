//! Data-altitude furniture: the cartouche, the reading toggle, and the legend.
//! The same engraved plates the other two altitudes wear.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, Delta, HoldEvent, HoldKind};
use crate::views::codemap::chrome::{Altitude, AltitudeSwitch, decl_words, plural};
use crate::views::codemap::{Doors, RefDir, item_route, use_code};
use crate::views::datamap::data_type_route;
use crate::views::datamap::model::{Anchor, DataFacts, DataModel, RowState, upstream};

/// Which top-level modules the diff landed in, in plain words. The chart shows
/// a reviewer where the amber is; the cartouche says it out loud, because that
/// one sentence is the answer to why they climbed to this altitude.
fn insight(modules: &[String]) -> Option<String> {
    match modules {
        [] => None,
        [one] => Some(format!("the diff lands in {one} alone")),
        [a, b] => Some(format!("the diff lands in {a} and {b}")),
        rest => {
            let (last, first) = rest.split_last()?;
            Some(format!("the diff lands in {} and {last}", first.join(", ")))
        }
    }
}

/// The structural diff's own line: only what happened, in git's order. The
/// counts cover types and statics alike, so no noun — the marks are right
/// there on the chart.
fn diff_words(facts: &DataFacts) -> String {
    let mut parts: Vec<String> = Vec::new();
    if facts.added > 0 {
        parts.push(format!("{} added", facts.added));
    }
    if facts.removed > 0 {
        parts.push(format!("{} removed", facts.removed));
    }
    if facts.changed > 0 {
        parts.push(format!("{} changed", facts.changed));
    }
    parts.join(" · ")
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
                    "{plural(facts.structs, \"struct\")} · {plural(facts.enums, \"enum\")} · {plural(facts.fns, \"fn\")} · {facts.roots} roots"
                }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Data }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if !diff_words(&facts).is_empty() {
                        p { class: "text-flare", "{diff_words(&facts)}" }
                        if let Some(insight) = insight {
                            p { class: "text-ink-soft", "{insight}" }
                        }
                    } else {
                        p { class: "text-ink-soft", "no shape changes since the base" }
                    }
                }
            }
            DataRefToggle {}
            DataDoorToggle {}
        }
    }
}

/// Which reading of the chart's reference ties is drawn. It rides on the
/// cartouche because it acts on the whole plate, and it is the same reading the
/// code map is set to — one reviewer, one question, at either altitude.
#[component]
fn DataRefToggle() -> Element {
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

/// Which doors earn a block. It rides on the cartouche under the reference
/// reading because it acts on the whole plate the same way: both are readings
/// of one survey, so moving either re-seats the chart without surveying the
/// workspace again.
#[component]
fn DataDoorToggle() -> Element {
    let code = use_code();
    let current = *code.doors.read();
    let seg = |label: &'static str, hint: &'static str, val: Doors| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                title: hint,
                onclick: move |_| {
                    let mut doors = code.doors;
                    doors.set(val);
                },
                "{label}"
            }
        }
    };
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "which types the chart draws a block for",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "visibility"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("pub", "only types visible outside their crate get a block; every other type folds to its module's count", Doors::Pub)}
                {seg("pub(crate)", "crate-visible types get blocks too; only types with no `pub` at all fold", Doors::Crate)}
                {seg("private", "every type gets a block, no-`pub` ones included; nothing folds for visibility", Doors::All)}
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
    letter: Option<&'static str>,
    word: String,
    /// The relation's own diff event, in its word.
    event: Option<&'static str>,
}

/// The rows one side of the selection draws, from each hold's far end. A
/// fold-row end names its count and its module instead of a type — the
/// chart's own words for what it does not draw.
fn hold_rows(
    model: &DataModel,
    holds: Vec<(&Anchor, HoldKind, &str, Option<HoldEvent>)>,
) -> Vec<HoldRow> {
    let by_id: std::collections::HashMap<u32, &crate::views::datamap::model::DataMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();
    holds
        .into_iter()
        .map(|(anchor, kind, via, event)| {
            let event = match event {
                Some(HoldEvent::Added) => Some("added"),
                Some(HoldEvent::Removed) => Some("removed"),
                None => None,
            };
            match anchor {
                Anchor::Mark(id) => {
                    let mark = by_id.get(id);
                    HoldRow {
                        to: mark.map(|m| data_type_route(&m.path, &m.label)),
                        decl: mark.map(|m| decl_words(m.vis, m.kind)).unwrap_or_default(),
                        name: mark.map(|m| m.name.clone()).unwrap_or_default(),
                        letter: mark.and_then(|m| m.letter()),
                        word: hold_word(kind, via),
                        event,
                    }
                }
                Anchor::Private(frame) | Anchor::More(frame) => {
                    let frame = &model.frames[*frame as usize];
                    let count = if matches!(anchor, Anchor::Private(_)) {
                        format!(
                            "+ {}",
                            plural(frame.private as usize, model.doors.fold_word())
                        )
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
                        letter: None,
                        word: hold_word(kind, via),
                        event,
                    }
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
                            if let Some(letter) = row.letter {
                                span { class: "shrink-0 font-bold text-flare", "{letter}" }
                            }
                            span { class: "ml-auto shrink-0 text-[9px] text-ink-soft", "{row.word}" }
                            if let Some(event) = row.event {
                                span { class: "shrink-0 text-[9px] text-flare", "{event}" }
                            }
                        }
                    } else {
                        span { class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] text-ink-soft",
                            span { class: "truncate", "{row.name}" }
                            span { class: "ml-auto shrink-0 text-[9px]", "{row.word}" }
                            if let Some(event) = row.event {
                                span { class: "shrink-0 text-[9px] text-flare", "{event}" }
                            }
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

/// One selected mark's sheet: who holds it, which contracts name it, what it
/// holds, and the one step further to its definition. A function has only the
/// last of those — a signature names types; nothing names a function back. The
/// chart keeps the selection's blast radius inked; this plate says the same
/// thing in rows a reader can follow.
#[component]
pub fn DataSheet(graph: CodeGraph, path: String, item: String) -> Element {
    let code = use_code();
    // The sheet reads holding structure, never the tie reading, so that
    // toggle is peeked: it moves nothing on this plate. The doors are read —
    // they decide whether a held type is a link here or part of a count.
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *code.ref_dir.peek(), *code.doors.read())
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
                    "Nothing named “{item}” is drawn in {path} on this survey."
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
    // A function that names this type in its signature is not a holder: it
    // keeps nothing. The two readings sit in sections of their own, so
    // "held by" never quietly means "mentioned by".
    let contract = |anchor: &Anchor| match anchor {
        Anchor::Mark(id) => model.marks.iter().any(|m| m.id == *id && m.is_fn()),
        _ => false,
    };
    let held_by: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.held == at && !contract(&h.holder))
            .map(|h| (&h.holder, h.kind, h.via.as_str(), h.event))
            .collect(),
    );
    let contracts: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.held == at && contract(&h.holder))
            .map(|h| (&h.holder, h.kind, h.via.as_str(), h.event))
            .collect(),
    );
    let holds: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.holder == at)
            .map(|h| (&h.held, h.kind, h.via.as_str(), h.event))
            .collect(),
    );
    // The selection's own diff, in words: its letter's sentence, then every
    // added and dropped row exactly as the block draws them.
    let change_rows: Vec<(&'static str, String, bool)> = mark
        .fields
        .iter()
        .chain(mark.variants.iter())
        .filter_map(|row| {
            let mk = row.state.marker()?;
            let text = if row.name.is_empty() {
                row.decl.clone()
            } else {
                format!("{}: {}", row.name, row.decl)
            };
            Some((mk, text, row.state == RowState::Removed))
        })
        .collect();
    let change_line = if mark.ghost {
        Some("removed since the base — this block quotes the base edition.")
    } else if mark.delta == Delta::Added {
        Some("added since the base.")
    } else {
        None
    };
    // The blast radius in one line: how much further than its direct holders
    // a change to this shape travels. Contracts are counted apart from shapes
    // — a signature that has to change is not another type that has to.
    let pairs: Vec<(Anchor, Anchor)> = model.holds.iter().map(|h| (h.held, h.holder)).collect();
    let direct: std::collections::HashSet<Anchor> = model
        .holds
        .iter()
        .filter(|h| h.held == at)
        .map(|h| h.holder)
        .collect();
    let beyond: Vec<Anchor> = upstream(&pairs, at)
        .into_iter()
        .filter(|a| matches!(a, Anchor::Mark(_)) && !direct.contains(a))
        .collect();
    let beyond_fns = beyond.iter().filter(|a| contract(a)).count();
    let reach = match (beyond.len() - beyond_fns, beyond_fns) {
        (0, 0) => String::new(),
        (0, fns) => plural(fns, "more signature"),
        (types, 0) => plural(types, "more type"),
        (types, fns) => format!(
            "{} and {}",
            plural(types, "more type"),
            plural(fns, "more signature")
        ),
    };
    // What the empty "held by" says. A static is a root by nature; a type only
    // functions name enters the chart through them; anything else with no
    // holder and no contract is reached by nothing the chart draws at all.
    let root_line = if mark.is_static() {
        "no type holds it — a root."
    } else if !contracts.is_empty() {
        "no type holds it — it enters through the signatures below."
    } else {
        "nothing on this chart reaches it — no type holds it, and no signature names it."
    };

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
                    if let Some(letter) = mark.letter() {
                        span {
                            class: "shrink-0 font-bold text-flare",
                            title: match letter {
                                "A" => "added since the diff base",
                                "D" => "removed since the diff base — quoted from the base edition",
                                _ => "declaration changed since the diff base",
                            },
                            "{letter}"
                        }
                    }
                }
                p { class: "mt-0.5 font-data text-[9.5px] text-ink-soft", "{mark.locator()}" }
                if let Some(line) = change_line {
                    p { class: "mt-1 font-data text-[10px] leading-relaxed text-flare", "{line}" }
                }
                if !change_rows.is_empty() {
                    div { class: "mt-1 space-y-0.5 font-data text-[10px] leading-snug",
                        for (i , (mk , text , gone)) in change_rows.iter().enumerate() {
                            p { key: "{i}", class: "flex items-baseline gap-1",
                                span { class: "shrink-0 font-bold text-flare", "{mk}" }
                                span {
                                    class: if *gone { "text-ink-soft line-through" } else { "text-ink" },
                                    "{text}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                // A function holds nothing and nothing holds it: its one
                // reading is the contract it publishes.
                if mark.is_fn() {
                    h3 { class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Signature names ({holds.len()})"
                    }
                    if holds.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "its signature names no workspace types."
                        }
                    } else {
                        HoldList { rows: holds }
                    }
                } else {
                    h3 { class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Held by ({held_by.len()})"
                    }
                    if held_by.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft", "{root_line}" }
                    } else {
                        HoldList { rows: held_by }
                    }
                    if !contracts.is_empty() {
                        h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                            "In the contract of ({contracts.len()})"
                        }
                        HoldList { rows: contracts }
                    }
                    if !reach.is_empty() {
                        p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                            if mark.ghost {
                                "the removal reaches {reach} upstream."
                            } else {
                                "a shape change here reaches {reach} upstream."
                            }
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
            }
            div { class: "border-t border-ink-line px-4 py-2",
                if mark.ghost {
                    p { class: "font-data text-[9.5px] text-ink-soft",
                        "its definition left the working copy."
                    }
                } else {
                    Link {
                        class: "font-data text-[9.5px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                        to: item_route(&mark.path, &mark.label),
                        "open its definition →"
                    }
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
                        span { class: "dm-nm is-fn", "pub fn" }
                        span { class: "text-ink-soft",
                            " — a free function, drawn as the contract it is: its parameters quoted as rows and its return type under them. the types its signature names draw the same holding lines a field does, so a type only functions reach is no longer a root nobody holds. a method is not here — it belongs to the type its impl names — and no function body is on this chart."
                        }
                    }
                    p {
                        span { class: "font-bold text-flare", "A" }
                        span { class: "text-ink-soft", " added since the base · " }
                        span { class: "font-bold text-flare", "M" }
                        span { class: "text-ink-soft", " declaration changed · " }
                        span { class: "font-bold text-flare", "D" }
                        span { class: "text-ink-soft",
                            " removed — a dashed ghost quoting the base edition. a diff-touched block wears the flare on its own frame."
                        }
                    }
                    p {
                        span { class: "font-bold text-flare", "+" }
                        span { class: "text-ink-soft", " field or variant added · " }
                        span { class: "font-bold text-flare", "−" }
                        span { class: "text-ink-soft",
                            " removed — struck, quoted from the base, seated where it stood."
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-owns is-added", width: 1.4 }
                        span {
                            span { class: "text-ink", "added" }
                            span { class: "text-ink-soft",
                                " — a holding edge the base did not have, its word on the line."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-owns is-removed" }
                        span {
                            span { class: "text-ink", "removed" }
                            span { class: "text-ink-soft",
                                " — a holding edge only the base had, re-drawn from its edition."
                            }
                        }
                    }
                    p { class: "text-ink-soft",
                        "while the diff has anything to say, untouched types rest at a lighter pressure; hovering restores them. a clean diff draws none of this."
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
                        span { class: "text-ink-soft", " · " }
                        span { class: "font-medium", "named by 2 signatures" }
                        span { class: "text-ink-soft",
                            " — more than three marks reach this one, so its incoming edges rest folded; hover either end to ink them in. the two counts stay apart because a signature names a type without holding it."
                        }
                    }
                    p {
                        span { class: "font-medium", "+ 5 private types" }
                        span { class: "text-ink-soft",
                            " — a type or function below the visibility setting is never a mark, and every holds edge touching one lands on its module's counted row. at "
                        }
                        span { class: "text-ink", "pub" }
                        span { class: "text-ink-soft",
                            " the row counts internal types instead, and its words say so."
                        }
                    }
                    p { class: "text-ink-soft",
                        "the visibility toggle sets which doors earn a block: "
                        span { class: "text-ink", "pub" }
                        " draws only what leaves the crate, "
                        span { class: "text-ink", "pub(crate)" }
                        " adds the crate-visible types, "
                        span { class: "text-ink", "private" }
                        " draws every type there is. a static stands at every setting — state no type holds has nowhere else to be counted."
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
                        "a function body\u{2019}s references are not on this chart — only what its signature declares. the same goes for trait items: a tie is kept only where both ends land on a drawn type."
                    }
                    p {
                        "type parameters are holes: their fields quote as written, and the walk reads nothing through them. so is an "
                        span { class: "text-ink", "impl Trait" }
                        " in a signature — an anonymous parameter is still a parameter — and so is a trait bound."
                    }
                    p {
                        "the structural diff reads the base edition of each changed file syntactically: declarations match by kind and name, and a removed relation\u{2019}s target is matched by name — never type-resolved."
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
