//! The quotation plate at the function altitude: one row of a sheet, opened at
//! its source.
//!
//! The rows a sheet lists that this chart draws no block for — a type it
//! touches that the data chart draws none for either, a declaration narrower
//! than the reading — are quoted here rather than left as a file and a line.
//! Nothing is reconstructed: the server hands back the bytes the survey read,
//! minus the indent every line shared.
//!
//! What is different one rung down is where a resolved name goes. A name inside
//! a quoted body is usually a *call*, and this chart draws calls: those runs
//! re-centre the chart on the declaration they name. A name that resolves to a
//! type steps down to the altitude that draws types, and the reader lands on
//! its block there. Reading the code is the same move as reading either chart.

use dioxus::prelude::*;

use crate::Route;
use crate::graph::data::CodeGraph;
use crate::graph::quote::{ItemSource, SrcLink, SrcRun};
use crate::load::item_source;
use crate::views::func::{Sel, mark_route, peek_route};

impl SrcLink {
    /// Where a run inside a quotation goes: this chart's own block for anything
    /// that runs, the data chart's block for a type it draws, and this chart's
    /// quotation plate for everything else. A reference to a whole file goes
    /// nowhere — a module has no place at either altitude.
    fn route(&self, graph: &CodeGraph, sel: &Sel) -> Option<Route> {
        if self.label.is_empty() {
            return None;
        }
        let far = graph.declared(&self.path, &self.label)?;
        if far.head.kind.is_callable() {
            return Some(mark_route(&self.path, &self.label));
        }
        if far.head.kind.is_data() && far.parent.is_none() {
            return Some(crate::views::data::mark_route(&self.path, &self.label));
        }
        Some(peek_route(sel, &self.path, &self.label))
    }
}

/// What the gutter says where the file has lines this quotation does not carry.
const GAP: &str = "⋮";

/// One row of a sheet, quoted: the declaration it names, where it is written,
/// and its own source, on a plate that stands beside the sheet.
#[component]
pub(super) fn FnQuotation(graph: CodeGraph, sel: Sel, path: String, label: String) -> Element {
    let close = mark_route(&sel.0, &sel.1);
    let Some(mark) = graph.declared(&path, &label) else {
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
    // The kind's own word, never the sheet row's `pub(super) fn`: what a
    // declaration publishes is written in the quotation itself, which is the
    // authority here.
    let decl = mark.head.kind.words();
    let locator = format!("{path}:{}", mark.head.line);
    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[62dvh] w-full flex-col overflow-hidden sm:max-h-[calc(100dvh-4.25rem)] sm:w-auto sm:max-w-[min(46rem,calc(100vw-37rem))]",
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
            Quoted { graph, sel, path, label }
        }
    }
}

/// The source itself, fetched for one item and quoted. Its own component, so
/// the plate's head is drawn from the survey the client already has while the
/// bytes are still on the way.
#[component]
fn Quoted(graph: CodeGraph, sel: Sel, path: String, label: String) -> Element {
    let source = use_resource(use_reactive((&path, &label), |(path, label)| async move {
        item_source(path, label).await
    }));
    let state = source.read();
    match &*state {
        None => rsx! {
            p { class: "px-4 pb-3 font-data text-[10px] text-ink-soft", "reading {path}…" }
        },
        Some(Err(err)) => rsx! {
            p { class: "px-4 pb-3 font-data text-[10px] leading-relaxed text-ink-soft", "{err}" }
        },
        Some(Ok(source)) => rsx! {
            Pane { graph, sel, source: source.clone() }
        },
    }
}

/// The item's own source, as the reviewer's editor would show it: a gutter
/// counting from its first line in the real file, no wrapping, and the text
/// selectable so it can be copied straight off the plate.
#[component]
fn Pane(graph: CodeGraph, sel: Sel, source: ItemSource) -> Element {
    let nav = use_navigator();
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
                                            link.route(&graph, &sel).map(|route| (route, link.clone()))
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
