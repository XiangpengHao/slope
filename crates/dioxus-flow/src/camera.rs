//! The camera over a flow graph.
//!
//! The world holds still and the view travels. A camera is the same three
//! numbers a flow canvas has always been — a translation and a magnification,
//! written straight into one CSS transform — so panning and zooming a hundred
//! nodes costs one string, not a re-layout.
//!
//! Three rules the rest of the file exists to serve:
//!
//! - **Zoom anchors on the point being aimed at**, never the viewport centre.
//!   Centre-anchored zoom throws away whatever the reader was looking at.
//! - **A notch is exactly ±1.** Magnification is held as an exponent, so a wheel
//!   notch out and back returns to the same pixels while a pinch still lands
//!   anywhere between notches.
//! - **A flight interpolates magnification geometrically.** Half way through a
//!   move the camera sits at the geometric mean of the two magnifications; the
//!   arithmetic mean reads as a lurch at the far end.

/// One wheel notch, as a magnification ratio.
pub const ZOOM_STEP: f32 = 1.2;
pub const ZOOM_MIN: f32 = 0.06;
pub const ZOOM_MAX: f32 = 2.6;

/// How long the camera takes to cross the graph.
pub const FLIGHT_MS: f64 = 460.0;

/// How the reader is looking at the graph.
///
/// `screen = world * scale + (x, y)`, which is exactly the transform the
/// viewport layer carries.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    /// Magnification as an exponent of [`ZOOM_STEP`]. A notch adds exactly ±1.0,
    /// which is exact in binary floating point; a pinch adds a fraction.
    pub exp: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            exp: 0.0,
        }
    }
}

fn exp_of(scale: f32) -> f32 {
    scale.clamp(ZOOM_MIN, ZOOM_MAX).ln() / ZOOM_STEP.ln()
}

impl Camera {
    pub fn scale(&self) -> f32 {
        ZOOM_STEP.powf(self.exp).clamp(ZOOM_MIN, ZOOM_MAX)
    }

    /// The CSS transform for the viewport layer. Translation first, then scale:
    /// the node layer is laid out in world units and the camera is the only
    /// thing that ever touches this string.
    pub fn transform(&self) -> String {
        format!(
            "translate({:.3}px, {:.3}px) scale({:.5})",
            self.x,
            self.y,
            self.scale()
        )
    }

    pub fn to_world(self, sx: f32, sy: f32) -> (f32, f32) {
        let scale = self.scale();
        ((sx - self.x) / scale, (sy - self.y) / scale)
    }

    /// At a magnification limit, where zooming further does nothing. The
    /// controls say so rather than staying live and refusing.
    pub fn at_limit(self, closer: bool) -> bool {
        let scale = self.scale();
        if closer {
            scale >= ZOOM_MAX - 1e-4
        } else {
            scale <= ZOOM_MIN + 1e-4
        }
    }

    /// The world point sitting at the centre of a viewport of this size.
    pub fn centre(&self, viewport: (f32, f32)) -> (f32, f32) {
        self.to_world(viewport.0 / 2.0, viewport.1 / 2.0)
    }

    /// Slide the world under the camera by a screen-space delta.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    /// Zoom by `notches`, holding the world point under `anchor` still.
    ///
    /// `anchor` is in CSS pixels within the pane. Returns whether the
    /// magnification actually changed — at a limit it must not, because moving
    /// the translation while the scale stays put drifts the world sideways.
    pub fn zoom_by(&mut self, notches: f32, anchor: (f32, f32)) -> bool {
        if !notches.is_finite() || notches == 0.0 {
            return false;
        }
        let before = self.scale();
        let (wx, wy) = self.to_world(anchor.0, anchor.1);

        let wanted = self.exp + notches;
        self.exp = wanted.clamp(exp_of(ZOOM_MIN), exp_of(ZOOM_MAX));
        let after = self.scale();
        if after == before {
            return false;
        }

        self.x = anchor.0 - wx * after;
        self.y = anchor.1 - wy * after;
        true
    }

    /// Zoom by a continuous ratio — a trackpad pinch, which lands between
    /// notches rather than on one.
    pub fn zoom_by_ratio(&mut self, ratio: f32, anchor: (f32, f32)) -> bool {
        if !ratio.is_finite() || ratio <= 0.0 {
            return false;
        }
        self.zoom_by(ratio.ln() / ZOOM_STEP.ln(), anchor)
    }

    /// Put a world point at the centre of the viewport at a given magnification.
    pub fn looking_at(centre: (f32, f32), scale: f32, viewport: (f32, f32)) -> Self {
        let scale = scale.clamp(ZOOM_MIN, ZOOM_MAX);
        Self {
            x: viewport.0 / 2.0 - centre.0 * scale,
            y: viewport.1 / 2.0 - centre.1 * scale,
            exp: exp_of(scale),
        }
    }

    /// Frame a world rectangle inside a viewport, with `padding` screen pixels
    /// of air around it.
    ///
    /// `floor` is the magnification below which framing stops being worth doing:
    /// a graph squeezed until its cards are 8px of grey is a picture of a graph,
    /// not a graph. Below the floor the frame overflows instead, which is the
    /// honest outcome — there is more here than fits, and the reader pans to it.
    ///
    /// `anchor_left` puts the left edge of the bounds at the padding rather than
    /// centring, for the one case where the reader knows which end to start at:
    /// a graph that reads left to right, opened at its beginning.
    pub fn framing(
        bounds: Bounds,
        viewport: (f32, f32),
        padding: f32,
        floor: f32,
        anchor_left: bool,
    ) -> Self {
        let (w, h) = (bounds.width().max(1.0), bounds.height().max(1.0));
        let usable = (
            (viewport.0 - padding * 2.0).max(64.0),
            (viewport.1 - padding * 2.0).max(64.0),
        );
        // Never zoom *in* to fit: two cards blown up to fill the pane read as a
        // different product than the same two cards at rest.
        let scale = (usable.0 / w).min(usable.1 / h).clamp(floor.min(1.0), 1.0);
        let mut camera = Self::looking_at(bounds.centre(), scale, viewport);
        if anchor_left && w * camera.scale() > usable.0 {
            camera.x = padding - bounds.min_x * camera.scale();
        }
        camera
    }
}

/// A world rectangle.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Bounds {
    pub fn of(rects: impl IntoIterator<Item = (f32, f32, f32, f32)>) -> Option<Self> {
        let mut out: Option<Self> = None;
        for (x, y, w, h) in rects {
            let next = Self {
                min_x: x,
                min_y: y,
                max_x: x + w,
                max_y: y + h,
            };
            out = Some(match out {
                None => next,
                Some(b) => Self {
                    min_x: b.min_x.min(next.min_x),
                    min_y: b.min_y.min(next.min_y),
                    max_x: b.max_x.max(next.max_x),
                    max_y: b.max_y.max(next.max_y),
                },
            });
        }
        out
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn centre(&self) -> (f32, f32) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
}

/// A camera move. The only authored motion in the product: giving it weight is
/// what makes "the view travelled there" legible as motion rather than as the
/// world having jumped.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flight {
    /// Where the camera actually was when this began, captured as values rather
    /// than as a `Camera` — a flight started mid-flight would otherwise take off
    /// from the previous flight's destination and visibly jump.
    from: (f32, f32, f32),
    to: (f32, f32, f32),
    viewport: (f32, f32),
    started: f64,
}

impl Flight {
    pub fn new(from: &Camera, to: &Camera, viewport: (f32, f32), started: f64) -> Self {
        let a = from.centre(viewport);
        let b = to.centre(viewport);
        Self {
            from: (a.0, a.1, from.scale()),
            to: (b.0, b.1, to.scale()),
            viewport,
            started,
        }
    }

    /// Where the camera is, part-way through.
    pub fn at(&self, time: f64) -> Camera {
        let t = (((time - self.started) / FLIGHT_MS) as f32).clamp(0.0, 1.0);
        let e = ease(t);
        let (a, b) = (self.from.2, self.to.2);
        Camera::looking_at(
            (
                self.from.0 + (self.to.0 - self.from.0) * e,
                self.from.1 + (self.to.1 - self.from.1) * e,
            ),
            a * (b / a).powf(e),
            self.viewport,
        )
    }

    pub fn landed(&self, time: f64) -> bool {
        time - self.started >= FLIGHT_MS
    }
}

/// Exponential ease-out: quick to commit, slow to arrive, so the eye keeps hold
/// of where it came from.
pub fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[cfg(target_arch = "wasm32")]
pub fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> f64 {
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: (f32, f32) = (1440.0, 820.0);

    /// The property the whole navigation model leans on: a notch out and a notch
    /// back lands on the same pixels. Holding magnification as an exponent is
    /// what makes it exact — a pair of ratios like 0.86 and 1.16 is not
    /// reciprocal and drifts a fraction of a percent every pair.
    #[test]
    fn a_notch_out_and_back_is_exactly_reversible() {
        let mut camera = Camera {
            x: 120.0,
            y: -40.0,
            exp: 0.0,
        };
        let start = camera;
        assert!(camera.zoom_by(1.0, (410.0, 260.0)));
        assert!(camera.zoom_by(-1.0, (410.0, 260.0)));
        assert_eq!(camera.exp, start.exp);
        assert!((camera.x - start.x).abs() < 0.001);
        assert!((camera.y - start.y).abs() < 0.001);
    }

    /// Zoom holds the point under the pointer still. Anchoring on the centre
    /// instead throws away whatever the reader was aiming at.
    #[test]
    fn zoom_holds_the_anchored_point_still() {
        let mut camera = Camera {
            x: 0.0,
            y: 0.0,
            exp: 0.0,
        };
        let anchor = (400.0f32, 700.0f32);
        let before = camera.to_world(anchor.0, anchor.1);
        camera.zoom_by(3.0, anchor);
        let after = camera.to_world(anchor.0, anchor.1);
        assert!((before.0 - after.0).abs() < 0.01, "the anchor slid sideways");
        assert!(
            (before.1 - after.1).abs() < 0.01,
            "the anchor slid vertically"
        );
    }

    /// A control that cannot do anything says so. Before this the stylesheet
    /// carried a disabled state that nothing in the product could ever reach.
    #[test]
    fn the_camera_knows_when_it_is_at_a_limit() {
        let mut camera = Camera::default();
        assert!(!camera.at_limit(true) && !camera.at_limit(false));
        camera.zoom_by(100.0, (0.0, 0.0));
        assert!(camera.at_limit(true), "the camera is as close as it goes");
        assert!(!camera.at_limit(false));
        camera.zoom_by(-200.0, (0.0, 0.0));
        assert!(camera.at_limit(false), "the camera is as far out as it goes");
        assert!(!camera.at_limit(true));
    }

    #[test]
    fn a_pinch_lands_between_notches() {
        let mut camera = Camera::default();
        camera.zoom_by_ratio(1.07, (200.0, 200.0));
        assert!((camera.scale() - 1.07).abs() < 0.0001);
        assert!(camera.exp.fract().abs() > 0.0001, "a pinch quantised to a notch");
    }

    #[test]
    fn zoom_clamped_at_a_limit_does_not_drift_the_world() {
        let mut camera = Camera {
            x: 50.0,
            y: 50.0,
            exp: exp_of(ZOOM_MAX),
        };
        let before = camera;
        assert!(!camera.zoom_by(40.0, (10.0, 10.0)));
        assert_eq!(camera.x, before.x);
        assert_eq!(camera.y, before.y);
    }

    #[test]
    fn fitting_frames_the_whole_graph_and_centres_it() {
        let bounds = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 2000.0,
            max_y: 1000.0,
        };
        let camera = Camera::framing(bounds, VIEW, 40.0, ZOOM_MIN, false);
        let centre = camera.centre(VIEW);
        assert!((centre.0 - 1000.0).abs() < 0.01);
        assert!((centre.1 - 500.0).abs() < 0.01);
        // The wide axis binds: (1440 - 80) / 2000.
        assert!((camera.scale() - 0.68).abs() < 0.001);
    }

    /// A small graph is framed at its own size rather than blown up to fill the
    /// pane — magnifying two nodes to fill a desktop reads as a different tool.
    #[test]
    fn fitting_never_magnifies_past_life_size() {
        let bounds = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 200.0,
            max_y: 60.0,
        };
        assert!((Camera::framing(bounds, VIEW, 40.0, ZOOM_MIN, false).scale() - 1.0).abs() < 0.0001);
    }

    /// The floor is what keeps a first viewport readable. A wide graph is not
    /// crushed to fit; it overflows, and the reader pans.
    #[test]
    fn framing_stops_shrinking_at_the_floor() {
        let wide = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 6000.0,
            max_y: 300.0,
        };
        let camera = Camera::framing(wide, VIEW, 72.0, 0.85, true);
        assert!((camera.scale() - 0.85).abs() < 0.0001);
        // Left-anchored, so the beginning of the graph is what the reader sees.
        assert!((camera.x - 72.0).abs() < 0.01, "the graph did not start at the padding");
    }

    /// A frame that does fit is centred, anchor or no anchor: anchoring is for
    /// overflow, not a permanent leftward bias.
    #[test]
    fn framing_centres_whatever_actually_fits() {
        let small = Bounds {
            min_x: 100.0,
            min_y: 100.0,
            max_x: 900.0,
            max_y: 500.0,
        };
        let camera = Camera::framing(small, VIEW, 72.0, 0.85, true);
        let centre = camera.centre(VIEW);
        assert!((centre.0 - 500.0).abs() < 0.01 && (centre.1 - 300.0).abs() < 0.01);
    }

    #[test]
    fn a_flight_leaves_where_it_started_and_arrives_where_it_aimed() {
        let from = Camera::looking_at((0.0, 0.0), 0.5, VIEW);
        let to = Camera::looking_at((900.0, 300.0), 1.4, VIEW);
        let flight = Flight::new(&from, &to, VIEW, 1000.0);

        let start = flight.at(1000.0);
        assert!((start.scale() - 0.5).abs() < 0.001);
        assert!((start.centre(VIEW).0 - 0.0).abs() < 0.01);

        let end = flight.at(1000.0 + FLIGHT_MS);
        assert!((end.scale() - 1.4).abs() < 0.001);
        assert!((end.centre(VIEW).0 - 900.0).abs() < 0.05);
        assert!((end.centre(VIEW).1 - 300.0).abs() < 0.05);
        assert!(flight.landed(1000.0 + FLIGHT_MS));
    }

    /// Magnification interpolates geometrically, not arithmetically. Compared at
    /// the *same* eased progress — the ease is exponential, so half the clock is
    /// already most of the way there, and comparing against the plain midpoint
    /// measures the easing rather than the interpolation.
    #[test]
    fn a_flight_interpolates_magnification_geometrically() {
        let from = Camera::looking_at((0.0, 0.0), 0.25, VIEW);
        let to = Camera::looking_at((0.0, 0.0), 2.0, VIEW);
        let flight = Flight::new(&from, &to, VIEW, 0.0);
        for fraction in [0.25f32, 0.5, 0.75] {
            let scale = flight.at(FLIGHT_MS * fraction as f64).scale();
            let e = ease(fraction);
            let geometric = 0.25 * (2.0f32 / 0.25).powf(e);
            let arithmetic = 0.25 + (2.0 - 0.25) * e;
            assert!(
                (scale - geometric).abs() < 0.01,
                "at {fraction} the scale is {scale}, not the geometric {geometric}"
            );
            assert!(scale < arithmetic, "the move lurches at the far end");
        }
    }

    #[test]
    fn panning_moves_by_screen_pixels() {
        let mut camera = Camera::looking_at((0.0, 0.0), 2.0, VIEW);
        let before = camera.centre(VIEW);
        camera.pan(100.0, -50.0);
        let after = camera.centre(VIEW);
        assert!((after.0 - (before.0 - 50.0)).abs() < 0.001);
        assert!((after.1 - (before.1 + 25.0)).abs() < 0.001);
    }

    /// The CSS transform the viewport layer carries and the arithmetic the pane
    /// hit-tests with have to be the same transform, or a click lands somewhere
    /// other than where the reader aimed.
    #[test]
    fn screen_space_and_the_css_transform_agree() {
        let camera = Camera {
            x: -320.5,
            y: 88.25,
            exp: 2.5,
        };
        let scale = camera.scale();
        assert_eq!(
            camera.transform(),
            format!("translate(-320.500px, 88.250px) scale({scale:.5})")
        );
        // The inverse of `world * scale + offset`, which is what that string is.
        let (wx, wy) = camera.to_world(410.0 * scale - 320.5, -60.0 * scale + 88.25);
        assert!((wx - 410.0).abs() < 0.01 && (wy + 60.0).abs() < 0.01);
    }
}
