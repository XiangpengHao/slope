//! Throwaway: what the lane machinery costs on a real workspace.
//! `cargo run -p dioxus-flow --example measure -- <edge-list>`

use std::collections::HashMap;

use dioxus_flow::{Metrics, Slot, layered, rank};

fn main() {
    let path = std::env::args().nth(1).expect("edge list path");
    let text = std::fs::read_to_string(&path).expect("read");
    let mut lines = text.lines();
    let count: usize = lines.next().unwrap().trim().parse().unwrap();
    let edges: Vec<(usize, usize)> = lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            (
                parts.next().unwrap().parse().unwrap(),
                parts.next().unwrap().parse().unwrap(),
            )
        })
        .collect();
    let nodes: Vec<usize> = (0..count).collect();

    let columns = rank(&nodes, &edges);
    let slots: Vec<Slot> = nodes
        .iter()
        .map(|&id| Slot {
            id,
            column: columns[&id],
        })
        .collect();

    println!("graph: {count} cards, {} wires", edges.len());

    // How many lanes each column would carry, and how many cards.
    let mut cards_in: HashMap<i32, usize> = HashMap::new();
    for slot in &slots {
        *cards_in.entry(slot.column).or_default() += 1;
    }
    let mut lanes_in: HashMap<i32, usize> = HashMap::new();
    for &(from, to) in &edges {
        let (a, b) = (columns[&from], columns[&to]);
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        for column in (lo + 1)..hi {
            *lanes_in.entry(column).or_default() += 1;
        }
    }
    let total_lanes: usize = lanes_in.values().sum();
    let widest = cards_in.keys().chain(lanes_in.keys()).copied().max().unwrap();
    println!(
        "columns: {}, lanes: {total_lanes} ({:.1} per wire)",
        cards_in.len(),
        total_lanes as f32 / edges.len() as f32
    );

    println!("\n  col  cards  lanes   card-only  with-lanes  (world units across)");
    let m = Metrics::default();
    for column in 0..=widest {
        let cards = cards_in.get(&column).copied().unwrap_or(0);
        let lanes = lanes_in.get(&column).copied().unwrap_or(0);
        if cards == 0 && lanes == 0 {
            continue;
        }
        let card_only = cards as f32 * (m.across + m.gap);
        // Cards and lanes interleaved: worst case each lane pays node_lane_gap,
        // best case they clump and pay lane_gap. Report the clumped figure.
        let with_lanes = card_only + lanes as f32 * m.lane_gap;
        println!("  {column:3}  {cards:5}  {lanes:5}   {card_only:9.0}  {with_lanes:10.0}");
    }

    for (label, metrics) in [
        ("current       ", Metrics::default()),
        (
            "lane_gap 0    ",
            Metrics {
                lane_gap: 0.0,
                node_lane_gap: 0.0,
                ..Metrics::default()
            },
        ),
    ] {
        let started = std::time::Instant::now();
        let out = layered(&slots, &edges, &HashMap::new(), &metrics);
        let elapsed = started.elapsed();
        let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
        for place in &out.places {
            low = low.min(place.across);
            high = high.max(place.across + metrics.across);
        }
        // Tallest single column as actually placed, which is what sets the extent.
        let mut per_column: HashMap<i32, (f32, f32, usize)> = HashMap::new();
        for place in &out.places {
            let column = (place.along / metrics.pitch).round() as i32;
            let entry = per_column
                .entry(column)
                .or_insert((f32::INFINITY, f32::NEG_INFINITY, 0));
            entry.0 = entry.0.min(place.across);
            entry.1 = entry.1.max(place.across + metrics.across);
            entry.2 += 1;
        }
        // Do any two cards in a column end up closer than `gap` apart?
        let mut by_column: HashMap<i32, Vec<f32>> = HashMap::new();
        for place in &out.places {
            by_column
                .entry((place.along / metrics.pitch).round() as i32)
                .or_default()
                .push(place.across);
        }
        let mut collisions = 0usize;
        let mut worst: f32 = f32::INFINITY;
        for ys in by_column.values_mut() {
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in ys.windows(2) {
                let apart = pair[1] - pair[0];
                worst = worst.min(apart);
                if apart < metrics.across + metrics.gap - 0.5 {
                    collisions += 1;
                }
            }
        }

        let (tall_col, tall) = per_column
            .iter()
            .map(|(&c, &(lo, hi, n))| (c, (hi - lo, n)))
            .max_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap())
            .unwrap();
        let along = out
            .places
            .iter()
            .map(|p| p.along)
            .fold(0.0f32, f32::max)
            + metrics.along;
        let points: usize = out.wires.iter().map(|w| w.points.len()).sum();
        println!(
            "\n{label} extent {:.0} x {:.0} world units  wire points {points}  {elapsed:?}\n\
             {:16} tallest column {tall_col} holds {} cards spanning {:.0} ({:.0} if packed tight)\n\
             {:16} card pairs closer than {:.0} apart: {collisions} (closest {worst:.1})",
            along,
            high - low,
            "",
            tall.1,
            tall.0,
            tall.1 as f32 * (metrics.across + metrics.gap) - metrics.gap,
            "",
            metrics.across + metrics.gap
        );
    }

    // What the cards alone would need if wires were allowed to cross them.
    let tallest = cards_in.values().copied().max().unwrap();
    println!(
        "\ncards only, no lanes at all: tallest column {tallest} cards = {:.0} world units",
        tallest as f32 * (m.across + m.gap)
    );
}
