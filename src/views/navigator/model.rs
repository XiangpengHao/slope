//! The navigator's reading of the survey: every item is a mark, and the only
//! thing a position ever says is relation.
//!
//! Pure functions over the wire model — no layout and no rendering. The survey
//! ships everything it found; this reading keeps all of it, methods and private
//! items included, because a question about one mark has to be answerable
//! whatever door the mark stands behind. Nothing is ever dropped for a count:
//! what a page leaves out is what the question did not ask, never the tail of a
//! list that ran past a budget.
//!
//! Two edge families, the same two the surface chart reads. **Solid** is
//! interface coupling, aggregated per pair out of the survey's holds; a pair
//! that lost one edge and gained another was *rewritten*, not dropped. **Uses**
//! is implementation coupling, the resolved references at mark precision,
//! summed per pair.

use std::collections::HashMap;

use crate::api::{CodeGraph, Delta, HoldEvent, HoldKind, ItemKind, Vis};
use crate::views::surface::model::module_path;

/// How many hops out a focus page reads its blast radius.
pub const FOCUS_HOPS: usize = 6;
/// How many hops out the diff agenda reads.
pub const AGENDA_HOPS: usize = 8;
/// How many type-ahead hits the bar offers at once.
pub const MAX_HITS: usize = 18;
/// A uses count past this adds nothing to a mark's degree: a hundred calls and
/// three calls say the same thing about how central a mark is.
const USES_WEIGHT_CAP: u32 = 3;

/// Rust's own word for what a mark is.
pub fn kind_word(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Fn => "fn",
        ItemKind::Struct => "struct",
        ItemKind::Enum => "enum",
        ItemKind::Union => "union",
        ItemKind::Trait => "trait",
        ItemKind::TypeAlias => "type",
        ItemKind::Const => "const",
        ItemKind::Static => "static",
        ItemKind::Macro => "macro",
        ItemKind::Mod => "mod",
        ItemKind::Impl => "impl",
    }
}

/// One mark on any page: an item the working copy has, or a ghost the base had.
/// The quotation itself is not here — only the focused mark is ever quoted, and
/// it is quoted straight from the survey.
#[derive(Clone, PartialEq, Debug)]
pub struct NavItem {
    pub id: u32,
    pub name: String,
    /// The label a URL selects it by, matching [`crate::api::ItemMark::label`].
    pub label: String,
    pub kind: ItemKind,
    pub vis: Vis,
    /// The file that declares it, relative to the workspace root. A ghost's is
    /// the file that declared it at the base.
    pub path: String,
    pub line: u32,
    /// The module it is written in, in rust's own words — a label on the block,
    /// never a place on the page.
    pub module: String,
    pub delta: Delta,
    /// The base had it and the working copy does not.
    pub ghost: bool,
    /// What the diff did to its own rows, for the agenda's delta notes.
    pub fields_added: usize,
    pub fields_removed: usize,
    pub variants_added: usize,
    pub variants_removed: usize,
    pub methods_added: usize,
    pub methods_removed: usize,
}

impl NavItem {
    /// The declaration's keywords, as rust writes them.
    pub fn keyword(&self) -> String {
        let vis = match self.vis {
            Vis::Pub => "pub ",
            Vis::Crate => "pub(crate) ",
            Vis::Private => "",
        };
        format!("{vis}{}", kind_word(self.kind))
    }

    /// The bare kind word, for a chip too small to carry the visibility.
    pub fn word(&self) -> &'static str {
        kind_word(self.kind)
    }

    /// Inked in the palette's other type color. An enum is a sum of its
    /// variants and a function is a sum of its returns: neither is a product
    /// type, and the color is the second reading of the keyword in front.
    pub fn is_sum(&self) -> bool {
        matches!(self.kind, ItemKind::Enum | ItemKind::Fn)
    }

    /// A contract nothing can hold: it is named in signatures instead.
    pub fn is_contract(&self) -> bool {
        matches!(
            self.kind,
            ItemKind::Fn | ItemKind::Const | ItemKind::Static | ItemKind::TypeAlias
        )
    }

    /// The diff's letter: added, changed, or dropped.
    pub fn letter(&self) -> Option<&'static str> {
        if self.ghost {
            Some("D")
        } else {
            match self.delta {
                Delta::Added => Some("A"),
                Delta::Changed => Some("M"),
                Delta::Same => None,
            }
        }
    }

    /// The diff touched this declaration.
    pub fn touched(&self) -> bool {
        self.ghost || self.delta != Delta::Same
    }

    /// What the diff did to its rows, in the diff's own counts. Empty when the
    /// declaration changed in ways its rows do not show.
    pub fn note(&self) -> Option<String> {
        let parts = [
            (self.fields_added, "+", "fields"),
            (self.fields_removed, "−", "fields"),
            (self.variants_added, "+", "variants"),
            (self.variants_removed, "−", "variants"),
            (self.methods_added, "+", "fns"),
            (self.methods_removed, "−", "fns"),
        ];
        let words: Vec<String> = parts
            .iter()
            .filter(|(n, _, _)| *n > 0)
            .map(|(n, sign, what)| format!("{sign}{n} {what}"))
            .collect();
        (!words.is_empty()).then(|| words.join(" · "))
    }
}

/// One interface-coupling pair: `from`'s own published surface names `to`, so a
/// change at `to` forces a change at `from`. Every holds edge between the two
/// arrives on this one pair, which is what lets the diff read as *rewritten*
/// rather than as one removal beside one addition.
#[derive(Clone, PartialEq, Debug)]
pub struct SolidPair {
    pub from: u32,
    pub to: u32,
    /// `impl Trait for Type` — the one solid line no row writes.
    pub implements: bool,
    /// The holder only views what the other end owns.
    pub borrows: bool,
    /// The first wrapper word the walk met (`Arc`, `Signal`, `&mut`, …).
    pub via: Option<String>,
    /// The first row that draws the pair, by the name it is written under.
    pub field: Option<String>,
    /// Drawn by `from`'s methods rather than its fields: its API names the
    /// other end instead of keeping one.
    pub from_method: bool,
    pub added: bool,
    pub removed: bool,
}

impl SolidPair {
    /// The pair against the base. A pair that lost an edge and gained another
    /// is not a removal: the relation stands, written differently.
    pub fn event(&self) -> Option<HoldEvent> {
        match (self.added, self.removed) {
            (false, true) => Some(HoldEvent::Removed),
            (true, _) => Some(HoldEvent::Added),
            _ => None,
        }
    }

    /// The one word the wire carries. The diff speaks first — a reviewer came
    /// for it — then the impl, then the wrapper the walk met, then the row that
    /// wrote it.
    pub fn word(&self) -> String {
        if self.added && self.removed {
            return "rewritten".to_string();
        }
        if self.added {
            return "added".to_string();
        }
        if self.removed {
            return "removed".to_string();
        }
        if self.implements {
            return "implements".to_string();
        }
        if let Some(via) = &self.via {
            return via.clone();
        }
        if self.borrows {
            return "&".to_string();
        }
        match &self.field {
            Some(name) if !name.is_empty() => name.clone(),
            _ => "owns".to_string(),
        }
    }
}

/// One implementation-coupling pair: `from`'s body names `to`, summed over
/// every reference the survey resolved between them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct UsesPair {
    pub from: u32,
    pub to: u32,
    pub count: u32,
}

impl UsesPair {
    /// The count in words, for the wire.
    pub fn words(&self) -> String {
        if self.count == 1 {
            "1 ref".to_string()
        } else {
            format!("{} refs", self.count)
        }
    }
}

/// One step of a path the board threads between two pins: the mark it lands
/// on, and the words that got there from the step before.
#[derive(Clone, PartialEq, Debug)]
pub struct PathStep {
    pub id: u32,
    /// The direction word, arrow included, as it reads left to right.
    pub word: String,
    /// The step is the diff's: the words wear the flare.
    pub flare: bool,
}

/// The whole model, built once per survey.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct NavModel {
    /// Every mark, indexed by its own id: the survey's items, then its ghosts.
    pub items: Vec<NavItem>,
    pub solid: Vec<SolidPair>,
    pub uses: Vec<UsesPair>,
    /// Pairs leaving a mark — what its own surface names.
    solid_out: Vec<Vec<u32>>,
    /// Pairs landing on a mark — what names it.
    solid_in: Vec<Vec<u32>>,
    uses_out: Vec<Vec<u32>>,
    uses_in: Vec<Vec<u32>>,
    /// How much of the workspace touches each mark, both families counted.
    degree: Vec<u32>,
    /// Every mark the diff touched, in id order.
    pub touched: Vec<u32>,
    /// The tally the agenda's band header carries.
    pub tally: String,
    /// Name to mark, first declaration wins — what makes a quoted type name a
    /// bold run.
    names: HashMap<String, u32>,
    /// (file path, label) to mark, for resolving a route.
    by_route: HashMap<(String, String), u32>,
}

impl NavModel {
    pub fn item(&self, id: u32) -> Option<&NavItem> {
        self.items.get(id as usize)
    }

    /// The mark a route names, if this survey has it.
    pub fn find(&self, path: &str, label: &str) -> Option<u32> {
        self.by_route
            .get(&(path.to_string(), label.to_string()))
            .copied()
    }

    /// The mark a name resolves to, for the bold run inside a quotation.
    pub fn named(&self, name: &str) -> Option<u32> {
        self.names.get(name).copied()
    }

    pub fn degree(&self, id: u32) -> u32 {
        self.degree.get(id as usize).copied().unwrap_or(0)
    }

    /// Build the model out of the survey.
    pub fn build(graph: &CodeGraph) -> Self {
        let multi_crate = {
            let mut krates: Vec<&str> = graph.files.iter().map(|f| f.krate.as_str()).collect();
            krates.sort_unstable();
            krates.dedup();
            krates.len() > 1
        };
        let module = |krate: &str, path: &str| -> String {
            let dirs = module_path(path).join("::");
            match (multi_crate, dirs.is_empty()) {
                (false, true) => "crate root".to_string(),
                (false, false) => dirs,
                (true, true) => krate.to_string(),
                (true, false) => format!("{krate}::{dirs}"),
            }
        };

        let mut items: Vec<NavItem> = Vec::with_capacity(graph.items.len() + graph.ghosts.len());
        for mark in &graph.items {
            let file = graph.files.get(mark.file as usize);
            let path = file.map(|f| f.path.clone()).unwrap_or_default();
            let krate = file.map(|f| f.krate.as_str()).unwrap_or_default();
            items.push(NavItem {
                id: mark.id,
                name: mark.name.clone(),
                label: mark.label.clone(),
                kind: mark.kind,
                vis: mark.vis,
                module: module(krate, &path),
                path,
                line: mark.line,
                delta: mark.delta,
                ghost: false,
                fields_added: mark.fields_added.len(),
                fields_removed: mark.fields_removed.len(),
                variants_added: mark.variants_added.len(),
                variants_removed: mark.variants_removed.len(),
                methods_added: mark.methods_added.len(),
                methods_removed: mark.methods_removed.len(),
            });
        }
        // A ghost is a mark like any other: quoted from the base, focusable,
        // and reachable from whatever still names it.
        for ghost in &graph.ghosts {
            items.push(NavItem {
                id: ghost.id,
                name: ghost.name.clone(),
                label: ghost.name.clone(),
                kind: ghost.kind,
                vis: ghost.vis,
                module: module(&ghost.krate, &ghost.path),
                path: ghost.path.clone(),
                line: ghost.line,
                delta: Delta::Same,
                ghost: true,
                fields_added: 0,
                fields_removed: 0,
                variants_added: 0,
                variants_removed: 0,
                methods_added: 0,
                methods_removed: 0,
            });
        }
        // Ids index the vector directly, so a survey that ever handed out a
        // sparse id space would seat its marks wrong. Seat by id instead of
        // trusting the order.
        items.sort_by_key(|it| it.id);
        let known = |id: u32| (id as usize) < items.len();

        // The solid family, aggregated per pair. Self-edges say nothing about
        // relation: a linked structure holding itself is a fact of its own
        // block, not a line to draw.
        let mut solid: Vec<SolidPair> = Vec::new();
        let mut pair_at: HashMap<(u32, u32), usize> = HashMap::new();
        for hold in &graph.holds {
            if hold.from == hold.to || !known(hold.from) || !known(hold.to) {
                continue;
            }
            let at = *pair_at.entry((hold.from, hold.to)).or_insert_with(|| {
                solid.push(SolidPair {
                    from: hold.from,
                    to: hold.to,
                    implements: false,
                    borrows: false,
                    via: None,
                    field: None,
                    from_method: false,
                    added: false,
                    removed: false,
                });
                solid.len() - 1
            });
            let pair = &mut solid[at];
            pair.implements |= hold.kind == HoldKind::Implements;
            pair.borrows |= hold.kind == HoldKind::Borrows;
            if pair.via.is_none() && !hold.via.is_empty() {
                pair.via = Some(hold.via.clone());
            }
            if pair.field.is_none() {
                pair.field = hold.fields.first().map(|(name, _)| name.clone());
            }
            pair.from_method |= hold.from_method;
            pair.added |= hold.event == Some(HoldEvent::Added);
            pair.removed |= hold.event == Some(HoldEvent::Removed);
        }

        // The uses family: every reference the survey placed at mark precision,
        // whichever file it was written in. Which file a reference sits in says
        // nothing about whether one contract leans on another.
        let mut summed: HashMap<(u32, u32), u32> = HashMap::new();
        for edge in &graph.item_edges {
            let (Some(from), Some(to)) = (edge.from, edge.to) else {
                continue;
            };
            if from == to || !known(from) || !known(to) {
                continue;
            }
            *summed.entry((from, to)).or_insert(0) += edge.count;
        }
        for local in &graph.local_refs {
            if local.from == local.to || !known(local.from) || !known(local.to) {
                continue;
            }
            *summed.entry((local.from, local.to)).or_insert(0) += local.count;
        }
        let mut uses: Vec<UsesPair> = summed
            .into_iter()
            .map(|((from, to), count)| UsesPair { from, to, count })
            .collect();
        // A hash map hands its pairs back in no order at all, and every page
        // that reads them has to draw the same thing twice running.
        uses.sort_by_key(|u| (u.from, u.to));

        let mut model = NavModel {
            solid_out: vec![Vec::new(); items.len()],
            solid_in: vec![Vec::new(); items.len()],
            uses_out: vec![Vec::new(); items.len()],
            uses_in: vec![Vec::new(); items.len()],
            degree: vec![0; items.len()],
            names: HashMap::new(),
            by_route: HashMap::new(),
            items,
            solid,
            uses,
            touched: Vec::new(),
            tally: String::new(),
        };
        for (at, pair) in model.solid.iter().enumerate() {
            model.solid_out[pair.from as usize].push(at as u32);
            model.solid_in[pair.to as usize].push(at as u32);
        }
        for (at, pair) in model.uses.iter().enumerate() {
            model.uses_out[pair.from as usize].push(at as u32);
            model.uses_in[pair.to as usize].push(at as u32);
        }
        for id in 0..model.items.len() {
            let weigh = |pairs: &[u32], all: &[UsesPair]| -> u32 {
                pairs
                    .iter()
                    .map(|&at| all[at as usize].count.min(USES_WEIGHT_CAP))
                    .sum()
            };
            model.degree[id] = model.solid_out[id].len() as u32
                + model.solid_in[id].len() as u32
                + weigh(&model.uses_out[id], &model.uses)
                + weigh(&model.uses_in[id], &model.uses);
        }
        for item in &model.items {
            model
                .names
                .entry(item.name.clone())
                .or_insert_with(|| item.id);
            model
                .by_route
                .entry((item.path.clone(), item.label.clone()))
                .or_insert_with(|| item.id);
        }
        model.touched = model
            .items
            .iter()
            .filter(|it| it.touched())
            .map(|it| it.id)
            .collect();
        let added = model
            .items
            .iter()
            .filter(|it| !it.ghost && it.delta == Delta::Added)
            .count();
        let changed = model
            .items
            .iter()
            .filter(|it| it.delta == Delta::Changed)
            .count();
        let dropped = graph.ghosts.len();
        model.tally = format!("{added} added · {changed} changed · {dropped} removed");
        model
    }

    /// The pairs whose own surface names this mark, and the ones it names.
    pub fn solid_out(&self, id: u32) -> impl Iterator<Item = &SolidPair> {
        self.solid_out
            .get(id as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|&at| &self.solid[at as usize])
    }

    pub fn solid_in(&self, id: u32) -> impl Iterator<Item = &SolidPair> {
        self.solid_in
            .get(id as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|&at| &self.solid[at as usize])
    }

    pub fn uses_out(&self, id: u32) -> impl Iterator<Item = &UsesPair> {
        self.uses_out
            .get(id as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|&at| &self.uses[at as usize])
    }

    pub fn uses_in(&self, id: u32) -> impl Iterator<Item = &UsesPair> {
        self.uses_in
            .get(id as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|&at| &self.uses[at as usize])
    }

    /// The reach: everything a shape change at the seeds could travel to, tail
    /// to head, one BFS layer per hop. A removed pair is diff ink, not
    /// structure, so nothing travels along it.
    pub fn reach_layers(&self, seeds: &[u32], max_hops: usize) -> Vec<Vec<u32>> {
        let mut seen = vec![false; self.items.len()];
        for &seed in seeds {
            if let Some(slot) = seen.get_mut(seed as usize) {
                *slot = true;
            }
        }
        let mut frontier: Vec<u32> = seeds.to_vec();
        let mut layers: Vec<Vec<u32>> = Vec::new();
        for _ in 0..max_hops {
            if frontier.is_empty() {
                break;
            }
            let mut next: Vec<u32> = Vec::new();
            for &id in &frontier {
                for pair in self.solid_in(id) {
                    if pair.event() == Some(HoldEvent::Removed) {
                        continue;
                    }
                    if let Some(slot) = seen.get_mut(pair.from as usize)
                        && !*slot
                    {
                        *slot = true;
                        next.push(pair.from);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next.clone();
            layers.push(next);
        }
        layers
    }

    /// The shortest path between two marks, both families walked either way.
    /// Direction is not a wall here: the question is how two marks connect at
    /// all, and a reviewer reading a board does not care which end wrote it.
    pub fn path_between(&self, from: u32, to: u32) -> Option<Vec<PathStep>> {
        if from == to || self.items.is_empty() {
            return None;
        }
        // Both families, both ways, in the order the model holds them, so the
        // same two pins always thread the same path.
        let mut nbrs: Vec<Vec<(u32, String, bool)>> = vec![Vec::new(); self.items.len()];
        for pair in &self.solid {
            let word = pair.word();
            let flare = pair.event().is_some();
            nbrs[pair.to as usize].push((pair.from, format!("{word} →"), flare));
            nbrs[pair.from as usize].push((pair.to, format!("← {word}"), flare));
        }
        for pair in &self.uses {
            let words = pair.words();
            nbrs[pair.to as usize].push((pair.from, format!("{words} →"), false));
            nbrs[pair.from as usize].push((pair.to, format!("← {words}"), false));
        }
        let mut prev: Vec<Option<(u32, String, bool)>> = vec![None; self.items.len()];
        let mut reached = vec![false; self.items.len()];
        reached[from as usize] = true;
        let mut frontier = vec![from];
        while !frontier.is_empty() && !reached[to as usize] {
            let mut next = Vec::new();
            for &at in &frontier {
                for (other, word, flare) in &nbrs[at as usize] {
                    if reached[*other as usize] {
                        continue;
                    }
                    reached[*other as usize] = true;
                    prev[*other as usize] = Some((at, word.clone(), *flare));
                    next.push(*other);
                }
            }
            frontier = next;
        }
        if !reached[to as usize] {
            return None;
        }
        let mut chain: Vec<PathStep> = Vec::new();
        let mut cur = to;
        while cur != from {
            let (back, word, flare) = prev[cur as usize].clone()?;
            chain.push(PathStep {
                id: cur,
                word,
                flare,
            });
            cur = back;
        }
        chain.reverse();
        Some(chain)
    }

    /// Type-ahead over every mark: a prefix match first, then whatever the
    /// workspace leans on most.
    pub fn search(&self, query: &str) -> Vec<u32> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(i64, u32)> = Vec::new();
        for item in &self.items {
            let name = item.name.to_lowercase();
            let Some(at) = name.find(&needle) else {
                continue;
            };
            let rank = if at == 0 { 0 } else { 1000 };
            hits.push((rank - self.degree(item.id) as i64, item.id));
        }
        hits.sort_unstable();
        hits.truncate(MAX_HITS);
        hits.into_iter().map(|(_, id)| id).collect()
    }

    /// Rank a band's rows: whatever the workspace leans on most, first.
    fn rank(&self, rows: &mut [Row]) {
        rows.sort_by_key(|row| (std::cmp::Reverse(self.degree(row.id)), row.id));
    }

    /// Group a band's rows by the module they are written in, each group ranked.
    /// The module is a label, so the grouping is a reading aid and nothing more:
    /// it appears when there is more than one module in the band, or when the
    /// band is long enough that a reader needs a handle on it.
    pub fn grouped(&self, rows: Vec<Row>) -> Vec<Grouped> {
        let long = rows.len() > 4;
        let mut by_module: Vec<(String, Vec<Row>)> = Vec::new();
        for row in rows {
            let module = self
                .item(row.id)
                .map(|it| it.module.clone())
                .unwrap_or_default();
            match by_module.iter_mut().find(|(m, _)| *m == module) {
                Some((_, group)) => group.push(row),
                None => by_module.push((module, vec![row])),
            }
        }
        by_module.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
        let labelled = by_module.len() > 1 || long;
        by_module
            .into_iter()
            .map(|(module, mut rows)| {
                self.rank(&mut rows);
                Grouped {
                    label: labelled.then_some(module),
                    rows,
                }
            })
            .collect()
    }

    /// The reach, ranked inside each layer.
    fn ranked_layers(&self, layers: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
        layers
            .into_iter()
            .map(|mut layer| {
                layer.sort_by_key(|&id| (std::cmp::Reverse(self.degree(id)), id));
                layer
            })
            .collect()
    }

    /// The focus page: what this mark is, what it depends on, what depends on
    /// it, and how far a change to it could travel.
    pub fn focus(&self, id: u32) -> FocusRead {
        let item = self.item(id).expect("focus on a mark this survey has");
        let surface: Vec<Row> = self
            .solid_out(id)
            .map(|pair| Row::solid(pair.to, pair))
            .collect();
        let body_out: Vec<Row> = self
            .uses_out(id)
            .filter(|use_pair| self.solid_out(id).all(|pair| pair.to != use_pair.to))
            .map(|pair| Row::uses(pair.to, pair))
            .collect();

        // The right side, in three bands kept strictly apart. A signature
        // *names* a type; it does not hold one, and a reviewer reading "held
        // by" must never find a function in the list.
        let held: Vec<Row> = self
            .solid_in(id)
            .filter(|pair| !self.names_rather_than_holds(pair))
            .map(|pair| Row::solid(pair.from, pair))
            .collect();
        let named: Vec<Row> = self
            .solid_in(id)
            .filter(|pair| self.names_rather_than_holds(pair))
            .map(|pair| Row::solid(pair.from, pair))
            .collect();
        let body_in: Vec<Row> = self
            .uses_in(id)
            .filter(|use_pair| self.solid_in(id).all(|pair| pair.from != use_pair.from))
            .map(|pair| Row::uses(pair.from, pair))
            .collect();

        // The reach is seeded with the mark and everything that already stands
        // one hop from it: the first layer of the far column is the second hop.
        let mut seeds: Vec<u32> = vec![id];
        seeds.extend(self.solid_in(id).map(|pair| pair.from));
        seeds.sort_unstable();
        seeds.dedup();
        let reach = self.ranked_layers(self.reach_layers(&seeds, FOCUS_HOPS));

        let held_truth = if item.kind == ItemKind::Static {
            "a root — state no type holds."
        } else if !named.is_empty() {
            "nothing holds it — it enters through the signatures that name it."
        } else if !body_in.is_empty() {
            "no interface names it — only bodies reach it."
        } else {
            "nothing reaches it at all."
        };

        FocusRead {
            id,
            surface_title: if item.is_contract() {
                "its signature names"
            } else {
                "its surface names"
            },
            surface: self.grouped(surface),
            body_out: self.grouped(body_out),
            held: self.grouped(held),
            named: self.grouped(named),
            body_in: self.grouped(body_in),
            reach,
            held_truth,
        }
    }

    /// Which of the right-hand bands an incoming solid pair belongs in. A pair
    /// drawn by the dependent's methods, or by a contract nothing can hold, is
    /// a signature naming this mark — not a type keeping one.
    fn names_rather_than_holds(&self, pair: &SolidPair) -> bool {
        pair.from_method || self.item(pair.from).is_some_and(NavItem::is_contract)
    }

    /// The diff agenda: everything the change touched, the interface coupling it
    /// took on and gave back, and what it can reach that itself did not change.
    pub fn agenda(&self) -> AgendaRead {
        let changed: Vec<Row> = self.touched.iter().map(|&id| Row::plain(id)).collect();
        let new_leans = self.grouped(dedupe(
            self.solid
                .iter()
                .filter(|pair| pair.added)
                .map(|pair| Row::solid(pair.to, pair))
                .collect(),
        ));
        let cut_leans = self.grouped(dedupe(
            self.solid
                .iter()
                .filter(|pair| pair.removed && !pair.added)
                .map(|pair| Row {
                    id: pair.to,
                    word: "removed".to_string(),
                    family: Family::Solid,
                    event: Some(HoldEvent::Removed),
                })
                .collect(),
        ));
        let touched_set = {
            let mut flags = vec![false; self.items.len()];
            for &id in &self.touched {
                flags[id as usize] = true;
            }
            flags
        };
        let reaches: Vec<Vec<u32>> = self
            .ranked_layers(self.reach_layers(&self.touched, AGENDA_HOPS))
            .into_iter()
            .map(|layer| {
                layer
                    .into_iter()
                    .filter(|&id| !touched_set[id as usize])
                    .collect::<Vec<u32>>()
            })
            .filter(|layer| !layer.is_empty())
            .collect();
        AgendaRead {
            changed: self.grouped(changed),
            new_leans,
            cut_leans,
            reaches,
        }
    }
}

/// Keep the first row for each mark: two pairs can land a mark in the same
/// band, and it is one block either way.
fn dedupe(rows: Vec<Row>) -> Vec<Row> {
    let mut seen: Vec<u32> = Vec::new();
    rows.into_iter()
        .filter(|row| {
            let fresh = !seen.contains(&row.id);
            if fresh {
                seen.push(row.id);
            }
            fresh
        })
        .collect()
}

/// Which ink a wire wears.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    /// Interface coupling: a change at the other end forces a change here.
    Solid,
    /// Implementation coupling: a body leaning on the other end.
    Uses,
}

/// One row of a band: the mark it stands for, and the one word the wire to it
/// carries.
#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    pub id: u32,
    pub word: String,
    pub family: Family,
    pub event: Option<HoldEvent>,
}

impl Row {
    fn solid(id: u32, pair: &SolidPair) -> Self {
        Row {
            id,
            word: pair.word(),
            family: Family::Solid,
            event: pair.event(),
        }
    }

    fn uses(id: u32, pair: &UsesPair) -> Self {
        Row {
            id,
            word: pair.words(),
            family: Family::Uses,
            event: None,
        }
    }

    /// A row with no wire of its own — the agenda's own plates, and the marks
    /// its change reaches.
    pub fn plain(id: u32) -> Self {
        Row {
            id,
            word: String::new(),
            family: Family::Solid,
            event: None,
        }
    }
}

/// One module's rows inside a band.
#[derive(Clone, PartialEq, Debug)]
pub struct Grouped {
    /// The small-caps label, when the band is worth grouping at all.
    pub label: Option<String>,
    pub rows: Vec<Row>,
}

/// How many rows a whole band holds.
pub fn band_count(groups: &[Grouped]) -> usize {
    groups.iter().map(|g| g.rows.len()).sum()
}

/// The focus page's answer, bands and all.
#[derive(Clone, PartialEq, Debug)]
pub struct FocusRead {
    pub id: u32,
    pub surface_title: &'static str,
    pub surface: Vec<Grouped>,
    pub body_out: Vec<Grouped>,
    pub held: Vec<Grouped>,
    pub named: Vec<Grouped>,
    pub body_in: Vec<Grouped>,
    pub reach: Vec<Vec<u32>>,
    /// What an empty "held by" band says instead of a list.
    pub held_truth: &'static str,
}

/// The diff agenda's answer.
#[derive(Clone, PartialEq, Debug)]
pub struct AgendaRead {
    pub changed: Vec<Grouped>,
    pub new_leans: Vec<Grouped>,
    pub cut_leans: Vec<Grouped>,
    pub reaches: Vec<Vec<u32>>,
}

// ---------------------------------------------------------------------------
// The quotation: the focused mark's own declaration, whole.
// ---------------------------------------------------------------------------

/// One run of a quoted declaration, in the class that inks it.
#[derive(Clone, PartialEq, Debug)]
pub struct Ink {
    pub text: String,
    /// A workspace mark's own name — the bold run.
    pub held: bool,
    /// That mark is a sum type or a function.
    pub sum: bool,
    /// A CSS token class for everything else, empty for plain text.
    pub class: &'static str,
}

/// What a quoted row is against the base.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowState {
    Same,
    Added,
    Removed,
}

impl RowState {
    pub fn marker(self) -> Option<&'static str> {
        match self {
            RowState::Same => None,
            RowState::Added => Some("+"),
            RowState::Removed => Some("−"),
        }
    }
}

/// One row of the quotation: a field, a variant, a return, a method.
#[derive(Clone, PartialEq, Debug)]
pub struct QuoteRow {
    /// The field's or parameter's own name, with its colon. Empty for a row
    /// that is only a type.
    pub name: String,
    pub runs: Vec<Ink>,
    pub state: RowState,
    /// The row opens the method band: the promise, under a rule of its own.
    pub band: bool,
}

/// Words rust writes that are never a type name.
const KEYWORDS: [&str; 9] = [
    "fn", "pub", "crate", "self", "mut", "dyn", "impl", "const", "static",
];

impl NavModel {
    /// Ink one written type, the way the chart reads it: a run that names a
    /// workspace mark is the bold one, and the rest is token class.
    pub fn ink(&self, text: &str) -> Vec<Ink> {
        let mut runs: Vec<Ink> = Vec::new();
        let mut word = String::new();
        let mut rest = String::new();
        let flush_word = |word: &mut String, runs: &mut Vec<Ink>| {
            if word.is_empty() {
                return;
            }
            let (held, sum, class) = match self.named(word) {
                Some(id) => (true, self.item(id).is_some_and(NavItem::is_sum), ""),
                None if word.starts_with(char::is_uppercase) => (false, false, "tok-type"),
                None if KEYWORDS.contains(&word.as_str()) => (false, false, "tok-kw"),
                None => (false, false, ""),
            };
            runs.push(Ink {
                text: std::mem::take(word),
                held,
                sum,
                class,
            });
        };
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                if !rest.is_empty() {
                    runs.push(Ink {
                        text: std::mem::take(&mut rest),
                        held: false,
                        sum: false,
                        class: "",
                    });
                }
                word.push(ch);
            } else {
                flush_word(&mut word, &mut runs);
                rest.push(ch);
            }
        }
        flush_word(&mut word, &mut runs);
        if !rest.is_empty() {
            runs.push(Ink {
                text: rest,
                held: false,
                sum: false,
                class: "",
            });
        }
        runs
    }

    /// The focused mark's whole declaration, quoted. Rows the base had and the
    /// working copy dropped seat where the base wrote them, struck through and
    /// quoted from the base edition.
    pub fn quote(&self, graph: &CodeGraph, id: u32) -> Vec<QuoteRow> {
        let Some(item) = self.item(id) else {
            return Vec::new();
        };
        let mut rows: Vec<QuoteRow> = Vec::new();
        let plain = |name: String, runs: Vec<Ink>, state: RowState| QuoteRow {
            name,
            runs,
            state,
            band: false,
        };

        if item.ghost {
            let Some(ghost) = graph.ghosts.iter().find(|g| g.id == id) else {
                return rows;
            };
            for (name, decl) in &ghost.field_rows {
                rows.push(plain(format!("{name}: "), self.ink(decl), RowState::Same));
            }
            for written in &ghost.variants {
                rows.push(plain(String::new(), self.ink(written), RowState::Same));
            }
            if !ghost.ty.is_empty() {
                let lead = if ghost.kind == ItemKind::Fn {
                    "→ "
                } else if ghost.kind == ItemKind::TypeAlias {
                    "aliases "
                } else {
                    ""
                };
                rows.push(plain(
                    String::new(),
                    lead_runs(lead, self.ink(&ghost.ty)),
                    RowState::Same,
                ));
            }
            for (at, (_, sig)) in ghost.method_rows.iter().enumerate() {
                rows.push(QuoteRow {
                    name: String::new(),
                    runs: self.ink(sig),
                    state: RowState::Same,
                    band: at == 0,
                });
            }
            return rows;
        }

        let Some(mark) = graph.items.get(id as usize) else {
            return rows;
        };
        match mark.kind {
            ItemKind::Struct | ItemKind::Union | ItemKind::Fn => {
                let mut drawn: Vec<QuoteRow> = mark
                    .field_rows
                    .iter()
                    .enumerate()
                    .map(|(at, (name, decl))| {
                        plain(
                            format!("{name}: "),
                            self.ink(decl),
                            if mark.fields_added.contains(&(at as u32)) {
                                RowState::Added
                            } else {
                                RowState::Same
                            },
                        )
                    })
                    .collect();
                let dropped: Vec<(usize, QuoteRow)> = mark
                    .fields_removed
                    .iter()
                    .map(|(before, name, decl)| {
                        (
                            *before as usize,
                            plain(format!("{name}: "), self.ink(decl), RowState::Removed),
                        )
                    })
                    .collect();
                weave(&mut drawn, dropped);
                rows.extend(drawn);
                if mark.kind == ItemKind::Fn && !mark.ty.is_empty() {
                    rows.push(plain(
                        String::new(),
                        lead_runs("→ ", self.ink(&mark.ty)),
                        RowState::Same,
                    ));
                }
            }
            ItemKind::Enum => {
                let mut drawn: Vec<QuoteRow> = mark
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(at, written)| {
                        plain(
                            String::new(),
                            self.ink(written),
                            if mark.variants_added.contains(&(at as u32)) {
                                RowState::Added
                            } else {
                                RowState::Same
                            },
                        )
                    })
                    .collect();
                let dropped: Vec<(usize, QuoteRow)> = mark
                    .variants_removed
                    .iter()
                    .map(|(before, written)| {
                        (
                            *before as usize,
                            plain(String::new(), self.ink(written), RowState::Removed),
                        )
                    })
                    .collect();
                weave(&mut drawn, dropped);
                rows.extend(drawn);
            }
            ItemKind::TypeAlias => rows.push(plain(
                String::new(),
                lead_runs("aliases ", self.ink(&mark.ty)),
                RowState::Same,
            )),
            _ if !mark.ty.is_empty() => {
                rows.push(plain(String::new(), self.ink(&mark.ty), RowState::Same))
            }
            _ => {}
        }

        // The second band: the methods the type publishes, whatever their
        // door. This altitude has no door of its own — a question about one
        // mark is answered with the whole mark.
        let mut band: Vec<QuoteRow> = mark
            .method_rows
            .iter()
            .enumerate()
            .map(|(at, row)| QuoteRow {
                name: String::new(),
                runs: self.ink(&row.sig),
                state: if mark.methods_added.contains(&(at as u32)) {
                    RowState::Added
                } else {
                    RowState::Same
                },
                band: false,
            })
            .collect();
        let dropped: Vec<(usize, QuoteRow)> = mark
            .methods_removed
            .iter()
            .map(|(before, _, sig)| {
                (
                    *before as usize,
                    QuoteRow {
                        name: String::new(),
                        runs: self.ink(sig),
                        state: RowState::Removed,
                        band: false,
                    },
                )
            })
            .collect();
        weave(&mut band, dropped);
        if let Some(first) = band.first_mut() {
            first.band = true;
        }
        rows.extend(band);
        rows
    }
}

/// A lead word in front of a quoted type — rust's own arrow, or the word an
/// alias reads by.
fn lead_runs(lead: &str, mut runs: Vec<Ink>) -> Vec<Ink> {
    if lead.is_empty() {
        return runs;
    }
    runs.insert(
        0,
        Ink {
            text: lead.to_string(),
            held: false,
            sum: false,
            class: "tok-punct",
        },
    );
    runs
}

/// Seat the base's dropped rows back where the base wrote them: a removed
/// field belongs above the row that took its place, not in a list at the foot.
fn weave(drawn: &mut Vec<QuoteRow>, dropped: Vec<(usize, QuoteRow)>) {
    for (at, row) in dropped.into_iter().rev() {
        let at = at.min(drawn.len());
        drawn.insert(at, row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{FileInfo, HoldEdge, ItemEdge, ItemMark, MarkRef};

    fn file(id: u32, path: &str) -> FileInfo {
        FileInfo {
            id,
            path: path.to_string(),
            krate: "slope".to_string(),
            changed: false,
            lines: 10,
            items: 1,
            refs_in_files: 0,
        }
    }

    fn mark(id: u32, file: u32, name: &str, kind: ItemKind) -> ItemMark {
        ItemMark {
            id,
            file,
            local: id,
            name: name.to_string(),
            label: name.to_string(),
            kind,
            vis: Vis::Pub,
            line: 1,
            parent: None,
            fan_in: 0,
            impls: Vec::new(),
            field_rows: Vec::new(),
            variants: Vec::new(),
            ty: String::new(),
            method_rows: Vec::new(),
            delta: Delta::Same,
            fields_added: Vec::new(),
            fields_removed: Vec::new(),
            variants_added: Vec::new(),
            variants_removed: Vec::new(),
            methods_added: Vec::new(),
            methods_removed: Vec::new(),
        }
    }

    fn hold(from: u32, to: u32, via: &str, event: Option<HoldEvent>) -> HoldEdge {
        HoldEdge {
            from,
            to,
            kind: HoldKind::Owns,
            via: via.to_string(),
            fields: vec![("seat".to_string(), "T".to_string())],
            from_method: false,
            event,
        }
    }

    fn graph() -> CodeGraph {
        CodeGraph {
            files: vec![file(0, "src/api.rs"), file(1, "src/views/shell.rs")],
            refs: Vec::new(),
            items: vec![
                mark(0, 0, "Wire", ItemKind::Struct),
                mark(1, 0, "Tok", ItemKind::Enum),
                mark(2, 1, "shell", ItemKind::Fn),
                mark(3, 1, "Trail", ItemKind::Struct),
            ],
            implements: Vec::new(),
            item_edges: vec![ItemEdge {
                from_file: 1,
                from: Some(2),
                to_file: 0,
                to: Some(1),
                count: 4,
            }],
            local_refs: vec![MarkRef {
                from: 3,
                to: 2,
                count: 2,
            }],
            holds: vec![
                hold(0, 1, "Arc", None),
                hold(3, 0, "", None),
                hold(2, 0, "", Some(HoldEvent::Added)),
            ],
            ghosts: Vec::new(),
            unresolved: 0,
            notes: Vec::new(),
        }
    }

    /// A pair that lost one edge and gained another was rewritten: the relation
    /// still stands, written differently, and the word has to say so.
    #[test]
    fn a_pair_with_both_events_reads_as_rewritten() {
        let mut g = graph();
        g.holds.push(hold(3, 0, "Signal", Some(HoldEvent::Added)));
        g.holds.push(hold(3, 0, "", Some(HoldEvent::Removed)));
        let model = NavModel::build(&g);
        let pair = model
            .solid
            .iter()
            .find(|p| p.from == 3 && p.to == 0)
            .expect("the pair");
        assert_eq!(pair.word(), "rewritten");
        assert_eq!(pair.event(), Some(HoldEvent::Added));
    }

    /// The wrapper the walk met writes the wire's word when the diff has
    /// nothing to say about the pair.
    #[test]
    fn the_wire_word_falls_back_from_diff_to_impl_to_wrapper_to_row() {
        let model = NavModel::build(&graph());
        let word = |from: u32, to: u32| {
            model
                .solid
                .iter()
                .find(|p| p.from == from && p.to == to)
                .map(SolidPair::word)
        };
        assert_eq!(word(0, 1).as_deref(), Some("Arc"));
        assert_eq!(word(3, 0).as_deref(), Some("seat"));
        assert_eq!(word(2, 0).as_deref(), Some("added"));
    }

    /// The right-hand bands are kept strictly apart: a signature names a mark,
    /// it does not hold one.
    #[test]
    fn a_function_naming_a_type_is_never_in_held_by() {
        let model = NavModel::build(&graph());
        let read = model.focus(0);
        let ids = |groups: &[Grouped]| -> Vec<u32> {
            groups
                .iter()
                .flat_map(|g| g.rows.iter().map(|r| r.id))
                .collect()
        };
        assert_eq!(ids(&read.held), vec![3]);
        assert_eq!(ids(&read.named), vec![2]);
    }

    /// A body user that already names the mark in its own surface is not a body
    /// user twice: the solid band said it.
    #[test]
    fn body_bands_drop_whatever_the_solid_bands_already_said() {
        let mut g = graph();
        // `Trail` both holds `Wire` and calls it.
        g.local_refs.push(MarkRef {
            from: 3,
            to: 0,
            count: 5,
        });
        let model = NavModel::build(&g);
        let read = model.focus(0);
        let ids: Vec<u32> = read
            .body_in
            .iter()
            .flat_map(|g| g.rows.iter().map(|r| r.id))
            .collect();
        assert!(!ids.contains(&3), "a holder is not also a body reader");
    }

    /// The reach is layered, and a removed pair carries nothing: it is diff
    /// ink, not structure.
    #[test]
    fn the_reach_is_one_layer_per_hop_and_skips_removed_pairs() {
        let mut g = graph();
        g.holds.push(hold(1, 3, "", Some(HoldEvent::Removed)));
        let model = NavModel::build(&g);
        // Seeded with `Tok` alone: `Wire` holds it, and `Trail` and `shell`
        // hold `Wire`.
        let layers = model.reach_layers(&[1], 4);
        assert_eq!(layers[0], vec![0]);
        let mut second = layers[1].clone();
        second.sort_unstable();
        assert_eq!(second, vec![2, 3]);
        assert_eq!(layers.len(), 2, "the removed pair carried nothing");
    }

    /// The board threads the shortest path either way round, because the
    /// question is how two marks connect at all.
    #[test]
    fn a_path_walks_both_families_in_either_direction() {
        let model = NavModel::build(&graph());
        let chain = model.path_between(1, 3).expect("a path");
        assert_eq!(chain.iter().map(|s| s.id).collect::<Vec<_>>(), vec![0, 3]);
        assert!(model.path_between(1, 1).is_none());
    }

    /// A prefix match beats a middle match, and degree breaks the tie.
    #[test]
    fn search_ranks_prefix_first_then_by_degree() {
        let model = NavModel::build(&graph());
        let names = |query: &str| -> Vec<String> {
            model
                .search(query)
                .into_iter()
                .map(|id| model.item(id).unwrap().name.clone())
                .collect()
        };
        // Both are prefix matches, so degree decides which reads first.
        assert_eq!(names("t"), vec!["Tok", "Trail"]);
        // A match in the middle of a name still ranks, a band below.
        assert_eq!(names("ai"), vec!["Trail"]);
    }

    /// A dropped field seats where the base wrote it, not in a list at the foot.
    #[test]
    fn a_dropped_row_seats_where_the_base_wrote_it() {
        let mut g = graph();
        g.items[0].field_rows = vec![
            ("a".to_string(), "u32".to_string()),
            ("c".to_string(), "Tok".to_string()),
        ];
        g.items[0].fields_removed = vec![(1, "b".to_string(), "String".to_string())];
        g.items[0].fields_added = vec![1];
        let model = NavModel::build(&g);
        let rows = model.quote(&g, 0);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a: ", "b: ", "c: "]);
        assert_eq!(rows[1].state, RowState::Removed);
        assert_eq!(rows[2].state, RowState::Added);
        // The run naming a workspace mark is the bold one.
        assert!(rows[2].runs.iter().any(|run| run.held && run.text == "Tok"));
    }

    /// Every module label is a label, and the crate root says so in words.
    #[test]
    fn a_file_at_the_source_root_is_labelled_the_crate_root() {
        let mut g = graph();
        g.files.push(file(2, "src/main.rs"));
        g.items.push(mark(4, 2, "main", ItemKind::Fn));
        let model = NavModel::build(&g);
        assert_eq!(model.item(4).unwrap().module, "crate root");
        assert_eq!(model.item(0).unwrap().module, "api");
        assert_eq!(model.item(2).unwrap().module, "views");
    }
}
