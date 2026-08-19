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
use crate::views::datamap::chrome::{DataCartouche, DataLegend};
use crate::views::datamap::map::DataChart;
use crate::views::datamap::model::DataModel;

/// `/data` — the whole chart. The chart lives in the survey shell; this route
/// adds nothing else. There is no sub-focus at this altitude in v1: selecting a
/// type climbs to its definition plate at the code altitude.
#[component]
pub fn DataOverview() -> Element {
    rsx! {}
}

/// The data chart and its furniture. Mounted by the survey shell, which has
/// already loaded the survey both it and the code map read.
#[component]
pub fn DataShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let code = use_code();
    // The cartouche and the legend need the survey's totals, not its geometry:
    // one pass over the wire model, kept until the survey itself changes. The
    // reading toggle is peeked, not read: it moves which ties rest on the
    // paper, and no fact on this plate.
    let facts = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *code.ref_dir.peek()).facts(graph.unresolved)
    }));

    rsx! {
        DataChart { graph }
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
