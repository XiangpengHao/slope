//! The data chart: type marks, module frames, holding edges, reference ties.
//!
//! One block per type the workspace declares, seated in the frame of the module
//! that declares it, with a hairline from every held type to its holder. The
//! block is measured before it is placed, so its plate and its box agree to the
//! pixel, and the whole layout is a pure function of the survey — the same
//! workspace always draws the same chart.
//!
//! Two edge families share the paper. **Holds** is structure and is always
//! drawn: kind is dash grammar and the wrapper writes its own word on the line.
//! **References** is a reading of the same item-level edges the code map draws,
//! lifted to types and thinned by the cartouche's toggle; it rests at half ink
//! under the holds edges so the two never read as one family.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size, Viewport};

use crate::Route;
use crate::api::{CodeGraph, HoldKind};
use crate::views::codemap::chrome::{decl_words, plural};
use crate::views::codemap::map::{narrow_viewport, prefers_reduced_motion, tie_ends, window_size};
use crate::views::codemap::tree::{Placed, text_w};
use crate::views::codemap::use_code;
use crate::views::datamap::data_type_route;
use crate::views::datamap::layout::{self, DataLayout, Sizes};
use crate::views::datamap::model::{Anchor, DataMark, DataModel, FieldRow, upstream};

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
/// The counted folds: the rule above them, then one line each.
const FOLDS_TOP: f64 = 6.0;
const FOLD_H: f64 = 12.0;
const LOC_H: f64 = 14.0;
const MARK_MIN_W: f64 = 152.0;
const MARK_MAX_W: f64 = 300.0;
/// A counted fold row standing in for what a frame does not draw.
const ROW_MIN_W: f64 = 132.0;
const ROW_FOLD_H: f64 = 22.0;
/// What ragged line breaks cost over a straight width ratio.
const WRAP_SLACK: f64 = 1.12;

/// A reference tie's weight follows the code map's rule exactly: the more
/// references a pair of types has between them, the firmer the hairline.
fn tie_width(count: u32) -> f64 {
    (0.55 + count as f64 * 0.13).min(2.8)
}

/// A hold's weight. Structure is drawn at one steady pressure; the ties rest
/// lighter under it, so the two families never read as one.
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

/// One mark, measured and ready to engrave.
#[derive(Clone, PartialEq)]
pub struct MarkView {
    pub id: u32,
    /// `pub struct`, `static` — what rust writes in front of the name.
    pub decl: String,
    pub name: String,
    pub changed: bool,
    pub is_static: bool,
    pub fields: Vec<FieldRow>,
    /// A static's declared type, as written.
    pub ty: String,
    /// An enum's variants as written, one row each (the row text in `decl`).
    pub variants: Vec<FieldRow>,
    /// Every counted line at the foot, in words.
    pub folds: Vec<String>,
    pub locator: String,
    pub path: String,
    pub label: String,
    pub size: (f64, f64),
}

/// A frame's counted fold row: what it does not draw, and how much of it.
#[derive(Clone, PartialEq)]
pub struct FoldView {
    pub anchor: Anchor,
    pub words: String,
    pub size: (f64, f64),
}

/// One node on the data chart.
#[derive(Clone, PartialEq)]
pub enum DataNodeData {
    Mark(MarkView),
    Fold(FoldView),
}

impl DataNodeData {
    fn anchor(&self) -> Anchor {
        match self {
            DataNodeData::Mark(m) => Anchor::Mark(m.id),
            DataNodeData::Fold(f) => f.anchor,
        }
    }
}

/// A frame, placed, with the label it wears on its border.
#[derive(Clone, PartialEq)]
pub struct FrameView {
    pub id: u32,
    pub at: Placed,
    pub label: Option<String>,
}

/// One drawn edge — a hold or a reference tie — with its ends already found.
#[derive(Clone, PartialEq)]
pub struct WireView {
    pub key: String,
    pub from: Point,
    pub to: Point,
    pub a: Anchor,
    pub b: Anchor,
    /// The word engraved on the line: a wrapper for a hold, a count for a tie.
    pub label: Option<String>,
    pub width: f64,
    /// Drawn at rest; a folded wire inks in when either end is hovered.
    pub rest: bool,
    /// The kind's dash grammar, as a CSS class.
    pub class: &'static str,
}

/// Everything one build of the chart draws.
#[derive(Clone, PartialEq)]
pub struct Built {
    pub nodes: Vec<FlowNode<DataNodeData>>,
    pub frames: Vec<FrameView>,
    pub holds: Vec<WireView>,
    pub ties: Vec<WireView>,
    pub frame: Option<Rect>,
}

/// The selection's ink. One chosen mark; everything a shape change to it could
/// reach, walking holds edges holder-ward (the blast radius); and what it
/// directly holds, one hop the other way. While a selection stands the rest of
/// the chart recedes to a lighter pressure — a reading, never a re-layout, and
/// the camera does not move.
#[derive(Clone, PartialEq)]
pub struct KinView {
    pub sel: Anchor,
    /// Transitive holders. A counted fold row can join — its edge is drawn —
    /// but the walk ends there.
    pub up: HashSet<Anchor>,
    /// Directly held types.
    pub down: HashSet<Anchor>,
}

impl KinView {
    fn node_class(&self, a: Anchor) -> &'static str {
        if a == self.sel {
            "is-sel"
        } else if self.up.contains(&a) || self.down.contains(&a) {
            "is-kin"
        } else {
            "is-dim"
        }
    }

    /// A holds wire inside the selection's ink: a link in the chain toward the
    /// holders, or the one hop down to what the selection holds.
    fn wire_kin(&self, held: Anchor, holder: Anchor) -> bool {
        let upward = |x: Anchor| x == self.sel || self.up.contains(&x);
        (upward(held) && upward(holder)) || (holder == self.sel && self.down.contains(&held))
    }

    /// A reference tie the selection keeps at its own ink: one that touches the
    /// selected mark itself. Ties are a reading, not structure, so they never
    /// join the blast radius — they just escape the receding.
    fn tie_kept(&self, a: Anchor, b: Anchor) -> bool {
        a == self.sel || b == self.sel
    }
}

/// The counted words a mark writes at its foot. Every one of them stands where
/// something is hidden; a mark that hides nothing writes none.
fn fold_words(mark: &DataMark) -> Vec<String> {
    let mut folds = Vec::new();
    if mark.more_fields > 0 {
        folds.push(format!("+ {}", plural(mark.more_fields as usize, "more field")));
    }
    if mark.more_variants > 0 {
        folds.push(format!(
            "+ {}",
            plural(mark.more_variants as usize, "more variant")
        ));
    }
    if mark.held_by > 0 {
        folds.push(format!("held by {}", plural(mark.held_by as usize, "type")));
    }
    folds
}

/// A mark, measured. The width is the widest line it must not clip; the height
/// follows from how those lines wrap inside it.
fn measure(mark: &DataMark) -> MarkView {
    let decl = decl_words(mark.vis, mark.kind);
    let head = format!("{decl} {}", mark.name);
    let locator = mark.locator();
    let folds = fold_words(mark);

    let mut widest = text_w(&head, 10.5) + if mark.changed { 12.0 } else { 0.0 };
    widest = widest.max(text_w(&locator, 8.5));
    // A long row clips at the block's own maximum rather than stretching it
    // past the paper's patience.
    let wrapping = MARK_MAX_W - PAD_X;
    for row in &mark.fields {
        widest = widest.max(text_w(&format!("{}: {}", row.name, row.decl), 10.0).min(wrapping));
    }
    for row in &mark.variants {
        widest = widest.max(text_w(&row.decl, 10.0).min(wrapping));
    }
    for fold in &folds {
        widest = widest.max(text_w(fold, 9.0));
    }
    if !mark.ty.is_empty() {
        widest = widest.max(text_w(&mark.ty, 9.5).min(wrapping));
    }
    let w = (widest + PAD_X).clamp(MARK_MIN_W, MARK_MAX_W);
    let usable = w - PAD_X;

    let ty_lines = if mark.ty.is_empty() {
        0.0
    } else {
        wrapped(&mark.ty, 9.5, usable)
    };
    let fold_block = if folds.is_empty() {
        0.0
    } else {
        FOLDS_TOP + folds.len() as f64 * FOLD_H
    };
    let h = PAD_TOP
        + HEAD_H
        + ty_lines * TY_H
        + mark.fields.len() as f64 * ROW_H
        + mark.variants.len() as f64 * ROW_H
        + fold_block
        + LOC_H
        + PAD_BOTTOM;

    MarkView {
        id: mark.id,
        decl,
        name: mark.name.clone(),
        changed: mark.changed,
        is_static: mark.is_static(),
        fields: mark.fields.clone(),
        ty: mark.ty.clone(),
        variants: mark.variants.clone(),
        folds,
        locator,
        path: mark.path.clone(),
        label: mark.label.clone(),
        size: (w, h),
    }
}

/// A counted fold row, measured.
fn measure_row(anchor: Anchor, words: String) -> FoldView {
    let w = (text_w(&words, 9.5) + 20.0).clamp(ROW_MIN_W, MARK_MAX_W);
    FoldView {
        anchor,
        words,
        size: (w, ROW_FOLD_H),
    }
}

fn node_key(anchor: Anchor) -> String {
    match anchor {
        Anchor::Mark(id) => format!("m{id}"),
        Anchor::Private(frame) => format!("p{frame}"),
        Anchor::More(frame) => format!("x{frame}"),
    }
}

/// The dash grammar of a hold, as a CSS class. Kind is a line, and the wrapper
/// writes its own word beside it — no color, and nothing to memorize that the
/// legend does not draw.
fn hold_class(kind: HoldKind) -> &'static str {
    match kind {
        HoldKind::Owns => "is-owns",
        HoldKind::Shares => "is-shares",
        HoldKind::Borrows => "is-borrows",
        HoldKind::Dyn => "is-dyn",
    }
}

/// Measure everything, place it, and gather what the chart draws.
pub fn build_chart(model: &DataModel) -> Built {
    let mut sizes = Sizes::default();
    let mut views: HashMap<u32, MarkView> = HashMap::new();
    for mark in &model.marks {
        let view = measure(mark);
        sizes.marks.insert(mark.id, view.size);
        views.insert(mark.id, view);
    }
    let mut rows: HashMap<Anchor, FoldView> = HashMap::new();
    for frame in &model.frames {
        if frame.private > 0 {
            let anchor = Anchor::Private(frame.id);
            let words = format!("+ {}", plural(frame.private as usize, "private type"));
            let row = measure_row(anchor, words);
            sizes.rows.insert(anchor, row.size);
            rows.insert(anchor, row);
        }
        if frame.more > 0 {
            let anchor = Anchor::More(frame.id);
            let words = format!("+ {}", plural(frame.more as usize, "more type"));
            let row = measure_row(anchor, words);
            sizes.rows.insert(anchor, row.size);
            rows.insert(anchor, row);
        }
        if let Some(label) = frame.label(model.multi_crate) {
            sizes.labels.insert(frame.id, text_w(&label, 12.0) + 18.0);
        }
    }

    let placed: DataLayout = layout::layout(&model.frames, &sizes);

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
                DataNodeData::Mark(view.clone()),
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
        .map(|(id, at)| FrameView {
            id: *id,
            at: *at,
            label: model.frames[*id as usize].label(model.multi_crate),
        })
        .collect();

    // The arrowhead rests on the holder, so the wire runs held → holder.
    let holds: Vec<WireView> = model
        .holds
        .iter()
        .filter_map(|hold| {
            let (a, b) = (placed.rect(hold.held)?, placed.rect(hold.holder)?);
            let (from, to) = tie_ends(a, b);
            Some(WireView {
                key: hold.key(),
                from,
                to,
                a: hold.held,
                b: hold.holder,
                label: (!hold.via.is_empty()).then(|| hold.via.clone()),
                width: hold_width(hold.kind),
                rest: hold.rest,
                class: hold_class(hold.kind),
            })
        })
        .collect();

    // The reference reading: the arrowhead rests on the user, as everywhere.
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
            })
        })
        .collect();

    let frame = Rect::bounds(nodes.iter().map(|n| n.rect())).or_else(|| {
        frames
            .first()
            .map(|f| Rect::new(f.at.x, f.at.y, f.at.w, f.at.h))
    });

    Built {
        nodes,
        frames,
        holds,
        ties,
        frame,
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
fn spans(text: &str, target: &str) -> Vec<(&'static str, String, bool)> {
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
            while chars.peek().copied().is_some_and(|c| !ident(c) && c != '\'') {
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
#[component]
fn MarkPlate(view: MarkView, selected: bool) -> Element {
    let nav = use_navigator();
    let (w, h) = view.size;
    let to = if selected {
        Route::DataOverview {}
    } else {
        data_type_route(&view.path, &view.label)
    };
    let title = if selected {
        format!("{} {} — selected · click again to deselect", view.decl, view.name)
    } else {
        format!("{} {} — {} · select it", view.decl, view.name, view.locator)
    };
    let push = to.clone();
    rsx! {
        a {
            class: "data-mark",
            class: if view.is_static { "is-root" },
            style: "width: {w}px; height: {h}px;",
            href: to.to_string(),
            title: "{title}",
            onclick: move |e: Event<MouseData>| {
                e.prevent_default();
                e.stop_propagation();
                nav.push(push.clone());
            },
            header { class: "dm-head",
                span { class: "dm-kw", "{view.decl}" }
                span { class: "dm-nm", "{view.name}" }
                if view.changed {
                    span { class: "dm-chg", title: "changed since the diff base", "M" }
                }
            }
            if !view.ty.is_empty() {
                p { class: "dm-ty",
                    for (j , (class , run , held)) in spans(&view.ty, "").into_iter().enumerate() {
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
            for (i , row) in view.variants.iter().enumerate() {
                p { key: "v{i}", class: "dm-var",
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
            if !view.folds.is_empty() {
                div { class: "dm-folds",
                    for (i , fold) in view.folds.iter().enumerate() {
                        p { key: "{i}", class: "dm-fold", "{fold}" }
                    }
                }
            }
            p { class: "dm-loc", "{view.locator}" }
        }
    }
}

/// Node view for the data chart.
#[component]
fn DataNode(ctx: NodeViewCtx<DataNodeData>, selected: bool) -> Element {
    match ctx.node.data.clone() {
        DataNodeData::Mark(view) => rsx! {
            MarkPlate { view, selected }
        },
        DataNodeData::Fold(row) => {
            let (w, h) = row.size;
            let title = match row.anchor {
                Anchor::More(_) => {
                    "the quietest types in this module, folded to fit the chart's budget; \
                     every edge that touches one lands here"
                }
                _ => {
                    "private types are never drawn; every edge that touches one lands here"
                }
            };
            rsx! {
                p {
                    class: "data-foldrow",
                    style: "width: {w}px; height: {h}px;",
                    title,
                    "{row.words}"
                }
            }
        }
    }
}

/// The ground: crate and module frames, each with its label chipped onto its
/// own border. A frame is a container, so it states no counts — its types are
/// on the paper to be counted, and what it does not draw has a row of its own.
#[component]
fn FrameLayer(frames: Vec<FrameView>) -> Element {
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for f in frames.iter() {
                g { key: "{f.id}",
                    rect {
                        class: "data-frame",
                        x: "{f.at.x}",
                        y: "{f.at.y}",
                        width: "{f.at.w}",
                        height: "{f.at.h}",
                    }
                    if let Some(label) = f.label.clone() {
                        text {
                            class: "data-frame-label",
                            x: "{f.at.x + 14.0}",
                            y: "{f.at.y}",
                            "{label}"
                        }
                    }
                }
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

/// Both edge families as one engraved layer, over the frame tints and under the
/// blocks: the reference reading first and lighter, the holding structure over
/// it. Hovering either end of a wire brings it up to full ink, which is how a
/// folded wire is given back.
#[component]
fn WireLayer(
    holds: Vec<WireView>,
    ties: Vec<WireView>,
    hot: Signal<Option<Anchor>>,
    kin: Option<KinView>,
) -> Element {
    let hot = hot();
    let wire = |w: &WireView, family: &'static str, side: f64| {
        let (d, ctrl) = curve(w.from, w.to, side);
        let head = arrowhead(w.to, ctrl, 3.4 + w.width);
        let (lx, ly) = (
            0.25 * w.from.x + 0.5 * ctrl.x + 0.25 * w.to.x,
            0.25 * w.from.y + 0.5 * ctrl.y + 0.25 * w.to.y,
        );
        let is_hot = hot.is_some_and(|h| h == w.a || h == w.b);
        // The selection's ink: a hold inside the blast radius keeps full
        // pressure (folded ones ink back in); a tie touching the selection
        // keeps its own; everything else recedes with the unrelated marks.
        let is_ref = family.ends_with("data-ref");
        let is_kin = kin
            .as_ref()
            .is_some_and(|k| !is_ref && k.wire_kin(w.a, w.b));
        let is_dim = kin.as_ref().is_some_and(|k| {
            !is_kin
                && if is_ref {
                    !k.tie_kept(w.a, w.b)
                } else {
                    true
                }
        });
        rsx! {
            g {
                key: "{w.key}",
                class: "{family} {w.class}",
                class: if !w.rest { "is-folded" },
                class: if is_hot { "is-hot" },
                class: if is_kin { "is-kin" },
                class: if is_dim { "is-dim" },
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

/// Chrome insets at the data altitude: the cartouche column on the left, and —
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
/// route-variant remounts, like the code map's globals: opening a definition
/// plate unmounts the chart, and coming back must give the reader back their
/// own pan and zoom, not a fresh framing — the camera carries the mental map
/// (the Kept-Ground rule). `f` still refits on demand.
static CAMERA: GlobalSignal<Option<Viewport>> = Signal::global(|| None);

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

/// Keyboard at the data altitude: `f` refits, Escape deselects; `←` and `→`
/// retrace the trail from the shell, as they do on every route.
const DATA_KEYS_JS: &str = r#"
if (window.__slopifyKeys) {
    document.removeEventListener('keydown', window.__slopifyKeys);
}
window.__slopifyKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (['f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopifyKeys);
"#;

/// The data chart, mounted for `/data`.
#[component]
pub fn DataChart(graph: CodeGraph, sel: Option<(String, String)>) -> Element {
    let code = use_code();
    let flow = dioxus_flow::use_flow_handle::<DataNodeData>();
    let nav = use_navigator();

    // `graph` is a prop, not a signal; the reading toggle is a signal and
    // tracks itself.
    let built = use_memo(use_reactive((&graph,), {
        move |(graph,)| {
            let model = DataModel::build(&graph, *code.ref_dir.read());
            build_chart(&model)
        }
    }));

    // The route's selection, resolved to the chart's anchors: the mark, its
    // transitive holders, and what it directly holds. `None` while nothing is
    // selected, or when the route names a type this survey does not draw.
    let kin: Memo<Option<KinView>> = use_memo(use_reactive((&sel,), move |(sel,)| {
        let (path, label) = sel?;
        let b = built.read();
        let id = b.nodes.iter().find_map(|n| match &n.data {
            DataNodeData::Mark(m) if m.path == path && m.label == label => Some(m.id),
            _ => None,
        })?;
        let at = Anchor::Mark(id);
        let pairs: Vec<(Anchor, Anchor)> = b.holds.iter().map(|w| (w.a, w.b)).collect();
        let up = upstream(&pairs, at);
        let down = pairs
            .iter()
            .filter(|(_, holder)| *holder == at)
            .map(|(held, _)| *held)
            .collect();
        Some(KinView { sel: at, up, down })
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

    let nodes: Signal<Vec<FlowNode<DataNodeData>>> = use_signal(Vec::new);
    let framed = use_signal(|| false);
    let mut hot: Signal<Option<Anchor>> = use_signal(|| None);
    // True once the flow's core is live; the camera mirror below waits on it.
    let core_live: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        let b = built();
        let mut nodes = nodes;
        nodes.set(b.nodes);
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
            let frame = b.frame;
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
                if let Some(vp) = *CAMERA.peek() {
                    flow.set_viewport(vp, 0);
                } else if let Some(frame) = frame {
                    frame_chart(flow, frame, panel, 0);
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (framed, core_live);
            if let Some(frame) = b.frame {
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
        *CAMERA.write() = Some(*core.viewport.read());
    });

    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(DATA_KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                match key.as_str() {
                    "f" => {
                        if let Some(bounds) =
                            Rect::bounds(built.peek().nodes.iter().map(|n| n.rect()))
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

    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);
    let panel = sel.is_some();

    rsx! {
        div { class: "absolute inset-0",
            Flow {
                nodes,
                edges,
                fit_view: false,
                handle: flow,
                nodes_draggable: false,
                delete_key: false,
                // Bare paper deselects, the way Escape does.
                on_pane_click: move |_| {
                    if *sel_on.peek() {
                        nav.push(Route::DataOverview {});
                    }
                },
                node_view: move |ctx: NodeViewCtx<DataNodeData>| {
                    let anchor = ctx.node.data.anchor();
                    let kin_class = kin
                        .read()
                        .as_ref()
                        .map(|k| k.node_class(anchor))
                        .unwrap_or("");
                    let selected = kin_class == "is-sel";
                    rsx! {
                        div {
                            class: "data-node",
                            class: if !kin_class.is_empty() { "{kin_class}" },
                            onmouseenter: move |_| hot.set(Some(anchor)),
                            onmouseleave: move |_| hot.set(None),
                            DataNode { ctx, selected }
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
                    FrameLayer { frames: built.read().frames.clone() }
                }
                WorldLayer { class: "data-wires",
                    WireLayer {
                        holds: built.read().holds.clone(),
                        ties: built.read().ties.clone(),
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

    fn mark(name: &str, kind: ItemKind, fields: Vec<(&str, &str, &str)>) -> DataMark {
        DataMark {
            id: 0,
            frame: 0,
            kind,
            vis: Vis::Pub,
            name: name.to_string(),
            label: name.to_string(),
            path: "src/api.rs".to_string(),
            line: 10,
            changed: false,
            fields: fields
                .into_iter()
                .map(|(name, decl, target)| FieldRow {
                    name: name.to_string(),
                    decl: decl.to_string(),
                    target: target.to_string(),
                })
                .collect(),
            more_fields: 0,
            variants: Vec::new(),
            more_variants: 0,
            ty: String::new(),
            held_by: 0,
        }
    }

    #[test]
    fn a_block_is_tall_enough_for_every_line_it_draws() {
        let bare = measure(&mark("Wire", ItemKind::Struct, vec![]));
        let held = measure(&mark(
            "Wire",
            ItemKind::Struct,
            vec![("items", "Vec<ItemMark>", "ItemMark")],
        ));
        assert!(held.size.1 > bare.size.1);
        assert!(bare.size.0 >= MARK_MIN_W && bare.size.0 <= MARK_MAX_W);
    }

    #[test]
    fn a_variant_row_raises_the_block_and_the_fold_counts_the_rest() {
        let mut long = mark("Tok", ItemKind::Enum, vec![]);
        long.variants = vec![
            FieldRow {
                name: String::new(),
                decl: "File(String, String)".to_string(),
                target: String::new(),
            };
            3
        ];
        long.more_variants = 12;
        let bare = measure(&mark("Tok", ItemKind::Enum, vec![]));
        let view = measure(&long);
        assert!(view.size.1 > bare.size.1);
        assert!(view.folds.iter().any(|f| f.contains("12 more variants")));
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
            vec![("tok-type", "Vec"), ("tok-punct", "<"), ("tok-type", "FileDetail"), ("tok-punct", ">")]
        );
    }
}
