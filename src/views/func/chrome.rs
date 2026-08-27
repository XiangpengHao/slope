//! The function chart's furniture: the cartouche, the search plate, and the
//! sheet one selected declaration opens.
//!
//! Three readings ride the cartouche, because each acts on the whole sheet: in
//! what **order** the callees on a shelf are seated, which way round the
//! **wires** are read — calls, callers or both, against whatever is in focus —
//! and how narrow a declaration may be and still be drawn. Nothing here
//! paraphrases the survey — the limits fold quotes it, and every count is a
//! count of something a reviewer can go and read.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::CodeGraph;
use crate::views::chrome::{Altitude, AltitudeSwitch, plural};
use crate::views::data::VisFloor;
use crate::views::func::model::{FnFacts, FnModel, Touch};
use crate::views::func::{
    FnOrder, FnWires, Sel, band_route, mark_route, mod_route, peek_at, peek_key, peek_route,
    use_fns,
};

/// The chart's title block: the census of what the workspace runs, the ladder,
/// the diff, and the three readings.
#[component]
pub(super) fn FnCartouche(
    facts: FnFacts,
    workspace: String,
    diff_line: String,
    notes: Vec<String>,
) -> Element {
    let census = {
        let mut parts = vec![
            plural(facts.fns, "function"),
            plural(facts.methods, "method"),
        ];
        if facts.macros > 0 {
            parts.push(plural(facts.macros, "macro"));
        }
        parts.join(" · ")
    };
    let diff_words = {
        let mut parts: Vec<String> = Vec::new();
        if facts.added > 0 {
            parts.push(format!("{} added", facts.added));
        }
        if facts.changed > 0 {
            parts.push(format!("{} changed", facts.changed));
        }
        parts.join(" · ")
    };
    rsx! {
        section { class: "plate pointer-events-auto",
            div { class: "px-4 pt-3 pb-2",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{workspace}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft", "{census}" }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Fns }
                    // The entry points are this chart's first citizens and a
                    // band of their own, so the count is the way to them.
                    Link {
                        class: "block text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                        to: band_route(0),
                        title: "a declaration nothing in the workspace calls: main, a server function the client reaches through generated code, a method answering a foreign trait's contract — select the band",
                        "{plural(facts.entries, \"entry point\")}"
                    }
                    if facts.deepest > 0 {
                        p { class: "text-ink-soft",
                            "{plural(facts.deepest as usize, \"call\")} deep at the furthest"
                        }
                    }
                    if facts.ring > 0 {
                        Link {
                            class: "block text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                            to: band_route(facts.deepest + 1),
                            title: "in a ring of calls no entry point reaches — nothing on this paper starts them; select the band and read which",
                            "{facts.ring} in call rings"
                        }
                    }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if !diff_words.is_empty() {
                        p { class: "text-flare", "{diff_words}" }
                    } else if facts.off_paper > 0 {
                        p { class: "text-ink-soft", "no declaration changes in what this reading draws" }
                    } else {
                        p { class: "text-ink-soft", "no declaration changes since the base" }
                    }
                }
            }
            OrderSwitch {}
            WiresSwitch {}
            FnVisSlider { off_paper: facts.off_paper }
            FnKeys {}
            SurveyLimits { notes }
        }
    }
}

/// In what order the callees on a shelf are seated. The ground is the call tree
/// now, so what is left to read is the order the shelves read in.
#[component]
fn OrderSwitch() -> Element {
    let fns = use_fns();
    let current = *fns.order.read();
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "in what order the callees on a shelf are seated",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "order"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                for order in FnOrder::ALL {
                    button {
                        key: "{order.label()}",
                        class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                        class: if current == order { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                        "aria-pressed": if current == order { "true" } else { "false" },
                        title: "{order.hint()}",
                        onclick: move |_| {
                            let mut at = fns.order;
                            at.set(order);
                        },
                        "{order.label()}"
                    }
                }
            }
        }
    }
}

/// Which way round the paper reads what the shelving cannot say. A call that
/// seats its callee is drawn as containment and never as a line, so what this
/// reading takes a direction on is every other resolved call — read against
/// whatever is in focus: the selection, or the diff on the resting plate.
#[component]
fn WiresSwitch() -> Element {
    let fns = use_fns();
    let current = *fns.wires.read();
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "which way round the chart reads its calls",
            span {
                class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                title: "read against whatever is in focus — the selection, or the diff's own declarations · a call that seats its callee is drawn as containment, never as a line",
                "wires"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                for stop in FnWires::ALL_STOPS {
                    button {
                        key: "{stop.label()}",
                        class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                        class: if current == stop { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                        "aria-pressed": if current == stop { "true" } else { "false" },
                        title: "{stop.hint()}",
                        onclick: move |_| {
                            let mut at = fns.wires;
                            at.set(stop);
                        },
                        "{stop.label()}"
                    }
                }
            }
        }
    }
}

/// The keys, taught where they act: `↓` and `↑` walk the seating, and `←`/`→`
/// are the trail, as everywhere.
#[component]
fn FnKeys() -> Element {
    rsx! {
        div { class: "border-t border-ink-line px-4 py-1.5 font-data text-[9.5px] leading-relaxed text-ink-soft",
            span { class: "text-ink", "↓" }
            " into the first callee on the shelf · "
            span { class: "text-ink", "↑" }
            " to the caller it sits in · "
            span {
                class: "text-ink",
                title: "back and forward along the review trail — the browser's own keys, as at every altitude",
                "← →"
            }
            " step back and forward · "
            span {
                class: "text-ink",
                title: "fold the picked frame to its own box, or open it again — the same mark the head row carries",
                "z"
            }
            " folds it · "
            span {
                class: "text-ink",
                title: "quote the picked declaration's own source on the plate beside the sheet",
                "enter"
            }
            " reads its source · "
            span { class: "text-ink", "double-click" }
            " fits a frame · "
            span { class: "text-ink", "f" }
            " fits the sheet · "
            span {
                class: "text-ink",
                title: "a frame's own border selects everything the frame calls, down the chain",
                "click a frame's border"
            }
            " takes the whole box · "
            span {
                class: "text-ink",
                title: "the fold mark beside a head row: shift-click folds every frame inside it too",
                "shift-click a fold mark"
            }
            " folds all the way down"
        }
    }
}

/// How narrow a declaration may be and still be drawn, along the rungs rust
/// writes. The same scale the data altitude slides, reading the same keyword:
/// one ladder for the whole system.
#[component]
fn FnVisSlider(off_paper: usize) -> Element {
    let fns = use_fns();
    let current = *fns.vis_floor.read();
    let stop_at = move |stop: VisFloor| {
        let mut floor = fns.vis_floor;
        floor.set(stop);
    };
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "how narrow a declaration may be and still be drawn",
            span {
                class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                title: "the visibility each declaration writes — not what a chain of private modules leaves reachable from outside",
                "visibility"
            }
            input {
                class: "vis-slide mt-1.5 block w-full",
                r#type: "range",
                min: "0",
                max: "{VisFloor::STOPS.len() - 1}",
                step: "1",
                value: "{current.step()}",
                title: "{current.hint()}",
                "aria-valuetext": "{current.label()}",
                oninput: move |e| {
                    if let Some(stop) = VisFloor::at_step(&e.value()) {
                        stop_at(stop);
                    }
                },
            }
            div { class: "mt-0.5 flex items-baseline justify-between gap-1",
                for stop in VisFloor::STOPS {
                    button {
                        key: "{stop.label()}",
                        class: "whitespace-nowrap font-data text-[9px]",
                        class: if current == stop { "text-ink" } else { "text-ink-soft hover:text-ink" },
                        "aria-pressed": if current == stop { "true" } else { "false" },
                        title: "{stop.hint()}",
                        onclick: move |_| stop_at(stop),
                        "{stop.label()}"
                    }
                }
            }
            if off_paper > 0 {
                p {
                    class: "mt-1 font-data text-[9.5px] leading-snug text-ink-soft",
                    title: "{plural(off_paper, \"declaration\")} written narrower than this reading draws",
                    "{off_paper} off"
                }
            }
        }
    }
}

/// The survey's own limits, folded at the cartouche's foot — read once, to
/// trust the chart, and in the survey's own words.
#[component]
fn SurveyLimits(notes: Vec<String>) -> Element {
    if notes.is_empty() {
        return rsx! {};
    }
    rsx! {
        details { class: "fold border-t border-ink-line px-4 py-2",
            summary { class: "cursor-pointer select-none font-data text-[9.5px] tracking-[0.1em] uppercase text-ink-soft hover:text-ink",
                "what the survey cannot read"
            }
            div { class: "mt-1.5 space-y-1 pb-1 font-data text-[10px] leading-snug text-ink-soft",
                for (i , note) in notes.iter().enumerate() {
                    p { key: "{i}", "{note}" }
                }
            }
        }
    }
}

/// One row of a sheet.
#[derive(Clone, PartialEq)]
struct FnRow {
    /// Where the row goes when the chart draws a block for its end.
    to: Option<Route>,
    decl: String,
    name: String,
    letter: Option<&'static str>,
    /// What the relation says, in one word: `12 calls`, `answers`, `signature`.
    word: String,
    /// The row's hover words: where the end is written.
    hint: Option<String>,
    /// The end this row names, for the quotation plate: `file@label`.
    peek: Option<String>,
}

/// One chunked list of rows: the first eight, then a typographic "show all n".
#[component]
fn RowList(rows: Vec<FnRow>, sel: Sel, open: Option<String>) -> Element {
    let mut all = use_signal(|| false);
    let total = rows.len();
    let shown = if all() || total <= 8 { total } else { 8 };
    rsx! {
        ul { class: "mt-1",
            for (i , row) in rows.iter().take(shown).enumerate() {
                li { key: "{i}",
                    {
                        let quote = row.peek.as_deref().and_then(peek_at);
                        let to = row
                            .to
                            .clone()
                            .or_else(|| quote.map(|(path, label)| peek_route(&sel, path, label)));
                        let here = row.peek.is_some() && row.peek == open;
                        let title = match (&row.hint, row.to.is_none() && quote.is_some()) {
                            (Some(hint), true) => Some(format!("{hint} — quote its source")),
                            (hint, _) => hint.clone(),
                        };
                        let ink = match here {
                            true => "border-l-2 border-ink bg-ink/5 pr-1 pl-[2px]",
                            false => "px-1",
                        };
                        match to {
                            Some(to) => rsx! {
                                Link {
                                    class: "flex w-full items-baseline gap-1.5 py-0.5 font-data text-[10.5px] hover:bg-ink/5 {ink}",
                                    to,
                                    title,
                                    RowCells { row: row.clone(), dead: false }
                                }
                            },
                            None => rsx! {
                                span {
                                    class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] text-ink-soft",
                                    title: row.hint.clone(),
                                    RowCells { row: row.clone(), dead: true }
                                }
                            },
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

/// One row's own cells: what it is, its name, its diff letter, and what the
/// relation says.
#[component]
fn RowCells(row: FnRow, dead: bool) -> Element {
    rsx! {
        if !row.decl.is_empty() {
            span {
                class: "shrink-0",
                class: if !dead { "text-ink-soft" },
                "{row.decl}"
            }
        }
        span {
            class: if dead { "min-w-0 flex-1 truncate" } else { "flex-1 shrink-0 font-medium text-ink" },
            "{row.name}"
        }
        if let Some(letter) = row.letter {
            span { class: "shrink-0 font-bold text-flare", "{letter}" }
        }
        span {
            class: "max-w-[45%] shrink-0 truncate text-right text-[9px]",
            class: if !dead { "text-ink-soft" },
            title: "{row.word}",
            "{row.word}"
        }
    }
}

/// One heading and its rows, or the sentence that stands where rows would be.
/// The heading carries its own sentence in hover words, as every heading on the
/// rung above does: a reader who has to ask what `Called by` counts should get
/// the answer where they asked it.
#[component]
fn Section(
    title: String,
    hint: String,
    rows: Vec<FnRow>,
    empty: String,
    sel: Sel,
    open: Option<String>,
) -> Element {
    rsx! {
        section { class: "border-t border-ink-line px-3 pt-2 pb-2",
            h3 {
                class: "font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                title: "{hint}",
                if rows.is_empty() {
                    "{title}"
                } else {
                    "{title} ({rows.len()})"
                }
            }
            if rows.is_empty() {
                p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                    "{empty}"
                }
            } else {
                RowList { rows, sel, open }
            }
        }
    }
}

/// The impl block a method is written in, quoted as the header writes it, with
/// the owner's own name as the way to it. The header is the source's text and
/// nothing here rebuilds it: the owner's name is found *in* it and drawn as a
/// link, which is the same one-rung-down grammar the `Data touched` rows keep —
/// a type the data chart draws opens as its block there, and a trait, which that
/// chart draws no block for, opens as a quotation here.
#[component]
fn OwnerLine(
    section: String,
    owner: Option<crate::views::func::model::Owner>,
    sel: Sel,
    open: Option<String>,
) -> Element {
    let plain = rsx! {
        p { class: "mt-0.5 truncate font-data text-[10px] text-ink-soft", title: "{section}", "{section}" }
    };
    let Some(owner) = owner else {
        return plain;
    };
    let Some(cut) = section.rfind(&owner.name) else {
        return plain;
    };
    let (head, rest) = section.split_at(cut);
    let (name, tail) = rest.split_at(owner.name.len());
    let here = open.as_deref() == Some(peek_key(&owner.path, &owner.label).as_str());
    let to = match owner.on_data {
        true => crate::views::data::mark_route(&owner.path, &owner.label),
        false => peek_route(&sel, &owner.path, &owner.label),
    };
    let words = match owner.on_data {
        true => format!(
            "{} {} — down to its block on the data chart",
            owner.decl, owner.name
        ),
        false => format!("{} {} — quote its source", owner.decl, owner.name),
    };
    rsx! {
        p { class: "mt-0.5 truncate font-data text-[10px] text-ink-soft", title: "{section}",
            "{head}"
            Link {
                class: match here {
                    true => "text-ink underline-offset-4 hover:underline",
                    false => "underline-offset-4 hover:text-ink hover:underline",
                },
                to,
                title: "{words}",
                "{name}"
            }
            "{tail}"
        }
    }
}

/// One selected declaration's sheet: where it stands, what calls it, what it
/// calls, and every type it touches — each of those a link down to the rung
/// that draws types.
#[component]
pub(super) fn FnSheet(
    graph: CodeGraph,
    path: String,
    item: String,
    /// Which of its rows is open as a quotation, as the URL carries it.
    peek: Option<String>,
) -> Element {
    let fns = use_fns();
    let sel: Sel = (path.clone(), item.clone());
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        FnModel::build(&graph, &fns.reading())
    }));
    let model = model.read();

    let Some(mark) = model.find(&path, &item) else {
        // A selection this reading draws no block for: name the rung it is
        // written at, and offer the one move.
        let off = graph
            .items
            .iter()
            .find(|m| m.head.label == item && graph.path_of(m) == Some(path.as_str()))
            .filter(|m| !fns.vis_floor.read().admits(&m.head.vis));
        return rsx! {
            section { class: "plate pointer-events-auto w-full px-4 py-3 sm:w-72",
                if let Some(off) = off {
                    p { class: "font-data text-[11px] leading-relaxed text-ink",
                        "“{item}” is {off.head.kind.decl_words(&off.head.vis)}, and this reading draws {fns.vis_floor.read().label()} only."
                    }
                    button {
                        class: "mt-2 font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                        onclick: {
                            let vis = off.head.vis.clone();
                            move |_| {
                                let mut floor = fns.vis_floor;
                                floor.set(VisFloor::showing(&vis));
                            }
                        },
                        "draw it"
                    }
                } else {
                    p { class: "font-data text-[11px] text-ink",
                        "Nothing named “{item}” is drawn in {path} on this survey."
                    }
                }
                Link {
                    class: "mt-2 block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::FnOverview {},
                    "← whole chart"
                }
            }
        };
    };

    let by_id = model.by_id();
    // One row per far end. A mark the chart draws re-centres the chart on it;
    // one it draws no block for opens as a quotation of its own source.
    let far_row = |id: u32, word: String| -> Option<FnRow> {
        let item = graph.item(id)?;
        let file = graph.file(item.file)?;
        let drawn = by_id.get(&id);
        Some(FnRow {
            to: drawn.map(|m| mark_route(&m.head.path, &m.head.label)),
            decl: item.head.kind.decl_words(&item.head.vis),
            name: item.head.name.clone(),
            letter: drawn.and_then(|m| m.letter()),
            word,
            hint: Some(format!("{}:{}", file.path, item.head.line)),
            peek: (drawn.is_none()).then(|| peek_key(&file.path, &item.head.label)),
        })
    };
    let call_word = |count: u32, answers: bool| match (answers, count) {
        (true, _) => "answers".to_string(),
        (false, 1) => "1 call".to_string(),
        (false, n) => format!("{n} calls"),
    };

    // Whose code runs this, heaviest first — and the contract it answers,
    // which is the one caller a call graph cannot see.
    let mut called_by: Vec<(u32, FnRow)> = model
        .calls
        .iter()
        .filter(|c| c.def == mark.id)
        .filter_map(|c| {
            let answers = c.kind == crate::views::func::model::CallKind::Answers;
            far_row(c.user, call_word(c.count, answers)).map(|row| (c.count, row))
        })
        .collect();
    called_by.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    let called_by: Vec<FnRow> = called_by.into_iter().map(|(_, row)| row).collect();

    let mut calls: Vec<(u32, FnRow)> = model
        .calls
        .iter()
        .filter(|c| c.user == mark.id)
        .filter_map(|c| {
            let answers = c.kind == crate::views::func::model::CallKind::Answers;
            let word = match answers {
                true => "the contract it answers".to_string(),
                false => call_word(c.count, false),
            };
            far_row(c.def, word).map(|row| (c.count, row))
        })
        .collect();
    calls.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    let calls: Vec<FnRow> = calls.into_iter().map(|(_, row)| row).collect();

    // What it touches, one rung down. A type the data chart draws is a link
    // into that chart; one it draws no block for is quoted here.
    let touched: Vec<FnRow> = model
        .touches
        .get(&mark.id)
        .into_iter()
        .flatten()
        .map(|touch: &Touch| FnRow {
            to: touch
                .on_data
                .then(|| crate::views::data::mark_route(&touch.path, &touch.label)),
            decl: touch.decl.clone(),
            name: touch.name.clone(),
            letter: None,
            word: touch.word.clone(),
            hint: Some(format!("{}:{}", touch.path, touch.label)),
            peek: (!touch.on_data).then(|| peek_key(&touch.path, &touch.label)),
        })
        .collect();

    // Every caller a rewrite here could reach, minus the ones already listed
    // as direct callers: the fold under `Called by` is what the sheet used to
    // spend on a sentence nobody could follow.
    let direct: std::collections::HashSet<u32> = model
        .calls
        .iter()
        .filter(|c| c.def == mark.id)
        .map(|c| c.user)
        .collect();
    let mut upstream: Vec<FnRow> = model
        .upstream(mark.id)
        .into_iter()
        .filter(|id| !direct.contains(id))
        .filter_map(|id| far_row(id, "upstream".to_string()))
        .collect();
    upstream.sort_by(|a, b| a.name.cmp(&b.name));
    let stands = mark.stands();
    // The declaration's own source is one row of this sheet like any other: the
    // same `peek=` grammar, so back closes it and Escape steps out of the
    // quotation before it steps out of the selection.
    let own_key = peek_key(&mark.head.path, &mark.head.label);
    let reading_here = peek.as_deref() == Some(own_key.as_str());
    let read_it = match reading_here {
        true => mark_route(&mark.head.path, &mark.head.label),
        false => peek_route(&sel, &mark.head.path, &mark.head.label),
    };
    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[60dvh] w-full flex-col overflow-y-auto sm:max-h-[calc(100dvh-4.25rem)] sm:w-72",
            div { class: "px-3 pt-3 pb-2",
                h2 { class: "flex items-baseline gap-1.5 font-data text-[15px] font-semibold",
                    span { class: "shrink-0 text-[10.5px] font-normal text-ink-soft",
                        "{mark.head.decl()}"
                    }
                    span { class: "min-w-0 truncate text-ink", "{mark.head.name}" }
                    if let Some(letter) = mark.letter() {
                        span { class: "shrink-0 text-[11px] font-bold text-flare", "{letter}" }
                    }
                }
                // Whose method it is, quoted as the impl header writes it, with
                // the owner's own name the way to it: down to the block the
                // data chart draws for it, or its source quoted here where that
                // chart draws none.
                if !mark.head.section.is_empty() {
                    OwnerLine {
                        section: mark.head.section.clone(),
                        owner: mark.owner.clone(),
                        sel: sel.clone(),
                        open: peek.clone(),
                    }
                }
                // Where it is written, and the way to everything else written
                // there: the call tree seats a module's declarations all over
                // the sheet, so the module is a reading rather than a place.
                Link {
                    class: "mt-0.5 block truncate font-data text-[10px] text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: mod_route(mark.mod_key()),
                    title: "light every declaration written in {mark.written()}",
                    "{mark.written()}"
                }
                // Where it is written, and the one step to the whole of it: the
                // declaration's own source, on the same quotation plate every
                // other row of this sheet opens.
                div { class: "mt-0.5 flex items-baseline justify-between gap-2",
                    p { class: "min-w-0 truncate font-data text-[9.5px] text-ink-soft",
                        "{mark.head.locator()}"
                    }
                    Link {
                        class: match reading_here {
                            true => "shrink-0 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                            false => "shrink-0 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                        },
                        to: read_it.clone(),
                        title: "quote the whole declaration beside this sheet — enter does it too",
                        if reading_here { "reading it" } else { "read it" }
                    }
                }
                // The signature, quoted as rust writes it. The paper's blocks
                // quote it too; this is where there is room for the whole of
                // it, and clicking it opens the body under it.
                if !mark.rows.is_empty() {
                    Link {
                        class: "mt-1.5 block border-t border-ink-line pt-1.5 font-data text-[10px] leading-snug hover:bg-ink/5",
                        to: read_it,
                        title: "the signature as rust writes it — click to read the whole declaration",
                        p {
                            span { class: "text-tok-kw", "{mark.head.decl()} " }
                            span { class: "font-medium text-tok-fn", "{mark.head.name}" }
                            span { class: "text-tok-punct", "(" }
                        }
                        for (i , row) in mark.rows.iter().filter(|r| !r.returns).enumerate() {
                            p { key: "{i}", class: "pl-3",
                                class: if row.added { "text-flare" },
                                if row.ty.is_empty() {
                                    span { class: "text-tok-kw", "{row.name}" }
                                } else {
                                    span { class: "text-ink", "{row.name}: " }
                                    span { class: "text-tok-type", "{row.ty}" }
                                }
                            }
                        }
                        p {
                            span { class: "text-tok-punct", ")" }
                            if let Some(ret) = mark.rows.iter().find(|r| r.returns) {
                                span { class: "text-tok-punct", " -> " }
                                span { class: "text-tok-type", "{ret.ty}" }
                            }
                        }
                    }
                }
                p { class: "mt-1.5 border-t border-ink-line pt-1.5 font-data text-[10.5px] leading-relaxed text-ink",
                    "{stands}"
                }
            }
            Section {
                title: "Called by".to_string(),
                hint: "whose code runs this — the contract it answers included, which is the one caller a call graph cannot see"
                    .to_string(),
                rows: called_by,
                empty: "nothing in the workspace calls it.".to_string(),
                sel: sel.clone(),
                open: peek.clone(),
            }
            // The blast radius, in names rather than a number. A count of
            // resolvable ends with no way to reach them is the thing this
            // system calls a defect; the fold says how many and gives them
            // back.
            if !upstream.is_empty() {
                details { class: "fold border-t border-ink-line px-3 py-2",
                    summary { class: "cursor-pointer select-none font-data text-[9.5px] tracking-[0.1em] uppercase text-ink-soft hover:text-ink",
                        // Not `plural`: the noun here is the count itself —
                        // "10 mores upstream" is what asking a plural helper
                        // for a word that is already plural gets you.
                        "a rewrite reaches {upstream.len()} more upstream"
                    }
                    RowList { rows: upstream, sel: sel.clone(), open: peek.clone() }
                }
            }
            Section {
                title: "Calls".to_string(),
                hint: "what this declaration's own body runs, and the contract it answers"
                    .to_string(),
                rows: calls,
                empty: "it calls nothing the survey resolved.".to_string(),
                sel: sel.clone(),
                open: peek.clone(),
            }
            if !touched.is_empty() {
                Section {
                    title: "Data touched".to_string(),
                    hint: "every workspace type its signature names or its body uses — each row a step down to the rung that draws types"
                        .to_string(),
                    rows: touched,
                    empty: String::new(),
                    sel,
                    open: peek,
                }
            }
        }
    }
}

/// One selected boundary's sheet: which frame it is, how much runs under it,
/// and what crosses the line in each direction — by name, never by a count
/// alone. Containment is the call, so the box is a subtree, and the two
/// questions a reader brings to a subtree are what starts it from outside and
/// what it reaches out to.
#[component]
pub(super) fn FnTreeSheet(graph: CodeGraph, path: String, item: String) -> Element {
    let fns = use_fns();
    let sel: Sel = (path.clone(), item.clone());
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        FnModel::build(&graph, &fns.reading())
    }));
    let model = model.read();
    let Some(mark) = model.find(&path, &item) else {
        return rsx! {
            section { class: "plate pointer-events-auto w-full px-4 py-3 sm:w-72",
                p { class: "font-data text-[11px] text-ink",
                    "Nothing named “{item}” is drawn in {path} on this survey."
                }
                Link {
                    class: "mt-2 block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::FnOverview {},
                    "← whole chart"
                }
            }
        };
    };
    let by_id = model.by_id();
    let inside = model.subtree(mark.id);
    // Every call with exactly one end inside the boundary, gathered on the far
    // end: what runs the box from outside, and what the box reaches out to. One
    // row per end however many ways that end crosses — the same rule the data
    // sheet keeps, because a name engraved twice reads as two neighbours.
    let mut into: HashMap<u32, (u32, bool)> = HashMap::new();
    let mut out: HashMap<u32, (u32, bool)> = HashMap::new();
    for call in &model.calls {
        let answers = call.kind == crate::views::func::model::CallKind::Answers;
        let (far, side) = match (inside.contains(&call.def), inside.contains(&call.user)) {
            (true, false) => (call.user, &mut into),
            (false, true) => (call.def, &mut out),
            _ => continue,
        };
        let at = side.entry(far).or_insert((0, false));
        at.0 += call.count;
        at.1 |= answers;
    }
    let rows_of = |ends: HashMap<u32, (u32, bool)>| -> Vec<FnRow> {
        let mut rows: Vec<(u32, FnRow)> = ends
            .into_iter()
            .filter_map(|(id, (count, answers))| {
                let far = by_id.get(&id)?;
                Some((
                    count,
                    FnRow {
                        to: Some(mark_route(&far.head.path, &far.head.label)),
                        decl: far.head.decl(),
                        name: far.head.label.clone(),
                        letter: far.letter(),
                        word: match (answers, count) {
                            (true, 0) => "answers".to_string(),
                            (true, n) => format!("answers · {n}"),
                            (false, 1) => "1 call".to_string(),
                            (false, n) => format!("{n} calls"),
                        },
                        hint: Some(far.head.locator()),
                        peek: None,
                    },
                ))
            })
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
        rows.into_iter().map(|(_, row)| row).collect()
    };
    let (into, out) = (rows_of(into), rows_of(out));
    let held = mark.runs as usize;
    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[60dvh] w-full flex-col overflow-y-auto sm:max-h-[calc(100dvh-4.25rem)] sm:w-72",
            div { class: "px-3 pt-3 pb-2",
                h2 { class: "flex items-baseline gap-1.5 font-data text-[13px] font-semibold",
                    span { class: "shrink-0 text-[10px] font-normal text-ink-soft",
                        "{mark.head.decl()}"
                    }
                    span { class: "min-w-0 truncate text-ink", "{mark.head.label}" }
                }
                p { class: "mt-1.5 border-t border-ink-line pt-1.5 font-data text-[10.5px] leading-relaxed text-ink",
                    "{plural(held, \"declaration\")} run under it by the way in."
                }
                Link {
                    class: "mt-1 block font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                    to: mark_route(&mark.head.path, &mark.head.label),
                    title: "read the declaration this boundary belongs to",
                    "the declaration itself"
                }
            }
            Section {
                title: "Called from outside".to_string(),
                hint: "whose code, from outside this boundary, runs something inside it — the way in included"
                    .to_string(),
                rows: into,
                empty: "nothing outside the boundary calls into it.".to_string(),
                sel: sel.clone(),
                open: None,
            }
            Section {
                title: "Calls out".to_string(),
                hint: "what the code inside this boundary runs beyond it".to_string(),
                rows: out,
                empty: "nothing inside it calls out.".to_string(),
                sel,
                open: None,
            }
        }
    }
}

/// One selected band's sheet: what depth it is, how many declarations sit at
/// it, and every one of them by name.
///
/// A band selection used to draw nothing at all — the paper dimmed and the
/// reading had no words. A stratum is the whole organizing move of this chart,
/// so it says what it is and lists what it holds; a count of sixty resolvable
/// declarations with no way to reach them is exactly the defect this system
/// rejects everywhere else.
#[component]
pub(super) fn FnBandSheet(graph: CodeGraph, band: u32) -> Element {
    let fns = use_fns();
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        FnModel::build(&graph, &fns.reading())
    }));
    let model = model.read();
    let deepest = model.facts.deepest;
    let Some((_, caption)) = model.bands.iter().find(|(at, _)| *at == band) else {
        return rsx! {
            section { class: "plate pointer-events-auto w-full px-4 py-3 sm:w-72",
                p { class: "font-data text-[11px] leading-relaxed text-ink",
                    "No declaration this reading draws sits that many calls in."
                }
                Link {
                    class: "mt-2 block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::FnOverview {},
                    "← whole chart"
                }
            }
        };
    };
    let mut rows: Vec<FnRow> = model
        .marks
        .iter()
        .filter(|m| m.tier.band(deepest) == band)
        .map(|m| FnRow {
            to: Some(mark_route(&m.head.path, &m.head.label)),
            decl: m.head.decl(),
            name: m.head.name.clone(),
            letter: m.letter(),
            // The file and line, not the path: the row's column truncates
            // from the right, and six rows reading `src/views/dep/chrome…`
            // name nothing a reader can go to. The whole path is in the
            // hover words, where it has the room.
            word: match m.head.section.is_empty() {
                true => m.head.file_line(),
                false => m.head.section.clone(),
            },
            hint: Some(m.head.locator()),
            peek: None,
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    let held = rows.len();
    let sel: Sel = (String::new(), String::new());
    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[60dvh] w-full flex-col overflow-y-auto sm:max-h-[calc(100dvh-4.25rem)] sm:w-72",
            div { class: "px-3 pt-3 pb-2",
                h2 { class: "font-chart text-[13px] tracking-[0.22em] uppercase text-ink",
                    "{caption}"
                }
                p { class: "mt-1.5 border-t border-ink-line pt-1.5 font-data text-[10.5px] leading-relaxed text-ink",
                    if band == 0 {
                        "{plural(held, \"declaration\")} nothing in the workspace calls."
                    } else if band > deepest {
                        "{plural(held, \"declaration\")} in call rings no entry point reaches."
                    } else {
                        "{plural(held, \"declaration\")} at this depth, each by its own shortest way in."
                    }
                }
            }
            section { class: "border-t border-ink-line px-3 pt-2 pb-2",
                RowList { rows, sel, open: None }
            }
        }
    }
}

/// Find a declaration by name. The rows are this altitude's places: a
/// function, a method, a macro — each with the file and line it is written on.
#[component]
pub(super) fn FnSearch(graph: CodeGraph) -> Element {
    let fns = use_fns();
    let nav = use_navigator();
    let mut query = use_signal(String::new);
    let mut open = use_signal(|| false);
    let hits = use_memo(use_reactive((&graph, &query()), |(graph, q)| {
        let needle = q.trim().to_lowercase();
        if needle.len() < 2 {
            return Vec::new();
        }
        let mut hits: Vec<(bool, u32, String, String, String, String)> = graph
            .items
            .iter()
            .filter(|m| m.head.kind.is_callable())
            .filter(|m| m.head.label.to_lowercase().contains(&needle))
            .filter_map(|m| {
                let file = graph.file(m.file)?;
                Some((
                    !m.head.name.to_lowercase().starts_with(&needle),
                    u32::MAX - m.reach.fan_in,
                    m.head.kind.decl_words(&m.head.vis),
                    m.head.label.clone(),
                    file.path.clone(),
                    format!("{}:{}", file.path, m.head.line),
                ))
            })
            .collect();
        hits.sort_by(|a, b| (a.0, a.1, &a.3).cmp(&(b.0, b.1, &b.3)));
        hits.truncate(12);
        hits
    }));
    // A search must never land on a sheet that declines to show what was
    // asked for: naming a declaration widens the reading to the stop that
    // draws it.
    let mut go = move |vis: crate::graph::data::Vis, path: String, label: String| {
        let mut floor = fns.vis_floor;
        if !floor.peek().admits(&vis) {
            floor.set(VisFloor::showing(&vis));
        }
        open.set(false);
        query.set(String::new());
        nav.push(mark_route(&path, &label));
    };
    let vis_of = {
        let graph = graph.clone();
        move |path: &str, label: &str| {
            graph
                .items
                .iter()
                .find(|m| m.head.label == label && graph.path_of(m) == Some(path))
                .map(|m| m.head.vis.clone())
                .unwrap_or(crate::graph::data::Vis::Private)
        }
    };
    // The popover stands whenever the reader has typed enough for the search to
    // have looked: with hits it lists them, and with none it says so. A search
    // that answers an empty sheet with an empty sheet has not answered.
    let asked = query().trim().chars().count() >= 2;
    // Escape inside the field is the field's own key — the chart's handler
    // stands down over an input — and it steps all the way out: the query goes,
    // the popover closes, and the focus leaves, so the next Escape is the
    // chart's own step back out of the selection.
    let mut clear = move || {
        query.set(String::new());
        open.set(false);
        document::eval(
            "const s = [...document.querySelectorAll('#fn-search')]
                 .find((el) => el.offsetParent !== null);
             if (s) s.blur();",
        );
    };
    rsx! {
        div {
            class: "pointer-events-auto relative",
            // The popover closes when the focus leaves the plate. A press on a
            // result would otherwise move the focus before the click landed and
            // close the list out from under it, so the list declines the focus
            // shift instead of racing it: `pointerdown` is prevented, the field
            // keeps the focus, no `focusout` fires, and the click arrives.
            onfocusout: move |_| open.set(false),
            input {
                id: "fn-search",
                class: "plate w-full px-3 py-1.5 font-data text-[11px] text-ink placeholder:text-ink-soft",
                r#type: "search",
                autocomplete: "off",
                spellcheck: "false",
                placeholder: "find a declaration…   /",
                value: "{query}",
                oninput: move |e| {
                    query.set(e.value());
                    open.set(true);
                },
                onfocusin: move |_| open.set(true),
                onkeydown: move |e: Event<KeyboardData>| {
                    if e.key() == Key::Escape {
                        e.stop_propagation();
                        clear();
                    }
                },
            }
            if open() && asked {
                ul {
                    class: "plate absolute right-0 top-[calc(100%+4px)] z-20 max-h-[60dvh] w-full overflow-y-auto py-1",
                    onpointerdown: move |e: Event<PointerData>| e.prevent_default(),
                    if hits.read().is_empty() {
                        li { class: "px-3 py-1 font-data text-[10.5px] text-ink-soft",
                            "nothing runs by that name"
                        }
                    }
                    for (i , (_ , _ , decl , label , path , locator)) in hits.read().iter().enumerate() {
                        li { key: "{i}",
                            button {
                                class: "flex w-full items-baseline gap-1.5 px-3 py-1 text-left font-data text-[10.5px] hover:bg-ink/5",
                                onclick: {
                                    let (path, label) = (path.clone(), label.clone());
                                    let vis = vis_of(&path, &label);
                                    move |_| go(vis.clone(), path.clone(), label.clone())
                                },
                                span { class: "shrink-0 text-ink-soft", "{decl}" }
                                span { class: "min-w-0 flex-1 truncate font-medium text-ink", "{label}" }
                                span { class: "shrink-0 text-[9px] text-ink-soft", "{locator}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
