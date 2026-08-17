//! The neighbourhood of one crate: how much of the workspace sits on each side
//! of it, what is immediately attached, and by which chain the workspace reached
//! it.
//!
//! Nothing here returns a coordinate. A selection produces facts, and facts are
//! ink and words; where a card lands is the pane's business.
//!
//! All of it runs on the client, on data already there, because a round trip per
//! click is latency the interaction cannot afford.

use std::collections::VecDeque;

use super::Workspace;

/// How much of the workspace stands on each side of one crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reach {
    pub selected: usize,
    /// Every crate that reaches the selection, at any distance.
    ///
    /// From the reachability closure itself, deliberately *not* from hop counts.
    /// Longest-path levels put an immediate dependent that is also reachable by a
    /// longer route in a further column, so counting off levels lands the same
    /// crate in two buckets at once. That is how a crate with 41 dependents came
    /// to report "23 directly · 23 further out".
    pub total_dependents: usize,
    pub total_dependencies: usize,
}

/// Walk the closure both ways. A DAG keeps the two sets disjoint, so a crate is
/// never counted on both sides.
pub fn reach(workspace: &Workspace, selected: usize) -> Reach {
    let n = workspace.crates.len();

    let walk = |step: fn(&super::Crate) -> &Vec<usize>| -> usize {
        let mut seen = vec![false; n];
        let mut queue = VecDeque::from([selected]);
        let mut count = 0usize;
        while let Some(id) = queue.pop_front() {
            for &next in step(&workspace.crates[id]) {
                if !seen[next] {
                    seen[next] = true;
                    count += 1;
                    queue.push_back(next);
                }
            }
        }
        count
    };

    Reach {
        selected,
        total_dependents: walk(|entry| &entry.dependents),
        total_dependencies: walk(|entry| &entry.deps),
    }
}

/// The shortest chain of crates from a workspace member down to `target`.
///
/// This one line is the answer to "why is this here", and it is a plain BFS over
/// data already on the client. Read as text it beats any amount of tracing a
/// picture by eye — which is the comparison `cargo tree -i` wins by default and
/// this tool has to stop losing.
pub fn shortest_path_from_root(workspace: &Workspace, target: usize) -> Vec<usize> {
    let n = workspace.crates.len();
    let mut came_from = vec![usize::MAX; n];
    let mut seen = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for entry in workspace.crates.iter().filter(|entry| entry.is_root) {
        seen[entry.id] = true;
        queue.push_back(entry.id);
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
        for &next in &workspace.crates[current].deps {
            if !seen[next] {
                seen[next] = true;
                came_from[next] = current;
                queue.push_back(next);
            }
        }
    }
    Vec::new()
}

/// Crates one hop away, split by direction and ordered busiest first. This is
/// both what the record lists and what stepping with the arrow keys walks, so
/// the order the reader sees is the order they travel in.
pub fn immediate(workspace: &Workspace, selected: usize) -> (Vec<usize>, Vec<usize>) {
    let entry = &workspace.crates[selected];
    let mut dependents = entry.dependents.clone();
    let mut dependencies = entry.deps.clone();
    let busiest = |ids: &mut Vec<usize>| {
        ids.sort_by_key(|&i| {
            (
                std::cmp::Reverse(workspace.crates[i].dependents.len()),
                workspace.crates[i].name.clone(),
            )
        });
    };
    busiest(&mut dependents);
    busiest(&mut dependencies);
    (dependents, dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{build as graph_build, metadata};

    fn real() -> Workspace {
        let resolved = metadata::resolve().expect("cargo metadata should resolve this workspace");
        graph_build::build(resolved.packages, resolved.workspace, resolved.manifest_dir)
    }

    fn entry(workspace: &Workspace, name: &str) -> usize {
        workspace
            .crates
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("{name} should be in this workspace"))
            .id
    }

    /// Reach and rank have to agree: the graph promises dependents are left and
    /// dependencies right, and this is what the reader uses to believe it.
    #[test]
    fn direction_agrees_with_which_column_a_crate_is_in() {
        let workspace = real();
        let id = entry(&workspace, "serde");
        let selected = &workspace.crates[id];
        for &dependent in &selected.dependents {
            assert!(
                workspace.crates[dependent].rank < selected.rank,
                "{} depends on serde but sits right of it",
                workspace.crates[dependent].name
            );
        }
        for &dependency in &selected.deps {
            assert!(
                workspace.crates[dependency].rank > selected.rank,
                "{} is depended on by serde but sits left of it",
                workspace.crates[dependency].name
            );
        }
    }

    /// The counting bug this replaced: "direct" came from the crate and "further
    /// out" came from longest-path levels, so a crate that was both an immediate
    /// dependent *and* reachable by a longer route was counted twice. The buckets
    /// have to partition the total exactly.
    #[test]
    fn direct_and_further_out_partition_the_total() {
        let workspace = real();
        for name in ["serde", "syn", "quote", "libc"] {
            let id = entry(&workspace, name);
            let view = reach(&workspace, id);
            let (dependents, dependencies) = immediate(&workspace, id);

            assert!(
                view.total_dependents >= dependents.len(),
                "{name} lists {} direct dependents but totals only {}",
                dependents.len(),
                view.total_dependents
            );
            assert!(
                view.total_dependencies >= dependencies.len(),
                "{name} lists {} direct dependencies but totals only {}",
                dependencies.len(),
                view.total_dependencies
            );
        }
    }

    /// A closure counts each crate once however many routes reach it, and the
    /// two sides of a DAG never overlap.
    #[test]
    fn the_two_sides_never_count_the_same_crate() {
        let workspace = real();
        for name in ["serde", "syn", "quote"] {
            let id = entry(&workspace, name);
            let view = reach(&workspace, id);
            assert!(view.total_dependents + view.total_dependencies < workspace.crates.len());
        }
    }

    /// The path is the answer to "why is this here", so it must be a real chain
    /// of dependencies starting at something the workspace actually builds.
    #[test]
    fn the_why_path_is_a_real_chain() {
        let workspace = real();
        for name in ["serde", "syn", "quote", "libc"] {
            let target = entry(&workspace, name);
            let path = shortest_path_from_root(&workspace, target);
            assert!(!path.is_empty(), "{name} is reachable, so it has a path");
            assert!(
                workspace.crates[path[0]].is_root,
                "the path to {name} starts at a workspace member"
            );
            assert_eq!(*path.last().unwrap(), target);
            for pair in path.windows(2) {
                assert!(
                    workspace.crates[pair[0]].deps.contains(&pair[1]),
                    "{} does not actually depend on {}",
                    workspace.crates[pair[0]].name,
                    workspace.crates[pair[1]].name
                );
            }
        }
    }

    /// The record's lists, and what the arrow keys walk, come from the crate
    /// itself — never from hop counts.
    #[test]
    fn immediate_matches_the_crate_itself() {
        let workspace = real();
        let id = entry(&workspace, "serde");
        let (dependents, dependencies) = immediate(&workspace, id);
        assert_eq!(dependents.len(), workspace.crates[id].dependents.len());
        assert_eq!(dependencies.len(), workspace.crates[id].deps.len());
        for &d in &dependents {
            assert!(workspace.crates[d].deps.contains(&id));
        }
        for &d in &dependencies {
            assert!(workspace.crates[d].dependents.contains(&id));
        }
        // Busiest first, so a step lands somewhere worth landing.
        let counts: Vec<usize> = dependents
            .iter()
            .map(|&i| workspace.crates[i].dependents.len())
            .collect();
        assert!(
            counts.windows(2).all(|pair| pair[0] >= pair[1]),
            "dependents are not ordered busiest first"
        );
    }

    #[test]
    fn a_leaf_still_has_a_neighbourhood() {
        let workspace = real();
        let leaf = workspace
            .crates
            .iter()
            .find(|entry| entry.deps.is_empty() && !entry.dependents.is_empty())
            .expect("this workspace has leaf crates");
        let view = reach(&workspace, leaf.id);
        assert_eq!(view.total_dependencies, 0, "a leaf depends on nothing");
        assert!(
            view.total_dependents > 0,
            "{} has dependents, so its neighbourhood is not empty",
            leaf.name
        );
    }
}
