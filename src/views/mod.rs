//! The lenses, and the chrome they share.

use dioxus::prelude::*;

mod calls;
pub mod deps;
mod inspector;
mod shell;

pub use calls::{Calls, SheetState};
pub use deps::{Deps, DepsState};
pub use shell::{GraphResource, SheetResource, Shell};

/// The panel docked beside the pane. Both lenses use it, so a reader who has
/// learned where the answer appears in one has learned the other.
///
/// Beside the pane on a desktop, under it on a phone, and scrolling on its own
/// either way: the pane must never lose its height to a long record.
#[component]
pub fn Panel(label: String, children: Element) -> Element {
    rsx! {
        aside {
            class: "flex max-h-[45vh] w-full shrink-0 flex-col overflow-y-auto overscroll-contain border-t border-line bg-surface lg:max-h-none lg:w-[358px] lg:border-t-0 lg:border-l",
            "aria-label": "{label}",
            {children}
        }
    }
}

/// The one thing that moves while the server is thinking: a wire being walked.
/// It belongs to waiting and to nothing else.
/// Why the camera is moving, which decides what it frames — and, by being asked
/// for explicitly, which actions move it at all.
///
/// Both lenses follow the same rule. Pointing at a card on the pane asks for
/// nothing: the reader can already see it, they just aimed at it, and taking the
/// ground out from under the click removes what they aimed with. Naming a
/// thing — from the finder, a row in the record, an arrow key — is the opposite
/// case, because there is no telling whether it is even on screen.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Aim {
    /// Frame it and whatever it is attached to.
    Neighbourhood(usize),
    /// Frame the whole chain that explains it, end to end. Something that
    /// arrived from off the pane comes with that chain, and the chain is the
    /// answer to the question that summoned it.
    Route(usize),
}

#[component]
pub fn WaitingWire() -> Element {
    rsx! {
        svg {
            class: "h-8 w-44",
            view_box: "0 0 176 32",
            fill: "none",
            "aria-hidden": "true",
            path {
                d: "M8,16 C48,16 48,6 88,6 C128,6 128,26 168,26",
                stroke: "var(--color-line-strong)",
                stroke_width: "1.5",
            }
            path {
                d: "M8,16 C48,16 48,6 88,6 C128,6 128,26 168,26",
                stroke: "var(--color-outbound)",
                stroke_width: "2",
                stroke_dasharray: "12 240",
                class: "animate-[wire-travel_1.6s_linear_infinite]",
            }
            circle { cx: "8", cy: "16", r: "3.5", fill: "var(--color-ink)" }
            circle {
                cx: "168",
                cy: "26",
                r: "3.5",
                fill: "var(--color-surface)",
                stroke: "var(--color-line-strong)",
                stroke_width: "1.5",
            }
        }
    }
}
