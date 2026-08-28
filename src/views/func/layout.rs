//! Where the function chart's frames, containers and blocks sit: the
//! **household**.
//!
//! Containment is written-in, on this chart as on the one above it. A crate
//! frame holds module frames nested exactly as rust's modules nest; inside a
//! module frame stand its free declarations, one container per owner that
//! declares methods, and the module frames nested under it. Inside a container
//! stand that owner's methods, in the order the source writes them.
//!
//! A pure function of (model, measured boxes). Every block is measured before
//! anything is placed — its head row and the signature quoted under it — a
//! container is measured from its own label and what it holds, a frame from its
//! label and what it holds, and the same survey always draws the same chart: no
//! physics, no randomness, no measurement of anything the browser has already
//! laid out.
//!
//! A **fold** is part of that purity, not an overlay on it — but it is an
//! elision, not a re-packing. A box the reviewer folded keeps its whole
//! footprint here: everything inside it is still measured and still placed, so
//! no sibling, ancestor or wire moves by a pixel when the reader closes it, and
//! the eye stays on whatever it was anchored to. What the fold does is take
//! those marks off the *drawing*; the box says how many in words, where its
//! shelf stood.
//!
//! The one exception is `model.packed` — the folds the packer is allowed to
//! skip, which is only ever the set the paper was already being laid again
//! around: a `visibility` change, or a session's first build. A box in that set
//! is measured as its own label and counted row, and the sheet closes up over
//! it. Nothing folds itself and nothing folds by a count; both sets are boxes a
//! reader closed by hand.
//!
//! The packer is the data chart's own `skyline` — one packer for the whole
//! system, so two charts of one workspace never disagree about how a shelf
//! fills.

use std::collections::HashMap;

use dioxus_flow::prelude::Point;

use crate::views::data::layout::skyline;
use crate::views::func::model::{FnModel, Spot};

/// One placed box on the paper, in flow units — one unit is one CSS pixel at
/// zoom 1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Placed {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) w: f64,
    pub(super) h: f64,
}

/// Which edge of a block's own **band** — its head row and the quotation under
/// it, which is the only paper a block actually covers — a wire ties to.
///
/// Four sides, and each one is a real boundary of that band: a tie is somewhere
/// a wire can leave without crossing the block it belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum TieSide {
    Top,
    Left,
    Right,
    Under,
}

impl Placed {
    pub(super) fn center(&self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// How far along this block's head a wire may tie: the first `HEAD_TIE`
    /// units of it. A container is as wide as everything it holds, and a line
    /// tied out in a two-thousand-unit head names nothing, so the top and the
    /// foot tie beside the name.
    fn span(&self) -> (f64, f64) {
        let w = self.w.min(HEAD_TIE);
        let pad = (w * 0.18).min(FAN_PAD);
        (self.x + pad, self.x + w - pad)
    }

    /// The band this block covers: its head row and its quotation, `own` tall.
    fn band(&self, own: f64) -> Placed {
        Placed {
            x: self.x,
            y: self.y,
            w: self.w,
            h: own.max(HEAD_H),
        }
    }

    /// Which edge of this block's band faces `other` — the **nearest edge**,
    /// which is where a wire between the two ties. `own` is this block's own
    /// height, the band without anything shelved under it.
    ///
    /// The head row's own centre was the tie once, and centre-to-centre is
    /// exactly what drove every line through the text it was pointing at: a wire
    /// had to cross half a head row, and often the whole quotation under it, to
    /// reach the point it ended on. An edge tie starts where the paper ends.
    pub(super) fn tie_side(&self, own: f64, other: Placed) -> TieSide {
        let band = self.band(own);
        let them = other.band(HEAD_H);
        let (them_x, them_y) = (them.x + them.w.min(HEAD_TIE) / 2.0, them.y + them.h / 2.0);
        if them_y < band.y {
            return TieSide::Top;
        }
        if them_y > band.y + band.h && them_x > band.x && them_x < band.x + band.w {
            return TieSide::Under;
        }
        let (from, to) = self.span();
        match them_x < (from + to) / 2.0 {
            true => TieSide::Left,
            false => TieSide::Right,
        }
    }

    /// Where on that edge the wire ties. `slot` runs 0..1 across the edge's own
    /// fanning span, so several wires sharing one edge stand apart instead of
    /// stacking on one point; one wire alone takes the middle.
    pub(super) fn tie_at(&self, own: f64, side: TieSide, slot: f64) -> Point {
        let band = self.band(own);
        let slot = slot.clamp(0.0, 1.0);
        let (from, to) = self.span();
        match side {
            TieSide::Top => Point::new(from + slot * (to - from), band.y),
            TieSide::Under => Point::new(from + slot * (to - from), band.y + band.h),
            TieSide::Left | TieSide::Right => {
                let x = match side {
                    TieSide::Left => band.x,
                    _ => band.x + band.w,
                };
                Point::new(x, band.y + 2.0 + slot * (HEAD_H - 4.0))
            }
        }
    }

    /// Which sides of two boxes face each other — where a **corridor** between
    /// two module frames leaves one and lands on the other, on open paper rather
    /// than across either frame. The same geometry the data chart's boundary
    /// bundles use one rung up.
    pub(super) fn tie_ends(self, other: Self) -> (Point, Point) {
        let (ac, bc) = (self.center(), other.center());
        if (ac.x - bc.x).abs() > (ac.y - bc.y).abs() {
            let left = ac.x < bc.x;
            (
                Point::new(if left { self.x + self.w } else { self.x }, ac.y),
                Point::new(if left { other.x } else { other.x + other.w }, bc.y),
            )
        } else {
            let top = ac.y < bc.y;
            (
                Point::new(ac.x, if top { self.y + self.h } else { self.y }),
                Point::new(bc.x, if top { other.y } else { other.y + other.h }),
            )
        }
    }
}

/// The head row's own height, borders excluded. These numbers are the CSS in
/// `tailwind.css`; move one and the other must follow.
pub(super) const HEAD_H: f64 = 16.0;
/// Clear paper inside a **container**, around the methods it holds.
pub(super) const PAD: f64 = 6.0;
/// The container's own label row, on its top border.
pub(super) const OWNER_HEAD_H: f64 = 15.0;
/// Clear paper inside a **frame**, around everything it holds.
pub(super) const FRAME_PAD: f64 = 12.0;
/// Clear paper on a frame's top border for its engraved label chip.
pub(super) const FRAME_LABEL_H: f64 = 16.0;
/// The counted words a folded box writes where its shelf stood.
pub(super) const PACKED_H: f64 = 13.0;
/// Between two blocks on one shelf.
const GAP: f64 = 5.0;
/// Between two boxes inside a frame, wide enough for a wire to land between.
const FRAME_GAP: f64 = 14.0;
/// Between two crate frames on the sheet.
const SHEET_GAP: f64 = 20.0;
/// How much wider than tall a packed shelf aims to be.
const LANDSCAPE: f64 = 2.4;
/// A frame narrower than this reads as a column of unrelated plates.
const MIN_FRAME_W: f64 = 168.0;
/// How far along a block's head a wire ties, at the widest.
const HEAD_TIE: f64 = 150.0;
/// Clear head row left at each end of a fanned edge, so a wire never ties on
/// the corner it would round.
const FAN_PAD: f64 = 14.0;

/// What the layout must be told about what it seats. Measuring belongs with the
/// drawing; the layout only places what it is handed.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct Sizes {
    /// Every drawn mark's own box — its head row and the signature quoted under
    /// it — by mark id.
    pub(super) marks: HashMap<u32, (f64, f64)>,
    /// The width each container's own label row needs.
    pub(super) owners: HashMap<u32, f64>,
    /// The width each frame's border label needs.
    pub(super) labels: HashMap<u32, f64>,
    /// The counted words a fold the packer skipped writes in place of what it
    /// holds.
    pub(super) shut: HashMap<Spot, f64>,
}

impl Sizes {
    fn mark(&self, id: u32) -> (f64, f64) {
        self.marks
            .get(&id)
            .copied()
            .unwrap_or((120.0, HEAD_H + 2.0))
    }
}

/// The whole chart, placed and centered on the flow origin.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnLayout {
    pub(super) marks: HashMap<u32, Placed>,
    pub(super) owners: HashMap<u32, Placed>,
    /// Outermost first, so a nested frame paints over the one it stands in.
    pub(super) frames: Vec<(u32, Placed)>,
    pub(super) size: (f64, f64),
}

/// One packed box and everything placed inside it, relative to its own corner.
#[derive(Default)]
struct Packed {
    w: f64,
    h: f64,
    marks: Vec<(u32, Placed)>,
    owners: Vec<(u32, Placed)>,
    frames: Vec<(u32, Placed)>,
}

impl Packed {
    fn shift(self, dx: f64, dy: f64) -> Self {
        let at = |p: Placed| Placed {
            x: p.x + dx,
            y: p.y + dy,
            ..p
        };
        Packed {
            w: self.w,
            h: self.h,
            marks: self.marks.into_iter().map(|(k, p)| (k, at(p))).collect(),
            owners: self.owners.into_iter().map(|(k, p)| (k, at(p))).collect(),
            frames: self.frames.into_iter().map(|(k, p)| (k, at(p))).collect(),
        }
    }

    /// Seat a set of boxes on the shared skyline, in the order given.
    fn shelve(kids: Vec<(Packed, Spot)>, gap: f64, floor: f64) -> (Packed, f64, f64) {
        let boxes: Vec<(f64, f64)> = kids.iter().map(|(p, _)| (p.w, p.h)).collect();
        let widest = boxes.iter().map(|b| b.0).fold(floor, f64::max);
        let area: f64 = boxes.iter().map(|(w, h)| (w + gap) * (h + gap)).sum();
        let target = widest.max((area * LANDSCAPE).sqrt());
        let at = skyline(&boxes, target, gap);
        let mut out = Packed::default();
        let (mut content_w, mut content_h) = (0.0f64, 0.0f64);
        for ((packed, spot), (x, y)) in kids.into_iter().zip(at) {
            let (w, h) = (packed.w, packed.h);
            match spot {
                Spot::Mark(id) => out.marks.push((id, Placed { x, y, w, h })),
                Spot::Owner(id) => out.owners.push((id, Placed { x, y, w, h })),
                Spot::Frame(id) => out.frames.push((id, Placed { x, y, w, h })),
            }
            let inner = packed.shift(x, y);
            out.marks.extend(inner.marks);
            out.owners.extend(inner.owners);
            out.frames.extend(inner.frames);
            content_w = content_w.max(x + w);
            content_h = content_h.max(y + h);
        }
        (out, content_w, content_h)
    }
}

impl FnLayout {
    /// Seat every frame, container and block of the household.
    pub(super) fn build(model: &FnModel, sizes: &Sizes) -> Self {
        let mut out = Packed::default();
        let sheet: Vec<(Packed, Spot)> = model
            .frames
            .iter()
            .filter(|f| f.parent.is_none())
            .map(|f| (frame_box(model, sizes, f.id), Spot::Frame(f.id)))
            .collect();
        let (packed, w, h) = Packed::shelve(sheet, SHEET_GAP, 0.0);
        out.marks.extend(packed.marks);
        out.owners.extend(packed.owners);
        out.frames.extend(packed.frames);
        // Centered on the origin, so the camera's first fit is one move.
        let placed = out.shift(-w / 2.0, -h / 2.0);
        // Outermost first: a frame is drawn under the frames nested in it, so
        // the paint order has to be the nesting order.
        let mut frames = placed.frames;
        frames.sort_by_key(|(id, _)| model.over(*id).len());
        FnLayout {
            marks: placed.marks.into_iter().collect(),
            owners: placed.owners.into_iter().collect(),
            frames,
            size: (w, h),
        }
    }
}

/// One container, measured and placed: its label row on the top border, then
/// the owner's methods shelved under it in declaration order.
fn owner_box(model: &FnModel, sizes: &Sizes, id: u32) -> Packed {
    let Some(owner) = model.owners.get(id as usize) else {
        return Packed::default();
    };
    let label = sizes.owners.get(&id).copied().unwrap_or(0.0);
    // A fold the packer was allowed to skip really is just its own label and the
    // words that say what it holds; every other fold keeps its whole footprint.
    if model.packed.contains(&Spot::Owner(id)) {
        let shut = sizes.shut.get(&Spot::Owner(id)).copied().unwrap_or(0.0);
        return Packed {
            w: label.max(shut) + PAD * 2.0,
            h: OWNER_HEAD_H + PACKED_H + PAD,
            ..Default::default()
        };
    }
    let kids: Vec<(Packed, Spot)> = owner
        .marks
        .iter()
        .map(|&mark| {
            let (w, h) = sizes.mark(mark);
            (
                Packed {
                    w,
                    h,
                    ..Default::default()
                },
                Spot::Mark(mark),
            )
        })
        .collect();
    let (inner, content_w, content_h) = Packed::shelve(kids, GAP, 0.0);
    let w = (content_w + PAD * 2.0).max(label + PAD * 2.0);
    let h = content_h + OWNER_HEAD_H + PAD;
    let inner = inner.shift(PAD, OWNER_HEAD_H);
    Packed {
        w,
        h,
        marks: inner.marks,
        owners: inner.owners,
        frames: inner.frames,
    }
}

/// One frame, measured and placed: its free declarations, its containers and the
/// module frames nested under it, with room on the top border for its label.
fn frame_box(model: &FnModel, sizes: &Sizes, id: u32) -> Packed {
    let Some(frame) = model.frames.get(id as usize) else {
        return Packed::default();
    };
    let label = sizes.labels.get(&id).copied().unwrap_or(0.0);
    if model.packed.contains(&Spot::Frame(id)) {
        let shut = sizes.shut.get(&Spot::Frame(id)).copied().unwrap_or(0.0);
        return Packed {
            w: (label.max(shut) + FRAME_PAD * 2.0).max(MIN_FRAME_W),
            h: FRAME_LABEL_H + PACKED_H + FRAME_PAD,
            ..Default::default()
        };
    }
    // The reading order down a frame is the same everywhere on the chart: its
    // own free declarations, then the containers whose types it declares, then
    // the modules nested inside it.
    let mut kids: Vec<(Packed, Spot)> = Vec::new();
    for &mark in &frame.marks {
        let (w, h) = sizes.mark(mark);
        kids.push((
            Packed {
                w,
                h,
                ..Default::default()
            },
            Spot::Mark(mark),
        ));
    }
    for &owner in &frame.owners {
        kids.push((owner_box(model, sizes, owner), Spot::Owner(owner)));
    }
    for &kid in &frame.kids {
        kids.push((frame_box(model, sizes, kid), Spot::Frame(kid)));
    }
    let (inner, content_w, content_h) = Packed::shelve(kids, FRAME_GAP, 0.0);
    let w = (content_w + FRAME_PAD * 2.0)
        .max(label + FRAME_PAD * 2.0)
        .max(MIN_FRAME_W);
    let h = content_h + FRAME_PAD * 2.0 + FRAME_LABEL_H;
    let inner = inner.shift(FRAME_PAD, FRAME_PAD + FRAME_LABEL_H);
    Packed {
        w,
        h,
        marks: inner.marks,
        owners: inner.owners,
        frames: inner.frames,
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::data::{ItemKind, Vis};
    use crate::views::func::model::{Container, Frame};

    fn frame(id: u32, module: &[&str], parent: Option<u32>) -> Frame {
        Frame {
            id,
            krate: "slope".to_string(),
            module: module.iter().map(|s| (*s).to_string()).collect(),
            parent,
            marks: Vec::new(),
            owners: Vec::new(),
            kids: Vec::new(),
            folded: false,
            held: 0,
        }
    }

    fn owner(id: u32, frame: u32, name: &str, marks: &[u32]) -> Container {
        Container {
            id,
            frame,
            ty: 90 + id,
            decl: "impl",
            name: name.to_string(),
            kind: ItemKind::Struct,
            vis: Vis::Pub,
            path: "src/main.rs".to_string(),
            label: name.to_string(),
            on_data: true,
            marks: marks.to_vec(),
            folded: false,
        }
    }

    /// A crate frame holding one module frame; the module holds one free
    /// declaration and one container of two methods.
    fn model() -> FnModel {
        let mut root = frame(0, &[], None);
        root.kids = vec![1];
        root.held = 3;
        let mut views = frame(1, &["views"], Some(0));
        views.marks = vec![0];
        views.owners = vec![0];
        views.held = 3;
        FnModel {
            frames: vec![root, views],
            owners: vec![owner(0, 1, "Plate", &[1, 2])],
            home: HashMap::from([
                (0, Spot::Frame(1)),
                (1, Spot::Owner(0)),
                (2, Spot::Owner(0)),
            ]),
            ..Default::default()
        }
    }

    fn sizes() -> Sizes {
        Sizes {
            marks: (0..3).map(|id| (id, (170.0, 44.0))).collect(),
            owners: HashMap::from([(0, 80.0)]),
            labels: HashMap::from([(0, 40.0), (1, 60.0)]),
            shut: HashMap::new(),
        }
    }

    fn contains(outer: &Placed, inner: &Placed) -> bool {
        outer.x <= inner.x + 0.001
            && outer.y <= inner.y + 0.001
            && outer.x + outer.w + 0.001 >= inner.x + inner.w
            && outer.y + outer.h + 0.001 >= inner.y + inner.h
    }

    fn overlaps(a: &Placed, b: &Placed) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    /// **The household is the drawing.** A method sits inside its owner's
    /// container, the container inside the module frame the owner type is
    /// declared in, the module inside its crate — and a free declaration sits on
    /// the module's own shelf, outside every container.
    #[test]
    fn a_method_sits_in_its_owner_in_its_module_in_its_crate() {
        let placed = FnLayout::build(&model(), &sizes());
        let frame_of = |id: u32| placed.frames.iter().find(|(f, _)| *f == id).unwrap().1;
        let (root, views) = (frame_of(0), frame_of(1));
        let container = placed.owners[&0];
        assert!(contains(&root, &views), "the module is inside its crate");
        assert!(
            contains(&views, &container),
            "the container is inside its module"
        );
        for method in [1u32, 2] {
            assert!(
                contains(&container, &placed.marks[&method]),
                "{method} is outside its owner's container"
            );
        }
        // The free declaration is on the module's own shelf, never inside the
        // container: it belongs to no type.
        assert!(contains(&views, &placed.marks[&0]));
        assert!(!overlaps(&container, &placed.marks[&0]));
        // Outermost first, so a nested frame paints over the one it stands in.
        assert_eq!(
            placed.frames.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    /// Two boxes on one shelf never share paper.
    #[test]
    fn nothing_on_one_shelf_overlaps() {
        let placed = FnLayout::build(&model(), &sizes());
        let boxes: Vec<Placed> = placed.marks.values().copied().collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                assert!(!overlaps(a, b), "two blocks share paper: {a:?} {b:?}");
            }
        }
    }

    /// The same model always draws the same chart.
    #[test]
    fn the_same_model_always_draws_the_same_chart() {
        let a = FnLayout::build(&model(), &sizes());
        let b = FnLayout::build(&model(), &sizes());
        assert_eq!(a, b);
    }

    /// **A fold by hand moves nothing.** The invariant the whole elision exists
    /// for (2026-08-27, user): the reviewer folds a box to get it out of the way
    /// of something they are looking at, so if the sheet re-packs, the fold has
    /// thrown away the very thing it was serving.
    #[test]
    fn a_fold_by_hand_moves_no_other_box() {
        let open = FnLayout::build(&model(), &sizes());
        let mut shut = model();
        shut.folded = std::collections::HashSet::from([Spot::Owner(0)]);
        shut.owners[0].folded = true;
        shut.packs = HashMap::from([
            (Spot::Mark(1), Spot::Owner(0)),
            (Spot::Mark(2), Spot::Owner(0)),
        ]);
        let placed = FnLayout::build(&shut, &sizes());
        assert_eq!(placed.marks, open.marks, "a fold by hand re-laid the paper");
        assert_eq!(placed.owners, open.owners);
        assert_eq!(placed.frames, open.frames);
        assert_eq!(placed.size, open.size);
    }

    /// A fold the **packer** was allowed to skip is the other half of the rule:
    /// it is measured as its own label and counted row, and the sheet closes up
    /// over it. That only ever happens where the paper is being laid again
    /// regardless — a `visibility` change, or a session's first build.
    #[test]
    fn a_packed_fold_is_seated_as_its_own_box() {
        let mut shut = model();
        shut.folded = std::collections::HashSet::from([Spot::Owner(0)]);
        shut.packed = std::collections::HashSet::from([Spot::Owner(0)]);
        shut.owners[0].folded = true;
        let mut own = sizes();
        own.shut.insert(Spot::Owner(0), 70.0);
        let placed = FnLayout::build(&shut, &own);
        let container = placed.owners[&0];
        assert_eq!(container.h, OWNER_HEAD_H + PACKED_H + PAD);
        for hidden in [1u32, 2] {
            assert!(
                !placed.marks.contains_key(&hidden),
                "{hidden} is packed away"
            );
        }
        // The free declaration is still drawn, and the whole sheet got smaller.
        assert!(placed.marks.contains_key(&0));
        let whole = FnLayout::build(&model(), &sizes());
        assert!(placed.size.0 * placed.size.1 < whole.size.0 * whole.size.1);
    }

    /// A module frame with nothing admitted is never drawn — the model builds
    /// no husk for it, and the layout never invents one.
    #[test]
    fn the_layout_draws_only_the_frames_the_model_built() {
        let placed = FnLayout::build(&model(), &sizes());
        assert_eq!(placed.frames.len(), 2);
        assert_eq!(placed.owners.len(), 1);
    }
}
