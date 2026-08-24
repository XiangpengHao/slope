//! The chart: contract marks, module frames, and the two inks between them.
//!
//! One block per contract the workspace publishes — a type with its fields
//! and then a second band of the methods it publishes, a trait that is nearly
//! all band, a function wearing its signature, a static or const or alias one
//! line long — each seated in the frame of the module that declares it. The
//! block is measured before it is placed, so its plate and its box agree to
//! the pixel, and the whole layout is a pure function of the survey: the same
//! workspace always draws the same chart.
//!
//! Two families share the paper, and the ink tells them apart. **Holds** is
//! interface coupling, drawn solid at one pressure, with the wrapper's own
//! word on the line where the walk met one. **Uses** is implementation
//! coupling — a body leaning on a contract — drawn dashed and lighter, thinned
//! by the cartouche's toggle, with its count on the line. Both point at the
//! dependent.

use std::collections::{HashMap, HashSet};

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
use crate::views::surface::layout::{Sizes, SurfaceLayout};
use crate::views::surface::model::{
    Anchor, FieldRow, RowState, SurfaceMark, SurfaceModel, upstream,
};
use crate::views::surface::{SurfaceSel, mark_route, mod_route};

// ---------------------------------------------------------------------------
// Mark furniture, in flow units — one unit is one CSS pixel at zoom 1. These
// numbers are the CSS in `tailwind.css`; move one and the other must follow, or
// a block will stand on its neighbor.
// ---------------------------------------------------------------------------

/// The border and the top padding, then the bottom padding and border.
const PAD_TOP: f64 = 6.0;
const PAD_BOTTOM: f64 = 5.0;
/// Border plus side padding, both sides.
const PAD_X: f64 = 16.0;
const HEAD_H: f64 = 16.0;
const ROW_H: f64 = 15.0;
/// One wrapped line of a static's declared type.
const TY_H: f64 = 14.0;
/// The rule that opens the method band, when a block has one.
const BAND_TOP: f64 = 5.0;
/// The counted folds: the rule above them, then one line each.
const FOLDS_TOP: f64 = 6.0;
const FOLD_H: f64 = 12.0;
/// Small-type slack: at 9px the browser rounds each glyph up, so a fold line
/// measured with the font's exact 0.6em advance clips its last characters.
const META_SLACK: f64 = 1.08;
const MARK_MIN_W: f64 = 152.0;
const MARK_MAX_W: f64 = 300.0;
/// A counted fold row standing in for what a frame does not draw.
const ROW_MIN_W: f64 = 132.0;
const ROW_FOLD_H: f64 = 22.0;
/// What ragged line breaks cost over a straight width ratio.
const WRAP_SLACK: f64 = 1.12;

/// A uses edge's weight follows the code map's rule exactly: the more
/// references a pair of marks has between them, the firmer the hairline.
fn tie_width(count: u32) -> f64 {
    (0.55 + count as f64 * 0.13).min(2.8)
}

/// A hold's weight. The published surface is drawn at one steady pressure; the
/// uses edges rest lighter under it, so the two families never read as one.
fn hold_width(kind: HoldKind) -> f64 {
    match kind {
        HoldKind::Shares => 1.3,
        HoldKind::Dyn => 1.2,
        _ => 1.1,
    }
}

/// How many lines a run of text needs at a width. The chart would rather carry
/// slack than clip a name.
fn wrapped(text: &str, px: f64, usable: f64) -> f64 {
    (text_w(text, px) * WRAP_SLACK / usable.max(24.0))
        .ceil()
        .max(1.0)
}

/// One mark, measured and ready to engrave. It carries the whole quotation and
/// draws all of it: a declaration is not a preview, so no row of it waits
/// behind a count (user decision, 2026-08-20). The only counted lines left at
/// the foot are the chart's own folds — the fan-in it does not draw as ink.
#[derive(Clone, PartialEq)]
struct MeasuredMark {
    id: u32,
    /// `pub struct`, `static` — what rust writes in front of the name.
    decl: String,
    name: String,
    /// The structural diff's letter, in git's alphabet: `A`, `D`, or `M`.
    letter: Option<&'static str>,
    /// A removed type, quoted from the base edition and drawn dashed.
    ghost: bool,
    is_static: bool,
    /// A sum type. Its name takes the palette's other type color, so struct
    /// and enum tell apart at a glance the keyword can only be read at.
    is_enum: bool,
    /// A free function: its rows are parameters and its `ty` line is what it
    /// returns, so that line reads under them instead of over them.
    is_fn: bool,
    /// Every field, quoted as written — and every one of them drawn.
    fields: Vec<FieldRow>,
    /// A static's declared type, or a function's return type with the arrow
    /// rust writes in front of it — as written either way.
    ty: String,
    /// The workspace type that type holds, drawn in full ink. Empty where it
    /// holds nothing this chart draws.
    ty_target: String,
    /// An enum's variants as written, one row each (the row text in `decl`).
    variants: Vec<FieldRow>,
    /// The methods the door draws, quoted as written — the block's second
    /// band, under a rule of its own so the shape reads before the API.
    methods: Vec<FieldRow>,
    /// The counted lines at the foot: the fan-in the chart folds to words.
    /// Nothing the block quotes is ever counted there.
    folds: Vec<String>,
    /// `path:line`, for the block's hover words. It is off the resting paper
    /// (2026-08-21): stamped under 200 blocks at 8.5px it was texture, and it
    /// only said again where the block already stands — inside its module's
    /// frame — while the sheet quotes it in full the moment a block is picked.
    locator: String,
    path: String,
    label: String,
    size: (f64, f64),
}

/// A frame's counted fold row: what it does not draw, and how much of it.
#[derive(Clone, PartialEq)]
struct FoldView {
    anchor: Anchor,
    words: String,
    /// Why this row stands, in words, for its hover. The visibility fold's
    /// reason moves with the doors setting, so it is decided here rather
    /// than guessed at from the anchor.
    title: String,
    /// The module this row stands for, where the row is a module the reviewer
    /// folded by hand: clicking it puts the module back. The other two rows
    /// count code the chart was never going to draw, and open onto nothing.
    unfolds: Option<Vec<String>>,
    size: (f64, f64),
}

/// One node on the surface chart. A mark's view is much the wider of the two —
/// it carries the block's whole quotation — so it travels boxed rather than
/// making every fold row in the node list as large as a mark.
#[derive(Clone, PartialEq)]
enum SurfaceNodeData {
    Mark(Box<MeasuredMark>),
    Fold(FoldView),
}

impl SurfaceNodeData {
    fn anchor(&self) -> Anchor {
        match self {
            SurfaceNodeData::Mark(m) => Anchor::Mark(m.id),
            SurfaceNodeData::Fold(f) => f.anchor,
        }
    }

    /// The diff touched this node: it keeps full pressure while the rest of a
    /// dirty chart recedes.
    fn touched(&self) -> bool {
        match self {
            SurfaceNodeData::Mark(m) => m.letter.is_some(),
            SurfaceNodeData::Fold(_) => false,
        }
    }
}

/// A frame, placed, with the label it wears on its border and the two gestures
/// the border answers: selecting the module, and folding it.
#[derive(Clone, PartialEq)]
struct FrameView {
    id: u32,
    /// The frame this one sits in, so a reading knows which boundaries are
    /// inside the chosen one and which merely hold it.
    parent: Option<u32>,
    at: Placed,
    label: Option<String>,
    /// How wide that label draws, for the paper the gesture is caught on.
    label_w: f64,
    /// The module's name across builds: the crate, then the module path. What
    /// the URL selects by and what a fold is remembered by.
    key: Vec<String>,
    /// The whole path in prose, for the words the border says on hover.
    words: String,
    /// The reviewer folded this module: it draws one row and nothing else.
    folded: bool,
}

/// One drawn edge — a hold or a uses edge — with its ends already found.
#[derive(Clone, PartialEq)]
struct WireView {
    key: String,
    from: Point,
    to: Point,
    a: Anchor,
    b: Anchor,
    /// The word engraved on the line: a wrapper for a hold, a count for a
    /// uses edge.
    label: Option<String>,
    width: f64,
    /// Drawn at rest; a folded wire inks in when either end is hovered.
    rest: bool,
    /// Which family and, for a hold, which kind — as a CSS class.
    class: &'static str,
    /// The structural diff's class — `is-added` / `is-removed` — or empty.
    event: &'static str,
}

/// One drawing of the surface chart: the blocks, frames and wires a single
/// build puts on the paper, plus what the camera and the diff read back.
#[derive(Clone, PartialEq)]
struct SurfaceDrawing {
    nodes: Vec<FlowNode<SurfaceNodeData>>,
    frames: Vec<FrameView>,
    holds: Vec<WireView>,
    ties: Vec<WireView>,
    /// Which frame every drawn anchor is seated in — a mark's module, a
    /// counted row's own frame. A module reading is read off this.
    homes: HashMap<Anchor, u32>,
    frame: Option<Rect>,
    /// The diff touched this chart: untouched marks rest at lighter pressure.
    dirty: bool,
}

/// A module reading's own ink: the boundary the reviewer chose, everything
/// seated inside it, and the frames that must not recede with the strangers —
/// the modules nested in the chosen one, and the ones it is drawn inside,
/// which are the paper it stands on.
#[derive(Clone, PartialEq)]
struct ModHome {
    frame: u32,
    kept: HashSet<u32>,
    inside: HashSet<Anchor>,
}

/// The selection's ink. One chosen mark; everything a shape change to it could
/// reach, walking holds edges holder-ward (the blast radius); what it directly
/// holds, one hop the other way; and the marks its body leans on or that lean
/// on it. While a selection stands the rest of the chart recedes to a lighter
/// pressure — a reading, never a re-layout, and the camera does not move.
///
/// A module boundary reads the same way one altitude out: everything inside it
/// keeps full ink, whatever crosses it reads a step behind, and the other
/// modules recede — frames, blocks and wires alike.
#[derive(Clone, PartialEq)]
struct SurfaceKin {
    /// The chosen mark, where the reading is one contract's.
    sel: Option<Anchor>,
    /// The chosen boundary, where the reading is a module's.
    home: Option<ModHome>,
    /// Transitive holders. A counted fold row can join — its edge is drawn —
    /// but the walk ends there.
    up: HashSet<Anchor>,
    /// Directly held types.
    down: HashSet<Anchor>,
    /// The far ends of the uses edges touching the selection. Not the blast
    /// radius and never counted as it — implementation coupling stops where
    /// the body does — but neighbours all the same, and a neighbour the
    /// reader cannot read is one the chart may as well not have drawn: an
    /// edge that lands on a receded block says nothing.
    near: HashSet<Anchor>,
}

impl SurfaceKin {
    /// The selection's reading of one built chart. Both families arrive as
    /// their drawn pairs, tail first: a hold runs held → holder, a uses edge
    /// runs def → user.
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

    /// One module's reading: the boundary, what it holds, and what crosses it.
    /// A frame nested inside the chosen one is inside the boundary too — it is
    /// drawn within it, and receding what a reader can see inside the line
    /// they just chose would say the opposite of what the line says.
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
        let mut kept: HashSet<u32> = frames
            .iter()
            .filter(|f| f.id == frame || climb(f.id).contains(&frame))
            .map(|f| f.id)
            .collect();
        let inside: HashSet<Anchor> = homes
            .iter()
            .filter(|(_, home)| kept.contains(home))
            .map(|(anchor, _)| *anchor)
            .collect();
        kept.extend(climb(frame));
        // One hop over the line, either family: what the module publishes to
        // and what leans on it. Two hops would be the whole chart again.
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
            up: HashSet::new(),
            down: HashSet::new(),
            near,
        }
    }

    /// How a frame's own border reads: the chosen boundary in full ink, the
    /// boundaries it holds and the ones holding it left alone, every other
    /// module receded. A mark's reading leaves the ground as it found it —
    /// the blast radius is a walk between blocks, not a place on the paper.
    fn frame_class(&self, id: u32) -> &'static str {
        match &self.home {
            Some(home) if home.frame == id => "is-sel",
            Some(home) if !home.kept.contains(&id) => "is-dim",
            _ => "",
        }
    }

    fn node_class(&self, a: Anchor) -> &'static str {
        if let Some(home) = &self.home {
            return if home.inside.contains(&a) {
                "is-kin"
            } else if self.near.contains(&a) {
                "is-near"
            } else {
                "is-dim"
            };
        }
        if Some(a) == self.sel {
            "is-sel"
        } else if self.up.contains(&a) || self.down.contains(&a) {
            "is-kin"
        } else if self.near.contains(&a) {
            // A uses neighbour reads a step behind the blast radius and well
            // clear of the recede: the two families keep their own weights,
            // and neither is ever read as the other.
            "is-near"
        } else {
            "is-dim"
        }
    }

    /// A holds wire inside the selection's ink: a link in the chain toward the
    /// holders, or the one hop down to what the selection holds.
    fn wire_kin(&self, held: Anchor, holder: Anchor) -> bool {
        if let Some(home) = &self.home {
            // A module's published surface: every solid line the boundary
            // touches, inside it or across it.
            return home.inside.contains(&held) || home.inside.contains(&holder);
        }
        let upward = |x: Anchor| Some(x) == self.sel || self.up.contains(&x);
        (upward(held) && upward(holder)) || (Some(holder) == self.sel && self.down.contains(&held))
    }

    /// A uses edge the selection pins: one that touches the selected mark
    /// itself. Implementation coupling never joins the blast radius — a body
    /// can be rewritten without the surface moving — so these ink in beside
    /// the radius rather than inside it, folded ones included, and hold that
    /// ink for as long as the selection stands. Following an edge is a
    /// reading, and a reading must survive the cursor leaving the block.
    fn tie_near(&self, a: Anchor, b: Anchor) -> bool {
        if let Some(home) = &self.home {
            return home.inside.contains(&a) || home.inside.contains(&b);
        }
        Some(a) == self.sel || Some(b) == self.sel
    }
}

/// The counted words a mark writes at its foot. Only the chart's own folds
/// stand here now: a fan-in past [`HELD_CAP`](super::model) is ink the chart
/// will not draw and has to say instead. Nothing the block quotes is counted —
/// every field, variant, method row and parameter is drawn, so there is no
/// hidden row left to name (user decision, 2026-08-20).
fn fold_words(mark: &SurfaceMark) -> Vec<String> {
    let mut folds = Vec::new();
    if mark.held_by > 0 {
        folds.push(format!("held by {}", plural(mark.held_by as usize, "type")));
    }
    // The same fold, said for the contracts in it: a signature names a type,
    // it does not hold one, and the count has to keep that straight.
    if mark.named_by > 0 {
        folds.push(format!(
            "named by {}",
            plural(mark.named_by as usize, "signature")
        ));
    }
    folds
}

/// A mark, measured. The width is the widest line it must not clip and the
/// height is every line it draws: the whole quotation stands inside the box the
/// layout is handed, so the plate a reader meets is the plate the geometry
/// budgeted for.
impl From<&SurfaceMark> for MeasuredMark {
    fn from(mark: &SurfaceMark) -> Self {
        let decl = decl_words(mark.vis, mark.kind);
        let head = format!("{decl} {}", mark.name);
        let locator = mark.locator();
        let letter = mark.letter();
        // A signature's return line wears rust's own arrow, so the block reads as
        // the declaration it quotes rather than as a type sitting under a name.
        let ty = if mark.is_fn() && !mark.ty.is_empty() {
            format!("-> {}", mark.ty)
        } else {
            mark.ty.clone()
        };
        let folds = fold_words(mark);

        let mut widest = text_w(&head, 10.5) + if letter.is_some() { 12.0 } else { 0.0 };
        // A long row clips at the block's own maximum rather than stretching it
        // past the paper's patience. A marked row carries its `+`/`−` in front.
        let wrapping = MARK_MAX_W - PAD_X;
        let marker_w = |row: &FieldRow| {
            if row.state == RowState::Same {
                0.0
            } else {
                11.0
            }
        };
        for fold in &folds {
            // Browsers round each glyph up at this size; measured with the font's
            // exact advance the last characters clip. Carry slack.
            widest = widest.max(text_w(fold, 9.0) * META_SLACK);
        }
        for row in &mark.fields {
            widest = widest.max(
                (text_w(&format!("{}: {}", row.name, row.decl), 10.0) + marker_w(row))
                    .min(wrapping),
            );
        }
        for row in mark.variants.iter().chain(&mark.methods) {
            widest = widest.max((text_w(&row.decl, 10.0) + marker_w(row)).min(wrapping));
        }
        for fold in &folds {
            widest = widest.max(text_w(fold, 9.0));
        }
        if !ty.is_empty() {
            widest = widest.max(text_w(&ty, 9.5).min(wrapping));
        }
        let w = (widest + PAD_X).clamp(MARK_MIN_W, MARK_MAX_W);
        let usable = w - PAD_X;

        let ty_lines = if ty.is_empty() {
            0.0
        } else {
            wrapped(&ty, 9.5, usable)
        };
        let fold_block = if folds.is_empty() {
            0.0
        } else {
            FOLDS_TOP + folds.len() as f64 * FOLD_H
        };
        // The band opens on a rule of its own, so the shape reads before the API.
        let band = if mark.methods.is_empty() {
            0.0
        } else {
            BAND_TOP + mark.methods.len() as f64 * ROW_H
        };
        let h = PAD_TOP
            + HEAD_H
            + ty_lines * TY_H
            + mark.fields.len() as f64 * ROW_H
            + mark.variants.len() as f64 * ROW_H
            + band
            + fold_block
            + PAD_BOTTOM;

        MeasuredMark {
            id: mark.id,
            decl,
            name: mark.name.clone(),
            letter,
            ghost: mark.ghost,
            is_static: mark.is_static(),
            is_enum: mark.kind == ItemKind::Enum,
            is_fn: mark.is_fn(),
            fields: mark.fields.clone(),
            ty,
            ty_target: mark.ty_target.clone(),
            variants: mark.variants.clone(),
            methods: mark.methods.clone(),
            folds,
            locator,
            path: mark.path.clone(),
            label: mark.label.clone(),
            size: (w, h),
        }
    }
}

impl FoldView {
    /// A counted fold row, measured.
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

/// A hold's kind, as a CSS class. The kind no longer moves the ink — dash
/// means implementation coupling now, and every hold is solid — but the class
/// rides along for the hover and diff rules, and for whatever the grammar
/// needs to reach next.
fn hold_class(kind: HoldKind) -> &'static str {
    match kind {
        HoldKind::Owns => "is-owns",
        HoldKind::Shares => "is-shares",
        HoldKind::Borrows => "is-borrows",
        HoldKind::Dyn => "is-dyn",
        HoldKind::Implements => "is-impl",
    }
}

impl From<&SurfaceModel> for SurfaceDrawing {
    /// Measure everything, place it, and gather what the chart draws.
    fn from(model: &SurfaceModel) -> Self {
        let mut sizes = Sizes::default();
        let mut views: HashMap<u32, MeasuredMark> = HashMap::new();
        for mark in &model.marks {
            let view = MeasuredMark::from(mark);
            sizes.marks.insert(mark.id, view.size);
            views.insert(mark.id, view);
        }
        let mut rows: HashMap<Anchor, FoldView> = HashMap::new();
        for frame in &model.frames {
            // A folded module is one row: every contract inside it, its nested
            // modules included, and the way back out.
            if frame.folded {
                let anchor = Anchor::Mod(frame.id);
                // *Item*, the word the other counted rows use: what is inside a
                // boundary is contracts, private helpers and all, and one row
                // standing for a whole module cannot sort them out.
                let words = match frame.packed {
                    0 => "folded".to_string(),
                    n => format!("+ {}", plural(n as usize, "item")),
                };
                let mut row = FoldView::row(
                    anchor,
                    words,
                    format!(
                        "{} is folded to this row — every item inside it, and inside the \
                         modules nested in it; click to unfold",
                        frame.words()
                    ),
                );
                row.unfolds = Some(frame.key());
                sizes.rows.insert(anchor, row.size);
                rows.insert(anchor, row);
            }
            if frame.private > 0 {
                let anchor = Anchor::Private(frame.id);
                let words = format!(
                    "+ {}",
                    plural(frame.private as usize, model.doors.fold_word())
                );
                let row = FoldView::row(anchor, words, model.doors.fold_title().to_string());
                sizes.rows.insert(anchor, row.size);
                rows.insert(anchor, row);
            }
            if let Some(label) = frame.label(model.multi_crate) {
                sizes.labels.insert(frame.id, text_w(&label, 12.0) + 18.0);
            }
        }

        let placed: SurfaceLayout = SurfaceLayout::build(&model.frames, &sizes);

        let mut nodes: Vec<FlowNode<SurfaceNodeData>> = Vec::new();
        for (id, view) in &views {
            let Some(at) = placed.marks.get(id) else {
                continue;
            };
            nodes.push(
                FlowNode::with_data(
                    node_key(Anchor::Mark(*id)),
                    view.name.clone(),
                    (at.x, at.y),
                    SurfaceNodeData::Mark(Box::new(view.clone())),
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
                    SurfaceNodeData::Fold(row.clone()),
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

        // Where everything sits, in one map: a mark in the module that declares
        // it, a counted row in the frame that counts it.
        let homes: HashMap<Anchor, u32> = model
            .marks
            .iter()
            .map(|m| (Anchor::Mark(m.id), m.frame))
            .chain(
                rows.keys()
                    .filter_map(|&anchor| anchor.frame().map(|frame| (anchor, frame))),
            )
            .collect();

        // The arrowhead rests on the holder, so the wire runs held → holder.
        // A diff event writes its own word on the line, after the wrapper's.
        let holds: Vec<WireView> = model
            .holds
            .iter()
            .filter_map(|hold| {
                let (a, b) = (placed.rect(hold.held)?, placed.rect(hold.holder)?);
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
                })
            })
            .collect();

        // The uses family: the arrowhead rests on the dependent, as everywhere.
        let ties: Vec<WireView> = model
            .ties
            .iter()
            .filter_map(|tie| {
                let (a, b) = (placed.rect(tie.def)?, placed.rect(tie.user)?);
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
                })
            })
            .collect();

        let frame = Rect::bounds(nodes.iter().map(|n| n.rect())).or_else(|| {
            frames
                .first()
                .map(|f| Rect::new(f.at.x, f.at.y, f.at.w, f.at.h))
        });

        SurfaceDrawing {
            nodes,
            frames,
            holds,
            ties,
            homes,
            frame,
            dirty: model.marks.iter().any(|m| m.letter().is_some()),
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

/// One quoted row, split into token runs: `(class, run, held)`. A mark's field
/// and variant rows are quotations, so inside them color is token class, by
/// the plate's own grammar — keywords, uppercase-initial names and lifetimes,
/// numbers, punctuation. The one run that names the row's held workspace type
/// is bold on top of its class, so `Vec<FileDetail>` still reads as the
/// wrapper it is around the type it reaches.
pub(crate) fn spans(text: &str, target: &str) -> Vec<(&'static str, String, bool)> {
    const KEYWORDS: [&str; 8] = ["dyn", "mut", "impl", "fn", "pub", "crate", "const", "as"];
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut out: Vec<(&'static str, String, bool)> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if ident(c) {
            let mut run = String::new();
            while chars.peek().copied().is_some_and(ident) {
                run.push(chars.next().unwrap());
            }
            let class = if c.is_ascii_digit() {
                "tok-num"
            } else if KEYWORDS.contains(&run.as_str()) {
                "tok-kw"
            } else if c.is_uppercase() {
                "tok-type"
            } else {
                ""
            };
            let held = !target.is_empty() && run == target;
            out.push((class, run, held));
        } else if c == '\'' {
            let mut run = String::from(chars.next().unwrap());
            while chars.peek().copied().is_some_and(ident) {
                run.push(chars.next().unwrap());
            }
            out.push(("tok-type", run, false));
        } else {
            let mut run = String::new();
            while chars
                .peek()
                .copied()
                .is_some_and(|c| !ident(c) && c != '\'')
            {
                run.push(chars.next().unwrap());
            }
            out.push(("tok-punct", run, false));
        }
    }
    out
}

/// One type's block: what it is, what it holds, and where it is written. The
/// whole block is the link to its selection — clicking it keeps the chart and
/// inks its blast radius; the selected block clicked again deselects. Its
/// definition plate stays one step further, on the selection sheet's link.
///
/// The block draws its whole quotation whether it is selected or not: every
/// field, every variant, every method row, every parameter. Selecting it inks
/// the blast radius and lifts the plate; it opens nothing, because nothing was
/// ever closed (user decision, 2026-08-20).
///
/// The plate states no size of its own. It fills the node box the layout
/// measured — `width: 100%`, `height: 100%` in the stylesheet. Sizing it inline
/// instead cannot work: dioxus's interpreter re-applies every inline property a
/// new `style` string leaves out (so that separately-set `style:` properties
/// survive a whole-attribute write), so dropping `height` from the string does
/// not drop it from the element.
#[component]
fn MarkPlate(view: MeasuredMark, selected: bool) -> Element {
    let nav = use_navigator();
    let to = if selected {
        Route::SurfaceOverview {}
    } else {
        mark_route(&view.path, &view.label)
    };
    let title = if selected {
        format!(
            "{} {} — {} · selected · click again to deselect",
            view.decl, view.name, view.locator
        )
    } else {
        format!("{} {} — {} · select it", view.decl, view.name, view.locator)
    };
    let folds = &view.folds;
    let push = to.clone();
    rsx! {
        a {
            class: "data-mark",
            class: if view.is_static { "is-root" },
            class: if view.is_fn { "is-sig" },
            class: if view.letter.is_some() { "is-diff" },
            class: if view.ghost { "is-ghost" },
            href: to.to_string(),
            title: "{title}",
            onclick: move |e: Event<MouseData>| {
                e.prevent_default();
                e.stop_propagation();
                nav.push(push.clone());
            },
            header { class: "dm-head",
                span { class: "dm-kw", "{view.decl}" }
                span {
                    class: "dm-nm",
                    class: if view.is_enum { "is-sum" },
                    class: if view.is_fn { "is-fn" },
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
            }
            // A static's declared type stands under its name; a function's
            // return type stands under its parameters, where rust writes it.
            if !view.ty.is_empty() && !view.is_fn {
                p { class: "dm-ty",
                    for (j , (class , run , held)) in spans(&view.ty, &view.ty_target).into_iter().enumerate() {
                        span {
                            key: "{j}",
                            class: if !class.is_empty() { "{class}" },
                            class: if held { "dm-held" },
                            "{run}"
                        }
                    }
                }
            }
            for (i , row) in view.fields.iter().enumerate() {
                p { key: "{i}", class: "dm-row",
                    class: if !row.state.class().is_empty() { "{row.state.class()}" },
                    if let Some(mk) = row.state.marker() {
                        span { class: "dm-mk", "{mk}" }
                    }
                    span { class: "dm-fname", "{row.name}: " }
                    for (j , (class , run , held)) in spans(&row.decl, &row.target).into_iter().enumerate() {
                        span {
                            key: "{j}",
                            class: if !class.is_empty() { "{class}" },
                            class: if held { "dm-held" },
                            "{run}"
                        }
                    }
                }
            }
            if !view.ty.is_empty() && view.is_fn {
                p { class: "dm-ty",
                    for (j , (class , run , held)) in spans(&view.ty, &view.ty_target).into_iter().enumerate() {
                        span {
                            key: "{j}",
                            class: if !class.is_empty() { "{class}" },
                            class: if held { "dm-held" },
                            "{run}"
                        }
                    }
                }
            }
            for (i , row) in view.variants.iter().enumerate() {
                p { key: "v{i}", class: "dm-var",
                    class: if !row.state.class().is_empty() { "{row.state.class()}" },
                    if let Some(mk) = row.state.marker() {
                        span { class: "dm-mk", "{mk}" }
                    }
                    for (j , (class , run , held)) in spans(&row.decl, &row.target).into_iter().enumerate() {
                        span {
                            key: "{j}",
                            class: if !class.is_empty() { "{class}" },
                            class: if held { "dm-held" },
                            "{run}"
                        }
                    }
                }
            }
            // The second band: what the type promises, under a rule that
            // says the shape above it has ended.
            if !view.methods.is_empty() {
                div { class: "dm-band",
                    for (i , row) in view.methods.iter().enumerate() {
                        p { key: "m{i}", class: "dm-sig",
                            class: if !row.state.class().is_empty() { "{row.state.class()}" },
                            if let Some(mk) = row.state.marker() {
                                span { class: "dm-mk", "{mk}" }
                            }
                            for (j , (class , run , held)) in spans(&row.decl, &row.target).into_iter().enumerate() {
                                span {
                                    key: "{j}",
                                    class: if !class.is_empty() { "{class}" },
                                    class: if held { "dm-held" },
                                    "{run}"
                                }
                            }
                        }
                    }
                }
            }
            if !folds.is_empty() {
                div { class: "dm-folds",
                    for (i , fold) in folds.iter().enumerate() {
                        p { key: "{i}", class: "dm-fold", "{fold}" }
                    }
                }
            }
        }
    }
}

/// Node view for the surface chart.
#[component]
fn SurfaceNode(ctx: NodeViewCtx<SurfaceNodeData>, selected: bool) -> Element {
    // Read before the match: a node's kind can change under one component, and
    // a hook behind a branch would change with it.
    let mut folds = use_code().folds;
    match ctx.node.data.clone() {
        SurfaceNodeData::Mark(view) => rsx! {
            MarkPlate { view: *view, selected }
        },
        SurfaceNodeData::Fold(row) => match row.unfolds.clone() {
            // The row a folded module left behind is the way back into it:
            // the border's mark says the same thing, and this is the target
            // a reader's eye is already on.
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

/// The ground: crate and module frames, each with its label chipped onto its
/// own border. A frame is a container, so it states no counts — its types are
/// on the paper to be counted, and what it does not draw has a row of its own.
///
/// The border is the module's own control, and it answers two gestures. The
/// line itself — and the label chipped onto it — **selects** the module: the
/// same reading a mark gets, one altitude out. Everything inside the boundary
/// keeps full ink, whatever crosses it reads a step behind, and the other
/// modules recede. The mark at the border's other end **folds** it: the whole
/// module leaves the paper and one counted row stands where it was. A fold is
/// a re-layout, not a reading — the chart is drawn again around what is left —
/// which is exactly why the two gestures are two marks and never one.
#[component]
fn FrameLayer(frames: Vec<FrameView>, kin: Option<SurfaceKin>) -> Element {
    let nav = use_navigator();
    let mut folds = use_code().folds;
    // A boundary's whole reading, drawn: its tint, the band of paper the
    // selection is caught on, its label, and its fold mark.
    let boundary = |f: &FrameView| -> Element {
        let class = kin.as_ref().map_or("", |k| k.frame_class(f.id));
        let chosen = class == "is-sel";
        // Selecting the chosen module again lets it go, the way clicking a
        // selected block does.
        let to = match chosen {
            true => Route::SurfaceOverview {},
            false => mod_route(f.key.clone()),
        };
        // Each gesture owns what it needs: an event handler outlives the
        // frame it was drawn from.
        let (clicked, pressed) = (to.clone(), to);
        let (shut, shut_key) = (f.key.clone(), f.key.clone());
        let words = f.words.clone();
        // A crate frame is a boundary too, and it is not a module: the words
        // the border says have to know which one the reader is standing on.
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
                // Only a frame the paper names can be folded: a fold says
                // which module went away, and a nameless crate frame in a
                // one-crate workspace is the whole chart.
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

/// One curved hairline, bowed toward open paper, with its arrowhead resting on
/// the end the change travels to. The two families bow to opposite sides, so a
/// pair of types that both holds and references never draws its two edges on
/// top of each other.
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

/// The arrowhead as a filled triangle resting on `b`, pointing along the
/// curve's end tangent.
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

/// What one wire is saying past its family: the diff's own ink, the fold, the
/// hover, and the selection's reading. A hold inside the blast radius keeps
/// full pressure; a uses edge touching the selection keeps its own beside it;
/// either family inks its folded wires back in for as long as the reason
/// stands, a moment for a hover and indefinitely for a selection. Everything
/// else recedes with the unrelated marks.
fn wire_classes(
    w: &WireView,
    is_ref: bool,
    hot: Option<Anchor>,
    kin: Option<&SurfaceKin>,
) -> Vec<&'static str> {
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

/// Both edge families as one engraved layer, over the frame tints and under the
/// blocks: the uses family first and lighter, the published surface over
/// it. Hovering either end of a wire brings it up to full ink, and so does
/// selecting either end — which is how a folded wire is given back, once in
/// passing and once for as long as the reader wants it.
#[component]
fn WireLayer(
    holds: Vec<WireView>,
    ties: Vec<WireView>,
    hot: Signal<Option<Anchor>>,
    kin: Option<SurfaceKin>,
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

/// Chrome insets at the surface altitude: the cartouche column on the left, and —
/// while a type is selected — the selection sheet on the right. The narrow
/// layout docks the sheet at the foot and stays a serviceable fallback.
fn chrome_insets(narrow: bool, panel: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (312.0, 20.0, 70.0, 12.0)
    } else {
        (56.0, if panel { 330.0 } else { 24.0 }, 24.0, 284.0)
    }
}

/// Below this the block letters stop being letters; the reviewer pans instead.
const MIN_CHART_ZOOM: f64 = 0.22;

/// The camera as the reviewer last left it. Session state that must survive
/// route-variant remounts, like the code map's camera: opening a definition
/// plate unmounts the chart, and coming back must give the reader back their
/// own pan and zoom, not a fresh framing — the camera carries the mental map
/// (the Kept-Ground rule). `f` still refits on demand. Provided as a context
/// by the atlas shell, which outlives every remount.
#[derive(Clone, Copy)]
pub(crate) struct SurfaceCamera {
    pub(crate) viewport: Signal<Option<Viewport>>,
}

impl SurfaceCamera {
    pub(crate) fn new() -> Self {
        Self {
            viewport: Signal::new(None),
        }
    }
}

fn frame_chart(
    flow: dioxus_flow::prelude::FlowHandle<SurfaceNodeData>,
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

/// Keyboard at the surface altitude: `f` refits, Escape deselects; `←` and `→`
/// retrace the trail from the shell, as they do on every route.
const SURFACE_KEYS_JS: &str = r#"
if (window.__slopeKeys) {
    document.removeEventListener('keydown', window.__slopeKeys);
}
window.__slopeKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (['f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopeKeys);
"#;

/// The surface chart, mounted for `/surface`.
#[component]
pub(crate) fn SurfaceChart(graph: CodeGraph, sel: Option<SurfaceSel>) -> Element {
    let code = use_code();
    let camera = use_context::<SurfaceCamera>();
    let flow = dioxus_flow::use_flow_handle::<SurfaceNodeData>();
    let nav = use_navigator();

    // `graph` is a prop, not a signal; the two toggles are signals and track
    // themselves — the reading moves which ties rest, the doors move which
    // types are drawn at all, so this re-seats on either.
    let chart = use_memo(use_reactive((&graph,), {
        move |(graph,)| {
            let model = SurfaceModel::build(
                &graph,
                *code.ref_dir.read(),
                *code.doors.read(),
                &code.folds.read(),
            );
            SurfaceDrawing::from(&model)
        }
    }));

    // The route's selection, resolved to the chart's own anchors. A mark's
    // reading is the mark, its transitive holders, what it directly holds, and
    // the far ends of its uses edges; a module's is its boundary and one hop
    // across it. `None` while nothing is selected, or when the route names a
    // type or a module this survey does not draw.
    let kin: Memo<Option<SurfaceKin>> = use_memo(use_reactive((&sel,), move |(sel,)| {
        let drawing = chart.read();
        let pairs = |wires: &[WireView]| -> Vec<(Anchor, Anchor)> {
            wires.iter().map(|w| (w.a, w.b)).collect()
        };
        match sel? {
            SurfaceSel::Mark(path, label) => {
                let id = drawing.nodes.iter().find_map(|n| match &n.data {
                    SurfaceNodeData::Mark(m) if m.path == path && m.label == label => Some(m.id),
                    _ => None,
                })?;
                Some(SurfaceKin::read(
                    Anchor::Mark(id),
                    &pairs(&drawing.holds),
                    &pairs(&drawing.ties),
                ))
            }
            SurfaceSel::Mod(key) => {
                let frame = drawing.frames.iter().find(|f| f.key == key)?.id;
                Some(SurfaceKin::read_mod(
                    frame,
                    &drawing.frames,
                    &drawing.homes,
                    &pairs(&drawing.holds),
                    &pairs(&drawing.ties),
                ))
            }
        }
    }));

    // Whether a selection stands, for the keyboard hook and the pane click —
    // both outlive any one render, so they read a signal, not the prop.
    let sel_on: Signal<bool> = use_signal(|| false);
    use_effect(use_reactive((&sel.is_some(),), move |(on,)| {
        let mut sel_on = sel_on;
        if *sel_on.peek() != on {
            sel_on.set(on);
        }
    }));

    let nodes: Signal<Vec<FlowNode<SurfaceNodeData>>> = use_signal(Vec::new);
    let framed = use_signal(|| false);
    let mut hot: Signal<Option<Anchor>> = use_signal(|| None);
    // True once the flow's core is live; the camera mirror below waits on it.
    let core_live: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        let drawing = chart();
        let mut nodes = nodes;
        nodes.set(drawing.nodes);
        // Camera discipline: the first paint seats the reader where they left
        // the chart, and frames it only when there is nothing to give back (a
        // fresh session). After that, only an explicit refit moves it —
        // selecting and deselecting never do.
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
                // The canvas mounts on its own beat; wait for its core
                // (bounded) rather than framing into the void.
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

    // Mirror every camera move into the store, so the next mount can give the
    // reader back their place. The store has no reactive readers; the mount
    // logic peeks it, so per-frame writes during a pan or glide stay cheap.
    use_effect(move || {
        if !core_live() {
            return;
        }
        let Some(core) = flow.core() else { return };
        let mut saved = camera.viewport;
        saved.set(Some(*core.viewport.read()));
    });

    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(SURFACE_KEYS_JS);
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
                        nav.push(Route::SurfaceOverview {});
                    }
                    _ => {}
                }
            }
        });
    });

    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);
    // Only a mark's reading opens a sheet, so only a mark's reading owes the
    // fit an inset on the right.
    let panel = matches!(sel, Some(SurfaceSel::Mark(..)));

    rsx! {
        div { class: "absolute inset-0",
            Flow {
                nodes,
                edges,
                fit_view: false,
                handle: flow,
                nodes_draggable: false,
                delete_key: false,
                // A trackpad's two-finger travel is a pan, as every canvas
                // tool reads it; pinch (ctrl/meta wheel) zooms at the pointer.
                pan_on_scroll: true,
                // Bare paper deselects, the way Escape does.
                on_pane_click: move |_| {
                    if *sel_on.peek() {
                        nav.push(Route::SurfaceOverview {});
                    }
                },
                node_view: move |ctx: NodeViewCtx<SurfaceNodeData>| {
                    let anchor = ctx.node.data.anchor();
                    let kin_class = kin
                        .read()
                        .as_ref()
                        .map(|k| k.node_class(anchor))
                        .unwrap_or("");
                    let selected = kin_class == "is-sel";
                    // On a dirty chart, whatever the diff never touched rests
                    // at a lighter pressure; a selection's own ink outranks it.
                    let rest = kin_class.is_empty()
                        && chart.read().dirty
                        && !ctx.node.data.touched();
                    rsx! {
                        div {
                            class: "data-node",
                            class: if !kin_class.is_empty() { "{kin_class}" },
                            class: if rest { "is-rest" },
                            onmouseenter: move |_| hot.set(Some(anchor)),
                            onmouseleave: move |_| hot.set(None),
                            SurfaceNode { ctx, selected }
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
                        holds: chart.read().holds.clone(),
                        ties: chart.read().ties.clone(),
                        hot,
                        kin: kin(),
                    }
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
    use crate::api::{ItemKind, Vis};

    fn mark(name: &str, kind: ItemKind, fields: Vec<(&str, &str, &str)>) -> SurfaceMark {
        SurfaceMark {
            id: 0,
            frame: 0,
            kind,
            vis: Vis::Pub,
            name: name.to_string(),
            label: name.to_string(),
            path: "src/api.rs".to_string(),
            line: 10,
            delta: crate::api::Delta::Same,
            ghost: false,
            fields: fields
                .into_iter()
                .map(|(name, decl, target)| FieldRow {
                    name: name.to_string(),
                    decl: decl.to_string(),
                    target: target.to_string(),
                    state: RowState::Same,
                })
                .collect(),
            variants: Vec::new(),
            methods: Vec::new(),
            ty: String::new(),
            ty_target: String::new(),
            unseen_users: 0,
            unseen_uses: 0,
            held_by: 0,
            named_by: 0,
        }
    }

    #[test]
    fn a_block_is_tall_enough_for_every_line_it_draws() {
        let bare = MeasuredMark::from(&mark("Wire", ItemKind::Struct, vec![]));
        let held = MeasuredMark::from(&mark(
            "Wire",
            ItemKind::Struct,
            vec![("items", "Vec<ItemMark>", "ItemMark")],
        ));
        assert!(held.size.1 > bare.size.1);
        assert!(bare.size.0 >= MARK_MIN_W && bare.size.0 <= MARK_MAX_W);
    }

    /// A sum type is its variant list, so the block quotes every variant and
    /// grows to hold them. Nothing is counted at the foot: there is no hidden
    /// row left to count.
    #[test]
    fn a_long_variant_list_is_drawn_whole_and_never_counted() {
        let row = FieldRow {
            name: String::new(),
            decl: "File(String, String)".to_string(),
            target: String::new(),
            state: RowState::Same,
        };
        let mut long = mark("Tok", ItemKind::Enum, vec![]);
        long.variants = vec![row.clone(); 20];
        let bare = MeasuredMark::from(&mark("Tok", ItemKind::Enum, vec![]));
        let short = {
            let mut m = mark("Tok", ItemKind::Enum, vec![]);
            m.variants = vec![row.clone(); 8];
            MeasuredMark::from(&m)
        };
        let view = MeasuredMark::from(&long);
        // Every row past the old cap of eight is still one row of height.
        assert!(view.size.1 > short.size.1);
        assert!(short.size.1 > bare.size.1);
        assert_eq!(view.variants.len(), 20);
        assert!(view.folds.is_empty());
        // An enum's name takes the sum-type color; a struct's does not.
        assert!(view.is_enum);
        assert!(!MeasuredMark::from(&mark("Wire", ItemKind::Struct, vec![])).is_enum);
    }

    /// The block is as wide as its widest row, whichever row that is and
    /// however far down the list it stands — every one of them is drawn, so
    /// every one of them is measured.
    #[test]
    fn the_widest_row_sets_the_block_width_wherever_it_stands() {
        let mut wide = mark("Wire", ItemKind::Struct, vec![]);
        wide.fields = (0..12)
            .map(|i| FieldRow {
                name: format!("f{i}"),
                // The longest row is the last one in the block.
                decl: if i == 11 {
                    "HashMap<String, Vec<ItemMark>>".to_string()
                } else {
                    "u32".to_string()
                },
                target: String::new(),
                state: RowState::Same,
            })
            .collect();
        let view = MeasuredMark::from(&wide);
        assert_eq!(view.fields.len(), 12);
        assert!(view.folds.is_empty());
        assert!(view.size.0 >= text_w("f11: HashMap<String, Vec<ItemMark>>", 10.0));
    }

    /// The only counted lines left are the chart's own: a fan-in it will not
    /// draw as ink, said in words instead.
    #[test]
    fn the_foot_counts_the_fan_in_and_nothing_the_block_quotes() {
        let mut hub = mark("Id", ItemKind::Struct, vec![("raw", "u32", "")]);
        hub.held_by = 6;
        hub.named_by = 2;
        let view = MeasuredMark::from(&hub);
        assert_eq!(
            view.folds,
            vec![
                "held by 6 types".to_string(),
                "named by 2 signatures".to_string()
            ]
        );
    }

    /// A static's declared type bolds the workspace type it holds, and only
    /// that: `GlobalSignal<Option<Viewport>>` reaches a dependency's type, so
    /// nothing in the line is bold and nothing on the chart points at it.
    #[test]
    fn a_static_bolds_only_a_workspace_type_it_holds() {
        let mut held = mark("TRAIL", ItemKind::Static, vec![]);
        held.ty = "GlobalSignal<Trail>".to_string();
        held.ty_target = "Trail".to_string();
        let bold: Vec<String> = spans(&held.ty, &held.ty_target)
            .into_iter()
            .filter(|(_, _, held)| *held)
            .map(|(_, run, _)| run)
            .collect();
        assert_eq!(bold, vec!["Trail".to_string()]);

        let mut outside = mark("CAMERA", ItemKind::Static, vec![]);
        outside.ty = "GlobalSignal<Option<Viewport>>".to_string();
        assert!(
            spans(&outside.ty, &outside.ty_target)
                .iter()
                .all(|(_, _, held)| !held)
        );
    }

    /// A function's block is a type's block wearing a signature: parameters
    /// where a type keeps its fields, and the return type under them with
    /// rust's own arrow — over them would read as a type, not a declaration.
    #[test]
    fn a_signature_block_returns_under_its_parameters() {
        let mut survey = mark(
            "survey",
            ItemKind::Fn,
            vec![("graph", "&CodeGraph", "CodeGraph")],
        );
        survey.ty = "Nut".to_string();
        survey.ty_target = "Nut".to_string();
        let view = MeasuredMark::from(&survey);
        assert!(view.is_fn);
        assert_eq!(view.ty, "-> Nut");
        // The arrow is punctuation; what it hands back is still the bold run.
        let bold: Vec<String> = spans(&view.ty, &view.ty_target)
            .into_iter()
            .filter(|(_, _, held)| *held)
            .map(|(_, run, _)| run)
            .collect();
        assert_eq!(bold, vec!["Nut".to_string()]);

        // A long parameter list is quoted whole, like every other row family.
        let mut wide = mark("survey", ItemKind::Fn, vec![]);
        wide.fields = (0..11)
            .map(|i| FieldRow {
                name: format!("p{i}"),
                decl: "u32".to_string(),
                target: String::new(),
                state: RowState::Same,
            })
            .collect();
        let wide = MeasuredMark::from(&wide);
        assert_eq!(wide.fields.len(), 11);
        assert!(wide.folds.is_empty());
        assert!(wide.size.1 > view.size.1);
    }

    /// Selecting a mark is how a reader reads its neighbourhood, so the whole
    /// uses neighbourhood has to be on the paper while the selection stands:
    /// every dashed edge touching it, and the block at the other end of each.
    /// The two families stay apart — a uses neighbour is never kin — but a
    /// neighbour is never dimmed either, or the edge would point at nothing
    /// the reader can read.
    #[test]
    fn a_selection_pins_its_whole_uses_neighbourhood() {
        let (sel, holder, caller, callee, stranger) = (
            Anchor::Mark(0),
            Anchor::Mark(1),
            Anchor::Mark(2),
            Anchor::Mark(3),
            Anchor::Mark(4),
        );
        let kin = SurfaceKin::read(
            sel,
            &[(sel, holder)],
            &[(sel, caller), (callee, sel), (callee, stranger)],
        );

        assert_eq!(kin.node_class(sel), "is-sel");
        assert_eq!(kin.node_class(holder), "is-kin");
        // Both ways round: the mark's own users and what it uses are equally
        // its neighbours, and the arrowhead decides neither.
        assert_eq!(kin.node_class(caller), "is-near");
        assert_eq!(kin.node_class(callee), "is-near");
        assert_eq!(kin.node_class(stranger), "is-dim");
        // Implementation coupling stops at one hop: a neighbour's own
        // neighbour is nothing to this selection.
        assert!(!kin.near.contains(&stranger));
        // And it never joins the blast radius, which is the holds walk alone.
        assert!(kin.up.iter().chain(kin.down.iter()).all(|a| *a == holder));

        assert!(kin.tie_near(sel, caller));
        assert!(kin.tie_near(callee, sel));
        assert!(!kin.tie_near(callee, stranger));
        // A holds edge is read by the other rule; the two never cross.
        assert!(kin.wire_kin(sel, holder));
        assert!(!kin.wire_kin(callee, stranger));
    }

    /// Selecting a module boundary reads the boundary: everything inside it
    /// keeps full ink whatever module it was written in, one hop across the
    /// line reads a step behind, and every other module recedes — its frame
    /// with it. The frames the boundary is drawn inside never recede: they are
    /// the paper it stands on, and dimming them would say the opposite of what
    /// the chosen line says.
    #[test]
    fn a_module_reading_keeps_its_boundary_and_recedes_the_others() {
        // `slope` holds `views`, which holds `views::surface`; `api` stands
        // beside `views` in the crate.
        let frame = |id: u32, parent: Option<u32>, key: &[&str]| FrameView {
            id,
            parent,
            at: Placed {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            label: None,
            label_w: 0.0,
            key: key.iter().map(|s| s.to_string()).collect(),
            words: key.join("::"),
            folded: false,
        };
        let frames = vec![
            frame(0, None, &["slope"]),
            frame(1, Some(0), &["slope", "views"]),
            frame(2, Some(1), &["slope", "views", "surface"]),
            frame(3, Some(0), &["slope", "api"]),
        ];
        // One mark per frame, and the deep frame's counted row.
        let (root, own, deep, stranger) = (
            Anchor::Mark(0),
            Anchor::Mark(1),
            Anchor::Mark(2),
            Anchor::Mark(3),
        );
        let row = Anchor::Private(2);
        let homes: HashMap<Anchor, u32> = [(root, 0), (own, 1), (deep, 2), (stranger, 3), (row, 2)]
            .into_iter()
            .collect();
        let kin = SurfaceKin::read_mod(
            1,
            &frames,
            &homes,
            &[(deep, stranger), (own, deep)],
            &[(root, deep)],
        );

        // Inside the line, however deep — the nested module's row included.
        assert_eq!(kin.node_class(own), "is-kin");
        assert_eq!(kin.node_class(deep), "is-kin");
        assert_eq!(kin.node_class(row), "is-kin");
        // One hop over it, either family.
        assert_eq!(kin.node_class(stranger), "is-near");
        assert_eq!(kin.node_class(root), "is-near");

        // The boundary itself, the module inside it, and the crate frame
        // holding it: none of them recede. The module beside it does.
        assert_eq!(kin.frame_class(1), "is-sel");
        assert_eq!(kin.frame_class(2), "");
        assert_eq!(kin.frame_class(0), "");
        assert_eq!(kin.frame_class(3), "is-dim");

        // The wires: a solid line the boundary touches is the module's own
        // published surface, and a dashed one is what leans on it. A line
        // between two strangers is neither.
        assert!(kin.wire_kin(own, deep));
        assert!(kin.wire_kin(deep, stranger));
        assert!(!kin.wire_kin(root, stranger));
        assert!(kin.tie_near(root, deep));
        assert!(!kin.tie_near(root, stranger));

        // A mark's reading leaves the ground alone: the blast radius is a walk
        // between blocks, not a place on the paper.
        let marks = SurfaceKin::read(own, &[(own, root)], &[]);
        assert_eq!(marks.frame_class(3), "");
        assert_eq!(marks.node_class(own), "is-sel");
    }

    /// A folded uses edge is drawn `display: none` until something inks it
    /// back in. Hover does it in passing; a selection has to do it durably, or
    /// following the edge to its far end takes the edge away.
    #[test]
    fn a_folded_tie_touching_the_selection_is_inked_in() {
        let (sel, far) = (Anchor::Mark(0), Anchor::Mark(1));
        let kin = SurfaceKin::read(sel, &[], &[(sel, far)]);
        let folded = WireView {
            key: "folded".to_string(),
            from: Point::new(0.0, 0.0),
            to: Point::new(10.0, 10.0),
            a: sel,
            b: far,
            label: None,
            width: 1.0,
            // The resting cap left this one off the paper.
            rest: false,
            class: "is-ref",
            event: "",
        };
        let classes = wire_classes(&folded, true, None, Some(&kin));
        assert!(classes.contains(&"is-folded"));
        assert!(classes.contains(&"is-near"));
        assert!(!classes.contains(&"is-dim"));
        // The same wire away from the selection keeps receding, and stays
        // folded away with it.
        let elsewhere = WireView {
            a: Anchor::Mark(7),
            b: Anchor::Mark(8),
            ..folded.clone()
        };
        let classes = wire_classes(&elsewhere, true, None, Some(&kin));
        assert!(classes.contains(&"is-dim"));
        assert!(!classes.contains(&"is-near"));
        // With nothing selected the chart is the resting chart: the fold
        // stands, and hover is still the one way back — unchanged, and still
        // the only reading that costs the reader a held cursor.
        assert_eq!(wire_classes(&folded, true, None, None), vec!["is-folded"]);
        assert_eq!(
            wire_classes(&folded, true, Some(far), None),
            vec!["is-folded", "is-hot"]
        );
    }

    #[test]
    fn the_held_type_is_the_bold_run_of_its_quotation() {
        let runs = spans("Vec<FileDetail>", "FileDetail");
        let held: Vec<&str> = runs
            .iter()
            .filter(|(_, _, held)| *held)
            .map(|(_, run, _)| run.as_str())
            .collect();
        assert_eq!(held, vec!["FileDetail"]);
        // A name inside a longer name is not the name.
        assert!(
            spans("Vec<FileDetail>", "Detail")
                .iter()
                .all(|(_, _, held)| !held)
        );
        // Token classes follow the plate's grammar: uppercase-initial names
        // are types, the rest of the row is idents and punctuation.
        let classes: Vec<(&str, &str)> = runs
            .iter()
            .map(|(class, run, _)| (*class, run.as_str()))
            .collect();
        assert_eq!(
            classes,
            vec![
                ("tok-type", "Vec"),
                ("tok-punct", "<"),
                ("tok-type", "FileDetail"),
                ("tok-punct", ">")
            ]
        );
    }
}
