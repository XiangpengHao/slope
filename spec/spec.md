we're building slopify.

## Goal

slopify is a code reviewer for large LLM-made changes.

The thesis: a human cannot read every line of a big agent-written change.
That takes too much time and cognitive load. Instead, slopify works above
the raw rust code. The reviewer navigates from high level (crate
dependencies) down to details (function call graph), and gains confidence
by checking code structure without reading every line.

The dependency graph viewer is the first step. It bootstraps the tool.
It is not the end goal.

## Backend

the backend reads a cargo manifest, reads the dependencies and builds a dependency graph.

## Frontend

Frontend is ui heavy.
The first view is a dependency graph viewer.
For every crate, it shows what crates it depends on and what crates depend on it.

Decided so far:
- graph scope: the full resolved graph, including transitive external crates.
- core job: blast radius of a change. the viewer is diff-aware from the start.
- never show the whole graph at once. start from a focal point, disclose progressively.

## Dependency viewer (built 2026-08-18, interaction rework same day)

How it works today:

- backend (`src/analyze/`): runs `cargo metadata` on the target workspace
  (`SLOPIFY_WORKSPACE` env, default: current dir). detects the VCS (git
  first, jj fallback) and diffs the working copy against trunk
  (main/master merge-base). `SLOPIFY_BASE` overrides the base revision.
- a crate is CHANGED when files in its directory changed in that window.
  AFFECTED = transitive dependents of a changed crate, graded by hops.
- manifest edits are compared against the base revision Cargo.toml:
  added / removed / version-bumped dependencies become events on the
  edges. removed deps draw as ghosts.

Frontend (`src/views/`): one living chart that blooms.

- `/` is the change report: every changed crate and one hop downstream,
  plus manifest-event externals. on a clean epoch it falls back to the
  member atlas. a CHANGES plate lists the changed crates as links — the
  review queue — and marks each SEEN once visited.
- clicking a star blooms its neighborhood in place: the chart never
  remounts; new stars grow out of the star they came from, the camera
  glides. each bloom pushes `/crate/:name`, so browser back (or Esc)
  closes the most recent bloom — back is the undo of the review trail.
  a deep link opens a fresh chart bloomed on just that crate.
- clicking an open star again folds its bloom (the focal star's click is
  the way back); focusing a folded crate unfolds it. folds never touch
  the URL.
- stars already on the chart never move, and growth is one-directional:
  newcomers take ranked seats beside the star they bloom from — upstream
  to the left, downstream to the right (above/below on phones) — and
  each column stacks strictly downward from its star, never above.
  collision nudges continue downward. a bloom with no placed neighbor
  (search jump) opens new ground beyond the chart's right edge.
- blooms are capped per direction (downstream 10, upstream 8); hidden
  neighbors wait behind an on-canvas "+ n more downstream / upstream"
  mark that reveals a dozen per press. counts are exact.
- the focus panel shows the way back (back link + trail line), identity,
  state words, then USED BY (the blast radius) before DEPENDS ON, both
  chunked at 8 rows with a "show all n" reveal.
- keyboard: `/` search (arrows walk hits, Enter opens), `n`/`p` next /
  previous changed crate, `f` refit, Esc un-bloom, `←`/`→` browser back /
  forward on every route. every star, queue row,
  and search hit is a real link (middle-click works). route changes are
  announced to screen readers. the legend teaches all of this in a
  "using this chart" section.
- the full resolved graph is never drawn at once, and every
  simplification is written in words on the plates.
- design: light theme, ink on paper, per spec/dependency-viewer.md.
  DESIGN.md records the visual system.
- vocabulary (2026-08-19): every user-facing label is the word rust,
  cargo, or the VCS already uses — `pub(crate)`, `fn`, `(dev)`,
  `dev-dependencies`, `M`, `3 files changed`, `src/api.rs:10`. Invented
  uppercase abbreviations (EXT, DEV, GATE, `12 L`, "epoch") are retired,
  and the code altitude's focus plate quotes an item's own source instead
  of describing it. See spec/code-viewer.md and DESIGN.md.

Run it:

```
SLOPIFY_WORKSPACE=/path/to/workspace dx serve
```
