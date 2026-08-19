//! The file tree and its chart projection.
//!
//! The code map's organizing idea is the directory structure — the shape the
//! reviewer already knows from their editor. This module builds that tree
//! from the surveyed file paths and projects it onto the paper as an
//! engraved **plan** growing downward: every directory a named street, its
//! files as lots above the spine, subdirectories branching below.
//!
//! The layout is a pure function of (tree, disclosure): deterministic, no
//! physics, and the same workspace always draws the same map. Directories
//! past the disclosure depth fold to a single gate mark carrying its file
//! count; opening a gate is a local toggle, never a re-layout of settled
//! ground.

use std::collections::{HashMap, HashSet};

use dioxus_flow::prelude::Point;

use crate::api::CodeGraph;

/// The root directory's id: always first in [`FileTree::dirs`].
pub const ROOT: u32 = 0;

/// One directory of the workspace.
#[derive(Clone, PartialEq, Debug)]
pub struct DirNode {
    pub id: u32,
    /// Last path segment; the root keeps the empty string.
    pub name: String,
    /// Path relative to the workspace root; the root is "".
    pub path: String,
    pub parent: Option<u32>,
    /// Child directories, name-sorted.
    pub dirs: Vec<u32>,
    /// Files directly in this directory (ids into the survey), name-sorted.
    pub files: Vec<u32>,
    pub depth: u32,
    /// Files in the whole subtree.
    pub file_count: u32,
    /// The crate whose sources live under this directory, when this is the
    /// shallowest directory that holds them all — the district's engraved
    /// crate name.
    pub krate: Option<String>,
}

/// The workspace's directory tree.
#[derive(Clone, PartialEq, Debug)]
pub struct FileTree {
    pub dirs: Vec<DirNode>,
    /// Directory id for every file id.
    pub dir_of_file: HashMap<u32, u32>,
}

impl FileTree {
    pub fn build(graph: &CodeGraph) -> Self {
        let mut dirs = vec![DirNode {
            id: ROOT,
            name: String::new(),
            path: String::new(),
            parent: None,
            dirs: Vec::new(),
            files: Vec::new(),
            depth: 0,
            file_count: 0,
            krate: None,
        }];
        let mut by_path: HashMap<String, u32> = HashMap::new();
        by_path.insert(String::new(), ROOT);
        let mut dir_of_file = HashMap::new();

        for file in &graph.files {
            let dir_path = match file.path.rsplit_once('/') {
                Some((dir, _)) => dir,
                None => "",
            };
            // Create the chain of directories down to this file's.
            let mut current = ROOT;
            if !dir_path.is_empty() {
                let mut walked = String::new();
                for seg in dir_path.split('/') {
                    if !walked.is_empty() {
                        walked.push('/');
                    }
                    walked.push_str(seg);
                    current = match by_path.get(&walked) {
                        Some(&id) => id,
                        None => {
                            let id = dirs.len() as u32;
                            let parent = walked
                                .rsplit_once('/')
                                .map(|(p, _)| by_path[p])
                                .unwrap_or(ROOT);
                            let depth = dirs[parent as usize].depth + 1;
                            dirs.push(DirNode {
                                id,
                                name: seg.to_string(),
                                path: walked.clone(),
                                parent: Some(parent),
                                dirs: Vec::new(),
                                files: Vec::new(),
                                depth,
                                file_count: 0,
                                krate: None,
                            });
                            dirs[parent as usize].dirs.push(id);
                            by_path.insert(walked.clone(), id);
                            id
                        }
                    };
                }
            }
            dirs[current as usize].files.push(file.id);
            dir_of_file.insert(file.id, current);
        }

        // Deterministic child order: names, not insertion.
        let names: Vec<String> = dirs.iter().map(|d| d.name.clone()).collect();
        let file_names: HashMap<u32, &str> = graph
            .files
            .iter()
            .map(|f| (f.id, f.path.rsplit_once('/').map(|(_, n)| n).unwrap_or(&f.path)))
            .collect();
        for dir in &mut dirs {
            dir.dirs.sort_by(|a, b| names[*a as usize].cmp(&names[*b as usize]));
            dir.files
                .sort_by(|a, b| file_names[a].cmp(file_names[b]));
        }

        // Subtree file counts, children before parents (children always have
        // larger ids than their parent, by construction).
        for i in (0..dirs.len()).rev() {
            let own = dirs[i].files.len() as u32;
            let sub: u32 = dirs[i].dirs.iter().map(|&d| dirs[d as usize].file_count).sum();
            dirs[i].file_count = own + sub;
            if dirs[i].file_count == 0 {
                dirs[i].file_count = 0;
            }
        }

        // Crate districts: each crate labels the shallowest directory that
        // contains all of its files.
        let mut crate_dirs: HashMap<&str, u32> = HashMap::new();
        for file in &graph.files {
            let dir = dir_of_file[&file.id];
            crate_dirs
                .entry(file.krate.as_str())
                .and_modify(|d| *d = common_ancestor(&dirs, *d, dir))
                .or_insert(dir);
        }
        let mut labels: Vec<(&str, u32)> = crate_dirs.into_iter().collect();
        labels.sort();
        for (krate, dir) in labels {
            let slot = &mut dirs[dir as usize].krate;
            if slot.is_none() {
                *slot = Some(krate.to_string());
            }
        }

        Self { dirs, dir_of_file }
    }
}

fn common_ancestor(dirs: &[DirNode], mut a: u32, mut b: u32) -> u32 {
    while dirs[a as usize].depth > dirs[b as usize].depth {
        a = dirs[a as usize].parent.unwrap_or(ROOT);
    }
    while dirs[b as usize].depth > dirs[a as usize].depth {
        b = dirs[b as usize].parent.unwrap_or(ROOT);
    }
    while a != b {
        a = dirs[a as usize].parent.unwrap_or(ROOT);
        b = dirs[b as usize].parent.unwrap_or(ROOT);
    }
    a
}

// ---------------------------------------------------------------------------
// Disclosure: how much of the tree the first paint shows.
// ---------------------------------------------------------------------------

/// Marks the first paint budgets for. Beyond it, deep directories start
/// folded as gates — stated in words on the plate, opened with one click.
pub const MARK_BUDGET: usize = 320;

/// The deepest directory level that stays open by default: the largest depth
/// keeping the visible mark count within the budget. Depth 1 always opens,
/// so the map never greets the reviewer with a single closed gate.
pub fn default_open_depth(tree: &FileTree, budget: usize) -> u32 {
    let max_depth = tree.dirs.iter().map(|d| d.depth).max().unwrap_or(0);
    let mut best = 1;
    for depth in 1..=max_depth.max(1) {
        let mut marks = 0usize;
        for dir in &tree.dirs {
            if dir.depth <= depth {
                marks += dir.files.len() + 1; // its files + its own mark
            } else if dir.depth == depth + 1 {
                marks += 1; // a gate
            }
        }
        if depth == 1 || marks <= budget {
            best = depth;
        }
    }
    best
}

/// Which directories are open, from the default depth and the reviewer's
/// toggles. A directory under a closed ancestor is not in the set at all —
/// it is invisible, not merely closed.
pub fn open_dirs(tree: &FileTree, depth: u32, toggled: &HashSet<u32>) -> HashSet<u32> {
    let mut open = HashSet::new();
    let mut stack = vec![ROOT];
    while let Some(id) = stack.pop() {
        open.insert(id);
        for &child in &tree.dirs[id as usize].dirs {
            let default_open = tree.dirs[child as usize].depth <= depth;
            if default_open != toggled.contains(&child) {
                stack.push(child);
            }
        }
    }
    open
}

/// A visible directory that is not open: drawn as a gate carrying its count.
pub fn is_gate(tree: &FileTree, open: &HashSet<u32>, dir: u32) -> bool {
    !open.contains(&dir)
        && tree.dirs[dir as usize]
            .parent
            .is_some_and(|p| open.contains(&p))
}

// ---------------------------------------------------------------------------
// Shared geometry types.
// ---------------------------------------------------------------------------

/// One mark's key on the chart: a file or a directory.
pub fn file_key(id: u32) -> String {
    format!("f{id}")
}
pub fn dir_key(id: u32) -> String {
    format!("d{id}")
}

/// One placed mark.
#[derive(Clone, PartialEq, Debug)]
pub struct PlacedMark {
    pub point: Point,
}

/// Estimated on-chart label width for a mono name, for collision budgets.
/// The budget covers the mark's own box plus the name seated beside it.
pub fn label_w(name: &str) -> f64 {
    name.chars().count() as f64 * 6.8 + 46.0
}

// ---------------------------------------------------------------------------
// The plan: every directory a street, every file a lot.
// ---------------------------------------------------------------------------

/// Vertical distance from a spine down to its child spines.
const STUB_H: f64 = 44.0;
/// Height of one row of lots above a spine.
const LOT_ROW_H: f64 = 30.0;
/// Gap between sibling blocks.
const BLOCK_GAP: f64 = 48.0;
/// Lots wrap to a new row past this width.
const ROW_MAX: f64 = 460.0;
/// Clearance between a spine's start and its first lot or child.
const SPINE_PAD: f64 = 10.0;

/// One street to engrave: a horizontal spine with its name lettered on it,
/// and the stub connecting it to its parent street.
#[derive(Clone, PartialEq, Debug)]
pub struct Street {
    pub dir: u32,
    pub x0: f64,
    pub x1: f64,
    pub y: f64,
    /// Stub from the parent spine: (x, y_top, y_bottom). None for the root.
    pub stub: Option<(f64, f64, f64)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PlanLayout {
    pub pos: HashMap<String, PlacedMark>,
    pub streets: Vec<Street>,
    /// The whole plan's bounds (before centering), for tests.
    pub size: (f64, f64),
}

struct Block {
    width: f64,
    height: f64,
    /// The spine's y within the block.
    spine_y: f64,
    /// Marks at block-relative positions.
    marks: Vec<(String, f64, f64)>,
    streets: Vec<Street>,
}

/// Lay the visible tree as a town plan. The root street runs across the top;
/// every open directory hangs its own street below its parent's, files as
/// lots in rows above each spine. Everything is name-ordered: the same
/// workspace always draws the same plan.
pub fn plan_layout(
    tree: &FileTree,
    open: &HashSet<u32>,
    file_names: &HashMap<u32, String>,
) -> PlanLayout {
    let mut block = layout_block(tree, open, file_names, ROOT);
    // Center the plan on the flow origin.
    let (w, h) = (block.width, block.height);
    let (dx, dy) = (-w / 2.0, -h / 2.0);
    let mut pos = HashMap::new();
    for (key, x, y) in block.marks.drain(..) {
        pos.insert(
            key,
            PlacedMark {
                point: Point::new(x + dx, y + dy),
            },
        );
    }
    let streets = block
        .streets
        .into_iter()
        .map(|s| Street {
            x0: s.x0 + dx,
            x1: s.x1 + dx,
            y: s.y + dy,
            stub: s.stub.map(|(x, y0, y1)| (x + dx, y0 + dy, y1 + dy)),
            ..s
        })
        .collect();
    PlanLayout {
        pos,
        streets,
        size: (w, h),
    }
}

fn layout_block(
    tree: &FileTree,
    open: &HashSet<u32>,
    file_names: &HashMap<u32, String>,
    dir: u32,
) -> Block {
    let node = &tree.dirs[dir as usize];

    // Lots: files in name order, wrapped into rows.
    let mut rows: Vec<Vec<(u32, f64)>> = vec![Vec::new()];
    let mut row_w = 0.0f64;
    for &f in &node.files {
        let w = label_w(file_names.get(&f).map(String::as_str).unwrap_or("?"));
        if row_w + w > ROW_MAX && !rows.last().unwrap().is_empty() {
            rows.push(Vec::new());
            row_w = 0.0;
        }
        rows.last_mut().unwrap().push((f, w));
        row_w += w;
    }
    let lots_h = if node.files.is_empty() {
        0.0
    } else {
        rows.len() as f64 * LOT_ROW_H
    };
    let lots_w: f64 = rows
        .iter()
        .map(|r| r.iter().map(|(_, w)| *w).sum::<f64>())
        .fold(0.0, f64::max);

    // Child blocks: open dirs as full streets, closed dirs as gates.
    struct ChildSlot {
        name: String,
        gate: Option<u32>,
        block: Option<Block>,
    }
    let mut slots: Vec<ChildSlot> = Vec::new();
    for &child in &node.dirs {
        let name = tree.dirs[child as usize].name.clone();
        if open.contains(&child) {
            slots.push(ChildSlot {
                name,
                gate: None,
                block: Some(layout_block(tree, open, file_names, child)),
            });
        } else {
            slots.push(ChildSlot {
                name,
                gate: Some(child),
                block: None,
            });
        }
    }
    slots.sort_by(|a, b| a.name.cmp(&b.name));

    let spine_y = lots_h;
    let mut marks: Vec<(String, f64, f64)> = Vec::new();
    let mut streets: Vec<Street> = Vec::new();

    // Place lots above the spine, oldest row nearest it. The first lot
    // clears the directory's own mark at the street start.
    for (ri, row) in rows.iter().enumerate() {
        let y = spine_y - (ri as f64 + 0.5) * LOT_ROW_H;
        let mut x = SPINE_PAD + 18.0;
        for (f, w) in row {
            marks.push((file_key(*f), x + 9.0, y));
            x += w;
        }
    }

    // Place children below.
    let child_top = spine_y + STUB_H;
    let mut x = SPINE_PAD + 14.0;
    let mut max_child_h = 0.0f64;
    // Where the street's engraved line must still reach: the last stub that
    // hangs from it. Past that, a line is bare rule to nowhere.
    let mut last_stub_x = 0.0f64;
    for slot in &mut slots {
        match (&slot.gate, &mut slot.block) {
            (Some(gate), _) => {
                let w = label_w(&slot.name) + 18.0;
                marks.push((dir_key(*gate), x + 9.0, child_top + 10.0));
                streets.push(Street {
                    dir: *gate,
                    x0: x + 9.0,
                    x1: x + 9.0,
                    y: child_top + 10.0,
                    stub: Some((x + 9.0, spine_y, child_top + 10.0 - 12.0)),
                });
                last_stub_x = last_stub_x.max(x + 9.0);
                max_child_h = max_child_h.max(20.0 + 10.0);
                x += w + BLOCK_GAP;
            }
            (None, Some(block)) => {
                let child_dir = block.streets.last().map(|s| s.dir);
                // Shift the child block into place.
                for (key, mx, my) in block.marks.drain(..) {
                    marks.push((key, mx + x, my + child_top));
                }
                let child_spine_y = child_top + block.spine_y;
                for s in block.streets.drain(..) {
                    streets.push(Street {
                        x0: s.x0 + x,
                        x1: s.x1 + x,
                        y: s.y + child_top,
                        stub: s.stub.map(|(sx, y0, y1)| (sx + x, y0 + child_top, y1 + child_top)),
                        ..s
                    });
                }
                let _ = child_dir;
                // The stub connecting the parent spine to the child spine.
                streets.push(Street {
                    dir: u32::MAX, // stub-only entry
                    x0: x + SPINE_PAD,
                    x1: x + SPINE_PAD,
                    y: child_spine_y,
                    stub: Some((x + SPINE_PAD, spine_y, child_spine_y)),
                });
                last_stub_x = last_stub_x.max(x + SPINE_PAD);
                max_child_h = max_child_h.max(block.height);
                x += block.width + BLOCK_GAP;
            }
            _ => unreachable!(),
        }
    }
    let children_w = if slots.is_empty() {
        0.0
    } else {
        x - BLOCK_GAP + SPINE_PAD
    };

    let spine_w = lots_w
        .max(children_w)
        .max(label_w(&tree.dirs[dir as usize].name) + 30.0)
        .max(60.0)
        + SPINE_PAD;

    // The engraved line ends where its content does — at the last lot or
    // the last hanging stub — never running on as bare rule.
    let lots_end = if node.files.is_empty() {
        0.0
    } else {
        SPINE_PAD + 18.0 + lots_w
    };
    let street_end = lots_end
        .max(last_stub_x + 14.0)
        .max(label_w(&tree.dirs[dir as usize].name) + 30.0)
        .max(60.0)
        .min(spine_w);

    // The directory's own mark sits at the start of its street.
    marks.push((dir_key(dir), 0.0, spine_y));
    streets.push(Street {
        dir,
        x0: 0.0,
        x1: street_end,
        y: spine_y,
        stub: None,
    });

    let height = spine_y
        + if slots.is_empty() {
            18.0
        } else {
            STUB_H + max_child_h + 8.0
        };
    Block {
        width: spine_w.max(children_w),
        height,
        spine_y,
        marks,
        streets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{CodeGraph, FileInfo};

    fn file(id: u32, path: &str) -> FileInfo {
        FileInfo {
            id,
            path: path.to_string(),
            krate: "test".to_string(),
            lines: 10,
            items: 1,
            fns: 1,
            types: 0,
            traits: 0,
            refs_in_files: 0,
            refs_out_files: 0,
        }
    }

    fn graph(paths: &[&str]) -> CodeGraph {
        CodeGraph {
            files: paths
                .iter()
                .enumerate()
                .map(|(i, p)| file(i as u32, p))
                .collect(),
            refs: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    fn names(g: &CodeGraph) -> HashMap<u32, String> {
        g.files
            .iter()
            .map(|f| {
                (
                    f.id,
                    f.path
                        .rsplit_once('/')
                        .map(|(_, n)| n.to_string())
                        .unwrap_or_else(|| f.path.clone()),
                )
            })
            .collect()
    }

    #[test]
    fn tree_mirrors_directories() {
        let g = graph(&[
            "src/main.rs",
            "src/views/atlas.rs",
            "src/views/star.rs",
            "build.rs",
        ]);
        let tree = FileTree::build(&g);
        let root = &tree.dirs[ROOT as usize];
        assert_eq!(root.files.len(), 1); // build.rs
        assert_eq!(root.dirs.len(), 1); // src
        let src = &tree.dirs[root.dirs[0] as usize];
        assert_eq!(src.name, "src");
        assert_eq!(src.file_count, 3);
        assert_eq!(src.files.len(), 1);
        assert_eq!(src.dirs.len(), 1);
        let views = &tree.dirs[src.dirs[0] as usize];
        assert_eq!(views.path, "src/views");
        assert_eq!(views.depth, 2);
        assert_eq!(views.files.len(), 2);
        // The single crate labels the root: every file lives under it.
        assert_eq!(root.krate.as_deref(), Some("test"));
    }

    #[test]
    fn disclosure_respects_budget() {
        // 40 files spread across 8 deep directories.
        let paths: Vec<String> = (0..8)
            .flat_map(|d| {
                (0..5).map(move |f| format!("a{d}/b/c/file{f}.rs"))
            })
            .collect();
        let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
        let g = graph(&refs);
        let tree = FileTree::build(&g);
        // A tiny budget folds below depth 1; a huge one opens everything.
        assert_eq!(default_open_depth(&tree, 10), 1);
        let all = default_open_depth(&tree, 10_000);
        assert_eq!(all, 3);
        let open = open_dirs(&tree, 1, &HashSet::new());
        // Root and the eight a* dirs are open; b dirs are gates.
        assert!(open.contains(&ROOT));
        let gates: Vec<u32> = tree
            .dirs
            .iter()
            .filter(|d| is_gate(&tree, &open, d.id))
            .map(|d| d.id)
            .collect();
        assert_eq!(gates.len(), 8);
    }

    #[test]
    fn toggling_a_gate_opens_it() {
        let g = graph(&["a/b/one.rs", "a/b/two.rs", "a/top.rs"]);
        let tree = FileTree::build(&g);
        let b = tree
            .dirs
            .iter()
            .find(|d| d.path == "a/b")
            .unwrap()
            .id;
        let closed = open_dirs(&tree, 1, &HashSet::new());
        assert!(!closed.contains(&b));
        let mut toggled = HashSet::new();
        toggled.insert(b);
        let open = open_dirs(&tree, 1, &toggled);
        assert!(open.contains(&b));
    }

    #[test]
    fn plan_blocks_do_not_overlap() {
        let g = graph(&[
            "a/one.rs",
            "a/two.rs",
            "b/three.rs",
            "b/sub/four.rs",
            "top.rs",
        ]);
        let tree = FileTree::build(&g);
        let open = open_dirs(&tree, 9, &HashSet::new());
        let layout = plan_layout(&tree, &open, &names(&g));
        // Marks never coincide.
        let pts: Vec<(i64, i64)> = layout
            .pos
            .values()
            .map(|m| ((m.point.x * 10.0) as i64, (m.point.y * 10.0) as i64))
            .collect();
        let mut dedup = pts.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(pts.len(), dedup.len());
        // Sibling streets a and b do not overlap horizontally.
        let street = |path: &str| {
            let id = tree.dirs.iter().find(|d| d.path == path).unwrap().id;
            layout
                .streets
                .iter()
                .find(|s| s.dir == id)
                .unwrap()
                .clone()
        };
        let (a, b) = (street("a"), street("b"));
        assert!(a.x1 <= b.x0 || b.x1 <= a.x0, "sibling streets overlap");
        // Deeper streets sit lower.
        let sub = street("b/sub");
        assert!(sub.y > b.y);
        assert!(b.y > street("").y);
    }
}
