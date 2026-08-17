//! The dependency lens.
//!
//! A workspace resolves to hundreds of crates and thousands of edges, and this
//! lens draws none of that. It draws a **walk**: one crate at the centre, what
//! it depends on fanned around it, and nothing else until the reader clicks. A
//! click opens that crate the same way, so the drawing grows outward from where
//! they were already looking.
//!
//! Everything else here follows from that.
//!
//! **No levels.** A column used to say how deep a crate sits in the build, which
//! is a fact about the whole workspace rather than about the question being
//! asked. On a walk the only distance that means anything is how many opens it
//! took to get here, and that is what the drawing shows.
//!
//! **The shape is the same at every size.** Each opened card fans its own
//! dependencies around itself, in a cone facing away from where it was reached
//! from, so a branch is a smaller copy of the whole. The reader can see which
//! card a cluster belongs to because the cluster is arranged around it.
//!
//! **Only the walk's own edges are drawn at rest.** A crate the walk reached
//! another way is still joined to the ones on the pane, and after three opens
//! there are 591 of those against the 120 the walk is built from. Drawn together
//! they are a web across the picture that hides the shape the walk just made, so
//! they wait until a crate is held — which is the moment the reader has asked
//! about one crate rather than about the walk.
//!
//! Ports count what is attached, in each direction. With most edges undrawn that
//! number is the only way to tell a busy crate from a quiet one before opening
//! it, and it is what says the walk has more to give.
//!
//! Clicking a card on the pane never moves the camera: the reader aimed at that
//! card in that spot, and sliding the ground out from under the click takes away
//! what they aimed with. Naming a crate is the other case — from the finder or
//! the record there is no telling where it is, so that starts a fresh walk from
//! it and the camera goes.

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::api::GraphLoad;
use dioxus_flow::{
    Badge, Card, Edge, EdgeState, Flow, Graph as Scene, Node, NodeState, Port, Way, use_flow,
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
    /// The crate at the centre. The walk starts here and everything on the pane
    /// was reached from it.
    pub root: Signal<Option<usize>>,
    /// The crates whose dependencies have been opened. The centre is opened the
    /// moment it becomes the centre, so the first frame is never a lone card.
    pub opened: Signal<HashSet<usize>>,
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
    /// Every crate is on the pane, so this is always a camera move; the only
    /// question is whether the chain that explains the crate is worth framing
    /// with it, and [`reveal`](Self::reveal) is where that is decided.
    pub fn select(&mut self, _workspace: &Workspace, id: usize) {
        self.recentre(id);
    }

    /// Hold a crate the reader **pointed at** on the pane, and open it.
    ///
    /// One click, both verbs. The reader who clicks a card on a walk means
    /// "this one" — they want to read it *and* see where it goes, and asking
    /// them to say so twice would make every step of the walk two clicks. The
    /// camera stays where it is: they aimed at that card at that spot, and
    /// sliding it out from under the cursor is the one motion nobody asked for.
    pub fn hold(&mut self, id: usize) {
        self.held.set(Some(id));
        self.opened.write().insert(id);
    }

    /// Start a new walk from this crate: it becomes the centre, opened, and
    /// everything the last walk had opened is let go.
    pub fn recentre(&mut self, id: usize) {
        self.root.set(Some(id));
        self.opened.set(HashSet::from([id]));
        self.held.set(Some(id));
        self.aim.set(Some(Aim::Neighbourhood(id)));
    }

    /// Frame the chain from a workspace member to this crate.
    pub fn route_to(&mut self, _workspace: &Workspace, id: usize) {
        self.aim.set(Some(Aim::Route(id)));
        self.held.set(Some(id));
    }
}

/// The walk: every crate reached from the centre through crates the reader has
/// opened, and the crate each one was first reached through.
///
/// This replaces asking the workspace what is on the pane. Nothing is on the
/// pane that the reader did not open their way to, so the drawing is exactly as
/// large as the question they have asked so far — which is the only thing that
/// has ever kept a 718-crate graph readable.
fn walk(workspace: &Workspace, root: usize, opened: &HashSet<usize>) -> Vec<dioxus_flow::Shoot> {
    dioxus_flow::radial::spanning(
        root,
        &|id| opened.contains(&id),
        &|id| {
            workspace
                .crates
                .get(id)
                .map(|entry| entry.deps.clone())
                .unwrap_or_default()
        },
    )
}

/// How the dependency pane is drawn: the centre, and the walk fanned around it.
///
/// No levels. A level said how far a crate sits from the workspace, which is a
/// fact about the whole build and not about the question the reader is asking;
/// on a walk the only distance that matters is how many opens it took to get
/// here from the crate they started at, and that is the ring a card sits on.
///
/// Wires are always drawn. Hiding them was the right answer while the pane held
/// all 718 crates and the mesh between them was a texture; here the wires *are*
/// the drawing, and there are only ever as many as the reader has opened.
fn pane_style() -> dioxus_flow::Style {
    dioxus_flow::Style {
        arrangement: dioxus_flow::Arrangement::Radial,
        // Wires are drawn. Hiding them was the answer while the pane held all
        // 718 crates and the mesh between them was a texture; on a walk the
        // wires *are* the picture. Which wires reach the pane at all is decided
        // in `scene`: the walk's own always, the rest when a card is held.
        wires: dioxus_flow::Wires::Always,
        column_pitch: 300.0,
        gap: 28.0,
        ..Default::default()
    }
}

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
    tree: &[dioxus_flow::Shoot],
    held: Option<usize>,
    root: Option<usize>,
) -> Scene {
    let on_pane: Vec<usize> = tree.iter().map(|shoot| shoot.id).collect();
    let here: HashSet<usize> = on_pane.iter().copied().collect();
    // The edges the walk itself is made of: how each card got here. These are
    // the drawing's skeleton and are always shown.
    let spine: HashSet<(usize, usize)> = tree
        .iter()
        .filter_map(|shoot| Some((shoot.parent?, shoot.id)))
        .collect();

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
                // Every crate is on the pane, so a port no longer says "there is
                // more behind me" — nothing is behind it. What it says now is how
                // busy this crate is in each direction, which is the one thing a
                // reader cannot see for themselves when no wire is drawn until
                // something is held. Never lit: a count is not a state.
                inbound: (!entry.dependents.is_empty()).then_some(Port {
                    count: entry.dependents.len(),
                    open: false,
                }),
                outbound: (!entry.deps.is_empty()).then_some(Port {
                    count: entry.deps.len(),
                    open: false,
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
    for &id in &on_pane {
        for &dep in &workspace.crates[id].deps {
            if !here.contains(&dep) {
                continue;
            }
            // A crate the walk reached another way is still joined to this one,
            // and saying so is the point of drawing a graph rather than a tree.
            // But there are five of those for every one the walk is built from —
            // 591 against 120 after three opens — and drawn at rest they are a
            // web across the circle that hides the shape the walk just made. So
            // they wait until a card is held, which is when the reader has asked
            // about a particular crate rather than about the walk.
            let on_spine = spine.contains(&(id, dep));
            let touches_held = held == Some(id) || held == Some(dep);
            if !on_spine && !touches_held {
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

    Scene { nodes, edges, root }
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

    // The walk has to start somewhere. The first workspace member is the crate
    // the reader almost certainly came to ask about — it is the thing they
    // build — and starting on a bare pane with a finder would make the lens
    // answer nothing until it was interrogated.
    use_effect(move || {
        let loaded = resource.read();
        let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
            return;
        };
        if state.root.peek().is_some() {
            return;
        }
        if let Some(first) = workspace.members().map(|member| member.id).next() {
            state.recentre(first);
        }
    });

    let (scene_now, record) = {
        let loaded = resource.read();
        match loaded.as_ref() {
            Some(Ok(GraphLoad::Ready(workspace))) => {
                let root = (state.root)();
                let opened = (state.opened)();
                let tree = root
                    .map(|root| walk(workspace.as_ref(), root, &opened))
                    .unwrap_or_default();
                let held = (state.held)();
                let scene = scene(workspace, &tree, held, root);
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
        let on_pane: Vec<usize> = (state.root)()
            .map(|root| walk(workspace.as_ref(), root, &(state.opened)()))
            .unwrap_or_default()
            .into_iter()
            .map(|shoot| shoot.id)
            .collect();
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
                    style: pane_style(),
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

    /// Where the lens starts: one crate, opened, and what it depends on. Not the
    /// workspace, not three hops of it — one crate and its own dependencies.
    #[test]
    fn the_first_frame_is_one_crate_and_what_it_depends_on() {
        let workspace = real();
        let root = members(&workspace)[0];
        let tree = walk(&workspace, root, &HashSet::from([root]));

        let on_pane: HashSet<usize> = tree.iter().map(|shoot| shoot.id).collect();
        let expected: HashSet<usize> = std::iter::once(root)
            .chain(workspace.crates[root].deps.iter().copied())
            .collect();
        assert_eq!(on_pane, expected);
        assert_eq!(tree[0].id, root, "the centre comes first");
        assert_eq!(tree[0].parent, None, "the centre was reached from nothing");
    }

    /// An unopened crate keeps its dependencies to itself. This is the whole
    /// budget: the pane costs what the reader has asked for and nothing else.
    #[test]
    fn an_unopened_crate_brings_nothing_with_it() {
        let workspace = real();
        let root = members(&workspace)[0];
        let alone = walk(&workspace, root, &HashSet::new());
        assert_eq!(alone.len(), 1, "nothing is opened, so nothing else is drawn");
    }

    /// Opening a crate adds its dependencies and leaves the rest of the walk
    /// where it was.
    #[test]
    fn opening_a_crate_adds_exactly_what_it_depends_on() {
        let workspace = real();
        let root = members(&workspace)[0];
        let first = HashSet::from([root]);
        let before: HashSet<usize> = walk(&workspace, root, &first)
            .iter()
            .map(|shoot| shoot.id)
            .collect();

        // Open whichever dependency carries the most of its own.
        let Some(&next) = workspace.crates[root]
            .deps
            .iter()
            .max_by_key(|&&dep| workspace.crates[dep].deps.len())
        else {
            return;
        };
        let mut opened = first.clone();
        opened.insert(next);
        let after: HashSet<usize> = walk(&workspace, root, &opened)
            .iter()
            .map(|shoot| shoot.id)
            .collect();

        assert!(before.is_subset(&after), "opening took something away");
        for &dep in &workspace.crates[next].deps {
            assert!(
                after.contains(&dep),
                "{} was not brought in by opening {}",
                workspace.crates[dep].name,
                workspace.crates[next].name
            );
        }
    }

    /// Every card was reached from the centre through crates the reader opened.
    /// Nothing is on the pane that the walk did not put there.
    #[test]
    fn every_card_was_reached_through_something_opened() {
        let workspace = real();
        let root = members(&workspace)[0];
        let mut opened = HashSet::from([root]);
        opened.extend(workspace.crates[root].deps.iter().take(3).copied());
        let tree = walk(&workspace, root, &opened);

        for shoot in &tree {
            let Some(parent) = shoot.parent else {
                assert_eq!(shoot.id, root);
                continue;
            };
            assert!(
                opened.contains(&parent),
                "{} was reached through {}, which is not open",
                workspace.crates[shoot.id].name,
                workspace.crates[parent].name
            );
            assert!(
                workspace.crates[parent].deps.contains(&shoot.id),
                "{} is not actually a dependency of {}",
                workspace.crates[shoot.id].name,
                workspace.crates[parent].name
            );
        }
    }

    /// At rest the drawing is the walk and only the walk: one wire per card,
    /// less the centre. The crates that are joined some other way are still
    /// joined, and they wait until something is held.
    #[test]
    fn at_rest_only_the_walks_own_wires_are_drawn() {
        let workspace = real();
        let root = members(&workspace)[0];
        let mut opened = HashSet::from([root]);
        opened.extend(workspace.crates[root].deps.iter().take(3).copied());
        let tree = walk(&workspace, root, &opened);

        let drawn = scene(&workspace, &tree, None, Some(root));
        assert_eq!(
            drawn.edges.len(),
            tree.len() - 1,
            "at rest there should be exactly one wire per card but the centre"
        );
        assert_eq!(drawn.root, Some(root));
        for edge in &drawn.edges {
            assert_eq!(edge.state, EdgeState::Rest);
        }
    }

    /// Holding a crate brings in the edges the walk did not draw — the ones that
    /// join it to cards it did not arrive through.
    #[test]
    fn holding_a_crate_brings_in_the_wires_the_walk_left_out() {
        let workspace = real();
        let root = members(&workspace)[0];
        let mut opened = HashSet::from([root]);
        opened.extend(workspace.crates[root].deps.iter().take(4).copied());
        let tree = walk(&workspace, root, &opened);
        let on_pane: HashSet<usize> = tree.iter().map(|shoot| shoot.id).collect();

        let cold = scene(&workspace, &tree, None, Some(root));
        // A card with edges to cards it did not arrive through.
        let spine: HashSet<(usize, usize)> = tree
            .iter()
            .filter_map(|shoot| Some((shoot.parent?, shoot.id)))
            .collect();
        let Some(&busy) = on_pane.iter().max_by_key(|&&id| {
            workspace.crates[id]
                .deps
                .iter()
                .filter(|&&dep| on_pane.contains(&dep) && !spine.contains(&(id, dep)))
                .count()
        }) else {
            return;
        };
        let warm = scene(&workspace, &tree, Some(busy), Some(root));
        assert!(
            warm.edges.len() >= cold.edges.len(),
            "holding took wires away"
        );
        for edge in &warm.edges {
            if !spine.contains(&(edge.from, edge.to)) {
                assert!(
                    edge.from == busy || edge.to == busy,
                    "a wire was drawn that neither the walk nor the selection asked for"
                );
            }
        }
    }

    /// Direction still earns the only hue in the system, and it has to agree
    /// with which side of the held crate the edge runs.
    #[test]
    fn edges_take_their_hue_from_which_way_they_run() {
        let workspace = real();
        let root = members(&workspace)[0];
        let mut opened = HashSet::from([root]);
        opened.extend(workspace.crates[root].deps.iter().take(3).copied());
        let tree = walk(&workspace, root, &opened);
        let held = tree[1].id;
        let drawn = scene(&workspace, &tree, Some(held), Some(root));

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

    /// Every wire joins two cards that are actually on the pane.
    #[test]
    fn every_edge_joins_two_cards_that_are_on_the_pane() {
        let workspace = real();
        let root = members(&workspace)[0];
        let tree = walk(&workspace, root, &HashSet::from([root]));
        let drawn = scene(&workspace, &tree, None, Some(root));
        let ids: HashSet<usize> = drawn.nodes.iter().map(|node| node.id).collect();
        for edge in &drawn.edges {
            assert!(ids.contains(&edge.from) && ids.contains(&edge.to));
        }
    }

    /// A port counts what is attached and never lights up. With most wires
    /// undrawn it is the only thing that says a crate has more behind it.
    #[test]
    fn a_port_counts_what_is_attached_and_is_never_lit() {
        let workspace = real();
        let root = members(&workspace)[0];
        let tree = walk(&workspace, root, &HashSet::from([root]));
        let drawn = scene(&workspace, &tree, None, Some(root));
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
            for port in [node.inbound, node.outbound].into_iter().flatten() {
                assert!(!port.open, "a port has nothing to open");
            }
        }
    }

    /// Depth is still measured in shortest hops. Nothing draws columns any more,
    /// but the route and the record both lean on it.
    #[test]
    fn depth_is_the_shortest_route_from_the_workspace() {
        let workspace = real();
        let seeds = members(&workspace);
        let depth = dioxus_flow::depths(&workspace, &seeds, Way::Out);
        for &seed in &seeds {
            assert_eq!(depth[&seed], 0, "a workspace member is its own starting point");
            for &dep in &workspace.crates[seed].deps {
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
}
