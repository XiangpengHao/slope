use dioxus::prelude::*;

use crate::api::BoardLoad;
use crate::components::{IconFit, IconMinus, IconPlus};
use crate::graph::focus;
use crate::views::canvas::{self, BoardCanvas};
use crate::views::record::{Record, RecordPanel};
use crate::views::{BoardResource, BoardState};

#[component]
pub fn Board() -> Element {
    let resource: BoardResource = use_context();
    let state: BoardState = use_context();
    let scene = canvas::use_scene();
    let mut hold = state.held;

    let loaded_shape = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(BoardLoad::Ready(board))) => {
                Some((board.package_count, board.traces.is_empty()))
            }
            _ => None,
        }
    };
    let failure = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(BoardLoad::Failed(message))) => Some(message.clone()),
            Some(Err(err)) => Some(err.to_string()),
            _ => None,
        }
    };

    // Seat the board once, when it arrives. This is the only time geometry is
    // ever written: from here on the world holds still and only the camera and
    // the lighting change.
    {
        let scene = scene.clone();
        use_effect(move || {
            let loaded = resource.read();
            if let Some(Ok(BoardLoad::Ready(board))) = loaded.as_ref() {
                let mut scene = scene.borrow_mut();
                if scene.pads.is_empty() {
                    canvas::seat(&mut scene, board);
                }
            }
        });
    }

    // Holding a crate lights its copper and flies the camera to it. Nothing
    // moves on the board; the reader does.
    let mut record = use_signal(|| None::<Record>);
    {
        let scene = scene.clone();
        use_effect(move || {
            let held = (state.held)();
            let loaded = resource.read();
            let Some(Ok(BoardLoad::Ready(board))) = loaded.as_ref() else {
                return;
            };
            let mut scene = scene.borrow_mut();
            if scene.pads.is_empty() {
                return;
            }
            let mut counts = state.counts;
            match held {
                Some(id) => {
                    let view = focus::build(board, id, focus::DEFAULT_DEPTH);
                    record.set(Some(Record::build(board, &view, id)));
                    counts.set(Some((view.total_consumers, view.total_producers)));
                    canvas::relight(&mut scene, Some(id), Some(&view));
                    scene.fly_to(id, reach_of(board, id), canvas::now());
                }
                None => {
                    record.set(None);
                    counts.set(None);
                    canvas::relight(&mut scene, None, None);
                }
            }
        });
    }

    // The finder lights what it matched.
    {
        let scene = scene.clone();
        use_effect(move || {
            let query = (state.query)();
            let mut scene = scene.borrow_mut();
            if scene.query != query {
                scene.query = query;
                scene.dirty = true;
            }
        });
    }

    let Some((package_count, no_traces)) = loaded_shape else {
        return rsx! {
            div { class: "flex h-full items-center justify-center px-6",
                if let Some(message) = failure {
                    Failure { message }
                } else {
                    Resolving {}
                }
            }
        };
    };

    let zoom_by = {
        let scene = scene.clone();
        move |notches: f32| {
            let mut scene = scene.borrow_mut();
            scene.flight = None;
            // Buttons zoom about the viewport centre, which is the point the
            // reader is looking at when they reach for a button rather than
            // pointing at something.
            canvas::zoom_about(&mut scene, notches, None);
        }
    };
    let zoom_in = {
        let zoom_by = zoom_by.clone();
        move |_| zoom_by(2.0)
    };
    let zoom_out = {
        let zoom_by = zoom_by.clone();
        move |_| zoom_by(-2.0)
    };
    let refit = {
        let scene = scene.clone();
        move |_| {
            let mut scene = scene.borrow_mut();
            scene.refit = true;
            scene.touched = false;
        }
    };

    rsx! {
        div { class: "flex h-full w-full flex-col overflow-hidden lg:flex-row",
            div { class: "relative min-h-0 min-w-0 flex-1",
                BoardCanvas {
                    scene: scene.clone(),
                    // Clicking bare mask lets go. Discarding `None` here made
                    // the most expected exit in the view a dead click.
                    on_hold: move |hit: Option<usize>| hold.set(hit),
                }

                CrateIndex {}
                PositionRegister {}

                div { class: "absolute right-3 top-8 flex flex-col overflow-hidden border border-edge bg-mask-deep",
                    button {
                        class: "flex h-11 w-11 items-center justify-center border-b border-edge hover:bg-mask-raised md:h-8 md:w-8",
                        "aria-label": "Zoom in",
                        onclick: zoom_in,
                        IconPlus { class: "h-4 w-4".to_string() }
                    }
                    button {
                        class: "flex h-11 w-11 items-center justify-center border-b border-edge hover:bg-mask-raised md:h-8 md:w-8",
                        "aria-label": "Zoom out",
                        onclick: zoom_out,
                        IconMinus { class: "h-4 w-4".to_string() }
                    }
                    button {
                        class: "flex h-11 w-11 items-center justify-center hover:bg-mask-raised md:h-8 md:w-8",
                        "aria-label": "Frame the whole board",
                        onclick: refit,
                        IconFit { class: "h-4 w-4".to_string() }
                    }
                }

                if no_traces {
                    p { class: "pointer-events-none absolute inset-x-0 top-1/2 text-center text-[13px] text-legend-soft",
                        if package_count <= 1 {
                            "This workspace declares no dependencies, so the board is bare."
                        } else {
                            "No dependencies to route."
                        }
                    }
                }
            }

            RecordPanel { record: record() }
        }
    }
}

/// How far a crate's own dependents and dependencies sit from its pad, so a
/// flight can land on a magnification where they are actually on screen.
///
/// The 80th percentile rather than the furthest one: a single crate eleven
/// columns away would otherwise pull every flight back to the whole board, and
/// framing 80% of the attachments is what the reader came to look at.
fn reach_of(board: &crate::graph::Board, id: usize) -> (f32, f32) {
    let here = &board.pads[id];
    let mut dx: Vec<f32> = Vec::new();
    let mut dy: Vec<f32> = Vec::new();
    for &n in here.dependents.iter().chain(here.deps.iter()) {
        let other = &board.pads[n];
        dx.push((other.x - here.x).abs());
        dy.push((other.y - here.y).abs());
    }
    if dx.is_empty() {
        return (0.0, 0.0);
    }
    let percentile = |mut v: Vec<f32>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[(v.len() as f32 * 0.8) as usize % v.len()]
    };
    (percentile(dx), percentile(dy))
}

/// Where you are, always, at every magnification — and the one element on the
/// board that never scales with it.
#[component]
fn PositionRegister() -> Element {
    let resource: BoardResource = use_context();
    let state: BoardState = use_context();
    let mut hold = state.held;

    let here = {
        let loaded = resource.read();
        match (loaded.as_ref(), (state.held)()) {
            (Some(Ok(BoardLoad::Ready(board))), Some(id)) => board
                .pads
                .get(id)
                .map(|pad| (pad.designator(), pad.name.clone(), pad.rank)),
            _ => None,
        }
    };

    let trail: Vec<(usize, String)> = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(BoardLoad::Ready(board))) => (state.history)()
                .iter()
                .rev()
                .skip(1)
                .take(3)
                .rev()
                .filter_map(|&id| board.pads.get(id).map(|p| (id, p.name.clone())))
                .collect(),
            _ => Vec::new(),
        }
    };

    rsx! {
        div {
            class: "pointer-events-none absolute bottom-3 left-3 flex max-w-[calc(100%-1.5rem)] items-stretch border border-edge bg-mask-deep",
            "aria-label": "Where you are",
            match here {
                Some((designator, name, rank)) => rsx! {
                    div { class: "flex items-center gap-2 border-r border-edge px-2.5 py-1.5",
                        span { class: "designator font-mono", "{designator}" }
                    }
                    div { class: "flex min-w-0 items-baseline gap-2 px-2.5 py-1.5",
                        if !trail.is_empty() {
                            nav {
                                class: "pointer-events-auto hidden shrink-0 items-baseline gap-1 lg:flex",
                                "aria-label": "Crates visited",
                                for (id , step) in trail {
                                    button {
                                        class: "max-w-[6rem] truncate text-[12px] text-legend-soft hover:text-legend",
                                        onclick: move |_| hold.set(Some(id)),
                                        "{step}"
                                    }
                                    span { class: "text-[11px] text-legend-soft", "›" }
                                }
                            }
                        }
                        span { class: "truncate text-[13px] font-semibold text-pad-lit", "{name}" }
                        span { class: "tabular shrink-0 text-[11px] text-legend-soft", "col {rank}" }
                    }
                },
                None => rsx! {
                    div { class: "px-2.5 py-1.5",
                        span { class: "text-[12px] text-legend-soft",
                            "Nothing held — click a pad, or press / to find one"
                        }
                    }
                },
            }
        }
    }
}

/// Canvas draws pixels, so assistive technology needs somewhere real to go.
/// Every crate on the board is here, busiest first; focusing one holds it
/// exactly as clicking its pad does.
///
/// Sighted keyboard users get a visible path instead: the finder, the record
/// panel's lists, and the arrow keys. Those are real, painted controls, which
/// an off-screen list can never be.
#[component]
fn CrateIndex() -> Element {
    let resource: BoardResource = use_context();
    let state: BoardState = use_context();
    let mut hold = state.held;

    let mut reachable: Vec<(usize, String, usize)> = {
        let loaded = resource.read();
        let Some(Ok(BoardLoad::Ready(board))) = loaded.as_ref() else {
            return rsx! {};
        };
        board
            .pads
            .iter()
            .map(|pad| {
                (
                    pad.id,
                    format!("{} {}", pad.name, pad.version),
                    pad.dependents.len(),
                )
            })
            .collect()
    };
    reachable.sort_by_key(|(_, _, dependents)| std::cmp::Reverse(*dependents));

    rsx! {
        ul { class: "sr-only", "aria-label": "Every crate on the board, most depended on first",
            for (id , label , dependents) in reachable {
                li { key: "{id}",
                    button { onclick: move |_| hold.set(Some(id)),
                        "{label}, {dependents} crates depend on it"
                    }
                }
            }
        }
    }
}

#[component]
fn Resolving() -> Element {
    rsx! {
        div { class: "flex flex-col items-center gap-4",
            // A continuity test walking the net while cargo resolves.
            svg {
                class: "h-6 w-44",
                view_box: "0 0 176 24",
                fill: "none",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                polyline {
                    points: "4,12 40,12 52,4 92,4 104,20 140,20 152,12 172,12",
                    stroke: "var(--color-edge)",
                    stroke_width: "2",
                }
                polyline {
                    class: "animate-[continuity_1.9s_linear_infinite]",
                    points: "4,12 40,12 52,4 92,4 104,20 140,20 152,12 172,12",
                    stroke: "var(--color-pad)",
                    stroke_width: "2",
                    stroke_dasharray: "24 208",
                }
            }
            p { class: "text-[13px] text-legend-soft",
                "Resolving the workspace — running cargo metadata."
            }
        }
    }
}

#[component]
fn Failure(message: String) -> Element {
    rsx! {
        div { class: "max-w-2xl",
            h1 { class: "text-[15px] font-semibold", "This path could not be resolved" }
            p { class: "mt-1 text-[13px] text-legend-soft",
                "rust-viewer reads a crate or workspace directory. Point it at one that contains a Cargo.toml, then reload."
            }
            pre { class: "plate mt-4 overflow-x-auto border p-3 font-mono text-[12px] whitespace-pre-wrap select-text",
                "{message}"
            }
        }
    }
}
