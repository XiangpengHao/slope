//! The code chart: files as marks on the directory structure, references as
//! chords drawn only for the selection, and the cutaway — the selected file
//! unfolded in place to show its items while every neighbor keeps its
//! ground.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size,
};

use crate::api::{CodeGraph, FileDetail, FileInfo, ItemKind};
use crate::views::codemap::tree::{
    self, FileTree, PlacedMark, ROOT, dir_key, file_key,
};
use crate::views::codemap::{CodeSel, RefDir, file_route, item_route, use_code};
use crate::views::star::star_radius;

/// The cutaway plate's width, and its item rows' height.
const CUT_W: f64 = 268.0;
const CUT_ROW_H: f64 = 16.0;
const CUT_HEADER_H: f64 = 46.0;
/// Item rows shown before the plate defers to the panel — unless the
/// selected item hides past the cap, which unfolds the whole file.
const CUT_CAP: usize = 24;

/// Room a mark needs beyond its core circle (focal ticks reach farthest).
const MARK_OVERHANG: f64 = 11.0;

/// One node on the code chart.
#[derive(Clone, PartialEq)]
pub enum CodeNodeData {
    File {
        info: FileInfo,
        /// File name without its directory.
        name: String,
        focal: bool,
    },
    Dir {
        id: u32,
        name: String,
        /// Files in the subtree, for the gate's state words.
        files: u32,
        open: bool,
        /// The crate whose district this directory is, when it is one.
        krate: Option<String>,
        focal: bool,
        root: bool,
        /// An open directory hands its name to its street; a gate keeps its
        /// own label, so it can carry its count.
        labeled: bool,
        /// Root label (workspace name) when this is the root mark.
        workspace: String,
    },
    Cutaway {
        info: FileInfo,
        name: String,
        detail: FileDetail,
        sel_item: Option<u32>,
    },
}

/// One reference chord, fully placed. Chords always run from the definition
/// to its user — the way change travels — with the arrowhead at the user.
#[derive(Clone, PartialEq)]
pub struct Chord {
    pub from: Point,
    pub to: Point,
    /// Trim radii so chords stop at mark rims.
    pub from_rim: f64,
    pub to_rim: f64,
    /// "dep" (ink — the selection uses this) or "user" (hairline — this uses
    /// the selection).
    pub role: &'static str,
    pub label: Option<String>,
    /// Where along the curve the count sits (0 = from, 1 = to). Kept away
    /// from the cutaway plate so counts never land on its text.
    pub label_t: f64,
    pub key: String,
}

/// The cutaway's geometry: the plate rect (top-left at the file's seat) and
/// each visible item row's vertical center.
pub struct CutawayGeom {
    pub rows: Vec<(u32, f64)>,
    pub width: f64,
    pub height: f64,
    /// Rows hidden past the cap.
    pub hidden: usize,
}

/// Which item rows the cutaway shows, and where. Impl headers count as rows.
pub fn cutaway_geom(detail: &FileDetail, sel_item: Option<u32>) -> CutawayGeom {
    let total = detail.items.len();
    let sel_idx = sel_item
        .and_then(|id| detail.items.iter().position(|i| i.id == id))
        .unwrap_or(0);
    let shown = if total > CUT_CAP && sel_idx >= CUT_CAP {
        total
    } else {
        total.min(CUT_CAP)
    };
    let rows = detail
        .items
        .iter()
        .take(shown)
        .enumerate()
        .map(|(i, item)| (item.id, CUT_HEADER_H + (i as f64 + 0.5) * CUT_ROW_H))
        .collect();
    let hidden = total - shown;
    CutawayGeom {
        rows,
        width: CUT_W,
        height: CUT_HEADER_H + shown as f64 * CUT_ROW_H + if hidden > 0 { 18.0 } else { 8.0 },
        hidden,
    }
}

/// `"Trail::note"` — the label an item selects by in the URL, matching the
/// server's cross-reference names.
pub fn item_sel_label(item: &crate::api::ItemInfo) -> String {
    if item.section.is_empty() {
        item.name.clone()
    } else {
        let s = item
            .section
            .rsplit_once(" for ")
            .map(|(_, ty)| ty)
            .unwrap_or(&item.section);
        let ty = s
            .strip_prefix("impl ")
            .or_else(|| s.strip_prefix("trait "))
            .unwrap_or(s);
        format!("{ty}::{}", item.name)
    }
}

/// Everything one build of the chart draws.
#[derive(Clone, PartialEq)]
struct Built {
    nodes: Vec<FlowNode<CodeNodeData>>,
    chords: Vec<Chord>,
    /// Flow-space rect to frame, and whether it is a focused neighborhood.
    frame: Option<Rect>,
    /// What must stay on screen when the legibility floor stops the camera
    /// from fitting everything: the cutaway, or the selected mark.
    focal: Option<Point>,
    /// The cutaway's top-center. On shallow viewports the plate pins its
    /// header row to the top of the free band — a plate whose name is
    /// hidden identifies nothing.
    focal_top: Option<Point>,
    focused: bool,
    /// A file is selected but its cutaway has not arrived yet: hold the
    /// camera — framing now and again on arrival lands mid-animation and
    /// the second move is lost.
    pending_cutaway: bool,
}

fn file_box(info: &FileInfo) -> f64 {
    2.0 * (star_radius(info.refs_in_files) + MARK_OVERHANG)
}

/// Bow a chord toward open paper: perpendicular offset by length, capped.
fn chord_ctrl(a: Point, b: Point) -> Point {
    let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-3 {
        return mid;
    }
    let bow = (len * 0.12).min(56.0);
    Point::new(mid.x - dy / len * bow, mid.y + dx / len * bow)
}

#[allow(clippy::too_many_arguments)]
fn build_chart(
    graph: &CodeGraph,
    tree: &FileTree,
    open: &HashSet<u32>,
    pos: &HashMap<String, PlacedMark>,
    sel: &CodeSel,
    ref_dir: RefDir,
    details: &HashMap<u32, FileDetail>,
    workspace: &str,
) -> Built {
    let by_path: HashMap<&str, &FileInfo> =
        graph.files.iter().map(|f| (f.path.as_str(), f)).collect();
    let file_name = |f: &FileInfo| -> String {
        f.path
            .rsplit_once('/')
            .map(|(_, n)| n.to_string())
            .unwrap_or_else(|| f.path.clone())
    };

    // The selection, resolved.
    let sel_file: Option<&FileInfo> = match sel {
        CodeSel::File(path, _) => by_path.get(path.as_str()).copied(),
        _ => None,
    };
    let sel_detail = sel_file.and_then(|f| details.get(&f.id));
    let sel_item: Option<u32> = match (sel, sel_detail) {
        (CodeSel::File(_, label), Some(detail)) if !label.is_empty() => detail
            .items
            .iter()
            .find(|i| &item_sel_label(i) == label)
            .map(|i| i.id),
        _ => None,
    };
    let sel_crate_dir: Option<u32> = match sel {
        CodeSel::Crate(name) => tree
            .dirs
            .iter()
            .find(|d| d.krate.as_deref() == Some(name.as_str()))
            .map(|d| d.id),
        _ => None,
    };
    let crate_files: HashSet<u32> = match sel {
        CodeSel::Crate(name) => graph
            .files
            .iter()
            .filter(|f| &f.krate == name)
            .map(|f| f.id)
            .collect(),
        _ => HashSet::new(),
    };

    // ---- Chords: only ever the selection's. -------------------------------
    let seat = |file: u32| -> Option<(Point, f64)> {
        let info = &graph.files[file as usize];
        let p = pos.get(&file_key(file))?;
        Some((p.point, star_radius(info.refs_in_files)))
    };
    // A file inside a closed directory still has references; its chord ends
    // at the gate standing in for it.
    let gate_seat = |file: u32| -> Option<(Point, f64)> {
        let mut dir = *tree.dir_of_file.get(&file)?;
        loop {
            if let Some(p) = pos.get(&dir_key(dir)) {
                return Some((p.point, 9.0));
            }
            dir = tree.dirs[dir as usize].parent?;
        }
    };
    let seat_or_gate = |file: u32| seat(file).or_else(|| gate_seat(file));

    let mut chords: Vec<Chord> = Vec::new();
    let mut hood: Vec<Point> = Vec::new();

    let cut_geom = sel_detail.map(|d| cutaway_geom(d, sel_item));
    let cut_anchor: Option<Point> = sel_file
        .and_then(|f| pos.get(&file_key(f.id)))
        .map(|p| p.point);

    match (sel_file, sel_item, sel_detail, &cut_geom) {
        // One item selected: its references, at item precision.
        (Some(file), Some(item), Some(detail), Some(geom)) => {
            let row_y: HashMap<u32, f64> = geom.rows.iter().copied().collect();
            let anchor = cut_anchor.unwrap_or(Point::ZERO);
            let mut rows: Vec<(&crate::api::ItemXRef, bool)> = Vec::new();
            match ref_dir {
                RefDir::Uses => {
                    for r in detail.refs_out.iter().filter(|r| r.item == item) {
                        rows.push((r, true));
                    }
                }
                RefDir::UsedBy => {
                    for r in detail.refs_in.iter().filter(|r| r.item == item) {
                        rows.push((r, false));
                    }
                }
            }
            for (r, uses) in rows {
                let Some((far, far_rim)) = seat_or_gate(r.file) else {
                    continue;
                };
                let Some(&y) = row_y.get(&item) else { continue };
                // Anchor on the plate edge facing the far end.
                let left = far.x < anchor.x + geom.width / 2.0;
                let at = Point::new(
                    anchor.x + if left { 0.0 } else { geom.width },
                    anchor.y + y,
                );
                hood.push(far);
                let label = (r.count > 1).then(|| format!("×{}", r.count));
                if uses {
                    // The item uses the far end: definition → user(here).
                    chords.push(Chord {
                        from: far,
                        to: at,
                        from_rim: far_rim + 2.0,
                        to_rim: 2.0,
                        role: "dep",
                        label,
                        label_t: 0.35,
                        key: format!("i{}-{}u", r.file, r.item),
                    });
                } else {
                    chords.push(Chord {
                        from: at,
                        to: far,
                        from_rim: 2.0,
                        to_rim: far_rim + 2.0,
                        role: "user",
                        label,
                        label_t: 0.65,
                        key: format!("i{}-{}d", r.file, r.item),
                    });
                }
            }
            let _ = file;
        }
        // A file selected: its file-level references. With the cutaway open,
        // chords meet the plate at the edge facing the far end — never the
        // corner pile.
        (Some(file), None, _, _) => {
            let anchor = cut_anchor.unwrap_or(Point::ZERO);
            for r in &graph.refs {
                let (far_id, uses) = match ref_dir {
                    RefDir::Uses if r.from == file.id => (r.to, true),
                    RefDir::UsedBy if r.to == file.id => (r.from, false),
                    _ => continue,
                };
                let Some((far, far_rim)) = seat_or_gate(far_id) else {
                    continue;
                };
                hood.push(far);
                let (at, own_rim, label_t_uses) = match &cut_geom {
                    Some(geom) => {
                        let left = far.x < anchor.x + geom.width / 2.0;
                        (
                            Point::new(
                                anchor.x + if left { 0.0 } else { geom.width },
                                anchor.y + CUT_HEADER_H * 0.55,
                            ),
                            2.0,
                            0.35,
                        )
                    }
                    None => (anchor, star_radius(file.refs_in_files) + 2.0, 0.5),
                };
                let label = (r.count > 1).then(|| format!("×{}", r.count));
                if uses {
                    chords.push(Chord {
                        from: far,
                        to: at,
                        from_rim: far_rim + 2.0,
                        to_rim: own_rim,
                        role: "dep",
                        label,
                        label_t: label_t_uses,
                        key: format!("f{far_id}u"),
                    });
                } else {
                    chords.push(Chord {
                        from: at,
                        to: far,
                        from_rim: own_rim,
                        to_rim: far_rim + 2.0,
                        role: "user",
                        label,
                        label_t: 1.0 - label_t_uses,
                        key: format!("f{far_id}d"),
                    });
                }
            }
        }
        // A crate selected: every reference crossing its boundary.
        _ if sel_crate_dir.is_some() => {
            for r in &graph.refs {
                let (from_in, to_in) = (
                    crate_files.contains(&r.from),
                    crate_files.contains(&r.to),
                );
                let wanted = match ref_dir {
                    RefDir::Uses => from_in && !to_in,
                    RefDir::UsedBy => to_in && !from_in,
                };
                if !wanted {
                    continue;
                }
                let (Some((def, def_rim)), Some((user, user_rim))) =
                    (seat_or_gate(r.to), seat_or_gate(r.from))
                else {
                    continue;
                };
                hood.push(def);
                hood.push(user);
                chords.push(Chord {
                    from: def,
                    to: user,
                    from_rim: def_rim + 2.0,
                    to_rim: user_rim + 2.0,
                    role: if matches!(ref_dir, RefDir::Uses) {
                        "dep"
                    } else {
                        "user"
                    },
                    label: (r.count > 1).then(|| format!("×{}", r.count)),
                    label_t: 0.5,
                    key: format!("c{}-{}", r.from, r.to),
                });
            }
        }
        _ => {}
    }
    chords.sort_by(|a, b| a.key.cmp(&b.key));

    // ---- Nodes. ------------------------------------------------------------
    let mut nodes: Vec<FlowNode<CodeNodeData>> = Vec::new();

    for file in &graph.files {
        let Some(p) = pos.get(&file_key(file.id)) else {
            continue;
        };
        let focal = sel_file.is_some_and(|f| f.id == file.id);
        // The selected file unfolds in place once its detail arrives.
        if focal && let Some(detail) = sel_detail {
            let geom = cutaway_geom(detail, sel_item);
            nodes.push(
                FlowNode::with_data(
                    file_key(file.id),
                    file.path.clone(),
                    (p.point.x - 10.0, p.point.y - 10.0),
                    CodeNodeData::Cutaway {
                        info: file.clone(),
                        name: file_name(file),
                        detail: detail.clone(),
                        sel_item,
                    },
                )
                .size(Size::new(geom.width, geom.height))
                .sides(Side::Left, Side::Right)
                .draggable(false)
                .selectable(false),
            );
            continue;
        }
        let b = file_box(file);
        nodes.push(
            FlowNode::with_data(
                file_key(file.id),
                file.path.clone(),
                (p.point.x - b / 2.0, p.point.y - b / 2.0),
                CodeNodeData::File {
                    info: file.clone(),
                    name: file_name(file),
                    focal,
                },
            )
            .size(Size::new(b, b))
            .sides(Side::Left, Side::Right)
            .draggable(false)
            .selectable(false),
        );
    }

    for dir in &tree.dirs {
        let visible = open.contains(&dir.id) || tree::is_gate(tree, open, dir.id);
        if !visible {
            continue;
        }
        let Some(p) = pos.get(&dir_key(dir.id)) else {
            continue;
        };
        let is_open = open.contains(&dir.id);
        let root = dir.id == ROOT;
        let b: f64 = if root { 44.0 } else { 36.0 };
        nodes.push(
            FlowNode::with_data(
                dir_key(dir.id),
                dir.path.clone(),
                (p.point.x - b / 2.0, p.point.y - b / 2.0),
                CodeNodeData::Dir {
                    id: dir.id,
                    name: dir.name.clone(),
                    files: dir.file_count,
                    open: is_open,
                    krate: dir.krate.clone(),
                    focal: sel_crate_dir == Some(dir.id),
                    root,
                    labeled: !is_open,
                    workspace: workspace.to_string(),
                },
            )
            .size(Size::new(b, b))
            .sides(Side::Left, Side::Right)
            .draggable(false)
            .selectable(false),
        );
    }
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    // ---- The frame. ---------------------------------------------------------
    let focused = !matches!(sel, CodeSel::None);
    let focal = match (&cut_anchor, &cut_geom) {
        (Some(anchor), Some(geom)) => Some(Point::new(
            anchor.x + geom.width / 2.0,
            anchor.y + geom.height / 2.0,
        )),
        _ => sel_file
            .and_then(|f| pos.get(&file_key(f.id)))
            .map(|p| p.point)
            .or_else(|| {
                sel_crate_dir
                    .and_then(|d| pos.get(&dir_key(d)))
                    .map(|p| p.point)
            }),
    };
    let focal_top = match (&cut_anchor, &cut_geom) {
        (Some(anchor), Some(geom)) => {
            Some(Point::new(anchor.x + geom.width / 2.0, anchor.y - 10.0))
        }
        _ => None,
    };
    let frame = if let (Some(anchor), Some(geom)) = (cut_anchor, &cut_geom) {
        let mut rects = vec![Rect::new(anchor.x, anchor.y, geom.width, geom.height)];
        rects.extend(hood.iter().map(|p| Rect::new(p.x - 30.0, p.y - 30.0, 60.0, 60.0)));
        Rect::bounds(rects)
    } else if let Some(f) = sel_file {
        let mut rects: Vec<Rect> = hood
            .iter()
            .map(|p| Rect::new(p.x - 30.0, p.y - 30.0, 60.0, 60.0))
            .collect();
        if let Some(p) = pos.get(&file_key(f.id)) {
            rects.push(Rect::new(p.point.x - 60.0, p.point.y - 60.0, 120.0, 120.0));
        }
        Rect::bounds(rects)
    } else if let Some(dir) = sel_crate_dir {
        // Frame the district: every visible mark under the crate's directory.
        let mut rects: Vec<Rect> = Vec::new();
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            if let Some(p) = pos.get(&dir_key(d)) {
                rects.push(Rect::new(p.point.x - 40.0, p.point.y - 40.0, 80.0, 80.0));
            }
            let node = &tree.dirs[d as usize];
            for &f in &node.files {
                if let Some(p) = pos.get(&file_key(f)) {
                    rects.push(Rect::new(p.point.x - 40.0, p.point.y - 40.0, 80.0, 80.0));
                }
            }
            stack.extend(node.dirs.iter().copied());
        }
        rects.extend(hood.iter().map(|p| Rect::new(p.x - 30.0, p.y - 30.0, 60.0, 60.0)));
        Rect::bounds(rects)
    } else {
        Rect::bounds(nodes.iter().map(|n| n.rect()))
    };

    Built {
        nodes,
        chords,
        frame,
        focal,
        focal_top,
        focused,
        pending_cutaway: sel_file.is_some() && sel_detail.is_none(),
    }
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

/// The engraved mark for one file: a solid ink star sized by how many files
/// reference it. Shared with the legend so the key can never drift.
#[component]
pub fn FileMarkSvg(
    refs_in: u32,
    focal: bool,
    #[props(default = 30.0)] box_px: f64,
) -> Element {
    let overhang = if focal { 10.0 } else { 3.5 };
    let r = star_radius(refs_in).min(box_px / 2.0 - overhang).max(2.5);
    let c = box_px / 2.0;
    rsx! {
        svg {
            class: "shrink-0",
            width: "{box_px}",
            height: "{box_px}",
            view_box: "0 0 {box_px} {box_px}",
            "aria-hidden": "true",
            if focal {
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r + 6.0}",
                    fill: "none",
                    stroke: "var(--color-ink)",
                    stroke_width: "0.7",
                }
                g { stroke: "var(--color-ink)", stroke_width: "0.9",
                    for angle in [0, 90, 180, 270] {
                        line {
                            x1: "{c + (r + 6.0) * (angle as f64).to_radians().cos()}",
                            y1: "{c + (r + 6.0) * (angle as f64).to_radians().sin()}",
                            x2: "{c + (r + 9.5) * (angle as f64).to_radians().cos()}",
                            y2: "{c + (r + 9.5) * (angle as f64).to_radians().sin()}",
                        }
                    }
                }
            }
            circle { cx: "{c}", cy: "{c}", r: "{r}", fill: "var(--color-ink)" }
            circle {
                cx: "{c}",
                cy: "{c}",
                r: "{r + 2.2}",
                fill: "none",
                stroke: "var(--color-ink)",
                stroke_width: "0.6",
                opacity: "0.85",
            }
        }
    }
}

/// The directory mark: a ruled square junction — paper when open, solid ink
/// when it is a closed gate holding its subtree.
#[component]
pub fn DirMarkSvg(
    open: bool,
    focal: bool,
    root: bool,
    #[props(default = 30.0)] box_px: f64,
) -> Element {
    let c = box_px / 2.0;
    let s: f64 = if root { 7.5 } else { 5.5 };
    rsx! {
        svg {
            class: "shrink-0",
            width: "{box_px}",
            height: "{box_px}",
            view_box: "0 0 {box_px} {box_px}",
            "aria-hidden": "true",
            if focal {
                rect {
                    x: "{c - s - 5.0}",
                    y: "{c - s - 5.0}",
                    width: "{(s + 5.0) * 2.0}",
                    height: "{(s + 5.0) * 2.0}",
                    fill: "none",
                    stroke: "var(--color-ink)",
                    stroke_width: "0.7",
                }
            }
            rect {
                x: "{c - s}",
                y: "{c - s}",
                width: "{s * 2.0}",
                height: "{s * 2.0}",
                fill: if open { "var(--color-paper)" } else { "var(--color-ink)" },
                stroke: "var(--color-ink)",
                stroke_width: "1.3",
            }
            if open {
                circle { cx: "{c}", cy: "{c}", r: "1.6", fill: "var(--color-ink)" }
            }
            if root {
                rect {
                    x: "{c - s - 2.8}",
                    y: "{c - s - 2.8}",
                    width: "{(s + 2.8) * 2.0}",
                    height: "{(s + 2.8) * 2.0}",
                    fill: "none",
                    stroke: "var(--color-ink)",
                    stroke_width: "0.6",
                    opacity: "0.85",
                }
            }
        }
    }
}

/// The tiny glyph vocabulary for items, shared by the cutaway, the panel,
/// and the legend. Every glyph is ink; kind is shape, never color.
#[component]
pub fn ItemGlyph(kind: ItemKind, #[props(default = 12.0)] box_px: f64) -> Element {
    let c = box_px / 2.0;
    rsx! {
        svg {
            class: "shrink-0",
            width: "{box_px}",
            height: "{box_px}",
            view_box: "0 0 {box_px} {box_px}",
            "aria-hidden": "true",
            match kind {
                ItemKind::Fn => rsx! {
                    circle { cx: "{c}", cy: "{c}", r: "2.6", fill: "var(--color-ink)" }
                },
                ItemKind::Struct => rsx! {
                    rect {
                        x: "{c - 2.6}",
                        y: "{c - 2.6}",
                        width: "5.2",
                        height: "5.2",
                        fill: "var(--color-ink)",
                    }
                },
                ItemKind::Enum => rsx! {
                    rect {
                        x: "{c - 2.7}",
                        y: "{c - 2.7}",
                        width: "5.4",
                        height: "5.4",
                        fill: "var(--color-ink)",
                        transform: "rotate(45 {c} {c})",
                    }
                },
                ItemKind::Union => rsx! {
                    rect {
                        x: "{c - 2.7}",
                        y: "{c - 2.7}",
                        width: "5.4",
                        height: "5.4",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                        transform: "rotate(45 {c} {c})",
                    }
                    circle { cx: "{c}", cy: "{c}", r: "1.1", fill: "var(--color-ink)" }
                },
                ItemKind::Trait => rsx! {
                    rect {
                        x: "{c - 2.8}",
                        y: "{c - 2.8}",
                        width: "5.6",
                        height: "5.6",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                    }
                },
                ItemKind::TypeAlias => rsx! {
                    rect {
                        x: "{c - 2.7}",
                        y: "{c - 2.7}",
                        width: "5.4",
                        height: "5.4",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                        transform: "rotate(45 {c} {c})",
                    }
                },
                ItemKind::Const | ItemKind::Static => rsx! {
                    rect {
                        x: "{c - 3.2}",
                        y: "{c - 0.9}",
                        width: "6.4",
                        height: "1.8",
                        fill: "var(--color-ink)",
                    }
                },
                ItemKind::Macro => rsx! {
                    g {
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                        stroke_linecap: "round",
                        line { x1: "{c}", y1: "{c - 3.2}", x2: "{c}", y2: "{c + 3.2}" }
                        line {
                            x1: "{c - 2.8}",
                            y1: "{c - 1.6}",
                            x2: "{c + 2.8}",
                            y2: "{c + 1.6}",
                        }
                        line {
                            x1: "{c + 2.8}",
                            y1: "{c - 1.6}",
                            x2: "{c - 2.8}",
                            y2: "{c + 1.6}",
                        }
                    }
                },
                ItemKind::Mod => rsx! {
                    rect {
                        x: "{c - 2.8}",
                        y: "{c - 2.8}",
                        width: "5.6",
                        height: "5.6",
                        fill: "none",
                        stroke: "var(--color-ink)",
                        stroke_width: "1.1",
                    }
                    circle { cx: "{c}", cy: "{c}", r: "1.2", fill: "var(--color-ink)" }
                },
                ItemKind::Impl => rsx! {
                    g { stroke: "var(--color-ink)", stroke_width: "1.1",
                        line { x1: "{c - 3.0}", y1: "{c - 3.0}", x2: "{c - 3.0}", y2: "{c + 3.0}" }
                        line { x1: "{c - 3.0}", y1: "{c - 3.0}", x2: "{c + 1.0}", y2: "{c - 3.0}" }
                        line { x1: "{c - 3.0}", y1: "{c + 3.0}", x2: "{c + 1.0}", y2: "{c + 3.0}" }
                    }
                },
            }
        }
    }
}

/// State words for a gate: "12 FILES".
fn gate_words(files: u32) -> String {
    if files == 1 {
        "1 FILE".to_string()
    } else {
        format!("{files} FILES")
    }
}

/// Node view for the code chart.
#[component]
fn CodeNode(ctx: NodeViewCtx<CodeNodeData>) -> Element {
    let nav = use_navigator();
    let code = use_code();
    match ctx.node.data.clone() {
        CodeNodeData::File { info, name, focal } => {
            let title = format!(
                "{} · {} lines · {} · referenced by {}",
                info.path,
                info.lines,
                plural(info.items as usize, "item"),
                plural(info.refs_in_files as usize, "file"),
            );
            let href = file_route(&info.path).to_string();
            let path = info.path.clone();
            let onclick = move |e: Event<MouseData>| {
                e.prevent_default();
                e.stop_propagation();
                if focal {
                    nav.push(crate::Route::CodeOverview {});
                } else {
                    nav.push(file_route(&path));
                }
            };
            rsx! {
                div {
                    class: "star-node code-mark is-named",
                    class: if focal { "is-focal" },
                    title: "{title}",
                    onkeydown: move |e| {
                        if e.key() == Key::Enter || e.key() == Key::Character(" ".to_string()) {
                            e.stop_propagation();
                        }
                    },
                    a {
                        href: "{href}",
                        class: "star-link",
                        aria_label: if focal { "deselect {info.path}" } else { "select {info.path} and draw its references" },
                        onclick,
                        FileMarkSvg {
                            refs_in: info.refs_in_files,
                            focal,
                            box_px: file_box(&info),
                        }
                        span { class: "star-label lab-e font-data font-medium text-ink", "{name}" }
                    }
                }
            }
        }
        CodeNodeData::Dir {
            id,
            name,
            files,
            open,
            krate,
            focal,
            root,
            labeled,
            workspace,
        } => {
            // The root's name hangs below its medallion; every other street
            // start letters its name above the spine it belongs to.
            let side = if root { "lab-s" } else { "lab-n" };
            let b = if root { 44.0 } else { 36.0 };
            let label = if root {
                workspace
            } else {
                format!("{name}/")
            };
            let title = if open {
                format!("{label} · click to fold")
            } else {
                format!("{label} · {} inside · click to open", gate_words(files))
            };
            rsx! {
                div {
                    class: "star-node code-mark is-named",
                    class: if focal { "is-focal" },
                    title: "{title}",
                    button {
                        class: "star-link",
                        aria_label: if open { "fold {label}" } else { "open {label} — {gate_words(files)} inside" },
                        onclick: move |e| {
                            e.stop_propagation();
                            // The root never folds.
                            if root {
                                return;
                            }
                            let mut toggled = code.toggled;
                            let mut set = toggled.peek().clone();
                            if !set.remove(&id) {
                                set.insert(id);
                            }
                            toggled.set(set);
                        },
                        DirMarkSvg { open, focal, root, box_px: b }
                        span {
                            class: "star-label {side} font-data",
                            class: if labeled { "" } else { "plan-quiet" },
                            span { class: "font-medium text-ink", "{label}" }
                            if !open {
                                span { class: "gate-count", " {gate_words(files)}" }
                            }
                        }
                    }
                    if let Some(krate) = krate {
                        Link {
                            class: "district-name",
                            to: crate::Route::CodeCrate { name: krate.clone() },
                            onclick: move |e: Event<MouseData>| e.stop_propagation(),
                            "CRATE {krate}"
                        }
                    }
                }
            }
        }
        CodeNodeData::Cutaway {
            info,
            name,
            detail,
            sel_item,
        } => rsx! {
            CutawayPlate {
                info,
                name,
                detail,
                sel_item,
            }
        },
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The cutaway: the selected file unfolded in place. Its roof comes off and
/// the items stand in source order; every neighbor keeps its ground.
#[component]
fn CutawayPlate(
    info: FileInfo,
    name: String,
    detail: FileDetail,
    sel_item: Option<u32>,
) -> Element {
    let nav = use_navigator();
    let geom = cutaway_geom(&detail, sel_item);
    let shown = geom.rows.len();
    let dir = info
        .path
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();

    rsx! {
        section {
            class: "cutaway plate",
            style: "width: {geom.width}px; height: {geom.height}px;",
            header { class: "flex items-baseline gap-1.5 px-3 pt-2",
                a {
                    href: crate::Route::CodeOverview {}.to_string(),
                    class: "cutaway-name font-data text-[11.5px] font-medium text-ink",
                    title: "fold {info.path} back to its mark",
                    onclick: move |e: Event<MouseData>| {
                        e.prevent_default();
                        e.stop_propagation();
                        nav.push(crate::Route::CodeOverview {});
                    },
                    "{name}"
                }
                span { class: "truncate font-data text-[9px] text-ink-soft", "{dir}/" }
                span { class: "ml-auto shrink-0 font-data text-[9px] text-ink-soft",
                    "{info.lines} L"
                }
            }
            p { class: "border-b border-ink-line px-3 pb-1 font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "{plural(info.fns as usize, \"fn\")} · {info.types} types · {info.traits} traits"
            }
            ul { class: "cutaway-items",
                for item in detail.items.iter().take(shown).cloned() {
                    li {
                        if item.kind == ItemKind::Impl {
                            div { class: "cutaway-section font-data",
                                ItemGlyph { kind: ItemKind::Impl, box_px: 10.0 }
                                span { class: "truncate", "{item.name}" }
                                span { class: "cutaway-line", "L{item.line}" }
                            }
                        } else {
                            {
                                let label = item_sel_label(&item);
                                let selected = sel_item == Some(item.id);
                                let href = item_route(&info.path, &label).to_string();
                                let path = info.path.clone();
                                let title = format!(
                                    "{label} · line {} · {}",
                                    item.line,
                                    if item.public { "pub" } else { "private" },
                                );
                                rsx! {
                                    a {
                                        href: "{href}",
                                        class: "cutaway-row font-data",
                                        class: if selected { "is-selected" },
                                        class: if item.public { "is-pub" },
                                        class: if !item.section.is_empty() { "in-section" },
                                        title: "{title}",
                                        onclick: move |e: Event<MouseData>| {
                                            e.prevent_default();
                                            e.stop_propagation();
                                            if selected {
                                                nav.push(file_route(&path));
                                            } else {
                                                nav.push(item_route(&path, &label));
                                            }
                                        },
                                        ItemGlyph { kind: item.kind, box_px: 11.0 }
                                        span { class: "truncate", "{item.name}" }
                                        span { class: "cutaway-line", "L{item.line}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if geom.hidden > 0 {
                p { class: "px-3 pt-0.5 font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                    "+ {geom.hidden} more — the panel lists all"
                }
            }
        }
    }
}

/// The reference chords, drawn as one engraved layer under the marks.
#[component]
fn ChordLayer(chords: Vec<Chord>) -> Element {
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for chord in chords {
                {
                    let ctrl = chord_ctrl(chord.from, chord.to);
                    let trim = |p: Point, toward: Point, by: f64| -> Point {
                        let (dx, dy) = (toward.x - p.x, toward.y - p.y);
                        let len = (dx * dx + dy * dy).sqrt();
                        if len < 1e-3 { p } else { Point::new(p.x + dx / len * by, p.y + dy / len * by) }
                    };
                    let start = trim(chord.from, ctrl, chord.from_rim);
                    let end = trim(chord.to, ctrl, chord.to_rim);
                    let d = format!(
                        "M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}",
                        start.x, start.y, ctrl.x, ctrl.y, end.x, end.y,
                    );
                    // The arrowhead at the user's end: change travels this way.
                    let (hdx, hdy) = {
                        let (dx, dy) = (end.x - ctrl.x, end.y - ctrl.y);
                        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                        (dx / len, dy / len)
                    };
                    let (hpx, hpy) = (-hdy, hdx);
                    let (hx, hy) = (end.x - hdx * 5.0, end.y - hdy * 5.0);
                    let head = format!(
                        "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
                        hx + hpx * 2.6, hy + hpy * 2.6, end.x, end.y, hx - hpx * 2.6, hy - hpy * 2.6,
                    );
                    let (lx, ly) = {
                        let t: f64 = chord.label_t;
                        let u = 1.0 - t;
                        (
                            u * u * start.x + 2.0 * u * t * ctrl.x + t * t * end.x,
                            u * u * start.y + 2.0 * u * t * ctrl.y + t * t * end.y,
                        )
                    };
                    rsx! {
                        g { key: "{chord.key}", class: "code-chord {chord.role}",
                            path { class: "chord-path", d, fill: "none" }
                            path {
                                class: "chord-head",
                                d: head,
                                fill: "none",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                            }
                            if let Some(label) = chord.label {
                                text { class: "chord-count", x: "{lx}", y: "{ly - 3.0}", text_anchor: "middle",
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The ground layer: the streets with their lettered names.
#[component]
fn GroundLayer(
    streets: Vec<tree::Street>,
    dir_names: HashMap<u32, (String, Option<String>)>,
) -> Element {
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for (i , street) in streets.iter().enumerate() {
                g { key: "{i}",
                    if street.x1 > street.x0 {
                        line {
                            class: "street-line",
                            x1: "{street.x0}",
                            y1: "{street.y}",
                            x2: "{street.x1}",
                            y2: "{street.y}",
                        }
                    }
                    if let Some((x, y0, y1)) = street.stub {
                        line {
                            class: "street-stub",
                            x1: "{x}",
                            y1: "{y0}",
                            x2: "{x}",
                            y2: "{y1}",
                        }
                    }
                    // The name letters below the line, in the stub gap,
                    // clear of the lots above. The crate district's caps
                    // hang from the directory's own mark instead.
                    if street.x1 > street.x0 && street.dir != u32::MAX {
                        if let Some((name, _)) = dir_names.get(&street.dir) {
                            text {
                                class: "street-name",
                                x: "{street.x0 + 16.0}",
                                y: "{street.y + 14.0}",
                                "{name}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Chrome insets at the code altitude. The phone's stack (cartouche, search,
/// refs) reaches ~300 CSS px and the reserve keeps slack above it; a focused
/// view adds the docked panel at 44dvh from the bottom. The free band between
/// them is shallow but honest — the cutaway pins its header there.
fn chrome_insets(narrow: bool, focused: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (312.0, 20.0, if focused { 424.0 } else { 70.0 }, 12.0)
    } else {
        (56.0, if focused { 330.0 } else { 20.0 }, 20.0, 284.0)
    }
}

fn narrow_viewport() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .map(|w| w < 640.0)
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

fn prefers_reduced_motion() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
            .flatten()
            .map(|m| m.matches())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

fn window_size() -> Option<(f64, f64)> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let w = window.inner_width().ok()?.as_f64()?;
        let h = window.inner_height().ok()?.as_f64()?;
        Some((w, h))
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

const MIN_FOCUS_ZOOM: f64 = 0.7;
const MIN_OVERVIEW_ZOOM: f64 = 0.18;

fn frame_chart(
    flow: dioxus_flow::prelude::FlowHandle<CodeNodeData>,
    bounds: Rect,
    focal: Option<Point>,
    focal_top: Option<Point>,
    focused: bool,
    duration_ms: u64,
) {
    let Some(core) = flow.core() else { return };
    let Some((w, h)) = window_size() else {
        return;
    };
    let narrow = narrow_viewport();
    let (t, r, b, l) = chrome_insets(narrow, focused);
    let free_w = (w - l - r).max(120.0);
    let free_h = (h - t - b).max(120.0);
    let fit = (free_w / bounds.width.max(1.0)).min(free_h / bounds.height.max(1.0)) * 0.88;
    // A phone's free band between the chrome stack and the docked panel is
    // shallow; holding the desktop floor there would seat the cutaway behind
    // the furniture. Legibility yields to visibility — the reader can zoom.
    let floor = if focused {
        if narrow { 0.42 } else { MIN_FOCUS_ZOOM }
    } else {
        MIN_OVERVIEW_ZOOM
    };
    let zoom = fit.clamp(floor, 1.0);
    // Past the legibility floor the whole neighborhood cannot fit: seat the
    // focal point mid-frame and let the reviewer pan for the rest. On a
    // shallow band the cutaway pins its header row to the band's top — the
    // file's name identifies the plate; the panel carries the overflow.
    let center = match (fit < floor, narrow, focal_top) {
        (true, true, Some(top)) => {
            Point::new(top.x, top.y + (free_h / 2.0 - 14.0) / zoom)
        }
        (true, _, _) => focal.unwrap_or_else(|| bounds.center()),
        _ => bounds.center(),
    };
    let cx = l + free_w / 2.0;
    let cy = t + free_h / 2.0;
    core.set_viewport(
        dioxus_flow::prelude::Viewport::new(cx - center.x * zoom, cy - center.y * zoom, zoom),
        duration_ms,
    );
}

/// Keyboard at the code altitude: `/` finds a file, `f` refits, Escape steps
/// up one focus level. Rebinds on every mount so altitude switches always
/// feed the living chart.
const CODE_KEYS_JS: &str = r#"
if (window.__slopifyKeys) {
    document.removeEventListener('keydown', window.__slopifyKeys);
}
window.__slopifyKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') {
        e.preventDefault();
        const s = document.getElementById('code-search');
        if (s) s.focus();
        return;
    }
    if (['f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopifyKeys);
"#;

/// The code chart, mounted once per stay at the code altitude.
#[component]
pub fn CodeChart(graph: CodeGraph, sel: CodeSel, workspace: String) -> Element {
    let code = use_code();
    let flow = dioxus_flow::use_flow_handle::<CodeNodeData>();
    let nav = use_navigator();

    let tree = use_memo({
        let graph = graph.clone();
        move || FileTree::build(&graph)
    });
    let open_depth = use_memo(move || tree::default_open_depth(&tree.read(), tree::MARK_BUDGET));

    // Selecting into folded ground opens the gates above it: disclosure
    // follows focus, and folding back is one click on the gate.
    let sel_for_open = sel.clone();
    let graph_for_open = graph.clone();
    use_effect(use_reactive((&sel_for_open,), move |(sel,)| {
        let tree = tree.read();
        let mut needed: Vec<u32> = Vec::new();
        let mut want_open = |dir: Option<u32>| {
            let mut dir = dir;
            while let Some(d) = dir {
                needed.push(d);
                dir = tree.dirs[d as usize].parent;
            }
        };
        match &sel {
            CodeSel::File(path, _) => {
                if let Some(f) = graph_for_open.files.iter().find(|f| &f.path == path) {
                    want_open(tree.dir_of_file.get(&f.id).copied());
                }
            }
            CodeSel::Crate(name) => {
                want_open(
                    tree.dirs
                        .iter()
                        .find(|d| d.krate.as_deref() == Some(name.as_str()))
                        .map(|d| d.id),
                );
            }
            CodeSel::None => {}
        }
        if needed.is_empty() {
            return;
        }
        let depth = *open_depth.peek();
        let toggled = code.toggled.peek().clone();
        let mut next = toggled.clone();
        for d in needed {
            let default_open = tree.dirs[d as usize].depth <= depth;
            // Open means default XOR toggled; force open.
            if default_open {
                next.remove(&d);
            } else {
                next.insert(d);
            }
        }
        if next != toggled {
            let mut sig = code.toggled;
            sig.set(next);
        }
    }));

    let open = use_memo(move || tree::open_dirs(&tree.read(), open_depth(), &code.toggled.read()));

    let file_names = use_memo({
        let graph = graph.clone();
        move || {
            graph
                .files
                .iter()
                .map(|f| {
                    (
                        f.id,
                        f.path
                            .rsplit_once('/')
                            .map(|(_, n)| n.to_string())
                            .unwrap_or_else(|| f.path.clone()),
                    )
                })
                .collect::<HashMap<u32, String>>()
        }
    });

    // The plan of the tree. Marks keep their node identity across a fold,
    // so the CSS transition draws them to their new seats.
    let layout_plan =
        use_memo(move || tree::plan_layout(&tree.read(), &open.read(), &file_names.read()));

    // `sel` is a prop, not a signal: the memo must be told to re-run when
    // the route hands the chart a new selection.
    let built = use_memo(use_reactive((&sel, &graph, &workspace), {
        move |(sel, graph, workspace)| {
            build_chart(
                &graph,
                &tree.read(),
                &open.read(),
                &layout_plan.read().pos,
                &sel,
                *code.ref_dir.read(),
                &code.details.read(),
                &workspace,
            )
        }
    }));

    let nodes: Signal<Vec<FlowNode<CodeNodeData>>> = use_signal(Vec::new);
    let framed = use_signal(|| false);

    use_effect(move || {
        let b = built();
        let mut nodes = nodes;
        nodes.set(b.nodes);
        let reduced = prefers_reduced_motion();
        #[cfg(target_arch = "wasm32")]
        {
            let mut framed = framed;
            if b.pending_cutaway {
                return;
            }
            let first = !*framed.peek();
            framed.set(true);
            if !b.focused && !first {
                return;
            }
            let duration = if first || reduced { 0 } else { 400 };
            let focused = b.focused;
            let frame = b.frame;
            let focal = b.focal;
            let focal_top = b.focal_top;
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(if first { 150 } else { 30 }).await;
                if let Some(frame) = frame {
                    frame_chart(flow, frame, focal, focal_top, focused, duration);
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (framed, reduced);
            if let Some(frame) = b.frame {
                frame_chart(flow, frame, b.focal, b.focal_top, b.focused, 0);
            }
        }
    });

    // Keyboard.
    use_hook(move || {
        let sel_now = move || built.peek().focused;
        spawn(async move {
            let mut eval = document::eval(CODE_KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                match key.as_str() {
                    "f" => {
                        let bounds =
                            Rect::bounds(built.peek().nodes.iter().map(|n| n.rect()));
                        if let Some(bounds) = bounds {
                            frame_chart(flow, bounds, None, None, false, 400);
                        }
                    }
                    "Escape" if sel_now() => {
                        nav.push(crate::Route::CodeOverview {});
                    }
                    _ => {}
                }
            }
        });
    });

    let dir_names = use_memo(use_reactive((&workspace,), move |(workspace,)| {
        tree.read()
            .dirs
            .iter()
            .map(|d| {
                let name = if d.id == ROOT {
                    workspace.clone()
                } else {
                    format!("{}/", d.name)
                };
                (d.id, (name, d.krate.clone()))
            })
            .collect::<HashMap<u32, (String, Option<String>)>>()
    }));
    let edges: Signal<Vec<dioxus_flow::prelude::Edge>> = use_signal(Vec::new);

    rsx! {
        div { class: "absolute inset-0",
            Flow {
                nodes,
                edges,
                fit_view: false,
                handle: flow,
                nodes_draggable: false,
                delete_key: false,
                node_view: move |ctx: NodeViewCtx<CodeNodeData>| rsx! {
                    CodeNode { ctx }
                },
                {
                    let (top, right, bottom, left) = chrome_insets(
                        narrow_viewport(),
                        built.read().focused,
                    );
                    rsx! {
                        FitInsets { top, right, bottom, left }
                    }
                }
                WorldLayer { class: "code-ground",
                    GroundLayer {
                        streets: layout_plan.read().streets.clone(),
                        dir_names: dir_names.read().clone(),
                    }
                }
                WorldLayer { class: "code-chords",
                    ChordLayer { chords: built.read().chords.clone() }
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
