//! Octilinear routing for the board's copper.
//!
//! Every segment of a trace sits at 0, 45, or 90 degrees. Board traces chamfer
//! their corners rather than turning square — a right angle in copper is an
//! etchant trap — and the same constraint is what keeps a run traceable by eye
//! where a bezier tangle is not.

use super::Point;

/// Intermediate corners turning a straight hop into horizontal, 45-degree, and
/// vertical segments only. Returns just the corners; the caller supplies ends.
pub fn octilinear(a: Point, b: Point) -> Vec<Point> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dy.abs() < 0.5 || dx <= 0.5 {
        return Vec::new();
    }
    let sign = if dy > 0.0 { 1.0 } else { -1.0 };
    if dx >= dy.abs() {
        // Room for the whole rise in one diagonal: run, climb, run.
        let lead = (dx - dy.abs()) / 2.0;
        vec![
            Point { x: a.x + lead, y: a.y },
            Point {
                x: a.x + lead + dy.abs(),
                y: b.y,
            },
        ]
    } else {
        // Steeper than 45 degrees: climb, go vertical, climb again.
        let half = dx / 2.0;
        vec![
            Point {
                x: a.x + half,
                y: a.y + sign * half,
            },
            Point {
                x: b.x - half,
                y: b.y - sign * half,
            },
        ]
    }
}

pub fn dedupe(points: &mut Vec<Point>) {
    points.dedup_by(|a, b| (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5);
}
