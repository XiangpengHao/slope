//! The app shell: loads the workspace analysis once, gates its loading and
//! error states, and hands the paper to whichever altitude the route asks for.

use dioxus::prelude::*;

use crate::Route;
use crate::graph::dep::DepGraph;
use crate::load::dep_graph;
use crate::views::data::DataState;
use crate::views::data::map::DataCamera;
use crate::views::dep::map::DrawnCap;
use crate::views::dep::{DepShell, DepState};
use crate::views::func::FnState;
use crate::views::func::map::FnCamera;
use crate::views::survey::{Rung, SurveyGate};

type GraphResource = Resource<Result<DepGraph, ServerFnError>>;

/// The loaded graph, for route components. `None` only while the shell is
/// still showing the survey or error state, which never renders an Outlet.
pub(super) fn use_graph() -> Option<DepGraph> {
    let res = use_context::<GraphResource>();
    let state = res.read();
    state.as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

/// The browser's back button, from code: one step back along the trail.
pub(super) fn history_back() {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window()
        && let Ok(history) = window.history()
    {
        let _ = history.back();
    }
}

/// Arrow keys retrace the review: every focus is a URL, so left and right
/// are the browser's back and forward on every route. Acts entirely in JS
/// and installs once — the guard keeps remounts from stacking listeners.
/// Typing fields keep their caret keys.
const NAV_KEYS_JS: &str = r#"
if (!window.__slopeNavKeys) {
    window.__slopeNavKeys = (e) => {
        if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
        const t = e.target, tag = t && t.tagName;
        if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || (t && t.isContentEditable)) return;
        if (e.key === 'ArrowLeft') { e.preventDefault(); history.back(); }
        else if (e.key === 'ArrowRight') { e.preventDefault(); history.forward(); }
    };
    document.addEventListener('keydown', window.__slopeNavKeys);
}
"#;

/// Wraps every page.
#[component]
pub(crate) fn AppShell() -> Element {
    let resource: GraphResource = use_resource(dep_graph);
    use_context_provider(|| resource);

    // Every store of review-session state is provided here, on the layout
    // above every route: the router remounts route components across
    // route-variant changes, and the trail, disclosure, and camera memories
    // must outlive every remount. Views reach them through context, so a
    // test can mount any view under a provider of its own.
    use_context_provider(DepState::new);
    use_context_provider(DrawnCap::new);
    use_context_provider(DataState::new);
    use_context_provider(DataCamera::new);
    use_context_provider(FnState::new);
    use_context_provider(FnCamera::new);

    // The back/forward keys live on the shell, not the views: they must
    // survive every route change, and they never need a channel back.
    use_effect(|| {
        document::eval(NAV_KEYS_JS);
    });

    let route = use_route::<Route>();
    // The two altitudes that read the rust-analyzer survey. Their gate is
    // mounted here, above the routes, so stepping between selections — or
    // between the two rungs — never re-runs the survey fetch.
    let survey_route = match &route {
        Route::DataOverview {} | Route::DataFocus { .. } | Route::DataModFocus { .. } => {
            Some(Rung::Data)
        }
        Route::FnOverview {}
        | Route::FnFocus { .. }
        | Route::FnModFocus { .. }
        | Route::FnBandFocus { .. } => Some(Rung::Fns),
        _ => None,
    };
    let state = resource.read();

    rsx! {
        main { class: "slope relative h-dvh w-dvw overflow-hidden bg-paper font-data text-ink",
            match &*state {
                None => rsx! {
                    Surveying {}
                },
                Some(Err(err)) => rsx! {
                    SurveyFailed { message: err.to_string(), resource }
                },
                Some(Ok(graph)) if survey_route.is_some() => {
                    // A code altitude, behind the shared survey gate. The
                    // workspace's identity and epoch ride along so every
                    // altitude stamps the same cartouche facts.
                    // Each side of the arrow is one quoted idiom — the plate
                    // may break the line at the arrow, never inside
                    // `master @ 1a2b3c4` or `working copy`.
                    let nb = |s: &str| s.replace(' ', "\u{a0}");
                    let diff_line = format!(
                        "diff {} → {}",
                        nb(&graph.epoch.base),
                        nb(&graph.epoch.target)
                    );
                    let rung = survey_route.unwrap_or(Rung::Data);
                    rsx! {
                        SurveyGate { rung, workspace: graph.name.clone(), diff_line }
                    }
                }
                Some(Ok(graph)) => rsx! {
                    DepShell { graph: graph.clone() }
                }
            }
        }
    }
}

/// Loading: the chart is being surveyed. One authored moment — a small
/// constellation draws itself in; under reduced motion it is simply there.
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
                        points: "12,78 52,30 88,52 128,18 158,60",
                        fill: "none",
                        stroke: "var(--color-ink-line)",
                        stroke_width: "0.9",
                    }
                    for (i , (x , y , r)) in [
                        (12.0, 78.0, 3.2),
                        (52.0, 30.0, 4.6),
                        (88.0, 52.0, 2.7),
                        (128.0, 18.0, 3.8),
                        (158.0, 60.0, 3.0),
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
                    "cargo metadata is resolving the dependency graph"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "and the VCS is being asked what changed"
                }
            }
        }
    }
}

/// The analysis failed. Say what happened and how to recover, in words.
#[component]
fn SurveyFailed(message: String, resource: GraphResource) -> Element {
    rsx! {
        div { class: "grid h-full place-items-center p-4",
            section { class: "plate max-w-lg px-5 py-4",
                h1 { class: "font-chart text-[17px] tracking-[0.18em] uppercase text-ink",
                    "The survey failed"
                }
                p { class: "mt-2 break-words border-t border-ink-line pt-2 font-data text-[11px] leading-relaxed text-ink",
                    "{message}"
                }
                p { class: "mt-3 font-data text-[10.5px] leading-relaxed text-ink-soft",
                    "Start slope inside a cargo workspace, or point it at one and reload:"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink",
                    code { "SLOPE_WORKSPACE=/path/to/workspace dx serve" }
                }
                button {
                    class: "mt-3 font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                    onclick: move |_| {
                        let mut resource = resource;
                        resource.restart();
                    },
                    "retry the survey"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::dioxus_core::NoOpMutations;

    use super::*;

    // What providing state on the layout leans on: the router keeps a layout
    // mounted while the route variant under it changes. A probe layout counts
    // its mounts; navigating between two variants must not add one.
    #[derive(Debug, Clone, Routable, PartialEq)]
    enum Probe {
        #[layout(ProbeShell)]
        #[route("/")]
        One {},
        #[route("/two/:name")]
        Two { name: String },
    }

    #[component]
    fn ProbeShell() -> Element {
        let mounts = use_context::<std::rc::Rc<std::cell::Cell<u32>>>();
        use_hook(move || mounts.set(mounts.get() + 1));
        rsx! {
            Outlet::<Probe> {}
        }
    }

    #[component]
    fn One() -> Element {
        rsx! {}
    }

    #[component]
    fn Two(name: String) -> Element {
        rsx! {}
    }

    #[test]
    fn the_layout_survives_route_variant_changes() {
        fn app() -> Element {
            rsx! {
                Router::<Probe> {}
            }
        }
        let mounts = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let mut vdom = VirtualDom::new(app).with_root_context(mounts.clone());
        vdom.rebuild_in_place();
        assert_eq!(mounts.get(), 1);

        // Navigate `/` → `/two/serde`, the shape of `/dep` → `/dep/crate/serde`.
        vdom.in_scope(ScopeId::APP, || {
            dioxus::history::history().push("/two/serde".to_string());
        });
        vdom.render_immediate(&mut NoOpMutations);
        vdom.render_immediate(&mut NoOpMutations);
        assert_eq!(
            mounts.get(),
            1,
            "the layout remounted across a route-variant change"
        );
    }
}
