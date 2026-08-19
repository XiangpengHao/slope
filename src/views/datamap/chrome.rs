//! Data-altitude furniture: the cartouche, the reading toggle, and the legend.
//! The same engraved plates the other two altitudes wear.

use dioxus::prelude::*;

use crate::views::codemap::chrome::{Altitude, AltitudeSwitch, plural};
use crate::views::codemap::{RefDir, use_code};
use crate::views::datamap::model::DataFacts;

/// Which top-level modules a change landed in, in plain words. The chart shows
/// a reviewer where the amber is; the cartouche says it out loud, because that
/// one sentence is the answer to why they climbed to this altitude.
fn insight(modules: &[String]) -> Option<String> {
    match modules {
        [] => None,
        [one] => Some(format!("changed types sit in {one} alone")),
        [a, b] => Some(format!("changed types sit in {a} and {b}")),
        rest => {
            let (last, first) = rest.split_last()?;
            Some(format!(
                "changed types sit in {} and {last}",
                first.join(", ")
            ))
        }
    }
}

/// The data chart's title block: what the workspace holds, what the diff moved,
/// and the reading control for the chart's reference ties.
#[component]
pub fn DataCartouche(facts: DataFacts, workspace: String, diff_line: String) -> Element {
    let insight = insight(&facts.changed_modules);
    rsx! {
        section { class: "plate pointer-events-auto",
            div { class: "px-4 pt-3 pb-2",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{workspace}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "{plural(facts.structs, \"struct\")} · {plural(facts.enums, \"enum\")} · {facts.roots} roots"
                }
                div { class: "mt-2 space-y-1 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    AltitudeSwitch { at: Altitude::Data }
                    p { class: "text-ink-soft", "{diff_line}" }
                    if facts.changed > 0 {
                        p { class: "text-flare",
                            "{plural(facts.changed, \"type\")} in changed files"
                        }
                        if let Some(insight) = insight {
                            p { class: "text-ink-soft", "{insight}" }
                        }
                    } else {
                        p { class: "text-ink-soft", "no files changed" }
                    }
                }
            }
            DataRefToggle {}
        }
    }
}

/// Which reading of the chart's reference ties is drawn. It rides on the
/// cartouche because it acts on the whole plate, and it is the same reading the
/// code map is set to — one reviewer, one question, at either altitude.
#[component]
pub fn DataRefToggle() -> Element {
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
            "aria-label": "which reading of the chart's references is drawn",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "references"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("uses", "each type's heaviest references out — what it reaches for", RefDir::Uses)}
                {seg("used by", "each type's heaviest references in — who names it", RefDir::UsedBy)}
                {seg("both", "every reference between two types, unthinned", RefDir::Both)}
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

/// The key: every mark and line the chart can draw that it cannot state for
/// itself, then the walk's own honesty notes. What the drawing already says —
/// a block is a type, the frame around it is its module — is not repeated here.
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
                        WireSample { dash: "is-owns" }
                        span {
                            span { class: "text-ink", "owns" }
                            span { class: "text-ink-soft",
                                " — a field of this type. the arrowhead rests on the holder: a shape change travels along the arrow."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-shares", width: 1.3, label: "Arc" }
                        span {
                            span { class: "text-ink", "shares" }
                            span { class: "text-ink-soft",
                                " — held through a shared handle, the wrapper's own word on the line. more than one holder can reach the same value."
                            }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-borrows", label: "&" }
                        span {
                            span { class: "text-ink", "borrows" }
                            span { class: "text-ink-soft",
                                " — a reference: the holder views state something else owns. "
                            }
                            span { class: "text-ink", "dyn" }
                            span { class: "text-ink-soft", " names a trait instead of a type." }
                        }
                    }
                    div { class: "flex items-start gap-2",
                        WireSample { dash: "is-ref", width: 1.6 }
                        span {
                            span { class: "text-ink", "references" }
                            span { class: "text-ink-soft",
                                " — lighter, and a reading rather than structure: how often one type names another, summed, with the arrow on the user."
                            }
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
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
                            span { class: "text-ink", "static" }
                            span { class: "text-ink-soft",
                                " — a root: state no type holds, drawn whether or not it is pub, with its declared type quoted under its name."
                            }
                        }
                    }
                    p {
                        span { class: "text-flare", "M" }
                        span { class: "text-ink-soft", " — defined in a file the diff touched" }
                    }
                    p {
                        span { class: "font-medium", "+ 4 plain fields" }
                        span { class: "text-ink-soft",
                            " — fields whose type walk reached no workspace type. a type that is all plain data states only its count."
                        }
                    }
                    p {
                        span { class: "font-medium", "held by 6 types" }
                        span { class: "text-ink-soft",
                            " — more than three types hold this one, so its incoming edges rest folded. hover either end to ink them in."
                        }
                    }
                    p {
                        span { class: "font-medium", "+ 5 private types" }
                        span { class: "text-ink-soft",
                            " — a private type is never a mark, and every holds edge touching one lands on its module's counted row."
                        }
                    }
                    p { class: "text-ink-soft",
                        "the references toggle sets the reading: "
                        span { class: "text-ink", "uses" }
                        " and "
                        span { class: "text-ink", "used by" }
                        " rest each type\u{2019}s two heaviest ties, "
                        span { class: "text-ink", "both" }
                        " rests every one."
                    }
                }
                div { class: "space-y-1 border-t border-ink-line pt-2.5 text-ink-soft",
                    p {
                        "the walk reads declared field types. "
                        span { class: "text-ink", "Arc" }
                        ", "
                        span { class: "text-ink", "Rc" }
                        ", "
                        span { class: "text-ink", "Weak" }
                        " and the dioxus signals — "
                        span { class: "text-ink", "Signal" }
                        ", "
                        span { class: "text-ink", "GlobalSignal" }
                        ", "
                        span { class: "text-ink", "ReadSignal" }
                        ", "
                        span { class: "text-ink", "Memo" }
                        ", "
                        span { class: "text-ink", "Resource" }
                        " — read as sharing; a reference as borrowing; "
                        span { class: "text-ink", "dyn Trait" }
                        " as its trait. every other generic type — Vec, Box, Option, HashMap, Mutex, an unknown external — is transparent, and the walk recurses into it."
                    }
                    p {
                        "references from free functions and trait items are not on this chart: a tie is kept only where both ends land on a drawn type."
                    }
                    p { "type parameters are holes and count as plain fields." }
                    if facts.trait_holds > 0 {
                        p {
                            "{plural(facts.trait_holds, \"dyn hold\")} land on a trait, and a trait has no mark of its own yet."
                        }
                    }
                    if facts.unresolved > 0 {
                        p {
                            "{facts.unresolved} names could not be resolved (type-inference limits) and are not on the chart."
                        }
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    UsageRow { gesture: "click a type", effect: "open its definition plate" }
                    UsageRow { gesture: "hover a type", effect: "all of its edges, at full ink" }
                    UsageRow { gesture: "f · ← · →", effect: "refit the chart · back · forward" }
                }
            }
        }
    }
}

/// One row of the legend's gesture section.
#[component]
fn UsageRow(gesture: &'static str, effect: &'static str) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "shrink-0 font-data text-[9.5px] tracking-[0.1em] uppercase text-ink",
                "{gesture}"
            }
            span { class: "text-ink-soft", "{effect}" }
        }
    }
}
