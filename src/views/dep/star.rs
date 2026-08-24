//! The star: how one crate is drawn on the chart.
//!
//! Engraved-ink discipline: everything is drawn in the chart's ink except
//! change, which is the only thing on the page allowed to take color.

use dioxus::prelude::*;
use dioxus_flow::prelude::*;

use crate::Route;
use crate::api::CrateInfo;
use crate::views::dep::use_dep;

/// Payload carried by chart nodes: one crate's star on the rings.
#[derive(Clone, PartialEq)]
pub(super) struct StarData {
    pub(crate) info: CrateInfo,
    /// Dependency distance from the chart's center.
    pub(crate) ring: u32,
    /// Unit direction from the chart center, so the label seats on the
    /// side facing away from the center's crowd.
    pub(crate) ux: f64,
    pub(crate) uy: f64,
    /// The selected crate: the one whose edges the chart is drawing.
    pub(crate) focal: bool,
    /// Label engraved at rest; unnamed stars reveal theirs on hover/focus.
    pub(crate) named: bool,
    /// Another version of this same crate is on the chart, so the label has
    /// to carry the version or the two stars read as one crate drawn twice.
    pub(crate) versioned: bool,
}

/// Star radius from magnitude (how many crates depend on this one).
pub(super) fn star_radius(dependents: u32) -> f64 {
    (4.0 + (dependents as f64).sqrt() * 1.3).min(11.0)
}

/// Room the mark needs beyond its core circle: the focal ring's ticks reach
/// farthest (r + 9.5), changed flare rays reach r + 6.5.
const MARK_OVERHANG: f64 = 11.0;

/// The square node box for one crate's star, from its magnitude. Constant
/// per crate so selection never moves a star.
pub(super) fn star_box(info: &CrateInfo) -> f64 {
    2.0 * (star_radius(info.dependents) + MARK_OVERHANG)
}

/// The state a star announces, in words — never color alone, and in the words
/// the tools themselves use.
fn state_words(info: &CrateInfo) -> Option<String> {
    if info.ghost {
        return Some("removed".into());
    }
    if info.changed {
        let files = info.changed_files;
        return Some(if files == 1 {
            "1 file changed".into()
        } else {
            format!("{files} files changed")
        });
    }
    if let Some(dist) = info.affected_dist {
        return Some(if dist == 1 {
            "1 hop downstream".into()
        } else {
            format!("{dist} hops downstream")
        });
    }
    None
}

/// The engraved star mark for one crate, reused by the chart nodes and the
/// legend so the key and the chart can never drift apart.
#[component]
pub(super) fn StarMark(
    info: CrateInfo,
    focal: bool,
    #[props(default = 32.0)] box_px: f64,
) -> Element {
    let changed = info.changed;
    let affected = info.affected_dist.filter(|_| !changed);
    // Everything drawn outside the core circle must still fit the box.
    let overhang = if focal {
        10.0
    } else if changed {
        7.5
    } else if affected.is_some() {
        5.0
    } else {
        3.5
    };
    let r = star_radius(info.dependents)
        .min(box_px / 2.0 - overhang)
        .max(2.5);
    let c = box_px / 2.0;
    // The halo fades with distance from the change: nearer is stronger.
    let halo_opacity = affected
        .map(|d| (0.85 - 0.22 * (d.saturating_sub(1)) as f64).max(0.3))
        .unwrap_or(0.0);

    rsx! {
        svg {
            class: "shrink-0",
            width: "{box_px}",
            height: "{box_px}",
            view_box: "0 0 {box_px} {box_px}",
            "aria-hidden": "true",

            // Flare rays: only a changed crate may emit light.
            if changed {
                g {
                    stroke: "var(--color-flare)",
                    stroke_width: "1.1",
                    stroke_linecap: "round",
                    for angle in [0, 45, 90, 135, 180, 225, 270, 315] {
                        line {
                            x1: "{c + (r + 2.5) * (angle as f64).to_radians().cos()}",
                            y1: "{c + (r + 2.5) * (angle as f64).to_radians().sin()}",
                            x2: "{c + (r + 6.5) * (angle as f64).to_radians().cos()}",
                            y2: "{c + (r + 6.5) * (angle as f64).to_radians().sin()}",
                        }
                    }
                }
            }

            // Blast-radius halo: an amber ring graded by hop distance.
            if affected.is_some() {
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r + 3.5}",
                    fill: "none",
                    stroke: "var(--color-flare)",
                    stroke_width: "1.6",
                    opacity: "{halo_opacity}",
                }
            }

            // Focal ring: the double circle of "you are here", plus ticks.
            if focal {
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r + 6.0}",
                    fill: "none",
                    stroke: "var(--color-ink)",
                    stroke_width: "0.7",
                }
                g {
                    stroke: "var(--color-ink)",
                    stroke_width: "0.9",
                    for angle in [0, 90, 180, 270] {
                        line {
                            x1: "{c + (r + 6.0) * (angle as f64).to_radians().cos()}",
                            y1: "{c + (r + 6.0) * (angle as f64).to_radians().sin()}",
                            x2: "{c + (r + 9.5) * (angle as f64).to_radians().cos()}",
                            y2: "{c + (r + 9.5) * (angle as f64).to_radians().sin()}",
                        }
                    }
                }
            }

            if info.ghost {
                // A removed dependency: only its dashed outline remains.
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r}",
                    fill: "none",
                    stroke: "var(--color-ink-soft)",
                    stroke_width: "1.2",
                    stroke_dasharray: "3 2.5",
                }
            } else if info.is_member {
                // Members: solid ink, ringed like a named star on the plate.
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r}",
                    fill: if changed { "var(--color-flare)" } else { "var(--color-ink)" },
                }
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r + 2.2}",
                    fill: "none",
                    stroke: if changed { "var(--color-flare)" } else { "var(--color-ink)" },
                    stroke_width: "0.6",
                    opacity: "0.85",
                }
            } else {
                // External crates: open circles, the unnamed field.
                circle {
                    cx: "{c}",
                    cy: "{c}",
                    r: "{r}",
                    fill: "var(--color-paper)",
                    stroke: if changed { "var(--color-flare)" } else { "var(--color-ink)" },
                    stroke_width: "1.3",
                }
            }
        }
    }
}

/// Node view for the rings chart: the mark, and the name seated on the side
/// facing away from the center. Every live star is a link — clicking selects
/// it and draws its edges; clicking the selected star steps back to the
/// whole chart.
#[component]
pub(super) fn StarNode(ctx: NodeViewCtx<StarData>) -> Element {
    let StarData {
        info,
        ring,
        ux,
        uy,
        focal,
        named,
        versioned,
    } = ctx.node.data.clone();
    let box_px = star_box(&info);

    // Label side: outward from the center, so it points into open paper.
    // Sideways seats win except near dead vertical — side-by-side names
    // collide far less than names stacked above or below a crowded arc.
    let side = if ring == 0 {
        "lab-s"
    } else if ux > 0.18 {
        "lab-e"
    } else if ux < -0.18 {
        "lab-w"
    } else if uy < 0.0 {
        "lab-n"
    } else {
        "lab-s"
    };

    let hops = match ring {
        0 => "center".to_string(),
        1 => "1 hop".to_string(),
        n => format!("{n} hops"),
    };
    let state = state_words(&info)
        .map(|w| format!(" · {w}"))
        .unwrap_or_default();
    let title = format!("{} v{} · {hops}{state}", info.name, info.version);
    let aria = if focal {
        format!(
            "deselect {} — ctrl-click removes it from a wider selection",
            info.name
        )
    } else {
        format!(
            "select {} and draw its edges — ctrl-click adds it to the selection",
            info.name
        )
    };

    // Plain click replaces the selection; ctrl / cmd / shift-click toggles
    // this crate in the current one. The href stays the solo route, so
    // middle-click still opens the crate in its own tab.
    let dep = use_dep();
    let nav = use_navigator();
    let solo_href = Route::DepFocus {
        name: info.name.clone(),
    }
    .to_string();
    let click_name = info.name.clone();
    let onclick = move |e: Event<MouseData>| {
        e.prevent_default();
        e.stop_propagation();
        let m = e.modifiers();
        let name = click_name.clone();
        if m.ctrl() || m.meta() || m.shift() {
            let mut set = dep.selected.peek().clone();
            match set.iter().position(|n| n == &name) {
                Some(i) => {
                    set.remove(i);
                }
                None => set.push(name),
            }
            if set.is_empty() {
                nav.push(Route::DepOverview {});
            } else {
                nav.push(Route::DepFocus {
                    name: set.join("+"),
                });
            }
        } else if focal && dep.selected.peek().len() == 1 {
            nav.push(Route::DepOverview {});
        } else {
            nav.push(Route::DepFocus { name });
        }
    };

    let body = rsx! {
        StarMark { info: info.clone(), focal, box_px }
        span {
            class: "star-label {side} font-data",
            class: if info.is_member && !info.ghost { "font-medium text-ink" } else { "text-ink-soft" },
            "{info.name}"
            if versioned {
                span { class: "star-version", " v{info.version}" }
            }
        }
    };

    rsx! {
        div {
            class: "star-node",
            class: if focal { "is-focal" },
            class: if named || focal { "is-named" },
            class: if info.ghost { "is-ghost" },
            title: "{title}",
            // Enter and Space must reach the link's native activation, not
            // the flow's node-selection handler above.
            onkeydown: move |e| {
                if e.key() == Key::Enter || e.key() == Key::Character(" ".to_string()) {
                    e.stop_propagation();
                }
            },
            if info.ghost {
                div { class: "star-link", {body} }
            } else {
                a {
                    href: "{solo_href}",
                    class: "star-link",
                    aria_label: "{aria}",
                    onclick,
                    {body}
                }
            }
        }
    }
}
