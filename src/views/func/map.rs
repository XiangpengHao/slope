//! The function chart: the **household**.
//!
//! The direction contract, and it is the whole of this file's reason: at this
//! altitude, as at the one above it, **containment is written-in** (2026-08-27,
//! user decision — *"enhance the concept of ownership… functions belong to
//! struct (if member method) or mod"*). A crate frame holds module frames
//! nested exactly as rust's modules nest; inside a module frame stand its free
//! declarations, one **container** per owner that declares methods (`impl
//! FnModel`, `trait Chart`), and the modules nested under it. Reading the
//! nesting is reading whose code this is.
//!
//! So the two charts are one grammar with two duals: `/data` draws what a type
//! keeps, `/fn` draws what it does, and both draw the same rooms. The **shelved
//! section** — every declaration seated inside the caller that reached it first
//! — stood here for one day and is recorded in `spec/function-viewer.md`.
//!
//! One block per declaration that runs, and one **head row** per block: the
//! keyword run, the name, the diff's letter. The signature is quoted under it.
//! An **entry point** wears the root's 2.5px ink left edge: this is where a
//! chain of running begins, wherever the household seats it.
//!
//! **Every call is a wire.** Nothing on this ground stands for a call, so a call
//! not drawn is a call not said. At rest the cross-module family bundles into
//! one **corridor** per ordered module pair, with its count riding the line;
//! anything in focus — a selection, the pointer, the diff — unbundles into that
//! anchor's own wires. Two families and only two: solid is a call, dashed and
//! lighter is a contract (`answers`), and a contract never bundles.
//!
//! Three gestures act on a box and they are three marks, never one: the border
//! **selects the boundary**, the label **descends** (a container's owner to its
//! block on `/data`) or selects, and the `–` / `+` at the border's other end
//! **folds what is inside**.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size, Viewport,
};

use crate::Route;
use crate::graph::data::{CodeGraph, ItemKind};
use crate::views::chrome::{narrow_viewport, prefers_reduced_motion, use_settled, window_size};
use crate::views::func::layout::{self, FnLayout, HEAD_H, Placed, Sizes, TieSide};
use crate::views::func::model::{CallKind, Container, FnMark, FnModel, SigRow, Spot, Tier};
use crate::views::func::{FnSel, FnWires, mark_route, mod_fold, mod_route, owner_route, use_fns};

// ---------------------------------------------------------------------------
// Head furniture, in flow units. These numbers are the CSS in `tailwind.css`;
// move one and the other must follow.
// ---------------------------------------------------------------------------

/// Both sides together: the block's border and the head row's padding.
const PAD_X: f64 = 12.0;
/// Between two runs of the head.
const RUN_GAP: f64 = 5.0;
const KW_PX: f64 = 8.5;
const NAME_PX: f64 = 10.5;
/// One quoted signature row: its type size, its line, and how far it stands in
/// from the block's edge — rust's own indent, narrowed to what a 10px
/// quotation can spare, with the diff's `+` sitting in that gutter.
const ROW_PX: f64 = 10.0;
const ROW_H: f64 = 14.0;
const ROW_INDENT: f64 = 12.0;
/// The block's border, top and bottom — the box is border-box, as the CSS is.
const BORDER: f64 = 2.0;
/// Clear paper under the last quoted row.
const PAD_BOTTOM: f64 = 4.0;
/// A container's or a frame's label, and the fold mark's own room beside it.
const LABEL_PX: f64 = 11.5;
const FRAME_LABEL_PX: f64 = 12.0;
const FOLD_W: f64 = 22.0;
/// The counted words a folded box writes.
const PACKED_PX: f64 = 9.5;
/// Small-type slack: at 8–9px the browser rounds each glyph up, so text
/// measured with the font's exact advance clips its last characters.
const META_SLACK: f64 = 1.08;
/// One em of advance in the data face (JetBrains Mono is monospaced).
const MONO_ADVANCE: f64 = 0.6;
/// A block narrower than this reads as a chip rather than a quotation; wider
/// than this, a quoted line ellipsizes and its hover words carry the rest. The
/// same two numbers the data chart clamps its blocks between.
const MARK_MIN_W: f64 = 152.0;
const MARK_MAX_W: f64 = 300.0;
/// The far edition's **landmark register**: the biggest a box's engraved name
/// may be drawn, the smallest that is worth drawing at all, and how much of a
/// box's width one name may spend. Below the floor the box gets no engraved
/// name — the far edition keeps its reading-size label instead, which is what
/// the Two-Ramps rule asks for when a box cannot carry a larger name.
const LANDMARK_MAX: f64 = 54.0;
const LANDMARK_MIN: f64 = 26.0;
const LANDMARK_OF_WIDTH: f64 = 0.055;
/// How much smaller a landmark is than the one it stands inside, and how much
/// clear paper it keeps below it. The ladder of sizes is the nesting.
const LANDMARK_STEP: f64 = 6.0;
const LANDMARK_GAP: f64 = 10.0;

fn text_w(text: &str, px: f64) -> f64 {
    text.chars().count() as f64 * px * MONO_ADVANCE
}

/// The engraved width of one wire: heavier the more references the survey
/// resolved for the pair, as everywhere in this system. A contract is one
/// promise, so it draws at the hairline.
fn wire_width(answers: bool, count: u32) -> f64 {
    match answers {
        true => 1.0,
        false => (1.0 + (count.max(1) as f64).ln() * 0.32).min(2.4),
    }
}

/// One block, measured. Everything the plate draws about one mark, and nothing
/// about where it sits.
///
/// The `Owner::` prefix the head wore until 2026-08-27 is gone: the container
/// the block stands in says whose method this is, and saying it twice cost
/// every method row the width of its own type's name. The qualified label
/// stays in the search rows, on the sheet and in every URL.
#[derive(Clone, PartialEq)]
struct MeasuredBlock {
    id: u32,
    /// `pub fn`, `fn`, `macro` — what rust writes in front of the name.
    decl: String,
    name: String,
    /// The bracket the head opens with: `(` where parameters follow, `()`
    /// where none do, nothing at all for a macro.
    open: String,
    /// The signature's parameter rows, quoted as the source writes them.
    rows: Vec<SigRow>,
    /// The line that closes the quotation: `) -> Result<…>`, `)`, `-> u32`, or
    /// nothing where the declaration says neither.
    tail: String,
    letter: Option<&'static str>,
    entry: bool,
    ring: bool,
    /// The label a URL selects this block by, and the file it is written in.
    path: String,
    label: String,
    title: String,
    /// The block's own box: its head row and the quoted signature.
    own: (f64, f64),
}

impl MeasuredBlock {
    fn measure(mark: &FnMark) -> Self {
        let decl = mark.head.decl();
        let macro_rules = mark.head.kind == ItemKind::Macro;
        let letter = mark.letter();
        let rows: Vec<SigRow> = mark.rows.iter().filter(|r| !r.returns).cloned().collect();
        let ret = mark.rows.iter().find(|r| r.returns);
        let open = match (macro_rules, rows.is_empty()) {
            (true, _) => String::new(),
            (false, true) => "()".to_string(),
            (false, false) => "(".to_string(),
        };
        let tail = match (macro_rules, rows.is_empty(), ret) {
            (true, ..) => String::new(),
            (false, false, Some(r)) => format!(") -> {}", r.ty),
            (false, false, None) => ")".to_string(),
            (false, true, Some(r)) => format!("-> {}", r.ty),
            (false, true, None) => String::new(),
        };

        // The head row, run by run — the whole of it, because a head that
        // clips its own name says nothing.
        let mut head_w = text_w(&decl, KW_PX) * META_SLACK + RUN_GAP;
        head_w += text_w(&mark.head.name, NAME_PX) + text_w(&open, NAME_PX);
        if letter.is_some() {
            head_w += RUN_GAP + text_w("M", NAME_PX);
        }
        // And the longest quoted line under it. A line wider than the block may
        // be ellipsizes, and its hover words carry the whole of it.
        let widest = rows
            .iter()
            .map(|row| text_w(&row.written(), ROW_PX) + ROW_INDENT)
            .chain(std::iter::once(text_w(&tail, ROW_PX)))
            .fold(head_w, f64::max);
        // The clamp governs the **quotation**: a type is a long thing, so a row
        // wider than the widest a quotation reads at ellipsizes and its hover
        // words carry the whole of it. The head is not a quotation — it is which
        // declaration this is — so it sets the floor under the box instead of
        // being cut inside it.
        let w = (widest + PAD_X)
            .clamp(MARK_MIN_W, MARK_MAX_W)
            .max(head_w + PAD_X)
            .ceil();

        let body_h = match rows.is_empty() && tail.is_empty() {
            true => 0.0,
            false => {
                rows.len() as f64 * ROW_H + if tail.is_empty() { 0.0 } else { ROW_H } + PAD_BOTTOM
            }
        };

        Self {
            id: mark.id,
            decl,
            name: mark.head.name.clone(),
            open,
            rows,
            tail,
            letter,
            entry: mark.tier == Tier::Entry,
            ring: mark.tier == Tier::Ring,
            path: mark.head.path.clone(),
            label: mark.head.label.clone(),
            title: mark.title(),
            own: (w, BORDER + HEAD_H + body_h),
        }
    }
}

/// One node on the function chart. Every node is a block: a container and a
/// module frame are drawn on the ground, under the blocks they hold.
#[derive(Clone, PartialEq)]
struct FnNodeData(Box<MeasuredBlock>);

/// One owner container on the paper: the border a click selects it by, the
/// owner's own name — in the kind colour the data chart inks that kind, and a
/// descent link to its block there — and the fold mark at the border's other
/// end.
#[derive(Clone, PartialEq)]
struct OwnerView {
    id: u32,
    at: Placed,
    /// `impl` or `trait`, then the owner's name: two runs, because only the
    /// second one takes a kind colour and only the second one is a link.
    decl: &'static str,
    name: String,
    /// Which kind colour the name takes — see the Kind-Color Rule.
    tint: &'static str,
    /// `impl FnModel`, for prose and hover words.
    words: String,
    /// The type's own (file, label): the fold key, the selection URL, and the
    /// descent link's target.
    path: String,
    label: String,
    /// Whether the data chart draws a block for the owner.
    on_data: bool,
    folded: bool,
    /// What a folded container states in words where its methods stood.
    packed: String,
    held: usize,
}

/// One crate or module frame on the paper.
#[derive(Clone, PartialEq)]
struct FrameView {
    id: u32,
    at: Placed,
    /// `mod func`, or a crate's own name — `None` on the single-crate
    /// workspace's crate frame, which the cartouche already names.
    label: Option<String>,
    label_w: f64,
    key: Vec<String>,
    words: String,
    folded: bool,
    packed: String,
    held: usize,
    /// Whether any frame is nested inside it, which is what decides where its
    /// far-edition name is engraved.
    parent: bool,
}

/// One drawn wire with its ends already found.
#[derive(Clone, PartialEq)]
struct WireView {
    key: String,
    from: Point,
    to: Point,
    /// The end being leaned on, and the end that leans — as the paper draws
    /// them, so an end a fold hides is the box that stands for it.
    def: Spot,
    user: Spot,
    /// What the survey resolved across the pair, summed over every call a fold
    /// or a corridor gathered onto this one line.
    count: u32,
    label: Option<String>,
    /// The words a corridor answers with. A count with no findable subject is
    /// the defect this system rejects everywhere else.
    title: Option<String>,
    /// Where clicking a corridor goes: the module at the far end of it.
    to_mod: Option<Vec<String>>,
    width: f64,
    /// Whether the diff touched each end. The resting plate's own anchor.
    def_dirty: bool,
    user_dirty: bool,
    /// Its two ends are written in different module frames, so at rest it is
    /// said by a corridor rather than by its own line.
    cross: bool,
    /// It **is** a corridor: one line per ordered module pair, standing for
    /// every cross-module call between them.
    bundle: bool,
    from_side: TieSide,
    to_side: TieSide,
    class: &'static str,
}

impl WireView {
    /// Whether the **diff anchor** reads this wire in the given direction:
    /// `calls` asks whether the diff touched the end that leans, `callers`
    /// whether it touched the end being leaned on, `both` either.
    fn anchored(&self, reading: FnWires) -> bool {
        match reading {
            FnWires::Calls => self.user_dirty,
            FnWires::Callers => self.def_dirty,
            FnWires::Both => self.def_dirty || self.user_dirty,
        }
    }
}

/// One landmark: a box's own name, engraved across it at the size that box can
/// carry, for the far edition alone.
#[derive(Clone, PartialEq)]
struct NameView {
    at: Spot,
    name: String,
    x: f64,
    y: f64,
    size: f64,
}

impl NameView {
    /// The engraved name for one box, or `None` where it cannot carry one.
    ///
    /// **Measured to the box, never guessed.** The size is the smallest of four
    /// limits: a share of the box's width, the width the identifier itself needs
    /// at that size, the room under the box's own label row, and the `ceiling`
    /// the register above it leaves — the name inside a named box is always a
    /// step smaller than the name of the box it stands in, so the ladder of
    /// sizes *is* the nesting. An identifier is never broken across two lines
    /// and never clipped: where even the floor does not fit, the box keeps its
    /// reading-size label and this register says nothing about it.
    fn measure(at: Spot, name: &str, place: Placed, ceiling: f64) -> Option<Self> {
        let chars = name.chars().count().max(1) as f64;
        let by_width = (place.w - PAD_X) / (chars * MONO_ADVANCE);
        let by_height = place.h - HEAD_H - 4.0;
        let size = (place.w * LANDMARK_OF_WIDTH)
            .min(by_width)
            .min(by_height)
            .min(ceiling)
            .floor();
        (size >= LANDMARK_MIN).then(|| Self {
            at,
            name: name.to_string(),
            // Left-aligned with the shelf inside the box and standing just under
            // the label row it grows out of, so the landmark is where the label
            // was — one place for one name, at two zooms.
            x: place.x + layout::FRAME_PAD,
            y: place.y + HEAD_H + size * 0.78,
            size,
        })
    }

    /// The lowest ink this landmark puts on the paper, descenders included.
    fn foot(&self) -> f64 {
        self.y + self.size * 0.22
    }
}

/// One drawing of the chart: the blocks, rooms and wires one build puts on the
/// paper, plus the indexes a reading walks.
#[derive(Clone, PartialEq)]
struct FnDrawing {
    nodes: Vec<FlowNode<FnNodeData>>,
    frames: Vec<FrameView>,
    owners: Vec<OwnerView>,
    /// The far edition's landmark register: one engraved name per box whose box
    /// can carry one.
    names: Vec<NameView>,
    /// Every call, one line each — what an anchor unbundles into.
    wires: Vec<WireView>,
    /// One corridor per ordered module pair, for the resting plate.
    bundles: Vec<WireView>,
    /// Which band every drawn mark sits in, for a band reading.
    bands: HashMap<u32, u32>,
    /// Every drawn box, for the camera and for a wire's ends.
    rects: HashMap<Spot, Placed>,
    /// The URL's (path, item) key for every drawn mark.
    locate: HashMap<(String, String), u32>,
    frame: Option<Rect>,
    dirty: bool,
}

/// The kind colour an owner's name takes on the paper.
///
/// **The Kind-Color Rule, extended** (2026-08-27). The rule reserved kind colour
/// to a data block's own name — the one place a type's kind was drawn rather
/// than only written. The owner container is the second, and it is the same
/// fact about the same declaration one rung down, so it takes the same two
/// colours: type-teal for a product type, the palette's purple for a sum type.
///
/// A **trait** stays ink. On this chart the purple is already the colour a
/// function's name takes, so a purple `trait Chart` would read as a function;
/// and `/data` draws no block for a trait, so there is no kind colour of its own
/// to agree with. Ink at weight 700 is what a trait's name gets here.
fn owner_tint(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Enum => "is-sum",
        ItemKind::Struct | ItemKind::Union => "is-type",
        _ => "",
    }
}

impl FnDrawing {
    fn build(model: &FnModel) -> Self {
        // **Every** mark is measured, whether or not a fold has it off the
        // paper, because the layout has to reserve what a fold hides: that
        // reserved footprint is what keeps the rest of the sheet still when the
        // reader closes a box.
        let mut sizes = Sizes::default();
        let mut views: Vec<MeasuredBlock> = Vec::with_capacity(model.marks.len());
        for mark in &model.marks {
            let view = MeasuredBlock::measure(mark);
            sizes.marks.insert(mark.id, view.own);
            if !model.hidden(Spot::Mark(mark.id)) {
                views.push(view);
            }
        }
        // The rooms' own labels, measured the same way.
        let owner_words: Vec<String> = model.owners.iter().map(Container::words).collect();
        for owner in &model.owners {
            let w = text_w(&owner_words[owner.id as usize], LABEL_PX) * META_SLACK + FOLD_W;
            sizes.owners.insert(owner.id, w);
            if owner.folded {
                sizes.shut.insert(
                    Spot::Owner(owner.id),
                    text_w(&packed_words(owner.marks.len()), PACKED_PX) * META_SLACK,
                );
            }
        }
        let frame_label: Vec<Option<String>> = model
            .frames
            .iter()
            .map(|f| f.label(model.multi_crate))
            .collect();
        for frame in &model.frames {
            let label = frame_label[frame.id as usize].as_deref().unwrap_or("");
            sizes.labels.insert(
                frame.id,
                text_w(label, FRAME_LABEL_PX) * META_SLACK + FOLD_W,
            );
            if frame.folded {
                sizes.shut.insert(
                    Spot::Frame(frame.id),
                    text_w(&packed_words(frame.held as usize), PACKED_PX) * META_SLACK,
                );
            }
        }

        let placed = FnLayout::build(model, &sizes);
        let mut rects: HashMap<Spot, Placed> = HashMap::new();
        for (&id, at) in &placed.marks {
            rects.insert(Spot::Mark(id), *at);
        }
        for (&id, at) in &placed.owners {
            rects.insert(Spot::Owner(id), *at);
        }
        for (id, at) in &placed.frames {
            rects.insert(Spot::Frame(*id), *at);
        }

        let mut nodes: Vec<FlowNode<FnNodeData>> = Vec::with_capacity(views.len());
        let mut locate: HashMap<(String, String), u32> = HashMap::new();
        for view in &views {
            let Some(at) = placed.marks.get(&view.id).copied() else {
                continue;
            };
            locate.insert((view.path.clone(), view.label.clone()), view.id);
            nodes.push(
                FlowNode::with_data(
                    format!("fn-{}", view.id),
                    view.name.clone(),
                    (at.x, at.y),
                    FnNodeData(Box::new(view.clone())),
                )
                .size(Size::new(at.w, at.h))
                .sides(Side::Top, Side::Bottom)
                .draggable(false)
                .selectable(false),
            );
        }

        // ---- The rooms, as the paper draws them. ---------------------------
        let nested: HashSet<u32> = model.frames.iter().filter_map(|f| f.parent).collect();
        let frames: Vec<FrameView> = placed
            .frames
            .iter()
            .filter_map(|(id, at)| {
                let frame = model.frames.get(*id as usize)?;
                (!model.hidden(Spot::Frame(*id))).then(|| FrameView {
                    id: *id,
                    at: *at,
                    label: frame_label[*id as usize].clone(),
                    label_w: text_w(
                        frame_label[*id as usize].as_deref().unwrap_or(""),
                        FRAME_LABEL_PX,
                    ),
                    key: frame.key(),
                    words: frame.words(),
                    folded: frame.folded,
                    packed: packed_words(frame.held as usize),
                    held: frame.held as usize,
                    parent: nested.contains(id),
                })
            })
            .collect();
        let mut owners: Vec<OwnerView> = model
            .owners
            .iter()
            .filter(|o| !model.hidden(Spot::Owner(o.id)))
            .filter_map(|o| {
                Some(OwnerView {
                    id: o.id,
                    at: *placed.owners.get(&o.id)?,
                    decl: o.decl,
                    name: o.name.clone(),
                    tint: owner_tint(o.kind),
                    words: owner_words[o.id as usize].clone(),
                    path: o.path.clone(),
                    label: o.label.clone(),
                    on_data: o.on_data,
                    folded: o.folded,
                    packed: packed_words(o.marks.len()),
                    held: o.marks.len(),
                })
            })
            .collect();
        owners.sort_by_key(|o| o.id);

        // ---- The wires. -----------------------------------------------------
        //
        // Every call is a line now, with both ends read through the folds: an
        // end a fold hides is **re-anchored to the box that stands for it**
        // rather than cut, because an elided line would take a chain off the
        // paper without saying so.
        let touched: HashSet<u32> = model
            .marks
            .iter()
            .filter(|m| m.letter().is_some())
            .map(|m| m.id)
            .collect();
        let own_h = |spot: Spot| -> f64 {
            match spot {
                Spot::Mark(id) => sizes.marks.get(&id).map_or(HEAD_H, |&(_, h)| h),
                Spot::Owner(_) => layout::OWNER_HEAD_H,
                Spot::Frame(_) => layout::FRAME_LABEL_H,
            }
        };
        let mut wires: Vec<WireView> = Vec::with_capacity(model.calls.len());
        let mut at_pair: HashMap<(Spot, Spot, bool), usize> = HashMap::new();
        for call in &model.calls {
            let answers = call.kind == CallKind::Answers;
            let (def, user) = (
                model.shown(Spot::Mark(call.def)),
                model.shown(Spot::Mark(call.user)),
            );
            // Both ends inside one fold: the box is the whole of what the reader
            // can see, and a line from a border to itself says nothing.
            if def == user {
                continue;
            }
            if let Some(&at) = at_pair.get(&(def, user, answers)) {
                let wire: &mut WireView = &mut wires[at];
                wire.count += call.count;
                wire.def_dirty |= touched.contains(&call.def);
                wire.user_dirty |= touched.contains(&call.user);
                continue;
            }
            let (Some(from), Some(to)) = (rects.get(&def), rects.get(&user)) else {
                continue;
            };
            at_pair.insert((def, user, answers), wires.len());
            let (from_own, to_own) = (own_h(def), own_h(user));
            let (from_side, to_side) = (from.tie_side(from_own, *to), to.tie_side(to_own, *from));
            let cross = model.frame_of(def) != model.frame_of(user);
            wires.push(WireView {
                key: format!("{def:?}-{user:?}-{}", answers as u8),
                from: from.tie_at(from_own, from_side, 0.5),
                to: to.tie_at(to_own, to_side, 0.5),
                def,
                user,
                count: call.count,
                label: None,
                title: None,
                to_mod: None,
                width: 0.0,
                def_dirty: touched.contains(&call.def),
                user_dirty: touched.contains(&call.user),
                cross,
                bundle: false,
                from_side,
                to_side,
                class: match answers {
                    true => "is-answers",
                    false => "is-call",
                },
            });
        }
        for wire in wires.iter_mut() {
            let answers = wire.class == "is-answers";
            wire.width = wire_width(answers, wire.count);
            wire.label = match answers {
                true => Some("answers".to_string()),
                false => (wire.count > 1).then(|| wire.count.to_string()),
            };
        }

        // **The fan.** Every end that ties to one edge of one head row spreads
        // across that edge instead of stacking on its middle: a head the survey
        // reaches from six places used to take six arrowheads on one point.
        let mut fan: HashMap<(Spot, TieSide), Vec<(usize, bool)>> = HashMap::new();
        for (at, wire) in wires.iter().enumerate() {
            fan.entry((wire.def, wire.from_side))
                .or_default()
                .push((at, false));
            fan.entry((wire.user, wire.to_side))
                .or_default()
                .push((at, true));
        }
        /// One edge of one box, and the wire ends that tie there: the wire's
        /// index, and whether it is that wire's arriving end.
        type FannedEdge = ((Spot, TieSide), Vec<(usize, bool)>);
        let mut edges: Vec<FannedEdge> = fan.into_iter().collect();
        edges.sort_by_key(|((spot, side), _)| (*spot, format!("{side:?}")));
        for ((id, side), mut ends) in edges {
            let Some(&place) = rects.get(&id) else {
                continue;
            };
            let along = |&(nth, to_end): &(usize, bool)| -> f64 {
                let wire = &wires[nth];
                let far = match to_end {
                    true => wire.def,
                    false => wire.user,
                };
                let Some(other) = rects.get(&far) else {
                    return 0.0;
                };
                match side {
                    TieSide::Top | TieSide::Under => other.x,
                    _ => other.y,
                }
            };
            ends.sort_by(|a, b| {
                along(a)
                    .total_cmp(&along(b))
                    .then_with(|| (a.0, a.1).cmp(&(b.0, b.1)))
            });
            let slots = ends.len() + 1;
            for (slot, (wire, to_end)) in ends.into_iter().enumerate() {
                let point = place.tie_at(own_h(id), side, (slot + 1) as f64 / slots as f64);
                match to_end {
                    true => wires[wire].to = point,
                    false => wires[wire].from = point,
                }
            }
        }

        // ---- The corridors. --------------------------------------------------
        //
        // At rest the cross-module family is one line per ordered module pair,
        // border to border, with the count it carries riding it. Drawing every
        // one of fifteen hundred cross-module calls at once is the hairball this
        // system forbids one rung up; drawing none of them would be a chart that
        // says nothing about how the modules talk. A contract is never bundled
        // and never counted into one — it is drawn whatever the reading rests,
        // so a corridor that counted it would be counting a line already on the
        // paper.
        let mut corridors: HashMap<(u32, u32), u32> = HashMap::new();
        for wire in wires.iter().filter(|w| w.cross && w.class == "is-call") {
            let (Some(def), Some(user)) = (model.frame_of(wire.def), model.frame_of(wire.user))
            else {
                continue;
            };
            *corridors.entry((def, user)).or_default() += wire.count.max(1);
        }
        let mut keys: Vec<(u32, u32)> = corridors.keys().copied().collect();
        keys.sort_unstable();
        let bundles: Vec<WireView> = keys
            .into_iter()
            .filter_map(|(def, user)| {
                let count = corridors[&(def, user)];
                let (from_at, to_at) = (
                    rects.get(&Spot::Frame(def))?,
                    rects.get(&Spot::Frame(user))?,
                );
                let (from, to) = from_at.tie_ends(*to_at);
                let (def_words, user_words) = (
                    model.frames.get(def as usize)?.words(),
                    model.frames.get(user as usize)?.words(),
                );
                Some(WireView {
                    key: format!("corridor-{def}-{user}"),
                    from,
                    to,
                    def: Spot::Frame(def),
                    user: Spot::Frame(user),
                    count,
                    label: Some(count.to_string()),
                    title: Some(format!(
                        "{user_words} calls {def_words} · {} · select {def_words}",
                        plural_calls(count)
                    )),
                    to_mod: Some(model.frames.get(def as usize)?.key()),
                    width: (1.2 + (count.max(1) as f64).ln() * 0.34).min(3.0),
                    def_dirty: false,
                    user_dirty: false,
                    cross: true,
                    bundle: true,
                    from_side: TieSide::Right,
                    to_side: TieSide::Left,
                    class: "is-call is-bundle",
                })
            })
            .collect();

        // ---- The far edition's landmark register. ----------------------------
        //
        // The frames big enough to be territory name themselves across it,
        // because below reading zoom a 10.5px head row is three pixels of dust
        // and the one question the opening view has to answer is *whose code is
        // this*. A box nested inside a named box has to **clear** that name —
        // one step smaller, and its own band of paper below the name above it —
        // so a chain of frames gets a ladder rather than a pile. Read outermost
        // first, so each box sees the register it stands under.
        let mut register: HashMap<u32, (f64, f64)> = HashMap::new();
        let mut names: Vec<NameView> = Vec::new();
        for view in &frames {
            let (mut ceiling, mut clear_of) = (LANDMARK_MAX, f64::NEG_INFINITY);
            for up in model.over(view.id) {
                if let Some(&(foot, size)) = register.get(&up) {
                    ceiling = size - LANDMARK_STEP;
                    clear_of = foot + LANDMARK_GAP;
                    break;
                }
            }
            if view.at.y < clear_of {
                continue;
            }
            let Some(label) = view.label.as_deref() else {
                continue;
            };
            if let Some(name) = NameView::measure(Spot::Frame(view.id), label, view.at, ceiling) {
                register.insert(view.id, (name.foot(), name.size));
                names.push(name);
            }
        }
        // Then the containers, under the register their frame leaves: the
        // heaviest rooms in a module name themselves too, because at far zoom
        // `impl FnModel` is the answer to the same question.
        let mut heavy: Vec<&OwnerView> = owners.iter().collect();
        heavy.sort_by_key(|o| (std::cmp::Reverse(o.held), o.id));
        for view in heavy {
            let frame = model.owners.get(view.id as usize).map(|o| o.frame);
            let (mut ceiling, mut clear_of) = (LANDMARK_MAX - LANDMARK_STEP, f64::NEG_INFINITY);
            if let Some(frame) = frame {
                let mut chain = vec![frame];
                chain.extend(model.over(frame));
                for up in chain {
                    if let Some(&(foot, size)) = register.get(&up) {
                        ceiling = size - LANDMARK_STEP;
                        clear_of = foot + LANDMARK_GAP;
                        break;
                    }
                }
            }
            if view.at.y < clear_of {
                continue;
            }
            if let Some(name) =
                NameView::measure(Spot::Owner(view.id), &view.name, view.at, ceiling)
            {
                names.push(name);
            }
        }

        let deepest = model.facts.deepest;
        let bands = model
            .marks
            .iter()
            .map(|m| (m.id, m.tier.band(deepest)))
            .collect();
        let frame = Rect::bounds(
            placed
                .frames
                .iter()
                .map(|(_, at)| Rect::new(at.x, at.y, at.w, at.h)),
        );
        FnDrawing {
            nodes,
            frames,
            owners,
            names,
            wires,
            bundles,
            bands,
            rects,
            locate,
            frame,
            dirty: model.dirty,
        }
    }
}

/// What a folded box states in words where what it holds stood. Nothing is
/// silently cut: the count is drawn only because something is hidden, and the
/// words are themselves the control that puts it back.
fn packed_words(held: usize) -> String {
    format!("+ {held} inside")
}

fn plural_calls(count: u32) -> String {
    match count {
        1 => "1 call".to_string(),
        n => format!("{n} calls"),
    }
}

/// The selection's own ink: what the chart lights, and what recedes.
///
/// A mark selection lights the mark and the far end of every wire the `wires`
/// reading keeps for it. An owner container lights its own methods and the far
/// end of everything crossing its border; a module boundary does the same one
/// room out. A band lights every mark at that depth.
#[derive(Clone, PartialEq)]
struct FnKin {
    /// The box in hand: a mark, or an owner container.
    sel: Option<Spot>,
    /// A module boundary.
    frame: Option<u32>,
    band: Option<u32>,
    lit: HashSet<Spot>,
    near: HashSet<Spot>,
    /// The wires this reading inks and keeps inked.
    wires: HashSet<(Spot, Spot)>,
}

impl FnKin {
    /// One mark in hand. The selection is the anchor the `wires` reading wants:
    /// `calls` keeps the wires leaving it, `callers` the wires arriving at it,
    /// `both` keeps both — so moving the switch with something selected moves
    /// the picture, which is the whole point of a direction.
    fn mark(sel: u32, model: &FnModel, reading: FnWires) -> Self {
        let wires: HashSet<(Spot, Spot)> = model
            .calls
            .iter()
            .filter(|c| reading.draws(&sel, &c.def, &c.user))
            .map(|c| {
                (
                    model.shown(Spot::Mark(c.def)),
                    model.shown(Spot::Mark(c.user)),
                )
            })
            .filter(|(def, user)| def != user)
            .collect();
        let at = model.shown(Spot::Mark(sel));
        let near: HashSet<Spot> = wires
            .iter()
            .map(|&(def, user)| if def == at { user } else { def })
            .filter(|spot| *spot != at)
            .collect();
        Self {
            sel: Some(at),
            frame: None,
            band: None,
            lit: HashSet::from([Spot::Mark(sel)]),
            near,
            wires,
        }
        .carry(model)
    }

    /// One box's whole boundary — an owner container, or a module frame. The
    /// box is the anchor, so the direction reads *the boundary*: `calls` keeps
    /// what the code inside runs beyond the line, `callers` whose code beyond
    /// the line runs something inside it.
    fn boundary(at: Spot, model: &FnModel, reading: FnWires) -> Self {
        let inside = model.inside(at);
        let wires: HashSet<(Spot, Spot)> = model
            .calls
            .iter()
            .filter(|c| inside.contains(&c.def) != inside.contains(&c.user))
            .filter(|c| reading.draws(&true, &inside.contains(&c.def), &inside.contains(&c.user)))
            .map(|c| {
                (
                    model.shown(Spot::Mark(c.def)),
                    model.shown(Spot::Mark(c.user)),
                )
            })
            .filter(|(def, user)| def != user)
            .collect();
        let mut lit: HashSet<Spot> = inside.iter().map(|&id| Spot::Mark(id)).collect();
        lit.insert(at);
        if let Spot::Frame(id) = at {
            // Everything the boundary holds keeps full ink, the rooms inside it
            // included: they are what the box is made of.
            let mut stack = vec![id];
            while let Some(frame) = stack.pop() {
                let Some(frame) = model.frames.get(frame as usize) else {
                    continue;
                };
                lit.extend(frame.owners.iter().map(|&o| Spot::Owner(o)));
                lit.extend(frame.kids.iter().map(|&k| Spot::Frame(k)));
                stack.extend(frame.kids.iter().copied());
            }
        }
        // Everything one call across the line reads a step behind — **both ways
        // round**, whatever direction the `wires` reading takes. What crosses a
        // boundary is what a reader came to the boundary for; the reading picks
        // which crossings are *drawn*, not which neighbours exist.
        let near: HashSet<Spot> = model
            .one_hop(&inside)
            .into_iter()
            .map(|id| model.shown(Spot::Mark(id)))
            .chain(wires.iter().flat_map(|&(def, user)| [def, user]))
            .filter(|spot| !lit.contains(spot))
            .collect();
        Self {
            sel: match at {
                Spot::Frame(_) => None,
                spot => Some(spot),
            },
            frame: match at {
                Spot::Frame(id) => Some(id),
                _ => None,
            },
            band: None,
            lit,
            near,
            wires,
        }
        .carry(model)
    }

    /// Where a fold hides part of what this reading lights, the box that stands
    /// for it carries the ink. Recede acts on a block's paint and never on its
    /// box, so a folded room whose methods are in the chain reads at full
    /// pressure: the reviewer has to see that the chain runs through it.
    fn carry(mut self, model: &FnModel) -> Self {
        if model.packs.is_empty() {
            return self;
        }
        for (hidden, rep) in &model.packs {
            if self.lit.contains(hidden) {
                self.lit.insert(*rep);
            }
        }
        for (hidden, rep) in &model.packs {
            if self.near.contains(hidden) {
                self.near.insert(*rep);
            }
        }
        self
    }

    fn whole_band(band: u32, model: &FnModel, drawing: &FnDrawing) -> Self {
        let lit: HashSet<Spot> = drawing
            .bands
            .iter()
            .filter(|(_, at)| **at == band)
            .map(|(id, _)| Spot::Mark(*id))
            .collect();
        Self {
            sel: None,
            frame: None,
            band: Some(band),
            lit,
            // A stratum holds sixty marks and every call on the sheet touches
            // one, so unfolding here would be the hairball this system forbids.
            near: HashSet::new(),
            wires: HashSet::new(),
        }
        .carry(model)
    }

    /// The box this reading is read from, where it is read from one: what the
    /// keyboard walks from, and what the fold key acts on.
    fn at(&self) -> Option<Spot> {
        self.sel.or(self.frame.map(Spot::Frame))
    }

    /// The class one box wears in this reading.
    fn class(&self, spot: Spot) -> &'static str {
        if self.sel == Some(spot) || self.frame.map(Spot::Frame) == Some(spot) {
            return "is-picked";
        }
        if self.lit.contains(&spot) {
            return "is-kin";
        }
        if self.near.contains(&spot) {
            return "is-near";
        }
        "is-dim"
    }

    /// The class one wire wears.
    fn wire_class(&self, wire: &WireView) -> &'static str {
        match self.inks(wire) || (self.lit.contains(&wire.def) && self.lit.contains(&wire.user)) {
            true => "is-kin",
            false => "is-dim",
        }
    }

    fn inks(&self, wire: &WireView) -> bool {
        self.wires.contains(&(wire.def, wire.user))
    }
}

// ---------------------------------------------------------------------------
// The drawing.
// ---------------------------------------------------------------------------

/// One block on the paper: the head row and the signature quoted under it.
#[component]
fn FnPlate(view: MeasuredBlock, kin: Option<FnKin>, hot: Signal<Option<u32>>) -> Element {
    let nav = use_navigator();
    let kin_class = kin.as_ref().map_or("", |k| k.class(Spot::Mark(view.id)));
    let picked = kin_class == "is-picked";
    let to = match picked {
        true => Route::FnOverview {},
        false => mark_route(&view.path, &view.label),
    };
    let title = match picked {
        true => format!(
            "{} {} — selected · click again to deselect, enter reads its source",
            view.decl, view.name
        ),
        false => view.title.clone(),
    };
    let push = to.clone();
    // Enter means one thing at this altitude: read the selected declaration's
    // own source. A head row that is not the selection yet is selected by it;
    // on the mark already in hand it opens the quotation. Clicking is what lets
    // a selection go, and Enter never does two things.
    let pressed = match picked {
        true => crate::views::func::peek_route(
            &(view.path.clone(), view.label.clone()),
            &view.path,
            &view.label,
        ),
        false => to.clone(),
    };
    let mut hot = hot;
    let id = view.id;
    rsx! {
        div {
            class: "fn-mark",
            class: if !kin_class.is_empty() { "{kin_class}" },
            class: if view.entry { "is-entry" },
            class: if view.ring { "is-ring" },
            class: if view.letter.is_some() { "is-diff" },
            header {
                class: "fm-head",
                role: "link",
                tabindex: "0",
                "aria-label": "{title}",
                title: "{title}",
                onclick: move |e: Event<MouseData>| {
                    e.prevent_default();
                    e.stop_propagation();
                    nav.push(push.clone());
                },
                onkeydown: move |e: Event<KeyboardData>| {
                    if e.key() == Key::Enter {
                        e.stop_propagation();
                        nav.push(pressed.clone());
                    }
                },
                onmouseenter: move |_| hot.set(Some(id)),
                onmouseleave: move |_| hot.set(None),
                span { class: "fm-kw", "{view.decl}" }
                // The declaration's own name, as rust writes it: the name and
                // the bracket the signature opens with, with no space invented
                // between them. Whose method it is, the container says.
                span { class: "fm-id",
                    span { class: "fm-nm", "{view.name}" }
                    if !view.open.is_empty() {
                        span { class: "fm-open", "{view.open}" }
                    }
                }
                if let Some(letter) = view.letter {
                    span {
                        class: "fm-chg",
                        title: match letter {
                            "A" => "added since the diff base",
                            _ => "declaration changed since the diff base",
                        },
                        "{letter}"
                    }
                }
            }
            // The signature, quoted as rust writes it: the receiver first, then
            // every parameter as the source declares it, then the return. A
            // line wider than the block ellipsizes and its hover words carry
            // the whole of it.
            if !view.rows.is_empty() || !view.tail.is_empty() {
                div { class: "fm-body",
                    for (i , row) in view.rows.iter().enumerate() {
                        p {
                            key: "{i}",
                            class: "fm-row",
                            class: if row.added { "is-add" },
                            title: "{row.written()}",
                            if row.added {
                                span { class: "fm-mk", "+" }
                            }
                            if row.ty.is_empty() {
                                span { class: "tok-kw", "{row.name}" }
                            } else {
                                span { class: "fm-pname", "{row.name}: " }
                                span { class: "fm-ty", "{row.ty}" }
                            }
                        }
                    }
                    if !view.tail.is_empty() {
                        p { class: "fm-tail", title: "{view.tail}", "{view.tail}" }
                    }
                }
            }
        }
    }
}

/// Node view for the function chart.
#[component]
fn FnNode(ctx: NodeViewCtx<FnNodeData>, kin: Option<FnKin>, hot: Signal<Option<u32>>) -> Element {
    let FnNodeData(view) = ctx.node.data.clone();
    rsx! {
        FnPlate { view: *view, kin, hot }
    }
}

/// The crate and module frames, on the ground under the blocks. Two gestures,
/// two marks, exactly as the data chart's module boundary carries them one rung
/// up: the border and its label **select the module**, and the mark at the
/// border's other end **folds** it.
#[component]
fn FrameLayer(
    frames: Vec<FrameView>,
    kin: Option<FnKin>,
    fold: Signal<Option<(Spot, bool)>>,
    /// A room a double-click asked the camera to fill the glass with.
    fit: Signal<Option<Spot>>,
) -> Element {
    let nav = use_navigator();
    let boundary = |f: &FrameView| -> Element {
        let class = kin.as_ref().map_or("", |k| k.class(Spot::Frame(f.id)));
        let chosen = class == "is-picked";
        let to = match chosen {
            true => Route::FnOverview {},
            false => mod_route(f.key.clone()),
        };
        let (clicked, pressed) = (to.clone(), to);
        let words = f.words.clone();
        let kind = match f.key.len() {
            1 => "crate",
            _ => "module",
        };
        let mut fold = fold;
        let mut fit = fit;
        let id = f.id;
        let shut = move |e: Event<MouseData>| {
            e.stop_propagation();
            let deep = e.modifiers().shift() || e.modifiers().alt();
            fold.set(Some((Spot::Frame(id), deep)));
        };
        let mark = if f.folded { "+" } else { "−" };
        let fold_words = match f.folded {
            true => format!("{words} is folded — {} · click to open it", f.packed),
            false => format!(
                "fold {words} to its own border — shift-click folds every room inside it too"
            ),
        };
        let (bx, by) = (f.at.x + f.at.w - 15.0, f.at.y);
        rsx! {
            g {
                key: "{f.id}",
                class: "fn-frame-group",
                class: if !class.is_empty() { "{class}" },
                rect {
                    class: "fn-frame",
                    x: "{f.at.x}",
                    y: "{f.at.y}",
                    width: "{f.at.w}",
                    height: "{f.at.h}",
                }
                g {
                    class: "fn-frame-pick",
                    tabindex: "0",
                    role: "link",
                    "aria-label": if chosen { "deselect {words}" } else { "select the {kind} {words}" },
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        nav.push(clicked.clone());
                    },
                    ondoubleclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        fit.set(Some(Spot::Frame(id)));
                    },
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter {
                            e.stop_propagation();
                            nav.push(pressed.clone());
                        }
                    },
                    title {
                        if chosen {
                            "{words} — selected · click the border again to let it go"
                        } else {
                            "{words} — {f.held} declarations · select this {kind} · double-click fills the glass with it"
                        }
                    }
                    rect {
                        class: "fn-frame-hit",
                        x: "{f.at.x}",
                        y: "{f.at.y}",
                        width: "{f.at.w}",
                        height: "{f.at.h}",
                    }
                    if let Some(label) = f.label.clone() {
                        rect {
                            class: "fn-frame-tab",
                            x: "{f.at.x + 8.0}",
                            y: "{f.at.y - 9.0}",
                            width: "{f.label_w + 12.0}",
                            height: "18",
                        }
                        text {
                            class: "fn-frame-label",
                            x: "{f.at.x + 14.0}",
                            y: "{f.at.y}",
                            "{label}"
                        }
                    }
                }
                // Nothing is silently cut: a folded room says what it holds
                // back, on the border, right after its own label — one line, so
                // the name and the count read together and neither collides
                // with the far edition's landmark across the box below.
                if f.folded {
                    text {
                        class: "fn-shut-words",
                        x: "{f.at.x + 26.0 + f.label_w}",
                        y: "{f.at.y}",
                        "{f.packed}"
                    }
                }
                if f.label.is_some() {
                    g {
                        class: "fn-frame-shut",
                        tabindex: "0",
                        role: "button",
                        // The mark rides the glass: see `FN_SLIDE_JS`. A crate
                        // frame is as wide as the sheet, so its right edge is
                        // off the glass whenever anything inside it is on.
                        style: "--own-r: {f.at.x + f.at.w}px; --own-w: {f.at.w}px;",
                        "aria-label": "{fold_words}",
                        onclick: shut,
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter {
                                e.stop_propagation();
                                fold.set(Some((Spot::Frame(id), false)));
                            }
                        },
                        title { "{fold_words}" }
                        rect {
                            class: "fn-frame-hit-mark",
                            x: "{bx - 10.0}",
                            y: "{by - 9.0}",
                            width: "20",
                            height: "18",
                        }
                        text {
                            class: "fn-frame-mark",
                            x: "{bx}",
                            y: "{by}",
                            text_anchor: "middle",
                            "{mark}"
                        }
                    }
                }
            }
        }
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for f in frames.iter() {
                {boundary(f)}
            }
        }
    }
}

/// The owner containers, on the ground over the module frames and under the
/// blocks. Three marks on one border, each its own gesture: the **border**
/// selects the container, the **name** descends to that type's block on `/data`
/// (or selects the container where that chart draws none), and the `–` / `+`
/// folds the methods inside it.
#[component]
fn OwnerLayer(
    owners: Vec<OwnerView>,
    kin: Option<FnKin>,
    fold: Signal<Option<(Spot, bool)>>,
    fit: Signal<Option<Spot>>,
) -> Element {
    let nav = use_navigator();
    let container = |o: &OwnerView| -> Element {
        let class = kin.as_ref().map_or("", |k| k.class(Spot::Owner(o.id)));
        let chosen = class == "is-picked";
        let to = match chosen {
            true => Route::FnOverview {},
            false => owner_route(&o.path, &o.label),
        };
        let (clicked, pressed) = (to.clone(), to.clone());
        // The name is a descent link where the rung above draws a block for the
        // owner; a trait has none there, so its name is the container's own
        // selection instead. The same rule the `Data touched` rows keep.
        let down = match o.on_data {
            true => crate::views::data::mark_route(&o.path, &o.label),
            false => to,
        };
        let down_words = match o.on_data {
            true => format!("{} — down to its block on the data chart", o.name),
            false => format!("{} — select this container", o.name),
        };
        let words = o.words.clone();
        let mut fold = fold;
        let mut fit = fit;
        let id = o.id;
        let shut = move |e: Event<MouseData>| {
            e.stop_propagation();
            fold.set(Some((Spot::Owner(id), false)));
        };
        let mark = if o.folded { "+" } else { "−" };
        let fold_words = match o.folded {
            true => format!("{words} is folded — {} · click to open it", o.packed),
            false => format!("fold {words} to its own border"),
        };
        let (bx, by) = (o.at.x + o.at.w - 12.0, o.at.y + 11.0);
        let decl_w = text_w(o.decl, LABEL_PX) + 4.0;
        rsx! {
            g {
                key: "{o.id}",
                class: "fn-owner-group",
                class: if !class.is_empty() { "{class}" },
                rect {
                    class: "fn-owner",
                    x: "{o.at.x}",
                    y: "{o.at.y}",
                    width: "{o.at.w}",
                    height: "{o.at.h}",
                }
                g {
                    class: "fn-owner-pick",
                    tabindex: "0",
                    role: "link",
                    "aria-label": if chosen { "deselect {words}" } else { "select {words}" },
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        nav.push(clicked.clone());
                    },
                    ondoubleclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        fit.set(Some(Spot::Owner(id)));
                    },
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter {
                            e.stop_propagation();
                            nav.push(pressed.clone());
                        }
                    },
                    title {
                        if chosen {
                            "{words} — selected · click the border again to let it go"
                        } else {
                            "{words} — {o.held} methods · click the border to select the whole container · double-click fills the glass with it"
                        }
                    }
                    rect {
                        class: "fn-owner-hit",
                        x: "{o.at.x}",
                        y: "{o.at.y}",
                        width: "{o.at.w}",
                        height: "{o.at.h}",
                    }
                    text {
                        class: "fn-owner-kw",
                        x: "{o.at.x + 5.0}",
                        y: "{o.at.y + 11.0}",
                        "{o.decl}"
                    }
                }
                g {
                    class: "fn-owner-down",
                    tabindex: "0",
                    role: "link",
                    "aria-label": "{down_words}",
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        nav.push(down.clone());
                    },
                    title { "{down_words}" }
                    text {
                        class: "fn-owner-name",
                        class: if !o.tint.is_empty() { "{o.tint}" },
                        x: "{o.at.x + 5.0 + decl_w}",
                        y: "{o.at.y + 11.0}",
                        "{o.name}"
                    }
                }
                if o.folded {
                    text {
                        class: "fn-shut-words",
                        x: "{o.at.x + 15.0 + decl_w + text_w(&o.name, LABEL_PX)}",
                        y: "{o.at.y + 11.0}",
                        "{o.packed}"
                    }
                }
                g {
                    class: "fn-owner-shut",
                    tabindex: "0",
                    role: "button",
                    style: "--own-r: {o.at.x + o.at.w}px; --own-w: {o.at.w}px;",
                    "aria-label": "{fold_words}",
                    onclick: shut,
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter {
                            e.stop_propagation();
                            fold.set(Some((Spot::Owner(id), false)));
                        }
                    },
                    title { "{fold_words}" }
                    rect {
                        class: "fn-frame-hit-mark",
                        x: "{bx - 9.0}",
                        y: "{by - 9.0}",
                        width: "18",
                        height: "16",
                    }
                    text {
                        class: "fn-owner-mark",
                        x: "{bx}",
                        y: "{by}",
                        text_anchor: "middle",
                        "{mark}"
                    }
                }
            }
        }
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for o in owners.iter() {
                {container(o)}
            }
        }
    }
}

/// The far edition's **landmark register**: each room's own name engraved
/// across its territory, the way the data chart names a module region one rung
/// up and for the same reason. At the opening zoom the camera has scaled a
/// 10.5px head row down to three pixels, and the question the opening view
/// exists to answer — whose code is this — is a question about names.
#[component]
fn NameLayer(names: Vec<NameView>, kin: Option<FnKin>) -> Element {
    let class = |at: Spot| match kin.as_ref() {
        None => "",
        Some(kin) if kin.at() == Some(at) => "is-sel",
        Some(kin) if kin.lit.contains(&at) => "",
        Some(_) => "is-dim",
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for (i , n) in names.iter().enumerate() {
                text {
                    key: "{i}",
                    class: "fn-frame-name",
                    class: if !class(n.at).is_empty() { "{class(n.at)}" },
                    x: "{n.x}",
                    y: "{n.y}",
                    font_size: "{n.size}",
                    style: "stroke-width: {(n.size * 0.38).round()}px;",
                    "{n.name}"
                }
            }
        }
    }
}

fn curve(a: Point, b: Point, side: f64) -> (String, Point) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let bow = (len * 0.14).min(46.0) * side;
    let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let ctrl = Point::new(mid.x - dy / len * bow, mid.y + dx / len * bow);
    (
        format!(
            "M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}",
            a.x, a.y, ctrl.x, ctrl.y, b.x, b.y
        ),
        ctrl,
    )
}

fn arrowhead(b: Point, ctrl: Point, size: f64) -> String {
    let (ex, ey) = (b.x - ctrl.x, b.y - ctrl.y);
    let len = (ex * ex + ey * ey).sqrt().max(1e-3);
    let (ux, uy) = (ex / len, ey / len);
    let (px, py) = (-uy, ux);
    format!(
        "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
        b.x,
        b.y,
        b.x - ux * size * 1.8 + px * size * 0.6,
        b.y - uy * size * 1.8 + py * size * 0.6,
        b.x - ux * size * 1.8 - px * size * 0.6,
        b.y - uy * size * 1.8 - py * size * 0.6,
    )
}

/// One wire, drawn: curve, head, and the label riding the curve's midpoint. A
/// corridor also carries its own words and is the way to the module at its far
/// end, because a count a reader cannot follow has no subject.
fn draw_wire(
    w: &WireView,
    side: f64,
    classes: &str,
    nav: Option<&dioxus::router::Navigator>,
) -> Element {
    let (d, ctrl) = curve(w.from, w.to, side);
    let head = arrowhead(w.to, ctrl, 3.2 + w.width);
    let (lx, ly) = (
        0.25 * w.from.x + 0.5 * ctrl.x + 0.25 * w.to.x,
        0.25 * w.from.y + 0.5 * ctrl.y + 0.25 * w.to.y,
    );
    let go = match (nav, w.to_mod.clone()) {
        (Some(nav), Some(key)) => Some((*nav, key)),
        _ => None,
    };
    rsx! {
        g { key: "{w.key}", class: "fn-wire {w.class}",
            class: "{classes}",
            if let Some(title) = w.title.clone() {
                title { "{title}" }
            }
            path {
                class: "wire-path",
                d: d.clone(),
                fill: "none",
                style: "stroke-width: {w.width}px;",
            }
            path { class: "wire-head", d: head }
            if let Some((nav, key)) = go {
                path {
                    class: "wire-hit",
                    d: d.clone(),
                    fill: "none",
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        nav.push(mod_route(key.clone()));
                    },
                }
            }
            if let Some(label) = w.label.clone() {
                text {
                    class: "wire-label",
                    x: "{lx}",
                    y: "{ly - 3.0}",
                    text_anchor: "middle",
                    "{label}"
                }
            }
        }
    }
}

/// Whether one wire is on the paper under this reading.
///
/// Direction is read **against an anchor**, and the anchor is whatever is in
/// focus: the selection where there is one, else the diff, else nothing. So:
///
/// - a **corridor** stands on the resting plate and nowhere else: a selection
///   unbundles, because the reader asked about one box and a bundle answers
///   about a whole module;
/// - a wire the selection's own reading inks is drawn, whichever stop is on —
///   the selection *is* the anchor, and its direction was already applied when
///   [`FnKin`] read it;
/// - with a selection, everything else is off the paper;
/// - with no selection and no diff, the sheet rests: the calls inside one module
///   draw as their own short lines, and the cross-module family is said by the
///   corridors rather than by fifteen hundred near-parallel diagonals;
/// - with no selection but a diff, the diff's touched declarations are the
///   anchor — `calls` draws the lines leaving them, `callers` the lines arriving
///   at them — **over** the corridors, which keep standing. The corridors are
///   the shape and the anchored lines are the change, and a review wants both:
///   a workspace under review is nearly always dirty, so a rule that took the
///   corridors away whenever the diff had something to say would be a resting
///   reading almost nobody ever saw.
///
/// A contract wire (`answers`) never folds and never bundles, whatever the
/// reading says: it is what makes the chart honest about a `dyn` call the survey
/// cannot follow.
fn drawn_under(reading: FnWires, w: &WireView, picked: bool, dirty: bool, lit: bool) -> bool {
    if w.bundle {
        return !picked;
    }
    if lit || w.class == "is-answers" {
        return true;
    }
    if picked {
        return false;
    }
    match dirty {
        true => w.anchored(reading),
        false => !w.cross,
    }
}

/// Both families as one engraved layer, the contracts first and lighter.
///
/// Drawn **twice**, at two altitudes, because a wire at rest and a wire the
/// reader lit are two different kinds of ink: the resting families and the
/// strangers a reading pushed back go **under** the blocks, where the paper of
/// every block they pass behind covers them; the selection's own lit reading
/// goes **over** them, because ink the reader asked for has to be followable
/// end to end.
///
/// No wire on this chart ever takes the flare. The structural diff reads the
/// base edition syntactically, so there is no such thing as a changed call for
/// it to ink.
#[component]
fn WireLayer(
    wires: Vec<WireView>,
    bundles: Vec<WireView>,
    kin: Option<FnKin>,
    reading: FnWires,
    /// Whether the diff has anything to say.
    dirty: bool,
    /// Whether this is the layer over the blocks — the lit reading — or the
    /// resting one under them.
    over: bool,
) -> Element {
    let nav = use_navigator();
    let picked = kin.is_some();
    let wire = |w: &WireView, side: f64| {
        let lit = kin.as_ref().is_some_and(|k| k.inks(w));
        if !drawn_under(reading, w, picked, dirty, lit) {
            return None;
        }
        let classes = match (w.bundle, kin.as_ref()) {
            // A corridor exists only on the resting plate and is that plate's
            // own reading: full pressure by construction, outside the kin
            // machinery entirely.
            (true, _) => "is-quiet",
            (false, Some(kin)) => kin.wire_class(w),
            // No selection: the diff is the anchor, so a wire it reads in this
            // direction carries the resting pressure and one the sheet merely
            // admits is drawn a step lighter.
            (false, None) if dirty && w.anchored(reading) => "is-quiet",
            (false, None) => "is-faint",
        };
        // One altitude per kind of ink: the lit reading rides over the blocks,
        // everything at rest under them.
        if (classes == "is-kin") != over {
            return None;
        }
        Some(draw_wire(w, side, classes, w.bundle.then_some(&nav)))
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for w in wires.iter().filter(|w| w.class == "is-answers") {
                {wire(w, -1.0)}
            }
            for w in wires.iter().filter(|w| w.class != "is-answers") {
                {wire(w, 1.0)}
            }
            for w in bundles.iter() {
                {wire(w, 1.0)}
            }
        }
    }
}

/// The hovered reading's own ink, drawn over the resting family in a compositor
/// layer of its own. **Hovering a mark unbundles it**: every wire that mark has
/// inks, both ways round, because what a reader hovers a block for is what the
/// rooms could not tell them.
#[component]
fn HotWireLayer(wires: Vec<WireView>, hot: Signal<Option<u32>>) -> Element {
    let h = hot();
    let lit = |w: &WireView, side: f64| {
        h.is_some_and(|h| w.def == Spot::Mark(h) || w.user == Spot::Mark(h))
            .then(|| draw_wire(w, side, "is-hot", None))
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for w in wires.iter().filter(|w| w.class == "is-answers") {
                {lit(w, -1.0)}
            }
            for w in wires.iter().filter(|w| w.class != "is-answers") {
                {lit(w, 1.0)}
            }
        }
    }
}

/// Chrome insets at this altitude: the cartouche column left, the sheet right
/// while something is selected — the choreography every altitude keeps.
fn chrome_insets(narrow: bool, panel: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (312.0, 20.0, 70.0, 12.0)
    } else {
        (56.0, if panel { 330.0 } else { 24.0 }, 24.0, 284.0)
    }
}

const MIN_CHART_ZOOM: f64 = 0.18;

/// Below this the quoted rows are dust and the chart holds its far edition:
/// names, borders and corridors alone. Hysteresis keeps the swap from flapping
/// while the reader hovers around the threshold.
const FAR_IN: f64 = 0.45;
const FAR_OUT: f64 = 0.55;
/// The zoom a selection is read at: when a chosen box sits below this, or off
/// the glass entirely, the camera glides to it.
#[cfg(target_arch = "wasm32")]
const READ_ZOOM: f64 = 0.5;

/// The zoom the chart opens at, at the lowest.
#[cfg(target_arch = "wasm32")]
const OPEN_FLOOR: f64 = 0.34;
/// And no closer than this on a small workspace, where the whole sheet fits.
#[cfg(target_arch = "wasm32")]
const OPEN_CEIL: f64 = 0.75;

/// The camera as the reviewer last left it, surviving route-variant remounts.
/// One camera, since 2026-08-27: the `order` reading that gave this chart three
/// sheets went with the shelved section, and the household has only one.
#[derive(Clone, Copy)]
pub(in crate::views) struct FnCamera {
    pub(in crate::views) viewport: Signal<Option<Viewport>>,
}

impl FnCamera {
    pub(in crate::views) fn new() -> Self {
        Self {
            viewport: Signal::new(None),
        }
    }
}

/// Put a reader down on a sheet they have not seen: the whole chart where that
/// is legible, and its top-left corner at reading scale where it is not.
#[cfg(target_arch = "wasm32")]
fn open_chart(flow: dioxus_flow::prelude::FlowHandle<FnNodeData>, whole: Rect, panel: bool) {
    let Some((w, h)) = window_size() else { return };
    let (t, r, b, l) = chrome_insets(narrow_viewport(), panel);
    let free_w = (w - l - r).max(120.0);
    let free_h = (h - t - b).max(120.0);
    let fit = (free_w / whole.width.max(1.0)).min(free_h / whole.height.max(1.0)) * 0.94;
    if fit >= OPEN_FLOOR {
        frame_chart(flow, whole, panel, 0);
        return;
    }
    // Too much sheet to read at once: hold the floor and open on its start.
    let Some(core) = flow.core() else { return };
    let zoom = fit.clamp(OPEN_FLOOR, OPEN_CEIL);
    core.set_viewport(
        Viewport::new(l + 12.0 - whole.x * zoom, t + 12.0 - whole.y * zoom, zoom),
        0,
    );
}

fn frame_chart(
    flow: dioxus_flow::prelude::FlowHandle<FnNodeData>,
    bounds: Rect,
    panel: bool,
    duration_ms: u64,
) {
    let Some(core) = flow.core() else { return };
    let Some((w, h)) = window_size() else {
        return;
    };
    let (t, r, b, l) = chrome_insets(narrow_viewport(), panel);
    let free_w = (w - l - r).max(120.0);
    let free_h = (h - t - b).max(120.0);
    let fit = (free_w / bounds.width.max(1.0)).min(free_h / bounds.height.max(1.0)) * 0.94;
    let zoom = fit.clamp(MIN_CHART_ZOOM, 1.6);
    let center = bounds.center();
    let cx = l + free_w / 2.0;
    let cy = t + free_h / 2.0;
    core.set_viewport(
        Viewport::new(cx - center.x * zoom, cy - center.y * zoom, zoom),
        duration_ms,
    );
}

/// Keyboard at this altitude: `↓` steps into the mark's heaviest callee and `↑`
/// to its heaviest caller — a glide across the paper now, because the household
/// seats a callee wherever its own code is written — `z` folds the room in hand
/// the way vim folds a block, `enter` opens the picked declaration's own source,
/// `f` refits, Escape steps back out of the deepest thing open (a quotation
/// first, then the selection), and `/` finds.
///
/// **Left and right are the trail**, always — the browser's own back and
/// forward, exactly as at every other altitude (2026-08-27, user). This chart
/// owns the whole arrow grammar rather than sharing it with the shell's
/// listener, so it has to answer those two keys itself.
///
/// `enter` is only the chart's while the page's own focus is not on a control:
/// a head row, a border and a fold mark all answer Enter themselves.
const FN_KEYS_JS: &str = r#"
if (window.__slopeKeys) {
    document.removeEventListener('keydown', window.__slopeKeys);
}
window.__slopeKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') {
        e.preventDefault();
        const s = [...document.querySelectorAll('#fn-search')]
            .find((el) => el.offsetParent !== null);
        if (s) s.focus();
        return;
    }
    const arrows = ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'];
    if (arrows.includes(e.key)) {
        e.preventDefault();
        dioxus.send(e.key);
        return;
    }
    if (e.key === 'Enter') {
        if (t && t.closest('a, button, [role="link"], [role="button"]')) return;
        e.preventDefault();
        dioxus.send('Enter');
        return;
    }
    if (['f', 'z', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopeKeys);
"#;

/// **The fold mark rides the glass.** One number, published on the chart root,
/// that lets every wide box's fold mark sit at the right edge of the *visible*
/// span instead of at the right edge of a two-thousand-unit border that is
/// mostly off screen: `--cam-r`, the world x the viewport's right edge is over.
///
/// The arithmetic is CSS, in `tailwind.css`: a mark translates left by
/// `--cam-r − --own-r`, clamped so it never leaves its own box. So the whole
/// slide costs **one custom property per animation frame** and no re-render at
/// all — this is not a component, because a component that read the viewport
/// would re-render on every pan frame.
const FN_SLIDE_JS: &str = r#"
if (window.__slopeSlide) cancelAnimationFrame(window.__slopeSlide);
(() => {
    let wide = 0, last = null;
    const measure = () => {
        const pane = document.querySelector('.fn-chart .df-canvas, .fn-chart');
        wide = pane ? pane.clientWidth : 0;
    };
    measure();
    window.addEventListener('resize', measure);
    const tick = () => {
        window.__slopeSlide = requestAnimationFrame(tick);
        const chart = document.querySelector('.fn-chart');
        const pane = chart && chart.querySelector('.df-viewport');
        if (!chart || !pane) return;
        if (!wide) measure();
        const m = /translate\(([-0-9.]+)px,\s*([-0-9.]+)px\)\s*scale\(([-0-9.]+)\)/
            .exec(pane.style.transform || '');
        if (!m) return;
        const zoom = parseFloat(m[3]) || 1;
        const right = (wide - parseFloat(m[1])) / zoom;
        if (last !== null && Math.abs(right - last) < 0.5) return;
        last = right;
        chart.style.setProperty('--cam-r', right + 'px');
    };
    tick();
})();
"#;

/// The function chart, mounted for `/fn`.
#[component]
pub(super) fn FnChart(
    graph: CodeGraph,
    sel: Option<FnSel>,
    /// Where Escape lands while a quotation is open: the same selection,
    /// unquoted. `None` when nothing is quoted, and Escape deselects.
    unquote: Option<Route>,
) -> Element {
    let fns = use_fns();
    let camera = use_context::<FnCamera>();
    let flow = dioxus_flow::use_flow_handle::<FnNodeData>();
    let nav = use_navigator();

    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        FnModel::build(&graph, &fns.reading())
    }));
    let chart = use_memo(move || FnDrawing::build(&model.read()));

    // The reading is part of the selection's ink, because direction is read
    // against the selection: `calls` and `callers` keep different wires of the
    // same chosen box, so moving the switch has to re-read the kin.
    let kin: Memo<Option<FnKin>> = use_memo(use_reactive(
        (&sel, &*fns.wires.read()),
        move |(sel, reading)| {
            let model = model.read();
            let drawing = chart.read();
            match sel? {
                FnSel::Mark(path, label) => {
                    let mark = model.find(&path, &label)?;
                    Some(FnKin::mark(mark.id, &model, reading))
                }
                FnSel::Owner(path, label) => {
                    let owner = model.owner_at(&path, &label)?;
                    Some(FnKin::boundary(Spot::Owner(owner.id), &model, reading))
                }
                FnSel::Mod(key) => {
                    let frame = model.frame_at(&key)?;
                    Some(FnKin::boundary(Spot::Frame(frame.id), &model, reading))
                }
                FnSel::Band(band) => Some(FnKin::whole_band(band, &model, &drawing)),
            }
        },
    ));

    let sel_on: Signal<bool> = use_signal(|| false);
    let panel = matches!(
        sel,
        Some(FnSel::Mark(..) | FnSel::Owner(..) | FnSel::Band(..))
    );
    use_effect(use_reactive((&panel,), move |(on,)| {
        let mut sel_on = sel_on;
        if *sel_on.peek() != on {
            sel_on.set(on);
        }
    }));
    // The keyboard listener is mounted once and outlives every route change,
    // so what Escape closes — and what the arrows walk from — ride in on
    // signals rather than the props.
    let quoted: Signal<Option<Route>> = use_signal(|| None);
    use_effect(use_reactive((&unquote,), move |(unquote,)| {
        let mut quoted = quoted;
        if *quoted.peek() != unquote {
            quoted.set(unquote);
        }
    }));
    let chosen: Signal<Option<Spot>> = use_signal(|| None);
    let anything: Signal<bool> = use_signal(|| false);
    use_effect(use_reactive((&sel.is_some(),), move |(on,)| {
        let mut anything = anything;
        if *anything.peek() != on {
            anything.set(on);
        }
    }));
    use_effect(move || {
        let at = kin.read().as_ref().and_then(FnKin::at);
        let mut chosen = chosen;
        if *chosen.peek() != at {
            chosen.set(at);
        }
    });

    let nodes: Signal<Vec<FlowNode<FnNodeData>>> = use_signal(Vec::new);
    // What the pointer is over, and what the chart answers it with.
    let hot: Signal<Option<u32>> = use_signal(|| None);
    let settled = use_settled(hot);
    // A box a fold mark asked to be closed or opened, and whether the reader
    // asked for every room under it.
    let fold: Signal<Option<(Spot, bool)>> = use_signal(|| None);
    // A room a double-click asked the camera to fill the glass with.
    let fit: Signal<Option<Spot>> = use_signal(|| None);
    // Bumped whenever a reveal opened the way in to a selection.
    let revealed: Signal<u32> = use_signal(|| 0);
    let core_live: Signal<bool> = use_signal(|| false);
    let far: Signal<bool> = use_signal(|| false);

    // The fold gesture, acted on where the model is. Nothing here folds by a
    // count — this is the only place a fold is ever written.
    use_effect(move || {
        let Some((at, deep)) = fold() else { return };
        let mut fold = fold;
        fold.set(None);
        // Every key this gesture touches is read out of the model *first*, and
        // the borrow dropped, before the fold set is written: the model is a
        // memo over that very set.
        let Some((shut, keys)) = ({
            let model = model.peek();
            match at {
                Spot::Owner(id) => model
                    .owners
                    .get(id as usize)
                    .map(|owner| (!owner.folded, vec![owner.key()])),
                Spot::Frame(id) => model.frames.get(id as usize).map(|frame| {
                    let shut = !frame.folded;
                    let mut keys = vec![mod_fold(&frame.key())];
                    if deep {
                        // Every room under it, in one gesture: fold them all so
                        // opening this one shows one layer, or open every fold
                        // inside it so the reader gets the whole household back
                        // at once.
                        let mut stack = vec![id];
                        while let Some(at) = stack.pop() {
                            let Some(under) = model.frames.get(at as usize) else {
                                continue;
                            };
                            for &owner in &under.owners {
                                if let Some(owner) = model.owners.get(owner as usize) {
                                    keys.push(owner.key());
                                }
                            }
                            for &kid in &under.kids {
                                if let Some(kid) = model.frames.get(kid as usize) {
                                    keys.push(mod_fold(&kid.key()));
                                }
                                stack.push(kid);
                            }
                        }
                    }
                    (shut, keys)
                }),
                Spot::Mark(_) => None,
            }
        }) else {
            return;
        };
        fns.fold(keys, shut);
    });

    // A double-click on a room's border fills the glass with that room — the
    // one camera move a gesture on the ground asks for, and the reason a reader
    // can walk down the household without the search.
    use_effect(move || {
        let Some(at) = fit() else { return };
        let mut fit = fit;
        fit.set(None);
        let Some(rect) = chart.peek().rects.get(&at).copied() else {
            return;
        };
        let duration = if prefers_reduced_motion() { 0 } else { 400 };
        frame_chart(
            flow,
            Rect::new(rect.x, rect.y, rect.w, rect.h),
            *sel_on.peek(),
            duration,
        );
    });

    // The one control that moves every block on the sheet. The paper is being
    // laid again for it anyway, so this is where the packer is allowed to catch
    // up with the folds and close the sheet over what they hide — the reader has
    // no anchor to lose, because nothing is where it was.
    use_effect(use_reactive((&*fns.vis_floor.read(),), move |_| {
        fns.repack()
    }));

    // A selection must never be invisible: every way to a box — a URL, the
    // search, a sheet row, the arrow walk — opens the folds it is hiding behind
    // first. The model is peeked, never read, so this fires when the *selection*
    // moves and not when the folds do.
    use_effect(use_reactive((&sel,), move |(sel,)| {
        let Some(sel) = sel.as_ref() else { return };
        let model = model.peek();
        let spot = match sel {
            FnSel::Mark(path, label) => model.find(path, label).map(|m| Spot::Mark(m.id)),
            FnSel::Owner(path, label) => model.owner_at(path, label).map(|o| Spot::Owner(o.id)),
            FnSel::Mod(key) => model.frame_at(key).map(|f| Spot::Frame(f.id)),
            FnSel::Band(_) => None,
        };
        let Some(spot) = spot else { return };
        let way_in = model.reveal(spot);
        if way_in.is_empty() {
            return;
        }
        fns.fold(way_in, false);
        let mut revealed = revealed;
        let now = *revealed.peek();
        revealed.set(now + 1);
    }));

    use_effect(move || {
        let drawing = chart();
        let mut nodes = nodes;
        nodes.set(drawing.nodes);
        // Camera discipline: the reader gets their place back, and a sheet they
        // have not seen is opened once.
        #[cfg(target_arch = "wasm32")]
        {
            let frame = drawing.frame;
            let panel = *sel_on.peek();
            let mut core_live = core_live;
            let kept = *camera.viewport.peek();
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(150).await;
                for _ in 0..40 {
                    if flow.core().is_some() {
                        break;
                    }
                    gloo_timers::future::TimeoutFuture::new(50).await;
                }
                core_live.set(true);
                match kept {
                    Some(vp) => flow.set_viewport(vp, 0),
                    None => {
                        if let Some(frame) = frame {
                            open_chart(flow, frame, panel);
                        }
                    }
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (core_live, camera);
            if let Some(frame) = drawing.frame {
                frame_chart(flow, frame, false, 0);
            }
        }
    });

    use_effect(move || {
        if !core_live() {
            return;
        }
        let Some(core) = flow.core() else { return };
        let vp = *core.viewport.read();
        let mut saved = camera.viewport;
        saved.set(Some(vp));
        // Zoom is the fold: crossing the threshold swaps the whole chart
        // between its near and far editions, once per crossing.
        let mut far = far;
        let now = *far.peek();
        if now && vp.zoom > FAR_OUT {
            far.set(false);
        } else if !now && vp.zoom < FAR_IN {
            far.set(true);
        }
    });

    // The camera glides to a selection it cannot show: off the glass, or below
    // reading zoom. A selection already legible moves nothing.
    #[cfg(target_arch = "wasm32")]
    use_effect(use_reactive((&sel,), move |(sel,)| {
        if !core_live() {
            return;
        }
        // A reveal is the one re-layout the camera answers.
        let _ = revealed();
        let Some(core) = flow.core() else { return };
        let drawing = chart.peek();
        let model = model.peek();
        let spot = match &sel {
            Some(FnSel::Mark(path, label)) => model.find(path, label).map(|m| Spot::Mark(m.id)),
            Some(FnSel::Owner(path, label)) => {
                model.owner_at(path, label).map(|o| Spot::Owner(o.id))
            }
            Some(FnSel::Mod(key)) => model.frame_at(key).map(|f| Spot::Frame(f.id)),
            _ => None,
        };
        let Some(at) = spot.and_then(|spot| drawing.rects.get(&spot).copied()) else {
            return;
        };
        let vp = *core.viewport.peek();
        let Some((w, h)) = window_size() else { return };
        let (vx, vy) = ((0.0 - vp.x) / vp.zoom, (0.0 - vp.y) / vp.zoom);
        let (vw, vh) = (w / vp.zoom, h / vp.zoom);
        // A room is as wide as everything it holds, so what has to be on the
        // glass is its label — the row that says which room this is.
        let head = Placed {
            w: at.w.min(320.0),
            h: at.h.min(HEAD_H + 2.0),
            ..at
        };
        let inside = head.x >= vx
            && head.y >= vy
            && head.x + head.w <= vx + vw
            && head.y + head.h <= vy + vh;
        if inside && vp.zoom >= READ_ZOOM {
            return;
        }
        let z = vp.zoom.clamp(0.85, 1.0);
        let (t, r, btm, l) = chrome_insets(narrow_viewport(), true);
        let free_w = (w - l - r).max(120.0);
        let free_h = (h - t - btm).max(120.0);
        let (cx, cy) = (l + free_w / 2.0, t + free_h / 2.0);
        let (mx, my) = (head.x + head.w / 2.0, head.y + head.h / 2.0);
        let duration = if prefers_reduced_motion() { 0 } else { 400 };
        core.set_viewport(Viewport::new(cx - mx * z, cy - my * z, z), duration);
    }));

    // The fold mark's slide, installed once beside the keys. It talks to CSS
    // through one custom property and never to dioxus, so a pan costs no
    // re-render.
    use_hook(|| {
        document::eval(FN_SLIDE_JS);
    });

    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(FN_KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                match key.as_str() {
                    "f" => {
                        if let Some(bounds) = chart.peek().frame {
                            let duration = if prefers_reduced_motion() { 0 } else { 400 };
                            frame_chart(flow, bounds, *sel_on.peek(), duration);
                        }
                    }
                    // One step out per press, deepest first: the quotation a
                    // row opened, then the selection itself.
                    "Escape" => {
                        if let Some(back) = quoted.peek().clone() {
                            nav.push(back);
                        } else if *anything.peek() {
                            nav.push(Route::FnOverview {});
                        }
                    }
                    // The fold, on the room in hand. Its own key, because it is
                    // its own gesture: nothing about the selection changes. A
                    // mark in hand folds the room it is written in — the box the
                    // reader can actually see it inside of.
                    "z" => {
                        let Some(at) = *chosen.peek() else { continue };
                        let at = match at {
                            Spot::Mark(id) => {
                                let model = model.peek();
                                let Some(&home) = model.home.get(&id) else {
                                    continue;
                                };
                                home
                            }
                            room => room,
                        };
                        let mut fold = fold;
                        fold.set(Some((at, false)));
                    }
                    // The whole declaration, read where the sheet's rows are
                    // read: on the quotation plate beside it.
                    "Enter" => {
                        let Some(Spot::Mark(at)) = *chosen.peek() else {
                            continue;
                        };
                        let model = model.peek();
                        if let Some(mark) = model.by_id().get(&at).copied() {
                            nav.push(crate::views::func::peek_route(
                                &(mark.head.path.clone(), mark.head.label.clone()),
                                &mark.head.path,
                                &mark.head.label,
                            ));
                        }
                    }
                    "ArrowLeft" => crate::views::shell::history_back(),
                    "ArrowRight" => crate::views::shell::history_forward(),
                    // Down into the heaviest callee, up to the heaviest caller.
                    // The household seats a callee wherever its own code is
                    // written, so this is a glide across the paper rather than a
                    // step into a shelf — and the glide only happens where the
                    // target is not already legible.
                    step => {
                        let Some(Spot::Mark(at)) = *chosen.peek() else {
                            continue;
                        };
                        let model = model.peek();
                        let next = match step {
                            "ArrowDown" => model.callees.get(&at),
                            _ => model.callers.get(&at),
                        }
                        .and_then(|list| list.first())
                        .copied();
                        if let Some(mark) = next.and_then(|id| model.by_id().get(&id).copied()) {
                            nav.push(mark_route(&mark.head.path, &mark.head.label));
                        }
                    }
                }
            }
        });
    });

    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);
    rsx! {
        div {
            class: "fn-chart absolute inset-0",
            class: if far() { "is-far" },
            Flow {
                nodes,
                edges,
                fit_view: false,
                handle: flow,
                nodes_draggable: false,
                delete_key: false,
                pan_on_scroll: true,
                on_pane_click: move |_| {
                    if *anything.peek() {
                        nav.push(Route::FnOverview {});
                    }
                },
                node_view: move |ctx: NodeViewCtx<FnNodeData>| {
                    // While the diff has anything to say, whatever it never
                    // touched rests at a lighter pressure.
                    let rest = kin.read().is_none()
                        && chart.read().dirty
                        && ctx.node.data.0.letter.is_none();
                    rsx! {
                        div { class: "fn-node", class: if rest { "is-rest" },
                            FnNode { ctx, kin: kin(), hot }
                        }
                    }
                },
                {
                    let (top, right, bottom, left) = chrome_insets(narrow_viewport(), panel);
                    rsx! {
                        FitInsets { top, right, bottom, left }
                    }
                }
                WorldLayer { class: "fn-ground",
                    FrameLayer { frames: chart.read().frames.clone(), kin: kin(), fold, fit }
                    OwnerLayer { owners: chart.read().owners.clone(), kin: kin(), fold, fit }
                }
                WorldLayer { class: "fn-wires",
                    WireLayer {
                        wires: chart.read().wires.clone(),
                        bundles: chart.read().bundles.clone(),
                        kin: kin(),
                        reading: *fns.wires.read(),
                        dirty: chart.read().dirty,
                        over: false,
                    }
                }
                WorldLayer { class: "fn-wires fn-wires-lit",
                    WireLayer {
                        wires: chart.read().wires.clone(),
                        bundles: chart.read().bundles.clone(),
                        kin: kin(),
                        reading: *fns.wires.read(),
                        dirty: chart.read().dirty,
                        over: true,
                    }
                }
                WorldLayer { class: "fn-wires fn-wires-hot",
                    HotWireLayer { wires: chart.read().wires.clone(), hot: settled }
                }
                WorldLayer { class: "fn-names",
                    NameLayer { names: chart.read().names.clone(), kin: kin() }
                }
                dioxus_flow::prelude::Controls {}
            }
        }
    }
}

#[component]
fn FitInsets(top: f64, right: f64, bottom: f64, left: f64) -> Element {
    dioxus_flow::use_overlay_inset(Side::Top, top);
    dioxus_flow::use_overlay_inset(Side::Right, right);
    dioxus_flow::use_overlay_inset(Side::Bottom, bottom);
    dioxus_flow::use_overlay_inset(Side::Left, left);
    rsx! {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::data::{Delta, ItemKind, Vis};
    use crate::views::func::model::{Call, FnHead, Frame};

    fn mark(id: u32, name: &str) -> FnMark {
        FnMark {
            id,
            tier: Tier::Entry,
            krate: "slope".to_string(),
            module: Vec::new(),
            head: FnHead {
                kind: ItemKind::Fn,
                vis: Vis::Pub,
                name: name.rsplit("::").next().unwrap_or(name).to_string(),
                label: name.to_string(),
                path: "src/main.rs".to_string(),
                line: id + 1,
                section: String::new(),
            },
            rows: Vec::new(),
            owner: None,
            delta: Delta::Same,
            callers: 0,
            calls: 0,
            touches: 0,
            runs: 0,
            recurses: false,
        }
    }

    /// One parameter row, as the source writes it.
    fn row(name: &str, ty: &str) -> SigRow {
        SigRow {
            name: name.to_string(),
            ty: ty.to_string(),
            returns: false,
            added: false,
        }
    }

    /// A head says what it is and what it is called — and, since the container
    /// says whose method it is, nothing about its owner.
    #[test]
    fn a_head_says_what_it_is_and_leaves_the_owner_to_the_container() {
        let method = MeasuredBlock::measure(&mark(0, "FnModel::build"));
        assert_eq!(method.decl, "pub fn");
        assert_eq!(method.name, "build");
        // The prefix is off the head, so a method's block is no wider than the
        // free declaration of the same name.
        let free = MeasuredBlock::measure(&mark(1, "build"));
        assert_eq!(method.own, free.own);
    }

    /// A block quotes its signature the way rust writes it: the name opens the
    /// parameter list, the parameters are its own rows, and the return closes
    /// it.
    #[test]
    fn a_block_quotes_its_signature_as_rust_writes_it() {
        let mut with = mark(0, "survey");
        with.rows = vec![
            row("dir", "&std::path::Path"),
            SigRow {
                name: String::new(),
                ty: "Result<CodeGraph, String>".to_string(),
                returns: true,
                added: false,
            },
        ];
        let block = MeasuredBlock::measure(&with);
        assert_eq!(block.open, "(");
        assert_eq!(block.rows.len(), 1, "the return is not a parameter row");
        assert_eq!(block.rows[0].written(), "dir: &std::path::Path");
        assert_eq!(block.tail, ") -> Result<CodeGraph, String>");

        // No parameters and a return: rust writes the brackets closed.
        let mut none = mark(1, "facts");
        none.rows = vec![SigRow {
            name: String::new(),
            ty: "FnFacts".to_string(),
            returns: true,
            added: false,
        }];
        let none = MeasuredBlock::measure(&none);
        assert_eq!(none.open, "()");
        assert_eq!(none.tail, "-> FnFacts");
        assert!(none.rows.is_empty());

        // Neither: the head is the whole quotation, and the box is one row.
        let bare = MeasuredBlock::measure(&mark(2, "main"));
        assert_eq!((bare.open.as_str(), bare.tail.as_str()), ("()", ""));
        assert_eq!(bare.own.1, BORDER + HEAD_H);
        assert!(block.own.1 > bare.own.1, "quoted rows take room");
    }

    /// Every line a block draws, and the width the browser will lay it out at.
    fn drawn_lines(view: &MeasuredBlock) -> Vec<(String, f64)> {
        let mut lines = Vec::new();
        let mut head = String::new();
        let mut head_w = text_w(&view.decl, KW_PX) + RUN_GAP;
        head.push_str(&view.decl);
        head.push_str(&view.name);
        head.push_str(&view.open);
        head_w += text_w(&view.name, NAME_PX) + text_w(&view.open, NAME_PX);
        if let Some(letter) = view.letter {
            head.push_str(letter);
            head_w += RUN_GAP + text_w(letter, 9.5);
        }
        lines.push((head, head_w));
        for row in &view.rows {
            lines.push((row.written(), text_w(&row.written(), ROW_PX) + ROW_INDENT));
        }
        if !view.tail.is_empty() {
            lines.push((view.tail.clone(), text_w(&view.tail, ROW_PX)));
        }
        lines
    }

    /// The measure is never smaller than what the browser draws.
    #[test]
    fn every_drawn_line_is_measured_at_least_as_wide_as_it_renders() {
        type Shape = (
            &'static str,
            &'static [(&'static str, &'static str)],
            &'static str,
        );
        let shapes: Vec<Shape> = vec![
            (
                "FnModel::build",
                &[("graph", "&CodeGraph"), ("reading", "&FnReading")],
                "Option<(u32, &std::collections::HashSet<Anchor>)>",
            ),
            (
                "DataModel::quote",
                &[
                    ("method_row", "impl Fn(&ItemMark) -> Option<&'g MethodRow>"),
                    ("flow", "dioxus_flow::prelude::FlowHandle<DataNodeData>"),
                ],
                "",
            ),
            ("main", &[], ""),
            ("survey", &[("dir", "&Path")], "Result<CodeGraph, String>"),
            ("Tier::band", &[("self", ""), ("deepest", "u32")], "u32"),
        ];
        for (name, rows, ret) in shapes {
            let mut m = mark(0, name);
            m.rows = rows.iter().map(|(n, ty)| row(n, ty)).collect();
            if !ret.is_empty() {
                m.rows.push(SigRow {
                    name: String::new(),
                    ty: ret.to_string(),
                    returns: true,
                    added: false,
                });
            }
            let view = MeasuredBlock::measure(&m);
            let room = view.own.0 - PAD_X;
            for (text, drawn) in drawn_lines(&view) {
                assert!(
                    drawn <= room || view.own.0 == MARK_MAX_W,
                    "{name}: `{text}` draws at {drawn} in a box of {} \
                     — the measure is short of the paper",
                    view.own.0
                );
            }
        }
    }

    /// A head row never clips its own identity.
    #[test]
    fn a_head_run_is_never_the_line_the_clamp_cuts() {
        for name in [
            "main",
            "FnModel::build",
            "MeasuredBlock::measure",
            "DataCartouche::visibility_floor",
        ] {
            let view = MeasuredBlock::measure(&mark(0, name));
            let (head, drawn) = drawn_lines(&view).remove(0);
            assert!(
                drawn <= view.own.0 - PAD_X,
                "`{head}` draws at {drawn} in a box of {}",
                view.own.0
            );
        }
    }

    /// The kind colour an owner's name takes: the data chart's own two, and ink
    /// for a trait, whose purple would read as a function's name on this chart.
    #[test]
    fn an_owner_name_takes_the_kind_colour_its_block_takes_one_rung_up() {
        assert_eq!(owner_tint(ItemKind::Struct), "is-type");
        assert_eq!(owner_tint(ItemKind::Union), "is-type");
        assert_eq!(owner_tint(ItemKind::Enum), "is-sum");
        assert_eq!(owner_tint(ItemKind::Trait), "");
    }

    /// The far edition's landmark register is measured to its box, never
    /// guessed: the name it engraves fits the box it names, or the box gets no
    /// name at all.
    #[test]
    fn a_landmark_fits_the_box_it_names_or_is_not_drawn() {
        let boxes = [
            (2400.0, 900.0),
            (900.0, 400.0),
            (520.0, 180.0),
            (160.0, 18.0),
        ];
        for name in ["mod func", "survey", "FnModel", "an_unusually_long_name"] {
            for (w, h) in boxes {
                let at = Placed {
                    x: 10.0,
                    y: 20.0,
                    w,
                    h,
                };
                let Some(mark) = NameView::measure(Spot::Frame(7), name, at, LANDMARK_MAX) else {
                    continue;
                };
                assert!(
                    (LANDMARK_MIN..=LANDMARK_MAX).contains(&mark.size),
                    "{name} in {w}x{h}: {} is off the far ramp",
                    mark.size
                );
                let drawn = text_w(name, mark.size);
                assert!(
                    mark.x + drawn <= at.x + at.w,
                    "{name} in {w}x{h}: the name runs {drawn} past the box"
                );
                assert!(mark.y <= at.y + at.h);
                assert!(mark.y > at.y + HEAD_H, "a landmark stands under the label");
            }
        }
        assert!(
            NameView::measure(
                Spot::Frame(0),
                "build",
                Placed {
                    x: 0.0,
                    y: 0.0,
                    w: MARK_MAX_W,
                    h: HEAD_H + 2.0,
                },
                LANDMARK_MAX,
            )
            .is_none(),
            "a block-sized box cannot carry a far name and must not get one"
        );
    }

    /// A household with one crate, one module, one container of two methods and
    /// one free declaration, and four calls between them — two inside the module
    /// and two crossing to a second module.
    fn house() -> FnModel {
        let call = |def: u32, user: u32, count: u32| Call {
            def,
            user,
            kind: CallKind::Call,
            count,
        };
        let mut root = Frame {
            id: 0,
            krate: "slope".to_string(),
            module: Vec::new(),
            parent: None,
            marks: Vec::new(),
            owners: Vec::new(),
            kids: vec![1, 2],
            folded: false,
            held: 4,
        };
        root.kids.sort_unstable();
        let mut views = Frame {
            id: 1,
            krate: "slope".to_string(),
            module: vec!["views".to_string()],
            parent: Some(0),
            marks: vec![0],
            owners: vec![0],
            kids: Vec::new(),
            folded: false,
            held: 3,
        };
        views.marks.sort_unstable();
        let graph = Frame {
            id: 2,
            krate: "slope".to_string(),
            module: vec!["graph".to_string()],
            parent: Some(0),
            marks: vec![3],
            owners: Vec::new(),
            kids: Vec::new(),
            folded: false,
            held: 1,
        };
        FnModel {
            marks: (0..4).map(|id| mark(id, &format!("fn{id}"))).collect(),
            calls: vec![
                // inside `views`
                call(1, 0, 2),
                call(2, 1, 1),
                // crossing into `graph`
                call(3, 0, 5),
                call(3, 1, 4),
            ],
            frames: vec![root, views, graph],
            owners: vec![Container {
                id: 0,
                frame: 1,
                ty: 90,
                decl: "impl",
                name: "Plate".to_string(),
                kind: ItemKind::Struct,
                vis: Vis::Pub,
                path: "src/main.rs".to_string(),
                label: "Plate".to_string(),
                on_data: true,
                marks: vec![1, 2],
                folded: false,
            }],
            home: HashMap::from([
                (0, Spot::Frame(1)),
                (1, Spot::Owner(0)),
                (2, Spot::Owner(0)),
                (3, Spot::Frame(2)),
            ]),
            multi_crate: false,
            ..Default::default()
        }
    }

    /// **The corridors are the resting reading of the cross-module family.**
    /// One line per ordered module pair, with the count it carries riding it —
    /// and the counts add up to exactly the cross-module calls the survey
    /// resolved, so nothing is quietly dropped into a bundle or counted twice.
    #[test]
    fn a_corridor_stands_for_every_cross_module_call_between_two_modules() {
        let drawing = FnDrawing::build(&house());
        assert_eq!(drawing.wires.len(), 4, "one line per call");
        // Two calls cross from `graph` into `views`, so one corridor stands for
        // both, carrying 5 + 4 references.
        assert_eq!(drawing.bundles.len(), 1);
        let corridor = &drawing.bundles[0];
        assert_eq!(corridor.count, 9);
        assert_eq!(corridor.label.as_deref(), Some("9"));
        assert_eq!(
            (corridor.def, corridor.user),
            (Spot::Frame(2), Spot::Frame(1))
        );
        assert!(
            corridor
                .title
                .as_deref()
                .is_some_and(|w| w.contains("9 calls"))
        );
        // And the corridor is the way to the module it names.
        assert_eq!(
            corridor.to_mod.as_deref(),
            Some(&["slope".to_string(), "graph".to_string()][..])
        );
        // The count is the sum of the cross-module lines and no more.
        let crossing: u32 = drawing
            .wires
            .iter()
            .filter(|w| w.cross && w.class == "is-call")
            .map(|w| w.count)
            .sum();
        assert_eq!(corridor.count, crossing);
    }

    /// The resting plate draws the corridors and the calls inside one module,
    /// and nothing else. An anchor — a selection, or the diff — unbundles.
    #[test]
    fn the_resting_plate_bundles_and_an_anchor_unbundles() {
        let drawing = FnDrawing::build(&house());
        let seen = |picked: bool, dirty: bool| -> (usize, usize) {
            let lines = drawing
                .wires
                .iter()
                .filter(|w| drawn_under(FnWires::Both, w, picked, dirty, false))
                .count();
            let corridors = drawing
                .bundles
                .iter()
                .filter(|w| drawn_under(FnWires::Both, w, picked, dirty, false))
                .count();
            (lines, corridors)
        };
        // At rest: the two calls inside `views`, and one corridor for the two
        // that cross.
        assert_eq!(seen(false, false), (2, 1));
        // With a diff the corridors keep standing — they are the shape — and
        // the individual lines thin to the diff's own. Nothing here is touched,
        // so no line stands.
        assert_eq!(seen(false, true), (0, 1));
        // With a selection the sheet answers about the selection alone, and the
        // corridors stand down: a bundle answers about a whole module.
        assert_eq!(seen(true, true), (0, 0));
    }

    /// A mark selection reads its own wires **in the chosen direction**: the
    /// same mark, three different sets.
    #[test]
    fn a_selection_reads_its_wires_in_the_chosen_direction() {
        let model = house();
        let of = |id: u32, reading: FnWires| FnKin::mark(id, &model, reading).wires;
        // `fn1` runs `fn2` and `fn3`, and `fn0` runs `fn1`. Read from `fn1`:
        // `calls` keeps what `fn1` runs, `callers` what runs `fn1`.
        assert_eq!(
            of(1, FnWires::Calls),
            HashSet::from([
                (Spot::Mark(2), Spot::Mark(1)),
                (Spot::Mark(3), Spot::Mark(1))
            ])
        );
        assert_eq!(
            of(1, FnWires::Callers),
            HashSet::from([(Spot::Mark(1), Spot::Mark(0))])
        );
        assert_eq!(
            of(1, FnWires::Both),
            HashSet::from([
                (Spot::Mark(2), Spot::Mark(1)),
                (Spot::Mark(3), Spot::Mark(1)),
                (Spot::Mark(1), Spot::Mark(0)),
            ]),
            "both ways round is the union, never a fourth answer"
        );
    }

    /// A container selection is the box read as what it is: everything written
    /// on that owner keeps full ink, everything one call across its border reads
    /// a step behind, and every crossing wire is the reading's own ink.
    #[test]
    fn an_owner_selection_reads_its_methods_and_its_crossings() {
        let model = house();
        let kin = FnKin::boundary(Spot::Owner(0), &model, FnWires::Both);
        assert_eq!(kin.sel, Some(Spot::Owner(0)));
        assert!(kin.lit.contains(&Spot::Mark(1)));
        assert!(kin.lit.contains(&Spot::Mark(2)));
        assert_eq!(kin.class(Spot::Mark(1)), "is-kin");
        // `fn0` and `fn3` are outside it and each calls into it.
        assert_eq!(kin.class(Spot::Mark(0)), "is-near");
        assert_eq!(kin.class(Spot::Mark(3)), "is-near");
        assert_eq!(
            kin.wires,
            HashSet::from([
                // `fn0`, on the module's own shelf, runs `fn1` inside it…
                (Spot::Mark(1), Spot::Mark(0)),
                // …and `fn1` runs `fn3`, which is written in the other module.
                (Spot::Mark(3), Spot::Mark(1)),
            ])
        );
        // Read in one direction only, the boundary keeps one side of it.
        let out = FnKin::boundary(Spot::Owner(0), &model, FnWires::Calls);
        assert!(out.wires.len() < kin.wires.len());
    }

    /// A module boundary reads the same way, one room out: everything written
    /// inside keeps full ink, the containers inside it included.
    #[test]
    fn a_module_boundary_lights_everything_written_inside_it() {
        let model = house();
        let kin = FnKin::boundary(Spot::Frame(1), &model, FnWires::Both);
        assert_eq!(kin.frame, Some(1));
        assert_eq!(kin.sel, None, "a module is not a mark's own focus");
        for inside in [Spot::Mark(0), Spot::Mark(1), Spot::Mark(2), Spot::Owner(0)] {
            assert!(kin.lit.contains(&inside), "{inside:?} is inside the module");
        }
        assert_eq!(
            kin.class(Spot::Mark(3)),
            "is-near",
            "one call across the line"
        );
        assert_eq!(kin.class(Spot::Frame(1)), "is-picked");
    }

    /// **Folding draws less; it does not move anything.** The end-to-end
    /// invariant, read off the drawing rather than the layout.
    #[test]
    fn a_fold_draws_less_and_moves_nothing() {
        let open = FnDrawing::build(&house());
        let mut shut = house();
        shut.folded = HashSet::from([Spot::Owner(0)]);
        shut.owners[0].folded = true;
        shut.packs = HashMap::from([
            (Spot::Mark(1), Spot::Owner(0)),
            (Spot::Mark(2), Spot::Owner(0)),
        ]);
        let folded = FnDrawing::build(&shut);

        let boxes = |d: &FnDrawing| -> Vec<(String, i64, i64, i64, i64)> {
            let mut out: Vec<(String, i64, i64, i64, i64)> = d
                .rects
                .iter()
                .map(|(spot, at)| {
                    (
                        format!("{spot:?}"),
                        at.x as i64,
                        at.y as i64,
                        at.w as i64,
                        at.h as i64,
                    )
                })
                .collect();
            out.sort();
            out
        };
        let common: Vec<(String, i64, i64, i64, i64)> = boxes(&open)
            .into_iter()
            .filter(|b| folded.rects.keys().any(|s| format!("{s:?}") == b.0))
            .collect();
        assert_eq!(boxes(&folded), common, "the fold re-laid the paper");

        // What the fold hides is off the *drawing*, and only that.
        let drawn = |d: &FnDrawing| -> Vec<u32> {
            let mut ids: Vec<u32> = d.nodes.iter().map(|n| n.data.0.id).collect();
            ids.sort_unstable();
            ids
        };
        assert_eq!(drawn(&open), vec![0, 1, 2, 3]);
        assert_eq!(
            drawn(&folded),
            vec![0, 3],
            "the methods are elided, not moved"
        );
        // And the container that folded still owns the whole box it reserved.
        assert_eq!(
            folded.rects.get(&Spot::Owner(0)),
            open.rects.get(&Spot::Owner(0))
        );

        // Opening it again is the drawing that was there before.
        shut.folded.clear();
        shut.owners[0].folded = false;
        shut.packs.clear();
        let back = FnDrawing::build(&shut);
        assert_eq!(boxes(&back), boxes(&open));
        assert_eq!(drawn(&back), drawn(&open));
    }

    /// A fold re-anchors ink instead of cutting it: a wire whose far end a fold
    /// hides is read on the border that stands for it.
    #[test]
    fn a_fold_re_anchors_the_ink_it_swallows() {
        let mut model = house();
        model.folded = HashSet::from([Spot::Owner(0)]);
        model.owners[0].folded = true;
        model.packs = HashMap::from([
            (Spot::Mark(1), Spot::Owner(0)),
            (Spot::Mark(2), Spot::Owner(0)),
        ]);
        assert_eq!(model.shown(Spot::Mark(1)), Spot::Owner(0));
        assert!(model.hidden(Spot::Mark(2)));

        // `fn0` calls `fn1`, which is now inside the folded container, so the
        // line ties to the container's own border.
        let kin = FnKin::mark(0, &model, FnWires::Calls);
        assert!(kin.wires.contains(&(Spot::Owner(0), Spot::Mark(0))));
        // The call between two methods inside the fold is a line from a border
        // to itself, and there is nothing left to draw.
        let drawing = FnDrawing::build(&model);
        assert!(
            drawing.wires.iter().all(|w| w.def != w.user),
            "a border names nothing to itself"
        );
    }

    /// Every stop of the `wires` reading draws a different set **around an
    /// anchor**, and the anchor is whatever is in focus.
    #[test]
    fn every_stop_of_the_wires_reading_changes_the_picture() {
        let wire = |def_dirty: bool, user_dirty: bool, contract: bool| WireView {
            key: "w".to_string(),
            from: Point::new(0.0, 0.0),
            to: Point::new(1.0, 1.0),
            def: Spot::Mark(0),
            user: Spot::Mark(1),
            count: 1,
            label: None,
            title: None,
            to_mod: None,
            width: 1.0,
            def_dirty,
            user_dirty,
            cross: false,
            bundle: false,
            from_side: TieSide::Right,
            to_side: TieSide::Left,
            class: match contract {
                true => "is-answers",
                false => "is-call",
            },
        };
        let seen = |w: &WireView, picked: bool, dirty: bool, lit: bool| {
            FnWires::ALL_STOPS
                .iter()
                .map(|&stop| drawn_under(stop, w, picked, dirty, lit))
                .collect::<Vec<bool>>()
        };
        let out = wire(false, true, false);
        assert_eq!(seen(&out, false, true, false), vec![true, false, true]);
        let into = wire(true, false, false);
        assert_eq!(seen(&into, false, true, false), vec![false, true, true]);
        // A wire the diff touched at neither end is no part of the anchor, so
        // no direction keeps it — `both` included. `both` used to draw the whole
        // family here, which is an *amount* of ink and not a direction, and it
        // is the very thing the stops were renamed away from on 2026-08-27; with
        // the corridors standing, it was also the hairball they exist to
        // prevent.
        let stranger = wire(false, false, false);
        assert_eq!(
            seen(&stranger, false, true, false),
            vec![false, false, false]
        );
        // A clean sheet has no anchor at all: a call inside one module rests as
        // its own line whichever stop is on…
        assert_eq!(seen(&stranger, false, false, false), vec![true, true, true]);
        // …and one that crosses a module is said by its corridor instead.
        let mut crossing = wire(false, false, false);
        crossing.cross = true;
        assert_eq!(
            seen(&crossing, false, false, false),
            vec![false, false, false]
        );
        // With a selection the sheet answers about the selection.
        assert_eq!(seen(&stranger, true, true, true), vec![true, true, true]);
        assert_eq!(seen(&out, true, true, false), vec![false, false, false]);
        // A contract never folds and never bundles, in any direction.
        let contract = wire(false, false, true);
        assert_eq!(seen(&contract, true, true, false), vec![true, true, true]);
        assert_eq!(seen(&contract, false, true, false), vec![true, true, true]);
    }

    /// Every block is wide enough for the longest line it draws, and none is
    /// narrower than a mark a pointer can find or wider than a quotation reads.
    #[test]
    fn a_block_is_measured_to_hold_its_longest_line() {
        let short = MeasuredBlock::measure(&mark(0, "at"));
        assert_eq!(short.own.0, MARK_MIN_W);
        let long = MeasuredBlock::measure(&mark(1, "a_declaration_with_a_very_long_name_indeed"));
        assert!(long.own.0 > short.own.0);
        let mut wide = mark(2, "read");
        wide.rows = vec![row(
            "reading",
            "&HashMap<u32, Vec<crate::views::func::model::Touch>>",
        )];
        let wide = MeasuredBlock::measure(&wide);
        assert!(wide.own.0 > short.own.0, "the row sets the width");
        assert!(wide.own.0 <= MARK_MAX_W);
    }
}
