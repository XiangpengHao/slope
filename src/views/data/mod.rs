//! The second altitude: the workspace's state, tiered by who holds it.
//!
//! The second rung of the review ladder — the crates, then the **data**:
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
//! where one type's impls lean on another. Methods are not rows on the paper:
//! a block is state only. The selection sheet is a list rather than a
//! drawing, so it does say what the selected type offers — the contracts it
//! promises, and every method written for it anywhere in the workspace, each
//! row naming the file and line it is written on. Clicking such a row —
//! anything this chart draws no block for — quotes its source on the
//! quotation plate beside the sheet, so the reading never leaves the chart.

pub(crate) mod chrome;
pub(crate) mod layout;
pub(crate) mod map;
pub(crate) mod model;
pub(crate) mod quote;

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::CodeGraph;
use crate::load::code_graph;
use crate::views::chrome::plural;
use crate::views::data::chrome::{DataCartouche, DataSearch, DataSheet};
use crate::views::data::map::DataChart;
use crate::views::data::model::{DataModel, Folds};
use crate::views::data::quote::Quotation;

/// What the route selects on the chart.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum DataSel {
    /// One datum: the defining file, then the label its selection sheet
    /// selects by.
    Mark(String, String),
    /// One module boundary: the crate — by its cargo package name — then the
    /// module path as rust nests it.
    Mod(Vec<String>),
}

impl DataSel {
    /// The selection the current route asks for, or `None` where the route
    /// is not this chart's.
    fn of(route: &Route) -> Option<Self> {
        match route {
            Route::DataFocus { path, item, .. } => {
                Some(DataSel::Mark(path.join("/"), item.clone()))
            }
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
        peek: None,
    }
}

/// The selection a sheet — and any quotation opened from it — hangs off: the
/// datum's own file, then the label it selects by.
pub(super) type Sel = (String, String);

/// Which row of a sheet is open as a quotation, in the URL: the file the
/// quoted item is written in, then its label. Two facts, joined by a
/// character neither a path nor a rust name can contain, so the address bar
/// still says in words what is on the plate.
pub(super) fn peek_key(path: &str, label: &str) -> String {
    format!("{path}@{label}")
}

/// The (file, label) a `peek=` names, or `None` where it names nothing this
/// survey can quote.
pub(super) fn peek_at(key: &str) -> Option<(&str, &str)> {
    let (path, label) = key.split_once('@')?;
    (!path.is_empty() && !label.is_empty()).then_some((path, label))
}

/// The route that keeps the current selection and opens one of its rows as a
/// quotation. The selection never moves: a quotation is a reading of the
/// sheet, not a step to another mark.
pub(super) fn peek_route(sel: &Sel, path: &str, label: &str) -> Route {
    Route::DataFocus {
        path: sel.0.split('/').map(str::to_string).collect(),
        item: sel.1.clone(),
        peek: Some(peek_key(path, label)),
    }
}

/// The route that selects one module boundary.
pub(super) fn mod_route(key: Vec<String>) -> Route {
    Route::DataModFocus { module: key }
}

/// Which reading of the chart's uses edges is drawn. Direction alone cannot
/// thin an unanchored chart — every edge is one type's use and another's users
/// — so each mode anchors on the marks themselves: a block draws only its own
/// heaviest edges in the chosen direction, and hovering it reveals the rest.
/// `Both` is the unthinned picture, kept as an explicit choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum RefDir {
    /// What each mark's own code reaches for — its heaviest edges out. The
    /// default: the question a reviewer brings to a change is what it leans on.
    #[default]
    Uses,
    /// Whose code leans on each mark — its heaviest edges in.
    UsedBy,
    /// Every edge, unthinned.
    Both,
}

impl RefDir {
    /// How many edges one mark draws at rest in this reading. `Both` keeps
    /// every edge; the anchored readings keep each mark's heaviest few.
    pub(crate) fn per_territory(self) -> Option<usize> {
        match self {
            RefDir::Both => None,
            _ => Some(2),
        }
    }
}

/// The data chart's own review-session state. Both stores are this altitude's:
/// a fold is a reading and so is the direction the uses edges are read in, and
/// what the chart draws is the reading, while the URL carries the selection.
/// Provided as a context by the app shell, which outlives every route change,
/// so stepping through selections — or out to the dependency chart and back —
/// never resets either one.
#[derive(Clone, Copy)]
pub(crate) struct DataState {
    /// The modules the reviewer folded by hand on this chart.
    pub(crate) folds: Signal<Folds>,
    /// Which reading of the chart's uses edges is drawn.
    pub(crate) ref_dir: Signal<RefDir>,
}

impl DataState {
    pub(crate) fn new() -> Self {
        Self {
            folds: Signal::new(Folds::new()),
            ref_dir: Signal::new(RefDir::default()),
        }
    }
}

/// This altitude's state, from the shell's context.
pub(super) fn use_data() -> DataState {
    use_context::<DataState>()
}

type SurveyResource = Resource<Result<CodeGraph, ServerFnError>>;

/// The loaded survey, for the route components under this shell.
pub(super) fn use_survey() -> Option<CodeGraph> {
    let res = try_use_context::<SurveyResource>()?;
    let state = res.read();
    state.as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

/// The survey gate: loads the rust-analyzer survey this chart reads, holds its
/// loading and failure moments, and hands it to the chart. Mounted by the app
/// shell for every `/data` route, so it stays mounted across a selection
/// change and the server is never asked for the survey twice.
#[component]
pub(crate) fn DataSurvey(workspace: String, diff_line: String) -> Element {
    let resource: SurveyResource = use_resource(code_graph);
    use_context_provider(|| resource);

    let state = resource.read();
    rsx! {
        match &*state {
            None => rsx! {
                Surveying {}
            },
            Some(Err(err)) => rsx! {
                SurveyFailed { message: err.to_string(), resource }
            },
            Some(Ok(graph)) => rsx! {
                DataShell {
                    graph: graph.clone(),
                    workspace: workspace.clone(),
                    diff_line: diff_line.clone(),
                }
            },
        }
    }
}

/// Loading: the sources are being read. Honest about the wait — the first
/// survey of a workspace runs rust-analyzer over everything.
#[component]
fn Surveying() -> Element {
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

/// The survey failed. Say what happened, in words, and offer a retry. This
/// chart is gone with it; the dependency chart does not need it and keeps
/// working.
#[component]
fn SurveyFailed(message: String, resource: SurveyResource) -> Element {
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
                    "The dependency chart still works without it."
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
                        to: Route::DepOverview {},
                        "← dependencies"
                    }
                }
            }
        }
    }
}

/// `/data` — the whole chart. The chart lives in the survey shell; this
/// route adds nothing else.
#[component]
pub(crate) fn DataOverview() -> Element {
    rsx! {}
}

/// `/data/mark/:..path?:item` — one datum selected, and optionally one of its
/// rows opened as a quotation (`peek=<file>@<label>`). The chart keeps the
/// blast radius inked; the sheet says who holds it, who names it, and who
/// uses it, in rows a reader can follow; the quotation stands to the sheet's
/// left, so the row and what it says are read side by side.
#[component]
pub(crate) fn DataFocus(path: Vec<String>, item: String, peek: Option<String>) -> Element {
    let Some(graph) = use_survey() else {
        return rsx! {};
    };
    let joined = path.join("/");
    let quoted = peek
        .as_deref()
        .and_then(peek_at)
        .map(|(at, label)| (at.to_string(), label.to_string()));
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:items-start sm:p-3",
            DataSheet {
                key: "{joined}|{item}",
                graph: graph.clone(),
                path: joined.clone(),
                item: item.clone(),
                peek,
            }
        }
        if let Some((at, label)) = quoted {
            // The quotation takes the width the sheet cannot: it seats to the
            // left of it on the desktop, and on a narrow viewport it covers
            // the sheet, which is one back-step away.
            div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-3 z-20 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-[19.5rem] sm:items-start sm:p-3 sm:pl-0",
                Quotation {
                    key: "{at}|{label}",
                    graph,
                    sel: (joined, item),
                    path: at,
                    label,
                }
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

/// The data chart and its furniture, over the survey the gate above loaded.
#[component]
fn DataShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let data = use_data();
    let route = use_route::<Route>();
    let sel = DataSel::of(&route);
    // What Escape closes first, while a row of the sheet is quoted: the same
    // selection with the quotation shut.
    let unquote = match &route {
        Route::DataFocus {
            path,
            item,
            peek: Some(_),
        } => Some(mark_route(&path.join("/"), item)),
        _ => None,
    };
    let facts = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *data.ref_dir.peek(), &data.folds.read()).facts(graph.unresolved)
    }));
    // The survey's own limits, for the cartouche's fold: the unresolved
    // census first, then the walk's notes, then the references' — this chart
    // draws both inks, and the holding line is the one it is about.
    let limits: Vec<String> = {
        let mut notes = Vec::new();
        if graph.unresolved > 0 {
            notes.push(format!(
                "{} the survey could not resolve.",
                plural(graph.unresolved as usize, "name")
            ));
        }
        notes.extend(graph.walk_notes.iter().cloned());
        notes.extend(graph.notes.iter().cloned());
        notes
    };

    rsx! {
        DataChart { graph: graph.clone(), sel, unquote }
        Outlet::<Route> {}
        div { class: "pointer-events-none absolute left-3 top-3 z-10 hidden w-64 sm:block",
            DataCartouche {
                facts: facts(),
                workspace: workspace.clone(),
                diff_line: diff_line.clone(),
                notes: limits.clone(),
            }
        }
        // Narrow viewports are a serviceable fallback, not a composition.
        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
            DataCartouche {
                facts: facts(),
                workspace,
                diff_line,
                notes: limits,
            }
            DataSearch { graph: graph.clone() }
        }
        // Search top-right, the choreography every altitude keeps. Wide, so
        // a hit's `src/analyze/manifest.rs:67` never squeezes the name.
        div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-72 flex-col gap-2 sm:flex",
            DataSearch { graph }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The URL is the review trail, so a quotation has to survive a round
    /// trip through it — including a method's label, which carries its own
    /// `::`, and a path, which carries its own `/`.
    #[test]
    fn a_quoted_row_round_trips_through_the_url() {
        let key = peek_key("src/views/data/chrome.rs", "DataModel::hold_rows");
        assert_eq!(
            peek_at(&key),
            Some(("src/views/data/chrome.rs", "DataModel::hold_rows"))
        );
        let route = peek_route(
            &("src/graph/data.rs".to_string(), "CodeGraph".to_string()),
            "src/views/data/chrome.rs",
            "DataModel::hold_rows",
        );
        // The selection is untouched by the quotation: a plate opens, the
        // chart does not move.
        assert_eq!(
            route.to_string(),
            "/data/mark/src/graph/data.rs?peek=src/views/data/chrome.rs@DataModel::hold_rows&item=CodeGraph"
        );
        // And an unquoted selection's URL is exactly what it always was.
        assert_eq!(
            mark_route("src/graph/data.rs", "CodeGraph").to_string(),
            "/data/mark/src/graph/data.rs?item=CodeGraph"
        );
    }

    /// Half a key names nothing: the sheet stays open and no plate does.
    #[test]
    fn a_key_that_names_no_pair_quotes_nothing() {
        assert_eq!(peek_at("src/graph/data.rs"), None);
        assert_eq!(peek_at("@CodeGraph"), None);
        assert_eq!(peek_at("src/graph/data.rs@"), None);
    }
}
