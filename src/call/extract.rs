//! Building the call graph, by asking rust-analyzer.
//!
//! The shape of the run, and why it is shaped this way:
//!
//! 1. **Index once.** Starting the analyser and letting it index is nearly the
//!    whole cost — 8.4s of a 10s run on a 43k-line workspace. Everything after
//!    it is cheap, so this asks for the entire workspace up front rather than
//!    lazily per click.
//! 2. **Read the unit tree off `documentSymbol`.** The analyser already knows
//!    the crate/file/impl/function nesting and hands it over with the impl
//!    blocks named `impl Display for Palette`. Every trait annotation on this
//!    sheet is read from there rather than inferred.
//! 3. **Ask every function what it calls.** Requests go out in flights rather
//!    than one at a time; that is the difference between 1.1s and a minute.
//! 4. **Open the far side.** A call that leaves the workspace still lands on a
//!    real function in a real file, so the same treatment is applied there:
//!    the dependency's own units, its own impl blocks, its own traits.
//!
//! Tests are excluded, as the brief asks. Not by dropping them at the end —
//! by never walking into them, so a `#[cfg(test)] mod tests` contributes
//! neither nodes nor edges, and the count of what was left out is reported
//! rather than silently swallowed.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::lsp::{Analyzer, CallTarget, Position, Symbol, collect, parse_outgoing, path_of};
use super::{Origin, Root, Sheet, Unit, UnitKind, Unopened};
use crate::graph::metadata;

/// How many dependency functions to open before falling back to naming the
/// crate and stopping there. The brief allows exactly this: unnest by default,
/// and when that is too large, say which dependency it pointed to.
const DEPENDENCY_BUDGET: usize = 2500;

/// How many requests to keep in flight. rust-analyzer answers on a thread pool,
/// so this is throughput; past a couple of dozen it stops helping and starts
/// costing memory in the pending map.
const FLIGHT: usize = 48;

/// Directories whose contents are not the crate's own shipped code. `examples`
/// is included: an example is a consumer of the crate, not part of it, and
/// letting one in makes the crate look like it calls itself from outside.
const NOT_SOURCE: [&str; 5] = ["tests", "benches", "examples", "fuzz", "target"];

pub fn build() -> Result<Sheet, String> {
    let started = Instant::now();
    let (workspace, root, members) = metadata::members()?;
    let analyzer = Analyzer::start(&root)?;
    let mut builder = Builder::new(&root);

    // --- The workspace's own units.
    let mut open_files: Vec<(PathBuf, usize)> = Vec::new();
    for member in &members {
        let crate_id = builder.crate_unit(&member.name, Origin::Workspace);
        for path in source_files(&member.dir) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            analyzer.open(&path, &text)?;
            let symbols = analyzer.document_symbols(&path).unwrap_or_default();
            let module = builder.module_unit(crate_id, &path, &member.dir);
            let lines: Vec<&str> = text.lines().collect();
            builder.add_symbols(module, &symbols, &lines, false);
            open_files.push((path, module));
        }
    }
    if builder.functions_in(Origin::Workspace) == 0 {
        return Err(format!(
            "no functions found in {workspace}. This lens reads the workspace's own crates; \
             if the source lives somewhere other than `src/`, it is not being read."
        ));
    }

    // --- What every workspace function calls.
    let workspace_functions: Vec<usize> = builder.functions_with_origin(Origin::Workspace);
    let mut edges = ask_outgoing(&analyzer, &builder, &workspace_functions)?;

    // --- Open the far side of every call that left the workspace, so a
    // dependency is a real unit tree rather than a name on a label.
    let mut unresolved = builder.resolve_targets(&analyzer, &edges, DEPENDENCY_BUDGET);

    // --- One hop into the dependencies: what the thing you called calls next.
    // This is the "unnest" the brief asks for, bounded so a workspace that
    // touches fifty crates does not drag all of them in whole.
    let opened: Vec<usize> = builder.functions_with_origin(Origin::Dependency);
    if !opened.is_empty() {
        let inner = ask_outgoing(&analyzer, &builder, &opened)?;
        let more = builder.resolve_targets(&analyzer, &inner, 0);
        for (name, count) in more {
            *unresolved.entry(name).or_insert(0) += count;
        }
        edges.extend(inner);
    }

    // --- Record the edges on both sides.
    let mut calls: Vec<(usize, usize)> = Vec::new();
    for (from, targets) in &edges {
        for target in targets {
            if let Some(to) = builder.target_unit(target)
                && to != *from
            {
                calls.push((*from, to));
            }
        }
    }
    calls.sort_unstable();
    calls.dedup();

    let mut unopened: Vec<Unopened> = unresolved
        .into_iter()
        .map(|(crate_name, calls)| Unopened { crate_name, calls })
        .collect();
    unopened.sort_by(|a, b| b.calls.cmp(&a.calls).then_with(|| a.crate_name.cmp(&b.crate_name)));

    Ok(builder.finish(
        workspace,
        root.display().to_string(),
        calls,
        unopened,
        started.elapsed().as_millis() as u64,
    ))
}

/// Ask a batch of functions what they call, keeping several requests in flight.
fn ask_outgoing(
    analyzer: &Analyzer,
    builder: &Builder,
    functions: &[usize],
) -> Result<Vec<(usize, Vec<CallTarget>)>, String> {
    let mut out: Vec<(usize, Vec<CallTarget>)> = Vec::with_capacity(functions.len());
    for chunk in functions.chunks(FLIGHT) {
        // Step one: where is the call-hierarchy handle for each of these.
        let mut handles = Vec::with_capacity(chunk.len());
        for &id in chunk {
            let unit = &builder.units[id];
            let rx = analyzer.start_prepare(
                &builder.uris[id],
                Position {
                    line: unit.line.saturating_sub(1),
                    character: builder.columns[id],
                },
            )?;
            handles.push((id, rx));
        }
        let mut items = Vec::with_capacity(chunk.len());
        for (id, rx) in handles {
            // A function with no handle is a trait declaration with no body or
            // something behind a macro; there is nothing to ask, not an error.
            if let Ok(value) = collect(rx)
                && let Some(item) = value.as_array().and_then(|a| a.first().cloned())
            {
                items.push((id, item));
            }
        }
        // Step two: what does each handle call.
        let mut waiting = Vec::with_capacity(items.len());
        for (id, item) in &items {
            if let Ok(rx) = analyzer.start_outgoing(item) {
                waiting.push((*id, rx));
            }
        }
        for (id, rx) in waiting {
            if let Ok(value) = collect(rx) {
                let targets = parse_outgoing(&value);
                if !targets.is_empty() {
                    out.push((id, targets));
                }
            }
        }
    }
    Ok(out)
}

/// Every `.rs` file that is part of a crate's own shipped code.
fn source_files(crate_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&crate_dir.join("src"), &mut found);
    found.sort();
    found
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !NOT_SOURCE.contains(&name.as_ref()) && !name.starts_with('.') {
                walk(&path, found);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// Accumulates the unit tree while the analyser is answering.
struct Builder {
    units: Vec<Unit>,
    /// One URI per unit, so call hierarchy can be asked without rebuilding it.
    uris: Vec<String>,
    /// The column its name starts at, which is where the handle must be asked
    /// for — asking at the line's start lands on the `fn` keyword or an
    /// attribute and prepares nothing.
    columns: Vec<u32>,
    crates: HashMap<String, usize>,
    modules: HashMap<PathBuf, usize>,
    /// Function units by the position rust-analyzer reports for them, which is
    /// how a call target is matched back to a unit.
    at: HashMap<(PathBuf, u32, u32), usize>,
    /// Files already walked, so a dependency file shared by many calls is read
    /// once.
    seen_files: HashSet<PathBuf>,
    tests_excluded: usize,
    workspace_root: PathBuf,
}

impl Builder {
    fn new(root: &Path) -> Self {
        Self {
            units: Vec::new(),
            uris: Vec::new(),
            columns: Vec::new(),
            crates: HashMap::new(),
            modules: HashMap::new(),
            at: HashMap::new(),
            seen_files: HashSet::new(),
            tests_excluded: 0,
            workspace_root: root.to_path_buf(),
        }
    }

    fn push(&mut self, unit: Unit, uri: String, column: u32) -> usize {
        let id = unit.id;
        if let Some(parent) = unit.parent {
            self.units[parent].children.push(id);
        }
        self.units.push(unit);
        self.uris.push(uri);
        self.columns.push(column);
        id
    }

    fn crate_unit(&mut self, name: &str, origin: Origin) -> usize {
        if let Some(&id) = self.crates.get(name) {
            return id;
        }
        let id = self.units.len();
        self.crates.insert(name.to_string(), id);
        self.push(
            Unit {
                id,
                parent: None,
                children: Vec::new(),
                kind: UnitKind::Crate,
                name: name.to_string(),
                qualified: name.to_string(),
                signature: None,
                trait_name: None,
                self_ty: None,
                origin,
                root: Root::No,
                flow: u32::MAX,
                crate_name: name.to_string(),
                file: String::new(),
                line: 0,
                depth: 0,
                calls: Vec::new(),
                callers: Vec::new(),
                function_count: 0,
            },
            String::new(),
            0,
        )
    }

    fn module_unit(&mut self, crate_id: usize, path: &Path, crate_dir: &Path) -> usize {
        if let Some(&id) = self.modules.get(path) {
            return id;
        }
        let relative = path
            .strip_prefix(crate_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let display = path
            .strip_prefix(&self.workspace_root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| {
                format!("{}/{relative}", self.units[crate_id].crate_name)
            });
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative.clone());
        let crate_name = self.units[crate_id].crate_name.clone();
        let origin = self.units[crate_id].origin;
        let id = self.units.len();
        self.modules.insert(path.to_path_buf(), id);
        self.push(
            Unit {
                id,
                parent: Some(crate_id),
                children: Vec::new(),
                kind: UnitKind::Module,
                name,
                qualified: display.clone(),
                signature: None,
                trait_name: None,
                self_ty: None,
                origin,
                root: Root::No,
                flow: u32::MAX,
                crate_name,
                file: display,
                line: 0,
                depth: 1,
                calls: Vec::new(),
                callers: Vec::new(),
                function_count: 0,
            },
            super::lsp::uri_of(path),
            0,
        )
    }

    /// Walk a file's symbol tree into units, skipping everything under a test
    /// attribute.
    fn add_symbols(&mut self, parent: usize, symbols: &[Symbol], lines: &[&str], in_test: bool) {
        for symbol in symbols {
            let Some(kind) = unit_kind(symbol.kind) else {
                // Fields, constants, enum variants and type parameters are real
                // symbols but not units this sheet draws: none of them call.
                continue;
            };
            let is_test = in_test || is_test_attributed(lines, symbol.selection.line);
            if is_test {
                if kind == UnitKind::Function {
                    self.tests_excluded += 1;
                }
                // Counting what is under a test module needs the walk to
                // continue, but nothing under it becomes a unit.
                self.count_tests(&symbol.children, lines);
                continue;
            }

            let parent_unit = &self.units[parent];
            let (trait_name, self_ty) = if kind == UnitKind::Impl {
                parse_impl(&symbol.name)
            } else if kind == UnitKind::Function {
                // A method's trait is its impl block's trait; a method declared
                // inside a `trait` block belongs to that trait by definition.
                let inherited = match parent_unit.kind {
                    UnitKind::Impl => parent_unit.trait_name.clone(),
                    UnitKind::Trait => Some(parent_unit.name.clone()),
                    _ => None,
                };
                (inherited, None)
            } else {
                (None, None)
            };

            let file = parent_unit.file.clone();
            let crate_name = parent_unit.crate_name.clone();
            let origin = parent_unit.origin;
            let depth = parent_unit.depth.saturating_add(1);
            let qualified = if parent_unit.kind == UnitKind::Module {
                format!("{crate_name}::{}", symbol.name)
            } else {
                format!("{}::{}", parent_unit.qualified, symbol.name)
            };
            let uri = self.uris[parent].clone();

            let id = self.units.len();
            self.push(
                Unit {
                    id,
                    parent: Some(parent),
                    children: Vec::new(),
                    kind,
                    name: symbol.name.clone(),
                    qualified,
                    signature: symbol.detail.clone().filter(|_| kind == UnitKind::Function),
                    trait_name,
                    self_ty,
                    origin,
                    root: Root::No,
                    flow: u32::MAX,
                    crate_name,
                    file,
                    line: symbol.selection.line + 1,
                    depth,
                    calls: Vec::new(),
                    callers: Vec::new(),
                    function_count: 0,
                },
                uri.clone(),
                symbol.selection.character,
            );
            if kind == UnitKind::Function {
                self.at.insert(
                    (
                        path_of(&uri),
                        symbol.selection.line,
                        symbol.selection.character,
                    ),
                    id,
                );
            }
            // A helper written inside another function, or a type declared in
            // one, is a sibling of it rather than a thing inside it. To the
            // parser it nests; to a reader it is one more item in that file, and
            // to the drawing it has to be, because a function is a card the pane
            // never opens — anything genuinely nested under one would be a call
            // with nowhere to land the moment it was.
            let into = if kind == UnitKind::Function { parent } else { id };
            self.add_symbols(into, &symbol.children, lines, false);
        }
    }

    /// Everything under a test module is excluded but still counted, so the
    /// title block can say how much was left out.
    fn count_tests(&mut self, symbols: &[Symbol], lines: &[&str]) {
        for symbol in symbols {
            if unit_kind(symbol.kind) == Some(UnitKind::Function) {
                self.tests_excluded += 1;
            }
            self.count_tests(&symbol.children, lines);
        }
    }

    /// Open every file a call landed in that is not already known, so the far
    /// side of the call becomes a real unit tree.
    ///
    /// Returns the crates that were named but not opened, with how many calls
    /// reach each — the "too large" case, reported rather than hidden.
    fn resolve_targets(
        &mut self,
        analyzer: &Analyzer,
        edges: &[(usize, Vec<CallTarget>)],
        budget: usize,
    ) -> HashMap<String, usize> {
        let mut wanted: Vec<PathBuf> = Vec::new();
        let mut skipped: HashMap<String, usize> = HashMap::new();
        let mut budget_left = budget;

        for (_, targets) in edges {
            for target in targets {
                let path = path_of(&target.uri);
                if self.seen_files.contains(&path) || self.modules.contains_key(&path) {
                    continue;
                }
                let (origin, crate_name, _) = classify(&path, &self.workspace_root);
                // std is opened for its units but never spent from the budget:
                // it is half of every call graph and none of it is code the
                // reader can change.
                if origin == Origin::Dependency {
                    if budget_left == 0 {
                        *skipped.entry(crate_name).or_insert(0) += 1;
                        continue;
                    }
                    budget_left = budget_left.saturating_sub(1);
                }
                self.seen_files.insert(path.clone());
                wanted.push(path);
            }
        }

        wanted.sort();
        wanted.dedup();
        for path in wanted {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if analyzer.open(&path, &text).is_err() {
                continue;
            }
            let symbols = analyzer.document_symbols(&path).unwrap_or_default();
            let (origin, crate_name, crate_dir) = classify(&path, &self.workspace_root);
            let crate_id = self.crate_unit(&crate_name, origin);
            let module = self.module_unit(crate_id, &path, &crate_dir);
            let lines: Vec<&str> = text.lines().collect();
            self.add_symbols(module, &symbols, &lines, false);
        }
        skipped
    }

    fn target_unit(&self, target: &CallTarget) -> Option<usize> {
        let path = path_of(&target.uri);
        self.at
            .get(&(path, target.selection.line, target.selection.character))
            .copied()
    }

    fn functions_with_origin(&self, origin: Origin) -> Vec<usize> {
        self.units
            .iter()
            .filter(|u| u.kind == UnitKind::Function && u.origin == origin)
            .map(|u| u.id)
            .collect()
    }

    fn functions_in(&self, origin: Origin) -> usize {
        self.units
            .iter()
            .filter(|u| u.kind == UnitKind::Function && u.origin == origin)
            .count()
    }

    fn finish(
        mut self,
        workspace: String,
        manifest_dir: String,
        calls: Vec<(usize, usize)>,
        unopened: Vec<Unopened>,
        took_ms: u64,
    ) -> Sheet {
        for &(from, to) in &calls {
            self.units[from].calls.push(to);
            self.units[to].callers.push(from);
        }

        // A struct and the impl blocks on it are one thing to the reader and
        // two things to the parser. Put them back together before anything is
        // measured, or the brief's "structs" unit is an empty declaration and
        // every method hangs off the file instead.
        self.gather_impls_under_their_types();

        // A container that ended up holding nothing worth drawing is not a
        // unit, it is an empty box with a name. Prune bottom-up so a module
        // whose only content was tests disappears with them.
        let keep = self.prune();

        // How much is under each lid, which is the number the reader decides to
        // open a unit on. Counted over the tree, and only over what survived
        // the prune, so a lid never advertises functions that are not there.
        for root in self.roots() {
            self.count_functions(root, &keep);
        }

        let tests_excluded = self.tests_excluded;
        let (units, calls) = self.compact(keep, calls);

        let function_count = units
            .iter()
            .filter(|u| u.kind == UnitKind::Function)
            .count();
        let workspace_crates = units
            .iter()
            .filter(|u| u.kind == UnitKind::Crate && u.origin == Origin::Workspace)
            .count();
        let dependency_crates = units
            .iter()
            .filter(|u| u.kind == UnitKind::Crate && u.origin != Origin::Workspace)
            .count();
        let roots: Vec<usize> = units
            .iter()
            .filter(|u| u.parent.is_none())
            .map(|u| u.id)
            .collect();
        let call_count = calls.len();

        let mut sheet = Sheet {
            workspace,
            manifest_dir,
            units,
            roots,
            entries: Vec::new(),
            reach: super::reach::Reach::none(),
            function_count,
            call_count,
            tests_excluded,
            workspace_crates,
            dependency_crates,
            unopened,
            took_ms,
        };
        mark_entries(&mut sheet);
        // Dominance last, because it is measured from the beginnings and those
        // were only just decided. Computed here, once, because it is a question
        // about the whole graph — see `reach::Reach`.
        sheet.reach = super::reach::analyse(&sheet);
        sheet
    }

    /// Move each `impl` block inside the type it is for, when that type is
    /// declared in the same file.
    ///
    /// `struct Palette` and `impl Default for Palette` arrive as siblings,
    /// because that is how they are written. To a reader they are one unit —
    /// the type and everything implemented on it — and that unit is the one the
    /// brief named. An impl for a type declared elsewhere (`impl Trait for
    /// Vec<u8>`) has no local type to join and stays where it is.
    fn gather_impls_under_their_types(&mut self) {
        for container in 0..self.units.len() {
            let children = self.units[container].children.clone();
            if children.is_empty() {
                continue;
            }
            // Types declared right here, by their bare name.
            let types: HashMap<String, usize> = children
                .iter()
                .filter(|&&c| self.units[c].kind.is_type())
                .map(|&c| (base_type_name(&self.units[c].name), c))
                .collect();
            if types.is_empty() {
                continue;
            }
            let mut moved: Vec<(usize, usize)> = Vec::new();
            for &child in &children {
                if self.units[child].kind != UnitKind::Impl {
                    continue;
                }
                let Some(self_ty) = self.units[child].self_ty.clone() else {
                    continue;
                };
                if let Some(&type_id) = types.get(&base_type_name(&self_ty)) {
                    moved.push((child, type_id));
                }
            }
            for (child, type_id) in moved {
                self.units[container].children.retain(|&c| c != child);
                if self.units[child].trait_name.is_none() {
                    // An inherent `impl Foo` is not a unit anybody thinks in —
                    // it is where Foo's own methods are written. Drawing it as
                    // its own box nests `Layout` inside `Layout` and spends a
                    // level of the hierarchy saying nothing. Its methods move
                    // up into the type; a *trait* impl keeps its box, because
                    // "this type speaks Display" is a real thing to know.
                    let methods = self.units[child].children.clone();
                    for method in methods {
                        self.units[method].parent = Some(type_id);
                        self.units[type_id].children.push(method);
                    }
                    self.units[child].children.clear();
                    self.units[child].parent = None;
                    continue;
                }
                self.units[type_id].children.push(child);
                self.units[child].parent = Some(type_id);
                self.deepen(child);
            }
        }
    }

    /// An impl that moved one level in gained one level of depth, and so did
    /// everything under it.
    fn deepen(&mut self, id: usize) {
        self.units[id].depth = self.units[id].depth.saturating_add(1);
        for child in self.units[id].children.clone() {
            self.deepen(child);
        }
    }

    /// Which units survive.
    ///
    /// Every workspace function, because it is the reader's own code and a
    /// function nothing calls is a finding rather than noise. Outside the
    /// workspace, only functions that actually take part in a call: opening
    /// `core/src/option.rs` to find `Option::unwrap` also reveals four hundred
    /// other functions, and drawing all of them puts the reader's 171 functions
    /// in the corner of a sheet made mostly of `std`.
    ///
    /// Then every container with a surviving descendant, so a file whose only
    /// content was tests disappears along with them.
    ///
    /// Walked over the tree rather than over the id range: gathering impls
    /// under their types re-parents units, so a parent no longer reliably has a
    /// lower id than its children.
    fn prune(&self) -> Vec<bool> {
        let mut keep = vec![false; self.units.len()];
        for &root in &self.roots() {
            self.prune_from(root, &mut keep);
        }
        keep
    }

    fn prune_from(&self, id: usize, keep: &mut [bool]) -> bool {
        let unit = &self.units[id];
        let mut survives = if unit.kind == UnitKind::Function {
            unit.origin == Origin::Workspace || !unit.calls.is_empty() || !unit.callers.is_empty()
        } else {
            false
        };
        for &child in &unit.children {
            survives |= self.prune_from(child, keep);
        }
        keep[id] = survives;
        survives
    }

    fn roots(&self) -> Vec<usize> {
        self.units
            .iter()
            .filter(|u| u.parent.is_none())
            .map(|u| u.id)
            .collect()
    }

    fn count_functions(&mut self, id: usize, keep: &[bool]) -> usize {
        let mine = usize::from(self.units[id].kind == UnitKind::Function && keep[id]);
        let mut total = mine;
        for child in self.units[id].children.clone() {
            if keep[child] {
                total += self.count_functions(child, keep);
            }
        }
        self.units[id].function_count = total;
        total
    }

    /// Drop the pruned units and renumber, so ids stay dense and index directly
    /// into the vector the client receives.
    fn compact(
        self,
        keep: Vec<bool>,
        calls: Vec<(usize, usize)>,
    ) -> (Vec<Unit>, Vec<(usize, usize)>) {
        let mut remap = vec![usize::MAX; self.units.len()];
        let mut next = 0usize;
        for (id, &alive) in keep.iter().enumerate() {
            if alive {
                remap[id] = next;
                next += 1;
            }
        }
        let units: Vec<Unit> = self
            .units
            .into_iter()
            .zip(keep.iter().copied())
            .filter(|&(_, alive)| alive)
            .map(|(mut unit, _)| {
                unit.id = remap[unit.id];
                unit.parent = unit.parent.filter(|&p| remap[p] != usize::MAX).map(|p| remap[p]);
                unit.children.retain(|&c| remap[c] != usize::MAX);
                for child in &mut unit.children {
                    *child = remap[*child];
                }
                unit.calls.retain(|&c| remap[c] != usize::MAX);
                for call in &mut unit.calls {
                    *call = remap[*call];
                }
                unit.callers.retain(|&c| remap[c] != usize::MAX);
                for caller in &mut unit.callers {
                    *caller = remap[*caller];
                }
                unit
            })
            .collect();
        let calls = calls
            .into_iter()
            .filter(|&(a, b)| remap[a] != usize::MAX && remap[b] != usize::MAX)
            .map(|(a, b)| (remap[a], remap[b]))
            .collect();
        (units, calls)
    }
}

/// Which LSP symbol kinds are units this sheet draws.
fn unit_kind(kind: u8) -> Option<UnitKind> {
    match kind {
        2 => Some(UnitKind::Module),
        // rust-analyzer names a struct `Struct` and an enum `Enum`, so the two
        // are kept apart rather than flattened into one "type": which one it is
        // changes how a reader reads the methods hanging off it.
        5 | 23 => Some(UnitKind::Struct),
        10 => Some(UnitKind::Enum),
        11 => Some(UnitKind::Trait),
        19 => Some(UnitKind::Impl),
        6 | 12 => Some(UnitKind::Function),
        _ => None,
    }
}

/// Is this item behind a test attribute?
///
/// Anchored on the line the item's **name** sits on, not on the start of its
/// range. rust-analyzer's range already includes the item's attributes, so
/// walking up from the range start begins *below* the `#[cfg(test)]` and finds
/// nothing — which is how a whole `mod tests` came through the first time.
///
/// Matches `test` as a whole word, so `#[cfg(test)]`, `#[test]` and
/// `#[tokio::test]` all count while `#[serde(rename = "latest")]` does not.
fn is_test_attributed(lines: &[&str], name_line: u32) -> bool {
    // `#[cfg(test)] mod tests {` puts the attribute on the item's own line.
    if let Some(line) = lines.get(name_line as usize) {
        let text = line.trim();
        if text.starts_with("#[") && !text.contains("not(test") && has_word_test(text) {
            return true;
        }
    }
    let mut index = name_line as i64 - 1;
    while index >= 0 {
        let Some(line) = lines.get(index as usize) else {
            break;
        };
        let text = line.trim();
        if text.is_empty() || text.starts_with("//") {
            index -= 1;
            continue;
        }
        if !text.starts_with("#[") && !text.starts_with("#!") {
            break;
        }
        // `#[cfg(not(test))]` marks code that exists in the real build, which
        // is exactly the code this lens is for.
        if !text.contains("not(test") && has_word_test(text) {
            return true;
        }
        index -= 1;
    }
    false
}

fn has_word_test(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(found) = text[from..].find("test") {
        let at = from + found;
        let before_ok = at == 0 || !bytes[at - 1].is_ascii_alphanumeric() && bytes[at - 1] != b'_';
        let after = at + 4;
        let after_ok = after >= bytes.len()
            || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
        if before_ok && after_ok {
            return true;
        }
        from = at + 4;
    }
    false
}

/// Split `impl Display for Palette` into the trait it implements and the type
/// it is for. Generic parameter lists are dropped from the `impl` itself but
/// kept on the type, because `Map<T>` and `Map` are different labels.
fn parse_impl(name: &str) -> (Option<String>, Option<String>) {
    let rest = name.trim().strip_prefix("impl").unwrap_or(name).trim_start();
    // `impl<T: Clone>` — skip the parameter list, balanced.
    let rest = if rest.starts_with('<') {
        let mut depth = 0i32;
        let mut end = 0usize;
        for (index, character) in rest.char_indices() {
            match character {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = index + character.len_utf8();
                        break;
                    }
                }
                _ => {}
            }
        }
        rest[end..].trim_start()
    } else {
        rest
    };

    // Split on ` for ` at angle-bracket depth zero, so
    // `From<Box<dyn Error>> for X` splits in the right place.
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'<' | b'(' => depth += 1,
            b'>' | b')' => depth -= 1,
            b' ' if depth == 0 && rest[index..].starts_with(" for ") => {
                let tr = rest[..index].trim();
                let ty = rest[index + 5..].trim();
                return (
                    (!tr.is_empty()).then(|| tr.trim_start_matches('!').trim().to_string()),
                    (!ty.is_empty()).then(|| ty.to_string()),
                );
            }
            _ => {}
        }
        index += 1;
    }
    let ty = rest.trim();
    (None, (!ty.is_empty()).then(|| ty.to_string()))
}

/// Where a file's crate lives, and what to call it.
fn classify(path: &Path, workspace_root: &Path) -> (Origin, String, PathBuf) {
    let text = path.to_string_lossy();

    // The standard library, out of the toolchain's own source.
    if let Some(at) = text.find("/lib/rustlib/src/rust/library/") {
        let after = &text[at + "/lib/rustlib/src/rust/library/".len()..];
        let name = after.split('/').next().unwrap_or("std").to_string();
        let root = PathBuf::from(&text[..at + "/lib/rustlib/src/rust/library/".len()]).join(&name);
        return (Origin::Std, name, root);
    }
    // A registry crate: `.../registry/src/<index>/<name>-<version>/...`
    if let Some(at) = text.find("/registry/src/") {
        let after = &text[at + "/registry/src/".len()..];
        let mut parts = after.splitn(3, '/');
        let index = parts.next().unwrap_or("");
        if let Some(stamped) = parts.next() {
            let name = crate_name_of(stamped);
            let root = PathBuf::from(&text[..at + "/registry/src/".len()])
                .join(index)
                .join(stamped);
            return (Origin::Dependency, name, root);
        }
    }
    // A git dependency: `.../git/checkouts/<name>-<hash>/<rev>/...`
    if let Some(at) = text.find("/git/checkouts/") {
        let after = &text[at + "/git/checkouts/".len()..];
        let mut parts = after.splitn(3, '/');
        if let (Some(stamped), Some(rev)) = (parts.next(), parts.next()) {
            let name = crate_name_of(stamped);
            let root = PathBuf::from(&text[..at + "/git/checkouts/".len()])
                .join(stamped)
                .join(rev);
            return (Origin::Dependency, name, root);
        }
    }
    // A path dependency outside the workspace, or anything else on disk. The
    // directory above `src` is the best name available.
    let root = path
        .ancestors()
        .find(|a| a.join("Cargo.toml").exists())
        .unwrap_or(workspace_root)
        .to_path_buf();
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());
    (Origin::Dependency, name, root)
}

/// Classify every way execution can get in, and measure everything from there.
///
/// The first version called all of these "entry points" and reported eighty-five
/// of them for a program with one `main`. They are three different facts and a
/// reader needs them apart:
///
/// - **`main`** is where the program starts. There is one.
/// - **Public and uncalled internally** is a library's surface. Useful as an
///   API inventory; not a claim about execution.
/// - **Uncalled and not public** is a framework callback, something a macro
///   invokes, or dead code — and static analysis genuinely cannot tell which.
///   Naming this bucket honestly is what lets the lens admit its blind spot
///   instead of rendering it as a fact about the code.
///
/// All three seed the flow measurement, because all three really are places
/// execution can begin. Only the naming is separated.
fn mark_entries(sheet: &mut Sheet) {
    for index in 0..sheet.units.len() {
        let unit = &sheet.units[index];
        if unit.kind != UnitKind::Function || unit.origin != Origin::Workspace {
            continue;
        }
        let public = unit
            .signature
            .as_deref()
            .is_some_and(|s| s.trim_start().starts_with("pub"));
        let root = if unit.name == "main" {
            Root::Main
        } else if !unit.callers.is_empty() {
            Root::No
        } else if public {
            Root::Api
        } else {
            Root::Detached
        };
        sheet.units[index].root = root;
    }

    let seeds: Vec<usize> = sheet
        .units
        .iter()
        .filter(|u| u.root.is_root())
        .map(|u| u.id)
        .collect();
    flow_from(sheet, &seeds);

    // A container is as near the beginning as the nearest thing inside it.
    for root in sheet.roots.clone() {
        settle_flow(sheet, root);
    }

    // `main` first, then whatever reaches the most: the order a reader should
    // work down, rather than alphabetical or whatever order the files were in.
    let mut ordered = seeds;
    let reachable: Vec<usize> = ordered
        .iter()
        .map(|&id| forward_reach(sheet, id))
        .collect();
    let weight: std::collections::HashMap<usize, usize> =
        ordered.iter().copied().zip(reachable).collect();
    ordered.sort_by_key(|&id| {
        let unit = &sheet.units[id];
        (
            match unit.root {
                Root::Main => 0,
                Root::Api => 1,
                _ => 2,
            },
            std::cmp::Reverse(weight.get(&id).copied().unwrap_or(0)),
            unit.qualified.clone(),
        )
    });
    sheet.entries = ordered;
}

/// How many functions this one can reach. What makes a beginning worth reading
/// first is how much of the program is behind it.
fn forward_reach(sheet: &Sheet, from: usize) -> usize {
    let mut seen = std::collections::HashSet::from([from]);
    let mut queue = std::collections::VecDeque::from([from]);
    while let Some(id) = queue.pop_front() {
        for &next in &sheet.units[id].calls {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen.len() - 1
}

/// Breadth-first hop count from a set of beginnings.
fn flow_from(sheet: &mut Sheet, seeds: &[usize]) {
    for unit in &mut sheet.units {
        unit.flow = u32::MAX;
    }
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for &id in seeds {
        sheet.units[id].flow = 0;
        queue.push_back(id);
    }
    while let Some(id) = queue.pop_front() {
        let here = sheet.units[id].flow;
        for index in 0..sheet.units[id].calls.len() {
            let next = sheet.units[id].calls[index];
            if sheet.units[next].flow > here + 1 {
                sheet.units[next].flow = here + 1;
                queue.push_back(next);
            }
        }
    }
}

fn settle_flow(sheet: &mut Sheet, id: usize) -> u32 {
    let mut nearest = sheet.units[id].flow;
    for index in 0..sheet.units[id].children.len() {
        let child = sheet.units[id].children[index];
        nearest = nearest.min(settle_flow(sheet, child));
    }
    sheet.units[id].flow = nearest;
    nearest
}

/// The bare name of a type, for matching an `impl` to the type it is for.
/// `Map<T>` and `crate::Map` are both `Map`; `&mut Palette` is `Palette`.
fn base_type_name(text: &str) -> String {
    let text = text.trim().trim_start_matches(['&', '*']).trim();
    let text = text.trim_start_matches("mut ").trim_start_matches("dyn ").trim();
    let head = text.split('<').next().unwrap_or(text).trim();
    head.rsplit("::").next().unwrap_or(head).trim().to_string()
}

/// `arrow-array-58.3.0` is the crate `arrow-array`.
///
/// The version starts at the *first* `-` followed by a digit, not the last: a
/// semver build metadata suffix can carry its own dashes, so
/// `wasi-0.11.0+wasi-snapshot` cut from the right yields the whole string back.
fn crate_name_of(stamped: &str) -> String {
    let bytes = stamped.as_bytes();
    for (at, &byte) in bytes.iter().enumerate() {
        if byte == b'-'
            && bytes
                .get(at + 1)
                .is_some_and(|next| next.is_ascii_digit())
        {
            return stamped[..at].to_string();
        }
    }
    stamped.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impl_headers_split_into_trait_and_type() {
        assert_eq!(
            parse_impl("impl Display for Palette"),
            (Some("Display".into()), Some("Palette".into()))
        );
        assert_eq!(parse_impl("impl Palette"), (None, Some("Palette".into())));
        assert_eq!(
            parse_impl("impl<T: Clone> Iterator for Map<T>"),
            (Some("Iterator".into()), Some("Map<T>".into()))
        );
        // The ` for ` inside the trait's own generics must not split it.
        assert_eq!(
            parse_impl("impl From<Box<dyn Error>> for Failure"),
            (Some("From<Box<dyn Error>>".into()), Some("Failure".into()))
        );
        assert_eq!(
            parse_impl("impl<'a> Deserialize<'a> for Board"),
            (Some("Deserialize<'a>".into()), Some("Board".into()))
        );
    }

    #[test]
    fn crate_names_lose_their_version_and_keep_their_dashes() {
        assert_eq!(crate_name_of("arrow-array-58.3.0"), "arrow-array");
        assert_eq!(crate_name_of("serde-1.0.151"), "serde");
        assert_eq!(crate_name_of("wasi-0.11.0+wasi-snapshot"), "wasi");
        // A git checkout stamped with a hash rather than a version.
        assert_eq!(crate_name_of("my-crate-a1b2c3d"), "my-crate-a1b2c3d");
    }

    /// The rule the brief set: tests are not in the call graph. This is the
    /// check that a crate using `latest` in a serde attribute does not lose its
    /// functions to a substring match.
    #[test]
    fn test_attributes_are_recognised_and_lookalikes_are_not() {
        let lines = vec![
            "#[test]",                            // 0
            "fn a() {}",                          // 1
            "#[cfg(test)]",                       // 2
            "mod b {}",                           // 3
            "#[tokio::test]",                     // 4
            "async fn c() {}",                    // 5
            "#[serde(rename = \"latest\")]",      // 6
            "fn d() {}",                          // 7
            "#[cfg(not(test))]",                  // 8
            "fn e() {}",                          // 9
            "/// docs",                           // 10
            "#[test]",                            // 11
            "",                                   // 12
            "fn f() {}",                          // 13
            "fn g() {}",                          // 14
            "#[derive(Debug)]",                   // 15
            "struct H;",                          // 16
            "#[cfg(test)] mod inline {}",         // 17
        ];
        assert!(is_test_attributed(&lines, 1), "#[test] marks a test");
        assert!(is_test_attributed(&lines, 3), "#[cfg(test)] marks a test");
        assert!(is_test_attributed(&lines, 5), "#[tokio::test] marks a test");
        assert!(
            !is_test_attributed(&lines, 7),
            "\"latest\" inside an attribute is not the word test"
        );
        assert!(
            !is_test_attributed(&lines, 9),
            "#[cfg(not(test))] is code that ships"
        );
        assert!(
            is_test_attributed(&lines, 13),
            "a blank line between the attribute and the item does not hide it"
        );
        assert!(!is_test_attributed(&lines, 14), "a bare fn is not a test");
        assert!(!is_test_attributed(&lines, 16), "#[derive] is not a test");
        assert!(
            is_test_attributed(&lines, 17),
            "an attribute on the item's own line still marks it"
        );
    }

    #[test]
    fn word_test_matches_only_whole_words() {
        assert!(has_word_test("#[cfg(test)]"));
        assert!(has_word_test("#[test]"));
        assert!(has_word_test("#[tokio::test]"));
        assert!(!has_word_test("#[serde(rename = \"latest\")]"));
        assert!(!has_word_test("#[doc = \"tested\"]"));
        assert!(!has_word_test("#[attr(testing)]"));
    }

    #[test]
    fn symbol_kinds_map_to_the_units_the_brief_named() {
        assert_eq!(unit_kind(12), Some(UnitKind::Function));
        assert_eq!(unit_kind(6), Some(UnitKind::Function));
        assert_eq!(unit_kind(23), Some(UnitKind::Struct));
        assert_eq!(unit_kind(10), Some(UnitKind::Enum));
        assert_eq!(unit_kind(11), Some(UnitKind::Trait));
        assert_eq!(unit_kind(19), Some(UnitKind::Impl));
        assert_eq!(unit_kind(2), Some(UnitKind::Module));
        // Fields and constants are symbols but not units: neither calls.
        assert_eq!(unit_kind(8), None);
        assert_eq!(unit_kind(14), None);
    }

    #[test]
    fn paths_classify_to_the_right_origin_and_crate() {
        let root = Path::new("/home/user/ws");
        let (origin, name, _) = classify(
            Path::new("/nix/store/x/lib/rustlib/src/rust/library/core/src/option.rs"),
            root,
        );
        assert_eq!(origin, Origin::Std);
        assert_eq!(name, "core");

        let (origin, name, crate_root) = classify(
            Path::new("/home/u/.cargo/registry/src/index.crates.io-1949/arrow-array-58.3.0/src/lib.rs"),
            root,
        );
        assert_eq!(origin, Origin::Dependency);
        assert_eq!(name, "arrow-array");
        assert!(crate_root.ends_with("arrow-array-58.3.0"));
    }
}
