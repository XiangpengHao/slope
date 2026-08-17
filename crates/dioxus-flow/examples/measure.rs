//! Throwaway prototype rig: today's layered layout vs a force-directed one,
//! on a real workspace graph.
//!
//! `cargo run --release -p dioxus-flow --example measure -- <edge-list> <out-dir> [node-budget]`

use std::collections::{HashMap, HashSet, VecDeque};

use dioxus_flow::{Air, Metrics, Ring, Slot, layered, radial, rank};

const NODE_W: f32 = 190.0;
const NODE_H: f32 = 48.0;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("edge list path");
    let out_dir = args.next().expect("output dir");
    let budget: usize = args.next().map(|a| a.parse().unwrap()).unwrap_or(usize::MAX);

    let (count, edges) = read_graph(&path);
    let (nodes, edges) = if budget < count {
        open_from_roots(count, &edges, budget)
    } else {
        ((0..count).collect::<Vec<_>>(), edges)
    };
    println!("=== {} cards, {} wires ===", nodes.len(), edges.len());

    neighbourhoods(&nodes, &edges);
    walks(&nodes, &edges, &out_dir);

    let layered_places = run_layered(&nodes, &edges);
    report("layered (today)", &layered_places, &edges);
    onscreen("layered (today)", &layered_places, &edges);
    write_svg(
        &format!("{out_dir}/layered-{}.svg", nodes.len()),
        &layered_places,
        &edges,
        "layered",
    );

    // The floor for a column layout: same columns, but no lanes at all, so each
    // column simply packs its cards. Wires would run straight and pass behind
    // whatever is in the way.
    let packed = {
        let columns = rank(&nodes, &edges);
        let slots: Vec<Slot> = nodes
            .iter()
            .map(|&id| Slot {
                id,
                column: columns[&id],
            })
            .collect();
        let started = std::time::Instant::now();
        let out = layered(&slots, &[], &HashMap::new(), &Metrics::default());
        println!("  packed took {:?}", started.elapsed());
        out.places
            .iter()
            .map(|p| (p.id, (p.along, p.across)))
            .collect::<HashMap<_, _>>()
    };
    report("columns, no lanes", &packed, &edges);
    onscreen("columns, no lanes", &packed, &edges);
    write_svg(
        &format!("{out_dir}/packed-{}.svg", nodes.len()),
        &packed,
        &edges,
        "layered",
    );

    // Two readings of "level of indirection", both with lanes off.
    for (label, depth) in [
        ("longest-path rank", rank(&nodes, &edges)),
        ("shortest hops    ", hops(&nodes, &edges)),
    ] {
        let slots: Vec<Slot> = nodes
            .iter()
            .map(|&id| Slot { id, column: depth[&id] })
            .collect();
        let metrics = Metrics { lanes: false, ..Metrics::default() };
        let started = std::time::Instant::now();
        let out = layered(&slots, &edges, &HashMap::new(), &metrics);
        let elapsed = started.elapsed();
        let places: HashMap<usize, (f32, f32)> =
            out.places.iter().map(|p| (p.id, (p.along, p.across))).collect();
        let columns: HashSet<i32> = depth.values().copied().collect();
        let mut per: HashMap<i32, usize> = HashMap::new();
        for id in &nodes { *per.entry(depth[id]).or_default() += 1; }
        let backward = edges.iter().filter(|(a, b)| depth[a] >= depth[b]).count();
        println!(
            "\n[{label}] {} columns, fullest {} cards, {backward} edges not pointing forward, {elapsed:?}",
            columns.len(),
            per.values().copied().max().unwrap_or(0)
        );
        report(label, &places, &edges);
        onscreen(label, &places, &edges);
        write_svg(&format!("{out_dir}/level-{}-{}.svg", label.trim().replace(' ', "-"), nodes.len()), &places, &edges, "layered");
    }

    {
        let depth = rank(&nodes, &edges);
        let slots: Vec<Slot> = nodes.iter().map(|&id| Slot { id, column: depth[&id] }).collect();
        let metrics = Metrics { lanes: false, ..Metrics::default() };
        let out = layered(&slots, &edges, &HashMap::new(), &metrics);
        let places: HashMap<usize, (f32, f32)> =
            out.places.iter().map(|p| (p.id, (p.along, p.across))).collect();
        let mut degree: HashMap<usize, usize> = HashMap::new();
        for (a, b) in &edges {
            *degree.entry(*a).or_default() += 1;
            *degree.entry(*b).or_default() += 1;
        }
        let mut ranked: Vec<(usize, usize)> = degree.into_iter().collect();
        ranked.sort_by_key(|&(id, d)| (d, id));
        for (label, pick) in [
            ("typical", ranked[ranked.len() / 2].0),
            ("busy", ranked[ranked.len() * 97 / 100].0),
        ] {
            write_selection_svg(
                &format!("{out_dir}/level-held-{label}-{}.svg", nodes.len()),
                &places,
                &edges,
                pick,
            );
        }
    }

    let tuned = run_columns_tuned(&nodes, &edges);
    report("columns, y tuned", &tuned, &edges);
    onscreen("columns, y tuned", &tuned, &edges);
    write_svg(
        &format!("{out_dir}/tuned-{}.svg", nodes.len()),
        &tuned,
        &edges,
        "layered",
    );

    for spread in [1.15f32, 2.0, 3.0, 4.5] {
        let force_places = shipped(&nodes, &edges, spread);
        report(&format!("force x{spread}"), &force_places, &edges);
        onscreen(&format!("force x{spread}"), &force_places, &edges);
        write_svg(
            &format!("{out_dir}/force-{}-{spread}.svg", nodes.len()),
            &force_places,
            &edges,
            "force",
        );

        // The proposed reading, on the widest setting: a typical card and a busy
        // one, so the picture is checked at both ends of the range.
        if spread == 4.5 {
            let mut degree: HashMap<usize, usize> = HashMap::new();
            for (a, b) in &edges {
                *degree.entry(*a).or_default() += 1;
                *degree.entry(*b).or_default() += 1;
            }
            let mut ranked: Vec<(usize, usize)> = degree.into_iter().collect();
            ranked.sort_by_key(|&(id, d)| (d, id));
            for (label, pick) in [
                ("typical", ranked[ranked.len() / 2].0),
                ("busy", ranked[ranked.len() * 97 / 100].0),
            ] {
                write_selection_svg(
                    &format!("{out_dir}/held-{label}-{}.svg", nodes.len()),
                    &force_places,
                    &edges,
                    pick,
                );
            }
        }
    }
}

/// If edges are drawn only for the selected card, the question is how much
/// arrives with a selection. Counted undirected: picking a card lights what it
/// depends on and what depends on it alike.
fn neighbourhoods(nodes: &[usize], edges: &[(usize, usize)]) {
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let n = nodes.len();
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b) in edges {
        let (Some(&a), Some(&b)) = (index.get(a), index.get(b)) else {
            continue;
        };
        if a == b {
            continue;
        }
        near[a].push(b);
        near[b].push(a);
    }

    // For each card: cards reached in one hop, in two, and how many edges the
    // drawing would carry if every edge inside the lit set were shown.
    let mut one = Vec::with_capacity(n);
    let mut two = Vec::with_capacity(n);
    let mut wires_two = Vec::with_capacity(n);
    let mut seen = vec![usize::MAX; n];
    for start in 0..n {
        let mut lit = vec![start];
        seen[start] = start;
        let mut frontier = vec![start];
        let mut counts = [0usize; 2];
        for hop in 0..2 {
            let mut next = Vec::new();
            for &node in &frontier {
                for &side in &near[node] {
                    if seen[side] != start {
                        seen[side] = start;
                        lit.push(side);
                        next.push(side);
                    }
                }
            }
            counts[hop] = next.len();
            frontier = next;
        }
        one.push(counts[0]);
        two.push(counts[0] + counts[1]);
        // Edges with both ends lit — what the reader would actually see.
        let inside: HashSet<usize> = lit.iter().copied().collect();
        let drawn = edges
            .iter()
            .filter(|(a, b)| match (index.get(a), index.get(b)) {
                (Some(a), Some(b)) => inside.contains(a) && inside.contains(b),
                _ => false,
            })
            .count();
        wires_two.push(drawn);
    }

    let stat = |label: &str, values: &mut Vec<usize>| {
        values.sort_unstable();
        let at = |q: f32| values[((values.len() - 1) as f32 * q) as usize];
        let over = |limit: usize| {
            values.iter().filter(|&&v| v > limit).count() as f32 / values.len() as f32 * 100.0
        };
        println!(
            "  {label:22} median {:4}  p75 {:4}  p90 {:4}  p99 {:5}  max {:5}   over 60: {:.0}%",
            at(0.5),
            at(0.75),
            at(0.9),
            at(0.99),
            values[values.len() - 1],
            over(60)
        );
    };

    println!("\n--- if edges appear only on selection ---");
    stat("cards lit, 1 hop", &mut one);
    stat("cards lit, 2 hops", &mut two);
    stat("wires drawn, 2 hops", &mut wires_two);
}

/// The proposed reading: every card drawn, no wires at rest, and one card
/// selected. Hop 1 gets its wires, coloured by direction. Hop 2 gets a mark and
/// no wires — which is what keeps a second level from costing hundreds of lines.
fn write_selection_svg(
    path: &str,
    places: &HashMap<usize, (f32, f32)>,
    edges: &[(usize, usize)],
    held: usize,
) {
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut inn: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, b) in edges {
        out.entry(*a).or_default().push(*b);
        inn.entry(*b).or_default().push(*a);
    }
    let depends_on: Vec<usize> = out.get(&held).cloned().unwrap_or_default();
    let depended_by: Vec<usize> = inn.get(&held).cloned().unwrap_or_default();
    let hop1: HashSet<usize> = depends_on.iter().chain(&depended_by).copied().collect();
    let mut hop2: HashSet<usize> = HashSet::new();
    for one in &hop1 {
        for side in out.get(one).into_iter().chain(inn.get(one)).flatten() {
            if *side != held && !hop1.contains(side) {
                hop2.insert(*side);
            }
        }
    }

    let (mut lx, mut hx, mut ly, mut hy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y) in places.values() {
        lx = lx.min(x);
        hx = hx.max(x + NODE_W);
        ly = ly.min(y);
        hy = hy.max(y + NODE_H);
    }
    let pad = 70.0;
    let (w, h) = (hx - lx + pad * 2.0, hy - ly + pad * 2.0);
    let mut svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{:.0} {:.0} {:.0} {:.0}' width='{:.0}' height='{:.0}'>\n\
         <rect x='{:.0}' y='{:.0}' width='{:.0}' height='{:.0}' fill='#f3f5f9'/>\n",
        lx - pad, ly - pad, w, h, w.min(2400.0), h * (2400.0 / w).min(1.0),
        lx - pad, ly - pad, w, h
    );

    // Every other card: present, quiet, unmistakably still there.
    for (id, &(x, y)) in places {
        if *id == held || hop1.contains(id) || hop2.contains(id) {
            continue;
        }
        svg.push_str(&format!(
            "<rect x='{x:.0}' y='{y:.0}' width='{NODE_W}' height='{NODE_H}' rx='8' fill='#ffffff' stroke='#dfe4ea' stroke-width='1'/>\n"
        ));
    }
    // Hop 2: marked, no wires.
    for id in &hop2 {
        let Some(&(x, y)) = places.get(id) else {
            continue;
        };
        svg.push_str(&format!(
            "<rect x='{x:.0}' y='{y:.0}' width='{NODE_W}' height='{NODE_H}' rx='8' fill='#ffffff' stroke='#9aa5b5' stroke-width='1.25'/>\n"
        ));
    }
    // Hop 1 wires, by direction: rust for what depends on the held card, blue
    // for what it depends on.
    let Some(&(hx0, hy0)) = places.get(&held) else {
        return;
    };
    let (cx, cy) = (hx0 + NODE_W / 2.0, hy0 + NODE_H / 2.0);
    for (list, colour) in [(&depended_by, "#bf3d10"), (&depends_on, "#1d4ed8")] {
        for side in list.iter() {
            let Some(&(sx, sy)) = places.get(side) else {
                continue;
            };
            let (mx, my) = (sx + NODE_W / 2.0, sy + NODE_H / 2.0);
            svg.push_str(&format!(
                "<path d='M{cx:.0},{cy:.0} L{mx:.0},{my:.0}' stroke='{colour}' stroke-width='2' fill='none' opacity='0.85'/>\n"
            ));
        }
    }
    for (list, colour) in [(&depended_by, "#bf3d10"), (&depends_on, "#1d4ed8")] {
        for side in list.iter() {
            let Some(&(x, y)) = places.get(side) else {
                continue;
            };
            svg.push_str(&format!(
                "<rect x='{x:.0}' y='{y:.0}' width='{NODE_W}' height='{NODE_H}' rx='8' fill='#ffffff' stroke='{colour}' stroke-width='2'/>\n"
            ));
        }
    }
    svg.push_str(&format!(
        "<rect x='{hx0:.0}' y='{hy0:.0}' width='{NODE_W}' height='{NODE_H}' rx='8' fill='#14171d' stroke='#14171d' stroke-width='2'/>\n"
    ));
    svg.push_str("</svg>\n");
    std::fs::write(path, svg).expect("write svg");
    println!(
        "  wrote {path}  (held has {} in, {} out; hop2 marks {})",
        depended_by.len(),
        depends_on.len(),
        hop2.len()
    );
}

/// The real `dioxus_flow::force` module, on real data. The hand-rolled copy
/// below was the prototype; this is what actually ships.
fn shipped(nodes: &[usize], edges: &[(usize, usize)], spread: f32) -> HashMap<usize, (f32, f32)> {
    let depth = rank(nodes, edges);
    let air = Air {
        width: NODE_W,
        height: NODE_H,
        spread,
        gap: (20.0, 14.0),
    };
    let started = std::time::Instant::now();
    let spots = dioxus_flow::force::place(nodes, edges, &depth, &HashMap::new(), &air);
    println!("  shipped force took {:?}", started.elapsed());
    spots.into_iter().map(|s| (s.id, (s.x, s.y))).collect()
}

/// Levels of indirection as *shortest* hops from the roots, which is the other
/// reading of "how far from the workspace is this".
fn hops(nodes: &[usize], edges: &[(usize, usize)]) -> HashMap<usize, i32> {
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut has_owner: HashSet<usize> = HashSet::new();
    for (a, b) in edges {
        out.entry(*a).or_default().push(*b);
        has_owner.insert(*b);
    }
    let mut depth: HashMap<usize, i32> = HashMap::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    for &id in nodes {
        if !has_owner.contains(&id) {
            depth.insert(id, 0);
            queue.push_back(id);
        }
    }
    while let Some(id) = queue.pop_front() {
        let here = depth[&id];
        for &next in out.get(&id).into_iter().flatten() {
            if !depth.contains_key(&next) {
                depth.insert(next, here + 1);
                queue.push_back(next);
            }
        }
    }
    for &id in nodes {
        depth.entry(id).or_insert(0);
    }
    depth
}

/// The radial walk, drawn at three stages: the centre just opened, one card
/// opened after it, and a few more. This is the shipped `radial` module.
fn walks(nodes: &[usize], edges: &[(usize, usize)], out_dir: &str) {
    let mut out: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut indeg: HashMap<usize, usize> = HashMap::new();
    for &(a, b) in edges {
        out.entry(a).or_default().push(b);
        *indeg.entry(b).or_default() += 1;
    }
    // Start where the lens starts: a crate nothing depends on, i.e. a member.
    let root = *nodes
        .iter()
        .filter(|id| indeg.get(id).copied().unwrap_or(0) == 0)
        .max_by_key(|id| out.get(id).map(|v| v.len()).unwrap_or(0))
        .expect("a root");

    let air = Ring { node: (NODE_W, NODE_H), gap: 28.0, step: 300.0 };
    let mut opened: HashSet<usize> = HashSet::from([root]);
    for stage in 0..3 {
        let tree = radial::spanning(
            root,
            &|id| opened.contains(&id),
            &|id| out.get(&id).cloned().unwrap_or_default(),
        );
        let spots = radial::radial(&tree, &air);
        let places: HashMap<usize, (f32, f32)> =
            spots.iter().map(|s| (s.id, (s.x, s.y))).collect();
        let inside: HashSet<usize> = places.keys().copied().collect();
        let all: Vec<(usize, usize)> = edges
            .iter()
            .copied()
            .filter(|(a, b)| inside.contains(a) && inside.contains(b))
            .collect();
        // What the lens actually draws at rest: the walk's own edges only.
        let drawn: Vec<(usize, usize)> = tree
            .iter()
            .filter_map(|shoot| Some((shoot.parent?, shoot.id)))
            .collect();
        println!(
            "  walk stage {stage}: {} cards, {} wires drawn ({} exist among them)",
            places.len(),
            drawn.len(),
            all.len()
        );
        write_svg(&format!("{out_dir}/walk-{stage}.svg"), &places, &drawn, "force");

        // Open the busiest thing currently on the rim.
        let next: Vec<usize> = places
            .keys()
            .copied()
            .filter(|id| !opened.contains(id))
            .collect();
        let mut by_fan = next;
        by_fan.sort_by_key(|id| std::cmp::Reverse(out.get(id).map(|v| v.len()).unwrap_or(0)));
        for id in by_fan.into_iter().take(if stage == 0 { 1 } else { 3 }) {
            opened.insert(id);
        }
    }
}

fn read_graph(path: &str) -> (usize, Vec<(usize, usize)>) {
    let text = std::fs::read_to_string(path).expect("read");
    let mut lines = text.lines();
    let count: usize = lines.next().unwrap().trim().parse().unwrap();
    let edges = lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            (
                parts.next().unwrap().parse().unwrap(),
                parts.next().unwrap().parse().unwrap(),
            )
        })
        .collect();
    (count, edges)
}

/// Breadth-first from the graph's roots, the way a reader opens the pane:
/// the first `budget` cards reached, and the edges among them.
fn open_from_roots(
    count: usize,
    edges: &[(usize, usize)],
    budget: usize,
) -> (Vec<usize>, Vec<(usize, usize)>) {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut indegree = vec![0usize; count];
    for &(from, to) in edges {
        out[from].push(to);
        indegree[to] += 1;
    }
    let mut queue: VecDeque<usize> = (0..count).filter(|&i| indegree[i] == 0).collect();
    let mut seen: HashSet<usize> = queue.iter().copied().collect();
    let mut kept = Vec::new();
    while let Some(node) = queue.pop_front() {
        if kept.len() >= budget {
            break;
        }
        kept.push(node);
        for &next in &out[node] {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    let inside: HashSet<usize> = kept.iter().copied().collect();
    let kept_edges = edges
        .iter()
        .copied()
        .filter(|(a, b)| inside.contains(a) && inside.contains(b))
        .collect();
    (kept, kept_edges)
}

fn run_layered(nodes: &[usize], edges: &[(usize, usize)]) -> HashMap<usize, (f32, f32)> {
    let columns = rank(nodes, edges);
    let slots: Vec<Slot> = nodes
        .iter()
        .map(|&id| Slot {
            id,
            column: columns[&id],
        })
        .collect();
    let started = std::time::Instant::now();
    let out = layered(&slots, edges, &HashMap::new(), &Metrics::default());
    println!("  layered took {:?}", started.elapsed());
    out.places
        .iter()
        .map(|p| (p.id, (p.along, p.across)))
        .collect()
}

/// Columns keep their meaning — x is still depth — but there are no lanes, and
/// y is solved for short edges instead of for straight wires.
///
/// This is the layout the "edges only on selection" idea actually wants: the
/// depth axis still reads with every wire hidden, and lighting a card pulls up
/// neighbours that are close by rather than a screen away.
fn run_columns_tuned(nodes: &[usize], edges: &[(usize, usize)]) -> HashMap<usize, (f32, f32)> {
    let started = std::time::Instant::now();
    let n = nodes.len();
    let index: HashMap<usize, usize> = nodes.iter().enumerate().map(|(i, &id)| (id, i)).collect();
    let columns_of = rank(nodes, edges);
    let depth: Vec<i32> = nodes.iter().map(|id| columns_of[id]).collect();

    let mut near: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b) in edges {
        let (Some(&a), Some(&b)) = (index.get(a), index.get(b)) else {
            continue;
        };
        if a != b {
            near[a].push(b);
            near[b].push(a);
        }
    }

    // Compact the occupied columns onto a pitch, as the real layout does.
    let mut keys: Vec<i32> = depth.clone();
    keys.sort_unstable();
    keys.dedup();
    let slot_of: HashMap<i32, usize> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); keys.len()];
    for i in 0..n {
        members[slot_of[&depth[i]]].push(i);
    }

    // Start each column stacked, then repeatedly pull every card to the average
    // of its neighbours and re-stack to remove overlap. Barycentre plus
    // separation: the standard way to get short edges under a fixed x.
    let pitch = 68.0f32; // card height plus gap
    let mut y = vec![0.0f32; n];
    for column in &members {
        for (slot, &i) in column.iter().enumerate() {
            y[i] = slot as f32 * pitch;
        }
    }

    for _ in 0..60 {
        let wanted: Vec<f32> = (0..n)
            .map(|i| {
                if near[i].is_empty() {
                    y[i]
                } else {
                    near[i].iter().map(|&j| y[j]).sum::<f32>() / near[i].len() as f32
                }
            })
            .collect();
        for column in &mut members {
            column.sort_by(|&a, &b| wanted[a].partial_cmp(&wanted[b]).unwrap().then(a.cmp(&b)));
            // Place each card as near its wanted y as the stack allows. Shifting
            // by -slot*pitch turns "keep them `pitch` apart" into "keep them in
            // order", which is plain isotonic regression, solved exactly by
            // pool-adjacent-violators. Blocks of cards that would overlap end up
            // sharing the average of what they wanted.
            let shifted: Vec<f32> = column
                .iter()
                .enumerate()
                .map(|(slot, &i)| wanted[i] - slot as f32 * pitch)
                .collect();
            // Each pool: (value, how many cards it covers).
            let mut pools: Vec<(f32, usize)> = Vec::with_capacity(shifted.len());
            for &value in &shifted {
                pools.push((value, 1));
                while pools.len() > 1 && pools[pools.len() - 2].0 > pools[pools.len() - 1].0 {
                    let (v2, n2) = pools.pop().unwrap();
                    let (v1, n1) = pools.pop().unwrap();
                    let total = n1 + n2;
                    pools.push(((v1 * n1 as f32 + v2 * n2 as f32) / total as f32, total));
                }
            }
            let mut slot = 0usize;
            for (value, run) in pools {
                for _ in 0..run {
                    y[column[slot]] = value + slot as f32 * pitch;
                    slot += 1;
                }
            }
        }
    }

    println!("  tuned took {:?}", started.elapsed());
    nodes
        .iter()
        .enumerate()
        .map(|(i, &id)| {
            (
                id,
                (slot_of[&depth[i]] as f32 * 280.0, y[i] - NODE_H / 2.0),
            )
        })
        .collect()
}



/// The question the "edges on selection" design actually turns on: click a
/// card, and does what lights up fit on the screen?
///
/// A 1600x900 viewport at 1:1. Anything bigger has to be zoomed out to be seen
/// whole, and past about 3x zoom-out a card's name is unreadable, so a
/// neighbourhood needing more than that cannot be taken in at a glance.
fn onscreen(label: &str, places: &HashMap<usize, (f32, f32)>, edges: &[(usize, usize)]) {
    const VIEW_W: f32 = 1600.0;
    const VIEW_H: f32 = 900.0;

    let mut near: HashMap<usize, Vec<usize>> = HashMap::new();
    for (a, b) in edges {
        if a != b {
            near.entry(*a).or_default().push(*b);
            near.entry(*b).or_default().push(*a);
        }
    }

    let mut zooms: Vec<f32> = Vec::new();
    for (id, &(x, y)) in places {
        let Some(sides) = near.get(id) else { continue };
        let (mut lx, mut hx, mut ly, mut hy) = (x, x + NODE_W, y, y + NODE_H);
        for side in sides {
            let Some(&(sx, sy)) = places.get(side) else {
                continue;
            };
            lx = lx.min(sx);
            hx = hx.max(sx + NODE_W);
            ly = ly.min(sy);
            hy = hy.max(sy + NODE_H);
        }
        // How far out you must zoom for the lit set to fit. 1.0 means it already does.
        zooms.push(((hx - lx) / VIEW_W).max((hy - ly) / VIEW_H).max(1.0));
    }
    zooms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f32| zooms[((zooms.len() - 1) as f32 * q) as usize];
    let share = |limit: f32| {
        zooms.iter().filter(|&&z| z <= limit).count() as f32 / zooms.len() as f32 * 100.0
    };
    println!(
        "  {label:18} zoom-out to see neighbours: median {:.1}x  p90 {:.1}x  max {:.1}x   fits as-is {:.0}%   readable (<3x) {:.0}%",
        at(0.5),
        at(0.9),
        zooms[zooms.len() - 1],
        share(1.0),
        share(3.0)
    );
}

fn report(label: &str, places: &HashMap<usize, (f32, f32)>, edges: &[(usize, usize)]) {
    let (mut lx, mut hx, mut ly, mut hy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y) in places.values() {
        lx = lx.min(x);
        hx = hx.max(x + NODE_W);
        ly = ly.min(y);
        hy = hy.max(y + NODE_H);
    }
    let mut total = 0.0f32;
    let mut longest = 0.0f32;
    for (a, b) in edges {
        let (Some(&(ax, ay)), Some(&(bx, by))) = (places.get(a), places.get(b)) else {
            continue;
        };
        let d = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
        total += d;
        longest = longest.max(d);
    }
    let (w, h) = (hx - lx, hy - ly);
    let ink = places.len() as f32 * NODE_W * NODE_H;
    println!(
        "  {label:16} {w:.0} x {h:.0}   area {:.1}M   cards fill {:.1}%   wire total {:.0} (avg {:.0}, max {:.0})",
        w * h / 1.0e6,
        ink / (w * h) * 100.0,
        total,
        total / edges.len().max(1) as f32,
        longest
    );
}

fn write_svg(
    path: &str,
    places: &HashMap<usize, (f32, f32)>,
    edges: &[(usize, usize)],
    kind: &str,
) {
    let (mut lx, mut hx, mut ly, mut hy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y) in places.values() {
        lx = lx.min(x);
        hx = hx.max(x + NODE_W);
        ly = ly.min(y);
        hy = hy.max(y + NODE_H);
    }
    let pad = 80.0;
    let (w, h) = (hx - lx + pad * 2.0, hy - ly + pad * 2.0);
    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{:.0} {:.0} {:.0} {:.0}' width='{:.0}' height='{:.0}'>\n\
         <rect x='{:.0}' y='{:.0}' width='{:.0}' height='{:.0}' fill='#f3f5f9'/>\n",
        lx - pad, ly - pad, w, h, w.min(2400.0), (h * (2400.0 / w).min(1.0)).max(1.0),
        lx - pad, ly - pad, w, h
    ));
    svg.push_str("<g stroke='#c3cbd6' stroke-width='1.1' fill='none' opacity='0.75'>\n");
    for (a, b) in edges {
        let (Some(&(ax, ay)), Some(&(bx, by))) = (places.get(a), places.get(b)) else {
            continue;
        };
        let (x1, y1) = (ax + NODE_W, ay + NODE_H / 2.0);
        let (x2, y2) = (bx, by + NODE_H / 2.0);
        if kind == "layered" {
            let mid = (x1 + x2) / 2.0;
            svg.push_str(&format!(
                "<path d='M{x1:.0},{y1:.0} C{mid:.0},{y1:.0} {mid:.0},{y2:.0} {x2:.0},{y2:.0}'/>\n"
            ));
        } else {
            // Free placement: attach at centres, let the card cover the rest.
            let (cx1, cy1) = (ax + NODE_W / 2.0, ay + NODE_H / 2.0);
            let (cx2, cy2) = (bx + NODE_W / 2.0, by + NODE_H / 2.0);
            svg.push_str(&format!("<path d='M{cx1:.0},{cy1:.0} L{cx2:.0},{cy2:.0}'/>\n"));
        }
    }
    svg.push_str("</g>\n<g>\n");
    for &(x, y) in places.values() {
        svg.push_str(&format!(
            "<rect x='{x:.0}' y='{y:.0}' width='{NODE_W}' height='{NODE_H}' rx='8' fill='#ffffff' stroke='#7f8c9f' stroke-width='1.25'/>\n"
        ));
    }
    svg.push_str("</g>\n</svg>\n");
    std::fs::write(path, svg).expect("write svg");
    println!("  wrote {path}");
}
