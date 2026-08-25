//! Data-altitude furniture: the cartouche and the selection sheet — the same
//! engraved plates the other altitudes wear. There is no legend: the chart
//! states itself — the tier is the paper's own nesting, every block and wire
//! carries its words on hover, and the survey's limits rest behind one fold
//! at the cartouche's foot.

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::{CodeGraph, Delta, HoldEvent, HoldKind, ItemMark, Vis};
use crate::views::chrome::{Altitude, AltitudeSwitch, plural};
use crate::views::data::model::{
    Anchor, DataFacts, DataMark, DataModel, RowState, Stand, Tier, Unseen, upstream,
};
use crate::views::data::{
    RefDir, Sel, VisFloor, mark_route, mod_route, peek_at, peek_key, peek_route, use_data,
};

/// Which modules the diff landed in, in plain words.
fn insight(modules: &[String]) -> Option<String> {
    match modules {
        [] => None,
        [one] => Some(format!("the diff lands in {one} alone")),
        [a, b] => Some(format!("the diff lands in {a} and {b}")),
        rest => {
            let (last, first) = rest.split_last()?;
            Some(format!("the diff lands in {} and {last}", first.join(", ")))
        }
    }
}

impl DataFacts {
    /// The structural diff's own line: only what happened, in git's order.
    fn diff_words(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.diff.added > 0 {
            parts.push(format!("{} added", self.diff.added));
        }
        if self.diff.removed > 0 {
            parts.push(format!("{} removed", self.diff.removed));
        }
        if self.diff.changed > 0 {
            parts.push(format!("{} changed", self.diff.changed));
        }
        parts.join(" · ")
    }
}

/// The chart's title block: the census of what state the workspace keeps, the
/// altitude ladder, the diff, and the reading control for body dependence.
///
/// It states no tier counts and no edge counts (2026-08-21, distill): `55
/// roots · 63 nested · 16 standing` and `209 body dependences · 127 at rest`
/// were the model's own bookkeeping in four invented words, and no reviewer
/// decides anything on them. The tier is what the paper draws and a root's
/// own hover words teach; the edges are what the chart shows.
///
/// Two readings ride here, because both act on the whole plate: which
/// direction the body references are read in, and how narrow a declaration may
/// be and still be drawn.
#[component]
pub(super) fn DataCartouche(
    facts: DataFacts,
    workspace: String,
    diff_line: String,
    notes: Vec<String>,
) -> Element {
    let insight = insight(&facts.changed_modules);
    let kinds = {
        let mut parts = vec![
            plural(facts.kinds.structs, "struct"),
            plural(facts.kinds.enums, "enum"),
            plural(facts.kinds.statics, "static"),
        ];
        if facts.kinds.unions > 0 {
            parts.insert(2, plural(facts.kinds.unions, "union"));
        }
        parts.join(" · ")
    };
    rsx! {
        section { class: "plate pointer-events-auto",
            div { class: "px-4 pt-3 pb-2",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{workspace}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft", "{kinds}" }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Data }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if !facts.diff_words().is_empty() {
                        p { class: "text-flare", "{facts.diff_words()}" }
                        if let Some(insight) = insight {
                            p { class: "text-ink-soft", "{insight}" }
                        }
                    } else if facts.off_paper > 0 {
                        // The counts are the reading's, so its silence has to
                        // be too: the base may well have changed state this
                        // reading declines to draw.
                        p { class: "text-ink-soft", "no shape changes in what this reading draws" }
                    } else {
                        p { class: "text-ink-soft", "no shape changes since the base" }
                    }
                }
            }
            RefToggle {}
            VisSlider { off_paper: facts.off_paper }
            SurveyLimits { notes }
        }
    }
}

/// The survey's own limits, folded at the cartouche's foot. They are read
/// once, to trust the chart — not consulted while reading it — so they rest
/// behind one line, and the words are the survey's, never a paraphrase.
#[component]
fn SurveyLimits(notes: Vec<String>) -> Element {
    if notes.is_empty() {
        return rsx! {};
    }
    rsx! {
        details { class: "fold border-t border-ink-line px-4 py-2",
            summary { class: "cursor-pointer select-none font-data text-[9.5px] tracking-[0.1em] uppercase text-ink-soft hover:text-ink",
                "what the survey cannot read"
            }
            div { class: "mt-1.5 space-y-1 pb-1 font-data text-[10px] leading-snug text-ink-soft",
                for (i , note) in notes.iter().enumerate() {
                    p { key: "{i}", "{note}" }
                }
            }
        }
    }
}

/// Which direction the chart's body references are read in. It rides on the
/// cartouche because it acts on the whole plate, against whatever the reviewer
/// has in focus: the selected mark, the hovered block, or — with the paper at
/// rest — the declarations the diff touched.
#[component]
fn RefToggle() -> Element {
    let data = use_data();
    let current = *data.ref_dir.read();
    let seg = |label: &'static str, hint: &'static str, val: RefDir| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                title: hint,
                onclick: move |_| {
                    let mut dir = data.ref_dir;
                    dir.set(val);
                },
                "{label}"
            }
        }
    };
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "which direction the chart's body references are read in",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "references"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("uses", "what the selection — or, at rest, what the diff — leans on", RefDir::Uses)}
                {seg("used by", "whose code leans on the selection — or, at rest, on the diff", RefDir::UsedBy)}
                {seg("both", "both ways round; with nothing selected and no diff, every reference", RefDir::Both)}
            }
        }
    }
}

/// How narrow a declaration may be and still be drawn, as a slider along the
/// rungs rust writes: `pub`, `pub(crate)`, `pub(super)`, `all` (2026-08-25,
/// user). Sliding towards `pub` reads the surface a crate publishes and
/// nothing else, which is the audit a reviewer of a large change actually
/// runs; sliding back to `all` is one move.
///
/// The rungs are the visibility each declaration **writes**, not what a chain
/// of private modules leaves reachable from outside. That distinction is the
/// group's hover words, not its label: `visibility as declared` spent three
/// words of plate on a caveat, and the scale underneath — `pub`, `pub(crate)`
/// — already says which alphabet it reads (2026-08-25, distill). The foot
/// states how many declarations the reading leaves off, in as few characters
/// as say it, because a narrow reading and an empty workspace must never look
/// alike.
#[component]
fn VisSlider(off_paper: usize) -> Element {
    let data = use_data();
    let current = *data.vis_floor.read();
    let stop_at = move |stop: VisFloor| {
        let mut floor = data.vis_floor;
        floor.set(stop);
    };
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "how narrow a declaration may be and still be drawn",
            span {
                class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                title: "the visibility each declaration writes — not what a chain of private modules leaves reachable from outside",
                "visibility"
            }
            input {
                class: "vis-slide mt-1.5 block w-full",
                r#type: "range",
                min: "0",
                max: "{VisFloor::STOPS.len() - 1}",
                step: "1",
                value: "{current.step()}",
                title: "{current.hint()}",
                "aria-valuetext": "{current.label()}",
                oninput: move |e| {
                    if let Some(stop) = VisFloor::at_step(&e.value()) {
                        stop_at(stop);
                    }
                },
            }
            div { class: "mt-0.5 flex items-baseline justify-between gap-1",
                for stop in VisFloor::STOPS {
                    button {
                        key: "{stop.label()}",
                        class: "whitespace-nowrap font-data text-[9px]",
                        class: if current == stop { "text-ink" } else { "text-ink-soft hover:text-ink" },
                        "aria-pressed": if current == stop { "true" } else { "false" },
                        title: "{stop.hint()}",
                        onclick: move |_| stop_at(stop),
                        "{stop.label()}"
                    }
                }
            }
            if off_paper > 0 {
                p {
                    class: "mt-1 font-data text-[9.5px] leading-snug text-ink-soft",
                    title: "{plural(off_paper, \"declaration\")} written narrower than this reading draws",
                    "{off_paper} off"
                }
            }
        }
    }
}

impl Delta {
    /// The letter a declaration wears, in git's own alphabet. A mark's own
    /// letter climbs through [`DataMark::letter`]; this is for the ends the
    /// sheet quotes from the survey directly — a trait it promises, a method
    /// written for it.
    fn letter(self) -> Option<&'static str> {
        match self {
            Delta::Added => Some("A"),
            Delta::Changed => Some("M"),
            Delta::Same => None,
        }
    }
}

impl HoldEvent {
    /// The relation's diff event in its own word, for the right of a row.
    fn word(self) -> &'static str {
        match self {
            HoldEvent::Added => "added",
            HoldEvent::Removed => "removed",
        }
    }
}

/// What an impl header promises, and for what: `impl Clone for Vis` reads
/// `("Clone", "Vis")`. An inherent `impl Vis` promises no contract, and so
/// answers `None` — which is exactly how the sheet tells a type's own methods
/// from the ones a trait asked for.
fn header_promise(header: &str) -> Option<(&str, &str)> {
    let (promise, for_ty) = header.strip_prefix("impl ")?.rsplit_once(" for ")?;
    Some((promise.trim(), for_ty.trim()))
}

/// The bare name a trait path ends in, generic arguments dropped:
/// `fmt::Display` is `Display`, `From<&str>` is `From`. The survey resolved
/// its impl edges properly; this is what a written header is matched to one
/// on, the same way the structural diff matches names.
fn bare_trait(promise: &str) -> &str {
    let base = promise.split('<').next().unwrap_or(promise);
    base.rsplit("::").next().unwrap_or(base).trim()
}

impl HoldKind {
    /// How a row says this hold: the survey's own words for the type it went
    /// through when it has any, the kind's plain word otherwise.
    fn word(self, via: &str) -> String {
        if !via.is_empty() {
            return via.to_string();
        }
        match self {
            HoldKind::Owns => "owns",
            HoldKind::Shares => "shares",
            HoldKind::Borrows => "borrows",
            HoldKind::Dyn => "dyn",
            HoldKind::Implements => "implements",
        }
        .to_string()
    }
}

/// One row of the sheet's relation lists: a drawn mark (a link that
/// re-centers the selection on it), an end this chart draws no block for (a
/// link that quotes its source beside the sheet), or a folded module's counted
/// row.
#[derive(Clone, PartialEq)]
struct HoldRow {
    pub(crate) to: Option<Route>,
    pub(crate) decl: String,
    pub(crate) name: String,
    pub(crate) letter: Option<&'static str>,
    pub(crate) word: String,
    /// The relation's own diff event, in its word.
    pub(crate) event: Option<&'static str>,
    /// What the row cannot fit and a reader still wants: a method's signature
    /// as written. The row's hover words, never its ink — 256 pixels of mono
    /// is a name, not a signature.
    pub(crate) hint: Option<String>,
    /// The end this row names, for the quotation plate: `file@label`, as the
    /// URL carries it. Set on every row whose end has no block on this chart
    /// — a function, a trait, a method — and `None` where the row already
    /// goes somewhere the chart draws.
    pub(crate) peek: Option<String>,
}

/// One chunked list of relation rows: the first eight, then a typographic
/// "show all n".
///
/// A row is a link wherever it names something a reviewer can go to. Which
/// door it opens depends on what the end is: an end the chart draws a block
/// for re-centers the selection on it; an end it draws none for — a function,
/// a trait, a method — opens beside the sheet as a quotation of its own
/// source, and the row stays inked while it is open, so the plate is never
/// loose from the row that asked for it.
#[component]
fn HoldList(rows: Vec<HoldRow>, sel: Sel, open: Option<String>) -> Element {
    let mut all = use_signal(|| false);
    let total = rows.len();
    let shown = if all() || total <= 8 { total } else { 8 };
    rsx! {
        ul { class: "mt-1",
            for (i , row) in rows.iter().take(shown).enumerate() {
                li { key: "{i}",
                    {
                        let quote = row.peek.as_deref().and_then(peek_at);
                        let to = row
                            .to
                            .clone()
                            .or_else(|| quote.map(|(path, label)| peek_route(&sel, path, label)));
                        let here = row.peek.is_some() && row.peek == open;
                        // Where a row's own words already say where it is
                        // written, its hover words say what opening it does —
                        // the same sentence the chart's quoted rows use for
                        // the run that goes to a block.
                        let title = match (&row.hint, row.to.is_none() && quote.is_some()) {
                            (Some(hint), true) => Some(format!("{hint} — quote its source")),
                            (hint, _) => hint.clone(),
                        };
                        // The open row keeps its ink and takes its left
                        // padding back off the rule, so nothing shifts when a
                        // quotation opens.
                        let ink = match here {
                            true => "border-l-2 border-ink bg-ink/5 pr-1 pl-[2px]",
                            false => "px-1",
                        };
                        match to {
                            Some(to) => rsx! {
                                Link {
                                    class: "flex w-full items-baseline gap-1.5 py-0.5 font-data text-[10.5px] hover:bg-ink/5 {ink}",
                                    to,
                                    title,
                                    RowCells { row: row.clone(), dead: false }
                                }
                            },
                            None => rsx! {
                                span {
                                    class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] text-ink-soft",
                                    title: row.hint.clone(),
                                    RowCells { row: row.clone(), dead: true }
                                }
                            },
                        }
                    }
                }
            }
        }
        if shown < total {
            button {
                class: "mt-1 px-1 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                onclick: move |_| all.set(true),
                "show all {total}"
            }
        }
    }
}

/// One row's own cells, the same however the row is opened — or not opened at
/// all: the declaration it wears, its name, its diff letter, what the relation
/// says, and the diff event. A row that goes nowhere is one voice quieter,
/// which is the only difference.
#[component]
fn RowCells(row: HoldRow, dead: bool) -> Element {
    rsx! {
        if !row.decl.is_empty() {
            span {
                class: "shrink-0",
                class: if !dead { "text-ink-soft" },
                "{row.decl}"
            }
        }
        // The name is the one thing the row exists to state: it takes the
        // row's free width, and the count-and-clause column truncates against
        // a hard cap before the name gives up a pixel.
        span {
            class: if dead { "min-w-0 flex-1 truncate" } else { "flex-1 shrink-0 font-medium text-ink" },
            "{row.name}"
        }
        if let Some(letter) = row.letter {
            span { class: "shrink-0 font-bold text-flare", "{letter}" }
        }
        span {
            class: "max-w-[45%] shrink-0 truncate text-right text-[9px]",
            class: if !dead { "text-ink-soft" },
            title: "{row.word}",
            "{row.word}"
        }
        if let Some(event) = row.event {
            span { class: "shrink-0 text-[9px] text-flare", "{event}" }
        }
    }
}

/// One end of a uses edge as the sheet reads it.
type UsesRow<'a> = (&'a Anchor, u32, &'a [(String, u32)]);

impl DataModel {
    /// The rows one side of a selection draws, from each relation's far end.
    fn hold_rows(&self, holds: Vec<(&Anchor, HoldKind, &str, Option<HoldEvent>)>) -> Vec<HoldRow> {
        let by_id = self.by_id();
        holds
            .into_iter()
            .filter_map(|(anchor, kind, via, event)| {
                let event = event.map(HoldEvent::word);
                match anchor {
                    Anchor::Mark(id) => {
                        let mark = by_id.get(id);
                        Some(HoldRow {
                            to: mark.map(|m| mark_route(&m.head.path, &m.head.label)),
                            decl: mark
                                .map(|m| m.head.kind.decl_words(&m.head.vis))
                                .unwrap_or_default(),
                            name: mark.map(|m| m.head.name.clone()).unwrap_or_default(),
                            letter: mark.and_then(|m| m.letter()),
                            word: kind.word(via),
                            event,
                            hint: None,
                            peek: None,
                        })
                    }
                    Anchor::Mod(frame) => {
                        // A folded module is the only counted row this chart
                        // leaves, and the row can be selected, so it is a link.
                        let frame = self.frame(*frame)?;
                        let place = match frame.module.is_empty() {
                            true => frame.krate.clone(),
                            false => format!("mod {}", frame.words()),
                        };
                        Some(HoldRow {
                            to: Some(mod_route(frame.key())),
                            decl: String::new(),
                            name: format!(
                                "+ {} · {place}",
                                plural(frame.fold.packed as usize, "item")
                            ),
                            letter: None,
                            word: kind.word(via),
                            event,
                            hint: None,
                            peek: None,
                        })
                    }
                }
            })
            .collect()
    }

    /// The rows one side of a body-dependence reading draws, weight and all.
    fn uses_rows(&self, rows: Vec<UsesRow<'_>>) -> Vec<(u32, HoldRow)> {
        let by_id = self.by_id();
        rows.into_iter()
            .filter_map(|(anchor, count, clauses)| {
                let Anchor::Mark(id) = anchor else {
                    return None;
                };
                let far = by_id.get(id)?;
                let mut word = plural(count as usize, "ref");
                // One clause at most, and only when the whole row fits: the
                // name is the one thing the row exists to state, so the clause
                // drops before the name would clip.
                if let Some((row, _)) = clauses.first()
                    && far.head.name.chars().count() + row.chars().count() <= 13
                {
                    word = format!("{word} · {row}");
                }
                Some((
                    count,
                    HoldRow {
                        to: Some(mark_route(&far.head.path, &far.head.label)),
                        decl: far.head.kind.decl_words(&far.head.vis),
                        name: far.head.name.clone(),
                        letter: far.letter(),
                        word,
                        event: None,
                        hint: None,
                        peek: None,
                    },
                ))
            })
            .collect()
    }
}

/// One relation, one ranking: a drawn type and a body with no block here are
/// the same ink, so they rank together and the heaviest reads first. The
/// list's first eight are what the sheet shows before `show all`, which is
/// the whole reason the order has to be weight and not provenance.
fn ranked(mut rows: Vec<(u32, HoldRow)>) -> Vec<HoldRow> {
    rows.sort_by_key(|(count, _)| std::cmp::Reverse(*count));
    rows.into_iter().map(|(_, row)| row).collect()
}

/// One row per end, however many ways that end reaches this one. A type whose
/// API names the selection *and* whose bodies lean on it is two facts about
/// one neighbour; under separate headings that was two lists, and in one list
/// it would read as the same row engraved twice. The first mention keeps the
/// place — the order inside a heading is the strength of the claim — and the
/// later words join it, so the row says `API · 13 references` and the name is
/// written once.
fn per_end(rows: Vec<HoldRow>) -> Vec<HoldRow> {
    let mut out: Vec<HoldRow> = Vec::new();
    for row in rows {
        let same = out.iter_mut().find(|r| {
            r.name == row.name && r.decl == row.decl && r.to == row.to && r.peek == row.peek
        });
        let Some(seen) = same else {
            out.push(row);
            continue;
        };
        // One word per kind, and no more: a merged row says `owns · API · 6
        // refs` and stops there. What a body reference carries past its count
        // — the heaviest method the count came through — is a caption, and a
        // caption on a merged row is the one thing wide enough to truncate.
        let add = row.word.split(" · ").next().unwrap_or_default();
        if !add.is_empty() && !seen.word.split(" · ").any(|w| w == add) {
            seen.word = match seen.word.is_empty() {
                true => add.to_string(),
                false => format!("{} · {add}", seen.word),
            };
        }
        seen.event = seen.event.or(row.event);
        seen.hint = seen.hint.take().or(row.hint);
    }
    out
}

impl DataMark {
    /// The tier, said out loud — the one sentence this altitude exists for.
    fn tier_line(&self) -> String {
        if self.state.ghost {
            return "removed since the base — whoever held it, the removed edges say.".to_string();
        }
        match self.seat.tier {
            Tier::Root if self.is_static() => "a root — state no type holds.".to_string(),
            Tier::Root => "a root: no type holds it.".to_string(),
            // The holder is the first row of the section right below this line;
            // naming it twice in six words was the sheet saying the same thing to
            // itself (2026-08-21, distill).
            Tier::Nested(_) => "drawn inside its holder's block.".to_string(),
            Tier::Standing(Stand::Shared) => {
                "shared through a handle: no single container, so it stands beside its holders."
                    .to_string()
            }
            Tier::Standing(Stand::Vocab) => format!(
                "{} hold it — too many to seat under one, so its fan-in rests folded.",
                plural(self.seat.held_by as usize, "type")
            ),
            Tier::Standing(Stand::Afar) => {
                "its holder is in another module, so the ownership stays a line.".to_string()
            }
            Tier::Standing(Stand::Ring) => {
                "it and its holder own each other, so the seat that would close the loop stays a \
                 line."
                    .to_string()
            }
            Tier::Standing(Stand::Narrower) => {
                "every type that holds it is narrower than this reading draws — widen it to see \
                 them."
                    .to_string()
            }
        }
    }
}

/// What the selected type offers, and neither ink on this paper draws: the
/// contracts it promises, and the methods written for it. A block is state
/// only — the sheet is a list, so both are rows (2026-08-24, user), each one
/// naming the file and line its own source is written on, since a trait and a
/// method keep no state and this chart draws them no block to open.
struct Offers {
    /// One row per hand-written trait impl, wherever in the workspace it is
    /// written. A derive is not here: it stands in the type's own source.
    promises: Vec<HoldRow>,
    /// One row per method the survey read for the type, its own first, then
    /// the ones a contract asked for, under the promise they answer.
    methods: Vec<HoldRow>,
}

impl DataMark {
    /// Read both lists off the survey. Nothing is reconstructed: a promise is
    /// an impl header as written, a method is a signature as written, and a
    /// row links to the definition the survey engraved for it.
    fn offers(&self, graph: &CodeGraph) -> Offers {
        let mut promises: Vec<HoldRow> = Vec::new();
        let mut methods: Vec<HoldRow> = Vec::new();
        let where_written = |m: &ItemMark| {
            graph
                .files
                .get(m.file as usize)
                .map(|f| format!("{}:{}", f.path, m.head.line))
        };
        // What a row of this section can be quoted from: a trait and a method
        // are written somewhere, and this chart draws neither a block.
        let quotable = |m: &ItemMark| {
            graph
                .files
                .get(m.file as usize)
                .map(|f| peek_key(&f.path, &m.head.label))
        };
        let trait_row =
            |t: &ItemMark, event: Option<HoldEvent>, name: String, word: String| HoldRow {
                to: None,
                decl: t.head.kind.decl_words(&t.head.vis),
                name,
                letter: t.diff.delta.letter(),
                word,
                event: event.map(HoldEvent::word),
                hint: where_written(t),
                peek: quotable(t),
            };
        if let Some(item) = graph.item(self.id) {
            // Which promises the workspace declares itself: those the survey
            // resolved to a trait mark, so those rows link to the contract and
            // carry the impl's own diff event. A foreign trait is a name.
            let ours: Vec<(&ItemMark, Option<HoldEvent>)> = graph
                .implements
                .iter()
                .filter(|e| e.ty == self.id)
                .filter_map(|e| Some((graph.item(e.trait_mark)?, e.event)))
                .collect();
            let mut quoted: std::collections::HashSet<u32> = std::collections::HashSet::new();
            for header in &item.reach.impls {
                let Some((promise, for_ty)) = header_promise(header) else {
                    continue;
                };
                let known = ours
                    .iter()
                    .find(|(t, _)| bare_trait(&t.head.name) == bare_trait(promise));
                if let Some((t, _)) = known {
                    quoted.insert(t.id);
                }
                // The header's self type, but only where it is not simply this
                // mark: `impl Held for Frame<'a>` says which edition of the type
                // promises, and `impl Held for Frame` would only say the name the
                // sheet's own title says.
                let word = match for_ty == self.head.name {
                    true => String::new(),
                    false => format!("for {for_ty}"),
                };
                match known {
                    Some((t, event)) => {
                        promises.push(trait_row(t, *event, promise.to_string(), word))
                    }
                    None => promises.push(HoldRow {
                        to: None,
                        decl: "trait".to_string(),
                        name: promise.to_string(),
                        letter: None,
                        word,
                        event: None,
                        hint: None,
                        // A foreign trait is written outside the workspace:
                        // the survey never read it, so there is nothing to
                        // quote.
                        peek: None,
                    }),
                }
            }
            // A contract the base promised and the working copy dropped: its impl
            // block is gone, so no header quotes it and the removed edge is the
            // whole row.
            for (t, event) in ours
                .iter()
                .filter(|(t, event)| *event == Some(HoldEvent::Removed) && !quoted.contains(&t.id))
            {
                promises.push(trait_row(t, *event, t.head.name.clone(), String::new()));
            }

            // The methods: the type's own first, then the ones a contract asked
            // for, gathered under the promise they answer. The row states the
            // keyword and the name; the signature is its hover words, because 256
            // pixels of mono is a name and not a signature.
            let added: std::collections::HashSet<u32> =
                item.diff.methods_added.iter().copied().collect();
            let mut rows: Vec<(&str, usize)> = item
                .body
                .method_rows
                .iter()
                .enumerate()
                .map(|(at, row)| {
                    (
                        header_promise(&row.section).map_or("", |(promise, _)| promise),
                        at,
                    )
                })
                .collect();
            rows.sort_by_key(|&(promise, at)| (!promise.is_empty(), promise, at));
            methods.extend(rows.into_iter().map(|(promise, at)| {
                let row = &item.body.method_rows[at];
                let own = graph.item(row.mark);
                HoldRow {
                    to: None,
                    decl: own.map_or_else(String::new, |m| m.head.kind.decl_words(&m.head.vis)),
                    name: row.name.clone(),
                    letter: added.contains(&(at as u32)).then_some("A"),
                    word: promise.to_string(),
                    event: None,
                    // The signature first, then where it is written: an impl
                    // block sits wherever it likes, so the file is a fact.
                    hint: Some(match own.and_then(where_written) {
                        Some(at) => format!("{} · {at}", row.sig),
                        None => row.sig.clone(),
                    }),
                    peek: own.and_then(quotable),
                }
            }));
            // What the base wrote for it and this copy does not, quoted from the
            // base edition the way a dropped field is.
            methods.extend(
                item.diff
                    .methods_removed
                    .iter()
                    .map(|(_, name, sig)| HoldRow {
                        to: None,
                        decl: "fn".to_string(),
                        name: name.clone(),
                        letter: None,
                        word: String::new(),
                        event: Some(HoldEvent::Removed.word()),
                        hint: Some(sig.clone()),
                        // Its source left the working copy with it; the base's
                        // signature is all there is to say.
                        peek: None,
                    }),
            );
        } else if let Some(ghost) = graph.ghost(self.id) {
            // A removed type's API left with it: the base wrote these, and the
            // base edition is all there is of them.
            methods.extend(ghost.body.method_rows.iter().map(|(name, sig)| HoldRow {
                to: None,
                decl: "fn".to_string(),
                name: name.clone(),
                letter: None,
                word: String::new(),
                event: None,
                hint: Some(sig.clone()),
                peek: None,
            }));
        }
        Offers { promises, methods }
    }
}

/// One selected datum's sheet: its tier, everything that reaches it,
/// everything it reaches, and what it offers — with the counted residue said
/// out loud in rows, because `named by 12 signatures` is ink this chart
/// refuses to draw and a reviewer must never mistake for silence.
///
/// Two relation headings, not six. Holding, being named in a contract, being
/// named in an API and being used by a body are four kinds of one fact — that
/// something reaches this — and the sheet said each of them in its own
/// heading, which read as four unrelated questions (2026-08-25, user). The
/// kind is a word on the row now; the heading is the direction.
#[component]
pub(super) fn DataSheet(
    graph: CodeGraph,
    path: String,
    item: String,
    /// Which of its rows is open as a quotation, as the URL carries it.
    peek: Option<String>,
) -> Element {
    let data = use_data();
    let sel: Sel = (path.clone(), item.clone());
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, &data.reading())
    }));
    let model = model.read();

    let Some(mark) = model
        .marks
        .iter()
        .find(|m| m.head.path == path && m.head.label == item)
    else {
        // A selection the chart draws no block for is one of two things, and a
        // reviewer stepping back into a URL kept from a wider reading meets the
        // second one: a declaration the survey read and the visibility reading
        // left off. Name the rung it is written at, and offer the one move.
        let off = graph
            .items
            .iter()
            .find(|m| m.head.label == item && graph.path_of(m) == Some(path.as_str()))
            .filter(|m| !data.vis_floor.read().admits(&m.head.vis));
        return rsx! {
            section { class: "plate pointer-events-auto w-full px-4 py-3 sm:w-72",
                if let Some(off) = off {
                    p { class: "font-data text-[11px] leading-relaxed text-ink",
                        "“{item}” is {off.head.kind.decl_words(&off.head.vis)}, and this reading draws {data.vis_floor.read().label()} only."
                    }
                    button {
                        class: "mt-2 font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                        onclick: {
                            let vis = off.head.vis.clone();
                            move |_| {
                                let mut floor = data.vis_floor;
                                floor.set(VisFloor::showing(&vis));
                            }
                        },
                        "draw it"
                    }
                } else {
                    p { class: "font-data text-[11px] text-ink",
                        "Nothing named “{item}” is drawn in {path} on this survey."
                    }
                }
                Link {
                    class: "mt-2 block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::DataOverview {},
                    "← whole chart"
                }
            }
        };
    };

    let at = Anchor::Mark(mark.id);
    let decl = mark.head.kind.decl_words(&mark.head.vis);
    let by_id = model.by_id();

    // One row for any end this chart draws no block for: the declaration's own
    // words, and a quotation of its source, since there is no block to step to.
    let namer_row = |namer: u32, event: Option<HoldEvent>, word: &str| -> Option<HoldRow> {
        let item = graph.item(namer)?;
        let file = graph.file(item.file)?;
        Some(HoldRow {
            to: None,
            decl: item.head.kind.decl_words(&item.head.vis),
            name: item.head.name.clone(),
            letter: None,
            word: word.to_string(),
            event: event.map(HoldEvent::word),
            hint: Some(format!("{}:{}", file.path, item.head.line)),
            peek: Some(peek_key(&file.path, &item.head.label)),
        })
    };

    // Everything that reaches it, in one list. The sheet used to spend three
    // headings on this — `Held by`, `In the contract of`, `In the API of` —
    // and two more on `Holds` and `Uses`, five names for two directions
    // (2026-08-25, user). A relation's *kind* is a word on its row, which is
    // where a word that varies row by row belongs; the heading says only which
    // way the arrow points.
    //
    // The order inside is the strength of the claim: the structure that holds
    // it first — the block it is nested in leads, because the paper says that
    // first — then the signatures that name it, then the bodies that use it,
    // heaviest first.
    let mut used_by: Vec<HoldRow> = Vec::new();
    if let Tier::Nested(holder) = mark.seat.tier
        && let Some(h) = by_id.get(&holder)
    {
        used_by.push(HoldRow {
            to: Some(mark_route(&h.head.path, &h.head.label)),
            decl: h.head.kind.decl_words(&h.head.vis),
            name: h.head.name.clone(),
            letter: h.letter(),
            word: "owns".to_string(),
            event: None,
            hint: None,
            peek: None,
        });
    }
    used_by.extend(
        model.hold_rows(
            model
                .holds
                .iter()
                .filter(|h| h.held == at)
                .map(|h| (&h.holder, h.kind, h.via.as_str(), h.event))
                .collect(),
        ),
    );
    // The holders this reading left off the paper. `Stand::Narrower` says the
    // tier in one sentence; these are the rows behind it, each quoting the
    // holder's own source — an empty list under that sentence would be the
    // sheet withholding names it has (2026-08-25).
    used_by.extend(
        mark.undrawn
            .holders_off
            .iter()
            .filter_map(|&holder| namer_row(holder, None, "owns · off")),
    );
    // The naming ink this chart refuses to draw, in rows: the free
    // declarations whose own signature names it (each a link to its
    // definition, since none has a block here), and the types whose API says
    // the word. One word each — the declaration column already says which is
    // which.
    used_by.extend(
        model
            .naming
            .iter()
            .filter(|n| n.ty == mark.id && !n.from_method)
            .filter_map(|n| namer_row(n.namer, n.event, "signature")),
    );
    used_by.extend(
        model
            .naming
            .iter()
            .filter(|n| n.ty == mark.id && n.from_method)
            .filter_map(|n| match by_id.get(&n.namer) {
                Some(far) => Some(HoldRow {
                    to: Some(mark_route(&far.head.path, &far.head.label)),
                    decl: far.head.kind.decl_words(&far.head.vis),
                    name: far.head.name.clone(),
                    letter: far.letter(),
                    word: "API".to_string(),
                    event: None,
                    hint: None,
                    peek: None,
                }),
                // The visibility reading left the naming type off the paper. An
                // API that says this type's name is a fact about this type, so
                // the row stays and quotes its source instead of stepping to a
                // block that is not drawn.
                None => namer_row(n.namer, n.event, "API"),
            }),
    );

    // What it reaches, the same way round: the blocks nested inside it, then
    // the structure it holds, then the marks its own bodies use.
    let mut uses: Vec<HoldRow> = mark
        .seat
        .kids
        .iter()
        .filter_map(|kid| by_id.get(kid))
        .map(|k| HoldRow {
            to: Some(mark_route(&k.head.path, &k.head.label)),
            decl: k.head.kind.decl_words(&k.head.vis),
            name: k.head.name.clone(),
            letter: k.letter(),
            word: "owns".to_string(),
            event: None,
            hint: None,
            peek: None,
        })
        .collect();
    uses.extend(
        model.hold_rows(
            model
                .holds
                .iter()
                .filter(|h| h.holder == at)
                .map(|h| (&h.held, h.kind, h.via.as_str(), h.event))
                .collect(),
        ),
    );

    // The implementation ink, both ways round — and then the same ink from
    // the ends this chart draws no block for (2026-08-23, user). A free
    // function's body is a real item, so it gets a row like any other and
    // names the file and line it is written on; the sheet used to spend a
    // sentence counting them instead, which named nothing a reviewer could go
    // and read.
    let unseen_row = |end: &Unseen| -> Option<(u32, HoldRow)> {
        let item = graph.item(end.item)?;
        let file = graph.file(item.file)?;
        Some((
            end.count,
            HoldRow {
                to: None,
                decl: item.head.kind.decl_words(&item.head.vis),
                name: item.head.name.clone(),
                letter: None,
                word: plural(end.count as usize, "ref"),
                event: None,
                hint: Some(format!("{}:{}", file.path, item.head.line)),
                peek: Some(peek_key(&file.path, &item.head.label)),
            },
        ))
    };
    used_by.extend(ranked(
        model
            .uses_rows(
                model
                    .ties
                    .iter()
                    .filter(|t| t.def == at)
                    .map(|t| (&t.user, t.count, t.rows.as_slice()))
                    .collect(),
            )
            .into_iter()
            .chain(mark.undrawn.used_by.iter().filter_map(unseen_row))
            .collect(),
    ));
    uses.extend(ranked(
        model
            .uses_rows(
                model
                    .ties
                    .iter()
                    .filter(|t| t.user == at)
                    .map(|t| (&t.def, t.count, t.rows.as_slice()))
                    .collect(),
            )
            .into_iter()
            .chain(mark.undrawn.unseen_uses.iter().filter_map(unseen_row))
            .collect(),
    ));

    let used_by = per_end(used_by);
    let uses = per_end(uses);

    let Offers { promises, methods } = mark.offers(&graph);

    // The selection's own diff rows, exactly as the block draws them.
    let change_rows: Vec<(&'static str, String, bool)> = mark
        .rows
        .fields
        .iter()
        .chain(mark.rows.variants.iter())
        .filter_map(|row| {
            let mk = row.state.marker()?;
            let text = if row.name.is_empty() {
                row.decl.clone()
            } else {
                row.written()
            };
            Some((mk, text, row.state == RowState::Removed))
        })
        .collect();
    let change_line = if mark.state.ghost {
        Some("removed since the base — this block quotes the base edition.")
    } else if mark.state.delta == Delta::Added {
        Some("added since the base.")
    } else {
        None
    };

    // The blast radius in one line: how much further than its direct holders
    // a shape change travels, the nesting walked like any holding — plus the
    // signatures that would have to change with what it reaches.
    let direct: std::collections::HashSet<Anchor> = model
        .pairs
        .iter()
        .filter(|(held, _)| *held == at)
        .map(|(_, holder)| *holder)
        .collect();
    let radius = upstream(&model.pairs, at);
    let beyond = radius.iter().filter(|a| !direct.contains(a)).count();
    let reached: std::collections::HashSet<u32> = radius
        .iter()
        .chain(std::iter::once(&at))
        .filter_map(|a| match a {
            Anchor::Mark(id) => Some(*id),
            _ => None,
        })
        .collect();
    let signatures: std::collections::HashSet<u32> = model
        .naming
        .iter()
        .filter(|n| n.event != Some(HoldEvent::Removed) && reached.contains(&n.ty))
        .map(|n| n.namer)
        .collect();
    let reach = match (beyond, signatures.len()) {
        (0, 0) => String::new(),
        (0, sigs) => format!("reaches {}", plural(sigs, "signature")),
        (types, 0) => format!("reaches {} upstream", plural(types, "more type")),
        (types, sigs) => format!(
            "reaches {} upstream and {}",
            plural(types, "more type"),
            plural(sigs, "signature")
        ),
    };

    // One rule opens what the type offers, so the pair reads as one section
    // however few of it there is to say.
    let promised = !promises.is_empty();
    let tier = mark.tier_line();

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3 pb-2",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::DataOverview {},
                    "← whole chart"
                }
                h2 { class: "mt-1.5 flex items-baseline gap-1.5 font-data text-[15px]",
                    span { class: "shrink-0 text-[11px] text-ink-soft", "{decl}" }
                    span { class: "truncate font-semibold text-ink", "{mark.head.name}" }
                    if let Some(letter) = mark.letter() {
                        span {
                            class: "shrink-0 font-bold text-flare",
                            title: match letter {
                                "A" => "added since the diff base",
                                "D" => "removed since the diff base — quoted from the base edition",
                                _ => "declaration changed since the diff base",
                            },
                            "{letter}"
                        }
                    }
                }
                p { class: "mt-0.5 font-data text-[9.5px] text-ink-soft", "{mark.locator()}" }
                p { class: "mt-1 font-data text-[10px] leading-relaxed text-ink", "{tier}" }
                if let Some(line) = change_line {
                    p { class: "mt-1 font-data text-[10px] leading-relaxed text-flare", "{line}" }
                }
                if !change_rows.is_empty() {
                    div { class: "mt-1 space-y-0.5 font-data text-[10px] leading-snug",
                        for (i , (mk , text , gone)) in change_rows.iter().enumerate() {
                            p { key: "{i}", class: "flex items-baseline gap-1",
                                span { class: "shrink-0 font-bold text-flare", "{mk}" }
                                span {
                                    class: if *gone { "text-ink-soft line-through" } else { "text-ink" },
                                    "{text}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                h3 {
                    class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    title: "everything that reaches it — the types that hold it, the signatures that name it, the bodies that use it; each row's own word says which",
                    "Used by ({used_by.len()})"
                }
                if used_by.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        "nothing in the workspace reaches it."
                    }
                } else {
                    HoldList { sel: sel.clone(), open: peek.clone(), rows: used_by }
                }
                if !reach.is_empty() {
                    p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                        if mark.state.ghost {
                            "the removal {reach}."
                        } else {
                            "a shape change here {reach}."
                        }
                    }
                }
                h3 {
                    class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    title: "everything it reaches — the state it holds, and the marks its own bodies use",
                    "Uses ({uses.len()})"
                }
                if uses.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        "it reaches nothing in the workspace."
                    }
                } else {
                    HoldList { sel: sel.clone(), open: peek.clone(), rows: uses }
                }
                if !promises.is_empty() {
                    h3 {
                        class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        title: "the trait impls written for it by hand, wherever in the workspace they are written; a derive stands in the type's own source and is not one of these",
                        "Implements ({promises.len()})"
                    }
                    HoldList { sel: sel.clone(), open: peek.clone(), rows: promises }
                }
                if !methods.is_empty() {
                    h3 {
                        class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        class: if !promised { "border-t border-ink-line pt-3" },
                        title: "every method written for it anywhere in the workspace, its own first, then the ones a contract asked for; each row's hover words are its signature as written",
                        "Methods ({methods.len()})"
                    }
                    HoldList { sel: sel.clone(), open: peek.clone(), rows: methods }
                }
            }
            if mark.state.ghost {
                div { class: "border-t border-ink-line px-4 py-2",
                    p { class: "font-data text-[9.5px] text-ink-soft",
                        "its definition left the working copy."
                    }
                }
            }
        }
    }
}

/// Find a datum by name. Marks only — this altitude's places are types and
/// statics — ranked the way the code search ranks: a prefix match is what the
/// reviewer meant, then whatever more of the workspace leans on.
#[component]
pub(super) fn DataSearch(graph: CodeGraph) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let nav = use_navigator();
    let data = use_data();
    // A reviewer who names a declaration has asked for it, so the visibility
    // reading widens to the rung that draws it rather than landing them on a
    // sheet that says the chart declines to. The slider moves with it, in
    // sight, so the reading is never changed behind the reader's back.
    let widen = move |vis: &Vis| {
        let mut floor = data.vis_floor;
        if !floor.peek().admits(vis) {
            floor.set(VisFloor::showing(vis));
        }
    };

    // How one hit ranks: a prefix match is what the reviewer meant, then
    // whatever more of the workspace leans on, then the name itself.
    type Rank = (bool, std::cmp::Reverse<u32>, String);
    let results = use_memo(move || {
        let q = query().trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let datum = |kind: crate::graph::data::ItemKind| {
            matches!(
                kind,
                crate::graph::data::ItemKind::Struct
                    | crate::graph::data::ItemKind::Enum
                    | crate::graph::data::ItemKind::Union
                    | crate::graph::data::ItemKind::Static
            )
        };
        let mut hits: Vec<(Rank, (ItemMark, String))> = graph
            .items
            .iter()
            .filter(|m| m.parent.is_none() && datum(m.head.kind))
            .filter(|m| m.head.name.to_lowercase().contains(&q))
            .filter_map(|m| {
                let path = graph.path_of(m)?.to_string();
                Some((
                    (
                        !m.head.name.to_lowercase().starts_with(&q),
                        std::cmp::Reverse(m.reach.fan_in),
                        m.head.name.clone(),
                    ),
                    (m.clone(), path),
                ))
            })
            .collect();
        hits.sort_by(|a, b| a.0.cmp(&b.0));
        hits.truncate(10);
        hits.into_iter().map(|(_, hit)| hit).collect::<Vec<_>>()
    });

    rsx! {
        div { class: "pointer-events-auto relative w-full",
            input {
                id: "data-search",
                class: "plate w-full px-3 py-1.5 font-data text-[11px] text-ink placeholder:text-ink-soft focus:outline-none",
                r#type: "search",
                placeholder: "find a struct, enum or static…   /",
                autocomplete: "off",
                spellcheck: "false",
                "aria-label": "Find a struct, enum, union or static",
                value: "{query}",
                oninput: move |e| {
                    query.set(e.value());
                    active.set(0);
                },
                onkeydown: move |e| {
                    let n = results().len();
                    match e.key() {
                        Key::ArrowDown if n > 0 => {
                            e.prevent_default();
                            active.set((active() + 1) % n);
                        }
                        Key::ArrowUp if n > 0 => {
                            e.prevent_default();
                            active.set((active() + n - 1) % n);
                        }
                        Key::Enter => {
                            if let Some((m, path)) = results().get(active().min(n.saturating_sub(1)))
                            {
                                widen(&m.head.vis);
                                nav.push(mark_route(path, &m.head.label));
                                query.set(String::new());
                            }
                        }
                        Key::Escape => query.set(String::new()),
                        _ => {}
                    }
                },
            }
            if !query().trim().is_empty() {
                if results().is_empty() {
                    div { class: "plate absolute left-0 right-0 top-full z-20 mt-1 px-3 py-2",
                        p { class: "font-data text-[10px] text-ink-soft", "no matches" }
                    }
                } else {
                    ul { class: "plate absolute left-0 right-0 top-full z-20 mt-1 max-h-72 overflow-auto py-1",
                        for (i , (m , path)) in results().into_iter().enumerate() {
                            li { key: "{path}|{m.head.label}",
                                Link {
                                    to: mark_route(&path, &m.head.label),
                                    class: if i == active() { "flex w-full items-baseline gap-1.5 px-2.5 py-1 bg-ink/5" } else { "flex w-full items-baseline gap-1.5 px-2.5 py-1 hover:bg-ink/5" },
                                    onclick: {
                                        let vis = m.head.vis.clone();
                                        move |_| {
                                            widen(&vis);
                                            query.set(String::new());
                                        }
                                    },
                                    span { class: "shrink-0 font-data text-[9.5px] text-ink-soft",
                                        "{m.head.kind.words()}"
                                    }
                                    span { class: "truncate font-data text-[11px] text-ink", "{m.head.name}" }
                                    span { class: "ml-auto shrink-0 truncate font-data text-[9px] text-ink-soft",
                                        "{path}:{m.head.line}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{bare_trait, header_promise};
    use crate::graph::data::{
        BaseBody, DeclHead, GhostAt, GhostDecl, HoldEvent, ImplEdge, ItemKind, MethodRow, Vis,
    };
    use crate::views::data::model::tests::{build, by_name, graph, mark};

    #[test]
    fn an_impl_header_says_what_it_promises_and_for_what() {
        assert_eq!(header_promise("impl Clone for Vis"), Some(("Clone", "Vis")));
        assert_eq!(
            header_promise("impl From<Option<ast::Visibility>> for Vis"),
            Some(("From<Option<ast::Visibility>>", "Vis"))
        );
        // The self type is what the header ends in, whatever `for` stands
        // inside the trait's own arguments.
        assert_eq!(
            header_promise("impl Held for Frame<'a>"),
            Some(("Held", "Frame<'a>"))
        );
        // An inherent impl promises nothing, and a trait's own header is not
        // an impl at all: both are a type's own API.
        assert_eq!(header_promise("impl DataModel"), None);
        assert_eq!(header_promise("trait Held"), None);
    }

    #[test]
    fn a_promise_matches_a_trait_mark_on_its_bare_name() {
        assert_eq!(bare_trait("Clone"), "Clone");
        assert_eq!(bare_trait("fmt::Display"), "Display");
        assert_eq!(bare_trait("From<&str>"), "From");
        assert_eq!(bare_trait("crate::views::data::model::Held<'a>"), "Held");
        // A mark's own name carries its inline-module path; the survey's
        // structural diff bares it the same way.
        assert_eq!(bare_trait("tests::Held"), "Held");
    }

    #[test]
    fn a_type_lists_the_contracts_it_promises_and_the_methods_written_for_it() {
        let mut wire = mark(0, 0, "Wire", ItemKind::Struct);
        // Two promises as their headers write them: one the workspace declares
        // itself, one from outside it.
        wire.reach.impls = vec![
            "impl Clone for Wire".to_string(),
            "impl Held for Wire".to_string(),
        ];
        wire.body.method_rows = vec![
            MethodRow {
                name: "note".to_string(),
                sig: "pub(crate) fn note(&self) -> String".to_string(),
                mark: 2,
                section: "impl Wire".to_string(),
            },
            MethodRow {
                name: "clone".to_string(),
                sig: "fn clone(&self) -> Self".to_string(),
                mark: 3,
                section: "impl Clone for Wire".to_string(),
            },
        ];
        wire.diff.methods_added = vec![0];
        wire.diff.methods_removed = vec![(2, "gone".to_string(), "fn gone(&self)".to_string())];
        let mut note = mark(2, 0, "Wire::note", ItemKind::Fn);
        note.head.vis = Vis::Crate;
        note.parent = Some(0);
        let mut clone = mark(3, 0, "Wire::clone", ItemKind::Fn);
        clone.parent = Some(0);
        let mut g = graph(
            vec![wire, mark(1, 0, "Held", ItemKind::Trait), note, clone],
            Vec::new(),
        );
        g.implements = vec![ImplEdge {
            trait_mark: 1,
            ty: 0,
            event: Some(HoldEvent::Added),
        }];
        let model = build(&g);
        let offers = by_name(&model, "Wire").offers(&g);

        // The promises, in the order the headers stand. A foreign trait has no
        // mark, so its row is a name and nothing else; the workspace's own
        // names where the contract is written, and this epoch's promise takes
        // the flare.
        let names: Vec<&str> = offers.promises.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Clone", "Held"]);
        assert!(offers.promises[0].hint.is_none());
        assert_eq!(offers.promises[0].event, None);
        assert_eq!(
            offers.promises[1].hint.as_deref(),
            Some("src/graph/data.rs:2")
        );
        assert_eq!(offers.promises[1].decl, "trait");
        assert_eq!(offers.promises[1].event, Some("added"));
        // The self type is not restated where it is simply this mark.
        assert!(offers.promises.iter().all(|r| r.word.is_empty()));

        // Its own method first, then the one a contract asked for under the
        // promise it answers, then what the base wrote and this copy dropped.
        let rows: Vec<(&str, &str)> = offers
            .methods
            .iter()
            .map(|r| (r.name.as_str(), r.word.as_str()))
            .collect();
        assert_eq!(rows, [("note", ""), ("clone", "Clone"), ("gone", "")]);
        assert_eq!(offers.methods[0].decl, "pub(crate) fn");
        assert_eq!(offers.methods[0].letter, Some("A"));
        // The signature as written, then where it is written: an impl block
        // sits wherever it likes.
        assert_eq!(
            offers.methods[0].hint.as_deref(),
            Some("pub(crate) fn note(&self) -> String · src/graph/data.rs:3")
        );
        assert_eq!(offers.methods[2].event, Some("removed"));
    }

    #[test]
    fn a_ghosts_api_left_with_it_and_the_sheet_quotes_the_base() {
        let mut g = graph(vec![mark(0, 0, "Wire", ItemKind::Struct)], Vec::new());
        g.push_ghost(
            GhostAt {
                path: "src/graph/data.rs".to_string(),
                krate: "slope".to_string(),
            },
            GhostDecl {
                head: DeclHead {
                    name: "Nut".to_string(),
                    label: String::new(),
                    kind: ItemKind::Struct,
                    vis: Vis::Crate,
                    line: 12,
                },
                body: BaseBody {
                    method_rows: vec![("tighten".to_string(), "fn tighten(&mut self)".to_string())],
                    ..BaseBody::default()
                },
            },
        );
        let model = build(&g);
        let offers = by_name(&model, "Nut").offers(&g);
        assert!(offers.promises.is_empty());
        assert_eq!(offers.methods.len(), 1);
        assert_eq!(offers.methods[0].name, "tighten");
        // No place to name: the definition left the working copy, so the row
        // is the base's signature and nothing more.
        assert_eq!(
            offers.methods[0].hint.as_deref(),
            Some("fn tighten(&mut self)")
        );
    }
}
