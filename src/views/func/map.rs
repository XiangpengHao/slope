//! The function chart: the **shelved section**.
//!
//! The direction contract, and it is the whole of this file's reason: at this
//! altitude **containment is the call**. The data chart never draws plain
//! ownership as a line — a held type nests inside its holder — and one rung
//! down the commonest edge is the way-in call, so every declaration seats
//! *inside* the frame of the caller that reaches it first, and what it calls
//! shelves in wrapped rows under its own head. Ink is spent only on what the
//! shelving cannot say. Approved 2026-08-26, after two prototypes were read on
//! this workspace and rejected; the band × prism section it replaces is
//! recorded in `spec/function-viewer.md`.
//!
//! One block per declaration that runs, and one **head row** per block: the
//! keyword run, the name, the diff's letter, and the module it is written in
//! where that is not its caller's. The signature is quoted on the sheet, which
//! has the room for it. An **entry point** wears the root's 2.5px ink left
//! edge: this is where a chain of running begins.
//!
//! Two families run between the blocks, and only two. Solid is a **call**:
//! every resolved call that is not a way in, because the way in is already
//! drawn as containment. Dashed and lighter is a **contract**: a trait's own
//! clause and the methods that answer it, which is what keeps the chart honest
//! about a `dyn` call it cannot follow, and which never folds. Both rest on the
//! dependent — the caller, the answering method — the way change travels.
//!
//! The resting reading is each mark's heaviest crossing call in each direction,
//! the same two-per-mark rule the data chart rests its references at. Hover of
//! either end inks all of that mark's wires; a selection's ink and stay.
//!
//! Three gestures act on a frame and they are three marks, never one: the head
//! row **selects the declaration**, the border ring **selects the boundary** —
//! the box is a subtree, because containment is the call — and the `–` / `+` at
//! the end of the head row **folds what shelves inside**. A folded frame keeps
//! its head and its whole quoted signature, states `+ n inside` where its shelf
//! stood, and every wire whose far end it swallowed re-anchors to it.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size, Viewport,
};

use crate::Route;
use crate::graph::data::{CodeGraph, ItemKind};
use crate::views::chrome::{narrow_viewport, prefers_reduced_motion, use_settled, window_size};
use crate::views::func::layout::{self, FnLayout, HEAD_H, Placed, RingStrip, Sizes, TieSide};
use crate::views::func::model::{CallKind, FnMark, FnModel, SigRow, Tier};
use crate::views::func::{
    FnOrder, FnSel, FnWires, band_route, fold_key, mark_route, tree_route, use_fns,
};

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
const META_PX: f64 = 8.0;
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
/// The counted words a folded frame writes where its shelf stood.
const PACKED_H: f64 = 13.0;
/// The fold mark's own room at the end of a head row.
const FOLD_W: f64 = 13.0;
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
/// The far edition's **landmark register**: the biggest a ground frame's
/// engraved name may be drawn, the smallest that is worth drawing at all, and
/// how much of a frame's width one name may spend. Below the floor the frame
/// gets no engraved name — the far edition keeps its reading-size head instead,
/// which is what the Two-Ramps rule asks for when a box cannot carry a larger
/// name. These are the far ramp's own sizes; nothing here is a reading size.
const LANDMARK_MAX: f64 = 54.0;
const LANDMARK_MIN: f64 = 26.0;
const LANDMARK_OF_WIDTH: f64 = 0.055;
/// How much smaller a landmark is than the one it stands inside, and how much
/// clear paper it keeps below it. The ladder of sizes is the nesting, and the
/// gap is what stops ten frames naming themselves over one square of paper.
const LANDMARK_STEP: f64 = 6.0;
const LANDMARK_GAP: f64 = 10.0;

fn text_w(text: &str, px: f64) -> f64 {
    text.chars().count() as f64 * px * MONO_ADVANCE
}

/// The engraved width of one wire: heavier the more references the survey
/// resolved for the pair, as everywhere in this system. A contract is one
/// promise, so it draws at the hairline. Weight is the whole of what a resting
/// wire says about itself — it has no colour, and under the blocks it has no
/// crossing either.
fn wire_width(answers: bool, count: u32) -> f64 {
    match answers {
        true => 1.0,
        false => (1.0 + (count.max(1) as f64).ln() * 0.32).min(2.4),
    }
}

/// One block, measured. Everything the plate draws about one mark, and nothing
/// about where it sits.
#[derive(Clone, PartialEq)]
struct MeasuredBlock {
    id: u32,
    /// `pub fn`, `fn`, `macro` — what rust writes in front of the name.
    decl: String,
    /// The type or trait whose impl this method is written in, as the survey's
    /// own label writes it, with rust's `::` after it: `FnModel::`. `None` on a
    /// free declaration, which is written under no type at all.
    owner: Option<String>,
    name: String,
    /// The bracket the head opens with: `(` where parameters follow, `()`
    /// where none do, nothing at all for a macro.
    open: String,
    /// The signature's parameter rows, quoted as the source writes them.
    rows: Vec<SigRow>,
    /// The line that closes the quotation: `) -> Result<…>`, `)`, `-> u32`, or
    /// nothing where the declaration says neither.
    tail: String,
    /// The module it is written in, where the caller it seats inside is written
    /// in another one. A same-module call is quiet; crossing a module is the
    /// fact the head spends a word on.
    module: Option<String>,
    letter: Option<&'static str>,
    entry: bool,
    ring: bool,
    /// Anything shelves inside this frame: it has a boundary to select and a
    /// fold mark to close. A leaf has neither.
    shelves: bool,
    /// The reviewer folded it. What shelves inside is off the paper, and the
    /// counted words below say how much.
    folded: bool,
    /// What a folded frame states in words where its shelf stood.
    packed: String,
    /// The label a URL selects this block by, and the file it is written in.
    path: String,
    label: String,
    title: String,
    /// The words the frame's own boundary answers with.
    edge_title: String,
    /// The block's own box: its head row, the quoted signature, and — folded —
    /// the counted words.
    own: (f64, f64),
    /// The same box **as though the frame were not folded**: no counted words,
    /// no rule under them. This is what the layout seats a frame by, so that
    /// folding one moves nothing — the shelf under its head keeps the room it
    /// always had, and the counted words are drawn inside that room rather than
    /// added to it. Only a fold the packer was allowed to skip is seated by
    /// `own`, because that is the one that really is just its own box.
    own_open: (f64, f64),
}

impl MeasuredBlock {
    fn measure(mark: &FnMark) -> Self {
        let decl = mark.head.decl();
        let macro_rules = mark.head.kind == ItemKind::Macro;
        let owner = match mark.qualifier() {
            "" => None,
            ty => Some(format!("{ty}::")),
        };
        let module = mark.crosses.then(|| mark.written());
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
        let shelves = mark.runs > 0;
        let packed = match mark.folded {
            true => format!("+ {} inside", mark.runs),
            false => String::new(),
        };

        // The head row, run by run — the whole of it, because a head that
        // clips its own name says nothing.
        let mut head_w = text_w(&decl, KW_PX) * META_SLACK + RUN_GAP;
        if let Some(owner) = owner.as_deref() {
            head_w += text_w(owner, NAME_PX);
        }
        head_w += text_w(&mark.head.name, NAME_PX) + text_w(&open, NAME_PX);
        if letter.is_some() {
            head_w += RUN_GAP + text_w("M", NAME_PX);
        }
        if let Some(module) = module.as_deref() {
            head_w += RUN_GAP + text_w(module, META_PX) * META_SLACK;
        }
        if shelves {
            head_w += RUN_GAP + FOLD_W;
        }
        // And the longest quoted line under it. A line wider than the block may
        // be ellipsizes, and its hover words carry the whole of it.
        let widest = rows
            .iter()
            .map(|row| text_w(&row.written(), ROW_PX) + ROW_INDENT)
            .chain(std::iter::once(text_w(&tail, ROW_PX)))
            .chain(std::iter::once(text_w(&packed, 9.0) * META_SLACK))
            .fold(head_w, f64::max);
        // The clamp governs the **quotation**: a type is a long thing, so a row
        // wider than the widest a quotation reads at ellipsizes and its hover
        // words carry the whole of it. The head is not a quotation — it is which
        // declaration this is — so it sets the floor under the box instead of
        // being cut inside it. A head that clips its own name says nothing, and
        // an owner prefix long enough to push `visibility_floor` off its own
        // block would say the wrong name rather than a shorter one.
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
        let open_h = BORDER + HEAD_H + body_h;
        let fold_h = match mark.folded {
            true => layout::RULE + PACKED_H,
            false => 0.0,
        };

        Self {
            id: mark.id,
            decl,
            owner,
            name: mark.head.name.clone(),
            open,
            rows,
            tail,
            module,
            letter,
            entry: mark.tier == Tier::Entry,
            ring: mark.tier == Tier::Ring,
            shelves,
            folded: mark.folded,
            packed,
            path: mark.head.path.clone(),
            label: mark.head.label.clone(),
            title: mark.title(),
            // The boundary teaches in place: at rest the stronger border says
            // *a control*, and the hover words say which control and what it
            // takes. A count of what runs inside it is a fact about the box, so
            // it stays.
            edge_title: match mark.runs {
                1 => format!(
                    "everything {} calls, down the chain — 1 declaration · click to select the whole box",
                    mark.head.name
                ),
                n => format!(
                    "everything {} calls, down the chain — {n} declarations · click to select the whole box",
                    mark.head.name
                ),
            },
            own: (w, open_h + fold_h),
            own_open: (w, open_h),
        }
    }
}

/// One node on the function chart. Every node is a block: a folded frame writes
/// its counted words inside its own box, so no fold is ever a node of its own.
#[derive(Clone, PartialEq)]
struct FnNodeData(Box<MeasuredBlock>);

/// One drawn wire with its ends already found.
#[derive(Clone, PartialEq)]
struct WireView {
    key: String,
    from: Point,
    to: Point,
    /// The end being leaned on, and the end that leans — as the paper draws
    /// them, so an end a fold hides is the folded frame that stands for it.
    def: u32,
    user: u32,
    /// What the survey resolved across the pair, summed over every call a fold
    /// gathered onto this one line.
    count: u32,
    label: Option<String>,
    width: f64,
    /// Whether the diff touched each end. The resting plate's own anchor: with
    /// nothing selected, `calls` draws the wires whose *caller* the diff
    /// touched and `callers` the wires whose *callee* it touched, which is the
    /// blast-radius question read in each direction. Carried on the wire rather
    /// than looked up per render, because the `wires` reading may never re-read
    /// the survey — it inks lines, and nothing else.
    def_dirty: bool,
    user_dirty: bool,
    /// Which edge of each end's head row the wire ties to — the edge facing the
    /// other end. Carried on the wire because the fan pass needs it after every
    /// end has been found, and because it is what the drawn point *means*.
    from_side: TieSide,
    to_side: TieSide,
    class: &'static str,
}

impl WireView {
    /// Whether the **diff anchor** reads this wire in the given direction:
    /// `calls` asks whether the diff touched the end that leans (so the wire is
    /// something a changed declaration runs), `callers` whether it touched the
    /// end being leaned on (so the wire is somebody running a changed thing),
    /// `both` either. A wire the anchor reads carries the resting pressure; one
    /// it merely admits is drawn a step lighter.
    fn anchored(&self, reading: FnWires) -> bool {
        match reading {
            FnWires::Calls => self.user_dirty,
            FnWires::Callers => self.def_dirty,
            FnWires::Both => self.def_dirty || self.user_dirty,
        }
    }
}

/// One frame's boundary: the ring a click selects the whole subtree by.
#[derive(Clone, PartialEq)]
struct EdgeView {
    id: u32,
    at: Placed,
    title: String,
    path: String,
    label: String,
}

/// One landmark: a frame's own name, engraved across the frame at the size that
/// frame can carry, for the far edition alone.
#[derive(Clone, PartialEq)]
struct NameView {
    id: u32,
    name: String,
    x: f64,
    y: f64,
    size: f64,
}

impl NameView {
    /// The engraved name for one frame, or `None` where the box cannot carry
    /// one.
    ///
    /// **Measured to the box, never guessed.** The size is the smallest of four
    /// limits: a share of the frame's width, the width the identifier itself
    /// needs at that size, the room under the frame's own head row, and the
    /// `ceiling` the register above it leaves — the name inside a named frame is
    /// always a step smaller than the name of the frame it stands in, so the
    /// ladder of sizes *is* the nesting. An identifier is never broken across
    /// two lines and never clipped: where even the floor does not fit, the frame
    /// keeps its reading-size head and this register says nothing about it.
    fn measure(id: u32, name: &str, at: Placed, ceiling: f64) -> Option<Self> {
        let chars = name.chars().count().max(1) as f64;
        let by_width = (at.w - PAD_X) / (chars * MONO_ADVANCE);
        let by_height = at.h - HEAD_H - layout::RULE - 4.0;
        let size = (at.w * LANDMARK_OF_WIDTH)
            .min(by_width)
            .min(by_height)
            .min(ceiling)
            .floor();
        (size >= LANDMARK_MIN).then(|| Self {
            id,
            name: name.to_string(),
            // Left-aligned with the shelf inside the frame and standing just
            // under the head row it grows out of, so the landmark is where the
            // head's own name was — one place for one name, at two zooms.
            x: at.x + layout::PAD,
            y: at.y + HEAD_H + layout::RULE + size * 0.78,
            size,
        })
    }

    /// The lowest ink this landmark puts on the paper, descenders included. What
    /// a frame nested under it has to clear.
    fn foot(&self) -> f64 {
        self.y + self.size * 0.22
    }
}

/// One drawing of the chart: the blocks, ground and wires one build puts on the
/// paper, plus the indexes a reading walks.
#[derive(Clone, PartialEq)]
struct FnDrawing {
    nodes: Vec<FlowNode<FnNodeData>>,
    ring: Option<RingStrip>,
    /// One selectable boundary per frame that shelves anything.
    edges: Vec<EdgeView>,
    /// The far edition's landmark register: one engraved name per ground frame
    /// whose box can carry one.
    names: Vec<NameView>,
    wires: Vec<WireView>,
    /// Which band every drawn mark sits in, for a band reading.
    bands: HashMap<u32, u32>,
    /// Every drawn mark's box — what the camera glides to, and what a
    /// double-click fits.
    rects: HashMap<u32, Placed>,
    /// The URL's (path, item) key for every drawn mark.
    locate: HashMap<(String, String), u32>,
    frame: Option<Rect>,
    /// The bounds of the frames on the ground alone — where the running starts,
    /// and so where a reader opening this chart is put down.
    entry: Option<Rect>,
    dirty: bool,
}

impl FnDrawing {
    fn build(model: &FnModel) -> Self {
        // **Every** mark is measured, whether or not a fold has it off the
        // paper, because the layout has to reserve what a fold hides: that
        // reserved footprint is what keeps the rest of the sheet still when the
        // reader closes a frame. A frame is seated by its *open* box — the head
        // and its quotation, without the counted words a fold writes — so the
        // shelf under it keeps exactly the room it had, and the counted words
        // are drawn inside that room rather than added to it. The one exception
        // is a fold the packer was allowed to skip: that frame really is just
        // its own box, counted words and all.
        let mut sizes = Sizes::default();
        let mut views: Vec<MeasuredBlock> = Vec::with_capacity(model.marks.len());
        for mark in &model.marks {
            let view = MeasuredBlock::measure(mark);
            sizes.own.insert(
                mark.id,
                match model.packed.contains(&mark.id) {
                    true => view.own,
                    false => view.own_open,
                },
            );
            // What a fold hides is still measured and still placed. It is not
            // *drawn*: the frame that folded stands for all of it.
            if !model.hidden(mark.id) {
                views.push(view);
            }
        }

        let placed = FnLayout::build(model, &sizes);

        // How deep in the seating each block is, which is the order it paints
        // in: a frame is drawn under what shelves inside it.
        let mut depth: HashMap<u32, u32> = HashMap::new();
        for view in &views {
            let mut at = view.id;
            let mut deep = 0;
            while let Some(&up) = model.via.get(&at) {
                deep += 1;
                at = up;
                if deep > 512 {
                    break;
                }
            }
            depth.insert(view.id, deep);
        }
        views.sort_by_key(|v| (depth.get(&v.id).copied().unwrap_or(0), v.id));

        let mut nodes: Vec<FlowNode<FnNodeData>> = Vec::with_capacity(views.len());
        let mut rects: HashMap<u32, Placed> = HashMap::new();
        let mut locate: HashMap<(String, String), u32> = HashMap::new();
        for view in &views {
            let Some(at) = placed.marks.get(&view.id).copied() else {
                continue;
            };
            rects.insert(view.id, at);
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
                // Containment is the call, so a block stands over the frame it
                // seats in. The tiles the flow paints in are stacking contexts
                // of their own, so the paint order is stated twice: deepest
                // last in the list, and the depth on the node itself.
                .style(format!(
                    "z-index:{};",
                    depth.get(&view.id).copied().unwrap_or(0)
                ))
                .draggable(false)
                .selectable(false),
            );
        }

        // The wires, with every end read through the folds. A wire whose far
        // end a fold hides is **re-anchored to the folded frame's head** — the
        // frame stands for what it hides — rather than cut: an elided line
        // would take a chain off the paper without saying so, and this is the
        // same answer the data chart gives one rung up, where a folded module's
        // edges land on its counted row. Two calls that gather onto one line
        // gather their counts with them, so the line still says what it
        // carries.
        // Which marks the diff touched, so a wire carries the fact at both of
        // its ends and the resting reading can take a direction against it
        // without re-reading the survey.
        let touched: HashSet<u32> = model
            .marks
            .iter()
            .filter(|m| m.letter().is_some())
            .map(|m| m.id)
            .collect();
        let mut wires: Vec<WireView> = Vec::with_capacity(model.calls.len());
        let mut at_pair: HashMap<(u32, u32, bool), usize> = HashMap::new();
        // One block's own height — the band the seating reserved for its head
        // and its quotation, which is the paper a wire may not cross.
        let own_h = |id: u32| sizes.own.get(&id).map_or(HEAD_H, |&(_, h)| h);
        for call in &model.calls {
            // What the shelving already says takes no ink at all.
            if call.seats {
                continue;
            }
            let answers = call.kind == CallKind::Answers;
            let (def, user) = (model.shown(call.def), model.shown(call.user));
            // Both ends inside one fold: the frame is the whole of what the
            // reader can see, and a line from a head to itself says nothing.
            if def == user {
                continue;
            }
            if let Some(&at) = at_pair.get(&(def, user, answers)) {
                let wire: &mut WireView = &mut wires[at];
                wire.count += call.count;
                // A fold gathers real ends onto one line, so the line inherits
                // what the diff said about every one of them: a folded frame
                // standing for a changed callee is a changed callee.
                wire.def_dirty |= touched.contains(&call.def);
                wire.user_dirty |= touched.contains(&call.user);
                continue;
            }
            let (Some(from), Some(to)) = (rects.get(&def), rects.get(&user)) else {
                continue;
            };
            at_pair.insert((def, user, answers), wires.len());
            // Each end ties on its own band's facing edge, and the band is the
            // block's *own* height — a frame's box is mostly the shelf it
            // holds, which is the sheet's ground and not the block's paper.
            let (from_own, to_own) = (own_h(def), own_h(user));
            let (from_side, to_side) = (from.tie_side(from_own, *to), to.tie_side(to_own, *from));
            wires.push(WireView {
                key: format!("{def}-{user}-{}", answers as u8),
                // The middle of the facing edge for now; the fan pass below
                // spreads the ends that share one edge.
                from: from.tie_at(from_own, from_side, 0.5),
                to: to.tie_at(to_own, to_side, 0.5),
                def,
                user,
                count: call.count,
                label: None,
                width: 0.0,
                def_dirty: touched.contains(&call.def) || touched.contains(&def),
                user_dirty: touched.contains(&call.user) || touched.contains(&user),
                from_side,
                to_side,
                class: match answers {
                    true => "is-answers",
                    false => "is-call",
                },
            });
        }
        // The count is only final once every folded call has been gathered, so
        // the label and the engraved width are read from it at the end.
        for wire in wires.iter_mut() {
            let answers = wire.class == "is-answers";
            wire.width = wire_width(answers, wire.count);
            wire.label = match answers {
                true => Some("answers".to_string()),
                false => (wire.count > 1).then(|| wire.count.to_string()),
            };
        }
        // **The fan.** Every end that ties to one edge of one head row spreads
        // across that edge instead of stacking on its middle. A head the survey
        // reaches from six places used to take six arrowheads on one point —
        // a blot exactly where the reader was looking for a name — and six
        // lines converging through the head text to reach it. Spread, each line
        // arrives on its own bit of boundary and the head stays readable.
        //
        // Ordered along the edge by where the other end stands, so the fan
        // spreads rather than braids, and by the wire itself where two other
        // ends stand together, so one survey always draws one chart.
        let mut fan: HashMap<(u32, TieSide), Vec<(usize, bool)>> = HashMap::new();
        for (at, wire) in wires.iter().enumerate() {
            fan.entry((wire.def, wire.from_side))
                .or_default()
                .push((at, false));
            fan.entry((wire.user, wire.to_side))
                .or_default()
                .push((at, true));
        }
        for ((id, side), mut ends) in fan {
            let Some(&place) = rects.get(&id) else {
                continue;
            };
            // Where the other end of one wire stands, along this edge's own
            // axis: across the paper for a top or a foot, down it for a side.
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

        // One boundary per frame that shelves anything, folded or not: the ring
        // a reader clicks to take the whole subtree, drawn outermost first so a
        // nested boundary is the one a click inside it lands on. A folded
        // frame's boundary still reads — its head and its counted words carry
        // the lit ink for everything they stand for. A leaf has no boundary at
        // all: its box is the mark, and the head already selects it.
        let mut edges: Vec<EdgeView> = views
            .iter()
            .filter(|v| v.shelves)
            .filter_map(|v| {
                Some(EdgeView {
                    id: v.id,
                    at: *rects.get(&v.id)?,
                    title: v.edge_title.clone(),
                    path: v.path.clone(),
                    label: v.label.clone(),
                })
            })
            .collect();
        edges.sort_by_key(|e| (depth.get(&e.id).copied().unwrap_or(0), e.id));

        // The far edition's **landmark register**: the frames big enough to be
        // territory name themselves across it, because below reading zoom a
        // 10.5px head row is three pixels of dust and the one question the
        // opening view has to answer is where running starts.
        //
        // Two rules keep a register out of a pile. A landmark is drawn only
        // where the box carries the far ramp's floor, which is what excludes
        // every leaf and most of the small frames. And a frame nested inside a
        // named frame has to **clear** that name — one step smaller, and its own
        // band of paper below the name above it — because the seating nests ten
        // frames deep along one corner and every one of them naming itself would
        // be ten names over one square of paper. Read outermost first, so each
        // frame sees the register it stands under.
        let mut register: HashMap<u32, (f64, f64)> = HashMap::new();
        let mut names: Vec<NameView> = Vec::new();
        for view in &views {
            let Some(&at) = rects.get(&view.id) else {
                continue;
            };
            // The nearest frame above this one that took a landmark: what it
            // leaves is the ceiling on this one's size and the floor under its
            // top.
            let (mut ceiling, mut clear_of) = (LANDMARK_MAX, f64::NEG_INFINITY);
            let mut up = model.via.get(&view.id).copied();
            for _ in 0..512 {
                let Some(at) = up else { break };
                if let Some(&(foot, size)) = register.get(&at) {
                    ceiling = size - LANDMARK_STEP;
                    clear_of = foot + LANDMARK_GAP;
                    break;
                }
                up = model.via.get(&at).copied();
            }
            if at.y < clear_of {
                continue;
            }
            if let Some(name) = NameView::measure(view.id, &view.name, at, ceiling) {
                register.insert(view.id, (name.foot(), name.size));
                names.push(name);
            }
        }

        let deepest = model.facts.deepest;
        let bands = model
            .marks
            .iter()
            .map(|m| (m.id, m.tier.band(deepest)))
            .collect();
        let frame = Rect::bounds(nodes.iter().map(|n| n.rect()));
        let ground: HashSet<u32> = model.seats.iter().copied().collect();
        let entry = Rect::bounds(
            nodes
                .iter()
                .filter(|n| ground.contains(&n.data.0.id))
                .map(|n| n.rect()),
        );
        FnDrawing {
            nodes,
            ring: placed.ring,
            edges,
            names,
            wires,
            bands,
            rects,
            locate,
            frame,
            entry,
            dirty: model.dirty,
        }
    }
}

/// The selection's own ink: what the chart lights, and what recedes.
///
/// A mark selection is read **geometrically**, because the seating is the
/// reading: the frames it stands inside and everything shelved within it keep
/// full ink, the blocks at the far end of its lit wires read a step behind, and
/// every stranger recedes. A module lights every mark written in it, wherever
/// the call tree seated them; a band lights every mark at that depth and keeps
/// the way in to each readable.
#[derive(Clone, PartialEq)]
struct FnKin {
    sel: Option<u32>,
    /// The frame whose whole boundary is selected.
    tree: Option<u32>,
    band: Option<u32>,
    lit: HashSet<u32>,
    near: HashSet<u32>,
    /// The wires this reading inks and keeps inked.
    wires: HashSet<(u32, u32)>,
}

impl FnKin {
    /// One mark in hand. The selection is the anchor the `wires` reading wants:
    /// `calls` keeps the wires leaving it, `callers` the wires arriving at it,
    /// `both` keeps both — so moving the switch with something selected moves
    /// the picture, which is the whole point of a direction.
    fn mark(sel: u32, model: &FnModel, reading: FnWires) -> Self {
        let mut lit = model.ancestors(sel);
        lit.extend(model.subtree(sel));
        let wires: HashSet<(u32, u32)> = model
            .calls
            .iter()
            .filter(|c| !c.seats && reading.draws(&sel, &c.def, &c.user))
            .map(|c| (model.shown(c.def), model.shown(c.user)))
            .filter(|(def, user)| def != user)
            .collect();
        let near: HashSet<u32> = wires
            .iter()
            .map(|&(def, user)| if def == sel { user } else { def })
            .filter(|id| !lit.contains(id))
            .collect();
        Self {
            sel: Some(sel),
            tree: None,
            band: None,
            lit,
            near,
            wires,
        }
        .carry(model)
    }

    /// One frame's whole boundary. Containment is the call, so the box *is* a
    /// subtree: everything shelved inside keeps full ink, everything one call
    /// across the line reads a step behind, and every wire that crosses the
    /// boundary inks and stays inked — what enters and leaves the box is what a
    /// reader came to the boundary to read. Wires wholly inside stay as the
    /// wires reading draws them.
    fn tree(sel: u32, model: &FnModel, reading: FnWires) -> Self {
        let inside = model.subtree(sel);
        // The box is the anchor, so the direction reads *the boundary*: `calls`
        // keeps what the code inside runs beyond the line, `callers` keeps whose
        // code beyond the line runs something inside it. The same three lines as
        // a mark selection, with `inside` standing where the mark stood.
        let wires: HashSet<(u32, u32)> = model
            .calls
            .iter()
            .filter(|c| !c.seats)
            .filter(|c| inside.contains(&c.def) != inside.contains(&c.user))
            .filter(|c| reading.draws(&true, &inside.contains(&c.def), &inside.contains(&c.user)))
            .map(|c| (model.shown(c.def), model.shown(c.user)))
            .filter(|(def, user)| def != user)
            .collect();
        let mut near: HashSet<u32> = wires
            .iter()
            .flat_map(|&(def, user)| [def, user])
            .filter(|id| !inside.contains(id))
            .collect();
        // The frames it stands inside stay readable, the way a band reading
        // keeps the way in to each of its marks: they are the paper this box is
        // drawn on, not strangers to it.
        near.extend(model.ancestors(sel));
        Self {
            sel: None,
            tree: Some(sel),
            band: None,
            lit: inside,
            near,
            wires,
        }
        .carry(model)
    }

    /// Where a fold hides part of what this reading lights, the head that
    /// stands for it carries the ink. Recede acts on a block's paint and never
    /// on its box, so a folded frame whose subtree is in the chain reads at
    /// full pressure: the reviewer has to see that the chain runs through it.
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

    fn module(key: &[String], model: &FnModel) -> Self {
        // Everything written inside the module, however deep the path runs
        // under it — `views` lights `views::func` too.
        let lit: HashSet<u32> = model
            .marks
            .iter()
            .filter(|m| m.mod_key().starts_with(key))
            .map(|m| m.id)
            .collect();
        let near = model.one_hop(&lit);
        Self {
            sel: None,
            tree: None,
            band: None,
            lit,
            near,
            // A module folds nothing back. A hundred declarations are written
            // in `views::func` and the call tree seats them all over the
            // sheet, so giving their calls back inked a hairball across the
            // whole plate and said nothing about where the code is written —
            // which is the only question this reading asks. The wires already
            // drawn between two lit marks still ink, and those are bounded.
            wires: HashSet::new(),
        }
        .carry(model)
    }

    fn whole_band(band: u32, model: &FnModel, drawing: &FnDrawing) -> Self {
        let lit: HashSet<u32> = drawing
            .bands
            .iter()
            .filter(|(_, at)| **at == band)
            .map(|(id, _)| *id)
            .collect();
        // The way in to each stays readable: a band is a depth, and the frames
        // a mark stands inside are how the paper says how it got there.
        let near: HashSet<u32> = lit
            .iter()
            .flat_map(|&id| model.ancestors(id))
            .filter(|id| !lit.contains(id))
            .collect();
        Self {
            sel: None,
            tree: None,
            band: Some(band),
            lit,
            near,
            // A stratum holds sixty marks and every call on the sheet touches
            // one, so unfolding here would be the hairball this system forbids.
            wires: HashSet::new(),
        }
        .carry(model)
    }

    /// The mark this reading is read from, where it is read from one: what the
    /// keyboard walks from, and what the fold key acts on.
    fn at(&self) -> Option<u32> {
        self.sel.or(self.tree)
    }

    /// The class one block wears in this reading.
    fn block_class(&self, id: u32) -> &'static str {
        if self.sel == Some(id) {
            return "is-picked";
        }
        if self.lit.contains(&id) {
            return "is-kin";
        }
        if self.near.contains(&id) {
            return "is-near";
        }
        "is-dim"
    }

    /// The class one wire wears. A wire this reading inked is the reading's own
    /// ink; every other wire recedes with the strangers.
    fn wire_class(&self, wire: &WireView) -> &'static str {
        match self.inks(wire) || (self.lit.contains(&wire.def) && self.lit.contains(&wire.user)) {
            true => "is-kin",
            false => "is-dim",
        }
    }

    /// Whether this reading gives a folded wire back.
    fn inks(&self, wire: &WireView) -> bool {
        self.wires.contains(&(wire.def, wire.user))
    }
}

// ---------------------------------------------------------------------------
// The drawing.
// ---------------------------------------------------------------------------

/// One block on the paper: the head row, the signature quoted under it, and —
/// where the declaration calls anything — the frame that holds what it calls.
#[component]
fn FnPlate(
    view: MeasuredBlock,
    kin: Option<FnKin>,
    hot: Signal<Option<u32>>,
    fit: Signal<Option<u32>>,
    /// A frame a fold mark asked to be closed or opened: the mark, and whether
    /// the reader asked for the whole subtree under it.
    fold: Signal<Option<(u32, bool)>>,
    /// This frame's right edge and its width, in world units: what the fold mark
    /// rides, and how far it may ride before it would leave its own frame.
    right: f64,
    width: f64,
) -> Element {
    let nav = use_navigator();
    let kin_class = kin.as_ref().map_or("", |k| k.block_class(view.id));
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
    // own source. A head row that is not the selection yet is selected by it —
    // there is nothing else Enter could mean on a mark the reader has not
    // picked — but on the mark already in hand it opens the quotation, which is
    // exactly what the same key does from the chart. Clicking is what lets a
    // selection go; Enter never does two things.
    let pressed = match picked {
        true => crate::views::func::peek_route(
            &(view.path.clone(), view.label.clone()),
            &view.path,
            &view.label,
        ),
        false => to.clone(),
    };
    let mut hot = hot;
    let mut fit = fit;
    let mut fold = fold;
    let id = view.id;
    // Folding is its own gesture and never a selection's side effect: the mark
    // is the control, and a shift- or alt-click asks for the whole subtree.
    let shut = move |e: Event<MouseData>| {
        e.prevent_default();
        e.stop_propagation();
        let deep = e.modifiers().shift() || e.modifiers().alt();
        fold.set(Some((id, deep)));
    };
    let fold_words = match view.folded {
        true => format!(
            "{} is folded — {} · click to open it, shift-click to open every fold inside it",
            view.name, view.packed
        ),
        false => format!(
            "fold {} to its own box — shift-click folds every frame inside it too",
            view.name
        ),
    };
    rsx! {
        div {
            class: "fn-mark",
            class: if !kin_class.is_empty() { "{kin_class}" },
            class: if view.entry { "is-entry" },
            class: if view.ring { "is-ring" },
            // A frame that shelves something carries a control on its border,
            // so its border is drawn a step stronger than a leaf's: the
            // affordance is the line itself, at rest, before any hover.
            class: if view.shelves { "is-frame" },
            class: if view.letter.is_some() { "is-diff" },
            // Where this frame stands in the world — its right edge, and how
            // wide it is — so the fold mark can ride that edge without ever
            // sliding off the frame's own left end. Two numbers per frame,
            // written once per build; the camera writes the third.
            style: if view.shelves { "--own-r: {right}px; --own-w: {width}px;" },
            // The head row and the quotation under it are the hit target, not
            // the box: a frame is as wide as everything it calls, and a box
            // that size would take the clicks meant for the blocks shelved
            // inside it. The box's own boundary is a control of its own, drawn
            // as a ring on the ground layer.
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
                ondoubleclick: move |e: Event<MouseData>| {
                    e.prevent_default();
                    e.stop_propagation();
                    fit.set(Some(id));
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
                // The declaration's own name, as rust writes it and in one run:
                // the owner it is written under, the name, and the bracket the
                // signature opens with, with no space invented between them.
                span { class: "fm-id",
                    if let Some(owner) = view.owner.clone() {
                        span { class: "fm-own", "{owner}" }
                    }
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
                if let Some(module) = view.module.clone() {
                    span { class: "fm-mod", "{module}" }
                }
                // The fold is a mark of its own, at the **right-most end of the
                // head row** — where the data chart puts its own, one rung up,
                // and where a reader who has used one chart looks on the other
                // (2026-08-27, user: *"the fold button should be on the right
                // most side? make the ui ux consistent please."*).
                //
                // A frame is as wide as everything it calls, so on the widest
                // frames the right end of the head row is off the glass. That is
                // what `--fold-slide` answers: the mark rides its frame's right
                // edge, or the right edge of the *visible* span where the frame
                // runs past it, so it is always on the glass and always in the
                // same place relative to the reader rather than to the paper.
                if view.shelves {
                    button {
                        class: "fm-fold",
                        "aria-label": "{fold_words}",
                        title: "{fold_words}",
                        onclick: shut,
                        if view.folded { "+" } else { "−" }
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
            // The hairline that closes the quotation and opens the shelf under
            // it — the same rule a data block draws over the state it owns.
            if view.shelves {
                div { class: "fm-rule" }
            }
            // Nothing is silently cut: a folded frame says what it is holding
            // back, where it was holding it — under the mark that did it, riding
            // the same edge, because the words and the mark are one control.
            if view.folded {
                button {
                    class: "fm-packed",
                    "aria-label": "{fold_words}",
                    title: "{fold_words}",
                    onclick: shut,
                    "{view.packed}"
                }
            }
        }
    }
}

/// Node view for the function chart.
#[component]
fn FnNode(
    ctx: NodeViewCtx<FnNodeData>,
    kin: Option<FnKin>,
    hot: Signal<Option<u32>>,
    fit: Signal<Option<u32>>,
    fold: Signal<Option<(u32, bool)>>,
) -> Element {
    let FnNodeData(view) = ctx.node.data.clone();
    // Where this frame's right edge stands in the world. The fold mark rides
    // that edge, or the right edge of the glass where the frame runs past it —
    // see `--cam-r` and [`FN_SLIDE_JS`]. Static per build, so the slide costs
    // no re-render: the camera moves one custom property on the chart root and
    // the browser does the arithmetic.
    let width = ctx.node.size.map_or(0.0, |s| s.width);
    let right = ctx.node.position.x + width;
    rsx! {
        FnPlate { view: *view, kin, hot, fit, fold, right, width }
    }
}

/// The frames' own boundaries, on the ground under the blocks: one ring per
/// frame that shelves anything, and a click on it selects the whole subtree the
/// box holds. Only the geometry a reader aims at is hittable — a wide invisible
/// stroke over the box's own rectangle — so the frame's interior stays open
/// paper for the blocks, the wires and the pan, exactly as a module boundary
/// does one rung up.
#[component]
fn EdgeLayer(edges: Vec<EdgeView>, kin: Option<FnKin>) -> Element {
    let nav = use_navigator();
    let chosen = kin.as_ref().and_then(|k| k.tree);
    let dim = |id: u32| {
        kin.as_ref()
            .is_some_and(|k| !k.lit.contains(&id) && k.tree != Some(id))
    };
    let ring = |e: &EdgeView| -> Element {
        let sel = chosen == Some(e.id);
        let to = match sel {
            true => Route::FnOverview {},
            false => tree_route(&e.path, &e.label),
        };
        let pressed = to.clone();
        let words = match sel {
            true => format!(
                "{} — the boundary is selected · click it again to let it go",
                e.title
            ),
            false => e.title.clone(),
        };
        rsx! {
            g {
                key: "{e.id}",
                class: "fn-edge",
                class: if sel { "is-sel" },
                class: if dim(e.id) { "is-dim" },
                role: "link",
                tabindex: "0",
                "aria-label": "{words}",
                onclick: move |ev: Event<MouseData>| {
                    ev.stop_propagation();
                    nav.push(to.clone());
                },
                onkeydown: move |ev: Event<KeyboardData>| {
                    if ev.key() == Key::Enter {
                        ev.stop_propagation();
                        nav.push(pressed.clone());
                    }
                },
                title { "{words}" }
                rect {
                    class: "fn-edge-line",
                    x: "{e.at.x}",
                    y: "{e.at.y}",
                    width: "{e.at.w}",
                    height: "{e.at.h}",
                }
                rect {
                    class: "fn-edge-hit",
                    x: "{e.at.x}",
                    y: "{e.at.y}",
                    width: "{e.at.w}",
                    height: "{e.at.h}",
                }
            }
        }
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for e in edges.iter() {
                {ring(e)}
            }
        }
    }
}

/// The far edition's **landmark register**: each ground frame's own name
/// engraved across its territory, the way the data chart names a module region
/// one rung up and for the same reason. At the opening zoom the camera has
/// scaled a 10.5px head row down to three pixels, and the question the opening
/// view exists to answer — where does running start — is a question about
/// names.
///
/// This is a mark of its own, not the block's name grown: a block still keeps
/// its reading size at every zoom, because a block sits in a box the call tree
/// sized. The landmark is measured to the frame it names, haloed in paper,
/// drawn over the blocks — the shelved section packs frames tight, so on the
/// ground the name would spend its ink behind the paper — and shown only while
/// the chart is far.
#[component]
fn NameLayer(names: Vec<NameView>, kin: Option<FnKin>) -> Element {
    // Under a reading the landmarks recede with their frames: a name at full
    // pressure over a receded frame would say the opposite of what the frame
    // says.
    let class = |id: u32| match kin.as_ref() {
        None => "",
        Some(kin) if kin.at() == Some(id) => "is-sel",
        Some(kin) if kin.lit.contains(&id) => "",
        Some(_) => "is-dim",
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for n in names.iter() {
                text {
                    key: "{n.id}",
                    class: "fn-frame-name",
                    class: if !class(n.id).is_empty() { "{class(n.id)}" },
                    x: "{n.x}",
                    y: "{n.y}",
                    font_size: "{n.size}",
                    // The halo scales with the lettering, as the ring strip's
                    // caption does: the name is engraved on the world, so it
                    // crosses whatever the reader has panned under it.
                    style: "stroke-width: {(n.size * 0.38).round()}px;",
                    "{n.name}"
                }
            }
        }
    }
}

/// The ring strip's caption: what stands below the ground, and the one band
/// this chart still captions. The words are the control — a band is a focus
/// like any other — and the hairline under them is never a pointer target, so
/// a click always means the words it lands on.
#[component]
fn RingLayer(ring: Option<RingStrip>, kin: Option<FnKin>) -> Element {
    let nav = use_navigator();
    let Some(strip) = ring else {
        return rsx! {};
    };
    let chosen = kin
        .as_ref()
        .and_then(|k| k.band)
        .is_some_and(|at| at == strip.band);
    let dim = kin.is_some() && !chosen;
    let to = match chosen {
        true => Route::FnOverview {},
        false => band_route(strip.band),
    };
    let words = match chosen {
        true => format!("{} — selected · click again to deselect", strip.caption),
        false => format!("{} — select the band and read which", strip.caption),
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            g {
                class: "fn-lane",
                class: if chosen { "is-sel" },
                class: if dim { "is-dim" },
                line {
                    class: "fn-lane-rule",
                    x1: "{strip.at.x}",
                    y1: "{strip.at.y - 9.0}",
                    x2: "{strip.at.x + strip.at.w}",
                    y2: "{strip.at.y - 9.0}",
                }
                g {
                    class: "fn-lane-pick",
                    role: "link",
                    tabindex: "0",
                    "aria-label": "{words}",
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        nav.push(to.clone());
                    },
                    title { "{words}" }
                    rect {
                        class: "fn-lane-hit",
                        x: "{strip.at.x}",
                        y: "{strip.at.y - 22.0}",
                        width: "260",
                        height: "18",
                    }
                    text {
                        class: "fn-lane-caption",
                        x: "{strip.at.x}",
                        y: "{strip.at.y - 13.0}",
                        "{strip.caption}"
                    }
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

/// One wire, drawn: curve, head, and the label riding the curve's midpoint.
fn draw_wire(w: &WireView, side: f64, classes: &str) -> Element {
    let (d, ctrl) = curve(w.from, w.to, side);
    let head = arrowhead(w.to, ctrl, 3.2 + w.width);
    let (lx, ly) = (
        0.25 * w.from.x + 0.5 * ctrl.x + 0.25 * w.to.x,
        0.25 * w.from.y + 0.5 * ctrl.y + 0.25 * w.to.y,
    );
    rsx! {
        g { key: "{w.key}", class: "fn-wire {w.class}",
            class: "{classes}",
            path {
                class: "wire-path",
                d,
                fill: "none",
                style: "stroke-width: {w.width}px;",
            }
            path { class: "wire-head", d: head }
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
/// - a wire the selection's own reading inks is drawn, whichever stop is on —
///   the selection *is* the anchor, and its direction was already applied when
///   [`FnKin`] read it;
/// - with a selection, everything else is off the paper: the reader asked about
///   one mark, and the sheet answers about that mark;
/// - with no selection but a diff, the diff's touched declarations are the
///   anchor — `calls` draws the wires leaving them, `callers` the wires arriving
///   at them, `both` draws every wire;
/// - with neither, every stop draws everything, because a reading with nothing
///   in focus has no direction to take. This is the data chart's rule for its
///   `references` reading, word for word.
///
/// A contract wire (`answers`) never folds, whatever the reading says: it is
/// what makes the shelved section honest about a `dyn` call the survey cannot
/// follow, and there is no direction in which that stops being true.
fn drawn_under(reading: FnWires, w: &WireView, picked: bool, dirty: bool, lit: bool) -> bool {
    if lit || w.class == "is-answers" {
        return true;
    }
    if picked {
        return false;
    }
    !dirty || reading == FnWires::Both || w.anchored(reading)
}

/// Both families as one engraved layer, the contracts first and lighter.
///
/// Drawn **twice**, at two altitudes, because a wire at rest and a wire the
/// reader lit are two different kinds of ink (2026-08-27, user: *"the lines
/// crossing over the boxes … they are too pronounced"*):
///
/// - the resting families and the strangers a reading pushed back go **under**
///   the blocks (`over: false`), where the paper of every block they pass
///   behind covers them. They keep the gutters — between the frames on the
///   ground, between the shelved rows — which is all a resting wire ever needed:
///   a trace of where the line runs, not a line across a quotation;
/// - the selection's own lit reading goes **over** them (`over: true`), because
///   ink the reader asked for has to be followable end to end.
///
/// The two together are exactly what one layer drew before, so no reading gains
/// or loses a wire by this split.
///
/// No wire on this chart ever takes the flare. The structural diff reads the
/// base edition syntactically, so it is exact about *declarations* and knows
/// nothing about a rewritten body: there is no such thing as a changed call for
/// it to ink. Colouring every wire that merely touched a changed declaration
/// washed a large change's sheet amber and said nothing true — the diff is on
/// the blocks, where the survey can actually see it.
#[component]
fn WireLayer(
    wires: Vec<WireView>,
    kin: Option<FnKin>,
    reading: FnWires,
    /// Whether the diff has anything to say. With no selection this is what
    /// gives the reading an anchor to take a direction against.
    dirty: bool,
    /// Whether this is the layer over the blocks — the lit reading — or the
    /// resting one under them.
    over: bool,
) -> Element {
    // A wire this reading does not draw is not in the DOM at all; the hover
    // reading gives it back through [`HotWireLayer`] instead, in a layer of its
    // own, so this layer never changes while the pointer merely travels. (Kept
    // out entirely, never `display: none`: toggling display on an svg child
    // relayouts the whole svg, and this svg's painted bounds are the world.)
    let picked = kin.is_some();
    let wire = |w: &WireView, side: f64| {
        let lit = kin.as_ref().is_some_and(|k| k.inks(w));
        if !drawn_under(reading, w, picked, dirty, lit) {
            return None;
        }
        let classes = match kin.as_ref() {
            Some(kin) => kin.wire_class(w),
            // No selection: the diff is the anchor, so a wire it reads in this
            // direction carries the resting pressure and one the sheet merely
            // admits is drawn a step lighter. With no diff either, nothing is in
            // focus and every wire is admitted rather than asked for.
            None if dirty && w.anchored(reading) => "is-quiet",
            None => "is-faint",
        };
        // One altitude per kind of ink: the lit reading rides over the blocks,
        // everything at rest under them.
        if (classes == "is-kin") != over {
            return None;
        }
        Some(draw_wire(w, side, classes))
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for w in wires.iter().filter(|w| w.class == "is-answers") {
                {wire(w, -1.0)}
            }
            for w in wires.iter().filter(|w| w.class == "is-call") {
                {wire(w, 1.0)}
            }
        }
    }
}

/// The hovered reading's own ink, drawn over the resting family in a
/// compositor layer of its own — the same split, for the same measured
/// reason, as the data chart's [`HotWireLayer`]. Hovering either end inks every
/// wire that mark has, both ways round: what a reader hovers a block for is
/// what the shelving could not tell them.
#[component]
fn HotWireLayer(wires: Vec<WireView>, hot: Signal<Option<u32>>) -> Element {
    // The svg stays mounted through an empty reading — see the data chart's
    // overlay for why.
    let h = hot();
    let lit = |w: &WireView, side: f64| {
        h.is_some_and(|h| w.def == h || w.user == h)
            .then(|| draw_wire(w, side, "is-hot"))
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for w in wires.iter().filter(|w| w.class == "is-answers") {
                {lit(w, -1.0)}
            }
            for w in wires.iter().filter(|w| w.class == "is-call") {
                {lit(w, 1.0)}
            }
        }
    }
}

/// Chrome insets at this altitude: the cartouche column left, the sheet right
/// while a mark is selected — the choreography every altitude keeps.
fn chrome_insets(narrow: bool, panel: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (312.0, 20.0, 70.0, 12.0)
    } else {
        (56.0, if panel { 330.0 } else { 24.0 }, 24.0, 284.0)
    }
}

const MIN_CHART_ZOOM: f64 = 0.18;

/// Below this the module words are dust and the chart holds its far edition:
/// names, edges and wires alone. Hysteresis keeps the swap from flapping while
/// the reader hovers around the threshold.
const FAR_IN: f64 = 0.45;
const FAR_OUT: f64 = 0.55;
/// The zoom a selection is read at: when a chosen mark sits below this, or off
/// the glass entirely, the camera glides to it.
#[cfg(target_arch = "wasm32")]
const READ_ZOOM: f64 = 0.5;

/// The zoom the chart opens at, at the lowest. Where the whole sheet cannot be
/// read at once the opening view holds this much scale and puts the reader on
/// the ground, where the running starts. `f` still fits the whole sheet, at
/// whatever zoom that takes.
#[cfg(target_arch = "wasm32")]
const OPEN_FLOOR: f64 = 0.34;
/// And no closer than this on a small workspace, where the whole sheet fits.
#[cfg(target_arch = "wasm32")]
const OPEN_CEIL: f64 = 0.75;

/// Which seating a remembered camera belongs to. Each order is a different
/// sheet — a shelf read by weight is not a shelf read by module — and handing a
/// reader one camera on the other would lose their place.
pub(in crate::views) type Seating = FnOrder;

/// The camera as the reviewer last left it, surviving route-variant remounts.
#[derive(Clone, Copy)]
pub(in crate::views) struct FnCamera {
    pub(in crate::views) viewport: Signal<Option<(Seating, Viewport)>>,
}

impl FnCamera {
    pub(in crate::views) fn new() -> Self {
        Self {
            viewport: Signal::new(None),
        }
    }
}

/// Put a reader down on a sheet they have not seen: the whole chart where that
/// is legible, and the ground at reading scale where it is not.
#[cfg(target_arch = "wasm32")]
fn open_chart(
    flow: dioxus_flow::prelude::FlowHandle<FnNodeData>,
    whole: Rect,
    entry: Option<Rect>,
    panel: bool,
) {
    let Some((w, h)) = window_size() else { return };
    let (t, r, b, l) = chrome_insets(narrow_viewport(), panel);
    let free_w = (w - l - r).max(120.0);
    let free_h = (h - t - b).max(120.0);
    let fit = (free_w / whole.width.max(1.0)).min(free_h / whole.height.max(1.0)) * 0.94;
    if fit >= OPEN_FLOOR {
        frame_chart(flow, whole, panel, 0);
        return;
    }
    // Too much sheet to read at once: hold the floor and open on the ground —
    // at the *start* of it. The ground's frames are packed heaviest first, left
    // to right and then down, so its top-left corner is where the running
    // starts and where a reader's eye goes; centring on the bounds of every
    // entry point at once puts them in the middle of the widest gap between two
    // of them, looking at blank paper.
    let Some(core) = flow.core() else { return };
    let at = entry.unwrap_or(whole);
    let zoom = ((free_w / at.width.max(1.0)).min(free_h / at.height.max(1.0)) * 0.94)
        .clamp(OPEN_FLOOR, OPEN_CEIL);
    core.set_viewport(
        Viewport::new(l + 12.0 - at.x * zoom, t + 12.0 - at.y * zoom, zoom),
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

/// Keyboard at this altitude: the arrows walk the seating — down into the first
/// callee on the shelf (which the shelf order decides, so it is the heaviest
/// chain under `weight` and the first cluster under `module` or `owner`), up to
/// the caller a block seats in, left and right along the shelf — `z` folds the
/// picked frame the way vim folds a block, `enter` opens the picked
/// declaration's own source, `f` refits, Escape steps back out of the deepest
/// thing open (a quotation first, then the selection), and `/` finds.
///
/// **Left and right are the trail**, always — the browser's own back and
/// forward, exactly as at every other altitude, selection or no selection
/// (2026-08-27, user). This chart owns the whole arrow grammar rather than
/// sharing it with the shell's listener, so it has to answer those two keys
/// itself; what it no longer does is give them a second meaning. Walking a
/// shelf sideways is what clicking is for, and what stepping down into a shelf
/// and back up already does.
///
/// `enter` is only the chart's while the page's own focus is not on a control:
/// a head row, a fold mark and a sheet row all answer Enter themselves, and a
/// key that fired twice would be two gestures for one press.
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
/// that lets every wide frame's fold mark sit at the right edge of the *visible*
/// span instead of at the right edge of a two-thousand-unit box that is mostly
/// off screen: `--cam-r`, the world x the viewport's right edge is over.
///
/// The arithmetic is CSS, in `tailwind.css`: a mark translates left by
/// `--cam-r − --own-r`, clamped so it never leaves its own frame and never
/// passes the frame's real right edge. So the whole slide costs **one custom
/// property per animation frame** and no re-render at all — this is not a
/// component, because a component that read the viewport would re-render on
/// every pan frame, and the marks are the one layer this system may never
/// invalidate while the pointer is merely travelling (see the flicker note in
/// `DESIGN.md`).
///
/// It reads the pan straight off the viewport pane's own inline transform, which
/// costs no layout, caches the container width and re-measures it only on
/// resize, and writes nothing unless the value actually moved.
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
        // `translate(Xpx, Ypx) scale(Z)` — the pane writes its own transform, so
        // reading the string costs nothing and forces no layout.
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
    // same chosen mark, so moving the switch has to re-read the kin.
    let kin: Memo<Option<FnKin>> = use_memo(use_reactive(
        (&sel, &*fns.wires.read()),
        move |(sel, reading)| {
            let model = model.read();
            let drawing = chart.read();
            // A selection a fold has off the paper is read on the head that stands
            // for it: the frame stands for what it hides, so a reader who folds the
            // frame their selection sits in still sees where the selection went.
            let seat = |path: String, label: String| -> Option<u32> {
                match drawing.locate.get(&(path.clone(), label.clone())) {
                    Some(&id) => Some(id),
                    None => Some(model.shown(model.find(&path, &label)?.id)),
                }
            };
            match sel? {
                FnSel::Mark(path, label) => Some(FnKin::mark(seat(path, label)?, &model, reading)),
                FnSel::Tree(path, label) => Some(FnKin::tree(seat(path, label)?, &model, reading)),
                FnSel::Mod(key) => Some(FnKin::module(&key, &model)),
                FnSel::Band(band) => Some(FnKin::whole_band(band, &model, &drawing)),
            }
        },
    ));

    let sel_on: Signal<bool> = use_signal(|| false);
    use_effect(use_reactive((&sel.is_some(),), move |(on,)| {
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
    let picked: Signal<Option<u32>> = use_signal(|| None);
    use_effect(move || {
        let at = kin.read().as_ref().and_then(FnKin::at);
        let mut picked = picked;
        if *picked.peek() != at {
            picked.set(at);
        }
    });

    let nodes: Signal<Vec<FlowNode<FnNodeData>>> = use_signal(Vec::new);
    // What the pointer is over, and what the chart answers it with. The
    // second is the first once it has been held long enough to be a
    // question — see `use_settled`.
    let hot: Signal<Option<u32>> = use_signal(|| None);
    let settled = use_settled(hot);
    // A frame a double-click asked to be fitted to.
    let fit: Signal<Option<u32>> = use_signal(|| None);
    // A frame a fold mark asked to be closed or opened, and whether the reader
    // asked for the whole subtree under it.
    let fold: Signal<Option<(u32, bool)>> = use_signal(|| None);
    // Bumped whenever a reveal opened the way in to a selection: the camera
    // reads it so a mark that was behind a fold can still be glided to, without
    // making every fold a camera move.
    let revealed: Signal<u32> = use_signal(|| 0);
    let core_live: Signal<bool> = use_signal(|| false);
    let far: Signal<bool> = use_signal(|| false);

    // The fold gesture, acted on where the model is: toggle the frame the mark
    // names, and with shift or alt every frame under it. Nothing here folds by
    // a count — this is the only place a fold is ever written.
    use_effect(move || {
        let Some((at, deep)) = fold() else { return };
        let mut fold = fold;
        fold.set(None);
        // Every key this gesture touches is read out of the model *first*, and
        // the borrow dropped, before the fold set is written: the model is a
        // memo over that very set, so reading it mid-write would be reading a
        // reading of the thing being written.
        let Some((shut, keys)) = ({
            let model = model.peek();
            let by_id = model.by_id();
            by_id.get(&at).copied().map(|mark| {
                let shut = !mark.folded;
                let mut keys = vec![fold_key(&mark.head.path, &mark.head.label)];
                if deep {
                    // The whole subtree, in one gesture: fold every frame under
                    // this one so opening it again shows one layer, or open
                    // every fold inside it so the reader gets the whole chain
                    // back at once.
                    let mut under: Vec<u32> = model.subtree(at).into_iter().collect();
                    under.sort_unstable();
                    keys.extend(
                        under
                            .into_iter()
                            .filter(|id| *id != at)
                            .filter_map(|id| by_id.get(&id).copied())
                            .filter(|kid| kid.runs > 0)
                            .map(|kid| fold_key(&kid.head.path, &kid.head.label)),
                    );
                }
                (shut, keys)
            })
        }) else {
            return;
        };
        // Written through the session state, which knows the difference between
        // eliding a frame in place — what a fold by hand does, moving nothing —
        // and giving a packed-away frame its room back, which is the one fold
        // gesture that has to lay the paper again.
        fns.fold(keys, shut);
    });

    // The two controls that move every block on the sheet. The paper is being
    // laid again for them anyway, so this is where the packer is allowed to
    // catch up with the folds and close the sheet over what they hide — the
    // reader has no anchor to lose, because nothing is where it was.
    use_effect(use_reactive(
        (&*fns.order.read(), &*fns.vis_floor.read()),
        move |_| fns.repack(),
    ));

    // A selection must never be invisible: every way to a mark — a URL, the
    // search, a sheet row, the arrow walk — opens the folds it is hiding behind
    // first. The model is peeked, never read, so this fires when the *selection*
    // moves and not when the folds do: folding the frame a selection sits in is
    // the reader's own move, and the head that folded stands for it.
    use_effect(use_reactive((&sel,), move |(sel,)| {
        let Some((path, label)) = sel.as_ref().and_then(FnSel::at) else {
            return;
        };
        let model = model.peek();
        let Some(mark) = model.find(path, label) else {
            return;
        };
        let way_in = model.reveal(mark.id);
        if way_in.is_empty() {
            return;
        }
        // A reveal opens the folds on the way in, and it goes through the same
        // door a hand does: a frame the packer had skipped needs its room back,
        // so opening it lays the paper again — which is exactly why the camera
        // answers a reveal and nothing else.
        fns.fold(way_in.into_iter().collect(), false);
        let mut revealed = revealed;
        let now = *revealed.peek();
        revealed.set(now + 1);
    }));

    use_effect(move || {
        let drawing = chart();
        let seating: Seating = *fns.order.read();
        let mut nodes = nodes;
        nodes.set(drawing.nodes);
        // Camera discipline: the reader gets their place back on the seating
        // they left it on, and a seating they have not seen is opened once.
        #[cfg(target_arch = "wasm32")]
        {
            let (frame, entry) = (drawing.frame, drawing.entry);
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
                    Some((at, vp)) if at == seating => flow.set_viewport(vp, 0),
                    _ => {
                        if let Some(frame) = frame {
                            open_chart(flow, frame, entry, panel);
                        }
                    }
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (core_live, camera, seating);
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
        // Peeked, not read: this effect must fire when the camera moves, never
        // when the seating changes — the effect above owns that, and a save
        // racing it would store the old sheet's camera under the new sheet.
        saved.set(Some((*fns.order.peek(), vp)));
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

    // A double-click fits the frame's whole subtree: the block's own box is
    // that subtree, because containment is the call.
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

    // The camera glides to a selection it cannot show: off the glass, or below
    // reading zoom. A selection already legible moves nothing.
    #[cfg(target_arch = "wasm32")]
    use_effect(use_reactive((&sel,), move |(sel,)| {
        if !core_live() {
            return;
        }
        // A reveal is the one re-layout the camera answers: the mark the reader
        // asked for was behind a fold, and now it is on the paper. Every other
        // fold leaves the camera exactly where it was.
        let _ = revealed();
        // A boundary is glided to the same way a mark is: what has to be on the
        // glass is the head that names the box, and a boundary the reader can
        // already read moves nothing.
        let (Some(FnSel::Mark(path, label)) | Some(FnSel::Tree(path, label))) = sel else {
            return;
        };
        let Some(core) = flow.core() else { return };
        let drawing = chart.peek();
        let Some(&id) = drawing.locate.get(&(path.clone(), label.clone())) else {
            return;
        };
        let Some(at) = drawing.rects.get(&id).copied() else {
            return;
        };
        let vp = *core.viewport.peek();
        let Some((w, h)) = window_size() else { return };
        let (vx, vy) = ((0.0 - vp.x) / vp.zoom, (0.0 - vp.y) / vp.zoom);
        let (vw, vh) = (w / vp.zoom, h / vp.zoom);
        // A frame is as wide as everything it calls, so what has to be on the
        // glass is its head — the row that says which declaration this is.
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
                        if let Some(bounds) =
                            Rect::bounds(chart.peek().nodes.iter().map(|n| n.rect()))
                        {
                            let duration = if prefers_reduced_motion() { 0 } else { 400 };
                            frame_chart(flow, bounds, *sel_on.peek(), duration);
                        }
                    }
                    // One step out per press, deepest first: the quotation a
                    // row opened, then the selection itself.
                    "Escape" => {
                        if let Some(back) = quoted.peek().clone() {
                            nav.push(back);
                        } else if *sel_on.peek() {
                            nav.push(Route::FnOverview {});
                        }
                    }
                    // The fold, on the frame in hand. Its own key, because it
                    // is its own gesture: nothing about the selection changes.
                    "z" => {
                        let Some(at) = *picked.peek() else { continue };
                        let mut fold = fold;
                        fold.set(Some((at, false)));
                    }
                    // The whole declaration, read where the sheet's rows are
                    // read: on the quotation plate beside it.
                    "Enter" => {
                        let Some(at) = *picked.peek() else { continue };
                        let model = model.peek();
                        if let Some(mark) = model.by_id().get(&at).copied() {
                            nav.push(crate::views::func::peek_route(
                                &(mark.head.path.clone(), mark.head.label.clone()),
                                &mark.head.path,
                                &mark.head.label,
                            ));
                        }
                    }
                    // Left and right are the trail, here as at every other
                    // altitude, whether anything is selected or not (2026-08-27,
                    // user). They used to walk the shelf while a mark was in
                    // hand, which made one pair of keys mean two unrelated
                    // things depending on state a reader could not see — and it
                    // took the browser's own grammar away from the one chart
                    // where a reviewer steps between marks most.
                    "ArrowLeft" => crate::views::shell::history_back(),
                    "ArrowRight" => crate::views::shell::history_forward(),
                    // Up and down walk the seating, which is the one walk the
                    // paper's own nesting spells out. The shelf is walked by
                    // clicking, and by stepping down into it and back up.
                    step => {
                        let Some(at) = *picked.peek() else { continue };
                        let model = model.peek();
                        let next = match step {
                            "ArrowDown" => {
                                model.kids.get(&at).and_then(|kids| kids.first()).copied()
                            }
                            _ => model.via.get(&at).copied(),
                        };
                        if let Some(mark) = next.and_then(|id| model.by_id().get(&id).copied()) {
                            nav.push(mark_route(&mark.head.path, &mark.head.label));
                        }
                    }
                }
            }
        });
    });

    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);
    let panel = matches!(sel, Some(FnSel::Mark(..) | FnSel::Tree(..)));
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
                    if *sel_on.peek() {
                        nav.push(Route::FnOverview {});
                    }
                },
                node_view: move |ctx: NodeViewCtx<FnNodeData>| {
                    // While the diff has anything to say, whatever it never
                    // touched rests at a lighter pressure; the reading's own
                    // pressure is on the block, not on the node.
                    let rest = kin.read().is_none()
                        && chart.read().dirty
                        && ctx.node.data.0.letter.is_none();
                    rsx! {
                        div { class: "fn-node", class: if rest { "is-rest" },
                            FnNode { ctx, kin: kin(), hot, fit, fold }
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
                    RingLayer { ring: chart.read().ring.clone(), kin: kin() }
                    EdgeLayer { edges: chart.read().edges.clone(), kin: kin() }
                }
                WorldLayer { class: "fn-wires",
                    WireLayer {
                        wires: chart.read().wires.clone(),
                        kin: kin(),
                        reading: *fns.wires.read(),
                        dirty: chart.read().dirty,
                        over: false,
                    }
                }
                WorldLayer { class: "fn-wires fn-wires-lit",
                    WireLayer {
                        wires: chart.read().wires.clone(),
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
    use crate::views::func::model::FnHead;

    fn mark(id: u32, name: &str, module: &[&str], crosses: bool) -> FnMark {
        FnMark {
            id,
            tier: Tier::Entry,
            krate: "slope".to_string(),
            module: module.iter().map(|s| (*s).to_string()).collect(),
            head: FnHead {
                kind: ItemKind::Fn,
                vis: Vis::Pub,
                name: name.rsplit("::").next().unwrap_or(name).to_string(),
                label: name.to_string(),
                path: "src/main.rs".to_string(),
                line: 1,
                section: String::new(),
            },
            rows: Vec::new(),
            owner: None,
            delta: Delta::Same,
            callers: 0,
            calls: 0,
            touches: 0,
            runs: 0,
            crosses,
            recurses: false,
            folded: false,
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

    /// A head says what it is, whose it is, and — only where the seating
    /// crosses a module — the module it is written in.
    #[test]
    fn a_head_says_what_it_is_and_where_it_crosses_from() {
        let quiet = MeasuredBlock::measure(&mark(0, "survey", &["analyze"], false));
        assert_eq!(quiet.decl, "pub fn");
        assert_eq!(quiet.name, "survey");
        assert_eq!(quiet.module, None, "a same-module call is quiet");
        assert_eq!(
            quiet.owner, None,
            "a free declaration is written under none"
        );

        let crossing = MeasuredBlock::measure(&mark(1, "survey", &["analyze", "code"], true));
        assert_eq!(crossing.module.as_deref(), Some("analyze::code"));
        assert!(
            crossing.own.0 > quiet.own.0 || crossing.own.0 == MARK_MAX_W,
            "the module word takes room on the head"
        );
    }

    /// A method's head quotes the survey's own label: the owner in front of the
    /// name, with rust's `::` between them, and the name still the run that
    /// carries. Nothing is rebuilt from a header.
    #[test]
    fn a_method_head_quotes_the_owner_in_front_of_the_name() {
        let method = MeasuredBlock::measure(&mark(0, "FnModel::build", &[], false));
        assert_eq!(method.owner.as_deref(), Some("FnModel::"));
        assert_eq!(method.name, "build");
        // And the prefix is paid for in the measure, so the head cannot clip it.
        let free = MeasuredBlock::measure(&mark(1, "build", &[], false));
        assert!(method.own.0 >= free.own.0);
    }

    /// A block quotes its signature the way rust writes it: the name opens the
    /// parameter list, the parameters are its own rows, and the return closes
    /// it. Rows take room, so a block with a signature is taller than one
    /// without.
    #[test]
    fn a_block_quotes_its_signature_as_rust_writes_it() {
        let mut with = mark(0, "survey", &[], false);
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
        let mut none = mark(1, "facts", &[], false);
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
        let bare = MeasuredBlock::measure(&mark(2, "main", &[], false));
        assert_eq!((bare.open.as_str(), bare.tail.as_str()), ("()", ""));
        assert_eq!(bare.own.1, BORDER + HEAD_H);
        assert!(block.own.1 > bare.own.1, "quoted rows take room");
    }

    /// A frame that shelves nothing has nothing to fold. One that does says so
    /// in words when it is folded, and the words are measured into its box —
    /// nothing is silently cut, and nothing clips.
    #[test]
    fn a_folded_frame_states_what_it_holds_back() {
        let mut frame = mark(0, "survey", &[], false);
        frame.runs = 41;
        let open = MeasuredBlock::measure(&frame);
        assert!(open.shelves);
        assert!(!open.folded);
        assert_eq!(open.packed, "");

        frame.folded = true;
        let shut = MeasuredBlock::measure(&frame);
        assert_eq!(shut.packed, "+ 41 inside");
        assert_eq!(
            shut.own.1,
            open.own.1 + layout::RULE + PACKED_H,
            "the counted words stand where the shelf stood"
        );

        // A leaf draws no fold mark at all: there is nothing behind it.
        let leaf = MeasuredBlock::measure(&mark(1, "note", &[], false));
        assert!(!leaf.shelves);
    }

    /// Every line a block draws, and the width the browser will lay it out at.
    /// The head is one flex run after another with the row's gap between them;
    /// a quoted parameter row stands in from the edge; the tail closes the
    /// quotation on the edge itself. Each is exactly the composition
    /// [`MeasuredBlock::measure`] pays for, written out a second time so a
    /// change to one of them has to be a change to both.
    fn drawn_lines(view: &MeasuredBlock) -> Vec<(String, f64)> {
        let mut lines = Vec::new();
        let mut head = String::new();
        let mut head_w = text_w(&view.decl, KW_PX) + RUN_GAP;
        head.push_str(&view.decl);
        if let Some(owner) = view.owner.as_deref() {
            head.push_str(owner);
            head_w += text_w(owner, NAME_PX);
        }
        head.push_str(&view.name);
        head.push_str(&view.open);
        head_w += text_w(&view.name, NAME_PX) + text_w(&view.open, NAME_PX);
        if let Some(letter) = view.letter {
            head.push_str(letter);
            head_w += RUN_GAP + text_w(letter, 9.5);
        }
        if let Some(module) = view.module.as_deref() {
            head.push_str(module);
            head_w += RUN_GAP + text_w(module, META_PX);
        }
        if view.shelves {
            head_w += RUN_GAP + 12.0;
        }
        lines.push((head, head_w));
        for row in &view.rows {
            lines.push((row.written(), text_w(&row.written(), ROW_PX) + ROW_INDENT));
        }
        if !view.tail.is_empty() {
            lines.push((view.tail.clone(), text_w(&view.tail, ROW_PX)));
        }
        if !view.packed.is_empty() {
            lines.push((view.packed.clone(), text_w(&view.packed, 9.0)));
        }
        lines
    }

    /// The measure is never smaller than what the browser draws.
    ///
    /// A block's box is `widest + PAD_X`, clamped between the two widths a
    /// quotation reads at. So for every line there are exactly two honest
    /// outcomes: the line fits inside the box, or the box is at the widest a
    /// block may be and the line ellipsizes with the whole of itself in its
    /// hover words. A line that overflows a box *under* the clamp would be the
    /// measure and the paper disagreeing, and that is the one thing this
    /// asserts can never happen — over the shapes the real survey hands it,
    /// which are the long generic returns and fully-qualified parameter types
    /// no invented fixture would produce.
    #[test]
    fn every_drawn_line_is_measured_at_least_as_wide_as_it_renders() {
        /// One real declaration's shape: its label, its parameter rows as the
        /// source writes them, and the return type.
        type Shape = (
            &'static str,
            &'static [(&'static str, &'static str)],
            &'static str,
        );
        let shapes: Vec<Shape> = vec![
            // The one the live measure caught: a return long enough to want
            // 324 units in a box that may be 300.
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
            (
                "DepModel::hold",
                &[
                    ("holds", "Vec<(&Anchor, HoldKind, &str, Option<HoldEvent>)>"),
                    ("owner", "Option<crate::views::func::model::Owner>"),
                ],
                "Vec<(CrateInfo, DepKind, Option<DepEvent>)>",
            ),
            // And the short shapes, which must fit outright.
            ("main", &[], ""),
            ("survey", &[("dir", "&Path")], "Result<CodeGraph, String>"),
            ("Tier::band", &[("self", ""), ("deepest", "u32")], "u32"),
        ];
        for (name, rows, ret) in shapes {
            for shelves in [false, true] {
                for crosses in [false, true] {
                    let mut m = mark(0, name, &["views", "func"], crosses);
                    m.runs = u32::from(shelves) * 41;
                    m.folded = shelves;
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
                    // Where the clamp is what cut a line, the whole of it is
                    // still on the block: the plate hands these very strings to
                    // the hover words, so nothing is silently cut.
                    for r in &view.rows {
                        assert!(!r.written().is_empty());
                    }
                    if !ret.is_empty() {
                        assert!(
                            view.tail.contains(ret),
                            "the tail keeps the whole return type for its hover words"
                        );
                    }
                }
            }
        }
    }

    /// A head row never clips its own identity. The quoted rows under it may
    /// ellipsize — a type is a long thing and its hover words carry the rest —
    /// but the run that says *which declaration this is* is paid for in the
    /// measure at every shape the survey produces.
    #[test]
    fn a_head_run_is_never_the_line_the_clamp_cuts() {
        for name in [
            "main",
            "FnModel::build",
            "MeasuredBlock::measure",
            "DataCartouche::visibility_floor",
        ] {
            for crosses in [false, true] {
                let mut m = mark(0, name, &["views", "func"], crosses);
                m.runs = 3;
                let view = MeasuredBlock::measure(&m);
                let (head, drawn) = drawn_lines(&view).remove(0);
                assert!(
                    drawn <= view.own.0 - PAD_X,
                    "`{head}` draws at {drawn} in a box of {}",
                    view.own.0
                );
            }
        }
    }

    /// The far edition's landmark register is measured to its frame, never
    /// guessed: the name it engraves fits the box it names, or the box gets no
    /// name at all. An identifier is never broken across lines and never
    /// clipped — that would be a wrong name, not a smaller one.
    #[test]
    fn a_landmark_fits_the_frame_it_names_or_is_not_drawn() {
        let boxes = [
            // A ground frame the width of a sheet: capped, not unbounded.
            (2400.0, 900.0),
            (900.0, 400.0),
            (520.0, 180.0),
            // Too small to carry the far ramp's floor: no landmark.
            (300.0, 120.0),
            (160.0, 18.0),
        ];
        for name in [
            "main",
            "survey",
            "build",
            "an_unusually_long_entry_point_name",
        ] {
            for (w, h) in boxes {
                let at = Placed {
                    x: 10.0,
                    y: 20.0,
                    w,
                    h,
                };
                let Some(mark) = NameView::measure(7, name, at, LANDMARK_MAX) else {
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
                    "{name} in {w}x{h}: the name runs {drawn} past the frame"
                );
                assert!(
                    mark.y <= at.y + at.h,
                    "{name} in {w}x{h}: the baseline is below the frame"
                );
                assert!(mark.y > at.y + HEAD_H, "a landmark stands under the head");
            }
        }
        // The floor holds: a leaf-sized box is never engraved.
        assert!(
            NameView::measure(
                0,
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
        // And the ceiling holds: a frame standing inside a named frame is a step
        // smaller however wide its own box is, and below the floor it declines
        // the name rather than repeating its parent's size.
        let huge = Placed {
            x: 0.0,
            y: 0.0,
            w: 4000.0,
            h: 2000.0,
        };
        let outer = NameView::measure(0, "AppShell", huge, LANDMARK_MAX).expect("a landmark");
        assert_eq!(outer.size, LANDMARK_MAX);
        let inner = NameView::measure(1, "SurveyGate", huge, outer.size - LANDMARK_STEP)
            .expect("a landmark");
        assert!(
            inner.size <= outer.size - LANDMARK_STEP,
            "the ladder of sizes is the nesting"
        );
        assert!(
            NameView::measure(2, "deep", huge, LANDMARK_MIN - 1.0).is_none(),
            "past the floor the register goes quiet instead of repeating itself"
        );
    }

    /// One entry frame holding two callees and one leaf on the ground, with a
    /// third declaration shelved inside the first callee — enough nesting for a
    /// fold to have both siblings and an ancestor to disturb.
    fn nested_model() -> FnModel {
        let call = |def: u32, user: u32, seats: bool| crate::views::func::model::Call {
            def,
            user,
            kind: CallKind::Call,
            count: 2,
            seats,
        };
        let mut marks: Vec<FnMark> = (0..5)
            .map(|id| mark(id, &format!("fn{id}"), &[], false))
            .collect();
        for (id, runs) in [(0u32, 3u32), (1, 1)] {
            marks[id as usize].runs = runs;
        }
        FnModel {
            marks,
            calls: vec![
                call(1, 0, true),
                call(2, 0, true),
                call(3, 1, true),
                call(3, 2, false),
            ],
            via: HashMap::from([(1, 0), (2, 0), (3, 1)]),
            kids: HashMap::from([(0, vec![1, 2]), (1, vec![3])]),
            seats: vec![0, 4],
            ..Default::default()
        }
    }

    /// **Folding draws less; it does not move anything.** The end-to-end
    /// invariant, read off the drawing rather than the layout: every node that
    /// is still on the paper keeps its exact box, the folded frame keeps its
    /// whole footprint (so its boundary is still the territory it reserved),
    /// what it hides is off the paper, and opening it again gives back the
    /// drawing that was there before — pixel for pixel.
    #[test]
    fn a_fold_draws_less_and_moves_nothing() {
        let open = FnDrawing::build(&nested_model());
        let mut shut = nested_model();
        shut.folded = HashSet::from([1]);
        shut.packs = HashMap::from([(3, 1)]);
        let folded = FnDrawing::build(&shut);

        let boxes = |d: &FnDrawing| -> Vec<(u32, i64, i64, i64, i64)> {
            let mut out: Vec<(u32, i64, i64, i64, i64)> = d
                .rects
                .iter()
                .map(|(&id, at)| (id, at.x as i64, at.y as i64, at.w as i64, at.h as i64))
                .collect();
            out.sort_unstable();
            out
        };
        // Every box still on the paper is identical — the folded frame's own
        // included, because the layout reserved the whole footprint. `rects`
        // holds the drawn marks, so the elided one is simply absent from it.
        let common: Vec<(u32, i64, i64, i64, i64)> = boxes(&open)
            .into_iter()
            .filter(|b| folded.rects.contains_key(&b.0))
            .collect();
        assert_eq!(boxes(&folded), common, "the fold re-laid the paper");
        assert_eq!(common.len(), 4, "one mark was elided and no other");

        // What the fold hides is off the *drawing*, and only that.
        let drawn = |d: &FnDrawing| -> Vec<u32> {
            let mut ids: Vec<u32> = d.nodes.iter().map(|n| n.data.0.id).collect();
            ids.sort_unstable();
            ids
        };
        assert_eq!(drawn(&open), vec![0, 1, 2, 3, 4]);
        assert_eq!(drawn(&folded), vec![0, 1, 2, 4], "3 is elided, not moved");

        // And the frame that folded still owns the whole box it reserved, so its
        // boundary is still the territory a click takes.
        let ring = |d: &FnDrawing| d.edges.iter().find(|e| e.id == 1).map(|e| e.at);
        assert_eq!(ring(&folded), ring(&open));

        // Opening it again is the drawing that was there before.
        shut.folded.clear();
        shut.packs.clear();
        let back = FnDrawing::build(&shut);
        assert_eq!(boxes(&back), boxes(&open));
        assert_eq!(drawn(&back), drawn(&open));
    }

    /// The register never piles: one chain of frames nested along one corner
    /// gets a descending ladder of names with clear paper between them, not one
    /// name per frame over one square of paper.
    #[test]
    fn a_chain_of_frames_gets_a_ladder_and_not_a_pile() {
        // Six frames, each seated inside the last and each also holding a shelf
        // of plain callees — the shape `AppShell` → `SurveyGate` → `DataShell`
        // really has, where every frame is nearly as wide as the one it stands
        // in and the whole chain runs down one corner.
        let (deep, wide) = (6u32, 40u32);
        let mut marks = Vec::new();
        let (mut via, mut kids) = (HashMap::new(), HashMap::<u32, Vec<u32>>::new());
        let mut next = deep;
        for at in 0..deep {
            let mut frame = mark(at, &format!("Frame{at}"), &[], false);
            frame.runs = wide;
            marks.push(frame);
            let mut shelf: Vec<u32> = Vec::new();
            if at + 1 < deep {
                shelf.push(at + 1);
                via.insert(at + 1, at);
            }
            for _ in 0..wide {
                marks.push(mark(next, &format!("leaf{next}"), &[], false));
                via.insert(next, at);
                shelf.push(next);
                next += 1;
            }
            kids.insert(at, shelf);
        }
        let model = FnModel {
            marks,
            seats: vec![0],
            via,
            kids,
            ..Default::default()
        };
        let drawing = FnDrawing::build(&model);
        assert!(
            drawing.names.len() >= 2,
            "the register named nothing at all: nothing to check"
        );
        assert!(
            drawing.names.len() < deep as usize,
            "every frame in the chain named itself: {} of {deep}",
            drawing.names.len()
        );
        // What names it does draw descend, and no two of them share paper.
        let mut ladder: Vec<&NameView> = drawing.names.iter().collect();
        ladder.sort_by(|a, b| a.y.total_cmp(&b.y));
        for pair in ladder.windows(2) {
            let (over, under) = (pair[0], pair[1]);
            assert!(
                under.size <= over.size - LANDMARK_STEP,
                "{} at {} does not step down from {} at {}",
                under.name,
                under.size,
                over.name,
                over.size
            );
            assert!(
                under.y - under.size * 0.78 >= over.foot(),
                "{} sits in {}'s ink",
                under.name,
                over.name
            );
        }
    }

    /// One entry frame holding two callees, one of which holds a third, and one
    /// call from the second callee into the third — a call that crosses the
    /// boundary of the frame in the middle.
    fn boundary_model() -> FnModel {
        let call = |def: u32, user: u32, seats: bool| crate::views::func::model::Call {
            def,
            user,
            kind: CallKind::Call,
            count: 3,
            seats,
        };
        FnModel {
            marks: (0..4)
                .map(|id| mark(id, &format!("fn{id}"), &[], false))
                .collect(),
            calls: vec![
                call(1, 0, true),
                call(2, 0, true),
                call(3, 1, true),
                // The crossing: `fn2`, outside the boundary, calls `fn3` inside.
                call(3, 2, false),
            ],
            via: HashMap::from([(1, 0), (2, 0), (3, 1)]),
            kids: HashMap::from([(0, vec![1, 2]), (1, vec![3])]),
            seats: vec![0],
            ..Default::default()
        }
    }

    /// A boundary is the box read as what it is — a subtree. Everything inside
    /// keeps full ink, everything one call across the line reads a step behind
    /// (the frames it stands inside included, because they are the paper it is
    /// drawn on), and every crossing wire is the reading's own ink.
    #[test]
    fn a_boundary_selection_reads_the_whole_subtree_and_its_crossings() {
        let model = boundary_model();
        let kin = FnKin::tree(1, &model, FnWires::Both);
        assert_eq!(kin.tree, Some(1));
        assert_eq!(kin.sel, None, "a boundary is not the mark's own focus");
        assert_eq!(kin.lit, HashSet::from([1, 3]), "the box is the subtree");
        // The crossing's far end, and the frame the box stands inside.
        assert_eq!(kin.near, HashSet::from([2, 0]));
        assert_eq!(kin.wires, HashSet::from([(3, 2)]));
        // Read on the paper: inside is full ink, the crossings a step behind,
        // strangers receded — and the box itself is not the picked mark.
        assert_eq!(kin.block_class(3), "is-kin");
        assert_eq!(kin.block_class(1), "is-kin");
        assert_eq!(kin.block_class(2), "is-near");
        assert_eq!(kin.block_class(0), "is-near");
    }

    /// A fold re-anchors ink instead of cutting it: a wire whose far end a fold
    /// hides is read on the head that stands for it, and a fold that swallows
    /// both ends of a wire draws no line from a head to itself.
    #[test]
    fn a_fold_re_anchors_the_ink_it_swallows() {
        let mut model = boundary_model();
        // `fn1` is folded, so `fn3` is off the paper and `fn1` stands for it.
        model.folded = HashSet::from([1]);
        model.packs = HashMap::from([(3, 1)]);
        assert_eq!(model.shown(3), 1);
        assert!(model.hidden(3));

        // The crossing call now reads from `fn1`'s own head.
        let kin = FnKin::tree(1, &model, FnWires::Both);
        assert_eq!(kin.wires, HashSet::from([(1, 2)]));
        // And the folded frame carries the ink of what it hides: `fn3` is in
        // the chain, so the head standing for it reads at full pressure.
        assert!(kin.lit.contains(&1));

        // A selection inside the fold is read on the same head, and its own
        // wires re-anchor the same way.
        let inside = FnKin::mark(3, &model, FnWires::Both);
        assert!(inside.lit.contains(&1), "the fold stands for what it hides");
        assert_eq!(
            inside.wires,
            HashSet::from([(1, 2)]),
            "the wire ties to the head on the paper, not to a hidden box"
        );

        // Fold the frame above it too and both ends of the crossing are inside
        // one head: there is no line left to draw.
        model.folded = HashSet::from([0]);
        model.packs = HashMap::from([(1, 0), (2, 0), (3, 0)]);
        let whole = FnKin::tree(0, &model, FnWires::Both);
        assert!(whole.wires.is_empty(), "a head names nothing to itself");
    }

    /// **A wire ties to the edge of a head row, never through it — and ends
    /// that share one edge fan across it.**
    ///
    /// The head's own centre was the tie once, which is why the lines read as
    /// crossing the boxes: every wire had to cut half a head row of quoted
    /// source to reach the point it ended on, and a head six callers reach took
    /// six arrowheads on one pixel. Both facts are geometry, so both are pinned
    /// here.
    #[test]
    fn a_wire_ties_on_the_edge_of_a_head_and_fans_where_it_shares_one() {
        // Six declarations on the ground, all called by a seventh — the shape
        // that piled six arrowheads on one head.
        let marks: Vec<FnMark> = (0..7)
            .map(|id| mark(id, &format!("fn{id}"), &[], false))
            .collect();
        let model = FnModel {
            marks,
            calls: (1..7)
                .map(|def| crate::views::func::model::Call {
                    def,
                    user: 0,
                    kind: CallKind::Call,
                    count: 1,
                    seats: false,
                })
                .collect(),
            seats: (0..7).collect(),
            ..Default::default()
        };
        let drawing = FnDrawing::build(&model);
        assert_eq!(drawing.wires.len(), 6, "one line per call");

        let mut shared: HashMap<(u32, TieSide), Vec<(i64, i64)>> = HashMap::new();
        for w in &drawing.wires {
            for (id, at, side) in [(w.def, w.from, w.from_side), (w.user, w.to, w.to_side)] {
                let box_of = drawing.rects[&id];
                // Every one of these blocks is a leaf, so its whole box is its
                // own band: the tie has to sit on that box's boundary.
                let on_edge = match side {
                    TieSide::Top => (at.y - box_of.y).abs() < 0.01,
                    TieSide::Under => (at.y - (box_of.y + box_of.h)).abs() < 0.01,
                    TieSide::Left => (at.x - box_of.x).abs() < 0.01,
                    TieSide::Right => (at.x - (box_of.x + box_of.w)).abs() < 0.01,
                };
                assert!(on_edge, "an end sits on the edge it faces, not inside");
                assert!(
                    at.x >= box_of.x - 0.01 && at.x <= box_of.x + box_of.w + 0.01,
                    "and on the box, never off it"
                );
                assert!(
                    at.y >= box_of.y - 0.01 && at.y <= box_of.y + box_of.h + 0.01,
                    "and never inside the quotation"
                );
                shared
                    .entry((id, side))
                    .or_default()
                    .push(((at.x * 100.0) as i64, (at.y * 100.0) as i64));
            }
        }
        // Every end that shares an edge with another stands on its own point.
        let mut fanned = 0;
        for ((_, _), mut points) in shared {
            let count = points.len();
            points.sort_unstable();
            points.dedup();
            assert_eq!(points.len(), count, "two ends stacked on one point");
            fanned += (count > 1) as usize;
        }
        assert!(fanned > 0, "the shape under test shares no edge at all");
    }

    /// One drawn wire, with the diff's word about each of its ends.
    fn wire(def_dirty: bool, user_dirty: bool, contract: bool) -> WireView {
        WireView {
            key: "w".to_string(),
            from: Point::new(0.0, 0.0),
            to: Point::new(1.0, 1.0),
            def: 0,
            user: 1,
            count: 1,
            label: None,
            width: 1.0,
            def_dirty,
            user_dirty,
            from_side: TieSide::Right,
            to_side: TieSide::Left,
            class: match contract {
                true => "is-answers",
                false => "is-call",
            },
        }
    }

    /// Every stop of the `wires` reading draws a different set **around an
    /// anchor**, and the anchor is whatever is in focus. The stops are
    /// directions now, not amounts of ink: a reader who moves this switch is
    /// asking a question about one mark's code, and each answer has to look
    /// different from the others.
    #[test]
    fn every_stop_of_the_wires_reading_changes_the_picture() {
        // → drawn under calls / callers / both
        let seen = |w: &WireView, picked: bool, dirty: bool, lit: bool| {
            FnWires::ALL_STOPS
                .iter()
                .map(|&stop| drawn_under(stop, w, picked, dirty, lit))
                .collect::<Vec<bool>>()
        };
        // The resting plate of a dirty sheet: the diff is the anchor. A wire
        // leaving a changed declaration is what `calls` is about; one arriving
        // at a changed declaration is what `callers` is about; `both` takes
        // every wire, because with a whole diff in focus there is no thinning
        // left to do that a direction would explain.
        let out = wire(false, true, false);
        assert_eq!(seen(&out, false, true, false), vec![true, false, true]);
        let into = wire(true, false, false);
        assert_eq!(seen(&into, false, true, false), vec![false, true, true]);
        // A wire the diff touched at neither end: only `both` admits it.
        let stranger = wire(false, false, false);
        assert_eq!(
            seen(&stranger, false, true, false),
            vec![false, false, true]
        );

        // A clean sheet has no anchor at all, so every stop draws everything —
        // a reading with nothing in focus has no direction to take. This is the
        // data chart's own rule for its `references` reading.
        assert_eq!(seen(&stranger, false, false, false), vec![true, true, true]);

        // With a selection the sheet answers about the selection: its own wires
        // (already read in the chosen direction by `FnKin`) and nothing else.
        assert_eq!(seen(&stranger, true, true, true), vec![true, true, true]);
        assert_eq!(seen(&out, true, true, false), vec![false, false, false]);

        // A contract never folds, in any direction, anchored or not: it is what
        // makes the shelved section honest about a `dyn` call.
        let contract = wire(false, false, true);
        assert_eq!(seen(&contract, true, true, false), vec![true, true, true]);
        assert_eq!(seen(&contract, false, true, false), vec![true, true, true]);
    }

    /// A selection reads its own wires **in the chosen direction**: the same
    /// mark, three different sets. This is the assertion the data chart's
    /// ancestor of this control shipped without, and why it shipped with three
    /// stops that looked identical.
    #[test]
    fn a_selection_reads_its_wires_in_the_chosen_direction() {
        let mut model = boundary_model();
        // `fn2` (outside) calls `fn3` (inside `fn1`), so `fn3` has one caller
        // and no calls of its own, and `fn2` the reverse.
        model.calls.push(crate::views::func::model::Call {
            def: 2,
            user: 3,
            kind: CallKind::Call,
            count: 1,
            seats: false,
        });
        let of = |id: u32, reading: FnWires| FnKin::mark(id, &model, reading).wires;
        assert_eq!(of(3, FnWires::Calls), HashSet::from([(2, 3)]));
        assert_eq!(of(3, FnWires::Callers), HashSet::from([(3, 2)]));
        assert_eq!(
            of(3, FnWires::Both),
            HashSet::from([(2, 3), (3, 2)]),
            "both ways round is the union, never a fourth answer"
        );

        // And a boundary selection reads the same three ways, with the box
        // standing where the mark stood: what the code inside runs beyond the
        // line, or whose code beyond the line runs something inside it.
        let box_of = |reading: FnWires| FnKin::tree(1, &model, reading).wires;
        assert_eq!(box_of(FnWires::Calls), HashSet::from([(2, 3)]));
        assert_eq!(box_of(FnWires::Callers), HashSet::from([(3, 2)]));
        assert_eq!(box_of(FnWires::Both), HashSet::from([(2, 3), (3, 2)]));
    }

    /// Every block is wide enough for the longest line it draws, and none is
    /// narrower than a mark a pointer can find or wider than a quotation reads.
    #[test]
    fn a_block_is_measured_to_hold_its_longest_line() {
        let short = MeasuredBlock::measure(&mark(0, "at", &[], false));
        assert_eq!(short.own.0, MARK_MIN_W);
        let long = MeasuredBlock::measure(&mark(
            1,
            "a_declaration_with_a_very_long_name_indeed",
            &[],
            false,
        ));
        assert!(long.own.0 > short.own.0);
        // A row longer than the block may be widens it to the cap and no
        // further: past that a quoted line ellipsizes and hover carries it.
        let mut wide = mark(2, "read", &[], false);
        wide.rows = vec![row(
            "reading",
            "&HashMap<u32, Vec<crate::views::func::model::Touch>>",
        )];
        let wide = MeasuredBlock::measure(&wide);
        assert!(wide.own.0 > short.own.0, "the row sets the width");
        assert!(wide.own.0 <= MARK_MAX_W);
    }
}
