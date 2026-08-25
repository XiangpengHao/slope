//! Where the function chart's blocks sit: the **section**.
//!
//! Bands of call depth run the full width of the sheet, captioned at the left
//! margin the way the dependency chart's rings caption their hops, and prisms
//! cross every band — one per frame the grouping asks for, a module or a type
//! or a file. A mark sits at the crossing of its depth and its frame, so both
//! readings come off the paper at once.
//!
//! A pure function of (model, measured sizes). Every block is measured before
//! anything is placed, and the same survey always draws the same chart: no
//! physics, no randomness, no measurement of anything the browser has already
//! laid out.
//!
//! Two other seatings stood here until 2026-08-25 and were cut by the user
//! after all three were built and read on a real workspace: a **mechanism**
//! plate (nested module frames with parts in depth rows inside them) read
//! whose code before what runs from where, and a **strips** plate (one road
//! per entry point, stations running right) came out a twenty-thousand-unit
//! ribbon — twenty-four roads, the busy ones stacking a hundred marks at one
//! depth. What the strips plate wanted is a selection on this one.

use std::collections::HashMap;

use dioxus_flow::prelude::Point;

use crate::views::data::layout::skyline;
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

impl Placed {
    pub(super) fn center(&self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Which sides of two boxes face each other, so a wire leaves and lands on
    /// open paper instead of crossing its own block.
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

/// Between two seated blocks. Wide enough that a wire can leave a block and
/// land on its neighbour without crossing a third.
const GAP: f64 = 18.0;
/// How much wider than tall a packed shelf aims to be.
const LANDSCAPE: f64 = 2.4;
/// A frame narrower than this reads as a column of unrelated plates.
const MIN_FRAME_W: f64 = 170.0;
/// The strata plate's left margin, where a band is captioned, and the head
/// room its prisms need for their engraved names.
const BAND_CAP_W: f64 = 96.0;
const PRISM_NAME_H: f64 = 22.0;
const BAND_GAP: f64 = 22.0;
const PRISM_GAP: f64 = 26.0;

/// What the layout must be told about what it seats. Measuring belongs with
/// the drawing — the layout only places what it is handed.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct Sizes {
    /// Every drawn mark's block, by mark id.
    pub(super) marks: HashMap<u32, (f64, f64)>,
    /// The width each frame's engraved name needs along the top of its prism.
    /// A column narrower than its own name would clip the one word that says
    /// whose column it is.
    pub(super) labels: HashMap<u32, f64>,
}

impl Sizes {
    fn of(&self, id: u32) -> (f64, f64) {
        self.marks.get(&id).copied().unwrap_or((MIN_FRAME_W, 40.0))
    }
}

/// One band of the section: a full-width lane of one call depth, captioned at
/// the left margin. The caption is the band's own words, and the band is a
/// focus like any other, so the lane is a control and not only a rule.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Lane {
    pub(super) at: Placed,
    pub(super) caption: String,
    /// The band this lane draws — the focus its caption selects.
    pub(super) band: u32,
}

/// One packed cell of the strata grid: where each of its marks sits inside the
/// cell, which marks those are, and the box the packing needed.
type Cell = (Vec<(f64, f64)>, Vec<u32>, f64, f64);

/// One prism of the section: a column crossing every band, standing for one
/// frame — a module, a type, or a file, as the grouping asks.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Prism {
    pub(super) frame: u32,
    pub(super) at: Placed,
    /// The name engraved along its top — the frame's **whole** path, because a
    /// section has no nesting to disambiguate one: this workspace writes three
    /// modules called `data`, and three prisms saying `data` name nothing.
    pub(super) written: String,
    /// What selecting the boundary pushes.
    pub(super) key: Vec<String>,
}

/// The whole chart, placed and centered on the flow origin.
#[derive(Clone, PartialEq, Debug, Default)]
pub(super) struct FnLayout {
    pub(super) marks: HashMap<u32, Placed>,
    /// Outermost first, so a nested tint lays over its parent's.
    pub(super) frames: Vec<(u32, Placed)>,
    pub(super) lanes: Vec<Lane>,
    pub(super) prisms: Vec<Prism>,
    pub(super) size: (f64, f64),
}

impl FnLayout {
    /// Seat every mark on the section.
    pub(super) fn build(model: &FnModel, sizes: &Sizes) -> Self {
        let mut out = Self::strata(model, sizes);
        // Centered on the origin, so the camera's first fit is one move.
        let (w, h) = out.size;
        out.shift(-w / 2.0, -h / 2.0);
        out
    }

    fn shift(&mut self, dx: f64, dy: f64) {
        let at = |p: &mut Placed| {
            p.x += dx;
            p.y += dy;
        };
        for placed in self.marks.values_mut() {
            at(placed);
        }
        for (_, placed) in self.frames.iter_mut() {
            at(placed);
        }
        for lane in self.lanes.iter_mut() {
            at(&mut lane.at);
        }
        for prism in self.prisms.iter_mut() {
            at(&mut prism.at);
        }
    }
}

impl FnLayout {
    /// One packed shelf: boxes seated bottom-left inside a target width.
    fn shelve(boxes: &[(f64, f64)], gap: f64) -> (Vec<(f64, f64)>, f64, f64) {
        if boxes.is_empty() {
            return (Vec::new(), 0.0, 0.0);
        }
        let widest = boxes.iter().map(|b| b.0).fold(0.0, f64::max);
        let area: f64 = boxes.iter().map(|(w, h)| (w + gap) * (h + gap)).sum();
        let target = widest.max((area * LANDSCAPE).sqrt());
        let at = skyline(boxes, target, gap);
        let (mut w, mut h) = (0.0f64, 0.0f64);
        for ((x, y), (bw, bh)) in at.iter().zip(boxes) {
            w = w.max(x + bw);
            h = h.max(y + bh);
        }
        (at, w, h)
    }

    fn strata(model: &FnModel, sizes: &Sizes) -> FnLayout {
        let bands: Vec<u32> = model.bands.iter().map(|(band, _)| *band).collect();
        // Every cell packed first: a column is as wide as its widest cell, and a
        // band as tall as its tallest, so the grid is a true section.
        let mut cells: HashMap<(u32, u32), Cell> = HashMap::new();
        for column in &model.columns {
            for (band, ids) in &column.cells {
                let boxes: Vec<(f64, f64)> = ids.iter().map(|&id| sizes.of(id)).collect();
                let (at, w, h) = Self::shelve(&boxes, GAP);
                cells.insert((column.frame, *band), (at, ids.clone(), w, h));
            }
        }
        let column_w: HashMap<u32, f64> = model
            .columns
            .iter()
            .map(|c| {
                let w = bands
                    .iter()
                    .filter_map(|band| cells.get(&(c.frame, *band)))
                    .map(|(_, _, w, _)| *w)
                    .fold(0.0, f64::max);
                let named = sizes.labels.get(&c.frame).copied().unwrap_or(0.0);
                (c.frame, w.max(named).max(MIN_FRAME_W))
            })
            .collect();
        let band_h: HashMap<u32, f64> = bands
            .iter()
            .map(|band| {
                let h = model
                    .columns
                    .iter()
                    .filter_map(|c| cells.get(&(c.frame, *band)))
                    .map(|(_, _, _, h)| *h)
                    .fold(0.0, f64::max);
                (*band, h.max(24.0))
            })
            .collect();

        let mut out = FnLayout::default();
        let mut x = BAND_CAP_W;
        let mut prism_x: HashMap<u32, f64> = HashMap::new();
        for column in &model.columns {
            prism_x.insert(column.frame, x);
            x += column_w[&column.frame] + PRISM_GAP;
        }
        let sheet_w = (x - PRISM_GAP).max(BAND_CAP_W);

        let mut y = PRISM_NAME_H;
        let mut band_y: HashMap<u32, f64> = HashMap::new();
        for band in &bands {
            band_y.insert(*band, y);
            out.lanes.push(Lane {
                at: Placed {
                    x: 0.0,
                    y,
                    w: sheet_w,
                    h: band_h[band],
                },
                caption: model.caption_of(*band),
                band: *band,
            });
            y += band_h[band] + BAND_GAP;
        }
        let sheet_h = (y - BAND_GAP).max(PRISM_NAME_H);

        for column in &model.columns {
            let cx = prism_x[&column.frame];
            out.prisms.push(Prism {
                frame: column.frame,
                at: Placed {
                    x: cx,
                    y: 0.0,
                    w: column_w[&column.frame],
                    h: sheet_h,
                },
                written: column.written.clone(),
                key: column.key.clone(),
            });
            for (band, _) in &column.cells {
                let Some((at, ids, _, _)) = cells.get(&(column.frame, *band)) else {
                    continue;
                };
                for ((&id, (dx, dy)), (w, h)) in ids
                    .iter()
                    .zip(at)
                    .zip(ids.iter().map(|&id| sizes.of(id)).collect::<Vec<_>>())
                {
                    out.marks.insert(
                        id,
                        Placed {
                            x: cx + dx,
                            y: band_y[band] + dy,
                            w,
                            h,
                        },
                    );
                }
            }
        }
        out.size = (sheet_w, sheet_h);
        out
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::data::{Delta, ItemKind, Vis};
    use crate::views::func::model::{Column, FnFacts, FnHead, FnMark, Frame, Tier};

    fn mark(id: u32, frame: u32, tier: Tier) -> FnMark {
        FnMark {
            id,
            frame,
            tier,
            road: None,
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
            delta: Delta::Same,
            callers: 0,
            calls: 0,
            touches: 0,
            recurses: false,
        }
    }

    fn frame(id: u32, module: &[&str], parent: Option<u32>, marks: &[u32]) -> Frame {
        Frame {
            id,
            krate: "slope".to_string(),
            module: module.iter().map(|s| (*s).to_string()).collect(),
            group: String::new(),
            parent,
            marks: marks.to_vec(),
        }
    }

    /// Two entry points, two one-deep calls, in two modules.
    fn model() -> FnModel {
        FnModel {
            marks: vec![
                mark(0, 1, Tier::Entry),
                mark(1, 1, Tier::Deep(1)),
                mark(2, 2, Tier::Entry),
                mark(3, 2, Tier::Deep(1)),
            ],
            frames: vec![
                frame(0, &[], None, &[]),
                frame(1, &["analyze"], Some(0), &[0, 1]),
                frame(2, &["views"], Some(0), &[2, 3]),
            ],
            bands: vec![(0, "entry".to_string()), (1, "1 call deep".to_string())],
            columns: vec![
                Column {
                    frame: 1,
                    written: "analyze".to_string(),
                    key: vec!["slope".to_string(), "analyze".to_string()],
                    cells: vec![(0, vec![0]), (1, vec![1])],
                },
                Column {
                    frame: 2,
                    written: "views".to_string(),
                    key: vec!["slope".to_string(), "views".to_string()],
                    cells: vec![(0, vec![2]), (1, vec![3])],
                },
            ],
            facts: FnFacts {
                deepest: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn sizes() -> Sizes {
        Sizes {
            marks: (0..4).map(|id| (id, (180.0, 60.0))).collect(),
            labels: HashMap::new(),
        }
    }

    fn overlaps(a: &Placed, b: &Placed) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    /// No two blocks may share paper, and every mark must be placed exactly
    /// once.
    #[test]
    fn the_section_seats_every_mark_and_overlaps_nothing() {
        let placed = FnLayout::build(&model(), &sizes());
        assert_eq!(placed.marks.len(), 4, "a mark went missing");
        let boxes: Vec<Placed> = placed.marks.values().copied().collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                assert!(!overlaps(a, b), "blocks overlap: {a:?} {b:?}");
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

    /// A band runs the full width and a prism crosses every band, so a mark
    /// sits at the crossing of its depth and its frame.
    #[test]
    fn bands_are_crossed_by_every_prism() {
        let placed = FnLayout::build(&model(), &sizes());
        // Same band, different modules: same row, different columns.
        assert_eq!(placed.marks[&0].y, placed.marks[&2].y);
        assert!(placed.marks[&0].x < placed.marks[&2].x);
        // Same module, different bands: same column, different rows.
        assert_eq!(placed.marks[&0].x, placed.marks[&1].x);
        assert!(placed.marks[&0].y < placed.marks[&1].y);
        // Every prism spans every band, and every band is captioned.
        assert_eq!(placed.prisms.len(), 2);
        for prism in &placed.prisms {
            for mark in [&placed.marks[&0], &placed.marks[&1]] {
                assert!(prism.at.y <= mark.y && prism.at.y + prism.at.h >= mark.y + mark.h);
            }
        }
        let captions: Vec<&str> = placed.lanes.iter().map(|l| l.caption.as_str()).collect();
        assert_eq!(captions, vec!["entry", "1 call deep"]);
        assert_eq!(
            placed.lanes.iter().map(|l| l.band).collect::<Vec<_>>(),
            vec![0, 1],
            "every band is a lane, and every lane selects its band"
        );
    }
}
