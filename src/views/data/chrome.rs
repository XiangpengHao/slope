//! Data-altitude furniture: the cartouche, the selection sheet, and the
//! legend — the same engraved plates the other three altitudes wear.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, Delta, HoldEvent, HoldKind};
use crate::views::codemap::chrome::{Altitude, AltitudeSwitch, decl_words, plural};
use crate::views::codemap::{item_route, use_code};
use crate::views::data::model::{DataFacts, DataMark, DataModel, Stand, Tier};
use crate::views::data::{mark_route, mod_route, use_data};
use crate::views::surface::chrome::{HoldList, HoldRow, RefToggle};
use crate::views::surface::model::{Anchor, RowState, upstream};

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

/// The structural diff's own line: only what happened, in git's order.
fn diff_words(facts: &DataFacts) -> String {
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

/// The chart's title block: what state the workspace keeps, how it tiers,
/// and the reading control for its body dependence. No doors toggle: state
/// does not fold at a door, so every datum is drawn whatever its `pub`.
#[component]
pub fn DataCartouche(facts: DataFacts, workspace: String, diff_line: String) -> Element {
    let insight = insight(&facts.changed_modules);
    let kinds = {
        let mut parts = vec![
            plural(facts.structs, "struct"),
            plural(facts.enums, "enum"),
            plural(facts.statics, "static"),
        ];
        if facts.unions > 0 {
            parts.insert(2, plural(facts.unions, "union"));
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
                p { class: "mt-0.5 font-data text-[10.5px] text-ink-soft",
                    "{plural(facts.roots, \"root\")} · {facts.nested} nested · {facts.standing} standing"
                }
                p { class: "mt-0.5 font-data text-[10.5px] text-ink-soft",
                    "{plural(facts.ties, \"body dependence\")} drawn"
                }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Data }
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
        }
    }
}

/// A hold's kind in its own lowercase word; the wrapper's word wins.
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

/// The rows one side of a selection draws, from each relation's far end.
fn hold_rows(
    model: &DataModel,
    holds: Vec<(&Anchor, HoldKind, &str, Option<HoldEvent>)>,
) -> Vec<HoldRow> {
    let by_id: std::collections::HashMap<u32, &DataMark> =
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
                Anchor::Private(frame) | Anchor::More(frame) | Anchor::Mod(frame) => {
                    // Only a folded module leaves a counted row on this
                    // chart; the row can be selected, so it is a link.
                    let frame = &model.frames[*frame as usize];
                    let place = match frame.module.is_empty() {
                        true => frame.krate.clone(),
                        false => format!("mod {}", frame.words()),
                    };
                    HoldRow {
                        to: matches!(anchor, Anchor::Mod(_)).then(|| mod_route(frame.key())),
                        decl: String::new(),
                        name: format!("+ {} · {place}", plural(frame.packed as usize, "item")),
                        letter: None,
                        word: hold_word(kind, via),
                        event,
                    }
                }
            }
        })
        .collect()
}

/// One end of a uses edge as the sheet reads it.
type UsesRow<'a> = (&'a Anchor, u32, &'a [(String, u32)]);

fn uses_rows(model: &DataModel, rows: Vec<UsesRow<'_>>) -> Vec<HoldRow> {
    let by_id: std::collections::HashMap<u32, &DataMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();
    rows.into_iter()
        .filter_map(|(anchor, count, clauses)| {
            let Anchor::Mark(id) = anchor else {
                return None;
            };
            let far = by_id.get(id)?;
            let mut word = plural(count as usize, "reference");
            // One clause at most: the sheet's row has a name to keep legible,
            // and the selection can always be re-centered for the rest.
            if let Some((row, _)) = clauses.first() {
                word = format!("{word} · {row}");
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

/// The tier, said out loud — the one sentence this altitude exists for.
fn tier_line(model: &DataModel, mark: &DataMark) -> String {
    if mark.ghost {
        return "removed since the base — whoever held it, the removed edges say.".to_string();
    }
    match mark.tier {
        Tier::Root if mark.is_static() => "a root — state no type holds.".to_string(),
        Tier::Root => "top-level data: no type holds it — a root.".to_string(),
        Tier::Nested(holder) => {
            let name = model
                .marks
                .iter()
                .find(|m| m.id == holder)
                .map(|m| m.name.clone())
                .unwrap_or_default();
            format!("secondary data: state of {name}, drawn inside its block.")
        }
        Tier::Standing(Stand::Shared) => {
            "secondary data, shared: a handle holds it, and sharing has no single container, \
             so it stands beside its holders with every line drawn."
                .to_string()
        }
        Tier::Standing(Stand::Vocab) => format!(
            "secondary data, vocabulary: {} hold it — too many to seat under one — so its \
             fan-in rests folded on its own foot.",
            plural(mark.held_by as usize, "type")
        ),
        Tier::Standing(Stand::Afar) => {
            "secondary data: its holder is in another module, and cross-module ownership \
             stays drawn ink."
                .to_string()
        }
        Tier::Standing(Stand::Ring) => {
            "secondary data, in a ring: it and its holder own each other, and the seat that \
             would close the loop stays a line."
                .to_string()
        }
    }
}

/// One selected datum's sheet: its tier, who holds it, which contracts name
/// it, what it holds, and who uses it — with the counted residue said out
/// loud, because `named by 12 signatures` is ink this chart refuses to draw
/// and a reviewer must never mistake for silence.
#[component]
pub fn DataSheet(graph: CodeGraph, path: String, item: String) -> Element {
    let code = use_code();
    let data = use_data();
    let model = use_memo(use_reactive((&graph,), move |(graph,)| {
        DataModel::build(&graph, *code.ref_dir.peek(), &data.folds.read())
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
                    to: Route::DataOverview {},
                    "← whole chart"
                }
            }
        };
    };

    let at = Anchor::Mark(mark.id);
    let decl = decl_words(mark.vis, mark.kind);
    let by_id: std::collections::HashMap<u32, &DataMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();

    // Who holds it: the drawn relations landing on it, and — first, because
    // the paper says it first — the block it is nested inside.
    let mut held_by: Vec<HoldRow> = Vec::new();
    if let Tier::Nested(holder) = mark.tier
        && let Some(h) = by_id.get(&holder)
    {
        held_by.push(HoldRow {
            to: Some(mark_route(&h.path, &h.label)),
            decl: decl_words(h.vis, h.kind),
            name: h.name.clone(),
            letter: h.letter(),
            word: "owns · nested".to_string(),
            event: None,
        });
    }
    held_by.extend(hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.held == at)
            .map(|h| (&h.holder, h.kind, h.via.as_str(), h.event))
            .collect(),
    ));

    // What it holds: the blocks nested inside it, then the drawn relations.
    let mut holds: Vec<HoldRow> = mark
        .kids
        .iter()
        .filter_map(|kid| by_id.get(kid))
        .map(|k| HoldRow {
            to: Some(mark_route(&k.path, &k.label)),
            decl: decl_words(k.vis, k.kind),
            name: k.name.clone(),
            letter: k.letter(),
            word: "owns · nested".to_string(),
            event: None,
        })
        .collect();
    holds.extend(hold_rows(
        &model,
        model
            .holds
            .iter()
            .filter(|h| h.holder == at)
            .map(|h| (&h.held, h.kind, h.via.as_str(), h.event))
            .collect(),
    ));

    // The naming ink this chart refuses to draw, in rows: the free contracts
    // whose declared surface names it (each a link to its definition, since
    // none has a block here), and the types whose API says the word.
    let files = &graph.files;
    let namer_row = |namer: u32, event: Option<HoldEvent>| -> Option<HoldRow> {
        let item = graph.items.get(namer as usize)?;
        let file = files.get(item.file as usize)?;
        Some(HoldRow {
            to: Some(item_route(&file.path, &item.label)),
            decl: decl_words(item.vis, item.kind),
            name: item.name.clone(),
            letter: None,
            word: "names it".to_string(),
            event: match event {
                Some(HoldEvent::Added) => Some("added"),
                Some(HoldEvent::Removed) => Some("removed"),
                None => None,
            },
        })
    };
    let contracts: Vec<HoldRow> = model
        .naming
        .iter()
        .filter(|n| n.ty == mark.id && !n.from_method)
        .filter_map(|n| namer_row(n.namer, n.event))
        .collect();
    let in_api: Vec<HoldRow> = model
        .naming
        .iter()
        .filter(|n| n.ty == mark.id && n.from_method)
        .filter_map(|n| {
            let far = by_id.get(&n.namer)?;
            Some(HoldRow {
                to: Some(mark_route(&far.path, &far.label)),
                decl: decl_words(far.vis, far.kind),
                name: far.name.clone(),
                letter: far.letter(),
                word: "its API names it".to_string(),
                event: None,
            })
        })
        .collect();

    // The implementation ink, both ways round.
    let used_by: Vec<HoldRow> = uses_rows(
        &model,
        model
            .ties
            .iter()
            .filter(|t| t.def == at)
            .map(|t| (&t.user, t.count, t.rows.as_slice()))
            .collect(),
    );
    let uses: Vec<HoldRow> = uses_rows(
        &model,
        model
            .ties
            .iter()
            .filter(|t| t.user == at)
            .map(|t| (&t.def, t.count, t.rows.as_slice()))
            .collect(),
    );

    // The selection's own diff rows, exactly as the block draws them.
    let change_rows: Vec<(&'static str, String, bool)> = mark
        .fields
        .iter()
        .chain(mark.variants.iter())
        .filter_map(|row| {
            let mk = row.state.marker()?;
            let text = if row.name.is_empty() {
                row.decl.clone()
            } else {
                format!("{}: {}", row.name, row.decl)
            };
            Some((mk, text, row.state == RowState::Removed))
        })
        .collect();
    let change_line = if mark.ghost {
        Some("removed since the base — this block quotes the base edition.")
    } else if mark.delta == Delta::Added {
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

    let tier = tier_line(&model, mark);
    // The four-way truth on a quiet datum: the verdict a reviewer deletes on.
    let quiet = mark.is_root()
        && !mark.is_static()
        && held_by.is_empty()
        && contracts.is_empty()
        && in_api.is_empty()
        && used_by.is_empty()
        && mark.used_by == 0;

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
                h3 { class: "mt-1 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Held by ({held_by.len()})"
                }
                if held_by.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        if quiet {
                            "nothing in the workspace reaches it: no type holds it, no signature names it, no body uses it."
                        } else if mark.is_static() {
                            "no type holds it — a static is where holding begins."
                        } else {
                            "no type holds it — it enters through the contracts below."
                        }
                    }
                } else {
                    HoldList { rows: held_by }
                }
                if !contracts.is_empty() {
                    h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "In the contract of ({contracts.len()})"
                    }
                    HoldList { rows: contracts }
                }
                if !in_api.is_empty() {
                    h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "In the API of ({in_api.len()})"
                    }
                    HoldList { rows: in_api }
                }
                if !reach.is_empty() {
                    p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                        if mark.ghost {
                            "the removal {reach}."
                        } else {
                            "a shape change here {reach}."
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
                if !mark.ghost {
                    h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Used by ({used_by.len()})"
                    }
                    if used_by.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            if mark.used_by > 0 {
                                "no type's body reaches it."
                            } else {
                                "no body in the workspace reaches it."
                            }
                        }
                    } else {
                        HoldList { rows: used_by }
                    }
                    if mark.used_by > 0 {
                        p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                            "{plural(mark.used_by as usize, \"reference\")} reach it from code with no block here — function bodies, and items with no mark of their own."
                        }
                    }
                    h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Uses ({uses.len()})"
                    }
                    if uses.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "its impls reach nothing the chart draws."
                        }
                    } else {
                        HoldList { rows: uses }
                    }
                    if mark.unseen_uses > 0 {
                        p { class: "mt-1 px-1 font-data text-[10px] leading-relaxed text-ink-soft",
                            "{plural(mark.unseen_uses as usize, \"reference\")} of its own land on code with no block here."
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

/// One drawn edge sample for the legend, off the chart's own classes.
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

/// The key: the tier's two moves, the two inks, and the walk's own honesty
/// notes. What the drawing already says is not repeated.
#[component]
pub fn DataLegend(facts: DataFacts, #[props(default = true)] start_open: bool) -> Element {
    rsx! {
        details {
            class: "plate fold pointer-events-auto w-full open:pb-3 sm:w-64",
            open: start_open,
            summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this chart"
            }
            div { class: "legend-scroll space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink max-h-[42dvh] sm:max-h-[calc(100dvh_-_300px)]",
                div { class: "space-y-1.5",
                    div { class: "flex items-start gap-2",
                        svg {
                            class: "mt-0.5 shrink-0",
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
                        span {
                            span { class: "text-ink", "top-level data" }
                            span { class: "text-ink-soft",
                                " — the ink left edge marks a root: a static, or a type no other type keeps in a field. state the code reaches directly, where every chain of holding begins."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        svg {
                            class: "mt-0.5 shrink-0",
                            width: "46",
                            height: "18",
                            view_box: "0 0 46 18",
                            "aria-hidden": "true",
                            rect {
                                x: "1",
                                y: "1",
                                width: "44",
                                height: "16",
                                fill: "var(--color-paper)",
                                stroke: "var(--color-ink-line)",
                            }
                            rect {
                                x: "1",
                                y: "1",
                                width: "2.5",
                                height: "16",
                                fill: "var(--color-ink)",
                            }
                            rect {
                                x: "7",
                                y: "8",
                                width: "16",
                                height: "7",
                                fill: "var(--color-paper)",
                                stroke: "var(--color-ink-line)",
                            }
                        }
                        span {
                            span { class: "text-ink", "secondary data" }
                            span { class: "text-ink-soft",
                                " — drawn inside the block of the type that owns it hardest, under a hairline rule. the nesting is the ownership: no line restates it, and the bold run in a field row above is the block below."
                            }
                        }
                    }
                    p { class: "text-ink-soft",
                        "a held type the chart cannot seat under one holder stands at module level with its lines drawn: shared state (a handle has no single container), state owned from another module, a ring of mutual owners, and a type so widely held its fan-in folds to "
                        span { class: "text-ink", "held by n types" }
                        span { class: "text-ink-soft", " — hover or select inks the lines back in." }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    p { class: "text-ink-soft",
                        "two inks, and only two. the arrowhead rests on the dependent: a change at the tail travels along the arrow."
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "data-hold is-shares", width: 1.3, label: "Arc" }
                        span {
                            span { class: "text-ink", "holding" }
                            span { class: "text-ink-soft",
                                " — solid, with the wrapper's own word: "
                            }
                            span { class: "text-ink", "Arc" }
                            span { class: "text-ink-soft", ", " }
                            span { class: "text-ink", "&" }
                            span { class: "text-ink-soft",
                                ", and no word is plain ownership. only what the nesting cannot say is a line; a borrow is a view, not a hold, so a type only borrowed is still a root."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-ref", width: 1.6, label: "4" }
                        span {
                            span { class: "text-ink", "uses" }
                            span { class: "text-ink-soft",
                                " — dashed: one type's impls lean on another, summed and counted. each block rests its heaviest few; the rest ink in on hover and stay while either end is selected."
                            }
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    p {
                        span { class: "dm-nm", "Struct" }
                        span { class: "text-ink-soft", " is type-teal, " }
                        span { class: "dm-nm is-sum", "Enum" }
                        span { class: "text-ink-soft",
                            " the palette's purple — a second reading of the keyword in front of the name. rows are quoted as written and colored by token class; the bold run names the workspace type the row reaches."
                        }
                    }
                    p { class: "text-ink-soft",
                        "no methods. a block is state only — fields, variants, a static's declared type. what a type promises is the surface chart's ink, one rung up, and "
                        span { class: "text-ink", "open its definition →" }
                        span { class: "text-ink-soft", " stays the one quotation surface." }
                    }
                    p { class: "text-ink-soft",
                        "every datum is drawn whatever its visibility: state does not fold at a door. "
                        span { class: "text-ink", "pub" }
                        span { class: "text-ink-soft",
                            " on a header is the whole visibility story, in rust's own word."
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    p { class: "text-flare", "the diff's key" }
                    p { class: "text-ink-soft",
                        span { class: "text-flare", "A" }
                        " added · "
                        span { class: "text-flare", "M" }
                        " declaration changed · "
                        span { class: "text-flare", "D" }
                        " removed, quoted from the base as a dashed ghost. an added row wears "
                        span { class: "text-flare", "+" }
                        ", a dropped one is struck where it stood. an added or removed holding takes flare with its word on the line. while the diff has anything to say, untouched blocks rest lighter; hover restores."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5 text-ink-soft",
                    p {
                        "what has no block here is counted, never cut: "
                        span { class: "text-ink", "named by n signatures" }
                        " is every free fn, const, alias and method row whose declared surface names the type, and "
                        span { class: "text-ink", "used by n bodies" }
                        " is every reference from code with no mark of its own. the sheet lists both."
                    }
                    p {
                        "a "
                        span { class: "text-ink", "dyn Trait" }
                        " field names a contract, and contracts live one rung up: the row quotes it, and no line is drawn."
                    }
                    p {
                        "type parameters, trait bounds and impl Trait are holes: the row quotes them, the walk reads nothing through them. what a macro declares, the survey cannot read."
                    }
                    if facts.unresolved > 0 {
                        p { "{plural(facts.unresolved as usize, \"name\")} the survey could not resolve." }
                    }
                    p {
                        "a folded module is one counted row; its state is off the paper and its holding lines land on the row. the border's − / + folds and unfolds; the line itself selects the module."
                    }
                }
            }
        }
    }
}
