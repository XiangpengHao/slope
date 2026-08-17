//! Free placement: position from attraction, not from a column.
//!
//! [`layered`](crate::layered) draws a graph whose x axis is a law. This draws
//! the other kind, where no axis means anything and a node's place is only ever
//! "near the things it is joined to". It exists for the reading where wires are
//! hidden until a node is selected: with nothing drawn at rest, a column buys
//! nothing, and the only thing position still has to do is put a node's
//! neighbours where the reader can see them the moment they light up.
//!
//! Measured on a 718-crate workspace, that is the whole difference. Selecting a
//! card and asking how far you must zoom out to see everything it is joined to:
//! a column layout answers 3.3x at the median and keeps 47% of selections
//! readable, and this answers 1.3x and 97%. Total wire is less than half. The
//! column layout is not badly tuned — depth pins x, so a neighbour four hops
//! along is four columns away however well the rest is solved.
//!
//! The model is the standard spring-electrical one: every node pushes every
//! other apart, every edge pulls its ends together, and the whole thing cools.
//! Three things here are not standard, and each is load-bearing:
//!
//! 1. **It is deterministic.** The usual random start draws a different picture
//!    every run, which is intolerable for a tool you reopen. Nodes start from
//!    their graph depth and a golden-ratio scatter, so the same workspace always
//!    lands the same way.
//! 2. **Cards are wide.** A 190x48 card is not the disc the force model assumes.
//!    All of it runs in a space where y is stretched by the card's aspect, which
//!    makes the card round again; undoing the stretch at the end leaves cards
//!    spaced further apart across than down, which is what wide cards need.
//! 3. **Repulsion is bucketed.** Every-pair repulsion is O(n^2) per round and at
//!    718 cards that was 5.3 seconds. Cells far enough away are treated as one
//!    lump at their centre of mass, which brings the same drawing in 0.4s.

/// What the layout measures in, for a graph with no columns.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Air {
    /// A node's width in world units.
    pub width: f32,
    /// A node's height in world units.
    pub height: f32,
    /// Ideal distance between two joined nodes, as a multiple of node width.
    ///
    /// This is the one dial worth turning. Small values pack tighter and make
    /// the drawing a brick wall; large values open it out and make a selection's
    /// wires longer. 4.5 measured best on a real workspace: it keeps 97% of
    /// neighbourhoods readable without zooming, where 1.15 keeps 87%.
    pub spread: f32,
    /// Least air left between two cards once the springs have settled.
    pub gap: (f32, f32),
}

impl Default for Air {
    fn default() -> Self {
        Self {
            width: 190.0,
            height: 48.0,
            spread: 4.5,
            gap: (16.0, 14.0),
        }
    }
}

/// Where a node landed: the leading corner of its box.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Spot {
    pub id: usize,
    pub x: f32,
    pub y: f32,
}

/// Place `nodes` by attraction alone.
///
/// `depth` is each node's rank in the graph, used only to seed the solver — the
/// result is not held to it. Pass whatever the host has; [`rank`](crate::rank)
/// computes one for a graph that has no natural order.
///
/// `settled` is where each node sat in the last frame. A node that has one keeps
/// it as its start, so adding a node to a drawn graph nudges the picture rather
/// than redrawing it.
pub fn place(
    nodes: &[usize],
    edges: &[(usize, usize)],
    depth: &std::collections::HashMap<usize, i32>,
    settled: &std::collections::HashMap<usize, (f32, f32)>,
    air: &Air,
) -> Vec<Spot> {
    let n = nodes.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![Spot {
            id: nodes[0],
            x: 0.0,
            y: 0.0,
        }];
    }

    let index: std::collections::HashMap<usize, usize> =
        nodes.iter().enumerate().map(|(at, &id)| (id, at)).collect();
    let mut pairs: Vec<(usize, usize)> = Vec::with_capacity(edges.len());
    for &(from, to) in edges {
        let (Some(&a), Some(&b)) = (index.get(&from), index.get(&to)) else {
            continue;
        };
        if a != b {
            pairs.push((a, b));
        }
    }

    let aspect = air.width / air.height;
    let k = air.width * air.spread;
    let reach = (n as f32).sqrt();

    // --- Start. A node the reader was already looking at keeps its place;
    // anything new is seeded from its depth so the solver begins from something
    // already roughly untangled rather than from noise.
    let deepest = nodes
        .iter()
        .filter_map(|id| depth.get(id))
        .copied()
        .max()
        .unwrap_or(0)
        .max(1) as f32;
    let mut x = vec![0.0f32; n];
    let mut y = vec![0.0f32; n];
    for (at, id) in nodes.iter().enumerate() {
        if let Some(&(was_x, was_y)) = settled.get(id) {
            x[at] = was_x;
            y[at] = was_y * aspect;
            continue;
        }
        let along = depth.get(id).copied().unwrap_or(0) as f32 / deepest;
        // Golden-ratio scatter: even spread with no clumps and no randomness.
        let across = ((at as f32 * 0.618_034) % 1.0) - 0.5;
        x[at] = along * k * reach * 0.5;
        y[at] = across * k * reach;
    }

    // Busy nodes resist the pull to the middle, so a hub is not dragged into the
    // centre of its own fan.
    let mut degree = vec![0.0f32; n];
    for &(a, b) in &pairs {
        degree[a] += 1.0;
        degree[b] += 1.0;
    }

    // How much of this graph is new. Re-solving a drawing the reader is looking
    // at from a cold start would throw every card across the pane to arrive at
    // the same answer, so the solver is only given as much energy as there is
    // new work: all of it for a first draw, a nudge for one added card.
    let fresh = nodes.iter().filter(|id| !settled.contains_key(id)).count();
    let churn = if settled.is_empty() {
        1.0
    } else {
        (fresh as f32 / n as f32).clamp(0.03, 1.0)
    };

    // Enough rounds to settle, capped so a big graph does not stall the tab.
    // Quality plateaus well before this on every graph measured.
    let full = (12_000 / (n as u32).max(1)).clamp(90, 400);
    let rounds = ((full as f32 * churn.sqrt()).round() as u32).max(12);
    let mut heat = k * 4.0 * churn;
    let cool = (0.02f32 / 4.0).powf(1.0 / rounds as f32);

    let mut fx = vec![0.0f32; n];
    let mut fy = vec![0.0f32; n];
    let mut grid = Grid::default();

    for _ in 0..rounds {
        fx.iter_mut().for_each(|f| *f = 0.0);
        fy.iter_mut().for_each(|f| *f = 0.0);

        grid.fill(&x, &y, k * 2.0);
        for i in 0..n {
            grid.repel(i, &x, &y, k, &mut fx, &mut fy);
        }

        for &(a, b) in &pairs {
            let (dx, dy) = (x[a] - x[b], y[a] - y[b]);
            let d = (dx * dx + dy * dy).sqrt().max(0.01);
            let pull = d * d / k;
            let (ux, uy) = (dx / d * pull, dy / d * pull);
            fx[a] -= ux;
            fy[a] -= uy;
            fx[b] += ux;
            fy[b] += uy;
        }

        // A weak pull to the origin, or an island with nothing attached sails
        // off and takes the whole bounding box with it.
        for i in 0..n {
            let home = 0.012 / (1.0 + degree[i].sqrt());
            fx[i] -= x[i] * home;
            fy[i] -= y[i] * home;
        }

        for i in 0..n {
            let d = (fx[i] * fx[i] + fy[i] * fy[i]).sqrt().max(0.01);
            let step = d.min(heat);
            x[i] += fx[i] / d * step;
            y[i] += fy[i] / d * step;
        }
        heat *= cool;
    }

    for value in y.iter_mut() {
        *value /= aspect;
    }
    separate(&mut x, &mut y, air);

    // Centre on the middle of the drawing, not on its top-left corner. This is
    // the frame the solver itself works in — gravity pulls towards the origin —
    // so a drawing handed back in any other frame would be dragged sideways the
    // next time it was used as a seed.
    let (mut low_x, mut high_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut low_y, mut high_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for i in 0..n {
        low_x = low_x.min(x[i]);
        high_x = high_x.max(x[i]);
        low_y = low_y.min(y[i]);
        high_y = high_y.max(y[i]);
    }
    let (mid_x, mid_y) = ((low_x + high_x) / 2.0, (low_y + high_y) / 2.0);
    nodes
        .iter()
        .enumerate()
        .map(|(at, &id)| Spot {
            id,
            x: x[at] - mid_x,
            y: y[at] - mid_y,
        })
        .collect()
}

/// Nodes bucketed by cell, so repulsion can treat a distant cell as one lump.
#[derive(Default)]
struct Grid {
    cell: f32,
    /// Cell key, centre of mass, how many, and where its members start in `by_cell`.
    lumps: Vec<(i32, i32, f32, f32, f32, usize, usize)>,
    by_cell: Vec<usize>,
}

impl Grid {
    fn fill(&mut self, x: &[f32], y: &[f32], cell: f32) {
        self.cell = cell.max(1.0);
        self.lumps.clear();
        self.by_cell.clear();

        let mut keyed: Vec<(i32, i32, usize)> = (0..x.len())
            .map(|i| {
                (
                    (x[i] / self.cell).floor() as i32,
                    (y[i] / self.cell).floor() as i32,
                    i,
                )
            })
            .collect();
        keyed.sort_unstable();

        let mut at = 0;
        while at < keyed.len() {
            let (gx, gy, _) = keyed[at];
            let start = self.by_cell.len();
            let (mut sx, mut sy, mut mass) = (0.0f32, 0.0f32, 0.0f32);
            while at < keyed.len() && keyed[at].0 == gx && keyed[at].1 == gy {
                let i = keyed[at].2;
                self.by_cell.push(i);
                sx += x[i];
                sy += y[i];
                mass += 1.0;
                at += 1;
            }
            self.lumps
                .push((gx, gy, sx / mass, sy / mass, mass, start, self.by_cell.len()));
        }
    }

    /// Push `i` away from everything else: exactly for the nine cells around it,
    /// and as one lump for every cell beyond them.
    fn repel(&self, i: usize, x: &[f32], y: &[f32], k: f32, fx: &mut [f32], fy: &mut [f32]) {
        let (cx, cy) = (
            (x[i] / self.cell).floor() as i32,
            (y[i] / self.cell).floor() as i32,
        );
        for &(gx, gy, mx, my, mass, start, end) in &self.lumps {
            if (gx - cx).abs() <= 1 && (gy - cy).abs() <= 1 {
                for &j in &self.by_cell[start..end] {
                    if i == j {
                        continue;
                    }
                    let (dx, dy) = (x[i] - x[j], y[i] - y[j]);
                    let d2 = (dx * dx + dy * dy).max(1.0);
                    let d = d2.sqrt();
                    let push = k * k / d2;
                    fx[i] += dx / d * push;
                    fy[i] += dy / d * push;
                }
            } else {
                let (dx, dy) = (x[i] - mx, y[i] - my);
                let d2 = (dx * dx + dy * dy).max(1.0);
                let d = d2.sqrt();
                let push = k * k * mass / d2;
                fx[i] += dx / d * push;
                fy[i] += dy / d * push;
            }
        }
    }
}

/// Nudge overlapping cards apart.
///
/// The springs settle centres, not boxes, so wide cards can still overlap when
/// the solver stops. Overlap is measured on each axis and resolved along
/// whichever needs the smaller move, scaled by the card's own proportions so a
/// wide card prefers to step sideways.
fn separate(x: &mut [f32], y: &mut [f32], air: &Air) {
    let n = x.len();
    let (pad_x, pad_y) = (air.width + air.gap.0, air.height + air.gap.1);
    let cell = pad_x.max(pad_y);
    let mut keyed: Vec<(i32, i32, usize)> = Vec::with_capacity(n);

    for _ in 0..60 {
        let mut moved = false;
        keyed.clear();
        keyed.extend(
            (0..n).map(|i| ((x[i] / cell).floor() as i32, (y[i] / cell).floor() as i32, i)),
        );
        keyed.sort_unstable();

        for at in 0..keyed.len() {
            let (gx, gy, i) = keyed[at];
            // Only the cells at or after this one, so each pair is tested once.
            for other in keyed[at + 1..].iter() {
                let (ox, oy, j) = *other;
                if ox > gx + 1 {
                    break;
                }
                if (oy - gy).abs() > 1 {
                    continue;
                }
                let (dx, dy) = (x[j] - x[i], y[j] - y[i]);
                let (over_x, over_y) = (pad_x - dx.abs(), pad_y - dy.abs());
                if over_x <= 0.0 || over_y <= 0.0 {
                    continue;
                }
                moved = true;
                if over_x / pad_x < over_y / pad_y {
                    let push = over_x / 2.0 * if dx < 0.0 { -1.0 } else { 1.0 };
                    x[i] -= push;
                    x[j] += push;
                } else {
                    let push = over_y / 2.0 * if dy < 0.0 { -1.0 } else { 1.0 };
                    y[i] -= push;
                    y[j] += push;
                }
            }
        }
        if !moved {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn run(nodes: &[usize], edges: &[(usize, usize)]) -> HashMap<usize, (f32, f32)> {
        let depth = crate::rank(nodes, edges);
        place(nodes, edges, &depth, &HashMap::new(), &Air::default())
            .into_iter()
            .map(|spot| (spot.id, (spot.x, spot.y)))
            .collect()
    }

    #[test]
    fn nothing_at_all_is_nothing() {
        assert!(run(&[], &[]).is_empty());
    }

    #[test]
    fn one_node_sits_at_the_origin() {
        let out = run(&[7], &[]);
        assert_eq!(out[&7], (0.0, 0.0));
    }

    /// The property the whole drawing rests on: no two cards overlap.
    #[test]
    fn no_two_cards_overlap() {
        let nodes: Vec<usize> = (0..120).collect();
        let edges: Vec<(usize, usize)> = (0..119)
            .flat_map(|i| [(i, i + 1), (i, (i * 7 + 3) % 120)])
            .filter(|(a, b)| a != b)
            .collect();
        let out = run(&nodes, &edges);
        let air = Air::default();
        for &a in &nodes {
            for &b in &nodes {
                if a >= b {
                    continue;
                }
                let (ax, ay) = out[&a];
                let (bx, by) = out[&b];
                assert!(
                    (ax - bx).abs() >= air.width - 0.5 || (ay - by).abs() >= air.height - 0.5,
                    "{a} and {b} overlap: {:?} vs {:?}",
                    (ax, ay),
                    (bx, by)
                );
            }
        }
    }

    /// Same graph in, same picture out — a tool you reopen must not redraw
    /// itself differently every time.
    #[test]
    fn the_same_graph_lands_the_same_way_twice() {
        let nodes: Vec<usize> = (0..60).collect();
        let edges: Vec<(usize, usize)> = (0..59).map(|i| (i, i + 1)).collect();
        assert_eq!(run(&nodes, &edges), run(&nodes, &edges));
    }

    /// What the layout is for: joined nodes end up near each other, and much
    /// nearer than unjoined ones.
    #[test]
    fn joined_nodes_end_up_closer_than_unjoined_ones() {
        let nodes: Vec<usize> = (0..80).collect();
        // Two clusters of forty, joined by a single edge.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for group in [0, 40] {
            for i in group..group + 39 {
                edges.push((i, i + 1));
                edges.push((i, group + (i * 5 + 1) % 40));
            }
        }
        edges.retain(|(a, b)| a != b);
        edges.push((0, 40));
        let out = run(&nodes, &edges);

        let span = |a: usize, b: usize| {
            let ((ax, ay), (bx, by)) = (out[&a], out[&b]);
            ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
        };
        let joined: f32 = edges.iter().map(|&(a, b)| span(a, b)).sum::<f32>() / edges.len() as f32;
        let all: f32 = nodes
            .iter()
            .flat_map(|&a| nodes.iter().map(move |&b| (a, b)))
            .filter(|(a, b)| a < b)
            .map(|(a, b)| span(a, b))
            .sum::<f32>()
            / (nodes.len() * (nodes.len() - 1) / 2) as f32;
        assert!(
            joined < all * 0.6,
            "joined pairs average {joined:.0} and all pairs {all:.0}; the springs did nothing"
        );
    }

    /// A node already on the pane starts from where it was, so adding one card
    /// does not redraw the world.
    #[test]
    fn a_settled_node_starts_from_where_it_was() {
        let nodes: Vec<usize> = (0..40).collect();
        let edges: Vec<(usize, usize)> = (0..39).map(|i| (i, i + 1)).collect();
        let depth = crate::rank(&nodes, &edges);
        let first: HashMap<usize, (f32, f32)> =
            place(&nodes, &edges, &depth, &HashMap::new(), &Air::default())
                .into_iter()
                .map(|spot| (spot.id, (spot.x, spot.y)))
                .collect();
        let again: HashMap<usize, (f32, f32)> =
            place(&nodes, &edges, &depth, &first, &Air::default())
                .into_iter()
                .map(|spot| (spot.id, (spot.x, spot.y)))
                .collect();
        let drift: f32 = nodes
            .iter()
            .map(|id| {
                let ((ax, ay), (bx, by)) = (first[id], again[id]);
                ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
            })
            .sum::<f32>()
            / nodes.len() as f32;
        assert!(
            drift < Air::default().width,
            "re-running a settled graph moved every card {drift:.0} on average"
        );
    }
}
