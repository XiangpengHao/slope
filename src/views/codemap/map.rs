//! The ambient code map: districts, blocks, ties.
//!
//! Nested territories on paper — a bordered district per directory, a block
//! per file, landmark rows inside a block — and one summed tie per pair of
//! territories that reference each other. The altitude engraves only the
//! landmarks that clear its interest bar; everything else folds into a line
//! that counts what it hides. Item precision belongs to the focus plate: this
//! altitude never draws item-level spaghetti.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Flow, Node as FlowNode, NodeViewCtx, Point, Rect, Side, Size, Viewport,
};

use crate::api::{CodeGraph, FileInfo, ItemKind, Vis};
use crate::views::codemap::chrome::{file_name, plural};
use crate::views::codemap::model::{self, Containment, Territory};
use crate::views::codemap::tree::{self, FileTree, Measures, Placed, ROOT, dir_key, file_key};
use crate::views::codemap::{CodeSel, RefDir, file_route, item_route, use_code};

/// One landmark row inside a block.
#[derive(Clone, PartialEq)]
struct LandmarkRow {
    name: String,
    /// The label this item selects by in its URL.
    label: String,
    kind: ItemKind,
    /// 1 = loudest. Engraved size follows fan-in, in three tiers.
    tier: u8,
    fan_in: u32,
    vis: Vis,
}

/// One node on the code map: a file's block, or the gate of a folded
/// directory standing in for everything inside it.
#[derive(Clone, PartialEq)]
enum CodeNodeData {
    Block {
        info: FileInfo,
        name: String,
        rows: Vec<LandmarkRow>,
        /// What the block folded away, in words, and the height those words
        /// were measured to need.
        fold: Option<String>,
        fold_h: f64,
        size: (f64, f64),
        /// Inside the selected crate's district.
        focal: bool,
    },
    Gate {
        dir: u32,
        name: String,
        words: String,
        size: (f64, f64),
    },
}

/// A district frame, ready to engrave. An open frame names itself and stops
/// there: its files are on the paper to be counted. Only a fold counts what it
/// holds — that is the gate's line, not the frame's.
#[derive(Clone, PartialEq)]
struct DistrictView {
    dir: u32,
    at: Placed,
    label: String,
    /// The crate whose district this is, and its offset. Drawn only where the
    /// survey has more than one crate to tell apart.
    krate: Option<(String, f64)>,
    depth: u32,
    root: bool,
    focal: bool,
}

/// The engraved label band, matched to the CSS that draws it: both segments in
/// the data face, because both are data — a directory path and a crate name.
/// The name sets at 12px, the crate at 10px. These numbers move with
/// `tailwind.css` or the band starts colliding again.
fn name_w(text: &str) -> f64 {
    tree::text_w(text, 12.0)
}
fn crate_w(text: &str) -> f64 {
    tree::text_w(text, 10.0)
}

/// Where the band starts inside the frame, and the clear paper between its
/// segments.
const LABEL_X: f64 = 14.0;
const LABEL_GAP: f64 = 11.0;

/// The crate tag as engraved: the name cargo knows it by, and nothing else.
fn crate_tag(krate: &str) -> String {
    krate.to_string()
}

/// One tie, placed: from the definition's territory to the user's, where the
/// arrowhead rests — the way change travels.
#[derive(Clone, PartialEq)]
struct TieView {
    key: String,
    def: Territory,
    user: Territory,
    count: u32,
    /// The tie said as a sentence — "map.rs uses model.rs · 12 references" —
    /// its hover words, so the drawn count always has a findable subject.
    words: String,
    from: Point,
    to: Point,
    width: f64,
    /// The heaviest resting ties carry their count on the paper; the lighter
    /// ones keep it folded until the reader hovers an end.
    labeled: bool,
    /// Drawn at rest. A tie the reading folded away stays in the set and inks
    /// in when the reader hovers either of its ends — the fold is reachable,
    /// never silently cut.
    rest: bool,
}

/// Resting ties whose counts are engraved. Past this the labels would be the
/// map's texture instead of its data.
const TIE_LABELS: usize = 12;

// TODO: hovering an endpoint could split an aggregate bundle into per-block
// strands. It needs a second tie set built at the unfolded state and a way to
// pick which end unfolds; descoped until the aggregate reading is settled.

/// What ragged line breaks cost over a straight width ratio. The fold's words
/// buy this slack in width and spend it in the wrap estimate, so the same
/// number decides both and a fold never lands half-clipped.
const FOLD_SLACK: f64 = 1.12;

/// Landmark type size by tier, in CSS pixels.
fn row_px(tier: u8) -> f64 {
    match tier {
        1 => 12.5,
        2 => 11.0,
        _ => 10.0,
    }
}

impl LandmarkRow {
    /// One landmark row as it is written: `pub fn parse`, `struct Parse`. The
    /// row is measured on this string, not on the name alone, or the keyword
    /// the row exists to state would be the first thing clipped.
    fn words(&self) -> String {
        format!("{} {}", self.kind.decl_words(self.vis), self.name)
    }
}

/// A block's measured size, and the height its fold's words need. The layout
/// must know both before anything is drawn, so the plate and its box agree to
/// the pixel — and so a fold never has its count clipped.
fn block_size(name: &str, rows: &[LandmarkRow], fold: Option<&str>) -> (f64, f64, f64) {
    // Each line on its own face and tracking: the name at 10.5px, the fold's
    // words at 8.5px tracked 0.02em. The header holds the name and the changed
    // marker and nothing else, so the name is never the half that gets clipped.
    let head = tree::text_w(name, 10.5) + 22.0;
    // The fold buys the same slack in width that the wrap estimate spends, so
    // its one sentence stays one line whenever the block is allowed to widen.
    let fold_w = fold
        .map(|words| tree::tracked_w(words, 8.5, tree::MONO_ADVANCE, 0.02) * FOLD_SLACK + 4.0)
        .unwrap_or(0.0);
    let widest = rows
        .iter()
        .map(|r| tree::text_w(&r.words(), row_px(r.tier)) + 10.0)
        .fold(head.max(fold_w), f64::max);
    let w = (widest + tree::BLOCK_PAD_X * 2.0).clamp(tree::BLOCK_MIN_W, tree::BLOCK_MAX_W);
    // Ragged line breaks cost a little more than the ratio; the slack keeps
    // the last words inside the frame.
    let fold_h = fold
        .map(|words| {
            let usable = (w - tree::BLOCK_PAD_X * 2.0).max(40.0);
            let lines = (tree::tracked_w(words, 8.5, tree::MONO_ADVANCE, 0.02) * FOLD_SLACK
                / usable)
                .ceil()
                .max(1.0);
            4.0 + lines * tree::BLOCK_FOLD_LINE
        })
        .unwrap_or(0.0);
    let h = tree::BLOCK_HEAD_H + rows.len() as f64 * tree::BLOCK_ROW_H + fold_h + tree::BLOCK_FOOT;
    (w, h, fold_h)
}

/// One drawing of the code map: the blocks, districts and ties a single
/// build puts on the paper, plus what the camera needs to frame them.
#[derive(Clone, PartialEq)]
struct CodeDrawing {
    nodes: Vec<FlowNode<CodeNodeData>>,
    districts: Vec<DistrictView>,
    ties: Vec<TieView>,
    /// Flow-space rect to frame.
    frame: Option<Rect>,
    /// A crate district is selected: the camera goes there.
    focused: bool,
}

/// State words for a gate: what folding this directory hid. The `▸` already
/// says it is folded; the words only have to count.
fn gate_words(files: u32, items: u32) -> String {
    format!(
        "{} · {}",
        plural(files as usize, "file"),
        plural(items as usize, "item")
    )
}

impl CodeDrawing {
    fn build(
        graph: &CodeGraph,
        tree: &FileTree,
        open: &HashSet<u32>,
        containment: &Containment,
        sel: &CodeSel,
        workspace: &str,
        ref_dir: RefDir,
    ) -> Self {
        // Visible files: everything whose directory chain is open. A file behind
        // a gate keeps its references — they gather onto the gate.
        let visible: Vec<u32> = graph
            .files
            .iter()
            .filter(|f| {
                tree.dir_of_file
                    .get(&f.id)
                    .is_some_and(|d| open.contains(d))
            })
            .map(|f| f.id)
            .collect();

        let blocks = model::Block::all(graph, &visible, containment);

        // ---- Measure, then place. ---------------------------------------------
        /// A block's drawn content, measured before anything is seated.
        struct BlockView {
            info: FileInfo,
            name: String,
            rows: Vec<LandmarkRow>,
            fold: Option<String>,
            fold_h: f64,
        }
        let mut node_data: HashMap<u32, BlockView> = HashMap::new();
        let mut measures = Measures::default();
        for block in &blocks {
            let info = graph.files[block.file as usize].clone();
            let name = file_name(&info.path).to_string();
            let rows: Vec<LandmarkRow> = block
                .rows
                .iter()
                .map(|&m| {
                    let mark = &graph.items[m as usize];
                    LandmarkRow {
                        name: mark.name.clone(),
                        label: mark.label.clone(),
                        kind: mark.kind,
                        tier: mark.tier(),
                        fan_in: mark.fan_in,
                        vis: mark.vis,
                    }
                })
                .collect();
            let fold = block.fold_words();
            let (w, h, fold_h) = block_size(&name, &rows, fold.as_deref());
            measures.blocks.insert(block.file, (w, h));
            node_data.insert(
                block.file,
                BlockView {
                    info,
                    name,
                    rows,
                    fold,
                    fold_h,
                },
            );
        }

        // Items under a directory, for the district labels and the gates' words.
        let mut dir_items: Vec<u32> = vec![0; tree.dirs.len()];
        for file in &graph.files {
            if let Some(&dir) = tree.dir_of_file.get(&file.id) {
                let mut at = Some(dir);
                while let Some(d) = at {
                    dir_items[d as usize] += file.items;
                    at = tree.dirs[d as usize].parent;
                }
            }
        }

        let mut gate_text: HashMap<u32, String> = HashMap::new();
        for dir in &tree.dirs {
            if tree.is_gate(open, dir.id) {
                let words = gate_words(dir.file_count, dir_items[dir.id as usize]);
                // Two lines, each measured on its own face: the gate's name, then
                // its counts.
                let w = tree::text_w(&format!("▸ {}/", dir.name), 10.0).max(tree::tracked_w(
                    &words,
                    8.5,
                    tree::MONO_ADVANCE,
                    0.05,
                )) + tree::BLOCK_PAD_X * 2.0
                    + 6.0;
                measures.gates.insert(
                    dir.id,
                    (w.clamp(tree::BLOCK_MIN_W, tree::BLOCK_MAX_W), tree::GATE_H),
                );
                gate_text.insert(dir.id, words);
            }
        }

        // A crate tag only earns its place where there is more than one crate to
        // tell apart; in a single-crate workspace it repeats the cartouche.
        let multi_crate = graph
            .files
            .iter()
            .map(|f| f.krate.as_str())
            .collect::<HashSet<&str>>()
            .len()
            > 1;
        let district_label = |dir: &tree::DirNode| -> String {
            // The directory as it is on disk, with the marker that folds it. The
            // root holds the whole survey and never folds, so it carries none.
            if dir.id == ROOT {
                workspace.to_string()
            } else {
                format!("▾ {}/", dir.name)
            }
        };
        fn district_crate(dir: &tree::DirNode, multi_crate: bool) -> Option<&str> {
            dir.krate.as_deref().filter(|_| multi_crate)
        }
        for dir in &tree.dirs {
            if !open.contains(&dir.id) {
                continue;
            }
            // The frame must be wide enough for the whole band.
            let mut band = LABEL_X + name_w(&district_label(dir)) + LABEL_X;
            if let Some(krate) = district_crate(dir, multi_crate) {
                band += LABEL_GAP + crate_w(&crate_tag(krate));
            }
            measures.labels.insert(dir.id, band);
        }

        let layout = tree.layout(open, &measures);

        // ---- The selection: a crate district, or nothing. ---------------------
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

        // ---- Nodes. -----------------------------------------------------------
        let mut nodes: Vec<FlowNode<CodeNodeData>> = Vec::new();
        for (file, at) in &layout.blocks {
            let Some(view) = node_data.get(file) else {
                continue;
            };
            nodes.push(
                FlowNode::with_data(
                    file_key(*file),
                    view.info.path.clone(),
                    (at.x, at.y),
                    CodeNodeData::Block {
                        info: view.info.clone(),
                        name: view.name.clone(),
                        rows: view.rows.clone(),
                        fold: view.fold.clone(),
                        fold_h: view.fold_h,
                        size: (at.w, at.h),
                        focal: crate_files.contains(file),
                    },
                )
                .size(Size::new(at.w, at.h))
                .sides(Side::Left, Side::Right)
                .draggable(false)
                .selectable(false),
            );
        }
        for (dir, at) in &layout.gates {
            nodes.push(
                FlowNode::with_data(
                    dir_key(*dir),
                    tree.dirs[*dir as usize].path.clone(),
                    (at.x, at.y),
                    CodeNodeData::Gate {
                        dir: *dir,
                        name: tree.dirs[*dir as usize].name.clone(),
                        words: gate_text.get(dir).cloned().unwrap_or_default(),
                        size: (at.w, at.h),
                    },
                )
                .size(Size::new(at.w, at.h))
                .sides(Side::Left, Side::Right)
                .draggable(false)
                .selectable(false),
            );
        }
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        // ---- Districts. -------------------------------------------------------
        let districts: Vec<DistrictView> = layout
            .districts
            .iter()
            .map(|d| {
                let dir = &tree.dirs[d.dir as usize];
                let label = district_label(dir);
                // Two segments, seated left to right, the crate clear of the name.
                let after_name = LABEL_X + name_w(&label) + LABEL_GAP;
                DistrictView {
                    dir: d.dir,
                    at: d.at,
                    label,
                    krate: district_crate(dir, multi_crate)
                        .map(|name| (name.to_string(), after_name)),
                    depth: d.depth,
                    root: d.dir == ROOT,
                    focal: sel_crate_dir == Some(d.dir),
                }
            })
            .collect();

        // ---- Ties: every reference between two territories, summed. -----------
        let gate_for = |file: u32| -> Option<Territory> {
            let mut dir = tree.dir_of_file.get(&file).copied();
            while let Some(d) = dir {
                if layout.gates.contains_key(&d) {
                    return Some(Territory::Dir(d));
                }
                dir = tree.dirs[d as usize].parent;
            }
            None
        };
        let territory = |file: u32| -> Option<Territory> {
            if layout.blocks.contains_key(&file) {
                Some(Territory::File(file))
            } else {
                gate_for(file)
            }
        };
        let rect_of = |t: Territory| -> Option<Placed> {
            match t {
                Territory::File(f) => layout.blocks.get(&f).copied(),
                Territory::Dir(d) => layout.gates.get(&d).copied(),
            }
        };
        let all_ties = containment.ties(graph, territory);
        // A selected crate reads like the dependency chart's selection: it is the
        // anchor, so only ties crossing its boundary draw, in the direction the
        // reading asks for. A territory is the selection's when its file belongs to
        // the crate, or its gate stands inside the crate's district.
        let in_sel = |t: Territory| -> bool {
            let Some(sel_dir) = sel_crate_dir else {
                return false;
            };
            match t {
                Territory::File(f) => crate_files.contains(&f),
                Territory::Dir(mut d) => loop {
                    if d == sel_dir {
                        break true;
                    }
                    match tree.dirs[d as usize].parent {
                        Some(p) => d = p,
                        None => break false,
                    }
                },
            }
        };
        let kept: Vec<model::Tie> = all_ties
            .into_iter()
            .filter(|tie| {
                if sel_crate_dir.is_none() {
                    return true;
                }
                match ref_dir {
                    RefDir::Both => in_sel(tie.def) || in_sel(tie.user),
                    RefDir::UsedBy => in_sel(tie.def) && !in_sel(tie.user),
                    RefDir::Uses => in_sel(tie.user) && !in_sel(tie.def),
                }
            })
            .collect();

        // Which ties rest on the paper. Direction alone cannot thin an unanchored
        // map — every tie is one territory's use and another's users — so each
        // territory becomes its own anchor and draws its heaviest few in the
        // reading's direction. Everything else stays in the set and inks in when
        // the reader hovers one of its ends. A crate selection is already an
        // anchor, so its boundary set draws whole.
        let resting: HashSet<usize> = match (sel_crate_dir, ref_dir.per_territory()) {
            (None, Some(cap)) => {
                let mut by_anchor: HashMap<Territory, Vec<usize>> = HashMap::new();
                for (i, tie) in kept.iter().enumerate() {
                    // `uses` anchors on the territory writing the reference; `used
                    // by` anchors on the one being referenced.
                    let anchor = match ref_dir {
                        RefDir::UsedBy => tie.def,
                        _ => tie.user,
                    };
                    by_anchor.entry(anchor).or_default().push(i);
                }
                by_anchor
                    .into_values()
                    .flat_map(|mut idx| {
                        idx.sort_unstable_by_key(|&i| (std::cmp::Reverse(kept[i].count), i));
                        idx.into_iter().take(cap)
                    })
                    .collect()
            }
            _ => (0..kept.len()).collect(),
        };

        // The label bar: among the resting ties, the heaviest handful state their
        // counts, so the labels stay data instead of texture. Every other resting
        // tie says its count when the reader hovers either end.
        let label_bar = {
            let mut counts: Vec<u32> = resting.iter().map(|&i| kept[i].count).collect();
            counts.sort_unstable_by(|a, b| b.cmp(a));
            counts.get(TIE_LABELS).copied().unwrap_or(0).max(2)
        };
        // A territory's name, for the tie's own hover words.
        let name_of = |t: Territory| -> String {
            match t {
                Territory::File(f) => graph
                    .files
                    .get(f as usize)
                    .map(|f| file_name(&f.path).to_string())
                    .unwrap_or_default(),
                Territory::Dir(d) => format!("{}/", tree.dirs[d as usize].name),
            }
        };
        let ties: Vec<TieView> = kept
            .iter()
            .enumerate()
            .filter_map(|(i, tie)| {
                let (def, user) = (rect_of(tie.def)?, rect_of(tie.user)?);
                let (from, to) = def.tie_ends(user);
                let rest = resting.contains(&i);
                Some(TieView {
                    key: format!("{:?}-{:?}", tie.def, tie.user),
                    def: tie.def,
                    user: tie.user,
                    count: tie.count,
                    // The tie's own sentence, on hover: it names both ends the
                    // way the arrow reads them, so the count on the paper is
                    // never a number without a subject.
                    words: format!(
                        "{} uses {} · {}",
                        name_of(tie.user),
                        name_of(tie.def),
                        plural(tie.count as usize, "reference")
                    ),
                    from,
                    to,
                    width: (0.55 + tie.count as f64 * 0.13).min(2.8),
                    labeled: rest && tie.count > label_bar,
                    rest,
                })
            })
            .collect();

        // ---- The frame. -------------------------------------------------------
        let frame = match sel_crate_dir.and_then(|d| {
            layout
                .districts
                .iter()
                .find(|dv| dv.dir == d)
                .map(|dv| dv.at)
        }) {
            Some(at) => Some(Rect::new(
                at.x - 30.0,
                at.y - 30.0,
                at.w + 60.0,
                at.h + 60.0,
            )),
            None => Rect::bounds(nodes.iter().map(|n| n.rect())).or_else(|| {
                layout
                    .districts
                    .first()
                    .map(|d| Rect::new(d.at.x, d.at.y, d.at.w, d.at.h))
            }),
        };

        CodeDrawing {
            nodes,
            districts,
            ties,
            frame,
            focused: sel_crate_dir.is_some(),
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering.
// ---------------------------------------------------------------------------

/// One file's block: its name on the header, its landmarks engraved beneath,
/// and the fold's own words at the foot. The header states the name and whether
/// the file changed — the tallies belong where something is actually hidden.
#[component]
fn BlockPlate(
    info: FileInfo,
    name: String,
    rows: Vec<LandmarkRow>,
    fold: Option<String>,
    fold_h: f64,
    size: (f64, f64),
    focal: bool,
) -> Element {
    let nav = use_navigator();
    let path = info.path.clone();
    let (w, h) = size;
    rsx! {
        section {
            class: "code-block",
            class: if focal { "is-focal" },
            style: "width: {w}px; height: {h}px;",
            header {
                class: "cb-head",
                style: "height: {tree::BLOCK_HEAD_H}px;",
                a {
                    class: "cb-name",
                    href: file_route(&info.path).to_string(),
                    title: "{info.path} · {info.lines} lines · {plural(info.items as usize, \"item\")} · focus this file",
                    onclick: {
                        let path = path.clone();
                        move |e: Event<MouseData>| {
                            e.prevent_default();
                            e.stop_propagation();
                            nav.push(file_route(&path));
                        }
                    },
                    "{name}"
                    if info.changed {
                        span { class: "cb-chg", title: "changed since the diff base", "M" }
                    }
                }
            }
            for row in rows.iter() {
                a {
                    key: "{row.label}",
                    class: "cb-row t{row.tier}",
                    style: "height: {tree::BLOCK_ROW_H}px; font-size: {row_px(row.tier)}px;",
                    href: item_route(&info.path, &row.label).to_string(),
                    title: "{row.label} · {plural(row.fan_in as usize, \"reference\")} in from other files",
                    onclick: {
                        let path = path.clone();
                        let label = row.label.clone();
                        move |e: Event<MouseData>| {
                            e.prevent_default();
                            e.stop_propagation();
                            nav.push(item_route(&path, &label));
                        }
                    },
                    span { class: "cb-kw", "{row.kind.decl_words(row.vis)}" }
                    span { class: "cb-nm", "{row.name}" }
                }
            }
            if let Some(fold) = fold {
                p {
                    class: "cb-fold",
                    style: "height: {fold_h}px;",
                    title: "the file's quieter items, folded — open the file to read them",
                    "{fold}"
                }
            }
        }
    }
}

/// Node view for the code map.
#[component]
fn CodeNode(ctx: NodeViewCtx<CodeNodeData>) -> Element {
    let code = use_code();
    match ctx.node.data.clone() {
        CodeNodeData::Block {
            info,
            name,
            rows,
            fold,
            fold_h,
            size,
            focal,
        } => rsx! {
            BlockPlate {
                info,
                name,
                rows,
                fold,
                fold_h,
                size,
                focal,
            }
        },
        CodeNodeData::Gate {
            dir,
            name,
            words,
            size,
        } => {
            let (w, h) = size;
            rsx! {
                button {
                    class: "code-gate",
                    style: "width: {w}px; height: {h}px;",
                    title: "{name}/ — {words} · click to open",
                    onclick: move |e: Event<MouseData>| {
                        e.stop_propagation();
                        let mut toggled = code.toggled;
                        let mut set = toggled.peek().clone();
                        if !set.remove(&dir) {
                            set.insert(dir);
                        }
                        toggled.set(set);
                    },
                    span { class: "cg-name", "▸ {name}/" }
                    span { class: "cg-meta", "{words}" }
                }
            }
        }
    }
}

/// The ground: bordered districts with their names engraved on the border.
/// A district's label folds it; its crate name climbs to the crate sheet.
#[component]
fn DistrictLayer(districts: Vec<DistrictView>) -> Element {
    let code = use_code();
    let nav = use_navigator();
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for d in districts.iter() {
                g { key: "{d.dir}",
                    rect {
                        class: "district-frame",
                        class: if d.focal { "is-focal" },
                        x: "{d.at.x}",
                        y: "{d.at.y}",
                        width: "{d.at.w}",
                        height: "{d.at.h}",
                    }
                    text {
                        class: "district-label",
                        class: if d.root { "is-root" },
                        x: "{d.at.x + LABEL_X}",
                        y: "{d.at.y + 4.0}",
                        onclick: {
                            let dir = d.dir;
                            let root = d.root;
                            move |e: Event<MouseData>| {
                                e.stop_propagation();
                                // The root holds the whole survey; it never folds.
                                if root {
                                    return;
                                }
                                let mut toggled = code.toggled;
                                let mut set = toggled.peek().clone();
                                if !set.remove(&dir) {
                                    set.insert(dir);
                                }
                                toggled.set(set);
                            }
                        },
                        "{d.label}"
                    }
                    if let Some((krate, dx)) = d.krate.clone() {
                        text {
                            class: "district-crate",
                            x: "{d.at.x + dx}",
                            y: "{d.at.y + 4.0}",
                            onclick: move |e: Event<MouseData>| {
                                e.stop_propagation();
                                nav.push(crate::Route::CodeCrate { name: krate.clone() });
                            },
                            "{krate}"
                        }
                    }
                }
            }
        }
    }
}

/// The ties, drawn as one engraved layer over the district tints and under
/// the blocks. Hovering a territory brings its own ties up to full ink — the
/// signal arrives as a signal, not a value, so a hover redraws this layer
/// alone and never the blocks.
#[component]
fn TieLayer(ties: Vec<TieView>, hot: Signal<Option<Territory>>) -> Element {
    let hot = hot();
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for tie in ties.iter() {
                {
                    let (a, b) = (tie.from, tie.to);
                    let (dx, dy) = (b.x - a.x, b.y - a.y);
                    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                    let bow = (len * 0.16).min(52.0);
                    let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                    let ctrl = Point::new(mid.x - dy / len * bow, mid.y + dx / len * bow);
                    let d = format!(
                        "M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}",
                        a.x, a.y, ctrl.x, ctrl.y, b.x, b.y,
                    );
                    // The arrowhead rests on the user's edge.
                    let (hdx, hdy) = {
                        let (ex, ey) = (b.x - ctrl.x, b.y - ctrl.y);
                        let l = (ex * ex + ey * ey).sqrt().max(1e-3);
                        (ex / l, ey / l)
                    };
                    let (px, py) = (-hdy, hdx);
                    let head_at = 4.0 + tie.width;
                    let (hx, hy) = (b.x - hdx * head_at, b.y - hdy * head_at);
                    let head = format!(
                        "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
                        hx + px * (1.9 + tie.width * 0.5),
                        hy + py * (1.9 + tie.width * 0.5),
                        b.x,
                        b.y,
                        hx - px * (1.9 + tie.width * 0.5),
                        hy - py * (1.9 + tie.width * 0.5),
                    );
                    let (lx, ly) = (
                        0.25 * a.x + 0.5 * ctrl.x + 0.25 * b.x,
                        0.25 * a.y + 0.5 * ctrl.y + 0.25 * b.y,
                    );
                    let is_hot = hot.is_some_and(|h| h == tie.def || h == tie.user);
                    rsx! {
                        g {
                            key: "{tie.key}",
                            class: "code-tie",
                            class: if !tie.rest { "is-folded" },
                            class: if !tie.labeled { "is-quiet" },
                            class: if is_hot { "is-hot" },
                            title { "{tie.words}" }
                            path {
                                class: "tie-path",
                                d,
                                fill: "none",
                                style: "stroke-width: {tie.width}px;",
                            }
                            path {
                                class: "tie-head",
                                d: head,
                                fill: "none",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                            }
                            text {
                                class: "tie-count",
                                x: "{lx}",
                                y: "{ly - 3.0}",
                                text_anchor: "middle",
                                "{tie.count}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Chrome insets at the code altitude: the cartouche column on the left, the
/// crate sheet docked right when one is selected.
fn chrome_insets(narrow: bool, focused: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (312.0, 20.0, if focused { 424.0 } else { 70.0 }, 12.0)
    } else {
        (56.0, if focused { 330.0 } else { 24.0 }, 24.0, 284.0)
    }
}

/// What the browser says about the window it is drawing in. Every chart that
/// reserves room for its furniture needs the same three answers, so they are
/// asked once here.
pub(crate) fn narrow_viewport() -> bool {
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

pub(crate) fn prefers_reduced_motion() -> bool {
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

pub(crate) fn window_size() -> Option<(f64, f64)> {
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

/// Below this the block letters stop being letters; the reviewer pans instead.
const MIN_MAP_ZOOM: f64 = 0.22;

/// The camera as the reviewer last left it. Session state that must survive
/// route-variant remounts, like the disclosure state: opening a focus plate
/// unmounts the map, and coming back must give the reader back their own pan
/// and zoom, not a fresh framing — the camera carries the mental map (the
/// Kept-Ground rule). `f` still refits on demand. Provided as a context by
/// the app shell, which outlives every remount.
#[derive(Clone, Copy)]
pub(crate) struct CodeCamera {
    pub(crate) viewport: Signal<Option<Viewport>>,
}

impl CodeCamera {
    pub(crate) fn new() -> Self {
        Self {
            viewport: Signal::new(None),
        }
    }
}

fn frame_chart(
    flow: dioxus_flow::prelude::FlowHandle<CodeNodeData>,
    bounds: Rect,
    focused: bool,
    duration_ms: u64,
) {
    let Some(core) = flow.core() else { return };
    let Some((w, h)) = window_size() else {
        return;
    };
    let (t, r, b, l) = chrome_insets(narrow_viewport(), focused);
    let free_w = (w - l - r).max(120.0);
    let free_h = (h - t - b).max(120.0);
    let fit = (free_w / bounds.width.max(1.0)).min(free_h / bounds.height.max(1.0)) * 0.94;
    let zoom = fit.clamp(MIN_MAP_ZOOM, 1.0);
    let center = bounds.center();
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
if (window.__slopeKeys) {
    document.removeEventListener('keydown', window.__slopeKeys);
}
window.__slopeKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') {
        e.preventDefault();
        // The map lays out one search plate for the desktop and one for a
        // narrow viewport; only one of them is on the page at a time, so `/`
        // must reach whichever is actually showing.
        const s = [...document.querySelectorAll('#code-search')]
            .find((el) => el.offsetParent !== null);
        if (s) s.focus();
        return;
    }
    if (['f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopeKeys);
"#;

/// The ambient map, mounted while no file or item holds the focus.
#[component]
pub(super) fn CodeChart(graph: CodeGraph, sel: CodeSel, workspace: String) -> Element {
    let code = use_code();
    let camera = use_context::<CodeCamera>();
    let flow = dioxus_flow::use_flow_handle::<CodeNodeData>();
    let nav = use_navigator();

    let tree = use_memo({
        let graph = graph.clone();
        move || FileTree::build(&graph)
    });
    let containment = use_memo({
        let graph = graph.clone();
        move || Containment::build(&graph)
    });
    let open_depth = use_memo(move || tree.read().default_open_depth(tree::MARK_BUDGET));

    // Selecting a crate opens the gates above its district: disclosure follows
    // focus, and folding back is one click on the gate.
    let sel_for_open = sel.clone();
    use_effect(use_reactive((&sel_for_open,), move |(sel,)| {
        let tree = tree.read();
        let CodeSel::Crate(name) = &sel else { return };
        let mut needed: Vec<u32> = Vec::new();
        let mut dir = tree
            .dirs
            .iter()
            .find(|d| d.krate.as_deref() == Some(name.as_str()))
            .map(|d| d.id);
        while let Some(d) = dir {
            needed.push(d);
            dir = tree.dirs[d as usize].parent;
        }
        if needed.is_empty() {
            return;
        }
        let depth = *open_depth.peek();
        let toggled = code.toggled.peek().clone();
        let mut next = toggled.clone();
        for d in needed {
            // Open means default XOR toggled; force open.
            if tree.dirs[d as usize].depth <= depth {
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

    let open = use_memo(move || tree.read().open_dirs(open_depth(), &code.toggled.read()));

    // `sel` is a prop, not a signal: the memo must be told when the route
    // hands the map a new selection. The direction toggle is a signal and
    // tracks itself.
    let chart = use_memo(use_reactive((&sel, &graph, &workspace), {
        move |(sel, graph, workspace)| {
            CodeDrawing::build(
                &graph,
                &tree.read(),
                &open.read(),
                &containment.read(),
                &sel,
                &workspace,
                *code.ref_dir.read(),
            )
        }
    }));

    let nodes: Signal<Vec<FlowNode<CodeNodeData>>> = use_signal(Vec::new);
    let framed = use_signal(|| false);
    let mut hot: Signal<Option<Territory>> = use_signal(|| None);
    // True once the flow's core is live; the camera mirror below waits on it.
    let core_live: Signal<bool> = use_signal(|| false);

    use_effect(move || {
        let drawing = chart();
        let mut nodes = nodes;
        nodes.set(drawing.nodes);
        let reduced = prefers_reduced_motion();
        #[cfg(target_arch = "wasm32")]
        {
            let mut framed = framed;
            let first = !*framed.peek();
            framed.set(true);
            if !drawing.focused && !first {
                return;
            }
            let duration = if first || reduced { 0 } else { 400 };
            let (frame, focused) = (drawing.frame, drawing.focused);
            let mut core_live = core_live;
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(if first { 150 } else { 30 }).await;
                if first {
                    // The canvas mounts on its own beat; wait for its core
                    // (bounded) rather than framing into the void.
                    for _ in 0..40 {
                        if flow.core().is_some() {
                            break;
                        }
                        gloo_timers::future::TimeoutFuture::new(50).await;
                    }
                    core_live.set(true);
                    // A remount is a return — from a focus plate or another
                    // altitude, never a new focus: focus changes glide above
                    // without remounting. Seat the reader where they left.
                    if let Some(vp) = *camera.viewport.peek() {
                        flow.set_viewport(vp, 0);
                        return;
                    }
                }
                if let Some(frame) = frame {
                    frame_chart(flow, frame, focused, duration);
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (framed, reduced, core_live);
            if let Some(frame) = drawing.frame {
                frame_chart(flow, frame, drawing.focused, 0);
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

    // Keyboard.
    use_hook(move || {
        spawn(async move {
            let mut eval = document::eval(CODE_KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                match key.as_str() {
                    "f" => {
                        if let Some(bounds) =
                            Rect::bounds(chart.peek().nodes.iter().map(|n| n.rect()))
                        {
                            frame_chart(flow, bounds, false, 400);
                        }
                    }
                    "Escape" if chart.peek().focused => {
                        nav.push(crate::Route::CodeOverview {});
                    }
                    _ => {}
                }
            }
        });
    });

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
                // A trackpad's two-finger travel is a pan, as every canvas
                // tool reads it; pinch (ctrl/meta wheel) zooms at the pointer.
                pan_on_scroll: true,
                node_view: move |ctx: NodeViewCtx<CodeNodeData>| {
                    let territory = match &ctx.node.data {
                        CodeNodeData::Block { info, .. } => Territory::File(info.id),
                        CodeNodeData::Gate { dir, .. } => Territory::Dir(*dir),
                    };
                    rsx! {
                        div {
                            class: "code-territory",
                            onmouseenter: move |_| hot.set(Some(territory)),
                            onmouseleave: move |_| hot.set(None),
                            CodeNode { ctx }
                        }
                    }
                },
                {
                    let (top, right, bottom, left) = chrome_insets(
                        narrow_viewport(),
                        chart.read().focused,
                    );
                    rsx! {
                        FitInsets { top, right, bottom, left }
                    }
                }
                WorldLayer { class: "code-ground",
                    DistrictLayer { districts: chart.read().districts.clone() }
                }
                WorldLayer { class: "code-ties",
                    TieLayer { ties: chart.read().ties.clone(), hot }
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
