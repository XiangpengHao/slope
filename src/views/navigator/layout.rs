//! Where the navigator's blocks sit on the page.
//!
//! A pure function of (the reading, the measured text): four columns, bands
//! down each one, plates down each band. Position says relation and nothing
//! else — the left column is what the focus depends on, the right is what
//! depends on it, the far right is how far a change could travel. There is no
//! camera and no map, so there is nothing to fit: the page is as tall as the
//! answer is long.
//!
//! Everything is measured before it is placed, the way the other charts do it,
//! and for the same reason: the wires are drawn from these numbers, so the
//! harness and the blocks have to agree without asking the browser where it put
//! anything.

use crate::api::{CodeGraph, HoldEvent};
use crate::views::codemap::tree::{Placed, text_w, tracked_w};
use crate::views::navigator::model::{
    Family, Grouped, NavItem, NavModel, QuoteRow, Row, RowState, band_count,
};

/// Between two columns. Wide enough for a trunk to leave the focus, turn, and
/// run down a rail without touching either column's text.
const COL_GAP: f64 = 46.0;
/// The left and right columns on a focus page.
const SIDE_W: f64 = 300.0;
/// The far column: one line per mark, so it needs no more room than a name.
const FAR_W: f64 = 230.0;
/// The quoted focus block, which is the one block on the page that is read
/// rather than scanned.
const FOCUS_MIN_W: f64 = 360.0;
const FOCUS_MAX_W: f64 = 620.0;
/// The diff agenda's columns: no far column, so the three share the paper.
const AGENDA_SIDE_W: f64 = 360.0;
const AGENDA_CENTER_W: f64 = 480.0;

/// A band's header: the engraved line and the rule under it.
const BAND_HEAD_H: f64 = 25.0;
/// Between one band and the next.
const BAND_GAP: f64 = 18.0;
/// A module's small-caps label inside a band, and the same label when it opens
/// the band and needs no air above it.
const GROUP_H: f64 = 21.0;
const GROUP_FIRST_H: f64 = 14.0;
/// The label's own line, and the air under it.
const GROUP_LABEL_H: f64 = 11.0;
const GROUP_BELOW: f64 = 3.0;
/// Between two plates in a band.
const PLATE_GAP: f64 = 6.0;
/// One line of the far column.
const FAR_ROW_H: f64 = 16.0;
/// One line of an empty band's sentence.
const TRUTH_LINE_H: f64 = 19.0;
/// EB Garamond's advance at the truths' size, near enough to wrap by.
const SERIF_ADVANCE: f64 = 0.46;

/// A compact plate: the header line, the module label, and the delta note.
const PLATE_PAD_X: f64 = 16.0;
const PLATE_MIN_W: f64 = 120.0;
const PLATE_H: f64 = 33.0;
const PLATE_NOTE_H: f64 = 13.0;
/// Between the parts of a header line: the keyword, the name, the letter.
const HEAD_GAP: f64 = 5.0;
/// A header would rather carry slack than lose its last letter to an ellipsis.
const HEAD_SLACK: f64 = 3.0;

/// The focus block's own furniture.
const FOCUS_PAD_X: f64 = 20.0;
const FOCUS_HEAD_H: f64 = 25.0;
const FOCUS_ROW_H: f64 = 15.0;
const FOCUS_BAND_H: f64 = 6.0;
const FOCUS_LOC_H: f64 = 15.0;
const FOCUS_ACTS_H: f64 = 26.0;
const FOCUS_PAD_Y: f64 = 11.0;

/// How far a wire's rail stands off the plates it serves.
const RAIL_OFF: f64 = 16.0;
/// The agenda's stub beside a plate.
const STUB: f64 = 34.0;

/// One placed thing inside a band.
#[derive(Clone, PartialEq, Debug)]
pub enum Entry {
    /// A compact plate: one mark, refocusable.
    Plate(PlateBox),
    /// One line: a mark further out than the page draws a plate for.
    Row(RowBox),
    /// A module label, or the hop a layer stands at.
    Group { label: String, at: Placed },
    /// What an empty band says instead of a list.
    Truth { text: String, at: Placed },
}

#[derive(Clone, PartialEq, Debug)]
pub struct PlateBox {
    pub id: u32,
    /// What the diff did to its rows, on the agenda's own plates.
    pub note: Option<String>,
    pub at: Placed,
}

#[derive(Clone, PartialEq, Debug)]
pub struct RowBox {
    pub id: u32,
    pub at: Placed,
}

/// One band of one column: a question's sub-answer, headed in words.
#[derive(Clone, PartialEq, Debug)]
pub struct BandBox {
    pub title: String,
    pub count: Option<String>,
    pub head: Placed,
    pub entries: Vec<Entry>,
}

/// The focused mark, quoted whole.
#[derive(Clone, PartialEq, Debug)]
pub struct FocusBox {
    pub id: u32,
    pub rows: Vec<QuoteRow>,
    pub locator: String,
    pub at: Placed,
}

/// Where a wire's word sits, and which way it runs from there.
#[derive(Clone, PartialEq, Debug)]
pub struct Label {
    pub x: f64,
    pub y: f64,
    pub text: String,
    /// The text starts at `x` and runs right; otherwise it ends there.
    pub start: bool,
}

/// One drawn part of the harness: a trunk, a rail, or one plate's branch.
#[derive(Clone, PartialEq, Debug)]
pub struct Wire {
    /// The plate this part belongs to, for the hover. A trunk belongs to none.
    pub id: Option<u32>,
    pub family: Family,
    pub event: Option<HoldEvent>,
    pub line: String,
    pub head: Option<String>,
    pub label: Option<Label>,
}

/// A whole page, placed. The columns are already absolute, so drawing it is a
/// walk and nothing more.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Page {
    pub bands: Vec<BandBox>,
    pub focus: Option<FocusBox>,
    pub wires: Vec<Wire>,
    pub size: (f64, f64),
}

/// One plate the harness has to reach, as the column left it.
#[derive(Clone, PartialEq, Debug)]
struct Plan {
    band: usize,
    side: i32,
    at: Placed,
    row: Row,
}

/// A header line, measured as the flex row it is: the keyword, the name, the
/// diff's letter, and the gap between each of them. Measuring it as one string
/// loses the gaps, and a name that loses its last letter to an ellipsis is the
/// one thing a plate must not do.
fn head_w(item: &NavItem, px: f64) -> f64 {
    let mut w = text_w(&item.keyword(), px) + HEAD_GAP + text_w(&item.name, px);
    if item.letter().is_some() {
        w += HEAD_GAP + text_w("A", px);
    }
    w + HEAD_SLACK
}

/// How wide and tall one compact plate is: the widest line it must not clip,
/// and every line it draws.
fn plate_size(item: &NavItem, note: Option<&str>, column: f64, full: bool) -> (f64, f64) {
    let mut widest = head_w(item, 10.5).max(text_w(&item.module, 8.0));
    if let Some(note) = note {
        widest = widest.max(text_w(note, 9.0));
    }
    let w = if full {
        column
    } else {
        (widest + PLATE_PAD_X).clamp(PLATE_MIN_W, column)
    };
    let h = PLATE_H + if note.is_some() { PLATE_NOTE_H } else { 0.0 };
    (w, h)
}

/// How many lines an empty band's sentence takes at this width.
fn truth_h(text: &str, column: f64) -> f64 {
    let w = tracked_w(text, 12.0, SERIF_ADVANCE, 0.0);
    let lines = (w / column.max(1.0)).ceil().max(1.0);
    lines * TRUTH_LINE_H
}

/// The focus block, measured. Its rows are quotations and they never wrap: a
/// row too long for the block ellipses, the way every quoted row on every chart
/// in this app does.
fn focus_size(item: &NavItem, rows: &[QuoteRow], locator: &str) -> (f64, f64) {
    let mut widest = head_w(item, 12.0).max(text_w(locator, 8.5));
    for row in rows {
        let text: String = row.runs.iter().map(|run| run.text.as_str()).collect();
        let marker = if row.state == RowState::Same {
            0.0
        } else {
            9.0
        };
        widest = widest.max(text_w(&format!("{}{text}", row.name), 10.0) + marker);
    }
    let w = (widest + FOCUS_PAD_X).clamp(FOCUS_MIN_W, FOCUS_MAX_W);
    let band = if rows.iter().any(|row| row.band) {
        FOCUS_BAND_H
    } else {
        0.0
    };
    let h = FOCUS_PAD_Y
        + FOCUS_HEAD_H
        + rows.len() as f64 * FOCUS_ROW_H
        + band
        + FOCUS_LOC_H
        + FOCUS_ACTS_H;
    (w, h)
}

/// One column, filled band by band. The y it carries is the page's, so a
/// column never has to be shifted after the fact.
struct Col {
    x: f64,
    w: f64,
    y: f64,
    /// Which way this column's wires run: -1 is what the focus depends on, 1
    /// what depends on it, 0 no wires at all.
    side: i32,
    bands: Vec<BandBox>,
    plan: Vec<Plan>,
}

impl Col {
    fn new(x: f64, w: f64, side: i32) -> Self {
        Col {
            x,
            w,
            y: 0.0,
            side,
            bands: Vec::new(),
            plan: Vec::new(),
        }
    }

    /// Open a band: its header takes the head of the column, and the entries
    /// stack under it.
    fn head(&mut self) -> Placed {
        let at = Placed {
            x: self.x,
            y: self.y,
            w: self.w,
            h: BAND_HEAD_H,
        };
        self.y += BAND_HEAD_H;
        at
    }

    /// A group label. It keeps air above it unless it opens the band, where the
    /// band's own rule is the air.
    fn group(&mut self, entries: &mut Vec<Entry>, label: &str, first: bool) {
        let slot = if first { GROUP_FIRST_H } else { GROUP_H };
        let above = slot - GROUP_LABEL_H - GROUP_BELOW;
        self.y += above;
        entries.push(Entry::Group {
            label: label.to_string(),
            at: Placed {
                x: self.x,
                y: self.y,
                w: self.w,
                h: GROUP_LABEL_H,
            },
        });
        self.y += GROUP_LABEL_H + GROUP_BELOW;
    }

    fn plate(
        &mut self,
        entries: &mut Vec<Entry>,
        model: &NavModel,
        row: &Row,
        note: Option<String>,
        full: bool,
        wired: bool,
    ) {
        let Some(item) = model.item(row.id) else {
            return;
        };
        let (w, h) = plate_size(item, note.as_deref(), self.w, full);
        let at = Placed {
            x: self.x,
            y: self.y,
            w,
            h,
        };
        entries.push(Entry::Plate(PlateBox {
            id: row.id,
            note,
            at,
        }));
        if wired && self.side != 0 {
            self.plan.push(Plan {
                band: self.bands.len(),
                side: self.side,
                at,
                row: row.clone(),
            });
        }
        self.y += h + PLATE_GAP;
    }

    fn row(&mut self, entries: &mut Vec<Entry>, id: u32) {
        let at = Placed {
            x: self.x,
            y: self.y,
            w: self.w,
            h: FAR_ROW_H,
        };
        entries.push(Entry::Row(RowBox { id, at }));
        self.y += FAR_ROW_H;
    }

    fn truth(&mut self, entries: &mut Vec<Entry>, text: &str) {
        let h = truth_h(text, self.w);
        entries.push(Entry::Truth {
            text: text.to_string(),
            at: Placed {
                x: self.x,
                y: self.y,
                w: self.w,
                h,
            },
        });
        self.y += h;
    }

    fn close(&mut self, title: &str, head: Placed, count: Option<String>, entries: Vec<Entry>) {
        self.y += BAND_GAP;
        self.bands.push(BandBox {
            title: title.to_string(),
            count,
            head,
            entries,
        });
    }

    /// A band of plates, grouped by module — or the one sentence that stands
    /// in for an empty band.
    fn plates(
        &mut self,
        model: &NavModel,
        title: &str,
        groups: &[Grouped],
        empty: &str,
        wired: bool,
    ) {
        let count = band_count(groups);
        let head = self.head();
        let mut entries = Vec::new();
        if count == 0 {
            self.truth(&mut entries, empty);
        } else {
            for (at, group) in groups.iter().enumerate() {
                if let Some(label) = &group.label {
                    self.group(&mut entries, label, at == 0);
                }
                for row in &group.rows {
                    self.plate(&mut entries, model, row, None, false, wired);
                }
            }
        }
        self.close(title, head, (count > 0).then(|| count.to_string()), entries);
    }

    /// The reach, layer by layer: one group label per hop, one line per mark.
    fn layers(&mut self, title: &str, layers: &[Vec<u32>], hop_from: usize) {
        let count: usize = layers.iter().map(Vec::len).sum();
        let head = self.head();
        let mut entries = Vec::new();
        for (at, layer) in layers.iter().enumerate() {
            self.group(&mut entries, &format!("{} hops", at + hop_from), at == 0);
            for &id in layer {
                self.row(&mut entries, id);
            }
        }
        self.close(title, head, (count > 0).then(|| count.to_string()), entries);
    }
}

/// The wires between the hop-1 plates and the focus block: one trunk per band
/// from its own port on the block's edge, a rail beside the band's plates, and
/// one short labelled branch to each plate. A band is family-pure, so the whole
/// harness of a band wears one ink.
fn harness(focus: Placed, plan: &[Plan]) -> Vec<Wire> {
    let mut wires: Vec<Wire> = Vec::new();
    let mut order: Vec<(i32, usize)> = Vec::new();
    for item in plan {
        if !order.contains(&(item.side, item.band)) {
            order.push((item.side, item.band));
        }
    }
    for side in [-1i32, 1] {
        let bands: Vec<(i32, usize)> = order.iter().copied().filter(|(s, _)| *s == side).collect();
        let total = bands.len() as f64;
        for (at, key) in bands.iter().enumerate() {
            let ws: Vec<&Plan> = plan.iter().filter(|p| (p.side, p.band) == *key).collect();
            let Some(first) = ws.first() else { continue };
            let family = first.row.family;
            let port_x = if side < 0 { focus.x } else { focus.x + focus.w };
            let port_y = focus.y + (at as f64 + 1.0) / (total + 1.0) * focus.h;
            let rail_x = if side < 0 {
                ws.iter().map(|p| p.at.x + p.at.w).fold(f64::MIN, f64::max) + RAIL_OFF
            } else {
                ws.iter().map(|p| p.at.x).fold(f64::MAX, f64::min) - RAIL_OFF
            };
            let ys: Vec<f64> = ws.iter().map(|p| p.at.y + p.at.h / 2.0).collect();
            let low = ys.iter().copied().fold(f64::MAX, f64::min);
            let high = ys.iter().copied().fold(f64::MIN, f64::max);
            let turn = port_y.clamp(low, high);
            let mid = (port_x + rail_x) / 2.0;
            wires.push(Wire {
                id: None,
                family,
                event: None,
                line: format!(
                    "M {port_x:.1} {port_y:.1} C {mid:.1} {port_y:.1}, {rail_x:.1} {port_y:.1}, {rail_x:.1} {turn:.1}"
                ),
                // The arrowhead rests on the dependent: leftward, that is the
                // focus's own port, because the focus is what would have to
                // change.
                head: (side < 0).then(|| format!("M {port_x:.1} {port_y:.1} l 6 -3 v 6 z")),
                label: None,
            });
            if ys.len() > 1 {
                wires.push(Wire {
                    id: None,
                    family,
                    event: None,
                    line: format!("M {rail_x:.1} {low:.1} L {rail_x:.1} {high:.1}"),
                    head: None,
                    label: None,
                });
            }
            for (item, &y) in ws.iter().zip(&ys) {
                let px = if side < 0 {
                    item.at.x + item.at.w
                } else {
                    item.at.x
                };
                wires.push(Wire {
                    id: Some(item.row.id),
                    family: item.row.family,
                    event: item.row.event,
                    line: format!("M {rail_x:.1} {y:.1} L {px:.1} {y:.1}"),
                    head: (side > 0).then(|| format!("M {px:.1} {y:.1} l 6 -3 v 6 z")),
                    label: (!item.row.word.is_empty()).then(|| Label {
                        x: px + if side < 0 { 5.0 } else { -5.0 },
                        y: y - 4.0,
                        text: item.row.word.clone(),
                        start: side < 0,
                    }),
                });
            }
        }
    }
    wires
}

/// The agenda's wires: no focus block to run to, so each plate gets a stub
/// that says the family and the word, and nothing pretends to be a harness.
fn stubs(plan: &[Plan]) -> Vec<Wire> {
    plan.iter()
        .map(|item| {
            let side = item.side as f64;
            let from_x = if item.side < 0 {
                item.at.x + item.at.w
            } else {
                item.at.x
            };
            let to_x = from_x - side * STUB;
            let y = item.at.y + item.at.h / 2.0;
            Wire {
                id: Some(item.row.id),
                family: item.row.family,
                event: item.row.event,
                line: format!("M {to_x:.1} {y:.1} L {from_x:.1} {y:.1}"),
                head: Some(format!(
                    "M {from_x:.1} {y:.1} l {:.1} -3 v 6 z",
                    -6.0 * side
                )),
                label: (!item.row.word.is_empty()).then(|| Label {
                    x: to_x - side * -4.0,
                    y: y - 4.0,
                    text: item.row.word.clone(),
                    start: item.side > 0,
                }),
            }
        })
        .collect()
}

/// Gather the placed columns into one page.
fn page(cols: Vec<Col>, focus: Option<FocusBox>, wires: Vec<Wire>, width: f64) -> Page {
    let height = cols
        .iter()
        .map(|c| c.y)
        .chain(focus.iter().map(|f| f.at.y + f.at.h))
        .fold(0.0, f64::max);
    Page {
        bands: cols.into_iter().flat_map(|c| c.bands).collect(),
        focus,
        wires,
        size: (width, height),
    }
}

/// The focus page: one mark, quoted, with everything that stands in relation to
/// it seated by the relation it stands in.
pub fn focus_page(model: &NavModel, graph: &CodeGraph, id: u32) -> Option<Page> {
    let item = model.item(id)?;
    let read = model.focus(id);
    let rows = model.quote(graph, id);
    let locator = format!(
        "{}:{}{}",
        item.path,
        item.line,
        if item.ghost { " (base)" } else { "" }
    );
    let (center_w, center_h) = focus_size(item, &rows, &locator);

    let center_x = SIDE_W + COL_GAP;
    let right_x = center_x + center_w + COL_GAP;
    let far_x = right_x + SIDE_W + COL_GAP;
    let has_far = !read.reach.is_empty();

    let mut left = Col::new(0.0, SIDE_W, -1);
    left.plates(
        model,
        read.surface_title,
        &read.surface,
        "its surface names nothing in the workspace.",
        true,
    );
    left.plates(
        model,
        "only its body reaches",
        &read.body_out,
        "its body leans on nothing it does not already name.",
        true,
    );

    let mut right = Col::new(right_x, SIDE_W, 1);
    right.plates(model, "held by", &read.held, read.held_truth, true);
    // The band is drawn only when it has something to say: an empty one here
    // would repeat the sentence "held by" already said.
    if band_count(&read.named) > 0 {
        right.plates(model, "named in the signatures of", &read.named, "", true);
    }
    right.plates(
        model,
        "read by the bodies of",
        &read.body_in,
        "no body reads it.",
        true,
    );

    let mut far = Col::new(far_x, FAR_W, 0);
    if has_far {
        far.layers("the reach beyond", &read.reach, 2);
    }

    let focus = FocusBox {
        id,
        rows,
        locator,
        at: Placed {
            x: center_x,
            y: 0.0,
            w: center_w,
            h: center_h,
        },
    };
    let mut plan = left.plan.clone();
    plan.extend(right.plan.clone());
    let wires = harness(focus.at, &plan);
    let width = if has_far {
        far_x + FAR_W
    } else {
        right_x + SIDE_W
    };
    Some(page(vec![left, right, far], Some(focus), wires, width))
}

/// The opening page: what the diff touched, what coupling it took on and gave
/// back, and everything it reaches that itself did not change.
pub fn agenda_page(model: &NavModel) -> Page {
    let read = model.agenda();
    let center_x = AGENDA_SIDE_W + COL_GAP;
    let right_x = center_x + AGENDA_CENTER_W + COL_GAP;

    let mut left = Col::new(0.0, AGENDA_SIDE_W, -1);
    left.plates(
        model,
        "now leans on",
        &read.new_leans,
        "the change takes on no new interface coupling.",
        true,
    );
    left.plates(
        model,
        "no longer leans on",
        &read.cut_leans,
        "no coupling was given back.",
        true,
    );

    // The centre: the agenda itself, every touched contract in its own words.
    let mut center = Col::new(center_x, AGENDA_CENTER_W, 0);
    let count = band_count(&read.changed);
    let head = center.head();
    let mut entries = Vec::new();
    if count == 0 {
        center.truth(
            &mut entries,
            "the working copy is the base — nothing to review here. \
             search for a contract, or press / to start anywhere.",
        );
    } else {
        for (at, group) in read.changed.iter().enumerate() {
            if let Some(label) = &group.label {
                center.group(&mut entries, label, at == 0);
            }
            for row in &group.rows {
                let note = model.item(row.id).and_then(NavItem::note);
                center.plate(&mut entries, model, row, note, true, false);
            }
        }
    }
    center.close("the change", head, Some(model.tally.clone()), entries);

    // The right: the blast radius of the whole change, nearest layer first,
    // untouched marks only — the reviewer's list of what to look at next.
    let mut right = Col::new(right_x, AGENDA_SIDE_W, 1);
    let reached: usize = read.reaches.iter().map(Vec::len).sum();
    let head = right.head();
    let mut entries = Vec::new();
    if read.reaches.is_empty() {
        right.truth(&mut entries, "nothing upstream depends on what changed.");
    } else {
        for (at, layer) in read.reaches.iter().enumerate() {
            let label = if at == 0 {
                "directly".to_string()
            } else {
                format!("{} hops out", at + 1)
            };
            right.group(&mut entries, &label, at == 0);
            for &id in layer {
                // The nearest layer is plates — those are the marks a reviewer
                // opens next. Further out is one line each: a name and a way in.
                if at == 0 {
                    right.plate(&mut entries, model, &Row::plain(id), None, false, false);
                } else {
                    right.row(&mut entries, id);
                }
            }
        }
    }
    right.close(
        "the change reaches",
        head,
        (reached > 0).then(|| reached.to_string()),
        entries,
    );

    let wires = stubs(&left.plan);
    let width = right_x + AGENDA_SIDE_W;
    page(vec![left, center, right], None, wires, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Delta, ItemKind, Vis};

    fn item(id: u32, name: &str) -> NavItem {
        NavItem {
            id,
            name: name.to_string(),
            label: name.to_string(),
            kind: ItemKind::Struct,
            vis: Vis::Pub,
            path: "src/api.rs".to_string(),
            line: 1,
            module: "api".to_string(),
            delta: Delta::Same,
            ghost: false,
            fields_added: 0,
            fields_removed: 0,
            variants_added: 0,
            variants_removed: 0,
            methods_added: 0,
            methods_removed: 0,
        }
    }

    /// A plate never grows past its column, and never shrinks below a name.
    #[test]
    fn a_plate_is_measured_but_stays_inside_its_column() {
        let long = item(0, "AnExtremelyLongContractNameThatWillNotFit");
        let (w, h) = plate_size(&long, None, SIDE_W, false);
        assert_eq!(w, SIDE_W);
        assert_eq!(h, PLATE_H);
        let (w, h) = plate_size(&item(1, "Wire"), Some("+2 fields"), SIDE_W, false);
        assert_eq!(w, PLATE_MIN_W);
        assert_eq!(h, PLATE_H + PLATE_NOTE_H);
    }

    /// One band, one trunk, one arrowhead on the dependent — and a branch and a
    /// word for every plate.
    #[test]
    fn the_harness_gives_each_band_one_trunk_and_each_plate_one_branch() {
        let focus = Placed {
            x: 346.0,
            y: 0.0,
            w: 400.0,
            h: 200.0,
        };
        let seat = |x: f64, y: f64, w: f64| Placed { x, y, w, h: 33.0 };
        let row = |id: u32, word: &str, family: Family| Row {
            id,
            word: word.to_string(),
            family,
            event: None,
        };
        let plan = vec![
            Plan {
                band: 0,
                side: -1,
                at: seat(0.0, 25.0, 200.0),
                row: row(1, "Arc", Family::Solid),
            },
            Plan {
                band: 0,
                side: -1,
                at: seat(0.0, 64.0, 160.0),
                row: row(2, "owns", Family::Solid),
            },
            Plan {
                band: 0,
                side: 1,
                at: seat(792.0, 25.0, 180.0),
                row: row(3, "3 refs", Family::Uses),
            },
        ];
        let wires = harness(focus, &plan);
        // Two bands: a trunk each, one rail (the left band has two plates),
        // and one branch per plate.
        assert_eq!(wires.len(), 2 + 1 + 3);
        let trunks: Vec<&Wire> = wires.iter().filter(|w| w.id.is_none()).collect();
        assert_eq!(trunks.len(), 3);
        // Leftward the head rests on the focus; rightward on each plate.
        let left_trunk = &wires[0];
        assert!(left_trunk.head.is_some(), "the left trunk carries the head");
        let branch = wires.iter().find(|w| w.id == Some(3)).unwrap();
        assert!(branch.head.is_some(), "a rightward branch carries its own");
        let word = branch.label.as_ref().unwrap();
        assert_eq!(word.text, "3 refs");
        assert!(!word.start, "a rightward word ends at the plate");
        // The rail stands clear of the widest plate it serves.
        assert!(wires[0].line.contains("216.0"));
    }

    /// Bands stack down a column, and a band with nothing in it says one
    /// sentence instead of nothing at all.
    #[test]
    fn an_empty_band_keeps_its_header_and_says_why() {
        let mut col = Col::new(0.0, SIDE_W, -1);
        col.plates(
            &NavModel::default(),
            "held by",
            &[],
            "nothing reaches it at all.",
            true,
        );
        assert_eq!(col.bands.len(), 1);
        assert_eq!(col.bands[0].count, None);
        let Entry::Truth { text, at } = &col.bands[0].entries[0] else {
            panic!("an empty band draws its truth");
        };
        assert_eq!(text, "nothing reaches it at all.");
        assert_eq!(at.y, BAND_HEAD_H);
        assert!(col.y > at.y + at.h);
    }
}
