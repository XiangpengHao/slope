//! A point quadtree over station positions.
//!
//! Canvas draws pixels, not elements, so there are no DOM nodes left to hit-test
//! against — the map has to answer "what is under the pointer" itself. Station
//! density here is extremely uneven (hundreds of crates crowd the low ranks,
//! a handful trail off to the right), which is exactly the case a uniform grid
//! handles badly and a quadtree handles well.

#[derive(Clone, Copy, Debug)]
struct Bounds {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Bounds {
    /// Closest possible distance from a point to anything inside this box. Used
    /// to prune whole branches during a nearest-neighbour search.
    fn distance_to(&self, x: f32, y: f32) -> f32 {
        let dx = (self.x0 - x).max(0.0).max(x - self.x1);
        let dy = (self.y0 - y).max(0.0).max(y - self.y1);
        (dx * dx + dy * dy).sqrt()
    }

    fn quadrant(&self, i: usize) -> Bounds {
        let (mx, my) = ((self.x0 + self.x1) / 2.0, (self.y0 + self.y1) / 2.0);
        match i {
            0 => Bounds { x0: self.x0, y0: self.y0, x1: mx, y1: my },
            1 => Bounds { x0: mx, y0: self.y0, x1: self.x1, y1: my },
            2 => Bounds { x0: self.x0, y0: my, x1: mx, y1: self.y1 },
            _ => Bounds { x0: mx, y0: my, x1: self.x1, y1: self.y1 },
        }
    }
}

enum Node {
    Leaf(Vec<(usize, f32, f32)>),
    Split(Box<[Node; 4]>),
}

pub struct QuadTree {
    root: Node,
    bounds: Bounds,
}

/// Points per leaf before it splits, and how deep the tree may go. The depth cap
/// matters: several crates can share a position closely enough that splitting
/// would never separate them.
const LEAF_CAPACITY: usize = 8;
const MAX_DEPTH: usize = 12;

impl QuadTree {
    pub fn build(points: &[(usize, f32, f32)]) -> Self {
        let mut bounds = Bounds {
            x0: f32::INFINITY,
            y0: f32::INFINITY,
            x1: f32::NEG_INFINITY,
            y1: f32::NEG_INFINITY,
        };
        for (_, x, y) in points {
            bounds.x0 = bounds.x0.min(*x);
            bounds.y0 = bounds.y0.min(*y);
            bounds.x1 = bounds.x1.max(*x);
            bounds.y1 = bounds.y1.max(*y);
        }
        if !bounds.x0.is_finite() {
            bounds = Bounds { x0: 0.0, y0: 0.0, x1: 1.0, y1: 1.0 };
        }
        // Pad so points exactly on the edge still land inside.
        bounds.x1 += 1.0;
        bounds.y1 += 1.0;

        let mut root = Node::Leaf(Vec::new());
        for &p in points {
            insert(&mut root, bounds, p, 0);
        }
        Self { root, bounds }
    }

    /// The nearest point within `radius`, or nothing.
    pub fn nearest(&self, x: f32, y: f32, radius: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        search(&self.root, self.bounds, x, y, radius, &mut best);
        best.map(|(id, _)| id)
    }
}

fn insert(node: &mut Node, bounds: Bounds, point: (usize, f32, f32), depth: usize) {
    match node {
        Node::Leaf(items) => {
            items.push(point);
            if items.len() > LEAF_CAPACITY && depth < MAX_DEPTH {
                let taken = std::mem::take(items);
                let mut children = Box::new([
                    Node::Leaf(Vec::new()),
                    Node::Leaf(Vec::new()),
                    Node::Leaf(Vec::new()),
                    Node::Leaf(Vec::new()),
                ]);
                for p in taken {
                    let q = quadrant_of(bounds, p.1, p.2);
                    insert(&mut children[q], bounds.quadrant(q), p, depth + 1);
                }
                *node = Node::Split(children);
            }
        }
        Node::Split(children) => {
            let q = quadrant_of(bounds, point.1, point.2);
            insert(&mut children[q], bounds.quadrant(q), point, depth + 1);
        }
    }
}

fn quadrant_of(bounds: Bounds, x: f32, y: f32) -> usize {
    let (mx, my) = ((bounds.x0 + bounds.x1) / 2.0, (bounds.y0 + bounds.y1) / 2.0);
    usize::from(x >= mx) + if y >= my { 2 } else { 0 }
}

fn search(node: &Node, bounds: Bounds, x: f32, y: f32, radius: f32, best: &mut Option<(usize, f32)>) {
    let ceiling = best.map(|(_, d)| d).unwrap_or(radius);
    if bounds.distance_to(x, y) > ceiling {
        return;
    }
    match node {
        Node::Leaf(items) => {
            for &(id, px, py) in items {
                let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
                if d <= radius && best.is_none_or(|(_, bd)| d < bd) {
                    *best = Some((id, d));
                }
            }
        }
        Node::Split(children) => {
            // Descend the quadrant the point is in first, so the pruning
            // ceiling is as tight as possible for the siblings.
            let first = quadrant_of(bounds, x, y);
            search(&children[first], bounds.quadrant(first), x, y, radius, best);
            for q in 0..4 {
                if q != first {
                    search(&children[q], bounds.quadrant(q), x, y, radius, best);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(points: &[(usize, f32, f32)], x: f32, y: f32, radius: f32) -> Option<usize> {
        points
            .iter()
            .map(|&(id, px, py)| (id, ((px - x).powi(2) + (py - y).powi(2)).sqrt()))
            .filter(|&(_, d)| d <= radius)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }

    /// The tree must answer exactly what a linear scan would, including the
    /// clustered case it exists to handle.
    #[test]
    fn matches_a_linear_scan() {
        let mut points = Vec::new();
        // A dense cluster and a sparse tail, the shape a real dependency map has.
        for i in 0..400 {
            let t = i as f32;
            points.push((i, 100.0 + (t * 7.0) % 90.0, 100.0 + (t * 13.0) % 90.0));
        }
        for i in 400..440 {
            let t = (i - 400) as f32;
            points.push((i, 2000.0 + t * 300.0, 500.0 + t * 40.0));
        }
        let tree = QuadTree::build(&points);

        for probe in 0..200 {
            let t = probe as f32;
            let x = (t * 61.0) % 4000.0;
            let y = (t * 37.0) % 2000.0;
            for radius in [5.0f32, 40.0, 500.0] {
                let expected = brute(&points, x, y, radius);
                let got = tree.nearest(x, y, radius);
                match (expected, got) {
                    (None, None) => {}
                    (Some(e), Some(g)) => {
                        // Ties are allowed to differ, equal distance is equal.
                        let de = ((points[e].1 - x).powi(2) + (points[e].2 - y).powi(2)).sqrt();
                        let dg = ((points[g].1 - x).powi(2) + (points[g].2 - y).powi(2)).sqrt();
                        assert!(
                            (de - dg).abs() < 0.001,
                            "probe ({x},{y}) r={radius}: tree gave {g} at {dg}, scan gave {e} at {de}"
                        );
                    }
                    (e, g) => panic!("probe ({x},{y}) r={radius}: scan {e:?}, tree {g:?}"),
                }
            }
        }
    }

    #[test]
    fn empty_tree_finds_nothing() {
        let tree = QuadTree::build(&[]);
        assert_eq!(tree.nearest(0.0, 0.0, 100.0), None);
    }

    #[test]
    fn coincident_points_do_not_blow_the_stack() {
        let points: Vec<_> = (0..500).map(|i| (i, 42.0, 42.0)).collect();
        let tree = QuadTree::build(&points);
        assert!(tree.nearest(42.0, 42.0, 1.0).is_some());
    }
}
