//! The pane: the one surface both lenses draw on.
//!
//! The DOM shape is the shape every flow canvas converged on, and it is chosen
//! for one reason. Nodes are absolutely positioned elements inside a single
//! transformed layer, so panning and zooming write **one** transform string
//! rather than touching a hundred nodes; the dot lattice is a background image
//! on the pane itself, re-offset by the same camera, so the grid costs nothing
//! at all.
//!
//! Gestures are installed natively rather than through the framework's own
//! listeners: a trackpad pinch reaches the page as a `ctrl`+wheel event, which
//! is the browser's own page-zoom binding, and refusing it needs a listener
//! registered `passive: false`. The listener reports back through the eval
//! channel, so every signal write still happens inside the runtime — a write
//! from outside it is not a write at all, it fails silently.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use dioxus::prelude::*;

use super::camera::{Bounds, Camera, Flight, ZOOM_MIN, now};
use super::geometry;
use super::layout::{self, Placement, Slot};
use super::{EdgeState, Graph, NodeState, Style, Way};

/// The rectangle a placed node occupies on the pane.
fn rect(place: &Placement, node: (f32, f32)) -> (f32, f32, f32, f32) {
    (place.along, place.across, node.0, node.1)
}

/// The lattice steps the ground is drawn on, in world units. As the camera
/// pulls back the step climbs the ladder so the on-screen gap stays in a band
/// the eye can read — the ground never smears into a wash and never disappears,
/// which is what a canvas that drops its grid at the widest view does exactly
/// when orientation is scarcest.
const DOT_LADDER: [f32; 5] = [20.0, 40.0, 80.0, 160.0, 320.0];
/// The smallest on-screen gap a lattice is still a lattice at.
const DOT_FLOOR: f32 = 14.0;
/// A frame around one node's neighbourhood stops shrinking here: below it a node
/// is a grey box with grey marks in it, and a graph nobody can read is not a
/// frame. Framing *everything* has no floor — a control named for framing
/// everything has to actually frame everything.
const FRAME_FLOOR: f32 = 0.75;

/// A camera move a lens has asked for. Requested by node id, never by
/// coordinate: no lens knows where anything sits, and none should.
#[derive(Clone, PartialEq, Debug)]
enum Command {
    /// Frame the whole graph, at whatever magnification shows all of it.
    Fit,
    /// Frame these nodes. `focus` is the card the reader acted on, and it is
    /// pulled back into view whatever else does or does not fit — a move that
    /// pushes the card you just clicked off the edge has arrived nowhere.
    Frame {
        ids: Vec<usize>,
        focus: Option<usize>,
    },
    /// Frame a chain end to end. A route is one sentence and it is read whole
    /// or not at all, so this is the one move allowed to shrink until both ends
    /// are on screen — a short chain still lands at full size, because framing
    /// never magnifies past life size.
    Route { ids: Vec<usize> },
}

/// The lens's handle on the pane.
#[derive(Clone, Copy, PartialEq)]
pub struct FlowHandle {
    camera: Signal<Camera>,
    viewport: Signal<(f32, f32)>,
    flight: Signal<Option<Flight>>,
    command: Signal<Option<Command>>,
    /// Whether the reader has taken the camera. Until they do, the pane keeps
    /// the graph framed as it arrives; afterwards it never moves on its own.
    touched: Signal<bool>,
    /// The reader has asked their system for less motion.
    still: Signal<bool>,
}

pub fn use_flow() -> FlowHandle {
    FlowHandle {
        camera: use_signal(Camera::default),
        viewport: use_signal(|| (0.0f32, 0.0f32)),
        flight: use_signal(|| None),
        command: use_signal(|| None),
        touched: use_signal(|| false),
        still: use_signal(|| false),
    }
}

impl FlowHandle {
    /// Frame the whole graph.
    pub fn fit(&mut self) {
        self.command.set(Some(Command::Fit));
    }

    /// Frame a card and whatever it is attached to, keeping the card in view.
    pub fn frame(&mut self, focus: usize, ids: Vec<usize>) {
        if !ids.is_empty() {
            self.touched.set(true);
            self.command.set(Some(Command::Frame {
                ids,
                focus: Some(focus),
            }));
        }
    }

    /// Frame a chain that has just arrived: the whole route where it fits, read
    /// from the end it starts at, and never without the crate that was asked
    /// for. Framing only the destination is what left the chain running off the
    /// edge; framing only the chain is what left the destination off it.
    pub fn route(&mut self, ids: Vec<usize>) {
        if !ids.is_empty() {
            self.touched.set(true);
            self.command.set(Some(Command::Route { ids }));
        }
    }

    /// Magnification as a percentage, for the readout.
    pub fn percent(&self) -> u32 {
        ((self.camera)().scale() * 100.0).round() as u32
    }
}

/// What the last placement left behind: the row each card sat on, and which
/// cards were on the pane at all. The first is the seed that holds a reader's
/// place while a column grows around them; the second is how a card that has
/// just arrived knows to fade up rather than blink in.
///
/// `inputs` is the graph the next placement will be computed from. It lives
/// here rather than being a memo dependency because the memo keys on a cheap
/// signature instead: hashing a few thousand integers costs microseconds, and
/// comparing two `Vec`s of them on every render — of which there is one per
/// camera frame — did not.
#[derive(Default)]
struct Previously {
    rows: HashMap<usize, f32>,
    ids: HashSet<usize>,
    inputs: (Vec<Slot>, Vec<(usize, usize)>, Style),
    /// Which of the two settle animations the last drawing used, alternated on
    /// each new one so the browser restarts it — re-applying an animation name
    /// that is already set does nothing.
    parity: bool,
    /// The graph the cached drawing was computed from, and the drawing itself.
    ///
    /// The memo's body is not promised to run exactly once per change, and this
    /// pipeline is **not** idempotent: each run seeds the next from the rows it
    /// just produced, so running it twice on the same graph gives two different
    /// arrangements. Left ungated that cost a redundant layout of the whole
    /// workspace, flipped the settle parity twice so the animation never
    /// restarted, and reported that no card had arrived. One graph, one drawing,
    /// however many times anyone asks for it.
    shape: Option<u64>,
    last: Option<Drawn>,
}

/// Everything the layout decided, ready to render: where each card sits, which
/// of them are new, the path and label anchor for each wire, and whether this
/// drawing is an arrival or a re-tidy.
#[derive(Clone, PartialEq)]
struct Drawn {
    places: Vec<Placement>,
    arrived: HashSet<usize>,
    curves: HashMap<(usize, usize), (String, (f32, f32))>,
    tidy: Tidy,
}

/// Which drawing this is, which is what the stylesheet needs to know to move
/// the cards honestly.
///
/// Opening or folding a port re-runs the whole layout, so cards that stayed on
/// the pane are usually somewhere else afterwards. Teleporting them there breaks
/// the reader's hold on which card was which — but gliding them while the wires
/// snap is worse, because a wire's path is a different number of segments after
/// a re-tidy and cannot be interpolated, so the wires spend the glide hanging
/// off nothing.
///
/// So the cards glide and the wires wait: the stylesheet hides the wire layer
/// while the cards travel and fades it back in once they have landed, drawn at
/// the positions they arrived at. Both halves are the same length, and the two
/// named states exist only so the browser restarts the animation each time —
/// re-applying an identical animation name does nothing.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Tidy {
    /// Nothing was on the pane before. The cards arrive; nothing glides.
    #[default]
    First,
    Even,
    Odd,
}

impl Tidy {
    /// What the pane element carries, for the stylesheet to key on.
    fn attribute(self) -> &'static str {
        match self {
            Tidy::First => "first",
            Tidy::Even => "even",
            Tidy::Odd => "odd",
        }
    }
}

/// A cheap stand-in for "is this the same graph, drawn the same way".
fn signature(slots: &[Slot], pairs: &[(usize, usize)], style: &Style) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    slots.len().hash(&mut hasher);
    for slot in slots {
        slot.id.hash(&mut hasher);
        slot.column.hash(&mut hasher);
    }
    pairs.len().hash(&mut hasher);
    for pair in pairs {
        pair.hash(&mut hasher);
    }
    // Style is all floats and plain enums; its bits are what matter.
    format!("{style:?}").hash(&mut hasher);
    hasher.finish()
}

/// One message from the native gesture listener.
#[derive(serde::Deserialize)]
struct Gesture {
    t: String,
    a: f32,
    b: f32,
    c: f32,
}

/// Pointer, wheel and resize handling, installed on the pane itself.
///
/// Everything zooms about the point being aimed at rather than the viewport
/// centre, a wheel notch is told apart from a trackpad's continuous scroll by
/// delta mode and by the characteristic large quantised jump a wheel produces,
/// and a drag past four pixels swallows the click it would otherwise end with —
/// otherwise panning across the graph selects whatever you let go over.
const GESTURES: &str = r#"
(async () => {
  if (window.__flowCleanup) window.__flowCleanup();

  let pane = null;
  for (let i = 0; i < 240 && !pane; i++) {
    pane = document.getElementById('flow-pane');
    if (!pane) await new Promise(requestAnimationFrame);
  }
  if (!pane) return;

  const send = (t, a, b, c) => dioxus.send({ t, a, b: b || 0, c: c || 0 });
  const at = (event) => {
    const box = pane.getBoundingClientRect();
    return [event.clientX - box.left, event.clientY - box.top];
  };

  let dragging = false, travelled = 0, lastX = 0, lastY = 0, swallow = false;

  const down = (event) => {
    if (event.button !== 0 || event.target.closest('.flow-port, .flow-node')) return;
    dragging = true; travelled = 0; lastX = event.clientX; lastY = event.clientY;
    pane.dataset.panning = 'true';
  };
  const move = (event) => {
    if (!dragging) return;
    const dx = event.clientX - lastX, dy = event.clientY - lastY;
    lastX = event.clientX; lastY = event.clientY;
    travelled += Math.abs(dx) + Math.abs(dy);
    send('pan', dx, dy);
  };
  const up = () => {
    if (!dragging) return;
    dragging = false;
    delete pane.dataset.panning;
    // A gesture that travelled was a pan, and the click it ends with belongs
    // to the pan, not to whatever happens to sit under the pointer.
    if (travelled > 4) { swallow = true; setTimeout(() => { swallow = false; }, 0); }
  };
  const click = (event) => {
    if (swallow) { event.stopPropagation(); event.preventDefault(); }
  };

  // A wheel notch is a large, purely vertical, quantised jump. A trackpad is
  // small, usually carries some horizontal drift, and is rarely a round
  // multiple of 100.
  const isNotch = (event) => {
    if (event.deltaMode !== 0) return true;
    const dy = Math.abs(event.deltaY);
    return event.deltaX === 0 && dy >= 100 && (dy % 100 === 0 || dy % 120 === 0);
  };
  const wheel = (event) => {
    event.preventDefault();
    const [x, y] = at(event);
    if (event.ctrlKey || event.metaKey) {
      send('pinch', Math.exp(-event.deltaY * 0.01), x, y);
    } else if (isNotch(event)) {
      send('notch', event.deltaY < 0 ? 1 : -1, x, y);
    } else {
      send('pan', -event.deltaX, -event.deltaY);
    }
  };

  // Touch: one finger pans, two pinch about the point between them.
  let touches = null;
  const spread = (t) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);
  const touchStart = (event) => {
    if (event.touches.length === 2) { touches = spread(event.touches); dragging = false; }
  };
  const touchMove = (event) => {
    if (event.touches.length !== 2 || touches === null) return;
    event.preventDefault();
    const next = spread(event.touches);
    const box = pane.getBoundingClientRect();
    const mx = (event.touches[0].clientX + event.touches[1].clientX) / 2 - box.left;
    const my = (event.touches[0].clientY + event.touches[1].clientY) / 2 - box.top;
    if (touches > 0) send('pinch', next / touches, mx, my);
    touches = next;
  };
  const touchEnd = () => { touches = null; };

  const stillness = window.matchMedia('(prefers-reduced-motion: reduce)');
  const measure = () => send('size', pane.clientWidth, pane.clientHeight, stillness.matches ? 1 : 0);
  // A motion preference can change while the app is open, and it applies then
  // rather than at the next resize.
  stillness.addEventListener('change', measure);
  const observer = new ResizeObserver(measure);
  observer.observe(pane);
  measure();

  pane.addEventListener('pointerdown', down);
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', up);
  window.addEventListener('pointercancel', up);
  pane.addEventListener('click', click, true);
  pane.addEventListener('wheel', wheel, { passive: false });
  pane.addEventListener('touchstart', touchStart, { passive: true });
  pane.addEventListener('touchmove', touchMove, { passive: false });
  pane.addEventListener('touchend', touchEnd);

  window.__flowCleanup = () => {
    observer.disconnect();
    stillness.removeEventListener('change', measure);
    pane.removeEventListener('pointerdown', down);
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    window.removeEventListener('pointercancel', up);
    pane.removeEventListener('click', click, true);
    pane.removeEventListener('wheel', wheel);
    pane.removeEventListener('touchstart', touchStart);
    pane.removeEventListener('touchmove', touchMove);
    pane.removeEventListener('touchend', touchEnd);
    window.__flowCleanup = null;
  };
})();
"#;

/// A frame clock that runs for exactly one flight and then stops. The camera is
/// the only authored motion in the product, so this is the only loop.
const TICKER: &str = r#"
(() => {
  const start = performance.now();
  const step = (t) => {
    dioxus.send(t);
    if (t - start < 900) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
})();
"#;

#[component]
pub fn Flow(
    flow: FlowHandle,
    graph: Graph,
    /// Sizes, direction, wire shape and background. Defaults to [`Style::default`].
    #[props(default)]
    style: Style,
    /// Draw a node's contents yourself. The built-in card is used when this is
    /// absent; the card's title still names the node to assistive technology
    /// either way.
    node_view: Option<Callback<usize, Element>>,
    /// A node was clicked. Leave it out for a chart nobody selects.
    on_select: Option<EventHandler<usize>>,
    /// A port was clicked: open or fold what is attached this way. Leave it out
    /// and a port still states its count without offering to open.
    on_port: Option<EventHandler<(usize, Way)>>,
    /// The pane itself was clicked, which is how a reader lets go.
    on_clear: Option<EventHandler<()>>,
) -> Element {
    let mut camera = flow.camera;
    let mut viewport = flow.viewport;
    let mut flight = flow.flight;
    let mut command = flow.command;
    let mut touched = flow.touched;
    let mut still = flow.still;
    let node_size = style.node;
    let padding = style.padding;
    let background = style.background.class();

    // Placement is recomputed when the graph's *shape* changes and reused when
    // it does not, so selecting a node relights the graph without moving it.
    // The previous frame's rows are the seed, which is what holds a reader's
    // place while a column grows around them.
    let history: Rc<RefCell<Previously>> =
        use_hook(|| Rc::new(RefCell::new(Previously::default())));

    // Placement and the curves through it are one computation: the wires depend
    // on nothing but where the cards landed, so they are built in the same pass
    // and neither is rebuilt while the camera moves.
    // The memo's inputs are the graph's *shape* alone — which cards, in which
    // columns, joined to which. An edge's state is not in here on purpose: it
    // changes on every selection, and a selection must never cost a placement.
    let slots: Vec<Slot> = graph
        .nodes
        .iter()
        .map(|node| Slot {
            id: node.id,
            column: node.column,
        })
        .collect();
    let pairs: Vec<(usize, usize)> = graph
        .edges
        .iter()
        .map(|edge| (edge.from, edge.to))
        .collect();

    // The memo keys on the graph's *shape* and the style, and on nothing else.
    // An edge's state is deliberately absent: it changes on every selection, and
    // a selection must never cost a placement.
    let shape = signature(&slots, &pairs, &style);
    history.borrow_mut().inputs = (slots, pairs, style);

    let geometry = {
        let history = history.clone();
        use_memo(use_reactive!(|(shape,)| {
            let mut cell = history.borrow_mut();
            // One graph, one drawing. See the note on `Previously::shape`.
            if cell.shape == Some(shape)
                && let Some(done) = cell.last.clone()
            {
                return done;
            }
            let (slots, pairs, style) = std::mem::take(&mut cell.inputs);
            let metrics = style.metrics();
            let drawn = layout::layered(&slots, &pairs, &cell.rows, &metrics);
            cell.inputs = (slots, pairs, style);
            let places: Vec<Placement> = drawn
                .places
                .iter()
                .map(|place| {
                    let (x, y) = style.place(place.along, place.across);
                    Placement {
                        id: place.id,
                        along: x,
                        across: y,
                    }
                })
                .collect();
            let arrived: HashSet<usize> = places
                .iter()
                .map(|place| place.id)
                .filter(|id| !cell.ids.contains(id))
                .collect();
            // A pane that was empty is arriving, not re-tidying: the cards have
            // nowhere to travel from, and holding them back to wait for a glide
            // that cannot happen is just a slower first paint.
            let tidy = if cell.ids.is_empty() {
                Tidy::First
            } else {
                cell.parity = !cell.parity;
                if cell.parity { Tidy::Even } else { Tidy::Odd }
            };
            // The seed is the card's centre, which is what the next layout's
            // ordering and relaxation both work in.
            cell.rows = drawn
                .places
                .iter()
                .map(|place| (place.id, place.across + metrics.across / 2.0))
                .collect();
            cell.ids = places.iter().map(|place| place.id).collect();
            drop(cell);

            // Each wire is drawn through the lane the layout opened for it, so
            // a run that crosses a column goes round the cards there rather
            // than over them.
            let axis = style.axis();
            let curves: HashMap<(usize, usize), (String, (f32, f32))> = drawn
                .wires
                .iter()
                .map(|wire| {
                    let points: Vec<(f32, f32)> = wire
                        .points
                        .iter()
                        .map(|&(along, across)| style.place(along, across))
                        .collect();
                    let middle = geometry::midpoint(&points);
                    (
                        (wire.from, wire.to),
                        (geometry::wire(&points, style.shape, axis), middle),
                    )
                })
                .collect();

            let done = Drawn {
                places,
                arrived,
                curves,
                tidy,
            };
            let mut cell = history.borrow_mut();
            cell.shape = Some(shape);
            cell.last = Some(done.clone());
            done
        }))
    };

    // A lens asks for a camera move by node id; this is where it becomes a
    // flight. Framing a set rather than a point is what stops a move landing on
    // a node with everything it is attached to outside the frame.
    use_effect(move || {
        let Some(request) = command() else {
            return;
        };
        command.set(None);
        let size = viewport();
        if size.0 < 1.0 {
            return;
        }
        let snapshot = geometry.read();
        let places = &snapshot.places;
        let wanted: Vec<(f32, f32, f32, f32)> = match &request {
            Command::Fit => places.iter().map(|place| rect(place, node_size)).collect(),
            Command::Frame { ids, .. } | Command::Route { ids } => places
                .iter()
                .filter(|place| ids.contains(&place.id))
                .map(|place| rect(place, node_size))
                .collect(),
        };
        let Some(bounds) = Bounds::of(wanted) else {
            return;
        };
        let framed = match &request {
            Command::Fit | Command::Route { .. } => {
                Camera::framing(bounds, size, padding, ZOOM_MIN, false)
            }
            Command::Frame { .. } => Camera::framing(bounds, size, padding, FRAME_FLOOR, false),
        };
        // The frame is of what was revealed; the card that revealed it is then
        // pulled back into view if the frame left it outside. Centring on the
        // focus card alone hid the answer the reader had just asked for;
        // centring on the bounds alone pushed the question off the edge.
        let mut target = framed;
        if let Command::Frame {
            focus: Some(id), ..
        } = &request
            && let Some(place) = places.iter().find(|place| place.id == *id)
        {
            let scale = target.scale();
            let (left, top) = (
                place.along * scale + target.x,
                place.across * scale + target.y,
            );
            let (right, bottom) = (left + node_size.0 * scale, top + node_size.1 * scale);
            let pad = padding.min(size.0 / 4.0).min(size.1 / 4.0);
            if left < pad {
                target.x += pad - left;
            } else if right > size.0 - pad {
                target.x -= right - (size.0 - pad);
            }
            if top < pad {
                target.y += pad - top;
            } else if bottom > size.1 - pad {
                target.y -= bottom - (size.1 - pad);
            }
        }
        let from = camera();
        if (from.x - target.x).abs() < 0.5
            && (from.y - target.y).abs() < 0.5
            && (from.scale() - target.scale()).abs() < 0.001
        {
            return;
        }
        if still() {
            camera.set(target);
        } else {
            flight.set(Some(Flight::new(&from, &target, size, now())));
        }
    });

    // One frame clock per flight, started when the flight is and stopped by the
    // script itself. Ticks arrive inside the runtime, so the camera write is a
    // real write.
    use_effect(move || {
        if flight().is_none() {
            return;
        }
        spawn(async move {
            let mut clock = document::eval(TICKER);
            while let Ok(time) = clock.recv::<f64>().await {
                let Some(current) = flight.peek().as_ref().copied() else {
                    break;
                };
                camera.set(current.at(time));
                if current.landed(time) {
                    flight.set(None);
                    break;
                }
            }
        });
    });

    // Gestures, and the pane's own size.
    use_future(move || async move {
        let mut channel = document::eval(GESTURES);
        while let Ok(gesture) = channel.recv::<Gesture>().await {
            match gesture.t.as_str() {
                "size" => {
                    if gesture.a > 0.0 {
                        viewport.set((gesture.a, gesture.b));
                    }
                    still.set(gesture.c > 0.5);
                }
                // Any gesture is the reader taking the camera. From here on the
                // pane never moves the view on its own.
                "pan" => {
                    flight.set(None);
                    touched.set(true);
                    camera.write().pan(gesture.a, gesture.b);
                }
                "notch" => {
                    flight.set(None);
                    touched.set(true);
                    camera.write().zoom_by(gesture.a, (gesture.b, gesture.c));
                }
                "pinch" => {
                    flight.set(None);
                    touched.set(true);
                    camera
                        .write()
                        .zoom_by_ratio(gesture.a, (gesture.b, gesture.c));
                }
                _ => {}
            }
        }
    });

    // Keep the graph framed while it is still arriving. A graph loads in two
    // steps — the seed, then what it opens onto — so framing once, on the first
    // non-empty placement, frames a single card and then watches the rest of the
    // graph land outside the viewport. This holds the frame until the reader
    // takes the camera, and never moves it again after that.
    let mut opened = use_signal(|| 0usize);
    use_effect(move || {
        let size = viewport();
        let ids: Vec<usize> = geometry.read().places.iter().map(|place| place.id).collect();
        if size.0 < 1.0 || ids.is_empty() || touched() || ids.len() == opened() {
            return;
        }
        opened.set(ids.len());
        command.set(Some(Command::Fit));
    });

    let view = camera();
    let scale = view.scale();
    let step = DOT_LADDER
        .iter()
        .copied()
        .find(|step| step * scale >= DOT_FLOOR)
        .unwrap_or(DOT_LADDER[DOT_LADDER.len() - 1]);
    let gap = step * scale;
    let lattice = format!(
        "background-size: {gap:.2}px {gap:.2}px; background-position: {:.2}px {:.2}px;",
        view.x, view.y
    );

    let snapshot = geometry.read();
    let (places, arrived, curves, tidy) =
        (&snapshot.places, &snapshot.arrived, &snapshot.curves, snapshot.tidy);
    let at: HashMap<usize, Placement> = places.iter().map(|place| (place.id, *place)).collect();

    // Lit edges are painted over the resting mesh, never under it — an answer
    // half-hidden behind the graph it belongs to is not an answer.
    // Cards are rendered in reading order — column, then row — rather than in
    // whatever order the lens produced them. Nothing on screen moves, because
    // position is a transform; what changes is the order a keyboard walks them
    // in, which becomes the same left-to-right order the graph's own law reads
    // in rather than an arbitrary one.
    let mut ordered: Vec<(&super::Node, &Placement)> = graph
        .nodes
        .iter()
        .filter_map(|node| at.get(&node.id).map(|place| (node, place)))
        .collect();
    ordered.sort_by(|a, b| {
        a.1.along
            .partial_cmp(&b.1.along)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.1.across
                    .partial_cmp(&b.1.across)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    let mut wired: Vec<(&super::Edge, &String, (f32, f32))> = graph
        .edges
        .iter()
        .filter_map(|edge| {
            curves
                .get(&(edge.from, edge.to))
                .map(|(d, middle)| (edge, d, *middle))
        })
        .collect();
    wired.sort_by_key(|(edge, _, _)| edge.state.layer());
    let marks: Vec<Mark> = graph
        .nodes
        .iter()
        .filter_map(|node| {
            at.get(&node.id).map(|place| Mark {
                x: place.along,
                y: place.across,
                filled: node.card.filled,
                held: node.state == NodeState::Held,
            })
        })
        .collect();

    rsx! {
        div {
            class: "flow-root",
            "data-direction": match style.direction {
                super::Direction::LeftRight => "left-right",
                super::Direction::TopBottom => "top-bottom",
            },

            div {
                id: "flow-pane",
                class: "{background} flow-pane--fill",
                style: "{lattice}",
                // The cards glide to their new places and the wires wait for
                // them; see `Tidy`.
                "data-tidy": tidy.attribute(),

                // Clicking the pane lets go. It sits under the transformed
                // layer, so a click that reaches it is a click that missed
                // every node.
                div {
                    class: "flow-backdrop",
                    onclick: move |_| {
                        if let Some(handler) = &on_clear {
                            handler.call(());
                        }
                    },
                }

                div { class: "flow-viewport", style: "transform: {view.transform()};",

                    svg {
                        class: "flow-wires",
                        width: "1",
                        height: "1",
                        defs {
                            Arrow { name: "wire".to_string(), colour: "var(--flow-wire)".to_string() }
                            Arrow { name: "muted".to_string(), colour: "var(--flow-wire-muted)".to_string() }
                            Arrow { name: "in".to_string(), colour: "var(--flow-in)".to_string() }
                            Arrow { name: "out".to_string(), colour: "var(--flow-out)".to_string() }
                            Arrow { name: "route".to_string(), colour: "var(--flow-in-lit)".to_string() }
                        }
                        for (edge , d , _) in wired.iter() {
                            path {
                                key: "{edge.from}-{edge.to}",
                                class: "flow-edge",
                                "data-way": edge.state.as_str(),
                                "data-route": edge.state == EdgeState::Route,
                                d: "{d}",
                                "marker-end": match edge.state {
                                    EdgeState::Incoming => "url(#arrow-in)",
                                    EdgeState::Outgoing => "url(#arrow-out)",
                                    EdgeState::Route => "url(#arrow-route)",
                                    EdgeState::Muted => "url(#arrow-muted)",
                                    EdgeState::Rest => "url(#arrow-wire)",
                                },
                            }
                        }
                        // A label rides on the wire's own middle, knocked out of
                        // the ground so it stays readable over whatever it
                        // crosses.
                        for (edge , _ , middle) in wired.iter() {
                            if let Some(label) = &edge.label {
                                text {
                                    key: "label-{edge.from}-{edge.to}",
                                    class: "flow-label",
                                    x: "{middle.0}",
                                    y: "{middle.1}",
                                    "{label}"
                                }
                            }
                        }
                    }

                    for (node , place) in ordered.iter() {
                        div {
                            key: "{node.id}",
                            class: "flow-node-wrap",
                            style: "transform: translate({place.along}px, {place.across}px); width: {node_size.0}px; height: {node_size.1}px;",

                            button {
                                class: "flow-node",
                                "data-filled": node.card.filled,
                                "data-held": node.state == NodeState::Held,
                                "data-onroute": node.state == NodeState::OnRoute,
                                "data-fresh": arrived.contains(&node.id),
                                "aria-current": if node.state == NodeState::Held { "true" } else { "false" },
                                onclick: {
                                    let id = node.id;
                                    move |_| {
                                        if let Some(handler) = &on_select {
                                            handler.call(id);
                                        }
                                    }
                                },
                                match node_view {
                                    Some(view) => view.call(node.id),
                                    None => rsx! {
                                        div { class: "flow-node__body",
                                            div { class: "flow-node__line",
                                                span { class: "flow-node__title", "{node.card.title}" }
                                                if let Some(badge) = &node.card.badge {
                                                    span {
                                                        class: "{badge.tone.class()}",
                                                        title: "{badge.title}",
                                                        "{badge.label}"
                                                    }
                                                }
                                            }
                                            span { class: "flow-node__sub", "{node.card.subtitle}" }
                                        }
                                    },
                                }
                            }

                            if let Some(port) = node.inbound {
                                Knob {
                                    id: node.id,
                                    way: Way::In,
                                    count: port.count,
                                    open: port.open,
                                    name: node.card.title.clone(),
                                    on_port,
                                }
                            }
                            if let Some(port) = node.outbound {
                                Knob {
                                    id: node.id,
                                    way: Way::Out,
                                    count: port.count,
                                    open: port.open,
                                    name: node.card.title.clone(),
                                    on_port,
                                }
                            }
                        }
                    }
                }
            }

            MiniMap { flow, marks, node: node_size }
            Controls { flow }
        }
    }
}

/// A port: the count of what is attached this way, and the control that opens
/// it. Nothing is hidden by folding — the number is on the card either way.
#[component]
fn Knob(
    id: usize,
    way: Way,
    count: usize,
    open: bool,
    name: String,
    on_port: Option<EventHandler<(usize, Way)>>,
) -> Element {
    let verb = if open { "Fold" } else { "Open" };
    let what = match way {
    Way::In => "what depends on",
    Way::Out => "what is depended on by",
    };
    rsx! {
    button {
        class: "flow-port",
        "data-way": way.as_str(),
        "data-open": open,
        title: "{verb} {what} {name} ({count})",
        "aria-label": "{verb} {what} {name}, {count}",
        "aria-expanded": open,
        onclick: move |event| {
            event.stop_propagation();
            if let Some(handler) = &on_port {
                handler.call((id, way));
            }
        },
        "{count}"
    }
}
}

#[component]
fn Arrow(name: String, colour: String) -> Element {
rsx! {
    marker {
        id: "arrow-{name}",
        "markerWidth": "9",
        "markerHeight": "9",
        "refX": "8",
        "refY": "4.5",
        "orient": "auto-start-reverse",
        "markerUnits": "userSpaceOnUse",
        path { d: "M1,1.5 L8,4.5 L1,7.5 z", fill: colour }
    }
}
}

/// Zoom, and the way back to the whole graph. Bottom-left, where every canvas
/// has kept them.
#[component]
fn Controls(flow: FlowHandle) -> Element {
    let mut handle = flow;
    let mut camera = flow.camera;
    let viewport = flow.viewport;
    let mut flight = flow.flight;
    let mut touched = flow.touched;
    let view = camera();

    rsx! {
        div {
            class: "flow-plate flow-controls",
            "aria-label": "View controls",
            button {
                class: "flow-ctrl",
                "aria-label": "Zoom out",
                // A control that cannot do anything says so, rather than
                // staying live and quietly refusing.
                disabled: view.at_limit(false),
                onclick: move |_| {
                    let (w, h) = viewport();
                    flight.set(None);
                    touched.set(true);
                    camera.write().zoom_by(-1.0, (w / 2.0, h / 2.0));
                },
                svg {
                    class: "flow-icon",
                    view_box: "0 0 16 16",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    path { d: "M3.5 8 H12.5" }
                }
            }
            button {
                class: "flow-ctrl flow-ctrl--readout",
                title: "Reset to actual size",
                onclick: move |_| {
                    let (w, h) = viewport();
                    flight.set(None);
                    touched.set(true);
                    let mut view = camera.write();
                    let notches = -view.exp;
                    view.zoom_by(notches, (w / 2.0, h / 2.0));
                },
                "{flow.percent()}%"
            }
            button {
                class: "flow-ctrl",
                "aria-label": "Zoom in",
                disabled: view.at_limit(true),
                onclick: move |_| {
                    let (w, h) = viewport();
                    flight.set(None);
                    touched.set(true);
                    camera.write().zoom_by(1.0, (w / 2.0, h / 2.0));
                },
                svg {
                    class: "flow-icon",
                    view_box: "0 0 16 16",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    path { d: "M8 3.5 V12.5" }
                    path { d: "M3.5 8 H12.5" }
                }
            }
            button {
                class: "flow-ctrl flow-ctrl--last",
                "aria-label": "Frame everything on the pane",
                title: "Frame everything",
                onclick: move |_| handle.fit(),
                svg {
                    class: "flow-icon",
                    view_box: "0 0 16 16",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    path { d: "M2.75 6 V2.75 H6" }
                    path { d: "M10 2.75 H13.25 V6" }
                    path { d: "M13.25 10 V13.25 H10" }
                    path { d: "M6 13.25 H2.75 V10" }
                }
            }
        }
}
}

/// One node on the minimap. The map carries position and two facts and nothing
/// else — a map that tries to be legible is a second graph.
#[derive(Clone, Copy, PartialEq)]
struct Mark {
    x: f32,
    y: f32,
    filled: bool,
    held: bool,
}

impl Mark {
    /// The map carries position and two facts. A map that tries to be legible
    /// is a second graph.
    fn ink(self) -> &'static str {
        if self.held {
            "var(--flow-in)"
        } else if self.filled {
            "var(--flow-ink)"
        } else {
            "var(--flow-border)"
        }
    }
}

const MAP_W: f32 = 148.0;
const MAP_H: f32 = 94.0;

/// The graph at a glance, and where the camera is standing in it. Clicking
/// moves the camera there.
#[component]
fn MiniMap(flow: FlowHandle, marks: Vec<Mark>, node: (f32, f32)) -> Element {
    let mut camera = flow.camera;
    let mut flight = flow.flight;
    let mut touched = flow.touched;

    let view = (flow.camera)();
    let size = (flow.viewport)();
    if size.0 < 1.0 || marks.is_empty() {
        return rsx! {};
}

// Where the camera is standing, in world units.
let (top_left, bottom_right) = (view.to_world(0.0, 0.0), view.to_world(size.0, size.1));
let seen = Bounds {
    min_x: top_left.0,
    min_y: top_left.1,
    max_x: bottom_right.0,
    max_y: bottom_right.1,
};

// The map is of the graph, not of wherever the camera has wandered to.
// Unioning the two makes the map mostly empty pane the moment the reader
// zooms out, which is exactly when they need it.
let graph_box = Bounds::of(marks.iter().map(|mark| (mark.x, mark.y, node.0, node.1)))
    .unwrap_or(seen);
let world = letterbox(graph_box, MAP_W / MAP_H, node.1);
// One world unit is this many map pixels; strokes divide by it so a hairline
// stays a hairline however far out the map is showing.
let unit = (world.width() / MAP_W).max(0.0001);

rsx! {
    div {
        class: "flow-plate flow-map",
        svg {
            class: "flow-map__canvas",
            width: "{MAP_W}",
            height: "{MAP_H}",
            "viewBox": "{world.min_x} {world.min_y} {world.width()} {world.height()}",
            role: "img",
            "aria-label": "Map of the graph. Click to move the view.",
            onclick: move |event| {
                let point = event.element_coordinates();
                let wx = world.min_x + (point.x as f32 / MAP_W) * world.width();
                let wy = world.min_y + (point.y as f32 / MAP_H) * world.height();
                flight.set(None);
                touched.set(true);
                camera.set(Camera::looking_at((wx, wy), view.scale(), size));
            },
            rect {
                x: "{seen.min_x}",
                y: "{seen.min_y}",
                width: "{seen.width()}",
                height: "{seen.height()}",
                fill: "var(--color-canvas)",
                stroke: "var(--color-line-strong)",
                stroke_width: "{1.5 * unit}",
                rx: "{4.0 * unit}",
            }
            for (index , mark) in marks.iter().enumerate() {
                rect {
                    key: "{index}",
                    x: "{mark.x}",
                    y: "{mark.y}",
                    width: "{node.0}",
                    height: "{node.1}",
                    rx: "{8.0 * unit}",
                    fill: mark.ink(),
                }
            }
        }
    }
}
}

/// Grow a world rectangle to a given aspect ratio, so mapping a click back into
/// it is a plain fraction rather than an unpicking of `preserveAspectRatio`.
fn letterbox(bounds: Bounds, aspect: f32, floor: f32) -> Bounds {
    let pad = (bounds.width().max(bounds.height()) * 0.08).max(floor);
    let (mut w, mut h) = (bounds.width() + pad * 2.0, bounds.height() + pad * 2.0);
    if w / h < aspect {
        w = h * aspect;
    } else {
        h = w / aspect;
    }
    let (cx, cy) = bounds.centre();
    Bounds {
        min_x: cx - w / 2.0,
        min_y: cy - h / 2.0,
        max_x: cx + w / 2.0,
        max_y: cy + h / 2.0,
    }
}
