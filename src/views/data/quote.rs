//! The quotation plate: one row of a sheet, opened at its source.
//!
//! This chart draws state, so the ends a sheet lists that keep none — the
//! function whose body leans on the selection, the trait it promises, the
//! method written for it — have no block on the paper. Until now they were
//! rows naming a file and a line, and reading them meant leaving the chart
//! for an editor. Clicking one now quotes it: the item's own source, lexed
//! and coloured the way a quoted row is, on a plate that opens beside the
//! sheet and leaves the selection exactly where it was.
//!
//! Nothing here is reconstructed. The server hands back the bytes the survey
//! read, minus the indent every line shared, and every run whose name the
//! survey resolved is a link — to that datum's block where the chart draws
//! one, to its own quotation where it does not. Reading the code is the same
//! move as reading the chart.

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::{CodeGraph, ItemMark};
use crate::graph::quote::{ItemSource, SrcLink, SrcRun, Tok};
use crate::load::item_source;
use crate::views::data::{Sel, mark_route, peek_route};

impl Tok {
    /// The class that inks one run. Colour inside a quotation says one thing
    /// only: what kind of token this is.
    fn class(self) -> &'static str {
        match self {
            Tok::Kw => "tok-kw",
            Tok::Comment => "tok-comment",
            Tok::Doc => "tok-doc",
            Tok::Str => "tok-str",
            Tok::Num => "tok-num",
            Tok::Lifetime => "tok-lifetime",
            Tok::Attr => "tok-attr",
            Tok::Type => "tok-type",
            Tok::Fn => "tok-fn",
            Tok::Macro => "tok-macro",
            Tok::Punct => "tok-punct",
            Tok::Ident | Tok::Space => "tok-ident",
        }
    }
}

/// Where a run inside a quotation goes: the datum's own block where this
/// chart draws one, its quotation where it does not, and nowhere at all for a
/// reference to a whole file — a module has no place on this altitude.
fn run_route(graph: &CodeGraph, sel: &Sel, link: &SrcLink) -> Option<Route> {
    if link.label.is_empty() {
        return None;
    }
    let far = find(graph, &link.path, &link.label)?;
    match far.head.kind.is_data() && far.parent.is_none() {
        true => Some(mark_route(&link.path, &link.label)),
        false => Some(peek_route(sel, &link.path, &link.label)),
    }
}

/// What the gutter says where the file has lines this quotation does not
/// carry: the engraver's own elision, and nothing else.
const GAP: &str = "⋮";

/// The item a (file, label) pair names in this survey.
fn find<'g>(graph: &'g CodeGraph, path: &str, label: &str) -> Option<&'g ItemMark> {
    graph.items.iter().find(|m| {
        m.head.label == label
            && graph
                .files
                .get(m.file as usize)
                .is_some_and(|f| f.path == path)
    })
}

/// One row of a sheet, quoted: the declaration it names, where it is written,
/// and its own source. The plate stands beside the sheet, so the row that
/// opened it stays inked and readable next to what it says.
#[component]
pub(super) fn Quotation(graph: CodeGraph, sel: Sel, path: String, label: String) -> Element {
    let close = mark_route(&sel.0, &sel.1);
    let Some(mark) = find(&graph, &path, &label) else {
        return rsx! {
            section { class: "plate pointer-events-auto w-full max-w-[34rem] px-4 py-3 sm:w-auto",
                p { class: "font-data text-[11px] text-ink",
                    "Nothing named “{label}” is written in {path} on this survey."
                }
                Link {
                    class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: close,
                    "close"
                }
            }
        };
    };
    // The kind's own word, and not the sheet row's `pub(super) fn`: the
    // survey reads `pub(super)` and `pub(in path)` as one crate-wide
    // visibility, and a head that said `pub(crate)` would contradict the
    // source quoted two lines under it. What a declaration publishes is
    // written in the quotation itself, which is the authority here.
    let decl = mark.head.kind.words();
    let locator = format!("{path}:{}", mark.head.line);
    let id = mark.id;
    rsx! {
        // The plate takes the room between the cartouche and the sheet and
        // stops: a hundred columns of rust fit on a desktop, and neither piece
        // of furniture is ever covered. Beyond that the pane scrolls.
        section { class: "plate pointer-events-auto flex max-h-[62dvh] w-full flex-col overflow-hidden sm:max-h-[calc(100dvh-1.5rem)] sm:w-auto sm:max-w-[min(46rem,calc(100vw-37rem))]",
            div { class: "flex items-baseline gap-3 px-4 pt-3 pb-2",
                div { class: "min-w-0 flex-1",
                    h2 { class: "flex items-baseline gap-1.5 font-data text-[13px]",
                        span { class: "shrink-0 text-[10.5px] text-ink-soft", "{decl}" }
                        span { class: "truncate font-semibold text-ink", "{label}" }
                    }
                    p { class: "mt-0.5 font-data text-[9.5px] text-ink-soft", "{locator}" }
                }
                Link {
                    class: "shrink-0 font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: close,
                    "close ×"
                }
            }
            Quoted { graph, sel, item: id, path }
        }
    }
}

/// The source itself, fetched for one item and quoted. Its own component, so
/// the plate's head is drawn from the survey the client already has while the
/// bytes are still on the way.
#[component]
fn Quoted(graph: CodeGraph, sel: Sel, item: u32, path: String) -> Element {
    // Reactive on the item, not merely captured with it: stepping from one row
    // of a sheet to the next changes this component's props and nothing else,
    // and a fetch keyed on the first item it ever saw would leave the second
    // row's plate quoting the first row's source.
    let source = use_resource(use_reactive((&item,), |(item,)| async move {
        item_source(item).await
    }));
    let state = source.read();
    match &*state {
        None => rsx! {
            p { class: "px-4 pb-3 font-data text-[10px] text-ink-soft", "reading {path}…" }
        },
        Some(Err(err)) => rsx! {
            p { class: "px-4 pb-3 font-data text-[10px] leading-relaxed text-ink-soft",
                "{err}"
            }
        },
        Some(Ok(source)) => rsx! {
            Pane { graph, sel, source: source.clone() }
        },
    }
}

/// The item's own source, as the reviewer's editor would show it: a gutter
/// counting from its first line in the real file, no wrapping, and the text
/// selectable so it can be copied straight off the plate. A long definition
/// scrolls inside the pane; nothing is cut.
///
/// A method is drawn inside the `impl` or `trait` block it is written in —
/// the header, the method at the indent it stands at, the brace that closes
/// the block — because `fn edge_style(self, …)` alone is not rust and says
/// nothing about whose method it is.
#[component]
fn Pane(graph: CodeGraph, sel: Sel, source: ItemSource) -> Element {
    let nav = use_navigator();
    // The rows the pane draws: every block in source order, and — wherever the
    // file writes lines between two of them that this quotation does not carry
    // — one row whose gutter says so. Not how many: a count of skipped lines
    // is a number nobody acts on, and the gutter's own numbers already say how
    // far the jump is.
    let no_runs: &[SrcRun] = &[];
    let mut rows: Vec<(String, &[SrcRun])> = Vec::new();
    let mut quoted: Option<u32> = None;
    for block in source.blocks.iter() {
        if quoted.is_some_and(|last| last + 1 < block.first_line) {
            rows.push((GAP.to_string(), no_runs));
        }
        for (i, line) in block.lines.iter().enumerate() {
            rows.push(((block.first_line + i as u32).to_string(), line));
        }
        quoted = Some(block.first_line + block.lines.len().saturating_sub(1) as u32);
    }
    rsx! {
        div { class: "quote-pane min-h-0 flex-1",
            div { class: "quote-lines",
                for (i , (at , line)) in rows.iter().enumerate() {
                    div { key: "{i}", class: "quote-line",
                        span {
                            class: "quote-ln",
                            class: if at == GAP { "is-gap" },
                            "{at}"
                        }
                        span { class: "quote-src",
                            for (n , run) in line.iter().enumerate() {
                                {
                                    let target = run
                                        .link
                                        .and_then(|l| source.links.get(l as usize))
                                        .and_then(|link| {
                                            run_route(&graph, &sel, link).map(|route| (route, link.clone()))
                                        });
                                    match target {
                                        Some((route, link)) => {
                                            let push = route.clone();
                                            rsx! {
                                                a {
                                                    key: "{n}",
                                                    class: "{run.tok.class()} tok-link",
                                                    href: route.to_string(),
                                                    title: "{link.label} · {link.path}",
                                                    onclick: move |e: Event<MouseData>| {
                                                        e.prevent_default();
                                                        e.stop_propagation();
                                                        nav.push(push.clone());
                                                    },
                                                    "{run.text}"
                                                }
                                            }
                                        }
                                        None => rsx! {
                                            span { key: "{n}", class: run.tok.class(), "{run.text}" }
                                        },
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
