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
