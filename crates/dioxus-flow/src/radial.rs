//! Radial placement: one node at the centre, and what it reaches fanned around
//! it, ring by ring.
//!
//! This is the arrangement for a graph that is *walked* rather than surveyed.
//! The reader names a starting node, sees it and what it points at, opens one of
//! those, and the drawing grows outward from where they were looking. Nothing on
//! the pane is there because an algorithm decided it was interesting; everything
//! on it was asked for, one click at a time.
//!
//! # Wedges
//!
//! Every node owns an angular wedge, and a branch's wedge is the share of the
//! circle its own size has earned — a branch holding half the drawn cards gets
//! half the circle. Wedges are re-shared each time the walk grows, which is why
//! the drawing reflows rather than only ever appending.
//!
//! The tempting alternative is to subdivide and never re-share: each node cuts
//! up the wedge it already holds, so a card that is drawn keeps its place for
//! good and opening something at the rim moves nothing. That is a better promise
//! and it does not survive a real graph. Every level divides again, so a node
//! four hops out owns a slice of a slice of a slice, and the radius at which a
//! card still fits across its wedge grows by the fan-out at *every* hop — three
//! opens into liquid-cache asked for a pane 133,000 units across. Capping the
//! radius and stepping the children outward instead only trades the blow-up for
//! a diagonal spike of cards marching off the corner.
//!
//! So the reflow is the price of a drawing that stays the size of what is on it.
//! What is kept instead is orientation: the centre stays at the centre, rings
//! stay in depth order, and siblings keep their order around the circle, so a
//! branch does not jump to the other side while the reader is looking at it.
//!
//! # Radius
//!
//! A ring sits at least `step` past the one inside it, and further out when it
//! has to: a wedge of `w` radians has to fit a card across it, so a ring cannot
//! be closer in than the radius at which the arc `r * w` is as wide as the card.
//! Because the wedges on a ring add up to the whole circle, that bound depends on
//! how many cards are on the ring and never on how deep the walk has gone.
//!
//! Cards are wider than they are tall, so how much room one needs across its
//! wedge depends on where on the circle it sits — a card due east turns its
//! 48-unit side to the arc, and the same card due north turns its 190-unit side.
//! That is measured per card rather than assumed, or every ring would be spaced
//! for the worst case and the drawing would be mostly air.

use std::collections::HashMap;

/// What a radial drawing measures in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ring {
    /// A card's width and height in world units.
    pub node: (f32, f32),
    /// Least air between two cards sharing a ring.
    pub gap: f32,
    /// Least distance from one ring to the next.
    pub step: f32,
}

impl Default for Ring {
    fn default() -> Self {
        Self {
            node: (190.0, 48.0),
            gap: 28.0,
            step: 240.0,
        }
    }
}

/// One node of the opened tree: the card, and the card it was reached through.
/// The centre is the one with no parent.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shoot {
    pub id: usize,
    pub parent: Option<usize>,
}

/// Where a card landed: the leading corner of its box.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Spot {
    pub id: usize,
    pub x: f32,
    pub y: f32,
}

/// Fan `tree` out around its centre.
///
/// `tree` is the spanning tree of what the reader has opened, and it must be in
/// the order the walk discovered it — centre first, then each node before its
/// own children. That order is what makes the drawing reproducible: the same
/// walk always produces the same picture.
pub fn radial(tree: &[Shoot], air: &Ring) -> Vec<Spot> {
    if tree.is_empty() {
        return Vec::new();
    }
    let (width, height) = air.node;

    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut root = None;
    for shoot in tree {
        match shoot.parent {
            Some(parent) => children.entry(parent).or_default().push(shoot.id),
            None if root.is_none() => root = Some(shoot.id),
            // A second parentless node is not part of this tree. It would have
            // no wedge to sit in, so it is left at the centre rather than
            // silently dropped; the caller should not be sending one.
            None => {}
        }
    }
    let Some(root) = root else {
        return Vec::new();
    };

    // Depth-first order, so a node can be handled after everything under it.
    let mut order: Vec<usize> = Vec::with_capacity(tree.len());
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        order.push(node);
        if let Some(kids) = children.get(&node) {
            stack.extend(kids.iter().copied());
        }
    }

    // How much room each subtree needs, as the radius of a disc that holds all
    // of it. Bottom up: a card on its own needs a disc that covers the card, and
    // a card with children needs however far its children sit plus whatever the
    // biggest of them needs in turn.
    let leaf = (width * width + height * height).sqrt() / 2.0;
    let mut room: HashMap<usize, f32> = HashMap::new();
    // How far each child of a node sits from it.
    let mut away: HashMap<usize, HashMap<usize, f32>> = HashMap::new();
    for &node in order.iter().rev() {
        let kids = children.get(&node).map(Vec::as_slice).unwrap_or(&[]);
        if kids.is_empty() {
            room.insert(node, leaf);
            continue;
        }
        // Each child claims a share of the fan in proportion to the room it
        // needs, so a heavy branch gets the angle it deserves and a single card
        // does not get the same slice as a subtree of forty.
        let total: f32 = kids.iter().map(|kid| room[kid]).sum::<f32>();
        let span = if node == root {
            std::f32::consts::TAU
        } else {
            // Not the whole circle: the way back to the parent is kept clear,
            // so a branch reads as growing away from where it came from. This
            // is what makes the drawing look like a tree rather than a target.
            std::f32::consts::TAU * 0.62
        };
        // Each child sits at its own distance — as far out as its own subtree
        // needs to clear the wedge it was given, and no further. A single card
        // stays close to its parent while a branch of forty stands off. That is
        // what makes the drawing read as a shape inside a shape rather than as
        // one outline with everything pinned to it.
        let mut span_of: HashMap<usize, f32> = HashMap::new();
        let mut far: f32 = 0.0;
        for &kid in kids {
            let share = span * room[&kid] / total.max(f32::EPSILON);
            // A disc of radius r clears its wedge at distance d when
            // d * sin(share / 2) covers r, plus air.
            let out = ((room[&kid] + air.gap) / (share / 2.0).sin().max(1e-3)).max(air.step);
            span_of.insert(kid, out);
            far = far.max(out + room[&kid]);
        }
        away.insert(node, span_of);
        room.insert(node, far);
    }

    // Positions, top down. Every node fans its children around itself, in a cone
    // pointing away from where it was reached from — so each subtree is the same
    // shape as the whole, one size smaller. That self-similarity is the point:
    // the reader can see at a glance which card a group belongs to, because the
    // group is arranged around it.
    let mut at: HashMap<usize, (f32, f32)> = HashMap::new();
    at.insert(root, (0.0, 0.0));
    let mut facing: HashMap<usize, f32> = HashMap::new();
    facing.insert(root, 0.0);

    let mut queue = std::collections::VecDeque::from([root]);
    while let Some(node) = queue.pop_front() {
        let kids = children.get(&node).map(Vec::as_slice).unwrap_or(&[]);
        if kids.is_empty() {
            continue;
        }
        let (x, y) = at[&node];
        let out = &away[&node];
        let total: f32 = kids.iter().map(|kid| room[kid]).sum::<f32>();
        let span = if node == root {
            std::f32::consts::TAU
        } else {
            std::f32::consts::TAU * 0.62
        };
        let outward = facing[&node];
        let mut cursor = outward - span / 2.0;
        for &kid in kids {
            let share = span * room[&kid] / total.max(f32::EPSILON);
            let angle = cursor + share / 2.0;
            cursor += share;
            let reach = out[&kid];
            at.insert(kid, (x + reach * angle.cos(), y + reach * angle.sin()));
            facing.insert(kid, angle);
            queue.push_back(kid);
        }
    }

    tree.iter()
        .filter_map(|shoot| {
            let &(x, y) = at.get(&shoot.id)?;
            Some(Spot {
                id: shoot.id,
                x: x - width / 2.0,
                y: y - height / 2.0,
            })
        })
        .collect()
}

/// The spanning tree of what is reachable from `root` through nodes the reader
/// has opened.
///
/// Breadth-first, so a node is reached by the shortest chain of opens that gets
/// to it, and its parent is the card it first arrived through. Every other edge
/// between two nodes on the pane is a real edge and is still drawn — it just
/// does not decide where anything sits. `order` breaks ties, so the same walk
/// always yields the same tree.
pub fn spanning(
    root: usize,
    opened: &dyn Fn(usize) -> bool,
    out: &dyn Fn(usize) -> Vec<usize>,
) -> Vec<Shoot> {
    let mut tree = vec![Shoot {
        id: root,
        parent: None,
    }];
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
    seen.insert(root);
    let mut queue = std::collections::VecDeque::from([root]);
    while let Some(node) = queue.pop_front() {
        if !opened(node) {
            continue;
        }
        for next in out(node) {
            if seen.insert(next) {
                tree.push(Shoot {
                    id: next,
                    parent: Some(node),
                });
                queue.push_back(next);
            }
        }
    }
    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grow(edges: &[(usize, usize)], root: usize, opened: &[usize]) -> Vec<Shoot> {
        let open: std::collections::HashSet<usize> = opened.iter().copied().collect();
        let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
        for &(a, b) in edges {
            out.entry(a).or_default().push(b);
        }
        spanning(
            root,
            &|id| open.contains(&id),
            &|id| out.get(&id).cloned().unwrap_or_default(),
        )
    }

    fn spots(tree: &[Shoot]) -> HashMap<usize, (f32, f32)> {
        radial(tree, &Ring::default())
            .into_iter()
            .map(|spot| (spot.id, (spot.x, spot.y)))
            .collect()
    }

    #[test]
    fn an_unopened_centre_is_alone() {
        let tree = grow(&[(0, 1), (0, 2)], 0, &[]);
        assert_eq!(tree.len(), 1);
        assert_eq!(spots(&tree)[&0], (-95.0, -24.0));
    }

    /// The opening reading: the centre, and what it points at, around it.
    #[test]
    fn opening_the_centre_fans_its_dependencies_around_it() {
        let edges: Vec<(usize, usize)> = (1..=6).map(|i| (0, i)).collect();
        let tree = grow(&edges, 0, &[0]);
        assert_eq!(tree.len(), 7);

        let at = spots(&tree);
        let centre = at[&0];
        let radii: Vec<f32> = (1..=6)
            .map(|i| {
                let (x, y) = at[&i];
                ((x - centre.0).powi(2) + (y - centre.1).powi(2)).sqrt()
            })
            .collect();
        // All on one ring, and none of them on top of the centre.
        for r in &radii {
            assert!((r - radii[0]).abs() < 1.0, "the ring is not round: {radii:?}");
            assert!(*r > 100.0);
        }
    }

    /// Opening reflows the drawing — wedges are re-shared, so radii move. What
    /// it must not do is disorient: the centre stays at the centre, nothing that
    /// was drawn disappears, the rings stay in depth order, and siblings keep
    /// the order they were going round in, so a branch that was clockwise of
    /// another still is.
    #[test]
    fn opening_a_node_keeps_the_reader_oriented() {
        let mut edges: Vec<(usize, usize)> = (1..=5).map(|i| (0, i)).collect();
        edges.extend((10..=14).map(|i| (3, i)));

        let before = spots(&grow(&edges, 0, &[0]));
        let after = spots(&grow(&edges, 0, &[0, 3]));
        assert_eq!(after.len(), before.len() + 5);
        for id in before.keys() {
            assert!(after.contains_key(id), "card {id} vanished");
        }

        let at_centre = |at: &HashMap<usize, (f32, f32)>, id: usize| {
            let (x, y) = at[&id];
            (x + 95.0, y + 24.0)
        };
        // Card centres, back from the leading corner the layout reports.
        let polar = |at: &HashMap<usize, (f32, f32)>, id: usize| {
            let (x, y) = at[&id];
            let (cx, cy) = (x + 95.0, y + 24.0);
            ((cx * cx + cy * cy).sqrt(), cy.atan2(cx))
        };

        assert!(polar(&after, 0).0 < 1.0, "the centre is not at the centre");

        // The four childless siblings still share a distance; card 3 does not,
        // because it now carries a branch and has to stand off to make room for
        // it. That difference is the shape.
        let plain = polar(&after, 1).0;
        for id in [2, 4, 5] {
            assert!(
                (polar(&after, id).0 - plain).abs() < 1.0,
                "card {id} should sit with the other childless siblings"
            );
        }
        assert!(
            polar(&after, 3).0 > plain,
            "card 3 carries five children and should have stood off"
        );
        // A child sits around its own parent, not on a ring of its own: it is
        // nearer to card 3 than to the centre.
        let three = at_centre(&after, 3);
        for id in 10..=14 {
            let kid = at_centre(&after, id);
            let to_parent = ((kid.0 - three.0).powi(2) + (kid.1 - three.1).powi(2)).sqrt();
            let to_middle = (kid.0 * kid.0 + kid.1 * kid.1).sqrt();
            assert!(
                to_parent < to_middle,
                "card {id} is {to_parent:.0} from its parent but {to_middle:.0} from the centre"
            );
        }

        let angles_before: Vec<f32> = (1..=5).map(|id| polar(&before, id).1).collect();
        let angles_after: Vec<f32> = (1..=5).map(|id| polar(&after, id).1).collect();
        let rank = |angles: &[f32]| {
            let mut by: Vec<usize> = (0..angles.len()).collect();
            by.sort_by(|&a, &b| angles[a].partial_cmp(&angles[b]).unwrap());
            by
        };
        assert_eq!(
            rank(&angles_before),
            rank(&angles_after),
            "the branches swapped places round the circle"
        );
    }

    /// The failure that killed strict wedge subdivision: three opens into a real
    /// fan asked for a pane 133,000 units across. Wedges by subtree size keep the
    /// drawing bounded by how many cards are on it.
    #[test]
    fn a_deep_walk_stays_a_sane_size() {
        let mut edges: Vec<(usize, usize)> = (1..=34).map(|i| (0, i)).collect();
        for parent in 1..=6 {
            for step in 0..12 {
                edges.push((parent, parent * 100 + step));
            }
        }
        let opened: Vec<usize> = std::iter::once(0).chain(1..=6).collect();
        let at = spots(&grow(&edges, 0, &opened));
        let reach = at
            .values()
            .map(|(x, y)| (x * x + y * y).sqrt())
            .fold(0.0f32, f32::max);
        assert!(
            reach < 8_000.0,
            "{} cards reached {reach:.0} units from the centre",
            at.len()
        );
    }

    /// A card reached two ways is drawn once, through whichever chain of opens
    /// found it first. The other edge is still an edge; it just places nothing.
    #[test]
    fn a_card_reached_twice_is_placed_once() {
        // Both 1 and 2 depend on 3.
        let edges = [(0, 1), (0, 2), (1, 3), (2, 3)];
        let tree = grow(&edges, 0, &[0, 1, 2]);
        assert_eq!(tree.len(), 4, "3 should be on the pane exactly once");
        let three = tree.iter().find(|shoot| shoot.id == 3).unwrap();
        assert_eq!(three.parent, Some(1), "reached through 1 first");
    }

    /// No two cards overlap, however lopsided the tree.
    #[test]
    fn no_two_cards_overlap() {
        let mut edges: Vec<(usize, usize)> = (1..=9).map(|i| (0, i)).collect();
        for parent in 1..=9 {
            for step in 0..6 {
                edges.push((parent, parent * 10 + step));
            }
        }
        let opened: Vec<usize> = std::iter::once(0).chain(1..=9).collect();
        let tree = grow(&edges, 0, &opened);
        let at = spots(&tree);
        let (w, h) = Ring::default().node;

        let ids: Vec<usize> = at.keys().copied().collect();
        for (index, &a) in ids.iter().enumerate() {
            for &b in &ids[index + 1..] {
                let (ax, ay) = at[&a];
                let (bx, by) = at[&b];
                assert!(
                    (ax - bx).abs() >= w - 0.5 || (ay - by).abs() >= h - 0.5,
                    "{a} at {:?} overlaps {b} at {:?}",
                    (ax, ay),
                    (bx, by)
                );
            }
        }
    }

    /// The same walk always draws the same picture.
    #[test]
    fn the_same_walk_lands_the_same_way_twice() {
        let edges: Vec<(usize, usize)> = (1..=8)
            .map(|i| (0, i))
            .chain((1..=8).map(|i| (i, i + 20)))
            .collect();
        let opened = [0, 2, 5];
        assert_eq!(
            spots(&grow(&edges, 0, &opened)),
            spots(&grow(&edges, 0, &opened))
        );
    }
}
