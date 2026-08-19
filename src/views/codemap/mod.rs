//! The code altitude: the workspace's code charted as nested territory.
//!
//! One containment tree — crate → directory → file → type → member — drawn as
//! districts holding blocks holding landmark rows. References are resolved
//! semantically by rust-analyzer on the server and always drawn between the
//! lowest containers the reader can see, summed. Selecting anything replaces
//! the ambient map with a focus plate at item precision. Every focus is a URL.

pub(crate) mod chrome;
pub(crate) mod ego;
pub(crate) mod map;
pub(crate) mod model;
pub mod tree;

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, FileDetail, ItemSource, code_graph};
use crate::views::codemap::chrome::{CodeCartouche, CodeLegend, CodeSearch, CratePanel};
use crate::views::codemap::ego::EgoPlate;
use crate::views::codemap::map::CodeChart;

/// What the route selects on the code map.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum CodeSel {
    #[default]
    None,
    Crate(String),
    /// (file path, selected item label — empty for the file itself).
    File(String, String),
}

// Session state that must survive route-variant remounts, like the dep
// chart's globals.
/// Directories the reviewer folded or unfolded by hand, as flips against
/// the default disclosure depth.
static TOGGLED: GlobalSignal<HashSet<u32>> = Signal::global(HashSet::new);
/// File details already fetched, by file id: item lists and same-file
/// references for the focus plate.
static DETAILS: GlobalSignal<HashMap<u32, FileDetail>> = Signal::global(HashMap::new);
/// Item source already fetched, by (file id, item id): the definition the
/// focus plate quotes.
static SOURCES: GlobalSignal<HashMap<(u32, u32), ItemSource>> = Signal::global(HashMap::new);

#[derive(Clone, Copy)]
pub struct CodeState {
    pub toggled: Signal<HashSet<u32>>,
    pub details: Signal<HashMap<u32, FileDetail>>,
    pub sources: Signal<HashMap<(u32, u32), ItemSource>>,
}

pub fn use_code() -> CodeState {
    CodeState {
        toggled: TOGGLED.signal(),
        details: DETAILS.signal(),
        sources: SOURCES.signal(),
    }
}

type CodeResource = Resource<Result<CodeGraph, ServerFnError>>;

/// The loaded code survey, for route components under the code shell.
pub fn use_code_graph() -> Option<CodeGraph> {
    let res = try_use_context::<CodeResource>()?;
    let state = res.read();
    state.as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

/// The selection the current route asks for.
pub fn route_selection(route: &Route) -> CodeSel {
    match route {
        Route::CodeCrate { name } => CodeSel::Crate(name.clone()),
        Route::CodeFile { path, item } => CodeSel::File(path.join("/"), item.clone()),
        _ => CodeSel::None,
    }
}

/// The route that selects a file on the map.
pub fn file_route(path: &str) -> Route {
    Route::CodeFile {
        path: path.split('/').map(str::to_string).collect(),
        item: String::new(),
    }
}

/// The route that selects one item inside a file.
pub fn item_route(path: &str, item: &str) -> Route {
    Route::CodeFile {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
    }
}

/// The code shell: loads the survey, gates its loading and error states, and
/// lays the code furniture over whichever altitude the route asks for — the
/// ambient map, or one selection's focus plate. Mounted by the atlas shell for
/// every `/code` route.
#[component]
pub fn CodeShell(workspace: String, diff_line: String) -> Element {
    let resource: CodeResource = use_resource(code_graph);
    use_context_provider(|| resource);

    let route = use_route::<Route>();
    let sel = route_selection(&route);
    // A file or item focus replaces the map with its own plate; the map's
    // cartouche and legend are map furniture and go with it.
    let focused = matches!(sel, CodeSel::File(_, _));

    let state = resource.read();
    rsx! {
        match &*state {
            None => rsx! {
                SurveyingCode {}
            },
            Some(Err(err)) => rsx! {
                CodeSurveyFailed { message: err.to_string(), resource }
            },
            Some(Ok(graph)) => rsx! {
                if !focused {
                    CodeChart {
                        graph: graph.clone(),
                        sel: sel.clone(),
                        workspace: workspace.clone(),
                    }
                }
                Outlet::<Route> {}
                if !focused {
                    div { class: "pointer-events-none absolute bottom-3 left-3 top-3 z-10 hidden w-64 flex-col gap-2 sm:flex",
                        CodeCartouche {
                            graph: graph.clone(),
                            workspace: workspace.clone(),
                            diff_line: diff_line.clone(),
                        }
                        div { class: "mt-auto",
                            CodeLegend { graph: graph.clone(), start_open: true }
                        }
                    }
                    // Phone: everything stacks under the cartouche.
                    div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
                        CodeCartouche {
                            graph: graph.clone(),
                            workspace: workspace.clone(),
                            diff_line: diff_line.clone(),
                        }
                        CodeSearch { graph: graph.clone() }
                    }
                    div { class: "pointer-events-none absolute bottom-3 left-3 z-10 sm:hidden",
                        CodeLegend { graph: graph.clone(), start_open: false }
                    }
                }
                // Wider than the dependency chart's search: an item hit
                // carries `src/analyze/manifest.rs:67`, and the name must not
                // be the half that gets squeezed.
                div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-72 flex-col gap-2 sm:flex",
                    CodeSearch { graph: graph.clone() }
                }
            },
        }
    }
}

/// `/code` — the whole map. The chart lives in the code shell; this route
/// adds nothing else.
#[component]
pub fn CodeOverview() -> Element {
    rsx! {}
}

/// `/code/crate/:name` — one crate's district selected; its boundary
/// references are drawn and the panel lists what crosses it.
#[component]
pub fn CodeCrate(name: String) -> Element {
    let Some(graph) = use_code_graph() else {
        return rsx! {};
    };
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:items-start sm:p-3 sm:pt-[168px]",
            CratePanel { key: "{name}", graph, name }
        }
    }
}

/// `/code/file/:..path` — one file in focus; `?item=` focuses one item inside
/// it. Either way the map steps aside for the focus plate. The key carries the
/// whole selection, so re-centering starts every plate's folds closed.
#[component]
pub fn CodeFile(path: Vec<String>, item: String) -> Element {
    let Some(graph) = use_code_graph() else {
        return rsx! {};
    };
    let joined = path.join("/");
    rsx! {
        EgoPlate {
            key: "{joined}|{item}",
            graph,
            path: joined.clone(),
            item,
        }
    }
}

/// Loading: the sources are being read. Honest about the wait — the first
/// survey of a workspace runs rust-analyzer over everything.
#[component]
fn SurveyingCode() -> Element {
    rsx! {
        div { class: "grid h-full place-items-center",
            div { class: "text-center",
                svg {
                    class: "constellation mx-auto",
                    width: "170",
                    height: "100",
                    view_box: "0 0 170 100",
                    "aria-hidden": "true",
                    polyline {
                        class: "constellation-line",
                        points: "12,22 52,70 88,48 128,82 158,40",
                        fill: "none",
                        stroke: "var(--color-ink-line)",
                        stroke_width: "0.9",
                    }
                    for (i , (x , y , r)) in [
                        (12.0, 22.0, 3.0),
                        (52.0, 70.0, 4.4),
                        (88.0, 48.0, 2.7),
                        (128.0, 82.0, 3.8),
                        (158.0, 40.0, 3.2),
                    ]
                        .iter()
                        .enumerate()
                    {
                        circle {
                            class: "constellation-star",
                            style: "animation-delay: {i as f64 * 0.45}s",
                            cx: "{x}",
                            cy: "{y}",
                            r: "{r}",
                            fill: "var(--color-ink)",
                        }
                    }
                }
                p { class: "mt-4 font-data text-[12.5px] text-ink",
                    "rust-analyzer is reading every source file and resolving references"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "the first survey of a workspace takes a while"
                }
            }
        }
    }
}

/// The code survey failed. Say what happened, in words, and offer a retry.
#[component]
fn CodeSurveyFailed(message: String, resource: CodeResource) -> Element {
    rsx! {
        div { class: "grid h-full place-items-center p-4",
            section { class: "plate max-w-lg px-5 py-4",
                h1 { class: "font-chart text-[17px] tracking-[0.18em] uppercase text-ink",
                    "The code survey failed"
                }
                p { class: "mt-2 break-words border-t border-ink-line pt-2 font-data text-[11px] leading-relaxed text-ink",
                    "{message}"
                }
                p { class: "mt-3 font-data text-[10.5px] leading-relaxed text-ink-soft",
                    "The dependency atlas still works without it."
                }
                div { class: "mt-3 flex gap-4",
                    button {
                        class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                        onclick: move |_| {
                            let mut resource = resource;
                            resource.restart();
                        },
                        "retry the survey"
                    }
                    Link {
                        class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                        to: Route::Overview {},
                        "← dependencies"
                    }
                }
            }
        }
    }
}
