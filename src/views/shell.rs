//! The atlas shell: loads the analysis once, gates the loading and error
//! states, keeps the review trail in step with the URL, and lays the chart
//! furniture over whichever route is active.

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{WorkspaceGraph, workspace_graph};
use crate::views::atlas::{Chart, DrawnCap};
use crate::views::chrome::{Legend, SearchBox, TitleBlock};
use crate::views::codemap::CodeState;
use crate::views::codemap::map::CodeCamera;
use crate::views::datamap::map::DataCamera;
use crate::views::survey::SurveyShell;

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
type TrailStep = Option<String>;

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

/// Shared review state: the trail, which crates were visited, the edge
/// direction filter, and the screen-reader announcement line. Provided as a
/// context by the shell — the one scope that survives every route change —
/// so the trail outlives the route-component remounts, and every app
/// instance (a test's `VirtualDom` included) owns its own copy.
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

impl AtlasState {
    pub(crate) fn new() -> Self {
        Self {
            trail: Signal::new(Trail::default()),
            visited: Signal::new(HashSet::new()),
            announce: Signal::new(String::new()),
            dir: Signal::new(DirFilter::default()),
            selected: Signal::new(Vec::new()),
        }
    }
}

pub fn use_atlas() -> AtlasState {
    use_context()
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
pub fn AtlasShell() -> Element {
    let resource: GraphResource = use_resource(workspace_graph);
    use_context_provider(|| resource);

    // Every store of review-session state is provided here, on the layout
    // above every route: the router remounts route components across
    // route-variant changes, and the trail, disclosure, and camera memories
    // must outlive every remount. Views reach them through context, so a
    // test can mount any view under a provider of its own.
    let atlas = use_context_provider(AtlasState::new);
    use_context_provider(DrawnCap::new);
    use_context_provider(CodeState::new);
    use_context_provider(CodeCamera::new);
    use_context_provider(DataCamera::new);

    // The back/forward keys live on the shell, not the views: they must
    // survive every route change, and they never need a channel back.
    use_effect(|| {
        document::eval(NAV_KEYS_JS);
    });

    // Keep the trail in step with the URL. An effect, not a render-time
    // write: writes during the hydration render do not stick, which would
    // silently drop the trail's first step. The code altitude keeps its own
    // selection state; it never writes the dependency trail.
    let route = use_route::<Route>();
    // The two altitudes that read the code survey. One shell serves both, so
    // stepping between them never re-runs the survey fetch.
    let survey_route = matches!(
        &route,
        Route::CodeOverview {}
            | Route::CodeCrate { .. }
            | Route::CodeFile { .. }
            | Route::DataOverview {}
            | Route::DataType { .. }
    );
    let step: Option<TrailStep> = match &route {
        Route::Overview {} => Some(None),
        Route::Focus { name } => Some(Some(name.clone())),
        Route::RingSel { hop } => Some(Some(format!("ring:{hop}"))),
        _ => None,
    };
    use_effect(use_reactive((&step,), move |(step,)| {
        let Some(step) = step else { return };
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
                Some(Ok(graph)) if survey_route => {
                    // The code and data altitudes: one survey shell, and its
                    // own chart and furniture inside. The workspace's identity
                    // and epoch ride along so every altitude stamps the same
                    // cartouche facts.
                    let diff_line = format!(
                        "diff {} → {}",
                        graph.epoch.base, graph.epoch.target
                    );
                    rsx! {
                        SurveyShell { workspace: graph.name.clone(), diff_line }
                    }
                }
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

    /// A miniature shell: provides the review state the way [`AtlasShell`]
    /// does, and takes one trail step on mount.
    fn shell() -> Element {
        let mut atlas = use_context_provider(AtlasState::new);
        use_hook(move || atlas.trail.write().note(Some("alpha".to_string())));
        rsx! {}
    }

    // Why the state lives in context, not globals: every mounted app owns
    // its own review session, so a test can drive one shell's trail without
    // a second shell — or the next test — seeing it move.
    #[test]
    fn each_mounted_shell_owns_its_own_review_state() {
        let mut one = VirtualDom::new(shell);
        one.rebuild_in_place();
        let mut two = VirtualDom::new(shell);
        two.rebuild_in_place();

        // Walk the first shell a step further; the second must not move.
        one.in_scope(ScopeId::APP, || {
            let mut trail = consume_context::<AtlasState>().trail;
            trail.write().note(Some("beta".to_string()));
        });

        let walked = |vdom: &VirtualDom| {
            vdom.in_scope(ScopeId::APP, || {
                consume_context::<AtlasState>().trail.peek().walked()
            })
        };
        assert_eq!(walked(&one), vec!["alpha", "beta"]);
        assert_eq!(walked(&two), vec!["alpha"]);
    }

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

        // Navigate `/` → `/two/serde`, the shape of `/` → `/crate/serde`.
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
