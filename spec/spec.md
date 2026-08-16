we're building a rust viewer.

the backend reads a cargo manifest, reads the dependencies and build a dependency graph.

## Frontend

Frontend is ui heavy.
The first view is dependency graph viewer.
For every crate, it shows what crates it depends on and what crates depend on it.

---

# Built

## Entry

`rust-viewer /path/to/workspace` resolves that workspace, then serves the web UI.
With no argument it reads the current directory. The path is server state; there
is no picker and no upload flow.

## The idea

The workspace is drawn as a bare two-layer board. Every crate is an identical
gold pad on a lattice; every dependency is a length of routed copper between two
pads. Nothing about a crate — how many things need it, how deep it sits, whether
you wrote it — changes the pad, because what a crate *is* lives entirely in the
copper running into and out of it.

Three rules hold the whole design up:

1. **One continuous space.** Zoom is the only navigation verb. There is no
   overview mode and no focus mode to cut between; there is one board, read at
   three distances.
2. **Uniform objects.** Every pad is the same object. All meaning is in the
   wiring and the arrangement.
3. **The camera moves and the world does not.** Layout is computed once,
   server-side. Holding a crate flies the camera and changes which copper is lit.
   It never re-seats a pad. A world that rebuilds itself under you is a world you
   cannot learn.

## Backend

`cargo metadata --format-version 1` does the resolving, so feature unification
and platform resolution are cargo's answer rather than ours. Dev-dependencies
are dropped: they are in nobody's build, and they are what puts cycles in an
otherwise acyclic graph. Whatever the workspace no longer reaches after that is
pruned.

Layout runs server-side, once; the client receives finished coordinates and
computes none of its own. `src/graph/layout.rs`:

1. **Rank** by longest path from the workspace members. This is the law of the
   board: everything a crate depends on has a strictly greater column, so
   dependencies are always to the right and dependents always to the left, at
   every zoom, without exception.
2. **Layer**, inserting routing channels for traces that span more than one
   column so copper never cuts through a pad column.
3. **Reduce crossings** with barycentre sweeps.
4. **Relax** the coordinates toward each node's neighbours so a trace that could
   be straight is straight, then **seat everything on the lattice**: pads land on
   exact multiples of a 36-unit pitch and never share a row, channels on a 2-unit
   sub-pitch. Every column is centred, otherwise the relaxation bends the board
   into a wedge.
5. **Route** every dependency — all of them — as a polyline of horizontal,
   vertical, and 45-degree segments. A right angle in copper is an etchant trap
   on a real board, and the chamfer is also what keeps a run traceable by eye.

On this workspace that is 376 packages, 346 crates, 1174 traces, 22 columns, 46
pads in the widest column, and a board 7250 × 4876 units — landscape, which is
what the channel pitch is tuned to hold.

## Frontend

A board, drawn to a `<canvas>` (`src/views/canvas.rs`). Both directions are first
class, and direction is the one variable that earns a hue:

- **Gold** copper runs into the held pad: what depends on it. It always enters
  from the left.
- **Blue** copper runs out of it: what it depends on. It always leaves to the
  right.
- **Etched copper** is every other dependency, always drawn. Density is the
  material a backplane is made of; the previous design drew one edge per crate at
  weight and the other ~800 at 18% alpha, which meant most of the real dependency
  relationships in the workspace were rendered as noise.

Everything else about a crate is said in silkscreen beside its pad rather than by
changing the pad: a component outline for a crate this workspace builds, a coral
flag for a name that resolves at more than one version.

**Zoom is level of detail**, not scale. Three tiers, and each shows something the
others do not: *Board* names only what the workspace routes through; *Component*
resolves pads and fills in the legend; *Pad* draws drill holes and the substrate
lattice the parts are seated on. Pad radius is capped against the on-screen
lattice gap, because holding a marker at full size while the board shrinks is
what packs a dense column into a solid bar of gold.

A **depth ruler** across the top numbers the columns and names what they count,
so hop distance is a literal scale rather than something inferred from x
position — the encoding that collapses when a layout is squeezed.

## Navigation

Every gesture zooms or pans about the point you are aiming at, never about the
viewport centre — centre-anchored zoom throws away whatever you were looking at.

- **Pinch** (trackpad or touchscreen) zooms, continuously.
- **Wheel** zooms one notch at a time.
- **Two-finger scroll** and **drag** pan.

A trackpad pinch reaches the page as a wheel event with `ctrlKey` set, which is
the same signal the browser binds its own page zoom to; the listener is attached
natively to the canvas with `passive: false` so it can `preventDefault()`, or
pinching scales the whole browser page instead of the board. Wheel notches are
told apart from a trackpad's continuous scroll by delta mode and by the
characteristic large, purely-vertical, quantised jump a wheel produces.

Zoom is held as a float, so a pinch can land between notches; a notch still adds
exactly ±1.0, which is exact in binary floating point, so wheel zoom stays
perfectly reversible — verified by driving the real browser and confirming the
canvas returns to an identical pixel signature.

Drag travel past 4px means the gesture was a pan and the pad under it is not
held. A second finger cancels a drag and becomes a pinch. Clicking bare mask
lets go.

Holding a crate **flies the camera** to it — the only authored motion in the app,
on a 560ms exponential ease-out, position interpolating linearly and magnification
geometrically. The flight frames the crate's own attachments (the 80th percentile
of the distances to its direct dependents and dependencies) rather than a fixed
magnification: landing on the pad with 25 of its 27 dependents outside the frame
is a camera move that arrives nowhere.

`←`/`↑` steps to what depends on the held crate, `→`/`↓` to what it depends on,
both taking the busiest first. `Backspace` retraces the walk, `/` jumps to the
finder, `Escape` lets go. The record panel's lists are the precise path when the
busiest is not the one you want.

## The record

`src/views/record.rs` docks a panel that states the answer in words, because
`cargo tree -i` answers the same question in 400ms of copyable text and a picture
that has to be traced by eye has not answered it at all.

Every record carries the same fields in the same order — learn one and you can
read all 346: what it is (name, version, reference designator), why it is here
(the chain from a workspace member, each step clickable), what depends on it, and
what it depends on, each list with versions, dependent counts, and duplicate
badges.

Counts are reported in buckets that partition their headline exactly: "64 · 27
directly · 37 further out". Totals come from the reachability closure, never from
hop levels — levels are longest-path, so a crate that is both an immediate
dependent *and* reachable by a longer route would land in two buckets at once.
That is how an earlier build reported 41 dependents and then listed 23 + 23.

When nothing is held the panel is the board's key: what the marks mean and how to
move. The tool invents a small vocabulary and now documents all of it.

## Renderer

Canvas costs three things SVG gave away, and each is paid for explicitly:
hit-testing is a quadtree over pad positions (`src/graph/quadtree.rs`), keyboard
access is a real focusable list of every crate beside the canvas, and animation is
a frame loop — which is also what lets panning and zooming skip the virtual DOM
entirely. Not WebGL: the bottleneck was element count and diffing, not raster.

Traces are culled by their own routed bounding box rather than by their two pads,
since a routed trace detours through channels outside its endpoints.

## Shell

`src/views/shell.rs` is the lens frame, not the board's chrome. Dependencies is
the first lens. Later lenses mount as siblings; they are not advertised in the
title block until they exist, because a permanently disabled tab spends
credibility on first run for nothing.

## Theme

Two renditions, and neither is a switch bolted onto the other. Dark is the bare
board. Light is the same board as a **fabrication drawing** — black line on white
paper, which is what the world looks like before it is manufactured. Canvas
colours are read from the same CSS custom properties as the chrome at draw time,
and the frame loop compares the resolved palette every frame, because a system
theme change marks nothing dirty and the board would otherwise sit in board ink
under drawing-paper chrome.

## Tests

Tests build the board for whichever workspace they run in — a real graph, not a
fixture. `src/graph/layout.rs` asserts every edge points forward, every pad is
seated on the lattice and in its rank's column, no two pads in a column share a
row, no column drifts from centre, every trace segment sits at 0/45/90 degrees,
and — the rule the previous design broke — that **every declared dependency is
routed**. `src/graph/focus.rs` asserts direction agrees with where a pad sits,
levels are real hop counts, depth bounds what is lit without changing what is
counted, direct and further-out partition the total exactly, the "why it's here"
path is a genuine chain starting at a workspace member, and the direct lists match
the crate itself. `src/graph/quadtree.rs` is checked against a linear scan over a
deliberately clustered point set.

## Not built

The semantic engine behind the later lenses is undecided; nothing here assumes
one. Analysis stays inside workspace crates. See `PRODUCT.md`.

The visual system is recorded in `DESIGN.md`; the surface's strategy lives in
`.impeccable/surfaces/src-views-canvas-rs.md`.
