//! The first altitude: the workspace's crates as concentric dependency rings.
//!
//! The first rung of the review ladder — the crates, then the data they keep.
//! The crate under review sits at the center (the workspace's root crate by
//! default) and every ring outward is one more dependency hop, so a change's
//! blast radius is a distance a reader can measure by eye. Stars never move.
//! Selecting one draws its edges — ink lines to what it depends on, hairlines
//! from what depends on it — and manifest events are always drawn, in flare.
//! Every selection is a URL, so the browser's back button retraces the review.

pub(crate) mod chrome;
pub(crate) mod layout;
pub(crate) mod map;
pub(crate) mod star;

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::Route;
use crate::graph::dep::DepGraph;
use crate::views::dep::chrome::{SearchBox, TitleBlock};
use crate::views::dep::map::Chart;
use crate::views::shell::use_graph;

/// One step of the review trail: `None` is the whole chart, `Some(step)` a
/// selection — crate names joined with `+`, or a whole ring as `ring:N`
/// (crate names can contain neither `+` nor `:`).
type TrailStep = Option<String>;

/// The ring index a trail step selects, if it is a ring step.
pub(super) fn step_ring(step: &str) -> Option<u32> {
    step.strip_prefix("ring:")?.parse().ok()
}

/// Which of the selection's edges the chart draws. Manifest events are
/// always drawn regardless. Defaults to dependencies only: the compact
/// reading; the other two readings are one toggle away.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum DirFilter {
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
pub(super) struct Trail {
    pub(super) steps: Vec<TrailStep>,
    /// Where in `steps` the current route sits.
    pub(super) at: usize,
}

impl Trail {
    /// Record a route change, telling a back/forward retrace apart from a
    /// new step by comparing against the recorded neighbors.
    pub(super) fn note(&mut self, step: TrailStep) {
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
    pub(super) fn current_focus(&self) -> Option<String> {
        self.steps.get(self.at).cloned().flatten()
    }

    /// The crates this stretch of the trail walked through since it last
    /// passed the whole chart, in visiting order — the review's breadcrumb.
    pub(super) fn walked(&self) -> Vec<String> {
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

/// This altitude's review state: the trail, which crates were visited, the
/// edge direction filter, and the screen-reader announcement line. Provided as
/// a context by the app shell — the one scope that survives every route change
/// — so the trail outlives the route-component remounts, and every app
/// instance (a test's `VirtualDom` included) owns its own copy.
#[derive(Clone, Copy)]
pub(super) struct DepState {
    pub(super) trail: Signal<Trail>,
    pub(super) visited: Signal<HashSet<String>>,
    pub(super) announce: Signal<String>,
    /// Which direction of the selection's edges the chart draws.
    pub(super) dir: Signal<DirFilter>,
    /// The current selection, materialized to crate names (a ring step
    /// resolves to every name on that ring; the overview resolves to the
    /// center). Written by the chart; read by modifier-clicks to toggle.
    pub(super) selected: Signal<Vec<String>>,
}

impl DepState {
    pub(super) fn new() -> Self {
        Self {
            trail: Signal::new(Trail::default()),
            visited: Signal::new(HashSet::new()),
            announce: Signal::new(String::new()),
            dir: Signal::new(DirFilter::default()),
            selected: Signal::new(Vec::new()),
        }
    }
}

pub(super) fn use_dep() -> DepState {
    use_context()
}

/// `/dep` — the whole chart, edges drawn for the center crate. The chart
/// lives in this altitude's shell and the rings caption their own hops, so
/// this route adds nothing: the overview is the chart, unobstructed.
#[component]
pub(crate) fn DepOverview() -> Element {
    rsx! {}
}

/// `/dep/crate/:name` — the selection's edges are drawn on the rings and this
/// route adds the panel: one crate's fact sheet, or the multi-selection
/// roster when names are joined with `+`.
#[component]
pub(crate) fn DepFocus(name: String) -> Element {
    let Some(graph) = use_graph() else {
        return rsx! {};
    };
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:items-start sm:p-3 sm:pt-[58px]",
            if name.contains('+') {
                chrome::MultiPanel { key: "{name}", graph: graph.clone(), joined: name }
            } else {
                chrome::FocusPanel { key: "{name}", graph: graph.clone(), name }
            }
        }
    }
}

/// `/dep/ring/:hop` — every crate on one ring selected; this route adds the
/// ring's roster panel.
#[component]
pub(crate) fn DepRing(hop: u32) -> Element {
    let Some(graph) = use_graph() else {
        return rsx! {};
    };
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:items-start sm:p-3 sm:pt-[58px]",
            chrome::RingPanel { key: "{hop}", graph: graph.clone(), hop }
        }
    }
}

/// The dependency chart and its furniture, with the review trail kept in step
/// with the URL. Mounted by the app shell, which has already loaded the
/// analysis this chart reads.
#[component]
pub(super) fn DepShell(graph: DepGraph) -> Element {
    let dep = use_dep();
    let route = use_route::<Route>();

    // Keep the trail in step with the URL. An effect, not a render-time
    // write: writes during the hydration render do not stick, which would
    // silently drop the trail's first step. The data altitude keeps its own
    // selection state; it never writes this trail.
    let step: Option<TrailStep> = match &route {
        Route::DepOverview {} => Some(None),
        Route::DepFocus { name } => Some(Some(name.clone())),
        Route::DepRing { hop } => Some(Some(format!("ring:{hop}"))),
        _ => None,
    };
    use_effect(use_reactive((&step,), move |(step,)| {
        let Some(step) = step else { return };
        let mut trail = dep.trail;
        if trail.peek().steps.get(trail.peek().at) == Some(&step) {
            return;
        }
        let mut visited = dep.visited;
        let mut announce = dep.announce;
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

    rsx! {
        div { class: "sr-only", role: "status", aria_live: "polite", "{dep.announce}" }
        Chart { graph: graph.clone() }
        Outlet::<Route> {}
        // Desktop: the left column is what this is and what changed. The chart
        // needs no key beside it — the rings caption their own hops, and every
        // mark carries its words on hover.
        div { class: "pointer-events-none absolute left-3 top-3 z-10 hidden w-64 sm:block",
            TitleBlock { graph: graph.clone() }
        }
        div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-56 sm:block",
            SearchBox { graph: graph.clone() }
        }
        // Phone: the title block stacks over search, its changes folded away.
        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
            TitleBlock { graph: graph.clone(), changes_open: false }
            SearchBox { graph: graph.clone() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A miniature shell: provides the review state the way [`DepShell`]'s own
    /// layout does, and takes one trail step on mount.
    fn shell() -> Element {
        let mut dep = use_context_provider(DepState::new);
        use_hook(move || dep.trail.write().note(Some("alpha".to_string())));
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
            let mut trail = consume_context::<DepState>().trail;
            trail.write().note(Some("beta".to_string()));
        });

        let walked = |vdom: &VirtualDom| {
            vdom.in_scope(ScopeId::APP, || {
                consume_context::<DepState>().trail.peek().walked()
            })
        };
        assert_eq!(walked(&one), vec!["alpha", "beta"]);
        assert_eq!(walked(&two), vec!["alpha"]);
    }
}
