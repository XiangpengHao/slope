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
pub(crate) mod tree;

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, FileDetail, ItemSource};
use crate::views::codemap::chrome::{CodeCartouche, CodeLegend, CodeSearch, CratePanel};
use crate::views::codemap::ego::EgoPlate;
use crate::views::codemap::map::CodeChart;
use crate::views::survey::use_code_graph;

/// What the route selects on the code map.
#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) enum CodeSel {
    #[default]
    None,
    Crate(String),
    /// (file path, selected item label — empty for the file itself).
    File(String, String),
}

/// Which reading of the map's ties is drawn. Direction alone cannot thin an
/// unanchored map — every tie is one territory's use and another's users — so
/// each mode anchors on the territories themselves: a block draws only its own
/// heaviest ties in the chosen direction, and hovering it reveals the rest.
/// `Both` is the unthinned picture, kept as an explicit choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum RefDir {
    /// What each file reaches for — its heaviest outgoing ties. The default:
    /// the question a reviewer brings to a change is what it leans on.
    #[default]
    Uses,
    /// Who leans on each file — its heaviest incoming ties.
    UsedBy,
    /// Every tie, unthinned.
    Both,
}

impl RefDir {
    /// How many ties one territory draws at rest in this reading. `Both` keeps
    /// every tie; the anchored readings keep each territory's heaviest few.
    pub(crate) fn per_territory(self) -> Option<usize> {
        match self {
            RefDir::Both => None,
            _ => Some(2),
        }
    }
}

/// Code-altitude session state that must survive route-variant remounts,
/// like the dependency chart's [`DepState`](crate::views::dep::DepState).
/// Provided as a context by the app shell, which outlives every route
/// change, so stepping through focuses — or out to the dependency chart and
/// back — never resets it.
#[derive(Clone, Copy)]
pub(crate) struct CodeState {
    /// Directories the reviewer folded or unfolded by hand, as flips against
    /// the default disclosure depth.
    pub(crate) toggled: Signal<HashSet<u32>>,
    /// File details already fetched, by file id: item lists and same-file
    /// references for the focus plate.
    pub(crate) details: Signal<HashMap<u32, FileDetail>>,
    /// Item source already fetched, by (file id, item id): the definition the
    /// focus plate quotes.
    pub(crate) sources: Signal<HashMap<(u32, u32), ItemSource>>,
    /// Rows the reviewer expanded in place on a focus plate, by (file id,
    /// local item id). View state, not a URL: expansion never leaves the
    /// plate.
    pub(crate) expanded: Signal<HashSet<(u32, u32)>>,
    /// Which reading of the map's ties is drawn.
    pub(crate) ref_dir: Signal<RefDir>,
}

impl CodeState {
    pub(crate) fn new() -> Self {
        Self {
            toggled: Signal::new(HashSet::new()),
            details: Signal::new(HashMap::new()),
            sources: Signal::new(HashMap::new()),
            expanded: Signal::new(HashSet::new()),
            ref_dir: Signal::new(RefDir::default()),
        }
    }
}

pub(crate) fn use_code() -> CodeState {
    use_context()
}

impl From<&Route> for CodeSel {
    /// The selection the current route asks for.
    fn from(route: &Route) -> Self {
        match route {
            Route::CodeCrate { name } => CodeSel::Crate(name.clone()),
            Route::CodeFile { path, item } => CodeSel::File(path.join("/"), item.clone()),
            _ => CodeSel::None,
        }
    }
}

/// The route that selects a file on the map.
pub(crate) fn file_route(path: &str) -> Route {
    Route::CodeFile {
        path: path.split('/').map(str::to_string).collect(),
        item: String::new(),
    }
}

/// The route that selects one item inside a file.
pub(crate) fn item_route(path: &str, item: &str) -> Route {
    Route::CodeFile {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
    }
}

/// The code shell: lays the code furniture over whichever altitude the route
/// asks for — the ambient map, or one selection's focus plate. Mounted by the
/// survey shell, which has already loaded the survey this map reads.
#[component]
pub(crate) fn CodeShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let route = use_route::<Route>();
    let sel = CodeSel::from(&route);
    // A file or item focus replaces the map with its own plate; the map's
    // cartouche and legend are map furniture and go with it.
    let focused = matches!(sel, CodeSel::File(_, _));
    let changed = graph.files.iter().any(|f| f.changed);

    rsx! {
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
                // Directly under the cartouche: one stack, not a plate at each
                // end of 480px of empty paper.
                CodeLegend {
                    notes: graph.notes.clone(),
                    changed,
                    start_open: true,
                }
            }
            // Phone: everything stacks under the cartouche.
            div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
                CodeCartouche {
                    graph: graph.clone(),
                    workspace: workspace.clone(),
                    diff_line,
                }
                CodeSearch { graph: graph.clone() }
            }
            div { class: "pointer-events-none absolute bottom-3 left-3 z-10 sm:hidden",
                CodeLegend {
                notes: graph.notes.clone(),
                changed,
                start_open: false,
            }
            }
        }
        // Wider than the dependency chart's search: an item hit carries
        // `src/analyze/manifest.rs:67`, and the name must not be the half that
        // gets squeezed.
        div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-72 flex-col gap-2 sm:flex",
            CodeSearch { graph }
        }
    }
}

/// `/code` — the whole map. The chart lives in the code shell; this route
/// adds nothing else.
#[component]
pub(crate) fn CodeOverview() -> Element {
    rsx! {}
}

/// `/code/crate/:name` — one crate's district selected; its boundary
/// references are drawn and the panel lists what crosses it.
#[component]
pub(crate) fn CodeCrate(name: String) -> Element {
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
pub(crate) fn CodeFile(path: Vec<String>, item: String) -> Element {
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
