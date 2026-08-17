//! The record: what the pane cannot say.
//!
//! A picture shows a neighbourhood; it does not *state* anything. `cargo tree -i`
//! answers "why is this here" in 400ms of copyable text, and a drawing that has
//! to be traced by eye has not answered it at all — so holding a crate produces
//! words too: names you can read, counts that add up, and a chain you can paste
//! into a pull request.
//!
//! Every record carries the same fields in the same order: what it is, why it is
//! here, what depends on it, what it depends on. Learn one and you can read them
//! all without looking for where anything moved to.

use dioxus::prelude::*;

use crate::api::GraphLoad;
use crate::graph::{Workspace, focus};
use crate::views::deps::DepsState;
use crate::views::{GraphResource, Panel};

#[derive(Clone, PartialEq)]
pub struct Row {
    pub id: usize,
    pub name: String,
    pub version: String,
    pub dependents: usize,
    pub duplicate: bool,
}

/// Which way a list runs. Direction is the graph's strongest signal, so the
/// record spends the same two colours on it that the edges do.
#[derive(Clone, Copy, PartialEq)]
pub enum Way {
    /// Rust: crates that pull this one in. Enters from the left.
    In,
    /// Blue: crates this one pulls in. Leaves to the right.
    Out,
}

#[derive(Clone, PartialEq)]
pub struct Record {
    pub id: usize,
    pub name: String,
    pub version: String,
    pub duplicate: bool,
    pub own: bool,
    /// Workspace member → … → this crate. Empty only if unreachable.
    pub path: Vec<Row>,
    pub direct_dependents: Vec<Row>,
    pub direct_dependencies: Vec<Row>,
    /// Every crate reachable in that direction, at any distance. The listed rows
    /// plus `total - listed` account for all of it and nothing twice.
    pub total_dependents: usize,
    pub total_dependencies: usize,
    /// Where to read more. Both are optional and for different reasons: a crate
    /// resolved from a path or a git checkout has no crates.io page, and plenty
    /// of crates never declare a repository at all.
    pub crates_io: Option<String>,
    pub repository: Option<String>,
}

fn row(workspace: &Workspace, id: usize) -> Row {
    let entry = &workspace.crates[id];
    Row {
        id,
        name: entry.name.clone(),
        version: entry.version.clone(),
        dependents: entry.dependents.len(),
        duplicate: entry.duplicate,
    }
}

impl Record {
    pub fn build(workspace: &Workspace, view: &focus::Reach, id: usize) -> Self {
        let entry = &workspace.crates[id];
        let (dependents, dependencies) = focus::immediate(workspace, id);

        // "Direct" comes from the crate itself, never from hop levels: levels are
        // longest-path, so a crate that is both an immediate dependent and
        // reachable by a longer route sits in a further column. The reader asked
        // for immediate, and immediate is exact.
        Self {
            id,
            name: entry.name.clone(),
            version: entry.version.clone(),
            duplicate: entry.duplicate,
            own: entry.is_root,
            path: focus::shortest_path_from_root(workspace, id)
                .into_iter()
                .map(|step| row(workspace, step))
                .collect(),
            direct_dependents: dependents.iter().map(|&i| row(workspace, i)).collect(),
            direct_dependencies: dependencies.iter().map(|&i| row(workspace, i)).collect(),
            total_dependents: view.total_dependents,
            total_dependencies: view.total_dependencies,
            // Pinned to the resolved version rather than the crate's landing
            // page: the reader is looking at what this build actually pulled in,
            // and "latest" is a different crate as often as not.
            crates_io: entry.registry.then(|| {
                format!("https://crates.io/crates/{}/{}", entry.name, entry.version)
            }),
            repository: entry.repository.clone(),
        }
    }
}

#[component]
pub fn Inspector(record: Option<Record>) -> Element {
    let resource: GraphResource = use_context();
    let mut state: DepsState = use_context();

    let mut open = move |id: usize| {
        let loaded = resource.read();
        if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
            state.select(workspace, id);
        }
    };

    let Some(record) = record else {
        return rsx! {
            Panel { label: "Key".to_string(), KeyPanel {} }
        };
    };

    let route = record.path.clone();
    let target = record.id;

    rsx! {
        Panel { label: "Crate".to_string(),

            // Screen readers get the answer too: the pane can say nothing.
            div {
                class: "sr-only",
                "aria-live": "polite",
                "{record.name} {record.version}. {record.total_dependents} crates depend on it. It depends on {record.total_dependencies}."
            }

            header { class: "border-b border-line px-4 py-3.5",
                div { class: "flex items-start justify-between gap-2",
                    h2 { class: "min-w-0 flex-1 truncate text-[17px] font-semibold tracking-[-0.01em]",
                        "{record.name}"
                    }
                    button {
                        class: "-mr-1 -mt-0.5 shrink-0 rounded p-1 text-ink-faint hover:bg-sunken hover:text-ink",
                        title: "Let go (Esc)",
                        "aria-label": "Let go",
                        onclick: move |_| state.held.set(None),
                        svg {
                            class: "h-3.5 w-3.5",
                            view_box: "0 0 16 16",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.5",
                            stroke_linecap: "round",
                            path { d: "M4.5 4.5 L11.5 11.5" }
                            path { d: "M11.5 4.5 L4.5 11.5" }
                        }
                    }
                }
                div { class: "mt-1 flex flex-wrap items-center gap-1.5",
                    span { class: "tabular font-mono text-[11.5px] text-ink-muted", "{record.version}" }
                    if record.own {
                        span { class: "chip bg-ink text-ink-invert", "WORKSPACE" }
                    }
                    if record.duplicate {
                        span {
                            class: "chip bg-flag-field text-flag",
                            title: "This crate name resolves at more than one version in this workspace",
                            "ALSO AT ANOTHER VERSION"
                        }
                    }
                }
                // Where to read the thing itself. These leave the app, so they
                // say so and open away from it rather than replacing the graph
                // the reader has been building.
                if record.crates_io.is_some() || record.repository.is_some() {
                    div { class: "mt-2 flex flex-wrap items-center gap-1.5",
                        if let Some(url) = record.crates_io.clone() {
                            a {
                                class: "link-out",
                                href: "{url}",
                                target: "_blank",
                                rel: "noreferrer noopener",
                                title: "{record.name} {record.version} on crates.io",
                                "crates.io"
                                Outward {}
                            }
                        }
                        if let Some(url) = record.repository.clone() {
                            a {
                                class: "link-out",
                                href: "{url}",
                                target: "_blank",
                                rel: "noreferrer noopener",
                                title: "{url}",
                                "Repository"
                                Outward {}
                            }
                        }
                    }
                }
            }

            // The answer to "why is this here", as copyable text — and as a
            // route the pane will draw on request.
            if !record.path.is_empty() {
                section { class: "border-b border-line px-4 py-3.5",
                    div { class: "flex items-baseline justify-between gap-2",
                        h3 { class: "label", "Why it's here" }
                        if record.path.len() > 1 {
                            button {
                                class: "action",
                                title: "Put every crate on this chain on the pane",
                                onclick: move |_| {
                                    let loaded = resource.read();
                                    if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
                                        state.route_to(workspace, target);
                                    }
                                },
                                "Draw this route"
                            }
                        }
                    }
                    p { class: "mt-1.5 font-mono text-[11.5px] leading-relaxed break-words select-text",
                        for (index , step) in route.iter().enumerate() {
                            span { key: "{step.id}",
                                if index > 0 {
                                    span { class: "text-ink-faint", " → " }
                                }
                                button {
                                    class: "rounded-[3px] px-0.5 hover:bg-sunken",
                                    class: if index + 1 == route.len() { "font-semibold text-inbound" } else { "text-ink-muted hover:text-ink" },
                                    onclick: {
                                        let id = step.id;
                                        move |_| open(id)
                                    },
                                    "{step.name}"
                                }
                            }
                        }
                    }
                }
            }

            CrateList {
                heading: "Depended on by".to_string(),
                empty: "Nothing depends on this crate.".to_string(),
                way: Way::In,
                total: record.total_dependents,
                rows: record.direct_dependents.clone(),
            }

            CrateList {
                heading: "Depends on".to_string(),
                empty: "This crate depends on nothing.".to_string(),
                way: Way::Out,
                total: record.total_dependencies,
                rows: record.direct_dependencies.clone(),
            }
        }
    }
}

/// How many rows a list shows before the reader asks for the rest.
///
/// Both directions are first class, and a crate with 27 direct dependents pushes
/// "Depends on" two screens down — which is exactly how a first-class direction
/// comes to read as a footnote.
const ROWS: usize = 7;

#[component]
fn CrateList(heading: String, empty: String, way: Way, total: usize, rows: Vec<Row>) -> Element {
    let resource: GraphResource = use_context();
    let mut state: DepsState = use_context();
    let mut all = use_signal(|| false);

    // The same two colours the edges use, so the record and the pane are
    // obviously reporting the same fact.
    let (ink, rule) = match way {
        Way::In => ("text-inbound", "bg-inbound"),
        Way::Out => ("text-outbound", "bg-outbound"),
    };
    let folded = rows.len().saturating_sub(ROWS);
    let shown: Vec<Row> = if all() || folded == 0 {
        rows.clone()
    } else {
        rows.iter().take(ROWS).cloned().collect()
    };

    rsx! {
        section { class: "border-b border-line px-4 py-3.5",
            h3 { class: "flex items-center justify-between gap-2",
                span { class: "flex items-center gap-2",
                    span { class: "h-[2px] w-4 shrink-0 rounded-full {rule}" }
                    span { class: "label", "{heading}" }
                }
                span { class: "tabular text-[15px] font-semibold {ink}", "{total}" }
            }

            if rows.is_empty() && total == 0 {
                p { class: "mt-1.5 text-[12px] text-ink-muted", "{empty}" }
            } else {
                // Two buckets that add up to the total, every time.
                p { class: "tabular mt-0.5 text-[11px] text-ink-faint",
                    "{rows.len()} directly"
                    if total > rows.len() {
                        " · {total - rows.len()} further out"
                    }
                }
                ul { class: "mt-2 -mx-1.5 flex flex-col",
                    for entry in shown {
                        li { key: "{entry.id}",
                            button {
                                class: "flex w-full items-baseline gap-2 rounded-[5px] px-1.5 py-[5px] text-left hover:bg-sunken",
                                onclick: {
                                    let id = entry.id;
                                    move |_| {
                                        let loaded = resource.read();
                                        if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
                                            state.select(workspace, id);
                                        }
                                    }
                                },
                                span { class: "min-w-0 flex-1 truncate text-[13px]", "{entry.name}" }
                                if entry.duplicate {
                                    span {
                                        class: "chip shrink-0 bg-flag-field text-flag",
                                        title: "Resolves at more than one version",
                                        "DUP"
                                    }
                                }
                                span { class: "tabular shrink-0 font-mono text-[11px] text-ink-faint",
                                    "{entry.version}"
                                }
                                span {
                                    class: "tabular w-7 shrink-0 text-right text-[11px] text-ink-faint",
                                    title: "{entry.dependents} crates depend on {entry.name}",
                                    "{entry.dependents}"
                                }
                            }
                        }
                    }
                }
                if folded > 0 {
                    button {
                        class: "action mt-1",
                        onclick: move |_| all.set(!all()),
                        if all() { "Show fewer" } else { "Show all {rows.len()}" }
                    }
                }
            }
        }
    }
}

/// What the marks mean and how to move, in the graph's own key. A tool that
/// invents a vocabulary owes the reader its definition.
#[component]
fn KeyPanel() -> Element {
    rsx! {
        div { class: "flex flex-col gap-6 px-4 py-4",
            section {
                h3 { class: "label", "The graph" }
                p { class: "mt-1.5 text-[13px] leading-relaxed text-ink-muted",
                    "Cards are crates, wires are dependencies. Everything to the right of a card is something it depends on; everything to the left depends on it. Columns count hops from your workspace."
                }
            }

            section {
                h3 { class: "label", "Marks" }
                ul { class: "mt-2.5 flex flex-col gap-2.5 text-[12px]",
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-4 w-8 shrink-0 rounded-[4px] border border-ink bg-ink" }
                        span { "a crate this workspace builds" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-4 w-8 shrink-0 rounded-[4px] border border-line-strong bg-surface" }
                        span { "a crate it pulls in" }
                    }
                    // Drawn exactly as a port at rest is drawn, because a key
                    // whose marks do not appear on the pane in that form is a
                    // key to a different picture.
                    li { class: "flex items-center gap-2.5",
                        span { class: "port-sample", "7" }
                        span { "on the left: how many crates depend on this one" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "port-sample", "3" }
                        span { "on the right: how many it depends on — click either to open or fold it" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-[2px] w-8 shrink-0 rounded-full bg-inbound" }
                        span { "what depends on the crate you hold" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "h-[2px] w-8 shrink-0 rounded-full bg-outbound" }
                        span { "what it depends on" }
                    }
                    li { class: "flex items-center gap-2.5",
                        span { class: "flex w-8 shrink-0 justify-center",
                            span { class: "chip bg-flag-field text-flag", "DUP" }
                        }
                        span { "resolves at more than one version" }
                    }
                }
            }

            Duplicates {}

            section {
                h3 { class: "label", "Moving" }
                dl { class: "mt-2.5 grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 text-[12px]",
                    Key { keys: "click a port".to_string(), note: "open that side, or fold it away".to_string() }
                    Key { keys: "click a card".to_string(), note: "hold it; the view flies there".to_string() }
                    Key { keys: "scroll · drag".to_string(), note: "pan".to_string() }
                    Key { keys: "wheel · pinch".to_string(), note: "zoom about the pointer".to_string() }
                    Key { keys: "← ↑".to_string(), note: "step to what depends on it".to_string() }
                    Key { keys: "→ ↓".to_string(), note: "step to what it depends on".to_string() }
                    Key { keys: "⌫".to_string(), note: "back the way you came".to_string() }
                    Key { keys: "/".to_string(), note: "find a crate by name".to_string() }
                    Key { keys: "esc".to_string(), note: "let go".to_string() }
                }
            }
        }
    }
}

/// The crate names this workspace resolves at more than one version, which the
/// top bar counts and nothing else named. One of the three jobs the product
/// carries is auditing exactly this.
#[component]
fn Duplicates() -> Element {
    let resource: GraphResource = use_context();
    let mut state: DepsState = use_context();
    let mut all = use_signal(|| false);

    let groups: Vec<(usize, String, String)> = {
        let loaded = resource.read();
        let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
            return rsx! {};
        };
        workspace
            .duplicates
            .iter()
            .map(|group| {
                (
                    group.ids[0],
                    group.name.clone(),
                    group.versions.join(" · "),
                )
            })
            .collect()
    };
    if groups.is_empty() {
        return rsx! {};
    }
    let folded = groups.len().saturating_sub(ROWS);
    let shown: Vec<(usize, String, String)> = if all() || folded == 0 {
        groups.clone()
    } else {
        groups.iter().take(ROWS).cloned().collect()
    };

    rsx! {
        section {
            h3 { class: "flex items-baseline justify-between gap-2",
                span { class: "label", "At more than one version" }
                span { class: "tabular text-[15px] font-semibold text-flag", "{groups.len()}" }
            }
            p { class: "mt-1 text-[11px] leading-relaxed text-ink-faint",
                "Each of these names resolves twice or more in this build. Open one to see which crates pull which version."
            }
            ul { class: "mt-2 -mx-1.5 flex flex-col",
                for (id , name , versions) in shown {
                    li { key: "{id}",
                        button {
                            class: "flex w-full items-baseline gap-2 rounded-[5px] px-1.5 py-[5px] text-left hover:bg-sunken",
                            onclick: move |_| {
                                let loaded = resource.read();
                                if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
                                    state.select(workspace.as_ref(), id);
                                }
                            },
                            span { class: "min-w-0 flex-1 truncate text-[13px]", "{name}" }
                            span { class: "tabular shrink-0 truncate font-mono text-[10.5px] text-ink-faint",
                                "{versions}"
                            }
                        }
                    }
                }
            }
            if folded > 0 {
                button {
                    class: "action mt-1",
                    onclick: move |_| all.set(!all()),
                    if all() { "Show fewer" } else { "Show all {groups.len()}" }
                }
            }
        }
    }
}

#[component]
fn Key(keys: String, note: String) -> Element {
    rsx! {
        dt { class: "whitespace-nowrap rounded-[5px] border border-line bg-sunken px-1.5 py-0.5 text-center font-mono text-[10.5px] text-ink-muted",
            "{keys}"
        }
        dd { class: "self-center text-ink-muted", "{note}" }
    }
}

/// The mark on a link that leaves the app. Small, and the same one every time:
/// a reader should be able to tell before clicking whether they are staying.
#[component]
fn Outward() -> Element {
    rsx! {
        svg {
            class: "size-[9px] shrink-0 opacity-60",
            view_box: "0 0 10 10",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.4",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "M3.5 1.5 H8.5 V6.5" }
            path { d: "M8.5 1.5 L1.5 8.5" }
        }
    }
}
