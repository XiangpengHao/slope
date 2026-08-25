use dioxus::prelude::*;

use views::{AppShell, DataFocus, DataModFocus, DataOverview, DepFocus, DepOverview, DepRing};

/// Server-side analysis: cargo metadata, VCS diff, manifest events.
#[cfg(feature = "server")]
mod analyze;
/// The data model: the graph structures the charts draw.
mod graph;
/// The server functions that carry a graph to the client.
mod load;
/// Layouts and route pages.
mod views;

/// Internal routes. Each variant is a URL pattern; the matching component is
/// rendered when that pattern matches. Every chart selection is a URL, so
/// the browser's back button retraces the review trail. One family per
/// altitude, each named for what it charts: `/dep` is the dependency
/// altitude — crates on rings of hops, a multi-selection joining crate names
/// with `+` (impossible in a crate name), a whole ring as `/dep/ring/:hop`.
/// `/data` is the data altitude: every struct, enum, union and static the
/// workspace keeps, tiered into roots and the state nested inside them;
/// selecting one is `/data/mark/:..path?item=` (its defining file, then its
/// label), and a whole module boundary is `/data/mod/:..module`. A selection
/// may also carry `peek=<file>@<label>`: one row of its sheet opened as a
/// quotation of its own source, for the ends this chart draws no block for —
/// a function, a trait, a method. `/` is the dependency chart, the rung a
/// review starts on.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(AppShell)]
        #[redirect("/", || Route::DepOverview {})]
        #[route("/dep")]
        DepOverview {},
        #[route("/dep/crate/:name")]
        DepFocus { name: String },
        #[route("/dep/ring/:hop")]
        DepRing { hop: u32 },
        #[route("/data")]
        DataOverview {},
        // The query fields are declared in the order the pattern names them:
        // the router's macro walks the two lists together. `peek` leads so an
        // unquoted selection's URL is exactly what it always was.
        #[route("/data/mark/:..path?:peek&:item")]
        DataFocus { path: Vec<String>, peek: Option<String>, item: String },
        #[route("/data/mod/:..module")]
        DataModFocus { module: Vec<String> },
}

/* impeccable direction contract — served inside the page, greppable in the
built output via the hidden div in `App`. */
const CONTRACT: &str = "<!--
THESIS: A cargo workspace charted as an engraved star atlas; refuses the glowing graph-dashboard with sidebar filters.
OWN-WORLD: Paper #f6f4ed, engraving ink #23303c, hairline constellation lines; EB Garamond spaced caps for chart furniture, JetBrains Mono for crate data; amber #a54c06 exists only as CHANGED and its fading blast halo; double-ruled paper plates for panels.
STORY: A reviewer opens the chart after an agent session, sees the workspace whole, reads which stars flare, follows the halo to judge blast radius, and descends crate by crate - every focus a URL.
FIRST VIEWPORT: Full-bleed chart; cartouche top-left with workspace name, epoch, change count; search top-right; rings captioning their own hops and every mark carrying its state in words at the mark; changed stars flaring amber mid-chart.
FORM: Star atlas, candidate 7 of 7 grounded directions, seed 93a80ceb.
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
-->";

/// An ink star on paper, as the tab icon.
const FAVICON_SVG: &str = "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><rect width='32' height='32' fill='%23f6f4ed'/><circle cx='16' cy='16' r='5' fill='%2323303c'/><circle cx='16' cy='16' r='8' fill='none' stroke='%2323303c' stroke-width='0.8'/><g stroke='%23a54c06' stroke-width='1.4' stroke-linecap='round'><line x1='16' y1='2' x2='16' y2='6'/><line x1='16' y1='26' x2='16' y2='30'/><line x1='2' y1='16' x2='6' y2='16'/><line x1='26' y1='16' x2='30' y2='16'/></g></svg>";

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const FONT_CHART_400: Asset = asset!("/assets/fonts/eb-garamond-400.woff2");
const FONT_CHART_400I: Asset = asset!("/assets/fonts/eb-garamond-400-italic.woff2");
const FONT_CHART_600: Asset = asset!("/assets/fonts/eb-garamond-600.woff2");
const FONT_DATA_400: Asset = asset!("/assets/fonts/jetbrains-mono-400.woff2");
const FONT_DATA_500: Asset = asset!("/assets/fonts/jetbrains-mono-500.woff2");

/// `@font-face` rules pointing at the bundled font assets. Built here so the
/// URLs go through the asset pipeline like everything else.
fn font_css() -> String {
    let faces = [
        ("EB Garamond", 400, "normal", &FONT_CHART_400),
        ("EB Garamond", 400, "italic", &FONT_CHART_400I),
        ("EB Garamond", 600, "normal", &FONT_CHART_600),
        ("JetBrains Mono", 400, "normal", &FONT_DATA_400),
        ("JetBrains Mono", 500, "normal", &FONT_DATA_500),
    ];
    faces
        .iter()
        .map(|(family, weight, style, asset)| {
            format!(
                "@font-face {{ font-family: '{family}'; font-weight: {weight}; \
                 font-style: {style}; font-display: swap; \
                 src: url('{asset}') format('woff2'); }}"
            )
        })
        .collect()
}

/// rust-analyzer's crates trace every query they run at INFO — a line per
/// type inference, per path resolution — which buries everything else while
/// the code survey runs. These directives keep them, their query engine, and
/// their trait solver quiet unless something actually goes wrong. They are
/// appended to `RUST_LOG` rather than replacing it, so a more specific
/// directive (`RUST_LOG=ra_ap_hir_ty=info`) still wins: `EnvFilter` matches
/// the longest target prefix first.
#[cfg(feature = "server")]
const QUIET_SURVEY: &str = "ra_ap=warn,salsa=warn,chalk=warn";

fn main() {
    #[cfg(feature = "server")]
    {
        let base = std::env::var("RUST_LOG")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "info,slope=debug".to_string());
        // SAFETY: first statement of `main`; no other thread exists yet to
        // read the environment.
        unsafe { std::env::set_var("RUST_LOG", format!("{base},{QUIET_SURVEY}")) };
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div { style: "display:none", dangerous_inner_html: CONTRACT }
        document::Link { rel: "icon", href: FAVICON_SVG }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Style { {font_css()} }
        document::Title { "slope — workspace atlas" }

        Router::<Route> {}
    }
}
