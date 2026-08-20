//! The data altitude: the workspace's state charted as types that hold types.
//!
//! The third rung of the review ladder — crates, then files and items, then
//! **types**. It answers what the other two cannot: what state exists, what
//! shape it has, and who can reach it. In rust that answer is statically
//! readable, because ownership is written in the types: `Arc<Mutex<T>>` says
//! shared mutable state in the signature, `&'a T` says view, a `static` says
//! state no type holds. The chart draws exactly that and quotes the rest.
//!
//! One block per type, seated in the frame of the module that declares it, a
//! hairline from every held type to its holder, and the code map's reference
//! ties lifted to type precision underneath. Clicking a type opens the plate
//! that already quotes its source, one altitude up — this altitude adds no
//! second plate of its own.

pub(crate) mod chrome;
pub(crate) mod layout;
pub(crate) mod map;
pub(crate) mod model;

use dioxus::prelude::*;

use crate::Route;
use crate::api::CodeGraph;
use crate::views::codemap::use_code;
use crate::views::datamap::chrome::{DataCartouche, DataLegend, DataSheet};
use crate::views::datamap::map::DataChart;
use crate::views::datamap::model::DataModel;
use crate::views::survey::use_code_graph;

/// The selection the current route asks for: the defining file, then the label
/// the type's definition plate selects by.
pub fn data_selection(route: &Route) -> Option<(String, String)> {
    match route {
        Route::DataType { path, item } => Some((path.join("/"), item.clone())),
        _ => None,
    }
}

/// The route that selects one type on the chart.
pub fn data_type_route(path: &str, item: &str) -> Route {
    Route::DataType {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
    }
}

/// `/data` — the whole chart. The chart lives in the survey shell; this route
/// adds nothing else.
#[component]
pub fn DataOverview() -> Element {
    rsx! {}
}

/// `/data/type/:..path?:item` — one type selected. On the chart, what a shape
/// change to it could reach keeps its ink; this sheet says who holds it and
/// what it holds. Its definition plate stays one step further, on the sheet's
/// own link. The key carries the whole selection, so re-centering starts the
/// sheet's folds closed.
#[component]
pub fn DataType(path: Vec<String>, item: String) -> Element {
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

/// The data chart and its furniture. Mounted by the survey shell, which has
/// already loaded the survey both it and the code map read.
#[component]
pub fn DataShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let code = use_code();
    let route = use_route::<Route>();
    let sel = data_selection(&route);
    // The cartouche and the legend need the survey's totals, not its geometry:
    // one pass over the wire model, kept until the survey itself changes. The
    // reading toggle is peeked, not read: it moves which ties rest on the
    // paper, and no fact on this plate.
    let facts = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *code.ref_dir.peek()).facts(graph.unresolved)
    }));

    rsx! {
        DataChart { graph, sel }
        Outlet::<Route> {}
        div { class: "pointer-events-none absolute bottom-3 left-3 top-3 z-10 hidden w-64 flex-col gap-2 sm:flex",
            DataCartouche {
                facts: facts(),
                workspace: workspace.clone(),
                diff_line: diff_line.clone(),
            }
            DataLegend { facts: facts(), start_open: true }
        }
        // Narrow viewports are not a designed composition, only a serviceable
        // one: the chrome stacks and the chart keeps the rest of the paper.
        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
            DataCartouche { facts: facts(), workspace, diff_line }
        }
        div { class: "pointer-events-none absolute bottom-3 left-3 z-10 sm:hidden",
            DataLegend { facts: facts(), start_open: false }
        }
    }
}
