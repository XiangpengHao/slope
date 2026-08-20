//! Chart furniture: the title block, the key, search, and the selection
//! panels. All of it is drawn in the same engraved ink as the chart itself.

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::Route;
use crate::api::{CrateInfo, DepEvent, DepKind, WorkspaceGraph};
use crate::views::radial::{DEFAULT_CAP, radial_layout};
use crate::views::shell::{DirFilter, step_ring, use_atlas};
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
        description: None,
        license: None,
        repository: None,
        homepage: None,
        documentation: None,
        crates_io: false,
        rel_path: None,
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The title block and the review agenda in one plate: what workspace this
/// is, which epoch it is charted against, and every crate that changed in
/// it. One plate, because the epoch and its changes are one thought.
#[component]
pub fn TitleBlock(graph: WorkspaceGraph, #[props(default = true)] changes_open: bool) -> Element {
    let atlas = use_atlas();
    let members = graph.crates.iter().filter(|c| c.is_member).count();
    let externals = graph
        .crates
        .iter()
        .filter(|c| !c.is_member && !c.ghost)
        .count();
    let affected = graph
        .crates
        .iter()
        .filter(|c| !c.changed && c.affected_dist.is_some())
        .count();
    let epoch = &graph.epoch;

    let mut changed: Vec<CrateInfo> = graph.crates.iter().filter(|c| c.changed).cloned().collect();
    changed.sort_by(|a, b| a.name.cmp(&b.name));
    let visited = atlas.visited.read();
    let seen = changed.iter().filter(|c| visited.contains(&c.name)).count();
    let total = changed.len();
    let focus = atlas.trail.read().current_focus();

    rsx! {
        section { class: "plate pointer-events-auto w-full",
            div { class: "px-4 pt-3",
                h1 { class: "font-chart text-[19px] leading-tight tracking-[0.18em] uppercase text-ink",
                    "{graph.name}"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "{plural(members, \"workspace crate\")} · {externals} external"
                }
                p { class: "mt-2 border-t border-ink-line pt-2 font-data text-[10.5px] leading-relaxed text-ink",
                    span { class: "text-ink-soft", "diff " }
                    "{epoch.base} → {epoch.target}"
                }
                div { class: "pb-2 pt-1",
                    crate::views::codemap::chrome::AltitudeSwitch {
                        at: crate::views::codemap::chrome::Altitude::Deps,
                    }
                }
            }
            details { class: "fold border-t border-ink-line open:pb-3", open: changes_open,
                summary { class: "cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                    "Changes ({total})"
                }
                if let Some(note) = &epoch.note {
                    p { class: "px-4 font-data text-[10px] leading-relaxed text-ink-soft", "{note}" }
                } else if total == 0 {
                    p { class: "px-4 font-data text-[10px] leading-relaxed text-ink",
                        "the working copy matches {epoch.base} — the chart shows the whole workspace"
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
                                    span { class: "shrink-0 font-data text-[9px] text-flare",
                                        "{plural(info.changed_files as usize, \"file\")}"
                                    }
                                    if info.manifest_changed {
                                        span { class: "shrink-0 font-data text-[9px] text-flare",
                                            "Cargo.toml"
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
                    p { class: "mx-4 mt-2 border-t border-ink-line pt-2 font-data text-[9.5px] tracking-[0.1em] uppercase text-ink",
                        if seen == total { "all {total} seen" } else { "{seen} of {total} seen" }
                        span { class: "text-ink-soft", " · {affected} downstream" }
                    }
                }
            }
        }
    }
}

/// Which of the selection's edges the chart draws. It rides inside the
/// selection's own panel, because it has nothing to act on without one.
/// Active segment wears a 1px ink border — no fills on this plate, ever.
#[component]
fn DirectionToggle() -> Element {
    let atlas = use_atlas();
    let current = *atlas.dir.read();
    let seg = |label: &'static str, hint: &'static str, val: DirFilter| {
        rsx! {
            button {
                class: "flex-1 whitespace-nowrap border px-1 py-0.5 font-data text-[9px] tracking-[0.08em] uppercase",
                class: if current == val { "border-ink text-ink" } else { "border-transparent text-ink-soft hover:text-ink" },
                "aria-pressed": if current == val { "true" } else { "false" },
                title: hint,
                onclick: move |_| {
                    let mut dir = atlas.dir;
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
            "aria-label": "which of the selection's edges the chart draws",
            span { class: "block font-data text-[9px] tracking-[0.1em] uppercase text-ink-soft",
                "dependencies"
            }
            div { class: "mt-1 flex items-stretch gap-0.5",
                {seg("depends on", "what the selection depends on", DirFilter::Deps)}
                {seg("used by", "what depends on the selection, one hop out", DirFilter::Users)}
                {
                    seg(
                        "reverse deps",
                        "what depends on the selection, and what depends on those, all the way to the root — cargo tree -i",
                        DirFilter::PathToRoot,
                    )
                }
            }
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
        details {
            class: "plate fold pointer-events-auto w-full open:pb-3 sm:flex sm:max-h-full sm:w-64 sm:flex-col",
            open: start_open,
            summary {
                class: "shrink-0 cursor-pointer select-none px-4 py-2 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                "Reading this chart"
            }
            div { class: "max-h-[60dvh] space-y-2.5 overflow-y-auto px-4 font-data text-[10px] leading-snug text-ink sm:max-h-none sm:min-h-0",
                div { class: "flex items-center gap-2",
                    RingsSample {}
                    span {
                        span { class: "font-medium", "{center}" }
                        " sits at the center; each ring outward is one dependency hop — the outermost gathers everything farther, and expands as you select into it"
                    }
                }
                div { class: "space-y-1.5 border-t border-ink-line pt-2.5",
                    div { class: "flex items-baseline gap-2",
                        span { class: "shrink-0 font-data text-[9.5px] tracking-[0.1em] uppercase text-ink",
                            "ctrl-click"
                        }
                        span { class: "text-ink-soft", "add or remove a star from the selection" }
                    }
                    div { class: "flex flex-wrap items-baseline gap-x-3 gap-y-1",
                        for (key , what) in [
                            ("/", "find"),
                            ("n p", "walk changes"),
                            ("f", "refit"),
                            ("esc", "deselect"),
                            ("← →", "back · forward"),
                        ]
                        {
                            span { key: "{key}", class: "whitespace-nowrap",
                                span { class: "font-medium uppercase tracking-[0.1em] text-ink", "{key}" }
                                span { class: "text-ink-soft", " {what}" }
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-2 border-t border-ink-line pt-2.5",
                    div { class: "flex items-end",
                        StarMark { info: sample(0, true), focal: false, box_px: 18.0 }
                        StarMark { info: sample(9, true), focal: false, box_px: 22.0 }
                        StarMark { info: sample(60, true), focal: false, box_px: 28.0 }
                    }
                    span { "size — how many crates depend on it" }
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
                            span { class: "font-medium text-flare", "changed" }
                            " — files under it changed since the diff base"
                        }
                    }
                    div { class: "flex items-center gap-2",
                        StarMark { info: affected, focal: false, box_px: 26.0 }
                        span {
                            span { class: "font-medium", "downstream" }
                            " — it depends on a changed crate; the halo fades with distance"
                        }
                    }
                    div { class: "flex items-center gap-2",
                        StarMark { info: ghost, focal: false, box_px: 20.0 }
                        span {
                            span { class: "font-medium", "removed" }
                            " — a dependency the diff deleted"
                        }
                    }
                }
                div { class: "border-t border-ink-line pt-2.5 space-y-1.5",
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "", stroke: "var(--color-ink)", width: 1.25 }
                        span { "ink: the selected crate depends on it" }
                    }
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "", stroke: "var(--color-ink-line)", width: 1.1 }
                        span { "hairline: it depends on the selected crate" }
                    }
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "6 4", stroke: "var(--color-ink-line)" }
                        span { "dev-dependencies — dashed · build-dependencies — dotted" }
                    }
                    div { class: "flex items-center gap-2",
                        LineSample { dasharray: "", stroke: "var(--color-flare)", width: 1.4 }
                        span { "manifest event — added, removed, or bumped" }
                    }
                    p { class: "pt-1 text-ink-soft",
                        "only the selection's edges are drawn; arrows point the way change travels"
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
                                        span { class: "ml-auto shrink-0 font-data text-[9.5px] text-ink-soft",
                                            "v{hit.version}"
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

/// Names that appear more than once in a list, because cargo resolved
/// several versions of the same crate. Rows and stars for these have to carry
/// their version; everything else stays a bare name.
fn repeated_names<'a>(crates: impl Iterator<Item = &'a CrateInfo>) -> HashSet<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut twice: HashSet<String> = HashSet::new();
    for c in crates {
        if !seen.insert(c.name.as_str()) {
            twice.insert(c.name.clone());
        }
    }
    twice
}

/// Which cargo table an edge comes from, spelled the way `cargo tree` spells
/// it. A normal dependency needs no tag: it is the default table.
fn kind_words(kind: DepKind) -> Option<&'static str> {
    match kind {
        DepKind::Normal => None,
        DepKind::Dev => Some("(dev)"),
        DepKind::Build => Some("(build)"),
    }
}

fn event_words(event: &DepEvent) -> String {
    match event {
        DepEvent::Added => "added".into(),
        DepEvent::Removed => "removed".into(),
        DepEvent::Bumped(old, new) => format!("{old} → {new}"),
    }
}

/// One row in the focus panel's dependency lists. Live crates are links; a
/// removed crate has no neighborhood left to visit and stays plain text.
#[component]
fn CrateRow(
    info: CrateInfo,
    kind: DepKind,
    event: Option<DepEvent>,
    /// The list holds another version of this crate, so the row has to say
    /// which one it is.
    #[props(default = false)]
    versioned: bool,
) -> Element {
    let row = rsx! {
        StarMark { info: info.clone(), focal: false, box_px: 18.0 }
        span { class: "truncate font-data text-[11px] text-ink",
            "{info.name}"
            if versioned {
                span { class: "text-ink-line", " v{info.version}" }
            }
        }
        if let Some(k) = kind_words(kind) {
            span { class: "shrink-0 font-data text-[9.5px] text-ink-soft", "{k}" }
        }
        if let Some(ev) = &event {
            span { class: "ml-auto shrink-0 font-data text-[9.5px] text-flare",
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
    let repeated = repeated_names(rows.iter().map(|(c, _, _)| c));

    rsx! {
        ul { class: "mt-1",
            for (info , kind , event) in rows.into_iter().take(shown) {
                CrateRow {
                    versioned: repeated.contains(info.name.as_str()),
                    info,
                    kind,
                    event,
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
    }
}

/// Where a crate lives off the plate. A crate resolved from crates.io always
/// has a registry page and a docs.rs build, even when its manifest names
/// neither; everything else is only as good as what the manifest declared.
fn out_links(info: &CrateInfo) -> Vec<(&'static str, String)> {
    let url = |s: &Option<String>| {
        s.as_deref()
            .filter(|u| u.starts_with("http"))
            .map(str::to_string)
    };
    let mut links: Vec<(&'static str, String)> = Vec::new();
    if let Some(repo) = url(&info.repository) {
        links.push(("repo", repo));
    }
    if info.crates_io {
        links.push((
            "crates.io",
            format!("https://crates.io/crates/{}", info.name),
        ));
    }
    match url(&info.documentation) {
        Some(docs) => links.push(("docs", docs)),
        None if info.crates_io => links.push((
            "docs.rs",
            format!("https://docs.rs/{}/{}", info.name, info.version),
        )),
        None => {}
    }
    if let Some(home) = url(&info.homepage)
        && !links.iter().any(|(_, u)| *u == home)
    {
        links.push(("homepage", home));
    }
    links
}

/// One labelled fact in a crate's fact sheet.
#[component]
fn FactRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "w-[84px] shrink-0 font-data text-[9.5px] text-ink-soft",
                "{label}"
            }
            span { class: "min-w-0 break-words font-data text-[10px] text-ink", "{value}" }
        }
    }
}

/// A link off the plate, opened in its own tab so the review never loses its
/// place. The arrow says the reader is leaving.
#[component]
fn OutLink(label: &'static str, href: String) -> Element {
    rsx! {
        a {
            href: "{href}",
            target: "_blank",
            rel: "noreferrer",
            class: "font-data text-[10px] tracking-[0.06em] text-ink underline decoration-ink-line underline-offset-4 hover:decoration-ink",
            title: "{href}",
            "{label} ↗"
        }
    }
}

/// What the manifest says about one crate: its own words, its license, where
/// it sits on disk, and every page it has elsewhere.
#[component]
fn CrateFacts(info: CrateInfo) -> Element {
    let links = out_links(&info);
    if info.description.is_none()
        && info.license.is_none()
        && info.rel_path.is_none()
        && links.is_empty()
    {
        return rsx! {};
    }
    rsx! {
        div { class: "mt-3 space-y-1.5 border-b border-ink-line pb-3",
            if let Some(desc) = info.description.clone() {
                p { class: "font-chart text-[12px] italic leading-snug text-ink", "{desc}" }
            }
            div { class: "space-y-0.5",
                if let Some(license) = info.license.clone() {
                    FactRow { label: "license", value: license }
                }
                if let Some(path) = info.rel_path.clone() {
                    FactRow { label: "path", value: path }
                }
                FactRow {
                    label: "dependencies",
                    value: "{info.direct_deps} direct · {info.external_deps} external",
                }
            }
            if !links.is_empty() {
                div { class: "flex flex-wrap gap-x-3 gap-y-1",
                    for (label , href) in links {
                        OutLink { key: "{label}", label, href }
                    }
                }
            }
        }
    }
}

/// How one trail step reads in the breadcrumb: a ring, a lone crate, or a
/// multi-selection too wide to spell out.
fn step_label(step: &str) -> String {
    if let Some(hop) = step_ring(step) {
        return format!("ring {hop}");
    }
    match step.split('+').count() {
        1 => step.to_string(),
        n => format!("{n} crates"),
    }
}

fn step_route(step: &str) -> Route {
    match step_ring(step) {
        Some(hop) => Route::RingSel { hop },
        None => Route::Focus {
            name: step.to_string(),
        },
    }
}

/// The review trail as one line: the whole chart, then every step behind the
/// current one, each a link back to it. The panel's own heading names where
/// the review stands now, so the trail never repeats it.
#[component]
fn Breadcrumb() -> Element {
    let atlas = use_atlas();
    let mut walked = atlas.trail.read().walked();
    walked.pop();

    rsx! {
        nav {
            class: "flex flex-wrap items-baseline gap-x-1.5 font-data text-[10px] tracking-[0.12em] uppercase text-ink-soft",
            "aria-label": "review trail",
            Link {
                class: "underline-offset-4 hover:text-ink hover:underline",
                to: Route::Overview {},
                "← whole chart"
            }
            for step in walked {
                span { key: "{step}", class: "flex items-baseline gap-x-1.5",
                    span { class: "text-ink-line", "→" }
                    Link {
                        class: "max-w-36 truncate underline-offset-4 hover:text-ink hover:underline",
                        to: step_route(&step),
                        "{step_label(&step)}"
                    }
                }
            }
        }
    }
}

/// The focused crate's fact sheet: the trail, identity, state, and both
/// directions of its neighborhood as clickable lists.
#[component]
pub fn FocusPanel(graph: WorkspaceGraph, name: String) -> Element {
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
    // Cargo can resolve several versions of one crate at once; each is its
    // own star on its own ring, and the selection holds all of them.
    let mut versions: Vec<&str> = graph
        .crates
        .iter()
        .filter(|c| c.name == name && !c.ghost)
        .map(|c| c.version.as_str())
        .collect();
    versions.sort_unstable();
    versions.dedup();
    let version_line = versions
        .iter()
        .map(|v| format!("v{v}"))
        .collect::<Vec<_>>()
        .join(" · ");

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
            "{} changed",
            plural(focal.changed_files as usize, "file")
        ))
    } else {
        focal
            .affected_dist
            .map(|d| format!("{} downstream of a change", plural(d as usize, "hop")))
    };

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3 pb-2",
                Breadcrumb {}
                h2 { class: "mt-1.5 break-all font-data text-[15px] font-semibold text-ink",
                    "{focal.name}"
                }
                p { class: "font-data text-[10.5px] text-ink-soft",
                    "{version_line} · "
                    if focal.is_member { "workspace member" } else { "external crate" }
                }
                if focal.is_member {
                    Link {
                        class: "mt-1 inline-block font-data text-[9.5px] tracking-[0.12em] uppercase text-ink-soft underline underline-offset-4 hover:text-ink",
                        to: Route::CodeCrate { name: focal.name.clone() },
                        "its files ↓"
                    }
                }
                if versions.len() > 1 {
                    p { class: "mt-0.5 font-data text-[10px] leading-snug text-ink-soft",
                        "cargo resolved {versions.len()} versions — one star each, all selected"
                    }
                }
                if let Some(state) = state {
                    p { class: "mt-1.5 font-data text-[10px] text-flare", "{state}" }
                }
                if focal.manifest_changed {
                    p { class: "mt-0.5 font-data text-[10px] text-flare",
                        "Cargo.toml changed — its dependency list"
                    }
                }
            }
            DirectionToggle {}
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                CrateFacts { info: focal.clone() }
                h3 { class: "mt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Used by ({used_by.len()})"
                }
                if used_by.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft",
                        if focal.changed { "nothing depends on it — the change stops here" }
                        else { "nothing in the resolved graph" }
                    }
                } else {
                    CrateList { rows: used_by }
                }
                h3 { class: "mt-3 border-t border-ink-line pt-3 font-chart text-[11px] tracking-[0.22em] uppercase text-ink",
                    "Depends on ({depends_on.len()})"
                }
                if depends_on.is_empty() {
                    p { class: "mt-1 font-data text-[10px] text-ink-soft", "nothing — it has no dependencies" }
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
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3 pb-2",
                Breadcrumb {}
                h2 { class: "mt-1.5 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                    "Selection ({names.len()})"
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    "together they depend on {deps.len()} crates · {users.len()} depend on them"
                }
            }
            DirectionToggle {}
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
    let repeated = repeated_names(crates.iter());

    rsx! {
        section { class: "plate pointer-events-auto flex max-h-[44dvh] w-full flex-col overflow-hidden sm:max-h-full sm:w-72",
            div { class: "px-4 pt-3 pb-2",
                Breadcrumb {}
                h2 { class: "mt-1.5 font-chart text-[12px] tracking-[0.22em] uppercase text-ink",
                    if collapsed { "Ring {hop}+ ({total})" } else { "Ring {hop} ({total})" }
                }
                p { class: "mt-1 font-data text-[10.5px] text-ink-soft",
                    if collapsed { "every crate {hop} or more hops from the center" }
                    else if hop == 1 { "every crate 1 hop from the center" }
                    else { "every crate {hop} hops from the center" }
                }
            }
            DirectionToggle {}
            div { class: "min-h-0 flex-1 overflow-y-auto px-4 pb-3",
                ul { class: "mt-2",
                    for info in crates.into_iter().take(shown) {
                        li {
                            Link {
                                to: Route::Focus { name: info.name.clone() },
                                class: "flex w-full items-center gap-1.5 px-1 py-0.5 hover:bg-ink/5",
                                StarMark { info: info.clone(), focal: false, box_px: 18.0 }
                                span { class: "truncate font-data text-[11px] text-ink",
                                    "{info.name}"
                                    if repeated.contains(info.name.as_str()) {
                                        span { class: "text-ink-line", " v{info.version}" }
                                    }
                                }
                                if !info.is_member && !repeated.contains(info.name.as_str()) {
                                    span { class: "ml-auto shrink-0 font-data text-[9.5px] text-ink-soft",
                                        "v{info.version}"
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
            }
        }
    }
}
