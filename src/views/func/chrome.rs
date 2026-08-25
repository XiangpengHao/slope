//! The function chart's furniture: the cartouche, the search plate, and the
//! sheet one selected declaration opens.
//!
//! Three readings ride the cartouche, because each acts on the whole sheet:
//! how the marks are **grouped** inside their module, which direction the
//! **calls** are read in, and how narrow a declaration may be and still be
//! drawn. Nothing here paraphrases the survey — the limits fold quotes it, and
//! every count is a count of something a reviewer can go and read.

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::CodeGraph;
use crate::views::chrome::{Altitude, AltitudeSwitch, plural};
use crate::views::data::VisFloor;
use crate::views::func::model::{FnFacts, FnModel, Touch};
use crate::views::func::{
    CallDir, Group, Sel, band_route, mark_route, peek_at, peek_key, peek_route, use_fns,
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
            GroupSwitch {}
            CallToggle {}
            FnVisSlider { off_paper: facts.off_paper }
            SurveyLimits { notes }
        }
    }
}

/// What a frame is: the module alone, or a frame per type or per file inside
/// it. Two thirds of what runs is a method, and whose it is is a method's first
/// fact — so the paper can say it (2026-08-25, user).
#[component]
fn GroupSwitch() -> Element {
    let fns = use_fns();
    let current = *fns.group.read();
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "how the marks are grouped inside their module",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "group"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                for group in Group::ALL {
                    button {
                        key: "{group.label()}",
                        class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                        class: if current == group { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                        "aria-pressed": if current == group { "true" } else { "false" },
                        title: "{group.hint()}",
                        onclick: move |_| {
                            let mut at = fns.group;
                            at.set(group);
                        },
                        "{group.label()}"
                    }
                }
            }
        }
    }
}

/// Which direction the chart's calls are read in, against whatever the reader
/// has in hand: the selected mark, the block under the cursor, or — with the
/// paper at rest — the declarations the epoch touched.
#[component]
fn CallToggle() -> Element {
    let fns = use_fns();
    let current = *fns.calls.read();
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "which direction the chart's calls are read in",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "calls"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                for dir in [CallDir::Calls, CallDir::Callers, CallDir::Both] {
                    button {
                        key: "{dir.label()}",
                        class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                        class: if current == dir { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                        "aria-pressed": if current == dir { "true" } else { "false" },
                        title: "{dir.hint()}",
                        onclick: move |_| {
                            let mut at = fns.calls;
                            at.set(dir);
                        },
                        "{dir.label()}"
                    }
                }
            }
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
                if !mark.head.section.is_empty() {
                    p { class: "mt-0.5 truncate font-data text-[10px] text-ink-soft",
                        title: "{mark.head.section}",
                        "{mark.head.section}"
                    }
                }
                p { class: "mt-0.5 font-data text-[9.5px] text-ink-soft", "{mark.head.locator()}" }
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
    rsx! {
        div { class: "pointer-events-auto relative",
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
            }
            if open() && !hits.read().is_empty() {
                ul { class: "plate absolute right-0 top-[calc(100%+4px)] z-20 max-h-[60dvh] w-full overflow-y-auto py-1",
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
