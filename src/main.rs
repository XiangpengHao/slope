use dioxus::prelude::*;

mod api;
mod call;
mod components;
mod graph;
mod views;

use views::{Calls, Deps, Shell};

/// Internal routes. The shell is the lens frame: the dependency graph is the
/// first lens, the call graph the second, and later lenses mount as siblings
/// here rather than as tabs bolted onto either.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Shell)]
        #[route("/")]
        Deps {},
        #[route("/calls")]
        Calls {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// The direction contract. Kept as a comment in the emitted markup so it
/// survives the production build and can be audited against the render.
const DIRECTION_CONTRACT: &str = r#"<!--
THESIS: A Rust workspace read as a flow chart you open one hop at a time. Cards are crates, wires are dependencies, and the two numbered ports on every card say exactly how much is still folded behind it. Refuses the whole-graph picture — 346 nodes and 1175 edges at once is a texture, not a diagram — and refuses selection that re-lays-out the world.
OWN-WORLD: Paper-white pane #F3F5F9 under a dot lattice that climbs a 20/40/80/160/320-unit ladder as the camera pulls back, so the ground never smears and never disappears; white cards with a hairline #7F8C9F border and 8px corners; crates this workspace builds are the same card filled with ink #14171D. On the pane, hue is spent on direction and nothing else: rust #BF3D10 runs into the crate you hold, blue #1D4ED8 runs out of it. The chrome keeps exactly one borrowed use of that blue — the focus ring, which is the one mark that has to read on the paper card and the ink one alike; every other action in the chrome is a word in ink. System UI face, mono for every version, path and signature. No webfont: a local tool renders its first frame without a CDN.
STORY: A developer who ran `cargo tree` and still cannot see why a crate is in their build finds it, watches the chain that pulled it in arrive as a lit route, then opens the two ports on either side of it a hop at a time until the answer is on screen.
FIRST VIEWPORT: The crates this workspace builds as ink cards at the left edge, their direct dependencies stepping out to the right one column per hop of longest path — a chain, not a fan, because the column law is a depth and not a layout convenience — with more of the graph continuing past the right edge; zoom and frame controls bottom-left, a map of the graph bottom-right, and the key to every mark docked in the panel on the right until something is held.
SIGNATURE INTERACTION: Asking for a crate that is not on the pane brings the chain that put it in the build with it, drawn as a walking dashed wire and framed end to end — the one camera move allowed to shrink until both ends of the chain are on screen.
MOTION: One authored moment — the camera flight, 460ms, exponential ease-out, magnification interpolated geometrically. A card arriving fades up over 180ms and a route's dash walks. Nothing else moves, and a reader who asked their system for less motion gets the destination instead of the flight.
FORM: The category standard, played straight. The user pinned React Flow's nodes-and-edges canvas and named React Flow's own examples and tldraw/Figma canvas feel as the craft bar, which is the standing exit taken in the user's own words — so no direction roll was dealt, and this is the canon executed at full fidelity rather than a costume over it. Code-led: there is no approved comp, and the promises above are audited in behaviour.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
-->"#;

fn main() {
    // `rust-viewer <path>` resolves that workspace, then serves the app.
    // Anything flag-shaped belongs to dx or the runtime, not to us.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let target = std::env::args()
            .skip(1)
            .find(|argument| !argument.starts_with('-'))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
        eprintln!("rust-viewer: reading {}", target.display());
        graph::metadata::set_target(target);
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // First child of body, so a production build still carries it.
        div { dangerous_inner_html: DIRECTION_CONTRACT }

        document::Link { rel: "icon", href: FAVICON }
        // The pane's own stylesheet, then this product's, which themes it.
        document::Stylesheet { href: dioxus_flow::STYLESHEET }
        document::Stylesheet { href: TAILWIND_CSS }

        Router::<Route> {}
    }
}
