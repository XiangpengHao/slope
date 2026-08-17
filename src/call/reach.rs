//! Reachability: what runs, from where, through what.
//!
//! This is the module the whole lens turns on, and it exists because of a
//! measurement. This workspace has 240 functions of its own and 225 calls
//! between them — average degree below one, maximum fan-in six, six hops deep.
//! There is no mesh to draw. Drawing one spent the entire screen rendering
//! structure that carried almost no information.
//!
//! What the data *does* carry is routes. A call graph is entered at a few known
//! places and fans out, and the useful questions are all about that fan:
//!
//! - **Where does it start?** `main`, and whatever a framework calls.
//! - **What does each start reach?** The size of a beginning is what makes it
//!   worth reading first.
//! - **What must every route cross?** Not the most-called function — the ones
//!   that *dominate*: functions no route to a region can avoid. That set is the
//!   architecture, and it is computed here rather than asserted by a human.
//! - **How do I get from a start to this?** A path, written the way a stack
//!   trace is written, because that is the one representation of a call chain
//!   every developer already reads fluently.
//!
//! Nothing here returns a coordinate, and nothing here draws.

use std::collections::VecDeque;

use super::{Origin, Sheet, UnitKind};

/// A route from an entry point to a target, nearest first.
pub type Path = Vec<usize>;

/// What the analysis found about one function.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Standing {
    /// Functions this one can reach, itself excluded.
    pub reaches: usize,
    /// Functions that can reach it.
    pub reached_by: usize,
    /// How many functions this one *dominates*: every route from an entry point
    /// to any of them passes through here. This is the chokepoint weight, and
    /// it is what separates an architectural boundary from a popular utility.
    ///
    /// `Vec::len` has enormous fan-in and dominates nothing, because there is
    /// always another way round. A dispatcher has modest fan-in and dominates
    /// half the program, because there is not.
    pub dominates: usize,
    /// How many of the sheet's entry points reach it.
    pub entries: usize,
}

/// The whole reachability answer for a sheet, computed once.
#[derive(Debug, Clone, PartialEq)]
pub struct Reach {
    pub standing: Vec<Standing>,
    /// Immediate dominator of each function, where one exists.
    idom: Vec<Option<usize>>,
    /// Functions no entry point reaches. Dead, or reached only through a
    /// mechanism the analyser cannot see.
    pub unreached: Vec<usize>,
}

impl Reach {
    pub fn of(&self, id: usize) -> Standing {
        self.standing.get(id).copied().unwrap_or_default()
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

    /// Functions that can reach this one *and* be reached by it: the cycle it
    /// takes part in.
    ///
    /// The dependency board never has to say this, because a dependency graph
    /// is acyclic and its two closures are disjoint. A call graph's are not, so
    /// "12 callers" and "3 callees" can genuinely count the same function
    /// twice. Naming the overlap is the difference between two honest numbers
    /// and two that look like they should add up.
    pub fn both_ways(&self, sheet: &Sheet, id: usize) -> usize {
        let up = collect(sheet, id, Direction::Callers);
        let down = collect(sheet, id, Direction::Calls);
        up.intersection(&down).count()
    }

    /// The functions worth reading first: the ones that dominate the most.
    pub fn chokepoints(&self, sheet: &Sheet, want: usize) -> Vec<usize> {
        let mut ranked: Vec<usize> = sheet
            .units
            .iter()
            .filter(|u| {
                u.kind == UnitKind::Function
                    && u.origin == Origin::Workspace
                    && self.of(u.id).dominates > 0
            })
            .map(|u| u.id)
            .collect();
        ranked.sort_by_key(|&id| {
            (
                std::cmp::Reverse(self.of(id).dominates),
                sheet.units[id].flow,
                sheet.units[id].qualified.clone(),
            )
        });
        ranked.truncate(want);
        ranked
    }
}

/// Compute reach, dominance, and what nothing reaches.
pub fn analyse(sheet: &Sheet) -> Reach {
    let count = sheet.units.len();
    let entries = &sheet.entries;

    // --- Forward reach from each entry, and the union.
    let mut reached_from_any = vec![false; count];
    let mut entry_hits = vec![0usize; count];
    for &entry in entries {
        let mut seen = vec![false; count];
        let mut queue = VecDeque::from([entry]);
        seen[entry] = true;
        while let Some(id) = queue.pop_front() {
            reached_from_any[id] = true;
            for &next in &sheet.units[id].calls {
                if !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }
        for (id, &hit) in seen.iter().enumerate() {
            if hit {
                entry_hits[id] += 1;
            }
        }
    }

    // --- How much each function reaches, and is reached by. Computed per
    // function rather than by transitive closure of the whole graph: at 900
    // nodes a closure is 810k bits, and two breadth-first walks are cheaper to
    // write correctly than a bitset that has to stay in step with folding.
    let mut standing = vec![Standing::default(); count];
    for unit in &sheet.units {
        if unit.kind != UnitKind::Function {
            continue;
        }
        standing[unit.id].reaches = walk(sheet, unit.id, Direction::Calls);
        standing[unit.id].reached_by = walk(sheet, unit.id, Direction::Callers);
        standing[unit.id].entries = entry_hits[unit.id];
    }

    let idom = dominators(sheet, entries, count);
    for (id, &parent) in idom.iter().enumerate() {
        // Every function is dominated by its whole idom chain, so walking each
        // node's chain and crediting it upward gives the subtree size.
        if parent.is_none() {
            continue;
        }
        let mut here = id;
        let mut guard = 0;
        while let Some(next) = idom[here] {
            standing[next].dominates += 1;
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
            u.kind == UnitKind::Function && u.origin == Origin::Workspace && !reached_from_any[u.id]
        })
        .map(|u| u.id)
        .collect();

    Reach {
        standing,
        idom,
        unreached,
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Calls,
    Callers,
}

fn walk(sheet: &Sheet, from: usize, direction: Direction) -> usize {
    collect(sheet, from, direction).len()
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

/// Enumerate routes from an entry point down to a target, shortest first.
///
/// Written the way a stack trace is written, because that is the representation
/// of a call chain every developer already reads without being taught. Bounded:
/// a leaf utility is reachable thousands of ways and listing all of them
/// answers nothing, so the count is reported and the listing is capped.
pub fn paths_to(sheet: &Sheet, target: usize, want: usize) -> (Vec<Path>, usize) {
    if sheet.units[target].kind != UnitKind::Function {
        return (Vec::new(), 0);
    }
    // Breadth-first backwards from the target, keeping whole routes. Backwards
    // because the target is one node and the beginnings are many.
    let mut found: Vec<Path> = Vec::new();
    let mut total = 0usize;
    let mut queue: VecDeque<Path> = VecDeque::from([vec![target]]);
    let mut guard = 0usize;

    while let Some(route) = queue.pop_front() {
        guard += 1;
        if guard > 200_000 {
            break;
        }
        let head = *route.last().unwrap();
        if sheet.units[head].root.is_root() {
            total += 1;
            if found.len() < want {
                let mut ordered = route.clone();
                ordered.reverse();
                found.push(ordered);
            }
            continue;
        }
        if route.len() > 24 {
            continue;
        }
        for &caller in &sheet.units[head].callers {
            // A route never revisits a function: a cycle adds length, not
            // information, and a reader following a chain that loops learns
            // nothing except that the tool is lost.
            if route.contains(&caller) {
                continue;
            }
            let mut next = route.clone();
            next.push(caller);
            queue.push_back(next);
        }
    }
    (found, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call::{Call, Root, Unit};

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
            calls: edges
                .iter()
                .map(|&(from, to)| Call {
                    from,
                    to,
                    through_trait: None,
                })
                .collect(),
            roots: Vec::new(),
            entries: entries.to_vec(),
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
            reach.of(3).dominates >= 1,
            "gate is on every route to deep, so it dominates it"
        );
        assert_eq!(
            reach.of(5).dominates,
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
        let reach = analyse(&s);
        assert_eq!(reach.of(0).reaches, 2);
        assert_eq!(reach.of(2).reached_by, 2);
        assert_eq!(reach.of(0).reached_by, 0);
    }

    #[test]
    fn every_entry_that_reaches_a_function_is_counted() {
        let s = sheet(&["one", "two", "shared"], &[(0, 2), (1, 2)], &[0, 1]);
        let reach = analyse(&s);
        assert_eq!(reach.of(2).entries, 2, "both beginnings reach it");
        assert_eq!(reach.of(0).entries, 1, "an entry reaches itself and no other");
    }

    #[test]
    fn what_nothing_reaches_is_reported() {
        let s = sheet(&["main", "live", "dead"], &[(0, 1)], &[0]);
        let reach = analyse(&s);
        assert_eq!(reach.unreached, vec![2]);
    }

    /// Paths read like a stack trace: entry point first, target last.
    #[test]
    fn paths_run_from_the_beginning_to_the_target() {
        let s = sheet(
            &["main", "a", "b", "target"],
            &[(0, 1), (0, 2), (1, 3), (2, 3)],
            &[0],
        );
        let (paths, total) = paths_to(&s, 3, 10);
        assert_eq!(total, 2, "there are two ways in");
        assert_eq!(paths.len(), 2);
        for path in &paths {
            assert_eq!(path[0], 0, "a path starts at a beginning");
            assert_eq!(*path.last().unwrap(), 3, "and ends at what was asked for");
        }
    }

    #[test]
    fn a_cycle_does_not_make_infinite_paths() {
        let s = sheet(
            &["main", "a", "b", "target"],
            &[(0, 1), (1, 2), (2, 1), (2, 3)],
            &[0],
        );
        let (paths, total) = paths_to(&s, 3, 10);
        assert_eq!(total, 1, "the loop adds length, not routes");
        assert_eq!(paths[0], vec![0, 1, 2, 3]);
    }

    #[test]
    fn paths_are_capped_but_the_total_is_still_honest() {
        // A target reachable many ways: ten independent entries into one node.
        let mut names: Vec<String> = (0..10).map(|i| format!("entry{i}")).collect();
        names.push("target".into());
        let borrowed: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let edges: Vec<(usize, usize)> = (0..10).map(|i| (i, 10)).collect();
        let entries: Vec<usize> = (0..10).collect();
        let s = sheet(&borrowed, &edges, &entries);
        let (paths, total) = paths_to(&s, 10, 3);
        assert_eq!(paths.len(), 3, "the listing is capped");
        assert_eq!(total, 10, "the count is not");
    }

    #[test]
    fn a_function_nothing_reaches_has_no_paths() {
        let s = sheet(&["main", "orphan"], &[], &[0]);
        let (paths, total) = paths_to(&s, 1, 10);
        assert!(paths.is_empty());
        assert_eq!(total, 0);
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
