//! The call graph, and the sheet geometry derived from it.
//!
//! The second lens. Where the dependency board draws crates wired to crates,
//! this draws the code inside them: functions, the types and impl blocks they
//! sit in, the files those sit in, and the crates those sit in — with a length
//! of copper for every call.
//!
//! A call graph is a different shape from a dependency graph and the difference
//! decides the whole design:
//!
//! - **It has containment.** A dependency graph is flat; a call graph is nested
//!   four levels deep. That nesting is not decoration, it is the only thing that
//!   makes 1500 functions readable, so it is the sheet's primary structure.
//! - **It has cycles.** Two functions can call each other. Longest-path ranking
//!   assumes a DAG, so ranking here runs over the condensation of the strongly
//!   connected components rather than over the graph itself.
//! - **It leaves the workspace.** Nearly half of all calls land in `std`, and a
//!   third land in dependency crates. Those are units like any other, so they
//!   are drawn as units rather than thrown away.
//!
//! - **It has a beginning.** A dependency graph is entered anywhere; a program
//!   is entered at `main`, or at the public surface a library offers. That is
//!   the flow axis, and ordering by it is what turns a mesh of functions into
//!   something with a direction a reader can follow.
//!
//! What crosses the wire is the **tree and the calls**, and no geometry at all.
//! The drawing is measured on the client, against the current fold state, and
//! re-measured whenever the reader opens or closes a unit — because a folded
//! unit shrinks to a plate, so its neighbours genuinely do move.
//!
//! That is a departure from the dependency board's law that the world never
//! moves, taken deliberately. Folding that preserved a unit's footprint kept the
//! law and produced a drawing that was two-thirds hatching over reserved space
//! nobody could use. What replaces the law is weaker but real: the order is
//! fixed and knowable — entry points left, everything ranked rightward by hops
//! from them, at every level — and the reflow is animated from where things
//! were, so a fold reads as a movement rather than as a cut.

use serde::{Deserialize, Serialize};

// Extraction drives rust-analyzer over LSP, so it stays server-side.
#[cfg(not(target_arch = "wasm32"))]
pub mod extract;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;

// Reachability: what runs, from where, through what. This is where the lens's
// answers come from now that it answers in words rather than in a picture.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod reach;

/// What a unit is. These are the natural seams a Rust workspace already has —
/// not categories invented for the drawing — which is why they can carry the
/// whole navigation model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    /// A crate: one compilation unit.
    Crate,
    /// A file, which in Rust is a module.
    Module,
    /// A `struct` or `enum` together with nothing — the type's own declaration.
    Type,
    /// A `trait` declaration.
    Trait,
    /// An `impl` block. Carries a trait name when it is a trait impl, which is
    /// where every trait annotation on this sheet comes from.
    Impl,
    /// A function, method, or associated function. The only unit that calls.
    Function,
}

/// How execution gets into a function without being called from inside this
/// workspace — if it does at all.
///
/// Three different facts, named separately, because collapsing them is how a
/// program with one `main` came to report eighty-five beginnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Root {
    /// Not a beginning: something inside this workspace calls it.
    No,
    /// `fn main`. Where the program actually starts.
    Main,
    /// Public, and nothing inside this workspace calls it. A library's surface:
    /// real, useful as an API inventory, and *not* the same as execution
    /// starting here.
    Api,
    /// Nothing calls it and it is not public. A framework callback, something a
    /// macro invokes, or dead code — and the analysis cannot tell which.
    ///
    /// This is the honest home of the blind spot. On a Dioxus app eight of this
    /// workspace's own files land here, because a component is passed as a
    /// value and invoked by the framework, so no static call edge exists.
    Detached,
}

impl Root {
    pub fn is_root(self) -> bool {
        !matches!(self, Root::No)
    }

    pub fn noun(self) -> &'static str {
        match self {
            Root::No => "called internally",
            Root::Main => "main",
            Root::Api => "public API",
            Root::Detached => "no static caller",
        }
    }
}

/// Where a unit's source lives. This is the one property that decides whether
/// the reader can change it, so it is the one that decides what is drawn open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    /// A crate this workspace builds. The reader owns it and can change it.
    Workspace,
    /// A crate from the registry or a path dependency.
    Dependency,
    /// `std`, `core`, or `alloc`. Drawn as one unit and folded by default:
    /// nearly half of every call in a real workspace lands here, and
    /// `Vec::push` is not a relationship anybody reasons about.
    Std,
}

/// One unit on the sheet.
///
/// Carries no geometry: where a unit sits depends on what is folded, and that
/// is the client's question. See `layout::Layout`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub kind: UnitKind,
    /// The short name, as it appears in source.
    pub name: String,
    /// The full path a reader could paste into a search: `crate::mod::Type::fn`.
    pub qualified: String,
    /// The signature, for a function. Straight from rust-analyzer, so it is the
    /// real one rather than a reconstruction.
    pub signature: Option<String>,
    /// The trait this unit belongs to: the trait an `impl` block implements, or
    /// for a function, the trait whose method it is. This is the annotation the
    /// brief asked for, and it is read off the impl block rather than inferred.
    pub trait_name: Option<String>,
    /// The type an `impl` block is for.
    pub self_ty: Option<String>,
    pub origin: Origin,
    /// What kind of beginning this is, if any.
    ///
    /// Three separate facts used to share one label, and calling all of them
    /// "entry point" made this workspace report **85 places execution starts**
    /// when it has one. `Rect::contains` being public and unused internally is
    /// a true and useful thing to know; it is not a front door.
    pub root: Root,
    /// Hops from the nearest entry point. `u32::MAX` when nothing reaches it.
    ///
    /// This is the flow axis. Everything on the sheet is ordered by it at every
    /// level, so left-to-right means the same thing inside an impl block as it
    /// does across the whole drawing: further from where execution starts.
    pub flow: u32,
    /// The crate this unit is in, named once so a call can say where it landed
    /// without walking the tree.
    pub crate_name: String,
    /// Path as the reader would type it: workspace-relative, or
    /// `crate-1.2.3/src/lib.rs` for a dependency.
    pub file: String,
    pub line: u32,
    /// 0 for a crate, deeper for everything inside it.
    pub depth: u8,
    /// Functions this one calls, and functions that call it. Unit ids, and only
    /// ever populated on `Function` units.
    pub calls: Vec<usize>,
    pub callers: Vec<usize>,
    /// Every function at or below this unit. Precomputed because the lid needs
    /// to say how much is under it, and because the reader is choosing whether
    /// to open it on exactly that number.
    pub function_count: usize,
}

impl Unit {
}

/// One call, routed. From a function to a function, always — a container never
/// calls anything, it only contains things that do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub from: usize,
    pub to: usize,
    /// Set when the call goes through a trait method rather than to an
    /// inherent one. Drawn dashed, because a call through a trait is a call to
    /// whichever impl is selected — naming one body would be a lie.
    pub through_trait: Option<String>,
}

/// A dependency crate the extraction named but did not open, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unopened {
    pub crate_name: String,
    /// How many calls from this workspace land in it.
    pub calls: usize,
}

/// Everything the client needs to draw the sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub workspace: String,
    pub manifest_dir: String,
    pub units: Vec<Unit>,
    pub calls: Vec<Call>,
    /// Top-level units, in drawing order. Crates.
    pub roots: Vec<usize>,
    /// Every function execution can enter from outside, ordered `main` first,
    /// then by how much each one reaches. The answer to "where do I start".
    pub entries: Vec<usize>,
    // --- What the title block reports. Counted here so the chrome and the
    // sheet can never quote two different numbers for the same drawing.
    pub function_count: usize,
    pub call_count: usize,
    /// Functions left out because they are tests. Reported rather than silently
    /// dropped: a count that vanishes is a count nobody can check.
    pub tests_excluded: usize,
    pub workspace_crates: usize,
    pub dependency_crates: usize,
    /// Dependency crates named but not opened, because opening every one of
    /// them is the "too large" case the brief allowed for.
    pub unopened: Vec<Unopened>,
    /// Seconds the extraction took, so the reader knows what they waited for.
    pub took_ms: u64,
}

impl Sheet {}

/// The sheet is built for whichever workspace the tests run in, which is this
/// one — a real call graph out of a real analyser, not a fixture. A fixture
/// would agree with whatever this file believed on the day it was written;
/// rust-analyzer disagrees when the belief is wrong.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    /// Indexing costs about fifteen seconds, so every test shares one sheet.
    fn real_sheet() -> &'static Sheet {
        static SHEET: OnceLock<Sheet> = OnceLock::new();
        SHEET.get_or_init(|| extract::build().expect("rust-analyzer should read this workspace"))
    }

    fn analysis() -> &'static reach::Reach {
        static REACH: OnceLock<reach::Reach> = OnceLock::new();
        REACH.get_or_init(|| reach::analyse(real_sheet()))
    }

    fn find<'a>(sheet: &'a Sheet, kind: UnitKind, name: &str) -> Option<&'a Unit> {
        sheet.units.iter().find(|u| u.kind == kind && u.name == name)
    }

    fn mine(unit: &Unit) -> bool {
        unit.kind == UnitKind::Function && unit.origin == Origin::Workspace
    }

    /// The brief's first rule, and the one a reader would notice broken
    /// immediately: the call graph is of the program, and tests are not the
    /// program. These are real test names in this file's neighbours.
    #[test]
    fn no_test_function_is_on_the_sheet() {
        let sheet = real_sheet();
        for banned in [
            "ranks_move_forward_along_every_edge",
            "every_dependency_is_routed",
            "matches_a_linear_scan",
            "no_test_function_is_on_the_sheet",
        ] {
            assert!(
                find(sheet, UnitKind::Function, banned).is_none(),
                "{banned} is a test and must not be a unit"
            );
        }
        assert!(
            sheet.tests_excluded > 0,
            "this workspace has tests, so the count of what was excluded cannot be zero"
        );
    }

    /// And the other half of that rule: excluding tests must not exclude the
    /// program.
    #[test]
    fn the_workspaces_own_functions_are_on_the_sheet() {
        let sheet = real_sheet();
        // Named after the domain rather than after a module's current shape:
        // an assertion pinned to whatever a file happens to call its helpers
        // today fails on the next rename and says nothing about the extraction.
        for wanted in ["resolve", "immediate", "shortest_path_from_root", "analyse"] {
            assert!(
                find(sheet, UnitKind::Function, wanted).is_some(),
                "{wanted} is real code in this workspace and should be a unit"
            );
        }
        let mine = sheet
            .units
            .iter()
            .filter(|unit| unit.kind == UnitKind::Function && unit.origin == Origin::Workspace)
            .count();
        assert!(
            mine > 50,
            "only {mine} of this workspace's own functions reached the sheet"
        );
    }

    /// The edge this lens exists to record. Named by the property rather than by
    /// two specific functions: an assertion pinned to today's call sites is one
    /// that fails the next time the code is refactored, which says nothing about
    /// the extraction.
    #[test]
    fn a_real_call_between_two_files_is_recorded() {
        let sheet = real_sheet();
        let target = find(sheet, UnitKind::Function, "shortest_path_from_root")
            .expect("focus::shortest_path_from_root is real code in this workspace");
        let caller = sheet
            .units
            .iter()
            .find(|u| u.calls.contains(&target.id) && u.file != target.file)
            .expect("something outside graph/focus.rs calls it");
        assert!(
            target.callers.contains(&caller.id),
            "the edge has to be recorded from both ends"
        );
        assert!(caller.file != target.file, "and it crosses two files");
    }

    /// Trait annotation, which rust-analyzer hands over in the impl block's own
    /// name rather than being inferred.
    #[test]
    fn trait_impls_carry_their_trait() {
        let sheet = real_sheet();
        let default_impl = sheet
            .units
            .iter()
            .find(|u| {
                u.kind == UnitKind::Impl
                    && u.trait_name.as_deref() == Some("Default")
                    && u.origin == Origin::Workspace
                    && u.children
                        .iter()
                        .any(|&c| sheet.units[c].name == "default")
            })
            .expect("this workspace writes at least one `impl Default`");
        let method = default_impl
            .children
            .iter()
            .map(|&c| &sheet.units[c])
            .find(|u| u.name == "default")
            .expect("Default::default");
        assert_eq!(
            method.trait_name.as_deref(),
            Some("Default"),
            "a method inherits its impl block's trait, which is what lets a call \
             to it be reported as a call through a trait"
        );
    }

    /// Only functions call. A container that appeared to call something would
    /// be an edge with no source a reader could open.
    #[test]
    fn only_functions_have_edges() {
        let sheet = real_sheet();
        for unit in &sheet.units {
            if unit.kind != UnitKind::Function {
                assert!(unit.calls.is_empty() && unit.callers.is_empty());
            }
        }
        for call in &sheet.calls {
            assert_eq!(sheet.units[call.from].kind, UnitKind::Function);
            assert_eq!(sheet.units[call.to].kind, UnitKind::Function);
        }
    }

    /// The tree has to be a tree: dense ids, parents and children agreeing, and
    /// every unit reachable from a root.
    #[test]
    fn the_unit_tree_is_consistent() {
        let sheet = real_sheet();
        for (index, unit) in sheet.units.iter().enumerate() {
            assert_eq!(unit.id, index, "ids index the vector directly");
            for &child in &unit.children {
                assert_eq!(sheet.units[child].parent, Some(unit.id));
            }
            if let Some(parent) = unit.parent {
                assert!(sheet.units[parent].children.contains(&unit.id));
            }
        }
        let mut seen = vec![false; sheet.units.len()];
        let mut stack = sheet.roots.clone();
        while let Some(id) = stack.pop() {
            assert!(!seen[id], "the tree has a cycle");
            seen[id] = true;
            stack.extend(sheet.units[id].children.iter().copied());
        }
        assert!(seen.iter().all(|&s| s), "every unit hangs off a root");
    }

    /// Every edge is recorded from both ends, or a caller list and a callee
    /// list would disagree about the same call.
    #[test]
    fn every_call_is_recorded_from_both_ends() {
        let sheet = real_sheet();
        assert_eq!(sheet.calls.len(), sheet.call_count);
        assert!(sheet.call_count > 0, "this workspace calls its own functions");
        for call in &sheet.calls {
            assert!(
                sheet.units[call.from].calls.contains(&call.to),
                "{} does not list {} as a callee",
                sheet.units[call.from].qualified,
                sheet.units[call.to].qualified
            );
            assert!(
                sheet.units[call.to].callers.contains(&call.from),
                "{} does not list {} as a caller",
                sheet.units[call.to].qualified,
                sheet.units[call.from].qualified
            );
        }
    }

    /// The workspace boundary: a call that leaves lands on a named unit in a
    /// named crate, so "which dependency did this point at" always has an
    /// answer.
    #[test]
    fn calls_that_leave_the_workspace_land_somewhere_named() {
        let sheet = real_sheet();
        let leaving: Vec<&Call> = sheet
            .calls
            .iter()
            .filter(|c| {
                sheet.units[c.from].origin == Origin::Workspace
                    && sheet.units[c.to].origin != Origin::Workspace
            })
            .collect();
        assert!(!leaving.is_empty(), "this workspace calls its dependencies");
        for call in leaving {
            let target = &sheet.units[call.to];
            assert!(!target.name.is_empty());
            assert!(!target.crate_name.is_empty());
            assert!(!target.file.is_empty());
        }
        assert!(
            sheet.units.iter().any(|u| u.origin == Origin::Std),
            "std is where half of every call graph goes; it has to be here"
        );
    }

    #[test]
    fn dependency_interiors_are_unnested_at_least_one_hop() {
        let sheet = real_sheet();
        let inner = sheet
            .calls
            .iter()
            .filter(|c| sheet.units[c.from].origin == Origin::Dependency)
            .count();
        assert!(inner > 0, "a dependency function this workspace calls has callees of its own");
    }

    // ------------------------------------------------------------ beginnings

    /// The question a reader of unfamiliar code asks first, and the one this
    /// lens got badly wrong at first: it reported **85** places execution
    /// starts for a program with one `main`. Three different facts had been
    /// collapsed into one label.
    #[test]
    fn the_program_has_exactly_one_main_and_it_is_listed_first() {
        let sheet = real_sheet();
        let mains: Vec<&Unit> = sheet.units.iter().filter(|u| u.root == Root::Main).collect();
        assert_eq!(mains.len(), 1, "this workspace is one binary");
        assert_eq!(mains[0].name, "main");
        assert_eq!(
            sheet.entries.first().copied(),
            Some(mains[0].id),
            "and `main` leads the list a reader is meant to start from"
        );
    }

    /// The three buckets stay distinct, and nothing lands in two of them.
    #[test]
    fn the_ways_in_are_kept_apart() {
        let sheet = real_sheet();
        for unit in &sheet.units {
            if !mine(unit) {
                assert_eq!(unit.root, Root::No, "only the workspace's own code has a root kind");
                continue;
            }
            match unit.root {
                Root::No => assert!(
                    !unit.callers.is_empty(),
                    "{} has no root kind, so something must call it",
                    unit.qualified
                ),
                Root::Main => assert_eq!(unit.name, "main"),
                Root::Api | Root::Detached => assert!(
                    unit.callers.is_empty(),
                    "{} is listed as a way in but something calls it",
                    unit.qualified
                ),
            }
        }
        // The public/private split is the whole point of separating the two.
        for unit in sheet.units.iter().filter(|u| u.root == Root::Api) {
            assert!(
                unit.signature
                    .as_deref()
                    .is_some_and(|s| s.trim_start().starts_with("pub")),
                "{} is in the public bucket and is not public",
                unit.qualified
            );
        }
    }

    /// Flow is a real hop count from a beginning: anything a function calls is
    /// at most one step further out than the function itself.
    #[test]
    fn flow_depth_is_a_real_hop_count() {
        let sheet = real_sheet();
        assert!(
            sheet.units.iter().any(|u| mine(u) && u.flow != u32::MAX),
            "the beginnings reach something"
        );
        for unit in &sheet.units {
            if unit.flow == u32::MAX {
                continue;
            }
            for &callee in &unit.calls {
                assert!(
                    sheet.units[callee].flow <= unit.flow + 1,
                    "{} is {} hops out but calls {}, which is {}",
                    unit.qualified,
                    unit.flow,
                    sheet.units[callee].qualified,
                    sheet.units[callee].flow
                );
            }
        }
    }

    // ---------------------------------------------------------- what it says

    /// The claim the lens makes about chokepoints has to hold on the real
    /// graph: if it says nothing reaches `x` without crossing `c`, then
    /// deleting `c` really must cut `x` off from every beginning.
    ///
    /// Checked by re-running reachability with the chokepoint removed, which is
    /// the definition rather than a restatement of the implementation.
    #[test]
    fn a_chokepoint_really_is_unavoidable() {
        let sheet = real_sheet();
        let analysis = analysis();
        let mut checked = 0;

        for unit in sheet.units.iter().filter(|u| mine(u)) {
            let must_cross = analysis.spine_to(unit.id);
            let Some(&gate) = must_cross.last() else {
                continue;
            };
            if gate == unit.id {
                continue;
            }
            // Walk from every beginning without ever stepping on `gate`.
            let mut seen = vec![false; sheet.units.len()];
            let mut queue: std::collections::VecDeque<usize> = sheet
                .entries
                .iter()
                .copied()
                .filter(|&e| e != gate)
                .inspect(|&e| seen[e] = true)
                .collect();
            while let Some(id) = queue.pop_front() {
                for &next in &sheet.units[id].calls {
                    if next != gate && !seen[next] {
                        seen[next] = true;
                        queue.push_back(next);
                    }
                }
            }
            assert!(
                !seen[unit.id],
                "the lens claims every route to {} crosses {}, but there is a way round",
                unit.qualified,
                sheet.units[gate].qualified
            );
            checked += 1;
            if checked >= 40 {
                break;
            }
        }
        assert!(checked > 0, "this workspace has chokepoints to check");
    }

    /// Dominance is inevitability, not popularity — and the two really do
    /// disagree on the real graph, which is why the distinction is worth
    /// computing rather than approximating with a fan-in count.
    #[test]
    fn dominance_disagrees_with_popularity() {
        let sheet = real_sheet();
        let analysis = analysis();
        let popular = sheet
            .units
            .iter()
            .filter(|u| u.kind == UnitKind::Function)
            .max_by_key(|u| u.callers.len())
            .expect("some function is the most called");
        assert!(
            analysis.of(popular.id).dominates < analysis.of(popular.id).reached_by,
            "{} is the most-called function here; if it also dominated everything \
             that reaches it, fan-in would be a good enough proxy and this whole \
             computation would be unnecessary",
            popular.qualified
        );
    }

    /// Every route the lens prints is a real chain of calls, starts at a real
    /// beginning, and ends where the reader asked.
    #[test]
    fn every_printed_route_is_a_real_chain() {
        let sheet = real_sheet();
        let mut checked = 0;
        for unit in sheet.units.iter().filter(|u| mine(u)) {
            let (routes, total) = reach::paths_to(sheet, unit.id, 4);
            assert!(routes.len() <= total, "more routes listed than counted");
            for route in &routes {
                assert!(
                    sheet.units[route[0]].root.is_root(),
                    "a route starts where execution does"
                );
                assert_eq!(*route.last().unwrap(), unit.id, "and ends where asked");
                for pair in route.windows(2) {
                    assert!(
                        sheet.units[pair[0]].calls.contains(&pair[1]),
                        "{} does not actually call {}",
                        sheet.units[pair[0]].qualified,
                        sheet.units[pair[1]].qualified
                    );
                }
                let mut seen = route.clone();
                seen.sort_unstable();
                seen.dedup();
                assert_eq!(seen.len(), route.len(), "a route never revisits a function");
            }
            checked += 1;
            if checked >= 60 {
                break;
            }
        }
        assert!(checked > 0);
    }

    /// Anything reachable from a beginning has at least one route to print. A
    /// lens that says "reached by 12" and then lists no route has contradicted
    /// itself on screen.
    #[test]
    fn anything_reachable_has_a_route_to_show() {
        let sheet = real_sheet();
        let analysis = analysis();
        let mut checked = 0;
        for unit in sheet.units.iter().filter(|u| mine(u)) {
            if analysis.of(unit.id).entries == 0 {
                continue;
            }
            let (routes, total) = reach::paths_to(sheet, unit.id, 1);
            assert!(
                total > 0 && !routes.is_empty(),
                "{} is reachable from {} beginnings but the lens can show no route",
                unit.qualified,
                analysis.of(unit.id).entries
            );
            checked += 1;
            if checked >= 60 {
                break;
            }
        }
        assert!(checked > 0);
    }

    #[test]
    fn the_lens_reports_what_it_found() {
        let sheet = real_sheet();
        let analysis = analysis();
        let by_kind = |kind: UnitKind| sheet.units.iter().filter(|u| u.kind == kind).count();
        let by_root = |root: Root| sheet.units.iter().filter(|u| u.root == root).count();
        println!(
            "\nunits: {} crates, {} files, {} types, {} traits, {} impls, {} functions",
            by_kind(UnitKind::Crate),
            by_kind(UnitKind::Module),
            by_kind(UnitKind::Type),
            by_kind(UnitKind::Trait),
            by_kind(UnitKind::Impl),
            by_kind(UnitKind::Function),
        );
        println!(
            "ways in: {} main, {} public API, {} no static caller",
            by_root(Root::Main),
            by_root(Root::Api),
            by_root(Root::Detached),
        );
        println!(
            "{} calls; {} functions nothing reaches; read in {}ms",
            sheet.call_count,
            analysis.unreached.len(),
            sheet.took_ms,
        );
        println!("what every route crosses:");
        for id in analysis.chokepoints(sheet, 8) {
            println!(
                "   stands over {:>4}   {}",
                analysis.of(id).dominates,
                sheet.units[id].qualified
            );
        }
        println!("where to start:");
        for &id in sheet.entries.iter().take(6) {
            println!(
                "   reaches {:>4}   {}  [{}]",
                analysis.of(id).reaches,
                sheet.units[id].qualified,
                sheet.units[id].root.noun(),
            );
        }
    }
}
