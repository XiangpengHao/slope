//! What the function chart reads out of the survey: the code that runs, tiered
//! by how far it is from something that starts.
//!
//! The rung above asks what the workspace *keeps*; this one asks what it
//! *does*. Its marks are the declarations that run — every function, every
//! method, every trait clause a method answers, every `macro_rules!` — and its
//! one organizing move is the **call depth**. An **entry point** is a
//! declaration nothing in the workspace calls: `main`, a server function the
//! client reaches through generated code, a component the router mounts, a
//! method answering a foreign trait's contract. Everything else is some number
//! of calls away from the nearest one, and that number is where it sits on the
//! paper.
//!
//! Two families of ink run between the marks, and only two.
//!
//! * **Calls** — the solid family. At this altitude a body *is* the
//!   declaration: a struct's fields are its shape, and a function's calls are
//!   its shape, so what would be body coupling one rung up is structure here.
//! * **Contracts** — dashed and lighter: a trait's own method clause, and the
//!   methods that answer it. A call graph on its own lies about a trait-heavy
//!   workspace, because a `dyn` call lands on the clause and the code that runs
//!   is somewhere else entirely. The dashed family is what carries reachability
//!   across that gap, so a method answering a workspace trait is *not* an entry
//!   point: what calls the promise calls it.
//!
//! Types have no block here — this chart draws behavior — so what a function
//! touches is counted in its hover words and spent back out as rows on its
//! sheet, each one a link down to the data altitude that does draw it.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::data::{
    CodeGraph, DeclRow, Delta, HoldEdge, ImplEdge, ItemKind, ItemMark, MarkRef, MethodRow, Vis,
};
use crate::views::chrome::plural;
use crate::views::data::model::module_path;
use crate::views::func::{CallDir, FnReading, Group};

/// Where a mark stands in the running order — this chart's one verdict.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tier {
    /// Nothing in the workspace calls it, and no contract it answers is
    /// called: this is where a chain of running starts.
    Entry,
    /// This many calls from the nearest entry point, by the shortest way in.
    Deep(u32),
    /// In a ring of calls no entry point reaches. Not dead — the survey cannot
    /// see every caller — but nothing on this paper starts it.
    Ring,
}

impl Tier {
    /// The band this mark sits in: entries first, then one band per depth,
    /// with the unreached ring last. The number is an index, not a depth.
    pub(super) fn band(self, deepest: u32) -> u32 {
        match self {
            Tier::Entry => 0,
            Tier::Deep(n) => n,
            Tier::Ring => deepest + 1,
        }
    }

    /// The band's own caption, in the plate's plain vocabulary.
    pub(super) fn caption(self) -> String {
        match self {
            Tier::Entry => "entry".to_string(),
            Tier::Deep(1) => "1 call deep".to_string(),
            Tier::Deep(n) => format!("{n} calls deep"),
            Tier::Ring => "in a call ring".to_string(),
        }
    }

    /// What the mark's own hover words say about where it stands.
    pub(super) fn words(self) -> String {
        match self {
            Tier::Entry => "an entry point — nothing in the workspace calls it".to_string(),
            Tier::Deep(1) => "1 call from an entry point".to_string(),
            Tier::Deep(n) => format!("{n} calls from an entry point"),
            Tier::Ring => "in a call ring — no entry point reaches it".to_string(),
        }
    }
}

/// What kind of ink runs between two marks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum CallKind {
    /// One body names the other: a call, or a function taken as a value.
    /// Solid.
    Call,
    /// A trait's method clause, and the method that answers it. Dashed and
    /// lighter: a promise is not a call, and what runs is decided elsewhere.
    Answers,
}

/// One drawn relation. Drawn the way change travels, as every altitude draws
/// it: from the end being leaned on to the end that leans, so the arrowhead
/// rests on the dependent — the caller of a call, the answering method of a
/// contract.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Call {
    /// The end being leaned on: the callee, or the trait's clause.
    pub(super) def: u32,
    /// The end that leans: the caller, or the method that answers.
    pub(super) user: u32,
    pub(super) kind: CallKind,
    /// References the survey resolved for this pair. A contract has none: it
    /// is one promise, not a count of them.
    pub(super) count: u32,
    /// Whether the resting plate draws it.
    ///
    /// The resting reading is the **way in**: for every mark, the one call
    /// that put it at its depth — the shortest way something that starts
    /// reaches it. That is a tree, one wire per mark, and it is the whole of
    /// "what runs from where"; drawing all fifteen hundred resolved calls at
    /// rest would be the hairball this system forbids one rung up. Every
    /// other call stays in the set folded, and inks back in on hover or
    /// selection of either end. A contract wire never folds: it is what makes
    /// the tree honest about a `dyn` call. Nothing else earns a place at rest
    /// — the diff cannot mark a call (it reads declarations, not bodies), and
    /// un-folding every wire that merely touched a changed declaration washed
    /// a large change's whole sheet.
    pub(super) rest: bool,
}

/// One row of a function's quoted signature: a receiver, a parameter, or the
/// return. Nothing here is reconstructed — every row is the source's own text.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct SigRow {
    /// The parameter's binding as written (`graph`, `mut at`, `_`), the whole
    /// receiver where it is one (`&mut self`), and empty for the return.
    pub(super) name: String,
    /// The declared type as written; empty on a receiver, which declares none.
    pub(super) ty: String,
    /// The arrow in front of the return type, so the row reads as rust writes
    /// it.
    pub(super) returns: bool,
    /// Whether the diff added this row since the base.
    pub(super) added: bool,
}

impl SigRow {
    /// The row as the block draws it. One string, so measuring a row and
    /// drawing it can never disagree.
    pub(super) fn written(&self) -> String {
        match (self.returns, self.ty.is_empty()) {
            (true, _) => format!("-> {}", self.ty),
            (false, true) => self.name.clone(),
            (false, false) => format!("{}: {}", self.name, self.ty),
        }
    }

    /// One declaration's signature, quoted row by row: the receiver a method
    /// takes, every parameter as the source writes it, and the return type
    /// under them. A function's parameters are its fields — the same
    /// quotation the data altitude draws for a struct, of the other half of
    /// the language.
    fn quote(item: &ItemMark) -> Vec<SigRow> {
        let added: HashSet<u32> = item.diff.fields_added.iter().copied().collect();
        let mut rows: Vec<SigRow> = item
            .body
            .field_rows
            .iter()
            .enumerate()
            .map(|(at, DeclRow { name, ty, .. })| SigRow {
                name: name.clone(),
                ty: ty.clone(),
                returns: false,
                added: added.contains(&(at as u32)),
            })
            .collect();
        if !item.body.ty.is_empty() {
            rows.push(SigRow {
                name: String::new(),
                ty: item.body.ty.clone(),
                returns: true,
                added: false,
            });
        }
        rows
    }
}

/// What a declaration's head says, and where a reader goes to read the source.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct FnHead {
    pub(super) kind: ItemKind,
    pub(super) vis: Vis,
    pub(super) name: String,
    /// The label this mark selects by in a URL: `Type::method` for a method,
    /// the plain name for a free declaration.
    pub(super) label: String,
    pub(super) path: String,
    pub(super) line: u32,
    /// The impl or trait block this method is written in, as the survey
    /// headers one (`impl Clone for Vis`); empty for a free declaration. It is
    /// whose method this is, which the name alone never says.
    pub(super) section: String,
}

impl FnHead {
    /// What rust writes in front of the name: `pub fn`, `fn`, `macro`.
    pub(super) fn decl(&self) -> String {
        self.kind.decl_words(&self.vis)
    }

    /// `src/views/func/model.rs:210` — where the declaration is written.
    pub(super) fn locator(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }

    /// `model.rs:210` — the same fact where a row has no room for the path.
    /// The discriminating end of a locator is its tail, so this is the half a
    /// truncating column must keep.
    pub(super) fn file_line(&self) -> String {
        let file = self.path.rsplit('/').next().unwrap_or(&self.path);
        format!("{file}:{}", self.line)
    }
}

impl MethodRow {
    /// The trait this row's own impl header promises, by its leading name:
    /// `impl From<Option<ast::Visibility>> for Vis` promises `From`, and a row
    /// written in an inherent impl promises nothing. The header is the
    /// source's own text, so this reads it rather than rebuilding it.
    fn promises(&self) -> Option<&str> {
        let rest = self.section.strip_prefix("impl ")?;
        let (promise, _) = rest.rsplit_once(" for ")?;
        let end = promise
            .find(['<', ':', ' '])
            .unwrap_or(promise.len())
            .max(1);
        Some(&promise[..end])
    }
}

/// One workspace type a function's signature names or its body uses. This
/// chart draws no block for a type, so every one of them is a row on the
/// sheet — with a link down to the altitude that does draw it.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Touch {
    /// The type's own mark, for the descent link.
    pub(super) ty: u32,
    pub(super) decl: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) label: String,
    /// What this function does with it, in one word: `signature` where the
    /// declaration names it, `n refs` where only the body does.
    pub(super) word: String,
    /// Whether the data chart draws a block for it: a struct, an enum, a union
    /// or a static stands there; a trait or an alias does not, and its row
    /// opens as a quotation instead.
    pub(super) on_data: bool,
    /// References from this function's body, for ranking.
    pub(super) count: u32,
}

/// One mark on the paper: a declaration that runs.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct FnMark {
    pub(super) id: u32,
    /// The frame it is written in — its module, or the group inside it the
    /// reading asked for.
    pub(super) frame: u32,
    pub(super) tier: Tier,
    /// The entry point whose road reached it first, for the strips plate.
    /// `None` for an entry point itself and for a mark in a call ring.
    pub(super) road: Option<u32>,
    pub(super) head: FnHead,
    pub(super) rows: Vec<SigRow>,
    /// How this declaration differs from the diff base.
    pub(super) delta: Delta,
    /// Callers, callees, and the types it touches — the counts its hover words
    /// and its sheet spend. Never drawn on the resting paper: a count stamped
    /// on every block is texture, not signal.
    pub(super) callers: u32,
    pub(super) calls: u32,
    pub(super) touches: usize,
    /// It calls itself. Recursion is a fact about one mark, so it is a word on
    /// that mark rather than a wire that leaves and comes back.
    pub(super) recurses: bool,
}

impl FnMark {
    /// The diff's own letter for this declaration, or none where the epoch
    /// left it alone.
    pub(super) fn letter(&self) -> Option<&'static str> {
        match self.delta {
            Delta::Added => Some("A"),
            Delta::Changed => Some("M"),
            Delta::Same => None,
        }
    }

    /// Where this mark stands, in one sentence: its tier, and whether it calls
    /// itself. What the sheet says under the locator.
    pub(super) fn stands(&self) -> String {
        let mut words = self.tier.words();
        if self.recurses {
            words.push_str(" · it calls itself");
        }
        words
    }

    /// Everything the mark's hover words say: what it is, where it is written,
    /// where it stands, and what the paper does not draw about it.
    pub(super) fn title(&self) -> String {
        let mut parts = vec![
            format!("{} {}", self.head.decl(), self.head.name),
            self.head.locator(),
            self.tier.words(),
        ];
        if !self.head.section.is_empty() {
            parts.insert(1, self.head.section.clone());
        }
        parts.push(match self.callers {
            0 => "called by nothing in the workspace".to_string(),
            n => plural(n as usize, "caller"),
        });
        if self.calls > 0 {
            parts.push(format!("calls {}", self.calls));
        }
        if self.recurses {
            parts.push("calls itself".to_string());
        }
        if self.touches > 0 {
            parts.push(format!("touches {}", plural(self.touches, "type")));
        }
        parts.push("select it".to_string());
        parts.join(" · ")
    }
}

/// One frame on the paper: a workspace crate, one module inside a crate, or —
/// where the reading groups inside a module — one type or one file inside that.
/// Module frames nest the way rust's modules do, so the ground reads as the
/// tree the code is written in.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Frame {
    pub(super) id: u32,
    pub(super) krate: String,
    pub(super) module: Vec<String>,
    /// The group this frame draws inside its module, where the reading groups
    /// inside one: a type's name, or a file's. Empty on a crate or module
    /// frame, which is what makes this frame one of those.
    pub(super) group: String,
    pub(super) parent: Option<u32>,
    /// The marks written in this frame, in reading order — deepest band last,
    /// so the mechanism plate's rows come out in running order.
    pub(super) marks: Vec<u32>,
}

/// One frame's identity: its crate, the module path rust reads it at, and the
/// group it draws inside that module (empty on a crate or module frame).
type FrameKey = (String, Vec<String>, String);

impl Frame {
    /// The frame's own name on its border: the group it draws, else the
    /// module's last segment, else the crate's name at the top.
    pub(super) fn label(&self) -> String {
        if !self.group.is_empty() {
            return self.group.clone();
        }
        match self.module.last() {
            Some(last) => last.clone(),
            None => self.krate.clone(),
        }
    }

    /// The key this frame selects by in a URL: the crate, then the module path
    /// as rust nests it, then the group where it draws one. The first two are
    /// the same key the data altitude's frames carry, so a reviewer reading one
    /// module at two altitudes says one word for it.
    pub(super) fn key(&self) -> Vec<String> {
        let mut key = vec![self.krate.clone()];
        key.extend(self.module.iter().cloned());
        if !self.group.is_empty() {
            key.push(self.group.clone());
        }
        key
    }

    /// The whole path as rust writes it, for hover words and engraved names.
    pub(super) fn written(&self) -> String {
        let mut parts: Vec<&str> = self.module.iter().map(String::as_str).collect();
        if !self.group.is_empty() {
            parts.push(&self.group);
        }
        match parts.is_empty() {
            true => self.krate.clone(),
            false => parts.join("::"),
        }
    }

    /// Which module a file's declarations are framed in: its crate, and the
    /// module path rust reads the file at.
    fn module_of(krate: &str, path: &str) -> (String, Vec<String>) {
        (
            krate.to_string(),
            module_path(path).into_iter().map(str::to_string).collect(),
        )
    }
}

impl Group {
    /// Which group inside its module one declaration belongs to, under this
    /// reading. Empty means the module's own shelf — which is where a free
    /// declaration always sits, because nothing owns it.
    fn of(self, item: &ItemMark, module: &[String], path: &str) -> String {
        match self {
            Group::Module => String::new(),
            // The label the survey wrote is the authority: `Vis::keyword` is
            // `keyword` of `Vis`, and the type is whatever the impl's own
            // header named — a trait's clause included, which files under the
            // trait.
            Group::Owner => item
                .head
                .label
                .rsplit_once("::")
                .map(|(owner, _)| owner.to_string())
                .unwrap_or_default(),
            Group::File => {
                let file = path.rsplit('/').next().unwrap_or(path);
                let stem = file.strip_suffix(".rs").unwrap_or(file);
                // A file that already gave its module its name would draw a
                // frame around a frame saying the same word: `src/load.rs` is
                // `mod load`.
                match module.last().map(String::as_str) == Some(stem) {
                    true => String::new(),
                    false => file.to_string(),
                }
            }
        }
    }
}

/// One column of the section: one frame's marks, crossing every band. The
/// frame's own words ride along, because a column is a boundary a reader can
/// select and the layout must not go back to the model to name it.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Column {
    pub(super) frame: u32,
    /// The frame's whole path — what the prism engraves along its top, and
    /// what its hover words say.
    pub(super) written: String,
    /// The key selecting this boundary pushes.
    pub(super) key: Vec<String>,
    /// The marks in this frame, by band.
    pub(super) cells: Vec<(u32, Vec<u32>)>,
}

/// What the cartouche states about the survey at this altitude.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnFacts {
    pub(super) fns: usize,
    pub(super) methods: usize,
    pub(super) macros: usize,
    pub(super) entries: usize,
    pub(super) ring: usize,
    pub(super) deepest: u32,
    pub(super) added: usize,
    pub(super) changed: usize,
    pub(super) unresolved: u32,
    /// Declarations the visibility reading leaves off the paper.
    pub(super) off_paper: usize,
}

/// Everything one build of the function chart reads out of the survey.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnModel {
    pub(super) marks: Vec<FnMark>,
    pub(super) calls: Vec<Call>,
    pub(super) frames: Vec<Frame>,
    pub(super) columns: Vec<Column>,
    /// Every band the paper has, in order, with its caption.
    pub(super) bands: Vec<(u32, String)>,
    pub(super) facts: FnFacts,
    /// The types each mark touches, by mark id — read by the sheet only, so
    /// the chart never pays to draw what it does not draw.
    pub(super) touches: HashMap<u32, Vec<Touch>>,
    /// Whether the epoch touched anything this reading draws.
    pub(super) dirty: bool,
}

impl FnModel {
    pub(super) fn by_id(&self) -> HashMap<u32, &FnMark> {
        self.marks.iter().map(|m| (m.id, m)).collect()
    }

    /// One mark by the (file, label) a URL names.
    pub(super) fn find(&self, path: &str, label: &str) -> Option<&FnMark> {
        self.marks
            .iter()
            .find(|m| m.head.path == path && m.head.label == label)
    }

    /// One band's caption, from the model's own list — never rebuilt, so a
    /// lane and a sheet say the same words about the same band.
    pub(super) fn caption_of(&self, band: u32) -> String {
        self.bands
            .iter()
            .find(|(at, _)| *at == band)
            .map(|(_, caption)| caption.clone())
            .unwrap_or_default()
    }

    /// Every mark a rewrite of `from` could reach, walking callers outward: the
    /// transitive callers, and the methods answering a clause it answers. The
    /// blast radius, in the same sense the two rungs above use the word.
    pub(super) fn upstream(&self, from: u32) -> HashSet<u32> {
        let mut users: HashMap<u32, Vec<u32>> = HashMap::new();
        for call in &self.calls {
            users.entry(call.def).or_default().push(call.user);
        }
        let mut seen: HashSet<u32> = HashSet::new();
        let mut queue: VecDeque<u32> = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            for &user in users.get(&at).into_iter().flatten() {
                if user != from && seen.insert(user) {
                    queue.push_back(user);
                }
            }
        }
        seen
    }

    /// Every mark one call away from a set, in either direction — what a
    /// boundary reading reads a step behind itself.
    pub(super) fn one_hop(&self, inside: &HashSet<u32>) -> HashSet<u32> {
        self.calls
            .iter()
            .filter_map(
                |c| match (inside.contains(&c.def), inside.contains(&c.user)) {
                    (true, false) => Some(c.user),
                    (false, true) => Some(c.def),
                    _ => None,
                },
            )
            .collect()
    }

    /// Read one build of the chart out of the survey.
    pub(super) fn build(graph: &CodeGraph, reading: &FnReading) -> Self {
        let floor = reading.vis_floor;
        let kind_of = |id: u32| graph.item(id).map(|m| m.head.kind);
        // Every declaration that runs, whatever the reading — the census the
        // slider narrows, and the id space every walk below indexes by.
        let runs: Vec<&ItemMark> = graph
            .items
            .iter()
            .filter(|m| m.head.kind.is_callable())
            .collect();
        let drawn: HashSet<u32> = runs
            .iter()
            .filter(|m| floor.admits(&m.head.vis))
            .map(|m| m.id)
            .collect();

        // ---- The two families. --------------------------------------------
        //
        // Calls first: every resolved reference whose ends both run. A
        // reference from a body to a type is not a call — it is a touch, and it
        // is read further down.
        let mut recurses: HashSet<u32> = HashSet::new();
        let mut pairs: HashMap<(u32, u32), u32> = HashMap::new();
        for MarkRef { from, to, count } in &graph.refs {
            if !kind_of(*from).is_some_and(ItemKind::is_callable)
                || !kind_of(*to).is_some_and(ItemKind::is_callable)
            {
                continue;
            }
            if from == to {
                recurses.insert(*from);
                continue;
            }
            *pairs.entry((*to, *from)).or_default() += count;
        }

        // Then the contracts: a trait's own clause, and the methods that
        // answer it. The impl is resolved semantically by the survey; which of
        // the type's methods answers which clause is read off the impl header
        // the method is written under, by name — the one thing the header says
        // that the marks do not.
        let mut answers: Vec<(u32, u32)> = Vec::new();
        for ImplEdge { trait_mark, ty, .. } in &graph.implements {
            let (Some(promise), Some(implementer)) = (graph.item(*trait_mark), graph.item(*ty))
            else {
                continue;
            };
            for row in &implementer.body.method_rows {
                if row.promises() != Some(promise.head.name.as_str()) {
                    continue;
                }
                let Some(clause) = promise
                    .body
                    .method_rows
                    .iter()
                    .find(|c| c.name == row.name && c.mark != row.mark)
                else {
                    continue;
                };
                answers.push((clause.mark, row.mark));
            }
        }

        // ---- The tier: how far from something that starts. -----------------
        //
        // In-degree over both families, self-calls excluded: a function that
        // calls itself has not been started by anything.
        let mut callers_of: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut callees_of: HashMap<u32, Vec<u32>> = HashMap::new();
        for &(def, user) in pairs.keys() {
            callers_of.entry(def).or_default().push(user);
            callees_of.entry(user).or_default().push(def);
        }
        for &(clause, answer) in &answers {
            callers_of.entry(answer).or_default().push(clause);
            callees_of.entry(clause).or_default().push(answer);
        }
        // Deterministic order: the maps are walked below, and a HashMap's
        // iteration order is not the chart's to inherit.
        for list in callers_of.values_mut().chain(callees_of.values_mut()) {
            list.sort_unstable();
            list.dedup();
        }

        // A multi-source walk, so depth and road come out of one pass: the
        // entry points are seeded in id order and each mark keeps the road that
        // reached it first.
        let mut entries: Vec<u32> = runs
            .iter()
            .map(|m| m.id)
            .filter(|id| callers_of.get(id).is_none_or(|list| list.is_empty()))
            .collect();
        entries.sort_unstable();
        let mut depth: HashMap<u32, u32> = HashMap::new();
        let mut road: HashMap<u32, u32> = HashMap::new();
        // The way in: which caller reached this mark first. One wire per mark,
        // and the whole of the resting plate.
        let mut via: HashMap<u32, u32> = HashMap::new();
        let mut queue: VecDeque<u32> = VecDeque::new();
        for &entry in &entries {
            depth.insert(entry, 0);
            queue.push_back(entry);
        }
        while let Some(at) = queue.pop_front() {
            let next = depth[&at] + 1;
            let from = road.get(&at).copied().unwrap_or(at);
            for &callee in callees_of.get(&at).into_iter().flatten() {
                if depth.contains_key(&callee) {
                    continue;
                }
                depth.insert(callee, next);
                road.insert(callee, from);
                via.insert(callee, at);
                queue.push_back(callee);
            }
        }
        let tier_of = |id: u32| match depth.get(&id) {
            Some(0) => Tier::Entry,
            Some(&n) => Tier::Deep(n),
            None => Tier::Ring,
        };
        let deepest = depth.values().copied().max().unwrap_or(0);

        // ---- The frames. ---------------------------------------------------
        let mut frame_of: HashMap<FrameKey, u32> = HashMap::new();
        let mut frames: Vec<Frame> = Vec::new();
        // One frame per (crate, module path, group), each standing inside the
        // one above it all the way up to the crate — whether or not that level
        // declares a mark of its own, because a module the code nests inside
        // another is a module drawn inside another.
        let mut frame_for =
            |krate: &str, module: &[String], group: &str, frames: &mut Vec<Frame>| -> u32 {
                let key = (krate.to_string(), module.to_vec(), group.to_string());
                if let Some(&id) = frame_of.get(&key) {
                    return id;
                }
                let up: Option<(Vec<String>, String)> = match group.is_empty() {
                    // A group frame's parent is its module.
                    false => Some((module.to_vec(), String::new())),
                    // A module frame's parent is the module above it; the
                    // crate's frame has none.
                    true => module
                        .split_last()
                        .map(|(_, up)| (up.to_vec(), String::new())),
                };
                let parent = up.map(|(module, group)| {
                    let up_key = (krate.to_string(), module.clone(), group.clone());
                    match frame_of.get(&up_key) {
                        Some(&id) => id,
                        None => {
                            let id = frames.len() as u32;
                            frames.push(Frame {
                                id,
                                krate: krate.to_string(),
                                module,
                                group,
                                parent: None,
                                marks: Vec::new(),
                            });
                            frame_of.insert(up_key, id);
                            id
                        }
                    }
                });
                let id = frames.len() as u32;
                frames.push(Frame {
                    id,
                    krate: krate.to_string(),
                    module: module.to_vec(),
                    group: group.to_string(),
                    parent,
                    marks: Vec::new(),
                });
                frame_of.insert(key, id);
                id
            };

        // ---- The marks. ----------------------------------------------------
        let method_row = |mark: &ItemMark| -> Option<&MethodRow> {
            let parent = graph.item(mark.parent?)?;
            parent.body.method_rows.iter().find(|r| r.mark == mark.id)
        };
        let mut marks: Vec<FnMark> = Vec::new();
        for item in runs.iter().filter(|m| drawn.contains(&m.id)) {
            let Some(file) = graph.file(item.file) else {
                continue;
            };
            let (krate, module) = Frame::module_of(&file.krate, &file.path);
            let group = reading.group.of(item, &module, &file.path);
            let frame = frame_for(&krate, &module, &group, &mut frames);
            marks.push(FnMark {
                id: item.id,
                frame,
                tier: tier_of(item.id),
                road: road.get(&item.id).copied(),
                head: FnHead {
                    kind: item.head.kind,
                    vis: item.head.vis.clone(),
                    name: item.head.name.clone(),
                    label: item.head.label.clone(),
                    path: file.path.clone(),
                    line: item.head.line,
                    section: method_row(item)
                        .map(|r| r.section.clone())
                        .unwrap_or_default(),
                },
                rows: SigRow::quote(item),
                delta: item.diff.delta,
                callers: callers_of.get(&item.id).map_or(0, |l| l.len() as u32),
                calls: callees_of.get(&item.id).map_or(0, |l| l.len() as u32),
                touches: 0,
                recurses: recurses.contains(&item.id),
            });
        }
        marks.sort_by(|a, b| {
            (a.tier.band(deepest), &a.head.name, a.id).cmp(&(
                b.tier.band(deepest),
                &b.head.name,
                b.id,
            ))
        });

        // ---- What each mark touches, one rung down. ------------------------
        let touches = Touch::read(graph, &marks, method_row);
        for mark in marks.iter_mut() {
            mark.touches = touches.get(&mark.id).map_or(0, Vec::len);
        }

        // ---- The drawn ink, narrowed to what the reading draws. ------------
        let mut calls: Vec<Call> = pairs
            .iter()
            .filter(|((def, user), _)| drawn.contains(def) && drawn.contains(user))
            .map(|(&(def, user), &count)| Call {
                def,
                user,
                kind: CallKind::Call,
                count,
                rest: via.get(&def) == Some(&user),
            })
            .collect();
        calls.extend(
            answers
                .iter()
                .filter(|(clause, answer)| drawn.contains(clause) && drawn.contains(answer))
                .map(|&(clause, answer)| Call {
                    def: clause,
                    user: answer,
                    kind: CallKind::Answers,
                    count: 0,
                    rest: true,
                }),
        );
        calls.sort_by_key(|c| (c.def, c.user, c.kind == CallKind::Answers));

        // ---- The three seatings. -------------------------------------------
        let bands = {
            let mut seen: Vec<u32> = marks.iter().map(|m| m.tier.band(deepest)).collect();
            seen.sort_unstable();
            seen.dedup();
            seen.into_iter()
                .map(|band| {
                    let caption = marks
                        .iter()
                        .find(|m| m.tier.band(deepest) == band)
                        .map(|m| m.tier.caption())
                        .unwrap_or_default();
                    (band, caption)
                })
                .collect::<Vec<_>>()
        };
        for frame in frames.iter_mut() {
            frame.marks = marks
                .iter()
                .filter(|m| m.frame == frame.id)
                .map(|m| m.id)
                .collect();
        }
        let columns = Column::read(&frames, &marks, deepest);

        let facts = FnFacts {
            fns: marks
                .iter()
                .filter(|m| m.head.kind == ItemKind::Fn && m.head.section.is_empty())
                .count(),
            methods: marks
                .iter()
                .filter(|m| m.head.kind == ItemKind::Fn && !m.head.section.is_empty())
                .count(),
            macros: marks
                .iter()
                .filter(|m| m.head.kind == ItemKind::Macro)
                .count(),
            entries: marks.iter().filter(|m| m.tier == Tier::Entry).count(),
            ring: marks.iter().filter(|m| m.tier == Tier::Ring).count(),
            deepest,
            added: marks.iter().filter(|m| m.delta == Delta::Added).count(),
            changed: marks.iter().filter(|m| m.delta == Delta::Changed).count(),
            unresolved: graph.limits.unresolved,
            // What the slider leaves off the paper: the chart states what it
            // draws, and this is the one number that states what it does not.
            off_paper: runs.iter().filter(|m| !floor.admits(&m.head.vis)).count(),
        };
        let dirty = facts.added + facts.changed > 0;
        FnModel {
            marks,
            calls,
            frames,
            columns,
            bands,
            facts,
            touches,
            dirty,
        }
    }
}

impl Touch {
    /// Every type each mark's signature names or its body uses. Read once
    /// for the whole model: a sheet asks for one mark's worth, and
    /// re-walking the survey per selection would read the same edges a
    /// hundred times.
    fn read<'g>(
        graph: &'g CodeGraph,
        marks: &[FnMark],
        method_row: impl Fn(&ItemMark) -> Option<&'g MethodRow>,
    ) -> HashMap<u32, Vec<Touch>> {
        let touch = |ty: u32, word: String, count: u32| -> Option<Touch> {
            let item = graph.item(ty)?;
            let file = graph.file(item.file)?;
            Some(Touch {
                ty,
                decl: item.head.kind.decl_words(&item.head.vis),
                name: item.head.name.clone(),
                path: file.path.clone(),
                label: item.head.label.clone(),
                word,
                on_data: item.head.kind.is_data() && item.parent.is_none(),
                count,
            })
        };
        let mut out: HashMap<u32, Vec<Touch>> = HashMap::new();
        for mark in marks {
            let Some(item) = graph.item(mark.id) else {
                continue;
            };
            let mut seen: HashMap<u32, Touch> = HashMap::new();
            // The signature's own reach. A free function's holding edges are its
            // own; a method's are filed on the type its impl names, under the
            // method's name — so that is the row to read them back from.
            let signature: Vec<&HoldEdge> = match method_row(item) {
                None => graph.holds.iter().filter(|h| h.from == mark.id).collect(),
                Some(row) => graph
                    .holds
                    .iter()
                    .filter(|h| {
                        h.from == item.parent.unwrap_or(mark.id)
                            && h.from_method
                            && h.fields.iter().any(|(name, _)| *name == row.name)
                    })
                    .collect(),
            };
            for edge in signature {
                if let Some(t) = touch(edge.to, "signature".to_string(), 0) {
                    seen.insert(edge.to, t);
                }
            }
            // What only the body reaches. A pair the signature already named
            // keeps its stronger word: a declaration is a promise, a body is not.
            for MarkRef { from, to, count } in &graph.refs {
                if *from != mark.id {
                    continue;
                }
                let Some(far) = graph.item(*to) else { continue };
                if far.head.kind.is_callable() {
                    continue;
                }
                match seen.get_mut(to) {
                    Some(existing) => existing.count += count,
                    None => {
                        if let Some(t) = touch(*to, plural(*count as usize, "ref"), *count) {
                            seen.insert(*to, t);
                        }
                    }
                }
            }
            if seen.is_empty() {
                continue;
            }
            let mut rows: Vec<Touch> = seen.into_values().collect();
            // Structure first, then bodies heaviest first — the sheet's order
            // everywhere in this system.
            rows.sort_by(|a, b| {
                (a.word != "signature", b.count, &a.name).cmp(&(
                    b.word != "signature",
                    a.count,
                    &b.name,
                ))
            });
            out.insert(mark.id, rows);
        }
        out
    }
}

/// The strata plate's columns: one per leaf module that declares a mark, in
/// the order the modules read, each carrying its marks by band.
impl Column {
    /// The strata plate's columns: one per frame that holds a mark, in the
    /// order the frames read, each carrying its marks by band.
    fn read(frames: &[Frame], marks: &[FnMark], deepest: u32) -> Vec<Column> {
        let mut out: Vec<Column> = Vec::new();
        for frame in frames.iter().filter(|f| !f.marks.is_empty()) {
            let mut cells: Vec<(u32, Vec<u32>)> = Vec::new();
            for mark in marks.iter().filter(|m| m.frame == frame.id) {
                let band = mark.tier.band(deepest);
                match cells.iter_mut().find(|(at, _)| *at == band) {
                    Some((_, ids)) => ids.push(mark.id),
                    None => cells.push((band, vec![mark.id])),
                }
            }
            cells.sort_by_key(|(band, _)| *band);
            out.push(Column {
                frame: frame.id,
                written: frame.written(),
                key: frame.key(),
                cells,
            });
        }
        out.sort_by(|a, b| a.written.cmp(&b.written));
        out
    }
}

/// Which reading of the calls a wire is drawn in, anchored on what the reader
/// has in hand. The same rule the data altitude reads its references by: a
/// direction means nothing without an anchor, because one line is this
/// function's call and that function's caller.
impl CallDir {
    pub(super) fn draws(self, at: u32, def: u32, user: u32) -> bool {
        match self {
            CallDir::Calls => user == at,
            CallDir::Callers => def == at,
            CallDir::Both => user == at || def == at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::data::{DeclBody, DeclHead, Reach};
    use crate::views::data::VisFloor;

    fn item(id: u32, name: &str, kind: ItemKind, vis: Vis, parent: Option<u32>) -> ItemMark {
        ItemMark {
            id,
            file: 0,
            parent,
            head: DeclHead {
                name: name.to_string(),
                label: name.to_string(),
                kind,
                vis,
                line: id + 1,
            },
            body: DeclBody::default(),
            reach: Reach::default(),
            diff: Default::default(),
        }
    }

    /// `main` calls `walk`, `walk` calls `note`, and nothing calls `main`.
    fn chain() -> CodeGraph {
        CodeGraph {
            files: vec![crate::graph::data::FileInfo {
                path: "src/main.rs".to_string(),
                krate: "slope".to_string(),
            }],
            items: vec![
                item(0, "main", ItemKind::Fn, Vis::Pub, None),
                item(1, "walk", ItemKind::Fn, Vis::Crate, None),
                item(2, "note", ItemKind::Fn, Vis::Private, None),
                item(3, "Held", ItemKind::Struct, Vis::Pub, None),
            ],
            implements: Vec::new(),
            refs: vec![
                MarkRef {
                    from: 0,
                    to: 1,
                    count: 2,
                },
                MarkRef {
                    from: 1,
                    to: 2,
                    count: 5,
                },
                MarkRef {
                    from: 1,
                    to: 3,
                    count: 3,
                },
            ],
            holds: Vec::new(),
            ghosts: Vec::new(),
            limits: Default::default(),
        }
    }

    fn reading(vis_floor: VisFloor) -> FnReading {
        FnReading {
            calls: CallDir::default(),
            vis_floor,
            group: Group::default(),
        }
    }

    /// The tier is the whole reading: what nothing calls starts, and every
    /// other mark is as many calls in as the shortest way to it.
    #[test]
    fn depth_counts_calls_from_the_nearest_entry_point() {
        let model = FnModel::build(&chain(), &reading(VisFloor::All));
        let tier = |name: &str| {
            model
                .marks
                .iter()
                .find(|m| m.head.name == name)
                .map(|m| m.tier)
        };
        assert_eq!(tier("main"), Some(Tier::Entry));
        assert_eq!(tier("walk"), Some(Tier::Deep(1)));
        assert_eq!(tier("note"), Some(Tier::Deep(2)));
        // A struct is not a mark on this chart at all.
        assert_eq!(tier("Held"), None);
        assert_eq!(model.facts.deepest, 2);
        assert_eq!(model.facts.entries, 1);
    }

    /// A reference to a type is a touch, never a call — and it is a row on the
    /// sheet with the descent link on it, because this chart draws no block
    /// for a type.
    #[test]
    fn a_type_is_touched_and_never_called() {
        let model = FnModel::build(&chain(), &reading(VisFloor::All));
        assert!(
            model.calls.iter().all(|c| c.def != 3 && c.user != 3),
            "a type must never take a call wire"
        );
        let walk = model.touches.get(&1).expect("walk touches something");
        assert_eq!(walk.len(), 1);
        assert_eq!(walk[0].name, "Held");
        assert_eq!(walk[0].word, "3 refs");
        assert!(walk[0].on_data, "a struct has a block one rung down");
    }

    /// Narrowing the reading takes marks off the paper — and takes their wires
    /// with them, never leaving a wire pointing at nothing.
    #[test]
    fn a_narrow_reading_draws_no_wire_to_an_undrawn_mark() {
        let model = FnModel::build(&chain(), &reading(VisFloor::Crate));
        let drawn: HashSet<u32> = model.marks.iter().map(|m| m.id).collect();
        assert_eq!(drawn, HashSet::from([0, 1]));
        assert!(
            model
                .calls
                .iter()
                .all(|c| drawn.contains(&c.def) && drawn.contains(&c.user))
        );
        assert_eq!(model.facts.off_paper, 1, "`note` is private");
        // The tier is a fact about the workspace, not about the reading: what
        // the slider hides still called it.
        let walk = model.marks.iter().find(|m| m.head.name == "walk").unwrap();
        assert_eq!(walk.tier, Tier::Deep(1));
    }

    /// A ring nothing reaches is said in words, not dropped: two functions
    /// that only call each other are on the paper, in their own band.
    #[test]
    fn a_call_ring_no_entry_point_reaches_says_so() {
        let mut graph = chain();
        graph
            .items
            .push(item(4, "ping", ItemKind::Fn, Vis::Private, None));
        graph
            .items
            .push(item(5, "pong", ItemKind::Fn, Vis::Private, None));
        graph.refs.push(MarkRef {
            from: 4,
            to: 5,
            count: 1,
        });
        graph.refs.push(MarkRef {
            from: 5,
            to: 4,
            count: 1,
        });
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        for name in ["ping", "pong"] {
            let mark = model.marks.iter().find(|m| m.head.name == name).unwrap();
            assert_eq!(mark.tier, Tier::Ring);
        }
        assert_eq!(model.facts.ring, 2);
        // And the ring is the last band on the paper, under every depth.
        let last = model.bands.last().expect("a band");
        assert_eq!(last.1, "in a call ring");
    }

    /// Recursion is a word on the mark, never a wire that leaves and comes
    /// back to the same block.
    #[test]
    fn a_function_that_calls_itself_says_so_and_draws_no_wire() {
        let mut graph = chain();
        graph.refs.push(MarkRef {
            from: 2,
            to: 2,
            count: 4,
        });
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        let note = model.marks.iter().find(|m| m.head.name == "note").unwrap();
        assert!(note.recurses);
        assert!(model.calls.iter().all(|c| c.def != c.user));
        // And it is still two calls deep: calling itself starts nothing.
        assert_eq!(note.tier, Tier::Deep(2));
    }

    /// A method answering a workspace trait's clause is not an entry point:
    /// what calls the promise calls it, and the dashed family is what carries
    /// that across a `dyn` call the survey cannot follow.
    #[test]
    fn a_method_answering_a_contract_is_reached_through_it() {
        let mut graph = chain();
        // `trait Held { fn note(); }` with `impl Held for Held { fn note() }`.
        let mut promise = item(4, "Words", ItemKind::Trait, Vis::Pub, None);
        promise.body.method_rows = vec![MethodRow {
            name: "say".to_string(),
            sig: "fn say(&self)".to_string(),
            mark: 5,
            section: "trait Words".to_string(),
        }];
        let mut ty = item(6, "Plate", ItemKind::Struct, Vis::Pub, None);
        ty.body.method_rows = vec![MethodRow {
            name: "say".to_string(),
            sig: "fn say(&self)".to_string(),
            mark: 7,
            section: "impl Words for Plate".to_string(),
        }];
        graph.items.push(promise);
        graph
            .items
            .push(item(5, "say", ItemKind::Fn, Vis::Pub, Some(4)));
        graph.items.push(ty);
        graph
            .items
            .push(item(7, "say", ItemKind::Fn, Vis::Private, Some(6)));
        graph.implements.push(ImplEdge {
            trait_mark: 4,
            ty: 6,
            event: None,
        });
        // `main` calls the clause, the way a `dyn` call does.
        graph.refs.push(MarkRef {
            from: 0,
            to: 5,
            count: 1,
        });
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        let by_id = model.by_id();
        assert_eq!(by_id[&5].tier, Tier::Deep(1), "the clause is called");
        assert_eq!(
            by_id[&7].tier,
            Tier::Deep(2),
            "the method that answers it runs one call further in"
        );
        let answer = model
            .calls
            .iter()
            .find(|c| c.kind == CallKind::Answers)
            .expect("a contract wire");
        assert_eq!((answer.def, answer.user), (5, 7));
    }

    /// Grouping nests inside the module frames; it never replaces them. A
    /// method sits with the type its impl names, a free declaration stays on
    /// the module's own shelf, and a file that already named its module draws
    /// no second frame saying the same word.
    #[test]
    fn a_grouping_nests_inside_the_module_it_is_written_in() {
        let mut graph = chain();
        // `impl Held { fn say() }`, written in the same file. Its own name is
        // not `note`, which the chain already has free: two marks of one name
        // would make this test read whichever the map kept.
        graph
            .items
            .push(item(4, "say", ItemKind::Fn, Vis::Pub, Some(3)));
        graph.items[4].head.label = "Held::say".to_string();
        // And one declaration in `src/load.rs`, which is `mod load` itself.
        graph.files.push(crate::graph::data::FileInfo {
            path: "src/load.rs".to_string(),
            krate: "slope".to_string(),
        });
        let mut loaded = item(5, "load", ItemKind::Fn, Vis::Pub, None);
        loaded.file = 1;
        graph.items.push(loaded);

        // The frame each named declaration lands in, under one grouping.
        let framed = |group: Group| -> HashMap<String, Frame> {
            let model = FnModel::build(
                &graph,
                &FnReading {
                    calls: CallDir::default(),
                    vis_floor: VisFloor::All,
                    group,
                },
            );
            model
                .marks
                .iter()
                .filter_map(|mark| {
                    let frame = model.frames.iter().find(|f| f.id == mark.frame)?;
                    Some((mark.head.name.clone(), frame.clone()))
                })
                .collect()
        };

        // By module: `src/main.rs` is the crate root and names no module, so
        // both sit on the crate's own shelf.
        let at = framed(Group::Module);
        assert_eq!(at["main"].id, at["say"].id);
        assert_eq!(at["main"].label(), "slope");
        assert!(at["main"].group.is_empty());
        assert_eq!(at["load"].label(), "load", "src/load.rs is mod load");

        // By type: the method moves into a frame named for the type its impl
        // names, inside the frame it was already in. The free function does
        // not move at all.
        let at = framed(Group::Owner);
        assert_eq!(at["main"].label(), "slope");
        assert_eq!(at["say"].label(), "Held");
        assert_eq!(
            at["say"].parent,
            Some(at["main"].id),
            "a type frame stands inside the module that writes it"
        );
        assert_eq!(at["say"].key(), vec!["slope", "Held"]);
        assert_eq!(at["say"].written(), "Held");

        // By file: `main.rs` earns a frame inside the crate, and `load.rs`
        // does not — it already gave `mod load` its name, and a frame around
        // a frame saying the same word says nothing.
        let at = framed(Group::File);
        assert_eq!(at["main"].id, at["say"].id);
        assert_eq!(at["main"].label(), "main.rs");
        assert_eq!(at["load"].label(), "load");
        assert!(at["load"].group.is_empty());
    }

    /// The header the survey wrote is what says which contract a method
    /// answers; nothing here rebuilds it.
    #[test]
    fn the_promise_is_read_off_the_impl_header() {
        let row = |section: &str| MethodRow {
            name: "say".to_string(),
            sig: "fn say(&self)".to_string(),
            mark: 0,
            section: section.to_string(),
        };
        assert_eq!(row("impl Clone for Vis").promises(), Some("Clone"));
        assert_eq!(
            row("impl From<Option<ast::Visibility>> for Vis").promises(),
            Some("From")
        );
        assert_eq!(row("impl Vis").promises(), None);
        assert_eq!(row("trait Words").promises(), None);
    }
}
