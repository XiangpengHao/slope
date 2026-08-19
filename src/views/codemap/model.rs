//! The survey read as one containment tree.
//!
//! Every node the code altitude can draw — crate, directory, file, type,
//! method — sits in one tree, and every node has a fold state. References are
//! recorded between leaf items but always *rendered* between the lowest
//! containers the reader can currently see, with their counts summed. Fold a
//! file and the edges into its items gather onto its block; unfold it and they
//! redistribute. Nothing is ever dropped on the way: privacy is a permanent
//! fold, not a deletion, and a fold always states what it hides.
//!
//! This module is the arithmetic of that reading — pure functions over the
//! wire model, no layout and no rendering.

use std::collections::HashMap;

use crate::api::{CodeGraph, ItemMark, Vis};

/// The lowest container the reader can see: a file's block, or the gate of a
/// folded directory standing in for everything inside it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Territory {
    File(u32),
    Dir(u32),
}

/// Where a reference lands once it has been lifted to what is visible.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lifted {
    /// The item itself may be drawn.
    Item(u32),
    /// Private the whole way up: the reference belongs to this file's edge,
    /// counted but unnamed.
    Private(u32),
}

/// Parent chains are shallow by construction (file → type → method); the
/// bound only keeps a malformed link from spinning.
const MAX_DEPTH: usize = 8;

/// The containment tree, indexed by item mark.
#[derive(Clone, PartialEq, Debug)]
pub struct Containment {
    /// The outermost ancestor of every mark — the item the file itself holds.
    root: Vec<u32>,
    /// The file that root lives in: where the mark's territory is.
    home: Vec<u32>,
    /// Direct children, by parent mark, in source order.
    kids: HashMap<u32, Vec<u32>>,
}

impl Containment {
    pub fn build(graph: &CodeGraph) -> Self {
        let marks = &graph.items;
        let mut root = vec![0u32; marks.len()];
        let mut home = vec![0u32; marks.len()];
        for i in 0..marks.len() {
            let mut cur = i as u32;
            for _ in 0..MAX_DEPTH {
                match marks[cur as usize].parent {
                    Some(p) if (p as usize) < marks.len() && p != cur => cur = p,
                    _ => break,
                }
            }
            root[i] = cur;
            home[i] = marks[cur as usize].file;
        }
        let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
        for (i, mark) in marks.iter().enumerate() {
            if let Some(p) = mark.parent {
                kids.entry(p).or_default().push(i as u32);
            }
        }
        Self { root, home, kids }
    }

    /// The item the file holds directly — a method's type, a type itself.
    pub fn root(&self, mark: u32) -> u32 {
        self.root.get(mark as usize).copied().unwrap_or(mark)
    }

    /// The file whose block draws this mark. For a method written in another
    /// file's impl block, that is the type's file: an impl is attribution,
    /// not geometry.
    pub fn home(&self, mark: u32) -> u32 {
        self.home.get(mark as usize).copied().unwrap_or(0)
    }

    pub fn kids(&self, mark: u32) -> &[u32] {
        self.kids.get(&mark).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Every mark inside `mark`, itself included.
    pub fn inside(&self, mark: u32, out: &mut Vec<u32>) {
        out.push(mark);
        for &kid in self.kids(mark) {
            self.inside(kid, out);
        }
    }

    /// True when `mark` is `center` or sits inside it.
    pub fn within(&self, marks: &[ItemMark], center: u32, mark: u32) -> bool {
        let mut cur = mark;
        for _ in 0..MAX_DEPTH {
            if cur == center {
                return true;
            }
            match marks.get(cur as usize).and_then(|m| m.parent) {
                Some(p) if p != cur => cur = p,
                _ => return false,
            }
        }
        false
    }

    /// Lift a reference to the lowest ancestor that may be drawn. A private
    /// method lifts to its type; a private top-level item lifts to its file
    /// and shows there as a counted, unnamed line.
    pub fn lift(&self, marks: &[ItemMark], mark: u32) -> Lifted {
        let mut cur = mark;
        for _ in 0..MAX_DEPTH {
            let Some(m) = marks.get(cur as usize) else {
                break;
            };
            if m.vis != Vis::Private {
                return Lifted::Item(cur);
            }
            match m.parent {
                Some(p) if p != cur => cur = p,
                _ => break,
            }
        }
        Lifted::Private(self.home(mark))
    }
}

// ---------------------------------------------------------------------------
// Cartographic generalization: which landmarks the altitude engraves.
// ---------------------------------------------------------------------------

/// Landmark rows the whole map may carry before it stops being a map.
pub const LANDMARK_BUDGET: usize = 210;
/// Rows one file block engraves, however loud its items are.
pub const BLOCK_CAP: usize = 7;

/// How loudly an item asks to be named: item-level fan-in, the width of its
/// door, and whether this epoch touched its file. Private items never ask —
/// they are folded for good.
///
// TODO: `changed` is file-level. Item-level epoch ticks need the diff's hunks
// against item line ranges; the survey does not read hunks yet.
pub fn interest(mark: &ItemMark, changed: bool) -> u32 {
    if mark.vis == Vis::Private {
        return 0;
    }
    mark.fan_in + mark.vis.weight() + if changed { 2 } else { 0 }
}

/// The engraved weight of a landmark: three tiers by fan-in, so the eye reads
/// magnitude before it reads names.
pub fn tier(fan_in: u32) -> u8 {
    match fan_in {
        0..=2 => 3,
        3..=9 => 2,
        _ => 1,
    }
}

/// What one file block engraves, and what it says it hides.
#[derive(Clone, PartialEq, Debug)]
pub struct Block {
    pub file: u32,
    /// Landmark marks, in source order.
    pub rows: Vec<u32>,
    /// Items wide enough to draw that this altitude still folded away.
    pub quiet: u32,
    /// Private items, folded for good; their outside references lift here.
    pub private: u32,
}

impl Block {
    /// The fold's own words: counts in rust's vocabulary, and nothing else. A
    /// fold that does not count what it hides is a lie by omission — but the
    /// sentence explaining what "pub" means here, and where a private item's
    /// references go, belongs to the legend, said once, not to fifteen blocks
    /// saying it at each other.
    pub fn fold_words(&self) -> Option<String> {
        match (self.quiet, self.private) {
            (0, 0) => None,
            (q, 0) => Some(format!("+ {q} pub")),
            (0, p) => Some(format!("+ {p} private")),
            (q, p) => Some(format!("+ {q} pub · {p} private")),
        }
    }
}

/// Choose the landmarks for every visible file. The bar is the altitude's,
/// not the file's: the loudest items across the whole map are engraved until
/// the budget is spent, and every block still names its loudest item so no
/// block goes mute.
pub fn blocks(graph: &CodeGraph, visible: &[u32], containment: &Containment) -> Vec<Block> {
    let changed: Vec<bool> = graph.files.iter().map(|f| f.changed).collect();

    // Candidates are the items a file holds directly; methods and fields live
    // on their type's plate, one altitude down.
    let mut per_file: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut private: HashMap<u32, u32> = HashMap::new();
    for (i, mark) in graph.items.iter().enumerate() {
        if mark.parent.is_some() || containment.root(i as u32) != i as u32 {
            continue;
        }
        let file = mark.file;
        if mark.vis == Vis::Private {
            *private.entry(file).or_default() += 1;
            continue;
        }
        per_file.entry(file).or_default().push(i as u32);
    }

    // Each file's shortlist: loudest first, capped, so one enormous file
    // cannot spend the whole map's budget.
    let mut shortlists: Vec<(u32, Vec<u32>, usize)> = Vec::new();
    for &file in visible {
        let mut cands = per_file.remove(&file).unwrap_or_default();
        let touched = changed.get(file as usize).copied().unwrap_or(false);
        cands.sort_by_key(|&m| {
            let mark = &graph.items[m as usize];
            (
                std::cmp::Reverse(interest(mark, touched)),
                mark.line,
                mark.name.clone(),
            )
        });
        let total = cands.len();
        cands.truncate(BLOCK_CAP);
        shortlists.push((file, cands, total));
    }

    // The bar: the lowest interest the budget can afford across every block.
    let mut weights: Vec<u32> = shortlists
        .iter()
        .flat_map(|(file, rows, _)| {
            let touched = changed.get(*file as usize).copied().unwrap_or(false);
            rows.iter()
                .map(move |&m| interest(&graph.items[m as usize], touched))
        })
        .collect();
    weights.sort_unstable_by(|a, b| b.cmp(a));
    let bar = weights.get(LANDMARK_BUDGET).copied().unwrap_or(0);

    let mut out: Vec<Block> = Vec::with_capacity(shortlists.len());
    for (file, cands, total) in shortlists {
        let touched = changed.get(file as usize).copied().unwrap_or(false);
        let mut rows: Vec<u32> = cands
            .iter()
            .enumerate()
            .filter(|&(i, &m)| i == 0 || interest(&graph.items[m as usize], touched) > bar)
            .map(|(_, &m)| m)
            .collect();
        rows.sort_by_key(|&m| graph.items[m as usize].line);
        out.push(Block {
            file,
            quiet: (total - rows.len()) as u32,
            private: private.get(&file).copied().unwrap_or(0),
            rows,
        });
    }
    out.sort_by_key(|b| b.file);
    out
}

// ---------------------------------------------------------------------------
// Ambient coupling: territory-to-territory ties, aggregated.
// ---------------------------------------------------------------------------

/// Every reference between two territories, summed. The arrowhead rests on
/// the user — the way change travels.
#[derive(Clone, PartialEq, Debug)]
pub struct Tie {
    /// Where the definition lives.
    pub def: Territory,
    /// Where the reference is written.
    pub user: Territory,
    pub count: u32,
}

/// Aggregate every item-level reference into ties between the territories
/// currently on the paper. `territory` answers where a file is drawn right
/// now — its own block, or the gate of the folded directory holding it.
pub fn ties(
    graph: &CodeGraph,
    containment: &Containment,
    territory: impl Fn(u32) -> Option<Territory>,
) -> Vec<Tie> {
    let side = |item: Option<u32>, file: u32| -> Option<Territory> {
        // Containment governs: a method drawn under a type in another file
        // ties from that type's territory, not from the impl's file.
        let file = item.map(|m| containment.home(m)).unwrap_or(file);
        territory(file)
    };
    let mut acc: HashMap<(Territory, Territory), u32> = HashMap::new();
    for edge in &graph.item_edges {
        let (Some(user), Some(def)) =
            (side(edge.from, edge.from_file), side(edge.to, edge.to_file))
        else {
            continue;
        };
        if user == def {
            continue;
        }
        *acc.entry((def, user)).or_default() += edge.count;
    }
    let mut ties: Vec<Tie> = acc
        .into_iter()
        .map(|((def, user), count)| Tie { def, user, count })
        .collect();
    ties.sort_by(|a, b| {
        (a.def, a.user)
            .cmp(&(b.def, b.user))
            .then(b.count.cmp(&a.count))
    });
    ties
}

// ---------------------------------------------------------------------------
// The focus plate: one hop, grouped by container.
// ---------------------------------------------------------------------------

/// What the focus plate centers on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Center {
    File(u32),
    Item(u32),
}

/// Which way a focus column reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    /// Who leans on the selection.
    UsedBy,
    /// What the selection reaches for.
    Uses,
}

/// One row of a focus column: a named item, or the counted line standing in
/// for a file's folded private items.
#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    /// `None` for the lifted-private line.
    pub mark: Option<u32>,
    pub count: u32,
}

/// One container's rows, with the container's own total.
#[derive(Clone, PartialEq, Debug)]
pub struct Group {
    pub file: u32,
    pub total: u32,
    pub rows: Vec<Row>,
}

/// Group one hop of the selection's references by container: heaviest group
/// first, heaviest row first, private items lifted into one counted line.
///
/// `within` carries the references that never leave the selection's own file —
/// the file detail knows those, the global edges do not — as already-resolved
/// far-side marks. They are grouped by the same rules, so the plate owes its
/// own neighbors the reading it gives the rest of the workspace.
pub fn groups(
    graph: &CodeGraph,
    containment: &Containment,
    center: Center,
    dir: Dir,
    within: impl Iterator<Item = (Option<u32>, u32)>,
) -> Vec<Group> {
    let holds = |item: Option<u32>, file: u32| -> bool {
        match center {
            Center::File(f) => item.map(|m| containment.home(m)).unwrap_or(file) == f,
            Center::Item(c) => item.is_some_and(|m| containment.within(&graph.items, c, m)),
        }
    };
    let mut acc: HashMap<(u32, Option<u32>), u32> = HashMap::new();
    for edge in &graph.item_edges {
        let (near, near_file, far, far_file) = match dir {
            Dir::UsedBy => (edge.to, edge.to_file, edge.from, edge.from_file),
            Dir::Uses => (edge.from, edge.from_file, edge.to, edge.to_file),
        };
        if !holds(near, near_file) {
            continue;
        }
        let key = match far {
            Some(m) => match containment.lift(&graph.items, m) {
                Lifted::Item(m) => (containment.home(m), Some(m)),
                Lifted::Private(file) => (file, None),
            },
            // A reference to a file as a whole: its module, not its items.
            None => (far_file, None),
        };
        *acc.entry(key).or_default() += edge.count;
    }
    let home = match center {
        Center::File(f) => f,
        Center::Item(c) => containment.home(c),
    };
    for (mark, count) in within {
        let key = match mark {
            Some(m) => match containment.lift(&graph.items, m) {
                Lifted::Item(m) => (containment.home(m), Some(m)),
                Lifted::Private(f) => (f, None),
            },
            None => (home, None),
        };
        *acc.entry(key).or_default() += count;
    }
    collect_groups(graph, acc)
}

fn collect_groups(graph: &CodeGraph, acc: HashMap<(u32, Option<u32>), u32>) -> Vec<Group> {
    let mut by_file: HashMap<u32, Vec<Row>> = HashMap::new();
    for ((file, mark), count) in acc {
        by_file.entry(file).or_default().push(Row { mark, count });
    }
    let mut groups: Vec<Group> = by_file
        .into_iter()
        .map(|(file, mut rows)| {
            rows.sort_by_key(|r| {
                (
                    std::cmp::Reverse(r.count),
                    // The lifted-private line rests at the foot of its group.
                    r.mark.is_none(),
                    r.mark
                        .map(|m| graph.items[m as usize].name.clone())
                        .unwrap_or_default(),
                )
            });
            Group {
                total: rows.iter().map(|r| r.count).sum(),
                file,
                rows,
            }
        })
        .collect();
    groups.sort_by_key(|g| {
        (
            std::cmp::Reverse(g.total),
            graph
                .files
                .get(g.file as usize)
                .map(|f| f.path.clone())
                .unwrap_or_default(),
        )
    });
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{FileInfo, ItemEdge, ItemKind};

    fn file(id: u32, path: &str) -> FileInfo {
        FileInfo {
            id,
            path: path.to_string(),
            krate: "test".to_string(),
            changed: false,
            lines: 100,
            items: 2,
            fns: 1,
            types: 1,
            traits: 0,
            refs_in_files: 0,
            refs_out_files: 0,
        }
    }

    fn mark(id: u32, file: u32, name: &str, vis: Vis, parent: Option<u32>) -> ItemMark {
        ItemMark {
            id,
            file,
            local: id,
            name: name.to_string(),
            label: name.to_string(),
            kind: ItemKind::Fn,
            vis,
            line: id + 1,
            parent,
            fan_in: 0,
            impls: Vec::new(),
            plain_fields: 0,
            variants: Vec::new(),
            ty: String::new(),
        }
    }

    /// file 0: `Plate` (pub) with a private method `seat`;
    /// file 1: `draw` (pub) and `helper` (private).
    fn graph() -> CodeGraph {
        CodeGraph {
            files: vec![file(0, "src/plate.rs"), file(1, "src/draw.rs")],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Plate", Vis::Pub, None),
                mark(1, 0, "seat", Vis::Private, Some(0)),
                mark(2, 1, "draw", Vis::Pub, None),
                mark(3, 1, "helper", Vis::Private, None),
            ],
            item_edges: vec![
                // `draw` uses `Plate`.
                ItemEdge {
                    from_file: 1,
                    from: Some(2),
                    to_file: 0,
                    to: Some(0),
                    count: 3,
                },
                // A private helper uses `Plate` too: the coupling stays.
                ItemEdge {
                    from_file: 1,
                    from: Some(3),
                    to_file: 0,
                    to: Some(0),
                    count: 2,
                },
                // `Plate`'s private method reaches into the other file.
                ItemEdge {
                    from_file: 0,
                    from: Some(1),
                    to_file: 1,
                    to: Some(2),
                    count: 4,
                },
            ],
            holds: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    #[test]
    fn privates_lift_but_never_vanish() {
        let g = graph();
        let c = Containment::build(&g);
        // The private method lifts to its type; the private free function
        // lifts to its file.
        assert_eq!(c.lift(&g.items, 1), Lifted::Item(0));
        assert_eq!(c.lift(&g.items, 3), Lifted::Private(1));

        let used = groups(&g, &c, Center::Item(0), Dir::UsedBy, std::iter::empty());
        assert_eq!(used.len(), 1);
        let group = &used[0];
        assert_eq!(group.file, 1);
        // ×3 named + ×2 lifted, and the total says so.
        assert_eq!(group.total, 5);
        assert_eq!(group.rows.len(), 2);
        assert_eq!(group.rows[0].mark, Some(2));
        assert_eq!(group.rows[1].mark, None);
    }

    #[test]
    fn a_types_methods_count_as_the_type() {
        let g = graph();
        let c = Containment::build(&g);
        // The reference is written by `Plate::seat`; centered on `Plate`, it
        // is `Plate` reaching out.
        let uses = groups(&g, &c, Center::Item(0), Dir::Uses, std::iter::empty());
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].total, 4);
        assert_eq!(uses[0].rows[0].mark, Some(2));
    }

    #[test]
    fn ties_sum_between_territories() {
        let g = graph();
        let c = Containment::build(&g);
        let ties = ties(&g, &c, |f| Some(Territory::File(f)));
        assert_eq!(ties.len(), 2);
        let into_plate = ties
            .iter()
            .find(|t| t.def == Territory::File(0))
            .expect("a tie into the plate's file");
        assert_eq!(into_plate.user, Territory::File(1));
        assert_eq!(into_plate.count, 5);
    }

    #[test]
    fn folding_a_directory_gathers_its_ties() {
        let g = graph();
        let c = Containment::build(&g);
        // Both files behind one gate: the coupling is internal and no tie is
        // left to draw.
        let ties = ties(&g, &c, |_| Some(Territory::Dir(7)));
        assert!(ties.is_empty());
    }

    #[test]
    fn a_block_states_what_it_hides() {
        let g = graph();
        let c = Containment::build(&g);
        let blocks = blocks(&g, &[0, 1], &c);
        let draw = blocks.iter().find(|b| b.file == 1).unwrap();
        assert_eq!(draw.rows, vec![2]);
        assert_eq!(draw.private, 1);
        assert!(draw.fold_words().is_some_and(|w| w.contains("1 private")));
    }
}
