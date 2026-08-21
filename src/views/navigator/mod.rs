//! The navigator: the code graph read as a page of answers, never as a map.
//!
//! The fourth rung of the review ladder, and the first one with no chart on it.
//! The other three altitudes draw a picture of the workspace and let the reader
//! find their way around it; this one refuses to. Four rules hold it up:
//!
//! 1. **Position encodes relation, never location.** The focused mark stands in
//!    the middle, quoted whole. Left of it is what it depends on — first what
//!    its own surface names, then what only its body reaches. Right of it is
//!    what depends on it, in three bands kept strictly apart: what holds it,
//!    what names it in a signature, what only reads it in a body. Farther right
//!    is the reach, layer by layer. A module is a small label on a block, not a
//!    place on the paper.
//! 2. **The screen is always the answer to a question, never the graph.** The
//!    opening screen answers "what changed, and what does it reach?"; a focus
//!    answers "what is this, and what would a change here break?"; the board
//!    answers "how do these connect?".
//! 3. **Navigation is refocusing, and the trail is the map.** No camera: no
//!    pan, no zoom, nothing to fit. Every focus is a URL, so the browser's own
//!    back and forward retrace the review.
//! 4. **Pins accumulate the review.** The board keeps every pinned mark and
//!    threads the shortest path between each consecutive pair — the subgraph the
//!    reviewer walked away with, which is what a review actually produces.

pub(crate) mod layout;
pub(crate) mod model;
pub(crate) mod page;

use dioxus::prelude::*;

use crate::Route;
use crate::api::CodeGraph;
use crate::views::codemap::chrome::{Altitude, AltitudeSwitch};
use crate::views::navigator::layout::{Page, agenda_page, focus_page};
use crate::views::navigator::model::NavModel;

/// One mark, by the two things a URL selects it with: the file that declares it
/// and the label its declaration reads by.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MarkKey {
    pub path: String,
    pub label: String,
}

/// One step of the trail: the opening question, or one mark.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NavStep {
    /// The diff agenda — what changed, and what it reaches.
    Agenda,
    Focus(MarkKey),
}

/// The navigator's session state: the trail it walked and the board it kept.
/// Provided by the atlas shell, the one scope that outlives every route change,
/// so refocusing never forgets where the review has been.
#[derive(Clone, Copy)]
pub struct NavState {
    /// Every question asked this session, in order.
    pub trail: Signal<Vec<NavStep>>,
    /// Where in the trail the current URL stands.
    pub at: Signal<usize>,
    /// The marks this review is keeping.
    pub pins: Signal<Vec<MarkKey>>,
}

impl NavState {
    pub(crate) fn new() -> Self {
        NavState {
            trail: Signal::new(Vec::new()),
            at: Signal::new(0),
            pins: Signal::new(Vec::new()),
        }
    }

    /// Record where the URL now stands. A step already on the trail is a
    /// retrace — a trail chip, or the browser's own back button — and moves
    /// where we stand without dropping what comes after; anything else is a new
    /// question, and the questions after it were about somewhere else.
    pub fn note(self, step: NavStep) {
        let mut trail = self.trail;
        let mut at = self.at;
        if trail.peek().is_empty() {
            // The agenda is always the first question, whatever a deep link
            // asked for: without it there is no way back to the whole change.
            let mut first = vec![NavStep::Agenda];
            if step != NavStep::Agenda {
                first.push(step);
            }
            let last = first.len() - 1;
            trail.set(first);
            at.set(last);
            return;
        }
        let here = *at.peek();
        if trail.peek().get(here) == Some(&step) {
            return;
        }
        if let Some(found) = trail.peek().iter().position(|other| *other == step) {
            at.set(found);
            return;
        }
        let next = {
            let mut steps = trail.write();
            steps.truncate(here + 1);
            steps.push(step);
            steps.len() - 1
        };
        at.set(next);
    }

    /// Pin a mark to the board, or take it off again.
    pub fn toggle_pin(self, key: MarkKey) {
        let mut pins = self.pins;
        let found = pins.peek().iter().position(|other| *other == key);
        match found {
            Some(at) => {
                pins.write().remove(at);
            }
            None => pins.write().push(key),
        }
    }

    pub fn clear_pins(self) {
        let mut pins = self.pins;
        pins.set(Vec::new());
    }
}

pub fn use_nav_state() -> NavState {
    use_context()
}

/// The route that focuses one mark: the file that declares it, then its label.
/// The same shape `/surface/mark/:..path?:item` uses — one selection idiom
/// across the altitudes.
pub fn mark_route(path: &str, label: &str) -> Route {
    Route::NavigatorFocus {
        path: path.split('/').map(str::to_string).collect(),
        item: label.to_string(),
    }
}

/// Which question the current route asks.
fn route_step(route: &Route) -> NavStep {
    match route {
        Route::NavigatorFocus { path, item } => NavStep::Focus(MarkKey {
            path: path.join("/"),
            label: item.clone(),
        }),
        _ => NavStep::Agenda,
    }
}

/// `/navigator` — the diff agenda. The whole page depends on the route, so the
/// shell draws it and this route adds nothing else.
#[component]
pub fn NavigatorAgenda() -> Element {
    rsx! {}
}

/// `/navigator/mark/:..path?:item` — one mark in focus.
#[component]
pub fn NavigatorFocus(path: Vec<String>, item: String) -> Element {
    let _ = (path, item);
    rsx! {}
}

/// `/` opens the search, Escape asks the opening question again. Left and right
/// are the browser's own back and forward, installed once by the atlas shell:
/// every focus is a URL, so retracing needs nothing of its own.
const NAVIGATOR_KEYS_JS: &str = r#"
if (window.__slopeKeys) {
    document.removeEventListener('keydown', window.__slopeKeys);
}
window.__slopeKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const typing = tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable);
    if (e.key === '/' && !typing) {
        e.preventDefault();
        const s = document.getElementById('navigator-search');
        if (s) s.focus();
        return;
    }
    // Escape inside the search box closes the search; the box handles that
    // itself, and the page must not move under it.
    if (e.key === 'Escape' && !typing) dioxus.send('Escape');
};
document.addEventListener('keydown', window.__slopeKeys);
"#;

/// The navigator, mounted by the survey shell for every `/navigator` route.
#[component]
pub fn NavigatorShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let state = use_nav_state();
    let route = use_route::<Route>();
    let step = route_step(&route);
    let nav = use_navigator();

    // One reading of the survey, kept until the survey itself changes. Nothing
    // else moves it: this altitude has no toggles, because a toggle is a thing
    // the reader has to know about before the page answers them.
    let survey = use_memo(use_reactive((&graph,), |(graph,)| NavModel::build(&graph)));

    // The trail follows the URL, in an effect rather than at render time:
    // writes during the hydration render do not stick, and the first step is
    // the one that must never be dropped.
    use_effect(use_reactive((&step,), move |(step,)| state.note(step)));

    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(NAVIGATOR_KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                if key == "Escape" {
                    nav.push(Route::NavigatorAgenda {});
                }
            }
        });
    });

    // The page itself: measured and placed once per question.
    let placed: Memo<Option<Page>> =
        use_memo(use_reactive((&graph, &step), move |(graph, step)| {
            let model = survey.read();
            match &step {
                NavStep::Agenda => Some(agenda_page(&model)),
                NavStep::Focus(key) => model
                    .find(&key.path, &key.label)
                    .and_then(|id| focus_page(&model, &graph, id)),
            }
        }));

    let mut query = use_signal(String::new);
    let mut hot = use_signal(|| 0usize);
    let hits = use_memo(move || survey.read().search(&query()));

    let model = survey.read();
    let placed = placed.read();
    // The reading line above the page takes the page's own measure, so the two
    // share a left edge instead of each centering on its own width.
    let lede = match &*placed {
        Some(page) => format!("max-width:{:.0}px", page.size.0),
        None => String::new(),
    };
    let focused = match &step {
        NavStep::Focus(key) => model
            .find(&key.path, &key.label)
            .and_then(|id| model.item(id))
            .map(|item| item.name.clone()),
        NavStep::Agenda => None,
    };

    rsx! {
        div { class: "nav-shell",
            header { class: "nav-bar",
                div { class: "nav-brand",
                    "slope"
                    span { class: "rung", "{workspace}" }
                }
                // The ladder back to the charted altitudes. This page has no
                // cartouche to carry it, and a rung with no way off it is a
                // trap.
                div { class: "flex-none",
                    AltitudeSwitch { at: Altitude::Navigator }
                }
                {page::trail(&model, state)}
                div { class: "nav-searchwrap",
                    input {
                        id: "navigator-search",
                        class: "nav-search",
                        r#type: "search",
                        placeholder: "/ find a contract — fn, struct, enum …",
                        autocomplete: "off",
                        spellcheck: "false",
                        "aria-label": "Find a contract",
                        value: "{query}",
                        oninput: move |e| {
                            query.set(e.value());
                            hot.set(0);
                        },
                        onkeydown: move |e| {
                            let found = hits();
                            let n = found.len();
                            match e.key() {
                                Key::ArrowDown if n > 0 => {
                                    e.prevent_default();
                                    hot.set((hot() + 1).min(n - 1));
                                }
                                Key::ArrowUp if n > 0 => {
                                    e.prevent_default();
                                    hot.set(hot().saturating_sub(1));
                                }
                                Key::Enter => {
                                    let opening = found
                                        .get(hot().min(n.saturating_sub(1)))
                                        .and_then(|&id| {
                                            let model = survey.read();
                                            model.item(id).map(|it| mark_route(&it.path, &it.label))
                                        });
                                    if let Some(route) = opening {
                                        nav.push(route);
                                        query.set(String::new());
                                    }
                                }
                                Key::Escape => query.set(String::new()),
                                _ => {}
                            }
                        },
                    }
                    if !query().trim().is_empty() {
                        if hits().is_empty() {
                            ul { class: "nav-hits",
                                li { class: "nav-hit", "no mark of that name in this survey" }
                            }
                        } else {
                            ul { class: "nav-hits",
                                for (at , id) in hits().into_iter().enumerate() {
                                    if let Some(item) = model.item(id) {
                                        li {
                                            Link {
                                                to: mark_route(&item.path, &item.label),
                                                class: if at == hot() { "nav-hit hot" } else { "nav-hit" },
                                                onclick: move |_| query.set(String::new()),
                                                span { class: "hk", "{item.word()}" }
                                                span {
                                                    class: if item.is_sum() { "hn is-sum" } else { "hn" },
                                                    "{item.name}"
                                                }
                                                if let Some(letter) = item.letter() {
                                                    span { class: "nm-chg", "{letter}" }
                                                }
                                                span { class: "hm", "{item.module}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            main { class: "nav-main",
                match (&step, &focused) {
                    (NavStep::Agenda, _) => rsx! {
                        p { class: "nav-howto", style: "{lede}",
                            b { "the diff agenda" }
                            " — {diff_line}. center: every contract the diff touched, in its own words. "
                            b { "left" }
                            ": coupling the change took on, and gave back. "
                            b { "right" }
                            ": everything the change can reach that itself did not change — nearest first. "
                            "click anything to ask about it; "
                            b { "/" }
                            " searches; pins collect the marks worth keeping on the board."
                        }
                    },
                    (NavStep::Focus(_), Some(name)) => rsx! {
                        p { class: "nav-howto", style: "{lede}",
                            "reading: "
                            b { "left" }
                            " is what "
                            b { "{name}" }
                            " depends on · "
                            b { "right" }
                            " is what depends on it · "
                            b { "far right" }
                            " is everything a shape change here could reach, layer by layer. "
                            "click any block to refocus; the trail above remembers the way back."
                        }
                    },
                    (NavStep::Focus(_), None) => rsx! {},
                }
                match &*placed {
                    Some(page) => rsx! {
                        {page::canvas(&model, page, state)}
                    },
                    None => rsx! {
                        section { class: "plate mx-auto mt-8 max-w-md px-5 py-4",
                            h1 { class: "font-chart text-[15px] tracking-[0.18em] uppercase text-ink",
                                "No such mark"
                            }
                            p { class: "mt-2 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink-soft",
                                "This survey has no declaration at that address. It may have been renamed since the link was made."
                            }
                            Link {
                                class: "mt-3 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                                to: Route::NavigatorAgenda {},
                                "back to the diff"
                            }
                        }
                    },
                }
            }
            {page::board(&model, state)}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(label: &str) -> MarkKey {
        MarkKey {
            path: "src/api.rs".to_string(),
            label: label.to_string(),
        }
    }

    /// A shell of its own, the way the atlas shell provides the review state:
    /// every mounted app owns its own trail.
    fn walked() -> Element {
        let state = use_context_provider(NavState::new);
        use_hook(move || {
            state.note(NavStep::Focus(key("Wire")));
            state.note(NavStep::Focus(key("Tok")));
        });
        rsx! {}
    }

    fn bare() -> Element {
        use_context_provider(NavState::new);
        rsx! {}
    }

    /// The trail records new questions and retraces old ones: a step already on
    /// it moves where we stand instead of throwing away the way forward.
    #[test]
    fn the_trail_notes_new_steps_and_retraces_old_ones() {
        let mut vdom = VirtualDom::new(walked);
        vdom.rebuild_in_place();
        vdom.in_scope(ScopeId::APP, || {
            let state = consume_context::<NavState>();
            // A deep link still gets the agenda in front of it.
            assert_eq!(
                *state.trail.peek(),
                vec![
                    NavStep::Agenda,
                    NavStep::Focus(key("Wire")),
                    NavStep::Focus(key("Tok")),
                ]
            );
            assert_eq!(*state.at.peek(), 2);

            // Back to the first mark: we stand there, and the way forward stays.
            state.note(NavStep::Focus(key("Wire")));
            assert_eq!(*state.at.peek(), 1);
            assert_eq!(state.trail.peek().len(), 3);

            // A new question from there drops what the trail no longer explains.
            state.note(NavStep::Focus(key("Trail")));
            assert_eq!(*state.at.peek(), 2);
            assert_eq!(
                *state.trail.peek(),
                vec![
                    NavStep::Agenda,
                    NavStep::Focus(key("Wire")),
                    NavStep::Focus(key("Trail")),
                ]
            );
        });
    }

    /// A pin is a toggle, and the board can be emptied.
    #[test]
    fn pins_toggle_and_clear() {
        let mut vdom = VirtualDom::new(bare);
        vdom.rebuild_in_place();
        vdom.in_scope(ScopeId::APP, || {
            let state = consume_context::<NavState>();
            state.toggle_pin(key("Wire"));
            state.toggle_pin(key("Tok"));
            assert_eq!(state.pins.peek().len(), 2);
            state.toggle_pin(key("Wire"));
            assert_eq!(*state.pins.peek(), vec![key("Tok")]);
            state.clear_pins();
            assert!(state.pins.peek().is_empty());
        });
    }
}
