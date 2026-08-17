//! Ranking, and the graph the client is handed.
//!
//! Rank is longest path from the workspace members. It is computed once,
//! server-side, and it is the only spatial fact in the payload — everything a
//! crate depends on has a strictly greater rank, so a lens can put dependencies
//! to the right and dependents to the left without ever consulting a layout.
//!
//! A shortest-path rank would put `serde` one column from the workspace because
//! *something* depends on it directly, even where twenty other routes reach it
//! four hops out. Longest path is what makes a column a real depth.

use std::collections::VecDeque;

use super::{Crate, DuplicateGroup, RawPackage, Workspace};

pub fn build(packages: Vec<RawPackage>, name: String, manifest_dir: String) -> Workspace {
    let n = packages.len();
    let ranks = rank(&packages);

    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, package) in packages.iter().enumerate() {
        for &dep in &package.deps {
            dependents[dep].push(i);
        }
    }

    let duplicates = duplicate_groups(&packages);
    let mut duplicated = vec![false; n];
    for group in &duplicates {
        for &id in &group.ids {
            duplicated[id] = true;
        }
    }

    let dependency_count = packages.iter().map(|package| package.deps.len()).sum();

    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let distinct_count = names.len();

    let crates: Vec<Crate> = packages
        .iter()
        .enumerate()
        .map(|(i, package)| Crate {
            id: i,
            name: package.name.clone(),
            version: package.version.clone(),
            rank: ranks[i],
            deps: package.deps.clone(),
            dependents: std::mem::take(&mut dependents[i]),
            is_root: package.is_root,
            duplicate: duplicated[i],
            repository: package.repository.clone(),
            registry: package.registry,
        })
        .collect();

    Workspace {
        name,
        manifest_dir,
        package_count: n,
        distinct_count,
        dependency_count,
        crates,
        duplicates,
    }
}

/// Longest-path ranking, so every dependency points forward and no crate sits in
/// the same column as something it depends on.
fn rank(packages: &[RawPackage]) -> Vec<usize> {
    let n = packages.len();
    let mut indegree = vec![0usize; n];
    for package in packages {
        for &dep in &package.deps {
            indegree[dep] += 1;
        }
    }
    let mut ranks = vec![0usize; n];
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut settled = 0usize;
    while let Some(u) = queue.pop_front() {
        settled += 1;
        for &dep in &packages[u].deps {
            ranks[dep] = ranks[dep].max(ranks[u] + 1);
            indegree[dep] -= 1;
            if indegree[dep] == 0 {
                queue.push_back(dep);
            }
        }
    }
    // Dev edges are already filtered out, so a cycle here would be unusual.
    // Place anything left over past its deepest settled predecessor rather than
    // dropping it out of the graph.
    if settled < n {
        for i in 0..n {
            if indegree[i] > 0 {
                let deepest = (0..n)
                    .filter(|&j| packages[j].deps.contains(&i))
                    .map(|j| ranks[j])
                    .max()
                    .unwrap_or(0);
                ranks[i] = ranks[i].max(deepest + 1);
            }
        }
    }
    ranks
}

fn duplicate_groups(packages: &[RawPackage]) -> Vec<DuplicateGroup> {
    let mut by_name: std::collections::BTreeMap<&str, Vec<usize>> = Default::default();
    for (i, package) in packages.iter().enumerate() {
        by_name.entry(package.name.as_str()).or_default().push(i);
    }
    by_name
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(name, ids)| DuplicateGroup {
            name: name.to_string(),
            versions: ids.iter().map(|&i| packages[i].version.clone()).collect(),
            ids,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::metadata;

    /// Build the graph for whichever workspace the tests run in, which is this
    /// one: a real several-hundred-package graph rather than a toy fixture.
    fn real() -> Workspace {
        let resolved = metadata::resolve().expect("cargo metadata should resolve this workspace");
        build(resolved.packages, resolved.workspace, resolved.manifest_dir)
    }

    /// The law of the graph. Every affordance in the product leans on it: which
    /// side a port sits on, which way an arrow key travels, which hue an edge
    /// takes.
    #[test]
    fn ranks_move_forward_along_every_dependency() {
        let workspace = real();
        for entry in &workspace.crates {
            for &dep in &entry.deps {
                assert!(
                    workspace.crates[dep].rank > entry.rank,
                    "{} (rank {}) depends on {} (rank {}); that edge runs backwards",
                    entry.name,
                    entry.rank,
                    workspace.crates[dep].name,
                    workspace.crates[dep].rank
                );
            }
        }
    }

    /// Dependents are the exact inverse of dependencies. The whole reverse
    /// direction — the one existing tooling makes hardest — is this one list.
    #[test]
    fn dependents_invert_dependencies_exactly() {
        let workspace = real();
        let mut edges = 0usize;
        for entry in &workspace.crates {
            for &dep in &entry.deps {
                assert!(
                    workspace.crates[dep].dependents.contains(&entry.id),
                    "{} depends on {} but is not listed as one of its dependents",
                    entry.name,
                    workspace.crates[dep].name
                );
                edges += 1;
            }
        }
        let back: usize = workspace
            .crates
            .iter()
            .map(|entry| entry.dependents.len())
            .sum();
        assert_eq!(edges, back);
        assert_eq!(edges, workspace.dependency_count);
    }

    #[test]
    fn the_workspace_is_in_its_own_graph_and_every_crate_is_named() {
        let workspace = real();
        assert!(workspace.package_count > 0);
        assert_eq!(workspace.crates.len(), workspace.package_count);
        assert!(workspace.members().next().is_some());
        for entry in &workspace.crates {
            assert!(!entry.name.is_empty(), "crate {} has no name", entry.id);
            assert!(!entry.version.is_empty(), "{} has no version", entry.name);
        }
        assert!(workspace.distinct_count <= workspace.package_count);
    }

    /// A duplicate is a crate name at more than one version, and every member of
    /// such a group has to be flagged — a badge on one of the two is worse than
    /// no badge at all.
    #[test]
    fn every_member_of_a_duplicate_group_is_flagged() {
        let workspace = real();
        for group in &workspace.duplicates {
            assert!(group.ids.len() > 1);
            assert_eq!(group.ids.len(), group.versions.len());
            for &id in &group.ids {
                assert!(workspace.crates[id].duplicate);
                assert_eq!(workspace.crates[id].name, group.name);
            }
        }
        let flagged = workspace.crates.iter().filter(|c| c.duplicate).count();
        let grouped: usize = workspace.duplicates.iter().map(|g| g.ids.len()).sum();
        assert_eq!(flagged, grouped);
    }

    /// Ranking is longest path, not shortest: a crate reached both directly and
    /// by a longer route belongs in the further column, or the column stops
    /// being a depth.
    #[test]
    fn rank_is_the_longest_route_not_the_shortest() {
        // 0 -> 1 -> 2 and 0 -> 2. The direct edge must not pull 2 to rank 1.
        let packages = vec![
            RawPackage {
                name: "root".into(),
                version: "0.1.0".into(),
                deps: vec![1, 2],
                is_root: true,
                ..RawPackage::default()
            },
            RawPackage {
                name: "middle".into(),
                version: "0.1.0".into(),
                deps: vec![2],
                is_root: false,
                ..RawPackage::default()
            },
            RawPackage {
                name: "leaf".into(),
                version: "0.1.0".into(),
                deps: vec![],
                is_root: false,
                ..RawPackage::default()
            },
        ];
        let ranks = rank(&packages);
        assert_eq!(ranks, vec![0, 1, 2]);
    }

    #[test]
    fn report_graph_shape() {
        let workspace = real();
        let mut per_rank: std::collections::BTreeMap<usize, usize> = Default::default();
        for entry in &workspace.crates {
            *per_rank.entry(entry.rank).or_default() += 1;
        }
        let members = workspace.members().count();
        let direct: usize = workspace
            .members()
            .map(|member| member.deps.len())
            .sum();
        println!(
            "{} packages, {} crates, {} dependencies, {} columns, widest {}",
            workspace.package_count,
            workspace.distinct_count,
            workspace.dependency_count,
            per_rank.len(),
            per_rank.values().copied().max().unwrap_or(0),
        );
        println!(
            "{members} workspace members with {direct} direct dependencies; {} duplicate names",
            workspace.duplicates.len()
        );
    }
}
