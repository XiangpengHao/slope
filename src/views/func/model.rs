//! What the function chart reads out of the survey: the code that runs, seated
//! inside whatever calls it.
//!
//! The rung above asks what the workspace *keeps*; this one asks what it
//! *does*. Its marks are the declarations that run — every function, every
//! method, every trait clause a method answers, every `macro_rules!` — and its
//! one organizing move is the **way in**: the call that is the shortest way
//! something that starts reaches a declaration. That is a tree, and the chart
//! draws it as containment. An **entry point** is a declaration nothing in the
//! workspace calls: `main`, a server function the client reaches through
//! generated code, a component the router mounts, a method answering a foreign
//! trait's contract. It is a top-level frame, and everything it reaches shelves
//! inside it.
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
use crate::views::func::{FnOrder, FnReading, fold_key};

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
    /// The shelving already says this one: the callee seats **inside** this
    /// caller's frame, because this call is its way in. Containment is the
    /// call, so a wire here would engrave a second time what the paper's own
    /// nesting states — and drawing all fifteen hundred resolved calls would
    /// be the hairball this system forbids one rung up.
    pub(super) seats: bool,
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
    /// The row as a block draws it: `graph: &CodeGraph`, or a receiver alone
    /// (`&mut self`), which declares no type of its own. One string, so
    /// measuring a row and drawing it can never disagree.
    pub(super) fn written(&self) -> String {
        match self.ty.is_empty() {
            true => self.name.clone(),
            false => format!("{}: {}", self.name, self.ty),
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

/// The type or trait whose impl block a method is written in — whose method
/// this is, which the name alone never says. A free declaration has none.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Owner {
    /// The owner's own mark, for the descent link.
    pub(super) ty: u32,
    pub(super) decl: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) label: String,
    /// Whether the data chart draws a block for it: a struct, an enum, a union
    /// or a static stands there; a trait does not, and its row opens as a
    /// quotation instead — the same rule the `Data touched` rows keep.
    pub(super) on_data: bool,
}

/// One mark on the paper: a declaration that runs.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct FnMark {
    pub(super) id: u32,
    pub(super) tier: Tier,
    /// The crate the declaration is written in, by its cargo package name, and
    /// the module path rust reads it at. Not a box on the paper — the ground
    /// is the call tree — but the key `/fn/mod/:..module` selects by, and the
    /// word a head wears where its module is not its caller's.
    pub(super) krate: String,
    pub(super) module: Vec<String>,
    pub(super) head: FnHead,
    pub(super) rows: Vec<SigRow>,
    /// Whose method this is, where it is one: the type or trait the impl block
    /// names. The head quotes its name, the sheet descends to its block.
    pub(super) owner: Option<Owner>,
    /// How this declaration differs from the diff base.
    pub(super) delta: Delta,
    /// Callers, callees, and the types it touches — the counts its hover words
    /// and its sheet spend. Never drawn on the resting paper: a count stamped
    /// on every block is texture, not signal.
    pub(super) callers: u32,
    pub(super) calls: u32,
    pub(super) touches: usize,
    /// How many declarations shelve inside this one's frame, however deep —
    /// what it runs by the way in. It is the weight a shelf orders by.
    pub(super) runs: u32,
    /// Its module is not the module of the caller it seats inside. A
    /// same-module call is quiet; a frame reaching across a module boundary is
    /// signal, and that is the one place a head writes a module word.
    pub(super) crosses: bool,
    /// It calls itself. Recursion is a fact about one mark, so it is a word on
    /// that mark rather than a wire that leaves and comes back.
    pub(super) recurses: bool,
    /// The reviewer folded this frame by hand: its head, its signature and its
    /// counted words are on the paper, and what shelves inside it is not.
    /// Never true where `runs` is 0 — a frame that shelves nothing has nothing
    /// to fold and draws no mark.
    pub(super) folded: bool,
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
        if self.runs > 0 {
            words.push_str(&format!(" · runs {} by the way in", self.runs));
        }
        if self.recurses {
            words.push_str(" · it calls itself");
        }
        words
    }

    /// The owner's name as the survey's own label writes it: `FnModel` in
    /// front of `build`, empty for a free declaration. The label is the
    /// survey's text and this reads it rather than rebuilding it — the head
    /// draws it with rust's own `::` after it, and a shelf ordered by owner
    /// clusters on it.
    pub(super) fn qualifier(&self) -> &str {
        match self.head.label.rsplit_once("::") {
            Some((ty, _)) => ty,
            None => "",
        }
    }

    /// The module as rust writes it — `views::func`, or the crate's own name
    /// at its root. What a head wears where it crosses one, and what prose
    /// away from the paper spells out.
    pub(super) fn written(&self) -> String {
        match self.module.is_empty() {
            true => self.krate.clone(),
            false => self.module.join("::"),
        }
    }

    /// The key `/fn/mod/:..module` selects this mark's module by: the crate,
    /// then the module path as rust nests it. The same key the data altitude's
    /// frames carry, so a reviewer reading one module at two altitudes says
    /// one word for it.
    pub(super) fn mod_key(&self) -> Vec<String> {
        let mut key = vec![self.krate.clone()];
        key.extend(self.module.iter().cloned());
        key
    }

    /// Everything the mark's hover words say: what it is, where it is written,
    /// where it stands, and what the paper does not draw about it.
    pub(super) fn title(&self) -> String {
        let mut parts = vec![
            format!("{} {}", self.head.decl(), self.head.name),
            self.written(),
            self.head.locator(),
            self.tier.words(),
        ];
        if !self.head.section.is_empty() {
            parts.insert(1, self.head.section.clone());
        }
        parts.push(format!("calls {} · called by {}", self.calls, self.callers));
        if self.runs > 0 {
            parts.push(format!("runs {} by the way in", self.runs));
        }
        if self.folded {
            parts.push("folded — click the + to open it".to_string());
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

/// Which module a file's declarations are written in: its crate, and the
/// module path rust reads the file at.
fn module_of(krate: &str, path: &str) -> (String, Vec<String>) {
    (
        krate.to_string(),
        module_path(path).into_iter().map(str::to_string).collect(),
    )
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
    /// The seating tree, by mark id: which caller each declaration shelves
    /// inside. A mark with no entry here is a frame on the ground.
    pub(super) via: HashMap<u32, u32>,
    /// The same tree read downward — what shelves inside each frame, in the
    /// order the reading seats them.
    pub(super) kids: HashMap<u32, Vec<u32>>,
    /// The frames on the ground, in seating order: an entry point, or a mark
    /// whose way in this reading does not draw.
    pub(super) seats: Vec<u32>,
    /// The frames in the ring strip: what no entry point reaches, in seating
    /// order. Each carries whatever shelves inside it.
    pub(super) ring: Vec<u32>,
    /// The frames the reviewer folded by hand. Their heads, signatures and
    /// counted words are on the paper; what shelves inside them is not.
    pub(super) folded: HashSet<u32>,
    /// Which of those folds the **packer** was allowed to skip. A folded frame
    /// outside this set keeps its whole footprint, so folding it moved nothing
    /// else on the sheet; one inside it was packed as its own box. See
    /// [`crate::views::func::FnReading::packed`] for when the sets diverge.
    pub(super) packed: HashSet<u32>,
    /// Every mark a fold hides, and the folded frame that stands for it on the
    /// paper — the head its wires re-anchor to, and the head that carries a lit
    /// chain's ink where the chain runs through the fold. The outermost fold
    /// wins: a fold inside a fold is spoken for by the one the reader can see.
    pub(super) packs: HashMap<u32, u32>,
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

    /// Every frame this mark stands inside, outward: the way in, read as the
    /// paper reads it.
    pub(super) fn ancestors(&self, of: u32) -> HashSet<u32> {
        let mut out = HashSet::new();
        let mut at = of;
        while let Some(&up) = self.via.get(&at) {
            if !out.insert(up) {
                break;
            }
            at = up;
        }
        out
    }

    /// The mark and everything shelved inside it, however deep.
    pub(super) fn subtree(&self, of: u32) -> HashSet<u32> {
        let mut out = HashSet::from([of]);
        let mut stack = vec![of];
        while let Some(at) = stack.pop() {
            for &kid in self.kids.get(&at).into_iter().flatten() {
                if out.insert(kid) {
                    stack.push(kid);
                }
            }
        }
        out
    }

    /// One mark by the (file, label) a URL names.
    pub(super) fn find(&self, path: &str, label: &str) -> Option<&FnMark> {
        self.marks
            .iter()
            .find(|m| m.head.path == path && m.head.label == label)
    }

    /// Whether a fold has this mark off the paper.
    pub(super) fn hidden(&self, id: u32) -> bool {
        self.packs.contains_key(&id)
    }

    /// The head that stands for a mark on the paper: itself where it is drawn,
    /// and the outermost fold hiding it where it is not. Every wire and every
    /// lit chain reads through this, so a fold re-anchors ink instead of
    /// cutting it.
    pub(super) fn shown(&self, id: u32) -> u32 {
        self.packs.get(&id).copied().unwrap_or(id)
    }

    /// The folds standing between the paper and one mark, outermost first —
    /// what a reveal has to open before the mark can be seen. A selection the
    /// reader cannot see is not a focus, so every way to a mark (a URL, the
    /// search, a sheet row, the arrow walk) opens the frames it hides behind
    /// first.
    pub(super) fn reveal(&self, of: u32) -> Vec<(String, String)> {
        if self.folded.is_empty() {
            return Vec::new();
        }
        let by_id = self.by_id();
        let mut out: Vec<(String, String)> = Vec::new();
        let mut at = of;
        let mut seen: HashSet<u32> = HashSet::from([of]);
        while let Some(&up) = self.via.get(&at) {
            if !seen.insert(up) {
                break;
            }
            if let Some(mark) = by_id.get(&up).filter(|_| self.folded.contains(&up)) {
                out.push(fold_key(&mark.head.path, &mark.head.label));
            }
            at = up;
        }
        out.reverse();
        out
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

        // A multi-source walk over the whole workspace: the entry points are
        // seeded in id order, and every other mark keeps the depth of the
        // shortest way in. The tier is a fact about the workspace, never about
        // the reading — what the visibility slider hides still called it.
        let mut entries: Vec<u32> = runs
            .iter()
            .map(|m| m.id)
            .filter(|id| callers_of.get(id).is_none_or(|list| list.is_empty()))
            .collect();
        entries.sort_unstable();
        let mut depth: HashMap<u32, u32> = HashMap::new();
        let mut queue: VecDeque<u32> = VecDeque::new();
        for &entry in &entries {
            depth.insert(entry, 0);
            queue.push_back(entry);
        }
        while let Some(at) = queue.pop_front() {
            let next = depth[&at] + 1;
            for &callee in callees_of.get(&at).into_iter().flatten() {
                if depth.contains_key(&callee) {
                    continue;
                }
                depth.insert(callee, next);
                queue.push_back(callee);
            }
        }
        let tier_of = |id: u32| match depth.get(&id) {
            Some(0) => Tier::Entry,
            Some(&n) => Tier::Deep(n),
            None => Tier::Ring,
        };
        let deepest = depth.values().copied().max().unwrap_or(0);

        // ---- The marks. ----------------------------------------------------
        let method_row = |mark: &ItemMark| -> Option<&MethodRow> {
            let parent = graph.item(mark.parent?)?;
            parent.body.method_rows.iter().find(|r| r.mark == mark.id)
        };
        // Whose method a declaration is: the survey resolved the impl block's
        // own type, so the owner is read off it rather than parsed back out of
        // a header. A free declaration has no parent and no owner.
        let owner_of = |ty: u32| -> Option<Owner> {
            let item = graph.item(ty)?;
            let file = graph.file(item.file)?;
            Some(Owner {
                ty,
                decl: item.head.kind.decl_words(&item.head.vis),
                name: item.head.name.clone(),
                path: file.path.clone(),
                label: item.head.label.clone(),
                on_data: item.head.kind.is_data() && item.parent.is_none(),
            })
        };
        let mut marks: Vec<FnMark> = Vec::new();
        for item in runs.iter().filter(|m| drawn.contains(&m.id)) {
            let Some(file) = graph.file(item.file) else {
                continue;
            };
            let (krate, module) = module_of(&file.krate, &file.path);
            marks.push(FnMark {
                id: item.id,
                tier: tier_of(item.id),
                krate,
                module,
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
                owner: item.parent.and_then(owner_of),
                delta: item.diff.delta,
                callers: callers_of.get(&item.id).map_or(0, |l| l.len() as u32),
                calls: callees_of.get(&item.id).map_or(0, |l| l.len() as u32),
                touches: 0,
                runs: 0,
                crosses: false,
                recurses: recurses.contains(&item.id),
                folded: false,
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
                seats: false,
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
                    seats: false,
                }),
        );
        calls.sort_by_key(|c| (c.def, c.user, c.kind == CallKind::Answers));

        // ---- The seating: the way in, over what this reading draws. --------
        //
        // The tier above is the workspace's own fact; this walk is the paper's.
        // A mark whose only caller the visibility slider left off would have no
        // way in at all, so the seeds are the marks nothing *drawn* calls, and
        // the tree that grows from them covers every block on the sheet.
        let written: HashMap<u32, String> = marks.iter().map(|m| (m.id, m.written())).collect();
        // What an owner-ordered shelf clusters on. A free declaration's key is
        // empty, so the free declarations cluster together and read first.
        let owned: HashMap<u32, String> = marks
            .iter()
            .map(|m| (m.id, m.qualifier().to_string()))
            .collect();
        let seating = Seating::read(&marks, &calls, reading.order, &written, &owned);
        for mark in marks.iter_mut() {
            mark.runs = seating.reach.get(&mark.id).copied().unwrap_or(0);
            mark.crosses = seating
                .via
                .get(&mark.id)
                .is_some_and(|up| written.get(up) != written.get(&mark.id));
        }

        // ---- The folds: what the reviewer took off the paper by hand. -------
        //
        // Nothing here folds by a count, a depth or a budget — a fold is the
        // reviewer's own gesture. A frame that shelves nothing has nothing to
        // fold, so it draws no mark at all, and a fold names its frame by the
        // same (file, label) pair the URL does, so it survives the next build of
        // the chart.
        let resolve = |set: &crate::views::func::FnFolds| -> HashSet<u32> {
            marks
                .iter()
                .filter(|m| m.runs > 0)
                .filter(|m| set.contains(&fold_key(&m.head.path, &m.head.label)))
                .map(|m| m.id)
                .collect()
        };
        let folded = resolve(&reading.folds);
        // Only a fold the packer was allowed to skip changes where anything
        // sits. The rest are elisions in place, and the layout still reserves
        // every footprint they hide.
        let packed_folds: HashSet<u32> = resolve(&reading.packed)
            .into_iter()
            .filter(|id| folded.contains(id))
            .collect();
        let mut packs: HashMap<u32, u32> = HashMap::new();
        if !folded.is_empty() {
            // Down the seating from the ground, carrying the outermost fold met
            // on the way: what a reader can see is what stands for everything
            // under it.
            let mut stack: Vec<(u32, Option<u32>)> = seating
                .seats
                .iter()
                .chain(seating.ring.iter())
                .map(|&id| (id, None))
                .collect();
            while let Some((at, under)) = stack.pop() {
                let rep = under.or_else(|| folded.contains(&at).then_some(at));
                for &kid in seating.kids.get(&at).into_iter().flatten() {
                    if kid == at {
                        continue;
                    }
                    if let Some(rep) = rep {
                        packs.insert(kid, rep);
                    }
                    stack.push((kid, rep));
                }
            }
        }
        for mark in marks.iter_mut() {
            mark.folded = folded.contains(&mark.id);
        }
        for call in calls.iter_mut() {
            call.seats =
                call.kind == CallKind::Call && seating.via.get(&call.def) == Some(&call.user);
        }

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
            via: seating.via,
            kids: seating.kids,
            seats: seating.seats,
            ring: seating.ring,
            folded,
            packed: packed_folds,
            packs,
            bands,
            facts,
            touches,
            dirty,
        }
    }
}

/// The way-in tree, read over what one reading draws: which caller every
/// declaration shelves inside, what shelves inside each of them, how much each
/// frame carries, and the frames that stand on the ground.
struct Seating {
    via: HashMap<u32, u32>,
    kids: HashMap<u32, Vec<u32>>,
    reach: HashMap<u32, u32>,
    seats: Vec<u32>,
    ring: Vec<u32>,
}

impl Seating {
    fn read(
        marks: &[FnMark],
        calls: &[Call],
        order: FnOrder,
        written: &HashMap<u32, String>,
        owned: &HashMap<u32, String>,
    ) -> Self {
        let on: HashSet<u32> = marks.iter().map(|m| m.id).collect();
        let mut callees: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut callers: HashMap<u32, Vec<u32>> = HashMap::new();
        for call in calls {
            callees.entry(call.user).or_default().push(call.def);
            callers.entry(call.def).or_default().push(call.user);
        }
        for list in callees.values_mut().chain(callers.values_mut()) {
            list.sort_unstable();
            list.dedup();
        }

        // A breadth-first walk from what nothing drawn calls, in id order:
        // the first caller to reach a mark is its way in, and the tree is the
        // same for one survey however the shelves are later ordered.
        let mut ids: Vec<u32> = marks.iter().map(|m| m.id).collect();
        ids.sort_unstable();
        let mut via: HashMap<u32, u32> = HashMap::new();
        let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut order_seen: Vec<u32> = Vec::with_capacity(ids.len());
        let mut seen: HashSet<u32> = HashSet::new();
        let grow = |from: Vec<u32>,
                    via: &mut HashMap<u32, u32>,
                    kids: &mut HashMap<u32, Vec<u32>>,
                    seen: &mut HashSet<u32>,
                    order_seen: &mut Vec<u32>| {
            let mut queue: VecDeque<u32> = VecDeque::new();
            for id in from {
                if seen.insert(id) {
                    order_seen.push(id);
                    queue.push_back(id);
                }
            }
            while let Some(at) = queue.pop_front() {
                for &callee in callees.get(&at).into_iter().flatten() {
                    if !on.contains(&callee) || !seen.insert(callee) {
                        continue;
                    }
                    via.insert(callee, at);
                    kids.entry(at).or_default().push(callee);
                    order_seen.push(callee);
                    queue.push_back(callee);
                }
            }
        };
        let ground: Vec<u32> = ids
            .iter()
            .copied()
            .filter(|id| callers.get(id).is_none_or(|list| list.is_empty()))
            .collect();
        let mut seats = ground.clone();
        grow(ground, &mut via, &mut kids, &mut seen, &mut order_seen);
        // What is left is in a call ring: nothing reaches it from the ground.
        // Each ring is grown from its lowest id, so the ring strip holds one
        // frame per ring rather than one block per mark caught in one.
        let mut ring: Vec<u32> = Vec::new();
        for &id in &ids {
            if seen.contains(&id) {
                continue;
            }
            ring.push(id);
            grow(vec![id], &mut via, &mut kids, &mut seen, &mut order_seen);
        }

        // What each frame carries, deepest first: a mark's own weight is every
        // block shelved inside it, however deep.
        let mut reach: HashMap<u32, u32> = HashMap::new();
        for &id in order_seen.iter().rev() {
            let carried = kids
                .get(&id)
                .into_iter()
                .flatten()
                .map(|kid| 1 + reach.get(kid).copied().unwrap_or(0))
                .sum();
            reach.insert(id, carried);
        }

        // Only now does the reading decide what reads first on a shelf. The
        // cluster is the reading's; the weight breaks ties inside it, so every
        // stop moves the shelves and none of them moves the tree.
        let key = |id: u32| -> (Option<&String>, std::cmp::Reverse<u32>, u32) {
            (
                match order {
                    FnOrder::Weight => None,
                    FnOrder::Module => written.get(&id),
                    FnOrder::Owner => owned.get(&id),
                },
                std::cmp::Reverse(reach.get(&id).copied().unwrap_or(0)),
                id,
            )
        };
        for shelf in kids.values_mut() {
            shelf.sort_by_key(|&id| key(id));
        }
        seats.sort_by_key(|&id| key(id));
        ring.sort_by_key(|&id| key(id));
        Seating {
            via,
            kids,
            reach,
            seats,
            ring,
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
            vis_floor,
            order: FnOrder::default(),
            folds: Default::default(),
            packed: Default::default(),
        }
    }

    /// The same reading with some frames folded by hand — elided in place, which
    /// is what a fold does: nothing is packed away.
    fn folding(folds: &[(&str, &str)]) -> FnReading {
        FnReading {
            vis_floor: VisFloor::All,
            order: FnOrder::default(),
            folds: folds
                .iter()
                .map(|(path, label)| fold_key(path, label))
                .collect(),
            packed: Default::default(),
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
        // On the paper the ring is one frame in the strip, not two loose
        // blocks: the lowest id stands, and what it calls shelves inside it.
        assert_eq!(model.ring, vec![4]);
        assert_eq!(model.kids.get(&4), Some(&vec![5]));
        assert_eq!(model.via.get(&5), Some(&4));
    }

    /// Containment is the call: what a mark reaches first shelves inside its
    /// frame, and that call takes no ink, because the paper already says it.
    #[test]
    fn the_way_in_seats_a_declaration_inside_its_caller() {
        let model = FnModel::build(&chain(), &reading(VisFloor::All));
        // `main` is the only frame on the ground; `walk` shelves in it and
        // `note` in `walk`.
        assert_eq!(model.seats, vec![0]);
        assert!(model.ring.is_empty());
        assert_eq!(model.kids.get(&0), Some(&vec![1]));
        assert_eq!(model.kids.get(&1), Some(&vec![2]));
        assert_eq!(model.ancestors(2), HashSet::from([0, 1]));
        assert_eq!(model.subtree(0), HashSet::from([0, 1, 2]));
        // What each frame carries, however deep — the weight a shelf orders by.
        let runs = |id: u32| model.marks.iter().find(|m| m.id == id).unwrap().runs;
        assert_eq!((runs(0), runs(1), runs(2)), (2, 1, 0));
        // Both calls are the way in, so neither is drawn.
        assert!(model.calls.iter().all(|c| c.seats));
    }

    /// A call the shelving cannot say is the ink this chart spends, and the
    /// model hands every one of them over with its two ends named. *Which* of
    /// them the paper draws is the `wires` reading's answer, not the model's —
    /// the direction is read against whatever is in focus, and the model cannot
    /// know what that is.
    #[test]
    fn a_call_the_shelving_cannot_say_is_drawn_and_the_model_names_both_ends() {
        // `main` reaches four helpers first, so all four seat inside it — and
        // every call between two helpers is a call the shelving cannot say.
        let mut graph = chain();
        graph.items.truncate(1);
        for (id, name) in [(1u32, "a"), (2, "b"), (3, "c"), (4, "d")] {
            graph
                .items
                .push(item(id, name, ItemKind::Fn, Vis::Pub, None));
        }
        graph.refs.clear();
        for (from, to, count) in [
            (0, 1, 1),
            (0, 2, 1),
            (0, 3, 1),
            (0, 4, 1),
            (1, 3, 10),
            (1, 4, 1),
            (2, 3, 5),
            (2, 4, 7),
        ] {
            graph.refs.push(MarkRef { from, to, count });
        }
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        assert_eq!(model.seats, vec![0]);
        assert_eq!(model.kids.get(&0), Some(&vec![1, 2, 3, 4]));

        let drawn: Vec<(u32, u32, u32)> = model
            .calls
            .iter()
            .filter(|c| !c.seats)
            .map(|c| (c.user, c.def, c.count))
            .collect();
        // The four ways in take no ink; the four calls between helpers do, each
        // with the count the survey resolved for the pair — the weight the wire
        // is engraved at, and the only thinning left on this chart.
        assert_eq!(model.calls.iter().filter(|c| c.seats).count(), 4);
        assert_eq!(
            drawn,
            vec![(1, 3, 10), (2, 3, 5), (1, 4, 1), (2, 4, 7)],
            "every call the shelving cannot say is handed over, both ends named"
        );
    }

    /// Its module is a word on the head only where the seating crosses one:
    /// a same-module call is quiet, and a frame reaching into another module
    /// says so.
    #[test]
    fn a_head_says_its_module_only_where_the_seating_crosses_one() {
        let mut graph = chain();
        graph.files.push(crate::graph::data::FileInfo {
            path: "src/load.rs".to_string(),
            krate: "slope".to_string(),
        });
        let mut loaded = item(4, "load", ItemKind::Fn, Vis::Pub, None);
        loaded.file = 1;
        graph.items.push(loaded);
        // `walk` calls `load`, which is written in another module.
        graph.refs.push(MarkRef {
            from: 1,
            to: 4,
            count: 1,
        });
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        let mark = |name: &str| model.marks.iter().find(|m| m.head.name == name).unwrap();
        assert_eq!(mark("load").written(), "load");
        assert_eq!(mark("load").mod_key(), vec!["slope", "load"]);
        assert!(mark("load").crosses, "seated under a caller in `slope`");
        assert_eq!(mark("main").written(), "slope", "the crate at its root");
        assert!(!mark("walk").crosses, "`main` calls it from its own module");
        // A frame on the ground has no caller to cross.
        assert!(!mark("main").crosses);
    }

    /// The order is a reading of the shelves, never of the tree: `weight` seats
    /// the heaviest chain first, `module` clusters siblings by where they are
    /// written, and both hold the same seating.
    #[test]
    fn the_order_reads_a_shelf_without_moving_the_tree() {
        let mut graph = chain();
        graph.files.push(crate::graph::data::FileInfo {
            path: "src/load.rs".to_string(),
            krate: "slope".to_string(),
        });
        // `main` also calls `load`, written in another module and carrying
        // nothing — so it reads last by weight and first by module.
        let mut loaded = item(4, "load", ItemKind::Fn, Vis::Pub, None);
        loaded.file = 1;
        graph.items.push(loaded);
        graph.refs.push(MarkRef {
            from: 0,
            to: 4,
            count: 1,
        });
        let shelf = |order: FnOrder| {
            let model = FnModel::build(
                &graph,
                &FnReading {
                    vis_floor: VisFloor::All,
                    order,
                    folds: Default::default(),
                    packed: Default::default(),
                },
            );
            model.kids.get(&0).cloned().unwrap_or_default()
        };
        // By weight, `walk` (which carries `note`) reads before `load`.
        assert_eq!(shelf(FnOrder::Weight), vec![1, 4]);
        // By module, `load` — written in `load` — reads before `slope`'s own.
        assert_eq!(shelf(FnOrder::Module), vec![4, 1]);
    }

    /// `owner` clusters a shelf by the type whose impl each sibling is written
    /// in, free declarations first, with the weight still breaking ties inside
    /// a cluster — and it moves the shelf that `weight` and `module` do not.
    #[test]
    fn the_owner_order_clusters_a_shelf_by_whose_method_it_is() {
        let mut graph = chain();
        graph.items.truncate(1);
        // `main` calls a free function and two methods of two types, all in one
        // module, so only the owner reading can tell them apart.
        graph
            .items
            .push(item(1, "Plate", ItemKind::Struct, Vis::Pub, None));
        graph
            .items
            .push(item(2, "Wire", ItemKind::Struct, Vis::Pub, None));
        for (id, name, parent) in [
            (3u32, "Wire::draw", Some(2)),
            (4, "Plate::rule", Some(1)),
            (5, "note", None),
        ] {
            let mut mark = item(id, name, ItemKind::Fn, Vis::Pub, parent);
            mark.head.name = name.rsplit("::").next().unwrap_or(name).to_string();
            graph.items.push(mark);
        }
        graph.refs.clear();
        for (from, to, count) in [(0, 3, 1), (0, 4, 1), (0, 5, 1)] {
            graph.refs.push(MarkRef { from, to, count });
        }
        let shelf = |order: FnOrder| {
            let model = FnModel::build(
                &graph,
                &FnReading {
                    vis_floor: VisFloor::All,
                    order,
                    folds: Default::default(),
                    packed: Default::default(),
                },
            );
            model.kids.get(&0).cloned().unwrap_or_default()
        };
        // Nothing to cluster on: the ids break the tie, in seating order.
        assert_eq!(shelf(FnOrder::Weight), vec![3, 4, 5]);
        // By owner: the free `note` first, then `Plate::rule`, then `Wire::draw`.
        assert_eq!(shelf(FnOrder::Owner), vec![5, 4, 3]);
        // And the owner is read off the survey's own label and mark.
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        let draw = model.marks.iter().find(|m| m.id == 3).unwrap();
        assert_eq!(draw.qualifier(), "Wire");
        let owner = draw.owner.as_ref().expect("a method has an owner");
        assert_eq!((owner.ty, owner.name.as_str()), (2, "Wire"));
        assert!(owner.on_data, "a struct has a block one rung down");
        let free = model.marks.iter().find(|m| m.id == 5).unwrap();
        assert_eq!(free.qualifier(), "");
        assert!(free.owner.is_none());
    }

    /// A fold is the reviewer's own gesture: it takes what shelves inside one
    /// frame off the paper, leaves the frame's own head standing for it, and
    /// says how many in words.
    #[test]
    fn a_folded_frame_stands_for_everything_it_hides() {
        let model = FnModel::build(&chain(), &folding(&[("src/main.rs", "walk")]));
        let walk = model.marks.iter().find(|m| m.head.name == "walk").unwrap();
        assert!(walk.folded);
        assert_eq!(walk.runs, 1, "the count it states in words");
        // `note` is off the paper, and `walk`'s head stands for it.
        assert!(model.hidden(2));
        assert_eq!(model.shown(2), 1);
        // The fold hides nothing above itself, and nothing outside it.
        assert!(!model.hidden(1));
        assert_eq!(model.shown(0), 0);
        // The seating is untouched: a fold is a re-layout, not a re-reading.
        assert_eq!(model.kids.get(&1), Some(&vec![2]));
        assert_eq!(model.folded, HashSet::from([1]));
    }

    /// Nothing folds itself. A frame nobody folded hides nothing, whatever it
    /// carries, and a fold naming a frame that shelves nothing draws no mark.
    #[test]
    fn nothing_folds_by_itself_or_by_a_count() {
        let open = FnModel::build(&chain(), &reading(VisFloor::All));
        assert!(open.folded.is_empty());
        assert!(open.packs.is_empty());
        // `note` shelves nothing, so folding it is not a thing the chart draws.
        let leaf = FnModel::build(&chain(), &folding(&[("src/main.rs", "note")]));
        assert!(leaf.folded.is_empty(), "a leaf has nothing to fold");
        assert!(leaf.packs.is_empty());
    }

    /// The outermost fold is what stands on the paper: a fold inside a fold is
    /// spoken for by the one the reader can actually see.
    #[test]
    fn the_outermost_fold_stands_for_the_folds_inside_it() {
        let model = FnModel::build(
            &chain(),
            &folding(&[("src/main.rs", "main"), ("src/main.rs", "walk")]),
        );
        assert_eq!(model.folded, HashSet::from([0, 1]));
        // Both `walk` and `note` are hidden, and `main` stands for both.
        assert_eq!(model.shown(1), 0);
        assert_eq!(model.shown(2), 0);
    }

    /// A selection the reader cannot see is not a focus, so the way to a mark
    /// names every fold it is hiding behind, outermost first.
    #[test]
    fn a_reveal_names_every_fold_on_the_way_in() {
        let model = FnModel::build(
            &chain(),
            &folding(&[("src/main.rs", "main"), ("src/main.rs", "walk")]),
        );
        assert_eq!(
            model.reveal(2),
            vec![
                fold_key("src/main.rs", "main"),
                fold_key("src/main.rs", "walk"),
            ]
        );
        // A mark standing on open paper needs nothing unfolded, and a folded
        // frame's own head is drawn — so nothing unfolds to reach it either.
        assert!(model.reveal(0).is_empty());
        let one = FnModel::build(&chain(), &folding(&[("src/main.rs", "walk")]));
        assert!(one.reveal(1).is_empty(), "a folded frame is on the paper");
        assert_eq!(one.reveal(2), vec![fold_key("src/main.rs", "walk")]);
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

    /// A mark whose only caller the reading leaves off the paper still has a
    /// seat: it stands on the ground, and its tier still says how far in it is.
    #[test]
    fn a_mark_whose_way_in_is_off_the_paper_stands_on_the_ground() {
        let mut graph = chain();
        // `walk` goes private and `note` widens, so nothing this reading draws
        // calls `note` any more.
        graph.items[1].head.vis = Vis::Private;
        graph.items[2].head.vis = Vis::Crate;
        let model = FnModel::build(&graph, &reading(VisFloor::Crate));
        let mark = |name: &str| model.marks.iter().find(|m| m.head.name == name).unwrap();
        assert_eq!(mark("note").tier, Tier::Deep(2), "the workspace's own fact");
        assert_eq!(model.seats, vec![0, 2], "both stand on the ground");
        assert!(!model.via.contains_key(&2));
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
