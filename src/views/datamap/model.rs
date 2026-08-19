//! The data chart's reading of the survey: types as marks, holding as edges.
//!
//! Pure functions over the wire model — no layout and no rendering. The survey
//! ships every type it found, private ones included; this module decides which
//! of them the chart draws as marks, which frame each one sits in, where the
//! rest fold to, and which anchor every edge lands on. Nothing is dropped
//! without a count: a private type folds to its frame's counted row and the
//! edges touching it land there, the way the code map's ties land on a gate.
//!
//! Reference ties are computed here too. The survey records references between
//! items; this altitude reads them at type precision, so each endpoint climbs
//! its containment chain to the outermost mark and a tie is kept only when both
//! ends land on a drawn type. References written in free functions never reach
//! a type, so they are not on this chart — the legend says so.

use std::collections::{HashMap, HashSet};

use crate::api::{CodeGraph, HoldKind, ItemKind, ItemMark, Vis};
use crate::views::codemap::RefDir;
use crate::views::codemap::model::Containment;

/// Marks the first paint budgets for. Past it, each frame folds its quietest
/// types into a counted row rather than drawing a wall of blocks.
pub const MARK_BUDGET: usize = 200;
/// Incoming holds edges a type draws before folding them to a count on its own
/// mark. A type four other types reach is a hub, and its fan-in drawn in full
/// is a star burst nobody can read.
pub const HELD_CAP: usize = 3;
/// Holding fields one mark quotes before it defers to a counted line.
pub const FIELD_CAP: usize = 8;
/// Resting reference ties whose counts are engraved. Past this the labels are
/// the chart's texture instead of its data.
pub const TIE_LABELS: usize = 12;
/// Reference ties one type rests in an anchored reading.
pub const TIES_PER_MARK: usize = 2;

/// Where an edge can land: a drawn mark, or one of a frame's counted fold rows.
/// Privacy folds a type for good and the budget folds the quietest ones; either
/// way the edge lands on the row that counts it instead of being cut.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Anchor {
    /// A type or static with a block of its own.
    Mark(u32),
    /// A frame's `+ n private types` row.
    Private(u32),
    /// A frame's `+ n more types` row, where the budget folded the quietest.
    More(u32),
}

/// One frame on the paper: a workspace crate, or one top-level module inside a
/// crate. One level of module frames only — a deeper module path stays in the
/// mark's locator, where rust already writes it.
#[derive(Clone, PartialEq, Debug)]
pub struct Frame {
    pub id: u32,
    pub krate: String,
    /// The top-level module, as rust names it. `None` is the crate's own frame,
    /// which holds the types its crate root declares.
    pub module: Option<String>,
    /// The crate frame a module frame sits in.
    pub parent: Option<u32>,
    /// Drawn marks seated here, in the survey's (file, source) order.
    pub marks: Vec<u32>,
    /// Private types, never drawn, counted here.
    pub private: u32,
    /// Types the budget folded away, counted here.
    pub more: u32,
}

impl Frame {
    /// The label engraved on the frame's border, in rust's own words. A crate
    /// frame names its crate only where the survey has more than one to tell
    /// apart; in a single-crate workspace that name is already the cartouche's.
    pub fn label(&self, multi_crate: bool) -> Option<String> {
        match &self.module {
            Some(module) => Some(format!("mod {module}")),
            None => multi_crate.then(|| self.krate.clone()),
        }
    }
}

/// One holding field, quoted from the source: the name as written and the
/// declared type as written. Nothing here is reconstructed.
#[derive(Clone, PartialEq, Debug)]
pub struct FieldRow {
    pub name: String,
    pub decl: String,
    /// The held type's name — the one run of the declaration drawn in full ink,
    /// so `Vec<FileDetail>` reads as the wrapper it is around the type it holds.
    pub target: String,
}

/// One type, static, or union with a block on the paper.
#[derive(Clone, PartialEq, Debug)]
pub struct DataMark {
    pub id: u32,
    pub frame: u32,
    pub kind: ItemKind,
    pub vis: Vis,
    pub name: String,
    /// The label its definition plate selects by, for the URL.
    pub label: String,
    /// The defining file, relative to the workspace root.
    pub path: String,
    pub line: u32,
    /// Its file changed since the diff base.
    pub changed: bool,
    /// Holding fields, quoted, capped at [`FIELD_CAP`].
    pub fields: Vec<FieldRow>,
    /// Holding fields past the cap.
    pub more_fields: u32,
    /// Fields whose type walk reached no workspace type.
    pub plain_fields: u32,
    /// An enum's variant names, as written.
    pub variants: Vec<String>,
    /// A static's declared type, as written.
    pub ty: String,
    /// Incoming holds edges folded to a count: how many types hold this one.
    /// Zero when they are all drawn.
    pub held_by: u32,
}

impl DataMark {
    /// A static is state no type holds — the chart's other kind of mark.
    pub fn is_static(&self) -> bool {
        self.kind == ItemKind::Static
    }

    /// Where it is written: `src/views/codemap/model.rs:278`.
    pub fn locator(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }
}

/// One holding relation, placed on the chart's anchors. The edge is drawn from
/// the held type to its holder, so the arrowhead rests on the holder — the way
/// a shape change travels.
#[derive(Clone, PartialEq, Debug)]
pub struct Hold {
    pub held: Anchor,
    pub holder: Anchor,
    pub kind: HoldKind,
    /// The strongest wrapper on the walk, in its own word. Empty for a plain
    /// hold, which needs none.
    pub via: String,
    /// Fields drawing this edge.
    pub fields: u32,
    /// Drawn at rest. A folded edge stays in the set and inks in the moment the
    /// reader hovers either of its ends.
    pub rest: bool,
}

impl Hold {
    pub fn key(&self) -> String {
        format!("{:?}>{:?}:{:?}:{}", self.held, self.holder, self.kind, self.via)
    }
}

/// One reference tie between two types, summed. The arrowhead rests on the
/// user, as it does at every other altitude.
#[derive(Clone, PartialEq, Debug)]
pub struct Tie {
    pub def: Anchor,
    pub user: Anchor,
    pub count: u32,
    /// Drawn at rest under the current reading.
    pub rest: bool,
    /// Heavy enough among the resting ties to carry its count on the paper.
    pub labeled: bool,
}

impl Tie {
    pub fn key(&self) -> String {
        format!("{:?}~{:?}", self.def, self.user)
    }
}

/// Everything one build of the data chart reads out of the survey.
#[derive(Clone, PartialEq, Debug)]
pub struct DataModel {
    pub frames: Vec<Frame>,
    /// Drawn marks, in the survey's (file, source) order.
    pub marks: Vec<DataMark>,
    pub holds: Vec<Hold>,
    pub ties: Vec<Tie>,
    /// More than one crate in the survey: crate frames earn their names.
    pub multi_crate: bool,
    // ---- Facts for the cartouche. ----
    pub structs: usize,
    pub enums: usize,
    /// Statics, plus every drawn type no other type holds.
    pub roots: usize,
    /// Drawn types whose defining file changed.
    pub changed: usize,
    /// Top-level modules holding a changed type, in name order.
    pub changed_modules: Vec<String>,
    /// Holds edges whose held end is a workspace trait. Traits get no mark of
    /// their own in v1, so these have nowhere to land.
    pub trait_holds: usize,
}

/// What the cartouche and the legend state about the survey. Small enough to
/// hand the furniture without carrying the whole chart along with it.
#[derive(Clone, PartialEq, Debug)]
pub struct DataFacts {
    pub structs: usize,
    pub enums: usize,
    pub roots: usize,
    pub changed: usize,
    pub changed_modules: Vec<String>,
    pub trait_holds: usize,
    /// Names the survey could not resolve, straight from the wire model.
    pub unresolved: u32,
}

impl DataModel {
    /// The facts, lifted off the model for the furniture that states them.
    pub fn facts(&self, unresolved: u32) -> DataFacts {
        DataFacts {
            structs: self.structs,
            enums: self.enums,
            roots: self.roots,
            changed: self.changed,
            changed_modules: self.changed_modules.clone(),
            trait_holds: self.trait_holds,
            unresolved,
        }
    }
}

/// A type the chart can draw as a block. A static is always drawn, whatever its
/// visibility: it is state no type holds, and the process has no other root.
/// Everything else must declare a door — privacy is a fold, not a mark.
fn drawable(mark: &ItemMark) -> bool {
    match mark.kind {
        ItemKind::Static => true,
        ItemKind::Struct | ItemKind::Enum | ItemKind::Union => mark.vis != Vis::Private,
        _ => false,
    }
}

/// A type the chart could draw or fold: everything the data walk starts from.
fn charted(mark: &ItemMark) -> bool {
    matches!(
        mark.kind,
        ItemKind::Static | ItemKind::Struct | ItemKind::Enum | ItemKind::Union
    )
}

/// The part of a path below the crate's source root — `src/views/star.rs`
/// becomes `views/star.rs`, wherever in the workspace the crate itself sits.
/// A path with no source root of its own keeps its last segment.
fn source_rest(path: &str) -> &str {
    let segments: Vec<&str> = path.split('/').collect();
    let root = segments
        .iter()
        .enumerate()
        .rev()
        .find(|(i, seg)| {
            *i + 1 < segments.len() && matches!(**seg, "src" | "tests" | "benches" | "examples")
        })
        .map(|(i, _)| i);
    match root {
        Some(i) => {
            let cut: usize = segments[..=i].iter().map(|s| s.len() + 1).sum();
            &path[cut..]
        }
        None => path.rsplit('/').next().unwrap_or(path),
    }
}

/// The top-level module a file's types are framed in — the first path segment
/// under the crate's source root, which is exactly how rust names the module.
/// `src/views/codemap/map.rs` is `mod views`; `src/api.rs` is `mod api`; the
/// crate root itself (`main.rs`, `lib.rs`) has no module and frames in the
/// crate.
pub fn module_of(path: &str) -> Option<&str> {
    let rest = source_rest(path);
    match rest.split_once('/') {
        Some((first, _)) => Some(first),
        None => {
            let stem = rest.strip_suffix(".rs").unwrap_or(rest);
            (!matches!(stem, "main" | "lib" | "mod" | "build")).then_some(stem)
        }
    }
}

/// How loudly a type asks for a block of its own: how many holding edges touch
/// it, how much of the workspace names it, and whether this epoch touched its
/// file. Only used once the chart is over budget.
fn interest(mark: &ItemMark, degree: u32, changed: bool) -> u32 {
    degree + mark.fan_in + if changed { 2 } else { 0 }
}

impl DataModel {
    pub fn build(graph: &CodeGraph, ref_dir: RefDir) -> Self {
        let changed_file: Vec<bool> = graph.files.iter().map(|f| f.changed).collect();
        let file_changed = |file: u32| changed_file.get(file as usize).copied().unwrap_or(false);

        // ---- Which marks are drawn, and which fold. ------------------------
        let mut degree = vec![0u32; graph.items.len()];
        for edge in &graph.holds {
            if let Some(d) = degree.get_mut(edge.from as usize) {
                *d += 1;
            }
            if let Some(d) = degree.get_mut(edge.to as usize) {
                *d += 1;
            }
        }

        let mut drawn: Vec<u32> = Vec::new();
        let mut private: Vec<u32> = Vec::new();
        for (i, mark) in graph.items.iter().enumerate() {
            if !charted(mark) {
                continue;
            }
            if drawable(mark) {
                drawn.push(i as u32);
            } else {
                private.push(i as u32);
            }
        }

        // Over budget, the quietest types fold to their frame's counted row.
        // Statics never fold: eleven of them are the whole session's state.
        let mut folded: HashSet<u32> = HashSet::new();
        if drawn.len() > MARK_BUDGET {
            let mut ranked: Vec<u32> = drawn
                .iter()
                .copied()
                .filter(|&m| graph.items[m as usize].kind != ItemKind::Static)
                .collect();
            ranked.sort_by_key(|&m| {
                let mark = &graph.items[m as usize];
                (
                    std::cmp::Reverse(interest(
                        mark,
                        degree[m as usize],
                        file_changed(mark.file),
                    )),
                    mark.file,
                    mark.line,
                )
            });
            let statics = drawn.len() - ranked.len();
            folded = ranked
                .into_iter()
                .skip(MARK_BUDGET.saturating_sub(statics))
                .collect();
            drawn.retain(|m| !folded.contains(m));
        }

        // ---- Frames: one per crate, one per top-level module inside it. ----
        let file_key: Vec<(String, Option<String>)> = graph
            .files
            .iter()
            .map(|f| {
                (
                    f.krate.clone(),
                    module_of(&f.path).map(str::to_string),
                )
            })
            .collect();
        let key_of = |mark: u32| -> Option<&(String, Option<String>)> {
            file_key.get(graph.items[mark as usize].file as usize)
        };

        let mut keys: Vec<(String, Option<String>)> = drawn
            .iter()
            .chain(folded.iter())
            .chain(private.iter())
            .filter_map(|&m| key_of(m).cloned())
            .collect();
        keys.sort();
        keys.dedup();
        let mut crates: Vec<String> = keys.iter().map(|(krate, _)| krate.clone()).collect();
        crates.dedup();

        let mut frames: Vec<Frame> = Vec::new();
        let mut frame_index: HashMap<(String, Option<String>), u32> = HashMap::new();
        // Crate frames first, so a module frame always has a parent to sit in.
        for krate in &crates {
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: krate.clone(),
                module: None,
                parent: None,
                marks: Vec::new(),
                private: 0,
                more: 0,
            });
            frame_index.insert((krate.clone(), None), id);
        }
        for key in &keys {
            if key.1.is_none() {
                continue;
            }
            let parent = frame_index.get(&(key.0.clone(), None)).copied();
            let id = frames.len() as u32;
            frames.push(Frame {
                id,
                krate: key.0.clone(),
                module: key.1.clone(),
                parent,
                marks: Vec::new(),
                private: 0,
                more: 0,
            });
            frame_index.insert(key.clone(), id);
        }
        let frame_of = |mark: u32| -> Option<u32> {
            key_of(mark).and_then(|key| frame_index.get(key).copied())
        };

        let mut anchor_of: Vec<Option<Anchor>> = vec![None; graph.items.len()];
        for &m in &drawn {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].marks.push(m);
                anchor_of[m as usize] = Some(Anchor::Mark(m));
            }
        }
        for &m in &private {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].private += 1;
                anchor_of[m as usize] = Some(Anchor::Private(frame));
            }
        }
        let mut folded_sorted: Vec<u32> = folded.iter().copied().collect();
        folded_sorted.sort_unstable();
        for m in folded_sorted {
            if let Some(frame) = frame_of(m) {
                frames[frame as usize].more += 1;
                anchor_of[m as usize] = Some(Anchor::More(frame));
            }
        }

        // ---- Holds: every edge, landed on an anchor. -----------------------
        let mut trait_holds = 0usize;
        let mut acc: HashMap<(Anchor, Anchor, HoldKind, String), u32> = HashMap::new();
        for edge in &graph.holds {
            let (holder, held) = (
                anchor_of.get(edge.from as usize).copied().flatten(),
                anchor_of.get(edge.to as usize).copied().flatten(),
            );
            if held.is_none()
                && graph
                    .items
                    .get(edge.to as usize)
                    .is_some_and(|m| m.kind == ItemKind::Trait)
            {
                trait_holds += 1;
            }
            let (Some(holder), Some(held)) = (holder, held) else {
                continue;
            };
            // A type holding itself, or two folded types holding each other,
            // draws nothing: the fold row already counts them both.
            if holder == held {
                continue;
            }
            *acc.entry((held, holder, edge.kind, edge.via.clone()))
                .or_default() += edge.fields.len() as u32;
        }
        let mut holds: Vec<Hold> = acc
            .into_iter()
            .map(|((held, holder, kind, via), fields)| Hold {
                held,
                holder,
                kind,
                via,
                fields,
                rest: true,
            })
            .collect();
        holds.sort_by(|a, b| {
            (a.held, a.holder, a.kind as u8, &a.via).cmp(&(b.held, b.holder, b.kind as u8, &b.via))
        });

        // Who holds what. The arrowhead rests on the holder, so a type's fan-in
        // is the set of edges leaving it — and a type more than three drawn
        // types hold folds them all to a count on its own mark, where hovering
        // either end inks them back in.
        let mut fan_in: HashMap<Anchor, HashSet<Anchor>> = HashMap::new();
        for hold in &holds {
            fan_in.entry(hold.held).or_default().insert(hold.holder);
        }
        // Only a drawn mark may fold its fan-in: it has a foot to say `held by
        // n types` on. A counted fold row has no room for a second count, so
        // the edges landing on it all stay drawn.
        let folded_fan: HashMap<Anchor, u32> = fan_in
            .iter()
            .filter(|(anchor, holders)| {
                matches!(anchor, Anchor::Mark(_)) && holders.len() > HELD_CAP
            })
            .map(|(anchor, holders)| (*anchor, holders.len() as u32))
            .collect();
        for hold in &mut holds {
            hold.rest = !folded_fan.contains_key(&hold.held);
        }

        // ---- The marks themselves. -----------------------------------------
        // Holding fields, quoted per holder. `graph.holds` arrives sorted by
        // (from, to, kind, via), so the rows are grouped by what they hold and
        // the same survey always writes the same block.
        let drawn_set: HashSet<u32> = drawn.iter().copied().collect();
        let mut fields_of: HashMap<u32, Vec<FieldRow>> = HashMap::new();
        for edge in &graph.holds {
            if !drawn_set.contains(&edge.from) {
                continue;
            }
            if graph.items[edge.from as usize].kind == ItemKind::Static {
                // A static quotes its declared type whole, right under its
                // name; a field row would say the same thing twice.
                continue;
            }
            let target = graph
                .items
                .get(edge.to as usize)
                .map(|m| m.name.clone())
                .unwrap_or_default();
            let rows = fields_of.entry(edge.from).or_default();
            for (name, decl) in &edge.fields {
                // One field can reach two workspace types (`Arc<(A, B)>`) and
                // is then written on both edges. It is still one field.
                if rows.iter().any(|r| &r.name == name && &r.decl == decl) {
                    continue;
                }
                rows.push(FieldRow {
                    name: name.clone(),
                    decl: decl.clone(),
                    target: target.clone(),
                });
            }
        }

        let marks: Vec<DataMark> = drawn
            .iter()
            .filter_map(|&id| {
                let mark = &graph.items[id as usize];
                let frame = frame_of(id)?;
                let file = graph.files.get(mark.file as usize)?;
                let mut fields = fields_of.remove(&id).unwrap_or_default();
                let more_fields = fields.len().saturating_sub(FIELD_CAP) as u32;
                fields.truncate(FIELD_CAP);
                Some(DataMark {
                    id,
                    frame,
                    kind: mark.kind,
                    vis: mark.vis,
                    name: mark.name.clone(),
                    label: mark.label.clone(),
                    path: file.path.clone(),
                    line: mark.line,
                    changed: file.changed,
                    fields,
                    more_fields,
                    plain_fields: mark.plain_fields,
                    variants: mark.variants.clone(),
                    ty: mark.ty.clone(),
                    held_by: folded_fan.get(&Anchor::Mark(id)).copied().unwrap_or(0),
                })
            })
            .collect();

        // ---- Reference ties, at type precision. ----------------------------
        let containment = Containment::build(graph);
        let is_type = |m: u32| -> bool {
            drawn_set.contains(&m)
                && matches!(
                    graph.items[m as usize].kind,
                    ItemKind::Struct | ItemKind::Enum | ItemKind::Union
                )
        };
        let mut tie_acc: HashMap<(u32, u32), u32> = HashMap::new();
        for edge in &graph.item_edges {
            let (Some(from), Some(to)) = (edge.from, edge.to) else {
                continue;
            };
            let (user, def) = (containment.root(from), containment.root(to));
            if user == def || !is_type(user) || !is_type(def) {
                continue;
            }
            *tie_acc.entry((def, user)).or_default() += edge.count;
        }
        let mut ties: Vec<Tie> = tie_acc
            .into_iter()
            .map(|((def, user), count)| Tie {
                def: Anchor::Mark(def),
                user: Anchor::Mark(user),
                count,
                rest: true,
                labeled: false,
            })
            .collect();
        ties.sort_by(|a, b| {
            (a.def, a.user)
                .cmp(&(b.def, b.user))
                .then(b.count.cmp(&a.count))
        });

        // Which ties rest on the paper. Direction alone cannot thin the chart —
        // every tie is one type's use and another's users — so each reading
        // anchors on the types themselves and hands the rest back on hover.
        if let Some(cap) = ref_dir.per_territory().map(|c| c.min(TIES_PER_MARK)) {
            let mut by_anchor: HashMap<Anchor, Vec<usize>> = HashMap::new();
            for (i, tie) in ties.iter().enumerate() {
                let anchor = match ref_dir {
                    RefDir::UsedBy => tie.def,
                    _ => tie.user,
                };
                by_anchor.entry(anchor).or_default().push(i);
            }
            let resting: HashSet<usize> = by_anchor
                .into_values()
                .flat_map(|mut idx| {
                    idx.sort_unstable_by_key(|&i| (std::cmp::Reverse(ties[i].count), i));
                    idx.into_iter().take(cap)
                })
                .collect();
            for (i, tie) in ties.iter_mut().enumerate() {
                tie.rest = resting.contains(&i);
            }
        }
        let label_bar = {
            let mut counts: Vec<u32> = ties.iter().filter(|t| t.rest).map(|t| t.count).collect();
            counts.sort_unstable_by(|a, b| b.cmp(a));
            counts.get(TIE_LABELS).copied().unwrap_or(0).max(2)
        };
        for tie in &mut ties {
            tie.labeled = tie.rest && tie.count > label_bar;
        }

        // ---- Facts. ---------------------------------------------------------
        let structs = marks
            .iter()
            .filter(|m| matches!(m.kind, ItemKind::Struct | ItemKind::Union))
            .count();
        let enums = marks
            .iter()
            .filter(|m| m.kind == ItemKind::Enum)
            .count();
        // A root is state nothing else holds: every static, and every type no
        // other type has a field of.
        let roots = marks
            .iter()
            .filter(|m| !fan_in.contains_key(&Anchor::Mark(m.id)))
            .count();
        let changed = marks.iter().filter(|m| m.changed).count();
        let mut changed_modules: Vec<String> = marks
            .iter()
            .filter(|m| m.changed)
            .map(|m| {
                let frame = &frames[m.frame as usize];
                frame.module.clone().unwrap_or_else(|| frame.krate.clone())
            })
            .collect();
        changed_modules.sort();
        changed_modules.dedup();

        let multi_crate = crates.len() > 1;
        Self {
            frames,
            marks,
            holds,
            ties,
            multi_crate,
            structs,
            enums,
            roots,
            changed,
            changed_modules,
            trait_holds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{FileInfo, HoldEdge, ItemEdge};

    fn file(id: u32, path: &str, changed: bool) -> FileInfo {
        FileInfo {
            id,
            path: path.to_string(),
            krate: "slopify".to_string(),
            changed,
            lines: 100,
            items: 2,
            fns: 0,
            types: 2,
            traits: 0,
            refs_in_files: 0,
            refs_out_files: 0,
        }
    }

    fn mark(id: u32, file: u32, name: &str, kind: ItemKind, vis: Vis) -> ItemMark {
        ItemMark {
            id,
            file,
            local: id,
            name: name.to_string(),
            label: name.to_string(),
            kind,
            vis,
            line: id + 1,
            parent: None,
            fan_in: 0,
            impls: Vec::new(),
            plain_fields: 0,
            variants: Vec::new(),
            ty: String::new(),
        }
    }

    #[test]
    fn a_module_frame_is_the_first_segment_under_src() {
        assert_eq!(module_of("src/views/codemap/map.rs"), Some("views"));
        assert_eq!(module_of("src/api.rs"), Some("api"));
        assert_eq!(module_of("src/main.rs"), None);
        assert_eq!(module_of("crates/engine/src/lib.rs"), None);
        assert_eq!(module_of("crates/engine/src/parse/lex.rs"), Some("parse"));
    }

    /// `Wire` (pub, in `mod api`) is held by `Index` (pub, in `mod analyze`)
    /// and by `Hidden` (private, same module).
    fn graph() -> CodeGraph {
        CodeGraph {
            files: vec![
                file(0, "src/api.rs", false),
                file(1, "src/analyze/code.rs", true),
            ],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Wire", ItemKind::Struct, Vis::Pub),
                mark(1, 1, "Index", ItemKind::Struct, Vis::Pub),
                mark(2, 1, "Hidden", ItemKind::Struct, Vis::Private),
                mark(3, 1, "CACHE", ItemKind::Static, Vis::Private),
            ],
            item_edges: vec![ItemEdge {
                from_file: 1,
                from: Some(1),
                to_file: 0,
                to: Some(0),
                count: 4,
            }],
            holds: vec![
                HoldEdge {
                    from: 1,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("wire".into(), "Wire".into())],
                },
                HoldEdge {
                    from: 2,
                    to: 0,
                    kind: HoldKind::Owns,
                    via: String::new(),
                    fields: vec![("wire".into(), "Wire".into())],
                },
                HoldEdge {
                    from: 3,
                    to: 1,
                    kind: HoldKind::Shares,
                    via: "Arc".into(),
                    fields: vec![("CACHE".into(), "OnceCell<Arc<Index>>".into())],
                },
            ],
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    #[test]
    fn privacy_folds_a_type_and_keeps_its_edge() {
        let model = DataModel::build(&graph(), RefDir::Uses);
        // Two module frames under one crate frame; the crate frame is empty
        // but holds them.
        assert_eq!(model.frames.len(), 3);
        assert!(!model.multi_crate);
        let api = model
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some("api"))
            .unwrap();
        assert_eq!(api.marks.len(), 1);
        let analyze = model
            .frames
            .iter()
            .find(|f| f.module.as_deref() == Some("analyze"))
            .unwrap();
        // The static is drawn although it is private; the struct is not.
        assert_eq!(analyze.marks.len(), 2);
        assert_eq!(analyze.private, 1);
        // The private holder's edge lands on the frame's counted row.
        assert!(
            model
                .holds
                .iter()
                .any(|h| h.holder == Anchor::Private(analyze.id)
                    && h.held == Anchor::Mark(0))
        );
        // Only the static is a root here: nothing can hold a static, and both
        // types are held — `Wire` by `Index`, `Index` by the static itself.
        assert_eq!(model.roots, 1);
        assert_eq!(model.structs, 2);
    }

    #[test]
    fn a_static_quotes_its_type_instead_of_a_field_row() {
        let model = DataModel::build(&graph(), RefDir::Uses);
        let cache = model.marks.iter().find(|m| m.name == "CACHE").unwrap();
        assert!(cache.is_static());
        assert!(cache.fields.is_empty());
        let index = model.marks.iter().find(|m| m.name == "Index").unwrap();
        assert_eq!(index.fields.len(), 1);
        assert_eq!(index.fields[0].target, "Wire");
        assert!(index.changed);
        assert_eq!(model.changed_modules, vec!["analyze".to_string()]);
    }

    #[test]
    fn ties_land_on_types_only() {
        let model = DataModel::build(&graph(), RefDir::Both);
        assert_eq!(model.ties.len(), 1);
        assert_eq!(model.ties[0].def, Anchor::Mark(0));
        assert_eq!(model.ties[0].user, Anchor::Mark(1));
        assert_eq!(model.ties[0].count, 4);
    }
}
