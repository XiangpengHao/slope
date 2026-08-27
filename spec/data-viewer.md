# The data chart — design brief and behavior

The second altitude of the review ladder: crates → **the workspace's
state**. Added 2026-08-21 (user decision) beside a surface chart that read the
same types as contracts; that chart was removed on 2026-08-24 (user decision)
and this one is the altitude that reads types as data. The code map that stood
between this rung and the crates was removed the same day — see
`spec/spec.md`. Route family: `/data`, `/data/mark/:..path?item=`,
`/data/mod/:..module`. A selection may carry one more query —
`peek=<file>@<label>`, one of its sheet rows opened as a quotation of its own
source (2026-08-24).

## Job and audience

The same reviewer, one rung down from the crates: **"what state does this
workspace keep — and which of it is top-level?"** After an
agent session the state shape is where quiet damage hides: a new struct
nobody holds, a field that turns owned state into shared state, a type that
silently became load-bearing. Visitor mode: **Operate**.

## The thesis, in four rules

1. **Marks are state.** One block per struct, enum, union and static the
   visibility reading admits. Functions, traits, consts and aliases have no
   block: a signature names state, it does not keep any. Methods are not rows
   either — a block is state only, and what a type promises is read on its
   selection sheet.
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
   saying its keyword and its count, a link where the chart draws that end a
   block and the file and line it is written on where it does not. No
   sentence is left counting what a reviewer cannot go and read. Since
   2026-08-24 (user) every one of those blockless rows is a link too: it
   quotes the item's own source beside the sheet, so nothing a row names has
   to be read in another window.

## Decisions (user-confirmed 2026-08-21)

- **Top-level = ownership root**, not "named by many signatures": the tier is
  decided by holding, and direct code access stays words on the mark.
- **Second rung.** Ladder reads `dependencies · data` in every cartouche (it
  read `dependencies · code · surface · data` until the surface chart and then
  the code map were removed on 2026-08-24).
- **Ownership as containment.** A module frame holds only top-level blocks;
  secondary data nests inside its heaviest holder. Additional holders keep
  drawn edges.

## The chart

- **Blocks** wear the chart's block anatomy — keyword + visibility in
  keyword-blue, name at 700 (teal product type, purple sum type), the diff's
  letter, token-colored quoted rows with the bold run naming the workspace
  type the row reaches. No method band — methods and trait impls are the
  sheet's rows, not the paper's; the locator lives in the hover words and on
  the sheet. A static quotes its declared type under its name.
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
  the type). Arrowhead on the dependent in both. The `references` toggle
  (`uses` / `used by` / `both`) is the shared reading, and it reads **against
  an anchor** (fixed 2026-08-25, user: "the edges won't change as I change,
  no?"). The same hairline is one type's use and another type's users, so
  direction can pick different edges only once the chart knows which mark the
  reader has in hand. The anchor is never invented — it is whatever the
  reviewer is looking at:
  - the **selection**, whenever there is one: `uses` inks the marks it leans
    on, `used by` the marks that lean on it, `both` either. Blocks and
    hairlines light together, so the switch moves the picture at once.
  - **hover**, the same way round — a wire lights only when the hovered mark
    is the end the reading asks for. Structure has no direction and lights
    from either end.
  - the **diff**, on the resting plate: `uses` rests what the changed
    declarations lean on, `used by` rests whose code leans on them. That is
    the blast-radius question this chart exists for, and it means the resting
    paper carries a handful of meaningful hairlines instead of every one the
    survey resolved.
  - and where a workspace has neither a selection nor a diff, every reference
    is drawn under all three readings, because a reading with nothing in focus
    has no direction to take.

  The first build had no anchor and thinned by a per-mark quota of two
  instead, so `uses` and `used by` kept nearly the same lines and moving the
  switch changed nothing a reader could see. The quota is gone;
  `each_references_reading_rests_a_different_set_of_ties` in
  `views::data::model` holds the three sets apart.
- **The visibility reading** (2026-08-25, user): a four-stop slider on the
  cartouche — `pub`, `pub(crate)`, `pub(super)`, `all` — sets how narrow a
  declaration may be and still have a block. Each stop keeps the rungs above
  it and adds the next one down, so the reviewer auditing what a crate
  publishes reads that surface alone and widens in one move. Three rules keep
  it honest:
  - **As declared, never as reached.** The rung is the keyword rust writes in
    front of the declaration. What a chain of private modules leaves reachable
    from outside is a resolution this survey does not run. The plate's label
    is `visibility` and the caveat is its hover words (2026-08-25, distill):
    the scale underneath already says which alphabet it reads, and the foot
    states what the reading left off as `n off` rather than a sentence.
  - **It cuts blocks, not rows.** A block is a quotation of a declaration, and
    a quotation with its private fields dropped misquotes it. Every row of a
    drawn block stays, wearing its own `pub` as always.
  - **A hidden holder is not the absence of a holder.** A type whose every
    holder is narrower than the reading draws does not become a root: it
    stands, and its sheet says the reading is what left the holder off.
  A declaration off the reading leaves no row and no count in its frame —
  only the cartouche's one line, `n narrower declarations off this reading`,
  so a narrow reading never reads as an empty workspace. Naming a declaration
  in the search widens the reading to the stop that draws it; a URL kept from
  a wider reading opens a sheet that names the rung and offers `draw it`.
- **No budget fold.** Every datum the reading admits is drawn. The only
  counted rows are a hand-folded module's `+ n items`, a hand-folded holder's
  `+ n inside`, and the vocabulary fan-in fold.
- **Selection** is a URL and a reading: blast radius walks every structural
  relation — nesting included — upward; uses neighbours read a step behind;
  strangers recede. Receding acts on a block's *own paint* (frame + text),
  never its box, so a lit kid never dims with its holder. The camera moves
  only for a selection the glass cannot show (see the far-edition section).
- **Module boundary**: click the border to select (`/data/mod/…`), the − / +
  mark to fold to one counted row. Folds are this chart's own view state.
  A module key is the cargo **package** name, then the module path as rust
  nests it — `/data/mod/slope-cli`, `/data/mod/slope-cli/views/data`. The
  package name and not rust-analyzer's crate display name, so a crate is
  called the same thing here and one rung up and the dep chart's focus
  panel can descend into a member's frame with "its data ↓".
- **A holder folds its shelf** (2026-08-26, user): the same `−` / `+` mark at
  the end of a block's head row, acting on **the state nested inside it** and
  nothing else. The hairline rule stays, the shelf becomes one counted row —
  `+ 6 inside`, itself the way back — and the block's own fields and variants
  never fold, because a block quotes its whole declaration and a quotation
  missing its fields is a misquotation. `held by n types` is a different fact
  (a fan-in the chart folded, not state a reader closed) and neither hides the
  other. Every edge into the state it swallowed **re-anchors to the holder's own
  block**, the way a folded module's edges land on its counted row, so nothing
  dangles; two references gathered onto one line gather their counts too. A
  selection URL, a search hit or a quoted row's bold run naming folded-away
  state **opens the folds on the way in first**, outermost first. Keyed by
  (file, label) — the same pair the selection URL carries — so a fold survives
  the next build. See DESIGN.md's Hand-Fold Rule, which this, the module fold
  and the function chart's frame fold all keep.
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

## The quotation plate (2026-08-24, user)

Clicking a sheet row the chart draws no block for — a function or a namer in
`Used by`, a trait in `Implements`, a method in `Methods` — quotes it on a
plate immediately left of the sheet. It answers
the one question those rows used to leave open: *what does that code actually
do?*

- **The URL carries it**: `/data/mark/:..path?peek=<file>@<label>&item=<label>`.
  The selection never moves — a quotation is a reading of the sheet, not a
  step to another mark — so the chart, its blast radius and the camera hold
  still. The back button closes the quotation; Escape closes it before it
  deselects; `close ×` says the same in words. An unquoted selection's URL is
  exactly what it was before.
- **The plate**: the item's kind and name (`fn AltitudeSwitch` — the kind's
  word alone, because the survey reads `pub(super)` as `pub(crate)` and the
  head must not contradict the source quoted under it), the locator
  (`src/views/chrome.rs:37`), then the item's own source — the bytes the
  survey read, dedented by the outermost quoted block's indent and nothing
  else — with a gutter counting from its first line in the real file. Long
  lines scroll; nothing wraps and nothing is cut. It caps at the room between
  the cartouche and the sheet and at the glass's height.
- **A method is quoted inside its block** (2026-08-25, user): the `impl` or
  `trait` header, the method at the indent it stands at, then the brace that
  closes the block. An associated item's span holds none of its header — the
  block is its own item in the survey — so quoting the span alone printed `fn
  edge_style(self, …)`, which is not rust and never says whose method it is.
  Lines the file writes between two quoted blocks are not carried, and the
  gutter marks the jump with `⋮` — not with a count of them, which is a
  number nobody acts on; the line numbers already say how far it is.
- **The row stays inked** while its quotation is open (2px ink left edge,
  `bg-ink/5`), so the plate is never loose from the row that asked for it.
- **Every resolved name inside is a link**: to that datum's block where the
  chart draws one (`item=`), to its own quotation where it does not (`peek=`),
  so following the code is the same gesture as following the chart. A
  reference to a whole module is not a link — this altitude has no place for
  one.
- **What has no source is not a link**: a foreign trait the survey never
  read, a method the base had and the working copy dropped, a ghost's API.
- The colours are the block-row palette plus the four classes only a body
  says — fn/macro name, string, doc comment, comment (see DESIGN.md).

## Chrome

- Cartouche: workspace name; `n structs · n enums · n statics` (unions when
  present) — the census of what the reading draws, not of the survey; the
  four-rung ladder; the diff line, flare counts, insight line. Two readings:
  the `references` toggle and the `visibility` slider, whose foot states what
  it left off in two characters and a word. The tier counts and the edge counts came off (2026-08-21,
  distill): four invented terms defined only in chrome prose, and no
  decision rides on them. The tier is what the paper draws; a root's own
  hover words teach it (`a root — no type holds it`), and the sheet's
  tier line says it in a sentence.
- Sheet (mark selection): header, locator, **the tier in one sentence**
  ("a root: no type holds it." / "drawn inside its holder's block." / the
  standing reasons, the visibility reading's own among them — the nested line
  does not name the holder, which is the first row of the section right below
  it), the diff's rows, then **two relation headings and no more**
  (2026-08-25, user): `Used by`, the reach line ("a shape change here reaches
  4 more types upstream and 9 signatures."), `Uses`, and what the type itself
  offers as `Implements` / `Methods` (2026-08-24, user).
  - `Held by`, `In the contract of`, `In the API of`, `Holds`, `Used by` and
    `Uses` were six headings for two directions, and read as six unrelated
    questions. Being held, being named in a contract, being named in an API
    and being used by a body are four kinds of one fact — something reaches
    this — so the heading says the direction and the **row's own word says
    the kind**: `owns`, the wrapper's word (`Vec`, `Arc`, `dyn`), `owns · off`
    for a holder this reading left off the paper, `signature` for a free
    declaration whose signature names it, `API` for a type whose methods do,
    and `n references` for a body.
  - Inside each heading the order is the strength of the claim: structure
    first (the block it nests in leads, because the paper says that first),
    then the signatures, then the bodies, heaviest first. The header's own
  `src/api.rs:67` locator is where the mark itself is written; the sheet had
  an `open its definition →` foot to the code plate until that chart was
  removed (2026-08-24); the rows themselves now open the quotation plate
  instead, one row at a time.
- **`Implements`** is one row per hand-written trait impl gathered from
  anywhere in the workspace, quoting the trait as its header writes it
  (`From<Option<ast::Visibility>>`), naming where the contract is written
  when the workspace declares it, wearing that trait's own `A`/`M` and this
  epoch's `added` / `removed` on the promise. A derive is not one of these: it
  stands in the type's own source. A contract the base promised and the
  working copy dropped keeps its row, from the removed impl edge alone.
- **`Methods`** is one row per method the survey read for the type, wherever
  its impl block is written: the keyword and visibility, the name, `A` where
  the epoch added it, the contract it answers where a trait asked for it, and
  `removed` rows quoted from the base for what the epoch dropped. The
  signature as written, then the file and line the impl block is written in,
  are the row's hover words — the sheet column is a name's width. The type's own methods read first, then the trait ones under their
  promise; a ghost lists the API that left with it. Nothing here is on the
  paper: a block is state only, and the sheet is a list.
- No legend (retired 2026-08-24; it had already been cut from ~600 words
  to a key on 2026-08-21). The chart teaches itself: the tier is the
  paper's own nesting, a root's hover words say what its ink edge draws,
  the diff letters carry their git meaning in their own tooltips, and the
  wires write their words on the line. The survey's own limits — the
  unresolved census, then `walk_notes` + `notes` in the survey's words —
  rest behind the cartouche's `what the survey cannot read` fold.

## Implementation notes

- `src/views/data/{model,layout,map,chrome,mod,quote}.rs`. The quotation
  plate is `data::quote`, over one server call: `api::item_source(item)` hands
  back the item's source lexed into token-classed runs with the resolved
  references attached as links. The server keeps what the graph does not carry
  over the wire — every surveyed file's text, each item's byte range, and
  every resolved reference's own name-token range — in
  `analyze::code::CodeIndex`, so a quotation is always the text the survey
  itself read and never a re-read of a file that has moved on. The sheet's
  `Implements` reads `ItemMark::impls` (hand-written headers) against
  `CodeGraph::implements` (the survey's resolved impl edges, workspace traits
  only, carrying the diff event), matching a written header to a resolved edge
  on the trait's bare name the way the structural diff does; its `Methods`
  reads `ItemMark::method_rows`, whose `section` (the impl header the method is
  written under, added 2026-08-24) is what tells a type's own API from a
  contract's. The model
  (`DataModel::build`) classifies tiers off `CodeGraph::holds`
  (`from_method == false` and a data-kind holder = structural; everything
  else = naming), nests greedily by field weight with cycle checks, and owns
  the `Frame`/`Seat`/`Anchor`/fold vocabulary (inherited from the removed
  surface model on 2026-08-24).
- Blocks are measured in code around their measured kids (post-order); the
  frame layout is `data::layout`, with every top-level block a leaf seat. Nested rects are derived from the parent's placement, so wires can
  land on state drawn three layers deep.
- The fold sets are the reading, not the drawing: `DataReading::folds` (module
  keys) and `DataReading::blocks` (holder keys) both travel into
  `DataModel::build`, which fills `DataModel::packs`, the map from each hidden
  mark to the holder standing for it. The model's own edges keep their real ends;
  only `DataDrawing` reads them through `packs`, which is what makes the sheets
  and the blast radius truthful while the paper re-anchors.
- **A holder fold elides in place** (2026-08-27, user). A folded holder *keeps*
  its `seat.kids`, so the post-order measure still measures them and the block
  still reserves the band its shelf filled — the skyline does not close up and no
  other block on the sheet moves. What leaves them off the paper is the drawing:
  the plate renders the counted row instead of the kid markup, and `abs_rects`,
  `keys_of` and `frames_of` stop at a folded holder, so nothing hidden has a box
  a camera, a selection or a wire can land on. Only `DataReading::packed_blocks`
  — the subset the skyline was allowed to skip, widened by `DataState::repack` on
  a `references` or `visibility` change and never by a fold — empties `seat.kids`
  and lets the paper close up. `DataState::fold_block` is the only writer of a
  hand fold, and opening a packed-away holder leaves both sets at once, because
  its state needs its room back. The function chart keeps the same two sets for
  its frame folds.
- `DataState` (the two fold sets and the `references` reading) and `DataCamera`
  live on the app shell; `DataSurvey` — the survey gate that fetches
  `code_graph()` and holds its loading and failure plates — is mounted there
  too, above the routes, so a selection change never re-runs rust-analyzer.

## Open decisions

- A ghost stands at module level whatever held it in the base; seating a
  ghost inside its base holder would need base-edition holds at full
  precision.
- `dyn Trait` fields draw no line here (a trait keeps no state, so it has no
  block). If review
  shows reviewers missing that coupling, a counted `holds n dyn` foot line
  is the grammar-consistent fix.
- Generic payloads (`Vec<T>`) stay holes, as everywhere.
