//! Folding: which of a graph's nodes are on the pane, and which are behind a
//! port waiting to be asked for.
//!
//! A pane that draws a whole graph is only useful while the graph is small. Past
//! a few hundred nodes the picture stops being a flow chart and becomes a
//! texture, and the browser starts paying for elements nobody is reading. So a
//! lens hands over the *whole* graph as adjacency and keeps a [`Folding`]: the
//! nodes it started from, and which nodes have been opened in which direction.
//! What lands on the pane is derived from those, never stored.
//!
//! Derived rather than stored is the whole point. A flat set of visible ids has
//! no answer to "what does folding this take away" — the nodes reachable only
//! through it have to come off too, and nothing in a flat set says which those
//! are. Walking from the seeds every time is what makes folding the exact
//! inverse of opening, at any depth, in any order.
//!
//! ```
//! use dioxus_flow::{Adjacency, Folding, Way};
//!
//! // 0 → 1 → 2, and 0 → 3.
//! let links = Adjacency::from_out(vec![vec![1, 3], vec![2], vec![], vec![]]);
//! let mut folding = Folding::to_depth(&links, vec![0], 1, Way::Out);
//!
//! // One hop: everything 0 points at, and nothing past that.
//! assert_eq!(folding.visible(&links), vec![0, 1, 3]);
//! assert_eq!(folding.folded(&links, 1, Way::Out), 1);
//!
//! // Opening the rim brings the next hop, and folding it takes it back.
//! folding.toggle(1, Way::Out);
//! assert_eq!(folding.visible(&links), vec![0, 1, 2, 3]);
//! folding.toggle(1, Way::Out);
//! assert_eq!(folding.visible(&links), vec![0, 1, 3]);
//! ```

use std::collections::{HashMap, HashSet};

use super::Way;

/// A graph's adjacency, both ways.
///
/// A trait rather than a container, because the host already has this. A lens
/// holding a workspace of crates, each with its dependencies and dependents,
/// implements two methods and is done; copying that into a structure this crate
/// owns would mean rebuilding the whole graph on every render, which on a large
/// workspace is more work than the layout it feeds. [`Adjacency`] is here for
/// hosts that have nothing to point at yet.
///
/// Both directions are required because a pane walks both: one port opens what a
/// node points at, the other opens what points at it.
pub trait Links {
    /// The number of nodes, which is one past the largest id.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// What this node is attached to, this way round.
    ///
    /// Must return empty for an id the graph does not have, so that a stale id
    /// from a previous reading cannot panic a pane.
    fn neighbours(&self, id: usize, way: Way) -> &[usize];
}

/// Adjacency this crate owns, for a host that has no structure of its own to
/// lend. A host that does should implement [`Links`] on it instead.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Adjacency {
    out: Vec<Vec<usize>>,
    inward: Vec<Vec<usize>>,
}

impl Adjacency {
    /// Both directions, already known. Each inner `Vec` is the neighbours of the
    /// node at that index.
    pub fn new(out: Vec<Vec<usize>>, inward: Vec<Vec<usize>>) -> Self {
        Self { out, inward }
    }

    /// Outgoing edges only; the incoming ones are inverted from them. Exact for
    /// any graph whose edges are all present, which is the usual case — a host
    /// with genuinely one-sided knowledge should use [`Adjacency::new`] and say
    /// so rather than letting this invent the other side.
    pub fn from_out(out: Vec<Vec<usize>>) -> Self {
        let mut inward = vec![Vec::new(); out.len()];
        for (from, targets) in out.iter().enumerate() {
            for &to in targets {
                if to < inward.len() {
                    inward[to].push(from);
                }
            }
        }
        Self { out, inward }
    }
}

impl Links for Adjacency {
    fn len(&self) -> usize {
        self.out.len()
    }

    fn neighbours(&self, id: usize, way: Way) -> &[usize] {
        let side = match way {
            Way::Out => &self.out,
            Way::In => &self.inward,
        };
        side.get(id).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// What the reader has opened.
///
/// Holds intent, not results: the seeds it starts from and the nodes whose ports
/// are open. Everything else is computed from those against a [`Links`].
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Folding {
    seeds: Vec<usize>,
    out: HashSet<usize>,
    inward: HashSet<usize>,
}

impl Folding {
    /// Start from these nodes with every port folded.
    pub fn new(seeds: Vec<usize>) -> Self {
        Self {
            seeds,
            ..Self::default()
        }
    }

    /// Start from these nodes with everything within `depth` hops already open.
    ///
    /// `depth` is how many hops of graph the reader sees, so the ports that are
    /// open are the ones strictly nearer than that — a node *at* the rim is on
    /// the pane with its own port still folded, which is what makes the rim
    /// something you can open rather than a wall.
    pub fn to_depth(links: &impl Links, seeds: Vec<usize>, depth: usize, way: Way) -> Self {
        let reach = depths(links, &seeds, way);
        let open: HashSet<usize> = reach
            .iter()
            .filter(|&(_, &hops)| hops < depth)
            .map(|(&id, _)| id)
            .collect();
        let mut folding = Self::new(seeds);
        match way {
            Way::Out => folding.out = open,
            Way::In => folding.inward = open,
        }
        folding
    }

    /// The nodes every reading starts from. Always on the pane, whatever is
    /// folded.
    pub fn seeds(&self) -> &[usize] {
        &self.seeds
    }

    pub fn set_seeds(&mut self, seeds: Vec<usize>) {
        self.seeds = seeds;
    }

    fn side(&mut self, way: Way) -> &mut HashSet<usize> {
        match way {
            Way::Out => &mut self.out,
            Way::In => &mut self.inward,
        }
    }

    /// Is this node's port open this way round?
    pub fn is_open(&self, id: usize, way: Way) -> bool {
        match way {
            Way::Out => self.out.contains(&id),
            Way::In => self.inward.contains(&id),
        }
    }

    /// Open one side of a node.
    pub fn open(&mut self, id: usize, way: Way) {
        self.side(way).insert(id);
    }

    /// Fold one side of a node. What was reachable only through it leaves the
    /// pane, because [`Folding::visible`] walks rather than remembers.
    pub fn fold(&mut self, id: usize, way: Way) {
        self.side(way).remove(&id);
    }

    /// Fold what is open, open what is folded. The one gesture a port offers.
    pub fn toggle(&mut self, id: usize, way: Way) {
        let side = self.side(way);
        if !side.remove(&id) {
            side.insert(id);
        }
    }

    /// Everything on the pane, in ascending id order.
    ///
    /// A closure from the seeds rather than a flat set: folding has to take away
    /// whatever was reachable only through what was folded, and only the walk
    /// knows what that is.
    pub fn visible(&self, links: &impl Links) -> Vec<usize> {
        let mut seen: HashSet<usize> = HashSet::new();
        let mut order: Vec<usize> = Vec::new();
        let mut queue: Vec<usize> = Vec::new();
        for &seed in &self.seeds {
            if seed < links.len() && seen.insert(seed) {
                order.push(seed);
                queue.push(seed);
            }
        }
        while let Some(id) = queue.pop() {
            for way in [Way::Out, Way::In] {
                if !self.is_open(id, way) {
                    continue;
                }
                for &next in links.neighbours(id, way) {
                    if seen.insert(next) {
                        order.push(next);
                        queue.push(next);
                    }
                }
            }
        }
        order.sort_unstable();
        order
    }

    /// How many of this node's neighbours *this way* are not on the pane.
    ///
    /// The number a port states when it offers to open. Distinct from the node's
    /// total degree on purpose: a lens may want to state either, and only this
    /// one answers "how much would arrive if I clicked".
    pub fn folded(&self, links: &impl Links, id: usize, way: Way) -> usize {
        let here: HashSet<usize> = self.visible(links).into_iter().collect();
        links
            .neighbours(id, way)
            .iter()
            .filter(|next| !here.contains(next))
            .count()
    }
}

/// Hops from the nearest seed, following `way`, by shortest route.
///
/// Shortest rather than longest: something a seed points at directly is one hop
/// away even when a longer chain also reaches it. Nodes no seed can reach are
/// absent from the map rather than present at some sentinel distance.
pub fn depths(links: &impl Links, seeds: &[usize], way: Way) -> HashMap<usize, usize> {
    let mut depth: HashMap<usize, usize> = HashMap::new();
    let mut frontier: Vec<usize> = Vec::new();
    for &seed in seeds {
        if seed < links.len() && depth.insert(seed, 0).is_none() {
            frontier.push(seed);
        }
    }
    let mut hop = 0;
    while !frontier.is_empty() {
        hop += 1;
        let mut next = Vec::new();
        for id in frontier.drain(..) {
            for &neighbour in links.neighbours(id, way) {
                // First arrival only. Writing unconditionally would let a longer
                // chain found later overwrite the short one, and a node a seed
                // points straight at would be recorded four hops out because
                // something else also reaches it the long way round.
                if let std::collections::hash_map::Entry::Vacant(slot) = depth.entry(neighbour) {
                    slot.insert(hop);
                    next.push(neighbour);
                }
            }
        }
        frontier = next;
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 → 1 → 2 → 3, with 0 → 3 as well.
    fn diamond() -> Adjacency {
        Adjacency::from_out(vec![vec![1, 3], vec![2], vec![3], vec![]])
    }

    #[test]
    fn inverting_the_edges_agrees_with_the_forward_ones() {
        let links = diamond();
        for from in 0..links.len() {
            for &to in links.neighbours(from, Way::Out) {
                assert!(
                    links.neighbours(to, Way::In).contains(&from),
                    "{from} → {to} is missing from the reverse edges"
                );
            }
        }
        let forward: usize = (0..links.len())
            .map(|id| links.neighbours(id, Way::Out).len())
            .sum();
        let backward: usize = (0..links.len())
            .map(|id| links.neighbours(id, Way::In).len())
            .sum();
        assert_eq!(forward, backward, "inverting invented or lost an edge");
    }

    #[test]
    fn depth_is_the_shortest_route_not_the_longest() {
        let reach = depths(&diamond(), &[0], Way::Out);
        assert_eq!(reach[&0], 0);
        assert_eq!(reach[&1], 1);
        assert_eq!(reach[&2], 2);
        // Reachable at 1 directly and at 3 the long way. One is the answer.
        assert_eq!(reach[&3], 1, "the direct edge is the shortest route");
    }

    #[test]
    fn a_node_no_seed_reaches_is_absent_rather_than_distant() {
        let links = Adjacency::from_out(vec![vec![1], vec![], vec![]]);
        let reach = depths(&links, &[0], Way::Out);
        assert!(!reach.contains_key(&2), "2 is unreachable from 0");
    }

    #[test]
    fn opening_to_a_depth_puts_the_rim_on_the_pane_with_its_port_folded() {
        let links = diamond();
        let folding = Folding::to_depth(&links, vec![0], 1, Way::Out);
        assert_eq!(folding.visible(&links), vec![0, 1, 3]);
        assert!(folding.is_open(0, Way::Out), "the seed is open");
        assert!(
            !folding.is_open(1, Way::Out),
            "a node at the rim is on the pane with its own port still folded"
        );
    }

    #[test]
    fn folding_takes_away_what_was_only_reachable_through_it() {
        let links = diamond();
        let mut folding = Folding::to_depth(&links, vec![0], 4, Way::Out);
        assert_eq!(folding.visible(&links), vec![0, 1, 2, 3]);

        folding.fold(1, Way::Out);
        // 2 was only reachable through 1, so it goes. 3 stays: 0 points at it.
        assert_eq!(folding.visible(&links), vec![0, 1, 3]);
    }

    #[test]
    fn folding_is_the_exact_inverse_of_opening() {
        let links = diamond();
        let folding = Folding::to_depth(&links, vec![0], 4, Way::Out);
        let whole = folding.visible(&links);

        for id in 0..links.len() {
            let mut probe = folding.clone();
            probe.toggle(id, Way::Out);
            probe.toggle(id, Way::Out);
            assert_eq!(
                probe.visible(&links),
                whole,
                "folding {id} and opening it again did not come back to the same pane"
            );
        }
    }

    #[test]
    fn a_cycle_does_not_hang_the_walk() {
        let links = Adjacency::from_out(vec![vec![1], vec![2], vec![0]]);
        let folding = Folding::to_depth(&links, vec![0], 9, Way::Out);
        assert_eq!(folding.visible(&links), vec![0, 1, 2]);
        assert_eq!(depths(&links, &[0], Way::Out)[&2], 2);
    }

    #[test]
    fn a_port_counts_what_is_actually_still_folded() {
        let links = diamond();
        let folding = Folding::to_depth(&links, vec![0], 1, Way::Out);
        // 1 points only at 2, which is off the pane.
        assert_eq!(folding.folded(&links, 1, Way::Out), 1);
        // 0's targets are both already on the pane, so opening it reveals none.
        assert_eq!(folding.folded(&links, 0, Way::Out), 0);
    }

    #[test]
    fn a_stale_id_is_empty_rather_than_a_panic() {
        let links = diamond();
        assert!(links.neighbours(99, Way::Out).is_empty());
        let folding = Folding::new(vec![99]);
        assert!(folding.visible(&links).is_empty());
    }

    #[test]
    fn seeds_are_on_the_pane_with_everything_folded() {
        let links = diamond();
        let folding = Folding::new(vec![0, 2]);
        assert_eq!(folding.visible(&links), vec![0, 2]);
    }

    #[test]
    fn the_two_directions_are_independent() {
        let links = diamond();
        let mut folding = Folding::new(vec![2]);
        folding.open(2, Way::In);
        // 1 and 0 both reach 2: 1 directly, 0 through the walk from 1? No —
        // only 1 points at 2, so opening 2 inward brings 1 and nothing else.
        assert_eq!(folding.visible(&links), vec![1, 2]);
        assert!(!folding.is_open(2, Way::Out), "the other side is untouched");
    }
}
