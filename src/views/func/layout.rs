//! Where the function chart's blocks sit: the **shelved section**.
//!
//! Containment is the call. Every declaration is seated inside the frame of the
//! caller that reaches it first, and what a frame holds shelves in wrapped rows
//! under its head — the same move the data chart makes when a held type nests
//! inside its holder, one rung down. The entry points are the frames on the
//! ground, packed in wrapped rows across the sheet; what no entry point reaches
//! shelves in its own strip below them.
//!
//! A pure function of (model, measured blocks). Every block is measured before
//! anything is placed — its head row and the signature quoted under it — a
//! frame is measured from that box and what shelves in it, and the same survey
//! always draws the same chart: no physics, no randomness, no measurement of
//! anything the browser has already laid out.
//!
//! A **fold** is part of that purity, not an overlay on it — but it is an
//! elision, not a re-packing. A frame the reviewer folded keeps its whole
//! footprint here: every mark inside it is still measured and still placed, so
//! no sibling, ancestor or wire moves by a pixel when the reader closes it, and
//! the eye stays on whatever it was anchored to. What the fold does is take
//! those marks off the *drawing*; the frame says how many in words, where its
//! shelf stood.
//!
//! The one exception is `model.packed` — the folds the packer is allowed to
//! skip, which is only ever the set the paper was already being laid again
//! around: an `order` or `visibility` change, or a session's first build. A
//! frame in that set is measured and placed as though it shelved nothing, and
//! the sheet closes up over it. Nothing folds itself and nothing folds by a
//! count; both sets are frames a reader closed by hand.
//!
//! The packing is wrapped rows rather than the data chart's skyline. A shelf
//! here reads left to right in the order the reading seats it — the order the
//! arrow keys walk — and a skyline drops a short box back into a hole two rows
//! up, which is a tighter box and a shelf nobody can follow.

use std::collections::HashMap;

use dioxus_flow::prelude::Point;

use crate::views::func::model::FnModel;

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
/// a wire can leave without crossing the block it belongs to. `Under` is the
/// band's own foot, which is the exit for a call into the block's own shelf; the
/// shelf is the sheet's ground, not the block's paper.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum TieSide {
    Top,
    Left,
    Right,
    Under,
}

impl Placed {
    /// How far along this block's head a wire may tie: the first `HEAD_TIE`
    /// units of it. A frame is as wide as everything it calls, and a line tied
    /// out in a two-thousand-unit head names nothing, so the top and the foot
    /// tie beside the name.
    fn span(&self) -> (f64, f64) {
        let w = self.w.min(HEAD_TIE);
        let pad = (w * 0.18).min(FAN_PAD);
        (self.x + pad, self.x + w - pad)
    }

    /// The band this block covers: its head row and its quotation, `own` tall.
    /// The rest of a frame's box is the shelf it holds, which is the sheet's own
    /// ground and no part of the block's paper.
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
    /// height, the band without the shelf under it.
    ///
    /// The head row's own centre was the tie once, and centre-to-centre is
    /// exactly what drove every line through the text it was pointing at: a wire
    /// had to cross half a head row, and often the whole quotation under it, to
    /// reach the point it ended on. An edge tie starts where the paper ends.
    pub(super) fn tie_side(&self, own: f64, other: Placed) -> TieSide {
        let band = self.band(own);
        let them = other.band(HEAD_H);
        let (them_x, them_y) = (them.x + them.w.min(HEAD_TIE) / 2.0, them.y + them.h / 2.0);
        // Above my band: the top edge faces it, and above a head row is always
        // somebody else's paper.
        if them_y < band.y {
            return TieSide::Top;
        }
        // Below my band and inside my own width: a call into my own shelf, so
        // the wire leaves by the band's foot and stays in the shelf it is
        // reaching into.
        if them_y > band.y + band.h && them_x > band.x && them_x < band.x + band.w {
            return TieSide::Under;
        }
        // Beside me: out of the band's own side. Never the head's cut span —
        // that edge is in the middle of the head text on any frame wider than
        // `HEAD_TIE`, which is every frame.
        let (from, to) = self.span();
        match them_x < (from + to) / 2.0 {
            true => TieSide::Left,
            false => TieSide::Right,
        }
    }

    /// Where on that edge the wire ties. `slot` runs 0..1 across the edge's own
    /// fanning span, so several wires sharing one edge stand apart instead of
    /// stacking on one point; one wire alone takes the middle, which is where
    /// the old centre tie sat.
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
                // Down the head row, which is the row the tie is about however
                // deep the quotation under it runs.
                Point::new(x, band.y + 2.0 + slot * (HEAD_H - 4.0))
            }
        }
    }
}

/// The head row's own height, borders excluded. These numbers are the CSS in
/// `tailwind.css`; move one and the other must follow.
pub(super) const HEAD_H: f64 = 16.0;
/// Clear paper inside a frame, around what shelves in it. Shared with the
/// drawing, which aligns the far edition's engraved landmark with the shelf
/// this reserves.
pub(super) const PAD: f64 = 5.0;
/// The hairline rule that closes a block's own quotation and opens the shelf
/// under it: the rule, and the clear paper on either side of it. Shared with
/// the drawing, which measures a folded frame's counted words under the same
/// rule.
pub(super) const RULE: f64 = 6.0;
/// Between two blocks on one shelf.
const GAP: f64 = 4.0;
/// Between two frames on the ground, which is a shelf of its own.
const GROUND_GAP: f64 = 14.0;
/// Above the ring strip: it is a different reading of the paper, and the gap is
/// what says so before its caption does.
const RING_GAP: f64 = 34.0;
/// How much wider than tall a packed shelf aims to be.
const LANDSCAPE: f64 = 2.4;
/// And the ground, which is read on a landscape glass whole.
const GROUND_LANDSCAPE: f64 = 2.3;
/// How far along a block's head a wire ties, at the widest.
const HEAD_TIE: f64 = 150.0;
/// Clear head row left at each end of a fanned edge, so a wire never ties on
/// the corner it would round.
const FAN_PAD: f64 = 14.0;

/// What the layout must be told about what it seats: every drawn mark's own
/// box — its head row, the signature quoted under it, and the counted words a
/// folded frame writes — by mark id. Measuring belongs with the drawing; the
/// layout only places what it is handed.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct Sizes {
    pub(super) own: HashMap<u32, (f64, f64)>,
}

impl Sizes {
    fn of(&self, id: u32) -> (f64, f64) {
        self.own.get(&id).copied().unwrap_or((120.0, HEAD_H + 2.0))
    }
}

/// The caption over the ring strip: what stands there, and the band selecting
/// it pushes. The strip is the one place this chart still captions a band,
/// because a frame no entry point reaches has nothing above it to say so.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct RingStrip {
    pub(super) at: Placed,
    pub(super) caption: String,
    pub(super) band: u32,
}

/// The whole chart, placed and centered on the flow origin.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnLayout {
    pub(super) marks: HashMap<u32, Placed>,
    pub(super) ring: Option<RingStrip>,
    pub(super) size: (f64, f64),
}

/// One shelf, packed: where each box sits relative to the shelf's corner, and
/// the box the packing needed.
struct Shelf {
    at: Vec<(f64, f64)>,
    w: f64,
    h: f64,
}

/// Seat boxes in wrapped rows no wider than `target`, in the order given.
fn shelve(boxes: &[(f64, f64)], target: f64, gap: f64) -> Shelf {
    let (mut x, mut y, mut row_h, mut w) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut at = Vec::with_capacity(boxes.len());
    for &(bw, bh) in boxes {
        if x > 0.0 && x + bw > target {
            y += row_h + gap;
            x = 0.0;
            row_h = 0.0;
        }
        at.push((x, y));
        x += bw + gap;
        row_h = row_h.max(bh);
        w = w.max(x - gap);
    }
    let h = if boxes.is_empty() { 0.0 } else { y + row_h };
    Shelf { at, w, h }
}

/// The width a shelf of these boxes aims for: never narrower than its widest
/// box, and otherwise the width that makes the shelf a landscape box — the
/// shape of the paper it is read on.
fn target_of(boxes: &[(f64, f64)], floor: f64, gap: f64, landscape: f64) -> f64 {
    let widest = boxes.iter().map(|b| b.0).fold(floor, f64::max);
    let area: f64 = boxes.iter().map(|(w, h)| (w + gap) * (h + gap)).sum();
    widest.max((area * landscape).sqrt())
}

impl FnLayout {
    /// Seat every mark on the shelved section.
    pub(super) fn build(model: &FnModel, sizes: &Sizes) -> Self {
        let mut out = Self::shelved(model, sizes);
        // Centered on the origin, so the camera's first fit is one move.
        let (w, h) = out.size;
        out.shift(-w / 2.0, -h / 2.0);
        out
    }

    fn shift(&mut self, dx: f64, dy: f64) {
        for placed in self.marks.values_mut() {
            placed.x += dx;
            placed.y += dy;
        }
        if let Some(ring) = self.ring.as_mut() {
            ring.at.x += dx;
            ring.at.y += dy;
        }
    }

    fn shelved(model: &FnModel, sizes: &Sizes) -> FnLayout {
        let mut measure = Measure {
            model,
            sizes,
            size: HashMap::new(),
            rel: HashMap::new(),
        };
        let mut out = FnLayout::default();

        // The ground: the entry points, and any frame whose way in this reading
        // does not draw, packed the way a frame packs its own shelves.
        let ground: Vec<(f64, f64)> = model.seats.iter().map(|&id| measure.of(id)).collect();
        let target = target_of(&ground, 0.0, GROUND_GAP, GROUND_LANDSCAPE);
        let shelf = shelve(&ground, target, GROUND_GAP);
        for (&id, (x, y)) in model.seats.iter().zip(&shelf.at) {
            measure.place(id, *x, *y, &mut out.marks);
        }

        // And the ring below it, in a strip of its own.
        let mut size = (shelf.w, shelf.h);
        if !model.ring.is_empty() {
            let top = shelf.h + RING_GAP;
            let boxes: Vec<(f64, f64)> = model.ring.iter().map(|&id| measure.of(id)).collect();
            let target = target_of(&boxes, 0.0, GROUND_GAP, GROUND_LANDSCAPE);
            let strip = shelve(&boxes, target, GROUND_GAP);
            for (&id, (x, y)) in model.ring.iter().zip(&strip.at) {
                measure.place(id, *x, top + *y, &mut out.marks);
            }
            out.ring = Some(RingStrip {
                at: Placed {
                    x: 0.0,
                    y: top,
                    w: strip.w.max(shelf.w),
                    h: strip.h,
                },
                caption: "in a call ring — no entry point reaches these".to_string(),
                band: model.facts.deepest + 1,
            });
            size = (size.0.max(strip.w), top + strip.h);
        }
        out.size = size;
        out
    }
}

/// One measuring pass over the seating tree: a frame is as wide as its own head
/// or the shelves inside it, whichever is wider, and as tall as it needs to
/// hold them.
struct Measure<'m> {
    model: &'m FnModel,
    sizes: &'m Sizes,
    size: HashMap<u32, (f64, f64)>,
    /// Where each mark sits inside the frame it shelves in.
    rel: HashMap<u32, (f64, f64)>,
}

impl Measure<'_> {
    /// What the packer seats inside one frame: everything, unless this is one of
    /// the folds the packer was allowed to skip. A fold by hand is *not* in that
    /// set, so its contents are still measured and still placed — the footprint
    /// is what keeps the rest of the sheet still — and the drawing is what
    /// leaves them off the paper.
    fn kids_of(&self, id: u32) -> Vec<u32> {
        if self.model.packed.contains(&id) {
            return Vec::new();
        }
        self.model
            .kids
            .get(&id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            // A tree, never a graph: the way in gives each mark one parent, so
            // this walk terminates. The guard is against a survey that ever
            // handed one a cycle.
            .filter(|kid| *kid != id)
            .collect()
    }

    fn of(&mut self, id: u32) -> (f64, f64) {
        if let Some(&size) = self.size.get(&id) {
            return size;
        }
        let (own_w, own_h) = self.sizes.of(id);
        let kids = self.kids_of(id);
        if kids.is_empty() {
            let size = (own_w, own_h);
            self.size.insert(id, size);
            return size;
        }
        let boxes: Vec<(f64, f64)> = kids.iter().map(|&kid| self.of(kid)).collect();
        let target = target_of(&boxes, own_w, GAP, LANDSCAPE);
        let shelf = shelve(&boxes, target, GAP);
        for (&kid, at) in kids.iter().zip(&shelf.at) {
            self.rel.insert(kid, *at);
        }
        let size = (own_w.max(shelf.w + PAD * 2.0), own_h + RULE + shelf.h + PAD);
        self.size.insert(id, size);
        size
    }

    fn place(&mut self, id: u32, x: f64, y: f64, out: &mut HashMap<u32, Placed>) {
        let (w, h) = self.of(id);
        out.insert(id, Placed { x, y, w, h });
        let (_, own_h) = self.sizes.of(id);
        for kid in self.kids_of(id) {
            if out.contains_key(&kid) {
                continue;
            }
            let (dx, dy) = self.rel.get(&kid).copied().unwrap_or((0.0, 0.0));
            self.place(kid, x + PAD + dx, y + own_h + RULE + dy, out);
        }
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::data::{Delta, ItemKind, Vis};
    use crate::views::func::model::{FnFacts, FnHead, FnMark, Tier};

    fn mark(id: u32, tier: Tier) -> FnMark {
        FnMark {
            id,
            tier,
            krate: "slope".to_string(),
            module: Vec::new(),
            head: FnHead {
                kind: ItemKind::Fn,
                vis: Vis::Pub,
                name: format!("fn{id}"),
                label: format!("fn{id}"),
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
            crosses: false,
            recurses: false,
            folded: false,
        }
    }

    /// One entry point holding three callees, a second holding none, and a
    /// ring of one.
    fn model() -> FnModel {
        FnModel {
            marks: vec![
                mark(0, Tier::Entry),
                mark(1, Tier::Deep(1)),
                mark(2, Tier::Deep(1)),
                mark(3, Tier::Deep(2)),
                mark(4, Tier::Entry),
                mark(5, Tier::Ring),
            ],
            via: HashMap::from([(1, 0), (2, 0), (3, 1)]),
            kids: HashMap::from([(0, vec![1, 2]), (1, vec![3])]),
            seats: vec![0, 4],
            ring: vec![5],
            facts: FnFacts {
                deepest: 2,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Every block the same size: a head row, two quoted signature rows and the
    /// paper around them.
    const OWN_H: f64 = 48.0;

    fn sizes() -> Sizes {
        Sizes {
            own: (0..6).map(|id| (id, (160.0, OWN_H))).collect(),
        }
    }

    fn overlaps(a: &Placed, b: &Placed) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    fn holds(frame: &Placed, kid: &Placed) -> bool {
        frame.x <= kid.x
            && frame.y <= kid.y
            && frame.x + frame.w >= kid.x + kid.w
            && frame.y + frame.h >= kid.y + kid.h
    }

    /// Every mark is seated exactly once, and a frame holds what shelves inside
    /// it outright — containment is the call, so a block outside its caller's
    /// box would be the chart saying something untrue.
    #[test]
    fn every_mark_is_seated_inside_the_caller_that_reaches_it() {
        let placed = FnLayout::build(&model(), &sizes());
        assert_eq!(placed.marks.len(), 6, "a mark went missing");
        for (kid, up) in [(1u32, 0u32), (2, 0), (3, 1)] {
            assert!(
                holds(&placed.marks[&up], &placed.marks[&kid]),
                "{up} does not hold {kid}"
            );
        }
        // And two frames on the ground never share paper.
        assert!(!overlaps(&placed.marks[&0], &placed.marks[&4]));
        assert!(!overlaps(&placed.marks[&1], &placed.marks[&2]));
    }

    /// The ring stands in a strip under the ground, captioned, with the band
    /// its caption selects.
    #[test]
    fn a_call_ring_shelves_in_its_own_strip() {
        let placed = FnLayout::build(&model(), &sizes());
        let strip = placed.ring.expect("a ring strip");
        assert_eq!(strip.band, 3);
        assert!(strip.caption.starts_with("in a call ring"));
        let ring = placed.marks[&5];
        let ground = placed.marks[&0];
        assert!(ring.y > ground.y + ground.h, "the strip stands below");
        assert!(strip.at.y <= ring.y);
    }

    /// The same model always draws the same chart.
    #[test]
    fn the_same_model_always_draws_the_same_chart() {
        let a = FnLayout::build(&model(), &sizes());
        let b = FnLayout::build(&model(), &sizes());
        assert_eq!(a, b);
    }

    /// A shelf reads in the order it was seated in, left to right and then
    /// down: the order the arrow keys walk is the order the eye walks.
    #[test]
    fn a_shelf_reads_in_seating_order() {
        let mut model = model();
        model.kids.insert(0, vec![2, 1]);
        let placed = FnLayout::build(&model, &sizes());
        let (first, second) = (placed.marks[&2], placed.marks[&1]);
        assert!(
            first.y < second.y || (first.y == second.y && first.x < second.x),
            "the shelf is out of order"
        );
    }

    /// A leaf is its own box — the head row and the signature quoted under it —
    /// and nothing more; a frame grows to hold what it calls.
    #[test]
    fn a_frame_grows_and_a_leaf_does_not() {
        let placed = FnLayout::build(&model(), &sizes());
        assert_eq!(placed.marks[&4].h, OWN_H);
        assert_eq!(placed.marks[&4].w, 160.0);
        assert!(placed.marks[&0].h > placed.marks[&1].h);
        assert!(placed.marks[&0].w >= 160.0);
    }

    /// **A fold by hand moves nothing.** This is the invariant the whole
    /// elision exists for (2026-08-27, user): the reviewer folds a frame to get
    /// it out of the way of something they are looking at, so if the sheet
    /// re-packs, the fold has thrown away the very thing it was serving. Every
    /// other frame keeps its exact position and its exact size, the folded
    /// frame keeps its own footprint, and the sheet keeps its own bounds.
    #[test]
    fn a_fold_by_hand_moves_no_other_frame() {
        let open = FnLayout::build(&model(), &sizes());
        let mut shut = model();
        shut.folded = std::collections::HashSet::from([1]);
        shut.packs = HashMap::from([(3, 1)]);
        let placed = FnLayout::build(&shut, &sizes());

        // Every mark still sits exactly where it sat — the folded frame and the
        // marks it hides included, because the footprint is what keeps the rest
        // of the sheet still.
        assert_eq!(placed.marks, open.marks, "a fold by hand re-laid the paper");
        assert_eq!(placed.size, open.size);
        assert_eq!(placed.ring, open.ring);
    }

    /// A fold the **packer** was allowed to skip is the other half of the rule:
    /// it is measured and placed as though it shelved nothing, the sheet closes
    /// up over it, and what it hides is not placed at all. That only ever
    /// happens where the paper was being laid again regardless — an `order` or
    /// `visibility` change, or a session's first build — so there is no anchor
    /// for it to disrupt.
    #[test]
    fn a_packed_fold_is_seated_as_its_own_box() {
        let mut shut = model();
        shut.folded = std::collections::HashSet::from([0]);
        shut.packed = std::collections::HashSet::from([0]);
        shut.packs = HashMap::from([(1, 0), (2, 0), (3, 0)]);
        // The packer is handed the folded frame's own box, counted words and
        // all, exactly as the drawing measures one.
        let mut own = sizes();
        own.own.insert(0, (160.0, OWN_H));
        let placed = FnLayout::build(&shut, &own);
        assert_eq!(placed.marks[&0].h, OWN_H);
        assert_eq!(placed.marks[&0].w, 160.0);
        // What it hides is not placed at all, so nothing on the paper overlaps
        // and nothing is drawn twice.
        for hidden in [1u32, 2, 3] {
            assert!(
                !placed.marks.contains_key(&hidden),
                "{hidden} is packed away"
            );
        }
        // The frames that were never folded still stand.
        assert!(placed.marks.contains_key(&4));
        assert!(placed.marks.contains_key(&5));
        // And the whole sheet got smaller, because the paper was laid again.
        let whole = FnLayout::build(&model(), &sizes());
        assert!(placed.size.0 * placed.size.1 < whole.size.0 * whole.size.1);
    }

    /// A fold is deterministic, like every other reading: the same folds always
    /// draw the same chart, and opening the fold again draws the first one.
    #[test]
    fn the_same_folds_always_draw_the_same_chart() {
        let mut folded = model();
        folded.folded = std::collections::HashSet::from([1]);
        folded.packed = std::collections::HashSet::from([1]);
        folded.packs = HashMap::from([(3, 1)]);
        assert_eq!(
            FnLayout::build(&folded, &sizes()),
            FnLayout::build(&folded, &sizes())
        );
        // Unfolding is not a new chart: it is the chart that was there before.
        let open = FnLayout::build(&model(), &sizes());
        folded.folded.clear();
        folded.packed.clear();
        folded.packs.clear();
        assert_eq!(FnLayout::build(&folded, &sizes()), open);
    }
}
