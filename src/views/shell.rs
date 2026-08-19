//! The atlas shell: loads the analysis once, gates the loading and error
//! states, keeps the review trail in step with the URL, and lays the chart
//! furniture over whichever route is active.

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{WorkspaceGraph, workspace_graph};
use crate::views::atlas::Chart;
use crate::views::chrome::{Legend, SearchBox, TitleBlock};

type GraphResource = Resource<Result<WorkspaceGraph, ServerFnError>>;

/// The loaded graph, for route components. `None` only while the shell is
/// still showing the survey or error state, which never renders an Outlet.
pub fn use_graph() -> Option<WorkspaceGraph> {
    let res = use_context::<GraphResource>();
    let state = res.read();
    state.as_ref().and_then(|r| r.as_ref().ok()).cloned()
}

/// One step of the review trail: `None` is the whole chart, `Some(step)` a
/// selection — crate names joined with `+`, or a whole ring as `ring:N`
/// (crate names can contain neither `+` nor `:`).
pub type TrailStep = Option<String>;

/// The ring index a trail step selects, if it is a ring step.
pub fn step_ring(step: &str) -> Option<u32> {
    step.strip_prefix("ring:")?.parse().ok()
}

/// Which of the selection's edges the chart draws. Manifest events are
/// always drawn regardless. Defaults to dependencies only: the compact
/// reading; the other two readings are one toggle away.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DirFilter {
    /// What the selection depends on.
    #[default]
    Deps,
    /// What depends on the selection.
    Users,
    /// Every route from the root to the selection: what depends on it, then
    /// what depends on those, hop by hop until the chain reaches the top.
    PathToRoot,
}

/// The review trail, kept in step with the browser history. Every selection
/// is a URL; back retraces the review, forward replays it.
#[derive(Clone, Default, PartialEq)]
pub struct Trail {
    pub steps: Vec<TrailStep>,
    /// Where in `steps` the current route sits.
    pub at: usize,
}

impl Trail {
    /// Record a route change, telling a back/forward retrace apart from a
    /// new step by comparing against the recorded neighbors.
    pub fn note(&mut self, step: TrailStep) {
        if self.steps.is_empty() {
            self.steps.push(step);
            self.at = 0;
            return;
        }
        if self.steps.get(self.at) == Some(&step) {
            return;
        }
        if self.at > 0 && self.steps.get(self.at - 1) == Some(&step) {
            self.at -= 1;
            return;
        }
        if self.steps.get(self.at + 1) == Some(&step) {
            self.at += 1;
            return;
        }
        self.steps.truncate(self.at + 1);
        self.steps.push(step);
        self.at = self.steps.len() - 1;
    }

    /// The crate the current route focuses, if any.
    pub fn current_focus(&self) -> Option<String> {
        self.steps.get(self.at).cloned().flatten()
    }

    /// The crates this stretch of the trail walked through since it last
    /// passed the whole chart, in visiting order — the review's breadcrumb.
    pub fn walked(&self) -> Vec<String> {
        if self.steps.is_empty() {
            return Vec::new();
        }
        let upto = &self.steps[..=self.at.min(self.steps.len() - 1)];
        let start = upto
            .iter()
            .rposition(|s| s.is_none())
            .map(|i| i + 1)
            .unwrap_or(0);
        let mut walked: Vec<String> = Vec::new();
        for step in &upto[start..] {
            if let Some(name) = step
                && !walked.iter().any(|n| n == name)
            {
                walked.push(name.clone());
            }
        }
        walked
    }
}

// The review session's state lives in globals, not component scope: the
// router remounts parts of the tree across route-variant changes, and the
// trail must outlive every remount.
static TRAIL: GlobalSignal<Trail> = Signal::global(Trail::default);
static VISITED: GlobalSignal<HashSet<String>> = Signal::global(HashSet::new);
static ANNOUNCE: GlobalSignal<String> = Signal::global(String::new);
static DIR: GlobalSignal<DirFilter> = Signal::global(DirFilter::default);
static SELECTED: GlobalSignal<Vec<String>> = Signal::global(Vec::new);

/// Shared review state: the trail, which crates were visited, the edge
/// direction filter, and the screen-reader announcement line.
#[derive(Clone, Copy)]
pub struct AtlasState {
    pub trail: Signal<Trail>,
    pub visited: Signal<HashSet<String>>,
    pub announce: Signal<String>,
    /// Which direction of the selection's edges the chart draws.
    pub dir: Signal<DirFilter>,
    /// The current selection, materialized to crate names (a ring step
    /// resolves to every name on that ring; the overview resolves to the
    /// center). Written by the chart; read by modifier-clicks to toggle.
    pub selected: Signal<Vec<String>>,
}

pub fn use_atlas() -> AtlasState {
    AtlasState {
        trail: TRAIL.signal(),
        visited: VISITED.signal(),
        announce: ANNOUNCE.signal(),
        dir: DIR.signal(),
        selected: SELECTED.signal(),
    }
}

/// The browser's back button, from code: one step back along the trail.
pub fn history_back() {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window()
        && let Ok(history) = window.history()
    {
        let _ = history.back();
    }
}

/// Wraps every page.
#[component]
pub fn AtlasShell() -> Element {
    let resource: GraphResource = use_resource(workspace_graph);
    use_context_provider(|| resource);

    let atlas = use_atlas();

    // Keep the trail in step with the URL. An effect, not a render-time
    // write: writes during the hydration render do not stick, which would
    // silently drop the trail's first step.
    let route = use_route::<Route>();
    let step: TrailStep = match &route {
        Route::Overview {} => None,
        Route::Focus { name } => Some(name.clone()),
        Route::RingSel { hop } => Some(format!("ring:{hop}")),
    };
    use_effect(use_reactive((&step,), move |(step,)| {
        let mut trail = atlas.trail;
        if trail.peek().steps.get(trail.peek().at) == Some(&step) {
            return;
        }
        let mut visited = atlas.visited;
        let mut announce = atlas.announce;
        trail.write().note(step.clone());
        match &step {
            Some(step) => {
                if let Some(hop) = step_ring(step) {
                    announce.set(format!(
                        "Selected ring {hop}: every crate {hop} hops from the center."
                    ));
                } else {
                    let names: Vec<&str> = step.split('+').collect();
                    for name in &names {
                        visited.write().insert(name.to_string());
                    }
                    match names.len() {
                        1 => announce.set(format!(
                            "Selected {step}; its edges are drawn. Back deselects."
                        )),
                        n => announce.set(format!("Selected {n} crates; their edges are drawn.")),
                    }
                }
            }
            None => announce.set("Whole chart.".to_string()),
        }
    }));

    let state = resource.read();

    rsx! {
        main { class: "atlas relative h-dvh w-dvw overflow-hidden bg-paper font-data text-ink",
            div { class: "sr-only", role: "status", aria_live: "polite", "{atlas.announce}" }
            match &*state {
                None => rsx! {
                    Surveying {}
                },
                Some(Err(err)) => rsx! {
                    SurveyFailed { message: err.to_string(), resource }
                },
                Some(Ok(graph)) => {
                    // What sits at the center of the rings, for the legend's
                    // words: the root crate, or the workspace itself.
                    let center = graph
                        .root_crate
                        .as_ref()
                        .and_then(|id| graph.crates.iter().find(|c| &c.id == id))
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| graph.name.clone());
                    rsx! {
                        Chart { graph: graph.clone() }
                        Outlet::<Route> {}
                        // Desktop: the left column is the reading order of a
                        // review — what this is and what changed, then the key.
                        div { class: "pointer-events-none absolute bottom-3 left-3 top-3 z-10 hidden w-64 flex-col sm:flex",
                            TitleBlock { graph: graph.clone() }
                            div { class: "mt-auto min-h-0 pt-2",
                                Legend { start_open: true, center: center.clone() }
                            }
                        }
                        div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-56 sm:block",
                            SearchBox { graph: graph.clone() }
                        }
                        // Phone: the title block stacks over search, its
                        // changes folded away; the key waits at the foot.
                        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
                            TitleBlock { graph: graph.clone(), changes_open: false }
                            SearchBox { graph: graph.clone() }
                        }
                        div { class: "pointer-events-none absolute bottom-3 left-3 z-10 sm:hidden",
                            Legend { start_open: false, center }
                        }
                    }
                },
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
                p { class: "mt-4 font-chart text-[16px] italic text-ink", "Surveying the workspace…" }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "cargo metadata is resolving the dependency graph"
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
                    "Start slopify inside a cargo workspace, or point it at one and reload:"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink",
                    code { "SLOPIFY_WORKSPACE=/path/to/workspace dx serve" }
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
