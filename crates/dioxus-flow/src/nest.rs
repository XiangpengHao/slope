//! Containment: one graph, drawn at a level of detail.
//!
//! [`Folding`](crate::Folding) answers *how much of the graph* is on the pane.
//! This answers a different question — *how finely* — and the two are
//! independent. A graph whose nodes nest (a crate holds files, a file holds
//! types, a type holds methods; a company holds teams holds people) has no one
//! right number of nodes to draw. It has a **frontier**: for each branch of the
//! hierarchy, the deepest container the reader has opened, or the container
//! itself where they have not.
//!
//! Everything below the frontier is still in the drawing — it is *inside* a
//! card. So an edge between two hidden nodes is not dropped, it is **lifted**
//! onto the pair of cards that hold them, and the edges that land on the same
//! pair are one wire carrying a count. That is what keeps a graph legible as it
//! grows: 23,000 calls between 8,000 functions is a texture, and the same
//! 23,000 calls between the 82 crates those functions live in is a diagram.
//!
//! ```
//! use dioxus_flow::{Forest, Nest};
//!
//! // Two crates; the first holds two files, and each file holds a function.
//! //   0 ─┬ 1 ─ 2      3 ─ 4 ─ 5
//! //      └ 6 ─ 7
//! let tree = Forest::new(vec![
//!     vec![1, 6], vec![2], vec![], vec![4], vec![5], vec![], vec![7], vec![],
//! ]);
//! let calls = [(2, 5), (7, 5), (2, 7)];
//!
//! // Folded, the whole program is two cards and one wire — carrying both of
//! // the calls that cross between them.
//! let mut nest = Nest::new();
//! assert_eq!(nest.frontier(&tree), vec![0, 3]);
//! let wires = nest.lift(&tree).bundle(&calls);
//! assert_eq!(wires.iter().find(|w| (w.from, w.to) == (0, 3)).unwrap().weight, 2);
//!
//! // Open the left crate and it becomes its files; the wires re-aim.
//! nest.open(0);
//! assert_eq!(nest.frontier(&tree), vec![1, 6, 3]);
//! let wires = nest.lift(&tree).bundle(&calls);
//! assert_eq!(wires.iter().find(|w| (w.from, w.to) == (1, 6)).unwrap().weight, 1);
//! ```

use std::collections::{HashMap, HashSet};

/// A containment hierarchy: what holds what.
///
/// A trait for the same reason [`Links`](crate::Links) is one — the host
/// already has this. A lens holding a tree of crates, files and functions
/// implements three methods and lends it; copying that tree into a structure
/// this crate owns would cost more per render than the layout it feeds.
pub trait Tree {
    /// The number of nodes, which is one past the largest id.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The nodes with nothing above them, in drawing order.
    fn roots(&self) -> &[usize];

    /// What sits inside this node, in drawing order. Empty for a leaf, and
    /// empty — rather than a panic — for an id the tree does not have.
    fn children(&self, id: usize) -> &[usize];

    /// What this node sits inside, if anything.
    fn parent(&self, id: usize) -> Option<usize>;
}

/// A hierarchy this crate owns, for a host with no tree of its own to lend.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Forest {
    children: Vec<Vec<usize>>,
    parent: Vec<Option<usize>>,
    roots: Vec<usize>,
}

impl Forest {
    /// Build from each node's children. Parents and roots are derived, so a
    /// node listed as nobody's child is a root.
    pub fn new(children: Vec<Vec<usize>>) -> Self {
        let mut parent = vec![None; children.len()];
        for (id, kids) in children.iter().enumerate() {
            for &kid in kids {
                if kid < parent.len() {
                    parent[kid] = Some(id);
                }
            }
        }
        let roots = (0..children.len()).filter(|&id| parent[id].is_none()).collect();
        Self {
            children,
            parent,
            roots,
        }
    }
}

impl Tree for Forest {
    fn len(&self) -> usize {
        self.children.len()
    }

    fn roots(&self) -> &[usize] {
        &self.roots
    }

    fn children(&self, id: usize) -> &[usize] {
        self.children.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    fn parent(&self, id: usize) -> Option<usize> {
        self.parent.get(id).copied().flatten()
    }
}

/// Which containers the reader has opened.
///
/// Holds intent, not results — the same discipline as [`Folding`](crate::Folding).
/// A container marked open whose own parent is closed contributes nothing, and
/// that is deliberate: opening is remembered, so closing a crate and opening it
/// again returns the reader to the file they were reading rather than to the
/// top.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Nest {
    open: HashSet<usize>,
}

impl Nest {
    /// Everything folded: the drawing is the roots.
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything above `depth` opened, so the drawing is the hierarchy's own
    /// `depth`-th level (0 being the roots).
    pub fn to_depth(tree: &impl Tree, depth: usize) -> Self {
        let mut nest = Self::new();
        let mut level: Vec<usize> = tree.roots().to_vec();
        for _ in 0..depth {
            let mut next = Vec::new();
            for id in level {
                if tree.children(id).is_empty() {
                    continue;
                }
                nest.open.insert(id);
                next.extend(tree.children(id).iter().copied());
            }
            if next.is_empty() {
                break;
            }
            level = next;
        }
        nest
    }

    pub fn is_open(&self, id: usize) -> bool {
        self.open.contains(&id)
    }

    pub fn open(&mut self, id: usize) {
        self.open.insert(id);
    }

    pub fn fold(&mut self, id: usize) {
        self.open.remove(&id);
    }

    pub fn toggle(&mut self, id: usize) {
        if !self.open.remove(&id) {
            self.open.insert(id);
        }
    }

    /// Open everything above `id`, so `id` itself lands on the frontier.
    ///
    /// What "show me this function" means when the function is four levels
    /// inside a folded crate.
    pub fn reveal(&mut self, tree: &impl Tree, id: usize) {
        let mut here = tree.parent(id);
        let mut guard = 0;
        while let Some(container) = here {
            self.open.insert(container);
            here = tree.parent(container);
            guard += 1;
            if guard > tree.len() {
                break;
            }
        }
    }

    /// The cards on the pane: for each branch, the deepest opened container's
    /// children, or the container itself where it is folded.
    ///
    /// Depth-first in the tree's own order, so a container's children appear
    /// exactly where the container was — the order a reader is holding in their
    /// head survives opening one of them.
    pub fn frontier(&self, tree: &impl Tree) -> Vec<usize> {
        self.descend(tree, tree.roots().iter().rev().copied().collect())
    }

    /// Walk down from a stack of nodes, stopping at the first folded container
    /// on each branch. The stack is in reverse drawing order, because popping
    /// reverses it again.
    fn descend(&self, tree: &impl Tree, mut stack: Vec<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        // A malformed tree — one with a cycle in it — must give a short wrong
        // answer rather than hang the frame it is drawing.
        let mut guard = tree.len() * 2 + 8;
        while let Some(id) = stack.pop() {
            guard -= 1;
            if guard == 0 {
                break;
            }
            let children = tree.children(id);
            if self.open.contains(&id) && !children.is_empty() {
                stack.extend(children.iter().rev().copied());
            } else {
                out.push(id);
            }
        }
        out
    }

    /// Where each node in the hierarchy is drawn: the frontier card that holds
    /// it, or itself when it is on the frontier.
    pub fn lift(&self, tree: &impl Tree) -> Lift {
        let mut to = vec![usize::MAX; tree.len()];
        let mut stack: Vec<(usize, usize)> = tree
            .roots()
            .iter()
            .map(|&root| (root, usize::MAX))
            .collect();
        let mut guard = tree.len() * 2 + 8;
        while let Some((id, above)) = stack.pop() {
            guard -= 1;
            if guard == 0 {
                break;
            }
            let children = tree.children(id);
            // The first container on the way down that is not opened is the card
            // this branch is drawn as; everything under it inherits that answer.
            let here = if above != usize::MAX {
                above
            } else if self.open.contains(&id) && !children.is_empty() {
                usize::MAX
            } else {
                id
            };
            if let Some(slot) = to.get_mut(id) {
                *slot = here;
            }
            for &child in children {
                stack.push((child, here));
            }
        }
        Lift { to }
    }

    /// The frontier cards at or under `id`: itself when it is drawn, its own
    /// visible descendants when it has been opened.
    ///
    /// What a request naming a node means once the reader has changed the level
    /// of detail under it — one seed, one camera target, one selection.
    pub fn project(&self, tree: &impl Tree, id: usize) -> Vec<usize> {
        if id >= tree.len() {
            return Vec::new();
        }
        // The path down to it, so the answer is found from the root rather than
        // from the node: a node the reader has never opened towards is drawn as
        // whichever container above it is still folded.
        let mut path = vec![id];
        let mut here = tree.parent(id);
        let mut guard = tree.len();
        while let Some(above) = here {
            path.push(above);
            here = tree.parent(above);
            guard -= 1;
            if guard == 0 {
                return Vec::new();
            }
        }
        for &node in path.iter().rev() {
            if !self.open.contains(&node) || tree.children(node).is_empty() {
                return vec![node];
            }
        }
        // Every container down to it is open, so it is drawn as what it holds.
        self.descend(tree, tree.children(id).iter().rev().copied().collect())
    }
}

/// Where every node in a hierarchy is drawn, and what that does to its edges.
#[derive(Clone, PartialEq, Debug)]
pub struct Lift {
    to: Vec<usize>,
}

impl Lift {
    /// The card a node is drawn as, or `None` for an id the tree does not have.
    pub fn of(&self, id: usize) -> Option<usize> {
        self.to.get(id).copied().filter(|&card| card != usize::MAX)
    }

    /// The graph's edges, aimed at the cards actually on the pane and gathered
    /// so that one pair of cards is one wire.
    ///
    /// **Edges inside a card are kept**, as a bundle whose two ends are the same
    /// card. That is not a wire — a lens should not draw it — but it is the
    /// answer to "how much of this happens in here", which is worth stating on
    /// the card rather than discarding. Filter on `from == to` to draw.
    ///
    /// **Give edges only to nodes you never open.** A node the reader has opened
    /// is no longer a card — it *is* its children — so an edge on it has nowhere
    /// on the pane to land, and it is dropped. There is no honest alternative:
    /// picking one of the children would invent a relationship the graph does not
    /// have. In practice this costs nothing, because a hierarchy's edges belong
    /// to its leaves; where the hierarchy disagrees, reshape it so the thing that
    /// has edges holds nothing.
    ///
    /// Sorted, so the same graph gives the same drawing every time.
    pub fn bundle(&self, edges: &[(usize, usize)]) -> Vec<Bundle> {
        let mut weight: HashMap<(usize, usize), usize> = HashMap::new();
        for &(from, to) in edges {
            let (Some(from), Some(to)) = (self.of(from), self.of(to)) else {
                continue;
            };
            *weight.entry((from, to)).or_insert(0) += 1;
        }
        let mut out: Vec<Bundle> = weight
            .into_iter()
            .map(|((from, to), weight)| Bundle { from, to, weight })
            .collect();
        out.sort_unstable_by_key(|bundle| (bundle.from, bundle.to));
        out
    }
}

/// Every edge between one pair of cards, as one wire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bundle {
    pub from: usize,
    pub to: usize,
    /// How many of the hierarchy's own edges this stands for. `1` when nothing
    /// was gathered, which is what makes a flat graph a special case of this
    /// one rather than a different code path.
    pub weight: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two crates. The first holds two files, each holding one function; the
    /// second holds one file holding one function.
    ///
    /// ```text
    /// 0 ─┬─ 1 ── 2          3 ── 4 ── 5
    ///    └─ 6 ── 7
    /// ```
    fn program() -> Forest {
        Forest::new(vec![
            vec![1, 6],
            vec![2],
            vec![],
            vec![4],
            vec![5],
            vec![],
            vec![7],
            vec![],
        ])
    }

    const CALLS: [(usize, usize); 4] = [(2, 5), (7, 5), (2, 7), (7, 2)];

    #[test]
    fn a_folded_hierarchy_draws_its_roots() {
        let tree = program();
        assert_eq!(Nest::new().frontier(&tree), vec![0, 3]);
    }

    #[test]
    fn opening_a_container_puts_its_children_where_it_was() {
        let tree = program();
        let mut nest = Nest::new();
        nest.open(0);
        assert_eq!(
            nest.frontier(&tree),
            vec![1, 6, 3],
            "the opened crate's files take its place in the order, and the crate \
             after it does not move"
        );
    }

    #[test]
    fn opening_a_leaf_changes_nothing() {
        let tree = program();
        let mut nest = Nest::new();
        nest.open(0);
        nest.open(1);
        nest.open(2);
        assert_eq!(nest.frontier(&tree), vec![2, 6, 3], "2 has nothing inside it");
    }

    /// A container opened while its own parent is folded contributes nothing
    /// now and everything later. Opening is remembered, so a reader who folds a
    /// crate and opens it again is back where they were reading.
    #[test]
    fn an_open_container_inside_a_folded_one_waits_its_turn() {
        let tree = program();
        let mut nest = Nest::new();
        nest.open(1);
        assert_eq!(nest.frontier(&tree), vec![0, 3]);
        nest.open(0);
        assert_eq!(nest.frontier(&tree), vec![2, 6, 3]);
    }

    #[test]
    fn folding_is_the_exact_inverse_of_opening() {
        let tree = program();
        let nest = Nest::to_depth(&tree, 9);
        let whole = nest.frontier(&tree);
        assert_eq!(whole, vec![2, 7, 5], "every leaf, and nothing else");

        for id in 0..tree.len() {
            let mut probe = nest.clone();
            probe.toggle(id);
            probe.toggle(id);
            assert_eq!(
                probe.frontier(&tree),
                whole,
                "folding {id} and opening it again did not come back to the same pane"
            );
        }
    }

    /// Everything the hierarchy holds is drawn somewhere, and the somewhere is
    /// always a card that is actually on the pane. The one exception is a
    /// container the reader has opened: it is no longer a card, it *is* its
    /// children, so there is nothing to point at and `of` says so.
    #[test]
    fn every_node_lifts_onto_a_card_that_is_on_the_pane() {
        let tree = program();
        for depth in 0..4 {
            let nest = Nest::to_depth(&tree, depth);
            let frontier: HashSet<usize> = nest.frontier(&tree).into_iter().collect();
            let lift = nest.lift(&tree);
            for id in 0..tree.len() {
                match lift.of(id) {
                    Some(card) => assert!(
                        frontier.contains(&card),
                        "at depth {depth}, {id} lifts onto {card}, which is not on the pane"
                    ),
                    None => assert!(
                        nest.is_open(id) && !tree.children(id).is_empty(),
                        "at depth {depth}, {id} is drawn nowhere and is not an opened container"
                    ),
                }
            }
        }
    }

    /// The property the whole idea rests on: hiding detail moves edges, it never
    /// loses them.
    #[test]
    fn lifting_conserves_every_edge() {
        let tree = program();
        for depth in 0..4 {
            let nest = Nest::to_depth(&tree, depth);
            let total: usize = nest
                .lift(&tree)
                .bundle(&CALLS)
                .iter()
                .map(|bundle| bundle.weight)
                .sum();
            assert_eq!(
                total,
                CALLS.len(),
                "at depth {depth} the drawing accounts for {total} of {} calls",
                CALLS.len()
            );
        }
    }

    #[test]
    fn edges_between_two_hidden_nodes_become_one_weighted_wire() {
        let tree = program();
        let wires = Nest::new().lift(&tree).bundle(&CALLS);
        // 2→5 and 7→5 both cross from the first crate to the second.
        let across = wires.iter().find(|w| (w.from, w.to) == (0, 3)).unwrap();
        assert_eq!(across.weight, 2);
        // 2→7 and 7→2 are both inside the first crate: one bundle, both ends the
        // same card, and a lens draws no wire for it.
        let inside = wires.iter().find(|w| (w.from, w.to) == (0, 0)).unwrap();
        assert_eq!(inside.weight, 2);
    }

    #[test]
    fn a_cycle_between_cards_stays_two_wires() {
        let tree = program();
        let mut nest = Nest::new();
        nest.open(0);
        let wires = nest.lift(&tree).bundle(&CALLS);
        assert_eq!(wires.iter().find(|w| (w.from, w.to) == (1, 6)).unwrap().weight, 1);
        assert_eq!(wires.iter().find(|w| (w.from, w.to) == (6, 1)).unwrap().weight, 1);
    }

    #[test]
    fn revealing_a_deep_node_opens_everything_above_it() {
        let tree = program();
        let mut nest = Nest::new();
        nest.reveal(&tree, 7);
        assert!(nest.frontier(&tree).contains(&7));
        assert!(!nest.is_open(7), "the node itself is not opened, only reached");
    }

    #[test]
    fn projecting_follows_the_reader_down() {
        let tree = program();
        let mut nest = Nest::new();
        assert_eq!(nest.project(&tree, 0), vec![0], "folded: the crate itself");
        assert_eq!(nest.project(&tree, 2), vec![0], "inside it: the card holding it");
        nest.open(0);
        assert_eq!(
            nest.project(&tree, 0),
            vec![1, 6],
            "opened: everything it became"
        );
        assert_eq!(nest.project(&tree, 2), vec![1]);
    }

    #[test]
    fn a_stale_id_is_empty_rather_than_a_panic() {
        let tree = program();
        assert!(tree.children(99).is_empty());
        assert_eq!(tree.parent(99), None);
        assert_eq!(Nest::new().project(&tree, 99), Vec::<usize>::new());
        assert_eq!(Nest::new().lift(&tree).of(99), None);
    }

    /// An edge on a container that the reader then opens has nowhere to land,
    /// and is dropped. Pinned as a test rather than left to be discovered:
    /// `bundle`'s contract is that only nodes you never open carry edges, and a
    /// contract with no test is a hope.
    #[test]
    fn an_edge_on_an_opened_container_has_nowhere_to_land() {
        let tree = program();
        let edges = [(0, 3), (2, 5)];

        let folded = Nest::new().lift(&tree).bundle(&edges);
        assert_eq!(
            folded.iter().map(|w| w.weight).sum::<usize>(),
            2,
            "folded, both edges are on the drawing"
        );

        let mut nest = Nest::new();
        nest.open(0);
        let opened = nest.lift(&tree).bundle(&edges);
        assert_eq!(
            opened.iter().map(|w| w.weight).sum::<usize>(),
            1,
            "0 is no longer a card, so the edge it owned is gone"
        );
        assert_eq!(nest.lift(&tree).of(0), None, "and it says so");
    }

    #[test]
    fn a_flat_graph_is_the_ordinary_case_of_this_one() {
        let tree = Forest::new(vec![vec![], vec![], vec![]]);
        let nest = Nest::new();
        assert_eq!(nest.frontier(&tree), vec![0, 1, 2]);
        let wires = nest.lift(&tree).bundle(&[(0, 1), (1, 2)]);
        assert!(wires.iter().all(|wire| wire.weight == 1));
    }
}
