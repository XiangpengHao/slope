//! Surface-altitude furniture: the cartouche, the two reading toggles, the
//! selection sheet, and the legend. The same engraved plates the other two
//! altitudes wear.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, Delta, HoldEvent, HoldKind};
use crate::views::codemap::chrome::{
    Altitude, AltitudeSwitch, Gestures, SurveyLimits, UsageRow, decl_words, plural,
};
use crate::views::codemap::{Doors, RefDir, item_route, use_code};
use crate::views::surface::model::{Anchor, RowState, SurfaceFacts, SurfaceModel, upstream};
use crate::views::surface::{mark_route, mod_route};

/// Which modules the diff landed in, each by its whole path
/// (`views::surface`), in plain words. The chart shows a reviewer where the
/// amber is; the cartouche says it out loud, because that one sentence is the
/// answer to why they climbed to this altitude.
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

/// The structural diff's own line: only what happened, in git's order. The
/// counts cover types and statics alike, so no noun — the marks are right
/// there on the chart.
fn diff_words(facts: &SurfaceFacts) -> String {
    let mut parts: Vec<String> = Vec::new();
    if facts.added > 0 {
        parts.push(format!("{} added", facts.added));
    }
    if facts.removed > 0 {
        parts.push(format!("{} removed", facts.removed));
    }
    if facts.changed > 0 {
        parts.push(format!("{} changed", facts.changed));
    }
    parts.join(" · ")
}

/// The chart's title block: what surface the workspace publishes, what the diff
/// moved, and the reading control for its body dependence.
#[component]
pub(crate) fn SurfaceCartouche(
    facts: SurfaceFacts,
    workspace: String,
    diff_line: String,
) -> Element {
    let insight = insight(&facts.changed_modules);
    rsx! {
        section { class: "plate pointer-events-auto",
            div { class: "px-4 pt-3 pb-2",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{workspace}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "{plural(facts.structs, \"struct\")} · {plural(facts.enums, \"enum\")} · {plural(facts.traits, \"trait\")}"
                }
                p { class: "mt-0.5 font-data text-[10.5px] text-ink-soft",
                    "{plural(facts.fns, \"fn\")} · {plural(facts.consts, \"const\")} · {plural(facts.aliases, \"alias\")}"
                }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Surface }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if !diff_words(&facts).is_empty() {
                        p { class: "text-flare", "{diff_words(&facts)}" }
                        if let Some(insight) = insight {
                            p { class: "text-ink-soft", "{insight}" }
                        }
                    } else {
                        p { class: "text-ink-soft", "no shape changes since the base" }
                    }
                }
            }
            RefToggle {}
            DoorToggle {}
        }
    }
}

/// Which reading of the chart's uses edges is drawn. It rides on the
/// cartouche because it acts on the whole plate, and it is the same reading the
/// code map is set to — one reviewer, one question, at either altitude.
#[component]
pub(crate) fn RefToggle() -> Element {
    let code = use_code();
    let current = *code.ref_dir.read();
    let seg = |label: &'static str, hint: &'static str, val: RefDir| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                title: hint,
                onclick: move |_| {
                    let mut dir = code.ref_dir;
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
            "aria-label": "which reading of the chart's body dependence is drawn",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "references"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("uses", "each mark's heaviest body dependence out — what its own code leans on", RefDir::Uses)}
                {seg("used by", "each mark's heaviest body dependence in — whose code leans on it", RefDir::UsedBy)}
                {seg("both", "every body dependence between two marks, unthinned", RefDir::Both)}
            }
        }
    }
}

/// Which doors earn a block — the setting that decides what this view *is*,
/// since the view is the surface that crosses the chosen door. It rides on the
/// cartouche under the reference reading because it acts on the whole plate
/// the same way: both are readings of one survey, so moving either re-seats
/// the chart without surveying the workspace again.
#[component]
fn DoorToggle() -> Element {
    let code = use_code();
    let current = *code.doors.read();
    let seg = |label: &'static str, hint: &'static str, val: Doors| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                title: hint,
                onclick: move |_| {
                    let mut doors = code.doors;
                    doors.set(val);
                },
                "{label}"
            }
        }
    };
    rsx! {
        div {
            class: "border-t border-ink-line px-4 py-1.5",
            role: "group",
            "aria-label": "which contracts the chart draws a block for",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "visibility"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("pub", "only what is visible outside its crate is drawn; everything else folds to its module's count", Doors::Pub)}
                {seg("pub(crate)", "crate-visible contracts are drawn too; only what has no `pub` at all folds", Doors::Crate)}
                {seg("private", "everything is drawn, no-`pub` items included; nothing folds for visibility", Doors::All)}
            }
        }
    }
}

/// A hold's kind, in its own lowercase word, for a sheet row's far column.
/// The wrapper's word wins where the walk met one — `Arc` says more than
/// `shares` — and the kind speaks only for a plain hold.
fn hold_word(kind: HoldKind, via: &str) -> String {
    if !via.is_empty() {
        return via.to_string();
    }
    match kind {
        HoldKind::Owns => "owns",
        HoldKind::Shares => "shares",
        HoldKind::Borrows => "borrows",
        HoldKind::Dyn => "dyn",
        HoldKind::Implements => "implements",
    }
    .to_string()
}

/// One row of the sheet's holds lists: a drawn type (a link that re-centers
/// the selection on it), or a frame's counted fold row, which is words.
#[derive(Clone, PartialEq)]
pub(crate) struct HoldRow {
    pub(crate) to: Option<Route>,
    pub(crate) decl: String,
    pub(crate) name: String,
    pub(crate) letter: Option<&'static str>,
    pub(crate) word: String,
    /// The relation's own diff event, in its word.
    pub(crate) event: Option<&'static str>,
}

/// The rows one side of the selection draws, from each hold's far end. A
/// fold-row end names its count and its module instead of a type — the
/// chart's own words for what it does not draw.
fn hold_rows(
    model: &SurfaceModel,
    holds: Vec<(&Anchor, HoldKind, &str, Option<HoldEvent>)>,
) -> Vec<HoldRow> {
    let by_id: std::collections::HashMap<u32, &crate::views::surface::model::SurfaceMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();
    holds
        .into_iter()
        .map(|(anchor, kind, via, event)| {
            let event = match event {
                Some(HoldEvent::Added) => Some("added"),
                Some(HoldEvent::Removed) => Some("removed"),
                None => None,
            };
            match anchor {
                Anchor::Mark(id) => {
                    let mark = by_id.get(id);
                    HoldRow {
                        to: mark.map(|m| mark_route(&m.path, &m.label)),
                        decl: mark.map(|m| decl_words(m.vis, m.kind)).unwrap_or_default(),
                        name: mark.map(|m| m.name.clone()).unwrap_or_default(),
                        letter: mark.and_then(|m| m.letter()),
                        word: hold_word(kind, via),
                        event,
                    }
                }
                Anchor::Private(frame) | Anchor::Mod(frame) => {
                    let frame = &model.frames[*frame as usize];
                    let count = match anchor {
                        Anchor::Private(_) => format!(
                            "+ {}",
                            plural(frame.private as usize, model.doors.fold_word())
                        ),
                        // A module the reviewer folded: the row counts the
                        // whole boundary, and the boundary can be selected,
                        // so this row is a link where the other two are not.
                        _ => format!("+ {}", plural(frame.packed as usize, "item")),
                    };
                    // The whole path, not the chip on the border: this row is
                    // read off the paper, where a nested `mod surface` names
                    // three different modules.
                    let place = match frame.module.is_empty() {
                        true => frame.krate.clone(),
                        false => format!("mod {}", frame.words()),
                    };
                    HoldRow {
                        to: matches!(anchor, Anchor::Mod(_)).then(|| mod_route(frame.key())),
                        decl: String::new(),
                        name: format!("{count} · {place}"),
                        letter: None,
                        word: hold_word(kind, via),
                        event,
                    }
                }
            }
        })
        .collect()
}

/// One end of a uses edge as the sheet reads it: the far mark, how many
/// references run between them, and which of the def's method rows they name.
type UsesRow<'a> = (&'a Anchor, u32, &'a [(String, u32)]);

/// The rows of the other family: a body leaning on a mark. Both ends of one
/// of these is a drawn mark — what the chart could not land is counted on the
/// mark instead — so every row is a link. Between two functions the word is
/// `calls`, because that is what a reader would say; anywhere else a body
/// merely names what it names.
fn uses_rows(model: &SurfaceModel, selected_is_fn: bool, rows: Vec<UsesRow<'_>>) -> Vec<HoldRow> {
    let by_id: std::collections::HashMap<u32, &crate::views::surface::model::SurfaceMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();
    rows.into_iter()
        .filter_map(|(anchor, count, clauses)| {
            let Anchor::Mark(id) = anchor else {
                return None;
            };
            let far = by_id.get(id)?;
            let mut word = if selected_is_fn && far.is_fn() {
                plural(count as usize, "call")
            } else {
                plural(count as usize, "reference")
            };
            // Which clause of the API it leans on, heaviest first: a call the
            // survey resolved to a method is a call to that row, and saying
            // which one is the difference between "uses this type" and "uses
            // this one method of it".
            if !clauses.is_empty() {
                let named: Vec<&str> = clauses.iter().take(2).map(|(r, _)| r.as_str()).collect();
                word = format!("{word} · {}", named.join(", "));
            }
            Some(HoldRow {
                to: Some(mark_route(&far.path, &far.label)),
                decl: decl_words(far.vis, far.kind),
                name: far.name.clone(),
                letter: far.letter(),
                word,
                event: None,
            })
        })
        .collect()
}

/// One chunked list of hold rows: the first eight, then a typographic
/// "show all n".
#[component]
pub(crate) fn HoldList(rows: Vec<HoldRow>) -> Element {
    let mut all = use_signal(|| false);
    let total = rows.len();
    let shown = if all() || total <= 8 { total } else { 8 };
    rsx! {
        ul { class: "mt-1",
            for (i , row) in rows.iter().take(shown).enumerate() {
                li { key: "{i}",
                    if let Some(to) = row.to.clone() {
                        Link {
                            class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] hover:bg-ink/5",
                            to,
                            if !row.decl.is_empty() {
                                span { class: "shrink-0 text-ink-soft", "{row.decl}" }
                            }
                            // The name is the one thing the row exists to
                            // state: it takes the row's free width, and the
                            // count-and-clause column truncates against a
                            // hard cap before the name gives up a pixel.
                            span { class: "flex-1 shrink-0 font-medium text-ink", "{row.name}" }
                            if let Some(letter) = row.letter {
                                span { class: "shrink-0 font-bold text-flare", "{letter}" }
                            }
                            span { class: "max-w-[45%] shrink-0 truncate text-right text-[9px] text-ink-soft",
                                "{row.word}"
                            }
                            if let Some(event) = row.event {
                                span { class: "shrink-0 text-[9px] text-flare", "{event}" }
                            }
                        }
                    } else {
                        span { class: "flex w-full items-baseline gap-1.5 px-1 py-0.5 font-data text-[10.5px] text-ink-soft",
                            span { class: "min-w-0 flex-1 truncate", "{row.name}" }
                            span { class: "max-w-[45%] shrink-0 truncate text-right text-[9px]",
                                "{row.word}"
                            }
                            if let Some(event) = row.event {
                                span { class: "shrink-0 text-[9px] text-flare", "{event}" }
                            }
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

/// One selected mark's sheet: who holds it, which contracts name it, what it
/// holds, and the one step further to its definition. A function has only the
/// last of those — a signature names types; nothing names a function back. The
/// chart keeps the selection's blast radius inked; this plate says the same
/// thing in rows a reader can follow.
#[component]
pub(crate) fn SurfaceSheet(graph: CodeGraph, path: String, item: String) -> Element {
    let code = use_code();
    // The sheet reads holding structure, never the tie reading, so that
    // toggle is peeked: it moves nothing on this plate. The doors are read —
    // they decide whether a held type is a link here or part of a count.
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        SurfaceModel::build(
            &graph,
            *code.ref_dir.peek(),
            *code.doors.read(),
            &code.folds.read(),
        )
    }));
    let model = model.read();

    let Some(mark) = model
        .marks
        .iter()
        .find(|m| m.path == path && m.label == item)
    else {
        return rsx! {
            section { class: "plate pointer-events-auto w-full px-4 py-3 sm:w-72",
                p { class: "font-data text-[11px] text-ink",
                    "Nothing named “{item}” is drawn in {path} on this survey."
                }
                Link {
                    class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::SurfaceOverview {},
                    "← whole chart"
                }
            }
        };
    };

    let at = Anchor::Mark(mark.id);
    let decl = decl_words(mark.vis, mark.kind);
    // Naming is not keeping. A function's signature, and a method row of some
    // other type, both *name* this mark without holding any of it — so each
    // gets a section of its own and "held by" never quietly means "mentioned
    // by". Three ways in, and two ways out.
    let contract = |anchor: &Anchor| match anchor {
        Anchor::Mark(id) => model.marks.iter().any(|m| m.id == *id && m.is_fn()),
        _ => false,
    };
    // `out` picks the far end: the held mark for an edge leaving this one,
    // the holder for an edge arriving.
    let rows_of = |want: fn(&crate::views::surface::model::Hold, Anchor) -> bool, out: bool| {
        hold_rows(
            &model,
            model
                .holds
                .iter()
                .filter(|h| want(h, at))
                .map(|h| {
                    let far = if out { &h.held } else { &h.holder };
                    (far, h.kind, h.via.as_str(), h.event)
                })
                .collect(),
        )
    };
    let held_by: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.held == at && !h.from_method && !contract(&h.holder))
            .map(|h| (&h.holder, h.kind, h.via.as_str(), h.event))
            .collect(),
    );
    let contracts: Vec<HoldRow> = hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.held == at && contract(&h.holder))
            .map(|h| (&h.holder, h.kind, h.via.as_str(), h.event))
            .collect(),
    );
    let in_api: Vec<HoldRow> = rows_of(
        |h, at| h.held == at && h.from_method && h.kind != HoldKind::Implements,
        false,
    );
    let holds: Vec<HoldRow> = rows_of(
        |h, at| h.holder == at && !h.from_method && h.kind != HoldKind::Implements,
        true,
    );
    let api_names: Vec<HoldRow> = rows_of(
        |h, at| h.holder == at && h.from_method && h.kind != HoldKind::Implements,
        true,
    );
    // Promising a contract runs trait → type, so the type reads it going out
    // and the trait reads it coming in. Neither is holding.
    let implements: Vec<HoldRow> = rows_of(
        |h, at| h.holder == at && h.kind == HoldKind::Implements,
        false,
    );
    let implemented_by: Vec<HoldRow> =
        rows_of(|h, at| h.held == at && h.kind == HoldKind::Implements, true);
    // The other ink: bodies. What leans on this mark from the inside, and
    // what its own inside leans on — the same edges the chart draws dashed.
    let used_by: Vec<HoldRow> = uses_rows(
        &model,
        mark.is_fn(),
        model
            .ties
            .iter()
            .filter(|t| t.def == at)
            .map(|t| (&t.user, t.count, t.rows.as_slice()))
            .collect(),
    );
    let uses: Vec<HoldRow> = uses_rows(
        &model,
        mark.is_fn(),
        model
            .ties
            .iter()
            .filter(|t| t.user == at)
            .map(|t| (&t.def, t.count, t.rows.as_slice()))
            .collect(),
    );
    // The selection's own diff, in words: its letter's sentence, then every
    // added and dropped row exactly as the block draws them.
    let change_rows: Vec<(&'static str, String, bool)> = mark
        .fields
        .iter()
        .chain(mark.variants.iter())
        .filter_map(|row| {
            let mk = row.state.marker()?;
            let text = if row.name.is_empty() {
                row.decl.clone()
            } else {
                row.written()
            };
            Some((mk, text, row.state == RowState::Removed))
        })
        // A method row is quoted whole, the way the block quotes it: its name
        // is already the first word of the signature.
        .chain(mark.methods.iter().filter_map(|row| {
            let mk = row.state.marker()?;
            Some((mk, row.decl.clone(), row.state == RowState::Removed))
        }))
        .collect();
    let change_line = if mark.ghost {
        Some("removed since the base — this block quotes the base edition.")
    } else if mark.delta == Delta::Added {
        Some("added since the base.")
    } else {
        None
    };
    // The blast radius in one line: how much further than its direct holders
    // a change to this shape travels. Contracts are counted apart from shapes
    // — a signature that has to change is not another type that has to.
    let pairs: Vec<(Anchor, Anchor)> = model.holds.iter().map(|h| (h.held, h.holder)).collect();
    let direct: std::collections::HashSet<Anchor> = model
        .holds
        .iter()
        .filter(|h| h.held == at)
        .map(|h| h.holder)
        .collect();
    let beyond: Vec<Anchor> = upstream(&pairs, at)
        .into_iter()
        .filter(|a| matches!(a, Anchor::Mark(_)) && !direct.contains(a))
        .collect();
    let beyond_fns = beyond.iter().filter(|a| contract(a)).count();
    let reach = match (beyond.len() - beyond_fns, beyond_fns) {
        (0, 0) => String::new(),
        (0, fns) => plural(fns, "more signature"),
        (types, 0) => plural(types, "more type"),
        (types, fns) => format!(
            "{} and {}",
            plural(types, "more type"),
            plural(fns, "more signature")
        ),
    };
    // What the empty "held by" says. A static is a root by nature; a type its
    // functions name enters the chart through them; a type only bodies reach
    // says that instead; and a mark nothing reaches at all says the whole
    // truth, because that verdict is what a reviewer deletes code on.
    let root_line = if mark.is_static() {
        "no type holds it — a root."
    } else if !contracts.is_empty() || !in_api.is_empty() {
        "no type holds it — it enters through the signatures below."
    } else if !implemented_by.is_empty() {
        "no type holds it — what reaches it are the types that promised it, below."
    } else if !used_by.is_empty() {
        "no type holds it — only bodies reach it, below."
    } else if mark.unseen_users > 0 {
        "nothing the chart draws holds, names, or uses it."
    } else {
        "nothing in the workspace reaches it: no type holds it, no signature names it, no body uses it."
    };
    // What the family could not land. Every reference the survey resolved is
    // drawn now — inside a file as well as across — so the only ones left
    // over are the ones with nowhere to land: a folded mark's, or an item the
    // chart gives no mark at all. Said out loud, because the difference
    // between "nothing uses this" and "nothing I drew uses this" is the whole
    // question a reviewer asks of a quiet contract.
    let unseen_line = |n: u32, way: &str| {
        format!(
            "{} {way} code this chart does not draw — what the visibility setting or the budget folded, and items with no mark of their own.",
            plural(n as usize, "reference")
        )
    };

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3 pb-2",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::SurfaceOverview {},
                    "← whole chart"
                }
                h2 { class: "mt-1.5 flex items-baseline gap-1.5 font-data text-[15px]",
                    span { class: "shrink-0 text-[11px] text-ink-soft", "{decl}" }
                    span { class: "truncate font-semibold text-ink", "{mark.name}" }
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
                // A function publishes a contract and nothing holds it, so its
                // surface reading is one section — and then the other ink: who
                // calls it, and what its own body leans on.
                if mark.is_fn() {
                    h3 { class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Signature names ({holds.len()})"
                    }
                    if holds.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "its signature names no workspace types."
                        }
                    } else {
                        HoldList { rows: holds }
                    }
                } else {
                    h3 { class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Held by ({held_by.len()})"
                    }
                    if held_by.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft", "{root_line}" }
                    } else {
                        HoldList { rows: held_by }
                    }
                    if !contracts.is_empty() {
                        h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                            "In the contract of ({contracts.len()})"
                        }
                        HoldList { rows: contracts }
                    }
                    // A method row of another type names it. That type keeps
                    // none of it: its API merely says the word.
                    if !in_api.is_empty() {
                        h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                            "In the API of ({in_api.len()})"
                        }
                        HoldList { rows: in_api }
                    }
                    if !reach.is_empty() {
                        p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                            if mark.ghost {
                                "the removal reaches {reach} upstream."
                            } else {
                                "a shape change here reaches {reach} upstream."
                            }
                        }
                    }
                    h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Holds ({holds.len()})"
                    }
                    if holds.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "holds no workspace types."
                        }
                    } else {
                        HoldList { rows: holds }
                    }
                    if !implements.is_empty() {
                        h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                            "Implements ({implements.len()})"
                        }
                        HoldList { rows: implements }
                    }
                    if !implemented_by.is_empty() {
                        h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                            "Implemented by ({implemented_by.len()})"
                        }
                        HoldList { rows: implemented_by }
                    }
                    // What its methods name is not what it keeps: these types
                    // pass through its API without being part of its shape.
                    if !api_names.is_empty() {
                        h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                            "Its API names ({api_names.len()})"
                        }
                        HoldList { rows: api_names }
                    }
                }
                // The implementation ink, both ways round. A ghost has no
                // body left and nothing left to lean on it.
                if !mark.ghost {
                    h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Used by ({used_by.len()})"
                    }
                    if used_by.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            if mark.unseen_users > 0 {
                                "no drawn mark's body reaches it."
                            } else {
                                "no body in the workspace reaches it."
                            }
                        }
                    } else {
                        HoldList { rows: used_by }
                    }
                    if mark.unseen_users > 0 {
                        p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                            "{unseen_line(mark.unseen_users, \"reach it from\")}"
                        }
                    }
                    h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Uses ({uses.len()})"
                    }
                    if uses.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "its body reaches nothing the chart draws."
                        }
                    } else {
                        HoldList { rows: uses }
                    }
                    if mark.unseen_uses > 0 {
                        p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                            "{unseen_line(mark.unseen_uses, \"of its own reach\")}"
                        }
                    }
                }
            }
            div { class: "border-t border-ink-line px-4 py-2",
                if mark.ghost {
                    p { class: "font-data text-[9.5px] text-ink-soft",
                        "its definition left the working copy."
                    }
                } else {
                    Link {
                        class: "font-data text-[9.5px] tracking-[0.12em] uppercase text-ink underline underline-offset-4 hover:text-ink-soft",
                        to: item_route(&mark.path, &mark.label),
                        "open its definition →"
                    }
                }
            }
        }
    }
}

/// One drawn edge sample for the legend, in the chart's own grammar — the same
/// classes the chart itself draws with, so the key cannot drift from the map.
#[component]
fn WireSample(
    dash: &'static str,
    #[props(default = 1.1)] width: f64,
    #[props(default = "")] label: &'static str,
) -> Element {
    rsx! {
        svg {
            class: "mt-0.5 shrink-0",
            width: "46",
            height: "14",
            view_box: "0 0 46 14",
            "aria-hidden": "true",
            g { class: "data-wire {dash}",
                path {
                    class: "wire-path",
                    d: "M1,10 Q22,5 40,9",
                    fill: "none",
                    style: "stroke-width: {width}px;",
                }
                path { class: "wire-head", d: "M45,9.2 L38.4,6.2 L38.8,11.6 Z" }
                if !label.is_empty() {
                    text {
                        class: "wire-label",
                        x: "21",
                        y: "5",
                        text_anchor: "middle",
                        "{label}"
                    }
                }
            }
        }
    }
}

/// The key: the two inks, the marks a block can be, the kind colors, the counted
/// folds, the diff, the gestures — and the survey's own limits behind a fold.
///
/// Cut from about eleven hundred words to a key (2026-08-21, distill). What came
/// off: the two paragraphs explaining the references and visibility toggles,
/// which restated their six button titles word for word; the clause announcing
/// that no row waits behind a count, which describes a fold that does not exist;
/// the sentences the sheet says the moment a mark is picked; and six paragraphs
/// paraphrasing the walk, which the survey now states in its own words behind
/// the fold at the foot.
///
/// What is left is shaped as a key, not as prose: sample beside word, then one
/// paragraph carrying the grammar for the whole family. At 224px of plate a
/// clause per line ran five lines deep, and four of them pushed the plate off
/// the page — the legend's budget is lines, not words.
#[component]
pub(crate) fn SurfaceLegend(
    facts: SurfaceFacts,
    notes: Vec<String>,
    #[props(default = true)] start_open: bool,
) -> Element {
    rsx! {
        details {
            class: "plate fold legend-plate pointer-events-auto flex min-h-0 w-full flex-col open:pb-3 sm:w-64",
            open: start_open,
            summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this chart"
            }
            div { class: "legend-scroll min-h-0 flex-1 space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink max-h-[42dvh] sm:max-h-none",
                div { class: "space-y-1",
                    div { class: "flex items-center gap-2",
                        WireSample { dash: "data-hold is-owns" }
                        span { class: "text-ink", "interface" }
                    }
                    div { class: "flex items-center gap-2",
                        WireSample { dash: "data-hold is-impl", width: 1.2, label: "implements" }
                        span { class: "text-ink", "implements" }
                    }
                    div { class: "flex items-center gap-2",
                        WireSample { dash: "is-ref", width: 1.6, label: "4" }
                        span { class: "text-ink", "uses" }
                    }
                    p { class: "pt-1 text-ink-soft",
                        "two inks, and the arrow rests on the dependent. solid: this block\u{2019}s own surface names the far end. dashed: a body leans on it, counted. a word on a line is the wrapper; no word is plain ownership."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5",
                    div { class: "flex items-center gap-2",
                        svg {
                            class: "shrink-0",
                            width: "46",
                            height: "14",
                            view_box: "0 0 46 14",
                            "aria-hidden": "true",
                            rect {
                                x: "1",
                                y: "1",
                                width: "44",
                                height: "12",
                                fill: "var(--color-paper)",
                                stroke: "var(--color-ink-line)",
                            }
                            rect {
                                x: "1",
                                y: "1",
                                width: "2.5",
                                height: "12",
                                fill: "var(--color-ink)",
                            }
                        }
                        span { class: "text-ink", "static" }
                    }
                    div { class: "flex items-center gap-2",
                        svg {
                            class: "sig-sample shrink-0",
                            width: "46",
                            height: "14",
                            view_box: "0 0 46 14",
                            "aria-hidden": "true",
                            rect {
                                class: "plate-ground",
                                x: "1",
                                y: "1",
                                width: "44",
                                height: "12",
                            }
                            rect {
                                class: "plate-rule",
                                x: "1",
                                y: "1",
                                width: "44",
                                height: "2.5",
                            }
                        }
                        span { class: "dm-nm is-fn", "pub fn" }
                    }
                    p { class: "pt-1 text-ink-soft",
                        "the ink edge is a root: state no type holds. a washed plate is a free function, parameters as rows. a "
                        span { class: "text-ink", "const" }
                        " or a "
                        span { class: "text-ink", "type" }
                        " alias is one line without the edge."
                    }
                    p { class: "text-ink-soft",
                        "a second band is the type\u{2019}s API; a trait is all band. a private method is no row."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5",
                    p { class: "text-ink-soft",
                        "rows are quoted as written; the bold run in one is the mark it reaches."
                    }
                    p {
                        span { class: "font-medium", "held by 6 · named by 2" }
                        span { class: "text-ink-soft",
                            " — a fan-in past three, folded; hover either end to ink it in."
                        }
                    }
                    p {
                        span { class: "font-medium", "+ 5 private items" }
                        span { class: "text-ink-soft",
                            " — below the door; every line touching one lands here."
                        }
                    }
                }
                // The diff's key only where there is a diff. A key for marks
                // the chart is not drawing is the same dead weight as a count
                // for nothing hidden, and it is six lines of a plate the
                // reader is short of (2026-08-21, distill).
                if !diff_words(&facts).is_empty() {
                    div { class: "border-t border-ink-line pt-2.5",
                        p { class: "text-ink-soft",
                            span { class: "font-bold text-flare", "A" }
                            " added · "
                            span { class: "font-bold text-flare", "M" }
                            " changed · "
                            span { class: "font-bold text-flare", "D" }
                            " removed — a ghost quoting the base. "
                            span { class: "font-bold text-flare", "+" }
                            " "
                            span { class: "font-bold text-flare", "−" }
                            " mark a row; a touched block wears the flare. untouched marks rest lighter."
                        }
                    }
                }
                Gestures {
                    UsageRow { gesture: "click", effect: "select a mark, or a module\u{2019}s border" }
                    UsageRow { gesture: "−", effect: "fold a module to one counted row" }
                    UsageRow { gesture: "hover", effect: "every edge of one mark" }
                    UsageRow { gesture: "f · esc · ← →", effect: "refit · deselect · retrace" }
                }
                if facts.unresolved > 0 {
                    p { class: "border-t border-ink-line pt-2.5 text-ink-soft",
                        "{plural(facts.unresolved as usize, \"name\")} the survey could not resolve, and so not on the chart."
                    }
                }
                SurveyLimits { notes }
            }
        }
    }
}
