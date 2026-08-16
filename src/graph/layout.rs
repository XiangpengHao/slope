//! Board layout: ranking, crossing reduction, lattice placement, and routing.
//!
//! Topology is drawn, not simulated. A force layout would settle into a
//! hairball and give a different answer every run; this produces the same board
//! every time, and it produces it **once** — the client never re-lays it out,
//! because the whole interaction depends on the world holding still while the
//! camera moves.
//!
//! Two rules the rest of the file exists to serve:
//!
//! 1. **Pads are uniform and sit on a lattice.** Every crate is the same object
//!    on the same grid. Nothing about a crate deforms its pad, so the board can
//!    be read as a fabricated thing rather than as a chart.
//! 2. **Every dependency is routed.** All of them, at weight. The previous
//!    design drew one edge per crate as a "spine" and the other ~800 at 18%
//!    alpha, which meant most of the real dependency relationships in the
//!    workspace were rendered as noise. Density is what a backplane looks like.

use std::collections::VecDeque;

use super::route::{dedupe, octilinear};
use super::{Board, DuplicateGroup, Pad, Point, RawPackage, Trace};

/// Horizontal distance between rank columns. Wide enough that a trace has room
/// for its 45-degree chamfer before it reaches the next pad's entry port, and
/// wide enough that the columns read as countable stops — they are the depth
/// scale the whole board is indexed by.
const COL_PITCH: f32 = 330.0;
/// The lattice. Every pad sits on an exact multiple of this, in both axes.
const PITCH: f32 = 36.0;
/// Pitch for a routing waypoint. A channel needs room for its own copper and
/// nothing else. This number decides the board's proportions far more than the
/// pad pitch does — a busy middle column carries a few dozen pads and several
/// hundred channels — so it is what holds the board landscape on a desktop
/// window rather than letting it grow into a tower.
const LANE: f32 = 2.0;
/// Length of the horizontal stub a trace leaves and enters a pad on, so copper
/// meets a pad square rather than at an angle.
const PORT: f32 = 14.0;
const MARGIN: f32 = 160.0;
/// Crossing-reduction sweeps. Gains flatten out well before this.
const SWEEPS: usize = 6;
/// Straightening passes over the y coordinates, run before the lattice snap so
/// a trace that could be straight is straight.
const STRAIGHTEN: usize = 4;

/// A node in the layered graph: either a real crate or a routing waypoint that
/// keeps a long trace out of the pad columns it passes.
#[derive(Clone)]
struct LNode {
    rank: usize,
    /// Index into `packages` for a pad; `None` for a routing waypoint.
    pad: Option<usize>,
    order: f32,
    y: f32,
    up: Vec<usize>,
    down: Vec<usize>,
}

impl LNode {
    fn is_pad(&self) -> bool {
        self.pad.is_some()
    }
}

pub fn build(packages: Vec<RawPackage>, workspace: String, manifest_dir: String) -> Board {
    let n = packages.len();
    let ranks = rank(&packages);

    // --- Layer the graph, inserting waypoints for traces that span more than
    // one rank so copper never cuts through a pad column.
    let mut nodes: Vec<LNode> = Vec::with_capacity(n * 2);
    for (i, &rank) in ranks.iter().enumerate() {
        nodes.push(LNode {
            rank,
            pad: Some(i),
            order: 0.0,
            y: 0.0,
            up: Vec::new(),
            down: Vec::new(),
        });
    }
    // chains[e] is the layer-node path for edge e, pad to pad.
    let mut chains: Vec<(usize, usize, Vec<usize>)> = Vec::new();
    for (from, pkg) in packages.iter().enumerate() {
        for &to in &pkg.deps {
            let (r0, r1) = (ranks[from], ranks[to]);
            let mut chain = vec![from];
            if r1 > r0 + 1 {
                for r in (r0 + 1)..r1 {
                    let id = nodes.len();
                    nodes.push(LNode {
                        rank: r,
                        pad: None,
                        order: 0.0,
                        y: 0.0,
                        up: Vec::new(),
                        down: Vec::new(),
                    });
                    chain.push(id);
                }
            }
            chain.push(to);
            for w in chain.windows(2) {
                nodes[w[0]].down.push(w[1]);
                nodes[w[1]].up.push(w[0]);
            }
            chains.push((from, to, chain));
        }
    }

    // --- Order within each layer, then reduce crossings by barycentre sweeps.
    let depth = nodes.iter().map(|n| n.rank).max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); depth];
    for (i, node) in nodes.iter().enumerate() {
        layers[node.rank].push(i);
    }
    for layer in &mut layers {
        // Seed deterministically: pads before waypoints, then by id.
        layer.sort_by_key(|&i| (!nodes[i].is_pad(), i));
    }
    reindex(&mut nodes, &layers);

    for sweep in 0..SWEEPS {
        let downward = sweep % 2 == 0;
        // Sweeping down orders each layer against the one above it, sweeping up
        // against the one below; alternating is what drives crossings out.
        let span = depth.saturating_sub(1);
        for step in 0..span {
            let r = if downward { step + 1 } else { span - 1 - step };
            sort_layer(&mut nodes, &mut layers[r], downward);
            reindex_layer(&mut nodes, &layers[r]);
        }
    }

    // --- Coordinates. Start from the slot grid, then pull each node toward its
    // neighbours' mean while holding the minimum separation, so a trace that
    // could be straight is straight before anything snaps.
    for layer in &layers {
        let mut y = MARGIN;
        let mut previous: Option<usize> = None;
        for &i in layer {
            if let Some(p) = previous {
                y += separation(&nodes[p], &nodes[i]);
            }
            nodes[i].y = y;
            previous = Some(i);
        }
    }
    for pass in 0..STRAIGHTEN {
        let downward = pass % 2 == 0;
        for step in 0..depth {
            let r = if downward { step } else { depth - 1 - step };
            let desired: Vec<f32> = layers[r]
                .iter()
                .map(|&i| {
                    let neighbours = if downward { &nodes[i].up } else { &nodes[i].down };
                    if neighbours.is_empty() {
                        nodes[i].y
                    } else {
                        neighbours.iter().map(|&j| nodes[j].y).sum::<f32>()
                            / neighbours.len() as f32
                    }
                })
                .collect();
            // Sweep in slot order, never letting a node pass its neighbour.
            let mut floor = f32::NEG_INFINITY;
            let mut previous: Option<usize> = None;
            for (slot, &i) in layers[r].iter().enumerate() {
                if let Some(p) = previous {
                    floor = nodes[p].y + separation(&nodes[p], &nodes[i]);
                }
                nodes[i].y = desired[slot].max(floor);
                previous = Some(i);
            }

            // That sweep can only push down, so repeating it walks every layer
            // steadily downward and bends the board into a wedge. Re-centre the
            // layer on where its nodes actually wanted to sit; separations are
            // preserved because the whole layer moves together.
            if !layers[r].is_empty() {
                let count = layers[r].len() as f32;
                let placed: f32 = layers[r].iter().map(|&i| nodes[i].y).sum::<f32>() / count;
                let wanted: f32 = desired.iter().sum::<f32>() / count;
                let shift = wanted - placed;
                for &i in &layers[r] {
                    nodes[i].y += shift;
                }
            }
        }
    }

    // Centre every column on one axis. Without this the columns sit wherever
    // the relaxation left them and the board reads as a slope. Centre on where
    // the *pads* sit, not on the routing channels: a column holding one pad and
    // two hundred channels must put that pad on the axis, not the channels.
    let centre_of = |nodes: &[LNode], group: &[usize]| -> Option<f32> {
        let pads: Vec<f32> = group
            .iter()
            .filter(|&&i| nodes[i].is_pad())
            .map(|&i| nodes[i].y)
            .collect();
        let sample = if pads.is_empty() {
            group.iter().map(|&i| nodes[i].y).collect()
        } else {
            pads
        };
        (!sample.is_empty()).then(|| sample.iter().sum::<f32>() / sample.len() as f32)
    };
    let all: Vec<usize> = (0..nodes.len()).collect();
    if let Some(axis) = centre_of(&nodes, &all) {
        for layer in &layers {
            let Some(mean) = centre_of(&nodes, layer) else {
                continue;
            };
            let shift = axis - mean;
            for &i in layer {
                nodes[i].y += shift;
            }
        }
    }

    // --- Seat everything on the lattice. Up to here the relaxation has been
    // working in continuous space to get traces straight; a board does not have
    // continuous space. Pads land on exact multiples of PITCH, waypoints on a
    // quarter of it, so every bend in the copper is on the grid too.
    seat_on_lattice(&mut nodes, &layers);

    // Normalise so the board starts at the margin, still on the lattice.
    let min_y = nodes.iter().map(|n| n.y).fold(f32::INFINITY, f32::min);
    let shift = ((MARGIN - min_y) / PITCH).round() * PITCH;
    for node in &mut nodes {
        node.y += shift;
    }

    let x_of = |rank: usize| MARGIN + rank as f32 * COL_PITCH;
    let pos = |nodes: &[LNode], i: usize| Point {
        x: x_of(nodes[i].rank),
        y: nodes[i].y,
    };

    // --- Route every dependency. All of them, through their own channels.
    let mut traces: Vec<Trace> = Vec::with_capacity(chains.len());
    for (from, to, chain) in &chains {
        let start = pos(&nodes, chain[0]);
        let end = pos(&nodes, *chain.last().unwrap());
        let mut points = vec![start];

        let mut waypoints: Vec<Point> = Vec::with_capacity(chain.len());
        waypoints.push(Point {
            x: start.x + PORT,
            y: start.y,
        });
        for &mid in &chain[1..chain.len() - 1] {
            waypoints.push(pos(&nodes, mid));
        }
        waypoints.push(Point {
            x: end.x - PORT,
            y: end.y,
        });

        points.push(waypoints[0]);
        for pair in waypoints.windows(2) {
            points.extend(octilinear(pair[0], pair[1]));
            points.push(pair[1]);
        }
        points.push(end);
        dedupe(&mut points);

        traces.push(Trace {
            from: *from,
            to: *to,
            points,
        });
    }

    // --- Pads.
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, pkg) in packages.iter().enumerate() {
        for &d in &pkg.deps {
            dependents[d].push(i);
        }
    }
    let duplicates = duplicate_groups(&packages);
    let duplicated: Vec<bool> = {
        let mut flags = vec![false; n];
        for group in &duplicates {
            for &id in &group.pad_ids {
                flags[id] = true;
            }
        }
        flags
    };

    let pads: Vec<Pad> = packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| Pad {
            id: i,
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            x: x_of(ranks[i]),
            y: nodes[i].y,
            rank: ranks[i],
            deps: pkg.deps.clone(),
            dependents: std::mem::take(&mut dependents[i]),
            is_root: pkg.is_root,
            duplicate: duplicated[i],
        })
        .collect();

    let width = pads.iter().map(|p| p.x).fold(0.0, f32::max) + MARGIN;
    let height = pads.iter().map(|p| p.y).fold(0.0, f32::max) + MARGIN;

    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();

    Board {
        workspace,
        manifest_dir,
        package_count: n,
        distinct_count: names.len(),
        pads,
        traces,
        duplicates,
        width,
        height,
        pitch: PITCH,
        col_pitch: COL_PITCH,
    }
}

/// Snap every node to the lattice, holding the order the crossing reduction
/// worked out. Pads take whole lattice rows and never share one; waypoints take
/// quarter rows, because a channel only has to clear its neighbour's copper.
fn seat_on_lattice(nodes: &mut [LNode], layers: &[Vec<usize>]) {
    for layer in layers {
        // Split first: the seating walk mutates the same nodes the filter reads.
        let (pads, channels): (Vec<usize>, Vec<usize>) =
            layer.iter().partition(|&&i| nodes[i].is_pad());

        // Pads: round to the nearest row, then walk the column pushing any pad
        // that would land on or above its predecessor down to the next free row.
        // Rounding alone can put two pads on one row; the walk is what makes the
        // lattice a seating plan rather than a suggestion.
        let mut previous_row: Option<i64> = None;
        for i in pads {
            let mut row = (nodes[i].y / PITCH).round() as i64;
            if let Some(p) = previous_row
                && row <= p
            {
                row = p + 1;
            }
            nodes[i].y = row as f32 * PITCH;
            previous_row = Some(row);
        }

        // Channels keep their relaxed position, seated on a sub-lattice and held
        // clear of each other so parallel copper stays parallel.
        let lane = LANE;
        let mut floor = f32::NEG_INFINITY;
        for i in channels {
            let y = ((nodes[i].y / lane).round() * lane).max(floor);
            nodes[i].y = y;
            floor = y + lane;
        }
    }
}

/// Longest-path ranking, so every trace points forward and no pad sits in the
/// same column as something it depends on. This is the law of the board:
/// dependencies right, dependents left, everywhere, always.
fn rank(packages: &[RawPackage]) -> Vec<usize> {
    let n = packages.len();
    let mut indegree = vec![0usize; n];
    for pkg in packages {
        for &d in &pkg.deps {
            indegree[d] += 1;
        }
    }
    let mut ranks = vec![0usize; n];
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut settled = 0usize;
    while let Some(u) = queue.pop_front() {
        settled += 1;
        for &d in &packages[u].deps {
            ranks[d] = ranks[d].max(ranks[u] + 1);
            indegree[d] -= 1;
            if indegree[d] == 0 {
                queue.push_back(d);
            }
        }
    }
    // Dev edges are already filtered out, so a cycle here would be unusual.
    // Place anything left over past its deepest settled predecessor rather than
    // dropping it off the board.
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
    for (i, pkg) in packages.iter().enumerate() {
        by_name.entry(pkg.name.as_str()).or_default().push(i);
    }
    by_name
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(name, ids)| DuplicateGroup {
            name: name.to_string(),
            versions: ids.iter().map(|&i| packages[i].version.clone()).collect(),
            pad_ids: ids,
        })
        .collect()
}

fn sort_layer(nodes: &mut [LNode], layer: &mut [usize], from_above: bool) {
    let mut keyed: Vec<(usize, f32)> = layer
        .iter()
        .map(|&i| {
            let neighbours = if from_above { &nodes[i].up } else { &nodes[i].down };
            let bary = if neighbours.is_empty() {
                nodes[i].order
            } else {
                neighbours.iter().map(|&j| nodes[j].order).sum::<f32>() / neighbours.len() as f32
            };
            (i, bary)
        })
        .collect();
    keyed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    for (slot, (i, _)) in keyed.iter().enumerate() {
        layer[slot] = *i;
    }
}

fn reindex(nodes: &mut [LNode], layers: &[Vec<usize>]) {
    for layer in layers {
        reindex_layer(nodes, layer);
    }
}

fn reindex_layer(nodes: &mut [LNode], layer: &[usize]) {
    for (slot, &i) in layer.iter().enumerate() {
        nodes[i].order = slot as f32;
    }
}

/// Minimum gap between two neighbours in a column. Pads claim a whole lattice
/// row; routing channels claim only their own copper.
fn separation(a: &LNode, b: &LNode) -> f32 {
    let pitch = |node: &LNode| if node.is_pad() { PITCH } else { LANE };
    (pitch(a) + pitch(b)) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::metadata;

    /// Build the board for whichever workspace the tests run in, which is this
    /// one: a real 380-package graph rather than a toy fixture.
    fn real_board() -> Board {
        let resolved = metadata::resolve().expect("cargo metadata should resolve this workspace");
        build(resolved.packages, resolved.workspace, resolved.manifest_dir)
    }

    /// The law of the board. Everything else in the design leans on this: it is
    /// why "left" can mean dependents and "right" dependencies at every zoom.
    #[test]
    fn ranks_move_forward_along_every_edge() {
        let board = real_board();
        for pad in &board.pads {
            for &dep in &pad.deps {
                assert!(
                    board.pads[dep].rank > pad.rank,
                    "{} (rank {}) depends on {} (rank {}); its copper would run backwards",
                    pad.name,
                    pad.rank,
                    board.pads[dep].name,
                    board.pads[dep].rank
                );
            }
        }
    }

    /// Uniform objects on a uniform grid: the brief's first constraint, and the
    /// one a relaxation pass would quietly erode.
    #[test]
    fn every_pad_is_seated_on_the_lattice() {
        let board = real_board();
        for pad in &board.pads {
            let rows = pad.y / board.pitch;
            assert!(
                (rows - rows.round()).abs() < 0.01,
                "{} sits at y={:.3}, which is {:.3} rows — off the lattice",
                pad.name,
                pad.y,
                rows
            );
            let expected = MARGIN + pad.rank as f32 * board.col_pitch;
            assert!(
                (pad.x - expected).abs() < 0.01,
                "{} sits at x={:.1}, not in its rank's column ({expected:.1})",
                pad.name,
                pad.x
            );
        }
    }

    #[test]
    fn pads_in_a_column_never_share_a_row() {
        let board = real_board();
        let mut by_rank: std::collections::BTreeMap<usize, Vec<f32>> = Default::default();
        for pad in &board.pads {
            by_rank.entry(pad.rank).or_default().push(pad.y);
        }
        for (rank, mut ys) in by_rank {
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in ys.windows(2) {
                assert!(
                    pair[1] - pair[0] >= board.pitch - 0.01,
                    "two pads in column {rank} are {:.1} apart, closer than the {} lattice",
                    pair[1] - pair[0],
                    board.pitch
                );
            }
        }
    }

    /// Density is the material. The design this replaced drew one edge per crate
    /// at weight and the other ~800 at 18% alpha; if a trace ever goes missing
    /// again, it should fail here rather than in someone's reading of the board.
    #[test]
    fn every_dependency_is_routed() {
        let board = real_board();
        let declared: usize = board.pads.iter().map(|p| p.deps.len()).sum();
        assert_eq!(
            board.traces.len(),
            declared,
            "the board routes {} traces for {declared} declared dependencies",
            board.traces.len()
        );
        for trace in &board.traces {
            assert!(
                trace.points.len() >= 2,
                "the trace {} -> {} has no copper",
                board.pads[trace.from].name,
                board.pads[trace.to].name
            );
        }
    }

    #[test]
    fn every_trace_is_octilinear() {
        let board = real_board();
        for trace in &board.traces {
            for pair in trace.points.windows(2) {
                let dx = (pair[1].x - pair[0].x).abs();
                let dy = (pair[1].y - pair[0].y).abs();
                let horizontal = dy < 0.6;
                let vertical = dx < 0.6;
                let diagonal = (dx - dy).abs() < 0.6;
                assert!(
                    horizontal || vertical || diagonal,
                    "the trace {}->{} has a segment at neither 0, 45, nor 90 degrees: \
                     ({:.1},{:.1}) -> ({:.1},{:.1})",
                    board.pads[trace.from].name,
                    board.pads[trace.to].name,
                    pair[0].x,
                    pair[0].y,
                    pair[1].x,
                    pair[1].y
                );
            }
        }
    }

    #[test]
    fn geometry_is_finite_and_positive() {
        let board = real_board();
        assert!(board.width.is_finite() && board.width > 0.0);
        assert!(board.height.is_finite() && board.height > 0.0);
        assert!(board.pitch > 0.0 && board.col_pitch > 0.0);
        for pad in &board.pads {
            assert!(pad.x.is_finite() && pad.y.is_finite());
            assert!(pad.y >= 0.0, "{} sits above the board", pad.name);
        }
    }

    #[test]
    fn the_workspace_is_on_the_board_and_every_pad_is_named() {
        let board = real_board();
        assert!(board.package_count > 0);
        assert_eq!(board.pads.len(), board.package_count);
        assert!(board.pads.iter().any(|p| p.is_root));
        for pad in &board.pads {
            assert!(!pad.name.is_empty(), "pad {} has no name", pad.id);
            assert_eq!(pad.designator(), format!("P{:03}", pad.id));
        }
    }

    /// The wedge check: if columns drifted steadily downward the board would
    /// bend into a triangle, so no column's centre may sit far from the board's.
    #[test]
    fn columns_stay_centred_rather_than_drifting() {
        let board = real_board();
        let mut by_rank: std::collections::BTreeMap<usize, Vec<f32>> = Default::default();
        for pad in &board.pads {
            by_rank.entry(pad.rank).or_default().push(pad.y);
        }
        let centre = board.height / 2.0;
        for (rank, ys) in by_rank {
            let mean = ys.iter().sum::<f32>() / ys.len() as f32;
            let drift = (mean - centre).abs() / board.height;
            assert!(
                drift < 0.35,
                "column {rank} sits {:.0}% of the board away from centre; it is bending",
                drift * 100.0
            );
        }
    }

    #[test]
    fn report_board_shape() {
        let board = real_board();
        let mut per_rank: std::collections::BTreeMap<usize, usize> = Default::default();
        for pad in &board.pads {
            *per_rank.entry(pad.rank).or_default() += 1;
        }
        let widest = per_rank.values().copied().max().unwrap_or(0);
        let shared = board
            .pads
            .iter()
            .filter(|p| p.dependents.len() >= 2)
            .count();
        let majors = board.pads.iter().filter(|p| p.major()).count();
        let tier0 = board.pads.iter().filter(|p| p.legend_tier() == 0).count();
        let tier1 = board.pads.iter().filter(|p| p.legend_tier() == 1).count();
        println!(
            "board {:.0}x{:.0} (aspect {:.2})  pitch {}  columns {}  widest column {widest}",
            board.width,
            board.height,
            board.width / board.height,
            board.pitch,
            per_rank.len(),
        );
        println!(
            "packages {}  crates {}  traces {}  shared {shared}  major {majors}  \
             duplicate groups {}  legend tier0 {tier0} tier1 {tier1}",
            board.package_count,
            board.distinct_count,
            board.traces.len(),
            board.duplicates.len(),
        );
    }
}
