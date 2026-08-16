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

/// Run `cargo metadata` against the target workspace and reduce it to the
/// packages a build actually pulls in.
pub fn resolve() -> Result<Resolved, String> {
    let dir = target();
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

    let meta: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("could not parse cargo metadata output: {e}"))?;

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
