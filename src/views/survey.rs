//! The survey gate: the rust-analyzer reading, loaded once for every altitude
//! that draws it.
//!
//! Two rungs of the ladder read the code survey — the data chart and the
//! function chart — and neither may pay for it twice. The gate is mounted by
//! the app shell above both route families, so stepping from a type to the
//! code that works on it, or out to the crates and back, never re-runs
//! rust-analyzer. It holds the two moments the survey has of its own: the
//! wait, which is honest about its length, and the failure, which says what
//! happened and offers the retry.

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::CodeGraph;
use crate::load::code_graph;
use crate::views::data::DataShell;
use crate::views::func::FnShell;

/// Which chart the gate hands the survey to. The gate loads one reading of
/// the workspace; which rung draws it is the route's business.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Rung {
    /// `/data` — the state the workspace keeps.
    Data,
    /// `/fn` — the code that runs, and what calls what.
    Fns,
}

pub(super) type SurveyResource = Resource<Result<CodeGraph, ServerFnError>>;

/// The loaded survey, for the route components under the gate.
pub(in crate::views) fn use_survey() -> Option<CodeGraph> {
    let res = try_use_context::<SurveyResource>()?;
    let state = res.read();
    state.as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

/// The gate itself: loads the survey this rung reads, holds its loading and
/// failure moments, and hands it to the chart.
#[component]
pub(super) fn SurveyGate(rung: Rung, workspace: String, diff_line: String) -> Element {
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
            Some(Ok(graph)) => match rung {
                Rung::Data => rsx! {
                    DataShell {
                        graph: graph.clone(),
                        workspace: workspace.clone(),
                        diff_line: diff_line.clone(),
                    }
                },
                Rung::Fns => rsx! {
                    FnShell {
                        graph: graph.clone(),
                        workspace: workspace.clone(),
                        diff_line: diff_line.clone(),
                    }
                },
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

/// The survey failed. Say what happened, in words, and offer a retry. Both
/// code altitudes are gone with it; the dependency chart does not need it and
/// keeps working.
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
