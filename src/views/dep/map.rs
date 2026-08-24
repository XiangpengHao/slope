//! The chart itself: the workspace as concentric dependency rings.
//!
//! The crate under review sits at the center (the workspace's root crate by
//! default); every ring outward is one more dependency hop. Stars never
//! move. Every star is a link: selecting one draws its edges — ink lines to
//! what it depends on, hairlines from what depends on it — and manifest
//! events are always drawn, in flare. Each selection is a URL, so the
//! browser's back button retraces the review.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_flow::WorldLayer;
use dioxus_flow::prelude::{
    Controls, Edge as FlowEdge, EdgeViewCtx, Flow, MarkerKind, Node as FlowNode, NodeViewCtx,
    Point, Rect, Side, Size,
};

use crate::api::{CrateInfo, DepEvent, DepKind, WorkspaceGraph};
use crate::views::dep::model::{DEFAULT_CAP, RadialLayout};
use crate::views::dep::star::{StarData, StarNode, star_box, star_radius};
use crate::views::dep::{DirFilter, step_ring, use_dep};

/// How many of the biggest external stars carry their name at rest. Every
/// other external is named on hover, on selection, or as a neighbor.
const NAMED_EXTERNALS: usize = 20;

/// Display priority when several dependency kinds share one pair of crates:
/// a normal edge outranks build, build outranks dev.
fn kind_rank(kind: DepKind) -> u8 {
    match kind {
        DepKind::Normal => 0,
        DepKind::Build => 1,
        DepKind::Dev => 2,
    }
}

/// Why an edge is on the chart: the selection depends on its far end, the
/// far end depends on the selection, or a manifest event demands to be seen.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Dep,
    User,
    Event,
}

fn edge_style(role: Role, kind: DepKind, event: &Option<DepEvent>) -> String {
    match event {
        Some(DepEvent::Removed) => {
            "stroke: var(--color-flare); stroke-width: 1.2; stroke-dasharray: 9 5; opacity: 0.65;"
                .into()
        }
        Some(_) => "stroke: var(--color-flare); stroke-width: 1.4;".into(),
        None => {
            let dash = match kind {
                DepKind::Normal => "",
                DepKind::Dev => "stroke-dasharray: 6 4;",
                DepKind::Build => "stroke-dasharray: 2 3;",
            };
            match role {
                Role::Dep => format!("stroke: var(--color-ink); stroke-width: 1.25; {dash}"),
                Role::User | Role::Event => {
                    format!("stroke: var(--color-ink-line); stroke-width: 1.1; {dash}")
                }
            }
        }
    }
}

fn event_label(event: &DepEvent) -> String {
    match event {
        DepEvent::Added => "ADDED".into(),
        DepEvent::Removed => "REMOVED".into(),
        DepEvent::Bumped(old, new) => format!("{old} → {new}"),
    }
}

/// A phone-width viewport gets tighter chrome insets. Charts only render on
/// the client, so the server value is never hydrated against.
fn narrow_viewport() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .map(|w| w < 640.0)
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Authored motion stands down when the reader asked it to.
fn prefers_reduced_motion() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
            .flatten()
            .map(|m| m.matches())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// One drawing of the dependency chart: everything a single selection puts on
/// the paper, derived from the graph and the layout. Built once per reading
/// and read back by the camera, the keys, and every layer.
#[derive(Clone, PartialEq)]
struct DepDrawing {
    stars: Vec<FlowNode<StarData>>,
    lines: Vec<FlowEdge>,
    /// Ids of the selection and its direct neighbors, for framing.
    hood: HashSet<String>,
    /// A selection route is focused (vs the whole-chart overview).
    focused: bool,
    /// The selected ring, when the selection is a whole ring.
    ring: Option<u32>,
    /// The selection, materialized to crate names.
    names: Vec<String>,
}

/// Every crate on a route from the root down to the selection: the crates
/// that depend on it, then the crates that depend on those, hop by hop
/// until the chain runs out of users — which is where the root sits. The
/// selection itself is in the set, so an edge is one hop of some route
/// exactly when its dependency end is in here. Removed dependencies are no
/// longer routes; the epoch already cut them.
fn uphill_from<'g>(graph: &'g WorkspaceGraph, sel_ids: &HashSet<&'g str>) -> HashSet<&'g str> {
    let mut users: HashMap<&'g str, Vec<&'g str>> = HashMap::new();
    for link in &graph.links {
        if link.event == Some(DepEvent::Removed) {
            continue;
        }
        users
            .entry(link.to.as_str())
            .or_default()
            .push(link.from.as_str());
    }
    let mut seen: HashSet<&'g str> = sel_ids.clone();
    let mut queue: Vec<&'g str> = sel_ids.iter().copied().collect();
    while let Some(id) = queue.pop() {
        for &user in users.get(id).into_iter().flatten() {
            if seen.insert(user) {
                queue.push(user);
            }
        }
    }
    seen
}

impl DepDrawing {
    /// Derive the drawn chart. Every placed crate is always on the chart as a
    /// star; what changes with the selection is which edges are drawn, which
    /// stars carry the focal ring, and which are named at rest.
    fn build(
        graph: &WorkspaceGraph,
        layout: &RadialLayout,
        sel_names: Vec<String>,
        dir: DirFilter,
        focused: bool,
        ring: Option<u32>,
    ) -> Self {
        let sel: HashSet<&str> = sel_names.iter().map(String::as_str).collect();
        let sel_ids: HashSet<&str> = graph
            .crates
            .iter()
            .filter(|c| !c.ghost && sel.contains(c.name.as_str()))
            .map(|c| c.id.as_str())
            .collect();

        // "Path to root" draws a whole chain of crates, so it needs the
        // transitive closure first; the other two readings only ever look at
        // the selection's own edges.
        let uphill: HashSet<&str> = match dir {
            DirFilter::PathToRoot => uphill_from(graph, &sel_ids),
            _ => HashSet::new(),
        };

        // The edge set: the selection's own edges in the toggled direction (or
        // every hop of its routes to the root), plus every manifest event
        // (always drawn). Everything else stays undrawn — the whole resolved
        // graph at once is exactly the hairball this chart exists to refuse.
        let mut pairs: HashMap<(&str, &str), (Role, DepKind, Option<DepEvent>)> = HashMap::new();
        for link in &graph.links {
            let (from, to) = (link.from.as_str(), link.to.as_str());
            if !layout.placed.contains_key(from) || !layout.placed.contains_key(to) {
                continue;
            }
            let from_sel = sel_ids.contains(from);
            let to_sel = sel_ids.contains(to);
            let wanted = match dir {
                DirFilter::Deps => from_sel,
                DirFilter::Users => to_sel,
                // An edge is one hop of a route to the selection exactly when
                // its dependency end still reaches the selection.
                DirFilter::PathToRoot => uphill.contains(to),
            };
            if !wanted && link.event.is_none() {
                continue;
            }
            // Every hop of a route runs the dependents way, so it reads in the
            // dependents grammar: hairline, arrow pointing at the user.
            let role = if from_sel {
                Role::Dep
            } else if to_sel || wanted {
                Role::User
            } else {
                Role::Event
            };
            let entry = pairs
                .entry((from, to))
                .or_insert((role, link.kind, link.event.clone()));
            if kind_rank(link.kind) < kind_rank(entry.1) {
                entry.1 = link.kind;
            }
            if entry.2.is_none() {
                entry.2 = link.event.clone();
            }
            if entry.0 == Role::Event && role != Role::Event {
                entry.0 = role;
            }
        }

        // The selection's neighborhood: itself plus every far end of its edges.
        let mut hood: HashSet<String> = sel_ids.iter().map(|id| id.to_string()).collect();
        for ((from, to), (role, _, _)) in &pairs {
            if *role != Role::Event {
                hood.insert(from.to_string());
                hood.insert(to.to_string());
            }
        }

        // Lines are drawn from the dependency to the crate that uses it: the
        // direction change travels.
        let mut lines: Vec<FlowEdge> = pairs
            .into_iter()
            .map(|((from, to), (role, kind, event))| {
                let mut edge = FlowEdge::new(to, from)
                    .id(format!("{to}->{from}"))
                    .style(edge_style(role, kind, &event));
                edge.marker_end = MarkerKind::None;
                // The role class colors the arrowhead; the active reading on
                // the panel's dependencies toggle names what is drawn.
                edge = match (&event, role) {
                    (Some(ev), _) => edge.label(event_label(ev)).class("evented"),
                    (None, Role::Dep) => edge.class("dep"),
                    (None, _) => edge.class("user"),
                };
                edge
            })
            .collect();
        lines.sort_by(|a, b| a.id.cmp(&b.id));
        // Event labels on nearby curves take staggered seats so they never pile
        // onto one another.
        for (i, edge) in lines.iter_mut().filter(|e| e.label.is_some()).enumerate() {
            edge.label_position = [0.38, 0.54, 0.68][i % 3];
        }

        // Names engraved at rest: members, the diff, the neighborhood, and the
        // handful of externals big enough to be landmarks.
        let mut named: HashSet<&str> = graph
            .crates
            .iter()
            .filter(|c| c.is_member || c.changed || c.ghost || c.affected_dist.is_some())
            .map(|c| c.id.as_str())
            .collect();
        for id in &hood {
            if let Some(c) = graph.crates.iter().find(|c| &c.id == id) {
                named.insert(c.id.as_str());
            }
        }
        let mut landmarks: Vec<&CrateInfo> = graph
            .crates
            .iter()
            .filter(|c| !c.is_member && !c.ghost)
            .collect();
        landmarks.sort_by_key(|c| (std::cmp::Reverse(c.dependents), c.name.as_str()));
        for c in landmarks.into_iter().take(NAMED_EXTERNALS) {
            named.insert(c.id.as_str());
        }

        // A crate name can resolve to several versions at once (cargo keeps a
        // 1.x and a 2.x side by side), and each version is its own star on its
        // own ring. Those stars must say which version they are, or one crate
        // looks like it was drawn twice.
        let mut seen_names: HashMap<&str, u32> = HashMap::new();
        for c in graph.crates.iter().filter(|c| !c.ghost) {
            if layout.placed.contains_key(&c.id) {
                *seen_names.entry(c.name.as_str()).or_default() += 1;
            }
        }

        let mut stars: Vec<FlowNode<StarData>> = graph
            .crates
            .iter()
            .filter_map(|c| {
                let p = layout.placed.get(&c.id)?;
                let focal = !c.ghost && sel.contains(c.name.as_str());
                let b = star_box(c);
                let (ux, uy) = if p.ring == 0 {
                    (0.0, 1.0)
                } else {
                    (p.angle.cos(), p.angle.sin())
                };
                Some(
                    FlowNode::with_data(
                        c.id.clone(),
                        c.name.clone(),
                        (p.point.x - b / 2.0, p.point.y - b / 2.0),
                        StarData {
                            info: c.clone(),
                            ring: p.hops,
                            ux,
                            uy,
                            focal,
                            named: named.contains(c.id.as_str()),
                            versioned: seen_names.get(c.name.as_str()).is_some_and(|n| *n > 1),
                        },
                    )
                    .size(Size::new(b, b))
                    .sides(Side::Left, Side::Right)
                    .draggable(false)
                    .selectable(false),
                )
            })
            .collect();
        stars.sort_by(|a, b| a.id.cmp(&b.id));

        DepDrawing {
            stars,
            lines,
            hood,
            focused,
            ring,
            names: sel_names,
        }
    }
}

/// One radial edge: a curve bowed toward the center's open ground, trimmed
/// to stop at each star's rim. Spokes touching the center stay straight.
fn radial_edge(seat: &HashMap<String, (Point, f64)>, ctx: &EdgeViewCtx) -> Element {
    let (Some(&(a, ra)), Some(&(b, rb))) = (seat.get(&ctx.edge.source), seat.get(&ctx.edge.target))
    else {
        return rsx! {};
    };
    let mid = Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    let touches_center = a.distance(Point::ZERO) < 1.0 || b.distance(Point::ZERO) < 1.0;
    // Chords bow toward the center's open ground, in proportion to their
    // length: a long chord arcs through the gap, a short hop between
    // neighbors stays nearly straight instead of looping inward.
    let m = mid.distance(Point::ZERO);
    let ctrl = if touches_center || m < 1e-3 {
        mid
    } else {
        let bow = (0.3 * a.distance(b)).min(0.25 * m);
        let pull = 1.0 - bow / m;
        Point::new(mid.x * pull, mid.y * pull)
    };
    let trim = |p: Point, toward: Point, by: f64| -> Point {
        let (dx, dy) = (toward.x - p.x, toward.y - p.y);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-3 {
            p
        } else {
            Point::new(p.x + dx / len * by, p.y + dy / len * by)
        }
    };
    let start = trim(a, ctrl, ra + 2.0);
    let end = trim(b, ctrl, rb + 2.0);
    let d = format!(
        "M{:.2},{:.2} Q{:.2},{:.2} {:.2},{:.2}",
        start.x, start.y, ctrl.x, ctrl.y, end.x, end.y
    );
    let t = ctx.edge.label_position;
    let u = 1.0 - t;
    let lx = u * u * start.x + 2.0 * u * t * ctrl.x + t * t * end.x;
    let ly = u * u * start.y + 2.0 * u * t * ctrl.y + t * t * end.y;

    // The arrowhead points the way change travels: edges run from the
    // dependency into the crate that uses it, so the chevron sits at the
    // user's end. Without it, an inward dependent edge reads as an outward
    // dependency edge.
    let (hdx, hdy) = {
        let (dx, dy) = (end.x - ctrl.x, end.y - ctrl.y);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        (dx / len, dy / len)
    };
    let (hpx, hpy) = (-hdy, hdx);
    const HEAD_L: f64 = 5.0;
    const HEAD_W: f64 = 2.6;
    let (hx, hy) = (end.x - hdx * HEAD_L, end.y - hdy * HEAD_L);
    let head = format!(
        "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
        hx + hpx * HEAD_W,
        hy + hpy * HEAD_W,
        end.x,
        end.y,
        hx - hpx * HEAD_W,
        hy - hpy * HEAD_W,
    );

    rsx! {
        path {
            class: "df-edge-path",
            d,
            fill: "none",
            style: ctx.edge.style.as_deref().unwrap_or_default(),
        }
        path {
            class: "edge-head",
            d: head,
            fill: "none",
            stroke_width: "1.1",
            stroke_linecap: "round",
            stroke_linejoin: "round",
        }
        if let Some(label) = ctx.edge.label.as_deref() {
            text {
                class: "df-edge-label",
                x: "{lx}",
                y: "{ly}",
                text_anchor: "middle",
                "{label}"
            }
        }
    }
}

/// The engraved ring guides — and the ring control itself: each hairline
/// circle is a link that selects every crate on its ring, so the ring needs
/// no caption of its own. On a virtual workspace (no root crate) the center
/// carries a small workspace medallion instead of a star.
///
/// The canvas is a fixed two-pixel box drawn with `overflow: visible`, so the
/// circles are laid out in flow coordinates around the origin. Only their
/// radii change when the outermost band expands, which is what lets the
/// guides glide outward with the stars instead of jumping.
#[component]
fn RingCircles(
    radii: Vec<f64>,
    hub: Option<String>,
    selected: Option<u32>,
    collapsed: bool,
) -> Element {
    let nav = use_navigator();
    if radii.is_empty() {
        return rsx! {};
    }
    // Ring zero is the center itself, radius zero: never a guide, never a
    // control.
    let last = radii.len() - 1;
    rsx! {
        svg {
            width: "2",
            height: "2",
            style: "position: absolute; left: 0; top: 0; overflow: visible;",
            for (k , r) in radii.iter().copied().enumerate().skip(1) {
                g {
                    key: "{k}",
                    class: "ring-guide",
                    class: if selected == Some(k as u32) { "is-selected" },
                    tabindex: "0",
                    role: "link",
                    "aria-label": if collapsed && k == last { "select every crate {k} or more hops from the center" } else { "select every crate {k} hops from the center" },
                    onclick: move |_| {
                        nav.push(crate::Route::DepRing { hop: k as u32 });
                    },
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            nav.push(crate::Route::DepRing { hop: k as u32 });
                        }
                    },
                    title {
                        if collapsed && k == last {
                            "ring {k}+ · select every crate {k} or more hops out"
                        } else if k == 1 {
                            "ring 1 · select every crate 1 hop out"
                        } else {
                            "ring {k} · select every crate {k} hops out"
                        }
                    }
                    circle { class: "ring-guide-hit", cx: "0", cy: "0", r: "{r}" }
                    circle { class: "ring-guide-line", cx: "0", cy: "0", r: "{r}" }
                    // The ring names its own distance, the way an atlas
                    // captions a parallel — so the chart needs no key to say
                    // that one ring out is one dependency hop. Positioned by
                    // transform, the one property that glides with the
                    // circle's radius when the outer band expands; sized to
                    // its ring the way a region name is sized to its frame,
                    // so the caption stays a glance's read at the fit zoom.
                    text {
                        class: "ring-caption",
                        x: "0",
                        y: "0",
                        text_anchor: "middle",
                        style: "transform: translateY({-(r + 8.0)}px); font-size: {(r * 0.045).clamp(12.0, 26.0)}px;",
                        if collapsed && k == last {
                            "{k}+ hops"
                        } else if k == 1 {
                            "1 hop"
                        } else {
                            "{k} hops"
                        }
                    }
                }
            }
            if let Some(name) = &hub {
                circle { cx: "0", cy: "0", r: "5", fill: "var(--color-ink)" }
                circle {
                    cx: "0",
                    cy: "0",
                    r: "9",
                    fill: "none",
                    stroke: "var(--color-ink)",
                    stroke_width: "0.7",
                }
                text { class: "hub-caption", x: "0", y: "24", text_anchor: "middle", "{name}" }
            }
        }
    }
}

/// The chart furniture's screen space, per side: (top, right, bottom, left).
fn chrome_insets(narrow: bool, focused: bool) -> (f64, f64, f64, f64) {
    if narrow {
        (195.0, 20.0, if focused { 400.0 } else { 70.0 }, 12.0)
    } else {
        // The title block and the key own the left column; search alone
        // sits top-right, and a selection's panel claims the right column.
        (52.0, if focused { 330.0 } else { 20.0 }, 20.0, 284.0)
    }
}

/// Reserves the chart furniture's screen space so framing centers the rings
/// in what remains. Must render inside `Flow`.
#[component]
fn FitInsets(top: f64, right: f64, bottom: f64, left: f64) -> Element {
    dioxus_flow::use_overlay_inset(Side::Top, top);
    dioxus_flow::use_overlay_inset(Side::Right, right);
    dioxus_flow::use_overlay_inset(Side::Bottom, bottom);
    dioxus_flow::use_overlay_inset(Side::Left, left);
    rsx! {}
}

/// A focused neighborhood never zooms lettering below legibility: past this
/// floor the chart stays at readable scale and pans instead.
const MIN_FOCUS_ZOOM: f64 = 0.7;
/// The overview's job is the whole shape of the workspace, so it may zoom
/// much further out — names fold away; the rings and the flares carry it.
const MIN_OVERVIEW_ZOOM: f64 = 0.22;

/// The window's inner size; `None` off the client, where nothing frames.
fn window_size() -> Option<(f64, f64)> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let w = window.inner_width().ok()?.as_f64()?;
        let h = window.inner_height().ok()?.as_f64()?;
        Some((w, h))
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// Frame the given flow-space bounds in the screen area the furniture leaves
/// free. Never zooms lettering below the legibility floor (the chart pans for
/// the rest, seated on the focal star) and never above natural scale.
fn frame_chart(
    flow: dioxus_flow::prelude::FlowHandle<StarData>,
    bounds: Rect,
    focal_center: Option<Point>,
    panel_open: bool,
    legible_floor: bool,
    duration_ms: u64,
) {
    let Some(core) = flow.core() else { return };
    let Some((w, h)) = window_size() else {
        return;
    };
    let narrow = narrow_viewport();
    let (t, r, b, l) = chrome_insets(narrow, panel_open);
    let free_w = (w - l - r).max(120.0);
    let free_h = (h - t - b).max(120.0);
    let fit = (free_w / bounds.width.max(1.0)).min(free_h / bounds.height.max(1.0)) * 0.88;
    let floor = if legible_floor {
        MIN_FOCUS_ZOOM
    } else {
        MIN_OVERVIEW_ZOOM
    };
    let zoom = fit.clamp(floor, 1.0);
    let floored = fit < floor;
    // Past the floor, seat the focal star mid-frame and let the reader pan.
    let center = if floored {
        focal_center.unwrap_or_else(|| bounds.center())
    } else {
        bounds.center()
    };
    let cx = l + free_w / 2.0;
    let cy = t + free_h / 2.0;
    core.set_viewport(
        dioxus_flow::prelude::Viewport::new(cx - center.x * zoom, cy - center.y * zoom, zoom),
        duration_ms,
    );
}

impl DepDrawing {
    /// The bounds the camera should frame: a focused view frames the
    /// selection's neighborhood; the overview frames every ring.
    fn frame_target(&self) -> (Option<Rect>, Option<Point>) {
        let focal_center = self
            .stars
            .iter()
            .filter(|n| n.data.focal)
            .min_by_key(|n| n.data.ring)
            .map(|n| n.rect().center());
        let rects: Vec<Rect> = self
            .stars
            .iter()
            .filter(|n| !self.focused || self.hood.contains(&n.id))
            .map(|n| n.rect())
            .collect();
        (Rect::bounds(rects), focal_center)
    }
}

/// The keyboard surface, taught where each key acts: `/` in the search
/// placeholder, `n`/`p` beside the changes list they walk, `f` on the fit
/// control; Escape steps back. Typing fields keep their keys. Rebinds on
/// every mount so the listener always feeds the living channel, not a
/// dropped one.
const KEYS_JS: &str = r#"
if (window.__slopeKeys) {
    document.removeEventListener('keydown', window.__slopeKeys);
}
window.__slopeKeys = (e) => {
    const t = e.target, tag = t && t.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (t && t.isContentEditable)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') {
        e.preventDefault();
        const s = document.getElementById('dep-search');
        if (s) s.focus();
        return;
    }
    if (['n', 'p', 'f', 'Escape'].includes(e.key)) dioxus.send(e.key);
};
document.addEventListener('keydown', window.__slopeKeys);
"#;

/// The ring cap the chart last painted. Provided as a context by the app
/// shell because stepping to another altitude (`/dep` ↔ `/code`) unmounts the
/// chart and throws its DOM away: without a memory of the geometry the
/// reader was just looking at, an expansion after a return could only be
/// drawn as a jump.
#[derive(Clone, Copy)]
pub(crate) struct DrawnCap {
    pub(crate) cap: Signal<Option<u32>>,
}

impl DrawnCap {
    pub(crate) fn new() -> Self {
        Self {
            cap: Signal::new(None),
        }
    }
}

/// The rings chart, mounted once for the whole session.
#[component]
pub(super) fn Chart(graph: WorkspaceGraph) -> Element {
    let dep = use_dep();
    let drawn_cap = use_context::<DrawnCap>().cap;
    let flow = dioxus_flow::use_flow_handle::<StarData>();
    let nav = use_navigator();

    // True distances, computed once: they decide how far the ring cap must
    // expand for a selection to sit on its exact ring.
    let base = use_memo({
        let graph = graph.clone();
        move || RadialLayout::build(&graph, u32::MAX)
    });
    let name_hops = use_memo({
        let graph = graph.clone();
        move || {
            let base = base.read();
            let mut hops: HashMap<String, u32> = HashMap::new();
            for c in graph.crates.iter().filter(|c| !c.ghost) {
                if let Some(p) = base.placed.get(&c.id) {
                    let e = hops.entry(c.name.clone()).or_insert(0);
                    *e = (*e).max(p.hops);
                }
            }
            hops
        }
    });
    // The ring cap: four rings at rest, the outermost a collapsed "4+"
    // band. Selecting into the band expands exact rings down to the
    // selection's true depth; deselecting folds them back.
    let cap = use_memo(move || {
        let step = dep.trail.read().current_focus();
        match step.as_deref() {
            None => DEFAULT_CAP,
            Some(step) => match step_ring(step) {
                Some(hop) => hop.max(DEFAULT_CAP),
                None => {
                    let hops = name_hops.read();
                    let deepest = step
                        .split('+')
                        .filter_map(|n| hops.get(n).copied())
                        .max()
                        .unwrap_or(0);
                    DEFAULT_CAP.max(deepest.saturating_add(1))
                }
            },
        }
    });
    // The cap the chart is drawing right now. It starts at whatever the last
    // paint used — including across a remount — and then slides to the target
    // one frame later, so the stars and ring guides have a position to travel
    // from and the expansion is drawn as a move.
    let drawn = use_signal(|| drawn_cap.peek().unwrap_or(*cap.peek()));
    use_effect(move || {
        let target = cap();
        let mut drawn = drawn;
        let mut drawn_cap = drawn_cap;
        if *drawn.peek() == target {
            if *drawn_cap.peek() != Some(target) {
                drawn_cap.set(Some(target));
            }
            return;
        }
        if prefers_reduced_motion() {
            drawn.set(target);
            drawn_cap.set(Some(target));
            return;
        }
        spawn(async move {
            // One frame at the old radii is what gives the CSS transition its
            // starting point; without the wait both paints coalesce into one.
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::TimeoutFuture::new(32).await;
            drawn.set(target);
            drawn_cap.set(Some(target));
        });
    });
    // The geometry: angles are cap-independent, so a cap change only slides
    // collapsed stars radially — nothing ever swings sideways.
    let layout = use_memo({
        let graph = graph.clone();
        move || RadialLayout::build(&graph, drawn())
    });
    // The default selection: the crate at the center.
    let center_name = use_memo({
        let graph = graph.clone();
        move || {
            layout().center.and_then(|id| {
                graph
                    .crates
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
            })
        }
    });
    // Each star's seat and rim radius, for drawing edges between them.
    let seats = use_memo({
        let graph = graph.clone();
        move || {
            let layout = layout();
            graph
                .crates
                .iter()
                .filter_map(|c| {
                    let p = layout.placed.get(&c.id)?;
                    Some((c.id.clone(), (p.point, star_radius(c.dependents))))
                })
                .collect::<HashMap<String, (Point, f64)>>()
        }
    });

    // The chart is a pure function of the selection and direction filter.
    let chart = use_memo({
        let graph = graph.clone();
        move || {
            let step = dep.trail.read().current_focus();
            let focused = step.is_some();
            let layout = layout.read();
            let (sel_names, ring) = match step.as_deref() {
                None => (center_name().into_iter().collect::<Vec<_>>(), None),
                Some(step) => match step_ring(step) {
                    Some(hop) => {
                        let mut names: Vec<String> = graph
                            .crates
                            .iter()
                            .filter(|c| {
                                !c.ghost && layout.placed.get(&c.id).is_some_and(|p| p.ring == hop)
                            })
                            .map(|c| c.name.clone())
                            .collect();
                        names.sort();
                        names.dedup();
                        (names, Some(hop))
                    }
                    None => (step.split('+').map(str::to_string).collect(), None),
                },
            };
            let dir = *dep.dir.read();
            DepDrawing::build(&graph, &layout, sel_names, dir, focused, ring)
        }
    });

    let nodes: Signal<Vec<FlowNode<StarData>>> = use_signal(Vec::new);
    let edges: Signal<Vec<FlowEdge>> = use_signal(Vec::new);
    let framed = use_signal(|| false);

    // Apply each build to the canvas and frame the camera: the whole rings
    // on the overview, the selection's neighborhood on a focus.
    use_effect(move || {
        let drawing = chart();
        let (bounds, focal_center) = drawing.frame_target();
        let focused = drawing.focused;
        // A whole-ring selection frames like the overview: the ring is the
        // shape, not a neighborhood to zoom into.
        let legible = drawing.focused && drawing.ring.is_none();
        let reduced = prefers_reduced_motion();
        let mut nodes = nodes;
        let mut edges = edges;
        // Materialize the selection for modifier-clicks to toggle against.
        let mut selected = dep.selected;
        if *selected.peek() != drawing.names {
            selected.set(drawing.names.clone());
        }
        nodes.set(drawing.stars);
        edges.set(drawing.lines);
        // While the rings are still sliding to the new cap, the bounds belong
        // to the geometry being left behind: frame once, when it settles.
        if drawn() != cap() {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut framed = framed;
            let first = !*framed.peek();
            framed.set(true);
            // Deselecting never yanks the camera: the reviewer keeps the
            // view they had. Only the first paint and selections frame.
            if !focused && !first {
                return;
            }
            let duration = if first || reduced { 0 } else { 400 };
            spawn(async move {
                // The container needs a beat to be measured on first mount.
                gloo_timers::future::TimeoutFuture::new(if first { 150 } else { 20 }).await;
                if let Some(bounds) = bounds {
                    frame_chart(flow, bounds, focal_center, focused, legible, duration);
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (framed, reduced);
            if let Some(bounds) = bounds {
                frame_chart(flow, bounds, focal_center, focused, legible, 0);
            }
        }
    });

    // Global keys: `/` is handled fully in the page, the rest arrive here.
    use_hook(move || {
        let changed: Vec<String> = {
            let mut names: Vec<String> = graph
                .crates
                .iter()
                .filter(|c| c.changed)
                .map(|c| c.name.clone())
                .collect();
            names.sort();
            names
        };
        spawn(async move {
            let mut eval = document::eval(KEYS_JS);
            while let Ok(key) = eval.recv::<String>().await {
                match key.as_str() {
                    "f" => {
                        let bounds = {
                            let drawing = chart.peek();
                            dioxus_flow::prelude::Rect::bounds(
                                drawing.stars.iter().map(|n| n.rect()),
                            )
                        };
                        if let Some(bounds) = bounds {
                            frame_chart(flow, bounds, None, false, false, 400);
                        }
                    }
                    "Escape" => {
                        let trail = dep.trail.peek().clone();
                        if trail.at > 0 {
                            crate::views::shell::history_back();
                        } else if trail.current_focus().is_some() {
                            nav.push(crate::Route::DepOverview {});
                        }
                    }
                    "n" | "p" if !changed.is_empty() => {
                        let current = dep.trail.peek().current_focus();
                        let at = current
                            .as_deref()
                            .and_then(|f| changed.iter().position(|n| n == f));
                        let next = match (key.as_str(), at) {
                            ("n", Some(i)) => (i + 1) % changed.len(),
                            ("p", Some(i)) => (i + changed.len() - 1) % changed.len(),
                            ("n", None) => 0,
                            _ => changed.len() - 1,
                        };
                        nav.push(crate::Route::DepFocus {
                            name: changed[next].clone(),
                        });
                    }
                    _ => {}
                }
            }
        });
    });

    let step = dep.trail.read().current_focus();
    let focused = step.is_some();
    let ring_sel = step.as_deref().and_then(step_ring);
    let hub = if center_name.read().is_none() {
        Some(graph.name.clone())
    } else {
        None
    };

    rsx! {
        div { class: "absolute inset-0",
            Flow {
                nodes,
                edges,
                fit_view: false,
                handle: flow,
                nodes_draggable: false,
                delete_key: false,
                // A trackpad's two-finger travel is a pan, as every canvas
                // tool reads it; pinch (ctrl/meta wheel) zooms at the pointer.
                pan_on_scroll: true,
                node_view: move |ctx: NodeViewCtx<StarData>| rsx! {
                    StarNode { ctx }
                },
                edge_view: move |ctx: EdgeViewCtx| radial_edge(&seats.read(), &ctx),
                {
                    let (top, right, bottom, left) = chrome_insets(narrow_viewport(), focused);
                    rsx! {
                        FitInsets { top, right, bottom, left }
                    }
                }
                WorldLayer { class: "ring-guides",
                    RingCircles {
                        radii: layout.read().radii.clone(),
                        hub,
                        selected: ring_sel,
                        collapsed: layout.read().max_hops > drawn(),
                    }
                }
                Controls {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{DepLink, Epoch};

    fn krate(id: &str) -> CrateInfo {
        CrateInfo {
            id: id.to_string(),
            name: id.split('@').next().unwrap().to_string(),
            version: "1.0.0".to_string(),
            is_member: false,
            changed: false,
            changed_files: 0,
            manifest_changed: false,
            affected_dist: None,
            dependents: 0,
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

    fn link(from: &str, to: &str, event: Option<DepEvent>) -> DepLink {
        DepLink {
            from: from.to_string(),
            to: to.to_string(),
            kind: DepKind::Normal,
            event,
        }
    }

    fn graph(names: &[&str], links: Vec<DepLink>) -> WorkspaceGraph {
        WorkspaceGraph {
            name: "test".into(),
            root: "/test".into(),
            root_crate: Some("root@1.0.0".into()),
            epoch: Epoch {
                base: "base".into(),
                target: "working copy".into(),
                note: None,
            },
            crates: names.iter().map(|n| krate(n)).collect(),
            links,
        }
    }

    /// Edge ids, sorted. Lines run from the dependency into its user, so an
    /// id reads `dependency->user`.
    fn drawn(graph: &WorkspaceGraph, sel: &str, dir: DirFilter) -> Vec<String> {
        let layout = RadialLayout::build(graph, DEFAULT_CAP);
        let drawing = DepDrawing::build(graph, &layout, vec![sel.to_string()], dir, true, None);
        let mut ids: Vec<String> = drawing.lines.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        ids
    }

    /// A diamond: both routes from the root down to the shared dependency
    /// are drawn whole, and nothing off those routes is.
    #[test]
    fn path_to_root_draws_every_route() {
        let g = graph(
            &[
                "root@1.0.0",
                "a@1.0.0",
                "b@1.0.0",
                "shared@1.0.0",
                "other@1.0.0",
            ],
            vec![
                link("root@1.0.0", "a@1.0.0", None),
                link("root@1.0.0", "b@1.0.0", None),
                link("root@1.0.0", "other@1.0.0", None),
                link("a@1.0.0", "shared@1.0.0", None),
                link("b@1.0.0", "shared@1.0.0", None),
            ],
        );
        assert_eq!(
            drawn(&g, "shared", DirFilter::PathToRoot),
            [
                "a@1.0.0->root@1.0.0",
                "b@1.0.0->root@1.0.0",
                "shared@1.0.0->a@1.0.0",
                "shared@1.0.0->b@1.0.0",
            ]
        );
        // The other two readings still see only the selection's own edges.
        assert_eq!(
            drawn(&g, "shared", DirFilter::Users),
            ["shared@1.0.0->a@1.0.0", "shared@1.0.0->b@1.0.0"]
        );
        assert!(drawn(&g, "shared", DirFilter::Deps).is_empty());
    }

    /// A dependency the epoch removed is no longer a route: the crate that
    /// dropped it does not light up the chain above it. The removed edge
    /// itself still draws, as every manifest event does.
    #[test]
    fn removed_dependencies_are_not_routes() {
        let g = graph(
            &["root@1.0.0", "x@1.0.0", "y@1.0.0", "sel@1.0.0"],
            vec![
                link("root@1.0.0", "x@1.0.0", None),
                link("root@1.0.0", "y@1.0.0", None),
                link("x@1.0.0", "sel@1.0.0", Some(DepEvent::Removed)),
                link("y@1.0.0", "sel@1.0.0", None),
            ],
        );
        assert_eq!(
            drawn(&g, "sel", DirFilter::PathToRoot),
            [
                "sel@1.0.0->x@1.0.0",
                "sel@1.0.0->y@1.0.0",
                "y@1.0.0->root@1.0.0",
            ]
        );
    }
}
