//! Authored icons. One family, 16-unit box, 1.5 stroke, round caps and joins,
//! so nothing here is a text glyph standing in for a drawing.

use dioxus::prelude::*;

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

#[component]
pub fn IconClear(class: String) -> Element {
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
            path { d: "M4.5 4.5 L11.5 11.5" }
            path { d: "M11.5 4.5 L4.5 11.5" }
        }
    }
}

#[component]
pub fn IconPlus(class: String) -> Element {
    rsx! {
        svg {
            class,
            view_box: "0 0 16 16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.5",
            stroke_linecap: "round",
            "aria-hidden": "true",
            path { d: "M8 3.5 V12.5" }
            path { d: "M3.5 8 H12.5" }
        }
    }
}

#[component]
pub fn IconMinus(class: String) -> Element {
    rsx! {
        svg {
            class,
            view_box: "0 0 16 16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.5",
            stroke_linecap: "round",
            "aria-hidden": "true",
            path { d: "M3.5 8 H12.5" }
        }
    }
}

/// Fit the whole sheet: a frame drawn in from four corners.
#[component]
pub fn IconFit(class: String) -> Element {
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
            path { d: "M3 6.25 V3 H6.25" }
            path { d: "M12.75 3 H16 M9.75 3 H12.75 V6.25" }
            path { d: "M13 9.75 V13 H9.75" }
            path { d: "M6.25 13 H3 V9.75" }
        }
    }
}
