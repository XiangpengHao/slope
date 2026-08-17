//! The dependency lens.
//!
//! A workspace resolves to hundreds of crates and thousands of edges. This opens
//! on **three hops of it**, laid out as one DAG: the crates this workspace
//! builds, what they pulled in, and what that pulled in. Making the reader
//! assemble that by clicking hid the answer behind the interface; drawing all
//! six or eight hops of it produced a texture rather than a flow chart, and cost
//! a second of frozen tab to do it.
//!
//! Nothing past the rim is lost — it is folded. Every card is *opened* rather
//! than merely present, at every depth, so folding a port takes away whatever
//! was only reachable through it and opening one at the rim brings the next hop
//! in. Ports always carry the count of what is folded behind them.
//!
//! Selection never changes what is on the pane, and clicking a card never moves
//! the camera: the reader aimed at that card in that spot, and sliding the
//! ground out from under the click takes away what they aimed with. Holding
//! relights the edges and fills the record, and that is all.
//!
//! Naming a crate is the other case. From the finder, a row in the record or an
//! arrow key there is no telling whether the crate is even on screen, so the
//! camera does go — and one that is not on the pane at all arrives with the
//! chain that put it in the build, drawn as a lit route.

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::api::GraphLoad;
use dioxus_flow::{
    Badge, Card, Edge, EdgeState, Flow, Folding, Graph as Scene, Node, NodeState, Port, Way,
    use_flow,
};
use crate::graph::{Workspace, focus};
use crate::views::inspector::{Inspector, Record};
use crate::views::{Aim, GraphResource, WaitingWire};

/// What the reader has opened, and what they are holding. Lives on the shell so
/// the finder, the record and the pane are one state rather than three copies.
#[derive(Clone, Copy)]
pub struct DepsState {
    /// The crate being held.
    pub held: Signal<Option<usize>>,
    pub query: Signal<String>,
    /// What is open and what is folded. The pane derives everything on it from
    /// this and the workspace; nothing here stores a list of visible crates.
    pub folding: Signal<Folding>,
    /// Crates held this session, oldest first. For "why is this here" the walk
    /// *is* the answer, so throwing it away on every click threw away the thing
    /// the reader came for.
    pub history: Signal<Vec<usize>>,
    /// What the camera has been asked to do about the selection, if anything.
    /// Cleared once used.
    ///
    /// Pointing at a card on the pane asks for nothing. The reader can already
    /// see it — they just aimed at it — and moving the ground out from under a
    /// click takes away the thing they were using to aim. Naming a crate is the
    /// opposite case: from the finder, a row in the record, or an arrow key,
    /// there is no telling whether it is even on screen, so the camera goes.
    pub aim: Signal<Option<Aim>>,
}

impl DepsState {
    /// Ask for a crate **by name** — from the finder, where the reader has said
    /// what they want rather than pointed at it. The chain that put it in the
    /// build is what they are owed, so the camera frames the route rather than
    /// the neighbourhood.
    ///
    /// This is intent, not visibility. Most of what anyone searches for is
    /// already on the pane — three hops covers a lot of a build — and keying the
    /// answer off "was it there already" would mean the finder almost never drew
    /// the route, which is the thing it was asked for.
    pub fn reveal(&mut self, workspace: &Workspace, id: usize) {
        self.select(workspace, id);
        if focus::shortest_path_from_root(workspace, id).len() > 1 {
            self.aim.set(Some(Aim::Route(id)));
        }
    }

    /// Ask for a crate **by name**, from anywhere that is not the pane itself.
    /// One already on the pane is framed where it stands; one that is not
    /// arrives with the chain from a workspace member that put it there.
    pub fn select(&mut self, workspace: &Workspace, id: usize) {
        let on_pane = (self.folding)().visible(workspace).contains(&id);
        if on_pane {
            self.aim.set(Some(Aim::Neighbourhood(id)));
        } else {
            self.route_to(workspace, id);
        }
        self.held.set(Some(id));
    }

    /// Hold a crate the reader **pointed at** on the pane. The camera stays
    /// where it is: they aimed at that card at that spot, and sliding it out
    /// from under the cursor is the one motion nobody asked for.
    pub fn hold(&mut self, id: usize) {
        self.held.set(Some(id));
    }

    /// Put the chain from a workspace member to this crate on the pane.
    pub fn route_to(&mut self, workspace: &Workspace, id: usize) {
        let mut seeds: Vec<usize> = workspace.members().map(|member| member.id).collect();
        for step in focus::shortest_path_from_root(workspace, id) {
            if !seeds.contains(&step) {
                seeds.push(step);
            }
        }
        if !seeds.contains(&id) {
            seeds.push(id);
        }
        self.folding.write().set_seeds(seeds);
        self.aim.set(Some(Aim::Route(id)));
        self.held.set(Some(id));
    }

    /// Fold or open one side of a crate.
    pub fn toggle(&mut self, id: usize, way: Way) {
        self.folding.write().toggle(id, way);
    }
}

/// How many hops past the workspace the first reading opens.
///
/// Three is where the shape of a build stops being informative and starts being
/// upholstery: the crates you chose, what they pulled in, and what that pulled
/// in. Past there it is transitive scenery — still reachable through the ports,
/// which carry the count, but not what anyone opened the lens to see. It is also
/// what keeps the pane responsive, since a large workspace resolves to thousands
/// of cards and every one of them is a mounted element.
pub const OPEN_DEPTH: usize = 3;

/// The workspace, read as a graph the pane can walk.
///
/// Implemented on the workspace itself rather than copied into a structure the
/// flow crate owns: a 700-crate build would otherwise be rebuilt into fresh
/// vectors on every render, which costs more than the layout it feeds.
impl dioxus_flow::Links for Workspace {
    fn len(&self) -> usize {
        self.crates.len()
    }

    fn neighbours(&self, id: usize, way: Way) -> &[usize] {
        let Some(entry) = self.crates.get(id) else {
            return &[];
        };
        match way {
            Way::Out => &entry.deps,
            Way::In => &entry.dependents,
        }
    }
}

/// What the camera should have in frame when it lands on a crate: the crate,
/// everything attached to it that is actually on the pane, and the route that
/// explains it. Landing on a card with everything it touches outside the frame
/// is a camera move that arrives nowhere.
pub fn frame_around(
    workspace: &Workspace,
    on_pane: &[usize],
    id: usize,
    route: bool,
) -> Vec<usize> {
    let here: HashSet<usize> = on_pane.iter().copied().collect();
    let entry = &workspace.crates[id];
    let mut frame = vec![id];
    frame.extend(
        entry
            .dependents
            .iter()
            .chain(entry.deps.iter())
            .filter(|neighbour| here.contains(neighbour)),
    );
    if route {
        frame.extend(
            focus::shortest_path_from_root(workspace, id)
                .into_iter()
                .filter(|step| here.contains(step)),
        );
    }
    frame
}

/// The cards and wires for one reading of the graph.
pub fn scene(
    workspace: &Workspace,
    on_pane: &[usize],
    held: Option<usize>,
    folding: &Folding,
) -> Scene {
    let here: HashSet<usize> = on_pane.iter().copied().collect();

    // The chain that put the held crate in the build, drawn as a route.
    let route: Vec<usize> = held
        .map(|id| focus::shortest_path_from_root(workspace, id))
        .unwrap_or_default();
    let steps: HashSet<(usize, usize)> = route
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();
    let on_route: HashSet<usize> = route.iter().copied().collect();

    let nodes: Vec<Node> = on_pane
        .iter()
        .map(|&id| {
            let entry = &workspace.crates[id];
            Node {
                id,
                column: entry.rank as i32,
                card: {
                    let mut card = Card::new(&entry.name).subtitle(entry.subtitle());
                    if entry.duplicate {
                        card = card.badge(
                            Badge::new("DUP")
                                .flag()
                                .titled(format!("{} resolves at more than one version", entry.name)),
                        );
                    }
                    if entry.is_root {
                        card = card.filled();
                    }
                    card
                },
                inbound: (!entry.dependents.is_empty()).then_some(Port {
                    count: entry.dependents.len(),
                    open: folding.is_open(id, Way::In),
                }),
                outbound: (!entry.deps.is_empty()).then_some(Port {
                    count: entry.deps.len(),
                    open: folding.is_open(id, Way::Out),
                }),
                // A crate holds no crates. Containment is the call lens's axis,
                // not this one's.
                inside: None,
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

    // Every edge between two cards on the pane, not just the ones the reader
    // opened through: the mesh inside a neighbourhood is most of what the
    // neighbourhood says.
    let mut edges: Vec<Edge> = Vec::new();
    for &id in on_pane {
        for &dep in &workspace.crates[id].deps {
            if !here.contains(&dep) {
                continue;
            }
            let state = match held {
                _ if steps.contains(&(id, dep)) => EdgeState::Route,
                Some(held) if dep == held => EdgeState::Incoming,
                Some(held) if id == held => EdgeState::Outgoing,
                Some(_) => EdgeState::Muted,
                None => EdgeState::Rest,
            };
            edges.push(Edge {
                from: id,
                to: dep,
                state,
                // One manifest edge is one wire; there is nothing to gather.
                weight: 1,
                label: None,
            });
        }
    }

    Scene { nodes, edges }
}

#[component]
pub fn Deps() -> Element {
    let resource: GraphResource = use_context();
    let mut state: DepsState = use_context();
    let mut flow = use_flow();

    let failure = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(GraphLoad::Failed(message))) => Some(message.clone()),
            Some(Err(error)) => Some(error.to_string()),
            _ => None,
        }
    };
    let ready = {
        let loaded = resource.read();
        matches!(loaded.as_ref(), Some(Ok(GraphLoad::Ready(_))))
    };

    // Open on the build three hops deep, laid out as one DAG. The reader folds
    // and opens from there rather than assembling the picture a click at a time.
    //
    // Every card is *opened* rather than merely present, at every depth: fold
    // any port and what was only reachable through it goes away, open one at the
    // rim and the next hop arrives. The rim is not a wall, and the ports there
    // carry the count of what is still folded behind them.
    use_effect(move || {
        let loaded = resource.read();
        let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
            return;
        };
        if !state.folding.peek().seeds().is_empty() {
            return;
        }
        let seeds: Vec<usize> = workspace.members().map(|member| member.id).collect();
        state.folding.set(Folding::to_depth(
            workspace.as_ref(),
            seeds,
            OPEN_DEPTH,
            Way::Out,
        ));
    });

    let (scene_now, record) = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(GraphLoad::Ready(workspace))) => {
                let folding = (state.folding)();
                let on_pane = folding.visible(workspace.as_ref());
                let held = (state.held)();
                let scene = scene(workspace, &on_pane, held, &folding);
                let record = held.map(|id| {
                    let view = focus::reach(workspace, id);
                    Record::build(workspace, &view, id)
                });
                (scene, record)
            }
            _ => (Scene::default(), None),
        }
    };

    // The camera moves only when something asked it to, and clicking a card on
    // the pane is not something asking. Holding still relights the edges and
    // fills the record — it just does not take the ground with it.
    use_effect(move || {
        let Some(aim) = (state.aim)() else {
            return;
        };
        let loaded = resource.read();
        let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
            return;
        };
        state.aim.set(None);
        let on_pane = (state.folding)().visible(workspace.as_ref());
        match aim {
            // The chain is the answer to the question that summoned the crate,
            // so it is framed end to end rather than just its destination.
            Aim::Route(id) => {
                let route: Vec<usize> = focus::shortest_path_from_root(workspace, id)
                    .into_iter()
                    .filter(|step| on_pane.contains(step))
                    .collect();
                if route.len() > 1 {
                    flow.route(route);
                } else {
                    flow.frame(id, frame_around(workspace, &on_pane, id, true));
                }
            }
            // Framed on what it is attached to rather than at a fixed
            // magnification: landing on a card with everything it touches
            // outside the frame arrives nowhere.
            Aim::Neighbourhood(id) => {
                flow.frame(id, frame_around(workspace, &on_pane, id, true));
            }
        }
    });

    if !ready {
        return rsx! {
            div { class: "flex h-full items-center justify-center px-6",
                if let Some(message) = failure {
                    Failure { message }
                } else {
                    Resolving {}
                }
            }
        };
    }

    // Pointing at a card holds it where it is. Naming one goes through
    // `select`, which is what moves the camera.
    let select = move |id: usize| state.hold(id);

    let empty = scene_now.nodes.is_empty();

    rsx! {
        div { class: "flex h-full w-full min-h-0 flex-col overflow-hidden lg:flex-row",

            div { class: "relative flex min-h-0 min-w-0 flex-1",
                Flow {
                    flow,
                    graph: scene_now,
                    on_select: select,
                    // Opening a port re-tidies the pane under the reader's
                    // cursor. The camera stays out of it: the cards glide to
                    // their new places and the newcomers fade in among them,
                    // which shows what changed without also moving the frame the
                    // reader was reading it in.
                    on_port: move |(id, way): (usize, Way)| state.toggle(id, way),
                    on_clear: move |_| state.held.set(None),
                }

                if empty {
                    p { class: "pointer-events-none absolute inset-x-0 top-1/2 text-center text-[13px] text-ink-muted",
                        "Nothing is open. Find a crate, or reload to start from the workspace."
                    }
                }

                CrateIndex {}
            }

            Inspector { record }
        }
    }
}

/// The pane draws cards, so assistive technology needs somewhere real to go.
/// Every crate in the workspace is here, most depended on first; focusing one
/// opens it exactly as clicking a card does.
///
/// Sighted keyboard users get painted controls instead: the finder, the record's
/// lists, and the arrow keys.
#[component]
fn CrateIndex() -> Element {
    let resource: GraphResource = use_context();
    let mut state: DepsState = use_context();

    let mut listing: Vec<(usize, String, usize)> = {
        let loaded = resource.read();
        let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
            return rsx! {};
        };
        workspace
            .crates
            .iter()
            .map(|entry| {
                (
                    entry.id,
                    format!("{} {}", entry.name, entry.version),
                    entry.dependents.len(),
                )
            })
            .collect()
    };
    listing.sort_by_key(|(_, _, dependents)| std::cmp::Reverse(*dependents));

    rsx! {
        ul { class: "sr-only", "aria-label": "Every crate in this workspace, most depended on first",
            for (id , label , dependents) in listing {
                li { key: "{id}",
                    button {
                        onclick: move |_| {
                            let loaded = resource.read();
                            if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
                                state.select(workspace, id);
                            }
                        },
                        "{label}, {dependents} crates depend on it"
                    }
                }
            }
        }
    }
}

#[component]
fn Resolving() -> Element {
    rsx! {
        div { class: "flex flex-col items-center gap-4",
            WaitingWire {}
            p { class: "text-[13px] text-ink-muted", "Resolving the workspace — running cargo metadata." }
        }
    }
}

#[component]
fn Failure(message: String) -> Element {
    rsx! {
        div { class: "max-w-2xl",
            h1 { class: "text-[15px] font-semibold", "This path could not be resolved" }
            p { class: "mt-1 text-[13px] text-ink-muted",
                "rust-viewer reads a crate or workspace directory. Point it at one that contains a Cargo.toml, then reload."
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
    use crate::graph::{build as graph_build, metadata};

    fn real() -> Workspace {
        let resolved = metadata::resolve().expect("cargo metadata should resolve this workspace");
        graph_build::build(resolved.packages, resolved.workspace, resolved.manifest_dir)
    }

    fn members(workspace: &Workspace) -> Vec<usize> {
        workspace.members().map(|member| member.id).collect()
    }

    /// The opening reading, as the lens actually builds it.
    fn opening(workspace: &Workspace) -> Folding {
        Folding::to_depth(workspace, members(workspace), OPEN_DEPTH, Way::Out)
    }

    /// The opening view: three hops of the build, in one DAG. Everything within
    /// the rim is there from the first frame, and nothing past it is.
    #[test]
    fn the_first_reading_is_three_hops_deep() {
        let workspace = real();
        let depth = dioxus_flow::depths(&workspace, &members(&workspace), Way::Out);
        let on_pane = opening(&workspace).visible(&workspace);

        for &id in &on_pane {
            assert!(
                depth[&id] <= OPEN_DEPTH,
                "{} is {} hops out and should still be folded away",
                workspace.crates[id].name,
                depth[&id]
            );
        }
        for (&id, &hops) in &depth {
            if hops <= OPEN_DEPTH {
                assert!(
                    on_pane.contains(&id),
                    "{} is only {hops} hops out and belongs on the opening pane",
                    workspace.crates[id].name
                );
            }
        }
    }

    /// The rim is a fold, not a wall: opening a port there brings the next hop
    /// in, exactly as opening one anywhere else does.
    #[test]
    fn opening_a_port_at_the_rim_brings_the_next_hop_in() {
        let workspace = real();
        let depth = dioxus_flow::depths(&workspace, &members(&workspace), Way::Out);
        let mut folding = opening(&workspace);
        let before = folding.visible(&workspace);

        // A crate sitting on the rim with something still folded behind it.
        let Some(&rim) = before
            .iter()
            .filter(|&&id| depth[&id] == OPEN_DEPTH && !workspace.crates[id].deps.is_empty())
            .max_by_key(|&&id| workspace.crates[id].deps.len())
        else {
            return; // A workspace shallower than the rim has nothing to prove.
        };

        folding.open(rim, Way::Out);
        let after = folding.visible(&workspace);
        assert!(
            after.len() > before.len(),
            "opening {} should have brought its {} dependencies in",
            workspace.crates[rim].name,
            workspace.crates[rim].deps.len()
        );
        for &dep in &workspace.crates[rim].deps {
            assert!(after.contains(&dep), "a dependency of the opened crate is missing");
        }
    }

    /// The depth this lens opens to is measured in shortest hops, which is what
    /// makes "three levels of dependency" mean what a reader expects. The walk
    /// itself belongs to the flow crate and is tested there; this pins the
    /// meaning against a real workspace.
    #[test]
    fn depth_is_the_shortest_route_from_the_workspace() {
        let workspace = real();
        let seeds = members(&workspace);
        let depth = dioxus_flow::depths(&workspace, &seeds, Way::Out);
        for &seed in &seeds {
            assert_eq!(depth[&seed], 0, "a workspace member is its own starting point");
            for &dep in &workspace.crates[seed].deps {
                // At most one, not exactly one: a workspace member can depend on
                // another workspace member, and that one is still a starting
                // point rather than a hop out.
                assert!(
                    depth[&dep] <= 1,
                    "{} is a direct dependency of {} but is recorded {} hops out",
                    workspace.crates[dep].name,
                    workspace.crates[seed].name,
                    depth[&dep]
                );
            }
        }
    }

    /// Every card is opened rather than merely present, at every depth: fold one
    /// port and what was only reachable through it goes away.
    #[test]
    fn a_card_at_any_depth_is_opened_rather_than_merely_present() {
        let workspace = real();
        let mut folding = opening(&workspace);
        let whole = folding.visible(&workspace);

        // Fold the busiest crate the workspace builds against.
        let seeds = members(&workspace);
        let busiest = *seeds
            .iter()
            .flat_map(|&id| workspace.crates[id].deps.iter())
            .max_by_key(|&&dep| workspace.crates[dep].deps.len())
            .expect("a workspace member depends on something");
        folding.fold(busiest, Way::Out);
        let folded = folding.visible(&workspace);
        assert!(
            folded.len() < whole.len(),
            "folding {} took nothing away",
            workspace.crates[busiest].name
        );
        assert!(folded.contains(&busiest), "the card folded is still on the pane");
    }

    /// Every card on the pane is reachable from a seed through open ports. A
    /// card nobody opened is a card nobody can fold.
    #[test]
    fn nothing_is_on_the_pane_that_was_not_opened() {
        let workspace = real();
        let seeds = members(&workspace);
        let folding = opening(&workspace);
        let on_pane = folding.visible(&workspace);
        for &id in &on_pane {
            let opened = seeds.contains(&id)
                || workspace.crates[id]
                    .dependents
                    .iter()
                    .any(|owner| folding.is_open(*owner, Way::Out));
            assert!(opened, "{} is on the pane but nothing opened it", workspace.crates[id].name);
        }
    }

    /// Holding a crate changes ink, never the cast: the same cards are on the
    /// pane before and after, in the same columns.
    #[test]
    fn holding_a_crate_never_changes_what_is_on_the_pane() {
        let workspace = real();
        let folding = opening(&workspace);
        let on_pane = folding.visible(&workspace);
        let held = *on_pane.last().unwrap();

        let cold = scene(&workspace, &on_pane, None, &folding);
        let warm = scene(&workspace, &on_pane, Some(held), &folding);
        assert_eq!(cold.nodes.len(), warm.nodes.len());
        for (before, after) in cold.nodes.iter().zip(warm.nodes.iter()) {
            assert_eq!(before.id, after.id);
            assert_eq!(before.column, after.column);
        }
        assert_eq!(cold.edges.len(), warm.edges.len());
    }

    /// Direction is the one variable that earns a hue, and it has to agree with
    /// which side of the held crate the edge is on.
    #[test]
    fn edges_take_their_hue_from_which_way_they_run() {
        let workspace = real();
        let folding = opening(&workspace);
        let on_pane = folding.visible(&workspace);
        let held = *on_pane
            .iter()
            .find(|&&id| !workspace.crates[id].is_root)
            .expect("the pane holds more than the workspace itself");
        let drawn = scene(&workspace, &on_pane, Some(held), &folding);

        let route = focus::shortest_path_from_root(&workspace, held);
        let steps: HashSet<(usize, usize)> =
            route.windows(2).map(|pair| (pair[0], pair[1])).collect();

        for edge in &drawn.edges {
            match edge.state {
                EdgeState::Incoming => assert_eq!(edge.to, held),
                EdgeState::Outgoing => assert_eq!(edge.from, held),
                EdgeState::Route => assert!(steps.contains(&(edge.from, edge.to))),
                EdgeState::Muted => assert!(edge.from != held && edge.to != held),
                EdgeState::Rest => panic!("something is held, so nothing rests"),
            }
        }
    }

    /// Every edge on the pane joins two cards that are actually on it. An edge
    /// to a card that is not there is a line into empty space.
    #[test]
    fn every_edge_joins_two_cards_that_are_on_the_pane() {
        let workspace = real();
        let folding = opening(&workspace);
        let on_pane = folding.visible(&workspace);
        let drawn = scene(&workspace, &on_pane, None, &folding);
        let ids: HashSet<usize> = drawn.nodes.iter().map(|node| node.id).collect();
        for edge in &drawn.edges {
            assert!(ids.contains(&edge.from) && ids.contains(&edge.to));
        }
    }

    /// A port's number is the whole count, open or folded — nothing is hidden by
    /// folding, only put away.
    #[test]
    fn a_port_always_carries_the_whole_count() {
        let workspace = real();
        let folding = opening(&workspace);
        let on_pane = folding.visible(&workspace);
        let drawn = scene(&workspace, &on_pane, None, &folding);
        for node in &drawn.nodes {
            let entry = &workspace.crates[node.id];
            assert_eq!(
                node.inbound.map(|port| port.count).unwrap_or(0),
                entry.dependents.len()
            );
            assert_eq!(
                node.outbound.map(|port| port.count).unwrap_or(0),
                entry.deps.len()
            );
            // Only where there is a port to say it. A crate with no
            // dependencies has nothing to open, whatever the fold state says
            // about a side that was never going to draw anything.
            if let Some(port) = node.outbound {
                assert_eq!(port.open, folding.is_open(node.id, Way::Out));
            }
        }
    }

    /// Asking for a crate that is not on the pane brings the chain that put it
    /// in the build with it, and every step of that chain is drawn as a route.
    #[test]
    fn a_crate_off_the_pane_arrives_with_its_route() {
        let workspace = real();
        let deep = workspace
            .crates
            .iter()
            .max_by_key(|entry| entry.rank)
            .expect("this workspace has a deepest crate");

        let mut seeds: Vec<usize> = members(&workspace);
        for step in focus::shortest_path_from_root(&workspace, deep.id) {
            if !seeds.contains(&step) {
                seeds.push(step);
            }
        }
        let folding = Folding::new(seeds);
        let on_pane = folding.visible(&workspace);
        assert!(on_pane.contains(&deep.id));

        let drawn = scene(&workspace, &on_pane, Some(deep.id), &folding);
        let route = focus::shortest_path_from_root(&workspace, deep.id);
        let lit = drawn
            .edges
            .iter()
            .filter(|edge| edge.state == EdgeState::Route)
            .count();
        assert_eq!(
            lit,
            route.len() - 1,
            "the route to {} is {} hops but {lit} of them are lit",
            deep.name,
            route.len() - 1
        );
    }
}
