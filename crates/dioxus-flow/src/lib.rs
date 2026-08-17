//! Nodes and edges on a pannable, zoomable pane, with layered auto-layout.
//!
//! ```no_run
//! use dioxus::prelude::*;
//! use dioxus_flow::{Card, Edge, Flow, Graph, Node, Port, use_flow};
//!
//! #[component]
//! fn App() -> Element {
//!     let flow = use_flow();
//!     let graph = Graph {
//!         nodes: vec![
//!             Node::new(0, 0, Card::new("main")),
//!             Node::new(1, 1, Card::new("parse")),
//!         ],
//!         edges: vec![Edge::new(0, 1)],
//!         ..Default::default()
//!     };
//!     rsx! {
//!         document::Stylesheet { href: dioxus_flow::STYLESHEET }
//!         Flow { flow, graph }
//!     }
//! }
//! ```
//!
//! # What it does for you
//!
//! - **Auto-layout.** You give every node a column; the crate does the rest —
//!   ordering that actually minimises crossings, coordinates that straighten
//!   chains, and a lane in every column a long edge crosses so it routes around
//!   the nodes there rather than over them. It re-runs whenever the graph
//!   changes, seeded from the last frame so a re-tidy moves what the topology
//!   forces and not what it does not.
//! - **A camera** that anchors zoom on the point being aimed at, holds
//!   magnification as an exponent so a wheel notch out and back is exactly
//!   reversible, and flies with weight.
//! - **Gestures** installed natively at `passive: false`, because a trackpad
//!   pinch arrives as `ctrl`+wheel and that is the browser's own page-zoom
//!   binding.
//! - **Ports** as a first-class idea: a node says how much is attached to each
//!   side and opening or folding it is one click.
//! - **Two kinds of folding**, because a large graph is large in two directions.
//!   [`Folding`] holds how much of the graph is drawn; [`Nest`] holds how finely,
//!   for a graph whose nodes contain one another — open a container and it
//!   becomes its children, with every edge that pointed at it re-aimed at
//!   whichever child answers it.
//! - **[`rank`]**, for when the host has no column to give because the graph has
//!   cycles in it.
//!
//! # What it does not do
//!
//! Nodes are not draggable. This crate's verb is expand and fold, not arrange:
//! the layout owns position, and a hand-placed node would be overwritten by the
//! next re-tidy. A hierarchy is drawn by drilling into it rather than as nested
//! boxes, and parallel edges that both stay on the pane are not braided.
//!
//! # Styling
//!
//! [`STYLESHEET`] carries everything the pane needs, written against `--flow-*`
//! custom properties with defaults. Override them anywhere above the pane to
//! theme it; nothing here reads a colour from anywhere else.

use dioxus::prelude::*;

mod camera;
mod fold;
pub mod force;
mod geometry;
pub mod layout;
mod nest;
mod pane;
pub mod radial;
mod rank;

pub use camera::{Bounds, Camera, Flight};
pub use fold::{Adjacency, Folding, Links, depths};
pub use force::{Air, Spot};
pub use geometry::{Axis, Shape};
pub use layout::{Metrics, Placement, Slot, Wire, layered};
pub use nest::{Bundle, Forest, Lift, Nest, Tree};
pub use radial::{Ring, Shoot};
pub use pane::{Flow, FlowHandle, use_flow};
pub use rank::rank;

/// The pane's own stylesheet. Mount it once, above the pane.
pub const STYLESHEET: Asset = asset!("/assets/flow.css");

/// Which way the graph reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Direction {
    /// Columns march left to right; an edge leaves a node's right side.
    #[default]
    LeftRight,
    /// Columns march top to bottom; an edge leaves a node's bottom.
    TopBottom,
}

/// What the pane draws on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Background {
    #[default]
    Dots,
    Lines,
    Blank,
}

impl Background {
    fn class(self) -> &'static str {
        match self {
            Background::Dots => "flow-pane flow-pane--dots",
            Background::Lines => "flow-pane flow-pane--lines",
            Background::Blank => "flow-pane",
        }
    }
}

/// Sizes, shapes and directions the pane draws with. Every field has a default
/// that produces the drawing this crate is opinionated about; change one and the
/// layout follows, because the layout is measured in these units rather than in
/// constants.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Style {
    /// A node's size in world units. Every node is this size: uniform objects
    /// are what let a column be read as a list.
    pub node: (f32, f32),
    /// Distance between one column's leading edge and the next.
    pub column_pitch: f32,
    /// Air between two nodes in the same column.
    pub gap: f32,
    /// Air between two lanes, which only have to clear each other's stroke.
    pub lane_gap: f32,
    /// Air where a node and a lane are neighbours.
    pub node_lane_gap: f32,
    /// Whether a long edge claims a lane in every column it crosses. See
    /// [`Metrics::lanes`]. Turn it off when wires are not drawn at rest: the
    /// room a lane reserves is only worth reserving for a wire that is there.
    pub lanes: bool,
    pub direction: Direction,
    pub background: Background,
    pub shape: Shape,
    /// Screen pixels of air left around a framed graph.
    pub padding: f32,
    /// How nodes are placed. See [`Arrangement`].
    pub arrangement: Arrangement,
    /// Ideal distance between two joined nodes under [`Arrangement::Free`], as a
    /// multiple of node width. Ignored in columns.
    pub spread: f32,
    /// When edges are drawn. See [`Wires`].
    pub wires: Wires,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            node: (190.0, 48.0),
            column_pitch: 280.0,
            gap: 20.0,
            lane_gap: 7.0,
            node_lane_gap: 12.0,
            lanes: true,
            direction: Direction::default(),
            background: Background::default(),
            shape: Shape::default(),
            padding: 72.0,
            arrangement: Arrangement::default(),
            spread: 4.5,
            wires: Wires::default(),
        }
    }
}

/// How the pane decides where a node goes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Arrangement {
    /// Layered: the host gives every node a column and the layout only solves
    /// the other axis. The column means something and the drawing keeps saying
    /// it, at the cost of putting a node's neighbours columns away.
    #[default]
    Columns,
    /// Free: no columns, position from attraction alone. Nothing about a
    /// position means anything except "near what it is joined to", which is the
    /// arrangement to pick when wires are hidden at rest and a selection has to
    /// light up neighbours the reader can actually see.
    Free,
    /// Radial: one node at the centre and what it reaches fanned around it, ring
    /// by ring. The arrangement for a graph that is walked rather than surveyed —
    /// the reader opens one card at a time and the drawing grows outward from
    /// where they were already looking. See [`radial`] and [`Graph::root`].
    Radial,
}

/// When the pane draws its edges.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Wires {
    /// Always. Every edge is on the pane whether anything is held or not.
    #[default]
    Always,
    /// Only the ones attached to what is held. A graph with a few thousand edges
    /// is a texture when it draws them all, and the texture is worth nothing:
    /// no reader traces one wire out of two thousand. Holding a node is what
    /// asks the question, so that is when the answer is drawn.
    OnHold,
}

impl Style {
    /// How the layout should measure this style: along the flow, and across it.
    pub fn metrics(&self) -> Metrics {
        let (w, h) = self.node;
        let (along, across) = match self.direction {
            Direction::LeftRight => (w, h),
            Direction::TopBottom => (h, w),
        };
        Metrics {
            along,
            across,
            pitch: self.column_pitch,
            gap: self.gap,
            lane_gap: self.lane_gap,
            node_lane_gap: self.node_lane_gap,
            lanes: self.lanes,
        }
    }

    /// Map a layout coordinate onto the pane.
    pub fn place(&self, along: f32, across: f32) -> (f32, f32) {
        match self.direction {
            Direction::LeftRight => (along, across),
            Direction::TopBottom => (across, along),
        }
    }

    /// Which way a wire leaves and arrives.
    pub fn axis(&self) -> Axis {
        match self.direction {
            Direction::LeftRight => Axis::Horizontal,
            Direction::TopBottom => Axis::Vertical,
        }
    }
}

/// Which side of a node something is attached to, from that node's point of
/// view. In is what points *at* it, out is what it points at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Way {
    In,
    Out,
}

impl Way {
    pub fn as_str(self) -> &'static str {
        match self {
            Way::In => "in",
            Way::Out => "out",
        }
    }
}

/// A port: how many nodes are attached this way, and whether they are open.
///
/// This is the crate's one interaction beyond selection, and it is what makes a
/// graph of hundreds readable — the count is always visible, so nothing is
/// hidden, only folded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Port {
    pub count: usize,
    pub open: bool,
}

impl Port {
    pub fn new(count: usize, open: bool) -> Self {
        Self { count, open }
    }
}

/// What a node contains, when the graph is a hierarchy drawn at a level of
/// detail — see [`Nest`].
///
/// Rendered as the node's own control, distinct from its two ports, because it
/// is a different verb: a port asks for *more of the graph*, this asks for
/// *more detail about this node*. Nothing is hidden either way; the count is on
/// the node whether it is open or shut.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Inside {
    /// How much is in there. What the reader decides to open on, so it should
    /// be the number they are choosing between — usually the leaves under this
    /// node rather than its immediate children.
    pub count: usize,
    pub open: bool,
}

impl Inside {
    pub fn new(count: usize, open: bool) -> Self {
        Self { count, open }
    }
}

/// A small mark on a node. Used sparingly: a badge on every node is a column,
/// not a badge.
#[derive(Clone, PartialEq, Debug)]
pub struct Badge {
    pub label: String,
    pub title: String,
    pub tone: Tone,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            title: label.clone(),
            label,
            tone: Tone::Quiet,
        }
    }

    pub fn flag(mut self) -> Self {
        self.tone = Tone::Flag;
        self
    }

    pub fn titled(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    /// The one alarm register. Spend it on nothing else.
    Flag,
    /// A fact worth carrying without raising the voice.
    Quiet,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Tone::Flag => "flow-chip flow-chip--flag",
            Tone::Quiet => "flow-chip",
        }
    }
}

/// What the built-in node says. Fill the same fields for every node and a
/// reader who has learned one has learned them all.
///
/// Replace it wholesale with [`Flow`]'s `node_view` when a node needs to be
/// something else; the title is still used to name the node to assistive
/// technology.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Card {
    pub title: String,
    pub subtitle: String,
    pub badge: Option<Badge>,
    /// Drawn filled rather than as paper. Spend it on one fact — the one that
    /// changes how the reader treats every other node.
    pub filled: bool,
}

impl Card {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    pub fn badge(mut self, badge: Badge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn filled(mut self) -> Self {
        self.filled = true;
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NodeState {
    #[default]
    Rest,
    /// The node the reader is holding.
    Held,
    /// On the route the reader asked to see.
    OnRoute,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    pub id: usize,
    /// Which column this belongs in. Smaller is earlier in the flow. Values need
    /// not be contiguous: gaps are compacted away. [`rank`] will work one out
    /// for a graph that has cycles in it and therefore has no natural order.
    pub column: i32,
    pub card: Card,
    pub inbound: Option<Port>,
    pub outbound: Option<Port>,
    /// What this node holds, when it is a container in a hierarchy. See
    /// [`Inside`] and [`Nest`].
    pub inside: Option<Inside>,
    pub state: NodeState,
}

impl Node {
    pub fn new(id: usize, column: i32, card: Card) -> Self {
        Self {
            id,
            column,
            card,
            inbound: None,
            outbound: None,
            inside: None,
            state: NodeState::Rest,
        }
    }

    pub fn ports(mut self, inbound: Option<Port>, outbound: Option<Port>) -> Self {
        self.inbound = inbound;
        self.outbound = outbound;
        self
    }

    pub fn inside(mut self, inside: Inside) -> Self {
        self.inside = Some(inside);
        self
    }

    pub fn state(mut self, state: NodeState) -> Self {
        self.state = state;
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EdgeState {
    #[default]
    Rest,
    /// Runs into the held node: what points at it.
    Incoming,
    /// Runs out of the held node: what it points at.
    Outgoing,
    /// Attached to nothing held, while something is held.
    Muted,
    /// On the route the reader asked to see.
    Route,
}

impl EdgeState {
    fn as_str(self) -> &'static str {
        match self {
            EdgeState::Rest | EdgeState::Route => "rest",
            EdgeState::Incoming => "in",
            EdgeState::Outgoing => "out",
            EdgeState::Muted => "muted",
        }
    }

    /// Drawing order. Lit edges are painted over the resting mesh, never under
    /// it — an answer half-hidden behind the graph it belongs to is not an
    /// answer.
    fn layer(self) -> u8 {
        match self {
            EdgeState::Muted => 0,
            EdgeState::Rest => 1,
            EdgeState::Incoming | EdgeState::Outgoing => 2,
            EdgeState::Route => 3,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub state: EdgeState,
    /// How much this one wire stands for, when several of the graph's own edges
    /// have been gathered onto it — see [`Bundle`]. Drawn heavier as it grows,
    /// on a log scale against the heaviest wire in the graph, so the difference
    /// a reader sees is the difference between one call and a hundred rather
    /// than between a hundred and a hundred and four.
    ///
    /// `1` for an ordinary edge, and `1` is what [`Edge::new`] gives you.
    pub weight: usize,
    /// Written on the wire, at its middle. Keep it to a word or two.
    pub label: Option<String>,
}

impl Edge {
    pub fn new(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            state: EdgeState::Rest,
            weight: 1,
            label: None,
        }
    }

    pub fn state(mut self, state: EdgeState) -> Self {
        self.state = state;
        self
    }

    pub fn weight(mut self, weight: usize) -> Self {
        self.weight = weight;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Everything the pane draws.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// The node at the centre, under [`Arrangement::Radial`]. Ignored by every
    /// other arrangement, and a drawing with no centre draws nothing radially —
    /// the walk has to start somewhere and only the lens knows where.
    pub root: Option<usize>,
}
