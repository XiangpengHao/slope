//! The neighbourhood of one crate: how far everything else sits from it, and
//! by which chain the workspace reached it.
//!
//! This used to be a second layout engine — selecting a crate rebuilt an
//! ego-network from scratch and the board rearranged itself around it. It does
//! not any more. The board is laid out once and holds still; what a selection
//! produces is **distance**, and distance is ink and words, not geometry.
//!
//! So nothing here returns a coordinate. It answers three questions:
//! how far is each crate from this one and in which direction, what is
//! immediately attached to it, and what chain from a workspace member put it
//! on the board at all.

use std::collections::VecDeque;

use super::Board;

/// Hops lit on each side before the rest is reported as a count. Two is what a
/// person holds in mind at once, and the third hop is where a dependency graph
/// stops being about the crate you asked about.
pub const DEFAULT_DEPTH: i32 = 2;

/// Not reachable from the selection in either direction.
const UNREACHED: i32 = i32::MAX;

#[derive(Debug, Clone, PartialEq)]
pub struct Neighbourhood {
    pub selected: usize,
    /// Hops from the selection, indexed by pad id. Negative depends on the
    /// selection, positive is depended on by it, zero is the selection itself,
    /// and `UNREACHED` is unrelated. Beyond `depth` the entry is still recorded
    /// here, so the counts below can be honest about what is not lit.
    level: Vec<i32>,
    /// The depth actually lit on each side.
    pub consumer_depth: i32,
    pub producer_depth: i32,
    /// Every crate that reaches the selection, at any distance, counted from
    /// the reachability closure itself.
    ///
    /// Deliberately *not* derived from hop levels. Levels are longest-path, so
    /// an immediate dependent that is also reachable by a longer route sits in a
    /// further column — count "direct" and "within the lit depth" off levels and
    /// the same crate lands in two buckets. That is how a crate with 41
    /// dependents came to report "23 directly · 23 further out".
    pub total_consumers: usize,
    pub total_producers: usize,
}

impl Neighbourhood {
    /// Hops from the selection, or `None` when this crate is unrelated to it or
    /// sits past the lit depth. The renderer asks this per pad per frame, so it
    /// is a lookup rather than a search.
    pub fn level_of(&self, id: usize) -> Option<i32> {
        let level = *self.level.get(id)?;
        if level == UNREACHED || level < self.consumer_depth || level > self.producer_depth {
            None
        } else {
            Some(level)
        }
    }

    /// Related at any distance, lit or not. A trace to a crate past the lit
    /// depth is still that crate's real relationship to the selection.
    pub fn reaches(&self, id: usize) -> bool {
        self.level.get(id).is_some_and(|&l| l != UNREACHED)
    }
}

/// The shortest chain of crates from a workspace member down to `target`.
///
/// This one line is the answer to "why is this here", and it is a plain BFS
/// over data already on the client. Read as text it beats any amount of tracing
/// a picture by eye — which is the comparison `cargo tree -i` wins by default
/// and this tool has to stop losing.
pub fn shortest_path_from_root(board: &Board, target: usize) -> Vec<usize> {
    let n = board.pads.len();
    let mut came_from = vec![usize::MAX; n];
    let mut seen = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for pad in board.pads.iter().filter(|p| p.is_root) {
        seen[pad.id] = true;
        queue.push_back(pad.id);
    }
    while let Some(current) = queue.pop_front() {
        if current == target {
            let mut chain = vec![current];
            let mut step = current;
            while came_from[step] != usize::MAX {
                step = came_from[step];
                chain.push(step);
            }
            chain.reverse();
            return chain;
        }
        for &next in &board.pads[current].deps {
            if !seen[next] {
                seen[next] = true;
                came_from[next] = current;
                queue.push_back(next);
            }
        }
    }
    Vec::new()
}

/// Crates one hop from the selection, split by direction and ordered busiest
/// first. This is both what the answer panel lists and what stepping up or down
/// with the arrow keys walks, so the order the reader sees is the order they
/// travel in.
pub fn immediate(board: &Board, selected: usize) -> (Vec<usize>, Vec<usize>) {
    let pad = &board.pads[selected];
    let mut dependents = pad.dependents.clone();
    let mut dependencies = pad.deps.clone();
    let busiest = |ids: &mut Vec<usize>| {
        ids.sort_by_key(|&i| {
            (
                std::cmp::Reverse(board.pads[i].dependents.len()),
                board.pads[i].name.clone(),
            )
        });
    };
    busiest(&mut dependents);
    busiest(&mut dependencies);
    (dependents, dependencies)
}

pub fn build(board: &Board, selected: usize, depth: i32) -> Neighbourhood {
    let n = board.pads.len();

    // Which crates are on each side. A DAG keeps the two sets disjoint, so a
    // crate never lands in both.
    let mut consumer = vec![false; n];
    let mut queue = VecDeque::from([selected]);
    while let Some(u) = queue.pop_front() {
        for &d in &board.pads[u].dependents {
            if !consumer[d] {
                consumer[d] = true;
                queue.push_back(d);
            }
        }
    }
    let mut producer = vec![false; n];
    let mut queue = VecDeque::from([selected]);
    while let Some(u) = queue.pop_front() {
        for &d in &board.pads[u].deps {
            if !producer[d] {
                producer[d] = true;
                queue.push_back(d);
            }
        }
    }

    // Levels by *longest* path, not shortest. Two crates can both be one hop
    // from the selection and still depend on each other; shortest-path levelling
    // calls them the same distance and the relationship between them disappears.
    //
    // The board's rank is a topological order (every edge increases it), so
    // walking it in the right direction settles each side in one pass.
    let mut by_rank: Vec<usize> = (0..n).collect();
    by_rank.sort_by_key(|&i| board.pads[i].rank);

    let mut level = vec![UNREACHED; n];
    level[selected] = 0;
    // A consumer's level depends on the crates it depends on, which rank later.
    // Take the nearest of them and step one further out: a hop count must clear
    // *every* dependency's level, not just the closest one.
    for &u in by_rank.iter().rev() {
        if !consumer[u] {
            continue;
        }
        let nearest = board.pads[u]
            .deps
            .iter()
            .filter(|&&d| d == selected || consumer[d])
            .filter_map(|&d| (level[d] != UNREACHED).then_some(level[d]))
            .min();
        if let Some(nearest) = nearest {
            level[u] = nearest - 1;
        }
    }
    // A producer's level depends on the crates that depend on it, ranked earlier.
    for &v in &by_rank {
        if !producer[v] {
            continue;
        }
        let deepest = board.pads[v]
            .dependents
            .iter()
            .filter(|&&u| u == selected || producer[u])
            .filter_map(|&u| (level[u] != UNREACHED).then_some(level[u]))
            .max();
        if let Some(deepest) = deepest {
            level[v] = deepest + 1;
        }
    }

    let depth = depth.max(1);

    Neighbourhood {
        selected,
        consumer_depth: -depth,
        producer_depth: depth,
        total_consumers: consumer.iter().filter(|&&c| c).count(),
        total_producers: producer.iter().filter(|&&p| p).count(),
        level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{layout, metadata};

    fn real_board() -> Board {
        let resolved = metadata::resolve().expect("cargo metadata should resolve this workspace");
        layout::build(resolved.packages, resolved.workspace, resolved.manifest_dir)
    }

    fn pad(board: &Board, name: &str) -> usize {
        board
            .pads
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} should be in this workspace"))
            .id
    }

    /// Distance and the board's own geometry have to agree: the layout promises
    /// dependents are left and dependencies right, and the neighbourhood is what
    /// the reader uses to believe it.
    #[test]
    fn direction_agrees_with_where_the_pad_sits() {
        let board = real_board();
        let id = pad(&board, "serde");
        let view = build(&board, id, DEFAULT_DEPTH);
        let selected = &board.pads[id];

        for other in &board.pads {
            let Some(level) = view.level_of(other.id) else {
                continue;
            };
            match level.cmp(&0) {
                std::cmp::Ordering::Less => assert!(
                    other.x < selected.x,
                    "{} depends on serde but sits right of it",
                    other.name
                ),
                std::cmp::Ordering::Greater => assert!(
                    other.x > selected.x,
                    "{} is depended on by serde but sits left of it",
                    other.name
                ),
                std::cmp::Ordering::Equal => assert_eq!(other.id, id),
            }
        }
    }

    #[test]
    fn levels_are_real_hop_counts() {
        let board = real_board();
        let view = build(&board, pad(&board, "syn"), DEFAULT_DEPTH);

        for other in &board.pads {
            let Some(level) = view.level_of(other.id) else {
                continue;
            };
            if level >= 0 {
                continue;
            }
            // A consumer at level -k must depend on something at -(k-1).
            let reaches_inward = other
                .deps
                .iter()
                .any(|&d| view.level_of(d) == Some(level + 1));
            assert!(
                reaches_inward,
                "{} sits at level {level} with nothing at {} to depend on",
                other.name,
                level + 1
            );
        }
    }

    /// Depth is the reader's control, so it has to actually bound what is lit —
    /// and everything it excludes has to turn up in the counts instead of just
    /// vanishing.
    #[test]
    fn depth_bounds_what_is_lit_and_the_rest_is_counted() {
        let board = real_board();
        let id = pad(&board, "syn");
        for depth in [1, 2, 3] {
            let view = build(&board, id, depth);
            for other in &board.pads {
                if let Some(level) = view.level_of(other.id) {
                    assert!(
                        level.abs() <= depth,
                        "{} is lit at level {level} in a depth-{depth} view",
                        other.name
                    );
                }
            }
            // Depth changes what is lit, never what is counted: the totals are
            // the reachability closure and do not move with the reader's depth.
            assert_eq!(view.total_consumers, build(&board, id, 1).total_consumers);
        }
    }

    /// The counting bug this replaced: "direct" came from the crate and
    /// "further out" came from longest-path levels, so a crate that was both an
    /// immediate dependent *and* reachable by a longer route was counted twice.
    /// `bytes` reported 41 dependents and then listed "23 directly · 23 further
    /// out". The buckets have to partition the total exactly.
    #[test]
    fn direct_and_further_out_partition_the_total() {
        let board = real_board();
        for name in ["serde", "syn", "quote", "bytes", "libc", "tokio"] {
            let id = pad(&board, name);
            let view = build(&board, id, DEFAULT_DEPTH);
            let (dependents, dependencies) = immediate(&board, id);

            assert!(
                view.total_consumers >= dependents.len(),
                "{name} lists {} direct dependents but totals only {}",
                dependents.len(),
                view.total_consumers
            );
            assert!(
                view.total_producers >= dependencies.len(),
                "{name} lists {} direct dependencies but totals only {}",
                dependencies.len(),
                view.total_producers
            );

            // And every direct neighbour is inside the closure it is counted in,
            // which is what makes `total - direct` a real remainder.
            for &d in dependents.iter().chain(dependencies.iter()) {
                assert!(
                    view.reaches(d),
                    "{} is directly attached to {name} but not in its closure",
                    board.pads[d].name
                );
            }
        }
    }

    /// The path is the answer to "why is this here", so it must be a real chain
    /// of dependencies starting at something the workspace actually builds.
    #[test]
    fn the_why_path_is_a_real_chain() {
        let board = real_board();
        for name in ["serde", "syn", "quote", "libc", "memchr"] {
            let target = pad(&board, name);
            let path = shortest_path_from_root(&board, target);
            assert!(!path.is_empty(), "{name} is reachable, so it has a path");
            assert!(
                board.pads[path[0]].is_root,
                "the path to {name} starts at a workspace member"
            );
            assert_eq!(*path.last().unwrap(), target);
            for pair in path.windows(2) {
                assert!(
                    board.pads[pair[0]].deps.contains(&pair[1]),
                    "{} does not actually depend on {}",
                    board.pads[pair[0]].name,
                    board.pads[pair[1]].name
                );
            }
        }
    }

    /// The panel's "direct" lists, and what the arrow keys walk, come from the
    /// crate itself — never from levels, which are longest-path and so put an
    /// immediate dependent reachable by a longer route in a further column.
    #[test]
    fn immediate_matches_the_crate_itself() {
        let board = real_board();
        let id = pad(&board, "serde");
        let (dependents, dependencies) = immediate(&board, id);
        assert_eq!(dependents.len(), board.pads[id].dependents.len());
        assert_eq!(dependencies.len(), board.pads[id].deps.len());
        for &d in &dependents {
            assert!(board.pads[d].deps.contains(&id));
        }
        for &d in &dependencies {
            assert!(board.pads[d].dependents.contains(&id));
        }
        // Busiest first, so stepping up lands somewhere worth landing.
        let counts: Vec<usize> = dependents
            .iter()
            .map(|&i| board.pads[i].dependents.len())
            .collect();
        assert!(
            counts.windows(2).all(|w| w[0] >= w[1]),
            "dependents are not ordered busiest first"
        );
    }

    #[test]
    fn a_leaf_still_has_a_neighbourhood() {
        let board = real_board();
        let leaf = board
            .pads
            .iter()
            .find(|p| p.deps.is_empty() && !p.dependents.is_empty())
            .expect("this workspace has leaf crates");
        let view = build(&board, leaf.id, DEFAULT_DEPTH);
        assert_eq!(view.total_producers, 0, "a leaf depends on nothing");
        assert!(
            view.total_consumers > 0,
            "{} has dependents, so its neighbourhood is not empty",
            leaf.name
        );
    }

    #[test]
    fn report_neighbourhood_shape() {
        let board = real_board();
        for name in ["serde", "syn", "quote", "tokio"] {
            let view = build(&board, pad(&board, name), DEFAULT_DEPTH);
            let (dependents, dependencies) = immediate(&board, pad(&board, name));
            println!(
                "{name}: {} in ({} direct), {} out ({} direct)",
                view.total_consumers,
                dependents.len(),
                view.total_producers,
                dependencies.len(),
            );
        }
    }
}
