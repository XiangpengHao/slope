//! The function chart: signature blocks tiered by call depth, drawn as a
//! section.
//!
//! One block per declaration that runs, quoting its signature the way the data
//! chart quotes a struct's fields — the receiver, every parameter, the return —
//! because a function's parameters are its fields. An **entry point** wears the
//! root's 2.5px ink left edge: this is where a chain of running begins.
//!
//! Two families run between the blocks, and only two. Solid is a **call**: at
//! this altitude a body is the declaration, so what would be body coupling one
//! rung up is structure here. Dashed and lighter is a **contract**: a trait's
//! own clause and the methods that answer it, which is what keeps the chart
//! honest about a `dyn` call it cannot follow. Both rest on the dependent — the
//! caller, the answering method — the way change travels.
//!
//! The resting plate draws the **way in**: for every mark, the one call that is
//! the shortest way something that starts reaches it. Every other call stays in
//! the set, folded, and inks back on hover or selection of either end; a
//! contract wire and a wire touching a changed declaration never fold. Drawing
//! all fifteen hundred resolved calls at rest would be exactly the hairball
//! this system forbids one rung up.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size, Viewport,
};

use crate::Route;
use crate::graph::data::{CodeGraph, ItemKind};
use crate::views::chrome::{narrow_viewport, prefers_reduced_motion, window_size};
use crate::views::func::layout::{FnLayout, Lane, Placed, Prism, Sizes};
use crate::views::func::model::{Call, CallKind, FnMark, FnModel, SigRow, Tier};
use crate::views::func::{CallDir, FnSel, Group, band_route, mark_route, mod_route, use_fns};

// ---------------------------------------------------------------------------
// Block furniture, in flow units. These numbers are the CSS in `tailwind.css`;
// move one and the other must follow.
// ---------------------------------------------------------------------------

const PAD_TOP: f64 = 6.0;
const PAD_BOTTOM: f64 = 5.0;
/// Both sides together: border and padding, left and right.
const PAD_X: f64 = 16.0;
/// How far a parameter stands in from the block's edge — rust's own indent,
/// narrowed to what a 10px quotation can spare. A row's diff marker sits in
/// this gutter, so marked and unmarked rows start their text on one column.
const ROW_INDENT: f64 = 12.0;
const HEAD_H: f64 = 16.0;
const ROW_H: f64 = 15.0;
const TAIL_H: f64 = 15.0;
const MARK_MIN_W: f64 = 152.0;
/// A row's clip width: a hundred columns of rust is a plate, not a mark.
const MARK_MAX_W: f64 = 300.0;
const WRAP_SLACK: f64 = 1.12;
/// Small-type slack: at 8.5–9px the browser rounds each glyph up, so text
/// measured with the font's exact advance clips its last characters.
const META_SLACK: f64 = 1.08;
/// One em of advance in the data face (JetBrains Mono is monospaced).
const MONO_ADVANCE: f64 = 0.6;

fn text_w(text: &str, px: f64) -> f64 {
    text.chars().count() as f64 * px * MONO_ADVANCE
}

/// Lines a text needs at `px` in `usable` width, with the browser's own
/// wrapping given some slack.
fn wrapped(text: &str, px: f64, usable: f64) -> f64 {
    (text_w(text, px) * WRAP_SLACK / usable.max(1.0))
        .ceil()
        .max(1.0)
}

impl Call {
    /// The engraved width of this wire: heavier the more references the survey
    /// resolved for the pair, as everywhere in this system. A contract is one
    /// promise, so it draws at the hairline.
    fn width(&self) -> f64 {
        match self.kind {
            CallKind::Answers => 1.0,
            CallKind::Call => (1.0 + (self.count.max(1) as f64).ln() * 0.32).min(2.4),
        }
    }
}

/// One block, measured. Everything the plate draws about one mark, and nothing
/// about where it sits.
#[derive(Clone, PartialEq)]
struct MeasuredBlock {
    id: u32,
    /// `pub fn`, `fn`, `macro` — what rust writes in front of the name.
    decl: String,
    name: String,
    /// The bracket the head opens with: `(` where parameters follow, `()`
    /// where none do, nothing at all for a macro.
    open: String,
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
    size: (f64, f64),
    /// The size the far edition draws this name at: what its own box can carry
    /// on one line, down to a floor where a long identifier ellipsizes instead
    /// of breaking. DESIGN.md's far edition always said "at a size its box can
    /// carry"; only the block knows its box, so only the block can say it.
    far_name: f64,
}

impl MeasuredBlock {
    fn measure(mark: &FnMark) -> Self {
        let decl = mark.head.decl();
        let macro_rules = mark.head.kind == ItemKind::Macro;
        let params: Vec<SigRow> = mark.rows.iter().filter(|r| !r.returns).cloned().collect();
        let ret = mark.rows.iter().find(|r| r.returns);
        let open = match (macro_rules, params.is_empty()) {
            (true, _) => String::new(),
            (false, true) => "()".to_string(),
            (false, false) => "(".to_string(),
        };
        let tail = match (macro_rules, params.is_empty(), ret) {
            (true, ..) => String::new(),
            (false, false, Some(r)) => format!(") -> {}", r.ty),
            (false, false, None) => ")".to_string(),
            (false, true, Some(r)) => format!("-> {}", r.ty),
            (false, true, None) => String::new(),
        };

        let head = format!("{decl} {}{open}", mark.head.name);
        let widest = params
            .iter()
            .map(|row| text_w(&row.written(), 10.0) + ROW_INDENT)
            .chain(std::iter::once(text_w(&tail, 10.0)))
            .fold(0.0, f64::max);
        let w = (text_w(&head, 10.5) * META_SLACK)
            .max(widest)
            .clamp(MARK_MIN_W - PAD_X, MARK_MAX_W - PAD_X)
            + PAD_X;

        let usable = w - PAD_X - ROW_INDENT;
        let rows_h: f64 = params
            .iter()
            .map(|row| ROW_H * wrapped(&row.written(), 10.0, usable))
            .sum();
        let tail_h = if tail.is_empty() {
            0.0
        } else {
            TAIL_H * wrapped(&tail, 10.0, w - PAD_X)
        };
        let h = PAD_TOP + HEAD_H + rows_h + tail_h + PAD_BOTTOM;

        // One line of the name inside the box's own width, between the size a
        // 30px plate would use and the smallest that still reads on glass.
        let far_name = ((w - PAD_X)
            / (mark.head.name.chars().count().max(1) as f64 * MONO_ADVANCE))
            .clamp(11.0, 26.0);
        Self {
            id: mark.id,
            decl,
            name: mark.head.name.clone(),
            open,
            rows: params,
            tail,
            letter: mark.letter(),
            entry: mark.tier == Tier::Entry,
            ring: mark.tier == Tier::Ring,
            path: mark.head.path.clone(),
            label: mark.head.label.clone(),
            title: mark.title(),
            size: (w, h),
            far_name,
        }
    }
}

/// One node on the function chart. Every node is a block: this chart has no
/// counted rows, because nothing on it folds by hand.
#[derive(Clone, PartialEq)]
struct FnNodeData(Box<MeasuredBlock>);

/// One drawn wire with its ends already found.
#[derive(Clone, PartialEq)]
struct WireView {
    key: String,
    from: Point,
    to: Point,
    /// The end being leaned on, and the end that leans.
    def: u32,
    user: u32,
    label: Option<String>,
    width: f64,
    rest: bool,
    class: &'static str,
}

/// One drawing of the chart: the blocks, ground and wires one build puts on the
/// paper, plus the indexes a reading walks.
#[derive(Clone, PartialEq)]
struct FnDrawing {
    nodes: Vec<FlowNode<FnNodeData>>,
    lanes: Vec<Lane>,
    prisms: Vec<Prism>,
    wires: Vec<WireView>,
    /// Which frame every drawn mark sits in, for a module reading.
    homes: HashMap<u32, u32>,
    /// Which band every drawn mark sits in, for a band reading.
    bands: HashMap<u32, u32>,
    /// Every drawn mark's box — what the camera glides to.
    rects: HashMap<u32, Placed>,
    /// The URL's (path, item) key for every drawn mark.
    locate: HashMap<(String, String), u32>,
    frame: Option<Rect>,
    /// The bounds of the entry band alone — where the running starts, and so
    /// where a reader opening this chart is put down.
    entry: Option<Rect>,
    dirty: bool,
}

impl FnDrawing {
    fn build(model: &FnModel) -> Self {
        let mut sizes = Sizes::default();
        let mut views: Vec<MeasuredBlock> = Vec::with_capacity(model.marks.len());
        for mark in &model.marks {
            let view = MeasuredBlock::measure(mark);
            sizes.marks.insert(mark.id, view.size);
            views.push(view);
        }
        for frame in &model.frames {
            sizes
                .labels
                .insert(frame.id, text_w(&frame.label(), 12.0) + 18.0);
        }

        let placed = FnLayout::build(model, &sizes);

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
                .draggable(false)
                .selectable(false),
            );
        }

        let mut wires: Vec<WireView> = Vec::with_capacity(model.calls.len());
        for call in &model.calls {
            let (Some(def), Some(user)) = (rects.get(&call.def), rects.get(&call.user)) else {
                continue;
            };
            let (from, to) = def.tie_ends(*user);
            let answers = call.kind == CallKind::Answers;
            wires.push(WireView {
                key: format!("{}-{}-{}", call.def, call.user, answers as u8),
                from,
                to,
                def: call.def,
                user: call.user,
                label: match answers {
                    true => Some("answers".to_string()),
                    false => (call.count > 1).then(|| call.count.to_string()),
                },
                width: call.width(),
                rest: call.rest,
                class: match answers {
                    true => "is-answers",
                    false => "is-call",
                },
            });
        }

        let homes = model.marks.iter().map(|m| (m.id, m.frame)).collect();
        let deepest = model.facts.deepest;
        let bands = model
            .marks
            .iter()
            .map(|m| (m.id, m.tier.band(deepest)))
            .collect();
        let frame = Rect::bounds(nodes.iter().map(|n| n.rect()));
        let entries: HashSet<u32> = model
            .marks
            .iter()
            .filter(|m| m.tier == Tier::Entry)
            .map(|m| m.id)
            .collect();
        let entry = Rect::bounds(
            nodes
                .iter()
                .filter(|n| entries.contains(&n.data.0.id))
                .map(|n| n.rect()),
        );
        FnDrawing {
            nodes,
            lanes: placed.lanes,
            prisms: placed.prisms,
            wires,
            homes,
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
/// A mark selection lights its **blast radius** — itself and every caller a
/// rewrite of it could reach — and reads its direct neighbours in the chosen
/// direction a step behind, because what a function calls does not change when
/// the function does. A module or a band lights everything inside its boundary
/// and reads one hop across it, both ways round: what crosses a boundary is
/// what a reader came to the boundary for.
#[derive(Clone, PartialEq)]
struct FnKin {
    sel: Option<u32>,
    /// A whole module boundary, and the marks inside it.
    home: Option<(u32, HashSet<u32>)>,
    /// A whole band, and the marks in it.
    band: Option<(u32, HashSet<u32>)>,
    dir: CallDir,
    lit: HashSet<u32>,
    near: HashSet<u32>,
}

impl FnKin {
    fn mark(sel: u32, dir: CallDir, model: &FnModel) -> Self {
        let mut lit = model.upstream(sel);
        lit.insert(sel);
        let near: HashSet<u32> = model
            .calls
            .iter()
            .filter(|c| dir.draws(sel, c.def, c.user))
            .map(|c| if c.def == sel { c.user } else { c.def })
            .filter(|id| !lit.contains(id))
            .collect();
        Self {
            sel: Some(sel),
            home: None,
            band: None,
            dir,
            lit,
            near,
        }
    }

    fn module(frame: u32, model: &FnModel, drawing: &FnDrawing) -> Self {
        // Everything written inside the boundary, however deep the module tree
        // runs under it.
        let mut kept: HashSet<u32> = HashSet::from([frame]);
        loop {
            let grown: HashSet<u32> = model
                .frames
                .iter()
                .filter(|f| f.parent.is_some_and(|p| kept.contains(&p)))
                .map(|f| f.id)
                .collect();
            let before = kept.len();
            kept.extend(grown);
            if kept.len() == before {
                break;
            }
        }
        let lit: HashSet<u32> = drawing
            .homes
            .iter()
            .filter(|(_, at)| kept.contains(at))
            .map(|(id, _)| *id)
            .collect();
        let near = model.one_hop(&lit);
        Self {
            sel: None,
            home: Some((frame, lit.clone())),
            band: None,
            dir: CallDir::Both,
            lit,
            near,
        }
    }

    fn whole_band(band: u32, model: &FnModel, drawing: &FnDrawing) -> Self {
        let lit: HashSet<u32> = drawing
            .bands
            .iter()
            .filter(|(_, at)| **at == band)
            .map(|(id, _)| *id)
            .collect();
        let near = model.one_hop(&lit);
        Self {
            sel: None,
            band: Some((band, lit.clone())),
            home: None,
            dir: CallDir::Both,
            lit,
            near,
        }
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

    /// The class one wire wears. A wire inside the lit set is the reading's own
    /// ink; a wire that only touches it reads a step behind.
    ///
    /// A **band** reads differently, and has to: a stratum holds sixty marks,
    /// so "touches the reading" is most of the sheet. What a band lights is the
    /// **way in** — the one resting wire that reaches each of its marks — which
    /// is one wire per lit block and reads as the descent into that depth.
    fn wire_class(&self, wire: &WireView) -> &'static str {
        let both = self.lit.contains(&wire.def) && self.lit.contains(&wire.user);
        if both {
            return "is-kin";
        }
        if self.band.is_some() {
            return match wire.rest && self.lit.contains(&wire.def) {
                true => "is-near",
                false => "is-dim",
            };
        }
        let touches = self.lit.contains(&wire.def) || self.lit.contains(&wire.user);
        match (touches, self.sel) {
            (true, Some(sel)) if self.dir.draws(sel, wire.def, wire.user) => "is-near",
            (true, None) => "is-near",
            _ => "is-dim",
        }
    }

    /// Whether this reading gives a folded wire back.
    ///
    /// A mark and a boundary both do: a reader who picked one wants the calls
    /// it does not draw at rest, and a frame is small enough that giving them
    /// back stays a reading. A **band** does not. A stratum holds sixty marks
    /// and every call on the sheet touches one, so unfolding there is the
    /// hairball this system forbids — and it is not what the reader asked
    /// anyway: a band's question is which declarations sit at this depth, and
    /// in which frames. The way-in tree keeps carrying that.
    fn inks(&self, wire: &WireView) -> bool {
        self.band.is_none() && (self.lit.contains(&wire.def) || self.lit.contains(&wire.user))
    }
}

// ---------------------------------------------------------------------------
// The drawing.
// ---------------------------------------------------------------------------

/// One signature block on the paper.
#[component]
fn FnPlate(view: MeasuredBlock, kin: Option<FnKin>, hot: Signal<Option<u32>>) -> Element {
    let nav = use_navigator();
    let kin_class = kin.as_ref().map_or("", |k| k.block_class(view.id));
    let picked = kin_class == "is-picked";
    let to = match picked {
        true => Route::FnOverview {},
        false => mark_route(&view.path, &view.label),
    };
    let title = match picked {
        true => format!(
            "{} {} — selected · click again to deselect",
            view.decl, view.name
        ),
        false => view.title.clone(),
    };
    let push = to.clone();
    let pressed = to.clone();
    let mut hot = hot;
    let id = view.id;
    rsx! {
        div {
            class: "fn-mark",
            style: "--far-name: {view.far_name:.1}px;",
            class: if !kin_class.is_empty() { "{kin_class}" },
            class: if view.entry { "is-entry" },
            class: if view.ring { "is-ring" },
            class: if view.letter.is_some() { "is-diff" },
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
            header { class: "fm-head",
                span { class: "fm-kw", "{view.decl}" }
                span { class: "fm-nm", "{view.name}" }
                if !view.open.is_empty() {
                    span { class: "fm-open", "{view.open}" }
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
            for (i , row) in view.rows.iter().enumerate() {
                p { key: "{i}", class: "fm-row",
                    class: if row.added { "is-add" },
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
                p { class: "fm-tail", "{view.tail}" }
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

/// The bands: one full-width lane per call depth, captioned at the left margin
/// the way the dependency chart's rings caption their hops. The caption is the
/// control — a band is a focus like any other — and the hairline under it is
/// never a pointer target, so a click always means the words it lands on.
#[component]
fn LaneLayer(lanes: Vec<Lane>, kin: Option<FnKin>, glass_left: f64) -> Element {
    let nav = use_navigator();
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for lane in lanes.iter() {
                {
                    let chosen = kin
                        .as_ref()
                        .and_then(|k| k.band.as_ref())
                        .is_some_and(|(at, _)| *at == lane.band);
                    let dim = kin.is_some() && !chosen;
                    let to = match chosen {
                        true => Route::FnOverview {},
                        false => band_route(lane.band),
                    };
                    let push = to.clone();
                    let words = match chosen {
                        true => format!("{} — selected · click again to deselect", lane.caption),
                        false => {
                            format!("{} — select the band and read what crosses it", lane.caption)
                        }
                    };
                    // The caption rides the left edge of the glass, never the
                    // sheet's own margin: a band spans the whole width, so a
                    // caption pinned to x=0 leaves the screen the moment the
                    // reader pans — and the ladder this chart is about would go
                    // unlabelled. It never leaves its own band.
                    let cap_x = glass_left.clamp(lane.at.x, lane.at.x + lane.at.w - 96.0);
                    rsx! {
                        g {
                            key: "{lane.band}",
                            class: "fn-lane",
                            class: if chosen { "is-sel" },
                            class: if dim { "is-dim" },
                            line {
                                class: "fn-lane-rule",
                                x1: "{lane.at.x}",
                                y1: "{lane.at.y - 7.0}",
                                x2: "{lane.at.x + lane.at.w}",
                                y2: "{lane.at.y - 7.0}",
                            }
                            g {
                                class: "fn-lane-pick",
                                role: "link",
                                tabindex: "0",
                                "aria-label": "{words}",
                                onclick: move |e: Event<MouseData>| {
                                    e.stop_propagation();
                                    nav.push(push.clone());
                                },
                                title { "{words}" }
                                rect {
                                    class: "fn-lane-hit",
                                    x: "{cap_x}",
                                    y: "{lane.at.y - 14.0}",
                                    width: "88",
                                    height: "20",
                                }
                                text {
                                    class: "fn-lane-caption",
                                    x: "{cap_x}",
                                    y: "{lane.at.y + 8.0}",
                                    "{lane.caption}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The prisms: one column per frame the grouping asks for, crossing every band
/// and named along the top the way an atlas names a region. The name is the
/// boundary's control: clicking it selects the module — or the type, or the
/// file — and reads what crosses the line.
#[component]
fn PrismLayer(prisms: Vec<Prism>, kin: Option<FnKin>) -> Element {
    let nav = use_navigator();
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for prism in prisms.iter() {
                {
                    let chosen = kin
                        .as_ref()
                        .and_then(|k| k.home.as_ref())
                        .is_some_and(|(id, _)| *id == prism.frame);
                    let dim = kin.is_some() && !chosen;
                    let to = match chosen {
                        true => Route::FnOverview {},
                        false => mod_route(prism.key.clone()),
                    };
                    let push = to.clone();
                    let words = match chosen {
                        true => format!("{} — selected · click again to deselect", prism.written),
                        false => {
                            format!("{} — select the boundary and read what crosses it", prism.written)
                        }
                    };
                    rsx! {
                        g {
                            key: "{prism.frame}",
                            class: "fn-prism",
                            class: if chosen { "is-sel" },
                            class: if dim { "is-dim" },
                            rect {
                                class: "fn-prism-box",
                                x: "{prism.at.x - 8.0}",
                                y: "{prism.at.y}",
                                width: "{prism.at.w + 16.0}",
                                height: "{prism.at.h}",
                                fill: "none",
                            }
                            g {
                                class: "fn-prism-pick",
                                role: "link",
                                tabindex: "0",
                                "aria-label": "{words}",
                                onclick: move |e: Event<MouseData>| {
                                    e.stop_propagation();
                                    nav.push(push.clone());
                                },
                                title { "{words}" }
                                rect {
                                    class: "fn-prism-hit",
                                    x: "{prism.at.x - 8.0}",
                                    y: "{prism.at.y}",
                                    width: "{prism.at.w + 16.0}",
                                    height: "20",
                                }
                                text {
                                    class: "fn-prism-name",
                                    x: "{prism.at.x}",
                                    y: "{prism.at.y + 12.0}",
                                    "{prism.written}"
                                }
                            }
                        }
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

/// Both families as one engraved layer, the contracts first and lighter.
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
    dir: CallDir,
    hot: Signal<Option<u32>>,
    kin: Option<FnKin>,
) -> Element {
    let hot = hot();
    let wire = |w: &WireView, side: f64| {
        let (d, ctrl) = curve(w.from, w.to, side);
        let head = arrowhead(w.to, ctrl, 3.2 + w.width);
        let (lx, ly) = (
            0.25 * w.from.x + 0.5 * ctrl.x + 0.25 * w.to.x,
            0.25 * w.from.y + 0.5 * ctrl.y + 0.25 * w.to.y,
        );
        // Hovering a block is the cheapest anchor there is, and it reads the
        // same direction the plate does: a wire lights when the hovered mark is
        // the end this reading asks for.
        let is_hot = hot.is_some_and(|h| dir.draws(h, w.def, w.user));
        let inked = w.rest || is_hot || kin.as_ref().is_some_and(|k| k.inks(w));
        let mut classes: Vec<&str> = vec![w.class];
        if !inked {
            classes.push("is-folded");
        }
        if is_hot {
            classes.push("is-hot");
        }
        if let Some(k) = kin.as_ref() {
            classes.push(k.wire_class(w));
        }
        let classes = classes.join(" ");
        rsx! {
            g { key: "{w.key}", class: "fn-wire {classes}",
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
            for w in wires.iter().filter(|w| w.class == "is-answers") {
                {wire(w, -1.0)}
            }
            for w in wires.iter().filter(|w| w.class == "is-call") {
                {wire(w, 1.0)}
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

/// Below this the quoted rows are dust and the chart holds its far edition:
/// names and wires alone. Hysteresis keeps the swap from flapping while the
/// reader hovers around the threshold.
const FAR_IN: f64 = 0.45;
const FAR_OUT: f64 = 0.55;
/// The zoom a selection is read at: when a chosen mark sits below this, or off
/// the glass entirely, the camera glides to it.
#[cfg(target_arch = "wasm32")]
const READ_ZOOM: f64 = 0.5;

/// The zoom the chart opens at, at the lowest. A sheet of three hundred
/// declarations fits the glass at around a fifth of full size, where even the
/// far edition's names are dust — so the opening view keeps this much scale and
/// puts the reader at the top of the running order instead, which is what this
/// altitude is about. `f` still fits the whole sheet, at whatever zoom that
/// takes.
#[cfg(target_arch = "wasm32")]
const OPEN_FLOOR: f64 = 0.34;
/// And no closer than this on a small workspace, where the whole sheet fits.
#[cfg(target_arch = "wasm32")]
const OPEN_CEIL: f64 = 0.75;

/// Which seating a remembered camera belongs to. Each grouping is a different
/// sheet — a section of types is not a section of modules — and handing a
/// reader one camera on the other would lose their place.
pub(in crate::views) type Seating = Group;

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
/// is legible, and the entry band at reading scale where it is not.
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
    // Too much sheet to read at once: hold the floor and open on the band the
    // running starts in.
    let Some(core) = flow.core() else { return };
    let at = entry.unwrap_or(whole);
    let zoom = ((free_w / at.width.max(1.0)).min(free_h / at.height.max(1.0)) * 0.94)
        .clamp(OPEN_FLOOR, OPEN_CEIL);
    let center = at.center();
    let (cx, cy) = (l + free_w / 2.0, t + free_h / 2.0);
    core.set_viewport(
        Viewport::new(cx - center.x * zoom, cy - center.y * zoom, zoom),
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
    let zoom = fit.clamp(MIN_CHART_ZOOM, 1.0);
    let center = bounds.center();
    let cx = l + free_w / 2.0;
    let cy = t + free_h / 2.0;
    core.set_viewport(
        Viewport::new(cx - center.x * zoom, cy - center.y * zoom, zoom),
        duration_ms,
    );
}

/// Keyboard at this altitude: `f` refits, Escape steps back out of the deepest
/// thing open — a quotation first, then the selection — and `/` finds.
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
    if (['f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopeKeys);
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

    let kin: Memo<Option<FnKin>> = use_memo(use_reactive((&sel,), move |(sel,)| {
        let model = model.read();
        let drawing = chart.read();
        match sel? {
            FnSel::Mark(path, label) => {
                let id = *drawing.locate.get(&(path, label))?;
                Some(FnKin::mark(id, *fns.calls.read(), &model))
            }
            FnSel::Mod(key) => {
                // The prism is the boundary on this plate, and the prism is
                // what carries its own key: resolving a module through a frame
                // list the section never fills selected nothing at all.
                let frame = drawing.prisms.iter().find(|p| p.key == key)?.frame;
                Some(FnKin::module(frame, &model, &drawing))
            }
            FnSel::Band(band) => Some(FnKin::whole_band(band, &model, &drawing)),
        }
    }));

    let sel_on: Signal<bool> = use_signal(|| false);
    use_effect(use_reactive((&sel.is_some(),), move |(on,)| {
        let mut sel_on = sel_on;
        if *sel_on.peek() != on {
            sel_on.set(on);
        }
    }));
    // The keyboard listener is mounted once and outlives every route change,
    // so what Escape closes rides in on a signal rather than the props.
    let quoted: Signal<Option<Route>> = use_signal(|| None);
    use_effect(use_reactive((&unquote,), move |(unquote,)| {
        let mut quoted = quoted;
        if *quoted.peek() != unquote {
            quoted.set(unquote);
        }
    }));

    let nodes: Signal<Vec<FlowNode<FnNodeData>>> = use_signal(Vec::new);
    let hot: Signal<Option<u32>> = use_signal(|| None);
    // Where the left edge of the readable glass falls in world units — what a
    // band's caption pins itself to, so the ladder is never off-screen.
    let glass_left: Signal<f64> = use_signal(|| 0.0);
    let core_live: Signal<bool> = use_signal(|| false);
    let far: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        let drawing = chart();
        let seating: Seating = *fns.group.read();
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
        let mut glass = glass_left;
        let inset = chrome_insets(narrow_viewport(), false).3 + 16.0;
        let at = (inset - vp.x) / vp.zoom.max(0.01);
        if (*glass.peek() - at).abs() > 0.5 {
            glass.set(at);
        }
        let mut saved = camera.viewport;
        // Peeked, not read: this effect must fire when the camera moves, never
        // when the seating changes — the effect above owns that, and a save
        // racing it would store the old sheet's camera under the new sheet.
        saved.set(Some((*fns.group.peek(), vp)));
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
        let Some(FnSel::Mark(path, label)) = sel else {
            return;
        };
        let Some(core) = flow.core() else { return };
        let drawing = chart.peek();
        let Some(&id) = drawing.locate.get(&(path, label)) else {
            return;
        };
        let Some(at) = drawing.rects.get(&id).copied() else {
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
                    _ => {}
                }
            }
        });
    });

    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);
    let panel = matches!(sel, Some(FnSel::Mark(..)));
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
                    PrismLayer { prisms: chart.read().prisms.clone(), kin: kin() }
                    LaneLayer {
                        lanes: chart.read().lanes.clone(),
                        kin: kin(),
                        glass_left: glass_left(),
                    }
                }
                WorldLayer { class: "fn-wires",
                    WireLayer {
                        wires: chart.read().wires.clone(),
                        dir: *fns.calls.read(),
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
    use crate::graph::data::{Delta, Vis};
    use crate::views::func::model::FnHead;

    fn mark(name: &str, params: &[(&str, &str)], ret: &str) -> FnMark {
        let mut rows: Vec<SigRow> = params
            .iter()
            .map(|(n, t)| SigRow {
                name: (*n).to_string(),
                ty: (*t).to_string(),
                returns: false,
                added: false,
            })
            .collect();
        if !ret.is_empty() {
            rows.push(SigRow {
                name: String::new(),
                ty: ret.to_string(),
                returns: true,
                added: false,
            });
        }
        FnMark {
            id: 0,
            frame: 0,
            tier: Tier::Entry,
            road: None,
            head: FnHead {
                kind: ItemKind::Fn,
                vis: Vis::Pub,
                name: name.to_string(),
                label: name.to_string(),
                path: "src/main.rs".to_string(),
                line: 1,
                section: String::new(),
            },
            rows,
            delta: Delta::Same,
            callers: 0,
            calls: 0,
            touches: 0,
            recurses: false,
        }
    }

    /// A block quotes the signature the way rust writes it: the name opens the
    /// parameter list, the parameters are its rows, and the return closes it.
    #[test]
    fn a_block_quotes_its_signature_as_rust_writes_it() {
        let block = MeasuredBlock::measure(&mark(
            "survey",
            &[("dir", "&std::path::Path")],
            "Result<CodeIndex, String>",
        ));
        assert_eq!(block.open, "(");
        assert_eq!(block.rows.len(), 1);
        assert_eq!(block.rows[0].written(), "dir: &std::path::Path");
        assert_eq!(block.tail, ") -> Result<CodeIndex, String>");

        // No parameters: the brackets close on the head and the return stands
        // alone.
        let none = MeasuredBlock::measure(&mark("analyze", &[], "Result<DepGraph, String>"));
        assert_eq!(none.open, "()");
        assert!(none.rows.is_empty());
        assert_eq!(none.tail, "-> Result<DepGraph, String>");

        // Nothing to say: no tail at all.
        let bare = MeasuredBlock::measure(&mark("main", &[], ""));
        assert_eq!(bare.open, "()");
        assert_eq!(bare.tail, "");
    }

    /// A block is never narrower than the plate's floor, never wider than a
    /// quoted row's clip, and always tall enough for every row it draws.
    #[test]
    fn a_block_is_measured_to_hold_what_it_draws() {
        let wide = MeasuredBlock::measure(&mark(
            "walk_ty",
            &[
                ("db", "&'db RootDatabase"),
                ("mark_of_def", "&dyn Fn(ModuleDef) -> Option<u32>"),
                ("tail", "&mut Vec<(String, String)>"),
            ],
            "Option<HoldKind>",
        ));
        assert!(wide.size.0 >= MARK_MIN_W && wide.size.0 <= MARK_MAX_W);
        let bare = MeasuredBlock::measure(&mark("main", &[], ""));
        assert!(wide.size.1 > bare.size.1, "rows take room");
        assert!(bare.size.1 >= PAD_TOP + HEAD_H + PAD_BOTTOM);
    }
}
