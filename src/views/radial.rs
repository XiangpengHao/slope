//! The radial layout: dependency distance as concentric rings.
//!
//! The chart's one organizing idea: the crate under review sits at the
//! center, and every other crate sits on the ring of its dependency
//! distance — ring one is what the center depends on directly, ring two is
//! what those need, and so on. Placement is a pure function of the graph
//! and the center: BFS gives each crate its ring (minimum hop distance), a
//! walk of the BFS tree gives it an angular sector shared with its
//! dependency lineage, and each ring's radius grows when it gets crowded.
//! No physics, no iteration order luck: the same workspace always draws
//! the same chart, and nothing ever moves.

use std::collections::{HashMap, HashSet, VecDeque};
use std::f64::consts::TAU;

use dioxus_flow::prelude::Point;

use crate::api::{DepEvent, WorkspaceGraph};

/// How many rings the chart shows by default: everything farther collapses
/// onto the outermost ring as one "N+ hops" band, so the plate stays
/// compact. Selecting a crate inside the band expands exact rings down to
/// its true depth.
pub const DEFAULT_CAP: u32 = 4;

/// Base gap between rings.
const RING_GAP: f64 = 150.0;
/// Arc room (flow px) each star wants from its ring neighbors.
const MIN_ARC: f64 = 24.0;
/// Crowding may push a ring at most this far past the previous one, so a
/// single tight pair can never blow the whole chart up.
const MAX_RING_GAP: f64 = 380.0;
/// Ghost stars (removed dependencies) hang this far past their user's ring.
const GHOST_OFFSET: f64 = 62.0;

/// One placed star.
#[derive(Clone, PartialEq)]
pub struct Placed {
    /// The ring the star is drawn on: its hops, collapsed onto the cap.
    pub ring: u32,
    /// True dependency distance from the center; 0 is the center itself.
    pub hops: u32,
    /// Radians; 0 points right, negative is up (SVG orientation).
    pub angle: f64,
    pub point: Point,
}

/// The whole chart's geometry. The chart center is flow origin (0, 0).
#[derive(Clone, PartialEq)]
pub struct RadialLayout {
    pub placed: HashMap<String, Placed>,
    /// Radius of each drawn ring, indexed by ring number (`radii[0] == 0`).
    pub radii: Vec<f64>,
    /// Node id at the center. `None` when a virtual workspace has no root
    /// package; members then share ring one around an implied hub.
    pub center: Option<String>,
    /// The deepest true distance in the graph; the outermost ring is a
    /// collapsed band whenever this exceeds the cap.
    pub max_hops: u32,
}

/// Lay the graph out on rings, collapsing everything past `cap` hops onto
/// the outermost ring. Angles never depend on the cap, so expanding only
/// moves collapsed stars radially outward to their true rings.
pub fn radial_layout(graph: &WorkspaceGraph, cap: u32) -> RadialLayout {
    let ghosts: HashSet<&str> = graph
        .crates
        .iter()
        .filter(|c| c.ghost)
        .map(|c| c.id.as_str())
        .collect();

    // Forward adjacency (user -> dependency), the direction rings grow.
    let mut fwd: HashMap<&str, Vec<&str>> = HashMap::new();
    for link in &graph.links {
        if link.event == Some(DepEvent::Removed) || ghosts.contains(link.to.as_str()) {
            continue;
        }
        fwd.entry(link.from.as_str())
            .or_default()
            .push(link.to.as_str());
    }
    for deps in fwd.values_mut() {
        deps.sort_unstable();
        deps.dedup();
    }

    let center: Option<&str> = graph.root_crate.as_deref().and_then(|id| {
        graph
            .crates
            .iter()
            .find(|c| c.id == id && !c.ghost)
            .map(|c| c.id.as_str())
    });

    // True distances and primary parents (the BFS tree the sectors follow).
    // The distance is the minimum hop count, so a diamond dependency
    // appears once, as close to the center as it truly is.
    let mut hops: HashMap<&str, u32> = HashMap::new();
    let mut parent: HashMap<&str, &str> = HashMap::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    if let Some(id) = center {
        hops.insert(id, 0);
        queue.push_back(id);
    }
    loop {
        while let Some(u) = queue.pop_front() {
            let d = hops[u];
            if let Some(deps) = fwd.get(u) {
                for &v in deps {
                    if !hops.contains_key(v) {
                        hops.insert(v, d + 1);
                        parent.insert(v, u);
                        queue.push_back(v);
                    }
                }
            }
        }
        // Members the center never reaches still belong beside it: they
        // seat on ring one, hung from the workspace itself. On a virtual
        // workspace this seats every member.
        let mut unseated: Vec<&str> = graph
            .crates
            .iter()
            .filter(|c| c.is_member && !c.ghost && !hops.contains_key(c.id.as_str()))
            .map(|c| c.id.as_str())
            .collect();
        let seat_at = if unseated.is_empty() {
            // A resolve-graph island (reachable from no member) still gets
            // ground: outside the settled rings, never lost.
            unseated = graph
                .crates
                .iter()
                .filter(|c| !c.ghost && !hops.contains_key(c.id.as_str()))
                .map(|c| c.id.as_str())
                .collect();
            hops.values().copied().max().unwrap_or(0) + 1
        } else {
            1
        };
        if unseated.is_empty() {
            break;
        }
        unseated.sort_unstable();
        for id in unseated {
            hops.insert(id, seat_at);
            queue.push_back(id);
        }
    }

    // The BFS tree. Roots are the center and every seated straggler.
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut roots: Vec<&str> = Vec::new();
    for &id in hops.keys() {
        match parent.get(id) {
            Some(&p) => children.entry(p).or_default().push(id),
            None => roots.push(id),
        }
    }
    for kids in children.values_mut() {
        kids.sort_unstable();
    }
    roots.sort_unstable();

    // Subtree weight = leaf count: a lineage's angular share is how much
    // sky its outermost crates need.
    let mut deepest_first: Vec<&str> = hops.keys().copied().collect();
    deepest_first.sort_unstable_by_key(|id| (std::cmp::Reverse(hops[id]), *id));
    let mut weight: HashMap<&str, f64> = HashMap::new();
    for id in &deepest_first {
        let w: f64 = children
            .get(id)
            .map(|kids| kids.iter().map(|k| weight[k]).sum())
            .unwrap_or(0.0);
        weight.insert(id, w.max(1.0));
    }

    // Sectors: the full circle divided among the roots by weight, then each
    // node's sector divided among its children the same way. A node sits at
    // the middle of its own sector, so children spread around their parent.
    // Shares go by the square root of the weight: a big lineage still gets
    // more sky, but can never starve its small siblings into a sliver.
    let share = |id: &str| weight[id].sqrt();
    let mut sector: HashMap<&str, (f64, f64)> = HashMap::new();
    let mut angle: HashMap<&str, f64> = HashMap::new();
    let total: f64 = roots.iter().map(|r| share(r)).sum::<f64>().max(1.0);
    let mut cursor = -TAU / 4.0; // start at twelve o'clock
    let mut stack: Vec<&str> = Vec::new();
    for &r in &roots {
        let w = TAU * share(r) / total;
        sector.insert(r, (cursor, w));
        angle.insert(r, cursor + w / 2.0);
        cursor += w;
        stack.push(r);
    }
    while let Some(u) = stack.pop() {
        let (start, width) = sector[u];
        let Some(kids) = children.get(u) else {
            continue;
        };
        let sum: f64 = kids.iter().map(|k| share(k)).sum::<f64>().max(1.0);
        let mut c = start;
        for &k in kids {
            let w = width * share(k) / sum;
            sector.insert(k, (c, w));
            angle.insert(k, c + w / 2.0);
            c += w;
            stack.push(k);
        }
    }

    // Ring radii: the base gap, pushed out when a ring is crowded. The
    // quartile angular gap sets the push, so real crowding counts but a few
    // tight pairs don't inflate the whole plate. Distances past the cap
    // share the outermost ring.
    let max_hops = hops.values().copied().max().unwrap_or(0);
    let max_ring = max_hops.min(cap) as usize;
    let mut ring_angles: Vec<Vec<f64>> = vec![Vec::new(); max_ring + 1];
    for (&id, &h) in &hops {
        ring_angles[h.min(cap) as usize].push(angle[&id]);
    }
    let mut radii = vec![0.0f64; max_ring + 1];
    for k in 1..=max_ring {
        let base = radii[k - 1] + RING_GAP;
        let angles = &mut ring_angles[k];
        angles.sort_by(f64::total_cmp);
        let needed = if angles.len() >= 2 {
            let mut gaps: Vec<f64> = angles.windows(2).map(|w| w[1] - w[0]).collect();
            gaps.push(angles[0] + TAU - angles[angles.len() - 1]);
            gaps.sort_by(f64::total_cmp);
            let gap = gaps[gaps.len() / 4].max(1e-4);
            MIN_ARC / gap
        } else {
            0.0
        };
        radii[k] = needed.clamp(base, radii[k - 1] + MAX_RING_GAP);
    }

    let mut placed: HashMap<String, Placed> = HashMap::new();
    for (&id, &h) in &hops {
        let a = angle[&id];
        let r = h.min(cap);
        let radius = radii[r as usize];
        placed.insert(
            id.to_string(),
            Placed {
                ring: r,
                hops: h,
                angle: a,
                point: Point::new(radius * a.cos(), radius * a.sin()),
            },
        );
    }

    // Ghost stars hang just past the crate that dropped them, staggered
    // when one crate dropped several.
    let mut ghost_links: Vec<(&str, &str)> = graph
        .links
        .iter()
        .filter(|l| l.event == Some(DepEvent::Removed) && ghosts.contains(l.to.as_str()))
        .map(|l| (l.from.as_str(), l.to.as_str()))
        .collect();
    ghost_links.sort_unstable();
    ghost_links.dedup();
    let mut ghost_seat: HashMap<&str, usize> = HashMap::new();
    for (from, to) in ghost_links {
        if placed.contains_key(to) {
            continue;
        }
        let (a0, r0, ring0, hops0) = placed
            .get(from)
            .map(|p| {
                let a = if p.ring == 0 { -TAU / 4.0 } else { p.angle };
                (a, radii[p.ring as usize], p.ring, p.hops)
            })
            .unwrap_or((-TAU / 4.0, 0.0, 0, 0));
        let seat = ghost_seat.entry(from).or_insert(0);
        let swing = (*seat as f64 / 2.0 + 1.0).floor() * 0.22;
        let a = a0 + if seat.is_multiple_of(2) { swing } else { -swing };
        *seat += 1;
        let radius = r0 + GHOST_OFFSET;
        placed.insert(
            to.to_string(),
            Placed {
                ring: ring0,
                hops: hops0,
                angle: a,
                point: Point::new(radius * a.cos(), radius * a.sin()),
            },
        );
    }

    RadialLayout {
        placed,
        radii,
        center: center.map(str::to_string),
        max_hops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{CrateInfo, DepKind, DepLink, Epoch};

    fn krate(id: &str, is_member: bool) -> CrateInfo {
        CrateInfo {
            id: id.to_string(),
            name: id.split('@').next().unwrap().to_string(),
            version: "1.0.0".to_string(),
            is_member,
            changed: false,
            changed_files: 0,
            manifest_changed: false,
            affected_dist: None,
            dependents: 0,
            direct_deps: 0,
            external_deps: 0,
            ghost: false,
            description: None,
            license: None,
            repository: None,
            homepage: None,
            documentation: None,
            crates_io: false,
            rel_path: None,
        }
    }

    fn link(from: &str, to: &str) -> DepLink {
        DepLink {
            from: from.to_string(),
            to: to.to_string(),
            kind: DepKind::Normal,
            event: None,
        }
    }

    fn graph(root: Option<&str>, crates: Vec<CrateInfo>, links: Vec<DepLink>) -> WorkspaceGraph {
        WorkspaceGraph {
            name: "test".into(),
            root: "/test".into(),
            root_crate: root.map(str::to_string),
            epoch: Epoch {
                vcs: None,
                base: "base".into(),
                target: "working copy".into(),
                clean: true,
                note: None,
            },
            crates,
            links,
        }
    }

    /// A diamond seats each crate once, at its minimum distance.
    #[test]
    fn rings_are_min_distance() {
        let g = graph(
            Some("root@1.0.0"),
            vec![
                krate("root@1.0.0", true),
                krate("a@1.0.0", false),
                krate("b@1.0.0", false),
                krate("shared@1.0.0", false),
            ],
            vec![
                link("root@1.0.0", "a@1.0.0"),
                link("root@1.0.0", "shared@1.0.0"),
                link("a@1.0.0", "b@1.0.0"),
                link("b@1.0.0", "shared@1.0.0"),
            ],
        );
        let l = radial_layout(&g, u32::MAX);
        assert_eq!(l.placed["root@1.0.0"].ring, 0);
        assert_eq!(l.placed["a@1.0.0"].ring, 1);
        assert_eq!(l.placed["shared@1.0.0"].ring, 1);
        assert_eq!(l.placed["b@1.0.0"].ring, 2);
        assert_eq!(l.max_hops, 2);
        assert_eq!(l.radii[0], 0.0);
        assert!(l.radii[1] >= RING_GAP);
    }

    /// A virtual workspace (no root package) seats every member on ring one;
    /// nothing is ever left unplaced.
    #[test]
    fn virtual_workspace_members_share_ring_one() {
        let g = graph(
            None,
            vec![
                krate("m1@1.0.0", true),
                krate("m2@1.0.0", true),
                krate("dep@1.0.0", false),
            ],
            vec![link("m1@1.0.0", "dep@1.0.0")],
        );
        let l = radial_layout(&g, u32::MAX);
        assert_eq!(l.center, None);
        assert_eq!(l.placed["m1@1.0.0"].ring, 1);
        assert_eq!(l.placed["m2@1.0.0"].ring, 1);
        assert_eq!(l.placed["dep@1.0.0"].ring, 2);
        assert_eq!(l.placed.len(), 3);
    }

    /// A member the root never reaches still seats on ring one.
    #[test]
    fn unreachable_member_seats_on_ring_one() {
        let g = graph(
            Some("root@1.0.0"),
            vec![
                krate("root@1.0.0", true),
                krate("tool@1.0.0", true),
                krate("toolutil@1.0.0", false),
            ],
            vec![link("tool@1.0.0", "toolutil@1.0.0")],
        );
        let l = radial_layout(&g, u32::MAX);
        assert_eq!(l.placed["tool@1.0.0"].ring, 1);
        assert_eq!(l.placed["toolutil@1.0.0"].ring, 2);
    }

    /// Everything past the cap collapses onto the outermost ring, keeping
    /// its true distance and its angle.
    #[test]
    fn cap_collapses_deep_crates_onto_last_ring() {
        let g = graph(
            Some("root@1.0.0"),
            vec![
                krate("root@1.0.0", true),
                krate("a@1.0.0", false),
                krate("b@1.0.0", false),
                krate("c@1.0.0", false),
            ],
            vec![
                link("root@1.0.0", "a@1.0.0"),
                link("a@1.0.0", "b@1.0.0"),
                link("b@1.0.0", "c@1.0.0"),
            ],
        );
        let capped = radial_layout(&g, 2);
        assert_eq!(capped.max_hops, 3);
        assert_eq!(capped.radii.len(), 3);
        assert_eq!(capped.placed["b@1.0.0"].ring, 2);
        assert_eq!(capped.placed["c@1.0.0"].ring, 2);
        assert_eq!(capped.placed["c@1.0.0"].hops, 3);
        let full = radial_layout(&g, u32::MAX);
        assert_eq!(full.placed["c@1.0.0"].ring, 3);
        // Expanding never swings a star sideways: same angle, new radius.
        assert_eq!(
            capped.placed["c@1.0.0"].angle,
            full.placed["c@1.0.0"].angle
        );
    }
}
