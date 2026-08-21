//! Where the surface chart's marks and frames sit on the paper.
//!
//! A pure function of (frames, measured sizes): every block is measured before
//! anything is placed, a frame's ownership forest is tidied into trees, the
//! trees are shelved inside their module's frame, a module frame is shelved
//! inside the module above it however deep the path runs, and the crates are
//! shelved on the sheet. Each shelf
//! aims for a landscape box — the shape of the paper it will be read on — and
//! never gets narrower than its widest child. The same survey always draws the
//! same chart: no physics, no randomness, no measurement of anything the
//! browser has already laid out.
//!
//! A tree is tidied one layer per ownership depth: children in a row under
//! their parent, the parent centered over whichever of the two is wider. The
//! layers are a tree's own, not the frame's — aligning depth across trees would
//! buy nothing and cost the frame its shelves.

use std::collections::HashMap;

use crate::views::codemap::tree::Placed;
use crate::views::surface::model::{Anchor, Frame, Seat};

/// Frame furniture, in flow units — one unit is one CSS pixel at zoom 1.
const PAD: f64 = 14.0;
/// Clear paper on the top border for the engraved label chip.
const LABEL_H: f64 = 16.0;
/// Between two seated boxes. Wide enough that an edge can leave a block and
/// land on its neighbor without crossing a third.
const GAP: f64 = 20.0;
/// Between a parent block and the row of children under it. The owns edge that
/// seated them is drawn in this band, arrowhead and all, so it has to read as a
/// line rather than as a seam between two touching blocks.
const LAYER_GAP: f64 = 34.0;
/// How much wider than tall a packed shelf aims to be.
const LANDSCAPE: f64 = 2.4;
/// A frame narrower than this reads as a column of unrelated plates.
const MIN_FRAME_W: f64 = 160.0;

/// What the layout must be told about what it seats. Measuring belongs with the
/// drawing — the layout only places what it is handed.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Sizes {
    /// Every drawn mark's block, by mark id.
    pub marks: HashMap<u32, (f64, f64)>,
    /// Every counted fold row, by the anchor it carries.
    pub rows: HashMap<Anchor, (f64, f64)>,
    /// The width each frame's engraved label needs on its border.
    pub labels: HashMap<u32, f64>,
}

/// The whole chart, placed and centered on the flow origin.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SurfaceLayout {
    pub marks: HashMap<u32, Placed>,
    pub rows: HashMap<Anchor, Placed>,
    /// Outermost first, so a nested tint lays over its parent's.
    pub frames: Vec<(u32, Placed)>,
    pub size: (f64, f64),
}

impl SurfaceLayout {
    /// Where an edge's end is, whichever kind of anchor it landed on.
    pub fn rect(&self, anchor: Anchor) -> Option<Placed> {
        match anchor {
            Anchor::Mark(id) => self.marks.get(&id).copied(),
            other => self.rows.get(&other).copied(),
        }
    }
}

/// One thing a shelf seats: a tidied tree out of a frame's forest, or a nested
/// frame. A lone block is a tree of one.
enum Kid {
    Tree(Packed),
    Frame(u32, Packed),
}

#[derive(Default)]
struct Packed {
    w: f64,
    h: f64,
    marks: Vec<(u32, Placed)>,
    rows: Vec<(Anchor, Placed)>,
    frames: Vec<(u32, Placed)>,
}

fn shift(packed: Packed, dx: f64, dy: f64) -> Packed {
    let at = |p: Placed| Placed {
        x: p.x + dx,
        y: p.y + dy,
        ..p
    };
    Packed {
        w: packed.w,
        h: packed.h,
        marks: packed.marks.into_iter().map(|(k, p)| (k, at(p))).collect(),
        rows: packed.rows.into_iter().map(|(k, p)| (k, at(p))).collect(),
        frames: packed.frames.into_iter().map(|(k, p)| (k, at(p))).collect(),
    }
}

/// Seat a frame's children in shelves and draw the frame around them. The
/// forest comes first — a frame's own types before the frames nested in it —
/// and the child frames last, so the reading order down a frame is the same
/// everywhere on the chart.
fn shelve(kids: Vec<(Kid, f64, f64)>, label_w: f64) -> Packed {
    let widest = kids.iter().map(|(_, w, _)| *w).fold(0.0, f64::max);
    let area: f64 = kids.iter().map(|(_, w, h)| (w + GAP) * (h + GAP)).sum();
    let target = widest.max((area * LANDSCAPE).sqrt());

    let mut out = Packed::default();
    let (mut x, mut y, mut row_h, mut content_w) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (kid, w, h) in kids {
        if x > 0.0 && x + w > target {
            y += row_h + GAP;
            x = 0.0;
            row_h = 0.0;
        }
        match kid {
            Kid::Tree(packed) => {
                let inner = shift(packed, x, y);
                out.marks.extend(inner.marks);
                out.rows.extend(inner.rows);
            }
            Kid::Frame(id, packed) => {
                out.frames.push((id, Placed { x, y, w, h }));
                let inner = shift(packed, x, y);
                out.marks.extend(inner.marks);
                out.rows.extend(inner.rows);
                out.frames.extend(inner.frames);
            }
        }
        x += w + GAP;
        content_w = content_w.max(x - GAP);
        row_h = row_h.max(h);
    }

    // The frame around them, with room on the border for its own label.
    let w = (content_w + PAD * 2.0)
        .max(label_w + PAD * 2.0)
        .max(MIN_FRAME_W);
    let h = y + row_h + PAD * 2.0 + LABEL_H;
    let inner = shift(
        Packed {
            marks: out.marks,
            rows: out.rows,
            frames: out.frames,
            ..Default::default()
        },
        PAD,
        PAD + LABEL_H,
    );
    Packed {
        w,
        h,
        marks: inner.marks,
        rows: inner.rows,
        frames: inner.frames,
    }
}

/// One seated box, as the drawing measured it. The fallbacks only matter to a
/// caller that forgot to measure something; the chart always measures first.
fn box_of(anchor: Anchor, sizes: &Sizes) -> (f64, f64) {
    match anchor {
        Anchor::Mark(id) => sizes.marks.get(&id).copied().unwrap_or((MIN_FRAME_W, 40.0)),
        row => sizes.rows.get(&row).copied().unwrap_or((MIN_FRAME_W, 22.0)),
    }
}

/// Tidy one tree of the frame's forest: the block itself, its children side by
/// side one layer below, and the parent centered over whichever span is wider.
/// The tree's box is that span — from here on it shelves like any other block.
fn pack_tree(seat: &Seat, sizes: &Sizes) -> Packed {
    let (own_w, own_h) = box_of(seat.anchor, sizes);
    let kids: Vec<Packed> = seat.children.iter().map(|s| pack_tree(s, sizes)).collect();
    let kids_w: f64 =
        kids.iter().map(|k| k.w).sum::<f64>() + GAP * kids.len().saturating_sub(1) as f64;
    let kids_h = kids.iter().map(|k| k.h).fold(0.0, f64::max);

    let w = own_w.max(kids_w);
    let h = if kids.is_empty() {
        own_h
    } else {
        own_h + LAYER_GAP + kids_h
    };
    let mut out = Packed {
        w,
        h,
        ..Default::default()
    };
    let at = Placed {
        x: (w - own_w) / 2.0,
        y: 0.0,
        w: own_w,
        h: own_h,
    };
    match seat.anchor {
        Anchor::Mark(id) => out.marks.push((id, at)),
        row => out.rows.push((row, at)),
    }
    let mut x = (w - kids_w) / 2.0;
    for kid in kids {
        let step = kid.w + GAP;
        let placed = shift(kid, x, own_h + LAYER_GAP);
        out.marks.extend(placed.marks);
        out.rows.extend(placed.rows);
        x += step;
    }
    out
}

/// One frame's children, measured and ready to seat.
fn kids_of(frame: &Frame, frames: &[Frame], sizes: &Sizes) -> Vec<(Kid, f64, f64)> {
    let mut kids: Vec<(Kid, f64, f64)> = Vec::new();
    for seat in &frame.forest {
        let packed = pack_tree(seat, sizes);
        let (w, h) = (packed.w, packed.h);
        kids.push((Kid::Tree(packed), w, h));
    }
    for child in frames.iter().filter(|f| f.parent == Some(frame.id)) {
        let packed = shelve(
            kids_of(child, frames, sizes),
            sizes.labels.get(&child.id).copied().unwrap_or(0.0),
        );
        let (w, h) = (packed.w, packed.h);
        kids.push((Kid::Frame(child.id, packed), w, h));
    }
    kids
}

/// Lay every frame and every mark, centered on the flow origin.
pub fn layout(frames: &[Frame], sizes: &Sizes) -> SurfaceLayout {
    // The crate frames, side by side on the sheet.
    let sheet: Vec<(Kid, f64, f64)> = frames
        .iter()
        .filter(|f| f.parent.is_none())
        .map(|frame| {
            let packed = shelve(
                kids_of(frame, frames, sizes),
                sizes.labels.get(&frame.id).copied().unwrap_or(0.0),
            );
            let (w, h) = (packed.w, packed.h);
            (Kid::Frame(frame.id, packed), w, h)
        })
        .collect();

    // The sheet itself has no frame of its own: seat the crates, then take the
    // bounds they actually used.
    let mut out = Packed::default();
    let widest = sheet.iter().map(|(_, w, _)| *w).fold(0.0, f64::max);
    let area: f64 = sheet.iter().map(|(_, w, h)| (w + GAP) * (h + GAP)).sum();
    let target = widest.max((area * LANDSCAPE).sqrt());
    let (mut x, mut y, mut row_h, mut content_w) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (kid, w, h) in sheet {
        if x > 0.0 && x + w > target {
            y += row_h + GAP;
            x = 0.0;
            row_h = 0.0;
        }
        if let Kid::Frame(id, packed) = kid {
            out.frames.push((id, Placed { x, y, w, h }));
            let inner = shift(packed, x, y);
            out.marks.extend(inner.marks);
            out.rows.extend(inner.rows);
            out.frames.extend(inner.frames);
        }
        x += w + GAP;
        content_w = content_w.max(x - GAP);
        row_h = row_h.max(h);
    }
    let (w, h) = (content_w, y + row_h);
    let placed = shift(out, -w / 2.0, -h / 2.0);
    SurfaceLayout {
        marks: placed.marks.into_iter().collect(),
        rows: placed.rows.into_iter().collect(),
        frames: placed.frames,
        size: (w, h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame whose marks all seat as roots — the shape a frame has when
    /// nothing in it owns anything else.
    fn frame(id: u32, module: &[&str], parent: Option<u32>, marks: &[u32]) -> Frame {
        Frame {
            id,
            krate: "slope".to_string(),
            module: module.iter().map(|s| (*s).to_string()).collect(),
            parent,
            marks: marks.to_vec(),
            private: 0,
            more: 0,
            folded: false,
            packed: 0,
            forest: marks.iter().map(|&m| Seat::leaf(Anchor::Mark(m))).collect(),
        }
    }

    fn seat(mark: u32, children: &[u32]) -> Seat {
        Seat {
            anchor: Anchor::Mark(mark),
            children: children
                .iter()
                .map(|&c| Seat::leaf(Anchor::Mark(c)))
                .collect(),
        }
    }

    fn sizes(marks: &[u32]) -> Sizes {
        Sizes {
            marks: marks.iter().map(|&m| (m, (170.0, 64.0))).collect(),
            rows: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    fn overlaps(a: &Placed, b: &Placed) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    fn contains(outer: &Placed, inner: &Placed) -> bool {
        outer.x <= inner.x
            && outer.y <= inner.y
            && outer.x + outer.w >= inner.x + inner.w
            && outer.y + outer.h >= inner.y + inner.h
    }

    #[test]
    fn marks_nest_in_their_frames_and_never_overlap() {
        let mut api = frame(1, &["api"], Some(0), &[0, 1, 2]);
        // `Wire` owns the other two, so the frame seats one tree, not three
        // loose blocks.
        api.forest = vec![seat(0, &[1, 2])];
        let frames = vec![
            frame(0, &[], None, &[9]),
            api,
            frame(2, &["views"], Some(0), &[3, 4]),
        ];
        let placed = layout(&frames, &sizes(&[0, 1, 2, 3, 4, 9]));

        let boxes: Vec<Placed> = placed.marks.values().copied().collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                assert!(!overlaps(a, b), "marks overlap: {a:?} {b:?}");
            }
        }
        let of = |id: u32| placed.frames.iter().find(|(f, _)| *f == id).unwrap().1;
        let (crate_frame, api, views) = (of(0), of(1), of(2));
        assert!(contains(&crate_frame, &api));
        assert!(contains(&crate_frame, &views));
        assert!(!overlaps(&api, &views));
        assert!(contains(&api, &placed.marks[&0]));
        assert!(contains(&views, &placed.marks[&3]));
        // A crate-root mark sits in the crate's own frame, outside both
        // module frames.
        assert!(contains(&crate_frame, &placed.marks[&9]));
        assert!(!overlaps(&api, &placed.marks[&9]));
        // Ancestors paint first, so a nested tint lays over its parent's.
        assert_eq!(placed.frames[0].0, 0);
    }

    #[test]
    fn a_child_seats_one_layer_under_the_parent_that_owns_it() {
        let mut api = frame(0, &["api"], None, &[0, 1, 2]);
        api.forest = vec![seat(0, &[1, 2])];
        let placed = layout(&[api], &sizes(&[0, 1, 2]));
        let (root, left, right) = (placed.marks[&0], placed.marks[&1], placed.marks[&2]);

        // One layer down, with room between them for the owns edge.
        assert!(left.y - (root.y + root.h) >= LAYER_GAP - 0.001);
        assert_eq!(left.y, right.y);
        // Siblings in a row, in the survey's order, a gap apart.
        assert!(left.x + left.w + GAP <= right.x + 0.001);
        // The parent stands over the middle of what it owns.
        let span = (left.x + right.x + right.w) / 2.0;
        assert!((root.x + root.w / 2.0 - span).abs() < 0.001);
    }

    #[test]
    fn a_parent_wider_than_its_children_centers_them_under_itself() {
        let mut api = frame(0, &["api"], None, &[0, 1]);
        api.forest = vec![seat(0, &[1])];
        let sizes = Sizes {
            marks: [(0, (240.0, 64.0)), (1, (100.0, 40.0))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let placed = layout(&[api], &sizes);
        let (root, kid) = (placed.marks[&0], placed.marks[&1]);
        assert!((root.x + root.w / 2.0 - (kid.x + kid.w / 2.0)).abs() < 0.001);
        assert!(kid.x > root.x);
    }

    #[test]
    fn the_same_frames_always_draw_the_same_chart() {
        let mut api = frame(1, &["api"], Some(0), &[0, 1]);
        api.forest = vec![seat(0, &[1])];
        let frames = vec![
            frame(0, &[], None, &[]),
            api,
            frame(2, &["views"], Some(0), &[2]),
        ];
        let a = layout(&frames, &sizes(&[0, 1, 2]));
        let b = layout(&frames, &sizes(&[0, 1, 2]));
        assert_eq!(a, b);
    }

    /// The module tree runs as deep as the code does: `mod views::surface` is
    /// drawn inside `mod views`, inside the crate, and every block still lands
    /// in the frame that declares it.
    #[test]
    fn a_module_frame_nests_inside_the_module_above_it() {
        let frames = vec![
            frame(0, &[], None, &[]),
            frame(1, &["views"], Some(0), &[0]),
            frame(2, &["views", "surface"], Some(1), &[1]),
            frame(3, &["views", "surface", "wire"], Some(2), &[2]),
        ];
        let placed = layout(&frames, &sizes(&[0, 1, 2]));
        let of = |id: u32| placed.frames.iter().find(|(f, _)| *f == id).unwrap().1;
        let (root, views, surface, wire) = (of(0), of(1), of(2), of(3));
        assert!(contains(&root, &views));
        assert!(contains(&views, &surface));
        assert!(contains(&surface, &wire));
        assert!(contains(&views, &placed.marks[&0]));
        assert!(!overlaps(&surface, &placed.marks[&0]));
        assert!(contains(&surface, &placed.marks[&1]));
        assert!(contains(&wire, &placed.marks[&2]));
        // Outermost first, so a nested tint lays over the one it sits in.
        assert_eq!(
            placed.frames.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }
}
