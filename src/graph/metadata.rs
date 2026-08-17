//! Resolution. Shells out to `cargo metadata` rather than parsing
//! `Cargo.toml`/`Cargo.lock` by hand, so feature unification and platform
//! resolution are cargo's answer instead of ours. Costs a dependency on cargo
//! being on PATH, which is a safe bet next to the source it is reading.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use serde::Deserialize;

use super::RawPackage;

/// The workspace this process was pointed at, from `rust-viewer <path>`.
static TARGET: OnceLock<PathBuf> = OnceLock::new();

pub fn set_target(path: PathBuf) {
    let _ = TARGET.set(path);
}

pub fn target() -> PathBuf {
    TARGET
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<PackageMeta>,
    resolve: Option<Resolve>,
    workspace_members: Vec<String>,
    workspace_root: String,
}

#[derive(Deserialize)]
struct PackageMeta {
    id: String,
    name: String,
    version: String,
    /// Where this package's `Cargo.toml` sits. The call lens needs it to know
    /// which directories hold source it can read.
    #[serde(default)]
    manifest_path: String,
    /// Where the crate says its own source lives. Absent for plenty of crates,
    /// which is why the record only offers the link when there is one.
    #[serde(default)]
    repository: Option<String>,
    /// Where cargo got it. `null` for a path dependency and a workspace member;
    /// a `registry+…` string for anything resolved from a registry. This is what
    /// says whether a crates.io page exists to link to, rather than guessing
    /// from the name and getting a stranger's crate.
    #[serde(default)]
    source: Option<String>,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<NodeDep>,
}

#[derive(Deserialize)]
struct NodeDep {
    pkg: String,
    #[serde(default)]
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    /// `None` is a normal dependency; `"build"` and `"dev"` are named.
    kind: Option<String>,
}

pub struct Resolved {
    pub packages: Vec<RawPackage>,
    pub workspace: String,
    pub manifest_dir: String,
}

/// One crate this workspace actually builds, and where its source sits. The
/// call lens reads source, so unlike the dependency board it needs to know
/// which directories on disk belong to the workspace.
pub struct Member {
    pub name: String,
    /// The directory holding the crate's `Cargo.toml`.
    pub dir: PathBuf,
}

/// The workspace's own crates, with their source directories.
///
/// A second `cargo metadata` run rather than a widened `Resolved`: the two
/// lenses ask different questions of the same command, and threading the call
/// lens's needs through the dependency board's type would couple them for no
/// gain. Cargo caches the resolve, so the second run is cheap next to the
/// indexing that follows it.
pub fn members() -> Result<(String, PathBuf, Vec<Member>), String> {
    let dir = target();
    let meta: Metadata = run(&dir)?;
    let members: Vec<Member> = meta
        .packages
        .iter()
        .filter(|pkg| meta.workspace_members.iter().any(|id| id == &pkg.id))
        .filter_map(|pkg| {
            let manifest = Path::new(&pkg.manifest_path);
            manifest.parent().map(|dir| Member {
                name: pkg.name.clone(),
                dir: dir.to_path_buf(),
            })
        })
        .collect();
    if members.is_empty() {
        return Err("cargo metadata reported no workspace members".to_string());
    }
    let workspace = Path::new(&meta.workspace_root)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| meta.workspace_root.clone());
    Ok((workspace, PathBuf::from(&meta.workspace_root), members))
}

/// Shell out to `cargo metadata` for a workspace directory.
fn run(dir: &Path) -> Result<Metadata, String> {
    let manifest = dir.join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "no Cargo.toml at {}. Point rust-viewer at a crate or workspace directory.",
            dir.display()
        ));
    }

    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .map_err(|e| format!("could not run cargo: {e}. rust-viewer needs cargo on PATH."))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // cargo's own diagnostic is more useful than anything we could write.
        return Err(format!(
            "cargo metadata failed for {}:\n{}",
            manifest.display(),
            stderr.trim()
        ));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("could not parse cargo metadata output: {e}"))
}

/// Run `cargo metadata` against the target workspace and reduce it to the
/// packages a build actually pulls in.
pub fn resolve() -> Result<Resolved, String> {
    let dir = target();
    let meta: Metadata = run(&dir)?;

    let resolve = meta
        .resolve
        .ok_or_else(|| "cargo metadata returned no resolve graph".to_string())?;

    build(meta.packages, resolve, meta.workspace_members, meta.workspace_root, &dir)
}

fn build(
    packages: Vec<PackageMeta>,
    resolve: Resolve,
    workspace_members: Vec<String>,
    workspace_root: String,
    dir: &Path,
) -> Result<Resolved, String> {
    // Index by package id so resolve nodes can be joined onto package metadata.
    let index: std::collections::HashMap<&str, usize> = packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();

    // Dev-dependencies are excluded: they are not in anyone's build, and
    // including them is what puts cycles in an otherwise acyclic graph.
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); packages.len()];
    for node in &resolve.nodes {
        let Some(&from) = index.get(node.id.as_str()) else {
            continue;
        };
        for dep in &node.deps {
            let ships = dep.dep_kinds.is_empty()
                || dep
                    .dep_kinds
                    .iter()
                    .any(|k| k.kind.as_deref() != Some("dev"));
            if !ships {
                continue;
            }
            if let Some(&to) = index.get(dep.pkg.as_str()) {
                edges[from].push(to);
            }
        }
    }

    let roots: Vec<usize> = workspace_members
        .iter()
        .filter_map(|id| index.get(id.as_str()).copied())
        .collect();
    if roots.is_empty() {
        return Err("cargo metadata reported no workspace members".to_string());
    }

    // Keep only what the workspace actually reaches once dev edges are gone.
    let mut keep = vec![false; packages.len()];
    let mut queue: std::collections::VecDeque<usize> = roots.iter().copied().collect();
    for &r in &roots {
        keep[r] = true;
    }
    while let Some(n) = queue.pop_front() {
        for &d in &edges[n] {
            if !keep[d] {
                keep[d] = true;
                queue.push_back(d);
            }
        }
    }

    // Compact to the reachable set, remapping indices as we go.
    let mut remap = vec![usize::MAX; packages.len()];
    let mut out: Vec<RawPackage> = Vec::new();
    for (i, pkg) in packages.iter().enumerate() {
        if !keep[i] {
            continue;
        }
        remap[i] = out.len();
        out.push(RawPackage {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            deps: Vec::new(),
            is_root: roots.contains(&i),
            repository: pkg.repository.clone().filter(|url| !url.trim().is_empty()),
            registry: pkg
                .source
                .as_deref()
                .is_some_and(|source| source.starts_with("registry+")),
        });
    }
    for (i, edge_list) in edges.iter().enumerate() {
        if remap[i] == usize::MAX {
            continue;
        }
        let mut deps: Vec<usize> = edge_list
            .iter()
            .filter(|&&d| remap[d] != usize::MAX)
            .map(|&d| remap[d])
            .collect();
        deps.sort_unstable();
        deps.dedup();
        out[remap[i]].deps = deps;
    }

    let workspace = Path::new(&workspace_root)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| workspace_root.clone());

    Ok(Resolved {
        packages: out,
        workspace,
        manifest_dir: dir.display().to_string(),
    })
}
