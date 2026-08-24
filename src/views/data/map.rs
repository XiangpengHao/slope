//! The data chart: state blocks nested by ownership, inside module frames.
//!
//! One block per shape the workspace keeps — struct, enum, union — and per
//! static. **Top-level data stands at module level and wears the root's 2.5px
//! ink left edge**; secondary data is drawn *inside* the block of the type
//! that owns it hardest, the way module frames nest, so the tier is read off
//! the paper's own nesting before any words are. A block quotes its whole
//! declaration — fields, variants, a static's type — and contains the blocks
//! it owns under a hairline rule. No methods: what a type promises is the
//! surface chart's ink, one rung up.
//!
//! Two families run between the blocks, the surface chart's grammar exactly:
//! solid holding lines with the wrapper's word (`Arc`, `&`) for what the
//! nesting cannot say, and dashed counted uses edges for one type's impls
//! leaning on another. Both rest on the dependent. What has no block here —
//! a function naming or using state — is counted on the mark it touches,
//! `named by n signatures · used by n bodies`, never silently cut; the
//! sheet spends those counts back out as rows, one per item, each linking to
//! the code the paper had no block for.

use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size, Viewport,
};

use crate::Route;
use crate::api::{CodeGraph, HoldEvent, HoldKind, ItemKind};
use crate::views::codemap::chrome::{decl_words, plural};
use crate::views::codemap::map::{narrow_viewport, prefers_reduced_motion, tie_ends, window_size};
use crate::views::codemap::tree::{Placed, text_w};
use crate::views::codemap::use_code;
use crate::views::data::model::{DataMark, DataModel, Tier};
use crate::views::data::{DataSel, mark_route, mod_route, use_data};
use crate::views::surface::layout::{self, Sizes, SurfaceLayout};
use crate::views::surface::map::Quoted;
use crate::views::surface::model::{Anchor, FieldRow, Held, brackets, upstream};

// ---------------------------------------------------------------------------
// Block furniture, in flow units — one unit is one CSS pixel at zoom 1. These
// numbers are the CSS in `tailwind.css`; move one and the other must follow.
// The core rows share the surface block's anatomy and its numbers.
// ---------------------------------------------------------------------------

const PAD_TOP: f64 = 6.0;
const PAD_BOTTOM: f64 = 5.0;
/// Both sides together: border and padding, left and right.
const PAD_X: f64 = 16.0;
/// How far a bracketed row stands in from its block's edge — rust's own
/// indent, narrowed to what a 10px quotation can spare. A row's diff marker
/// sits in this gutter, so `+`, `−` and unmarked rows all start their text on
/// the same column.
const ROW_INDENT: f64 = 12.0;
const HEAD_H: f64 = 16.0;
const ROW_H: f64 = 15.0;
const TY_H: f64 = 14.0;
const FOLDS_TOP: f64 = 6.0;
const FOLD_H: f64 = 12.0;
/// Small-type slack: at 8.5–9px the browser rounds each glyph up, so text
/// measured with the font's exact 0.6em advance clips its last characters.
const META_SLACK: f64 = 1.08;
const MARK_MIN_W: f64 = 152.0;
/// A row's clip width. A block holding kids may grow wider than this — the
/// kids set the box then — but no quoted row ever does.
const MARK_MAX_W: f64 = 300.0;
const ROW_MIN_W: f64 = 132.0;
const ROW_FOLD_H: f64 = 22.0;
const WRAP_SLACK: f64 = 1.12;
/// The kids band: the hairline rule under the rows (margin + border), the
/// clear paper inside it, and the gap between two nested blocks.
const KIDS_RULE: f64 = 5.0;
const KIDS_PAD: f64 = 4.0;
const KID_GAP: f64 = 10.0;
/// How much wider than tall a kids shelf aims to be.
const KID_LANDSCAPE: f64 = 2.2;
/// The left inset of a nested block: the parent's border and padding.
const KID_X: f64 = 8.0;

/// Dashed reference width from its count, as everywhere.
fn tie_width(count: u32) -> f64 {
    (0.9 + (count as f64).ln() * 0.35).min(2.4)
}

fn hold_width(kind: HoldKind) -> f64 {
    match kind {
        HoldKind::Owns => 1.4,
        HoldKind::Shares => 1.3,
        HoldKind::Borrows | HoldKind::Dyn | HoldKind::Implements => 1.1,
    }
}

/// Lines a text needs at `px` in `usable` width, with the browser's own
/// wrapping given some slack.
fn wrapped(text: &str, px: f64, usable: f64) -> f64 {
    (text_w(text, px) * WRAP_SLACK / usable.max(1.0))
        .ceil()
        .max(1.0)
}

/// One block, measured, with the blocks it contains measured inside it.
#[derive(Clone, PartialEq)]
struct MeasuredBlock {
    id: u32,
    decl: String,
    name: String,
    letter: Option<&'static str>,
    ghost: bool,
    is_static: bool,
    is_enum: bool,
    /// What the head opens with and the line that closes the quotation —
    /// `{` / `}` for a shape with rows, nothing for one without.
    open: &'static str,
    close: &'static str,
    /// Wears the root's ink left edge: a chain of holding begins here.
    is_root: bool,
    fields: Vec<FieldRow>,
    variants: Vec<FieldRow>,
    ty: String,
    ty_target: Held,
    /// The blocks nested inside this one, measured, with each one's offset
    /// from the kids shelf's origin.
    kids: Vec<MeasuredBlock>,
    kid_at: Vec<(f64, f64)>,
    /// The shelf's own height, kids and gaps included.
    kids_h: f64,
    /// From the block's top edge to where the kids band begins: the header
    /// and every quoted row above the rule.
    core_h: f64,
    /// The counted line at the foot: a vocabulary mark's folded fan-in.
    folds: Vec<String>,
    /// The undrawn-ink counts, said in the block's hover words.
    counts: String,
    locator: String,
    path: String,
    label: String,
    size: (f64, f64),
}

/// A frame's counted fold row: a module folded by hand.
#[derive(Clone, PartialEq)]
pub(crate) struct FoldView {
    anchor: Anchor,
    words: String,
    title: String,
    /// The module this row stands for; clicking it puts the module back.
    unfolds: Option<Vec<String>>,
    size: (f64, f64),
}

/// One node on the data chart.
#[derive(Clone, PartialEq)]
enum DataNodeData {
    Mark(Box<MeasuredBlock>),
    Fold(FoldView),
}

impl DataNodeData {
    /// The diff touched this node or anything nested in it: it keeps full
    /// pressure while the rest of a dirty chart recedes.
    fn touched(&self) -> bool {
        fn any(view: &MeasuredBlock) -> bool {
            view.letter.is_some() || view.kids.iter().any(any)
        }
        match self {
            DataNodeData::Mark(m) => any(m),
            DataNodeData::Fold(_) => false,
        }
    }
}

/// A frame, placed, with its label and the two gestures its border answers.
#[derive(Clone, PartialEq)]
struct FrameView {
    id: u32,
    parent: Option<u32>,
    at: Placed,
    label: Option<String>,
    label_w: f64,
    key: Vec<String>,
    words: String,
    folded: bool,
}

/// One drawn edge with its ends already found.
#[derive(Clone, PartialEq)]
struct WireView {
    key: String,
    from: Point,
    to: Point,
    a: Anchor,
    b: Anchor,
    label: Option<String>,
    width: f64,
    rest: bool,
    class: &'static str,
    event: &'static str,
    /// How much the wire carries: a tie's reference count, a hold's rows.
    /// What a boundary bundle sums.
    weight: u32,
    /// One line standing for a module reading's crossing family: full ink by
    /// construction, and outside the hover/kin machinery.
    bundle: bool,
}

/// One drawing of the data chart: the blocks, frames and wires a single
/// build puts on the paper, plus the indexes a reading walks — pairs, homes,
/// rects, locators.
#[derive(Clone, PartialEq)]
struct DataDrawing {
    nodes: Vec<FlowNode<DataNodeData>>,
    frames: Vec<FrameView>,
    holds: Vec<WireView>,
    ties: Vec<WireView>,
    /// Every structural pair (held, holder), the nesting included: the
    /// selection's blast radius walks these, drawn line or seated block.
    pairs: Vec<(Anchor, Anchor)>,
    /// Which frame every drawn anchor sits in — nested marks included, so a
    /// module reading keeps the state drawn inside its blocks.
    homes: HashMap<Anchor, u32>,
    /// Every drawn anchor's box, nested marks included — what the camera
    /// glides to when a selection is off the glass or below reading zoom.
    rects: HashMap<Anchor, Placed>,
    /// The URL's (path, item) key for every drawn mark.
    locate: HashMap<(String, String), Anchor>,
    frame: Option<Rect>,
    dirty: bool,
}

/// A module reading's own ink, as on the surface chart.
#[derive(Clone, PartialEq)]
struct ModHome {
    frame: u32,
    kept: std::collections::HashSet<u32>,
    inside: std::collections::HashSet<Anchor>,
}

/// The selection's ink: the chosen mark, its blast radius up the holding
/// order (nesting included), what it directly holds, and its uses
/// neighbours. Same reading as the surface chart's, over this chart's pairs.
#[derive(Clone, PartialEq)]
struct DataKin {
    sel: Option<Anchor>,
    home: Option<ModHome>,
    up: std::collections::HashSet<Anchor>,
    down: std::collections::HashSet<Anchor>,
    near: std::collections::HashSet<Anchor>,
}

impl DataKin {
    fn read(sel: Anchor, holds: &[(Anchor, Anchor)], ties: &[(Anchor, Anchor)]) -> Self {
        Self {
            sel: Some(sel),
            home: None,
            up: upstream(holds, sel),
            down: holds
                .iter()
                .filter(|(_, holder)| *holder == sel)
                .map(|(held, _)| *held)
                .collect(),
            near: ties
                .iter()
                .filter_map(|&(def, user)| match (def == sel, user == sel) {
                    (true, false) => Some(user),
                    (false, true) => Some(def),
                    _ => None,
                })
                .collect(),
        }
    }

    fn read_mod(
        frame: u32,
        frames: &[FrameView],
        homes: &HashMap<Anchor, u32>,
        holds: &[(Anchor, Anchor)],
        ties: &[(Anchor, Anchor)],
    ) -> Self {
        let parent: HashMap<u32, Option<u32>> = frames.iter().map(|f| (f.id, f.parent)).collect();
        let climb = |from: u32| -> Vec<u32> {
            let mut line = Vec::new();
            let mut at = parent.get(&from).copied().flatten();
            while let Some(id) = at {
                line.push(id);
                at = parent.get(&id).copied().flatten();
            }
            line
        };
        let mut kept: std::collections::HashSet<u32> = frames
            .iter()
            .filter(|f| f.id == frame || climb(f.id).contains(&frame))
            .map(|f| f.id)
            .collect();
        let inside: std::collections::HashSet<Anchor> = homes
            .iter()
            .filter(|(_, home)| kept.contains(home))
            .map(|(anchor, _)| *anchor)
            .collect();
        kept.extend(climb(frame));
        let near = holds
            .iter()
            .chain(ties)
            .filter_map(|&(a, b)| match (inside.contains(&a), inside.contains(&b)) {
                (true, false) => Some(b),
                (false, true) => Some(a),
                _ => None,
            })
            .collect();
        Self {
            sel: None,
            home: Some(ModHome {
                frame,
                kept,
                inside,
            }),
            up: std::collections::HashSet::new(),
            down: std::collections::HashSet::new(),
            near,
        }
    }

    /// The module reading, when this is one: the chosen frame and everything
    /// seated inside its boundary.
    fn mod_home(&self) -> Option<(u32, &std::collections::HashSet<Anchor>)> {
        self.home.as_ref().map(|h| (h.frame, &h.inside))
    }

    fn frame_class(&self, id: u32) -> &'static str {
        match &self.home {
            Some(home) if home.frame == id => "is-sel",
            Some(home) if !home.kept.contains(&id) => "is-dim",
            _ => "",
        }
    }

    /// A block's own pressure under this reading. The classes act on the
    /// block's *own paint* — never on the box — because a receded holder can
    /// have a lit block nested inside it, and opacity on the box would take
    /// the kid down with it.
    fn block_class(&self, a: Anchor) -> &'static str {
        if let Some(home) = &self.home {
            return if home.inside.contains(&a) {
                ""
            } else if self.near.contains(&a) {
                "is-near-own"
            } else {
                "is-dim-own"
            };
        }
        if Some(a) == self.sel {
            "is-picked"
        } else if self.up.contains(&a) || self.down.contains(&a) {
            ""
        } else if self.near.contains(&a) {
            "is-near-own"
        } else {
            "is-dim-own"
        }
    }

    fn wire_kin(&self, held: Anchor, holder: Anchor) -> bool {
        if let Some(home) = &self.home {
            return home.inside.contains(&held) || home.inside.contains(&holder);
        }
        let upward = |x: Anchor| Some(x) == self.sel || self.up.contains(&x);
        (upward(held) && upward(holder)) || (Some(holder) == self.sel && self.down.contains(&held))
    }

    fn tie_near(&self, a: Anchor, b: Anchor) -> bool {
        if let Some(home) = &self.home {
            return home.inside.contains(&a) || home.inside.contains(&b);
        }
        Some(a) == self.sel || Some(b) == self.sel
    }
}

/// The counted words a block writes at its foot: only the genuine fold — a
/// vocabulary mark's fan-in, whose lines rest folded. The naming and body-use
/// counts moved off the resting paper (2026-08-21, critique): stamped on
/// every block they were texture, not signal — 40 blocks saying `named by 1
/// signature` — and they were never elided ink, only the sheet's facts said
/// early. They live in the block's hover words and on the sheet.
fn fold_words(mark: &DataMark) -> Vec<String> {
    let mut folds = Vec::new();
    if mark.held_by > 0 {
        folds.push(format!("held by {}", plural(mark.held_by as usize, "type")));
    }
    folds
}

/// The undrawn-ink counts, for the block's hover words.
fn count_words(mark: &DataMark) -> String {
    let mut parts = Vec::new();
    if mark.named_by > 0 {
        parts.push(format!(
            "named by {}",
            plural(mark.named_by as usize, "signature")
        ));
    }
    // Bodies, not references: the sheet lists them one row each, so the
    // hover word counts the same things the rows do.
    if !mark.used_by.is_empty() {
        parts.push(match mark.used_by.len() {
            1 => "used by 1 body".to_string(),
            n => format!("used by {n} bodies"),
        });
    }
    match parts.is_empty() {
        true => String::new(),
        false => format!(" · {}", parts.join(" · ")),
    }
}

/// Seat measured kids on a skyline aiming at a landscape band: offsets from
/// the shelf origin, and the shelf's own size. Deterministic — the kids
/// arrive in the survey's order and are seated in it; a short block fills
/// the slack beside a tall one instead of opening a new row under it.
fn shelve_kids(kids: &[MeasuredBlock]) -> (Vec<(f64, f64)>, f64, f64) {
    let widest = kids.iter().map(|k| k.size.0).fold(0.0, f64::max);
    let area: f64 = kids
        .iter()
        .map(|k| (k.size.0 + KID_GAP) * (k.size.1 + KID_GAP))
        .sum();
    let target = widest.max((area * KID_LANDSCAPE).sqrt());
    let boxes: Vec<(f64, f64)> = kids.iter().map(|k| k.size).collect();
    let at = layout::skyline(&boxes, target, KID_GAP);
    let (mut w, mut h) = (0.0f64, 0.0f64);
    for (kid, &(x, y)) in kids.iter().zip(&at) {
        w = w.max(x + kid.size.0);
        h = h.max(y + kid.size.1);
    }
    (at, w, h)
}

impl MeasuredBlock {
    /// Measure one block around its already-measured kids. The width is the
    /// widest line it must not clip — or the kids shelf, where the state it
    /// contains is wider than its own words — and the height is every line and
    /// every nested block it draws.
    fn measure(mark: &DataMark, kids: Vec<Self>) -> Self {
        let decl = decl_words(mark.vis, mark.kind);
        let head = format!("{decl} {}", mark.name);
        let locator = mark.locator();
        let letter = mark.letter();
        let folds = fold_words(mark);

        // What the brackets have to enclose: a shape's rows, or — for a
        // static — the one line that declares its type.
        let body_rows = match mark.kind {
            ItemKind::Static | ItemKind::Const | ItemKind::TypeAlias => {
                usize::from(!mark.ty.is_empty())
            }
            _ => mark.fields.len() + mark.variants.len(),
        };
        let (open, close) = brackets(mark.kind, body_rows);
        let mut widest = text_w(&head, 10.5)
            + if letter.is_some() { 12.0 } else { 0.0 }
            + if open.is_empty() {
                0.0
            } else {
                text_w(open, 10.5) + 4.0
            };
        let wrapping = MARK_MAX_W - PAD_X;
        for row in &mark.fields {
            widest = widest.max((text_w(&row.written(), 10.0) + ROW_INDENT).min(wrapping));
        }
        for row in &mark.variants {
            widest = widest.max((text_w(&row.decl, 10.0) + ROW_INDENT).min(wrapping));
        }
        for fold in &folds {
            // Browsers round each glyph up at this size; measured with the
            // font's exact advance the last characters clip. Carry slack.
            widest = widest.max(text_w(fold, 9.0) * META_SLACK);
        }
        if !mark.ty.is_empty() {
            widest = widest.max((text_w(&mark.ty, 9.5) + ROW_INDENT).min(wrapping));
        }
        let core_w = (widest + PAD_X).clamp(MARK_MIN_W, MARK_MAX_W);

        let (kid_at, kids_w, kids_h) = shelve_kids(&kids);
        let w = core_w.max(if kids.is_empty() { 0.0 } else { kids_w + PAD_X });
        let usable = w - PAD_X;

        let ty_lines = if mark.ty.is_empty() {
            0.0
        } else {
            wrapped(&mark.ty, 9.5, usable - ROW_INDENT)
        };
        let core_h = PAD_TOP
            + HEAD_H
            + ty_lines * TY_H
            + mark.fields.len() as f64 * ROW_H
            + mark.variants.len() as f64 * ROW_H
            + if close.is_empty() { 0.0 } else { ROW_H };
        let kids_band = if kids.is_empty() {
            0.0
        } else {
            KIDS_RULE + KIDS_PAD + kids_h + KIDS_PAD
        };
        let fold_block = if folds.is_empty() {
            0.0
        } else {
            FOLDS_TOP + folds.len() as f64 * FOLD_H
        };
        let h = core_h + kids_band + fold_block + PAD_BOTTOM;

        MeasuredBlock {
            id: mark.id,
            decl,
            name: mark.name.clone(),
            letter,
            ghost: mark.ghost,
            is_static: mark.is_static(),
            is_enum: mark.kind == ItemKind::Enum,
            is_root: mark.is_root(),
            open,
            close,
            fields: mark.fields.clone(),
            variants: mark.variants.clone(),
            ty: mark.ty.clone(),
            ty_target: mark.ty_target.clone(),
            kids,
            kid_at,
            kids_h,
            core_h,
            folds,
            counts: count_words(mark),
            locator,
            path: mark.path.clone(),
            label: mark.label.clone(),
            size: (w, h),
        }
    }
}

impl FoldView {
    fn row(anchor: Anchor, words: String, title: String) -> Self {
        let w = (text_w(&words, 9.5) + 20.0).clamp(ROW_MIN_W, MARK_MAX_W);
        FoldView {
            anchor,
            words,
            title,
            unfolds: None,
            size: (w, ROW_FOLD_H),
        }
    }
}

fn node_key(anchor: Anchor) -> String {
    match anchor {
        Anchor::Mark(id) => format!("m{id}"),
        Anchor::Private(frame) => format!("p{frame}"),
        Anchor::Mod(frame) => format!("f{frame}"),
    }
}

fn hold_class(kind: HoldKind) -> &'static str {
    match kind {
        HoldKind::Owns => "is-owns",
        HoldKind::Shares => "is-shares",
        HoldKind::Borrows => "is-borrows",
        HoldKind::Dyn => "is-dyn",
        HoldKind::Implements => "is-impl",
    }
}

/// A view and every view nested in it, each with its absolute rect: the box
/// the wires land on, parent and kid alike.
fn abs_rects(view: &MeasuredBlock, x: f64, y: f64, out: &mut HashMap<Anchor, Placed>) {
    out.insert(
        Anchor::Mark(view.id),
        Placed {
            x,
            y,
            w: view.size.0,
            h: view.size.1,
        },
    );
    let band = y + view.core_h + KIDS_RULE + KIDS_PAD;
    for (kid, (dx, dy)) in view.kids.iter().zip(&view.kid_at) {
        abs_rects(kid, x + KID_X + dx, band + dy, out);
    }
}

fn frames_of(view: &MeasuredBlock, frame: u32, out: &mut HashMap<Anchor, u32>) {
    out.insert(Anchor::Mark(view.id), frame);
    for kid in &view.kids {
        frames_of(kid, frame, out);
    }
}

fn keys_of(view: &MeasuredBlock, out: &mut HashMap<(String, String), Anchor>) {
    out.insert(
        (view.path.clone(), view.label.clone()),
        Anchor::Mark(view.id),
    );
    for kid in &view.kids {
        keys_of(kid, out);
    }
}

impl From<&DataModel> for DataDrawing {
    /// Measure every block around its kids, place them, and gather what the
    /// chart draws.
    fn from(model: &DataModel) -> Self {
        let by_id: HashMap<u32, &DataMark> = model.marks.iter().map(|m| (m.id, m)).collect();
        // Post-order: a block is measured around its kids, so the kids go first.
        fn measured(id: u32, by_id: &HashMap<u32, &DataMark>) -> Option<MeasuredBlock> {
            let mark = by_id.get(&id)?;
            let kids = mark
                .kids
                .iter()
                .filter_map(|&kid| measured(kid, by_id))
                .collect();
            Some(MeasuredBlock::measure(mark, kids))
        }

        let mut sizes = Sizes::default();
        let mut views: HashMap<u32, MeasuredBlock> = HashMap::new();
        for mark in &model.marks {
            // Only the blocks the frames shelve directly; nested ones are inside.
            if matches!(mark.tier, Tier::Nested(_)) && !mark.ghost {
                continue;
            }
            if let Some(view) = measured(mark.id, &by_id) {
                sizes.marks.insert(mark.id, view.size);
                views.insert(mark.id, view);
            }
        }
        let mut rows: HashMap<Anchor, FoldView> = HashMap::new();
        for frame in &model.frames {
            if frame.folded {
                let anchor = Anchor::Mod(frame.id);
                let words = match frame.packed {
                    0 => "folded".to_string(),
                    n => format!("+ {}", plural(n as usize, "item")),
                };
                let mut row = FoldView::row(
                    anchor,
                    words,
                    format!(
                        "{} is folded to this row — every datum inside it, and inside the \
                         modules nested in it; click to unfold",
                        frame.words()
                    ),
                );
                row.unfolds = Some(frame.key());
                sizes.rows.insert(anchor, row.size);
                rows.insert(anchor, row);
            }
            if let Some(label) = frame.label(model.multi_crate) {
                sizes.labels.insert(frame.id, text_w(&label, 12.0) + 18.0);
            }
        }

        let placed: SurfaceLayout = SurfaceLayout::build(&model.frames, &sizes);

        // Every drawn anchor's box — the nested blocks' computed off their
        // parents — and every anchor's frame, for the module reading.
        let mut rects: HashMap<Anchor, Placed> = HashMap::new();
        let mut homes: HashMap<Anchor, u32> = HashMap::new();
        let mut locate: HashMap<(String, String), Anchor> = HashMap::new();
        for (id, view) in &views {
            let Some(at) = placed.marks.get(id) else {
                continue;
            };
            abs_rects(view, at.x, at.y, &mut rects);
            keys_of(view, &mut locate);
            if let Some(mark) = by_id.get(id) {
                frames_of(view, mark.frame, &mut homes);
            }
        }
        for (anchor, at) in &placed.rows {
            rects.insert(*anchor, *at);
            if let Some(frame) = anchor.frame() {
                homes.insert(*anchor, frame);
            }
        }

        let mut nodes: Vec<FlowNode<DataNodeData>> = Vec::new();
        for (id, view) in &views {
            let Some(at) = placed.marks.get(id) else {
                continue;
            };
            nodes.push(
                FlowNode::with_data(
                    node_key(Anchor::Mark(*id)),
                    view.name.clone(),
                    (at.x, at.y),
                    DataNodeData::Mark(Box::new(view.clone())),
                )
                .size(Size::new(at.w, at.h))
                .sides(Side::Left, Side::Right)
                .draggable(false)
                .selectable(false),
            );
        }
        for (anchor, row) in &rows {
            let Some(at) = placed.rows.get(anchor) else {
                continue;
            };
            nodes.push(
                FlowNode::with_data(
                    node_key(*anchor),
                    row.words.clone(),
                    (at.x, at.y),
                    DataNodeData::Fold(row.clone()),
                )
                .size(Size::new(at.w, at.h))
                .sides(Side::Left, Side::Right)
                .draggable(false)
                .selectable(false),
            );
        }
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        let frames: Vec<FrameView> = placed
            .frames
            .iter()
            .map(|(id, at)| {
                let frame = &model.frames[*id as usize];
                let label = frame.label(model.multi_crate);
                FrameView {
                    id: *id,
                    parent: frame.parent,
                    at: *at,
                    label_w: label.as_deref().map_or(0.0, |l| text_w(l, 12.0)),
                    label,
                    key: frame.key(),
                    words: frame.words(),
                    folded: frame.folded,
                }
            })
            .collect();

        // The arrowhead rests on the holder, so the wire runs held → holder.
        let holds: Vec<WireView> = model
            .holds
            .iter()
            .filter_map(|hold| {
                let (a, b) = (
                    rects.get(&hold.held).copied()?,
                    rects.get(&hold.holder).copied()?,
                );
                let (from, to) = tie_ends(a, b);
                let event_word = match hold.event {
                    Some(HoldEvent::Added) => Some("added"),
                    Some(HoldEvent::Removed) => Some("removed"),
                    None => None,
                };
                let label = match event_word {
                    Some(word) if hold.via.is_empty() => Some(word.to_string()),
                    Some(word) => Some(format!("{} · {word}", hold.via)),
                    None => (!hold.via.is_empty()).then(|| hold.via.clone()),
                };
                Some(WireView {
                    key: hold.key(),
                    from,
                    to,
                    a: hold.held,
                    b: hold.holder,
                    label,
                    width: hold_width(hold.kind),
                    rest: hold.rest,
                    class: hold_class(hold.kind),
                    event: match hold.event {
                        Some(HoldEvent::Added) => "is-added",
                        Some(HoldEvent::Removed) => "is-removed",
                        None => "",
                    },
                    weight: hold.fields,
                    bundle: false,
                })
            })
            .collect();

        let ties: Vec<WireView> = model
            .ties
            .iter()
            .filter_map(|tie| {
                let (a, b) = (
                    rects.get(&tie.def).copied()?,
                    rects.get(&tie.user).copied()?,
                );
                let (from, to) = tie_ends(a, b);
                Some(WireView {
                    key: tie.key(),
                    from,
                    to,
                    a: tie.def,
                    b: tie.user,
                    label: tie.labeled.then(|| tie.count.to_string()),
                    width: tie_width(tie.count),
                    rest: tie.rest,
                    class: "is-ref",
                    event: "",
                    weight: tie.count,
                    bundle: false,
                })
            })
            .collect();

        let frame = Rect::bounds(nodes.iter().map(|n| n.rect())).or_else(|| {
            frames
                .first()
                .map(|f| Rect::new(f.at.x, f.at.y, f.at.w, f.at.h))
        });

        DataDrawing {
            nodes,
            frames,
            holds,
            ties,
            pairs: model.pairs.clone(),
            homes,
            rects,
            locate,
            frame,
            dirty: model.marks.iter().any(|m| m.letter().is_some()),
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

/// One block, drawn — and the blocks it contains, drawn inside it by the
/// same component. The block's own paint (header, rows, foot, locator) sits
/// in its own two layers so a reading can recede a holder without taking the
/// lit state nested inside it down too.
#[component]
fn DataPlate(
    view: MeasuredBlock,
    kin: Option<DataKin>,
    hot: Signal<Option<Anchor>>,
    /// The block this one is nested in, for handing the hover back on leave.
    up: Option<u32>,
) -> Element {
    let nav = use_navigator();
    let anchor = Anchor::Mark(view.id);
    let kin_class = kin.as_ref().map_or("", |k| k.block_class(anchor));
    let picked = kin_class == "is-picked";
    let to = if picked {
        Route::DataOverview {}
    } else {
        mark_route(&view.path, &view.label)
    };
    let title = if picked {
        format!(
            "{} {} — selected · click again to deselect",
            view.decl, view.name
        )
    } else {
        // The locator and the undrawn-ink counts live here and on the sheet;
        // stamped on every resting block they were the chart's loudest text.
        format!(
            "{} {} — {}{} · select it",
            view.decl, view.name, view.locator, view.counts
        )
    };
    let push = to.clone();
    let pressed = to.clone();
    let mut hot = hot;
    rsx! {
        div {
            class: "data-mark",
            class: if !kin_class.is_empty() { "{kin_class}" },
            class: if view.is_root { "is-root" },
            class: if view.letter.is_some() { "is-diff" },
            class: if view.ghost { "is-ghost" },
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
            onmouseenter: move |_| hot.set(Some(anchor)),
            onmouseleave: move |_| hot.set(up.map(Anchor::Mark)),
            div { class: "dm-own",
                header { class: "dm-head",
                    span { class: "dm-kw", "{view.decl}" }
                    span {
                        class: "dm-nm",
                        class: if view.is_enum { "is-sum" },
                        "{view.name}"
                    }
                    if let Some(letter) = view.letter {
                        span {
                            class: "dm-chg",
                            title: match letter {
                                "A" => "added since the diff base",
                                "D" => "removed since the diff base — quoted from the base edition",
                                _ => "declaration changed since the diff base",
                            },
                            "{letter}"
                        }
                    }
                    if !view.open.is_empty() {
                        span {
                            class: "dm-open",
                            class: if view.open.starts_with(['(', ':']) { "is-tight" },
                            "{view.open}"
                        }
                    }
                }
                if !view.ty.is_empty() {
                    p { class: "dm-ty",
                        Quoted {
                            text: view.ty.clone(),
                            held: view.ty_target.clone(),
                        }
                    }
                }
                for (i , row) in view.fields.iter().enumerate() {
                    p { key: "{i}", class: "dm-row",
                        class: if !row.state.class().is_empty() { "{row.state.class()}" },
                        if let Some(mk) = row.state.marker() {
                            span { class: "dm-mk", "{mk}" }
                        }
                        // What the field declares for itself, in front of its
                        // name where rust writes it: a block whose own header
                        // says `pub(crate)` can still be holding private state.
                        if let Some(keyword) = row.vis.keyword() {
                            span { class: "tok-kw", "{keyword} " }
                        }
                        span { class: "dm-fname", "{row.name}: " }
                        Quoted { text: row.decl.clone(), held: row.target.clone() }
                    }
                }
                for (i , row) in view.variants.iter().enumerate() {
                    p { key: "v{i}", class: "dm-var",
                        class: if !row.state.class().is_empty() { "{row.state.class()}" },
                        if let Some(mk) = row.state.marker() {
                            span { class: "dm-mk", "{mk}" }
                        }
                        Quoted { text: row.decl.clone(), held: row.target.clone() }
                    }
                }
                // The brace that closes the quotation. What nests below it is
                // the state this block owns, not more of its own declaration.
                if !view.close.is_empty() {
                    p { class: "dm-close", "{view.close}" }
                }
            }
            // The state this block owns, nested under a hairline rule: the
            // paper's own nesting is the ownership, so no line restates it.
            if !view.kids.is_empty() {
                div { class: "dm-kids", style: "height: {KIDS_PAD + view.kids_h + KIDS_PAD}px;",
                    for (i , kid) in view.kids.iter().enumerate() {
                        div {
                            key: "{kid.id}",
                            class: "dm-kid",
                            style: format!(
                                "left: {}px; top: {}px; width: {}px; height: {}px;",
                                view.kid_at[i].0,
                                KIDS_PAD + view.kid_at[i].1,
                                kid.size.0,
                                kid.size.1,
                            ),
                            DataPlate {
                                view: kid.clone(),
                                kin: kin.clone(),
                                hot,
                                up: Some(view.id),
                            }
                        }
                    }
                }
            }
            if !view.folds.is_empty() {
                div { class: "dm-own dm-folds",
                    for (i , fold) in view.folds.iter().enumerate() {
                        p { key: "{i}", class: "dm-fold", "{fold}" }
                    }
                }
            }
        }
    }
}

/// Node view for the data chart.
#[component]
fn DataNode(
    ctx: NodeViewCtx<DataNodeData>,
    kin: Option<DataKin>,
    hot: Signal<Option<Anchor>>,
) -> Element {
    let mut folds = use_data().folds;
    match ctx.node.data.clone() {
        DataNodeData::Mark(view) => rsx! {
            DataPlate { view: *view, kin, hot, up: None }
        },
        DataNodeData::Fold(row) => match row.unfolds.clone() {
            Some(key) => {
                rsx! {
                    button {
                        class: "data-foldrow is-mod",
                        title: "{row.title}",
                        onclick: move |e: Event<MouseData>| {
                            e.stop_propagation();
                            folds.with_mut(|set| set.remove(&key));
                        },
                        "{row.words}"
                    }
                }
            }
            None => rsx! {
                p { class: "data-foldrow", title: "{row.title}", "{row.words}" }
            },
        },
    }
}

/// The ground: crate and module frames, the same two border gestures as the
/// surface chart — the line selects the module, the mark at its other end
/// folds it — pointed at this altitude's own routes and fold store.
#[component]
fn FrameLayer(frames: Vec<FrameView>, kin: Option<DataKin>) -> Element {
    let nav = use_navigator();
    let mut folds = use_data().folds;
    let boundary = |f: &FrameView| -> Element {
        let class = kin.as_ref().map_or("", |k| k.frame_class(f.id));
        let chosen = class == "is-sel";
        let to = match chosen {
            true => Route::DataOverview {},
            false => mod_route(f.key.clone()),
        };
        let (clicked, pressed) = (to.clone(), to);
        let (shut, shut_key) = (f.key.clone(), f.key.clone());
        let words = f.words.clone();
        let kind = match f.key.len() {
            1 => "crate",
            _ => "module",
        };
        let mark = if f.folded { "+" } else { "−" };
        let (bx, by) = (f.at.x + f.at.w - 15.0, f.at.y);
        rsx! {
            g {
                key: "{f.id}",
                class: "data-frame-group",
                class: if !class.is_empty() { "{class}" },
                rect {
                    class: "data-frame",
                    x: "{f.at.x}",
                    y: "{f.at.y}",
                    width: "{f.at.w}",
                    height: "{f.at.h}",
                }
                g {
                    class: "data-frame-pick",
                    tabindex: "0",
                    role: "link",
                    "aria-label": if chosen { "deselect {words}" } else { "select the {kind} {words}" },
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        nav.push(clicked.clone());
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
                            "{words} — select this {kind} · everything else recedes"
                        }
                    }
                    rect {
                        class: "data-frame-hit",
                        x: "{f.at.x}",
                        y: "{f.at.y}",
                        width: "{f.at.w}",
                        height: "{f.at.h}",
                    }
                    if let Some(label) = f.label.clone() {
                        rect {
                            class: "data-frame-tab",
                            x: "{f.at.x + 8.0}",
                            y: "{f.at.y - 9.0}",
                            width: "{f.label_w + 12.0}",
                            height: "18",
                        }
                        text {
                            class: "data-frame-label",
                            x: "{f.at.x + 14.0}",
                            y: "{f.at.y}",
                            "{label}"
                        }
                    }
                }
                if f.label.is_some() {
                    g {
                        class: "data-frame-shut",
                        tabindex: "0",
                        role: "button",
                        "aria-label": if f.folded { "unfold {words}" } else { "fold the {kind} {words} to one row" },
                        onclick: move |e: Event<MouseData>| {
                            e.stop_propagation();
                            folds
                                .with_mut(|set| {
                                    if !set.remove(&shut) {
                                        set.insert(shut.clone());
                                    }
                                });
                        },
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter {
                                e.stop_propagation();
                                folds
                                    .with_mut(|set| {
                                        if !set.remove(&shut_key) {
                                            set.insert(shut_key.clone());
                                        }
                                    });
                            }
                        },
                        title {
                            if f.folded {
                                "{words} is folded · unfold it"
                            } else {
                                "fold {words} to one counted row"
                            }
                        }
                        rect {
                            class: "data-frame-hit-mark",
                            x: "{bx - 10.0}",
                            y: "{by - 9.0}",
                            width: "20",
                            height: "18",
                        }
                        text {
                            class: "data-frame-mark",
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

/// The far edition's region names: at reading zoom the border chip is the
/// label, but below it a 12px chip is dust — so each module's name engraves
/// across its own territory the way an atlas names a region, sized to the
/// frame, haloed in paper, at half pressure, and shown only while the chart
/// is far. Its own layer, over the blocks: the skyline packs frames tight,
/// so drawn on the ground the name would spend its ink behind the paper.
#[component]
fn NameLayer(frames: Vec<FrameView>, kin: Option<DataKin>) -> Element {
    // A frame holding other frames names its own top band: centered it would
    // engrave over its children's territory and read as theirs. Under a
    // reading the names recede with their frames — an engraved name at full
    // pressure would say the opposite of what the receded boundary says.
    let parents: std::collections::HashSet<u32> = frames.iter().filter_map(|f| f.parent).collect();
    let class = |id: u32| kin.as_ref().map_or("", |k| k.frame_class(id));
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for f in frames.iter().filter(|f| f.label.is_some() && !f.folded) {
                if parents.contains(&f.id) {
                    text {
                        key: "{f.id}",
                        class: "data-frame-name",
                        class: if !class(f.id).is_empty() { "{class(f.id)}" },
                        x: "{f.at.x + 18.0}",
                        y: "{f.at.y + (f.at.w * 0.06).clamp(24.0, 54.0) + 10.0}",
                        font_size: "{(f.at.w * 0.06).clamp(24.0, 54.0)}",
                        "{f.label.clone().unwrap_or_default()}"
                    }
                } else {
                    text {
                        key: "{f.id}",
                        class: "data-frame-name",
                        class: if !class(f.id).is_empty() { "{class(f.id)}" },
                        x: "{f.at.x + f.at.w / 2.0}",
                        y: "{f.at.y + f.at.h * 0.55}",
                        text_anchor: "middle",
                        font_size: "{(f.at.w * 0.13).min(f.at.h * 0.45).clamp(26.0, 110.0)}",
                        "{f.label.clone().unwrap_or_default()}"
                    }
                }
            }
        }
    }
}

/// One curved hairline bowed toward open paper, arrowhead on the dependent.
fn curve(a: Point, b: Point, side: f64) -> (String, Point) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let bow = (len * 0.16).min(52.0) * side;
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

fn wire_classes(
    w: &WireView,
    is_ref: bool,
    hot: Option<Anchor>,
    kin: Option<&DataKin>,
) -> Vec<&'static str> {
    // A bundle exists only inside a module reading and is that reading's own
    // ink: the solid family at the radius pressure, the dashed a step behind.
    if w.bundle {
        return vec![if is_ref { "is-near" } else { "is-kin" }];
    }
    let is_kin = kin.is_some_and(|k| !is_ref && k.wire_kin(w.a, w.b));
    let is_near = kin.is_some_and(|k| is_ref && k.tie_near(w.a, w.b));
    let worn = [
        (!w.event.is_empty(), w.event),
        (!w.rest, "is-folded"),
        (hot.is_some_and(|h| h == w.a || h == w.b), "is-hot"),
        (is_kin, "is-kin"),
        (is_near, "is-near"),
        (kin.is_some() && !is_kin && !is_near, "is-dim"),
    ];
    worn.into_iter()
        .filter_map(|(on, class)| on.then_some(class))
        .collect()
}

/// Both families as one engraved layer, the uses family first and lighter.
#[component]
fn WireLayer(
    holds: Vec<WireView>,
    ties: Vec<WireView>,
    hot: Signal<Option<Anchor>>,
    kin: Option<DataKin>,
) -> Element {
    let hot = hot();
    let wire = |w: &WireView, family: &'static str, side: f64| {
        let (d, ctrl) = curve(w.from, w.to, side);
        let head = arrowhead(w.to, ctrl, 3.4 + w.width);
        let (lx, ly) = (
            0.25 * w.from.x + 0.5 * ctrl.x + 0.25 * w.to.x,
            0.25 * w.from.y + 0.5 * ctrl.y + 0.25 * w.to.y,
        );
        let classes = wire_classes(w, family.ends_with("data-ref"), hot, kin.as_ref()).join(" ");
        rsx! {
            g {
                key: "{w.key}",
                class: "{family} {w.class}",
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
    };
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for tie in ties.iter() {
                {wire(tie, "data-wire data-ref", -1.0)}
            }
            for hold in holds.iter() {
                {wire(hold, "data-wire data-hold", 1.0)}
            }
        }
    }
}

/// Chrome insets at the data altitude: the cartouche column left, the sheet
/// right while a mark is selected. The same choreography as the surface.
fn chrome_insets(narrow: bool, panel: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (312.0, 20.0, 70.0, 12.0)
    } else {
        (56.0, if panel { 330.0 } else { 24.0 }, 24.0, 284.0)
    }
}

const MIN_CHART_ZOOM: f64 = 0.22;

/// Below this the quoted rows are dust and the chart holds its far edition:
/// names, tiers and wires alone. Hysteresis keeps the swap from flapping
/// while the reader hovers around the threshold.
const FAR_IN: f64 = 0.45;
const FAR_OUT: f64 = 0.55;
/// The zoom a selection is read at: when a chosen mark sits below this, or
/// off the glass entirely, the camera glides to it (2026-08-21, user
/// decision — a focus is the one move the camera discipline permits, and a
/// selection the reader cannot see is not a focus).
#[cfg(target_arch = "wasm32")]
const READ_ZOOM: f64 = 0.5;

/// The camera as the reviewer last left it, surviving route-variant
/// remounts. Provided by the atlas shell, which outlives every remount.
#[derive(Clone, Copy)]
pub(crate) struct DataCamera {
    pub(crate) viewport: Signal<Option<Viewport>>,
}

impl DataCamera {
    pub(crate) fn new() -> Self {
        Self {
            viewport: Signal::new(None),
        }
    }
}

fn frame_chart(
    flow: dioxus_flow::prelude::FlowHandle<DataNodeData>,
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
    let zoom = fit.clamp(MIN_CHART_ZOOM, 1.0);
    let center = bounds.center();
    let cx = l + free_w / 2.0;
    let cy = t + free_h / 2.0;
    core.set_viewport(
        dioxus_flow::prelude::Viewport::new(cx - center.x * zoom, cy - center.y * zoom, zoom),
        duration_ms,
    );
}

/// Keyboard at the data altitude: `f` refits, Escape deselects, `/` finds.
const DATA_KEYS_JS: &str = r#"
if (window.__slopeKeys) {
    document.removeEventListener('keydown', window.__slopeKeys);
}
window.__slopeKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') {
        e.preventDefault();
        // One search plate for the desktop and one for a narrow viewport;
        // `/` must reach whichever is actually showing.
        const s = [...document.querySelectorAll('#data-search')]
            .find((el) => el.offsetParent !== null);
        if (s) s.focus();
        return;
    }
    if (['f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopeKeys);
"#;

/// The data chart, mounted for `/data`.
#[component]
pub(crate) fn DataChart(graph: CodeGraph, sel: Option<DataSel>) -> Element {
    let code = use_code();
    let data = use_data();
    let camera = use_context::<DataCamera>();
    let flow = dioxus_flow::use_flow_handle::<DataNodeData>();
    let nav = use_navigator();

    let chart = use_memo(use_reactive((&graph,), {
        move |(graph,)| {
            let model = DataModel::build(&graph, *code.ref_dir.read(), &data.folds.read());
            DataDrawing::from(&model)
        }
    }));

    // The route's selection, resolved to anchors. A mark's blast radius walks
    // every structural pair — the nesting included, because a kid's shape is
    // part of its holder's shape whether or not a line was drawn.
    let kin: Memo<Option<DataKin>> = use_memo(use_reactive((&sel,), move |(sel,)| {
        let drawing = chart.read();
        let tie_pairs: Vec<(Anchor, Anchor)> = drawing.ties.iter().map(|w| (w.a, w.b)).collect();
        match sel? {
            DataSel::Mark(path, label) => {
                fn find(view: &MeasuredBlock, path: &str, label: &str) -> Option<u32> {
                    if view.path == path && view.label == label {
                        return Some(view.id);
                    }
                    view.kids.iter().find_map(|kid| find(kid, path, label))
                }
                let id = drawing.nodes.iter().find_map(|n| match &n.data {
                    DataNodeData::Mark(m) => find(m, &path, &label),
                    _ => None,
                })?;
                Some(DataKin::read(Anchor::Mark(id), &drawing.pairs, &tie_pairs))
            }
            DataSel::Mod(key) => {
                let frame = drawing.frames.iter().find(|f| f.key == key)?.id;
                let hold_pairs: Vec<(Anchor, Anchor)> = drawing.pairs.clone();
                Some(DataKin::read_mod(
                    frame,
                    &drawing.frames,
                    &drawing.homes,
                    &hold_pairs,
                    &tie_pairs,
                ))
            }
        }
    }));

    let sel_on: Signal<bool> = use_signal(|| false);
    use_effect(use_reactive((&sel.is_some(),), move |(on,)| {
        let mut sel_on = sel_on;
        if *sel_on.peek() != on {
            sel_on.set(on);
        }
    }));

    let nodes: Signal<Vec<FlowNode<DataNodeData>>> = use_signal(Vec::new);
    let framed = use_signal(|| false);
    let hot: Signal<Option<Anchor>> = use_signal(|| None);
    let core_live: Signal<bool> = use_signal(|| false);
    // The chart is far below FAR_IN and near again above FAR_OUT.
    let far: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        let drawing = chart();
        let mut nodes = nodes;
        nodes.set(drawing.nodes);
        // Camera discipline: first paint gives the reader back their place,
        // and frames only a fresh session. After that, only `f` moves it.
        #[cfg(target_arch = "wasm32")]
        {
            let mut framed = framed;
            if *framed.peek() {
                return;
            }
            framed.set(true);
            let frame = drawing.frame;
            let panel = *sel_on.peek();
            let mut core_live = core_live;
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(150).await;
                for _ in 0..40 {
                    if flow.core().is_some() {
                        break;
                    }
                    gloo_timers::future::TimeoutFuture::new(50).await;
                }
                core_live.set(true);
                if let Some(vp) = *camera.viewport.peek() {
                    flow.set_viewport(vp, 0);
                } else if let Some(frame) = frame {
                    frame_chart(flow, frame, panel, 0);
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (framed, core_live);
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
        // between its near and far editions, so the swap happens once per
        // crossing, never per frame.
        let mut far = far;
        let now = *far.peek();
        if now && vp.zoom > FAR_OUT {
            far.set(false);
        } else if !now && vp.zoom < FAR_IN {
            far.set(true);
        }
    });

    // The camera glides to a selection it cannot show: off the glass, or
    // below reading zoom. A selection already legible moves nothing — the
    // Kept-Ground rule holds everywhere the reader can actually read.
    #[cfg(target_arch = "wasm32")]
    use_effect(use_reactive((&sel,), move |(sel,)| {
        if !core_live() {
            return;
        }
        let Some(DataSel::Mark(path, label)) = sel else {
            return;
        };
        let Some(core) = flow.core() else { return };
        let drawing = chart.peek();
        let Some(&anchor) = drawing.locate.get(&(path, label)) else {
            return;
        };
        let Some(at) = drawing.rects.get(&anchor).copied() else {
            return;
        };
        let vp = *core.viewport.peek();
        let Some((w, h)) = window_size() else { return };
        let (vx, vy) = ((0.0 - vp.x) / vp.zoom, (0.0 - vp.y) / vp.zoom);
        let (vw, vh) = (w / vp.zoom, h / vp.zoom);
        let inside = at.x >= vx && at.y >= vy && at.x + at.w <= vx + vw && at.y + at.h <= vy + vh;
        if inside && vp.zoom >= READ_ZOOM {
            return;
        }
        let z = vp.zoom.clamp(0.85, 1.0);
        let (t, r, btm, l) = chrome_insets(narrow_viewport(), true);
        let free_w = (w - l - r).max(120.0);
        let free_h = (h - t - btm).max(120.0);
        let (cx, cy) = (l + free_w / 2.0, t + free_h / 2.0);
        let (mx, my) = (at.x + at.w / 2.0, at.y + at.h / 2.0);
        let duration = if prefers_reduced_motion() { 0 } else { 400 };
        core.set_viewport(Viewport::new(cx - mx * z, cy - my * z, z), duration);
    }));

    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(DATA_KEYS_JS);
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
                    "Escape" if *sel_on.peek() => {
                        nav.push(Route::DataOverview {});
                    }
                    _ => {}
                }
            }
        });
    });

    // Under a module reading the crossing ink bundles: one line per far
    // module and direction, its count on the line, in place of the dozens of
    // near-parallel curves that were the hairball this system forbids.
    // Everything inside the boundary keeps its own lines; hovering a block
    // still inks that block's own wires, which is where a bundle opens.
    let wires = use_memo(move || {
        let drawing = chart.read();
        let k = kin.read();
        let Some((chosen, inside)) = k.as_ref().and_then(|k| k.mod_home()) else {
            return (drawing.holds.clone(), drawing.ties.clone());
        };
        let frame_rect = |id: u32| drawing.frames.iter().find(|f| f.id == id).map(|f| f.at);
        let Some(chosen_rect) = frame_rect(chosen) else {
            return (drawing.holds.clone(), drawing.ties.clone());
        };
        let bundle = |wires: &[WireView], dashed: bool| -> Vec<WireView> {
            let mut kept: Vec<WireView> = Vec::new();
            // (far frame, arrow lands inside) → (lines, carried weight).
            let mut groups: HashMap<(u32, bool), (u32, u32)> = HashMap::new();
            for w in wires {
                let (a_in, b_in) = (inside.contains(&w.a), inside.contains(&w.b));
                if a_in == b_in {
                    kept.push(w.clone());
                    continue;
                }
                let far = if a_in { w.b } else { w.a };
                let Some(&frame) = drawing.homes.get(&far) else {
                    kept.push(w.clone());
                    continue;
                };
                let entry = groups.entry((frame, b_in)).or_default();
                entry.0 += 1;
                entry.1 += w.weight;
            }
            let mut keys: Vec<(u32, bool)> = groups.keys().copied().collect();
            keys.sort_unstable();
            for (frame, inbound) in keys {
                let (lines, weight) = groups[&(frame, inbound)];
                let Some(far_rect) = frame_rect(frame) else {
                    continue;
                };
                let (from, to) = match inbound {
                    true => tie_ends(far_rect, chosen_rect),
                    false => tie_ends(chosen_rect, far_rect),
                };
                // The dashed family counts references, as its labels do
                // everywhere; the solid family counts its lines.
                let count = if dashed { weight } else { lines };
                kept.push(WireView {
                    key: format!(
                        "bundle-{}-{frame}-{inbound}",
                        if dashed { "t" } else { "h" }
                    ),
                    from,
                    to,
                    a: Anchor::Mod(frame),
                    b: Anchor::Mod(chosen),
                    label: Some(count.to_string()),
                    width: if dashed { tie_width(count) } else { 1.6 },
                    rest: true,
                    class: if dashed {
                        "is-ref is-bundle"
                    } else {
                        "is-owns is-bundle"
                    },
                    event: "",
                    weight: count,
                    bundle: true,
                });
            }
            kept
        };
        (bundle(&drawing.holds, false), bundle(&drawing.ties, true))
    });

    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);
    let panel = matches!(sel, Some(DataSel::Mark(..)));

    rsx! {
        div {
            class: "data-chart absolute inset-0",
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
                        nav.push(Route::DataOverview {});
                    }
                },
                node_view: move |ctx: NodeViewCtx<DataNodeData>| {
                    // The blocks carry the reading's pressure on their own
                    // paint; the node wrapper carries only the diff's rest,
                    // because a lit kid must never dim with its holder's box.
                    let rest = kin.read().is_none()
                        && chart.read().dirty
                        && !ctx.node.data.touched();
                    rsx! {
                        div {
                            class: "data-node",
                            class: if rest { "is-rest" },
                            DataNode { ctx, kin: kin(), hot }
                        }
                    }
                },
                {
                    let (top, right, bottom, left) = chrome_insets(narrow_viewport(), panel);
                    rsx! {
                        FitInsets { top, right, bottom, left }
                    }
                }
                WorldLayer { class: "data-ground",
                    FrameLayer { frames: chart.read().frames.clone(), kin: kin() }
                }
                WorldLayer { class: "data-wires",
                    WireLayer {
                        holds: wires.read().0.clone(),
                        ties: wires.read().1.clone(),
                        hot,
                        kin: kin(),
                    }
                }
                WorldLayer { class: "data-names",
                    NameLayer { frames: chart.read().frames.clone(), kin: kin() }
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
    use crate::api::Vis;
    use crate::views::data::model::Tier;
    use crate::views::surface::model::RowState;

    fn mark(id: u32, name: &str, fields: Vec<(&str, &str, &str)>, kids: Vec<u32>) -> DataMark {
        DataMark {
            id,
            frame: 0,
            kind: ItemKind::Struct,
            vis: Vis::Pub,
            name: name.to_string(),
            label: name.to_string(),
            path: "src/api.rs".to_string(),
            line: 1,
            delta: crate::api::Delta::Same,
            ghost: false,
            fields: fields
                .into_iter()
                .map(|(name, decl, target)| FieldRow {
                    name: name.to_string(),
                    decl: decl.to_string(),
                    vis: crate::api::Vis::Private,
                    target: Held::named(target),
                    state: RowState::Same,
                })
                .collect(),
            variants: Vec::new(),
            ty: String::new(),
            ty_target: Held::default(),
            tier: Tier::Root,
            kids,
            named_by: 0,
            used_by: Vec::new(),
            unseen_uses: Vec::new(),
            held_by: 0,
        }
    }

    #[test]
    fn a_block_grows_to_hold_the_blocks_nested_in_it() {
        let kid =
            MeasuredBlock::measure(&mark(1, "Nut", vec![("size", "u32", "")], vec![]), vec![]);
        let alone = MeasuredBlock::measure(
            &mark(0, "Wire", vec![("nuts", "Vec<Nut>", "Nut")], vec![]),
            vec![],
        );
        let parent = MeasuredBlock::measure(
            &mark(0, "Wire", vec![("nuts", "Vec<Nut>", "Nut")], vec![1]),
            vec![kid.clone()],
        );
        assert!(parent.size.1 > alone.size.1 + kid.size.1 * 0.9);
        assert!(parent.size.0 >= kid.size.0 + PAD_X);
    }

    #[test]
    fn nested_rects_land_inside_the_parents_box() {
        let kid =
            MeasuredBlock::measure(&mark(1, "Nut", vec![("size", "u32", "")], vec![]), vec![]);
        let parent = MeasuredBlock::measure(
            &mark(0, "Wire", vec![("nuts", "Vec<Nut>", "Nut")], vec![1]),
            vec![kid],
        );
        let mut rects = HashMap::new();
        abs_rects(&parent, 100.0, 200.0, &mut rects);
        let p = rects[&Anchor::Mark(0)];
        let k = rects[&Anchor::Mark(1)];
        assert!(k.x >= p.x && k.y >= p.y);
        assert!(k.x + k.w <= p.x + p.w + 0.01);
        assert!(k.y + k.h <= p.y + p.h + 0.01);
    }

    #[test]
    fn kids_shelve_without_overlap() {
        let kids: Vec<MeasuredBlock> = (1..=5u32)
            .map(|id| {
                MeasuredBlock::measure(
                    &mark(id, &format!("K{id}"), vec![("a", "u32", "")], vec![]),
                    vec![],
                )
            })
            .collect();
        let (at, w, h) = shelve_kids(&kids);
        assert!(w > 0.0 && h > 0.0);
        for (i, (ax, ay)) in at.iter().enumerate() {
            for (j, (bx, by)) in at.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (aw, ah) = kids[i].size;
                let (bw, bh) = kids[j].size;
                let clear = ax + aw <= *bx || bx + bw <= *ax || ay + ah <= *by || by + bh <= *ay;
                assert!(clear, "kid {i} overlaps kid {j}");
            }
        }
    }
}
