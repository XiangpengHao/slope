//! The title block.
//!
//! A fabrication drawing carries one plate that says what the drawing is of,
//! what revision it is, and what it counts. This is that plate: the workspace it
//! read, the lens it is in, how much is on the board, and the fab note for
//! crate names that resolve at more than one version.

use dioxus::prelude::*;

use crate::api::BoardLoad;
use crate::components::{IconClear, IconFind};
use crate::views::{BoardResource, BoardState};

/// Everything the plate shows, pulled out by value so no borrow of the resource
/// survives into the markup.
#[derive(Default, Clone, PartialEq)]
struct Readout {
    workspace: String,
    dir: String,
    packages: usize,
    distinct: usize,
    traces: usize,
    duplicates: usize,
    ready: bool,
    held: Option<Held>,
}

#[derive(Clone, PartialEq)]
struct Held {
    name: String,
    version: String,
    /// Everything reachable in each direction — the same two numbers the record
    /// panel headlines. Quoting direct counts here while the panel headlined
    /// totals made one crate look like it had two different answers.
    dependents: usize,
    dependencies: usize,
}

#[component]
pub fn TitleBlock() -> Element {
    let resource: BoardResource = use_context();
    let state: BoardState = use_context();
    let mut hold = state.held;
    let held = hold();

    let info = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(BoardLoad::Ready(board))) => Readout {
                workspace: board.workspace.clone(),
                dir: board.manifest_dir.clone(),
                packages: board.package_count,
                distinct: board.distinct_count,
                traces: board.traces.len(),
                duplicates: board.duplicates.len(),
                ready: true,
                held: held
                    .and_then(|id| board.pads.get(id))
                    .zip((state.counts)())
                    .map(|(pad, (dependents, dependencies))| Held {
                        name: pad.name.clone(),
                        version: pad.version.clone(),
                        dependents,
                        dependencies,
                    }),
            },
            _ => Readout::default(),
        }
    };

    rsx! {
        header { class: "flex h-14 shrink-0 items-stretch gap-4 border-b border-edge bg-mask-deep px-4",

            // What this is a drawing of.
            div { class: "flex min-w-0 shrink items-baseline gap-2 self-center",
                span { class: "truncate text-[15px] font-semibold tracking-tight",
                    if info.workspace.is_empty() { "rust-viewer" } else { "{info.workspace}" }
                }
                span { class: "hidden truncate font-mono text-[11px] text-legend-soft xl:block",
                    "{info.dir}"
                }
            }

            // The lens. Unbuilt lenses are not advertised here: a permanently
            // disabled tab spends credibility on first run for nothing.
            nav { class: "flex shrink-0 items-stretch self-stretch",
                span { class: "silkscreen flex items-center whitespace-nowrap border-b-2 border-pad px-2 text-[12px] font-semibold",
                    "Dependencies"
                }
            }

            div { class: "flex-1" }

            // What is on the board.
            if let Some(here) = info.held.clone() {
                div { class: "flex min-w-0 shrink items-center gap-2 self-center",
                    span { class: "truncate text-[13px] font-semibold", "{here.name}" }
                    span { class: "tabular shrink-0 font-mono text-[11px] text-legend-soft",
                        "{here.version}"
                    }
                    span { class: "tabular hidden shrink-0 items-center gap-2 text-[12px] md:flex",
                        span { class: "text-incoming", "{here.dependents} in" }
                        span { class: "text-legend-soft", "·" }
                        span { class: "text-outgoing", "{here.dependencies} out" }
                    }
                    button {
                        class: "flex shrink-0 items-center gap-1 px-1.5 py-1 text-[12px] text-legend-soft hover:bg-mask-raised hover:text-legend",
                        title: "Let go (Esc)",
                        onclick: move |_| hold.set(None),
                        IconClear { class: "h-3 w-3".to_string() }
                        span { class: "hidden lg:inline", "let go" }
                    }
                }
            } else if info.ready {
                div { class: "tabular hidden shrink items-center gap-3 self-center text-[12px] text-legend-soft sm:flex",
                    span { class: "whitespace-nowrap", "{info.distinct} crates" }
                    span { class: "hidden whitespace-nowrap md:inline", "{info.traces} dependencies" }
                }
            }

            // Fab note: crate names resolving at more than one version.
            if info.ready && info.duplicates > 0 {
                span {
                    class: "flex shrink-0 items-center gap-1.5 self-center bg-flag-field px-2 py-1 text-[12px] font-semibold text-flag",
                    title: "{info.duplicates} crate names resolve at more than one version in this workspace",
                    span { class: "tabular whitespace-nowrap", "{info.duplicates}" }
                    span { class: "silkscreen hidden whitespace-nowrap lg:inline", "flagged" }
                }
            }

            Finder {}
        }
    }
}

/// Crate finder. Lists what it matched rather than silently guessing one.
#[component]
fn Finder() -> Element {
    let resource: BoardResource = use_context();
    let state: BoardState = use_context();
    let mut query = state.query;
    let mut hold = state.held;
    let mut open = use_signal(|| false);

    let needle = query().trim().to_lowercase();
    let matches: Vec<(usize, String, String, usize)> = if needle.is_empty() {
        Vec::new()
    } else {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(BoardLoad::Ready(board))) => {
                let mut hits: Vec<&crate::graph::Pad> = board
                    .pads
                    .iter()
                    .filter(|p| p.name.to_lowercase().contains(&needle))
                    .collect();
                // Exact first, then by how much the board routes through it.
                hits.sort_by_key(|p| {
                    (
                        p.name.to_lowercase() != needle,
                        !p.name.to_lowercase().starts_with(&needle),
                        std::cmp::Reverse(p.dependents.len()),
                    )
                });
                hits.into_iter()
                    .take(8)
                    .map(|p| {
                        (
                            p.id,
                            p.name.clone(),
                            p.version.clone(),
                            p.dependents.len(),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    };
    let empty = !needle.is_empty() && matches.is_empty();
    let first = matches.first().map(|(id, _, _, _)| *id);

    rsx! {
        div { class: "relative flex w-32 min-w-[7.5rem] shrink items-center self-center lg:w-56",
            IconFind { class: "pointer-events-none absolute left-2 h-3.5 w-3.5 text-legend-soft".to_string() }
            input {
                r#type: "search",
                class: "w-full border border-edge bg-mask py-1.5 pl-7 pr-2 text-[13px] placeholder:text-legend-soft focus-visible:border-pad focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-pad-lit",
                placeholder: "find a crate  /",
                "aria-label": "Find a crate",
                value: "{query}",
                oninput: move |event| {
                    query.set(event.value());
                    open.set(true);
                },
                onfocus: move |_| open.set(true),
                onkeydown: move |event| {
                    if event.key() == Key::Enter && let Some(id) = first {
                        hold.set(Some(id));
                        open.set(false);
                    }
                },
            }

            if open() && (!matches.is_empty() || empty) {
                div { class: "plate absolute right-0 top-full z-10 mt-1 w-72 overflow-hidden border shadow-lg",
                    if empty {
                        p { class: "px-3 py-2 text-[12px] text-legend-soft",
                            "No crate on this board matches \"{needle}\"."
                        }
                    }
                    ul {
                        for (id , name , version , dependents) in matches {
                            li { key: "{id}",
                                button {
                                    class: "flex w-full items-baseline gap-2 px-3 py-1.5 text-left hover:bg-mask-raised",
                                    onclick: move |_| {
                                        hold.set(Some(id));
                                        open.set(false);
                                    },
                                    span { class: "min-w-0 flex-1 truncate text-[13px]", "{name}" }
                                    span { class: "tabular shrink-0 font-mono text-[11px] text-legend-soft",
                                        "{version}"
                                    }
                                    span { class: "tabular shrink-0 text-[11px] text-legend-soft",
                                        "{dependents}"
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
