//! The third altitude: the code that runs, seated inside whatever calls it.
//!
//! The crates, then the state, then the **functions**: every function, method,
//! trait clause and `macro_rules!` the workspace declares. It answers one
//! question — *what runs from where?* — and it answers it the way the rung
//! above answers *what holds this?*: by **containment**. One rung down the
//! commonest edge is the way-in call, so here **containment is the call**.
//! Every declaration seats inside the frame of the caller that reaches it
//! first, its own callees shelve in wrapped rows inside its frame, and ink is
//! spent only on what the shelving cannot say — the calls that are not the way
//! in. Reading the nesting is reading the way in.
//!
//! It is the data chart's dual, and deliberately so. A block there quotes a
//! struct's fields; a block here is one head row — the keyword, the name, the
//! diff letter — and the signature is quoted on the sheet, where it has the
//! room. Selecting a datum there lists what holds it; selecting a function
//! here lists what calls it, what it calls, and every type it touches, each of
//! those a link down to the rung that draws types.
//!
//! The seating is the **shelved section**, approved 2026-08-26 after two
//! prototypes were read and rejected. The band × prism section it replaced —
//! and the three seatings tried before that — are recorded in
//! `spec/function-viewer.md`.

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
use crate::views::func::chrome::{FnBandSheet, FnCartouche, FnSearch, FnSheet, FnTreeSheet};
use crate::views::func::map::FnChart;
use crate::views::func::model::FnModel;
use crate::views::func::quote::FnQuotation;

/// What the route selects on the chart.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum FnSel {
    /// One declaration: the file it is written in, then the label its sheet
    /// selects by.
    Mark(String, String),
    /// One frame's whole boundary, named by the declaration that owns it: the
    /// block and everything shelved inside it. Containment is the call, so a
    /// box is a subtree, and selecting the box is selecting the subtree.
    Tree(String, String),
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
            Route::FnTreeFocus { path, item } => Some(FnSel::Tree(path.join("/"), item.clone())),
            Route::FnModFocus { module } => Some(FnSel::Mod(module.clone())),
            Route::FnBandFocus { band } => Some(FnSel::Band(*band)),
            _ => None,
        }
    }

    /// The (file, label) of the declaration this selection is read from, where
    /// it is read from one — what a reveal unfolds the way in to.
    fn at(&self) -> Option<(&str, &str)> {
        match self {
            FnSel::Mark(path, label) | FnSel::Tree(path, label) => Some((path, label)),
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

/// The route that selects one frame's whole boundary — the declaration named
/// here and everything shelved inside it.
pub(super) fn tree_route(path: &str, item: &str) -> Route {
    Route::FnTreeFocus {
        path: path.split('/').map(str::to_string).collect(),
        item: item.to_string(),
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

/// In what order the callees on one shelf are seated.
///
/// The ground is the call tree, so a frame is a caller and its shelves are what
/// it runs. Nothing here is a *box* around anything — a second nesting system
/// fighting the call nesting was built once, as the retired per-type prism, and
/// it lost. What a declaration is written in and whose method it is are read as
/// the order its shelf seats in, and as words on its own head.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum FnOrder {
    /// Heaviest chain first: a callee that carries a hundred declarations
    /// under it seats before one that carries none.
    #[default]
    Weight,
    /// Siblings cluster by the module they are written in, so a frame reaching
    /// across the workspace says which parts of it.
    Module,
    /// Siblings cluster by the type or trait whose impl they are written in —
    /// a type's methods seat together — with the free declarations clustered
    /// ahead of them. Weight still orders inside a cluster.
    Owner,
}

impl FnOrder {
    pub(super) const ALL: [FnOrder; 3] = [FnOrder::Weight, FnOrder::Module, FnOrder::Owner];

    pub(super) fn label(self) -> &'static str {
        match self {
            FnOrder::Weight => "weight",
            FnOrder::Module => "module",
            FnOrder::Owner => "owner",
        }
    }

    pub(super) fn hint(self) -> &'static str {
        match self {
            FnOrder::Weight => "heaviest chain first on every shelf",
            FnOrder::Module => "siblings cluster by the module they are written in",
            FnOrder::Owner => {
                "siblings cluster by the type whose impl they are written in, free \
                 declarations first"
            }
        }
    }
}

/// Which calls the chart draws: **the direction it reads them in.**
///
/// A call that put a declaration where it sits is already said by the
/// shelving, so it takes no ink at all. What is left is every other resolved
/// call, and this reading says which way round to read them.
///
/// It is the data chart's `references` reading, one rung down, and it learned
/// the same lesson (2026-08-27, user: *"I don't understand the wires rest,
/// all, selection. I thought we should just show callers, callees, or
/// both?"*). The stops used to be `rest · all · selection` — three amounts of
/// ink, which is a word about the drawing rather than about the code, and the
/// reader has to hold a rule in their head to know what any of them will
/// show. `calls` and `callers` are the words a reviewer already has.
///
/// Direction only means something **against an anchor**: the same wire is one
/// declaration's call and another's caller, so `calls` and `callers` can pick
/// different wires only once the chart knows which mark the reader has in
/// hand. The anchor is never invented — it is whatever is in focus:
///
/// - the **selection**, whenever there is one: its own wires ink in the chosen
///   direction and stay inked;
/// - the **diff** on the resting plate — `calls` draws what the changed
///   declarations run, `callers` draws whose code runs them, which is the
///   blast-radius question a review brings to this chart;
/// - and where a workspace has neither, every wire the shelving cannot say is
///   drawn, because a reading with nothing in focus has no direction to take.
///
/// Hovering a mark is a fourth thing, and it is not a direction: it inks
/// *everything* that mark calls and everything that calls it, both ways round,
/// because what a reader hovers a block for is what the shelving could not
/// tell them. That ink lives in a layer of its own and never changes this one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum FnWires {
    /// What the anchor's own body runs — its wires out. The default: the
    /// question a reviewer brings to a change is what it reaches for.
    #[default]
    Calls,
    /// Whose code runs the anchor — its wires in.
    Callers,
    /// Both ways round, and — with no anchor at all — the whole unthinned
    /// family.
    Both,
}

impl FnWires {
    pub(super) const ALL_STOPS: [FnWires; 3] = [FnWires::Calls, FnWires::Callers, FnWires::Both];

    pub(super) fn label(self) -> &'static str {
        match self {
            FnWires::Calls => "calls",
            FnWires::Callers => "callers",
            FnWires::Both => "both",
        }
    }

    pub(super) fn hint(self) -> &'static str {
        match self {
            FnWires::Calls => {
                "what the mark in focus runs — with nothing selected, what the diff's own declarations run"
            }
            FnWires::Callers => {
                "whose code runs the mark in focus — with nothing selected, whose code runs the diff's own declarations"
            }
            FnWires::Both => {
                "both ways round; with nothing in focus, every call the shelving cannot say"
            }
        }
    }

    /// Whether this reading draws one wire anchored on `at`. `def` is the end
    /// being leaned on — the callee — and `user` the end that leans, which is
    /// the caller. The same shape, and the same three lines, as the data
    /// chart's [`crate::views::data::RefDir::draws`], because it is the same
    /// question about the other half of the language.
    pub(super) fn draws<T: PartialEq>(self, at: &T, def: &T, user: &T) -> bool {
        match self {
            FnWires::Calls => user == at,
            FnWires::Callers => def == at,
            FnWires::Both => user == at || def == at,
        }
    }
}

/// The frames the reviewer folded by hand, each named the way a fold has to
/// survive the next build: the file a declaration is written in, then the label
/// its own URL selects it by. A mark id is an index into one build and says
/// nothing across two.
pub(super) type FnFolds = std::collections::HashSet<(String, String)>;

/// One frame's name in a [`FnFolds`] set — the same pair `/fn/mark/:..path?item=`
/// carries, so a fold and a selection say one word for one declaration.
pub(super) fn fold_key(path: &str, label: &str) -> (String, String) {
    (path.to_string(), label.to_string())
}

/// The whole reading one build of the chart draws. The wires reading is not
/// here: it inks lines, never marks or seats, so switching it must not re-read
/// the survey. The order is, because it decides where every block sits — and so
/// are the folds, because a fold decides what is on the paper at all.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnReading {
    pub(super) vis_floor: VisFloor,
    pub(super) order: FnOrder,
    /// The frames whose contents are off the paper.
    pub(super) folds: FnFolds,
    /// The folds the paper was **packed** around — a subset of `folds`, and
    /// usually empty.
    ///
    /// A fold by hand does not re-lay the sheet (2026-08-27, user: *"when a
    /// thing is folded, try not to re-layout? because it just disrupts the
    /// visual anchor."*). Folding elides in place: what was inside the frame
    /// goes off the paper, the frame says how much in words, and its footprint
    /// stays reserved, so no sibling, ancestor or wire moves by a pixel. The
    /// reviewer's eye stays where they left it, which is the whole reason they
    /// folded something next to it.
    ///
    /// The compact packing still happens — but only where the paper is being
    /// laid again regardless, which means an `order` or `visibility` change, and
    /// a session's first build. Then, and only then, this set catches up with
    /// `folds` and the packer gets to skip what they hide. Opening a fold the
    /// packer *did* skip has to make room for it, so that one re-lays too, and
    /// the fold leaves both sets at once.
    pub(super) packed: FnFolds,
}

/// This altitude's own review-session state. Provided by the app shell, which
/// outlives every route change, so stepping between selections — or out to
/// another rung and back — never resets a reading or reopens a fold.
#[derive(Clone, Copy)]
pub(crate) struct FnState {
    pub(super) wires: Signal<FnWires>,
    pub(super) vis_floor: Signal<VisFloor>,
    pub(super) order: Signal<FnOrder>,
    /// The frames the reviewer folded by hand on this chart. Session state,
    /// kept like the camera and never in the URL: a fold is where the reviewer
    /// is looking, not what they are looking at.
    pub(super) folds: Signal<FnFolds>,
    /// Which of those folds the packer was allowed to skip — see
    /// [`FnReading::packed`].
    pub(super) packed: Signal<FnFolds>,
}

impl FnState {
    pub(crate) fn new() -> Self {
        Self {
            wires: Signal::new(FnWires::default()),
            vis_floor: Signal::new(VisFloor::default()),
            order: Signal::new(FnOrder::default()),
            folds: Signal::new(FnFolds::new()),
            packed: Signal::new(FnFolds::new()),
        }
    }

    /// The reading the chart draws right now. Every store is read, not peeked:
    /// a build that ignored one would go stale the moment the reviewer moved
    /// that control.
    pub(super) fn reading(&self) -> FnReading {
        FnReading {
            vis_floor: *self.vis_floor.read(),
            order: *self.order.read(),
            folds: self.folds.read().clone(),
            packed: self.packed.read().clone(),
        }
    }

    /// The paper is being laid again anyway, so the packer may as well pack
    /// around every fold that is open right now. Called when the `order` or the
    /// `visibility` reading moves — the two controls that move every block on
    /// the sheet — and never for a fold by hand.
    pub(super) fn repack(&self) {
        // **Peeked, never read.** This runs inside an effect keyed on the two
        // readings that lay the paper again; a tracked read of the fold set here
        // would subscribe that effect to the fold set, and every fold by hand
        // would immediately pack itself away — which is the one thing the
        // in-place elision exists to prevent. (It did, until it was measured.)
        let mut packed = self.packed;
        let folds = self.folds.peek().clone();
        if *packed.peek() != folds {
            packed.set(folds);
        }
    }

    /// One frame folded or opened by hand. Folding elides in place; opening a
    /// frame the packer skipped has to give its contents room again, which is
    /// the one fold gesture that re-lays the sheet.
    pub(super) fn fold(&self, keys: Vec<(String, String)>, shut: bool) {
        let (mut folds, mut packed) = (self.folds, self.packed);
        let mut open = folds.peek().clone();
        let mut skipped = packed.peek().clone();
        let mut moved = false;
        for key in keys {
            match shut {
                true => moved |= open.insert(key),
                false => {
                    moved |= open.remove(&key);
                    moved |= skipped.remove(&key);
                }
            }
        }
        if !moved {
            return;
        }
        folds.set(open);
        if *packed.peek() != skipped {
            packed.set(skipped);
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

/// `/fn/mod/:..module` — one module selected. The chart lights every mark
/// written in it, wherever the call tree seated them, and recedes the rest.
/// There is no sheet: the reading is the paper's own, and every lit head
/// already says its name.
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

/// `/fn/tree/:..path?:item` — one frame's whole boundary selected. Containment
/// is the call, so the box is a subtree: everything shelved inside keeps full
/// ink, everything one call across the line reads a step behind, and the sheet
/// lists what crosses the boundary in each direction.
#[component]
pub(crate) fn FnTreeFocus(path: Vec<String>, item: String) -> Element {
    let Some(graph) = crate::views::survey::use_survey() else {
        return rsx! {};
    };
    let joined = path.join("/");
    rsx! {
        div { class: "pointer-events-none absolute inset-x-3 bottom-12 top-auto z-10 flex items-end sm:inset-x-auto sm:bottom-0 sm:right-0 sm:top-14 sm:items-start sm:p-3 sm:pt-0",
            FnTreeSheet {
                key: "{joined}|{item}",
                graph,
                path: joined,
                item,
            }
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
        // A boundary is a focus of its own: the same pair of facts, read as the
        // whole subtree the box holds rather than as the one declaration.
        assert_eq!(
            tree_route("src/views/func/model.rs", "FnModel::build").to_string(),
            "/fn/tree/src/views/func/model.rs?item=FnModel::build"
        );
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
