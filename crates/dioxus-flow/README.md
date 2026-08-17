# dioxus-flow

Nodes and edges on a pannable, zoomable pane for [Dioxus](https://dioxuslabs.com),
with layered auto-layout.

You give every node a **column**. The crate does the rest: it orders each column
to minimise crossings, places nodes so chains come out straight and a parent sits
level with its children, and opens a lane in every column a long edge crosses so
that edge routes *around* the nodes there instead of over them. It re-runs
whenever the graph changes — seeded from the last frame, so expanding one node
moves what the topology forces and not what it does not.

```rust
use dioxus::prelude::*;
use dioxus_flow::{Card, Edge, Flow, Graph, Node, Port, Way, use_flow};

#[component]
fn Chart() -> Element {
    let flow = use_flow();
    let graph = Graph {
        nodes: vec![
            Node::new(0, 0, Card::new("main").subtitle("src/main.rs").filled())
                .ports(None, Some(Port::new(2, true))),
            Node::new(1, 1, Card::new("parse")).ports(Some(Port::new(1, false)), None),
            Node::new(2, 1, Card::new("render")).ports(Some(Port::new(1, false)), None),
        ],
        edges: vec![Edge::new(0, 1), Edge::new(0, 2).label("async")],
    };

    rsx! {
        document::Stylesheet { href: dioxus_flow::STYLESHEET }
        Flow {
            flow,
            graph,
            on_select: move |id| tracing::info!("held {id}"),
            on_port: move |(id, way): (usize, Way)| tracing::info!("{id} {way:?}"),
        }
    }
}
```

## What you get

**Auto-layout that stays a DAG.** Three stages, and each is there because the one
before it is not enough. Lanes, because a long edge that is not *in* the columns
it crosses gives nothing there a reason to move aside. Ordering by median sweeps
*and* a transpose pass, because every child of one freshly opened node has the
same median and a sweep has no opinion about their order at all. Coordinates
granted in rank order — lanes before nodes, busiest node first — because that is
the difference between a long edge drawn as a straight run and the same edge
drawn as a sweep across the whole picture.

**A camera with the properties you notice when they are missing.** Zoom anchors
on the point being aimed at rather than the viewport centre. Magnification is
held as an exponent, so a wheel notch out and back returns to exactly the same
pixels. Flights interpolate position linearly and magnification geometrically,
because the arithmetic mean reads as a lurch at the far end.

**Gestures that behave.** The wheel listener is installed natively at
`passive: false`, because a trackpad pinch arrives as `ctrl`+wheel and that is
the browser's own page-zoom binding — without refusing it, pinching scales the
whole page. A drag past four pixels swallows the click it would otherwise end
with, so panning never selects whatever you let go over.

**Ports as a first-class idea.** A node states how much is attached to each side,
and that statement is also the control that opens or folds it. Nothing is hidden,
only put away — the count is on the node either way.

**`prefers-reduced-motion`,** honoured in CSS *and* in code: the camera jumps to
its destination instead of flying, and it listens for the preference changing
rather than only reading it once.

## Configuring

Everything shape-related lives on [`Style`], and the layout is measured in those
units rather than in constants:

```rust
use dioxus_flow::{Background, Direction, Shape, Style};

let style = Style {
    node: (220.0, 56.0),
    column_pitch: 320.0,
    direction: Direction::TopBottom,
    shape: Shape::Step,
    background: Background::Lines,
    ..Style::default()
};
```

`Direction::TopBottom` turns the whole drawing a quarter turn: columns march
downward, ports move to the top and bottom edges, and edges leave and arrive
vertically. The layout is the same pipeline — it measures *along* the flow and
*across* it rather than in x and y.

Render a node yourself with `node_view` when the built-in card is not what you
need. The card's title still names the node to assistive technology:

```rust
Flow { flow, graph, node_view: move |id| rsx! { MyNode { id } } }
```

## Theming

[`STYLESHEET`] is written against `--flow-*` custom properties with defaults, so
you theme it rather than restyle it. Set any of them above the pane:

```css
.flow-root {
  --flow-canvas-color: #0e1116;
  --flow-surface-color: #171b22;
  --flow-ink-color: #e6edf3;
  --flow-in-color: #f78166;
  --flow-out-color: #58a6ff;
}
```

Nothing in the crate reads a colour, a face or a radius from anywhere else.

## Folding

A pane only draws what someone opened. Past a few hundred nodes a whole graph
stops being a flow chart and becomes a texture, and the browser starts paying for
elements nobody reads — so hand over the *whole* graph as adjacency and keep a
`Folding`, which holds what the reader has opened rather than what is visible.

```rust
use dioxus_flow::{Adjacency, Folding, Way};

// 0 → 1 → 2, and 0 → 3.
let links = Adjacency::from_out(vec![vec![1, 3], vec![2], vec![], vec![]]);
let mut folding = Folding::to_depth(&links, vec![0], 1, Way::Out);

assert_eq!(folding.visible(&links), vec![0, 1, 3]);   // one hop, no further
assert_eq!(folding.folded(&links, 1, Way::Out), 1);   // what a port would state

folding.toggle(1, Way::Out);
assert_eq!(folding.visible(&links), vec![0, 1, 2, 3]);
```

`visible` walks from the seeds every time rather than keeping a set, and that is
the point: a flat set of visible ids cannot answer *what does folding this take
away*, because nothing in it says which nodes were reachable only through the one
you folded. Walking is what makes folding the exact inverse of opening, at any
depth, in any order — there is a test that asserts exactly that, for every node.

`Links` is a trait, not a container. Implement it on whatever you already have
and the pane walks your structure in place; copying a large graph into a
structure this crate owns would cost more per render than the layout it feeds.

```rust
use dioxus_flow::{Links, Way};

struct Build { deps: Vec<Vec<usize>>, dependents: Vec<Vec<usize>> }

impl Links for Build {
    fn len(&self) -> usize { self.deps.len() }
    fn neighbours(&self, id: usize, way: Way) -> &[usize] {
        let side = match way { Way::Out => &self.deps, Way::In => &self.dependents };
        side.get(id).map(Vec::as_slice).unwrap_or(&[])
    }
}
```

`Adjacency` is the owned implementation, for when you have nothing to lend.

## Using the layout on its own

[`layered`] is a plain function over plain data — no Dioxus, no DOM — if you want
the placement for something else:

```rust
use dioxus_flow::{Metrics, Slot, layered};
use std::collections::HashMap;

let placed = layered(
    &[Slot { id: 0, column: 0 }, Slot { id: 1, column: 1 }],
    &[(0, 1)],
    &HashMap::new(),
    &Metrics::default(),
);
```

It returns each node's position and each edge as the points it runs through,
both in along/across space.

## What it does not do

- **Nodes are not draggable.** The verb here is expand and fold, not arrange:
  the layout owns position, and a hand-placed node would be overwritten by the
  next re-tidy.
- **Edges are not bundled**, and there is no sub-flow, group container or
  minimap drag.
- **The pane is a single graph.** There is no multi-select, no marquee and no
  copy/paste.

## What it costs

Measured on release builds, against two real Cargo workspaces.

| | 377 cards, 1179 wires | 700 cards, 2643 wires |
| --- | --- | --- |
| `layered`, native | 4.4ms | 28ms |

The larger graph pays for 24,000 lane waypoints, not for its 700 cards: a wire
crossing eight columns takes part in the ordering in every one of them, and that
is what buys the reader a picture with no wire cutting through a card.

Three things about the pipeline are worth knowing before you scale it up.
Crossing counts are taken by inversion count over a Fenwick tree, so ordering is
`O(E log E)` rather than the `O(E²)` the textbook formulation implies. Ordering
stops as soon as two rounds fail to improve, which on the larger graph costs
under a tenth of a percent in crossings and halves the wait. And the pane caches
its drawing against the graph's signature, because the pipeline is deliberately
**not** idempotent — each run seeds the next from the rows it just produced — so
an ungated second evaluation both doubles the work and quietly produces a
different arrangement.

In the browser, on a 479-card, 2115-wire pane of 7,881 elements: folding a
50-wide port re-lays and re-draws it in **93ms**, or 391ms on a CPU throttled 4×.
Panning holds 60fps with 2 frames past 24ms over a two-second drag; zooming holds
120fps.

Nothing here virtualises the pane, so first paint scales with the whole drawing —
mounting that graph is a ~700ms task. `Folding` is the answer to that, not
culling: draw the hops that answer the question and leave the rest behind ports.
Culling what is off-camera is the obvious next move and is not made.

## Licence

MIT or Apache-2.0, at your option.
