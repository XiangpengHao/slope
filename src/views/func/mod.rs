//! The third altitude: the code that runs, tiered by how far it is from
//! something that starts.
//!
//! The crates, then the state, then the **functions**: every function, method,
//! trait clause and `macro_rules!` the workspace declares, seated by module and
//! tiered by call depth. It answers one question — *what runs from where?* — by
//! putting the declarations nothing calls at the top of the paper and drawing,
//! for every other mark, the one call that is the shortest way something
//! reaches it. Reading the way in is reading the paper.
//!
//! It is the data chart's dual, and deliberately so. A block there quotes a
//! struct's fields; a block here quotes a function's signature — the receiver,
//! the parameters, the return — because a function's parameters are its
//! fields, of the other half of the language. Selecting a datum there lists
//! what holds it; selecting a function here lists what calls it, what it
//! calls, and every type it touches, each of those a link down to the rung
//! that draws types.
//!
//! The sheet is a **section**: bands of call depth run its full width,
//! captioned at the left margin the way the dependency chart's rings caption
//! their hops, and prisms cross every band — one per module, or per type, or
//! per file, as the cartouche's `group` reading asks. A mark sits at the
//! crossing of its depth and its frame, so both readings come off the paper at
//! once. Two other seatings were built and read on a real workspace on
//! 2026-08-25 and cut by the user; `spec/function-viewer.md` records what they
//! were and why.

pub(crate) mod chrome;
pub(crate) mod layout;
pub(crate) mod map;
pub(crate) mod model;
pub(crate) mod quote;

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::CodeGraph;
use crate::views::chrome::plural;
use crate::views::data::VisFloor;
use crate::views::func::chrome::{FnBandSheet, FnCartouche, FnSearch, FnSheet};
use crate::views::func::map::FnChart;
use crate::views::func::model::FnModel;
use crate::views::func::quote::FnQuotation;

/// What the route selects on the chart.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum FnSel {
    /// One declaration: the file it is written in, then the label its sheet
    /// selects by.
    Mark(String, String),
    /// One module boundary: the crate — by its cargo package name — then the
    /// module path as rust nests it.
    Mod(Vec<String>),
    /// One whole band of the running order.
    Band(u32),
}

impl FnSel {
    /// The selection the current route asks for, or `None` where the route is
    /// not this chart's.
    fn of(route: &Route) -> Option<Self> {
        match route {
            Route::FnFocus { path, item, .. } => Some(FnSel::Mark(path.join("/"), item.clone())),
            Route::FnModFocus { module } => Some(FnSel::Mod(module.clone())),
            Route::FnBandFocus { band } => Some(FnSel::Band(*band)),
            _ => None,
        }
    }
}

/// The route that selects one declaration on the chart.
pub(super) fn mark_route(path: &str, item: &str) -> Route {
    Route::FnFocus {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
        peek: None,
    }
}

/// The route that selects one module boundary.
pub(super) fn mod_route(key: Vec<String>) -> Route {
    Route::FnModFocus { module: key }
}

/// The route that selects one whole band of the running order.
pub(super) fn band_route(band: u32) -> Route {
    Route::FnBandFocus { band }
}

/// The selection a sheet — and any quotation opened from it — hangs off.
pub(super) type Sel = (String, String);

/// Which row of a sheet is open as a quotation, in the URL: the file the
/// quoted item is written in, then its label, joined by a character neither a
/// path nor a rust name can contain.
pub(super) fn peek_key(path: &str, label: &str) -> String {
    format!("{path}@{label}")
}

/// The (file, label) a `peek=` names, or `None` where it names nothing.
pub(super) fn peek_at(key: &str) -> Option<(&str, &str)> {
    let (path, label) = key.split_once('@')?;
    (!path.is_empty() && !label.is_empty()).then_some((path, label))
}

/// The route that keeps the current selection and opens one of its rows as a
/// quotation. The selection never moves: a quotation is a reading of the
/// sheet, not a step to another mark.
pub(super) fn peek_route(sel: &Sel, path: &str, label: &str) -> Route {
    Route::FnFocus {
        path: sel.0.split('/').map(str::to_string).collect(),
        item: sel.1.clone(),
        peek: Some(peek_key(path, label)),
    }
}

/// What a frame on the paper is — how the marks are grouped inside the module
/// they are written in.
///
/// Two thirds of what runs is a *method*, and a method's first fact is whose it
/// is. Seating every declaration of a module in one shelf loses that: `keyword`
/// and `at_step` and `showing` are all `VisFloor`'s, and the paper should say
/// so. So the grouping is a reading of its own (2026-08-25, user), and it
/// nests **inside** the module frames rather than replacing them: a module is
/// where rust reads the code, and no grouping may overwrite that.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Group {
    /// Module frames alone: everything a module declares on one shelf.
    #[default]
    Module,
    /// A frame per type inside the module: a method sits with the type its
    /// impl names, and with the trait clause it answers. A free declaration
    /// stays on the module's own shelf, because nothing owns it.
    Owner,
    /// A frame per file inside the module — where the declaration is written,
    /// which is the grouping a reviewer reading a diff already has in hand.
    File,
}

impl Group {
    pub(super) const ALL: [Group; 3] = [Group::Module, Group::Owner, Group::File];

    pub(super) fn label(self) -> &'static str {
        match self {
            Group::Module => "module",
            Group::Owner => "type",
            Group::File => "file",
        }
    }

    pub(super) fn hint(self) -> &'static str {
        match self {
            Group::Module => "one shelf per module — everything it declares, together",
            Group::Owner => {
                "a frame per type: a method sits with the type its impl names, \
                 a free declaration on the module's own shelf"
            }
            Group::File => "a frame per file — where the declaration is written",
        }
    }
}

/// Which reading of the chart's calls is drawn, against whatever the reader
/// has in hand.
///
/// Direction only means something **against an anchor**: one hairline is this
/// function's call and that function's caller, so `calls` and `callers` can
/// pick different wires only once the chart knows which mark is in focus. The
/// anchor is never invented — the selection, the block under the cursor, or,
/// on the resting plate, the declarations the epoch touched.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum CallDir {
    /// What the anchor's own body reaches for — the code it runs.
    #[default]
    Calls,
    /// Whose code runs the anchor.
    Callers,
    /// Both ways round.
    Both,
}

impl CallDir {
    pub(super) fn label(self) -> &'static str {
        match self {
            CallDir::Calls => "calls",
            CallDir::Callers => "callers",
            CallDir::Both => "both",
        }
    }

    pub(super) fn hint(self) -> &'static str {
        match self {
            CallDir::Calls => "what the selection — or, at rest, what the diff — runs",
            CallDir::Callers => "whose code runs the selection — or, at rest, the diff",
            CallDir::Both => "both ways round",
        }
    }
}

/// The whole reading one build of the chart draws. The plate is not here: it
/// moves blocks, never marks or wires, so switching it must not re-read the
/// survey. The grouping is, because it changes what a frame *is*.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnReading {
    pub(super) calls: CallDir,
    pub(super) vis_floor: VisFloor,
    pub(super) group: Group,
}

/// This altitude's own review-session state. Provided by the app shell, which
/// outlives every route change, so stepping between selections — or out to
/// another rung and back — never resets a reading.
#[derive(Clone, Copy)]
pub(crate) struct FnState {
    pub(super) calls: Signal<CallDir>,
    pub(super) vis_floor: Signal<VisFloor>,
    pub(super) group: Signal<Group>,
}

impl FnState {
    pub(crate) fn new() -> Self {
        Self {
            calls: Signal::new(CallDir::default()),
            vis_floor: Signal::new(VisFloor::default()),
            group: Signal::new(Group::default()),
        }
    }

    /// The reading the chart draws right now. Every store is read, not peeked:
    /// a build that ignored one would go stale the moment the reviewer moved
    /// that control.
    pub(super) fn reading(&self) -> FnReading {
        FnReading {
            calls: *self.calls.read(),
            vis_floor: *self.vis_floor.read(),
            group: *self.group.read(),
        }
    }
}

/// This altitude's state, from the shell's context.
pub(super) fn use_fns() -> FnState {
    use_context::<FnState>()
}

/// `/fn` — the whole chart. The chart lives in the survey shell; this route
/// adds nothing else.
#[component]
pub(crate) fn FnOverview() -> Element {
    rsx! {}
}

/// `/fn/mod/:..module` — one module boundary selected. There is no sheet: a
/// module is a place on the paper, and the paper is already saying it.
#[component]
pub(crate) fn FnModFocus(module: Vec<String>) -> Element {
    let _ = module;
    rsx! {}
}

/// `/fn/depth/:band` — one whole band of the running order selected. The chart
/// lights the band and the way in to each of its marks; the sheet says which
/// depth it is and lists every declaration standing at it.
#[component]
pub(crate) fn FnBandFocus(band: u32) -> Element {
    let Some(graph) = crate::views::survey::use_survey() else {
        return rsx! {};
    };
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:bottom-0 sm:right-0 sm:top-14 sm:items-start sm:p-3 sm:pt-0",
            FnBandSheet { key: "{band}", graph, band }
        }
    }
}

/// `/fn/mark/:..path?:item` — one declaration selected, and optionally one of
/// its rows opened as a quotation. The chart keeps the blast radius inked; the
/// sheet says who calls it, what it calls, and every type it touches.
#[component]
pub(crate) fn FnFocus(path: Vec<String>, item: String, peek: Option<String>) -> Element {
    let Some(graph) = crate::views::survey::use_survey() else {
        return rsx! {};
    };
    let joined = path.join("/");
    let quoted = peek
        .as_deref()
        .and_then(peek_at)
        .map(|(at, label)| (at.to_string(), label.to_string()));
    rsx! {
        // The sheet stands clear of the search plate above it: both live in the
        // right column, and a sheet starting at the top edge has its own name
        // covered by the input.
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:bottom-0 sm:right-0 sm:top-14 sm:items-start sm:p-3 sm:pt-0",
            FnSheet {
                key: "{joined}|{item}",
                graph: graph.clone(),
                path: joined.clone(),
                item: item.clone(),
                peek,
            }
        }
        if let Some((at, label)) = quoted {
            div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-3 z-20 flex items-end sm:inset-x-auto sm:bottom-0 sm:right-[19.5rem] sm:top-14 sm:items-start sm:p-3 sm:pl-0 sm:pt-0",
                FnQuotation {
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

/// The function chart and its furniture, over the survey the gate loaded.
#[component]
pub(super) fn FnShell(graph: CodeGraph, workspace: String, diff_line: String) -> Element {
    let fns = use_fns();
    let route = use_route::<Route>();
    let sel = FnSel::of(&route);
    // What Escape closes first, while a row of the sheet is quoted: the same
    // selection with the quotation shut.
    let unquote = match &route {
        Route::FnFocus {
            path,
            item,
            peek: Some(_),
        } => Some(mark_route(&path.join("/"), item)),
        _ => None,
    };
    let facts = use_memo(use_reactive((&graph,), move |(graph,)| {
        FnModel::build(&graph, &fns.reading()).facts
    }));
    // The survey's own limits, for the cartouche's fold: the unresolved census
    // first, then the reference walk's notes — the ink this chart draws is the
    // reference walk's, so those are the limits that bear on it — and then the
    // two this altitude adds, which no other rung has to state.
    let limits: Vec<String> = {
        let mut notes = Vec::new();
        if graph.limits.unresolved > 0 {
            notes.push(format!(
                "{} the survey could not resolve.",
                plural(graph.limits.unresolved as usize, "name")
            ));
        }
        notes.extend(graph.limits.notes.iter().cloned());
        notes.push(
            "a method answering a foreign trait's contract — `Display`, \
             `Iterator` — reads as an entry point: std calls it, and the \
             survey charts the workspace's own code only"
                .to_string(),
        );
        notes.push(
            "the diff is per declaration, never per call: the survey reads \
             the base edition syntactically, so a rewritten body moves no \
             wire on this chart"
                .to_string(),
        );
        notes
    };

    rsx! {
        FnChart { graph: graph.clone(), sel, unquote }
        Outlet::<Route> {}
        div { class: "pointer-events-none absolute left-3 top-3 z-10 hidden w-64 sm:block",
            FnCartouche {
                facts: facts(),
                workspace: workspace.clone(),
                diff_line: diff_line.clone(),
                notes: limits.clone(),
            }
        }
        // Narrow viewports are a serviceable fallback, not a composition.
        div { class: "pointer-events-none absolute inset-x-3 top-3 z-10 flex flex-col gap-2 sm:hidden",
            FnCartouche {
                facts: facts(),
                workspace,
                diff_line,
                notes: limits,
            }
            FnSearch { graph: graph.clone() }
        }
        // Search top-right, the choreography every altitude keeps.
        div { class: "pointer-events-none absolute right-3 top-3 z-10 hidden w-72 flex-col gap-2 sm:flex",
            FnSearch { graph }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The URL is the review trail, so every focus this chart has must survive
    /// a round trip through it — a method's label, which carries its own `::`,
    /// included.
    #[test]
    fn every_focus_is_a_url() {
        assert_eq!(
            mark_route("src/analyze/code.rs", "survey").to_string(),
            "/fn/mark/src/analyze/code.rs?item=survey"
        );
        assert_eq!(
            mark_route("src/graph/data.rs", "CodeGraph::item").to_string(),
            "/fn/mark/src/graph/data.rs?item=CodeGraph::item"
        );
        assert_eq!(
            mod_route(vec!["slope-cli".to_string(), "views".to_string()]).to_string(),
            "/fn/mod/slope-cli/views"
        );
        assert_eq!(band_route(2).to_string(), "/fn/depth/2");
        let sel = ("src/main.rs".to_string(), "main".to_string());
        assert_eq!(
            peek_route(&sel, "src/analyze/code.rs", "survey").to_string(),
            "/fn/mark/src/main.rs?peek=src/analyze/code.rs@survey&item=main"
        );
    }

    /// Half a key names nothing: the sheet stays open and no plate does.
    #[test]
    fn a_key_that_names_no_pair_quotes_nothing() {
        assert_eq!(peek_at("src/main.rs"), None);
        assert_eq!(peek_at("@main"), None);
        assert_eq!(peek_at("src/main.rs@"), None);
        assert_eq!(
            peek_at(&peek_key("src/main.rs", "main")),
            Some(("src/main.rs", "main"))
        );
    }
}
