//! The top bar: what this is a reading of, which lens is open, and the finder.
//!
//! One row, fixed height, never wraps. Everything in it is true whichever lens
//! is mounted, which is the test for whether something belongs here at all.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{GraphLoad, SheetLoad};
use crate::components::{IconFind, Mark};
use crate::views::{DepsState, GraphResource, SheetResource, SheetState};

#[component]
pub fn TopBar() -> Element {
    let on_calls = matches!(use_route::<Route>(), Route::Calls { .. });

    let resource: GraphResource = use_context();
    let sheet: SheetResource = use_context();

    let (name, dir) = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(GraphLoad::Ready(workspace))) => {
                (workspace.name.clone(), workspace.manifest_dir.clone())
            }
            _ => (String::new(), String::new()),
        }
    };

    rsx! {
        header { class: "relative flex h-12 shrink-0 items-center gap-3 border-b border-line bg-surface px-3",

            div { class: "flex min-w-0 shrink items-center gap-2",
                Mark {}
                span { class: "truncate text-[13.5px] font-semibold tracking-[-0.01em]",
                    if name.is_empty() { "rust-viewer" } else { "{name}" }
                }
                span { class: "hidden truncate font-mono text-[11px] text-ink-faint xl:block", "{dir}" }
            }

            Lenses {}

            div { class: "flex-1" }

            if on_calls {
                SheetTally { resource: sheet }
            } else {
                GraphTally { resource }
            }

            if on_calls {
                Finder { calls: true }
            } else {
                Finder { calls: false }
            }
        }
    }
}

/// The lenses over this workspace. Two exist, so two are shown; the rest stay
/// unadvertised until they are real, because a permanently disabled tab spends
/// credibility on first run for nothing.
#[component]
fn Lenses() -> Element {
    let current = use_route::<Route>();
    rsx! {
        nav {
            class: "flex shrink-0 items-center gap-0.5 rounded-[7px] bg-sunken p-[3px]",
            "aria-label": "Lenses",
            Lens { to: Route::Deps {}, label: "Dependencies".to_string(), current: current.clone() }
            Lens { to: Route::Calls {}, label: "Calls".to_string(), current }
        }
    }
}

#[component]
fn Lens(to: Route, label: String, current: Route) -> Element {
    let here = std::mem::discriminant(&to) == std::mem::discriminant(&current);
    // `Link` is a component, so its `class` is one prop rather than an element
    // attribute the macro would merge.
    let class = format!(
        "flex items-center whitespace-nowrap rounded-[5px] px-2.5 py-1 text-[12px] font-semibold transition-colors {}",
        if here {
            "bg-surface text-ink shadow-node"
        } else {
            "text-ink-muted hover:text-ink"
        }
    );
    rsx! {
        Link {
            to: to.clone(),
            class,
            "aria-current": if here { "page" } else { "false" },
            "{label}"
        }
    }
}

/// What is in the workspace. Quantities the reader can check against
/// `cargo tree`, never a summary that only this tool can produce.
#[component]
fn GraphTally(resource: GraphResource) -> Element {
    let loaded = resource.read();
    let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
        return rsx! {};
    };
    let duplicates = workspace.duplicates.len();
    rsx! {
        div { class: "tabular hidden shrink-0 items-center gap-2.5 text-[12px] text-ink-muted sm:flex",
            span { class: "whitespace-nowrap", "{workspace.distinct_count} crates" }
            span { class: "hidden whitespace-nowrap md:inline", "{workspace.dependency_count} dependencies" }
            if duplicates > 0 {
                span {
                    class: "chip bg-flag-field text-flag",
                    title: "{duplicates} crate names resolve at more than one version in this workspace",
                    "{duplicates} DUPLICATED"
                }
            }
        }
    }
}

#[component]
fn SheetTally(resource: SheetResource) -> Element {
    let loaded = resource.read();
    let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
        return rsx! {};
    };
    rsx! {
        div { class: "tabular hidden shrink-0 items-center gap-2.5 text-[12px] text-ink-muted sm:flex",
            span { class: "whitespace-nowrap", "{sheet.function_count} functions" }
            span { class: "hidden whitespace-nowrap md:inline", "{sheet.call_count} calls" }
        }
    }
}

/// The finder. Lists what it matched rather than silently guessing one, and
/// opening a match puts it on the pane with the chain that got it there.
#[component]
fn Finder(calls: bool) -> Element {
    let resource: GraphResource = use_context();
    let sheet: SheetResource = use_context();
    let mut deps: DepsState = use_context();
    let mut units: SheetState = use_context();

    let mut query = if calls { units.query } else { deps.query };
    let mut open = use_signal(|| false);

    let needle = query().trim().to_lowercase();
    let matches: Vec<(usize, String, String, usize)> = if needle.is_empty() {
        Vec::new()
    } else if calls {
        let loaded = sheet.read();
        match loaded.as_ref() {
            Some(Some(Ok(SheetLoad::Ready(sheet)))) => {
                // Every kind of unit, not only functions: the pane draws crates,
                // files, types, traits and impls too, and a finder that cannot
                // name what is on the pane is not a finder.
                let mut hits: Vec<&crate::call::Unit> = sheet
                    .units
                    .iter()
                    .filter(|unit| unit.name.to_lowercase().contains(&needle))
                    .collect();
                // Exact first, then the workspace's own code, then by how much
                // of the sheet routes through it.
                hits.sort_by_key(|unit| {
                    (
                        unit.name.to_lowercase() != needle,
                        unit.origin != crate::call::Origin::Workspace,
                        !unit.name.to_lowercase().starts_with(&needle),
                        std::cmp::Reverse(sheet.reach.dominates(unit.id)),
                        std::cmp::Reverse(unit.callers.len()),
                        // Breaks the tie between containers, which dominate
                        // nothing and are called by nothing: the bigger one is
                        // the one a reader meant.
                        std::cmp::Reverse(unit.function_count),
                    )
                });
                hits.into_iter()
                    .take(8)
                    .map(|unit| {
                        // Where it is, precisely enough to tell three functions
                        // of the same name in the same crate apart — which is
                        // the ordinary case, not a corner one.
                        let place = unit
                            .parent
                            .and_then(|parent| sheet.units.get(parent))
                            .filter(|parent| {
                                parent.kind != crate::call::UnitKind::Crate
                                    && unit.kind == crate::call::UnitKind::Function
                            })
                            .map(|parent| {
                                // An impl block is named for the type it is on;
                                // its own name is a whole sentence.
                                parent
                                    .self_ty
                                    .clone()
                                    .unwrap_or_else(|| parent.name.clone())
                            })
                            .unwrap_or_else(|| unit.crate_name.clone());
                        (
                            unit.id,
                            unit.name.clone(),
                            format!("{} · {place}", unit.kind.noun()),
                            // What ranks it: how much calls a function, how much
                            // code is in anything else.
                            if unit.kind == crate::call::UnitKind::Function {
                                unit.callers.len()
                            } else {
                                unit.function_count
                            },
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    } else {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(GraphLoad::Ready(workspace))) => {
                let mut hits: Vec<&crate::graph::Crate> = workspace
                    .crates
                    .iter()
                    .filter(|entry| entry.name.to_lowercase().contains(&needle))
                    .collect();
                // Exact first, then by how much of the graph routes through it.
                hits.sort_by_key(|entry| {
                    (
                        entry.name.to_lowercase() != needle,
                        !entry.name.to_lowercase().starts_with(&needle),
                        std::cmp::Reverse(entry.dependents.len()),
                    )
                });
                hits.into_iter()
                    .take(8)
                    .map(|entry| {
                        (
                            entry.id,
                            entry.name.clone(),
                            entry.version.clone(),
                            entry.dependents.len(),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    };
    let empty = !needle.is_empty() && matches.is_empty();
    let first = matches.first().map(|(id, _, _, _)| *id);

    let mut choose = move |id: usize| {
        if calls {
            let loaded = sheet.read();
            if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                units.reveal(sheet.as_ref(), id);
            }
        } else {
            let loaded = resource.read();
            if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
                deps.reveal(workspace.as_ref(), id);
            }
        }
        open.set(false);
    };

    rsx! {
        div { class: "finder flex shrink-0 items-center",
            IconFind { class: "pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-ink-faint".to_string() }
            input {
                r#type: "search",
                class: "w-full rounded-[7px] border border-line bg-sunken py-[7px] pr-2 pl-8 text-[12.5px] placeholder:text-ink-faint focus-visible:border-outbound focus-visible:bg-surface focus-visible:outline-none",
                placeholder: if calls { "find a function  /" } else { "find a crate  /" },
                "aria-label": if calls { "Find a function" } else { "Find a crate" },
                value: "{query}",
                oninput: move |event| {
                    query.set(event.value());
                    open.set(true);
                },
                onfocus: move |_| open.set(true),
                onkeydown: move |event| {
                    if event.key() == Key::Enter && let Some(id) = first {
                        choose(id);
                    }
                },
            }

            if open() && (!matches.is_empty() || empty) {
                div { class: "plate absolute top-full right-0 z-20 mt-1.5 w-80 max-w-[calc(100vw-1.5rem)] overflow-hidden py-1",
                    if empty {
                        p { class: "px-3 py-2 text-[12px] text-ink-muted",
                            "Nothing here matches \"{needle}\"."
                        }
                    }
                    ul {
                        for (id , name , detail , count) in matches {
                            li { key: "{id}",
                                button {
                                    class: "flex w-full items-baseline gap-2 px-3 py-1.5 text-left hover:bg-sunken",
                                    onclick: move |_| choose(id),
                                    span { class: "min-w-0 flex-1 truncate text-[13px]", "{name}" }
                                    span { class: "tabular shrink-0 truncate font-mono text-[10.5px] text-ink-faint",
                                        "{detail}"
                                    }
                                    span { class: "tabular w-6 shrink-0 text-right text-[11px] text-ink-faint",
                                        "{count}"
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
