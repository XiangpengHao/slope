//! Where the data chart's marks and frames sit on the paper.
//!
//! A pure function of (frames, measured sizes): every block is measured before
//! anything is placed, marks are shelved inside their module's frame, module
//! frames are shelved inside their crate's, and the crates are shelved on the
//! sheet. Each shelf aims for a landscape box — the shape of the paper it will
//! be read on — and never gets narrower than its widest child. The same survey
//! always draws the same chart: no physics, no randomness, no measurement of
//! anything the browser has already laid out.

use std::collections::HashMap;

use crate::views::codemap::tree::Placed;
use crate::views::datamap::model::{Anchor, Frame};

/// Frame furniture, in flow units — one unit is one CSS pixel at zoom 1.
const PAD: f64 = 14.0;
/// Clear paper on the top border for the engraved label chip.
const LABEL_H: f64 = 16.0;
/// Between two seated boxes. Wide enough that an edge can leave a block and
/// land on its neighbor without crossing a third.
const GAP: f64 = 20.0;
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
pub struct DataLayout {
    pub marks: HashMap<u32, Placed>,
    pub rows: HashMap<Anchor, Placed>,
    /// Outermost first, so a nested tint lays over its parent's.
    pub frames: Vec<(u32, Placed)>,
    pub size: (f64, f64),
}

impl DataLayout {
    /// Where an edge's end is, whichever kind of anchor it landed on.
    pub fn rect(&self, anchor: Anchor) -> Option<Placed> {
        match anchor {
            Anchor::Mark(id) => self.marks.get(&id).copied(),
            other => self.rows.get(&other).copied(),
        }
    }
}

/// One thing a shelf seats.
enum Kid {
    Mark(u32),
    Row(Anchor),
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

/// Seat a frame's children in shelves and draw the frame around them. Marks
/// come first — a frame's own types before the frames nested in it — then the
/// counted fold rows, then the child frames, so the reading order down a frame
/// is the same everywhere on the chart.
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
        let at = Placed { x, y, w, h };
        match kid {
            Kid::Mark(id) => out.marks.push((id, at)),
            Kid::Row(anchor) => out.rows.push((anchor, at)),
            Kid::Frame(id, packed) => {
                out.frames.push((id, at));
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

/// One frame's children, measured and ready to seat.
fn kids_of(frame: &Frame, frames: &[Frame], sizes: &Sizes) -> Vec<(Kid, f64, f64)> {
    let mut kids: Vec<(Kid, f64, f64)> = Vec::new();
    for &mark in &frame.marks {
        let (w, h) = sizes.marks.get(&mark).copied().unwrap_or((MIN_FRAME_W, 40.0));
        kids.push((Kid::Mark(mark), w, h));
    }
    for anchor in [Anchor::Private(frame.id), Anchor::More(frame.id)] {
        let counted = match anchor {
            Anchor::Private(_) => frame.private,
            _ => frame.more,
        };
        if counted == 0 {
            continue;
        }
        let (w, h) = sizes.rows.get(&anchor).copied().unwrap_or((MIN_FRAME_W, 22.0));
        kids.push((Kid::Row(anchor), w, h));
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
pub fn layout(frames: &[Frame], sizes: &Sizes) -> DataLayout {
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
    DataLayout {
        marks: placed.marks.into_iter().collect(),
        rows: placed.rows.into_iter().collect(),
        frames: placed.frames,
        size: (w, h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(id: u32, module: Option<&str>, parent: Option<u32>, marks: &[u32]) -> Frame {
        Frame {
            id,
            krate: "slopify".to_string(),
            module: module.map(str::to_string),
            parent,
            marks: marks.to_vec(),
            private: 0,
            more: 0,
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
        let frames = vec![
            frame(0, None, None, &[9]),
            frame(1, Some("api"), Some(0), &[0, 1, 2]),
            frame(2, Some("views"), Some(0), &[3, 4]),
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
    fn the_same_frames_always_draw_the_same_chart() {
        let frames = vec![
            frame(0, None, None, &[]),
            frame(1, Some("api"), Some(0), &[0, 1]),
            frame(2, Some("views"), Some(0), &[2]),
        ];
        let a = layout(&frames, &sizes(&[0, 1, 2]));
        let b = layout(&frames, &sizes(&[0, 1, 2]));
        assert_eq!(a, b);
    }
}
