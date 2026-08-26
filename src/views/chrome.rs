//! The furniture both charts wear, and the browser facts that place it.
//!
//! Two things live here because neither belongs to one altitude: the ladder
//! between the charts, which no chart may own, and the plain-English plural
//! every cartouche and sheet counts in. The viewport probes are here for the
//! same reason — each chart reads them to inset its own furniture and to
//! stand its motion down when the reader asked it to, and the two charts
//! must read the same answer.

use dioxus::prelude::*;

use crate::Route;

pub(super) fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        return format!("{n} {word}");
    }
    // English, not `+ "s"`: `alias` takes `es`, and the cartouche was printing
    // `2 aliass`. Only the endings rust's own vocabulary actually hands us.
    let suffix = match word.chars().last() {
        Some('s') | Some('x') | Some('z') => "es",
        Some('h') if word.ends_with("ch") || word.ends_with("sh") => "es",
        _ => "s",
    };
    format!("{n} {word}{suffix}")
}

/// Which rung of the ladder a cartouche stands on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Altitude {
    /// `/dep` — crates on rings of hops.
    Deps,
    /// `/data` — the workspace's state, tiered into roots and what they hold.
    Data,
    /// `/fn` — the code that runs, tiered by how far it is from an entry point.
    Fns,
}

/// The altitude line: the ladder between the charts, and the only navigation
/// between them. The current rung is engraved solid; the other is a link.
#[component]
pub(super) fn AltitudeSwitch(at: Altitude) -> Element {
    let rung = |label: &'static str, to: Route, mine: Altitude| {
        rsx! {
            if at == mine {
                span { class: "text-ink underline underline-offset-4", "{label}" }
            } else {
                Link {
                    class: "text-ink-soft underline-offset-4 hover:text-ink hover:underline",
                    to,
                    "{label}"
                }
            }
        }
    };
    rsx! {
        p { class: "flex gap-3 font-data text-[9.5px] tracking-[0.14em] uppercase",
            {rung("dependencies", Route::DepOverview {}, Altitude::Deps)}
            {rung("data", Route::DataOverview {}, Altitude::Data)}
            {rung("functions", Route::FnOverview {}, Altitude::Fns)}
        }
    }
}

/// A phone-width viewport gets tighter chrome insets. Charts only render on
/// the client, so the server value is never hydrated against.
pub(super) fn narrow_viewport() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .map(|w| w < 640.0)
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Authored motion stands down when the reader asked it to.
pub(super) fn prefers_reduced_motion() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
            .flatten()
            .map(|m| m.matches())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// The window's inner size; `None` off the client, where nothing frames.
pub(super) fn window_size() -> Option<(f64, f64)> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let w = window.inner_width().ok()?.as_f64()?;
        let h = window.inner_height().ok()?.as_f64()?;
        Some((w, h))
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// How long a block has to keep the pointer before the chart answers it.
/// Short enough that a reader who stops never waits for the ink.
#[cfg(target_arch = "wasm32")]
const DWELL_MS: u32 = 90;
/// How long the chart keeps its last reading once the pointer is off every
/// block. A gap shorter than this is a gutter, not a question withdrawn.
#[cfg(target_arch = "wasm32")]
const GRACE_MS: u32 = 240;

/// A hover reading that has to be meant.
///
/// Hovering a block lights the wires that answer for it, and those wires run
/// right across the glass. Taken straight off `mouseenter`/`mouseleave` that
/// is a strobe: a pointer crossing the chart clears the reading in the gutter
/// between every two blocks — and between a holder and each block nested in
/// it — then lights a different one on the far side. Half a second of
/// ordinary travel measured seven changes of the lit set on the data chart,
/// four of them all the way to dark, each repainting most of the sheet.
///
/// So what the pointer is over and what the chart is lit for are two
/// different things, and this turns the first into the second: the chart
/// answers a pointer that has stopped, and holds its last answer across the
/// gutters on the way to the next block. Plates go on writing the raw signal;
/// the wire layers read the one this returns.
pub(super) fn use_settled<T: Clone + PartialEq + 'static>(
    over: Signal<Option<T>>,
) -> Signal<Option<T>> {
    let read: Signal<Option<T>> = use_signal(|| None);
    // One wait may commit, and it is the newest one. Matching on the value
    // instead is not enough: a pointer that crosses a gutter, a block and a
    // second gutter leaves two waits for `None` in flight, and the older one
    // comes due while the pointer is in the second gutter — darkening the
    // sheet on a pause that had already been forgiven.
    let latest: Signal<u64> = use_signal(|| 0);
    use_effect(move || {
        let next = over();
        let (mut read, mut latest) = (read, latest);
        if *read.peek() == next {
            return;
        }
        let mine = *latest.peek() + 1;
        latest.set(mine);
        #[cfg(target_arch = "wasm32")]
        {
            // Going dark waits longer than lighting up does: the pause the
            // pointer makes crossing a gutter must not read as an answer
            // withdrawn.
            let wait = if next.is_none() { GRACE_MS } else { DWELL_MS };
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(wait).await;
                if *latest.peek() == mine && *read.peek() != next {
                    read.set(next);
                }
            });
        }
        // Nothing hovers a server render, and a test wants the plain answer.
        #[cfg(not(target_arch = "wasm32"))]
        read.set(next);
    });
    read
}
