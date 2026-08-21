//! Drawing a navigator page: the columns, the plates, the harness, the board.
//!
//! Every block is already placed by [`super::layout`], so this is a walk and
//! nothing more — each entry is seated at the coordinates the layout measured,
//! and the wires are drawn from the same numbers. Nothing here asks the browser
//! where anything is.

use dioxus::prelude::*;

use crate::api::{HoldEvent, ItemKind};
use crate::views::codemap::tree::Placed;
use crate::views::navigator::layout::{Entry, FocusBox, Page, PlateBox, RowBox, Wire};
use crate::views::navigator::model::{Family, Ink, NavItem, NavModel, QuoteRow, RowState};
use crate::views::navigator::{MarkKey, NavState, NavStep, mark_route};

/// Where one block sits, as the style attribute the browser reads.
fn seat(at: Placed) -> String {
    format!("left:{:.1}px;top:{:.1}px;width:{:.1}px", at.x, at.y, at.w)
}

/// A block that also owes the layout its height, because a wire lands on its
/// middle.
fn seat_h(at: Placed) -> String {
    format!("{};min-height:{:.1}px", seat(at), at.h)
}

/// What a mark's frame says about it before a word is read: a contract, a root,
/// a ghost, or a declaration the diff touched.
fn mark_class(item: &NavItem) -> String {
    let mut class = String::from("nav-mark");
    if item.is_contract() && item.kind != ItemKind::Static {
        class.push_str(" is-contract");
    }
    if item.kind == ItemKind::Static {
        class.push_str(" is-root");
    }
    if item.ghost {
        class.push_str(" is-ghost");
    }
    if item.letter().is_some() && !item.ghost {
        class.push_str(" is-diff");
    }
    class
}

/// One quoted run: the bold one names a workspace mark, the rest is token
/// class, and plain text is plain text.
fn ink(runs: &[Ink]) -> Element {
    rsx! {
        for run in runs.iter() {
            if run.held {
                span {
                    class: if run.sum { "nm-held is-sum" } else { "nm-held" },
                    "{run.text}"
                }
            } else if !run.class.is_empty() {
                span { class: "{run.class}", "{run.text}" }
            } else {
                "{run.text}"
            }
        }
    }
}

/// A mark's header line: rust's keywords, its name, and the diff's letter.
fn head(item: &NavItem) -> Element {
    rsx! {
        header { class: "nm-head",
            span { class: "nm-kw", "{item.keyword()}" }
            span {
                class: if item.is_sum() { "nm-nm is-sum" } else { "nm-nm" },
                "{item.name}"
            }
            if let Some(letter) = item.letter() {
                span { class: "nm-chg", "{letter}" }
            }
        }
    }
}

/// A compact plate: the header, the module it is written in, and — on the
/// agenda — what the diff did to its rows. The relation it stands in is said by
/// the column it stands in and the word on its wire.
fn plate(model: &NavModel, block: &PlateBox) -> Element {
    let Some(item) = model.item(block.id) else {
        return rsx! {};
    };
    rsx! {
        Link {
            to: mark_route(&item.path, &item.label),
            class: mark_class(item),
            style: seat_h(block.at),
            title: "{item.path}:{item.line}",
            {head(item)}
            p { class: "nm-mod", "{item.module}" }
            if let Some(note) = &block.note {
                p { class: "nm-note", "{note}" }
            }
        }
    }
}

/// One line of the reach: a name, its kind, and the module it is in. Every one
/// of them is one refocus away.
fn far_row(model: &NavModel, block: &RowBox) -> Element {
    let Some(item) = model.item(block.id) else {
        return rsx! {};
    };
    rsx! {
        Link {
            to: mark_route(&item.path, &item.label),
            class: "nav-far",
            style: seat(block.at),
            span { class: "fk", "{item.word()}" }
            span { class: "fn2", "{item.name}" }
            if let Some(letter) = item.letter() {
                span { class: "nm-chg", "{letter}" }
            }
            span { class: "fm", "{item.module}" }
        }
    }
}

/// The focused mark, quoted whole: the one block on the page that is read
/// rather than scanned.
fn focus_plate(model: &NavModel, block: &FocusBox, state: NavState) -> Element {
    let Some(item) = model.item(block.id) else {
        return rsx! {};
    };
    let key = MarkKey {
        path: item.path.clone(),
        label: item.label.clone(),
    };
    let pinned = state.pins.read().contains(&key);
    let mut class = mark_class(item);
    class.push_str(" nav-focus");
    rsx! {
        section { class, style: seat_h(block.at),
            {head(item)}
            for row in block.rows.iter() {
                {quote_row(row)}
            }
            p { class: "nm-loc", "{block.locator}" }
            div { class: "nm-acts",
                button {
                    class: if pinned { "on" } else { "" },
                    onclick: move |_| state.toggle_pin(key.clone()),
                    if pinned { "pinned ✓" } else { "pin to board" }
                }
            }
        }
    }
}

/// One row of the quotation. A row the epoch added flares with a `+`; one it
/// dropped is struck through and quoted from the base edition.
fn quote_row(row: &QuoteRow) -> Element {
    let mut class = match row.state {
        RowState::Same => "nm-row".to_string(),
        RowState::Added => "nm-row is-add".to_string(),
        RowState::Removed => "nm-row is-del".to_string(),
    };
    if row.band {
        class.push_str(" nm-band");
    }
    rsx! {
        p { class,
            if let Some(marker) = row.state.marker() {
                span { class: "nm-mk", "{marker}" }
            }
            if !row.name.is_empty() {
                span { class: "nm-fname", "{row.name}" }
            }
            {ink(&row.runs)}
        }
    }
}

/// The harness. Solid is interface coupling and dashed is implementation
/// coupling, the same two inks the surface chart runs; the diff's own ink
/// overrides both, because a reviewer came for it.
fn wires(page: &Page) -> Element {
    let (w, h) = page.size;
    rsx! {
        svg {
            class: "nav-wires",
            width: "{w:.0}",
            height: "{h:.0}",
            "aria-hidden": "true",
            for wire in page.wires.iter() {
                {wire_part(wire)}
            }
        }
    }
}

fn wire_part(wire: &Wire) -> Element {
    let mut class = match wire.family {
        Family::Solid => "wire".to_string(),
        Family::Uses => "wire is-uses".to_string(),
    };
    match wire.event {
        Some(HoldEvent::Added) => class.push_str(" is-added"),
        Some(HoldEvent::Removed) => class.push_str(" is-removed"),
        None => {}
    }
    rsx! {
        g { class,
            path { class: "wp", d: "{wire.line}" }
            if let Some(head) = &wire.head {
                path { class: "wh", d: "{head}" }
            }
            if let Some(label) = &wire.label {
                text {
                    x: "{label.x:.1}",
                    y: "{label.y:.1}",
                    text_anchor: if label.start { "start" } else { "end" },
                    "{label.text}"
                }
            }
        }
    }
}

/// The page: four columns of bands, the harness over them, and the focus block
/// in the middle when the question is about one mark.
pub fn canvas(model: &NavModel, page: &Page, state: NavState) -> Element {
    let (w, h) = page.size;
    rsx! {
        div {
            class: "nav-canvas",
            style: "width:{w:.0}px;height:{h:.0}px",
            {wires(page)}
            for band in page.bands.iter() {
                h2 { class: "nav-band-h", style: seat(band.head),
                    span { "{band.title}" }
                    if let Some(count) = &band.count {
                        span { class: "cnt", "{count}" }
                    }
                }
                for entry in band.entries.iter() {
                    match entry {
                        Entry::Plate(block) => rsx! {
                            {plate(model, block)}
                        },
                        Entry::Row(block) => rsx! {
                            {far_row(model, block)}
                        },
                        Entry::Group { label, at } => rsx! {
                            p { class: "nav-grp", style: seat(*at), "{label}" }
                        },
                        Entry::Truth { text, at } => rsx! {
                            p { class: "nav-truth", style: seat(*at), "{text}" }
                        },
                    }
                }
            }
            if let Some(block) = &page.focus {
                {focus_plate(model, block, state)}
            }
        }
    }
}

/// The trail: every question this session asked, in order. The trail is the
/// map — there is no other one.
pub fn trail(model: &NavModel, state: NavState) -> Element {
    let steps = state.trail.read().clone();
    let here = *state.at.read();
    rsx! {
        nav { class: "nav-trail", "aria-label": "the review trail",
            for (at , step) in steps.iter().enumerate() {
                if at > 0 {
                    span { class: "nav-sep", "▸" }
                }
                match step {
                    NavStep::Agenda => rsx! {
                        Link {
                            to: crate::Route::NavigatorAgenda {},
                            class: if at == here { "nav-chip here" } else { "nav-chip" },
                            span { class: "k", "⌂" }
                            " the diff"
                        }
                    },
                    NavStep::Focus(key) => {
                        let item = model.find(&key.path, &key.label).and_then(|id| model.item(id));
                        let word = item.map(NavItem::word).unwrap_or("mark");
                        let name = item.map(|it| it.name.clone()).unwrap_or_else(|| key.label.clone());
                        rsx! {
                            Link {
                                to: mark_route(&key.path, &key.label),
                                class: if at == here { "nav-chip here" } else { "nav-chip" },
                                span { class: "k", "{word}" }
                                " {name}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The board: what this review is keeping, and how each pin connects to the one
/// before it. The paths are the subgraph the reviewer walked away with, which is
/// the artifact a review produces.
pub fn board(model: &NavModel, state: NavState) -> Element {
    let pins = state.pins.read().clone();
    if pins.is_empty() {
        return rsx! {};
    }
    let ids: Vec<(MarkKey, Option<u32>)> = pins
        .iter()
        .map(|key| (key.clone(), model.find(&key.path, &key.label)))
        .collect();
    rsx! {
        footer { class: "nav-board",
            div { class: "b-h",
                span { class: "b-title", "The board" }
                span { class: "b-note",
                    "what this review is keeping — and how each pin connects to the last"
                }
                button {
                    class: "b-clear",
                    onclick: move |_| state.clear_pins(),
                    "clear"
                }
            }
            div { class: "b-line",
                for (at , (key , id)) in ids.iter().enumerate() {
                    if at > 0 {
                        {thread(model, ids[at - 1].1, *id)}
                    }
                    {chip(model, key.clone(), *id, state)}
                }
            }
        }
    }
}

/// The shortest path between two pins, threaded through the whole graph: the
/// hops between them are dotted chips, and the words say which way each edge
/// runs.
fn thread(model: &NavModel, from: Option<u32>, to: Option<u32>) -> Element {
    let chain = from
        .zip(to)
        .and_then(|(from, to)| model.path_between(from, to));
    let Some(chain) = chain else {
        return rsx! {
            span { class: "b-word", "· no path ·" }
        };
    };
    let last = chain.len().saturating_sub(1);
    rsx! {
        for (at , step) in chain.iter().enumerate() {
            span {
                class: if step.flare { "b-word is-flare" } else { "b-word" },
                "{step.word}"
            }
            if at < last {
                if let Some(item) = model.item(step.id) {
                    Link {
                        to: mark_route(&item.path, &item.label),
                        class: "b-chip is-via",
                        span { class: "k", "{item.word()}" }
                        span {
                            class: if item.is_sum() { "n is-sum" } else { "n" },
                            "{item.name}"
                        }
                    }
                }
            }
        }
    }
}

/// One pin on the board: the mark, and the way to take it back off.
fn chip(model: &NavModel, key: MarkKey, id: Option<u32>, state: NavState) -> Element {
    let item = id.and_then(|id| model.item(id));
    let word = item.map(NavItem::word).unwrap_or("mark");
    let name = item
        .map(|it| it.name.clone())
        .unwrap_or_else(|| key.label.clone());
    let sum = item.is_some_and(NavItem::is_sum);
    let unpin = key.clone();
    rsx! {
        span { class: "b-chip",
            Link { to: mark_route(&key.path, &key.label), class: "b-name",
                span { class: "k", "{word}" }
                span { class: if sum { "n is-sum" } else { "n" }, "{name}" }
            }
            button {
                class: "b-unpin",
                title: "unpin",
                onclick: move |_| state.toggle_pin(unpin.clone()),
                "×"
            }
        }
    }
}
