//! The board, drawn to a canvas.
//!
//! One world, laid out once on the server, that never moves. Selecting a crate
//! flies the camera and changes which copper is lit; it does not re-seat a
//! single pad. Everything below follows from that:
//!
//! - There is no layout interpolation, because there is no second layout. The
//!   only authored motion in the app is the camera, which has weight.
//! - Zoom is not scale, it is level of detail. A board read at arm's length
//!   shows its shape; read through a loupe it shows drill holes and legends.
//!   The three tiers are the whole navigation model.
//! - Every dependency is drawn, all 1174 of them on this workspace. Copper sits one value step
//!   above the mask in the same hue, so density reads as the material the board
//!   is made of rather than as noise over a chart.
//!
//! Canvas costs three things SVG gave away, and each is paid for explicitly:
//! hit-testing is a quadtree over pad positions, keyboard access is a real
//! focusable list beside the canvas, and animation is a frame loop. In exchange
//! panning and zooming never touch the virtual DOM at all.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::graph::focus::Neighbourhood;
use crate::graph::quadtree::QuadTree;
use crate::graph::{Board, Point};

/// How the reader is looking at the board.
#[derive(Clone, Copy, PartialEq)]
pub struct Camera {
    /// Board point held at the centre of the viewport.
    pub cx: f32,
    pub cy: f32,
    /// Screen pixels per board unit at the fitted view.
    pub base: f32,
    /// Zoom notches from the fitted view.
    ///
    /// A float, because a pinch lands between notches and quantising it to
    /// whole steps is what makes a trackpad feel like it is fighting you. A
    /// wheel notch still adds exactly ±1.0, and adding then subtracting 1.0 is
    /// exact in binary floating point, so discrete zoom stays perfectly
    /// reversible — in and back out lands where it started.
    pub zoom: f32,
}

/// One wheel notch.
pub const ZOOM_STEP: f32 = 1.18;
const SCALE_MIN: f32 = 0.02;
const SCALE_MAX: f32 = 14.0;

impl Camera {
    pub fn scale(&self) -> f32 {
        (self.base * ZOOM_STEP.powf(self.zoom)).clamp(SCALE_MIN, SCALE_MAX)
    }
}

/// What the board shows at this magnification. Zoom is the only navigation verb
/// in the product, so it has to mean something at each stop rather than just
/// making the same picture bigger.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Tier {
    /// Arm's length: the shape of the workspace. Only what the board routes
    /// through is named.
    Board,
    /// Working distance: pads resolve, legends fill in, columns are countable.
    Component,
    /// Through a loupe: drill holes, the lattice the parts are seated on, and
    /// every crate named.
    Pad,
}

pub fn tier_of(scale: f32) -> Tier {
    if scale < 0.34 {
        Tier::Board
    } else if scale < 0.92 {
        Tier::Component
    } else {
        Tier::Pad
    }
}

/// Scale at which a pad is comfortably inspectable — where a flight lands when
/// the reader was further out than this.
pub const INSPECT_SCALE: f32 = 1.15;

/// How a trace reads right now. Direction gets hue because direction is the one
/// variable a reader makes a decision on; the design this replaced spent hue on
/// which of the workspace's direct dependencies happened to reach a crate first,
/// which decides nothing.
#[derive(Clone, Copy, PartialEq)]
pub enum Lit {
    /// No selection, or unrelated to it. Etched copper.
    Rest,
    /// Runs into the held pad: something that depends on it.
    In,
    /// Runs out of the held pad: something it depends on.
    Out,
    /// Related within the lit depth, but not touching the held pad.
    NearIn,
    NearOut,
}

pub struct PadDraw {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub label: String,
    pub designator: String,
    /// One line for the readout: version and both counts. Canvas has no `title`
    /// attribute, so the only way a pad identifies itself under the pointer is
    /// for the board to draw it.
    pub detail: String,
    pub tier: u8,
    pub is_root: bool,
    pub duplicate: bool,
    /// Hops from the held pad, when it is in the lit neighbourhood.
    pub level: Option<i32>,
    /// Related to the held pad at *any* distance, lit or not. A crate four hops
    /// out is still connected to what you are holding, and saying so costs one
    /// value step — where calling it unrelated is simply wrong.
    pub related: bool,
}

pub struct TraceDraw {
    /// Indices into `Scene::pads`.
    pub a: usize,
    pub b: usize,
    pub points: Vec<Point>,
    pub lit: Lit,
    /// The polyline's own bounding box, computed once when the board is seated.
    /// Culling against the two pads instead would drop any trace that detours
    /// through a channel outside them — which, with routing, is most of them.
    pub lo: Point,
    pub hi: Point,
}

/// A camera move. The world holds still; this is the only thing that travels,
/// and giving it weight is what makes "the camera flew there" legible as
/// motion rather than as the board having jumped.
#[derive(Clone, Copy)]
pub struct Flight {
    /// Where the camera actually was when this began — captured as values, not
    /// as a `Camera`, because a flight started mid-flight would otherwise take
    /// off from the *previous* flight's destination and visibly jump.
    from_cx: f32,
    from_cy: f32,
    from_scale: f32,
    to: Camera,
    started: f64,
}

/// How long the camera takes to cross the board.
const FLIGHT_MS: f64 = 560.0;
/// Pointer travel past which a gesture is a pan, not a click.
const DRAG_SLOP: f64 = 4.0;

pub struct Scene {
    pub pads: Vec<PadDraw>,
    pub traces: Vec<TraceDraw>,
    /// Pad id to index into `pads`.
    pub index_of: Vec<usize>,
    pub camera: Camera,
    pub flight: Option<Flight>,
    pub hover: Option<usize>,
    pub held: Option<usize>,
    pub query: String,
    pub tree: QuadTree,
    pub dirty: bool,
    /// Set when the camera has never been framed to this viewport.
    pub refit: bool,
    /// True once the reader has panned or zoomed. Until then a resize refits,
    /// because a board framed for a desktop window is not framed for a phone.
    pub touched: bool,
    /// Board geometry the renderer needs but that is not per-pad.
    pub pitch: f32,
    pub col_pitch: f32,
    /// Column x for each rank, for the depth ruler across the top.
    pub columns: Vec<f32>,
    /// Last palette painted with. The canvas only repaints when something marks
    /// it dirty, and a system theme change marks nothing — so the board would
    /// sit there in board ink under drawing-paper chrome until the next pan.
    pub painted_with: Palette,
    /// Reported back out so the chrome can name the tier the reader is in.
    pub tier: Tier,
    /// The canvas's CSS size as of the last painted frame. A flight has to know
    /// how much room it is aiming at, and the canvas is the only thing that
    /// measures it.
    pub viewport: (f32, f32),
    /// Pointers currently down, by id. Two of them is a pinch, which is the only
    /// way to zoom on a touchscreen — the canvas sets `touch-action: none`, so
    /// the browser hands us the raw fingers and hands us the whole gesture too.
    pub touches: Vec<(i32, f32, f32)>,
    /// Distance between the two pinching fingers on the previous move.
    pub pinch_span: Option<f32>,
}

/// The canvas's colours come from the same CSS tokens as the chrome, read at
/// draw time. Hardcoding them here would mean the board ignored the light
/// palette and any token change silently desynced two sources of truth.
#[derive(Clone, PartialEq)]
pub struct Palette {
    pub mask: String,
    pub mask_deep: String,
    pub substrate: String,
    pub edge: String,
    pub legend: String,
    pub legend_soft: String,
    pub copper: String,
    pub pad: String,
    pub pad_lit: String,
    pub drill: String,
    pub incoming: String,
    pub outgoing: String,
    pub flag: String,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            mask: "#0a1e14".into(),
            mask_deep: "#06140d".into(),
            substrate: "#163a26".into(),
            edge: "#1e4530".into(),
            legend: "#e4e7de".into(),
            legend_soft: "#8ca396".into(),
            copper: "#2a6647".into(),
            pad: "#d9a93c".into(),
            pad_lit: "#f0c860".into(),
            drill: "#05100a".into(),
            incoming: "#f0c24a".into(),
            outgoing: "#5ab0e8".into(),
            flag: "#e2643c".into(),
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            pads: Vec::new(),
            traces: Vec::new(),
            index_of: Vec::new(),
            camera: Camera {
                cx: 0.0,
                cy: 0.0,
                base: 1.0,
                zoom: 0.0,
            },
            flight: None,
            hover: None,
            held: None,
            query: String::new(),
            tree: QuadTree::build(&[]),
            dirty: true,
            refit: true,
            touched: false,
            pitch: 36.0,
            col_pitch: 260.0,
            columns: Vec::new(),
            painted_with: Palette::default(),
            tier: Tier::Board,
            viewport: (1600.0, 900.0),
            touches: Vec::new(),
            pinch_span: None,
        }
    }
}

impl Scene {
    pub fn extents(&self) -> (Point, Point) {
        let mut lo = Point {
            x: f32::INFINITY,
            y: f32::INFINITY,
        };
        let mut hi = Point {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
        };
        for pad in &self.pads {
            lo.x = lo.x.min(pad.x);
            lo.y = lo.y.min(pad.y);
            hi.x = hi.x.max(pad.x);
            hi.y = hi.y.max(pad.y);
        }
        if !lo.x.is_finite() {
            (Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 1.0 })
        } else {
            (lo, hi)
        }
    }

    /// The camera as of `time`, part-way through any flight.
    pub fn view(&self, time: f64) -> (f32, f32, f32) {
        let Some(flight) = self.flight else {
            let camera = self.camera;
            return (camera.cx, camera.cy, camera.scale());
        };
        let t = (((time - flight.started) / FLIGHT_MS) as f32).clamp(0.0, 1.0);
        let e = ease(t);
        // Scale interpolates geometrically: a camera half way through the move
        // should sit at the geometric mean of the two magnifications, not the
        // arithmetic one, or the move reads as a lurch at one end.
        let (a, b) = (flight.from_scale, flight.to.scale());
        (
            flight.from_cx + (flight.to.cx - flight.from_cx) * e,
            flight.from_cy + (flight.to.cy - flight.from_cy) * e,
            a * (b / a).powf(e),
        )
    }

    /// Send the camera to a pad and frame what it is attached to.
    ///
    /// `reach` is how far the pad's own dependents and dependencies sit from it,
    /// so the flight lands on a magnification where the answer is actually on
    /// screen. Flying to a fixed magnification instead put serde's pad dead
    /// centre with 25 of its 27 dependents outside the frame, which is a camera
    /// move that arrives nowhere.
    ///
    /// The pad stays at the centre rather than the neighbourhood's midpoint:
    /// "you are here" is the one thing the fiducial has to keep saying.
    pub fn fly_to(&mut self, pad_id: usize, reach: (f32, f32), time: f64) {
        let Some(&index) = self.index_of.get(pad_id) else {
            return;
        };
        let Some(pad) = self.pads.get(index) else {
            return;
        };
        let (cx, cy, scale) = self.view(time);
        let mut to = self.camera;
        to.cx = pad.x;
        to.cy = pad.y;

        let (vw, vh) = self.viewport;
        let (rx, ry) = reach;
        // Centred on the pad, so the span to cover is twice the reach.
        let want = if rx > 1.0 || ry > 1.0 {
            let fx = vw / (2.0 * rx + 4.0 * self.pitch);
            let fy = vh / (2.0 * ry + 4.0 * self.pitch);
            fx.min(fy).clamp(0.20, INSPECT_SCALE)
        } else {
            // A crate attached to nothing has no neighbourhood to frame, so it
            // just gets a good look at itself.
            INSPECT_SCALE
        };
        let notches = (want / self.camera.base).log(ZOOM_STEP);
        if notches.is_finite() {
            to.zoom = notches;
        }
        self.camera = to;
        self.flight = Some(Flight {
            from_cx: cx,
            from_cy: cy,
            from_scale: scale,
            to,
            started: time,
        });
        self.dirty = true;
        self.touched = true;
    }
}

/// Zoom by `notches`, holding one board point still under a fixed screen point.
///
/// `anchor` is in CSS pixels within the canvas; `None` anchors on the viewport
/// centre. Anchoring matters more than the amount: zooming about the centre
/// throws away whatever the reader was pointing at, which reads as broken rather
/// than as zoom.
pub fn zoom_about(scene: &mut Scene, notches: f32, anchor: Option<(f32, f32)>) {
    if !notches.is_finite() || notches == 0.0 {
        return;
    }
    let (vw, vh) = scene.viewport;
    let (ax, ay) = anchor.unwrap_or((vw / 2.0, vh / 2.0));
    scene.flight = None;

    let before = scene.camera.scale();
    scene.camera.zoom += notches;
    let after = scene.camera.scale();
    if before == after {
        // Clamped at a limit; moving the centre now would drift the board
        // sideways while the magnification stayed put.
        return;
    }

    let (ox, oy) = (ax - vw / 2.0, ay - vh / 2.0);
    let bx = scene.camera.cx + ox / before;
    let by = scene.camera.cy + oy / before;
    scene.camera.cx = bx - ox / after;
    scene.camera.cy = by - oy / after;
    scene.dirty = true;
    scene.touched = true;
}

/// Slide the board under the camera by a screen-space delta.
pub fn pan_by(scene: &mut Scene, dx: f32, dy: f32) {
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    let scale = scene.camera.scale();
    scene.flight = None;
    scene.camera.cx += dx / scale;
    scene.camera.cy += dy / scale;
    scene.dirty = true;
    scene.touched = true;
}

/// Exponential ease-out: quick to commit, slow to arrive, so the eye keeps hold
/// of where it came from.
fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[cfg(target_arch = "wasm32")]
pub fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> f64 {
    0.0
}

/// Load the board into the scene. Called once, when the board arrives.
pub fn seat(scene: &mut Scene, board: &Board) {
    let mut index_of = vec![usize::MAX; board.pads.len()];
    let mut pads = Vec::with_capacity(board.pads.len());
    for pad in &board.pads {
        index_of[pad.id] = pads.len();
        pads.push(PadDraw {
            id: pad.id,
            x: pad.x,
            y: pad.y,
            label: pad.label(),
            designator: pad.designator(),
            detail: format!(
                "{}  ·  {} depend on it  ·  needs {}",
                pad.version,
                pad.dependents.len(),
                pad.deps.len()
            ),
            tier: pad.legend_tier(),
            is_root: pad.is_root,
            duplicate: pad.duplicate,
            level: None,
            related: false,
        });
    }

    let traces = board
        .traces
        .iter()
        .filter_map(|trace| {
            let (a, b) = (index_of[trace.from], index_of[trace.to]);
            (a != usize::MAX && b != usize::MAX).then(|| {
                let mut lo = Point {
                    x: f32::INFINITY,
                    y: f32::INFINITY,
                };
                let mut hi = Point {
                    x: f32::NEG_INFINITY,
                    y: f32::NEG_INFINITY,
                };
                for p in &trace.points {
                    lo.x = lo.x.min(p.x);
                    lo.y = lo.y.min(p.y);
                    hi.x = hi.x.max(p.x);
                    hi.y = hi.y.max(p.y);
                }
                TraceDraw {
                    a,
                    b,
                    points: trace.points.clone(),
                    lit: Lit::Rest,
                    lo,
                    hi,
                }
            })
        })
        .collect();

    let points: Vec<(usize, f32, f32)> = pads
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.x, p.y))
        .collect();
    scene.tree = QuadTree::build(&points);

    let mut columns: Vec<f32> = Vec::new();
    for pad in &board.pads {
        if columns.len() <= pad.rank {
            columns.resize(pad.rank + 1, 0.0);
        }
        columns[pad.rank] = pad.x;
    }

    scene.pads = pads;
    scene.traces = traces;
    scene.index_of = index_of;
    scene.pitch = board.pitch;
    scene.col_pitch = board.col_pitch;
    scene.columns = columns;
    scene.refit = true;
    scene.dirty = true;
}

/// Re-light the board for a selection. Geometry is untouched — this only decides
/// which copper is gold, which is blue, and which stays etched.
pub fn relight(scene: &mut Scene, held: Option<usize>, view: Option<&Neighbourhood>) {
    scene.held = held;

    for pad in &mut scene.pads {
        pad.level = view.and_then(|v| v.level_of(pad.id));
        pad.related = view.is_some_and(|v| v.reaches(pad.id));
    }

    for trace in &mut scene.traces {
        let (from, to) = (scene.pads[trace.a].id, scene.pads[trace.b].id);
        trace.lit = match (held, view) {
            (Some(held), Some(view)) => {
                if to == held {
                    Lit::In
                } else if from == held {
                    Lit::Out
                } else {
                    // A trace is only "near" when both ends are lit and on the
                    // same side, so a chain into the selection stays readable as
                    // a chain and nothing gets coloured for a relationship it
                    // does not have.
                    match (view.level_of(from), view.level_of(to)) {
                        (Some(a), Some(b)) if a < 0 && b <= 0 => Lit::NearIn,
                        (Some(a), Some(b)) if a >= 0 && b > 0 => Lit::NearOut,
                        _ => Lit::Rest,
                    }
                }
            }
            _ => Lit::Rest,
        };
    }
    scene.dirty = true;
}

/// Frame the camera on the whole board.
pub fn fit(scene: &mut Scene, css: (f64, f64)) {
    let (lo, hi) = scene.extents();
    let pad = 80.0f32;
    let w = (hi.x - lo.x).max(1.0) + pad * 2.0;
    let h = (hi.y - lo.y).max(1.0) + pad * 2.0;
    let scale = ((css.0 as f32 / w).min(css.1 as f32 / h)).clamp(SCALE_MIN, 4.0);
    scene.camera = Camera {
        cx: (lo.x + hi.x) / 2.0,
        cy: (lo.y + hi.y) / 2.0,
        base: scale,
        zoom: 0.0,
    };
    scene.flight = None;
    scene.dirty = true;
    scene.refit = false;
}

/// Shared, mutable scene. Component props must compare, and a scene has no
/// meaningful value equality, so two handles are equal when they are the same
/// scene — which is exactly the question the renderer is asking.
#[derive(Clone)]
pub struct SceneHandle(Rc<RefCell<Scene>>);

impl PartialEq for SceneHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl SceneHandle {
    pub fn borrow(&self) -> std::cell::Ref<'_, Scene> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Scene> {
        self.0.borrow_mut()
    }

    pub fn try_borrow_mut(
        &self,
    ) -> Result<std::cell::RefMut<'_, Scene>, std::cell::BorrowMutError> {
        self.0.try_borrow_mut()
    }
}

pub fn use_scene() -> SceneHandle {
    use_hook(|| SceneHandle(Rc::new(RefCell::new(Scene::default()))))
}

pub use platform::BoardCanvas;

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use super::*;

    /// Server render: the canvas is empty markup until the client mounts. The
    /// keyboard list is client-built too, so there is nothing to pre-render.
    #[component]
    pub fn BoardCanvas(scene: SceneHandle, on_hold: EventHandler<Option<usize>>) -> Element {
        let _ = (scene, on_hold);
        rsx! {
            canvas { class: "block h-full w-full" }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    #[component]
    pub fn BoardCanvas(scene: SceneHandle, on_hold: EventHandler<Option<usize>>) -> Element {
        let dragging = use_signal(|| false);
        let moved = use_signal(|| false);
        let last = use_signal(|| (0.0f64, 0.0f64));

        let mounted = {
            let scene = scene.clone();
            move |event: Event<MountedData>| {
                let Some(element) = event.downcast::<web_sys::Element>() else {
                    return;
                };
                let Ok(canvas) = element.clone().dyn_into::<web_sys::HtmlCanvasElement>() else {
                    return;
                };
                attach_gestures(scene.clone(), &canvas);
                start_frame_loop(scene.clone(), canvas);
            }
        };

        let pointer_down = {
            let scene = scene.clone();
            let mut dragging = dragging;
            let mut moved = moved;
            let mut last = last;
            move |event: Event<PointerData>| {
                let p = event.client_coordinates();
                let e = event.element_coordinates();
                let mut scene_mut = scene.borrow_mut();
                let id = event.pointer_id();
                scene_mut.touches.retain(|(other, _, _)| *other != id);
                scene_mut.touches.push((id, e.x as f32, e.y as f32));
                if scene_mut.touches.len() >= 2 {
                    // A second finger turns the gesture into a pinch, so the
                    // drag that the first finger started is abandoned rather
                    // than left to fight it.
                    scene_mut.pinch_span = None;
                    dragging.set(false);
                    moved.set(true);
                    return;
                }
                dragging.set(true);
                moved.set(false);
                last.set((p.x, p.y));
            }
        };

        let pointer_move = {
            let scene = scene.clone();
            let mut moved = moved;
            let mut last = last;
            move |event: Event<PointerData>| {
                let p = event.client_coordinates();
                let e = event.element_coordinates();
                let mut scene_mut = scene.borrow_mut();

                // Two fingers down is a pinch: the only way to zoom on a
                // touchscreen, where there is no wheel to turn.
                let id = event.pointer_id();
                if let Some(slot) = scene_mut.touches.iter_mut().find(|(o, _, _)| *o == id) {
                    slot.1 = e.x as f32;
                    slot.2 = e.y as f32;
                }
                if scene_mut.touches.len() >= 2 {
                    let (_, ax, ay) = scene_mut.touches[0];
                    let (_, bx, by) = scene_mut.touches[1];
                    let span = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
                    let anchor = ((ax + bx) / 2.0, (ay + by) / 2.0);
                    if let Some(previous) = scene_mut.pinch_span
                        && previous > 1.0
                        && span > 1.0
                    {
                        let notches = (span / previous).log(ZOOM_STEP);
                        zoom_about(&mut scene_mut, notches, Some(anchor));
                    }
                    scene_mut.pinch_span = Some(span);
                    return;
                }

                if dragging() {
                    let (lx, ly) = last();
                    let (dx, dy) = (p.x - lx, p.y - ly);
                    if dx.abs() > DRAG_SLOP || dy.abs() > DRAG_SLOP {
                        moved.set(true);
                    }
                    // Panning by hand cancels a flight: the reader has taken the
                    // camera back.
                    scene_mut.flight = None;
                    let scale = scene_mut.camera.scale();
                    scene_mut.camera.cx -= dx as f32 / scale;
                    scene_mut.camera.cy -= dy as f32 / scale;
                    scene_mut.dirty = true;
                    scene_mut.touched = true;
                    last.set((p.x, p.y));
                } else {
                    // Hover: the quadtree is what replaces DOM hit-testing.
                    let e = event.element_coordinates();
                    let hit = pick(&scene_mut, e.x, e.y);
                    if scene_mut.hover != hit {
                        scene_mut.hover = hit;
                        scene_mut.dirty = true;
                    }
                }
            }
        };

        let pointer_up = {
            let scene = scene.clone();
            let mut dragging = dragging;
            let on_hold = on_hold;
            move |event: Event<PointerData>| {
                let id = event.pointer_id();
                {
                    let mut scene_mut = scene.borrow_mut();
                    scene_mut.touches.retain(|(other, _, _)| *other != id);
                    if scene_mut.touches.len() < 2 {
                        scene_mut.pinch_span = None;
                    }
                }
                dragging.set(false);
                if moved() {
                    return;
                }
                let e = event.element_coordinates();
                let hit = {
                    let scene_ref = scene.borrow();
                    pick(&scene_ref, e.x, e.y)
                };
                on_hold.call(hit);
            }
        };

        // A cancelled pointer never becomes a click, so this only lets go of the
        // gesture state. Without it, a pinch interrupted by the OS leaves a
        // phantom finger down and the next one-finger drag reads as a pinch.
        let pointer_cancel = {
            let scene = scene.clone();
            let mut dragging = dragging;
            move |event: Event<PointerData>| {
                let id = event.pointer_id();
                let mut scene_mut = scene.borrow_mut();
                scene_mut.touches.retain(|(other, _, _)| *other != id);
                if scene_mut.touches.len() < 2 {
                    scene_mut.pinch_span = None;
                }
                dragging.set(false);
            }
        };

        let pointer_leave = {
            let scene = scene.clone();
            move |_| {
                let mut scene_mut = scene.borrow_mut();
                if scene_mut.hover.is_some() {
                    scene_mut.hover = None;
                    scene_mut.dirty = true;
                }
            }
        };

        rsx! {
            canvas {
                // `touch-none` hands us the raw fingers instead of letting the
                // browser scroll the page with them, which is what makes the
                // two-finger pinch below possible at all.
                class: "block h-full w-full touch-none",
                class: if dragging() { "cursor-grabbing" } else { "cursor-grab" },
                onmounted: mounted,
                onpointerdown: pointer_down,
                onpointermove: pointer_move,
                onpointerup: pointer_up,
                onpointercancel: pointer_cancel,
                onpointerleave: pointer_leave,
            }
        }
    }

    /// Wheel and trackpad gestures, on a native listener rather than a Dioxus
    /// one.
    ///
    /// A trackpad pinch reaches the page as a wheel event with `ctrlKey` set —
    /// the same signal the browser binds its own page zoom to. Stopping that
    /// needs `preventDefault()` on a listener registered `passive: false`, and a
    /// framework-delegated listener gives no guarantee of either. So this is
    /// attached straight to the canvas.
    ///
    /// Writing through the scene handle from outside the Dioxus runtime is safe
    /// because the scene is a `RefCell`, not a signal; the frame loop already
    /// does exactly this.
    fn attach_gestures(scene: SceneHandle, canvas: &web_sys::HtmlCanvasElement) {
        let element = canvas.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
            // Always: a pinch here must not also zoom the browser chrome, and a
            // two-finger scroll must not also scroll the page behind the board.
            event.prevent_default();

            let rect = element.get_bounding_client_rect();
            let ax = (event.client_x() as f64 - rect.left()) as f32;
            let ay = (event.client_y() as f64 - rect.top()) as f32;
            let (dx, dy) = (event.delta_x() as f32, event.delta_y() as f32);

            let Ok(mut scene) = scene.try_borrow_mut() else {
                return;
            };

            if event.ctrl_key() {
                // A pinch. Continuous, and scaled so a full trackpad pinch
                // travels a useful distance without launching the reader into
                // the substrate.
                zoom_about(&mut scene, -dy * 0.035, Some((ax, ay)));
            } else if is_wheel_notch(&event) {
                zoom_about(&mut scene, if dy > 0.0 { -1.0 } else { 1.0 }, Some((ax, ay)));
            } else {
                // Two-finger scroll. On a trackpad this is how you look around;
                // making it zoom means you cannot move without also rescaling.
                pan_by(&mut scene, dx, dy);
            }
        }) as Box<dyn FnMut(web_sys::WheelEvent)>);

        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        let target: &web_sys::EventTarget = canvas.as_ref();
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            closure.as_ref().unchecked_ref(),
            &options,
        );
        closure.forget();
    }

    /// Is this a mouse wheel notch rather than a trackpad's continuous scroll?
    ///
    /// There is no flag for it, so this reads the two tells that hold in
    /// practice: a wheel reporting in lines or pages rather than pixels, and a
    /// wheel's characteristic large, purely-vertical, quantised jump. A trackpad
    /// scroll is small, usually carries some horizontal drift, and is rarely a
    /// round multiple of 100.
    fn is_wheel_notch(event: &web_sys::WheelEvent) -> bool {
        if event.delta_mode() != web_sys::WheelEvent::DOM_DELTA_PIXEL {
            return true;
        }
        let (dx, dy) = (event.delta_x(), event.delta_y());
        dx == 0.0 && dy.abs() >= 100.0 && (dy.abs() % 100.0 == 0.0 || dy.abs() % 120.0 == 0.0)
    }

    /// Canvas has no layout, so its CSS size is read back off the element.
    fn css_size() -> (f32, f32) {
        web_sys::window()
            .and_then(|w| {
                let d = w.document()?;
                let c = d.query_selector("canvas").ok()??;
                let r = c.get_bounding_client_rect();
                Some((r.width() as f32, r.height() as f32))
            })
            .unwrap_or((1600.0, 900.0))
    }

    /// What is under the pointer, in board space, via the quadtree.
    fn pick(scene: &Scene, ex: f64, ey: f64) -> Option<usize> {
        let (css_w, css_h) = css_size();
        let (cx, cy, scale) = scene.view(now());
        let mx = cx + (ex as f32 - css_w / 2.0) / scale;
        let my = cy + (ey as f32 - css_h / 2.0) / scale;
        // A forgiving target: 15 screen pixels, in board units.
        let index = scene.tree.nearest(mx, my, 15.0 / scale)?;
        Some(scene.pads[index].id)
    }

    /// The frame callback has to hold a reference to itself to re-arm, which is
    /// what makes this type as involved as it is.
    type FrameLoop = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

    fn start_frame_loop(scene: SceneHandle, canvas: web_sys::HtmlCanvasElement) {
        let holder: FrameLoop = Rc::new(RefCell::new(None));
        let clone = holder.clone();
        *clone.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
            draw(&scene, &canvas, time);
            if let Some(cb) = holder.borrow().as_ref() {
                request_frame(cb);
            }
        }) as Box<dyn FnMut(f64)>));
        if let Some(cb) = clone.borrow().as_ref() {
            request_frame(cb);
        }
    }

    fn request_frame(cb: &Closure<dyn FnMut(f64)>) {
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }

    /// Resolve the design tokens off the document, once per painted frame —
    /// which is already gated on something having changed.
    fn palette() -> Palette {
        let fallback = Palette::default();
        let Some(style) = web_sys::window().and_then(|window| {
            let root = window.document()?.document_element()?;
            window.get_computed_style(&root).ok().flatten()
        }) else {
            return fallback;
        };
        let read = |name: &str, default: &str| {
            style
                .get_property_value(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| default.to_string())
        };
        Palette {
            mask: read("--color-mask", &fallback.mask),
            mask_deep: read("--color-mask-deep", &fallback.mask_deep),
            substrate: read("--color-substrate", &fallback.substrate),
            edge: read("--color-edge", &fallback.edge),
            legend: read("--color-legend", &fallback.legend),
            legend_soft: read("--color-legend-soft", &fallback.legend_soft),
            copper: read("--color-copper", &fallback.copper),
            pad: read("--color-pad", &fallback.pad),
            pad_lit: read("--color-pad-lit", &fallback.pad_lit),
            drill: read("--color-drill", &fallback.drill),
            incoming: read("--color-incoming", &fallback.incoming),
            outgoing: read("--color-outgoing", &fallback.outgoing),
            flag: read("--color-flag", &fallback.flag),
        }
    }

    /// Silkscreen is condensed lettering, and the board's legend is silkscreen,
    /// so `condensed` rides in every legend font string below — the same width
    /// the `.silkscreen` class sets on the chrome.
    ///
    /// Chrome applies `font-stretch` from the canvas font shorthand but drops it
    /// from the property when you read it back, so this looks inert and in fact
    /// narrows the lettering by about 20%. Measured, not assumed.
    const LEGEND_FACE: &str = "Archivo, ui-sans-serif, system-ui, sans-serif";
    const MONO_FACE: &str = "\"JetBrains Mono\", ui-monospace, monospace";

    fn draw(scene: &SceneHandle, canvas: &web_sys::HtmlCanvasElement, time: f64) {
        let Ok(mut scene) = scene.try_borrow_mut() else {
            return;
        };
        let rect = canvas.get_bounding_client_rect();
        let (css_w, css_h) = (rect.width(), rect.height());
        if css_w < 1.0 || css_h < 1.0 {
            return;
        }
        scene.viewport = (css_w as f32, css_h as f32);
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        let (want_w, want_h) = ((css_w * dpr) as u32, (css_h * dpr) as u32);
        if canvas.width() != want_w || canvas.height() != want_h {
            canvas.set_width(want_w);
            canvas.set_height(want_h);
            scene.dirty = true;
            // A window that changed shape needs a new frame, unless the reader
            // has already chosen one.
            if !scene.touched {
                scene.refit = true;
            }
        }

        if scene.refit {
            super::fit(&mut scene, (css_w, css_h));
        }

        // The only way the canvas notices the reader switched system theme.
        let paint = palette();
        if scene.painted_with != paint {
            scene.painted_with = paint.clone();
            scene.dirty = true;
        }

        // A flight repaints every frame until it lands.
        let flying = scene.flight.is_some_and(|f| time - f.started < FLIGHT_MS);
        if flying {
            scene.dirty = true;
        } else if scene.flight.is_some() {
            scene.flight = None;
            scene.dirty = true;
        }
        if !scene.dirty {
            return;
        }
        scene.dirty = false;

        let Some(ctx) = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
        else {
            return;
        };

        let (cam_x, cam_y, scale) = scene.view(time);
        let tier = tier_of(scale);
        scene.tier = tier;

        let _ = ctx.reset_transform();
        ctx.scale(dpr, dpr).ok();
        // The surround is not the board. Filling everything with mask made the
        // board edgeless, so its empty regions read as missing data rather than
        // as bare substrate — and a board with no edge is not an object.
        ctx.set_fill_style_str(&paint.mask_deep);
        ctx.fill_rect(0.0, 0.0, css_w, css_h);

        let to_screen = |p: Point| {
            (
                ((p.x - cam_x) * scale) as f64 + css_w / 2.0,
                ((p.y - cam_y) * scale) as f64 + css_h / 2.0,
            )
        };
        let sx_of = |x: f32| ((x - cam_x) * scale) as f64 + css_w / 2.0;

        ctx.set_line_join("round");
        ctx.set_line_cap("round");

        // --- The board itself: a routed outline with radiused corners and a
        // mounting hole in each, which is what a bare board is cut to. This is
        // the frame everything else sits inside.
        let (blo, bhi) = scene.extents();
        let bleed = scene.pitch * 2.5;
        let edge_lo = to_screen(Point {
            x: blo.x - bleed,
            y: blo.y - bleed,
        });
        let edge_hi = to_screen(Point {
            x: bhi.x + bleed,
            y: bhi.y + bleed,
        });
        let (bw, bh) = (edge_hi.0 - edge_lo.0, edge_hi.1 - edge_lo.1);
        let corner = ((scene.pitch * 2.0 * scale) as f64).min(bw.min(bh) / 2.0);
        ctx.begin_path();
        let _ = ctx.round_rect_with_f64(edge_lo.0, edge_lo.1, bw, bh, corner);
        ctx.set_fill_style_str(&paint.mask);
        ctx.fill();
        ctx.set_stroke_style_str(&paint.edge);
        ctx.set_line_width(1.5);
        ctx.stroke();

        // Mounting holes, once they are big enough to be holes rather than dirt.
        let hole = (scene.pitch * 0.75 * scale) as f64;
        if hole > 2.5 {
            let inset = (scene.pitch * 1.6 * scale) as f64;
            for (hx, hy) in [
                (edge_lo.0 + inset, edge_lo.1 + inset),
                (edge_hi.0 - inset, edge_lo.1 + inset),
                (edge_lo.0 + inset, edge_hi.1 - inset),
                (edge_hi.0 - inset, edge_hi.1 - inset),
            ] {
                ctx.set_fill_style_str(&paint.mask_deep);
                ctx.begin_path();
                let _ = ctx.arc(hx, hy, hole, 0.0, std::f64::consts::TAU);
                ctx.fill();
                ctx.set_stroke_style_str(&paint.edge);
                ctx.set_line_width(1.0);
                ctx.stroke();
            }
        }

        // --- The substrate the parts are seated on. Drawn only where a lattice
        // row is far enough apart to read as a grid rather than as a fog, and
        // at full strength when it is drawn: a grid nobody can see is a grid
        // that cost a pass and delivered nothing.
        let row_px = (scene.pitch * scale) as f64;
        if row_px > 22.0 {
            ctx.set_fill_style_str(&paint.substrate);
            ctx.set_global_alpha(1.0);
            let (lo, hi) = scene.extents();
            let first_row = ((cam_y - (css_h as f32 / 2.0) / scale) / scene.pitch).floor() as i32;
            let last_row = ((cam_y + (css_h as f32 / 2.0) / scale) / scene.pitch).ceil() as i32;
            for row in first_row..=last_row {
                let y = row as f32 * scene.pitch;
                if y < lo.y - scene.pitch || y > hi.y + scene.pitch {
                    continue;
                }
                let sy = ((y - cam_y) * scale) as f64 + css_h / 2.0;
                let mut x = lo.x;
                while x <= hi.x {
                    let sx = sx_of(x);
                    if sx > -4.0 && sx < css_w + 4.0 {
                        let d = if row_px > 60.0 { 1.5 } else { 1.0 };
                        ctx.fill_rect(sx - d, sy - d, d * 2.0, d * 2.0);
                    }
                    x += scene.pitch;
                }
            }
            ctx.set_global_alpha(1.0);
        }

        // --- Column rules. The board's law made visible: everything to the
        // right of a pad is something it depends on, at every zoom, always.
        ctx.set_stroke_style_str(&paint.edge);
        ctx.set_line_width(1.0);
        ctx.set_global_alpha(if tier == Tier::Board { 0.5 } else { 0.35 });
        for &x in &scene.columns {
            let sx = sx_of(x);
            if sx < -2.0 || sx > css_w + 2.0 {
                continue;
            }
            ctx.begin_path();
            ctx.move_to(sx, 0.0);
            ctx.line_to(sx, css_h);
            ctx.stroke();
        }
        ctx.set_global_alpha(1.0);

        // --- Copper. Every dependency in the workspace, drawn. At rest it sits
        // one value step above the mask in the same hue, so a dense region reads
        // as a dense board and not as a chart someone spilled ink on.
        let held = scene.held;
        let rest_alpha = if held.is_some() { 0.30 } else { 0.62 };
        let rest_width = if tier == Tier::Board { 0.8 } else { 1.1 };

        let order = [Lit::Rest, Lit::NearOut, Lit::NearIn, Lit::Out, Lit::In];
        for pass in order {
            let (stroke, width, alpha) = match pass {
                Lit::Rest => (&paint.copper, rest_width, rest_alpha),
                Lit::NearIn => (&paint.incoming, 1.4, 0.42),
                Lit::NearOut => (&paint.outgoing, 1.4, 0.42),
                Lit::In => (&paint.incoming, 2.6, 1.0),
                Lit::Out => (&paint.outgoing, 2.2, 1.0),
            };
            ctx.set_stroke_style_str(stroke);
            ctx.set_line_width(width);
            ctx.set_global_alpha(alpha);
            ctx.begin_path();
            let mut drawn = false;
            for trace in &scene.traces {
                if trace.lit != pass {
                    continue;
                }
                // Cull whole traces outside the viewport before touching points.
                let lo = to_screen(trace.lo);
                let hi = to_screen(trace.hi);
                if hi.0 < -40.0 || lo.0 > css_w + 40.0 || hi.1 < -40.0 || lo.1 > css_h + 40.0 {
                    continue;
                }
                for (i, p) in trace.points.iter().enumerate() {
                    let s = to_screen(*p);
                    if i == 0 {
                        ctx.move_to(s.0, s.1);
                    } else {
                        ctx.line_to(s.0, s.1);
                    }
                }
                drawn = true;
            }
            if drawn {
                ctx.stroke();
            }
        }
        ctx.set_global_alpha(1.0);

        // --- Pads. Every one identical: an annular ring in gold with a drill
        // hole through it. Nothing about a crate deforms its pad, so what the
        // reader learns to recognise is the wiring, which is the only thing a
        // dependency actually is.
        let mut visible: Vec<(usize, f64, f64)> = Vec::new();
        for (i, pad) in scene.pads.iter().enumerate() {
            let (sx, sy) = to_screen(Point { x: pad.x, y: pad.y });
            if sx < -60.0 || sy < -60.0 || sx > css_w + 60.0 || sy > css_h + 60.0 {
                continue;
            }
            visible.push((i, sx, sy));
        }

        let radius: f64 = match tier {
            Tier::Board => 2.6,
            Tier::Component => 4.2,
            Tier::Pad => 6.4,
        };
        // A pad never grows past the room it has, or a dense column packs into
        // one solid bar of gold.
        let radius = radius.min((row_px / 2.2).max(1.6));
        let drill = radius * 0.42;

        for &(i, sx, sy) in &visible {
            let pad = &scene.pads[i];
            let is_held = held == Some(pad.id);
            let level = pad.level;

            let fill = if is_held {
                &paint.pad_lit
            } else {
                match level {
                    Some(l) if l < 0 => &paint.incoming,
                    Some(l) if l > 0 => &paint.outgoing,
                    _ => &paint.pad,
                }
            };
            // Distance fades the pad, so one hop reads as the answer and two as
            // context, without either changing shape. Four value steps, and the
            // gap between "connected further out" and "nothing to do with this"
            // is the one the reader is actually asking about.
            let alpha = if held.is_none() {
                1.0
            } else {
                match level {
                    Some(l) if l.abs() <= 1 => 1.0,
                    Some(_) => 0.62,
                    None if pad.related => 0.44,
                    None => 0.20,
                }
            };
            ctx.set_global_alpha(alpha);
            ctx.set_fill_style_str(fill);
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, radius, 0.0, std::f64::consts::TAU);
            ctx.fill();
            if drill > 0.9 {
                ctx.set_fill_style_str(&paint.drill);
                ctx.begin_path();
                let _ = ctx.arc(sx, sy, drill, 0.0, std::f64::consts::TAU);
                ctx.fill();
            }
        }
        ctx.set_global_alpha(1.0);

        // --- Silkscreen marks. The board says what it needs to say about a part
        // in white ink beside it, rather than by making the part a different
        // part. A workspace crate gets a component outline; a crate resolving at
        // more than one version gets a fab-note flag.
        if tier != Tier::Board {
            for &(i, sx, sy) in &visible {
                let pad = &scene.pads[i];
                if pad.is_root {
                    let box_r = radius + 4.5;
                    ctx.set_stroke_style_str(&paint.legend);
                    ctx.set_global_alpha(0.75);
                    ctx.set_line_width(1.2);
                    ctx.stroke_rect(sx - box_r, sy - box_r, box_r * 2.0, box_r * 2.0);
                }
                if pad.duplicate {
                    ctx.set_fill_style_str(&paint.flag);
                    ctx.set_global_alpha(1.0);
                    let f = radius + 3.0;
                    ctx.begin_path();
                    ctx.move_to(sx, sy - f - 4.0);
                    ctx.line_to(sx - 3.4, sy - f);
                    ctx.line_to(sx + 3.4, sy - f);
                    ctx.close_path();
                    ctx.fill();
                }
            }
            ctx.set_global_alpha(1.0);
        }

        // --- The held pad's fiducial. The one thing on the board that says
        // "you are here", drawn as the registration mark a board actually uses.
        if let Some(held_id) = held
            && let Some(&(_, sx, sy)) = visible.iter().find(|&&(i, _, _)| scene.pads[i].id == held_id)
        {
            ctx.set_stroke_style_str(&paint.legend);
            ctx.set_line_width(1.4);
            ctx.set_global_alpha(0.9);
            ctx.begin_path();
            let _ = ctx.arc(sx, sy, radius + 7.0, 0.0, std::f64::consts::TAU);
            ctx.stroke();
            let reach = radius + 14.0;
            for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                ctx.begin_path();
                ctx.move_to(sx + dx * (radius + 9.5), sy + dy * (radius + 9.5));
                ctx.line_to(sx + dx * reach, sy + dy * reach);
                ctx.stroke();
            }
            ctx.set_global_alpha(1.0);
        }

        // --- The depth ruler. Column index is longest-path distance from the
        // workspace, which is the thing x position has been encoding all along;
        // drawing it as a literal scale is what stops that encoding from being
        // something the reader has to infer.
        ctx.set_font(&format!("500 condensed 10px {LEGEND_FACE}"));
        ctx.set_text_baseline("middle");
        let ruler_h = 20.0;
        ctx.set_fill_style_str(&paint.mask_deep);
        ctx.fill_rect(0.0, 0.0, css_w, ruler_h);
        ctx.set_stroke_style_str(&paint.edge);
        ctx.set_line_width(1.0);
        ctx.begin_path();
        ctx.move_to(0.0, ruler_h + 0.5);
        ctx.line_to(css_w, ruler_h + 0.5);
        ctx.stroke();
        // What the ruler counts, said in words, once. Right-aligned so it sits
        // at the deep end of the board, which is the direction it points.
        let caption = "\u{2192} hops from the workspace";
        let caption_w = ctx.measure_text(caption).map(|m| m.width()).unwrap_or(150.0);
        let caption_x = css_w - caption_w - 10.0;
        ctx.set_fill_style_str(&paint.legend_soft);
        let _ = ctx.fill_text(caption, caption_x, ruler_h / 2.0);

        let column_px = (scene.col_pitch * scale) as f64;
        let every = if column_px < 34.0 {
            (34.0 / column_px).ceil() as usize
        } else {
            1
        };
        for (rank, &x) in scene.columns.iter().enumerate() {
            if rank % every != 0 {
                continue;
            }
            let sx = sx_of(x);
            let label = rank.to_string();
            let w = ctx.measure_text(&label).map(|m| m.width()).unwrap_or(8.0);
            // Never let a column number land under the caption.
            if sx - w / 2.0 < 8.0 || sx + w / 2.0 > caption_x - 12.0 {
                continue;
            }
            ctx.set_fill_style_str(&paint.legend_soft);
            let _ = ctx.fill_text(&label, sx - w / 2.0, ruler_h / 2.0);
        }

        // --- Legends. Silkscreen goes on last and opaque, above every trace and
        // pad, so a name always wins against its ground. Names are laid down in
        // order of importance and one that would land on a name already placed
        // is not drawn at all.
        let query = scene.query.trim().to_lowercase();
        let name_size = if tier == Tier::Pad { 12.0 } else { 11.0 };
        ctx.set_font(&format!("500 condensed {name_size}px {LEGEND_FACE}"));

        let mut candidates: Vec<(usize, f64, f64)> = visible
            .iter()
            .copied()
            .filter(|&(i, _, _)| {
                let pad = &scene.pads[i];
                let matched = !query.is_empty() && pad.label.to_lowercase().contains(&query);
                held == Some(pad.id)
                    || matched
                    || pad.level.is_some_and(|l| l.abs() <= 1)
                    || match tier {
                        Tier::Board => pad.tier == 0,
                        Tier::Component => pad.tier <= 1,
                        Tier::Pad => true,
                    }
            })
            .collect();
        candidates.sort_by_key(|&(i, _, _)| {
            let pad = &scene.pads[i];
            (
                held != Some(pad.id),
                pad.level.map(|l| l.abs()).unwrap_or(9),
                pad.tier,
            )
        });

        let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new();
        for &(i, sx, sy) in candidates.iter().take(500) {
            let pad = &scene.pads[i];
            let width = ctx
                .measure_text(&pad.label)
                .map(|m| m.width())
                .unwrap_or(pad.label.len() as f64 * 6.5);
            let (bx, by) = (sx + radius + 5.0, sy - 8.0);
            let (bw, bh) = (width + 7.0, 16.0);
            // A name that would run off an edge is not drawn: half a crate name
            // is worse than none, because it reads as a different crate.
            if by < ruler_h || bx + bw > css_w - 2.0 {
                continue;
            }
            let clashes = placed
                .iter()
                .any(|p| bx < p.0 + p.2 && bx + bw > p.0 && by < p.1 + p.3 && by + bh > p.1);
            if clashes {
                continue;
            }
            placed.push((bx, by, bw, bh));

            let dimmed = held.is_some() && pad.level.is_none() && held != Some(pad.id);
            ctx.set_global_alpha(if dimmed { 0.42 } else { 1.0 });
            ctx.set_fill_style_str(&paint.mask);
            ctx.fill_rect(bx, by, bw, bh);
            ctx.set_fill_style_str(match pad.level {
                _ if held == Some(pad.id) => &paint.legend,
                Some(l) if l < 0 => &paint.incoming,
                Some(l) if l > 0 => &paint.outgoing,
                _ => &paint.legend,
            });
            let _ = ctx.fill_text(&pad.label, bx + 3.5, sy);
        }
        ctx.set_global_alpha(1.0);

        // --- The detent. A pad under the pointer comes proud of the board: it
        // lifts, it casts a real shadow, and it names itself — read, but not yet
        // committed to. This is the one thing between looking and holding.
        if let Some(hover_id) = scene.hover
            && let Some(&(i, sx, sy)) = visible
                .iter()
                .find(|&&(i, _, _)| scene.pads[i].id == hover_id)
        {
            let pad = &scene.pads[i];

            ctx.set_shadow_color("rgba(0,0,0,0.55)");
            ctx.set_shadow_blur(9.0);
            ctx.set_shadow_offset_x(0.0);
            ctx.set_shadow_offset_y(3.0);
            ctx.set_fill_style_str(&paint.pad_lit);
            ctx.begin_path();
            let _ = ctx.arc(sx, sy - 2.0, radius + 1.6, 0.0, std::f64::consts::TAU);
            ctx.fill();
            ctx.set_shadow_blur(0.0);
            ctx.set_shadow_offset_y(0.0);
            ctx.set_shadow_color("rgba(0,0,0,0)");

            ctx.set_font(&format!("600 condensed 13px {LEGEND_FACE}"));
            let name_w = ctx
                .measure_text(&pad.label)
                .map(|m| m.width())
                .unwrap_or(60.0);
            ctx.set_font(&format!("400 11px {MONO_FACE}"));
            let detail_w = ctx
                .measure_text(&pad.detail)
                .map(|m| m.width())
                .unwrap_or(150.0);
            let designator_w = ctx
                .measure_text(&pad.designator)
                .map(|m| m.width())
                .unwrap_or(30.0);

            let bw = name_w.max(detail_w) + designator_w + 26.0;
            let bh = 42.0;
            // Flip to whichever side has room, so the readout is never clipped.
            let bx = if sx + 20.0 + bw > css_w {
                sx - 20.0 - bw
            } else {
                sx + 20.0
            };
            let by = (sy - bh / 2.0).clamp(ruler_h + 4.0, (css_h - bh - 4.0).max(ruler_h + 4.0));

            ctx.set_shadow_color("rgba(0,0,0,0.45)");
            ctx.set_shadow_blur(14.0);
            ctx.set_shadow_offset_y(4.0);
            ctx.set_fill_style_str(&paint.mask_deep);
            ctx.fill_rect(bx, by, bw, bh);
            ctx.set_shadow_blur(0.0);
            ctx.set_shadow_offset_y(0.0);
            ctx.set_shadow_color("rgba(0,0,0,0)");

            ctx.set_stroke_style_str(&paint.edge);
            ctx.set_line_width(1.0);
            ctx.stroke_rect(bx + 0.5, by + 0.5, bw - 1.0, bh - 1.0);

            ctx.set_font(&format!("600 condensed 13px {LEGEND_FACE}"));
            ctx.set_fill_style_str(&paint.legend);
            let _ = ctx.fill_text(&pad.label, bx + 10.0, by + 15.0);
            ctx.set_font(&format!("400 10px {MONO_FACE}"));
            ctx.set_fill_style_str(&paint.legend_soft);
            let _ = ctx.fill_text(&pad.designator, bx + bw - designator_w - 10.0, by + 15.0);
            ctx.set_font(&format!("400 11px {MONO_FACE}"));
            let _ = ctx.fill_text(&pad.detail, bx + 10.0, by + 30.0);
        }
    }
}
