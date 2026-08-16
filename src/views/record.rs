//! The part record.
//!
//! The board shows a neighbourhood; it does not *say* anything. Holding a crate
//! has to produce words — names you can read, counts you can trust, and a chain
//! you can paste into a pull request. `cargo tree -i` answers that in 400ms of
//! copyable text, and a picture that has to be traced by eye has not answered it
//! at all.
//!
//! Every record carries the same fields in the same order: what it is, why it
//! is here, what depends on it, what it depends on. Learn one record and you can
//! read all 346 without looking for where anything moved to.

use dioxus::prelude::*;

use crate::graph::{Board, focus};
use crate::views::BoardState;

#[derive(Clone, PartialEq)]
pub struct Row {
    pub id: usize,
    pub name: String,
    pub version: String,
    pub dependents: usize,
    pub duplicate: bool,
}

/// Which way a list runs. Direction is the board's strongest signal, so the
/// panel spends the same two colours on it that the copper does.
#[derive(Clone, Copy, PartialEq)]
pub enum Way {
    /// Gold: crates that pull this one in. Enters from the left.
    In,
    /// Blue: crates this one pulls in. Leaves to the right.
    Out,
}

#[derive(Clone, PartialEq)]
pub struct Record {
    pub id: usize,
    pub designator: String,
    pub name: String,
    pub version: String,
    pub duplicate: bool,
    /// Workspace member → … → this crate. Empty only if unreachable.
    pub path: Vec<Row>,
    pub direct_dependents: Vec<Row>,
    pub direct_dependencies: Vec<Row>,
    /// Every crate reachable in that direction, at any distance. The listed
    /// rows plus `total - listed` account for all of it and nothing twice.
    pub total_dependents: usize,
    pub total_dependencies: usize,
}

fn row(board: &Board, id: usize) -> Row {
    let pad = &board.pads[id];
    Row {
        id,
        name: pad.name.clone(),
        version: pad.version.clone(),
        dependents: pad.dependents.len(),
        duplicate: pad.duplicate,
    }
}

impl Record {
    pub fn build(board: &Board, view: &focus::Neighbourhood, id: usize) -> Self {
        let pad = &board.pads[id];
        let (dependents, dependencies) = focus::immediate(board, id);

        // "Direct" comes from the crate itself, never from hop levels: levels are
        // longest-path, so a crate that is both an immediate dependent and
        // reachable by a longer route lands in a further column. The reader
        // asked for immediate, and immediate is exact.
        let direct_dependents: Vec<Row> = dependents.iter().map(|&i| row(board, i)).collect();
        let direct_dependencies: Vec<Row> = dependencies.iter().map(|&i| row(board, i)).collect();

        Self {
            id,
            designator: pad.designator(),
            name: pad.name.clone(),
            version: pad.version.clone(),
            duplicate: pad.duplicate,
            path: focus::shortest_path_from_root(board, id)
                .into_iter()
                .map(|i| row(board, i))
                .collect(),
            total_dependents: view.total_consumers,
            total_dependencies: view.total_producers,
            direct_dependents,
            direct_dependencies,
        }
    }
}

#[component]
pub fn RecordPanel(record: Option<Record>) -> Element {
    let state: BoardState = use_context();
    let mut hold = state.held;

    let Some(record) = record else {
        return rsx! {
            aside {
                class: "plate flex max-h-[42%] w-full shrink-0 flex-col overflow-y-auto border-t lg:max-h-none lg:w-[23rem] lg:border-l lg:border-t-0",
                "aria-label": "Part record",
                KeyPlate {}
            }
        };
    };

    rsx! {
        aside {
            class: "plate flex max-h-[42%] w-full shrink-0 flex-col overflow-y-auto border-t lg:max-h-none lg:w-[23rem] lg:border-l lg:border-t-0",
            "aria-label": "Part record",

            // Screen readers get the answer too: the canvas can say nothing.
            div {
                class: "sr-only",
                "aria-live": "polite",
                "{record.name} {record.version}. {record.total_dependents} crates depend on it. It depends on {record.total_dependencies}."
            }

            header { class: "border-b border-edge px-4 py-3",
                div { class: "flex items-baseline justify-between gap-2",
                    h2 { class: "truncate text-[17px] font-semibold tracking-tight", "{record.name}" }
                    span { class: "designator shrink-0 font-mono", "{record.designator}" }
                }
                div { class: "mt-0.5 flex items-center gap-2",
                    span { class: "tabular font-mono text-[12px] text-legend-soft", "{record.version}" }
                    if record.duplicate {
                        span { class: "bg-flag-field px-1.5 py-0.5 text-[10px] font-semibold text-flag",
                            "ALSO AT ANOTHER VERSION"
                        }
                    }
                }
            }

            // The answer to "why is this here", as copyable text.
            if !record.path.is_empty() {
                section { class: "border-b border-edge px-4 py-3",
                    h3 { class: "designator", "Why it's here" }
                    p { class: "mt-1.5 font-mono text-[12px] leading-relaxed break-words select-text",
                        for (index , step) in record.path.iter().enumerate() {
                            if index > 0 {
                                span { class: "text-legend-soft", " → " }
                            }
                            button {
                                class: "hover:bg-mask-raised",
                                class: if index + 1 == record.path.len() { "font-semibold text-pad-lit" } else { "" },
                                onclick: {
                                    let id = step.id;
                                    move |_| hold.set(Some(id))
                                },
                                "{step.name}"
                            }
                        }
                    }
                }
            }

            CrateList {
                heading: "Depended on by".to_string(),
                empty: "Nothing depends on this crate.".to_string(),
                way: Way::In,
                total: record.total_dependents,
                rows: record.direct_dependents.clone(),
            }

            CrateList {
                heading: "Depends on".to_string(),
                empty: "This crate depends on nothing.".to_string(),
                way: Way::Out,
                total: record.total_dependencies,
                rows: record.direct_dependencies.clone(),
            }
        }
    }
}

#[component]
fn CrateList(heading: String, empty: String, way: Way, total: usize, rows: Vec<Row>) -> Element {
    let state: BoardState = use_context();
    let mut hold = state.held;

    // The same two colours the copper uses, so the panel and the board are
    // obviously reporting the same fact.
    let (ink, rule) = match way {
        Way::In => ("text-incoming", "bg-incoming"),
        Way::Out => ("text-outgoing", "bg-outgoing"),
    };

    rsx! {
        section { class: "border-b border-edge px-4 py-3",
            h3 { class: "flex items-center justify-between gap-2",
                span { class: "flex items-center gap-2",
                    span { class: "h-[3px] w-4 shrink-0 {rule}" }
                    span { class: "designator", "{heading}" }
                }
                span { class: "tabular text-[15px] font-semibold {ink}", "{total}" }
            }

            if rows.is_empty() && total == 0 {
                p { class: "mt-1.5 text-[12px] text-legend-soft", "{empty}" }
            } else {
                // Two buckets that add up to the total, every time. Listed here,
                // and everything else beyond them.
                p { class: "tabular mt-0.5 text-[11px] text-legend-soft",
                    "{rows.len()} directly"
                    if total > rows.len() {
                        " · {total - rows.len()} further out"
                    }
                }
                ul { class: "mt-2 flex flex-col",
                    for entry in rows {
                        li { key: "{entry.id}",
                            button {
                                class: "flex w-full items-baseline gap-2 px-1.5 py-1 text-left hover:bg-mask-raised",
                                onclick: {
                                    let id = entry.id;
                                    move |_| hold.set(Some(id))
                                },
                                span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                                if entry.duplicate {
                                    span {
                                        class: "shrink-0 bg-flag-field px-1 text-[10px] font-semibold text-flag",
                                        title: "Resolves at more than one version",
                                        "DUP"
                                    }
                                }
                                span { class: "tabular shrink-0 font-mono text-[11px] text-legend-soft",
                                    "{entry.version}"
                                }
                                span {
                                    class: "tabular w-7 shrink-0 text-right text-[11px] text-legend-soft",
                                    title: "{entry.dependents} crates depend on {entry.name}",
                                    "{entry.dependents}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What the board's marks mean and how to move through it, in the drawing's own
/// key. The previous design documented none of its vocabulary and expected the
/// reader to infer it from the picture.
#[component]
fn KeyPlate() -> Element {
    rsx! {
        div { class: "flex flex-col gap-5 px-4 py-4",
            section {
                h3 { class: "designator", "The board" }
                p { class: "mt-1.5 text-[13px] leading-relaxed text-legend-soft",
                    "Every crate is the same pad. What a crate "
                    span { class: "text-legend", "is" }
                    " lives in the copper running into and out of it. Columns count hops from your workspace: anything right of a pad is something it depends on, anything left of it depends on the pad."
                }
            }

            section {
                h3 { class: "designator", "Marks" }
                ul { class: "mt-2 flex flex-col gap-2 text-[12px]",
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-[3px] w-5 shrink-0 bg-incoming" }
                        span { "gold — what depends on the crate you hold" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-[3px] w-5 shrink-0 bg-outgoing" }
                        span { "blue — what it depends on" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-[3px] w-5 shrink-0 bg-copper" }
                        span { class: "text-legend-soft", "copper — every other dependency, always drawn" }
                    }
                    li { class: "flex items-center gap-2.5",
                        // The same marks the board draws, at the same shapes.
                        span { class: "flex w-5 shrink-0 justify-center",
                            svg {
                                class: "h-2.5 w-2.5",
                                view_box: "0 0 10 10",
                                fill: "var(--color-flag)",
                                polygon { points: "5,1 9,8 1,8" }
                            }
                        }
                        span { "resolves at more than one version" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "flex w-5 shrink-0 justify-center",
                            span { class: "h-3 w-3 border border-legend" }
                        }
                        span { "a crate this workspace builds" }
                    }
                }
            }

            section {
                h3 { class: "designator", "Moving" }
                dl { class: "mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-[12px]",
                    Key { keys: "pinch", note: "zoom about the pointer — the board never rearranges" }
                    Key { keys: "wheel", note: "zoom, a notch at a time" }
                    Key { keys: "scroll · drag", note: "pan across the board" }
                    Key { keys: "click", note: "hold a crate; the camera flies to it" }
                    Key { keys: "← ↑", note: "step to what depends on it" }
                    Key { keys: "→ ↓", note: "step to what it depends on" }
                    Key { keys: "⌫", note: "back the way you came" }
                    Key { keys: "/", note: "find a crate by name" }
                    Key { keys: "esc", note: "let go" }
                }
            }
        }
    }
}

#[component]
fn Key(keys: String, note: String) -> Element {
    rsx! {
        dt { class: "whitespace-nowrap border border-edge bg-mask px-1.5 py-0.5 text-center font-mono text-[11px] text-legend",
            "{keys}"
        }
        dd { class: "self-center text-legend-soft", "{note}" }
    }
}
