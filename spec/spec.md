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

The workspace is a **flow chart of the build**, opened three hops deep: the
crates this workspace builds, what they pulled in, and what that pulled in, laid
out as one DAG from the first frame. Cards are crates, wires are dependencies,
and every card carries two numbered ports — how many crates depend on it, how
many it depends on — which are also the controls that fold a side away or open it
again.

Three hops rather than all of them, because past there a build is transitive
scenery rather than a shape anyone came to read, and because a large workspace
resolves to thousands of cards that all have to be mounted. On a 700-crate build
the rim cuts the pane from 700 cards to 479 and the drawing from 10,405 elements
to 7,929.

Three rules hold it up:

1. **Left points at right.** A card's column is its longest-path distance from
   the workspace, so everything a crate depends on is strictly to its right and
   everything that depends on it strictly to its left, at every zoom, without
   exception. Which side a port sits on, which way an arrow key travels and
   which hue an edge takes are all downstream of that.
2. **The graph is what you opened.** Nothing past the rim is lost, only folded:
   every card is *opened* rather than merely present, at every depth, so folding
   a port takes away whatever was only reachable through it and opening one at
   the rim brings the next hop in. The count is on the port either way.
3. **Selection changes ink, never the cast** — and never the camera. Holding a
   crate relights the wires and fills the record; it does not add, remove or move
   a card, and it does not take the ground with it. The reader aimed at that card
   in that spot. Naming one is the other case: from the finder, a row in the
   record or an arrow key there is no telling whether it is even on screen, so
   the camera goes, and a crate that is not on the pane at all arrives with the
   chain that put it in the build.

Opening or folding re-tidies the whole pane, and that is drawn rather than
jumped: the cards glide to their new places over 260ms while the wire layer is
held back, then the wires fade in where the cards landed. The wires cannot travel
with them — a re-tidy changes how many segments a path has, and a path whose
command count changes cannot be interpolated — so hiding them for the length of
the glide is what stops them hanging off nothing.

## Backend

`cargo metadata --format-version 1` does the resolving, so feature unification
and platform resolution are cargo's answer rather than ours. Dev-dependencies
are dropped: they are in nobody's build, and they are what puts cycles in an
otherwise acyclic graph. Whatever the workspace no longer reaches is pruned.

`src/graph/build.rs` ranks by longest path from the workspace members and
inverts every dependency list into a dependents list. That is the whole payload:
**no coordinates cross the wire**. Where a card sits depends on what the reader
has opened, and that is the client's question.

A shortest-path rank would put `serde` one column out because something depends
on it directly, even where twenty other routes reach it four hops later. Longest
path is what makes a column a real depth.

On this workspace: 376 packages, 346 distinct crates, 1175 dependencies, 22
columns, 46 crates in the widest one, 26 names resolving at more than one
version.

## The flow framework

`crates/dioxus-flow/` is the surface both lenses draw on — a **separate crate**,
not a module of this app, because a graph pane is not a thing about Rust
workspaces. It knows nothing about crates, functions or cargo: a lens hands it
nodes with a column each and the wires between them, and it owns the camera, the
layout, the geometry, the ports, the controls and the map. It carries its own
stylesheet, written against `--flow-*` custom properties, and this product
supplies one block mapping its palette onto them.

Its own options — node size, column pitch, left-to-right or top-to-bottom,
bezier/straight/stepped wires, dot/line/blank ground, edge labels, and drawing a
node's contents yourself — are exercised by `cargo run --example showcase`
rather than by this app, which uses the defaults.

- **`camera`** — a translation and a magnification written straight into one
  CSS transform. Magnification is held as an *exponent* of 1.2, so a wheel notch
  adds exactly ±1.0 and a notch out and back returns to the same pixels, while a
  pinch still lands anywhere between notches. Flights interpolate position
  linearly and magnification geometrically.
- **`layout`** — the layered pipeline, a plain function over plain data with no
  Dioxus and no DOM in it, re-run every time the reader opens or
  folds a card, which is what keeps the drawing a readable DAG rather than a
  first arrangement that degrades. Three stages, each because the one before it
  is not enough:
  - **Lanes.** A wire spanning more than one column gets a waypoint in every
    column it crosses. Without them a long wire is drawn straight over whatever
    cards are in the way and no amount of reordering helps, because the wire is
    not *in* those columns for anything to move aside from.
  - **Ordering.** Median sweeps, then a transpose pass that swaps adjacent pairs
    while that removes crossings. Sweeps alone plateau immediately on a freshly
    opened fan — every child of one card has the same median, so a sweep has no
    opinion about their order at all.
  - **Coordinates.** Every cell asks to sit level with the median of what it is
    attached to, and the column grants those requests in rank order: lanes
    outrank every card, and among cards the busiest wins. That priority is the
    difference between a long wire drawn as a straight run and the same wire
    drawn as a sweep across the whole picture.
  Columns are compacted onto a pitch, so a graph occupying ranks 0, 4 and 9
  draws as three adjacent columns. The previous frame seeds the ordering, so a
  re-tidy moves what the topology forces and not what it does not — measured:
  seeding never costs the drawing crossings against laying the same graph out
  cold.

  What it costs, on release builds: 4.4ms for 377 cards and 1179 wires, 28ms for
  700 cards and 2643 wires. The larger figure is paid for 24,000 lane waypoints
  rather than for 700 cards, since a wire crossing eight columns is ordered in
  every one of them. Three decisions carry that: crossings are counted by
  inversion count over a Fenwick tree, `O(E log E)` rather than the textbook
  `O(E²)`; ordering stops as soon as two rounds fail to improve, which costs
  under a tenth of a percent in crossings and halves the wait; and the pane
  caches its drawing against the graph's signature, because the pipeline is
  deliberately *not* idempotent — it seeds each run from the rows the last one
  produced — so an ungated second evaluation both doubled the work and produced a
  different arrangement.
- **`geometry`** — a wire leaves and arrives horizontally, always. Two points
  is a cubic with control distance half the horizontal gap, falling back to a
  square-root bulge when it runs backwards, which keeps a recursive call legible
  as a loop instead of a line through both cards. More points is a wire the
  layout routed, drawn as a chain of cubics with shared tangents so the run has
  no corner in it.
- **`fold`** — which nodes are on the pane, given the whole graph and what the
  reader has opened. A `Folding` holds intent — the seeds and which ports are
  open — and derives the rest by walking, never storing a set of visible ids: a
  flat set cannot answer *what does folding this take away*, since nothing in it
  records which nodes were reachable only through the one you folded. `Links` is
  a trait so a host's own structure is walked in place; this product implements
  it on `Workspace` rather than copying 700 crates into fresh vectors per render.
- **`pane`** — the DOM shape every flow canvas converged on: absolutely
  positioned cards inside a single transformed layer, so pan and zoom write
  **one** transform string rather than touching a hundred nodes, and the dot
  lattice is a background image on the pane itself.

## Navigation

Every gesture zooms or pans about the point being aimed at, never the viewport
centre — centre-anchored zoom throws away whatever you were looking at.

- **Wheel** zooms one notch. **Pinch** zooms continuously. **Two-finger scroll**
  and **drag** pan.
- A trackpad pinch reaches the page as a wheel event with `ctrlKey` set, which is
  the browser's own page-zoom binding; the listener is attached natively with
  `passive: false` so it can `preventDefault()`. It reports back through the eval
  channel, because a signal write from outside the runtime fails silently.
- A drag past 4px swallows the click it would otherwise end with, so panning
  across the graph never selects whatever you let go over.
- `←`/`↑` steps to what depends on the held crate, `→`/`↓` to what it depends on,
  both busiest first. `Backspace` retraces, `/` jumps to the finder, `Escape`
  lets go.

The arrangement lands on one frame rather than gliding to it. Cards used to
slide to their new places over 260ms, but a re-tidy changes how many segments a
wire has and a path whose command count changes cannot be transitioned — so the
wires snapped while the cards moved, and measurement put them up to 133 units off
their own cards for about a fifth of a second. The camera flight carries the
motion instead, and a card that has just arrived fades up.

The camera follows the graph while it is still arriving and stops the moment the
reader touches it. A move frames what was revealed and then pulls the card the
reader acted on back into view: centring on the card alone hid the answer it had
just asked for, and centring on the bounds alone pushed the question off the
edge. A chain that arrives gets the one move allowed to shrink until both of its
ends are on screen.

## The record

`src/views/inspector.rs` states the answer in words, because `cargo tree -i`
answers the same question in 400ms of copyable text and a picture that has to be
traced by eye has not answered it at all.

Every record carries the same fields in the same order: what it is, why it is
here (the chain from a workspace member, each step clickable, and a **Draw this
route** button that puts the whole chain on the pane as a lit, walking wire),
what depends on it, what it depends on. Long lists fold to seven with the rest
one click away, so both directions stay on one screen.

Counts are reported in buckets that partition their headline exactly: "64 · 27
directly · 37 further out". Totals come from the reachability closure, never from
hop levels — levels are longest-path, so a crate that is both an immediate
dependent *and* reachable by a longer route would land in two buckets at once.

## The call lens

The second lens, at `/calls`, on the same framework.

**The engine** is rust-analyzer over LSP (`src/call/lsp.rs`). "What does this
call" needs type inference: `x.len()` is `slice::len` or `Vec::len` or
`HashMap::len` depending on what `x` is, and a syntax tree cannot tell you which.
Extraction asks for the whole workspace at once and caches for the life of the
server process; on this workspace that is ~867 functions and ~1950 calls in
about sixteen seconds.

**Why it is a picture again.** It was one, then it was not: a whole-system call
graph has 240 workspace functions and 225 edges between them — average internal
degree below one — and drawing all of it spent every pixel on structure that
carries almost no information. A *focused subgraph* is a different object. One
function, its callers left and its callees right, opened a hop at a time, is
exactly the two-hop diagram the earlier reviews judged the one picture that could
earn its place. The aggregate answers stay in the record beside it.

A call graph has no global rank to borrow — it can contain cycles — so depth is
assigned when a function first arrives and never revised, which is what stops a
cycle walking a card back and forth across the pane.

**What the record answers.** Where to start, in three separately named buckets
(`main`, public API, no static caller), because collapsing them made a program
with one `main` report 85 places execution starts. What every route crosses:
immediate dominators over the call graph, which is inevitability rather than
popularity — `Vec::push` has enormous fan-in and dominates nothing, because there
is always another way round. For a held function, the chain of chokepoints above
it, drawn on the pane the same way the dependency lens draws the chain that put a
crate in the build: not *a* route but everything *every* route must cross.

**The honest limit, on the page.** A call edge exists only where one function
names another. Trait objects, function pointers, macro-invoked code and framework
callbacks leave no static edge, and the panel says in as many words that the
functions which appear to be called by nothing are a statement about the
analysis, not about the code.

## Shell

`src/views/shell.rs` is the lens frame: workspace identity, the lens tabs, the
finder and the keyboard. Later lenses mount here as siblings. Unbuilt lenses are
not advertised, because a permanently disabled tab spends credibility on first
run for nothing.

On a phone the record docks under the pane instead of beside it, and the finder
collapses to its own icon and takes the bar when focused rather than squeezing
the workspace's name to nothing.

## Tests

Tests build the real graph for whichever workspace they run in, not a fixture.

- `crates/dioxus-flow/src/camera.rs` — a notch out and back is exactly reversible, zoom holds
  the anchored point still, a clamped zoom does not drift the world, framing
  stops shrinking at the floor and left-anchors when it overflows, and a flight
  interpolates magnification geometrically rather than arithmetically.
- `crates/dioxus-flow/src/layout.rs` — every edge runs left to right, empty columns are
  compacted away, cards in a column never overlap, the same graph places
  identically twice, a chain is drawn straight, a card sits level with the middle
  of its children, a deliberately tangled ordering comes out with **zero**
  crossings rather than merely fewer, a wire spanning several columns gets a lane
  in each and that lane clears the cards it passes, a long wire comes out
  straight rather than wandering, and re-tidying from the previous frame is never
  worse than laying the same graph out cold.
- `crates/dioxus-flow/src/geometry.rs` — a curve touches both handles, leaves and arrives
  horizontally, a backwards wire bulges around itself, and a routed wire is one
  smooth run that actually passes through the lanes it was given.
- `src/graph/build.rs` — ranks move forward along every dependency, dependents
  invert dependencies exactly, every member of a duplicate group is flagged, and
  rank is the longest route rather than the shortest.
- `src/graph/focus.rs` — direction agrees with which column a crate is in, direct
  and further-out partition the total, the why-path is a real chain starting at a
  workspace member, and the direct lists match the crate itself.
- `crates/dioxus-flow/src/fold.rs` — inverting the edges neither invents nor
  loses one, depth is the shortest route, folding takes away exactly what was
  only reachable through what you folded, folding then opening returns the same
  pane **for every node**, a cycle does not hang the walk, and a stale id is
  empty rather than a panic.
- `src/views/deps.rs` — the first reading is three hops deep, the rim opens to
  bring the next hop in, nothing is on the
  pane that was not opened, **holding a crate never changes what is on the pane**,
  edges take their hue from which way they run, every edge joins two cards that
  are on the pane, a port always carries the whole count, and a crate off the pane
  arrives with its route lit end to end.
- `src/views/calls.rs` — a column is assigned once and kept, callers sit left and
  callees right, nothing but functions reaches the pane, the chain every route
  crosses is lit end to end, holding a function never changes the cast, and a
  function reached two ways is still one card.
- `src/call/` — the extraction's own suite, including
  `a_chokepoint_really_is_unavoidable`, which re-runs reachability with each
  claimed chokepoint deleted, and `dominance_disagrees_with_popularity`, which
  asserts the most-called function does *not* dominate what reaches it.

## Not built

Incoming call hierarchy is not queried — callers are derived by inverting the
outgoing edges, which is exact for everything the extraction reached but cannot
see callers in crates it never opened. Cards are not draggable: the verb is
expand and fold, not arrange.

Opening on the whole build is not free, and nothing here virtualises the pane. A
700-crate workspace mounts 10,405 elements; served locally it draws about 470ms
after navigation, but that first frame is one long task of roughly a second, so
the tab is unresponsive rather than blank while it happens. The waiting wire
covers the server resolving the workspace, not this. Culling what is off-camera
is the obvious next move and is not made.

The visual system is recorded in `DESIGN.md`; the surfaces' strategies live in
`.impeccable/surfaces/`.
