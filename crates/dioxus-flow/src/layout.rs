//! Layered placement: the layout that keeps the graph a readable DAG while the
//! reader opens it.
//!
//! The x axis is a law, not a layout convenience: a node's column is handed in
//! by the lens (hops from the workspace, hops from the held function) and this
//! only compacts the occupied columns onto a pitch. Everything a node points at
//! is strictly to its right, at every zoom, without exception.
//!
//! Everything else here is the classic layered pipeline, and each stage exists
//! because the stage before it is not enough on its own:
//!
//! 1. **Lanes.** An edge that spans more than one column gets a waypoint in
//!    every column it crosses. Without them a long wire is drawn straight over
//!    whatever cards happen to sit in the way, and no amount of reordering fixes
//!    that — the wire is not *in* those columns, so nothing there knows to move
//!    aside. With them the wire takes part in the ordering like any other node
//!    and the columns open a lane for it.
//! 2. **Ordering.** Median sweeps put each node near its neighbours; a transpose
//!    pass then swaps adjacent pairs while that actually removes crossings.
//!    Sweeps alone plateau early — every child of one freshly opened card has
//!    the same median, so a sweep has no opinion about their order at all, and
//!    the transpose is what untangles them against the column beyond.
//! 3. **Coordinates.** Slot order says who is above whom; it does not say where.
//!    Each node is pulled to the median of what it is attached to and then the
//!    column is opened just enough to keep everyone apart, which is what makes a
//!    chain come out straight and a parent sit level with its children.
//!
//! Expanding a card re-runs all of it. The previous frame seeds the ordering, so
//! the arrangement stays as close to what the reader was looking at as the new
//! topology allows — but the graph is re-tidied, because a picture that stays
//! put while it stops being a legible DAG has kept the wrong promise.

use std::collections::HashMap;

/// Ordering rounds. Each is a median sweep in one direction plus a transpose.
const ROUNDS: usize = 8;
/// Coordinate relaxation passes, alternating direction.
const RELAX: usize = 8;

/// What the layout measures in. Everything is in world units, and everything is
/// stated along the flow or across it rather than as width and height, so the
/// same pipeline lays out a left-to-right graph and a top-to-bottom one.
///
/// A lane claims no extent of its own — it is one wire's opinion about where it
/// should be, not an object — and only enough air to clear its neighbour's
/// stroke. Lanes are cheap on purpose: a busy column carries a few dozen nodes
/// and several hundred wires, and charging each wire a node's worth of room is
/// what turns a graph into a tower.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Metrics {
    /// A node's extent along the flow.
    pub along: f32,
    /// A node's extent across it.
    pub across: f32,
    /// Distance between one column's leading edge and the next.
    pub pitch: f32,
    /// Air between two nodes in the same column.
    pub gap: f32,
    /// Air between two lanes.
    pub lane_gap: f32,
    /// Air where a node and a lane are neighbours.
    pub node_lane_gap: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            along: 190.0,
            across: 48.0,
            pitch: 280.0,
            gap: 20.0,
            lane_gap: 7.0,
            node_lane_gap: 12.0,
        }
    }
}

/// A node to place: its identity, and the column it belongs in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Slot {
    pub id: usize,
    /// Smaller is further left. Values need not be contiguous — gaps are
    /// compacted away, so a graph occupying columns 0, 4 and 9 draws as three
    /// adjacent columns rather than as two screens of empty pane.
    pub column: i32,
}

/// Where a node landed: the leading corner of its box, along the flow and
/// across it. A caller maps that onto the pane's axes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Placement {
    pub id: usize,
    pub along: f32,
    pub across: f32,
}

/// A wire, as the points it actually runs through: the source's handle, a
/// waypoint in every column it crosses, and the target's handle — each in the
/// same along/across space as a [`Placement`].
#[derive(Clone, PartialEq, Debug)]
pub struct Wire {
    pub from: usize,
    pub to: usize,
    pub points: Vec<(f32, f32)>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Layout {
    pub places: Vec<Placement>,
    pub wires: Vec<Wire>,
}

/// One node in the layered graph: a card, or a lane for one wire.
#[derive(Clone)]
struct Cell {
    column: usize,
    /// The id of the card this is, or `None` for a lane.
    card: Option<usize>,
    /// Position within its column, as a float so a median can be compared to it.
    order: f32,
    /// Vertical centre, in world units.
    y: f32,
    left: Vec<usize>,
    right: Vec<usize>,
}

impl Cell {
    fn half(&self, metrics: &Metrics) -> f32 {
        if self.card.is_some() {
            metrics.across / 2.0
        } else {
            0.0
        }
    }
}

fn separation(a: &Cell, b: &Cell, metrics: &Metrics) -> f32 {
    let gap = match (a.card.is_none(), b.card.is_none()) {
        (true, true) => metrics.lane_gap,
        (false, false) => metrics.gap,
        _ => metrics.node_lane_gap,
    };
    a.half(metrics) + b.half(metrics) + gap
}

/// Who gets their way when a column cannot give everyone what they asked for.
///
/// A lane outranks every card, because a lane is one wire's opinion about where
/// it should be and a straight wire is worth more than a card sitting exactly on
/// its median. Among cards, the busiest wins: it has the most wires to keep
/// straight and the most to lose by being pushed.
fn priority(cell: &Cell) -> usize {
    if cell.card.is_none() {
        usize::MAX
    } else {
        cell.left.len() + cell.right.len()
    }
}

/// Place `slots` into columns and route `edges` between them.
///
/// `previous` is the vertical centre each card had in the last frame. It seeds
/// the ordering, so re-tidying after an expand moves what the topology forces
/// and not what it does not.
pub fn layered(
    slots: &[Slot],
    edges: &[(usize, usize)],
    previous: &HashMap<usize, f32>,
    metrics: &Metrics,
) -> Layout {
    if slots.is_empty() {
        return Layout::default();
    }

    // --- Columns, compacted onto a pitch.
    let mut keys: Vec<i32> = slots.iter().map(|slot| slot.column).collect();
    keys.sort_unstable();
    keys.dedup();
    let column_of: HashMap<i32, usize> = keys
        .iter()
        .enumerate()
        .map(|(index, &key)| (key, index))
        .collect();

    // --- Cards first, so a card's cell index is stable across the run.
    let mut cells: Vec<Cell> = slots
        .iter()
        .map(|slot| Cell {
            column: column_of[&slot.column],
            card: Some(slot.id),
            order: 0.0,
            y: 0.0,
            left: Vec::new(),
            right: Vec::new(),
        })
        .collect();
    let cell_of: HashMap<usize, usize> = slots
        .iter()
        .enumerate()
        .map(|(index, slot)| (slot.id, index))
        .collect();

    // --- Lanes. An edge crossing a column gets a waypoint in it, so the
    // ordering can open a lane rather than letting the wire cut across cards.
    let mut chains: Vec<(usize, usize, Vec<usize>)> = Vec::new();
    for &(from, to) in edges {
        let (Some(&a), Some(&b)) = (cell_of.get(&from), cell_of.get(&to)) else {
            continue;
        };
        let (start, end) = (cells[a].column, cells[b].column);
        if start == end {
            continue;
        }
        let mut chain = vec![a];
        let step: i32 = if end > start { 1 } else { -1 };
        let mut column = start as i32 + step;
        while column != end as i32 {
            let lane = cells.len();
            cells.push(Cell {
                column: column as usize,
                card: None,
                order: 0.0,
                y: 0.0,
                left: Vec::new(),
                right: Vec::new(),
            });
            chain.push(lane);
            column += step;
        }
        chain.push(b);
        for pair in chain.windows(2) {
            let (earlier, later) = if cells[pair[0]].column < cells[pair[1]].column {
                (pair[0], pair[1])
            } else {
                (pair[1], pair[0])
            };
            cells[earlier].right.push(later);
            cells[later].left.push(earlier);
        }
        chains.push((from, to, chain));
    }

    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); keys.len()];
    for (index, cell) in cells.iter().enumerate() {
        columns[cell.column].push(index);
    }

    // --- Seed the order. A card the reader was already looking at keeps its
    // place; anything new is seeded on whatever it is attached to that has one.
    let mut seed: Vec<f32> = vec![f32::MAX; cells.len()];
    for (index, cell) in cells.iter().enumerate() {
        if let Some(id) = cell.card
            && let Some(&y) = previous.get(&id)
        {
            seed[index] = y;
        }
    }
    for _ in 0..3 {
        for index in 0..cells.len() {
            if seed[index] != f32::MAX {
                continue;
            }
            let known: Vec<f32> = cells[index]
                .left
                .iter()
                .chain(cells[index].right.iter())
                .map(|&next| seed[next])
                .filter(|value| *value != f32::MAX)
                .collect();
            if !known.is_empty() {
                seed[index] = known.iter().sum::<f32>() / known.len() as f32;
            }
        }
    }
    for column in &mut columns {
        column.sort_by(|&a, &b| {
            seed[a]
                .partial_cmp(&seed[b])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
    }
    reindex(&columns, &mut cells);

    // --- Ordering. Sweeps put each node near its neighbours; the transpose
    // then earns the crossings the sweeps cannot see.
    let mut best = columns.clone();
    let mut best_score = crossings(&columns, &cells);
    let mut barren = 0;
    for round in 0..ROUNDS {
        median_sweep(&mut columns, &mut cells, round % 2 == 0);
        transpose(&mut columns, &mut cells);
        let score = crossings(&columns, &cells);
        if score < best_score {
            best_score = score;
            best = columns.clone();
            barren = 0;
        } else {
            barren += 1;
            // Two rounds that buy nothing mean the ordering has settled, and
            // the rest are close to pure cost on a graph large enough to
            // notice. Measured on a 700-card, 2600-wire workspace, stopping
            // here rather than running all eight rounds costs 159 crossings
            // out of 192,000 — under a tenth of a percent — and halves the
            // time the reader waits for the picture.
            if barren == 2 {
                break;
            }
        }
    }
    columns = best;
    reindex(&columns, &mut cells);

    // --- Coordinates.
    place(&columns, &mut cells, metrics);

    // --- Out.
    let along_of = |column: usize| column as f32 * metrics.pitch;
    let mut places: Vec<Placement> = cells
        .iter()
        .filter_map(|cell| {
            cell.card.map(|id| Placement {
                id,
                along: along_of(cell.column),
                across: cell.y - metrics.across / 2.0,
            })
        })
        .collect();
    places.sort_by_key(|place| place.id);

    let wires: Vec<Wire> = chains
        .iter()
        .map(|(from, to, chain)| {
            let first = &cells[chain[0]];
            let last = &cells[chain[chain.len() - 1]];
            let mut points = Vec::with_capacity(chain.len());
            // A wire leaves the far edge of its source and arrives at the near
            // edge of its target; a lane sits in the middle of its column.
            points.push((along_of(first.column) + metrics.along, first.y));
            for &lane in &chain[1..chain.len() - 1] {
                points.push((
                    along_of(cells[lane].column) + metrics.along / 2.0,
                    cells[lane].y,
                ));
            }
            points.push((along_of(last.column), last.y));
            straighten(&mut points);
            Wire {
                from: *from,
                to: *to,
                points,
            }
        })
        .collect();

    Layout { places, wires }
}

/// Drop the waypoints a wire does not need.
///
/// The coordinate stage exists to make a long wire straight, and it succeeds:
/// a wire crossing twelve columns usually comes out with twelve waypoints all on
/// the same line. Keeping them costs a cubic segment each — on a
/// dependency-heavy workspace that was 1.4 million characters of path data,
/// rebuilt every time the reader folded anything.
///
/// The lanes still did their job: they are what *made* the columns open up. This
/// only stops the drawing from carrying the scaffolding around afterwards.
fn straighten(points: &mut Vec<(f32, f32)>) {
    /// How far off the line between its neighbours a waypoint may sit and still
    /// be dropped. Under half a world unit is well under a pixel at any
    /// magnification this pane reaches.
    const FLAT: f32 = 0.4;

    if points.len() < 3 {
        return;
    }
    let mut kept: Vec<(f32, f32)> = Vec::with_capacity(points.len());
    kept.push(points[0]);
    for index in 1..points.len() - 1 {
        let previous = *kept.last().unwrap();
        let (here, next) = (points[index], points[index + 1]);
        // Twice the area of the triangle the three points make, over the span:
        // the distance from `here` to the line through `previous` and `next`.
        let (ax, ay) = (next.0 - previous.0, next.1 - previous.1);
        let (bx, by) = (here.0 - previous.0, here.1 - previous.1);
        let span = (ax * ax + ay * ay).sqrt();
        let off = if span > 0.0 {
            (ax * by - ay * bx).abs() / span
        } else {
            f32::INFINITY
        };
        if off > FLAT {
            kept.push(here);
        }
    }
    kept.push(points[points.len() - 1]);
    *points = kept;
}

fn reindex(columns: &[Vec<usize>], cells: &mut [Cell]) {
    for column in columns {
        for (slot, &index) in column.iter().enumerate() {
            cells[index].order = slot as f32;
        }
    }
}

/// The median of what a cell is attached to on one side, or `None` when it is
/// attached to nothing there — in which case it keeps the slot it has, rather
/// than being swept to an end it has no reason to be at.
fn median(cell: &Cell, cells: &[Cell], from_left: bool) -> Option<f32> {
    let neighbours = if from_left { &cell.left } else { &cell.right };
    if neighbours.is_empty() {
        return None;
    }
    let mut orders: Vec<f32> = neighbours.iter().map(|&next| cells[next].order).collect();
    orders.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let middle = orders.len() / 2;
    Some(if orders.len() % 2 == 1 {
        orders[middle]
    } else {
        (orders[middle - 1] + orders[middle]) / 2.0
    })
}

fn median_sweep(columns: &mut [Vec<usize>], cells: &mut [Cell], rightward: bool) {
    let count = columns.len();
    for step in 0..count {
        let c = if rightward { step } else { count - 1 - step };
        if (rightward && c == 0) || (!rightward && c + 1 == count) {
            continue;
        }
        let keyed: Vec<(usize, f32)> = columns[c]
            .iter()
            .map(|&index| {
                let key = median(&cells[index], cells, rightward).unwrap_or(cells[index].order);
                (index, key)
            })
            .collect();
        let mut ranked = keyed;
        ranked.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(cells[a.0].order.partial_cmp(&cells[b.0].order).unwrap())
        });
        for (slot, (index, _)) in ranked.iter().enumerate() {
            columns[c][slot] = *index;
        }
        for (slot, &index) in columns[c].iter().enumerate() {
            cells[index].order = slot as f32;
        }
    }
}

/// Crossings the two cells contribute between themselves, in the order given.
/// Local, because that is all a swap can change.
fn pair_crossings(a: usize, b: usize, cells: &[Cell]) -> usize {
    let mut total = 0;
    for left in [true, false] {
        let (first, second) = if left {
            (&cells[a].left, &cells[b].left)
        } else {
            (&cells[a].right, &cells[b].right)
        };
        for &p in first {
            for &q in second {
                if cells[p].order > cells[q].order {
                    total += 1;
                }
            }
        }
    }
    total
}

/// Swap adjacent pairs while doing so removes crossings. This is the stage that
/// untangles a freshly opened fan: every child of one card has the same median,
/// so a sweep has no opinion about them, and only a swap test against the next
/// column does.
fn transpose(columns: &mut [Vec<usize>], cells: &mut [Cell]) {
    let mut improved = true;
    let mut guard = 0;
    while improved && guard < 4 {
        improved = false;
        guard += 1;
        for column in columns.iter_mut() {
            for slot in 0..column.len().saturating_sub(1) {
                let (a, b) = (column[slot], column[slot + 1]);
                if pair_crossings(b, a, cells) < pair_crossings(a, b, cells) {
                    column.swap(slot, slot + 1);
                    cells[a].order = (slot + 1) as f32;
                    cells[b].order = slot as f32;
                    improved = true;
                }
            }
        }
    }
}

/// Every crossing in the drawing, for choosing between orderings.
///
/// Counted by inversions rather than by comparing every pair of edges with
/// every other: sort the edges leaving a column by where they start and where
/// they land, then sweep them into a Fenwick tree and ask, for each, how many
/// already-placed edges land below it. That is Barth, Jünger and Mutzel's
/// method, and it is `O(E log E)` where the pairwise version is `O(E²)`.
///
/// The pairwise version was 270ms per re-tidy on a dependency-heavy workspace,
/// in a column holding a few hundred wire lanes. This is the same number,
/// arrived at without the quadratic.
fn crossings(columns: &[Vec<usize>], cells: &[Cell]) -> usize {
    let mut total = 0;
    let mut edges: Vec<(u32, u32)> = Vec::new();
    let mut tree: Vec<u32> = Vec::new();

    for column in columns {
        edges.clear();
        for &a in column {
            for &p in &cells[a].right {
                edges.push((cells[a].order as u32, cells[p].order as u32));
            }
        }
        if edges.len() < 2 {
            continue;
        }
        edges.sort_unstable();

        let width = edges.iter().map(|edge| edge.1).max().unwrap_or(0) as usize + 1;
        tree.clear();
        tree.resize(width + 1, 0);

        // Sweep in source order; for each edge, everything already in the tree
        // that lands strictly below it is a crossing.
        for (placed, &(_, target)) in edges.iter().enumerate() {
            // Count entries with position <= target.
            let mut at_or_above = 0usize;
            let mut index = target as usize + 1;
            while index > 0 {
                at_or_above += tree[index] as usize;
                index -= index & index.wrapping_neg();
            }
            total += placed - at_or_above;

            let mut index = target as usize + 1;
            while index <= width {
                tree[index] += 1;
                index += index & index.wrapping_neg();
            }
        }
    }
    total
}

/// How far the cell in slot `k` can move before it would crowd something that
/// outranks it. Everything of lower rank in the way simply gives ground.
fn room(column: &[usize], cells: &[Cell], seps: &[f32], k: usize, down: bool) -> f32 {
    let rank = priority(&cells[column[k]]);
    let mut needed = 0.0;
    if down {
        for j in (k + 1)..column.len() {
            needed += seps[j];
            if priority(&cells[column[j]]) >= rank {
                return (cells[column[j]].y - needed - cells[column[k]].y).max(0.0);
            }
        }
    } else {
        for j in (0..k).rev() {
            needed += seps[j + 1];
            if priority(&cells[column[j]]) >= rank {
                return (cells[column[k]].y - (cells[column[j]].y + needed)).max(0.0);
            }
        }
    }
    f32::INFINITY
}

/// Move the cell in slot `k`, pushing whatever it runs into just far enough.
fn shove(column: &[usize], cells: &mut [Cell], seps: &[f32], k: usize, delta: f32) {
    cells[column[k]].y += delta;
    if delta > 0.0 {
        for j in (k + 1)..column.len() {
            let floor = cells[column[j - 1]].y + seps[j];
            if cells[column[j]].y < floor {
                cells[column[j]].y = floor;
            } else {
                break;
            }
        }
    } else {
        for j in (0..k).rev() {
            let ceiling = cells[column[j + 1]].y - seps[j + 1];
            if cells[column[j]].y > ceiling {
                cells[column[j]].y = ceiling;
            } else {
                break;
            }
        }
    }
}

/// Slot order says who is above whom. This says where they actually go.
///
/// Each cell asks to be level with the median of what it is attached to, and
/// the column grants those requests in rank order — lanes first, then the
/// busiest cards — each one allowed to displace anything of lower rank in its
/// way and stopped by anything of higher rank. That priority is the difference
/// between a long wire drawn as a straight run through the columns it crosses
/// and the same wire drawn as a sweep across the whole drawing.
fn place(columns: &[Vec<usize>], cells: &mut [Cell], metrics: &Metrics) {
    // Separations depend only on what kind of thing sits where, and that never
    // changes once the ordering is settled.
    let seps: Vec<Vec<f32>> = columns
        .iter()
        .map(|column| {
            column
                .iter()
                .enumerate()
                .map(|(slot, &index)| {
                    if slot == 0 {
                        0.0
                    } else {
                        separation(&cells[column[slot - 1]], &cells[index], metrics)
                    }
                })
                .collect()
        })
        .collect();

    for (c, column) in columns.iter().enumerate() {
        let mut y = 0.0;
        for (slot, &index) in column.iter().enumerate() {
            y += seps[c][slot];
            cells[index].y = y;
        }
    }

    for pass in 0..RELAX {
        let rightward = pass % 2 == 0;
        let sweep: Vec<usize> = if rightward {
            (0..columns.len()).collect()
        } else {
            (0..columns.len()).rev().collect()
        };
        for c in sweep {
            let column = &columns[c];
            if column.is_empty() {
                continue;
            }
            // Where each cell would like to be: level with what it is attached
            // to. That is what straightens a wire and centres a card on its
            // children.
            let wanted: Vec<Option<f32>> = column
                .iter()
                .map(|&index| {
                    let neighbours = if rightward {
                        &cells[index].left
                    } else {
                        &cells[index].right
                    };
                    if neighbours.is_empty() {
                        return None;
                    }
                    let mut ys: Vec<f32> = neighbours.iter().map(|&next| cells[next].y).collect();
                    ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let middle = ys.len() / 2;
                    Some(if ys.len() % 2 == 1 {
                        ys[middle]
                    } else {
                        (ys[middle - 1] + ys[middle]) / 2.0
                    })
                })
                .collect();

            let mut by_rank: Vec<usize> = (0..column.len()).collect();
            by_rank.sort_by_key(|&slot| std::cmp::Reverse(priority(&cells[column[slot]])));

            for slot in by_rank {
                let Some(target) = wanted[slot] else {
                    continue;
                };
                let delta = target - cells[column[slot]].y;
                if delta.abs() < 0.01 {
                    continue;
                }
                let available = room(column, cells, &seps[c], slot, delta > 0.0);
                let step = if delta > 0.0 {
                    delta.min(available)
                } else {
                    delta.max(-available)
                };
                if step.abs() > 0.01 {
                    shove(column, cells, &seps[c], slot, step);
                }
            }
        }
    }

    // Centre the whole drawing on the axis, rather than each column on its own —
    // centring per column is what stops a parent ever lining up with its only
    // child.
    let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
    for cell in cells.iter() {
        low = low.min(cell.y - cell.half(metrics));
        high = high.max(cell.y + cell.half(metrics));
    }
    if low.is_finite() {
        let shift = -(low + high) / 2.0;
        for cell in cells.iter_mut() {
            cell.y += shift;
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn slots(spec: &[(usize, i32)]) -> Vec<Slot> {
        spec.iter()
            .map(|&(id, column)| Slot { id, column })
            .collect()
    }

    fn metrics() -> Metrics {
        Metrics::default()
    }

    fn draw(
        spec: &[(usize, i32)],
        edges: &[(usize, usize)],
        previous: &HashMap<usize, f32>,
    ) -> Layout {
        layered(&slots(spec), edges, previous, &metrics())
    }

    fn place_them(
        spec: &[(usize, i32)],
        edges: &[(usize, usize)],
        previous: &HashMap<usize, f32>,
    ) -> HashMap<usize, (f32, f32)> {
        draw(spec, edges, previous)
            .places
            .into_iter()
            .map(|p| (p.id, (p.along, p.across)))
            .collect()
    }

    /// Crossings between the cards themselves, counted from the finished
    /// drawing rather than from the algorithm's own bookkeeping.
    fn drawn_crossings(placed: &HashMap<usize, (f32, f32)>, edges: &[(usize, usize)]) -> usize {
        let mut total = 0;
        for (i, a) in edges.iter().enumerate() {
            for b in &edges[i + 1..] {
                let (a0, a1) = (placed[&a.0], placed[&a.1]);
                let (b0, b1) = (placed[&b.0], placed[&b.1]);
                // Only edges spanning the same pair of columns can be compared
                // this simply, which is all this fixture needs.
                if a0.0 == b0.0 && a1.0 == b1.0 && a.0 != b.0 && a.1 != b.1 {
                    let above = (a0.1 - b0.1).signum();
                    let arrives = (a1.1 - b1.1).signum();
                    if above != arrives {
                        total += 1;
                    }
                }
            }
        }
        total
    }

    /// The law of the graph: everything a node points at is strictly to its
    /// right. Every other affordance is downstream of this.
    #[test]
    fn every_edge_runs_left_to_right() {
        let spec = [(0, 0), (1, 1), (2, 1), (3, 4), (4, 9)];
        let edges = [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)];
        let placed = place_them(&spec, &edges, &HashMap::new());
        for (from, to) in edges {
            assert!(
                placed[&from].0 < placed[&to].0,
                "{from} -> {to} runs backwards"
            );
        }
    }

    #[test]
    fn empty_columns_are_compacted_away() {
        let placed = place_them(&[(0, 0), (1, 4), (2, 9)], &[(0, 1), (1, 2)], &HashMap::new());
        assert_eq!(placed[&0].0, 0.0);
        assert_eq!(placed[&1].0, metrics().pitch);
        assert_eq!(placed[&2].0, metrics().pitch * 2.0);
    }

    #[test]
    fn cards_in_a_column_never_overlap() {
        let spec: Vec<(usize, i32)> = (0..12).map(|id| (id, (id % 3) as i32)).collect();
        let placed = place_them(&spec, &[], &HashMap::new());
        let mut by_column: HashMap<i32, Vec<f32>> = HashMap::new();
        for (id, column) in &spec {
            by_column.entry(*column).or_default().push(placed[id].1);
        }
        for (column, mut ys) in by_column {
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in ys.windows(2) {
                assert!(
                    pair[1] - pair[0] >= metrics().across,
                    "two cards in column {column} are {:.1} apart, closer than a card is tall",
                    pair[1] - pair[0]
                );
            }
        }
    }

    /// The same graph draws the same picture every time. A layout that settles
    /// differently per run is one nobody can learn.
    #[test]
    fn the_same_graph_places_identically_twice() {
        let spec = [(7, 0), (3, 1), (9, 1), (1, 2)];
        let edges = [(7, 3), (7, 9), (3, 1), (9, 1)];
        assert_eq!(
            place_them(&spec, &edges, &HashMap::new()),
            place_them(&spec, &edges, &HashMap::new())
        );
    }

    /// A chain comes out straight. This is the coordinate stage's whole job:
    /// slot order alone would stack each column from the top and leave a chain
    /// looking like a staircase.
    #[test]
    fn a_chain_is_drawn_straight() {
        let spec = [(0, 0), (1, 1), (2, 2), (3, 3)];
        let edges = [(0, 1), (1, 2), (2, 3)];
        let placed = place_them(&spec, &edges, &HashMap::new());
        for id in 1..4 {
            assert!(
                (placed[&id].1 - placed[&0].1).abs() < 0.5,
                "the chain bends at {id}: {:.1} against {:.1}",
                placed[&id].1,
                placed[&0].1
            );
        }
    }

    /// A card opened onto its children sits level with the middle of them,
    /// which is what makes a freshly expanded fan read as a fan.
    #[test]
    fn a_parent_sits_level_with_its_children() {
        let spec = [(0, 0), (1, 1), (2, 1), (3, 1), (4, 1), (5, 1)];
        let edges = [(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)];
        let placed = place_them(&spec, &edges, &HashMap::new());
        let mut children: Vec<f32> = (1..6).map(|id| placed[&id].1).collect();
        children.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let middle = children[children.len() / 2];
        assert!(
            (placed[&0].1 - middle).abs() < 0.5,
            "the parent sits at {:.1}, the middle child at {middle:.1}",
            placed[&0].1
        );
    }

    /// The transpose stage earns its place: this ordering cannot be fixed by
    /// medians alone, and the drawing has to come out with no crossings at all.
    #[test]
    fn crossings_are_removed_not_merely_reduced() {
        let spec = [(0, 0), (1, 0), (2, 1), (3, 1)];
        let edges = [(0, 3), (1, 2)];
        let placed = place_them(&spec, &edges, &HashMap::new());
        assert_eq!(drawn_crossings(&placed, &edges), 0);
    }

    /// A wider tangle, where the seed order is deliberately the worst one.
    #[test]
    fn a_tangle_is_untangled() {
        let spec = [(0, 0), (1, 0), (2, 0), (3, 1), (4, 1), (5, 1)];
        let edges = [(0, 5), (1, 4), (2, 3)];
        let previous = HashMap::from([(0, 0.0), (1, 100.0), (2, 200.0)]);
        let placed = place_them(&spec, &edges, &previous);
        assert_eq!(drawn_crossings(&placed, &edges), 0);
    }

    /// Seeding from the previous frame must not cost the drawing its quality:
    /// re-tidying after an expand is as good as laying the same graph out cold.
    #[test]
    fn re_tidying_is_as_good_as_starting_over() {
        let spec = [(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)];
        let edges = [(0, 3), (1, 2), (2, 5), (3, 4)];
        let stale = HashMap::from([(0, 0.0), (1, 80.0), (2, 0.0), (3, 80.0)]);
        let warm = place_them(&spec, &edges, &stale);
        let cold = place_them(&spec, &edges, &HashMap::new());
        assert!(
            drawn_crossings(&warm, &edges) <= drawn_crossings(&cold, &edges),
            "seeding from the last frame left {} crossings against {} cold",
            drawn_crossings(&warm, &edges),
            drawn_crossings(&cold, &edges)
        );
    }

    /// A wire that spans more than one column is given a waypoint in each one
    /// it crosses, so the columns can open a lane rather than letting it cut
    /// across whatever card is in the way.
    #[test]
    fn a_long_wire_runs_through_a_lane_in_every_column_it_crosses() {
        // Two cards in each crossed column, so the wire has to bend around
        // them and cannot be simplified back to a straight run.
        let spec = [(0, 0), (10, 1), (11, 1), (20, 2), (21, 2), (3, 3)];
        let edges = [(0, 3), (0, 10), (0, 11), (10, 20), (11, 21), (20, 3), (21, 3)];
        let out = draw(&spec, &edges, &HashMap::new());
        let long = out
            .wires
            .iter()
            .find(|wire| wire.from == 0 && wire.to == 3)
            .expect("the long wire is routed");
        assert!(
            long.points.len() > 2,
            "a wire that has to bend around two columns of cards kept no waypoints"
        );
        let xs: Vec<f32> = long.points.iter().map(|point| point.0).collect();
        assert!(
            xs.windows(2).all(|pair| pair[1] > pair[0]),
            "the wire doubles back on itself: {xs:?}"
        );

        let short = out
            .wires
            .iter()
            .find(|wire| wire.from == 0 && wire.to == 10)
            .expect("the short wire is routed");
        assert_eq!(short.points.len(), 2, "a one-column hop needs no waypoint");
    }

    /// And the lane actually clears the cards it passes: a wire crossing a
    /// column is not drawn through the middle of a card sitting there.
    #[test]
    fn a_lane_clears_the_cards_it_passes() {
        // 0 -> 3 spans the column that holds 1 and 2.
        let spec = [(0, 0), (1, 1), (2, 1), (3, 2)];
        let edges = [(0, 3), (0, 1), (0, 2)];
        let out = draw(&spec, &edges, &HashMap::new());
        let places: HashMap<usize, (f32, f32)> = out
            .places
            .iter()
            .map(|place| (place.id, (place.along, place.across)))
            .collect();
        let routed = out
            .wires
            .iter()
            .find(|wire| wire.from == 0 && wire.to == 3)
            .expect("routed");
        // Whether the lane survived simplification or not, no part of the run
        // may pass through a card in the column it crosses.
        let lane = routed.points[routed.points.len() / 2];
        for card in [1usize, 2] {
            let (_, top) = places[&card];
            let clear = lane.1 <= top || lane.1 >= top + metrics().across;
            assert!(
                clear,
                "the lane at {:.1} runs through card {card} at {top:.1}..{:.1}",
                lane.1,
                top + metrics().across
            );
        }
    }

    /// And the lane runs *straight*. A wire crossing four columns of busy cards
    /// comes out as one level run rather than a sweep across the drawing, which
    /// is what the rank order in the coordinate stage buys: a lane outranks
    /// every card, so the cards give way to it instead of the other way round.
    #[test]
    fn a_long_wire_comes_out_straight() {
        // One wire from column 0 to column 4, through three columns that each
        // hold four cards of their own.
        let mut spec = vec![(0usize, 0i32), (1, 4)];
        let mut edges = vec![(0usize, 1usize)];
        let mut id = 100;
        for column in 1..4 {
            for _ in 0..4 {
                spec.push((id, column));
                edges.push((0, id));
                id += 1;
            }
        }
        let out = draw(&spec, &edges, &HashMap::new());
        let long = out
            .wires
            .iter()
            .find(|wire| wire.from == 0 && wire.to == 1)
            .expect("the long wire is routed");
        // Straightness is now *observable*: a wire whose lanes all landed on one
        // line keeps none of them, because the drawing does not carry the
        // scaffolding that got it there. It crossed three columns and may keep
        // at most one waypoint — the step off its source, which is centred on
        // all thirteen of its own wires rather than on this one.
        assert!(
            long.points.len() <= 3,
            "the wire wandered across the columns it crosses: {:?}",
            long.points
        );
        // And from that step onward it runs level.
        let run = &long.points[long.points.len() - 2..];
        assert!(
            (run[0].1 - run[1].1).abs() < 1.0,
            "the wire is not level: {run:?}"
        );
    }

    /// A dependency graph the size of a real, dependency-heavy Rust workspace:
    /// 700 nodes over 25 columns with 2650 edges, most of them spanning several
    /// columns. This is the shape that made folding a port take two and a half
    /// seconds before the crossing count was rewritten.
    #[test]
    fn report_cost_on_a_heavy_graph() {
        let mut spec: Vec<(usize, i32)> = Vec::new();
        let mut edges: Vec<(usize, usize)> = Vec::new();
        let columns = 25usize;
        let per = 28usize;
        for column in 0..columns {
            for row in 0..per {
                spec.push((column * per + row, column as i32));
            }
        }
        // Every node reaches forward a few columns, which is what makes lanes.
        for column in 0..columns {
            for row in 0..per {
                let from = column * per + row;
                for (step, skip) in [(1usize, 1usize), (1, 7), (3, 3), (6, 11)] {
                    let target_column = column + step;
                    if target_column < columns {
                        edges.push((from, target_column * per + (row + skip) % per));
                    }
                }
            }
        }
        let started = std::time::Instant::now();
        let out = layered(&slots(&spec), &edges, &HashMap::new(), &metrics());
        let took = started.elapsed();
        let lanes: usize = out.wires.iter().map(|wire| wire.points.len() - 2).sum();
        println!(
            "heavy: {} nodes, {} wires, {lanes} lanes in {took:?}",
            out.places.len(),
            out.wires.len()
        );
        assert!(
            took.as_millis() < 900,
            "the layout took {took:?} on a heavy graph, which is a frozen tab"
        );
    }

    /// What the pipeline costs at the size a real graph reaches. Not an
    /// assertion about wall-clock — that belongs to a benchmark — but a guard
    /// that the shape of the work stays linear-ish in the graph rather than
    /// quadratic in it.
    #[test]
    fn report_cost_at_scale() {
        // A wide, deep, densely connected DAG: 400 nodes over 20 columns, every
        // node wired to a few in the next two columns.
        let mut spec: Vec<(usize, i32)> = Vec::new();
        let mut edges: Vec<(usize, usize)> = Vec::new();
        let per = 20usize;
        for column in 0..20i32 {
            for row in 0..per {
                spec.push((column as usize * per + row, column));
            }
        }
        for column in 0..18i32 {
            for row in 0..per {
                let from = column as usize * per + row;
                for step in 1..=3 {
                    let to = (column as usize + 1) * per + (row + step) % per;
                    edges.push((from, to));
                }
                edges.push((from, (column as usize + 2) * per + row % per));
            }
        }

        let started = std::time::Instant::now();
        let out = layered(&slots(&spec), &edges, &HashMap::new(), &metrics());
        let cold = started.elapsed();
        let lanes: usize = out.wires.iter().map(|wire| wire.points.len() - 2).sum();
        println!(
            "{} nodes, {} wires, {lanes} lanes in {cold:?}",
            out.places.len(),
            out.wires.len()
        );
        assert_eq!(out.places.len(), spec.len());
        assert!(
            cold.as_millis() < 4000,
            "the layout took {cold:?}, which is past anything a reader would wait for"
        );
    }

    #[test]
    fn an_empty_graph_places_nothing() {
        let out = layered(&[], &[], &HashMap::new(), &metrics());
        assert!(out.places.is_empty() && out.wires.is_empty());
    }

    /// The layout measures in whatever the caller hands it. A different node
    /// size is a different drawing, not a differently-scaled one.
    #[test]
    fn the_metrics_are_what_the_layout_measures_in() {
        let spec = [(0usize, 0i32), (1, 1), (2, 1)];
        let edges = [(0usize, 1usize), (0, 2)];
        let wide = Metrics {
            along: 400.0,
            across: 100.0,
            pitch: 600.0,
            ..Metrics::default()
        };
        let out = layered(&slots(&spec), &edges, &HashMap::new(), &wide);
        let at: HashMap<usize, (f32, f32)> = out
            .places
            .iter()
            .map(|p| (p.id, (p.along, p.across)))
            .collect();
        assert_eq!(at[&1].0, 600.0, "the column pitch is the caller's");
        assert!(
            (at[&1].1 - at[&2].1).abs() >= 100.0,
            "two nodes are at least a node apart"
        );
        // And the wire leaves the far edge of a node that is now 400 wide.
        assert_eq!(out.wires[0].points[0].0, 400.0);
    }
}
