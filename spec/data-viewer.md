# The data chart — design brief and behavior

The third altitude of the review ladder: crates → files/items → **the
workspace's state**. Added 2026-08-21 (user decision) beside a surface chart
that read the same types as contracts; that chart was removed on 2026-08-24
(user decision) and this one is the altitude that reads types as data. Route
family: `/data`, `/data/mark/:..path?item=`, `/data/mod/:..module`. The
`/code` and `/data` routes share one survey fetch.

## Job and audience

The same reviewer, one rung down from the code map: **"what state does this
workspace keep — and which of it is top-level?"** After an
agent session the state shape is where quiet damage hides: a new struct
nobody holds, a field that turns owned state into shared state, a type that
silently became load-bearing. Visitor mode: **Operate**.

## The thesis, in four rules

1. **Marks are state.** One block per struct, enum, union and static —
   whatever its visibility, because state does not fold at a door. Functions,
   traits, consts and aliases have no block: a signature names state, it does
   not keep any. Methods are not rows either — a block is state only, and what
   a type promises is read on its definition plate, one rung up.
2. **The tier is the chart.** Top-level data is a **root**: a static, or a
   type no other workspace type keeps in a field (`Owns` or `Shares`; a
   borrow is a view, not a hold). Roots stand at module level and wear the
   gate's 2.5px ink left edge — the static's mark, widened to every block a
   chain of holding begins at. Everything held is **secondary** and is drawn
   *inside* the block of the type that owns it hardest, the way module frames
   nest: reading the tier is reading the paper.
3. **Nesting is ownership; lines are what nesting cannot say.** Plain
   same-module ownership draws no edge — the containment is the edge. What
   stays drawn ink, solid with the wrapper's word: sharing (`Arc`, a signal —
   sharing has no single container, so a shared type stands beside its
   holders), borrowing (`&`), second holders, cross-module ownership (a type
   never leaves the frame that declares it), a ring of mutual owners, and the
   diff's added/removed relations. A type held by more than 3 types is
   vocabulary: it stands, its fan-in folds to `held by n types` on its own
   foot, and hover or selection inks the lines back in.
4. **What has no block is counted, never cut.** Naming from contracts is
   `named by n signatures`; body references from code with no mark here are
   `used by n bodies`. Both in the block's hover words, both listed as rows
   on the sheet (moved off the resting paper 2026-08-21) — that count is
   exactly the "directly accessed" reading, in words because its other end
   has no block to draw a line to. On the sheet the words become **names**
   (2026-08-23): `Used by` and `Uses` list every reference — drawn types and
   free functions, traits and consts in one weight-ranked list, each row
   saying its keyword and its count and linking to its definition (the code
   viewer for the ones with no block, since the chart cannot re-centre on
   what it does not draw). No sentence is left counting what a reviewer
   cannot open.

## Decisions (user-confirmed 2026-08-21)

- **Top-level = ownership root**, not "named by many signatures": the tier is
  decided by holding, and direct code access stays words on the mark.
- **Third rung.** Ladder reads `dependencies · code · data` in every
  cartouche (it read `dependencies · code · surface · data` until the surface
  chart was removed on 2026-08-24).
- **Ownership as containment.** A module frame holds only top-level blocks;
  secondary data nests inside its heaviest holder. Additional holders keep
  drawn edges.

## The chart

- **Blocks** wear the chart's block anatomy — keyword + visibility in
  keyword-blue, name at 700 (teal product type, purple sum type), the diff's
  letter, token-colored quoted rows with the bold run naming the workspace
  type the row reaches. No method band; the locator lives in the hover words
  and on the sheet. A static quotes its declared type under its name.
- **The bold run is a link to the block it names** (2026-08-24, user): click
  `ItemKind` in `kind: ItemKind` and the chart selects `ItemKind`, gliding
  there only if the glass cannot already show it. It underlines under the
  pointer and carries nothing at rest. A run with no block to reach — inside
  a hand-folded module, or naming the block it is written in — stays bold text
  and nothing more.
- **Nesting**: a hairline rule closes the rows; the owned blocks shelve on
  the clear paper under it, at full block anatomy, recursively. A block grows
  to hold what it contains. The bold run in a field row above is the block
  below.
- **Two inks**: solid holding (wrapper's word; width by kind) and dashed
  counted uses edges (one type's impls leaning on another, ends climbed to
  the type). Arrowhead on the dependent in both. The `references` toggle is
  the shared reading (uses / used by / both); each block rests its heaviest
  two ties, the rest ink in on hover and stay while either end is selected.
- **No doors toggle and no budget fold.** Every datum is drawn. The only
  counted rows are a hand-folded module's `+ n items` and the vocabulary
  fan-in fold.
- **Selection** is a URL and a reading: blast radius walks every structural
  relation — nesting included — upward; uses neighbours read a step behind;
  strangers recede. Receding acts on a block's *own paint* (frame + text),
  never its box, so a lit kid never dims with its holder. The camera moves
  only for a selection the glass cannot show (see the far-edition section).
- **Module boundary**: click the border to select (`/data/mod/…`), the − / +
  mark to fold to one counted row. Folds are this chart's own view state.
- **The structural diff** rides along in the shared grammar: `A`/`M`/`D`
  letters, flare frames, ghosts quoted from the base standing in their frame,
  `+`/struck rows woven in place, added/removed holding edges in flare with
  their words, untouched blocks resting lighter while the diff speaks.

## The far edition and the critique round (2026-08-21)

A dual-agent critique (19/40) found the first build's craft spent below its
own legibility floor. The fixes, all user-directed:

- **Zoom is the fold.** Below reading zoom (enter 0.45 / leave 0.55) the
  chart holds a far edition: name + keyword only per block (container-query
  sized to the box), root edge and selection ring at near-constant screen
  width, wire labels retired and strokes thickened, module names engraved
  across their frames in soft ink. Blocks keep their boxes — nothing moves,
  wires keep their landings. Rows return past the threshold.
- **The camera glides to a selection it cannot show** (off-viewport or below
  reading zoom) — recorded as a DESIGN.md camera-discipline amendment. A
  legible selection moves nothing.
- **Search** (`/`, top-right plate) over every datum, ranked prefix-first
  then fan-in.
- **A selected boundary bundles its crossing ink**: one line per far module
  and direction with its count, replacing the per-edge hairball; inside
  lines stay; hovering a block expands that block's own lines.
- **The foot quiets**: only `held by n types` (the genuine fold) stays on
  the paper; `named by n signatures · used by n bodies` and the locator
  move to the hover words and the sheet.
- **Skyline packing** replaces row-shelving in the shared layout (frames and
  kids both): short blocks fill the slack beside tall ones, raising the fit
  zoom the whole chart is read at.
- Sheet rows attach a clause only when the whole row fits; small-type
  measurement carries 8% slack so meta lines stop clipping; the survey's
  variant quotation drops interior doc comments (fixes a corrupted row on
  both altitudes).

## Chrome

- Cartouche: workspace name; `n structs · n enums · n statics` (unions when
  present); the four-rung ladder; the diff line, flare counts, insight line.
  One toggle: `references`. No visibility toggle — the legend states why.
  The tier counts and the edge counts came off (2026-08-21, distill): four
  invented terms defined only in legend prose, and no decision rides on
  them. The tier is what the paper draws; the legend teaches the word.
- Sheet (mark selection): header, locator, **the tier in one sentence**
  ("top-level data: no type holds it — a root." / "secondary data — drawn
  inside its holder's block." / the standing reasons — the nested line does
  not name the holder, which is the first row of the section right below
  it), the diff's rows,
  then kept strictly apart: `Held by` (nesting first, then drawn relations),
  `In the contract of` (each namer a link to its definition plate), `In the
  API of`, the reach line ("a shape change here reaches 4 more types
  upstream, and 9 signatures name what it reaches."), `Holds`, `Used by` /
  `Uses` with the undrawn-residue lines, and `open its definition →` — the
  code plate stays the only place source is quoted whole.
- Legend (cut to a key 2026-08-21, distill — about six hundred words to
  ~170): a key strip first (the root's left edge and the nested sample, drawn
  with the chart's own classes; then the two inks), one paragraph of grammar
  per family, the kind colors, the no-methods/no-doors line, the diff key
  *only while the diff has something to say*, the gestures on a two-column
  grid, the far-edition and counted-residue lines — and the survey's own
  limits behind a nested `what the survey cannot read` fold, printed from
  `walk_notes` + `notes` in the survey's words rather than paraphrased. The
  references-toggle paragraph came off: it restated the three button titles.

## Implementation notes

- `src/views/data/{model,layout,map,chrome,mod}.rs`. The model
  (`DataModel::build`) classifies tiers off `CodeGraph::holds`
  (`from_method == false` and a data-kind holder = structural; everything
  else = naming), nests greedily by field weight with cycle checks, and owns
  the `Frame`/`Seat`/`Anchor`/fold vocabulary (inherited from the removed
  surface model on 2026-08-24).
- Blocks are measured in code around their measured kids (post-order); the
  frame layout is `data::layout`, with every top-level block a leaf seat. Nested rects are derived from the parent's placement, so wires can
  land on state drawn three layers deep.
- `DataState` (own folds) and `DataCamera` live on the app shell;
  `use_code().ref_dir` is shared across altitudes.

## Open decisions

- A ghost stands at module level whatever held it in the base; seating a
  ghost inside its base holder would need base-edition holds at full
  precision.
- `dyn Trait` fields draw no line here (traits live one rung up). If review
  shows reviewers missing that coupling, a counted `holds n dyn` foot line
  is the grammar-consistent fix.
- Generic payloads (`Vec<T>`) stay holes, as everywhere.
