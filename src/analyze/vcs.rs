//! Diff detection. jj first, git fallback; failures degrade to a graph
//! without change tracking, stated in words, never a crash.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::graph::dep::Epoch;

pub(super) struct Diff {
    pub(crate) epoch: Epoch,
    /// Paths relative to the workspace root.
    pub(crate) changed_files: Vec<String>,
    /// How to read a file at the base revision.
    pub(crate) base_ref: Option<BaseRef>,
}

pub(crate) enum BaseRef {
    Git { repo_root: PathBuf, rev: String },
    Jj { repo_root: PathBuf, rev: String },
}

fn run(dir: &Path, program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Walk up from `start` looking for a directory containing `marker`.
fn find_up(start: &Path, marker: &str) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(marker).exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

impl Diff {
    /// Detect the VCS and compute changed files between the base and the working
    /// copy. `SLOPE_BASE` overrides the base revision (a git rev or jj revset).
    pub(super) fn detect(workspace_root: &Path) -> Self {
        let base_override = std::env::var("SLOPE_BASE").ok();

        // A colocated jj repo has both markers; git plumbing is the more
        // predictable of the two, so prefer it whenever .git exists.
        if let Some(repo) = find_up(workspace_root, ".git")
            && let Some(diff) = Self::from_git(workspace_root, &repo, base_override.as_deref())
        {
            return diff;
        }
        if let Some(repo) = find_up(workspace_root, ".jj")
            && let Some(diff) = Self::from_jj(workspace_root, &repo, base_override.as_deref())
        {
            return diff;
        }

        Diff {
            epoch: Epoch {
                base: "—".into(),
                target: "working copy".into(),
                note: Some("No version control detected — change tracking is off.".into()),
            },
            changed_files: Vec::new(),
            base_ref: None,
        }
    }
}

/// Rebase paths reported relative to `repo_root` onto `workspace_root`.
fn rebase_paths(repo_root: &Path, workspace_root: &Path, paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .filter_map(|p| {
            let abs = repo_root.join(&p);
            abs.strip_prefix(workspace_root)
                .ok()
                .map(|r| r.to_string_lossy().into_owned())
        })
        .collect()
}

impl Diff {
    /// The diff a git repo reports: the merge-base of HEAD and trunk against
    /// the working copy, untracked files included.
    fn from_git(workspace_root: &Path, repo: &Path, base_override: Option<&str>) -> Option<Self> {
        // Resolve the base: explicit override, else the merge-base of HEAD and
        // the first trunk branch that exists.
        let (base_rev, base_label) = if let Some(rev) = base_override {
            let resolved = run(
                repo,
                "git",
                &["rev-parse", "--verify", &format!("{rev}^{{commit}}")],
            )?;
            (resolved.trim().to_string(), rev.to_string())
        } else {
            let trunk = ["main", "master", "origin/main", "origin/master"]
                .iter()
                .find(|b| {
                    run(
                        repo,
                        "git",
                        &["rev-parse", "--verify", &format!("{b}^{{commit}}")],
                    )
                    .is_some()
                })
                .copied()?;
            let merge_base = run(repo, "git", &["merge-base", "HEAD", trunk])?;
            (merge_base.trim().to_string(), trunk.to_string())
        };
        let short = &base_rev[..base_rev.len().min(8)];

        // Tracked changes (worktree vs base) + untracked files: everything the
        // epoch introduced, whether or not it was committed along the way.
        let mut files: Vec<String> = run(repo, "git", &["diff", "--name-only", &base_rev])?
            .lines()
            .map(str::to_string)
            .collect();
        if let Some(untracked) = run(repo, "git", &["ls-files", "--others", "--exclude-standard"]) {
            files.extend(untracked.lines().map(str::to_string));
        }
        files.sort();
        files.dedup();
        let files = rebase_paths(repo, workspace_root, files);

        Some(Self {
            epoch: Epoch {
                base: format!("{base_label} @ {short}"),
                target: "working copy".into(),
                note: None,
            },
            changed_files: files,
            base_ref: Some(BaseRef::Git {
                repo_root: repo.to_path_buf(),
                rev: base_rev,
            }),
        })
    }

    /// The diff a jj repo reports, against `trunk()` unless overridden.
    fn from_jj(workspace_root: &Path, repo: &Path, base_override: Option<&str>) -> Option<Self> {
        let base = base_override.unwrap_or("trunk()");
        let files = run(
            repo,
            "jj",
            &["diff", "--from", base, "--name-only", "--no-pager"],
        )?;
        let files: Vec<String> = files.lines().map(str::to_string).collect();
        let files = rebase_paths(repo, workspace_root, files);
        let short = run(
            repo,
            "jj",
            &[
                "log",
                "-r",
                base,
                "--no-graph",
                "--no-pager",
                "-T",
                "commit_id.short(8)",
            ],
        )
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

        Some(Self {
            epoch: Epoch {
                base: if short.is_empty() {
                    base.to_string()
                } else {
                    format!("{base} @ {short}")
                },
                target: "working copy".into(),
                note: None,
            },
            changed_files: files,
            base_ref: Some(BaseRef::Jj {
                repo_root: repo.to_path_buf(),
                rev: base.to_string(),
            }),
        })
    }

    /// Contents of `rel_path` (relative to the workspace root) at this diff's base.
    pub(super) fn file_at_base(&self, workspace_root: &Path, rel_path: &str) -> Option<String> {
        match self.base_ref.as_ref()? {
            BaseRef::Git { repo_root, rev } => {
                let abs = workspace_root.join(rel_path);
                let repo_rel = abs.strip_prefix(repo_root).ok()?;
                run(
                    repo_root,
                    "git",
                    &["show", &format!("{rev}:{}", repo_rel.to_string_lossy())],
                )
            }
            BaseRef::Jj { repo_root, rev } => {
                let abs = workspace_root.join(rel_path);
                let repo_rel = abs.strip_prefix(repo_root).ok()?;
                run(
                    repo_root,
                    "jj",
                    &[
                        "file",
                        "show",
                        "-r",
                        rev,
                        "--no-pager",
                        &repo_rel.to_string_lossy(),
                    ],
                )
            }
        }
    }
}
