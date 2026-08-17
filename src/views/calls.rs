//! The call lens: a program's call graph, read at a level of detail.
//!
//! The previous build of this lens drew functions and only functions, opened one
//! hop at a time from `main`. On the workspace it was written against that was
//! defensible. On a real one it is not, and the failure is not gradual — pointed
//! at liquid-cache, an 8,259-function graph with 23,092 calls in it, the whole
//! lens rendered **two cards and one wire**. `main` calls `launch`, the framework
//! takes over, and a walk that can only follow static call edges stops there. The
//! reader is left holding a torch in a haystack.
//!
//! What replaces it is the observation that a call graph is not flat. Every
//! function is in an impl block or a type, in a file, in a crate, and those are
//! real seams a Rust developer already thinks in rather than categories invented
//! for a drawing. So the pane draws the **frontier** of that hierarchy — for each
//! branch, the deepest container the reader has opened — and every call is drawn
//! between the two cards that hold its ends, gathered so one pair of cards is one
//! wire carrying a count.
//!
//! Folded all the way, liquid-cache is 82 crates and 518 wires: the whole
//! program's call structure, on one screen, with the crates it builds on the left
//! and `core` on the right. Open a crate and it becomes its files, and the wires
//! that landed on the crate re-aim at the file that actually answers them. Open a
//! file and it becomes its structs, enums, traits, impl blocks and free
//! functions. Nothing is hidden at any point — every call is still on the
//! drawing, it is just inside a card, and the card says how much.
//!
//! Two consequences worth naming, because they are why this shape and not
//! another:
//!
//! - **The cost is what the reader opened, not what the graph holds.** No
//!   virtualisation, no sampling, no cap that quietly drops edges. The 82-card
//!   view is 82 cards because the reader has not asked for more.
//! - **There is no walk to get lost in.** A reader who cannot find where
//!   execution enters — which is the honest situation in any framework-driven
//!   program — still sees the entire program at crate granularity on the first
//!   frame, and drills toward whatever they recognise.
//!
//! Columns come from [`dioxus_flow::rank`], which layers the condensation of the
//! drawn graph rather than the graph: two crates that call each other take the
//! same column, because saying either comes first would be a claim the code does
//! not support.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;

use crate::api::SheetLoad;
use crate::call::{Origin, Root, Sheet, Unit, UnitKind, reach};
use crate::views::{Aim, Panel, SheetResource, WaitingWire};
use dioxus_flow::{
    Badge, Bundle, Card, Edge, EdgeState, Flow, Graph as Scene, Inside, Nest, Node, NodeState,
    Port, Style, Way, use_flow,
};

/// How many chokepoints to put in front of a reader at once. The list is a
/// reading order, and a reading order nobody finishes is not one.
const CHOKEPOINTS: usize = 6;
/// How many beginnings to name per bucket before the rest go behind a count.
const DOORS: usize = 4;
/// How many rows a record's in/out list shows before it says how many more.
const ROWS: usize = 12;

/// Cards here carry a kind word, a count and sometimes a trait name, which is
/// more than a crate name and a version needed. The extra 18 pixels of height is
/// the difference between three facts fitting and the subtitle being clipped.
///
/// Wires are drawn only for what is held, the same reading the dependency lens
/// takes. This lens already refuses the whole graph — a unit opens into its
/// children, so what is on the pane is one level of detail rather than all 8,259
/// functions — but at 82 units it still drew 518 wires between them, and that
/// mesh is a texture nobody traces. Holding a card is what asks the question.
///
/// Lanes go with them: the room a lane holds open in every column a wire crosses
/// is only worth holding open for a wire that is on the pane.
fn style() -> Style {
    Style {
        node: (208.0, 58.0),
        column_pitch: 300.0,
        lanes: false,
        wires: dioxus_flow::Wires::OnHold,
        ..Style::default()
    }
}

/// What the reader has opened, and what they are holding.
///
/// One verb, deliberately. The dependency board's ports open *more of the graph*
/// because a crate holds nothing; here every card holds something, so the verb is
/// open *this card* and the ports state fan-in and fan-out rather than offering a
/// second, differently-shaped fold for the reader to keep straight.
#[derive(Clone, Copy)]
pub struct SheetState {
    pub held: Signal<Option<usize>>,
    pub query: Signal<String>,
    /// The level of detail, per branch of the unit tree.
    pub nest: Signal<Nest>,
    /// The trail, so Backspace retraces it.
    pub history: Signal<Vec<usize>>,
    /// What the camera has been asked to do about the selection, if anything.
    /// Cleared once used. The same rule as the other lens: pointing at a card
    /// asks for nothing, naming a unit does.
    pub aim: Signal<Option<Aim>>,
}

impl SheetState {
    /// Ask for a unit **by name**, from the finder or from a row in the record.
    ///
    /// It may be four levels inside a folded crate, so the containers above it
    /// are opened until it is on the pane. That is what "show me this" has to
    /// mean once the pane draws a level of detail rather than a fixed set.
    pub fn reveal(&mut self, sheet: &Sheet, id: usize) {
        if sheet.units.get(id).is_none() {
            return;
        }
        self.nest.write().reveal(sheet, id);
        self.held.set(Some(id));
        let spine = sheet.reach.spine_to(id);
        self.aim.set(Some(if spine.is_empty() {
            Aim::Neighbourhood(id)
        } else {
            Aim::Route(id)
        }));
    }

    /// Hold a card the reader **pointed at**. The camera stays put: they aimed
    /// at that card where it was, and taking the ground out from under the click
    /// removes what they aimed with.
    pub fn hold(&mut self, id: usize) {
        self.held.set(Some(id));
    }

    /// Open a card into what it holds, or fold it back up.
    pub fn open(&mut self, id: usize) {
        self.nest.write().toggle(id);
    }

    /// Fold a container back up, and hold it.
    ///
    /// The way back. A lid goes with the card it was on, so once a crate has
    /// become its files there is nothing left on the pane to click to undo it —
    /// which is why the record's breadcrumb is a control and not a label, and
    /// why the record of a container you have opened keeps offering to close it.
    pub fn close(&mut self, sheet: &Sheet, id: usize) {
        if sheet.units.get(id).is_none() {
            return;
        }
        self.nest.write().fold(id);
        self.held.set(Some(id));
        self.aim.set(Some(Aim::Neighbourhood(id)));
    }

    /// Back to the whole program at crate granularity.
    pub fn fold_all(&mut self) {
        self.nest.set(Nest::new());
        self.held.set(None);
    }
}

/// One reading of the call graph: the cards, their columns, and the wires.
///
/// Rebuilt when the level of detail changes and at no other time — holding a
/// card changes ink, never the cast, so a selection must not cost a layout.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Level {
    /// Card id and the column it sits in.
    pub cards: Vec<(usize, i32)>,
    pub wires: Vec<Bundle>,
    /// Calls whose two ends are inside the same card. Not drawn — there is
    /// nowhere to draw them — but stated on the card, because "1,400 of these
    /// calls never leave this crate" is the sort of thing a reader is looking
    /// for.
    pub within: HashMap<usize, usize>,
    /// Cards pointing at each card, and cards it points at.
    pub fan: HashMap<usize, (usize, usize)>,
}

impl Level {
    /// Everything on the pane.
    pub fn build(sheet: &Sheet, nest: &Nest) -> Self {
        let cards = nest.frontier(sheet);
        let bundles = nest.lift(sheet).bundle(&sheet.pairs());

        let mut within: HashMap<usize, usize> = HashMap::new();
        let mut fan: HashMap<usize, (usize, usize)> = HashMap::new();
        let mut wires: Vec<Bundle> = Vec::with_capacity(bundles.len());
        for bundle in bundles {
            if bundle.from == bundle.to {
                within.insert(bundle.from, bundle.weight);
                continue;
            }
            fan.entry(bundle.from).or_default().1 += 1;
            fan.entry(bundle.to).or_default().0 += 1;
            wires.push(bundle);
        }

        let pairs: Vec<(usize, usize)> = wires.iter().map(|w| (w.from, w.to)).collect();
        let columns = dioxus_flow::rank(&cards, &pairs);
        let cards = cards
            .into_iter()
            .map(|id| (id, columns.get(&id).copied().unwrap_or(0)))
            .collect();

        Self {
            cards,
            wires,
            within,
            fan,
        }
    }

    fn holds(&self, id: usize) -> bool {
        self.cards.iter().any(|&(card, _)| card == id)
    }

    /// The cards attached to this one, either way, plus itself. What the camera
    /// should have in frame when it lands.
    fn around(&self, id: usize) -> Vec<usize> {
        let mut out = vec![id];
        for wire in &self.wires {
            if wire.from == id {
                out.push(wire.to);
            } else if wire.to == id {
                out.push(wire.from);
            }
        }
        out
    }
}

/// The cards and wires, with the held card's neighbourhood lit.
pub fn scene(
    sheet: &Sheet,
    level: &Level,
    nest: &Nest,
    held: Option<usize>,
    route: &[usize],
) -> Scene {
    let steps: HashSet<(usize, usize)> = route.windows(2).map(|p| (p[0], p[1])).collect();
    let on_route: HashSet<usize> = route.iter().copied().collect();

    let nodes: Vec<Node> = level
        .cards
        .iter()
        .map(|&(id, column)| {
            let unit = &sheet.units[id];
            let (inward, outward) = level.fan.get(&id).copied().unwrap_or((0, 0));
            Node {
                id,
                column,
                card: card(sheet, unit),
                inbound: (inward > 0).then_some(Port::new(inward, false)),
                outbound: (outward > 0).then_some(Port::new(outward, false)),
                // A container states how many cards opening it produces, which
                // is the thing the click actually does. How much *code* is in
                // there is on the subtitle, where it informs the decision
                // without pretending to be its outcome.
                inside: (unit.kind.holds() && !unit.children.is_empty())
                    .then(|| Inside::new(unit.children.len(), nest.is_open(id))),
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

    let edges: Vec<Edge> = level
        .wires
        .iter()
        .map(|wire| {
            let state = match held {
                _ if steps.contains(&(wire.from, wire.to)) => EdgeState::Route,
                Some(held) if wire.to == held => EdgeState::Incoming,
                Some(held) if wire.from == held => EdgeState::Outgoing,
                Some(_) => EdgeState::Muted,
                None => EdgeState::Rest,
            };
            Edge {
                from: wire.from,
                to: wire.to,
                state,
                weight: wire.weight,
                // How many calls a wire stands for is written on it only where
                // the reader is looking. A number on all 4,283 of them is a
                // texture, and the width already says which are the heavy ones.
                label: (wire.weight > 1
                    && matches!(state, EdgeState::Incoming | EdgeState::Outgoing))
                .then(|| wire.weight.to_string()),
            }
        })
        .collect();

    Scene {
        nodes,
        edges,
        // The call lens drills rather than walks; it has no centre.
        root: None,
    }
}

/// What a card says. Every kind fills the same three slots, so a reader who has
/// learned one card has learned them all: what it is called, what sort of thing
/// it is and how much is in it, and — where there is one — the trait it speaks.
fn card(sheet: &Sheet, unit: &Unit) -> Card {
    let mut card = Card::new(title(unit)).subtitle(subtitle(sheet, unit));
    if let Some(name) = unit.trait_name.as_ref() {
        card = card.badge(Badge::new(short_trait(name)).titled(match unit.kind {
            UnitKind::Impl => format!(
                "{} implements {name}",
                unit.self_ty.as_deref().unwrap_or(&unit.name)
            ),
            _ => format!("{} is {name}'s", unit.name),
        }));
    } else if unit.origin == Origin::Std && unit.kind == UnitKind::Crate {
        // The one card whose origin `filled` cannot carry: std is neither the
        // reader's code nor a crate they chose.
        card = card.badge(Badge::new("std").titled("the standard library"));
    }
    if unit.origin == Origin::Workspace {
        card = card.filled();
    }
    card
}

/// An impl block is named for the type it is on; the trait it implements is the
/// badge beside it. `impl Display for Palette` as a title is a sentence, and a
/// column of sentences is unreadable.
fn title(unit: &Unit) -> String {
    match unit.kind {
        UnitKind::Impl => unit
            .self_ty
            .clone()
            .unwrap_or_else(|| unit.name.clone()),
        _ => unit.name.clone(),
    }
}

/// What sort of thing this is, and either how much code is in it or where it
/// lives — whichever the reader needs to decide what to do next.
fn subtitle(sheet: &Sheet, unit: &Unit) -> String {
    if !unit.kind.holds() {
        // Whatever holds it — the impl's type, the trait, or the file. A bare
        // crate name is the least useful of the four and was what this said
        // first, so a free function in a 43-file crate reported only the crate.
        let place = unit
            .parent
            .and_then(|parent| sheet.units.get(parent))
            .filter(|parent| parent.kind != UnitKind::Crate)
            .map(title)
            .unwrap_or_else(|| unit.crate_name.clone());
        return format!("fn · {place}");
    }
    let noun = unit.kind.noun();
    match unit.function_count {
        0 => noun.to_string(),
        1 => format!("{noun} · 1 fn"),
        many => format!("{noun} · {many} fn"),
    }
}

/// `From<Box<dyn Error>>` is the whole truth and does not fit on a card. The
/// head of it does, and the full name is on the badge's own tooltip.
fn short_trait(name: &str) -> String {
    let head = name.split('<').next().unwrap_or(name).trim();
    let head = head.rsplit("::").next().unwrap_or(head);
    if head.is_empty() { name.to_string() } else { head.to_string() }
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

    // The drawing. Keyed on the level of detail and the sheet, and on nothing
    // else — holding a card must not cost a layout.
    let level = use_memo(move || {
        let loaded = resource.read();
        let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
            return Level::default();
        };
        Level::build(sheet.as_ref(), &(state.nest)())
    });

    let (scene_now, record) = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Some(Ok(SheetLoad::Ready(sheet)))) => {
                let sheet = sheet.as_ref();
                let level = level.read();
                let nest = (state.nest)();
                // What is held may not be a card: the reader can open the card
                // they are holding, and it becomes its children. An opened
                // container keeps the record — that is where the control to
                // close it lives. Anything else off the pane resolves to the
                // card that now holds it.
                let held = (state.held)().and_then(|id| {
                    if level.holds(id) || nest.is_open(id) {
                        Some(id)
                    } else {
                        nest.project(sheet, id).first().copied()
                    }
                });
                let route = route_cards(sheet, &nest, &level, held);
                let drawn = scene(sheet, &level, &nest, held, &route);
                let record = held.map(|id| UnitRecord::build(sheet, &level, id));
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
        let sheet = sheet.as_ref();
        let nest = (state.nest)();
        let level = level.read();
        state.aim.set(None);
        let target = match aim {
            Aim::Route(id) | Aim::Neighbourhood(id) => id,
        };
        // Whatever card the reader's request landed on, now that the level of
        // detail has had its say about where it is drawn.
        let Some(&card) = nest.project(sheet, target).first() else {
            return;
        };
        match aim {
            Aim::Route(_) => {
                let chain = route_cards(sheet, &nest, &level, Some(card));
                if chain.len() > 1 {
                    flow.route(chain);
                } else {
                    flow.frame(card, level.around(card));
                }
            }
            Aim::Neighbourhood(_) => flow.frame(card, level.around(card)),
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

    rsx! {
        div { class: "flex h-full w-full min-h-0 flex-col overflow-hidden lg:flex-row",
            // `flow-dense` steps the resting wire back; see the note in
            // tailwind.css. This lens draws several times the wire the other
            // one does, because every call between two cards is on one.
            div { class: "flow-dense relative flex min-h-0 min-w-0 flex-1",
                Flow {
                    flow,
                    graph: scene_now,
                    style: style(),
                    on_select: move |id: usize| state.hold(id),
                    // The lid is the one verb here, and it does not move the
                    // camera: the cards glide to their new places and the reader
                    // keeps the one they clicked.
                    on_open: move |id: usize| state.open(id),
                    on_clear: move |_| state.held.set(None),
                }
            }

            match record {
                Some(record) => rsx! { UnitPanel { record } },
                None => rsx! { StartRecord {} },
            }
        }
    }
}

/// The next card along the calls, at whatever level of detail is on the pane.
///
/// What an arrow key means, and it means the same thing on a crate as on a
/// function: the heaviest thing calling into this, or the heaviest thing it
/// calls. Rebuilding the drawing to answer one keypress is a millisecond, and
/// the alternative — a second notion of "next" that only works on functions —
/// is a key that does different things depending on how far in the reader has
/// opened.
pub fn step(sheet: &Sheet, nest: &Nest, from: Option<usize>, way: Way) -> Option<usize> {
    let level = Level::build(sheet, nest);
    let Some(from) = from else {
        // Nothing held: start at the card holding whatever the sheet calls a
        // beginning, or at the first card on the pane.
        let entry = sheet.entries.first().copied();
        return entry
            .and_then(|id| nest.project(sheet, id).first().copied())
            .or_else(|| level.cards.first().map(|&(id, _)| id));
    };
    let card = nest.project(sheet, from).first().copied()?;
    level
        .wires
        .iter()
        .filter_map(|wire| match way {
            Way::In if wire.to == card => Some((wire.weight, wire.from)),
            Way::Out if wire.from == card => Some((wire.weight, wire.to)),
            _ => None,
        })
        .max()
        .map(|(_, next)| next)
}

/// The chain of chokepoints above a held function, as the cards those
/// chokepoints are currently drawn as.
///
/// Every step is projected through the level of detail, so the guarantee holds
/// at any depth: fold everything and the chain is the crates every route
/// crosses, open the crates and it becomes the files, and so on down to the
/// functions themselves.
fn route_cards(sheet: &Sheet, nest: &Nest, level: &Level, held: Option<usize>) -> Vec<usize> {
    let Some(held) = held else {
        return Vec::new();
    };
    if sheet.units.get(held).map(|unit| unit.kind) != Some(UnitKind::Function) {
        return Vec::new();
    }
    let mut chain: Vec<usize> = Vec::new();
    for step in sheet.reach.spine_to(held).into_iter().chain([held]) {
        let Some(&card) = nest.project(sheet, step).first() else {
            continue;
        };
        if chain.last() != Some(&card) && level.holds(card) {
            chain.push(card);
        }
    }
    chain
}

enum Status {
    Indexing,
    Ready,
    Failed(String),
}

/// One row in a list: a unit, where it is, and the one number that ranks it.
#[derive(Clone, PartialEq)]
pub struct CallRow {
    pub id: usize,
    pub name: String,
    pub where_: String,
    /// The full path, for where a bare name is not enough to tell two apart.
    pub path: String,
    pub weight: usize,
    pub own: bool,
}

fn row(sheet: &Sheet, id: usize, weight: usize) -> CallRow {
    let unit = &sheet.units[id];
    CallRow {
        id,
        name: title(unit),
        where_: match unit.kind {
            UnitKind::Crate => unit.kind.noun().to_string(),
            UnitKind::Function => subtitle(sheet, unit),
            _ => format!("{} · {}", unit.kind.noun(), unit.crate_name),
        },
        // A chain of bare names is ambiguous the moment a workspace has three
        // functions called `train_compressor`, which is the ordinary case. The
        // rows have a second column for this; a chain drawn as one line does
        // not, so it carries the full path where the reader can ask for it.
        path: unit.qualified.clone(),
        weight,
        own: unit.origin == Origin::Workspace,
    }
}

/// What one unit says about itself. Six kinds share one form, because they are
/// six kinds of the same thing — a piece of a program with calls crossing its
/// boundary — and the reader should not have to learn six panels.
#[derive(Clone, PartialEq)]
pub struct UnitRecord {
    id: usize,
    kind: UnitKind,
    name: String,
    qualified: String,
    signature: Option<String>,
    trait_name: Option<String>,
    file: String,
    line: u32,
    own: bool,
    origin: Origin,
    root: Root,
    /// Is this a card, or a container the reader has opened into its children?
    /// An opened container has no wires of its own, so it reports what it holds
    /// and offers the way back rather than lists that would all read zero.
    on_pane: bool,
    /// Which crate, file and type this sits in, outermost first.
    place: Vec<CallRow>,
    /// For a container: what is in it, by kind.
    holds: Vec<(UnitKind, usize)>,
    /// Calls that begin and end inside this unit.
    within: usize,
    /// What calls in, and what it calls out to. Functions for a function;
    /// whatever is on the pane for a container.
    callers: Vec<CallRow>,
    callees: Vec<CallRow>,
    caller_total: usize,
    callee_total: usize,
    /// Functions only: the whole reachable closure, and the overlap.
    standing: Option<reach::Standing>,
    /// Functions only: every chokepoint above it, outermost first.
    spine: Vec<CallRow>,
    /// Functions only, and only where nothing stands above it that every route
    /// must cross: one concrete way in. "Nothing is inevitable above this" is
    /// true and unsatisfying on its own; a chain a reader can follow is not.
    way_in: Vec<CallRow>,
    /// Containers only: the chokepoints inside it, which is where to start
    /// reading in there.
    inner_chokepoints: Vec<CallRow>,
}

impl UnitRecord {
    fn build(sheet: &Sheet, level: &Level, id: usize) -> Self {
        let unit = &sheet.units[id];

        let mut place: Vec<CallRow> = Vec::new();
        let mut above = unit.parent;
        while let Some(at) = above {
            place.push(row(sheet, at, 0));
            above = sheet.units[at].parent;
            if place.len() > 8 {
                break;
            }
        }
        place.reverse();

        let mut holds: HashMap<UnitKind, usize> = HashMap::new();
        for &child in &unit.children {
            *holds.entry(sheet.units[child].kind).or_insert(0) += 1;
        }
        let mut holds: Vec<(UnitKind, usize)> = holds.into_iter().collect();
        holds.sort_by_key(|&(kind, _)| kind);

        let (callers, callees, caller_total, callee_total) = if unit.kind == UnitKind::Function {
            let (callers, callees) = reach::immediate(sheet, id);
            (
                callers.iter().take(ROWS).map(|&c| row(sheet, c, sheet.units[c].callers.len())).collect(),
                callees.iter().take(ROWS).map(|&c| row(sheet, c, sheet.units[c].callers.len())).collect(),
                callers.len(),
                callees.len(),
            )
        } else {
            // A container's neighbours are whatever the pane is drawing, with
            // the number of calls each wire carries — which is the number the
            // reader is looking at on the wire itself.
            let mut inward: Vec<(usize, usize)> = Vec::new();
            let mut outward: Vec<(usize, usize)> = Vec::new();
            for wire in &level.wires {
                if wire.to == id {
                    inward.push((wire.from, wire.weight));
                } else if wire.from == id {
                    outward.push((wire.to, wire.weight));
                }
            }
            inward.sort_by_key(|&(other, weight)| (std::cmp::Reverse(weight), other));
            outward.sort_by_key(|&(other, weight)| (std::cmp::Reverse(weight), other));
            (
                inward.iter().take(ROWS).map(|&(o, w)| row(sheet, o, w)).collect(),
                outward.iter().take(ROWS).map(|&(o, w)| row(sheet, o, w)).collect(),
                inward.len(),
                outward.len(),
            )
        };

        Self {
            id,
            kind: unit.kind,
            name: title(unit),
            qualified: unit.qualified.clone(),
            signature: unit.signature.clone(),
            trait_name: unit.trait_name.clone(),
            file: unit.file.clone(),
            line: unit.line,
            own: unit.origin == Origin::Workspace,
            origin: unit.origin,
            root: unit.root,
            on_pane: level.holds(id),
            place,
            holds,
            within: level.within.get(&id).copied().unwrap_or(0),
            callers,
            callees,
            caller_total,
            callee_total,
            standing: (unit.kind == UnitKind::Function).then(|| reach::standing(sheet, id)),
            spine: if unit.kind == UnitKind::Function {
                sheet
                    .reach
                    .spine_to(id)
                    .into_iter()
                    .map(|step| row(sheet, step, sheet.reach.dominates(step)))
                    .collect()
            } else {
                Vec::new()
            },
            way_in: if unit.kind == UnitKind::Function && sheet.reach.spine_to(id).is_empty() {
                reach::route_to(sheet, id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|step| row(sheet, step, sheet.reach.dominates(step)))
                    .collect()
            } else {
                Vec::new()
            },
            inner_chokepoints: if unit.kind == UnitKind::Function {
                Vec::new()
            } else {
                sheet
                    .reach
                    .chokepoints_under(sheet, id, CHOKEPOINTS)
                    .into_iter()
                    .map(|step| row(sheet, step, sheet.reach.dominates(step)))
                    .collect()
            },
        }
    }
}

#[component]
fn UnitPanel(record: UnitRecord) -> Element {
    let mut state: SheetState = use_context();
    let resource: SheetResource = use_context();

    let mut open = move |id: usize| {
        let loaded = resource.read();
        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
            state.reveal(sheet.as_ref(), id);
        }
    };
    // Every step of the breadcrumb is a container the reader opened to get
    // here, so clicking one means go back up to it.
    let mut close = move |id: usize| {
        let loaded = resource.read();
        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
            state.close(sheet.as_ref(), id);
        }
    };
    let function = record.kind == UnitKind::Function;
    let on_pane = record.on_pane;
    let here = record.id;
    let origin = match record.origin {
        Origin::Workspace => "WORKSPACE",
        Origin::Dependency => "DEPENDENCY",
        Origin::Std => "STD",
    };
    let origin_chip = if record.own {
        "chip mt-1 shrink-0 bg-ink text-ink-invert"
    } else {
        "chip mt-1 shrink-0 bg-raised text-ink-muted"
    };

    rsx! {
        Panel { label: "{record.kind.noun()}".to_string(),
            div {
                class: "sr-only",
                "aria-live": "polite",
                "{record.name}, {record.kind.noun()}. {record.caller_total} call in, {record.callee_total} out."
            }

            header { class: "border-b border-line px-4 py-3.5",
                div { class: "flex items-start justify-between gap-2",
                    h2 { class: "min-w-0 flex-1 truncate text-[17px] font-semibold tracking-[-0.01em]",
                        "{record.name}"
                    }
                    span { class: "{origin_chip}", "{origin}" }
                }
                p { class: "mt-1 flex flex-wrap items-baseline gap-x-1.5 text-[11px] text-ink-muted",
                    span { class: "font-mono", "{record.kind.noun()}" }
                    if let Some(name) = record.trait_name.clone() {
                        span { class: "text-ink-faint", "·" }
                        span { class: "font-mono", "{name}" }
                    }
                }
                // A file's path *is* its qualified name, so printing both puts
                // the same string on the panel twice.
                if record.qualified != record.file {
                    p { class: "mt-1 truncate font-mono text-[11px] text-ink-muted", "{record.qualified}" }
                }
                if let Some(signature) = record.signature.clone() {
                    p { class: "mt-1.5 font-mono text-[11px] break-words text-ink-faint select-text",
                        "{signature}"
                    }
                }
                if !record.file.is_empty() {
                    p { class: "tabular mt-1.5 font-mono text-[11px] text-ink-faint select-text",
                        "{record.file}"
                        if record.line > 0 {
                            ":{record.line}"
                        }
                    }
                }
                if !record.place.is_empty() {
                    p { class: "mt-2 flex flex-wrap items-center gap-x-1 text-[11px]",
                        for (index , step) in record.place.iter().enumerate() {
                            span { key: "{step.id}",
                                if index > 0 {
                                    span { class: "text-ink-faint", " / " }
                                }
                                button {
                                    class: "rounded-[3px] px-0.5 text-ink-muted hover:bg-sunken hover:text-ink",
                                    title: "Fold {step.name} back up",
                                    onclick: {
                                        let id = step.id;
                                        move |_| close(id)
                                    },
                                    "{step.name}"
                                }
                            }
                        }
                    }
                }
            }

            // What is in here, for anything that holds something. This is the
            // answer to "what am I about to open" — and, once it is open, the
            // only place the way back lives, because a lid goes with its card.
            if !record.holds.is_empty() {
                section { class: "border-b border-line px-4 py-3.5",
                    div { class: "flex items-baseline justify-between gap-2",
                        h3 { class: "label",
                            if on_pane {
                                "What is in it"
                            } else {
                                "Open — showing what is in it"
                            }
                        }
                        button {
                            class: "action",
                            onclick: move |_| {
                                if on_pane {
                                    state.open(here)
                                } else {
                                    close(here)
                                }
                            },
                            if on_pane {
                                "Open it"
                            } else {
                                "Fold it back up"
                            }
                        }
                    }
                    ul { class: "tabular mt-1.5 flex flex-wrap gap-x-3 gap-y-1 text-[12px]",
                        for (kind , count) in record.holds.iter() {
                            li { key: "{kind:?}",
                                span { class: "font-semibold", "{count}" }
                                span { class: "text-ink-muted", " {kind.noun()}" }
                                if *count != 1 {
                                    span { class: "text-ink-muted", "s" }
                                }
                            }
                        }
                    }
                    if record.within > 0 {
                        p { class: "mt-2 text-[12px] leading-relaxed text-ink-muted",
                            span { class: "tabular font-semibold text-ink", "{record.within}" }
                            " calls begin and end inside it. Those are the ones the pane cannot draw — open it and they become wires."
                        }
                    }
                }
            }

            // A function's guarantee: not a route but everything every route
            // must cross.
            if function {
                section { class: "border-b border-line px-4 py-3.5",
                    h3 { class: "label", "Every route crosses" }
                    if record.spine.is_empty() {
                        p { class: "mt-1.5 text-[12px] text-ink-muted",
                            if record.root.is_root() {
                                "Nothing stands above this function: it is a beginning."
                            } else if record.way_in.len() > 1 {
                                "Nothing is unavoidable above this function — it is reachable more than one way. Here is the shortest:"
                            } else {
                                "No entry point reaches this function. It runs through a mechanism this analysis cannot see, or it does not run."
                            }
                        }
                        if record.way_in.len() > 1 {
                            p { class: "mt-1.5 font-mono text-[11.5px] leading-relaxed break-words select-text",
                                for step in record.way_in.iter().take(record.way_in.len() - 1) {
                                    span { key: "{step.id}",
                                        button {
                                            class: "rounded-[3px] px-0.5 text-ink-muted hover:bg-sunken hover:text-ink",
                                            title: "{step.path}",
                                            onclick: {
                                                let id = step.id;
                                                move |_| open(id)
                                            },
                                            "{step.name}"
                                        }
                                        span { class: "text-ink-faint", " → " }
                                    }
                                }
                                span { class: "font-semibold text-inbound", "{record.name}" }
                            }
                        }
                    } else {
                        p { class: "mt-1.5 font-mono text-[11.5px] leading-relaxed break-words select-text",
                            for step in record.spine.iter() {
                                span { key: "{step.id}",
                                    button {
                                        class: "rounded-[3px] px-0.5 text-ink-muted hover:bg-sunken hover:text-ink",
                                        onclick: {
                                            let id = step.id;
                                            move |_| open(id)
                                        },
                                        "{step.name}"
                                    }
                                    span { class: "text-ink-faint", " → " }
                                }
                            }
                            span { class: "font-semibold text-inbound", "{record.name}" }
                        }
                    }
                }
            }

            // An opened container has no wires of its own — its children carry
            // them now — so it says nothing about callers rather than reporting
            // two zeroes that would read as a fact about the code.
            if on_pane {
                UnitList {
                    heading: if function { "Called by".to_string() } else { "Called from".to_string() },
                    empty: "Nothing on this sheet calls into it.".to_string(),
                    way: Way::In,
                    shown: record.callers.len(),
                    total: record.caller_total,
                    weight_title: if function { "callers".to_string() } else { "calls".to_string() },
                    rows: record.callers.clone(),
                }
                UnitList {
                    heading: if function { "Calls".to_string() } else { "Calls out to".to_string() },
                    empty: "It calls nothing this analysis reached.".to_string(),
                    way: Way::Out,
                    shown: record.callees.len(),
                    total: record.callee_total,
                    weight_title: if function { "callers".to_string() } else { "calls".to_string() },
                    rows: record.callees.clone(),
                }
            }

            if let Some(standing) = record.standing {
                section { class: "border-b border-line px-4 py-3.5",
                    h3 { class: "label", "Everything it touches" }
                    dl { class: "tabular mt-1.5 grid grid-cols-3 gap-2 text-[12px]",
                        div {
                            dt { class: "text-ink-muted", "reaches" }
                            dd { class: "text-[15px] font-semibold text-outbound", "{standing.reaches}" }
                        }
                        div {
                            dt { class: "text-ink-muted", "reached by" }
                            dd { class: "text-[15px] font-semibold text-inbound", "{standing.reached_by}" }
                        }
                        div {
                            dt { class: "text-ink-muted", "both ways" }
                            dd { class: "text-[15px] font-semibold", "{standing.both_ways}" }
                        }
                    }
                    if standing.both_ways > 0 {
                        p { class: "mt-2 text-[12px] leading-relaxed text-ink-muted",
                            "A call graph can have cycles, so those "
                            span { class: "tabular text-ink", "{standing.both_ways}" }
                            " functions are on both lists. The overlap is named rather than left to make two honest numbers look like they should add up."
                        }
                    }
                }
            }

            if !record.inner_chokepoints.is_empty() {
                section { class: "border-b border-line px-4 py-3.5",
                    h3 { class: "label", "Where to start reading in here" }
                    p { class: "mt-1 text-[11px] leading-relaxed text-ink-faint",
                        "Functions inside it that no route to what is behind them can avoid."
                    }
                    Ranked { rows: record.inner_chokepoints.clone(), suffix: "dominates".to_string() }
                }
            }
        }
    }
}

#[component]
fn UnitList(
    heading: String,
    empty: String,
    way: Way,
    shown: usize,
    total: usize,
    weight_title: String,
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
            if rows.is_empty() {
                p { class: "mt-1.5 text-[12px] text-ink-muted", "{empty}" }
            } else {
                if total > shown {
                    p { class: "tabular mt-0.5 text-[11px] text-ink-faint",
                        "the {shown} biggest of {total}"
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
                                            state.reveal(sheet.as_ref(), id);
                                        }
                                    }
                                },
                                span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                                span { class: "shrink-0 truncate font-mono text-[10.5px] text-ink-faint",
                                    "{entry.where_}"
                                }
                                span {
                                    class: "tabular w-9 shrink-0 text-right text-[11px] text-ink-faint",
                                    title: "{entry.weight} {weight_title}",
                                    "{entry.weight}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A ranked list with a bar, for the two places one number orders the rows.
#[component]
fn Ranked(rows: Vec<CallRow>, suffix: String) -> Element {
    let mut state: SheetState = use_context();
    let resource: SheetResource = use_context();
    let heaviest = rows.first().map(|row| row.weight).unwrap_or(1).max(1);

    rsx! {
        ul { class: "mt-2.5 -mx-1.5 flex flex-col gap-0.5",
            for entry in rows {
                li { key: "{entry.id}",
                    button {
                        class: "flex w-full items-center gap-2 rounded-[5px] px-1.5 py-[5px] text-left hover:bg-sunken",
                        onclick: {
                            let id = entry.id;
                            move |_| {
                                let loaded = resource.read();
                                if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                                    state.reveal(sheet.as_ref(), id);
                                }
                            }
                        },
                        span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                        span { class: "h-1.5 w-16 shrink-0 overflow-hidden rounded-full bg-raised",
                            span {
                                class: "block h-full rounded-full bg-outbound",
                                style: "width: {(entry.weight * 100).max(1) / heaviest}%",
                            }
                        }
                        span {
                            class: "tabular w-8 shrink-0 text-right text-[11px] text-ink-faint",
                            title: "{suffix} {entry.weight} functions",
                            "{entry.weight}"
                        }
                    }
                }
            }
        }
    }
}

/// What a reader sees before they have chosen anything: how to read the picture
/// in front of them, and the two aggregate answers a precomputed call graph can
/// give that an editor's one-symbol-at-a-time call hierarchy cannot.
#[component]
fn StartRecord() -> Element {
    let resource: SheetResource = use_context();
    let mut state: SheetState = use_context();

    let loaded = resource.read();
    let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
        return rsx! { Panel { label: "How to read this".to_string() } };
    };
    let analysis = &sheet.reach;

    let bucket = |root: Root| -> (Vec<CallRow>, usize) {
        let all: Vec<usize> = sheet
            .units
            .iter()
            .filter(|unit| {
                unit.kind == UnitKind::Function
                    && unit.origin == Origin::Workspace
                    && unit.root == root
            })
            .map(|unit| unit.id)
            .collect();
        // Ranked by how much stands behind each, which is cheap — dominance is
        // already on the sheet — where measuring what each one *reaches* would
        // be a walk of the whole graph per candidate, hundreds of times over.
        let mut ranked = all.clone();
        ranked.sort_by_key(|&id| {
            (
                std::cmp::Reverse(analysis.dominates(id)),
                sheet.units[id].qualified.clone(),
            )
        });
        (
            ranked
                .into_iter()
                .take(DOORS)
                .map(|id| row(sheet, id, analysis.dominates(id)))
                .collect(),
            all.len(),
        )
    };

    let chokepoints: Vec<CallRow> = analysis
        .chokepoints(sheet, CHOKEPOINTS)
        .into_iter()
        .map(|id| row(sheet, id, analysis.dominates(id)))
        .collect();
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
    let crates = sheet.workspace_crates + sheet.dependency_crates;

    let mut open_all = move || {
        let loaded = resource.read();
        if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
            let mut nest = state.nest.write();
            for &root in &sheet.roots {
                if sheet.units[root].origin == Origin::Workspace {
                    nest.open(root);
                }
            }
        }
    };
    let opened = {
        let nest = (state.nest)();
        sheet.roots.iter().any(|&root| nest.is_open(root))
    };

    rsx! {
        Panel { label: "How to read this".to_string(),
            header { class: "border-b border-line px-4 py-3.5",
                h2 { class: "text-[15px] font-semibold tracking-[-0.01em]",
                    "How to read {sheet.workspace}"
                }
                p { class: "tabular mt-1 text-[12px] leading-relaxed text-ink-muted",
                    "{sheet.function_count} functions and {sheet.call_count} calls, in {crates} crates."
                }
                p { class: "mt-2 text-[12px] leading-relaxed text-ink-muted",
                    "Every card is one of those crates and every wire is every call between two of them, drawn thicker the more it carries. Open a card and it becomes its files; open a file and it becomes its types, traits and functions. Nothing is hidden at any level — it is inside a card, and the card says how much."
                }
                div { class: "mt-2.5 flex flex-wrap items-center gap-3",
                    button {
                        class: "action",
                        onclick: move |_| open_all(),
                        "Open this workspace's own crates"
                    }
                    if opened {
                        button {
                            class: "action",
                            onclick: move |_| state.fold_all(),
                            "Fold everything back"
                        }
                    }
                }
            }

            // The honest answer to "I followed main and got as far as launch".
            section { class: "border-b border-line px-4 py-3.5",
                h3 { class: "label", "Why not just follow main" }
                p { class: "mt-1.5 text-[12px] leading-relaxed text-ink-muted",
                    "A call edge exists only where one function names another. A program that hands control to a framework — a runtime, a router, a component tree — leaves no static edge at the handover, so a walk outward from "
                    span { class: "font-mono text-ink", "main" }
                    " stops at the first one and reports a program two functions deep. "
                    span { class: "text-ink", "{uncalled} of this workspace's own functions have no caller this analysis can see" }
                    " — that is a statement about static analysis, not about the code. The map above does not depend on the walk."
                }
            }

            section { class: "border-b border-line px-4 py-3.5",
                h3 { class: "label", "Where execution can start" }
                p { class: "mt-1 text-[11px] leading-relaxed text-ink-faint",
                    "Three different facts, named separately. Ranked by how much of the program stands behind each."
                }
                for root in [Root::Main, Root::Api, Root::Detached] {
                    Doors { root, bucket: bucket(root) }
                }
            }

            section { class: "border-b border-line px-4 py-3.5",
                h3 { class: "label", "What every route crosses" }
                p { class: "mt-1 text-[11px] leading-relaxed text-ink-faint",
                    "Functions no route to what is behind them can avoid. Fan-in counts popularity; this counts inevitability."
                }
                Ranked { rows: chokepoints, suffix: "dominates".to_string() }
            }

            section { class: "px-4 py-3.5",
                h3 { class: "label", "The limit" }
                p { class: "mt-1.5 text-[12px] leading-relaxed text-ink-muted",
                    "Trait objects, function pointers, macro-invoked code and framework callbacks leave no static edge, so this is a floor on what runs rather than the whole of it."
                }
                if !sheet.unopened.is_empty() {
                    p { class: "tabular mt-2 text-[11px] text-ink-faint",
                        "{sheet.unopened.len()} crates are called but their source was not read."
                    }
                }
                if sheet.tests_excluded > 0 {
                    p { class: "tabular mt-1 text-[11px] text-ink-faint",
                        "{sheet.tests_excluded} test functions are left out: the call graph is of the program."
                    }
                }
            }
        }
    }
}

#[component]
fn Doors(root: Root, bucket: (Vec<CallRow>, usize)) -> Element {
    let mut state: SheetState = use_context();
    let resource: SheetResource = use_context();
    let (rows, total) = bucket;

    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "mt-2.5",
            p { class: "tabular flex items-baseline justify-between font-mono text-[10.5px] text-ink-faint",
                span { "{root.noun()}" }
                span { "{total}" }
            }
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
                                        state.reveal(sheet.as_ref(), id);
                                    }
                                }
                            },
                            span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                            span { class: "shrink-0 truncate font-mono text-[10.5px] text-ink-faint",
                                "{entry.where_}"
                            }
                            span {
                                class: "tabular w-8 shrink-0 text-right text-[11px] text-ink-faint",
                                title: "dominates {entry.weight} functions",
                                "{entry.weight}"
                            }
                        }
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
                "Seconds on a small crate, a minute on a large one. The answer is then cached for as long as this server runs."
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

    /// The first frame: one card per crate, and every call in the program
    /// accounted for on a wire or inside a card.
    #[test]
    fn the_opening_view_is_the_whole_program_at_crate_granularity() {
        let sheet = sheet();
        let level = Level::build(&sheet, &Nest::new());

        assert_eq!(level.cards.len(), sheet.roots.len());
        assert!(
            level.cards.iter().all(|&(id, _)| sheet.units[id].kind == UnitKind::Crate),
            "the folded sheet is crates"
        );
        let drawn: usize = level.wires.iter().map(|wire| wire.weight).sum();
        let held: usize = level.within.values().sum();
        assert_eq!(
            drawn + held,
            sheet.call_count,
            "every call is either a wire or inside a card"
        );
        assert!(drawn > 0 && held > 0, "this workspace calls across crates and within them");
    }

    /// Opening a card never loses a call and never leaves a wire pointing at
    /// something that is not drawn. Checked at every level of the tree.
    #[test]
    fn every_call_is_accounted_for_at_every_level() {
        let sheet = sheet();
        for depth in 0..5 {
            let level = Level::build(&sheet, &Nest::to_depth(&sheet, depth));
            let on_pane: HashSet<usize> = level.cards.iter().map(|&(id, _)| id).collect();
            let total: usize = level.wires.iter().map(|w| w.weight).sum::<usize>()
                + level.within.values().sum::<usize>();
            assert_eq!(total, sheet.call_count, "calls went missing at depth {depth}");
            for wire in &level.wires {
                assert!(
                    on_pane.contains(&wire.from) && on_pane.contains(&wire.to),
                    "a wire at depth {depth} lands on a card that is not on the pane"
                );
            }
        }
    }

    /// Columns are a property of the drawing, so every wire between two
    /// different components runs left to right. A call graph has cycles in it,
    /// which is exactly why this has to be checked rather than assumed.
    #[test]
    fn wires_run_forwards_unless_the_two_cards_call_each_other() {
        let sheet = sheet();
        let level = Level::build(&sheet, &Nest::new());
        let column: HashMap<usize, i32> = level.cards.iter().copied().collect();
        for wire in &level.wires {
            let (from, to) = (column[&wire.from], column[&wire.to]);
            assert!(
                from < to || from == to,
                "{} → {} runs backwards",
                sheet.units[wire.from].name,
                sheet.units[wire.to].name
            );
        }
    }

    /// Opening one card leaves every other card exactly where it was in the
    /// order, so the reader's place survives the change of detail.
    #[test]
    fn opening_a_card_replaces_it_and_disturbs_nothing_else() {
        let sheet = sheet();
        let before = Level::build(&sheet, &Nest::new());
        let target = before
            .cards
            .iter()
            .map(|&(id, _)| id)
            .find(|&id| sheet.units[id].children.len() > 1)
            .expect("some crate has more than one file in it");

        let mut nest = Nest::new();
        nest.open(target);
        let after = Level::build(&sheet, &nest);

        let gone: HashSet<usize> = after.cards.iter().map(|&(id, _)| id).collect();
        for &(id, _) in &before.cards {
            if id == target {
                assert!(!gone.contains(&id), "the opened card is replaced by what it holds");
            } else {
                assert!(gone.contains(&id), "{} left the pane and should not have", sheet.units[id].name);
            }
        }
        assert_eq!(
            after.cards.len(),
            before.cards.len() - 1 + sheet.units[target].children.len()
        );
    }

    /// Holding a card changes ink, never the cast.
    #[test]
    fn holding_a_card_never_changes_what_is_on_the_pane() {
        let sheet = sheet();
        let nest = Nest::new();
        let level = Level::build(&sheet, &nest);
        let id = level.cards[0].0;
        let cold = scene(&sheet, &level, &nest, None, &[]);
        let warm = scene(&sheet, &level, &nest, Some(id), &[]);
        assert_eq!(cold.nodes.len(), warm.nodes.len());
        assert_eq!(cold.edges.len(), warm.edges.len());
    }

    /// A weighted wire says how many calls it stands for, and the wires attached
    /// to what is held say it out loud.
    #[test]
    fn a_wire_carries_the_calls_it_gathered() {
        let sheet = sheet();
        let nest = Nest::new();
        let level = Level::build(&sheet, &nest);
        let heavy = level
            .wires
            .iter()
            .max_by_key(|wire| wire.weight)
            .expect("something calls across a crate boundary more than once");
        assert!(heavy.weight > 1, "crates call each other more than once");

        let drawn = scene(&sheet, &level, &nest, Some(heavy.from), &[]);
        let edge = drawn
            .edges
            .iter()
            .find(|edge| (edge.from, edge.to) == (heavy.from, heavy.to))
            .expect("the wire is on the pane");
        assert_eq!(edge.weight, heavy.weight);
        assert_eq!(edge.label.as_deref(), Some(heavy.weight.to_string().as_str()));
        // And a wire attached to nothing held carries no label, or 500 numbers
        // land on the picture at once.
        assert!(
            drawn
                .edges
                .iter()
                .filter(|edge| edge.state == EdgeState::Muted)
                .all(|edge| edge.label.is_none())
        );
    }

    /// Every card that holds something offers to open it, and the count it
    /// offers is what actually arrives.
    #[test]
    fn a_lid_states_what_opening_it_produces() {
        let sheet = sheet();
        let nest = Nest::new();
        let level = Level::build(&sheet, &nest);
        let drawn = scene(&sheet, &level, &nest, None, &[]);
        for node in &drawn.nodes {
            let unit = &sheet.units[node.id];
            match node.inside {
                Some(inside) => {
                    assert_eq!(inside.count, unit.children.len());
                    assert!(!inside.open, "nothing is open on the first frame");
                }
                None => assert!(unit.children.is_empty() || !unit.kind.holds()),
            }
        }
    }

    /// The invariant the whole level-of-detail idea rests on: only a card the
    /// pane never opens carries calls. A function that could be opened would
    /// take its own calls off the drawing the moment it was, and the loss would
    /// be silent.
    #[test]
    fn nothing_that_calls_can_be_opened() {
        let sheet = sheet();
        for unit in &sheet.units {
            if unit.calls.is_empty() && unit.callers.is_empty() {
                continue;
            }
            assert_eq!(unit.kind, UnitKind::Function, "only functions call");
            assert!(
                unit.children.is_empty(),
                "{} has calls and holds {} things, so opening it would lose them",
                unit.qualified,
                unit.children.len()
            );
        }
    }

    /// The chain every route crosses is drawn at whatever level of detail the
    /// reader is at, so the guarantee holds folded as well as opened.
    #[test]
    fn the_chain_every_route_crosses_survives_being_folded() {
        let sheet = sheet();
        let target = sheet
            .units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Function)
            .map(|unit| unit.id)
            .find(|&id| sheet.reach.spine_to(id).len() >= 2)
            .expect("something in this workspace sits behind two chokepoints");

        let mut nest = Nest::new();
        nest.reveal(&sheet, target);
        let level = Level::build(&sheet, &nest);
        let chain = route_cards(&sheet, &nest, &level, Some(target));
        assert!(!chain.is_empty(), "the chain reaches the pane");
        assert_eq!(*chain.last().unwrap(), target, "and ends where asked");
        for &card in &chain {
            assert!(level.holds(card), "every step of the chain is drawn");
        }
        // Folded to crates the chain is still there, as the crates it crosses.
        let folded = Level::build(&sheet, &Nest::new());
        let coarse = route_cards(&sheet, &Nest::new(), &folded, Some(target));
        assert!(
            coarse.iter().all(|&card| sheet.units[card].kind == UnitKind::Crate),
            "folded, the chain is the crates every route crosses"
        );
    }

    /// The way back exists and is exact.
    ///
    /// A lid goes with the card it was on, so once a crate has become its files
    /// there is nothing on the pane left to click to undo it — the control has
    /// to be in the record, and the record has to be showing the container the
    /// reader opened even though it is no longer a card.
    #[test]
    fn a_container_the_reader_opened_keeps_its_record_and_the_way_back() {
        let sheet = sheet();
        let before = Level::build(&sheet, &Nest::new());
        let target = before
            .cards
            .iter()
            .map(|&(id, _)| id)
            .find(|&id| sheet.units[id].children.len() > 1)
            .expect("some crate has more than one file in it");

        let mut nest = Nest::new();
        nest.open(target);
        let opened = Level::build(&sheet, &nest);
        assert!(!opened.holds(target), "it is no longer a card");

        let record = UnitRecord::build(&sheet, &opened, target);
        assert!(!record.on_pane, "and the record knows it");
        assert!(!record.holds.is_empty(), "so it reports what it became");
        assert!(
            record.callers.is_empty() && record.callees.is_empty(),
            "and claims no wires, because its children carry them now"
        );

        nest.fold(target);
        assert_eq!(
            Level::build(&sheet, &nest),
            before,
            "folding it back returns exactly the pane it came from"
        );
    }

    /// Naming a function that is four levels inside a folded crate puts it on
    /// the pane. This is the whole of "show me this".
    #[test]
    fn revealing_a_buried_function_brings_it_to_the_pane() {
        let sheet = sheet();
        let id = a_function(&sheet);
        let mut nest = Nest::new();
        assert!(
            !Level::build(&sheet, &nest).holds(id),
            "it starts inside its crate"
        );
        nest.reveal(&sheet, id);
        assert!(Level::build(&sheet, &nest).holds(id));
    }

    /// A record exists for every kind of unit, and says the things only that
    /// kind can say.
    #[test]
    fn every_kind_of_unit_has_a_record() {
        let sheet = sheet();
        let nest = Nest::to_depth(&sheet, 6);
        let level = Level::build(&sheet, &nest);
        let mut seen: HashSet<UnitKind> = HashSet::new();
        for &(id, _) in &level.cards {
            let record = UnitRecord::build(&sheet, &level, id);
            assert_eq!(record.kind, sheet.units[id].kind);
            match record.kind {
                UnitKind::Function => {
                    assert!(record.standing.is_some(), "a function reports its closure");
                    assert!(record.holds.is_empty(), "and holds nothing");
                }
                _ => assert!(
                    record.standing.is_none(),
                    "a container has no reachable closure of its own"
                ),
            }
            seen.insert(record.kind);
        }
        assert!(
            seen.contains(&UnitKind::Function),
            "opened all the way, the pane draws functions"
        );
    }
}
