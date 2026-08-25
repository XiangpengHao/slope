we're building slope.

## Goal

slope is a code reviewer for large LLM-made changes.

The thesis: a human cannot read every line of a big agent-written change.
That takes too much time and cognitive load. Instead, slope works above
the raw rust code. The reviewer navigates from high level (crate
dependencies) down to what the code keeps (the workspace's state), and
gains confidence by checking structure without reading every line.

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
  (`SLOPE_WORKSPACE` env, default: current dir), with that workspace as the
  cargo process working directory so its `.cargo/config.toml` is honored.
  detects the VCS (git first, jj fallback) and diffs the working copy against
  trunk (main/master merge-base). `SLOPE_BASE` overrides the base revision.
- a crate is CHANGED when files in its directory changed in that window.
  AFFECTED = transitive dependents of a changed crate, graded by hops.
- manifest edits are compared against the base revision Cargo.toml:
  added / removed / version-bumped dependencies become events on the
  edges. removed deps draw as ghosts.

Frontend (`src/views/`): one living chart that blooms.

- `/dep` is the change report: every changed crate and one hop downstream,
  plus manifest-event externals. on a clean epoch it falls back to the
  member atlas. a CHANGES plate lists the changed crates as links — the
  review queue — and marks each SEEN once visited.
- clicking a star blooms its neighborhood in place: the chart never
  remounts; new stars grow out of the star they came from, the camera
  glides. each bloom pushes `/dep/crate/:name`, so browser back (or Esc)
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
  announced to screen readers. there is no legend (retired 2026-08-24):
  each key is taught where it acts — `/` in the search placeholder,
  `n · p walk` beside the changes list, `f` on the fit control — and
  every mark carries its own hover words (a ring captions its hops, a
  star says its state and `ctrl-click adds it to the selection`).
- the full resolved graph is never drawn at once, and every
  simplification is written in words on the plates.
- design: light theme, ink on paper, per spec/dependency-viewer.md.
  DESIGN.md records the visual system.
- vocabulary (2026-08-19): every user-facing label is the word rust,
  cargo, or the VCS already uses — `pub(crate)`, `fn`, `(dev)`,
  `dev-dependencies`, `M`, `3 files changed`, `src/api.rs:10`. Invented
  uppercase abbreviations (EXT, DEV, GATE, `12 L`, "epoch") are retired,
  and a quoted row is the source's own bytes, never a paraphrase of them.
  See DESIGN.md.

## The code map, removed (2026-08-24)

there was a third viewer between the two: a **code map** at `/code`, which
drew the workspace's files as blocks inside nested directory frames, with
resolved reference ties between them, and a definition plate that quoted
an item's own source with every resolved name in it a link. it is gone
(user decision), along with `spec/code-viewer.md`, the `/code` route
family and the `file_detail` server function. `item_source` and the source
lexer went with it and came back the same day (user): the data sheet's rows
now quote one item at a time on their own plate — see
`spec/data-viewer.md`. what stayed gone is the map.

the ladder was **`dependencies · data`** after that, two rungs, and is
**`dependencies · data · functions`** since 2026-08-25 — see
`spec/function-viewer.md`. what went with the map:

- the dep chart's focus panel offered "its files ↓", to the code map's
  crate sheet. it went with the map, and on 2026-08-24 came back as
  **"its data ↓"** — `/data/mod/:package`, the member's own frame on the
  data chart. the link needed a fix first: the two altitudes did not share
  a key for a crate. cargo names the **package** (`slope-cli`); the survey
  named the **crate** rust-analyzer resolved (`slope` — this workspace's
  bin target is renamed), so `/data/mod/slope-cli` selected nothing, and
  the old link had the same bug silently — `/code/crate/slope-cli` printed
  "No crate named "slope-cli" in this survey." the survey now labels every
  file with the cargo member whose directory owns it (`member_dirs` in
  `src/analyze/mod.rs`, `package_of` in `src/analyze/code.rs`), matching a
  workspace-relative path against each member's `rel_path`, deepest member
  first. one consequence: `/data/mod/slope` is now `/data/mod/slope-cli`,
  and the crate frame's label and the cartouche's insight line say
  `slope-cli` with the dep cartouche.
- the data sheet's rows for what this chart draws no block for — a trait
  it promises, a method written for it, a free function that names or uses
  it — linked to the code plate. they keep their row, they name the file and
  line the declaration is written on in the row's hover words, and since
  2026-08-24 (user) clicking one quotes it beside the sheet on the
  **quotation plate** (`peek=<file>@<label>` on the selection's own URL).
  the sheet's `open its definition →` foot is still gone: a foot pointed at
  one definition, and every row now opens its own.
- the survey still runs rust-analyzer and still resolves every reference
  at item precision: the data chart's dashed uses edges are those. what
  went is the file-precision reference list and each file's per-item
  cutaway. the sources, the item spans and the reference spans are kept on
  the server again (`analyze::code::CodeIndex`), because a quotation must be
  the bytes the survey read.

## Function viewer (built 2026-08-25)

the third rung, `/fn`: every function, method, trait clause and
`macro_rules!` the workspace declares, tiered by **call depth** from the
declarations nothing calls. it is the data chart's dual — a block there
quotes a struct's fields, a block here quotes a function's signature — and
its sheet's `Data touched` rows are the one link between the two rungs,
each one a descent to that type's block on `/data`. full spec in
`spec/function-viewer.md`.

three things it needed that nothing else had:

- **the survey gate moved up** (`src/views/survey.rs`). both code altitudes
  read one rust-analyzer survey, and neither may pay for it twice, so the
  gate the data chart owned is now shared and mounted above both route
  families by the app shell.
- **a method's own signature rows.** the data walk filed a method's
  quotation on its type's contract and nowhere else; it now also writes the
  method's own mark — the receiver as written, the parameters, the return —
  which is what a block on this chart quotes. nothing on `/data` reads those
  slots, so that rung did not move.
- **contract edges.** a call graph alone lies about a trait-heavy workspace:
  a `dyn` call lands on the trait's clause and the code that runs is
  somewhere else. so a trait's clause and the methods answering it are a
  second, dashed family, and a method answering a *workspace* trait is not
  an entry point. one answering a foreign trait still is, and the limits
  fold says so.

## Name (renamed 2026-08-20)

the project was `slopify`, which reads as "make more slop". it is `slope`
now: "slop" with an e, and the gradient the reviewer walks between
altitudes. the env vars are `SLOPE_WORKSPACE` and `SLOPE_BASE`.

the crates.io package is `slope-cli`, because `slope` is held there by an
abandoned crate. the binary stays `slope`, the same split dioxus-cli uses
for `dx`.

## Develop

```
SLOPE_WORKSPACE=/path/to/workspace dx serve
```

## Ship it (added 2026-08-20)

the repo is flake-only. `nix build .#slope` runs `dx bundle` in the
sandbox: it compiles the wasm client, runs the tailwind pass, links the
server, and installs the `server` binary next to its `public/` directory
under `libexec`, wrapped as `bin/slope`. the wrapper sets
`DIOXUS_PUBLIC_PATH`, since the server otherwise looks for `public/`
beside its own executable. `nix run github:XiangpengHao/slope` is the
recommended install.

`dx bundle` defaults to a debug build, which yields a 110MB wasm client.
the package passes `--release --debug-symbols false`, which brings it to
2.6MB. a bundle without a wasm file still starts and serves an empty
page, so CI asserts the wasm is present and has real size.

two workflows:

- `.github/workflows/ci.yml` — fmt, clippy, and tests on every push, then
  a full `nix build` off the pull-request path. clippy runs twice, once
  per side: the `web` and `server` features gate disjoint dependency
  sets, so neither pass covers the other.
- `.github/workflows/release.yml` — on a `v*` tag: check the tag matches
  the manifest, build a linux tarball on a plain glibc runner (the nix
  binary is wired to /nix/store paths and is not a portable download),
  cut a github release, then publish to crates.io.

`cargo install slope-cli` compiles the sources but cannot produce the
wasm client or run tailwind, so its binary serves an empty page. the
crates.io package is source distribution and name reservation; the
working installs are nix and the release tarball.

`dioxus-flow` was published as 0.1.0 on 2026-08-20 (it had been a git
dependency, which cargo refuses to publish). slope now depends on it from
the registry, so the flake needs no per-git-dep output hash either.

one packaging trap: `assets/tailwind.css` is generated and gitignored, and
an `asset!` in main.rs reads it at compile time. cargo's git-derived file
list drops it and the published crate then fails to build, so Cargo.toml
uses an `include` allowlist that carries it, and the publish job generates
it before packaging.
