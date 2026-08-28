//! What the function chart reads out of the survey: the code that runs, drawn
//! in the household it is written in.
//!
//! The rung above asks what the workspace *keeps*; this one asks what it
//! *does*. Its marks are the declarations that run — every function, every
//! method, every trait clause a method answers, every `macro_rules!` — and its
//! one organizing move is **ownership**: a method belongs to the type or trait
//! whose impl block declares it, and a free declaration belongs to its module.
//! So the ground is a household: a crate frame, module frames nested the way
//! rust's modules nest, and inside a module one **container** per owner that
//! declares methods (`impl FnModel`, `trait Chart`), with the free declarations
//! on the module's own shelf.
//!
//! An **entry point** is still a declaration nothing in the workspace calls:
//! `main`, a server function the client reaches through generated code, a
//! component the router mounts, a method answering a foreign trait's contract.
//! It wears the root's ink left edge wherever the household seats it — the tier
//! is a fact about the code, and since 2026-08-27 it is no longer a place.
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
//!
//! The visibility reading narrows the **paper** and nothing else. A sheet lists
//! every caller, every callee and every method written on a container, whatever
//! rung it is written at (2026-08-28, user): the slider is how a reviewer
//! navigates a graph too big to read at once, not a redaction. So the model
//! carries both — [`FnModel::calls`], the wires the paper draws, and
//! [`FnModel::all_calls`], every call the survey resolved.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::data::{
    CodeGraph, DeclRow, Delta, HoldEdge, ImplEdge, ItemKind, ItemMark, MarkRef, MethodRow, Vis,
};
use crate::views::chrome::plural;
use crate::views::data::model::module_path;
use crate::views::func::{FnReading, FoldAt, mod_fold, owner_fold};

/// One box on the paper: a declaration's own block, an owner container, or a
/// module frame. Wires tie to these rather than to marks, because a fold takes
/// marks off the paper and the box that folded stands for them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub(super) enum Spot {
    /// One declaration's block, by its mark id.
    Mark(u32),
    /// One owner container, by its index in [`FnModel::owners`].
    Owner(u32),
    /// One crate or module frame, by its index in [`FnModel::frames`].
    Frame(u32),
}

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
    /// the module path rust reads it at. The key `/fn/mod/:..module` selects
    /// by, and — for a free declaration — the frame it is drawn in. A method is
    /// drawn in its **owner's** module, which is where the type is declared,
    /// and this stays the module the method's own source is written in.
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
    /// How many declarations this one reaches by the way in — the shortest way
    /// something that starts gets to each of them. The household no longer
    /// *places* anything by it, but it is still what a reviewer means by "how
    /// much runs under this", so it stays in the hover words and on the sheet.
    pub(super) runs: u32,
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
        if self.runs > 0 {
            words.push_str(&format!(" · runs {} by the way in", self.runs));
        }
        if self.recurses {
            words.push_str(" · it calls itself");
        }
        words
    }

    /// The module as rust writes it — `views::func`, or the crate's own name
    /// at its root. What prose away from the paper spells out, and where a free
    /// declaration is drawn.
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

/// One crate or module frame: the household's own rooms, nested exactly as
/// rust's modules nest. The same ground the data chart draws one rung up, and
/// keyed the same way, so a reviewer reading one module at two altitudes says
/// one word for it.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Frame {
    pub(super) id: u32,
    pub(super) krate: String,
    /// The module path, segment by segment, as rust names it: `["views",
    /// "func"]` is `mod views::func`. Empty is the crate's own frame, which
    /// holds what the crate root declares.
    pub(super) module: Vec<String>,
    /// The frame this one sits inside: the module one segment up, or the crate
    /// frame for a top-level module. `None` only on a crate frame.
    pub(super) parent: Option<u32>,
    /// The free declarations written here, in declaration order.
    pub(super) marks: Vec<u32>,
    /// The owner containers whose type or trait is declared here, in
    /// declaration order.
    pub(super) owners: Vec<u32>,
    /// The module frames nested inside it, in module order.
    pub(super) kids: Vec<u32>,
    /// The reviewer folded it by hand.
    pub(super) folded: bool,
    /// Every declaration inside it, however deep — what a fold states in words,
    /// and the weight the landmark register reads.
    pub(super) held: u32,
}

impl Frame {
    /// The label chipped onto the border, in rust's own words. A module frame
    /// wears its last segment alone — `mod func`, drawn inside `mod views` —
    /// because that is how rust writes it in the file, and the paper's own
    /// nesting says the rest of the path. A crate frame names its crate only
    /// where the survey has more than one to tell apart.
    pub(super) fn label(&self, multi_crate: bool) -> Option<String> {
        match self.module.last() {
            Some(segment) => Some(format!("mod {segment}")),
            None => multi_crate.then(|| self.krate.clone()),
        }
    }

    /// This frame's name in a [`crate::views::func::FnFolds`] set, and in the
    /// URL that selects it.
    pub(super) fn key(&self) -> Vec<String> {
        let mut key = vec![self.krate.clone()];
        key.extend(self.module.iter().cloned());
        key
    }

    /// The frame in prose, where no paper around it says which one it is: the
    /// whole path as rust would write it in a `use` line (`views::func`), or the
    /// crate's own name. The border's chip says `mod map`, and three modules in
    /// this workspace answer to that.
    pub(super) fn words(&self) -> String {
        match self.module.is_empty() {
            true => self.krate.clone(),
            false => self.module.join("::"),
        }
    }
}

/// One owner container: every method the workspace writes on one type or trait,
/// gathered from every impl block wherever it is written, standing in the module
/// the type itself is declared in.
///
/// This is the household's own answer to "show me this type's methods together",
/// and since 2026-08-27 it is a **box** rather than a shelf order: the data chart
/// draws what a type keeps, and this draws what it does.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Container {
    pub(super) id: u32,
    /// The module frame it stands in: the one the owner type is declared in.
    pub(super) frame: u32,
    /// The owner's own mark, for the descent link.
    pub(super) ty: u32,
    /// `impl` for a type's methods, `trait` for a trait's own clauses — rust's
    /// own word for the block those methods are written in.
    pub(super) decl: &'static str,
    pub(super) name: String,
    pub(super) kind: ItemKind,
    pub(super) vis: Vis,
    pub(super) path: String,
    pub(super) label: String,
    /// Whether the data chart draws a block for the owner: a struct, an enum, a
    /// union or a static stands there; a trait does not, and its name opens as
    /// this container's own sheet instead.
    pub(super) on_data: bool,
    /// Its methods, in declaration order — (file, line), which is the order the
    /// source writes them in.
    pub(super) marks: Vec<u32>,
    /// The reviewer folded it by hand.
    pub(super) folded: bool,
}

impl Container {
    /// The container as the paper labels it: `impl FnModel`, `trait Chart`.
    pub(super) fn words(&self) -> String {
        format!("{} {}", self.decl, self.name)
    }

    /// This container's name in a [`crate::views::func::FnFolds`] set, and in
    /// the URL that selects it.
    pub(super) fn key(&self) -> FoldAt {
        owner_fold(&self.path, &self.label)
    }
}

/// Everything one build of the function chart reads out of the survey.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnModel {
    pub(super) marks: Vec<FnMark>,
    pub(super) calls: Vec<Call>,
    /// Every call the survey resolved, the reading's floor included — the two
    /// lists above are this one narrowed to what the paper draws. A sheet reads
    /// these: the slider folds the picture, and folding a picture is not the
    /// same as withholding a name (2026-08-28, user).
    pub(super) all_calls: Vec<Call>,
    /// What calls each declaration over that whole set, for the blast radius.
    pub(super) all_callers: HashMap<u32, Vec<u32>>,
    /// The crate and module frames, crate frames first and every frame after
    /// the one it sits inside.
    pub(super) frames: Vec<Frame>,
    /// The owner containers, in the order their frames hold them.
    pub(super) owners: Vec<Container>,
    /// Which box each drawn mark is written in: its owner's container, or its
    /// module's frame where it is a free declaration.
    pub(super) home: HashMap<u32, Spot>,
    /// What each mark calls, and what calls it, heaviest first. The arrow walk
    /// steps along these, and a wire's own weight orders them.
    pub(super) callees: HashMap<u32, Vec<u32>>,
    pub(super) callers: HashMap<u32, Vec<u32>>,
    /// The boxes the reviewer folded by hand. What is inside them is not on the
    /// paper; their own borders, labels and counted words are.
    pub(super) folded: HashSet<Spot>,
    /// Which of those folds the **packer** was allowed to skip. A folded box
    /// outside this set keeps its whole footprint, so folding it moved nothing
    /// else on the sheet. See [`crate::views::func::FnReading::packed`].
    pub(super) packed: HashSet<Spot>,
    /// Every box a fold hides, and the folded box that stands for it on the
    /// paper — what its wires re-anchor to, and what carries a lit chain's ink
    /// where the chain runs through the fold. The outermost fold wins.
    pub(super) packs: HashMap<Spot, Spot>,
    /// Every band the paper has, in order, with its caption.
    pub(super) bands: Vec<(u32, String)>,
    pub(super) facts: FnFacts,
    /// The types each mark touches, by mark id — read by the sheet only, so
    /// the chart never pays to draw what it does not draw.
    pub(super) touches: HashMap<u32, Vec<Touch>>,
    /// Whether the epoch touched anything this reading draws.
    pub(super) dirty: bool,
    /// Whether the survey has more than one crate to tell apart, which is the
    /// one thing a crate frame's label depends on.
    pub(super) multi_crate: bool,
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

    /// One owner container by the (file, label) its own URL names.
    pub(super) fn owner_at(&self, path: &str, label: &str) -> Option<&Container> {
        self.owners
            .iter()
            .find(|o| o.path == path && o.label == label)
    }

    /// One frame by the key `/fn/mod/:..module` carries.
    pub(super) fn frame_at(&self, key: &[String]) -> Option<&Frame> {
        self.frames.iter().find(|f| f.key() == key)
    }

    /// The module frame one box stands in — a container's own frame, a frame
    /// itself, and a mark's home read through its container.
    pub(super) fn frame_of(&self, spot: Spot) -> Option<u32> {
        match spot {
            Spot::Frame(id) => Some(id),
            Spot::Owner(id) => self.owners.get(id as usize).map(|o| o.frame),
            Spot::Mark(id) => match self.home.get(&id)? {
                Spot::Frame(at) => Some(*at),
                Spot::Owner(at) => self.owners.get(*at as usize).map(|o| o.frame),
                Spot::Mark(_) => None,
            },
        }
    }

    /// Every frame this one stands inside, outward.
    pub(super) fn over(&self, frame: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut at = self.frames.get(frame as usize).and_then(|f| f.parent);
        while let Some(up) = at {
            if out.contains(&up) {
                break;
            }
            out.push(up);
            at = self.frames.get(up as usize).and_then(|f| f.parent);
        }
        out
    }

    /// Every mark one box holds: a container's methods, a frame's free
    /// declarations and everything the boxes nested inside it hold, a mark
    /// itself.
    pub(super) fn inside(&self, spot: Spot) -> HashSet<u32> {
        match spot {
            Spot::Mark(id) => HashSet::from([id]),
            Spot::Owner(id) => self
                .owners
                .get(id as usize)
                .map(|o| o.marks.iter().copied().collect())
                .unwrap_or_default(),
            Spot::Frame(id) => {
                let mut out: HashSet<u32> = HashSet::new();
                let mut stack = vec![id];
                while let Some(at) = stack.pop() {
                    let Some(frame) = self.frames.get(at as usize) else {
                        continue;
                    };
                    out.extend(frame.marks.iter().copied());
                    for &owner in &frame.owners {
                        if let Some(owner) = self.owners.get(owner as usize) {
                            out.extend(owner.marks.iter().copied());
                        }
                    }
                    stack.extend(frame.kids.iter().copied());
                }
                out
            }
        }
    }

    /// Whether a fold has this box off the paper.
    pub(super) fn hidden(&self, spot: Spot) -> bool {
        self.packs.contains_key(&spot)
    }

    /// The box that stands for another on the paper: itself where it is drawn,
    /// and the outermost fold hiding it where it is not. Every wire and every
    /// lit chain reads through this, so a fold re-anchors ink instead of
    /// cutting it.
    pub(super) fn shown(&self, spot: Spot) -> Spot {
        self.packs.get(&spot).copied().unwrap_or(spot)
    }

    /// The folds standing between the paper and one box, outermost first — what
    /// a reveal has to open before it can be seen. A selection the reader cannot
    /// see is not a focus, so every way to a mark (a URL, the search, a sheet
    /// row, the arrow walk) opens the boxes it hides behind first.
    pub(super) fn reveal(&self, spot: Spot) -> Vec<FoldAt> {
        if self.folded.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<FoldAt> = Vec::new();
        // The frames standing over it, outermost first. A box's **own** border
        // is on the paper whatever its fold says — a folded module still draws
        // its boundary and its counted words — so nothing here ever asks a box
        // to open itself.
        let chain: Vec<u32> = match spot {
            Spot::Frame(id) => {
                let mut chain = self.over(id);
                chain.reverse();
                chain
            }
            other => {
                let Some(frame) = self.frame_of(other) else {
                    return out;
                };
                let mut chain = self.over(frame);
                chain.reverse();
                chain.push(frame);
                chain
            }
        };
        for id in chain {
            if self.folded.contains(&Spot::Frame(id))
                && let Some(frame) = self.frames.get(id as usize)
            {
                out.push(mod_fold(&frame.key()));
            }
        }
        // Then the container it is written in, which is the innermost box a
        // mark can hide behind. A container's own border is on the paper, so a
        // container asks for nothing of itself.
        if let Spot::Mark(id) = spot
            && let Some(Spot::Owner(at)) = self.home.get(&id)
            && self.folded.contains(&Spot::Owner(*at))
            && let Some(owner) = self.owners.get(*at as usize)
        {
            out.push(owner.key());
        }
        out
    }

    /// Every mark a rewrite of `from` could reach, walking callers outward: the
    /// transitive callers, and the methods answering a clause it answers. The
    /// blast radius, in the same sense the two rungs above use the word.
    ///
    /// Over the whole call graph, never the reading's: a rewrite reaches what
    /// it reaches, and a private caller two hops out is exactly the one a
    /// narrowed walk would have promised was not there.
    pub(super) fn upstream(&self, from: u32) -> HashSet<u32> {
        let mut seen: HashSet<u32> = HashSet::new();
        let mut queue: VecDeque<u32> = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            for &user in self.all_callers.get(&at).into_iter().flatten() {
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
        // calls itself has not been started by anything. The household places
        // nothing by the tier any more — it is a fact about the code, said in
        // words on the mark and on its sheet — but an entry point still wears
        // the root's edge, and the bands are still a focus.
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
                recurses: recurses.contains(&item.id),
            });
        }
        // Declaration order, as the source writes it: the file, then the line.
        // Nothing here alphabetizes — the line number is the honest order, and
        // it is the order every shelf on the paper reads in.
        marks.sort_by(|a, b| {
            (&a.head.path, a.head.line, a.id).cmp(&(&b.head.path, b.head.line, b.id))
        });

        // ---- What each mark touches, one rung down. ------------------------
        let touches = Touch::read(graph, &marks, method_row);
        for mark in marks.iter_mut() {
            mark.touches = touches.get(&mark.id).map_or(0, Vec::len);
        }

        // ---- The ink, whole and then narrowed to what the reading draws. ---
        //
        // Every call the survey resolved, whatever the slider is holding. The
        // reading is how a reviewer navigates a graph too big to read at once,
        // and never a redaction (2026-08-28, user): the paper narrows, and a
        // declaration opened by name is owed every end that touches it.
        let mut all_calls: Vec<Call> = pairs
            .iter()
            .map(|(&(def, user), &count)| Call {
                def,
                user,
                kind: CallKind::Call,
                count,
            })
            .collect();
        all_calls.extend(answers.iter().map(|&(clause, answer)| Call {
            def: clause,
            user: answer,
            kind: CallKind::Answers,
            count: 0,
        }));
        all_calls.sort_by_key(|c| (c.def, c.user, c.kind == CallKind::Answers));

        // Every call is a wire now (2026-08-27, user). The shelved section drew
        // the way-in call as containment and spent ink only on the rest; the
        // household's containment says *whose code this is*, which no call can
        // stand for, so a call that is not drawn is a call not said.
        let calls: Vec<Call> = all_calls
            .iter()
            .filter(|c| drawn.contains(&c.def) && drawn.contains(&c.user))
            .cloned()
            .collect();

        // What each mark calls and what calls it, over what this reading draws,
        // heaviest first: the order the arrow walk steps in, and the order every
        // list of ends on a sheet reads in.
        let (mut callees, mut callers) = (
            HashMap::<u32, Vec<(u32, u32)>>::new(),
            HashMap::<u32, Vec<(u32, u32)>>::new(),
        );
        for call in &calls {
            callees
                .entry(call.user)
                .or_default()
                .push((call.count, call.def));
            callers
                .entry(call.def)
                .or_default()
                .push((call.count, call.user));
        }
        let rank = |list: HashMap<u32, Vec<(u32, u32)>>| -> HashMap<u32, Vec<u32>> {
            list.into_iter()
                .map(|(at, mut ends)| {
                    ends.sort_by_key(|&(count, id)| (std::cmp::Reverse(count), id));
                    (at, ends.into_iter().map(|(_, id)| id).collect())
                })
                .collect()
        };
        let (callees, callers) = (rank(callees), rank(callers));

        // ---- How much runs under each mark, by the way in. ------------------
        for mark in marks.iter_mut() {
            mark.runs = 0;
        }
        let reach = way_in_reach(&marks, &callees, &callers);
        for mark in marks.iter_mut() {
            mark.runs = reach.get(&mark.id).copied().unwrap_or(0);
        }

        // ---- The household: crates, modules, owner containers. --------------
        let house = House::read(graph, &marks, reading);

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
            all_calls,
            all_callers: callers_of,
            frames: house.frames,
            owners: house.owners,
            home: house.home,
            callees,
            callers,
            folded: house.folded,
            packed: house.packed,
            packs: house.packs,
            bands,
            facts,
            touches,
            dirty,
            multi_crate: house.multi_crate,
        }
    }
}

/// How much each mark reaches by the **way in** — the shortest way something
/// that starts gets to it, and everything that hangs off that way.
///
/// The shelved section seated the paper by this tree; the household does not.
/// It stays because "runs 41 by the way in" is still the sentence a reviewer
/// reads off an entry point, and a number the chart states has to be a number
/// the chart computed.
fn way_in_reach(
    marks: &[FnMark],
    callees: &HashMap<u32, Vec<u32>>,
    callers: &HashMap<u32, Vec<u32>>,
) -> HashMap<u32, u32> {
    let on: HashSet<u32> = marks.iter().map(|m| m.id).collect();
    let mut ids: Vec<u32> = marks.iter().map(|m| m.id).collect();
    ids.sort_unstable();
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut seen: HashSet<u32> = HashSet::new();
    let mut order: Vec<u32> = Vec::with_capacity(ids.len());
    let grow = |from: Vec<u32>,
                kids: &mut HashMap<u32, Vec<u32>>,
                seen: &mut HashSet<u32>,
                order: &mut Vec<u32>| {
        let mut queue: VecDeque<u32> = VecDeque::new();
        for id in from {
            if seen.insert(id) {
                order.push(id);
                queue.push_back(id);
            }
        }
        while let Some(at) = queue.pop_front() {
            for &callee in callees.get(&at).into_iter().flatten() {
                if !on.contains(&callee) || !seen.insert(callee) {
                    continue;
                }
                kids.entry(at).or_default().push(callee);
                order.push(callee);
                queue.push_back(callee);
            }
        }
    };
    let ground: Vec<u32> = ids
        .iter()
        .copied()
        .filter(|id| callers.get(id).is_none_or(|list| list.is_empty()))
        .collect();
    grow(ground, &mut kids, &mut seen, &mut order);
    // What is left is in a call ring: nothing on the paper reaches it. Each ring
    // is grown from its lowest id, so a ring counts once rather than per mark.
    for &id in &ids {
        if seen.contains(&id) {
            continue;
        }
        grow(vec![id], &mut kids, &mut seen, &mut order);
    }
    let mut reach: HashMap<u32, u32> = HashMap::new();
    for &id in order.iter().rev() {
        let carried = kids
            .get(&id)
            .into_iter()
            .flatten()
            .map(|kid| 1 + reach.get(kid).copied().unwrap_or(0))
            .sum();
        reach.insert(id, carried);
    }
    reach
}

/// The household, read over what one reading draws: the crate and module
/// frames, the owner containers inside them, where each mark is written, and
/// what the reviewer's folds hide.
struct House {
    frames: Vec<Frame>,
    owners: Vec<Container>,
    home: HashMap<u32, Spot>,
    folded: HashSet<Spot>,
    packed: HashSet<Spot>,
    packs: HashMap<Spot, Spot>,
    multi_crate: bool,
}

impl House {
    fn read(graph: &CodeGraph, marks: &[FnMark], reading: &FnReading) -> Self {
        // Which module each drawn mark belongs to. A **method** belongs to the
        // module its owner type is declared in, wherever the impl block that
        // writes it happens to sit — the container gathers every impl block a
        // type has, so it can only stand in one place, and the type's own module
        // is the place the data chart draws that type in.
        let owner_ty = |id: u32| -> Option<u32> {
            let item = graph.item(id)?;
            let parent = item.parent?;
            let ty = graph.item(parent)?;
            graph.file(ty.file)?;
            Some(parent)
        };
        let module_of_file = |file: u32| -> Option<(String, Vec<String>)> {
            let file = graph.file(file)?;
            Some(module_of(&file.krate, &file.path))
        };

        // ---- The rooms. -----------------------------------------------------
        //
        // A frame is drawn only where this reading has something to put in it,
        // exactly as the data chart draws none for an empty module: a husk with
        // nothing admitted is a box saying a thing the reading is not.
        let mut keys: Vec<(String, Vec<String>)> = Vec::new();
        let mut key_of_mark: HashMap<u32, (String, Vec<String>)> = HashMap::new();
        let mut ty_of_mark: HashMap<u32, u32> = HashMap::new();
        for mark in marks {
            let key = match owner_ty(mark.id) {
                Some(ty) => {
                    ty_of_mark.insert(mark.id, ty);
                    graph
                        .item(ty)
                        .and_then(|item| module_of_file(item.file))
                        .unwrap_or_else(|| (mark.krate.clone(), mark.module.clone()))
                }
                None => (mark.krate.clone(), mark.module.clone()),
            };
            keys.push(key.clone());
            key_of_mark.insert(mark.id, key);
        }
        // Every module on the way to a drawn one is a frame too: `views::func`
        // is drawn inside `views`, which is drawn inside the crate.
        let ancestors: Vec<(String, Vec<String>)> = keys
            .iter()
            .flat_map(|(krate, path)| {
                (0..path.len()).map(|cut| (krate.clone(), path[..cut].to_vec()))
            })
            .collect();
        keys.extend(ancestors);
        keys.sort();
        keys.dedup();
        let mut crates: Vec<String> = keys.iter().map(|(krate, _)| krate.clone()).collect();
        crates.dedup();
        let multi_crate = crates.len() > 1;

        let folds = &reading.folds;
        let mut frames: Vec<Frame> = Vec::new();
        let mut index: HashMap<(String, Vec<String>), u32> = HashMap::new();
        // Sorted, a path always follows the path it extends, so a parent is
        // always already indexed.
        for key in &keys {
            let id = frames.len() as u32;
            let parent = match key.1.is_empty() {
                true => None,
                false => index
                    .get(&(key.0.clone(), key.1[..key.1.len() - 1].to_vec()))
                    .copied(),
            };
            let mut key_words = vec![key.0.clone()];
            key_words.extend(key.1.iter().cloned());
            frames.push(Frame {
                id,
                krate: key.0.clone(),
                module: key.1.clone(),
                parent,
                marks: Vec::new(),
                owners: Vec::new(),
                kids: Vec::new(),
                folded: folds.contains(&mod_fold(&key_words)),
                held: 0,
            });
            index.insert(key.clone(), id);
            if let Some(parent) = parent {
                frames[parent as usize].kids.push(id);
            }
        }

        // ---- The containers. ------------------------------------------------
        let mut owners: Vec<Container> = Vec::new();
        let mut owner_index: HashMap<u32, u32> = HashMap::new();
        let mut home: HashMap<u32, Spot> = HashMap::new();
        for mark in marks {
            let Some(&frame) = key_of_mark.get(&mark.id).and_then(|key| index.get(key)) else {
                continue;
            };
            let Some(&ty) = ty_of_mark.get(&mark.id) else {
                frames[frame as usize].marks.push(mark.id);
                home.insert(mark.id, Spot::Frame(frame));
                continue;
            };
            let at = match owner_index.get(&ty) {
                Some(&at) => at,
                None => {
                    let Some(item) = graph.item(ty) else { continue };
                    let Some(file) = graph.file(item.file) else {
                        continue;
                    };
                    let id = owners.len() as u32;
                    let container = Container {
                        id,
                        frame,
                        ty,
                        // A trait declares its own clauses; everything else has
                        // its methods written in `impl` blocks.
                        decl: match item.head.kind {
                            ItemKind::Trait => "trait",
                            _ => "impl",
                        },
                        name: item.head.name.clone(),
                        kind: item.head.kind,
                        vis: item.head.vis.clone(),
                        path: file.path.clone(),
                        label: item.head.label.clone(),
                        on_data: item.head.kind.is_data() && item.parent.is_none(),
                        marks: Vec::new(),
                        folded: false,
                    };
                    let folded = folds.contains(&container.key());
                    owners.push(Container {
                        folded,
                        ..container
                    });
                    owner_index.insert(ty, id);
                    frames[frame as usize].owners.push(id);
                    id
                }
            };
            owners[at as usize].marks.push(mark.id);
            home.insert(mark.id, Spot::Owner(at));
        }
        // Declaration order everywhere: the containers by where their type is
        // written, the methods by where they are written. `marks` arrived in
        // that order, so the shelves are already in it; the containers are
        // sorted here because they are seeded in mark order rather than in
        // type order.
        let ty_line = |id: u32| -> (String, u32) {
            graph
                .item(id)
                .map(|item| {
                    (
                        graph
                            .file(item.file)
                            .map(|f| f.path.clone())
                            .unwrap_or_default(),
                        item.head.line,
                    )
                })
                .unwrap_or_default()
        };
        for frame in frames.iter_mut() {
            frame
                .owners
                .sort_by_key(|&at| (ty_line(owners[at as usize].ty), at));
            frame.kids.sort_by_key(|&at| at);
        }

        // What each frame holds, however deep: the counted words a fold writes.
        let mut held: HashMap<u32, u32> = HashMap::new();
        for frame in frames.iter().rev() {
            let own: u32 = frame.marks.len() as u32
                + frame
                    .owners
                    .iter()
                    .map(|&at| owners[at as usize].marks.len() as u32)
                    .sum::<u32>();
            let under: u32 = frame
                .kids
                .iter()
                .map(|kid| held.get(kid).copied().unwrap_or(0))
                .sum();
            held.insert(frame.id, own + under);
        }
        for frame in frames.iter_mut() {
            frame.held = held.get(&frame.id).copied().unwrap_or(0);
        }

        // ---- The folds. ------------------------------------------------------
        //
        // Nothing here folds by a count, a depth or a budget — a fold is the
        // reviewer's own gesture. A box with nothing inside it has nothing to
        // fold, so it draws no mark at all, and a fold names its box by the same
        // key its URL does, so it survives the next build of the chart.
        let folded: HashSet<Spot> = frames
            .iter()
            .filter(|f| f.folded && f.held > 0)
            .map(|f| Spot::Frame(f.id))
            .chain(
                owners
                    .iter()
                    .filter(|o| o.folded && !o.marks.is_empty())
                    .map(|o| Spot::Owner(o.id)),
            )
            .collect();
        for frame in frames.iter_mut() {
            frame.folded = folded.contains(&Spot::Frame(frame.id));
        }
        for owner in owners.iter_mut() {
            owner.folded = folded.contains(&Spot::Owner(owner.id));
        }
        // Only a fold the packer was allowed to skip changes where anything
        // sits. The rest are elisions in place, and the layout still reserves
        // every footprint they hide.
        let skipped = &reading.packed;
        let packed: HashSet<Spot> = folded
            .iter()
            .copied()
            .filter(|spot| match spot {
                Spot::Frame(id) => skipped.contains(&mod_fold(&frames[*id as usize].key())),
                Spot::Owner(id) => skipped.contains(&owners[*id as usize].key()),
                Spot::Mark(_) => false,
            })
            .collect();

        // What every fold hides, and the box standing for it: down the household
        // from the crates, carrying the outermost fold met on the way.
        let mut packs: HashMap<Spot, Spot> = HashMap::new();
        if !folded.is_empty() {
            let mut stack: Vec<(u32, Option<Spot>)> = frames
                .iter()
                .filter(|f| f.parent.is_none())
                .map(|f| (f.id, None))
                .collect();
            while let Some((at, under)) = stack.pop() {
                let rep =
                    under.or_else(|| folded.contains(&Spot::Frame(at)).then_some(Spot::Frame(at)));
                let frame = &frames[at as usize];
                for &mark in &frame.marks {
                    if let Some(rep) = rep {
                        packs.insert(Spot::Mark(mark), rep);
                    }
                }
                for &owner in &frame.owners {
                    let spot = Spot::Owner(owner);
                    if let Some(rep) = rep {
                        packs.insert(spot, rep);
                    }
                    let under = rep.or_else(|| folded.contains(&spot).then_some(spot));
                    for &mark in &owners[owner as usize].marks {
                        if let Some(under) = under {
                            packs.insert(Spot::Mark(mark), under);
                        }
                    }
                }
                for &kid in &frame.kids {
                    if let Some(rep) = rep {
                        packs.insert(Spot::Frame(kid), rep);
                    }
                    stack.push((kid, rep));
                }
            }
        }

        House {
            frames,
            owners,
            home,
            folded,
            packed,
            packs,
            multi_crate,
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
    use crate::graph::data::{DeclBody, DeclHead, FileInfo, Reach};
    use crate::views::data::VisFloor;

    fn item(id: u32, name: &str, kind: ItemKind, vis: Vis, parent: Option<u32>) -> ItemMark {
        ItemMark {
            id,
            file: 0,
            parent,
            head: DeclHead {
                name: name.rsplit("::").next().unwrap_or(name).to_string(),
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
            files: vec![FileInfo {
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
            folds: Default::default(),
            packed: Default::default(),
        }
    }

    /// The same reading with some rooms folded by hand — elided in place, which
    /// is what a fold does: nothing is packed away.
    fn folding(folds: &[FoldAt]) -> FnReading {
        FnReading {
            vis_floor: VisFloor::All,
            folds: folds.iter().cloned().collect(),
            packed: Default::default(),
        }
    }

    /// A workspace with two modules, a type declared in one of them with two
    /// methods (one written in a *different* file), and a free function beside
    /// it. This is the shape the household exists to draw.
    fn house() -> CodeGraph {
        let mut graph = CodeGraph {
            files: vec![
                FileInfo {
                    path: "src/views/plate.rs".to_string(),
                    krate: "slope".to_string(),
                },
                FileInfo {
                    path: "src/graph/wire.rs".to_string(),
                    krate: "slope".to_string(),
                },
            ],
            items: Vec::new(),
            implements: Vec::new(),
            refs: Vec::new(),
            holds: Vec::new(),
            ghosts: Vec::new(),
            limits: Default::default(),
        };
        // `struct Plate` and its two methods, one of them written in the other
        // module's file — an impl block written away from its type.
        graph
            .items
            .push(item(0, "Plate", ItemKind::Struct, Vis::Pub, None));
        graph
            .items
            .push(item(1, "Plate::rule", ItemKind::Fn, Vis::Pub, Some(0)));
        let mut away = item(2, "Plate::ink", ItemKind::Fn, Vis::Private, Some(0));
        away.file = 1;
        graph.items.push(away);
        // A free function in each module.
        graph
            .items
            .push(item(3, "draw", ItemKind::Fn, Vis::Pub, None));
        let mut wire = item(4, "trace", ItemKind::Fn, Vis::Crate, None);
        wire.file = 1;
        graph.items.push(wire);
        for (from, to, count) in [(3, 1, 4), (1, 2, 2), (4, 1, 7), (3, 4, 1)] {
            graph.refs.push(MarkRef { from, to, count });
        }
        graph
    }

    /// The tier is the whole verdict about where running starts: what nothing
    /// calls starts, and every other mark is as many calls in as the shortest
    /// way to it. The household places nothing by it — but it still says it.
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
        // And what runs under each, by the way in, is still the sentence a
        // reviewer reads off an entry point.
        let runs = |name: &str| {
            model
                .marks
                .iter()
                .find(|m| m.head.name == name)
                .map(|m| m.runs)
        };
        assert_eq!(
            (runs("main"), runs("walk"), runs("note")),
            (Some(2), Some(1), Some(0))
        );
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

    /// **Every call is a wire.** Nothing on this ground stands for a call, so a
    /// call the survey resolved between two drawn declarations is a line on the
    /// paper — the shelved section's `seats` flag went with the call tree.
    #[test]
    fn every_resolved_call_is_a_wire() {
        let model = FnModel::build(&chain(), &reading(VisFloor::All));
        let drawn: Vec<(u32, u32, u32)> = model
            .calls
            .iter()
            .map(|c| (c.user, c.def, c.count))
            .collect();
        assert_eq!(drawn, vec![(0, 1, 2), (1, 2, 5)]);
        // And each mark's ends are ranked heaviest first, which is the order the
        // arrows step in and every list of ends reads in.
        assert_eq!(model.callees.get(&0), Some(&vec![1]));
        assert_eq!(model.callers.get(&2), Some(&vec![1]));
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

    /// **The reading folds the picture; it never withholds a name.** The paper
    /// drops a wire whose end it does not draw — and the sheet keeps every one
    /// of them, because a reviewer who opened a declaration asked about that
    /// declaration and not about the slider (2026-08-28, user).
    #[test]
    fn a_sheet_keeps_the_calls_the_paper_drops() {
        let narrow = FnModel::build(&chain(), &reading(VisFloor::Crate));
        let wide = FnModel::build(&chain(), &reading(VisFloor::All));
        // `note` is private: no wire on the narrow paper, every call on both
        // sheets, and the same call either way the slider is set.
        assert!(narrow.calls.iter().all(|c| c.def != 2 && c.user != 2));
        assert_eq!(narrow.all_calls, wide.all_calls);
        let into_note: Vec<u32> = narrow
            .all_calls
            .iter()
            .filter(|c| c.def == 2)
            .map(|c| c.user)
            .collect();
        assert_eq!(into_note, vec![1], "`walk` calls it, drawn or not");
        // And the blast radius walks the whole call graph: a rewrite of the
        // private declaration reaches `walk`, and `main` behind it.
        assert_eq!(narrow.upstream(2), HashSet::from([1, 0]));
        assert_eq!(narrow.upstream(2), wide.upstream(2));
        // The hover words on the mark and the rows on its sheet count the same
        // ends — the count a sheet states is a count of rows it can name.
        let walk = narrow.marks.iter().find(|m| m.head.name == "walk").unwrap();
        assert_eq!(
            walk.calls, 1,
            "the private callee is still one of its calls"
        );
    }

    /// A ring nothing reaches is said in words, not dropped: two functions that
    /// only call each other are on the paper, in their own band, seated in the
    /// module they are written in like everything else.
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
            // And it is written in the crate root's frame, like every other
            // free declaration in `src/main.rs`.
            assert_eq!(model.home.get(&mark.id), Some(&Spot::Frame(0)));
        }
        assert_eq!(model.facts.ring, 2);
        // And the ring is the last band on the paper, under every depth.
        let last = model.bands.last().expect("a band");
        assert_eq!(last.1, "in a call ring");
    }

    /// **The household is the ground.** A method sits in its owner's container,
    /// a free declaration on its module's own shelf, and the container stands in
    /// the module the *type* is declared in — gathering every impl block that
    /// type has, wherever in the workspace it is written.
    #[test]
    fn a_method_seats_in_its_owner_and_a_free_fn_on_the_module_shelf() {
        let model = FnModel::build(&house(), &reading(VisFloor::All));
        let frame = |path: &[&str]| {
            let key: Vec<String> = std::iter::once("slope".to_string())
                .chain(path.iter().map(|s| (*s).to_string()))
                .collect();
            model.frame_at(&key).expect("a frame").id
        };
        let (root, views, graph) = (frame(&[]), frame(&["views"]), frame(&["graph"]));
        assert!(model.frames[views as usize].parent == Some(root));
        assert!(model.frames[graph as usize].parent == Some(root));

        // One container, for `Plate`, standing in `views` — the module the type
        // is declared in, not the module the second impl block is written in.
        assert_eq!(model.owners.len(), 1);
        let plate = &model.owners[0];
        assert_eq!((plate.decl, plate.name.as_str()), ("impl", "Plate"));
        assert_eq!(plate.frame, views);
        // Declaration order, as the source writes it: the file, then the line.
        // `Plate::ink` is written in `graph/wire.rs` and `Plate::rule` in
        // `views/plate.rs`, so the file is what orders them.
        assert_eq!(plate.marks, vec![2, 1], "in declaration order");
        assert_eq!(model.frames[views as usize].owners, vec![plate.id]);

        // Both methods live in it, the one written in `graph/wire.rs` included.
        assert_eq!(model.home.get(&1), Some(&Spot::Owner(plate.id)));
        assert_eq!(model.home.get(&2), Some(&Spot::Owner(plate.id)));
        // The free declarations sit on their own module's shelf.
        assert_eq!(model.home.get(&3), Some(&Spot::Frame(views)));
        assert_eq!(model.home.get(&4), Some(&Spot::Frame(graph)));
        assert_eq!(model.frames[views as usize].marks, vec![3]);
        assert_eq!(model.frames[graph as usize].marks, vec![4]);
        // And each frame counts everything under it, however deep.
        assert_eq!(model.frames[views as usize].held, 3);
        assert_eq!(model.frames[root as usize].held, 4);
        // The sheet still says where a method's own source is written, which is
        // not always its owner's module.
        let ink = model.marks.iter().find(|m| m.id == 2).unwrap();
        assert_eq!(ink.written(), "graph");
        assert_eq!(model.frame_of(Spot::Mark(2)), Some(views));
    }

    /// A trait's container is a `trait`, not an `impl`: rust's own word for the
    /// block those clauses are written in.
    #[test]
    fn a_trait_container_says_trait() {
        let mut graph = house();
        graph
            .items
            .push(item(5, "Words", ItemKind::Trait, Vis::Pub, None));
        graph
            .items
            .push(item(6, "Words::say", ItemKind::Fn, Vis::Pub, Some(5)));
        let model = FnModel::build(&graph, &reading(VisFloor::All));
        let words = model
            .owner_at("src/views/plate.rs", "Words")
            .expect("a room");
        assert_eq!(words.decl, "trait");
        assert!(!words.on_data, "the data chart draws no block for a trait");
        assert_eq!(words.marks, vec![6]);
    }

    /// **The visibility reading is the API lens.** At `pub` every declaration
    /// written narrower is off the paper, and a room with nothing admitted draws
    /// no husk at all — no empty container, no empty module frame.
    #[test]
    fn a_narrow_reading_draws_no_empty_room() {
        let model = FnModel::build(&house(), &reading(VisFloor::Pub));
        let drawn: HashSet<u32> = model.marks.iter().map(|m| m.id).collect();
        // `Plate::ink` is private and `trace` is `pub(crate)`.
        assert_eq!(drawn, HashSet::from([1, 3]));
        assert_eq!(model.facts.off_paper, 2);
        // `graph::wire` had only those two, so its frame is gone entirely.
        assert!(
            model
                .frame_at(&["slope".to_string(), "graph".to_string()])
                .is_none(),
            "a module with nothing admitted draws no husk"
        );
        // The container stands, with only the method the reading admits in it.
        let plate = model
            .owner_at("src/views/plate.rs", "Plate")
            .expect("a room");
        assert_eq!(plate.marks, vec![1]);
        // And nothing narrower than the stop is admitted anywhere.
        assert!(model.marks.iter().all(|m| m.head.vis == Vis::Pub));
    }

    /// A fold is the reviewer's own gesture: it takes what is inside one room
    /// off the paper, leaves the room's own border standing for it, and says how
    /// many in words.
    #[test]
    fn a_folded_room_stands_for_everything_it_hides() {
        let model = FnModel::build(
            &house(),
            &folding(&[owner_fold("src/views/plate.rs", "Plate")]),
        );
        let plate = model.owner_at("src/views/plate.rs", "Plate").unwrap();
        assert!(plate.folded);
        assert_eq!(plate.marks.len(), 2, "the count it states in words");
        // Both methods are off the paper, and the container stands for them.
        for method in [1u32, 2] {
            assert!(model.hidden(Spot::Mark(method)));
            assert_eq!(model.shown(Spot::Mark(method)), Spot::Owner(plate.id));
        }
        // The fold hides nothing above itself, and nothing outside it.
        assert!(!model.hidden(Spot::Owner(plate.id)));
        assert_eq!(model.shown(Spot::Mark(3)), Spot::Mark(3));
        assert_eq!(model.folded, HashSet::from([Spot::Owner(plate.id)]));
    }

    /// A folded module takes every room under it off the paper, and its own
    /// border stands for all of them.
    #[test]
    fn a_folded_module_stands_for_the_rooms_inside_it() {
        let model = FnModel::build(
            &house(),
            &folding(&[mod_fold(&["slope".to_string(), "views".to_string()])]),
        );
        let views = model
            .frame_at(&["slope".to_string(), "views".to_string()])
            .unwrap();
        assert!(views.folded);
        assert_eq!(views.held, 3);
        for mark in [1u32, 2, 3] {
            assert_eq!(model.shown(Spot::Mark(mark)), Spot::Frame(views.id));
        }
        // The container inside it is hidden too, and stands for nothing itself.
        let plate = model.owner_at("src/views/plate.rs", "Plate").unwrap();
        assert_eq!(model.shown(Spot::Owner(plate.id)), Spot::Frame(views.id));
        // What is written elsewhere is untouched.
        assert_eq!(model.shown(Spot::Mark(4)), Spot::Mark(4));
    }

    /// Nothing folds itself. A room nobody folded hides nothing, whatever it
    /// holds, and a fold naming a room with nothing inside it draws no mark.
    #[test]
    fn nothing_folds_by_itself_or_by_a_count() {
        let open = FnModel::build(&house(), &reading(VisFloor::All));
        assert!(open.folded.is_empty());
        assert!(open.packs.is_empty());
        // A module the reading emptied has nothing to fold.
        let shut = FnModel::build(
            &house(),
            &FnReading {
                vis_floor: VisFloor::Pub,
                folds: [mod_fold(&["slope".to_string(), "graph".to_string()])]
                    .into_iter()
                    .collect(),
                packed: Default::default(),
            },
        );
        assert!(shut.folded.is_empty(), "an empty room has nothing to fold");
        assert!(shut.packs.is_empty());
    }

    /// The outermost fold is what stands on the paper: a fold inside a fold is
    /// spoken for by the one the reader can actually see.
    #[test]
    fn the_outermost_fold_stands_for_the_folds_inside_it() {
        let model = FnModel::build(
            &house(),
            &folding(&[
                mod_fold(&["slope".to_string(), "views".to_string()]),
                owner_fold("src/views/plate.rs", "Plate"),
            ]),
        );
        let views = model
            .frame_at(&["slope".to_string(), "views".to_string()])
            .unwrap();
        let plate = model.owner_at("src/views/plate.rs", "Plate").unwrap();
        assert_eq!(model.folded.len(), 2);
        assert_eq!(model.shown(Spot::Owner(plate.id)), Spot::Frame(views.id));
        assert_eq!(model.shown(Spot::Mark(1)), Spot::Frame(views.id));
    }

    /// A selection the reader cannot see is not a focus, so the way to a box
    /// names every fold it is hiding behind, outermost first.
    #[test]
    fn a_reveal_names_every_fold_on_the_way_in() {
        let views = mod_fold(&["slope".to_string(), "views".to_string()]);
        let plate = owner_fold("src/views/plate.rs", "Plate");
        let model = FnModel::build(&house(), &folding(&[views.clone(), plate.clone()]));
        assert_eq!(model.reveal(Spot::Mark(1)), vec![views.clone(), plate]);
        // A box standing on open paper needs nothing unfolded, and a folded
        // room's own border is drawn — so nothing unfolds to reach it either.
        assert!(model.reveal(Spot::Mark(4)).is_empty());
        let one = FnModel::build(&house(), &folding(&[views]));
        let room = one
            .frame_at(&["slope".to_string(), "views".to_string()])
            .unwrap();
        assert!(one.reveal(Spot::Frame(room.id)).is_empty());
    }

    /// Recursion is a word on the mark, never a wire that leaves and comes back
    /// to the same block.
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
        // And each clause sits in its own trait's room, the answering method in
        // its type's.
        assert_eq!(model.home.get(&5), model.home.get(&5));
        let words = model.owner_at("src/main.rs", "Words").expect("a room");
        let plate = model.owner_at("src/main.rs", "Plate").expect("a room");
        assert_eq!(model.home.get(&5), Some(&Spot::Owner(words.id)));
        assert_eq!(model.home.get(&7), Some(&Spot::Owner(plate.id)));
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

    /// One room holds what it holds, whatever the folds: `inside` is what a
    /// boundary selection lights and what its sheet lists.
    #[test]
    fn a_room_knows_everything_written_in_it() {
        let model = FnModel::build(&house(), &reading(VisFloor::All));
        let views = model
            .frame_at(&["slope".to_string(), "views".to_string()])
            .unwrap();
        let root = model.frame_at(&["slope".to_string()]).unwrap();
        let plate = model.owner_at("src/views/plate.rs", "Plate").unwrap();
        assert_eq!(model.inside(Spot::Owner(plate.id)), HashSet::from([1, 2]));
        assert_eq!(
            model.inside(Spot::Frame(views.id)),
            HashSet::from([1, 2, 3])
        );
        assert_eq!(
            model.inside(Spot::Frame(root.id)),
            HashSet::from([1, 2, 3, 4]),
            "a crate frame holds every room under it"
        );
        assert_eq!(model.over(views.id), vec![root.id]);
    }

    /// The whole reading is deterministic: one survey and one set of folds
    /// always read one household.
    #[test]
    fn the_same_survey_always_reads_the_same_household() {
        let a = FnModel::build(&house(), &reading(VisFloor::All));
        let b = FnModel::build(&house(), &reading(VisFloor::All));
        assert_eq!(a, b);
    }
}
