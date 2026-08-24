# Dependency viewer — design brief

Confirmed via /impeccable shape, 2026-08-18. Built the same day.

**Vocabulary rework (2026-08-19, audit):** every invented label on this chart
was replaced by the word rust, cargo, or the VCS already uses. Nothing about
the geometry, the rings, the edges, or the interactions changed.

- "epoch" is gone from the surface. The cartouche reads
  `diff main @ 1a2b3c4 → working copy`; the code cartouche carries the same
  line. The internal `Epoch` type keeps its name.
- State words are counts and nouns, lowercase: `3 files changed`,
  `2 hops downstream`, `removed`, `added`, `1.0 → 1.2` (the arrow form is
  unchanged). The queue's per-crate badges read `3 files` and `Cargo.toml`;
  the focus panel reads `Cargo.toml changed — its dependency list`.
- `EXT` is gone: an external crate carries its version (`v1.0.229`), as
  `cargo tree` prints it. `DEV` / `BUILD` became `(dev)` / `(build)`, again
  `cargo tree`'s own output, carried on the focus panel's rows beside the
  crates they tag.
- The EDGES toggle is captioned `dependencies`, and its third reading is
  `reverse deps` — it is `cargo tree -i`.
- The fact sheet's `deps` row is `dependencies`.
- The loading screen leads with what is actually running: "cargo metadata is
  resolving the dependency graph".
- Counts left the Garamond voice: the cartouche's subtitle and every state
  line are mono, because Garamond never carries a count.

**Furniture distilled (2026-08-18, same day):** the resting chart carried
seven floating plates. Cut to four, without losing an affordance:

- The cartouche and the changes queue are one plate. They were always one
  thought — what this workspace is, and what changed in it — and stating
  the epoch twice (once as a title-block line, once as the queue's
  clean-epoch message) said nothing the second time. The changed count
  lives in the "Changes (n)" heading, the affected count in the queue's
  footer beside the seen ratio, and each fact appears once.
- The bottom hint plate is gone. Every sentence it carried was already on
  the key, and the flag that dismissed it is gone with it.
- The EDGES toggle moved out of its standing plate and into the selection's
  own panel. It has nothing to act on without a selection, and on the
  overview two of its three readings are dead — the root has no dependents
  and no path to itself.
- The key lost its gesture manual. Clicking a star, clicking a ring,
  dragging and scrolling are learned by doing, and the ring and toggle
  already carry tooltips; what stayed is what nothing on the plate reveals:
  ctrl-click, and the keys (`/`, `n`/`p`, `f`, esc). The paragraph that
  restated the two line samples above it is gone. The key now takes the
  left column's leftover height instead of a guessed cap that clipped its
  own last line.
- The panels trade three ways of saying where you came from — a back
  button, a "whole chart" link, and a trail line — for one breadcrumb:
  `← whole chart → a → b`, every step a link. The panel's heading names
  where the review stands, so the trail never repeats it.
- A selection panel on a phone is capped on the plate itself, not on a
  wrapper that never clipped it.

**Layout rework (2026-08-18, "Dependency Rings"):** the bloom/column chart
below was replaced by a radial chart. This section supersedes the "One
Living Chart" interaction notes and the Interaction and layout section.

- The chart is concentric rings. The crate under review sits at the
  center — the workspace's root crate by default; a virtual workspace (no
  root package) draws a small workspace medallion instead. Every other
  crate sits on the ring of its dependency distance: ring one is what the
  center depends on directly, ring two is what those need, and so on.
- Ring is minimum hop distance (BFS), so a diamond dependency appears
  once, as close to the center as it truly is. Edges may therefore skip
  rings or point back inward; that is information, not error.
- Angles follow the BFS tree: the circle divides among subtrees by leaf
  count, each crate seats at the middle of its sector, children subdivide
  it — a lineage shares a slice of sky. Crowded rings push their radius
  outward (tenth-percentile angular gap, capped). Placement is a pure
  function of (graph, center): deterministic, no physics, and no star
  ever moves.
- Members the center never reaches still seat on ring one, hung from the
  workspace itself. Ghost stars (removed deps) hang just past the crate
  that dropped them.
- The whole resolved graph is on the chart as stars (dots sized by
  magnitude); what is never drawn at once is the edges. **Edges are drawn
  only for the selected crate**: ink lines to what it depends on,
  hairlines from what depends on it, dash still meaning dev/build.
  Manifest-event edges (added / removed / bumped) are always drawn, in
  flare. Default selection is the center crate, so the first paint is the
  root and its direct spokes.
- Selection is the focus route (`/dep/crate/:name`); every star is a link;
  clicking the selected star (or back / Esc) returns to `/dep`. The camera
  frames the whole rings on `/dep` and the selection's neighborhood on a
  focus. With the root at the center, change travels inward: a changed
  star's blast radius points at the center.
- Ring cap (2026-08-18, same day): the chart draws at most four rings at
  rest; everything farther collapses onto the outermost ring as one "4+
  hops" band, so the plate stays compact and the ring count is bounded.
  Selecting a crate inside the band expands exact rings down to its true
  depth (the cap follows the deepest selected crate); deselecting folds
  them back. Angles never depend on the cap, so expansion only slides
  stars radially outward — nothing swings sideways. Tooltips always tell a
  star's true distance.
- Camera (same day): deselecting never moves the camera — the reviewer
  keeps the view they had. Only the first paint and selections frame.
- The edge toggle defaults to "depends on" (the compact reading);
  dependents and the path to the root are one press away.
- Selection extensions (2026-08-18, same day): the selection is a set.
  Ctrl / cmd / shift-click toggles a star in or out of it; the whole set
  is the URL (`/dep/crate/a+b+c` — `+` cannot appear in a crate name), so
  multi-selections stay shareable and back-button-retracable. Clicking a
  ring line itself selects every crate on that ring (`/dep/ring/:hop`).
  Middle-click on any star still opens its solo focus in a new tab. An
  edges toggle (depends on / used by / reverse deps) draws one reading of
  the selection's edges; manifest events are always drawn regardless. A
  multi-selection gets a roster panel (each crate removable), a ring
  selection gets the ring's roster; every edge carries an arrowhead
  pointing the way change travels — into the crate that uses the
  dependency.
- Reverse deps (2026-08-18, same day; named "path to root" until the
  2026-08-19 vocabulary rework): the third edges reading answers
  "why is this crate here?". It lights every route from the root down to
  the selection — what depends on it, then what depends on those, hop by
  hop until the chain runs out of users, which is where the root sits.
  Every hop draws in the dependents grammar (hairline, arrow into the
  user), so a long chain stays quiet on the paper, and every crate on a
  route is named at rest. A dependency the epoch removed is not a route:
  it still draws as a manifest event, but the crate that dropped it does
  not light the chain above it. This reading replaced "both", which was
  only the two other readings at once and said nothing they did not.
- Ring as control (2026-08-18, same day): the hop captions ("1 hop", "4+
  hops") are gone. The ring hairline is the control — hovering or focusing
  it raises the line to full ink, selecting dashes it, and its tooltip
  names the ring and what clicking does. A wide invisible stroke over the
  same circle is the target, so the ring's interior stays open paper and
  panning that starts on a ring still pans. The captions were furniture
  that said what the ring's own geometry already says; the reader's
  distance question is answered by every star's tooltip and by the ring
  roster's heading.
- Expanding and folding are drawn, not jumped (2026-08-18, same day): the
  router remounts the chart when the route variant changes, which threw
  the old geometry away and made a cap change land as a jump. The chart
  now remembers the cap it last painted, paints that geometry for one
  frame, then slides to the new one — stars glide radially and ring radii
  ease over 400ms, the same beat and curve as the camera. Edges are
  redrawn at the settled geometry and ink in over the same beat instead of
  snapping taut. `prefers-reduced-motion` skips straight to the new
  geometry.
- Several versions of one crate (2026-08-18, same day): cargo can resolve
  a 1.x and a 2.x of the same crate at once, and each is its own star on
  the ring of its own distance. Those stars carry their version beside the
  name — otherwise one crate reads as the same star drawn twice. The same
  rule holds in the panels: a roster row shows its version whenever
  another row in that list shares its name, and the focus panel names
  every resolved version, since selection is by name and holds all of
  them.
- Crate fact sheet (2026-08-18, same day): the focus panel carries what
  the manifest says about the crate — its own one-line description, its
  license, a member's directory relative to the workspace root, and its
  direct / external dependency counts — plus the pages it has elsewhere:
  repository, crates.io, docs (docs.rs for a crates.io crate that names
  no docs URL of its own), and homepage when it is not one of those. Each
  opens in its own tab so the review never loses its place.
- Names are engraved at rest for members, the diff (changed / affected),
  the selection's neighborhood, and the ~20 biggest externals; every
  other star names itself on hover or keyboard focus.
- The engraved-atlas world (paper, ink, one flare color, states named in
  words, every focus a URL) is unchanged. The `+ n more` marks, bloom
  caps, and fold gesture are gone — the rings make progressive disclosure
  positional instead of quantitative.

**Interaction rework (2026-08-18, "One Living Chart" — superseded by the
rings rework above):** after a critique run, the interaction model was
redesigned. What changed relative to the Interaction and layout section
below:

- `/dep` is now the change report (changed crates + one hop downstream, with
  a CHANGES queue plate), not the member atlas; the member atlas is the
  clean-epoch fallback.
- focus no longer swaps to a new chart: clicking a star blooms its
  neighborhood in place on one persistent canvas. clicking an open star
  again folds it; back / Esc close the most recent bloom. the "chart
  radius 1–3 hops" control is gone; expansion is directed, on-canvas
  ("+ n more downstream / upstream"), and capped with exact hidden
  counts.
- placement is stable and one-directional: after the first paint, stars
  never move; new stars seat in ranked columns beside their bloom's star
  (upstream left, downstream right), each column stacking strictly
  downward, instead of re-running the global layout on every bloom.
- every star, queue row, and search hit is a real link; keyboard surface:
  `/` search, `n`/`p` walk changed crates, `f` refit, Esc un-bloom,
  `←`/`→` browser back / forward (global, every route).
- the legend is retired (2026-08-24): the rings caption their own hops
  (`1 hop`, `2 hops`, `4+ hops`), every star's hover words carry its
  state, fan-in, and the multi-select gesture, and each key is taught at
  the control it acts on (`/` in the search placeholder, `n · p walk`
  beside the changes list, `f` on the fit control).

The visual world (engraved atlas, raises 1–5) is unchanged.

## Job and audience

A Rust systems engineer, right after an LLM agent session, runs
`slope <workspace>` and gets a browser window. They are in "trust but
verify" mode, time-boxed, cognitively loaded. Visitor mode: **Operate** —
the surface is a working instrument, not a showcase.

## Outcome

Within seconds of opening, the reviewer is oriented: they see the shape of
the workspace, which crates changed, and what those changes can affect.
Success = judging where a change landed and its blast radius without
reading every line. All data is real: `cargo metadata` of the actual
workspace, the actual VCS diff. Nothing is fabricated.

## Selected direction: Workspace Atlas

The workspace charted as a night sky, in the grammar of engraved celestial
atlases — the printed plate: dark ink on paper, not a glowing dashboard.
**Light theme only for now; a dark variant is a later addition** (the same
plate as its planetarium/field edition).

- Crates are **stars**, sized by how much depends on them (magnitude =
  dependent count).
- Dependencies are **constellation lines** — fine engraved hairlines.
- Changed crates **flare** in one reserved amber; their blast radius
  (transitive dependents) carries a graded halo that fades with distance.
- A **cartouche** (chart title block) stamps the workspace name and the
  diff epoch (`base..working`, or the explicit revision range).
- **Every state is written at its mark**: state words under the stars
  (`3 files changed`, `2 hops downstream`, `removed`), hop captions on the
  ring guides, and each star's hover words carrying its magnitude
  (`used by n crates`). No legend stands beside the chart.

Raises the direction must keep (won from the concept round):

1. **One material.** Nodes, edges, labels, panels, controls — everything is
   the same engraved-ink grammar. No foreign UI chrome.
2. **States named in words.** Every state is spelled out beside the mark
   that has it; color never carries meaning alone.
3. **Every focus is a URL.** Browser back retraces the exact review trail.
4. **Color is state.** The chart rests as monochrome ink engraving; only
   the diff and its blast radius take color.
5. **One severity color.** The reserved amber means CHANGED and appears
   nowhere else, ever.

Focal moment: the chart materializes, and the changed stars are already
flaring — orientation and diff in the same first glance.

## Scope and boundaries

- Fidelity: production-ready surface (backend analysis + frontend chart).
- Breadth: the dependency viewer only. The code structure viewer and call
  graph are later altitudes; design so drilling deeper can reuse the same
  focus/expand gesture and atlas grammar.
- Untouched: AGENTS.md. The scaffold navbar/home are placeholders and will
  be replaced by the full-bleed chart.
- Anti-goals: never render the whole resolved graph at once; no neon
  dashboard aesthetics; no invented crates, versions, or claims; no
  code-structure features yet.

## Data and diff model

- Graph source: `cargo metadata` — full resolved graph (workspace members,
  direct externals, transitive externals), edge kinds normal/dev/build.
- Diff source: auto-detect VCS (jj first, git fallback; repos may be
  colocated). Default epoch: working copy vs trunk (main/master). Explicit
  revision range via CLI flags overrides.
- A crate is CHANGED when files under it changed in the epoch.
- Manifest edits are first-class edge events: added, removed, or
  version-bumped dependencies are flagged on the edges themselves.
- AFFECTED = transitive dependents of any changed crate, graded by
  distance (hops).

## States and ranges

- Ranges: toy (≤5 members), typical (10–100 members, 300–1500 resolved
  crates), extreme (thousands). The overview renders workspace members
  only; external crates appear progressively on focus/expand. Rendered
  node count stays in dioxus-flow's comfortable range (~hundreds).
- Loading: analysis takes seconds; the loading state belongs to the atlas
  world (chart being surveyed), never a generic spinner.
- Clean epoch (no diff): the overview still orients; the cartouche says
  the epoch is clean. Designed as an invitation, not an apology.
- No VCS: graph works, diff features off, stated in words.
- Errors: not a cargo workspace / metadata failure — plain-language error
  in the chart grammar.
- Edge cases: changed crate with zero dependents (leaf — blast radius is
  itself); crate depended on by everything (huge halo — grading and
  progressive disclosure keep it readable).

## Interaction and layout

- Full-bleed chart (dioxus-flow `Flow` + custom node views, layered
  auto-layout). Chart furniture (cartouche, search, locator
  inset/minimap) drawn in the ink grammar at the edges.
- Overview: members as named stars, member-to-member edges. External deps
  are summarized per star (faint count/hint), not all rendered.
- Focus: click a star (or search) → viewport centers on it, URL becomes
  `/crate/:name`, its one-hop neighborhood brightens (upstream deps and
  downstream dependents), the rest recedes to faint field. Expanding
  reveals further hops and external crates progressively.
- Blast radius: from any changed star, walk dependents outward with the
  graded halo; a control shows/extends hops.
- Edge kinds: line-style differences (solid = normal, dashed = dev,
  dotted = build). Manifest events: added = bright new line with a mark,
  removed = ghost line, bumped = mark with version delta.
- Keyboard: dioxus-flow focusable nodes + arrow-key navigation preserved;
  search reachable by keyboard.
- Motion: viewport animation from dioxus-flow; changed-star flare is
  subtle and respects `prefers-reduced-motion`.
- Desktop-first (review happens at a desk); narrow widths stay usable
  (chart furniture stacks, chart remains pannable).

## Constraints and open decisions

- Stack (fixed): Dioxus 0.7 fullstack, dioxus-flow, Tailwind v4, Nix env,
  `dx serve` for dev.
- Build path: code-led (no image generation in this session).
- Diff detection implementation (shell out to `jj`/`git` vs a library) is
  the builder's choice; behavior above is what's fixed.
- Exact typefaces chosen at build inside the world's grammar
  (cartographic small-caps/serif for chart labels, a workhorse face for
  data readouts). Light theme, ink on paper: paper ground (crisp warm
  white, e.g. #f8f6f1), engraving ink for stars/labels (near-black
  blue-ink, e.g. #1c2733), lighter ink hairlines for edges (e.g.
  #7c8494), reserved CHANGED amber legible on paper (e.g. #c25e00) with
  graded washes for the blast halo. Exact values refined at build; roles
  fixed. Dark theme deferred.
- Accessibility: states never color-only (raise 2); keyboard navigation;
  reduced-motion respected.
