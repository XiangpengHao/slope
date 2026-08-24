//! The third altitude: the workspace's state, tiered by who holds it.
//!
//! The third rung of the review ladder — crates, files, then the **data**:
//! every struct, enum, union and static the workspace keeps, whatever its
//! visibility, seated by module. It answers one question — *which of this is
//! top-level state, and which is secondary?* — with one move: a root (a
//! static, or a type no other type keeps in a field) stands at module level
//! wearing the gate's ink left edge, and everything held is drawn inside the
//! block of the type that owns it hardest, the way module frames nest.
//! Reading the tier is reading the paper.
//!
//! Two inks run between the blocks, and only two: solid holding lines with the
//! wrapper's word for what nesting cannot say — sharing, borrowing,
//! cross-module ownership, second holders — and dashed counted uses edges
//! where one type's impls lean on another. Methods are not rows here: a block
//! is state only, and what a type promises is read on its definition plate,
//! one rung up.

pub(crate) mod chrome;
pub(crate) mod layout;
pub(crate) mod map;
pub(crate) mod model;

use dioxus::prelude::*;

use crate::Route;
use crate::api::CodeGraph;
use crate::views::codemap::use_code;
use crate::views::data::chrome::{DataCartouche, DataLegend, DataSearch, DataSheet};
use crate::views::data::map::DataChart;
use crate::views::data::model::{DataModel, Folds};
use crate::views::survey::use_code_graph;

/// What the route selects on the chart.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum DataSel {
    /// One datum: the defining file, then the label its definition plate
    /// selects by.
    Mark(String, String),
    /// One module boundary: the crate, then the module path as rust nests it.
    Mod(Vec<String>),
}

impl DataSel {
    /// The selection the current route asks for, or `None` where the route
    /// is not this chart's.
    fn of(route: &Route) -> Option<Self> {
        match route {
            Route::DataFocus { path, item } => Some(DataSel::Mark(path.join("/"), item.clone())),
            Route::DataModFocus { module } => Some(DataSel::Mod(module.clone())),
            _ => None,
        }
    }
}

/// The route that selects one datum on the chart.
pub(super) fn mark_route(path: &str, item: &str) -> Route {
    Route::DataFocus {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
    }
}

/// The route that selects one module boundary.
pub(super) fn mod_route(key: Vec<String>) -> Route {
    Route::DataModFocus { module: key }
}

/// The data chart's own review-session state. The fold store is this
/// altitude's: a fold is a reading, and what the chart draws is the reading,
/// while the URL carries the selection.
#[derive(Clone, Copy)]
pub(crate) struct DataState {
    /// The modules the reviewer folded by hand on this chart.
    pub(crate) folds: Signal<Folds>,
}

impl DataState {
    pub(crate) fn new() -> Self {
        Self {
            folds: Signal::new(Folds::new()),
        }
    }
}

/// This altitude's state, from the shell's context.
pub(super) fn use_data() -> DataState {
    use_context::<DataState>()
}

/// `/data` — the whole chart. The chart lives in the survey shell; this
/// route adds nothing else.
#[component]
pub(crate) fn DataOverview() -> Element {
    rsx! {}
}

/// `/data/mark/:..path?:item` — one datum selected. The chart keeps the
/// blast radius inked; this sheet says who holds it, who names it, and who
/// uses it, in rows a reader can follow.
#[component]
pub(crate) fn DataFocus(path: Vec<String>, item: String) -> Element {
    let Some(graph) = use_code_graph() else {
        return rsx! {};
    };
    let joined = path.join("/");
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:items-start sm:p-3",
            DataSheet {
                key: "{joined}|{item}",
                graph,
                path: joined.clone(),
                item,
            }
        }
    }
}

/// `/data/mod/:..module` — one module boundary selected. The chart is the
/// whole reading; there is no sheet, because a module is a place on the
/// paper and the paper is already saying it.
#[component]
pub(crate) fn DataModFocus(module: Vec<String>) -> Element {
    let _ = module;
    rsx! {}
}

/// The data chart and its furniture. Mounted by the survey shell, which has
/// already loaded the survey all three code-reading altitudes share.
#[component]
pub(crate) fn DataShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let code = use_code();
    let data = use_data();
    let route = use_route::<Route>();
    let sel = DataSel::of(&route);
    let facts = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *code.ref_dir.peek(), &data.folds.read()).facts(graph.unresolved)
    }));
    // The walk's limits first, then the references' — this chart draws both
    // inks, and the holding line is the one it is about.
    let limits: Vec<String> = [graph.walk_notes.clone(), graph.notes.clone()].concat();

    rsx! {
        DataChart { graph: graph.clone(), sel }
        Outlet::<Route> {}
        div { class: "pointer-events-none absolute bottom-3 left-3 top-3 z-10 hidden w-64 flex-col gap-2 sm:flex",
            DataCartouche {
                facts: facts(),
                workspace: workspace.clone(),
                diff_line: diff_line.clone(),
            }
            DataLegend { facts: facts(), notes: limits.clone(), start_open: true }
        }
        // Narrow viewports are a serviceable fallback, not a composition.
        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
            DataCartouche { facts: facts(), workspace, diff_line }
            DataSearch { graph: graph.clone() }
        }
        div { class: "pointer-events-none absolute bottom-3 left-3 z-10 sm:hidden",
            DataLegend { facts: facts(), notes: limits, start_open: false }
        }
        // Search top-right, the choreography every altitude keeps. Wide, so
        // a hit's `src/analyze/manifest.rs:67` never squeezes the name.
        div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-72 flex-col gap-2 sm:flex",
            DataSearch { graph }
        }
    }
}
