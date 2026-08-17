//! Columns for a graph that is not a DAG.
//!
//! [`Flow`](crate::Flow) asks the host which column each node belongs in,
//! because in most charts that is a fact the host already has. A call graph is
//! the case where it is not: two functions can call each other, so "how far
//! along is this" has no answer at all until the cycles are dealt with.
//!
//! [`rank`] deals with them the only way that keeps the reading honest. Nodes
//! that can all reach each other are one **component** and take one column —
//! saying `a` comes before `b` when `b` also comes before `a` would be a
//! statement the graph does not support. The components themselves cannot form
//! a cycle, so they get a longest-path layering, and every node takes its
//! component's.
//!
//! ```
//! use dioxus_flow::rank;
//!
//! // 0 → 1 ⇄ 2 → 3
//! let columns = rank(&[0, 1, 2, 3], &[(0, 1), (1, 2), (2, 1), (2, 3)]);
//! assert_eq!(columns[&0], 0);
//! assert_eq!(columns[&1], 1);
//! assert_eq!(columns[&2], 1, "1 and 2 call each other, so neither is first");
//! assert_eq!(columns[&3], 2);
//! ```

use std::collections::HashMap;

/// Which column each node belongs in, with each knot of mutually reachable
/// nodes counted as one step.
///
/// The layering is the *longest* route, so a node is always strictly past
/// everything that points at it, however many ways round there are, and a
/// node's column is how deep into the graph it actually sits rather than how
/// early it could be drawn.
///
/// Ids need not be dense or sorted. An edge naming a node that is not in
/// `nodes` is ignored, so a lens can hand over the edges it has without
/// filtering them against the nodes it is drawing.
///
/// Linear in nodes and edges: two depth-first passes for the components, two
/// sweeps for the layering.
pub fn rank(nodes: &[usize], edges: &[(usize, usize)]) -> HashMap<usize, i32> {
    let count = nodes.len();
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(at, &id)| (id, at)).collect();
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut inward: Vec<Vec<usize>> = vec![Vec::new(); count];
    for &(from, to) in edges {
        let (Some(&from), Some(&to)) = (index.get(&from), index.get(&to)) else {
            continue;
        };
        if from == to {
            continue;
        }
        out[from].push(to);
        inward[to].push(from);
    }

    // Kosaraju, both passes iterative: a chain of ten thousand calls must not
    // depend on the stack.
    let finished = finish_order(&out, count);
    let (component, components) = components(&inward, &finished);

    // Components come out of the second pass in topological order, so every
    // edge leaving one is read after that one's own column is final.
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); components];
    for (node, &owner) in component.iter().enumerate() {
        members[owner].push(node);
    }
    let mut ahead: Vec<Vec<usize>> = vec![Vec::new(); components];
    for owner in 0..components {
        for &node in &members[owner] {
            for &next in &out[node] {
                let theirs = component[next];
                if theirs != owner {
                    ahead[owner].push(theirs);
                }
            }
        }
        ahead[owner].sort_unstable();
        ahead[owner].dedup();
    }

    // Longest path, not shortest, and not tightened against the sinks either.
    //
    // Pulling every node as far along as its successors allow is the obvious
    // improvement — it is 19% less total wire on an 82-card call graph, and it
    // was tried. It draws a worse picture. Where a graph has a few near-universal
    // sinks, and a call graph always does because everything eventually reaches
    // `core`, tightening piles every node up against them: the far half of the
    // drawing becomes one dense stack and the near half is spent on the wires
    // getting there. Longest path spreads nodes across the whole width by their
    // actual depth, which is both more legible and the thing the column was
    // supposed to mean.
    let mut column = vec![0i32; components];
    for owner in 0..components {
        for &next in &ahead[owner] {
            column[next] = column[next].max(column[owner] + 1);
        }
    }

    nodes
        .iter()
        .enumerate()
        .map(|(at, &id)| (id, column[component[at]]))
        .collect()
}

/// Depth-first over the graph, recording each node as it is finished with.
fn finish_order(out: &[Vec<usize>], count: usize) -> Vec<usize> {
    let mut order = Vec::with_capacity(count);
    let mut seen = vec![false; count];
    // (node, how many of its edges have been walked)
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for start in 0..count {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push((start, 0));
        while let Some((node, step)) = stack.pop() {
            match out[node].get(step) {
                Some(&next) => {
                    stack.push((node, step + 1));
                    if !seen[next] {
                        seen[next] = true;
                        stack.push((next, 0));
                    }
                }
                None => order.push(node),
            }
        }
    }
    order
}

/// Depth-first over the reversed graph, taking nodes in reverse finishing
/// order. Each tree is one component, and the components come out in the order
/// the original graph runs in.
fn components(inward: &[Vec<usize>], finished: &[usize]) -> (Vec<usize>, usize) {
    let mut owner = vec![usize::MAX; inward.len()];
    let mut components = 0;
    for &start in finished.iter().rev() {
        if owner[start] != usize::MAX {
            continue;
        }
        let mut stack = vec![start];
        owner[start] = components;
        while let Some(node) = stack.pop() {
            for &back in &inward[node] {
                if owner[back] == usize::MAX {
                    owner[back] = components;
                    stack.push(back);
                }
            }
        }
        components += 1;
    }
    (owner, components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chain_is_one_column_per_step() {
        let columns = rank(&[0, 1, 2], &[(0, 1), (1, 2)]);
        assert_eq!((columns[&0], columns[&1], columns[&2]), (0, 1, 2));
    }

    /// The longest route, not the shortest: a node that can be reached in one
    /// step and in three sits at three, or the edge that took three steps would
    /// be drawn running backwards.
    #[test]
    fn a_node_sits_past_the_longest_route_to_it() {
        let columns = rank(&[0, 1, 2, 3], &[(0, 3), (0, 1), (1, 2), (2, 3)]);
        assert_eq!(columns[&3], 3);
    }

    #[test]
    fn nodes_that_reach_each_other_share_a_column() {
        let columns = rank(&[0, 1, 2, 3], &[(0, 1), (1, 2), (2, 1), (2, 3)]);
        assert_eq!(columns[&1], columns[&2], "a cycle has no first member");
        assert!(columns[&0] < columns[&1] && columns[&1] < columns[&3]);
    }

    /// A cycle spanning the whole graph is one column: there is genuinely no
    /// order, and inventing one would be a lie the drawing then tells.
    #[test]
    fn a_graph_that_is_all_one_cycle_is_one_column() {
        let columns = rank(&[0, 1, 2], &[(0, 1), (1, 2), (2, 0)]);
        assert!(columns.values().all(|&column| column == 0));
    }

    #[test]
    fn a_node_nothing_points_at_starts_at_the_left_edge() {
        let columns = rank(&[7, 8, 9], &[(7, 8)]);
        assert_eq!(columns[&7], 0);
        assert_eq!(columns[&9], 0, "an island is a beginning too");
        assert_eq!(columns.len(), 3, "ids need not be dense");
    }

    /// A node is placed by how deep it is, not by how near its successors are.
    /// The alternative — pulling each node along until it meets what it points
    /// at — is shorter in total wire and worse to look at; see the note in
    /// `rank`.
    #[test]
    fn a_node_is_placed_by_its_own_depth_rather_than_by_what_it_points_at() {
        //  0 → 1 → 2 → 3 → 4 sets the depth; 5 → 6 → 4 is a short way in to the
        //  far end of it.
        let columns = rank(
            &[0, 1, 2, 3, 4, 5, 6],
            &[(0, 1), (1, 2), (2, 3), (3, 4), (5, 6), (6, 4)],
        );
        assert_eq!(columns[&4], 4, "the chain sets the far edge");
        assert_eq!(columns[&5], 0, "a beginning is at the near edge");
        assert_eq!(columns[&6], 1, "and 6 is one step in, whatever it points at");
    }

    #[test]
    fn an_edge_naming_a_node_that_is_not_drawn_is_ignored() {
        let columns = rank(&[0, 1], &[(0, 1), (1, 99), (99, 0)]);
        assert_eq!((columns[&0], columns[&1]), (0, 1));
        assert_eq!(columns.len(), 2);
    }

    #[test]
    fn a_self_edge_does_not_push_a_node_past_itself() {
        let columns = rank(&[0, 1], &[(0, 0), (0, 1)]);
        assert_eq!((columns[&0], columns[&1]), (0, 1));
    }

    #[test]
    fn nothing_at_all_is_no_columns() {
        assert!(rank(&[], &[(0, 1)]).is_empty());
    }

    /// Every edge that is not inside a component runs forwards. This is the
    /// property the layout leans on, checked over a graph with a cycle, a
    /// diamond, an island and a back edge in it.
    #[test]
    fn every_edge_between_components_runs_forwards() {
        let nodes: Vec<usize> = (0..9).collect();
        let edges = [
            (0, 1), (1, 2), (2, 3), (3, 1), // a cycle hanging off a chain
            (0, 4), (4, 3),                 // a diamond round it
            (3, 5), (5, 6), (6, 7),         // a tail
            (7, 5),                         // and a back edge into the tail
        ];
        let columns = rank(&nodes, &edges);
        for (from, to) in edges {
            if columns[&from] == columns[&to] {
                continue; // one component: no order to check
            }
            assert!(
                columns[&from] < columns[&to],
                "{from} → {to} runs backwards, {} to {}",
                columns[&from],
                columns[&to]
            );
        }
        assert_eq!(columns[&8], 0, "the island stays at the left edge");
    }

    /// A deep chain must not depend on the call stack: both passes are
    /// iterative, and this is the size that proves it.
    #[test]
    fn a_hundred_thousand_deep_chain_does_not_blow_the_stack() {
        let nodes: Vec<usize> = (0..100_000).collect();
        let edges: Vec<(usize, usize)> = (0..99_999).map(|id| (id, id + 1)).collect();
        let columns = rank(&nodes, &edges);
        assert_eq!(columns[&99_999], 99_999);
    }
}
