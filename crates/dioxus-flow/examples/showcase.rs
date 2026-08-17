//! Every option this crate has, on one small graph.
//!
//! ```sh
//! dx serve --example showcase --platform web
//! ```
//!
//! Click a port to see what open and folded look like; the counts are the
//! crate's, not the example's.

use dioxus::prelude::*;
use dioxus_flow::{
    Background, Badge, Card, Direction, Edge, EdgeState, Flow, Graph, Node, NodeState, Port, Shape,
    Style, Way, use_flow,
};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut vertical = use_signal(|| false);
    let mut stepped = use_signal(|| false);
    let mut held = use_signal(|| None::<usize>);
    let flow = use_flow();

    let style = Style {
        direction: if vertical() {
            Direction::TopBottom
        } else {
            Direction::LeftRight
        },
        shape: if stepped() {
            Shape::Step
        } else {
            Shape::Bezier
        },
        background: Background::Dots,
        node: (200.0, 56.0),
        ..Style::default()
    };

    // Five stages and the wire that skips three of them, which is the case the
    // lanes exist for: it routes around the middle column rather than over it.
    let names = [
        ("ingest", "source", true),
        ("parse", "stage 1", false),
        ("validate", "stage 1", false),
        ("enrich", "stage 1", false),
        ("write", "sink", false),
    ];
    let columns = [0, 1, 1, 1, 2];
    let wires = [(0, 1, None), (0, 2, None), (0, 3, None), (1, 4, Some("rows")), (2, 4, None), (0, 4, Some("raw"))];

    let graph = Graph {
        nodes: names
            .iter()
            .enumerate()
            .map(|(id, (title, subtitle, filled))| {
                let mut card = Card::new(*title).subtitle(*subtitle);
                if *filled {
                    card = card.filled();
                }
                if *title == "validate" {
                    card = card.badge(Badge::new("slow").flag());
                }
                let inbound = wires.iter().filter(|(_, to, _)| *to == id).count();
                let outbound = wires.iter().filter(|(from, _, _)| *from == id).count();
                Node::new(id, columns[id], card)
                    .ports(
                        (inbound > 0).then_some(Port::new(inbound, true)),
                        (outbound > 0).then_some(Port::new(outbound, true)),
                    )
                    .state(if held() == Some(id) {
                        NodeState::Held
                    } else {
                        NodeState::Rest
                    })
            })
            .collect(),
        edges: wires
            .iter()
            .map(|(from, to, label)| {
                let mut edge = Edge::new(*from, *to).state(match held() {
                    Some(id) if id == *to => EdgeState::Incoming,
                    Some(id) if id == *from => EdgeState::Outgoing,
                    Some(_) => EdgeState::Muted,
                    None => EdgeState::Rest,
                });
                if let Some(label) = label {
                    edge = edge.label(*label);
                }
                edge
            })
            .collect(),
        // Columns, not a walk: this example has no centre.
        root: None,
    };

    rsx! {
        document::Stylesheet { href: dioxus_flow::STYLESHEET }
        div { style: "display:flex;flex-direction:column;height:100vh;font-family:system-ui",
            div { style: "display:flex;gap:12px;padding:10px 14px;border-bottom:1px solid #e4e8ee",
                button { onclick: move |_| vertical.toggle(),
                    if vertical() { "top to bottom" } else { "left to right" }
                }
                button { onclick: move |_| stepped.toggle(),
                    if stepped() { "stepped wires" } else { "bezier wires" }
                }
                span { style: "color:#5b6472",
                    match held() {
                        Some(id) => format!("holding {}", names[id].0),
                        None => "click a node".to_string(),
                    }
                }
            }
            Flow {
                flow,
                graph,
                style,
                on_select: move |id| held.set(Some(id)),
                on_clear: move |_| held.set(None),
                on_port: move |(id, way): (usize, Way)| {
                    tracing::info!("port {way:?} on {id}");
                },
            }
        }
    }
}
