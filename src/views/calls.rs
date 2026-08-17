//! The call lens.
//!
//! A workspace's call graph has 900 functions and no mesh: average internal
//! degree below one, the busiest function six callers deep. Drawing *all* of it
//! is what the previous build proved does not work — selecting a function named
//! twelve related functions in the panel and put none of them legibly in the
//! picture.
//!
//! A focused subgraph is a different object. One function, its callers to the
//! left and its callees to the right, expanded a hop at a time, is exactly the
//! two-hop diagram the reviews judged the one picture that could earn its place
//! — and the aggregate answers a precomputed call graph gives, which an editor's
//! one-symbol-at-a-time call hierarchy structurally cannot, stay in the record
//! beside it.
//!
//! The guarantee is drawn, not just listed: the chain of chokepoints above a
//! function — everything *every* route to it must cross — is lit on the pane the
//! same way the dependency lens lights the chain that put a crate in the build.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::api::SheetLoad;
use crate::call::{Origin, Root, Sheet, Unit, UnitKind, reach};
use dioxus_flow::{
    Badge, Card, Edge, EdgeState, Flow, Folding, Graph as Scene, Node, NodeState, Port, Way,
    use_flow,
};
use crate::views::{Aim, Panel, SheetResource, WaitingWire};

/// How many chokepoints to put in front of a reader at once. The list is a
/// reading order, and a reading order nobody finishes is not one.
const CHOKEPOINTS: usize = 6;
/// How many beginnings to name per bucket before the rest go behind a
/// disclosure. Three or four a reader can hold; eighty-five is a filing cabinet.
const DOORS: usize = 4;

/// What the reader has opened in the call graph.
///
/// The same shape as the dependency lens's state, deliberately: a reader who has
/// learned one lens has learned the other. The one addition is `columns`, because
/// a call graph has no global rank to borrow — it can contain cycles — so depth
/// is assigned as the reader opens outward from wherever they started.
#[derive(Clone, Copy)]
pub struct SheetState {
    pub held: Signal<Option<usize>>,
    pub query: Signal<String>,
    /// What is open and what is folded, in the flow crate's own terms. The
    /// columns are this lens's own business; the folding is not.
    pub folding: Signal<Folding>,
    /// Hops from where the reading started: negative towards callers, positive
    /// towards callees. Assigned once per function, when it first arrives, so a
    /// cycle cannot walk a card back and forth across the pane.
    pub columns: Signal<HashMap<usize, i32>>,
    pub history: Signal<Vec<usize>>,
    /// What the camera has been asked to do about the selection, if anything.
    /// Cleared once used. The same signal, and the same rule, as the other lens:
    /// pointing at a card asks for nothing, naming a function does.
    pub aim: Signal<Option<Aim>>,
}

impl SheetState {
    /// Ask for a function **by name**, from the finder. What every route to it
    /// must cross is the answer, so the camera frames that chain rather than
    /// the function's own neighbourhood.
    pub fn reveal(&mut self, sheet: &Sheet, id: usize) {
        self.select(sheet, id);
        if !reach::analyse(sheet).spine_to(id).is_empty() {
            self.aim.set(Some(Aim::Route(id)));
        }
    }

    /// Ask for a function **by name**, from anywhere that is not the pane
    /// itself. One already on the pane is framed where it stands; one that is
    /// not arrives with the chain of chokepoints every route to it must cross.
    pub fn select(&mut self, sheet: &Sheet, id: usize) {
        if sheet.units.get(id).map(|unit| unit.kind) != Some(UnitKind::Function) {
            return;
        }
        let on_pane = visible(sheet, &(self.folding)()).contains(&id);
        if on_pane {
            self.aim.set(Some(Aim::Neighbourhood(id)));
        } else {
            self.route_to(sheet, id);
        }
        self.held.set(Some(id));
    }

    /// Hold a function the reader **pointed at** on the pane. The camera stays
    /// put, for the reason given on the other lens: they aimed at that card
    /// where it was.
    pub fn hold(&mut self, id: usize, sheet: &Sheet) {
        if sheet.units.get(id).map(|unit| unit.kind) != Some(UnitKind::Function) {
            return;
        }
        self.held.set(Some(id));
    }

    /// Start a fresh reading at this function, with everything above it that no
    /// route can avoid.
    pub fn route_to(&mut self, sheet: &Sheet, id: usize) {
        let spine = reach::analyse(sheet).spine_to(id);
        let mut columns: HashMap<usize, i32> = HashMap::new();
        let depth = spine.len() as i32;
        for (step, &unit) in spine.iter().enumerate() {
            columns.insert(unit, step as i32 - depth);
        }
        columns.insert(id, 0);

        let mut seeds = spine;
        seeds.push(id);
        self.folding.set(Folding::new(seeds));
        self.columns.set(columns);
        self.aim.set(Some(Aim::Route(id)));
        self.held.set(Some(id));
    }

    pub fn toggle(&mut self, id: usize, way: Way) {
        self.folding.write().toggle(id, way);
    }
}

/// Every function currently on the pane, and how deep each one sits.
///
/// Depth is assigned on arrival and never revised: a call graph has cycles, and
/// a column recomputed every frame would walk a card back and forth across the
/// pane while the reader looked at it.
pub fn placed(
    sheet: &Sheet,
    folding: &Folding,
    known: &HashMap<usize, i32>,
) -> Vec<(usize, i32)> {
    let seeds = folding.seeds();
    let mut column: HashMap<usize, i32> = HashMap::new();
    let mut queue: Vec<usize> = Vec::new();
    for (index, &seed) in seeds.iter().enumerate() {
        if sheet.units.get(seed).map(|unit| unit.kind) != Some(UnitKind::Function) {
            continue;
        }
        let depth = known
            .get(&seed)
            .copied()
            .unwrap_or(index as i32 - seeds.len() as i32 + 1);
        column.entry(seed).or_insert(depth);
        queue.push(seed);
    }
    // Breadth-first so a function reached two ways lands at the shallower depth.
    let mut head = 0;
    while head < queue.len() {
        let id = queue[head];
        head += 1;
        let here = column[&id];
        let unit = &sheet.units[id];
        if folding.is_open(id, Way::In) {
            for &caller in &unit.callers {
                if sheet.units[caller].kind == UnitKind::Function
                    && !column.contains_key(&caller)
                {
                    column.insert(caller, known.get(&caller).copied().unwrap_or(here - 1));
                    queue.push(caller);
                }
            }
        }
        if folding.is_open(id, Way::Out) {
            for &callee in &unit.calls {
                if sheet.units[callee].kind == UnitKind::Function
                    && !column.contains_key(&callee)
                {
                    column.insert(callee, known.get(&callee).copied().unwrap_or(here + 1));
                    queue.push(callee);
                }
            }
        }
    }
    let mut out: Vec<(usize, i32)> = column.into_iter().collect();
    out.sort_unstable();
    out
}

fn visible(sheet: &Sheet, folding: &Folding) -> Vec<usize> {
    placed(sheet, folding, &HashMap::new())
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

/// The cards and wires for one reading of the call graph.
pub fn scene(
    sheet: &Sheet,
    on_pane: &[(usize, i32)],
    held: Option<usize>,
    spine: &[usize],
    folding: &Folding,
) -> Scene {
    let here: HashMap<usize, i32> = on_pane.iter().copied().collect();
    let mut route: Vec<usize> = spine.to_vec();
    if let Some(id) = held {
        route.push(id);
    }
    let steps: HashSet<(usize, usize)> = route.windows(2).map(|p| (p[0], p[1])).collect();
    let on_route: HashSet<usize> = route.iter().copied().collect();

    let nodes: Vec<Node> = on_pane
        .iter()
        .map(|&(id, column)| {
            let unit = &sheet.units[id];
            Node {
                id,
                column,
                card: {
                    let mut card = Card::new(&unit.name).subtitle(subtitle(sheet, unit));
                    if let Some(name) = unit.trait_name.as_ref() {
                        card = card
                            .badge(Badge::new(name).titled(format!("{} implements {name}", unit.name)));
                    }
                    if unit.origin == Origin::Workspace {
                        card = card.filled();
                    }
                    card
                },
                inbound: (!unit.callers.is_empty()).then_some(Port {
                    count: unit.callers.len(),
                    open: folding.is_open(id, Way::In),
                }),
                outbound: (!unit.calls.is_empty()).then_some(Port {
                    count: unit.calls.len(),
                    open: folding.is_open(id, Way::Out),
                }),
                state: if held == Some(id) {
                    NodeState::Held
                } else if on_route.contains(&id) {
                    NodeState::OnRoute
                } else {
                    NodeState::Rest
                },
            }
        })
        .collect();

    let mut edges: Vec<Edge> = Vec::new();
    let mut drawn: HashSet<(usize, usize)> = HashSet::new();
    for &(id, _) in on_pane {
        for &callee in &sheet.units[id].calls {
            if !here.contains_key(&callee) || !drawn.insert((id, callee)) {
                continue;
            }
            let state = match held {
                _ if steps.contains(&(id, callee)) => EdgeState::Route,
                Some(held) if callee == held => EdgeState::Incoming,
                Some(held) if id == held => EdgeState::Outgoing,
                Some(_) => EdgeState::Muted,
                None => EdgeState::Rest,
            };
            edges.push(Edge {
                from: id,
                to: callee,
                state,
                label: None,
            });
        }
    }

    Scene { nodes, edges }
}

/// What the camera should have in frame when it lands on a function: the
/// function and everything attached to it that is actually on the pane.
pub fn frame_around(sheet: &Sheet, on_pane: &[usize], id: usize) -> Vec<usize> {
    let here: HashSet<usize> = on_pane.iter().copied().collect();
    let unit = &sheet.units[id];
    let mut frame = vec![id];
    frame.extend(
        unit.callers
            .iter()
            .chain(unit.calls.iter())
            .filter(|next| here.contains(next)),
    );
    frame
}

/// A function's second line: where it lives, which is what tells a reader
/// whether they can change it.
fn subtitle(sheet: &Sheet, unit: &Unit) -> String {
    match unit.parent.and_then(|parent| sheet.units.get(parent)) {
        Some(parent) if parent.kind == UnitKind::Impl || parent.kind == UnitKind::Type => {
            format!("{}::{}", unit.crate_name, parent.name)
        }
        _ => unit.crate_name.clone(),
    }
}

#[component]
pub fn Calls() -> Element {
    let resource: SheetResource = use_context();
    let mut state: SheetState = use_context();
    let mut flow = use_flow();

    let status = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Some(Ok(SheetLoad::Ready(_)))) => Status::Ready,
            Some(Some(Ok(SheetLoad::Failed(message)))) => Status::Failed(message.clone()),
            Some(Some(Err(error))) => Status::Failed(error.to_string()),
            // `Some(None)` is the moment the lens has been asked for and the
            // fetch has not started yet, which reads the same as indexing.
            Some(None) | None => Status::Indexing,
        }
    };

    // Open where the program starts. Nothing else in a call graph is a
    // defensible place to begin a reading.
    use_effect(move || {
        let loaded = resource.read();
        let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
            return;
        };
        if !state.folding.peek().seeds().is_empty() {
            return;
        }
        let Some(&first) = sheet.entries.first() else {
            return;
        };
        state.columns.set(HashMap::from([(first, 0)]));
        let mut folding = Folding::new(vec![first]);
        folding.open(first, Way::Out);
        state.folding.set(folding);
    });

    let (scene_now, record) = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Some(Ok(SheetLoad::Ready(sheet)))) => {
                let folding = (state.folding)();
                let on_pane = placed(sheet, &folding, &(state.columns)());
                let held = (state.held)();
                let analysis = reach::analyse(sheet);
                let spine: Vec<usize> = held
                    .map(|id| analysis.spine_to(id))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|step| on_pane.iter().any(|(id, _)| id == step))
                    .collect();
                let drawn = scene(sheet, &on_pane, held, &spine, &folding);
                let record = held.map(|id| CallRecord::build(sheet, &analysis, id));
                (drawn, record)
            }
            _ => (Scene::default(), None),
        }
    };

    // The camera moves only when something asked it to. Clicking a card is not
    // something asking; see the note on `Aim`.
    use_effect(move || {
        let Some(aim) = (state.aim)() else {
            return;
        };
        let loaded = resource.read();
        let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
            return;
        };
        let on_pane = visible(sheet.as_ref(), &(state.folding)());
        state.aim.set(None);
        match aim {
            // Framed as a chain, end to end — the same move the dependency lens
            // makes for the same action. Framing it on the held card instead is
            // what made one action behave two ways in two lenses.
            Aim::Route(id) => {
                let mut chain: Vec<usize> = reach::analyse(sheet.as_ref())
                    .spine_to(id)
                    .into_iter()
                    .filter(|step| on_pane.contains(step))
                    .collect();
                if chain.is_empty() {
                    flow.frame(id, frame_around(sheet.as_ref(), &on_pane, id));
                } else {
                    chain.push(id);
                    flow.route(chain);
                }
            }
            Aim::Neighbourhood(id) => {
                flow.frame(id, frame_around(sheet.as_ref(), &on_pane, id));
            }
        }
    });

    match status {
        Status::Indexing => {
            return rsx! {
                div { class: "flex h-full items-center justify-center px-6", Indexing {} }
            };
        }
        Status::Failed(message) => {
            return rsx! {
                div { class: "flex h-full items-center justify-center px-6", Failure { message } }
            };
        }
        Status::Ready => {}
    }

    // Pointing at a card holds it where it is; naming one moves the camera.
    let select = move |id: usize| {
        let loaded = resource.read();
        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
            state.hold(id, sheet.as_ref());
        }
    };

    rsx! {
        div { class: "flex h-full w-full min-h-0 flex-col overflow-hidden lg:flex-row",
            div { class: "relative flex min-h-0 min-w-0 flex-1",
                Flow {
                    flow,
                    graph: scene_now,
                    on_select: select,
                    // The camera stays out of a re-tidy here too; see the note
                    // on the other lens.
                    on_port: move |(id, way): (usize, Way)| state.toggle(id, way),
                    on_clear: move |_| state.held.set(None),
                }
                FunctionIndex {}
            }

            match record {
                Some(record) => rsx! { FunctionRecord { record } },
                None => rsx! { StartRecord {} },
            }
        }
    }
}

enum Status {
    Indexing,
    Ready,
    Failed(String),
}

/// One row in a ranked list: a function, and the one number that ranks it.
#[derive(Clone, PartialEq)]
pub struct CallRow {
    pub id: usize,
    pub name: String,
    pub where_: String,
    pub weight: usize,
    pub own: bool,
}

fn call_row(sheet: &Sheet, id: usize, weight: usize) -> CallRow {
    let unit = &sheet.units[id];
    CallRow {
        id,
        name: unit.name.clone(),
        where_: subtitle(sheet, unit),
        weight,
        own: unit.origin == Origin::Workspace,
    }
}

#[derive(Clone, PartialEq)]
pub struct CallRecord {
    id: usize,
    name: String,
    qualified: String,
    signature: Option<String>,
    file: String,
    line: u32,
    own: bool,
    /// Every chokepoint above it, outermost first: not *a* route but everything
    /// *every* route must cross.
    spine: Vec<CallRow>,
    callers: Vec<CallRow>,
    callees: Vec<CallRow>,
    reaches: usize,
    reached_by: usize,
    /// Functions that reach it *and* are reached by it. A call graph's two
    /// closures are not disjoint, so naming the overlap is the difference
    /// between two honest numbers and two that look like they should add up.
    both_ways: usize,
}

impl CallRecord {
    fn build(sheet: &Sheet, analysis: &reach::Reach, id: usize) -> Self {
        let unit = &sheet.units[id];
        let (callers, callees) = reach::immediate(sheet, id);
        let standing = analysis.of(id);
        Self {
            id,
            name: unit.name.clone(),
            qualified: unit.qualified.clone(),
            signature: unit.signature.clone(),
            file: unit.file.clone(),
            line: unit.line,
            own: unit.origin == Origin::Workspace,
            spine: analysis
                .spine_to(id)
                .into_iter()
                .map(|step| call_row(sheet, step, analysis.of(step).dominates))
                .collect(),
            callers: callers
                .iter()
                .map(|&caller| call_row(sheet, caller, sheet.units[caller].callers.len()))
                .collect(),
            callees: callees
                .iter()
                .map(|&callee| call_row(sheet, callee, sheet.units[callee].callers.len()))
                .collect(),
            reaches: standing.reaches,
            reached_by: standing.reached_by,
            both_ways: analysis.both_ways(sheet, id),
        }
    }
}

#[component]
fn FunctionRecord(record: CallRecord) -> Element {
    let mut state: SheetState = use_context();
    let resource: SheetResource = use_context();

    let mut open = move |id: usize| {
        let loaded = resource.read();
        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
            state.select(sheet.as_ref(), id);
        }
    };
    let target = record.id;

    rsx! {
        Panel { label: "Function".to_string(),
            div {
                class: "sr-only",
                "aria-live": "polite",
                "{record.name}. {record.reached_by} functions reach it. It reaches {record.reaches}."
            }

            header { class: "border-b border-line px-4 py-3.5",
                div { class: "flex items-start justify-between gap-2",
                    h2 { class: "min-w-0 flex-1 truncate text-[17px] font-semibold tracking-[-0.01em]",
                        "{record.name}"
                    }
                    if record.own {
                        span { class: "chip mt-1 shrink-0 bg-ink text-ink-invert", "WORKSPACE" }
                    }
                }
                p { class: "mt-1 truncate font-mono text-[11px] text-ink-muted", "{record.qualified}" }
                if let Some(signature) = record.signature.clone() {
                    p { class: "mt-1.5 font-mono text-[11px] break-words text-ink-faint select-text",
                        "{signature}"
                    }
                }
                p { class: "tabular mt-1.5 font-mono text-[11px] text-ink-faint select-text",
                    "{record.file}:{record.line}"
                }
            }

            // The guarantee, and the one thing clicking through an editor cannot
            // give you: not a route but everything every route must cross.
            section { class: "border-b border-line px-4 py-3.5",
                div { class: "flex items-baseline justify-between gap-2",
                    h3 { class: "label", "Every route crosses" }
                    if !record.spine.is_empty() {
                        button {
                            class: "action",
                            onclick: move |_| {
                                let loaded = resource.read();
                                if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                                    state.route_to(sheet.as_ref(), target);
                                }
                            },
                            "Draw this chain"
                        }
                    }
                }
                if record.spine.is_empty() {
                    p { class: "mt-1.5 text-[12px] text-ink-muted",
                        "Nothing stands above this function: it is a beginning."
                    }
                } else {
                    p { class: "mt-1.5 font-mono text-[11.5px] leading-relaxed break-words select-text",
                        for (index , step) in record.spine.iter().enumerate() {
                            span { key: "{step.id}",
                                if index > 0 {
                                    span { class: "text-ink-faint", " → " }
                                }
                                button {
                                    class: "rounded-[3px] px-0.5 text-ink-muted hover:bg-sunken hover:text-ink",
                                    onclick: {
                                        let id = step.id;
                                        move |_| open(id)
                                    },
                                    "{step.name}"
                                }
                            }
                        }
                        span { class: "text-ink-faint", " → " }
                        span { class: "font-semibold text-inbound", "{record.name}" }
                    }
                }
            }

            // The same shape the dependency record uses for the same job: a
            // heading, its total on the right, the exact buckets under it, then
            // the rows. Two records, one form.
            CallList {
                heading: "Called by".to_string(),
                empty: "Nothing in this workspace calls it.".to_string(),
                way: Way::In,
                total: record.reached_by,
                rows: record.callers.clone(),
            }
            CallList {
                heading: "Calls".to_string(),
                empty: "It calls nothing this analysis reached.".to_string(),
                way: Way::Out,
                total: record.reaches,
                rows: record.callees.clone(),
            }

            if record.both_ways > 0 {
                section { class: "border-b border-line px-4 py-3.5",
                    h3 { class: "flex items-baseline justify-between gap-2",
                        span { class: "label", "Counted on both sides" }
                        span { class: "tabular text-[15px] font-semibold", "{record.both_ways}" }
                    }
                    p { class: "mt-1 text-[12px] leading-relaxed text-ink-muted",
                        "A call graph can have cycles, so these functions reach this one "
                        span { class: "text-ink", "and" }
                        " are reached by it. The overlap is named rather than left to make two honest numbers look like they should add up."
                    }
                }
            }
        }
    }
}

#[component]
fn CallList(
    heading: String,
    empty: String,
    way: Way,
    /// Everything reachable this way, at any distance.
    total: usize,
    rows: Vec<CallRow>,
) -> Element {
    let mut state: SheetState = use_context();
    let resource: SheetResource = use_context();

    let (ink, rule) = match way {
        Way::In => ("text-inbound", "bg-inbound"),
        Way::Out => ("text-outbound", "bg-outbound"),
    };

    rsx! {
        section { class: "border-b border-line px-4 py-3.5",
            h3 { class: "flex items-center justify-between gap-2",
                span { class: "flex items-center gap-2",
                    span { class: "h-[2px] w-4 shrink-0 rounded-full {rule}" }
                    span { class: "label", "{heading}" }
                }
                span { class: "tabular text-[15px] font-semibold {ink}", "{total}" }
            }
            if rows.is_empty() && total == 0 {
                p { class: "mt-1.5 text-[12px] text-ink-muted", "{empty}" }
            } else {
                p { class: "tabular mt-0.5 text-[11px] text-ink-faint",
                    "{rows.len()} directly"
                    if total > rows.len() {
                        " · {total - rows.len()} further out"
                    }
                }
                ul { class: "mt-2 -mx-1.5 flex flex-col",
                    for entry in rows {
                        li { key: "{entry.id}",
                            button {
                                class: "flex w-full items-baseline gap-2 rounded-[5px] px-1.5 py-[5px] text-left hover:bg-sunken",
                                onclick: {
                                    let id = entry.id;
                                    move |_| {
                                        let loaded = resource.read();
                                        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                                            state.select(sheet.as_ref(), id);
                                        }
                                    }
                                },
                                span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                                span { class: "shrink-0 truncate font-mono text-[10.5px] text-ink-faint",
                                    "{entry.where_}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What a reader sees before they have chosen anything: where to start, and what
/// every route crosses. Both are aggregate answers a precomputed call graph can
/// give and an editor's call hierarchy cannot.
#[component]
fn StartRecord() -> Element {
    let resource: SheetResource = use_context();
    let mut state: SheetState = use_context();

    let loaded = resource.read();
    let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
        return rsx! { Panel { label: "How to read this".to_string() } };
    };
    let analysis = reach::analyse(sheet);

    let bucket = |root: Root| -> Vec<CallRow> {
        let mut ranked: Vec<usize> = sheet
            .units
            .iter()
            .filter(|unit| {
                unit.kind == UnitKind::Function
                    && unit.origin == Origin::Workspace
                    && unit.root == root
            })
            .map(|unit| unit.id)
            .collect();
        ranked.sort_by_key(|&id| std::cmp::Reverse(analysis.of(id).reaches));
        ranked
            .into_iter()
            .take(DOORS)
            .map(|id| call_row(sheet, id, analysis.of(id).reaches))
            .collect()
    };

    let chokepoints: Vec<CallRow> = analysis
        .chokepoints(sheet, CHOKEPOINTS)
        .into_iter()
        .map(|id| call_row(sheet, id, analysis.of(id).dominates))
        .collect();
    let heaviest = chokepoints.first().map(|row| row.weight).unwrap_or(1).max(1);
    // What "called by nothing" means is that no function in this workspace names
    // it — not that no entry point reaches it, which is a different and much
    // rarer fact. Reporting the second under the first's wording is how a real
    // blind spot came to read as zero.
    let uncalled = sheet
        .units
        .iter()
        .filter(|unit| {
            unit.kind == UnitKind::Function
                && unit.origin == Origin::Workspace
                && unit.callers.is_empty()
        })
        .count();

    let mut open = move |id: usize| {
        let loaded = resource.read();
        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
            state.select(sheet.as_ref(), id);
        }
    };

    rsx! {
        Panel { label: "How to read this".to_string(),
            header { class: "border-b border-line px-4 py-3.5",
                h2 { class: "text-[15px] font-semibold tracking-[-0.01em]",
                    "How to read {sheet.workspace}"
                }
                p { class: "mt-1 text-[12px] leading-relaxed text-ink-muted",
                    "{sheet.function_count} functions, {sheet.call_count} calls. Open one to see its callers and callees."
                }
            }

            section { class: "border-b border-line px-4 py-3.5",
                h3 { class: "label", "Where to start" }
                p { class: "mt-1 text-[11px] leading-relaxed text-ink-faint",
                    "Three different facts, named separately. Ranked by how much each reaches, because the size of a beginning is what makes it worth reading first."
                }
                for root in [Root::Main, Root::Api, Root::Detached] {
                    Doors { root, rows: bucket(root) }
                }
            }

            section { class: "border-b border-line px-4 py-3.5",
                h3 { class: "label", "What every route crosses" }
                p { class: "mt-1 text-[11px] leading-relaxed text-ink-faint",
                    "Functions no route to what is behind them can avoid. Fan-in counts popularity; this counts inevitability."
                }
                ul { class: "mt-2.5 -mx-1.5 flex flex-col gap-0.5",
                    for entry in chokepoints {
                        li { key: "{entry.id}",
                            button {
                                class: "flex w-full items-center gap-2 rounded-[5px] px-1.5 py-[5px] text-left hover:bg-sunken",
                                onclick: {
                                    let id = entry.id;
                                    move |_| open(id)
                                },
                                span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                                span { class: "h-1.5 w-16 shrink-0 overflow-hidden rounded-full bg-raised",
                                    span {
                                        class: "block h-full rounded-full bg-outbound",
                                        style: "width: {(entry.weight * 100).max(1) / heaviest}%",
                                    }
                                }
                                span { class: "tabular w-7 shrink-0 text-right text-[11px] text-ink-faint",
                                    title: "dominates {entry.weight} functions",
                                    "{entry.weight}"
                                }
                            }
                        }
                    }
                }
            }

            section { class: "px-4 py-3.5",
                h3 { class: "label", "The limit" }
                p { class: "mt-1.5 text-[12px] leading-relaxed text-ink-muted",
                    "A call edge exists only where one function names another. Trait objects, function pointers, macro-invoked code and framework callbacks leave no static edge. "
                    span { class: "text-ink", "{uncalled} of this workspace's own functions appear to be called by nothing — that is a statement about the analysis, not about the code." }
                }
                if !sheet.unopened.is_empty() {
                    p { class: "tabular mt-2 text-[11px] text-ink-faint",
                        "{sheet.unopened.len()} crates are called but their source was not read."
                    }
                }
            }
        }
    }
}

#[component]
fn Doors(root: Root, rows: Vec<CallRow>) -> Element {
    let mut state: SheetState = use_context();
    let resource: SheetResource = use_context();

    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "mt-2.5",
            p { class: "font-mono text-[10.5px] text-ink-faint", "{root.noun()}" }
            ul { class: "mt-1 -mx-1.5 flex flex-col",
                for entry in rows {
                    li { key: "{entry.id}",
                        button {
                            class: "flex w-full items-baseline gap-2 rounded-[5px] px-1.5 py-[5px] text-left hover:bg-sunken",
                            onclick: {
                                let id = entry.id;
                                move |_| {
                                    let loaded = resource.read();
                                    if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                                        state.select(sheet.as_ref(), id);
                                    }
                                }
                            },
                            span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                            span { class: "shrink-0 truncate font-mono text-[10.5px] text-ink-faint",
                                "{entry.where_}"
                            }
                            span {
                                class: "tabular w-8 shrink-0 text-right text-[11px] text-ink-faint",
                                title: "reaches {entry.weight} functions",
                                "{entry.weight}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Every function on the sheet, for assistive technology and in-page search.
#[component]
fn FunctionIndex() -> Element {
    let resource: SheetResource = use_context();
    let mut state: SheetState = use_context();

    let listing: Vec<(usize, String)> = {
        let loaded = resource.read();
        let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
            return rsx! {};
        };
        sheet
            .units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Function)
            .map(|unit| (unit.id, unit.qualified.clone()))
            .collect()
    };

    rsx! {
        ul { class: "sr-only", "aria-label": "Every function on this sheet",
            for (id , qualified) in listing {
                li { key: "{id}",
                    button {
                        onclick: move |_| {
                            let loaded = resource.read();
                            if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                                state.select(sheet.as_ref(), id);
                            }
                        },
                        "{qualified}"
                    }
                }
            }
        }
    }
}

#[component]
fn Indexing() -> Element {
    rsx! {
        div { class: "flex max-w-md flex-col items-center gap-4 text-center",
            WaitingWire {}
            p { class: "text-[13px] text-ink-muted",
                "Reading the call graph — rust-analyzer is indexing this workspace."
            }
            p { class: "text-[11.5px] text-ink-faint",
                "Around ten seconds the first time. The answer is then cached for as long as this server runs."
            }
        }
    }
}

#[component]
fn Failure(message: String) -> Element {
    rsx! {
        div { class: "max-w-2xl",
            h1 { class: "text-[15px] font-semibold", "The call graph could not be read" }
            p { class: "mt-1 text-[13px] text-ink-muted",
                "This lens runs rust-analyzer over the workspace. The analyser reported:"
            }
            pre { class: "plate mt-4 overflow-x-auto p-3 font-mono text-[12px] whitespace-pre-wrap select-text",
                "{message}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> Sheet {
        crate::call::extract::build().expect("the call sheet should build for this workspace")
    }

    fn a_function(sheet: &Sheet) -> usize {
        sheet
            .units
            .iter()
            .find(|unit| {
                unit.kind == UnitKind::Function
                    && unit.origin == Origin::Workspace
                    && !unit.calls.is_empty()
                    && !unit.callers.is_empty()
            })
            .expect("this workspace has a function that both calls and is called")
            .id
    }

    /// A reading opened both ways from one function.
    fn both_ways(id: usize) -> Folding {
        let mut folding = Folding::new(vec![id]);
        folding.open(id, Way::In);
        folding.open(id, Way::Out);
        folding
    }

    /// Depth is assigned on arrival and never revised, which is what keeps a
    /// cycle from walking a card back and forth across the pane.
    #[test]
    fn a_column_is_assigned_once_and_kept() {
        let sheet = sheet();
        let id = a_function(&sheet);
        let known = HashMap::from([(id, 0)]);

        let out = placed(&sheet, &both_ways(id), &known);
        let columns: HashMap<usize, i32> = out.iter().copied().collect();
        assert_eq!(columns[&id], 0);

        // Opening further out must not move anything already placed.
        let mut wider = both_ways(id);
        for &callee in &sheet.units[id].calls {
            wider.open(callee, Way::Out);
        }
        let deeper = placed(&sheet, &wider, &columns);
        for (unit, column) in &deeper {
            if let Some(before) = columns.get(unit) {
                assert_eq!(before, column, "a card moved column while the reader watched");
            }
        }
    }

    /// Callers sit left of what they call, callees right. The same law as the
    /// dependency lens, and every affordance leans on it.
    #[test]
    fn callers_sit_left_and_callees_right() {
        let sheet = sheet();
        let id = a_function(&sheet);
        let out = placed(&sheet, &both_ways(id), &HashMap::from([(id, 0)]));
        let columns: HashMap<usize, i32> = out.iter().copied().collect();
        for &caller in &sheet.units[id].callers {
            if let Some(&column) = columns.get(&caller) {
                assert!(column < 0, "a caller landed right of what it calls");
            }
        }
        for &callee in &sheet.units[id].calls {
            if let Some(&column) = columns.get(&callee) {
                assert!(column > 0, "a callee landed left of what calls it");
            }
        }
    }

    /// Only functions are drawn. Crates, files, types and impl blocks are how
    /// the sheet is organised, not things that call each other.
    #[test]
    fn nothing_but_functions_reaches_the_pane() {
        let sheet = sheet();
        let id = a_function(&sheet);
        let out = placed(&sheet, &both_ways(id), &HashMap::new());
        for (unit, _) in out {
            assert_eq!(sheet.units[unit].kind, UnitKind::Function);
        }
    }

    /// The guarantee is drawn: every step of the chain that all routes cross is
    /// lit, and the chain ends at the function being held.
    #[test]
    fn the_chain_every_route_crosses_is_lit_end_to_end() {
        let sheet = sheet();
        let analysis = reach::analyse(&sheet);
        let target = sheet
            .units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Function)
            .map(|unit| unit.id)
            .find(|&id| analysis.spine_to(id).len() >= 2)
            .expect("something in this workspace sits behind two chokepoints");

        let spine = analysis.spine_to(target);
        let mut seeds = spine.clone();
        seeds.push(target);
        let folding = Folding::new(seeds);
        let on_pane = placed(&sheet, &folding, &HashMap::new());
        let drawn = scene(&sheet, &on_pane, Some(target), &spine, &folding);

        let lit: HashSet<(usize, usize)> = drawn
            .edges
            .iter()
            .filter(|edge| edge.state == EdgeState::Route)
            .map(|edge| (edge.from, edge.to))
            .collect();
        let mut chain = spine;
        chain.push(target);
        for pair in chain.windows(2) {
            // A chokepoint chain is a dominator chain, not a call chain: two
            // consecutive chokepoints need not call each other directly.
            if sheet.units[pair[0]].calls.contains(&pair[1]) {
                assert!(
                    lit.contains(&(pair[0], pair[1])),
                    "{} -> {} is on the chain but is not lit",
                    sheet.units[pair[0]].name,
                    sheet.units[pair[1]].name
                );
            }
        }
    }

    /// Holding a function changes ink, never the cast.
    #[test]
    fn holding_a_function_never_changes_what_is_on_the_pane() {
        let sheet = sheet();
        let id = a_function(&sheet);
        let folding = both_ways(id);
        let on_pane = placed(&sheet, &folding, &HashMap::new());
        let cold = scene(&sheet, &on_pane, None, &[], &folding);
        let warm = scene(&sheet, &on_pane, Some(id), &[], &folding);
        assert_eq!(cold.nodes.len(), warm.nodes.len());
        assert_eq!(cold.edges.len(), warm.edges.len());
    }

    /// One card per function, however many ways it was reached.
    #[test]
    fn a_function_reached_twice_is_still_one_card() {
        let sheet = sheet();
        let id = a_function(&sheet);
        let mut folding = Folding::new(vec![id]);
        folding.open(id, Way::Out);
        for &callee in &sheet.units[id].calls {
            folding.open(callee, Way::Out);
        }
        let on_pane = placed(&sheet, &folding, &HashMap::new());
        let drawn = scene(&sheet, &on_pane, None, &[], &folding);
        let ids: HashSet<usize> = drawn.nodes.iter().map(|node| node.id).collect();
        assert_eq!(ids.len(), drawn.nodes.len());

        let wires: HashSet<(usize, usize)> = drawn
            .edges
            .iter()
            .map(|edge| (edge.from, edge.to))
            .collect();
        assert_eq!(wires.len(), drawn.edges.len(), "an edge is drawn twice");
    }
}
