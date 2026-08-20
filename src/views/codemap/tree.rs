//! The file tree and its chart projection.
//!
//! The code map's organizing idea is the directory structure — the shape the
//! reviewer already knows from their editor. This module builds that tree from
//! the surveyed file paths and projects it onto the paper as **nested
//! territories**: every directory a bordered district with its name engraved
//! on the border, every file a block inside it, every district inside its
//! parent. Nesting is the only thing it means — belongs to.
//!
//! The layout is a pure function of (tree, disclosure, measured block sizes):
//! deterministic, no physics, and the same workspace always draws the same
//! map. Directories past the disclosure depth fold to a single gate carrying
//! its counts; opening a gate is a local toggle, never a re-survey of settled
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
            .map(|f| {
                (
                    f.id,
                    f.path.rsplit_once('/').map(|(_, n)| n).unwrap_or(&f.path),
                )
            })
            .collect();
        for dir in &mut dirs {
            dir.dirs
                .sort_by(|a, b| names[*a as usize].cmp(&names[*b as usize]));
            dir.files.sort_by(|a, b| file_names[a].cmp(file_names[b]));
        }

        // Subtree file counts, children before parents (children always have
        // larger ids than their parent, by construction).
        for i in (0..dirs.len()).rev() {
            let own = dirs[i].files.len() as u32;
            let sub: u32 = dirs[i]
                .dirs
                .iter()
                .map(|&d| dirs[d as usize].file_count)
                .sum();
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

/// Block furniture, in flow units — one unit is one CSS pixel at zoom 1. The
/// layout measures blocks itself, so the drawn plate must be handed exactly
/// these numbers: a plate taller than its box would stand on its neighbor.
pub const BLOCK_HEAD_H: f64 = 25.0;
pub const BLOCK_ROW_H: f64 = 17.0;
/// One wrapped line of a fold's words. A fold that clips its own count says
/// nothing, so the box grows to fit the sentence.
pub const BLOCK_FOLD_LINE: f64 = 11.0;
pub const BLOCK_PAD_X: f64 = 9.0;
/// Slack below the last row, so the frame never crowds the letters.
pub const BLOCK_FOOT: f64 = 7.0;
pub const BLOCK_MIN_W: f64 = 138.0;
pub const BLOCK_MAX_W: f64 = 296.0;
/// A folded directory's gate: one counted line, no rows.
pub const GATE_H: f64 = 31.0;

/// District furniture: inner padding, the band the engraved label sits in,
/// and the gap between siblings.
const D_PAD: f64 = 13.0;
const D_LABEL_H: f64 = 14.0;
const GAP: f64 = 11.0;

/// One em of advance in the data face (JetBrains Mono is monospaced). Every
/// measured label on the map is data, so this is the only advance the layout
/// needs.
pub const MONO_ADVANCE: f64 = 0.6;

/// Estimated width of mono text at a given size. The map would rather carry
/// slack than clip a name.
pub fn text_w(text: &str, px: f64) -> f64 {
    tracked_w(text, px, MONO_ADVANCE, 0.0)
}

/// Estimated width of a tracked run of letters. The engraved labels are
/// uppercase with heavy letter-spacing, and the tracking is most of what they
/// measure — leaving it out is what makes a label collide with its neighbor.
pub fn tracked_w(text: &str, px: f64, advance: f64, tracking_em: f64) -> f64 {
    text.chars().count() as f64 * px * (advance + tracking_em)
}

/// One placed box on the paper.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Placed {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Placed {
    pub fn center(&self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    fn shifted(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }
}

/// One district frame: a bordered territory with its name on the border.
#[derive(Clone, PartialEq, Debug)]
pub struct District {
    pub dir: u32,
    pub at: Placed,
    pub depth: u32,
}

/// What the layout must be told about what it seats: the measured size of
/// every file block and gate, and the width of every district's engraved
/// label. Measuring belongs with the drawing, not with the geometry.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Measures {
    pub blocks: HashMap<u32, (f64, f64)>,
    pub gates: HashMap<u32, (f64, f64)>,
    pub labels: HashMap<u32, f64>,
}

/// The whole map, placed.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct MapLayout {
    pub blocks: HashMap<u32, Placed>,
    pub gates: HashMap<u32, Placed>,
    /// Outermost first: districts paint in this order, so a nested tint lays
    /// over its parent's.
    pub districts: Vec<District>,
    pub size: (f64, f64),
}

/// Lay the visible tree as nested territories, centered on the flow origin.
/// Files come before subdirectories inside a district — the reading order of
/// a directory listing — and everything is name-ordered, so the same
/// workspace always draws the same map.
pub fn map_layout(tree: &FileTree, open: &HashSet<u32>, m: &Measures) -> MapLayout {
    let packed = pack_dir(tree, open, m, ROOT);
    let (dx, dy) = (-packed.w / 2.0, -packed.h / 2.0);
    // The root's own frame carries the workspace; every other district paints
    // over it, in the order they were packed — an ancestor always first.
    let root = District {
        dir: ROOT,
        at: Placed {
            x: dx,
            y: dy,
            w: packed.w,
            h: packed.h,
        },
        depth: 0,
    };
    MapLayout {
        blocks: packed
            .blocks
            .into_iter()
            .map(|(id, at)| (id, at.shifted(dx, dy)))
            .collect(),
        gates: packed
            .gates
            .into_iter()
            .map(|(id, at)| (id, at.shifted(dx, dy)))
            .collect(),
        districts: std::iter::once(root)
            .chain(packed.districts.into_iter().map(|d| District {
                at: d.at.shifted(dx, dy),
                ..d
            }))
            .collect(),
        size: (packed.w, packed.h),
    }
}

struct Packed {
    w: f64,
    h: f64,
    blocks: Vec<(u32, Placed)>,
    gates: Vec<(u32, Placed)>,
    districts: Vec<District>,
}

enum Kid {
    File(u32),
    Gate(u32),
    Dir(u32, Packed),
}

fn pack_dir(tree: &FileTree, open: &HashSet<u32>, m: &Measures, dir: u32) -> Packed {
    let node = &tree.dirs[dir as usize];
    let mut kids: Vec<(Kid, f64, f64)> = Vec::new();
    for &file in &node.files {
        let (w, h) = m
            .blocks
            .get(&file)
            .copied()
            .unwrap_or((BLOCK_MIN_W, BLOCK_HEAD_H + BLOCK_FOOT));
        kids.push((Kid::File(file), w, h));
    }
    for &child in &node.dirs {
        if open.contains(&child) {
            let packed = pack_dir(tree, open, m, child);
            let (w, h) = (packed.w, packed.h);
            kids.push((Kid::Dir(child, packed), w, h));
        } else {
            let (w, h) = m
                .gates
                .get(&child)
                .copied()
                .unwrap_or((BLOCK_MIN_W, GATE_H));
            kids.push((Kid::Gate(child), w, h));
        }
    }

    // Shelves aiming for a landscape district — the shape of the paper it
    // will be read on — and never narrower than its widest child. A district
    // that grew tall and thin would read as a column of unrelated plates.
    let widest = kids.iter().map(|(_, w, _)| *w).fold(0.0, f64::max);
    let area: f64 = kids.iter().map(|(_, w, h)| (w + GAP) * (h + GAP)).sum();
    let target = widest.max((area * 2.6).sqrt());

    let mut blocks: Vec<(u32, Placed)> = Vec::new();
    let mut gates: Vec<(u32, Placed)> = Vec::new();
    let mut districts: Vec<District> = Vec::new();
    let (mut x, mut y, mut row_h, mut content_w) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (kid, w, h) in kids {
        if x > 0.0 && x + w > target {
            y += row_h + GAP;
            x = 0.0;
            row_h = 0.0;
        }
        let at = Placed { x, y, w, h };
        match kid {
            Kid::File(file) => blocks.push((file, at)),
            Kid::Gate(child) => gates.push((child, at)),
            Kid::Dir(child, packed) => {
                districts.push(District {
                    dir: child,
                    at,
                    depth: tree.dirs[child as usize].depth,
                });
                blocks.extend(
                    packed
                        .blocks
                        .into_iter()
                        .map(|(id, p)| (id, p.shifted(x, y))),
                );
                gates.extend(
                    packed
                        .gates
                        .into_iter()
                        .map(|(id, p)| (id, p.shifted(x, y))),
                );
                districts.extend(packed.districts.into_iter().map(|d| District {
                    at: d.at.shifted(x, y),
                    ..d
                }));
            }
        }
        x += w + GAP;
        content_w = content_w.max(x - GAP);
        row_h = row_h.max(h);
    }
    let content_h = y + row_h;

    // The frame around them, with room on the border for the engraved label.
    let label = m.labels.get(&dir).copied().unwrap_or(0.0) + 30.0;
    let w = (content_w + D_PAD * 2.0).max(label).max(BLOCK_MIN_W);
    let h = content_h + D_PAD * 2.0 + D_LABEL_H;
    let (dx, dy) = (D_PAD, D_PAD + D_LABEL_H);
    Packed {
        w,
        h,
        blocks: blocks
            .into_iter()
            .map(|(id, at)| (id, at.shifted(dx, dy)))
            .collect(),
        gates: gates
            .into_iter()
            .map(|(id, at)| (id, at.shifted(dx, dy)))
            .collect(),
        districts: districts
            .into_iter()
            .map(|d| District {
                at: d.at.shifted(dx, dy),
                ..d
            })
            .collect(),
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
            changed: false,
            lines: 10,
            items: 1,
            refs_in_files: 0,
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
            items: Vec::new(),
            item_edges: Vec::new(),
            holds: Vec::new(),
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    /// Every file the same modest block; the geometry is what is under test.
    fn measures(g: &CodeGraph, tree: &FileTree) -> Measures {
        Measures {
            blocks: g.files.iter().map(|f| (f.id, (160.0, 60.0))).collect(),
            gates: tree.dirs.iter().map(|d| (d.id, (150.0, GATE_H))).collect(),
            labels: tree.dirs.iter().map(|d| (d.id, 80.0)).collect(),
        }
    }

    fn overlaps(a: &Placed, b: &Placed) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    fn contains(outer: &Placed, inner: &Placed) -> bool {
        outer.x <= inner.x
            && outer.y <= inner.y
            && outer.x + outer.w >= inner.x + inner.w
            && outer.y + outer.h >= inner.y + inner.h
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
            .flat_map(|d| (0..5).map(move |f| format!("a{d}/b/c/file{f}.rs")))
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
        let b = tree.dirs.iter().find(|d| d.path == "a/b").unwrap().id;
        let closed = open_dirs(&tree, 1, &HashSet::new());
        assert!(!closed.contains(&b));
        let mut toggled = HashSet::new();
        toggled.insert(b);
        let open = open_dirs(&tree, 1, &toggled);
        assert!(open.contains(&b));
    }

    #[test]
    fn territories_nest_and_never_overlap() {
        let g = graph(&[
            "a/one.rs",
            "a/two.rs",
            "b/three.rs",
            "b/sub/four.rs",
            "top.rs",
        ]);
        let tree = FileTree::build(&g);
        let open = open_dirs(&tree, 9, &HashSet::new());
        let layout = map_layout(&tree, &open, &measures(&g, &tree));

        // No two blocks share paper.
        let placed: Vec<Placed> = layout.blocks.values().copied().collect();
        for (i, a) in placed.iter().enumerate() {
            for b in &placed[i + 1..] {
                assert!(!overlaps(a, b), "blocks overlap: {a:?} {b:?}");
            }
        }

        let district = |path: &str| {
            let id = tree.dirs.iter().find(|d| d.path == path).unwrap().id;
            layout.districts.iter().find(|d| d.dir == id).unwrap().at
        };
        // Every file sits inside its own district, and a nested district
        // sits inside its parent.
        let (a, b, sub) = (district("a"), district("b"), district("b/sub"));
        assert!(contains(&b, &sub), "b/sub escapes b");
        assert!(!overlaps(&a, &b), "sibling districts overlap");
        let by_path = |path: &str| {
            let id = g.files.iter().find(|f| f.path == path).unwrap().id;
            layout.blocks[&id]
        };
        assert!(contains(&a, &by_path("a/one.rs")));
        assert!(contains(&sub, &by_path("b/sub/four.rs")));
        // Districts paint ancestors first, so a nested tint lays over its
        // parent's.
        let order: Vec<u32> = layout.districts.iter().map(|d| d.dir).collect();
        assert_eq!(order[0], ROOT);
        for (i, d) in layout.districts.iter().enumerate() {
            if let Some(parent) = tree.dirs[d.dir as usize].parent {
                let at = order.iter().position(|&x| x == parent).unwrap();
                assert!(at < i, "district {} paints before its parent", d.dir);
            }
        }
    }

    #[test]
    fn a_gate_takes_the_place_of_its_district() {
        let g = graph(&["a/one.rs", "a/deep/two.rs"]);
        let tree = FileTree::build(&g);
        let deep = tree.dirs.iter().find(|d| d.path == "a/deep").unwrap().id;
        let open = open_dirs(&tree, 1, &HashSet::new());
        let layout = map_layout(&tree, &open, &measures(&g, &tree));
        assert!(layout.gates.contains_key(&deep));
        assert!(!layout.districts.iter().any(|d| d.dir == deep));
        // The folded directory's file holds no ground of its own.
        let inner = g.files.iter().find(|f| f.path == "a/deep/two.rs").unwrap();
        assert!(!layout.blocks.contains_key(&inner.id));
    }
}
