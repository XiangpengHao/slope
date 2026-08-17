//! Reachability: what runs, from where, through what.
//!
//! The pane says what calls what. This says the things a picture cannot:
//!
//! - **What must every route cross?** Not the most-called function — the ones
//!   that *dominate*: functions no route to a region can avoid. That set is the
//!   architecture, and it is computed here rather than asserted by a human.
//! - **How much does this touch, and how much touches it?** Both closures, and
//!   the overlap between them, because a call graph has cycles and the two
//!   numbers genuinely count some functions twice.
//! - **How does execution get here?** One route, written the way a stack trace
//!   is written, because that is the one representation of a call chain every
//!   developer already reads fluently.
//!
//! Where each of these runs is not a detail. Dominance is a question about the
//! whole graph, the same for every reader, and on a workspace of eight thousand
//! functions it is seconds of arithmetic — so it runs **once, on the server**,
//! and rides along on the [`Sheet`]. Everything else is asked about one function,
//! when a reader selects it, and costs a walk or two.
//!
//! The first version of this module drew no such line: it filled a table of
//! reach counts for every function up front, which is `V·(V+E)`, and then walked
//! from each of 476 entry points separately to fill a column nothing read. In a
//! browser, on a real workspace, that is not slow — it is a hang.
//!
//! Nothing here returns a coordinate, and nothing here draws.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::{Origin, Sheet, UnitKind};

/// A route from an entry point to a target, nearest first.
pub type Path = Vec<usize>;

/// What the analysis found about one function, asked for one function at a time.
///
/// These used to be computed for every function up front, and on a workspace of
/// a few hundred that was fine. It is not fine at eight thousand: reachability
/// per node is two walks of the whole graph, so the table costs `V·(V+E)` to
/// fill and the reader looks at one row of it. Now the row is computed when it
/// is asked for — two walks, once, for the function actually on screen.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Standing {
    /// Functions this one can reach, itself excluded.
    pub reaches: usize,
    /// Functions that can reach it.
    pub reached_by: usize,
    /// Functions that reach it *and* are reached by it: the cycle it takes part
    /// in.
    ///
    /// The dependency board never has to say this, because a dependency graph is
    /// acyclic and its two closures are disjoint. A call graph's are not, so "12
    /// callers" and "3 callees" can genuinely count the same function twice.
    /// Naming the overlap is the difference between two honest numbers and two
    /// that look like they should add up.
    pub both_ways: usize,
}

/// What every route to a function must cross, over the whole call graph.
///
/// Dominance is the one answer here that cannot be narrowed to what is on
/// screen: whether there is a way round a function is a question about the
/// entire graph, so it is computed once, where the whole graph is — on the
/// server, during extraction — and carried on the [`Sheet`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reach {
    /// Immediate dominator of each unit, where one exists.
    idom: Vec<Option<usize>>,
    /// How many functions each one dominates: every route from an entry point to
    /// any of them passes through here. This is the chokepoint weight, and it is
    /// what separates an architectural boundary from a popular utility.
    ///
    /// `Vec::len` has enormous fan-in and dominates nothing, because there is
    /// always another way round. A dispatcher has modest fan-in and dominates
    /// half the program, because there is not.
    dominates: Vec<u32>,
    /// Functions in this workspace no entry point reaches. Dead, or reached only
    /// through a mechanism the analyser cannot see.
    pub unreached: Vec<usize>,
}

impl Reach {
    /// Nothing known — what an empty sheet answers.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn none() -> Self {
        Self {
            idom: Vec::new(),
            dominates: Vec::new(),
            unreached: Vec::new(),
        }
    }

    pub fn dominates(&self, id: usize) -> usize {
        self.dominates.get(id).copied().unwrap_or(0) as usize
    }

    /// The chain of chokepoints above a function: every unit that every route
    /// to it must cross, outermost first. This is a *guaranteed* answer, unlike
    /// any single path — nothing reaches the target without crossing all of it.
    pub fn spine_to(&self, id: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut here = id;
        let mut guard = 0;
        while let Some(parent) = self.idom.get(here).copied().flatten() {
            chain.push(parent);
            here = parent;
            guard += 1;
            if guard > 4096 {
                break;
            }
        }
        chain.reverse();
        chain
    }

    /// The functions worth reading first: the ones that dominate the most.
    pub fn chokepoints(&self, sheet: &Sheet, want: usize) -> Vec<usize> {
        let mut ranked: Vec<usize> = sheet
            .units
            .iter()
            .filter(|u| {
                u.kind == UnitKind::Function
                    && u.origin == Origin::Workspace
                    && self.dominates(u.id) > 0
            })
            .map(|u| u.id)
            .collect();
        ranked.sort_by_key(|&id| {
            (
                std::cmp::Reverse(self.dominates(id)),
                sheet.units[id].flow,
                sheet.units[id].qualified.clone(),
            )
        });
        ranked.truncate(want);
        ranked
    }

    /// The chokepoints inside one container, ranked. What a crate or a file
    /// answers to "where do I start reading in here".
    pub fn chokepoints_under(&self, sheet: &Sheet, root: usize, want: usize) -> Vec<usize> {
        let mut ranked: Vec<usize> = descendants(sheet, root)
            .into_iter()
            .filter(|&id| {
                sheet.units[id].kind == UnitKind::Function && self.dominates(id) > 0
            })
            .collect();
        ranked.sort_by_key(|&id| {
            (
                std::cmp::Reverse(self.dominates(id)),
                sheet.units[id].flow,
                sheet.units[id].qualified.clone(),
            )
        });
        ranked.truncate(want);
        ranked
    }
}

/// How much one function reaches, is reached by, and shares with itself round a
/// cycle. Three walks of the graph, for one function, when a reader asks.
pub fn standing(sheet: &Sheet, id: usize) -> Standing {
    if sheet.units.get(id).map(|unit| unit.kind) != Some(UnitKind::Function) {
        return Standing::default();
    }
    let down = collect(sheet, id, Direction::Calls);
    let up = collect(sheet, id, Direction::Callers);
    Standing {
        reaches: down.len(),
        reached_by: up.len(),
        both_ways: up.intersection(&down).count(),
    }
}

/// Every unit at or under a container, itself included.
pub fn descendants(sheet: &Sheet, root: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    let mut guard = sheet.units.len() + 1;
    while let Some(id) = stack.pop() {
        let Some(unit) = sheet.units.get(id) else {
            continue;
        };
        out.push(id);
        stack.extend(unit.children.iter().copied());
        guard = match guard.checked_sub(1) {
            Some(left) => left,
            None => break,
        };
    }
    out
}

/// Compute dominance over the whole call graph, and what nothing reaches.
///
/// Server-side only, and not compiled into the client at all. That is the point
/// of the split: the answer is the same for every reader, it costs a dominator
/// computation over tens of thousands of edges, and the browser is the worst
/// machine in the system to spend that on.
#[cfg(not(target_arch = "wasm32"))]
pub fn analyse(sheet: &Sheet) -> Reach {
    let count = sheet.units.len();
    let entries = &sheet.entries;

    // One walk from all the beginnings at once. The old shape walked from each
    // beginning separately to count how many reach a given function; nothing
    // ever asked for that number, and on a workspace with 476 ways in it was 476
    // walks of the whole graph for an answer nobody read.
    let mut reached = vec![false; count];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for &entry in entries {
        if entry < count && !reached[entry] {
            reached[entry] = true;
            queue.push_back(entry);
        }
    }
    while let Some(id) = queue.pop_front() {
        for &next in &sheet.units[id].calls {
            if !reached[next] {
                reached[next] = true;
                queue.push_back(next);
            }
        }
    }

    let idom = dominators(sheet, entries, count);
    let mut dominates = vec![0u32; count];
    for (id, &parent) in idom.iter().enumerate() {
        // Every function is dominated by its whole idom chain, so walking each
        // node's chain and crediting it upward gives the subtree size.
        if parent.is_none() {
            continue;
        }
        let mut here = id;
        let mut guard = 0;
        while let Some(next) = idom[here] {
            dominates[next] += 1;
            here = next;
            guard += 1;
            if guard > 4096 {
                break;
            }
        }
    }

    let unreached = sheet
        .units
        .iter()
        .filter(|u| {
            u.kind == UnitKind::Function && u.origin == Origin::Workspace && !reached[u.id]
        })
        .map(|u| u.id)
        .collect();

    Reach {
        idom,
        dominates,
        unreached,
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Calls,
    Callers,
}

/// Everything reachable from `from` in one direction, excluding itself.
fn collect(
    sheet: &Sheet,
    from: usize,
    direction: Direction,
) -> std::collections::HashSet<usize> {
    let mut seen = std::collections::HashSet::from([from]);
    let mut queue = VecDeque::from([from]);
    while let Some(id) = queue.pop_front() {
        let next = match direction {
            Direction::Calls => &sheet.units[id].calls,
            Direction::Callers => &sheet.units[id].callers,
        };
        for &step in next {
            if seen.insert(step) {
                queue.push_back(step);
            }
        }
    }
    seen.remove(&from);
    seen
}

/// Functions one hop from this one, each side ordered by how much it reaches,
/// so the order a reader sees is the order worth following.
pub fn immediate(sheet: &Sheet, id: usize) -> (Vec<usize>, Vec<usize>) {
    let unit = &sheet.units[id];
    let rank = |ids: &[usize]| -> Vec<usize> {
        let mut ids = ids.to_vec();
        ids.sort_unstable();
        ids.dedup();
        ids.sort_by_key(|&other| {
            let unit = &sheet.units[other];
            (
                unit.origin != Origin::Workspace,
                std::cmp::Reverse(unit.callers.len()),
                unit.qualified.clone(),
            )
        });
        ids
    };
    (rank(&unit.callers), rank(&unit.calls))
}

/// Immediate dominators over the call graph, rooted at a virtual node above
/// every beginning.
///
/// Cooper, Harvey and Kennedy's iterative formulation. The virtual root is a
/// real node here — index `count` — rather than an implied one, because the
/// implied version has to answer "what happens when the walk reaches a
/// beginning" at every step and the first attempt at that answered it wrongly:
/// it kept whichever candidate it already had, and so claimed a chokepoint
/// where there was a way round. With an explicit root the walk always
/// terminates somewhere real and the answer falls out.
///
/// A function's dominator is the last thing every route from a beginning must
/// pass through before reaching it. That is far stronger than "many things
/// call this": fan-in counts popularity, dominance counts *inevitability*, and
/// only one of those is architecture.
#[cfg(not(target_arch = "wasm32"))]
fn dominators(sheet: &Sheet, entries: &[usize], count: usize) -> Vec<Option<usize>> {
    if entries.is_empty() {
        return vec![None; count];
    }
    let root = count;
    let total = count + 1;

    // Predecessors: callers, plus the virtual root for every beginning.
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); total];
    for unit in &sheet.units {
        for &caller in &unit.callers {
            preds[unit.id].push(caller);
        }
    }
    for &entry in entries {
        preds[entry].push(root);
    }
    let succs = |id: usize| -> Vec<usize> {
        if id == root {
            entries.to_vec()
        } else {
            sheet.units[id].calls.clone()
        }
    };

    // Postorder from the virtual root, iteratively: a deep chain must not
    // depend on the stack.
    let mut postorder: Vec<usize> = Vec::with_capacity(total);
    let mut visited = vec![false; total];
    let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
    visited[root] = true;
    while let Some((id, step)) = stack.pop() {
        let next = succs(id);
        if step < next.len() {
            stack.push((id, step + 1));
            let child = next[step];
            if !visited[child] {
                visited[child] = true;
                stack.push((child, 0));
            }
        } else {
            postorder.push(id);
        }
    }
    let mut number = vec![usize::MAX; total];
    for (index, &id) in postorder.iter().enumerate() {
        number[id] = index;
    }

    let mut idom: Vec<Option<usize>> = vec![None; total];
    idom[root] = Some(root);

    // Reverse postorder, root first.
    let order: Vec<usize> = postorder.iter().rev().copied().collect();

    let mut changed = true;
    while changed {
        changed = false;
        for &id in &order {
            if id == root {
                continue;
            }
            let mut new: Option<usize> = None;
            for &pred in &preds[id] {
                if idom[pred].is_none() {
                    continue;
                }
                new = Some(match new {
                    None => pred,
                    Some(current) => intersect(pred, current, &idom, &number),
                });
            }
            if let Some(new) = new
                && idom[id] != Some(new)
            {
                idom[id] = Some(new);
                changed = true;
            }
        }
    }

    // A node whose dominator is the virtual root has none inside the graph:
    // that is what being a beginning, or being reachable from several
    // independent ones, means.
    idom.truncate(count);
    idom.into_iter()
        .map(|parent| parent.filter(|&p| p != root))
        .collect()
}

/// Walk two fingers up the dominator tree until they meet. Postorder numbers
/// increase toward the root, so the lower-numbered finger is the deeper one and
/// is the one that climbs.
#[cfg(not(target_arch = "wasm32"))]
fn intersect(a: usize, b: usize, idom: &[Option<usize>], number: &[usize]) -> usize {
    let (mut x, mut y) = (a, b);
    let mut guard = 0;
    while x != y {
        while number[x] < number[y] {
            let Some(next) = idom[x] else { return y };
            if next == x {
                return y;
            }
            x = next;
            guard += 1;
            if guard > 100_000 {
                return x;
            }
        }
        while number[y] < number[x] {
            let Some(next) = idom[y] else { return x };
            if next == y {
                return x;
            }
            y = next;
            guard += 1;
            if guard > 100_000 {
                return y;
            }
        }
    }
    x
}

/// The shortest route from a beginning down to a function, written the way a
/// stack trace is written — because that is the representation of a call chain
/// every developer already reads without being taught.
///
/// One route, not all of them. Enumerating routes is what the first version did,
/// and it is exponential in the worst case and merely ruinous in the ordinary
/// one: a leaf utility in a real workspace is reachable by hundreds of thousands
/// of distinct chains, all of them saying the same thing. This is a breadth-first
/// walk backwards keeping one parent pointer each, so it costs one pass over the
/// graph and stops at the first beginning it meets — which, being breadth-first,
/// is the nearest one.
///
/// The *guaranteed* answer is [`Reach::spine_to`]; this is the concrete one, for
/// where there is nothing above a function that every route must cross.
pub fn route_to(sheet: &Sheet, target: usize) -> Option<Path> {
    if sheet.units.get(target).map(|unit| unit.kind) != Some(UnitKind::Function) {
        return None;
    }
    if sheet.units[target].root.is_root() {
        return Some(vec![target]);
    }
    let mut came_from: Vec<usize> = vec![usize::MAX; sheet.units.len()];
    let mut queue = VecDeque::from([target]);
    came_from[target] = target;
    while let Some(id) = queue.pop_front() {
        for &caller in &sheet.units[id].callers {
            if came_from[caller] != usize::MAX {
                continue;
            }
            came_from[caller] = id;
            if sheet.units[caller].root.is_root() {
                // Walk the pointers back down, which puts the route in the
                // order it runs in.
                let mut route = vec![caller];
                let mut here = caller;
                while here != target {
                    here = came_from[here];
                    route.push(here);
                }
                return Some(route);
            }
            queue.push_back(caller);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call::{Root, Unit};

    fn function(id: usize, name: &str) -> Unit {
        Unit {
            id,
            parent: None,
            children: Vec::new(),
            kind: UnitKind::Function,
            name: name.into(),
            qualified: name.into(),
            signature: None,
            trait_name: None,
            self_ty: None,
            origin: Origin::Workspace,
            root: Root::No,
            flow: 0,
            crate_name: "k".into(),
            file: "f.rs".into(),
            line: 1,
            depth: 0,
            calls: Vec::new(),
            callers: Vec::new(),
            function_count: 1,
        }
    }

    /// Builds a sheet from `(from, to)` edges over named functions, with the
    /// given ids as entry points.
    fn sheet(names: &[&str], edges: &[(usize, usize)], entries: &[usize]) -> Sheet {
        let mut units: Vec<Unit> = names
            .iter()
            .enumerate()
            .map(|(id, name)| function(id, name))
            .collect();
        for &(from, to) in edges {
            units[from].calls.push(to);
            units[to].callers.push(from);
        }
        for &entry in entries {
            units[entry].root = Root::Main;
        }
        Sheet {
            workspace: "w".into(),
            manifest_dir: "/w".into(),
            units,
            roots: (0..names.len()).collect(),
            entries: entries.to_vec(),
            reach: Reach::none(),
            function_count: names.len(),
            call_count: edges.len(),
            tests_excluded: 0,
            workspace_crates: 1,
            dependency_crates: 0,
            unopened: Vec::new(),
            took_ms: 0,
        }
    }

    /// The distinction the whole design rests on: dominance is inevitability,
    /// not popularity.
    ///
    /// `main` reaches `work` two ways, through `a` or through `b`, and then
    /// everything funnels through `gate`. `util` is called by more things than
    /// `gate` is — and dominates nothing, because there is always another way
    /// to reach whatever it reaches. `gate` dominates, because there is not.
    #[test]
    fn dominance_finds_the_chokepoint_not_the_popular_function() {
        //        main
        //        /  \
        //       a    b        util  <- called by a, b and gate
        //        \  /
        //        gate
        //          |
        //        deep
        let s = sheet(
            &["main", "a", "b", "gate", "deep", "util"],
            &[
                (0, 1), (0, 2),
                (1, 3), (2, 3),
                (3, 4),
                (1, 5), (2, 5), (3, 5),
            ],
            &[0],
        );
        let reach = analyse(&s);

        assert!(
            reach.dominates(3) >= 1,
            "gate is on every route to deep, so it dominates it"
        );
        assert_eq!(
            reach.dominates(5),
            0,
            "util is called by three things and dominates nothing — popularity is not architecture"
        );
        assert!(
            s.units[5].callers.len() > s.units[3].callers.len(),
            "and util really is the more popular of the two"
        );
        // The guaranteed answer: everything must cross main, then gate.
        assert_eq!(reach.spine_to(4), vec![0, 3]);
    }

    #[test]
    fn an_entry_point_is_dominated_by_nothing() {
        let s = sheet(&["main", "a"], &[(0, 1)], &[0]);
        let reach = analyse(&s);
        assert!(reach.spine_to(0).is_empty());
        assert_eq!(reach.spine_to(1), vec![0]);
    }

    #[test]
    fn reach_counts_both_directions() {
        let s = sheet(&["main", "a", "b"], &[(0, 1), (1, 2)], &[0]);
        assert_eq!(standing(&s, 0).reaches, 2);
        assert_eq!(standing(&s, 2).reached_by, 2);
        assert_eq!(standing(&s, 0).reached_by, 0);
    }

    /// A call graph's two closures overlap wherever there is a cycle, so the
    /// overlap is counted rather than left to make two honest numbers look like
    /// they should add up.
    #[test]
    fn what_is_counted_on_both_sides_is_named() {
        //  main → a ⇄ b
        let s = sheet(&["main", "a", "b"], &[(0, 1), (1, 2), (2, 1)], &[0]);
        let a = standing(&s, 1);
        assert_eq!(a.reaches, 1, "a reaches b");
        assert_eq!(a.reached_by, 2, "main and b reach a");
        assert_eq!(a.both_ways, 1, "and b is on both lists");
        assert_eq!(standing(&s, 0).both_ways, 0, "nothing reaches main");
    }

    /// Only functions have a standing. Asking about a crate is not an error, it
    /// is a question with no answer, and it comes back as zero rather than as a
    /// number a reader would believe.
    #[test]
    fn a_container_has_no_standing() {
        let mut s = sheet(&["main", "a"], &[(0, 1)], &[0]);
        s.units[1].kind = UnitKind::Crate;
        assert_eq!(standing(&s, 1), Standing::default());
        assert_eq!(standing(&s, 99), Standing::default(), "and neither has a stale id");
    }

    #[test]
    fn what_nothing_reaches_is_reported() {
        let s = sheet(&["main", "live", "dead"], &[(0, 1)], &[0]);
        let reach = analyse(&s);
        assert_eq!(reach.unreached, vec![2]);
    }

    /// A route reads like a stack trace: entry point first, target last.
    #[test]
    fn a_route_runs_from_the_beginning_to_the_target() {
        let s = sheet(
            &["main", "a", "b", "target"],
            &[(0, 1), (0, 2), (1, 3), (2, 3)],
            &[0],
        );
        let route = route_to(&s, 3).expect("there are two ways in; one of them will do");
        assert_eq!(route[0], 0, "a route starts at a beginning");
        assert_eq!(*route.last().unwrap(), 3, "and ends at what was asked for");
        for pair in route.windows(2) {
            assert!(s.units[pair[0]].calls.contains(&pair[1]), "and every step is a real call");
        }
    }

    /// Breadth-first backwards, so the route that comes back is the shortest one
    /// — the answer to "how does execution get here" rather than a tour.
    #[test]
    fn the_route_is_the_shortest_one() {
        //  main → short → target, and main → a → b → c → target
        let s = sheet(
            &["main", "short", "a", "b", "c", "target"],
            &[(0, 1), (1, 5), (0, 2), (2, 3), (3, 4), (4, 5)],
            &[0],
        );
        assert_eq!(route_to(&s, 5), Some(vec![0, 1, 5]));
    }

    #[test]
    fn a_cycle_does_not_hang_the_walk() {
        let s = sheet(
            &["main", "a", "b", "target"],
            &[(0, 1), (1, 2), (2, 1), (2, 3)],
            &[0],
        );
        assert_eq!(route_to(&s, 3), Some(vec![0, 1, 2, 3]));
    }

    #[test]
    fn a_beginning_is_its_own_route() {
        let s = sheet(&["main", "a"], &[(0, 1)], &[0]);
        assert_eq!(route_to(&s, 0), Some(vec![0]));
    }

    #[test]
    fn a_function_nothing_reaches_has_no_route() {
        let s = sheet(&["main", "orphan"], &[], &[0]);
        assert_eq!(route_to(&s, 1), None);
    }

    /// Chokepoints come back in the order a reader should read them.
    #[test]
    fn chokepoints_rank_by_how_much_they_dominate() {
        let s = sheet(
            &["main", "big", "small", "x", "y", "z"],
            &[(0, 1), (0, 2), (1, 3), (1, 4), (2, 5)],
            &[0],
        );
        let reach = analyse(&s);
        let ranked = reach.chokepoints(&s, 4);
        assert!(!ranked.is_empty());
        let big = ranked.iter().position(|&id| id == 1);
        let small = ranked.iter().position(|&id| id == 2);
        assert!(
            big < small,
            "the function standing over more of the program is read first"
        );
    }
}
