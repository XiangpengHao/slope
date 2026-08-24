//! The third altitude: the workspace's own surface, and what leans on what.
//!
//! The third rung of the review ladder — crates, then files and items, then
//! the **contracts** the code publishes. Every item that crosses a door is
//! one, and every block is a header and its rows, each row a clause: a type
//! with its fields and the methods it publishes, a function with its
//! signature, a trait with the clauses it declares, a static or a const or an
//! alias with the one type it names. It answers what the other two cannot —
//! what this code promises, and what would have to change with it. In rust
//! that answer is statically readable, because the promise is written in the
//! types: `Arc<Mutex<T>>` says shared state in the signature, `&'a T` says
//! view, a `static` says state no type holds. The chart draws exactly that
//! and quotes the rest.
//!
//! Two inks run between the blocks, and only two. **Solid** is interface
//! coupling: a block's own published surface names the other end, so a change
//! there forces a change here — and `impl Trait for Type` is the one solid
//! line no row writes. **Dashed** is implementation coupling: a body leans on
//! the other end, a call or a name written inside a function, which a rewrite
//! can take back without anyone else noticing. Both point at the dependent.
//! Clicking a mark opens the plate that already quotes its source, one
//! altitude up — this altitude adds no second plate of its own.

pub(crate) mod chrome;
pub(crate) mod layout;
pub(crate) mod map;
pub(crate) mod model;

use dioxus::prelude::*;

use crate::Route;
use crate::api::CodeGraph;
use crate::views::codemap::use_code;
use crate::views::surface::chrome::{SurfaceCartouche, SurfaceLegend, SurfaceSheet};
use crate::views::surface::map::SurfaceChart;
use crate::views::surface::model::SurfaceModel;
use crate::views::survey::use_code_graph;

/// What the route selects on the chart. Both readings recede everything they
/// do not reach; neither ever moves a block or the camera.
#[derive(Clone, PartialEq, Debug)]
pub enum SurfaceSel {
    /// One contract: the defining file, then the label the type's definition
    /// plate selects by.
    Mark(String, String),
    /// One module boundary, by the name a fold and a URL both use: the crate,
    /// then the module path as rust nests it.
    Mod(Vec<String>),
}

/// The selection the current route asks for.
fn selection(route: &Route) -> Option<SurfaceSel> {
    match route {
        Route::SurfaceFocus { path, item } => Some(SurfaceSel::Mark(path.join("/"), item.clone())),
        Route::SurfaceModFocus { module } => Some(SurfaceSel::Mod(module.clone())),
        _ => None,
    }
}

/// The route that selects one type on the chart.
pub fn mark_route(path: &str, item: &str) -> Route {
    Route::SurfaceFocus {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
    }
}

/// The route that selects one module frame — the whole boundary, by the key
/// that names it in every build.
pub fn mod_route(key: Vec<String>) -> Route {
    Route::SurfaceModFocus { module: key }
}

/// `/surface` — the whole chart. The chart lives in the survey shell; this
/// route adds nothing else.
#[component]
pub fn SurfaceOverview() -> Element {
    rsx! {}
}

/// `/surface/mark/:..path?:item` — one contract selected. On the chart, what a
/// change to it could reach keeps its ink; this sheet says what reaches it and
/// what it reaches. Its definition plate stays one step further, on the sheet's
/// own link. The key carries the whole selection, so re-centering starts the
/// sheet's folds closed.
#[component]
pub fn SurfaceFocus(path: Vec<String>, item: String) -> Element {
    let Some(graph) = use_code_graph() else {
        return rsx! {};
    };
    let joined = path.join("/");
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:items-start sm:p-3",
            SurfaceSheet {
                key: "{joined}|{item}",
                graph,
                path: joined.clone(),
                item,
            }
        }
    }
}

/// `/surface/mod/:..module` — one module boundary selected. The chart is the
/// whole reading: everything inside the boundary keeps its ink, what crosses it
/// reads a step behind, and the other modules recede. There is no sheet — a
/// module is a place on the paper, and the paper is already saying it.
#[component]
pub fn SurfaceModFocus(module: Vec<String>) -> Element {
    let _ = module;
    rsx! {}
}

/// The surface chart and its furniture. Mounted by the survey shell, which has
/// already loaded the survey both it and the code map read.
#[component]
pub fn SurfaceShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let code = use_code();
    let route = use_route::<Route>();
    let sel = selection(&route);
    // The cartouche and the legend need the survey's totals, not its geometry:
    // one pass over the wire model, kept until the survey itself changes. The
    // reading toggle is peeked, not read: it moves which ties rest on the
    // paper, and no fact on this plate. The doors are read — they decide which
    // types are drawn at all, so every count on the plate follows them.
    let facts = use_memo(use_reactive((&graph,), move |(graph,)| {
        SurfaceModel::build(
            &graph,
            *code.ref_dir.peek(),
            *code.doors.read(),
            &code.folds.read(),
        )
        .facts(graph.unresolved)
    }));

    // The walk's limits first, then the references' — this chart draws both
    // inks, and the interface line is the one it is about.
    let limits: Vec<String> = [graph.walk_notes.clone(), graph.notes.clone()].concat();

    rsx! {
        SurfaceChart { graph, sel }
        Outlet::<Route> {}
        div { class: "pointer-events-none absolute bottom-3 left-3 top-3 z-10 hidden w-64 flex-col gap-2 sm:flex",
            SurfaceCartouche {
                facts: facts(),
                workspace: workspace.clone(),
                diff_line: diff_line.clone(),
            }
            SurfaceLegend {
                facts: facts(),
                notes: limits.clone(),
                start_open: true,
            }
        }
        // Narrow viewports are not a designed composition, only a serviceable
        // one: the chrome stacks and the chart keeps the rest of the paper.
        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
            SurfaceCartouche { facts: facts(), workspace, diff_line }
        }
        div { class: "pointer-events-none absolute bottom-3 left-3 z-10 sm:hidden",
            SurfaceLegend { facts: facts(), notes: limits, start_open: false }
        }
    }
}
