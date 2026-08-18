//! Chart furniture: cartouche, changes queue, legend, search, and the focus
//! panel. All of it is drawn in the same engraved ink as the chart itself.

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CrateInfo, DepEvent, DepKind, WorkspaceGraph};
use crate::views::radial::{DEFAULT_CAP, radial_layout};
use crate::views::shell::{DirFilter, history_back, use_atlas};
use crate::views::star::StarMark;

/// A synthetic crate for legend samples, so the key is drawn by the exact
/// same code as the chart and can never drift from it.
fn sample(dependents: u32, is_member: bool) -> CrateInfo {
    CrateInfo {
        id: String::new(),
        name: String::new(),
        version: String::new(),
        is_member,
        changed: false,
        changed_files: 0,
        manifest_changed: false,
        affected_dist: None,
        dependents,
        direct_deps: 0,
        external_deps: 0,
        ghost: false,
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The chart's title block: workspace name, epoch, and the change count.
#[component]
pub fn Cartouche(graph: WorkspaceGraph) -> Element {
    let members = graph.crates.iter().filter(|c| c.is_member).count();
    let externals = graph
        .crates
        .iter()
        .filter(|c| !c.is_member && !c.ghost)
        .count();
    let changed = graph.crates.iter().filter(|c| c.changed).count();
    let affected = graph
        .crates
        .iter()
        .filter(|c| !c.changed && c.affected_dist.is_some())
        .count();
    let epoch = &graph.epoch;

    rsx! {
        section { class: "plate pointer-events-auto px-4 py-3",
            h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                "{graph.name}"
            }
            p { class: "mt-0.5 font-chart text-[12px] italic text-ink-soft",
                "dependency atlas · {plural(members, \"workspace crate\")} · {externals} external"
            }
            div { class: "mt-2 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                p {
                    span { class: "text-ink-soft", "epoch " }
                    "{epoch.base} → {epoch.target}"
                }
                if let Some(note) = &epoch.note {
                    p { class: "mt-1 text-ink-soft", "{note}" }
                } else if epoch.clean {
                    p { class: "mt-1", "Epoch clean — the working copy matches {epoch.base}." }
                } else {
                    p { class: "mt-1",
                        span { class: "font-medium text-flare", "{changed} changed" }
                        span { class: "text-ink-soft", " · " }
                        span { "{affected} affected downstream" }
                    }
                }
            }
        }
    }
}

/// The review queue: every changed crate, each a link to its blast radius,
/// each marked SEEN once visited. This is the agenda the chart serves.
#[component]
pub fn ChangesQueue(graph: WorkspaceGraph, #[props(default = true)] start_open: bool) -> Element {
    let atlas = use_atlas();
    let epoch = graph.epoch.clone();
    let mut changed: Vec<CrateInfo> = graph.crates.iter().filter(|c| c.changed).cloned().collect();
    changed.sort_by(|a, b| a.name.cmp(&b.name));
    let visited = atlas.visited.read();
    let seen = changed
        .iter()
        .filter(|c| visited.contains(&c.name))
        .count();
    let total = changed.len();
    let focus = atlas.trail.read().current_focus();

    rsx! {
        details { class: "plate pointer-events-auto w-full open:pb-3", open: start_open,
            summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Changes ({total})"
            }
            if let Some(note) = &epoch.note {
                p { class: "px-4 font-data text-[10px] leading-relaxed text-ink-soft", "{note}" }
            } else if total == 0 {
                p { class: "px-4 font-data text-[10px] leading-relaxed text-ink",
                    "NOTHING CHANGED"
                }
                p { class: "px-4 pt-0.5 font-data text-[10px] leading-relaxed text-ink-soft",
                    "the working copy matches {epoch.base}; the chart shows the whole workspace"
                }
            } else {
                ul { class: "max-h-56 overflow-y-auto px-2.5",
                    for info in changed {
                        li {
                            Link {
                                to: Route::Focus { name: info.name.clone() },
                                class: if focus.as_deref() == Some(info.name.as_str()) {
                                    "flex w-full items-center gap-1.5 px-1.5 py-0.5 bg-ink/5"
                                } else {
                                    "flex w-full items-center gap-1.5 px-1.5 py-0.5 hover:bg-ink/5"
                                },
                                StarMark { info: info.clone(), focal: false, box_px: 20.0 }
                                span { class: "truncate font-data text-[11px] font-medium text-ink",
                                    "{info.name}"
                                }
                                span { class: "shrink-0 font-data text-[9px] tracking-[0.1em] text-flare",
                                    if info.changed_files == 1 { "1 FILE" } else { "{info.changed_files} FILES" }
                                }
                                if info.manifest_changed {
                                    span { class: "shrink-0 font-data text-[9px] tracking-[0.1em] text-flare",
                                        "MANIFEST"
                                    }
                                }
                                if visited.contains(&info.name) {
                                    span { class: "ml-auto shrink-0 font-data text-[9px] tracking-[0.12em] text-ink-soft",
                                        "SEEN"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "mx-4 mt-2 border-t border-ink-line pt-2 font-data text-[9.5px] tracking-[0.1em] uppercase",
                    if seen == total {
                        p { class: "text-ink", "ALL {total} SEEN" }
                    } else {
                        p { class: "text-ink", "{seen} OF {total} SEEN" }
                    }
                    p { class: "mt-1 normal-case tracking-normal text-ink-soft",
                        "changed crates flare on the rings — open one to chart what its change can reach"
                    }
                }
            }
        }
    }
}

/// The edge-direction toggle: which side of the selection's edges the chart
/// draws. Manifest events are always drawn regardless. Active segment wears
/// a 1px ink border — no fills on this plate, ever.
#[component]
pub fn DirectionToggle() -> Element {
    let atlas = use_atlas();
    let current = *atlas.dir.read();
    let seg = |label: &'static str, val: DirFilter| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                onclick: move |_| {
                    let mut dir = atlas.dir;
                    dir.set(val);
                },
                "{label}"
            }
        }
    };
    rsx! {
        div { class: "plate pointer-events-auto flex items-center gap-0.5 px-2 py-1",
            span { class: "shrink-0 pr-1.5 font-chart text-[10px] tracking-[0.18em] uppercase text-ink",
                "Edges"
            }
            {seg("depends on", DirFilter::Deps)}
            {seg("both", DirFilter::Both)}
            {seg("used by", DirFilter::Users)}
        }
    }
}

/// One line sample for the legend's edge key.
#[component]
fn LineSample(
    dasharray: &'static str,
    stroke: &'static str,
    #[props(default = 1.2)] width: f64,
) -> Element {
    rsx! {
        svg { width: "34", height: "8", view_box: "0 0 34 8", class: "shrink-0", "aria-hidden": "true",
            line {
                x1: "0",
                y1: "4",
                x2: "32",
                y2: "4",
                stroke,
                stroke_width: "{width}",
                stroke_dasharray: "{dasharray}",
            }
            path {
                d: "M27.5 1.4 L32.5 4 L27.5 6.6",
                fill: "none",
                stroke,
                stroke_width: "1.1",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

/// The legend's tiny rings sample: the chart's shape in miniature.
#[component]
fn RingsSample() -> Element {
    rsx! {
        svg { width: "30", height: "30", view_box: "0 0 30 30", class: "shrink-0", "aria-hidden": "true",
            circle { cx: "15", cy: "15", r: "6", fill: "none", stroke: "var(--color-ink-line)", stroke_width: "0.8" }
            circle { cx: "15", cy: "15", r: "11", fill: "none", stroke: "var(--color-ink-line)", stroke_width: "0.8" }
            circle { cx: "15", cy: "15", r: "2.4", fill: "var(--color-ink)" }
            circle { cx: "19.2", cy: "10.8", r: "1.7", fill: "var(--color-ink)" }
            circle { cx: "7.2", cy: "22.8", r: "1.4", fill: "var(--color-paper)", stroke: "var(--color-ink)", stroke_width: "0.9" }
        }
    }
}

/// One row of the legend's "using this chart" section.
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

/// The key. Every state the chart can draw, named in words — and every
/// gesture the chart answers to, taught in the same plate.
#[component]
pub fn Legend(#[props(default = true)] start_open: bool, center: String) -> Element {
    let changed = CrateInfo {
        changed: true,
        changed_files: 3,
        ..sample(4, true)
    };
    let affected = CrateInfo {
        affected_dist: Some(1),
        ..sample(4, true)
    };
    let ghost = CrateInfo {
        ghost: true,
        ..sample(0, false)
    };

    rsx! {
        details { class: "plate pointer-events-auto w-full open:pb-3 sm:w-64", open: start_open,
            summary {
                class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this chart"
            }
            div { class: "space-y-2.5 px-4 font-data text-[10px] leading-snug text-ink sm:max-h-[38dvh] sm:overflow-y-auto",
                div { class: "flex items-center gap-2",
                    RingsSample {}
                    span {
                        span { class: "font-medium", "{center}" }
                        " sits at the center; each ring outward is one more dependency hop — the outermost ring gathers everything farther, and expands as you select into it"
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    UsageRow { gesture: "click a star", effect: "select it — the chart draws its edges" }
                    UsageRow { gesture: "ctrl-click", effect: "add or remove a star from the selection" }
                    UsageRow { gesture: "hop label", effect: "select every crate on that ring" }
                    UsageRow { gesture: "edges toggle", effect: "draw one direction of the selection's edges" }
                    UsageRow { gesture: "back / esc", effect: "deselect, back to the whole chart" }
                    UsageRow { gesture: "drag · scroll", effect: "pan the paper · zoom" }
                    UsageRow { gesture: "n / p", effect: "next / previous changed crate" }
                    UsageRow { gesture: "/ · f", effect: "find a crate · refit the chart" }
                }
                div { class: "flex items-center gap-2 border-t border-ink-line pt-2.5",
                    div { class: "flex items-end",
                        StarMark { info: sample(0, true), focal: false, box_px: 18.0 }
                        StarMark { info: sample(9, true), focal: false, box_px: 22.0 }
                        StarMark { info: sample(60, true), focal: false, box_px: 28.0 }
                    }
                    span { "stars are sized by how many crates depend on them" }
                }
                div { class: "flex items-center gap-2",
                    StarMark { info: sample(4, true), focal: false, box_px: 20.0 }
                    span { "workspace member — solid ink" }
                }
                div { class: "flex items-center gap-2",
                    StarMark { info: sample(4, false), focal: false, box_px: 20.0 }
                    span { "external crate — open circle" }
                }
                div { class: "border-t border-ink-line pt-2.5 space-y-2",
                    div { class: "flex items-center gap-2",
                        StarMark { info: changed, focal: false, box_px: 26.0 }
                        span {
                            span { class: "font-medium text-flare", "CHANGED" }
                            " — its files were edited in this epoch"
                        }
                    }
                    div { class: "flex items-center gap-2",
                        StarMark { info: affected, focal: false, box_px: 26.0 }
                        span {
                            span { class: "font-medium", "AFFECTED" }
                            " — depends on a change; the ring fades with distance"
                        }
                    }
                    div { class: "flex items-center gap-2",
                        StarMark { info: ghost, focal: false, box_px: 20.0 }
                        span {
                            span { class: "font-medium", "REMOVED" }
                            " — a dependency this epoch deleted"
                        }
                    }
                }
                div { class: "border-t border-ink-line pt-2.5 space-y-1.5",
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "", stroke: "var(--color-ink)", width: 1.25 }
                        span { "ink, arrow in: the selected crate depends on it" }
                    }
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "", stroke: "var(--color-ink-line)", width: 1.1 }
                        span { "hairline, arrow out: it depends on the selected crate" }
                    }
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "6 4", stroke: "var(--color-ink-line)" }
                        span { "dev dependency — dashed (build: dotted)" }
                    }
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "", stroke: "var(--color-flare)", width: 1.4 }
                        span { "manifest event — added, removed, or bumped" }
                    }
                    p { class: "pt-1 text-ink-soft",
                        "arrows point the way change travels — into the crate that uses the dependency; edges are drawn for the selected crate only, manifest events always show"
                    }
                }
            }
        }
    }
}

/// Search: find a crate by name and focus it. Arrows walk the hits, Enter
/// opens the marked one, Escape clears.
#[component]
pub fn SearchBox(graph: WorkspaceGraph) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let nav = use_navigator();

    let results = use_memo(move || {
        let q = query().trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<CrateInfo> = graph
            .crates
            .iter()
            .filter(|c| !c.ghost && c.name.to_lowercase().contains(&q))
            .cloned()
            .collect();
        hits.sort_by_key(|c| {
            (
                !c.name.to_lowercase().starts_with(&q),
                !c.is_member,
                std::cmp::Reverse(c.dependents),
            )
        });
        hits.truncate(9);
        hits
    });

    rsx! {
        div { class: "pointer-events-auto relative w-full",
            input {
                id: "atlas-search",
                class: "plate w-full px-3 py-1.5 font-data text-[11px] text-ink placeholder:text-ink-soft focus:outline-none",
                r#type: "search",
                placeholder: "find a crate…   /",
                autocomplete: "off",
                spellcheck: "false",
                "aria-label": "Find a crate",
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
                            if let Some(hit) = results().get(active().min(n.saturating_sub(1))) {
                                nav.push(Route::Focus { name: hit.name.clone() });
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
                        p { class: "font-data text-[10px] tracking-[0.1em] uppercase text-ink-soft",
                            "no matches"
                        }
                    }
                } else {
                    ul { class: "plate absolute left-0 right-0 top-full z-20 mt-1 max-h-72 overflow-auto py-1",
                        for (i , hit) in results().into_iter().enumerate() {
                            li {
                                Link {
                                    to: Route::Focus { name: hit.name.clone() },
                                    class: if i == active() {
                                        "flex w-full items-center gap-1.5 px-2.5 py-1 bg-ink/5"
                                    } else {
                                        "flex w-full items-center gap-1.5 px-2.5 py-1 hover:bg-ink/5"
                                    },
                                    onclick: move |_| query.set(String::new()),
                                    StarMark { info: hit.clone(), focal: false, box_px: 18.0 }
                                    span { class: "truncate font-data text-[11px] text-ink", "{hit.name}" }
                                    if !hit.is_member {
                                        span { class: "ml-auto font-data text-[9.5px] tracking-[0.12em] text-ink-soft",
                                            "EXT"
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
}

/// What one dependency row says about its edge.
fn kind_words(kind: DepKind) -> Option<&'static str> {
    match kind {
        DepKind::Normal => None,
        DepKind::Dev => Some("DEV"),
        DepKind::Build => Some("BUILD"),
    }
}

fn event_words(event: &DepEvent) -> String {
    match event {
        DepEvent::Added => "ADDED".into(),
        DepEvent::Removed => "REMOVED".into(),
        DepEvent::Bumped(old, new) => format!("{old} → {new}"),
    }
}

/// One row in the focus panel's dependency lists. Live crates are links; a
/// removed crate has no neighborhood left to visit and stays plain text.
#[component]
fn CrateRow(info: CrateInfo, kind: DepKind, event: Option<DepEvent>) -> Element {
    let row = rsx! {
        StarMark { info: info.clone(), focal: false, box_px: 18.0 }
        span { class: "truncate font-data text-[11px] text-ink", "{info.name}" }
        if let Some(k) = kind_words(kind) {
            span { class: "font-data text-[9px] tracking-[0.12em] text-ink-soft", "{k}" }
        }
        if let Some(ev) = &event {
            span { class: "ml-auto shrink-0 font-data text-[9.5px] tracking-[0.1em] text-flare",
                "{event_words(ev)}"
            }
        }
    };
    rsx! {
        li {
            if info.ghost {
                div { class: "flex w-full items-center gap-1.5 px-1 py-0.5", {row} }
            } else {
                Link {
                    to: Route::Focus { name: info.name.clone() },
                    class: "flex w-full items-center gap-1.5 px-1 py-0.5 hover:bg-ink/5",
                    {row}
                }
            }
        }
    }
}

/// A chunked list of dependency rows: the first handful, then a counted
/// reveal — never an unbounded wall.
#[component]
fn CrateList(rows: Vec<(CrateInfo, DepKind, Option<DepEvent>)>) -> Element {
    const CHUNK: usize = 8;
    let mut all = use_signal(|| false);
    let total = rows.len();
    let shown = if all() { total } else { CHUNK.min(total) };

    rsx! {
        ul { class: "mt-1",
            for (info , kind , event) in rows.into_iter().take(shown) {
                CrateRow { info, kind, event }
            }
        }
        if total > shown {
            button {
                class: "mt-1 px-1 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                onclick: move |_| all.set(true),
                "show all {total}"
            }
        }
    }
}

/// The focused crate's fact sheet: the way back, identity, state, and both
/// directions of its neighborhood as clickable lists.
#[component]
pub fn FocusPanel(graph: WorkspaceGraph, name: String) -> Element {
    let atlas = use_atlas();
    let trail = atlas.trail.read();
    let can_go_back = trail.at > 0;
    let back_label = match trail.previous() {
        Some(Some(prev)) => format!("← back · {prev}"),
        Some(None) => "← back · whole chart".to_string(),
        None => String::new(),
    };
    let walked = trail.walked();

    let Some(focal) = graph
        .crates
        .iter()
        .find(|c| c.name == name && !c.ghost)
        .cloned()
    else {
        return rsx! {
            section { class: "plate pointer-events-auto w-72 px-4 py-3",
                p { class: "font-data text-[11px] text-ink", "No crate named “{name}” in this workspace." }
                Link {
                    class: "mt-2 inline-block font-data text-[10px] tracking-[0.12em] uppercase text-ink underline underline-offset-4",
                    to: Route::Overview {},
                    "← whole chart"
                }
            }
        };
    };

    let by_id: std::collections::HashMap<&str, &CrateInfo> =
        graph.crates.iter().map(|c| (c.id.as_str(), c)).collect();
    // All versions of the focal crate participate.
    let focal_ids: Vec<&str> = graph
        .crates
        .iter()
        .filter(|c| c.name == name && !c.ghost)
        .map(|c| c.id.as_str())
        .collect();

    let mut depends_on: Vec<(CrateInfo, DepKind, Option<DepEvent>)> = Vec::new();
    let mut used_by: Vec<(CrateInfo, DepKind, Option<DepEvent>)> = Vec::new();
    for link in &graph.links {
        if focal_ids.contains(&link.from.as_str())
            && let Some(c) = by_id.get(link.to.as_str())
        {
            depends_on.push(((*c).clone(), link.kind, link.event.clone()));
        }
        if focal_ids.contains(&link.to.as_str())
            && let Some(c) = by_id.get(link.from.as_str())
        {
            used_by.push(((*c).clone(), link.kind, link.event.clone()));
        }
    }
    for list in [&mut depends_on, &mut used_by] {
        list.sort_by(|a, b| (a.1 as u8, &a.0.name).cmp(&(b.1 as u8, &b.0.name)));
        list.dedup_by(|a, b| a.0.id == b.0.id && a.1 == b.1);
    }

    let state = if focal.changed {
        Some(format!(
            "CHANGED — {} file{} edited in this epoch",
            focal.changed_files,
            if focal.changed_files == 1 { "" } else { "s" }
        ))
    } else {
        focal.affected_dist.map(|d| {
            format!(
                "AFFECTED — {d} hop{} downstream of a change",
                if d == 1 { "" } else { "s" }
            )
        })
    };

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-full w-full flex-col overflow-hidden sm:w-72",
            div { class: "px-4 pt-3",
                div { class: "flex items-baseline justify-between gap-2",
                    if can_go_back {
                        button {
                            class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                            onclick: move |_| history_back(),
                            "{back_label}"
                        }
                        Link {
                            class: "shrink-0 font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                            to: Route::Overview {},
                            "whole chart"
                        }
                    } else {
                        Link {
                            class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                            to: Route::Overview {},
                            "← whole chart"
                        }
                    }
                }
                if walked.len() > 1 {
                    p { class: "mt-1 truncate font-data text-[9.5px] tracking-[0.08em] text-ink-soft",
                        "trail · {walked.join(\" → \")}"
                    }
                }
                h2 { class: "mt-1.5 break-all font-data text-[15px] font-semibold text-ink",
                    "{focal.name}"
                }
                p { class: "font-data text-[10.5px] text-ink-soft",
                    "v{focal.version} · "
                    if focal.is_member { "workspace member" } else { "external crate" }
                }
                if let Some(state) = state {
                    p { class: "mt-1.5 font-data text-[10px] tracking-[0.08em] text-flare", "{state}" }
                }
                if focal.manifest_changed {
                    p { class: "mt-0.5 font-data text-[10px] tracking-[0.08em] text-flare",
                        "MANIFEST EDITED — its dependency list changed"
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Used by ({used_by.len()})"
                }
                if used_by.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        if focal.changed { "nothing depends on it — the blast radius is this crate alone" }
                        else { "nothing in the resolved graph" }
                    }
                } else {
                    CrateList { rows: used_by }
                }
                h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Depends on ({depends_on.len()})"
                }
                if depends_on.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft", "nothing — a leaf crate" }
                } else {
                    CrateList { rows: depends_on }
                }
            }
        }
    }
}

/// The multi-selection roster: every selected crate, each removable, and
/// what the union is drawing. Refine on the chart with ctrl-click, or here.
#[component]
pub fn MultiPanel(graph: WorkspaceGraph, joined: String) -> Element {
    let names: Vec<String> = joined.split('+').map(str::to_string).collect();
    let sel: HashSet<&str> = names.iter().map(String::as_str).collect();
    let sel_ids: HashSet<&str> = graph
        .crates
        .iter()
        .filter(|c| !c.ghost && sel.contains(c.name.as_str()))
        .map(|c| c.id.as_str())
        .collect();
    let mut deps: HashSet<&str> = HashSet::new();
    let mut users: HashSet<&str> = HashSet::new();
    for link in &graph.links {
        let (from, to) = (link.from.as_str(), link.to.as_str());
        if sel_ids.contains(from) && !sel_ids.contains(to) {
            deps.insert(to);
        }
        if sel_ids.contains(to) && !sel_ids.contains(from) {
            users.insert(from);
        }
    }

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-full w-full flex-col overflow-hidden sm:w-72",
            div { class: "px-4 pt-3",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::Overview {},
                    "← whole chart"
                }
                h2 { class: "mt-1.5 font-chart text-[13px] tracking-[0.22em] uppercase text-ink",
                    "Selection ({names.len()})"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "together they depend on {deps.len()} crates · {users.len()} depend on them"
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                ul { class: "mt-2",
                    for name in names.clone() {
                        li {
                            div { class: "flex w-full items-center gap-1.5 px-1 py-0.5",
                                if let Some(info) = graph.crates.iter().find(|c| c.name == name && !c.ghost) {
                                    StarMark { info: info.clone(), focal: false, box_px: 18.0 }
                                }
                                Link {
                                    to: Route::Focus { name: name.clone() },
                                    class: "truncate font-data text-[11px] text-ink underline-offset-4 hover:underline",
                                    "{name}"
                                }
                                Link {
                                    to: {
                                        let rest: Vec<&String> = names.iter().filter(|n| *n != &name).collect();
                                        if rest.is_empty() {
                                            Route::Overview {}
                                        } else {
                                            Route::Focus {
                                                name: rest
                                                    .iter()
                                                    .map(|s| s.as_str())
                                                    .collect::<Vec<_>>()
                                                    .join("+"),
                                            }
                                        }
                                    },
                                    class: "ml-auto shrink-0 font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                                    aria_label: "remove {name} from the selection",
                                    "remove"
                                }
                            }
                        }
                    }
                }
                p { class: "mt-2 border-t border-ink-line pt-2 font-data text-[10px] leading-relaxed text-ink-soft",
                    "ctrl-click stars on the chart to add or remove them"
                }
            }
        }
    }
}

/// One ring's roster: every crate at that dependency distance. The chart is
/// drawing all of their edges (in the toggled direction).
#[component]
pub fn RingPanel(graph: WorkspaceGraph, hop: u32) -> Element {
    // Same cap the chart uses for this route, so the roster matches the
    // drawn ring exactly — including the collapsed "N+" band.
    let cap = hop.max(DEFAULT_CAP);
    let layout = use_memo({
        let graph = graph.clone();
        move || radial_layout(&graph, cap)
    });
    let collapsed = hop == cap && layout.read().max_hops > cap;
    let mut all = use_signal(|| false);

    let mut crates: Vec<CrateInfo> = {
        let layout = layout.read();
        graph
            .crates
            .iter()
            .filter(|c| !c.ghost && layout.placed.get(&c.id).is_some_and(|p| p.ring == hop))
            .cloned()
            .collect()
    };
    crates.sort_by(|a, b| {
        (std::cmp::Reverse(a.dependents), a.name.as_str())
            .cmp(&(std::cmp::Reverse(b.dependents), b.name.as_str()))
    });
    let total = crates.len();
    const CHUNK: usize = 14;
    let shown = if all() { total } else { CHUNK.min(total) };

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-full w-full flex-col overflow-hidden sm:w-72",
            div { class: "px-4 pt-3",
                Link {
                    class: "font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to: Route::Overview {},
                    "← whole chart"
                }
                h2 { class: "mt-1.5 font-chart text-[13px] tracking-[0.22em] uppercase text-ink",
                    if collapsed { "Ring {hop}+ ({total})" } else { "Ring {hop} ({total})" }
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    if collapsed { "every crate {hop} or more hops from the center" }
                    else if hop == 1 { "every crate 1 hop from the center" }
                    else { "every crate {hop} hops from the center" }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                ul { class: "mt-2",
                    for info in crates.into_iter().take(shown) {
                        li {
                            Link {
                                to: Route::Focus { name: info.name.clone() },
                                class: "flex w-full items-center gap-1.5 px-1 py-0.5 hover:bg-ink/5",
                                StarMark { info: info.clone(), focal: false, box_px: 18.0 }
                                span { class: "truncate font-data text-[11px] text-ink", "{info.name}" }
                                if !info.is_member {
                                    span { class: "ml-auto shrink-0 font-data text-[9.5px] tracking-[0.12em] text-ink-soft",
                                        "EXT"
                                    }
                                }
                            }
                        }
                    }
                }
                if total > shown {
                    button {
                        class: "mt-1 px-1 font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                        onclick: move |_| all.set(true),
                        "show all {total}"
                    }
                }
                p { class: "mt-2 border-t border-ink-line pt-2 font-data text-[10px] leading-relaxed text-ink-soft",
                    "ctrl-click a star to carve a smaller selection out of the ring"
                }
            }
        }
    }
}
