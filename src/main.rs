use dioxus::prelude::*;

mod api;
mod components;
mod graph;
mod views;

use views::{Board, Shell};

/// Internal routes. The shell is the lens frame: the dependency board is the
/// first lens, and later lenses mount as siblings here rather than as tabs
/// bolted onto this one.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Shell)]
        #[route("/")]
        Board {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// The direction contract. Kept as a comment in the emitted markup so it
/// survives the production build and can be audited against the render.
const DIRECTION_CONTRACT: &str = r#"<!--
THESIS: The workspace as a bare two-layer board — uniform gold pads wired by routed copper, one fixed world a camera flies across. Refuses the force-directed hairball and the re-laying-out focus view alike.
OWN-WORLD: Solder-mask green ground #0A1E14, ENIG gold pads, etched copper at full density, silkscreen legend in Archivo at 78% width. Gold enters from the left (what depends on this); blue leaves to the right (what this depends on). Geometry never changes — only ink.
STORY: A developer who ran `cargo tree` and still cannot see why a crate is present flies to its pad, reads the traces entering it, and steps up the chain that put it there.
FIRST VIEWPORT: The board as a cut object on a darker surround — routed outline, radiused corners, mounting holes — workspace pads at its left edge, columns marching right, every trace composed at once; a position register pinned bottom-left that never scales at any zoom.
FORM: Wire-wrap backplane and PCB fabrication artwork; candidate 1 of 7, taken over the roll by the user; seed key ba7c2295.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
-->"#;

fn main() {
    // `rust-viewer <path>` resolves that workspace, then serves the board.
    // Anything flag-shaped belongs to dx or the runtime, not to us.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let target = std::env::args()
            .skip(1)
            .find(|a| !a.starts_with('-'))
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
        // Archivo carries a width axis, so the silkscreen legend and the
        // document chrome are one family at two widths. JetBrains Mono is the
        // code register, and the face the reader already has open next door.
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Stylesheet { href: "https://fonts.googleapis.com/css2?family=Archivo:wdth,wght@62..125,400..700&family=JetBrains+Mono:wght@400;500;700&display=swap" }
        document::Stylesheet { href: TAILWIND_CSS }

        Router::<Route> {}
    }
}
