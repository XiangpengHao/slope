//! The code survey, fetched once for the two altitudes that read it.
//!
//! `/code` and `/data` are two readings of one rust-analyzer survey — the same
//! files, the same items, the same resolved references — so the fetch, its
//! loading moment, and its failure plate live here instead of in either
//! altitude. This shell stays mounted across a rung change on the altitude
//! ladder, so stepping from the code map to the data chart never asks the
//! server for the survey a second time.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, code_graph};
use crate::views::codemap::CodeShell;
use crate::views::datamap::DataShell;

pub type CodeResource = Resource<Result<CodeGraph, ServerFnError>>;

/// The loaded code survey, for route components under this shell.
pub fn use_code_graph() -> Option<CodeGraph> {
    let res = try_use_context::<CodeResource>()?;
    let state = res.read();
    state.as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

/// Loads the survey, gates its loading and error states, and hands it to
/// whichever altitude the route asks for. Mounted by the atlas shell for every
/// `/code` and `/data` route.
#[component]
pub fn SurveyShell(workspace: String, diff_line: String) -> Element {
    let resource: CodeResource = use_resource(code_graph);
    use_context_provider(|| resource);

    let route = use_route::<Route>();
    let data = matches!(route, Route::DataOverview {} | Route::DataType { .. });

    let state = resource.read();
    rsx! {
        match &*state {
            None => rsx! {
                Surveying {}
            },
            Some(Err(err)) => rsx! {
                SurveyFailed { message: err.to_string(), resource }
            },
            Some(Ok(graph)) if data => rsx! {
                DataShell {
                    graph: graph.clone(),
                    workspace: workspace.clone(),
                    diff_line: diff_line.clone(),
                }
            },
            Some(Ok(graph)) => rsx! {
                CodeShell {
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

/// The code survey failed. Say what happened, in words, and offer a retry. Both
/// altitudes that read the survey are gone with it; the dependency chart does
/// not need it and keeps working.
#[component]
fn SurveyFailed(message: String, resource: CodeResource) -> Element {
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
