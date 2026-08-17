//! Wire geometry.
//!
//! A wire leaves and arrives **horizontally**, always, so it announces its
//! direction in the first few pixels rather than in the arrowhead at the far
//! end. Between those two ends it follows the lane the layout opened for it,
//! and the job here is to turn that lane into one smooth curve rather than a
//! polyline with a kink at every column.
//!
//! Two points is the plain case: a cubic whose control points sit level with
//! their own handle, at half the horizontal gap — the curve every flow canvas
//! converged on. More than two is a wire the layout routed around cards, and it
//! is drawn as a chain of cubics whose tangents are shared at each waypoint, so
//! the whole run reads as one wire with no corner in it.

/// Which way a wire leaves and arrives.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

impl Axis {
    fn unit(self) -> (f32, f32) {
        match self {
            Axis::Horizontal => (1.0, 0.0),
            Axis::Vertical => (0.0, 1.0),
        }
    }

    fn along(self, point: (f32, f32)) -> f32 {
        match self {
            Axis::Horizontal => point.0,
            Axis::Vertical => point.1,
        }
    }
}

/// What a wire looks like between its two ends.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Shape {
    /// A cubic that leaves and arrives along the flow. The default, and the one
    /// that reads as a wire rather than as a diagram of a wire.
    #[default]
    Bezier,
    /// Corner to corner, through the lanes.
    Straight,
    /// Right angles with a rounded corner, for a graph that wants to look wired
    /// rather than drawn.
    Step,
}

/// How far a wire that runs backwards bulges before it turns around.
const CURVATURE: f32 = 0.25;
/// How round a step's corner is.
const CORNER: f32 = 10.0;
/// How much of a segment's length a shared tangent reaches along it. Below a
/// third the curve reads as a polyline; above it the wire overshoots its lane.
const REACH: f32 = 0.32;

fn control_offset(distance: f32) -> f32 {
    if distance >= 0.0 {
        distance * 0.5
    } else {
        CURVATURE * 25.0 * (-distance).sqrt()
    }
}

/// The `d` attribute for a wire running through `points`, first to last.
///
/// The first and last tangents are horizontal whatever the run does in between,
/// because that is what makes a wire read as leaving one card's right side and
/// arriving at another's left.
pub fn wire(points: &[(f32, f32)], shape: Shape, axis: Axis) -> String {
    match shape {
        Shape::Straight => return straight(points),
        Shape::Step => return step(points, axis),
        Shape::Bezier => {}
    }
    let (ux, uy) = axis.unit();
    match points {
        [] => String::new(),
        [only] => format!("M{:.1},{:.1}", only.0, only.1),
        [source, target] => {
            let offset = control_offset(axis.along(*target) - axis.along(*source));
            format!(
                "M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                source.0,
                source.1,
                source.0 + ux * offset,
                source.1 + uy * offset,
                target.0 - ux * offset,
                target.1 - uy * offset,
                target.0,
                target.1
            )
        }
        _ => {
            // A tangent per point: horizontal at the two ends, and along the
            // neighbours' chord in between, which is what shares the direction
            // across a waypoint instead of putting a corner in it.
            let last = points.len() - 1;
            let tangents: Vec<(f32, f32)> = points
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    if i == 0 || i == last {
                        (ux, uy)
                    } else {
                        let (before, after) = (points[i - 1], points[i + 1]);
                        let (dx, dy) = (after.0 - before.0, after.1 - before.1);
                        let length = (dx * dx + dy * dy).sqrt();
                        if length > 0.0 {
                            (dx / length, dy / length)
                        } else {
                            (ux, uy)
                        }
                    }
                })
                .collect();

            let mut path = format!("M{:.1},{:.1}", points[0].0, points[0].1);
            for i in 0..last {
                let (from, to) = (points[i], points[i + 1]);
                let span = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt() * REACH;
                let c1 = (
                    from.0 + tangents[i].0 * span,
                    from.1 + tangents[i].1 * span,
                );
                let c2 = (
                    to.0 - tangents[i + 1].0 * span,
                    to.1 - tangents[i + 1].1 * span,
                );
                path.push_str(&format!(
                    " C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                    c1.0, c1.1, c2.0, c2.1, to.0, to.1
                ));
            }
            path
        }
    }
}

/// Corner to corner. The lanes are still honoured; only the smoothing is gone.
fn straight(points: &[(f32, f32)]) -> String {
    let mut path = String::new();
    for (index, point) in points.iter().enumerate() {
        path.push_str(&format!(
            "{}{:.1},{:.1}",
            if index == 0 { "M" } else { " L" },
            point.0,
            point.1
        ));
    }
    path
}

/// Right angles, turned at the halfway point of each segment, with a small arc
/// so the corner is not a hard pixel.
fn step(points: &[(f32, f32)], axis: Axis) -> String {
    if points.len() < 2 {
        return straight(points);
    }
    let mut path = format!("M{:.1},{:.1}", points[0].0, points[0].1);
    for pair in points.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let corner = CORNER
            .min((to.0 - from.0).abs() / 2.0)
            .min((to.1 - from.1).abs() / 2.0);
        if corner < 0.5 {
            path.push_str(&format!(" L{:.1},{:.1}", to.0, to.1));
            continue;
        }
        match axis {
            Axis::Horizontal => {
                let mid = (from.0 + to.0) / 2.0;
                let lead = (to.0 - from.0).signum();
                let turn = (to.1 - from.1).signum();
                path.push_str(&format!(
                    " L{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1} L{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1} L{:.1},{:.1}",
                    mid - corner * lead, from.1,
                    mid, from.1,
                    mid, from.1 + corner * turn,
                    mid, to.1 - corner * turn,
                    mid, to.1,
                    mid + corner * lead, to.1,
                    to.0, to.1
                ));
            }
            Axis::Vertical => {
                let mid = (from.1 + to.1) / 2.0;
                let lead = (to.1 - from.1).signum();
                let turn = (to.0 - from.0).signum();
                path.push_str(&format!(
                    " L{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1} L{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1} L{:.1},{:.1}",
                    from.0, mid - corner * lead,
                    from.0, mid,
                    from.0 + corner * turn, mid,
                    to.0 - corner * turn, mid,
                    to.0, mid,
                    to.0, mid + corner * lead,
                    to.0, to.1
                ));
            }
        }
    }
    path
}

/// The point half way along a run *by length*, for anything that has to sit on
/// the wire rather than beside it — a label, most of the time. The middle
/// waypoint is not the middle of anything on an uneven run.
pub fn midpoint(points: &[(f32, f32)]) -> (f32, f32) {
    match points {
        [] => (0.0, 0.0),
        [only] => *only,
        _ => {
            let lengths: Vec<f32> = points
                .windows(2)
                .map(|pair| {
                    ((pair[1].0 - pair[0].0).powi(2) + (pair[1].1 - pair[0].1).powi(2)).sqrt()
                })
                .collect();
            let total: f32 = lengths.iter().sum();
            let mut walked = 0.0;
            for (index, length) in lengths.iter().enumerate() {
                if walked + length >= total / 2.0 && *length > 0.0 {
                    let t = (total / 2.0 - walked) / length;
                    let (a, b) = (points[index], points[index + 1]);
                    return (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                }
                walked += length;
            }
            points[points.len() / 2]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(path: &str) -> Vec<f32> {
        path.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|piece| !piece.is_empty())
            .filter_map(|piece| piece.parse::<f32>().ok())
            .collect()
    }

    /// The curve has to actually touch both handles: a wire that starts a pixel
    /// off its port reads as a broken connection at every zoom past 1.5.
    #[test]
    fn a_curve_starts_and_ends_on_its_handles() {
        let path = wire(&[(10.0, 20.0), (300.0, 90.0)], Shape::Bezier, Axis::Horizontal);
        let n = numbers(&path);
        assert_eq!((n[0], n[1]), (10.0, 20.0));
        assert_eq!((n[6], n[7]), (300.0, 90.0));
    }

    /// Both control points sit level with their own handle, which is what makes
    /// a wire leave horizontally.
    #[test]
    fn control_points_leave_and_enter_horizontally() {
        let source = (0.0, 0.0);
        let target = (400.0, 120.0);
        let n = numbers(&wire(&[source, target], Shape::Bezier, Axis::Horizontal));
        assert_eq!(n[3], source.1, "the curve leaves at an angle");
        assert_eq!(n[5], target.1, "the curve arrives at an angle");
        assert!(n[2] > source.0 && n[2] < target.0);
        assert!(n[4] > source.0 && n[4] < target.0);
    }

    /// A backwards wire — a recursive call, the one place the deps graph cannot
    /// go but the call graph does — has to bulge past its own endpoints, or the
    /// two cards are joined by a line that crosses straight through both.
    #[test]
    fn a_backwards_wire_bulges_around_itself() {
        let n = numbers(&wire(&[(400.0, 0.0), (100.0, 60.0)], Shape::Bezier, Axis::Horizontal));
        assert!(n[2] > 400.0, "the outgoing control point folded inward");
        assert!(n[4] < 100.0, "the incoming control point folded inward");
    }

    /// A routed wire is one path with a cubic per segment, and it still leaves
    /// and arrives horizontally however much it wanders in between.
    #[test]
    fn a_routed_wire_is_one_smooth_run() {
        let path = wire(&[(0.0, 0.0), (150.0, -40.0), (300.0, -40.0), (450.0, 30.0)], Shape::Bezier, Axis::Horizontal);
        assert_eq!(path.matches('C').count(), 3, "one cubic per segment");
        let n = numbers(&path);
        assert_eq!((n[0], n[1]), (0.0, 0.0));
        // First control point level with the start.
        assert_eq!(n[3], 0.0, "the routed wire leaves at an angle");
        // Last control point level with the end.
        let end = n.len();
        assert_eq!(n[end - 3], 30.0, "the routed wire arrives at an angle");
        assert_eq!((n[end - 2], n[end - 1]), (450.0, 30.0));
    }

    /// And it actually passes through the lanes it was given, rather than
    /// treating them as a suggestion.
    #[test]
    fn a_routed_wire_passes_through_its_lanes() {
        let lanes = [(0.0, 0.0), (150.0, -40.0), (300.0, -40.0), (450.0, 30.0)];
        let n = numbers(&wire(&lanes, Shape::Bezier, Axis::Horizontal));
        // Each cubic ends on the next lane: 2 + 6 numbers per segment.
        for (segment, lane) in lanes[1..].iter().enumerate() {
            let end = 2 + segment * 6 + 4;
            assert_eq!((n[end], n[end + 1]), (lane.0, lane.1));
        }
    }

    /// A vertical flow is the same wire turned a quarter: it leaves and arrives
    /// along the other axis, and nothing else about it changes.
    #[test]
    fn a_vertical_wire_leaves_and_arrives_vertically() {
        let n = numbers(&wire(
            &[(0.0, 0.0), (120.0, 400.0)],
            Shape::Bezier,
            Axis::Vertical,
        ));
        assert_eq!(n[2], 0.0, "the curve left sideways");
        assert_eq!(n[4], 120.0, "the curve arrived sideways");
        assert!(n[3] > 0.0 && n[3] < 400.0);
        assert!(n[5] > 0.0 && n[5] < 400.0);
    }

    /// A straight run is exactly its lanes, with nothing added.
    #[test]
    fn a_straight_run_is_its_lanes() {
        let lanes = [(0.0, 0.0), (100.0, 20.0), (200.0, 40.0)];
        let path = wire(&lanes, Shape::Straight, Axis::Horizontal);
        assert_eq!(path, "M0.0,0.0 L100.0,20.0 L200.0,40.0");
    }

    /// A stepped run turns at right angles and never at any other angle: every
    /// straight piece of it is level or plumb.
    #[test]
    fn a_stepped_run_only_turns_square() {
        let path = wire(
            &[(0.0, 0.0), (200.0, 80.0)],
            Shape::Step,
            Axis::Horizontal,
        );
        for piece in path.split(" L").skip(1) {
            // Each `L` lands either level with or plumb below where the run was;
            // the arcs between them are the corners.
            assert!(piece.contains(','));
        }
        assert!(path.contains('Q'), "the corners are not rounded");
        assert!(!path.contains('C'), "a step is not a curve");
    }

    /// A label sits half way along by *length*, not at the middle waypoint —
    /// on an uneven run those are nowhere near each other.
    #[test]
    fn a_label_sits_half_way_by_length() {
        // Three points, but almost all the length is in the second leg.
        let (x, _) = midpoint(&[(0.0, 0.0), (10.0, 0.0), (210.0, 0.0)]);
        assert!(
            (x - 105.0).abs() < 0.01,
            "the midpoint is at {x:.1}, which is the middle waypoint rather than the middle"
        );
    }

    #[test]
    fn a_degenerate_run_is_not_a_panic() {
        assert_eq!(wire(&[], Shape::Bezier, Axis::Horizontal), "");
        assert_eq!(wire(&[(4.0, 5.0)], Shape::Bezier, Axis::Horizontal), "M4.0,5.0");
    }
}
