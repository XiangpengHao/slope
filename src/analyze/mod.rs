//! Server-side workspace analysis: `cargo metadata` for the resolved graph,
//! the detected VCS for the diff, manifest comparison for dependency events.

pub mod code;
mod data;
mod manifest;
mod vcs;

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::path::PathBuf;

use cargo_metadata::{DependencyKind, MetadataCommand};

use crate::api::{CrateInfo, DepEvent, DepKind, DepLink, WorkspaceGraph};

/// Where to analyze: `SLOPIFY_WORKSPACE`, else the server's working dir.
pub(crate) fn workspace_dir() -> PathBuf {
    env::var_os("SLOPIFY_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn analyze() -> Result<WorkspaceGraph, String> {
    let dir = workspace_dir();
    let manifest = dir.join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "No Cargo.toml found in {}. Point slopify at a cargo workspace: \
             SLOPIFY_WORKSPACE=/path/to/workspace",
            dir.display()
        ));
    }

    let meta = MetadataCommand::new()
        .manifest_path(&manifest)
        .exec()
        .map_err(|e| format!("cargo metadata failed: {e}"))?;

    let root = PathBuf::from(meta.workspace_root.as_std_path());
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".to_string());

    let members: HashSet<_> = meta.workspace_members.iter().cloned().collect();
    let root_pkg = meta.root_package().map(|p| p.id.clone());

    // Package id -> our stable node id ("name@version").
    let mut node_id: HashMap<cargo_metadata::PackageId, String> = HashMap::new();
    let mut pkg_by_id: HashMap<cargo_metadata::PackageId, &cargo_metadata::Package> =
        HashMap::new();
    for pkg in &meta.packages {
        node_id.insert(pkg.id.clone(), format!("{}@{}", pkg.name, pkg.version));
        pkg_by_id.insert(pkg.id.clone(), pkg);
    }

    // Resolved edges from the resolve graph (deduplicated by version, the
    // graph cargo actually builds).
    let resolve = meta
        .resolve
        .as_ref()
        .ok_or("cargo metadata returned no resolve graph")?;

    let mut links: Vec<DepLink> = Vec::new();
    for node in &resolve.nodes {
        let Some(from) = node_id.get(&node.id) else {
            continue;
        };
        for dep in &node.deps {
            let Some(to) = node_id.get(&dep.pkg) else {
                continue;
            };
            // One resolved dep can appear in several tables (e.g. normal +
            // dev); keep each kind as its own link so the chart can style it.
            let mut kinds: Vec<DepKind> = dep
                .dep_kinds
                .iter()
                .map(|k| match k.kind {
                    DependencyKind::Development => DepKind::Dev,
                    DependencyKind::Build => DepKind::Build,
                    _ => DepKind::Normal,
                })
                .collect();
            kinds.dedup();
            if kinds.is_empty() {
                kinds.push(DepKind::Normal);
            }
            for kind in kinds {
                links.push(DepLink {
                    from: from.clone(),
                    to: to.clone(),
                    kind,
                    event: None,
                });
            }
        }
    }

    // The diff: changed files between the epoch base and the working copy.
    let diff = vcs::detect_diff(&root);

    // Map changed files to members by the longest matching crate directory,
    // so nested members claim their own files.
    let mut member_dirs: Vec<(cargo_metadata::PackageId, PathBuf)> = meta
        .workspace_members
        .iter()
        .filter_map(|id| {
            let pkg = pkg_by_id.get(id)?;
            let dir = pkg.manifest_path.parent()?.as_std_path().to_path_buf();
            Some((id.clone(), dir))
        })
        .collect();
    member_dirs.sort_by_key(|(_, dir)| std::cmp::Reverse(dir.components().count()));

    let mut changed_files: HashMap<cargo_metadata::PackageId, u32> = HashMap::new();
    let mut manifest_changed: HashSet<cargo_metadata::PackageId> = HashSet::new();
    for rel in &diff.changed_files {
        let abs = root.join(rel);
        if let Some((id, dir)) = member_dirs.iter().find(|(_, dir)| abs.starts_with(dir)) {
            *changed_files.entry(id.clone()).or_default() += 1;
            if abs == dir.join("Cargo.toml") {
                manifest_changed.insert(id.clone());
            }
        }
    }

    // Manifest events: compare each changed member manifest against its base
    // revision, then pin the events onto resolved links (or ghosts).
    let mut ghost_nodes: Vec<CrateInfo> = Vec::new();
    for id in &manifest_changed {
        let Some(pkg) = pkg_by_id.get(id) else {
            continue;
        };
        let Ok(rel) = pkg.manifest_path.as_std_path().strip_prefix(&root) else {
            continue;
        };
        let Some(old) = vcs::file_at_base(&root, &diff, &rel.to_string_lossy()) else {
            continue;
        };
        let Ok(new) = std::fs::read_to_string(pkg.manifest_path.as_std_path()) else {
            continue;
        };
        let events = manifest::diff_manifests(&old, &new);
        let from_id = node_id[id].clone();
        for ev in events {
            match ev.event {
                DepEvent::Removed => {
                    // The resolved graph no longer carries this edge; draw a
                    // ghost so the removal stays visible.
                    let ghost_id = ev.name.to_string();
                    if !ghost_nodes.iter().any(|g| g.id == ghost_id)
                        && !meta.packages.iter().any(|p| *p.name == ev.name)
                    {
                        ghost_nodes.push(CrateInfo {
                            id: ghost_id.clone(),
                            name: ev.name.clone(),
                            version: ev.detail.clone().unwrap_or_default(),
                            is_member: false,
                            changed: false,
                            changed_files: 0,
                            manifest_changed: false,
                            affected_dist: None,
                            dependents: 0,
                            direct_deps: 0,
                            external_deps: 0,
                            ghost: true,
                            description: None,
                            license: None,
                            repository: None,
                            homepage: None,
                            documentation: None,
                            // A removed dependency's manifest is gone with
                            // it; the name is all we know.
                            crates_io: false,
                            rel_path: None,
                        });
                    }
                    // Point at the live crate when one still exists (another
                    // member may still depend on it), else at the ghost.
                    let to = meta
                        .packages
                        .iter()
                        .find(|p| *p.name == ev.name)
                        .map(|p| node_id[&p.id].clone())
                        .unwrap_or(ghost_id);
                    links.push(DepLink {
                        from: from_id.clone(),
                        to,
                        kind: ev.kind,
                        event: Some(DepEvent::Removed),
                    });
                }
                event => {
                    // Added / Bumped: the resolved link exists; annotate it.
                    if let Some(link) = links.iter_mut().find(|l| {
                        l.from == from_id
                            && l.kind == ev.kind
                            && l.to.starts_with(&format!("{}@", ev.name))
                    }) {
                        link.event = Some(event);
                    }
                }
            }
        }
    }

    // Per-crate rollups.
    let mut dependents: HashMap<&str, u32> = HashMap::new();
    let mut direct_deps: HashMap<&str, u32> = HashMap::new();
    let mut external_deps: HashMap<&str, u32> = HashMap::new();
    let member_ids: HashSet<&str> = members
        .iter()
        .filter_map(|id| node_id.get(id).map(|s| s.as_str()))
        .collect();
    {
        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        for link in &links {
            if link.event == Some(DepEvent::Removed) {
                continue;
            }
            if seen.insert((link.from.as_str(), link.to.as_str())) {
                *dependents.entry(link.to.as_str()).or_default() += 1;
                *direct_deps.entry(link.from.as_str()).or_default() += 1;
                if !member_ids.contains(link.to.as_str()) {
                    *external_deps.entry(link.from.as_str()).or_default() += 1;
                }
            }
        }
    }

    // Blast radius: BFS downstream (along "is used by") from changed crates.
    let changed_ids: Vec<String> = meta
        .workspace_members
        .iter()
        .filter(|id| changed_files.contains_key(id))
        .map(|id| node_id[id].clone())
        .collect();
    let mut rev: HashMap<&str, Vec<&str>> = HashMap::new();
    for link in &links {
        if link.event == Some(DepEvent::Removed) {
            continue;
        }
        rev.entry(link.to.as_str())
            .or_default()
            .push(link.from.as_str());
    }
    let mut affected: HashMap<String, u32> = HashMap::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    for id in &changed_ids {
        affected.insert(id.clone(), 0);
        queue.push_back((id.clone(), 0));
    }
    while let Some((id, dist)) = queue.pop_front() {
        if let Some(users) = rev.get(id.as_str()) {
            for user in users {
                if !affected.contains_key(*user) {
                    affected.insert(user.to_string(), dist + 1);
                    queue.push_back((user.to_string(), dist + 1));
                }
            }
        }
    }

    let mut crates: Vec<CrateInfo> = meta
        .packages
        .iter()
        .map(|pkg| {
            let id = node_id[&pkg.id].clone();
            let is_member = members.contains(&pkg.id);
            // Members carry their directory so the reviewer knows where on
            // disk the crate lives; externals live in the registry cache,
            // which is no help to anyone.
            let rel_path = is_member
                .then(|| pkg.manifest_path.parent())
                .flatten()
                .and_then(|dir| dir.as_std_path().strip_prefix(&root).ok())
                .map(|rel| {
                    let rel = rel.display().to_string();
                    if rel.is_empty() { ".".to_string() } else { rel }
                });
            CrateInfo {
                changed: changed_files.contains_key(&pkg.id),
                changed_files: changed_files.get(&pkg.id).copied().unwrap_or(0),
                manifest_changed: manifest_changed.contains(&pkg.id),
                affected_dist: affected.get(&id).copied(),
                dependents: dependents.get(id.as_str()).copied().unwrap_or(0),
                direct_deps: direct_deps.get(id.as_str()).copied().unwrap_or(0),
                external_deps: external_deps.get(id.as_str()).copied().unwrap_or(0),
                name: pkg.name.to_string(),
                version: pkg.version.to_string(),
                is_member,
                ghost: false,
                description: pkg.description.clone(),
                license: pkg.license.clone(),
                repository: pkg.repository.clone(),
                homepage: pkg.homepage.clone(),
                documentation: pkg.documentation.clone(),
                crates_io: pkg.source.as_ref().is_some_and(|s| s.is_crates_io()),
                rel_path,
                id,
            }
        })
        .collect();
    crates.extend(ghost_nodes);
    crates.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(WorkspaceGraph {
        name,
        root: root.display().to_string(),
        root_crate: root_pkg.map(|id| node_id[&id].clone()),
        epoch: diff.epoch,
        crates,
        links,
    })
}
