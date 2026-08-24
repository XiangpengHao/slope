//! Data-altitude furniture: the cartouche, the selection sheet, and the
//! legend — the same engraved plates the other three altitudes wear.

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CodeGraph, Delta, HoldEvent, HoldKind, ItemMark};
use crate::views::codemap::chrome::{
    Altitude, AltitudeSwitch, Gestures, SurveyLimits, UsageRow, decl_words, kind_words, plural,
};
use crate::views::codemap::{item_route, use_code};
use crate::views::data::model::{DataFacts, DataMark, DataModel, Stand, Tier, Unseen};
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

/// The chart's title block: the census of what state the workspace keeps, the
/// altitude ladder, the diff, and the reading control for body dependence.
///
/// It states no tier counts and no edge counts (2026-08-21, distill): `55
/// roots · 63 nested · 16 standing` and `209 body dependences · 127 at rest`
/// were the model's own bookkeeping in four invented words, and no reviewer
/// decides anything on them. The tier is what the paper draws and the legend
/// teaches; the edges are what the chart shows. No doors toggle either: state
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
                Anchor::Private(frame) | Anchor::Mod(frame) => {
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

fn uses_rows(model: &DataModel, rows: Vec<UsesRow<'_>>) -> Vec<(u32, HoldRow)> {
    let by_id: std::collections::HashMap<u32, &DataMark> =
        model.marks.iter().map(|m| (m.id, m)).collect();
    rows.into_iter()
        .filter_map(|(anchor, count, clauses)| {
            let Anchor::Mark(id) = anchor else {
                return None;
            };
            let far = by_id.get(id)?;
            let mut word = plural(count as usize, "reference");
            // One clause at most, and only when the whole row fits: the name
            // is the one thing the row exists to state, so the clause drops
            // before the name would clip.
            if let Some((row, _)) = clauses.first()
                && far.name.chars().count() + row.chars().count() <= 13
            {
                word = format!("{word} · {row}");
            }
            Some((
                count,
                HoldRow {
                    to: Some(mark_route(&far.path, &far.label)),
                    decl: decl_words(far.vis, far.kind),
                    name: far.name.clone(),
                    letter: far.letter(),
                    word,
                    event: None,
                },
            ))
        })
        .collect()
}

/// One relation, one ranking: a drawn type and a body with no block here are
/// the same ink, so they rank together and the heaviest reads first. The
/// list's first eight are what the sheet shows before `show all`, which is
/// the whole reason the order has to be weight and not provenance.
fn ranked(mut rows: Vec<(u32, HoldRow)>) -> Vec<HoldRow> {
    rows.sort_by_key(|(count, _)| std::cmp::Reverse(*count));
    rows.into_iter().map(|(_, row)| row).collect()
}

/// The tier, said out loud — the one sentence this altitude exists for.
fn tier_line(mark: &DataMark) -> String {
    if mark.ghost {
        return "removed since the base — whoever held it, the removed edges say.".to_string();
    }
    match mark.tier {
        Tier::Root if mark.is_static() => "a root — state no type holds.".to_string(),
        Tier::Root => "top-level data: no type holds it — a root.".to_string(),
        // The holder is the first row of the section right below this line;
        // naming it twice in six words was the sheet saying the same thing to
        // itself (2026-08-21, distill).
        Tier::Nested(_) => "secondary data — drawn inside its holder's block.".to_string(),
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

    // The implementation ink, both ways round — and then the same ink from
    // the ends this chart draws no block for (2026-08-23, user). A free
    // function's body is a real item with a definition, so it gets a row like
    // any other and links to its code; the sheet used to spend a sentence
    // counting them instead, which named nothing a reviewer could open.
    let unseen_row = |end: &Unseen| -> Option<(u32, HoldRow)> {
        let item = graph.items.get(end.item as usize)?;
        let file = files.get(item.file as usize)?;
        Some((
            end.count,
            HoldRow {
                to: Some(item_route(&file.path, &item.label)),
                decl: decl_words(item.vis, item.kind),
                name: item.name.clone(),
                letter: None,
                word: plural(end.count as usize, "reference"),
                event: None,
            },
        ))
    };
    let used_by: Vec<HoldRow> = ranked(
        uses_rows(
            &model,
            model
                .ties
                .iter()
                .filter(|t| t.def == at)
                .map(|t| (&t.user, t.count, t.rows.as_slice()))
                .collect(),
        )
        .into_iter()
        .chain(mark.used_by.iter().filter_map(unseen_row))
        .collect(),
    );
    let uses: Vec<HoldRow> = ranked(
        uses_rows(
            &model,
            model
                .ties
                .iter()
                .filter(|t| t.user == at)
                .map(|t| (&t.def, t.count, t.rows.as_slice()))
                .collect(),
        )
        .into_iter()
        .chain(mark.unseen_uses.iter().filter_map(unseen_row))
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

    let tier = tier_line(mark);
    // The four-way truth on a quiet datum: the verdict a reviewer deletes on.
    let quiet = mark.is_root()
        && !mark.is_static()
        && held_by.is_empty()
        && contracts.is_empty()
        && in_api.is_empty()
        && used_by.is_empty();

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
                            "no body in the workspace reaches it."
                        }
                    } else {
                        HoldList { rows: used_by }
                    }
                    h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                        "Uses ({uses.len()})"
                    }
                    if uses.is_empty() {
                        p { class: "mt-1 font-data text-[10px] text-ink-soft",
                            "its impls reach nothing in the workspace."
                        }
                    } else {
                        HoldList { rows: uses }
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

/// Find a datum by name. Marks only — this altitude's places are types and
/// statics — ranked the way the code search ranks: a prefix match is what the
/// reviewer meant, then whatever more of the workspace leans on.
#[component]
pub fn DataSearch(graph: CodeGraph) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let nav = use_navigator();

    // How one hit ranks: a prefix match is what the reviewer meant, then
    // whatever more of the workspace leans on, then the name itself.
    type Rank = (bool, std::cmp::Reverse<u32>, String);
    let results = use_memo(move || {
        let q = query().trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let datum = |kind: crate::api::ItemKind| {
            matches!(
                kind,
                crate::api::ItemKind::Struct
                    | crate::api::ItemKind::Enum
                    | crate::api::ItemKind::Union
                    | crate::api::ItemKind::Static
            )
        };
        let mut hits: Vec<(Rank, (ItemMark, String))> = graph
            .items
            .iter()
            .filter(|m| m.parent.is_none() && datum(m.kind))
            .filter(|m| m.name.to_lowercase().contains(&q))
            .filter_map(|m| {
                let path = graph.files.get(m.file as usize)?.path.clone();
                Some((
                    (
                        !m.name.to_lowercase().starts_with(&q),
                        std::cmp::Reverse(m.fan_in),
                        m.name.clone(),
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
                                nav.push(mark_route(path, &m.label));
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
                            li { key: "{path}|{m.label}",
                                Link {
                                    to: mark_route(&path, &m.label),
                                    class: if i == active() { "flex w-full items-baseline gap-1.5 px-2.5 py-1 bg-ink/5" } else { "flex w-full items-baseline gap-1.5 px-2.5 py-1 hover:bg-ink/5" },
                                    onclick: move |_| query.set(String::new()),
                                    span { class: "shrink-0 font-data text-[9.5px] text-ink-soft",
                                        "{kind_words(m.kind)}"
                                    }
                                    span { class: "truncate font-data text-[11px] text-ink", "{m.name}" }
                                    span { class: "ml-auto shrink-0 truncate font-data text-[9px] text-ink-soft",
                                        "{path}:{m.line}"
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

/// The key: the tier's two moves, the two inks, the kind colors, the diff, the
/// gestures — and the survey's own limits behind a fold.
///
/// Cut from about six hundred words to a key (2026-08-21, distill). What came
/// off: the paragraph explaining the references toggle, which restated its
/// three button titles; the sentences the sheet says the moment a block is
/// picked; the walk's method, which the survey now states in its own words
/// behind the fold at the foot; and every clause that announced the absence of
/// a fold. What stays is what the drawing draws and cannot say — and the marks
/// stand as a key strip, sample beside word, because at 224px of plate a
/// sentence per mark cost five lines each and ran the plate off the page.
#[component]
pub fn DataLegend(
    facts: DataFacts,
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
                        span { class: "text-ink", "top-level data" }
                    }
                    div { class: "flex items-center gap-2",
                        svg {
                            class: "shrink-0",
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
                        span { class: "text-ink", "secondary data" }
                    }
                    p { class: "pt-1 text-ink-soft",
                        "the ink edge is a root: a static, or a type nothing keeps in a field. everything else nests inside its heaviest owner — the nesting is the ownership, and no line restates it. a type that fits under no one holder stands at module level with its lines drawn: shared handles, cross-module owners, rings, widely held vocabulary."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5",
                    div { class: "flex items-center gap-2",
                        WireSample { dash: "data-hold is-shares", width: 1.3, label: "Arc" }
                        span { class: "text-ink", "holding" }
                    }
                    div { class: "flex items-center gap-2",
                        WireSample { dash: "is-ref", width: 1.6, label: "4" }
                        span { class: "text-ink", "uses" }
                    }
                    p { class: "pt-1 text-ink-soft",
                        "the arrow rests on the dependent. a word on a solid line is the wrapper; no word is plain ownership, and a borrow is a view — a type only borrowed is still a root. each block rests its two heaviest dashed lines, and hover inks in the rest."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5",
                    p {
                        span { class: "dm-nm", "Struct" }
                        span { class: "text-ink-soft", " · " }
                        span { class: "dm-nm is-sum", "Enum" }
                        span { class: "text-ink-soft",
                            " — the kind, said again in color. the bold run in a row is the type it reaches."
                        }
                    }
                    p { class: "text-ink-soft",
                        "no methods, and no doors: a block is state only, drawn whatever its "
                        span { class: "text-ink", "pub" }
                        span { class: "text-ink-soft", "." }
                    }
                }
                // The diff's key only where there is a diff: a key for marks
                // the chart is not drawing is the same dead weight as a count
                // for nothing hidden (2026-08-21, distill).
                if !diff_words(&facts).is_empty() {
                    div { class: "border-t border-ink-line pt-2.5",
                        p { class: "text-ink-soft",
                            span { class: "text-flare", "A" }
                            " added · "
                            span { class: "text-flare", "M" }
                            " changed · "
                            span { class: "text-flare", "D" }
                            " removed, a dashed ghost quoting the base. "
                            span { class: "text-flare", "+" }
                            " and "
                            span { class: "text-flare", "−" }
                            " mark a row. while the diff speaks, untouched blocks rest lighter."
                        }
                    }
                }
                Gestures {
                    UsageRow { gesture: "click", effect: "select a block; its sheet opens" }
                    UsageRow { gesture: "−", effect: "fold a module to one counted row" }
                    UsageRow { gesture: "hover", effect: "every line of one block" }
                    UsageRow { gesture: "/ · f", effect: "find a datum · refit" }
                    UsageRow { gesture: "← · → · esc", effect: "back · forward · deselect" }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5 text-ink-soft",
                    p {
                        "below reading zoom each block draws its name alone; descend, or select, and the rows return. a selected boundary bundles its crossing lines, one per far module."
                    }
                    p {
                        "nothing is cut, only counted: a block\u{2019}s hover words say "
                        span { class: "text-ink", "named by n signatures · used by n bodies" }
                        ", and the sheet lists both. a "
                        span { class: "text-ink", "dyn Trait" }
                        " field names a contract one rung up, so no line is drawn."
                    }
                    if facts.unresolved > 0 {
                        p { "{plural(facts.unresolved as usize, \"name\")} the survey could not resolve." }
                    }
                }
                SurveyLimits { notes }
            }
        }
    }
}
