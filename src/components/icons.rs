//! Authored icons. One family, 16-unit box, 1.5 stroke, round caps and joins,
//! so nothing here is a text glyph standing in for a drawing.

use dioxus::prelude::*;

/// The product's mark: a node, a wire, a node. The whole tool in one glyph, and
/// the same two shapes the pane draws — a filled card for something this
/// workspace builds, an outlined one for something it pulls in.
#[component]
pub fn Mark() -> Element {
    rsx! {
        svg {
            class: "h-[18px] w-[18px] shrink-0",
            view_box: "0 0 18 18",
            fill: "none",
            "aria-hidden": "true",
            rect {
                x: "1",
                y: "5.25",
                width: "5.5",
                height: "4",
                rx: "1.25",
                fill: "var(--color-ink)",
            }
            path {
                d: "M6.5 7.25 C9 7.25 8 12.25 11 12.25",
                stroke: "var(--color-outbound)",
                stroke_width: "1.4",
                stroke_linecap: "round",
            }
            rect {
                x: "11",
                y: "10.25",
                width: "5.5",
                height: "4",
                rx: "1.25",
                fill: "var(--color-surface)",
                stroke: "var(--color-line-strong)",
                stroke_width: "1.2",
            }
            rect {
                x: "11",
                y: "2.25",
                width: "5.5",
                height: "4",
                rx: "1.25",
                fill: "var(--color-surface)",
                stroke: "var(--color-line-strong)",
                stroke_width: "1.2",
            }
            path {
                d: "M6.5 7.25 C9 7.25 8 4.25 11 4.25",
                stroke: "var(--color-line-strong)",
                stroke_width: "1.4",
                stroke_linecap: "round",
            }
        }
    }
}

#[component]
pub fn IconFind(class: String) -> Element {
    rsx! {
        svg {
            class,
            view_box: "0 0 16 16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.5",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            circle { cx: "7", cy: "7", r: "4.25" }
            path { d: "M10.2 10.2 L14 14" }
        }
    }
}
