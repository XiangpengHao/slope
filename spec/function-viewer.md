# The function viewer (`/fn`), built 2026-08-25, reseated 2026-08-26

The third rung of the ladder: `dependencies · data · functions`. The crates
say what the workspace depends on, the data chart says what it keeps, and this
one says **what it does**.

It is the data chart's dual, and it is built as one. The data chart says plain
ownership as **nesting** — a held type sits inside its holder — and this one
says the way-in call the same way: a declaration sits inside the caller that
reaches it first. Selecting a datum there lists what holds it; selecting a
function here lists what calls it, what it calls, and every type it touches —
and each of those types is a link down to `/data`, which is the rung that draws
types.

## The question it answers

*What runs from where?* (user decision, 2026-08-25.)

- An **entry point** is a declaration nothing in the workspace calls: `main`, a
  server function the client reaches through generated code, a component the
  router mounts, a method answering a foreign trait's contract. It wears the
  root's 2.5px ink left edge — the same mark the data chart's roots wear,
  because it is the same fact one rung down: nothing above it starts it.
- Every other mark is **as many calls in** as the shortest way something that
  starts reaches it, and that number is where it sits on the paper.
- A mark nothing reaches is **in a call ring**, said in words. Not dead — the
  survey cannot see every caller — but nothing on this paper starts it. The
  cartouche counts them, because "can I delete this" is a question a reviewer
  brings.

## Marks

Every `fn`, method, trait method clause and `macro_rules!` the visibility
reading admits. A block is a **head row and the signature quoted under it**
(2026-08-26, user — the signature came off the paper for one day and went back
on the same day, with the fold as its counterweight):

```
pub fn  FnModel::build(   A   views::func       −
    graph: &CodeGraph
    reading: &FnReading
) -> FnModel
────────────────────────────────
  ┌──────────────────┐  ┌─────────────────┐
  │ fn Seating::read( │  │ fn SigRow::quote(│   … the callees, shelved
```

The head row, run by run: the keyword as rust writes it; then — for a method —
**the owner, quoted out of the survey's own `head.label`** (`FnModel::`, soft
ink), which is how a reader learns whose method this is without a second
nesting system; then the name in the fn purple; the bracket the signature opens
with; the diff's letter; the module the declaration is written in, *only* where
the caller it seats inside is written in another one (a same-module call is
quiet, crossing one is signal); and the fold mark, where anything shelves
inside.

Under it, the signature as rust writes it — receiver, one row per parameter
(`name` in ink, type in type-teal, indented 12px), then the return on the
closing line. A block is measured to its own longest line and clamped to
152–300px, the same span the data chart clamps a block to; a longer line
ellipsizes and its hover words carry the whole of it. Nothing is reconstructed:
every row is the source's own text, from the same walk that quotes a struct's
fields (`analyze/data.rs` fills a method's *own* mark as well as the row it
files on the type's contract).

The sheet quotes the signature too, where there is room for the whole of it,
and `read it` / `enter` there opens the declaration's own **body** on the
quotation plate.

**Why the signature is worth the room.** Four hundred blocks four rows deep is
a sheet nobody can fit at once — that was the argument for taking it off, and it
was answered the right way instead: the reviewer folds the frames they are not
reading. A chart that quotes less of the source to fit is the wrong trade at
this altitude, where the block *is* the quotation.

## Two families of ink

- **Calls** — solid. At this altitude a body *is* the declaration: a struct's
  fields are its shape and a function's calls are its shape, so what would be
  body coupling one rung up is structure here.
- **Contracts** — dashed and lighter, labelled `answers`: a trait's own method
  clause and the methods that answer it. A call graph on its own lies about a
  trait-heavy workspace, because a `dyn` call lands on the clause and the code
  that runs is somewhere else. This family carries reachability across that
  gap, so a method answering a *workspace* trait is not an entry point. One
  answering a foreign trait (`Display`, `Iterator`) still is, and the limits
  fold says so.

Both rest on the dependent — the caller, the answering method — the way change
travels, as at every altitude.

**The way in is not a wire at all** (2026-08-26, user). For every mark there is
one call that put it where it sits — the shortest way something that starts
reaches it — and the chart draws that as **containment**: the callee seats
inside the caller's frame. So the tree costs no ink, and what is left to draw is
every *other* resolved call.

**The resting reading of those is two per mark**: each mark's heaviest crossing
call in each direction, which is the same rule the data chart rests its
references at. Hovering either end inks every wire that mark has, both ways
round; a selection's wires ink and stay inked. A contract wire never folds — it
is what makes the tree honest about a `dyn` call. Drawing all the resolved calls
at once would be exactly the hairball the Selection's-Ink rule forbids one rung
up.

**Resting ink goes under the blocks; ink the reader asked for goes over them**
(2026-08-27, user: *"don't you think the lines crossing over the boxes doesn't
look the best? they are too pronounced"*). The whole family used to paint above
the block layer, against the design's own layering rule, so every resting
diagonal crossed the quotations it passed — on a sheet whose subject is readable
source. Now the resting families and the strangers a reading pushed back are
drawn **beneath** the blocks: they keep the gutters between the frames on the
ground and between the shelved rows, and the paper of the head row and the
quotation covers them where they pass behind a block. Beneath the blocks they
can rest a step up — 0.34 for a wire the reading rests, 0.18 for one it merely
admits, 0.24 for one a selection dimmed — because a gutter is all they own. The
hover reading and the selection's lit wires stay **above**, at 0.85: a wire the
reader asked for has to be followable end to end, and the hair of transparency
is what keeps it from reading as a cut through the source it crosses.

**A wire ties on the edge of a block's band, and ends that share an edge fan
across it.** The band is the head row plus the quotation under it — a block's
own paper, since the rest of a frame's box is the shelf it holds. The tie is on
the side of that band facing the far end: the **top** where the far end is above
it, the band's **foot** where the far end is inside the block's own shelf,
otherwise the **near side**. Along the top and the foot the tie stays within the
first 150 units of the head, beside the name, because a frame is as wide as
everything it calls. The head row's own centre was the tie until 2026-08-27,
which is what drove the lines through the boxes: a wire had to cross half a head
row of quoted source to reach its end point, and a head six calls reach took six
arrowheads on one pixel — a blot exactly where the reader was looking for a
name. Ends sharing one edge now spread across it, ordered by where their far
ends stand so the fan spreads rather than braids.

The diff earns no place there either, and was tried: unfolding every wire that
merely touched a changed declaration washed a sixty-declaration change's whole
sheet amber. It was also untrue — the survey reads the base edition
syntactically, so a changed *call* is not a thing it can see. The diff's ink on
this chart is the block's own frame and its letter, and nothing else.

## The shelved section (chosen 2026-08-26 by the user)

**Containment is the call.** Every declaration seats inside the frame of the
caller that reaches it first, and its own callees shelve in wrapped rows inside
its frame — the same move the data chart makes when held state nests inside its
holder, one rung down.

- The **ground** is the entry points, packed in wrapped rows across the sheet at
  a landscape aspect, so the whole workspace fits a landscape glass. An entry
  frame wears the root's 2.5px ink left edge.
- A frame is measured from what it holds: the head row, then its callees packed
  into rows no wider than `max(head, widest child, sqrt(child area × 2.4))`.
  Pure and deterministic — the same survey always draws the same chart.
- A mark whose way in the visibility reading leaves off the paper stands on the
  ground too, without the entry edge; its tier words still say how far in it is.
- What no entry point reaches stands below the ground in the **ring strip**,
  dashed, under one caption: `in a call ring — no entry point reaches these`.
  Each ring is one frame with whatever it calls shelved inside it, not a scatter
  of loose blocks.

Three seatings were built on 2026-08-25 and the kept one was retired the next
day, with two prototypes read and rejected in between:

- **strata** (retired 2026-08-26) — the band × prism **section**: full-width
  bands of call depth crossed by one prism per module, type or file, as the
  `group` reading asked. It read both facts at once and the diff came straight
  off it, but the ground it drew was the module tree, not the call tree: the
  question this altitude asks was answered by wires rather than by the paper,
  and a sheet of four-row blocks never fitted one glass. The `group` reading
  went with it — the ground is the call tree now, and a mark's module is a word
  on its head.
- **mechanism** (cut 2026-08-25) — module frames nested the way rust's modules
  do, parts in depth rows inside them. It read *whose code* before *what runs
  from where*, and the packing left about a quarter of the sheet empty.
- **strips** (cut 2026-08-25) — one row per road, stations running right. On
  real data a twenty-thousand-unit ribbon that never fit the glass.
- **icicle** (prototype, rejected 2026-08-26) — the call tree as a sparse
  icicle: one column per depth, a block's children stacked in the column to its
  right. Honest about the tree and unreadable at this size — the width went to
  depth rather than to names, and most of the sheet was blank.
- **the loose seating** (prototype, rejected 2026-08-26) — the same containment
  idea with the signature still quoted on every block. A few dozen declarations
  filled the glass, and for one day the answer was the head row alone. The real
  answer, the same day: keep the quotation and give the reviewer a **fold**, so
  the density is theirs to spend rather than the chart's to refuse.

## Folds (2026-08-26, user)

The sheet is bigger than one glass, and that is what folds are for. Three rules
carry it, and they are the same three the data chart's module and holder folds
keep — recorded once in DESIGN.md as the **Hand-Fold Rule**:

- **Its own mark, its own gesture.** Every frame that shelves anything wears
  `–` / `+` at the **right-most end of its head row**, where the data chart puts
  its own (2026-08-27, user: *"the fold button should be on the right most side?
  make the ui ux consistent please."*). It sat beside the name for one build,
  because a head row is as wide as its frame and a frame is as wide as everything
  it calls — right-aligned, the mark for a two-thousand-unit frame is a thousand
  pixels from its own name and off the glass. The mark now **rides the glass**
  instead: `FN_SLIDE_JS` writes `--cam-r` (the world x under the viewport's right
  edge) on the chart root once per animation frame, each frame carries its own
  `--own-r` and `--own-w`, and the mark translates by the difference, clamped so
  it never leaves its frame. No component reads the camera, so a pan costs no
  re-render — the block layer is the one this system may never invalidate while
  the pointer is merely travelling. The counted words ride with it. The head beside it still selects, the
  border still selects the boundary, and no gesture is ever another's side
  effect. `z` does it from the keyboard (vim's fold key), on whatever frame is in
  hand; shift- or alt-click takes the whole subtree, closing every frame under it
  or opening every fold inside it at once.
- **A fold is an elision, not a re-layout** (2026-08-27, user: *"when a thing is
  folded, try not to re-layout? because it just disrupts the visual anchor."*).
  Folding takes a frame's contents off the *drawing*; the layout still measures
  and still places every one of them, so the frame keeps its whole footprint and
  no sibling, ancestor or wire moves by a pixel. The reviewer's eye stays where
  they left it, which is the reason they folded something next to it. It is
  deterministic, it is per-session state (kept like the camera, never in the
  URL), and **the camera does not move for it** — fold is not focus.
- **The compact packing is a reading's job, not a fold's.** The packer is allowed
  to skip a fold only where the paper is being laid again anyway: an `order` or
  `visibility` change, or the session's first build. `FnReading` carries two
  sets for it — `folds`, and the `packed` subset the packer skipped — and
  `FnState::repack` is the only thing that widens the second one. Opening a fold
  the packer *did* skip has to give its contents room, so that one re-lays and
  leaves both sets at once (`FnState::fold`). The same two sets, and the same
  rule, sit on the data chart's holder folds as `blocks` / `packed_blocks`.
- **Nothing is silently cut.** A folded frame keeps its head *and its whole
  signature* — the quotation is the mark's identity — and writes `+ 41 inside`
  where the shelf stood, itself a control that puts the shelf back. Every wire
  whose far end the fold hides **re-anchors to the folded frame's head** rather
  than disappearing (the frame stands for what it hides), with two calls that
  gather onto one line gathering their counts too. A selection, a search hit, a
  sheet row or an arrow step landing inside a fold **opens the way in first**,
  outermost fold first: a selection the reader cannot see is not a focus. And
  where a fold hides part of a lit chain, its head carries full ink, because
  recede acts on paint and never on the box.

Nothing folds itself, and nothing folds by a count — no depth, no budget, no
item cap. The retired surface chart's `MARK_BUDGET` is the standing example of
what this rule exists to prevent.

## Order (2026-08-26, user)

The ground is the call tree, so nothing here is a *box* except a caller's frame:
a second nesting system per module or per type would fight the one the chart is
about. What a declaration is written in and whose method it is are read as the
order its shelf seats in, and as words on its own head. On the cartouche as
`order`:

- **weight** (the default) — heaviest chain first: a callee carrying a hundred
  declarations seats before one carrying none.
- **module** — siblings cluster by the module they are written in, so a frame
  reaching across the workspace says which parts of it.
- **owner** — siblings cluster by the type or trait whose impl they are written
  in, so a type's methods seat together, with the free declarations clustered
  first. Weight still breaks ties inside a cluster. This is the answer to "show
  me this type's methods together" that costs no second ground; the head's own
  `FnModel::` prefix is the other half of it.

The order moves shelves, never the tree: the way in is the shortest way in
whatever order the shelves are read. Each order keeps its own camera.

## Readings and focuses

- **visibility** — the same four-stop slider the data chart slides, reading the
  same keyword each declaration writes. `n off` at its foot; naming a
  declaration in the search widens the reading to the stop that draws it.
- **wires** — `calls · callers · both`: **which way round** the paper reads what
  the shelving cannot say. It is the data chart's `references` reading one rung
  down, and it is read the same way: direction only means something against an
  anchor, and the anchor is whatever is in focus — the **selection** where there
  is one, else the **diff** on the resting plate (`calls` draws what the changed
  declarations run, `callers` whose code runs them), else nothing, in which case
  every stop draws every wire because a reading with nothing in focus has no
  direction to take. Hovering a mark is not a direction: it inks *everything*
  that mark calls and everything that calls it, in a layer of its own, because
  what a reader hovers a block for is what the shelving could not tell them.

  The stops were `rest · all · selection` until 2026-08-27 — three amounts of
  ink, which is a word about the drawing rather than about the code (user: *"I
  don't understand the wires rest, all, selection. I thought we should just show
  callers, callees, or both?"*). The old resting texture, each mark's heaviest
  crossing call in each direction, went with them: the diff anchor is what thins
  the resting plate now, and a workspace with no diff draws the lot.
- Every focus is a URL: `/fn`, `/fn/mark/:..path?item=` (with `peek=` for a
  quoted row — the mark's own source included), `/fn/tree/:..path?item=` (one
  frame's whole boundary), `/fn/mod/:..module` (every declaration written in that
  module — the crate, then the module path, and a parent module lights the
  modules under it), and `/fn/depth/:band` (one whole band). Back retraces the
  trail; `f`, `/` and Escape behave as at every altitude, and the arrows walk the
  seating. A fold is the one review state that is *not* a URL.
- A **mark selection is geometric**, because the seating is the reading: the
  frames it stands inside and everything shelved within it keep full ink, the
  blocks at the far end of its lit wires read a step behind, every stranger
  recedes to 0.32. The recede acts on a block's paint and never on its box — a
  receded frame can hold a lit block.
- A **boundary selection** is the dual of the data chart's module boundary, and
  it is the box read as what it is: a subtree. Clicking a frame's border (a 10px
  transparent stroke over the box's rectangle, on the ground layer, so the
  interior stays open paper for the heads, the wires and the pan) selects
  everything shelved inside. **The border says at rest that it is a control**
  (2026-08-27): a frame that shelves anything draws its box in soft ink where a
  leaf draws the hairline, and its hover words teach the gesture in place —
  `everything survey calls, down the chain — 41 declarations · click to select
  the whole box`. Hover ink alone taught nobody, because nobody puts a pointer
  on a box's edge without a reason to. Inside keeps full ink; every mark one call across
  the line reads a step behind, and so do the frames the box stands inside, which
  are the paper it is drawn on; every stranger recedes. Every wire that crosses
  the boundary inks and stays inked — what enters and leaves the box is what a
  reader came to the boundary for — and wires wholly inside stay as the wires
  reading draws them. Its sheet **lists** the crossings rather than counting
  them: `Called from outside` and `Calls out`, heaviest first, eight rows before
  `show all n`, each a link.
- A **band** reads differently, and has to: a stratum holds sixty marks, so
  "touches the selection" would be most of the sheet. A band lights every mark
  at its depth and reads the frames each of them stands inside a step behind —
  those frames *are* the way in — and folds no wire back.
- The **keys walk the seating**, and only up and down: `↓` into the **first
  callee on the shelf**, `↑` to the caller the block sits in, `z` to fold the
  frame in hand, `enter` to read its source, double-click to fit a frame's
  subtree, `f` to fit the sheet. Which callee is first is the `order` reading's
  answer, not the key's — heaviest chain under `weight`, the first cluster under
  `module` or `owner` — so the keys line says `the first callee on the shelf` and
  not `the heaviest callee`, which it said until 2026-08-27 while the handler
  took `kids.first()`.
- **`←`/`→` are the trail**, whatever is selected (2026-08-27, user: *"left and
  right should mean navigate backwards and forward."*). They walked the shelf
  sideways while a mark was in hand until then, which is one pair of keys with
  two meanings and no way for a reader to tell which is live. A shelf is walked
  by clicking, and by `↓` into it and `↑` back out. This chart still answers the
  two keys itself — the shell's own arrow listener stands down on `/fn`, so only
  one of them acts on a press. `enter`
  stands down while the page's own focus is on a control — a head row, a fold
  mark and a sheet row each answer Enter themselves — and it means the same
  thing when they do: a head row that is not the selection yet is selected by
  it, and a head row that *is* the selection opens its source, never
  deselecting. Deselecting is the click's job.
- The keys line also names the two gestures no key carries: **clicking a frame's
  border** takes the whole box, and **shift-clicking a fold mark** folds all the
  way down. Both lived only in a hover title before 2026-08-27, which is to say
  nowhere a reader looking at the chart would find them.

## What the sheet says

The declaration's own words first — the keyword run and name, the impl section
**with the owner's name as a descent link** (to its block on `/data`, or to its
own quotation where that chart draws none — the same rule the `Data touched`
rows keep), the module as a link that lights everything written in it, the
locator with `read it` beside it, and the **signature quoted as rust writes
it** — then two relation headings and one descent, in the data sheet's own
discipline: `Called by` (with `answers` for the contract, which is the one caller
a call graph cannot see), `Calls`, then `Data touched` — every workspace type the
signature names (`signature`) or the body uses (`12 refs`), each row linking to
that type's block on `/data`, or quoting its source where `/data` draws no block
for it either. An empty `Called by` is the whole verdict in one sentence:
`nothing in the workspace calls it.`

**The whole declaration is one step away** (2026-08-26, user). `read it`, the
quoted signature block, and `enter` all open the selected mark's own source on
the quotation plate — `peek=<file>@<label>` naming the selection itself, so back
closes it and Escape steps out of the quotation before the selection. A method is
quoted inside its `impl` block, gutter marking the jump, exactly as the data
chart quotes one. The signature answers *what does it take*; the next question is
always *what does it do*, and it was the one move this chart made a reviewer
leave for.

## Implementation notes

- `src/views/func/{model,layout,map,chrome,mod,quote}.rs`. `FnModel::build`
  reads the seating (`via` / `kids` / `seats` / `ring` / `reach`) and then the
  folds over it: `FnReading::folds` is a set of (file, label) pairs, `folded` is
  the ids it resolves to, `FnReading::packed` is the subset the packer may skip
  (`model.packed`), and `packs` maps every hidden mark to the outermost fold
  standing for it. The seating itself never changes when a fold does, and neither
  does the layout: `Measure::kids_of` skips only `model.packed`, so a fold by hand
  still gets every footprint measured and placed. `FnDrawing::build` measures
  **every** mark for that reason, seats a frame by its `own_open` box (the head
  and quotation without the counted words, so the shelf keeps the room it had),
  and pushes a node only for the marks a fold has not hidden. The model's `calls`
  keep their real ends while `FnDrawing` reads both ends through
  `FnModel::shown`.
- The `wires` reading never re-reads the survey, so the direction is applied
  where the ink is: `FnWires::draws` in `FnKin::mark` / `FnKin::tree` for a
  selection anchor, and `WireView::{def_dirty, user_dirty}` — the diff's word
  about each end, carried on the wire — for the resting plate. `drawn_under` puts
  the two together. `kin` is a memo over `(selection, wires)`, because the same
  chosen mark keeps different wires in each direction.
- `FnModel::reveal(id)` is the fold path a focus opens, outermost first; the
  chart runs it in an effect keyed on the *selection* alone (the model peeked,
  never read), so navigating into a fold opens it while folding the frame a
  selection sits in does not fight the reader — the folded head stands for the
  selection instead, and `FnKin::carry` gives it the lit ink.
- The wires are drawn by **three world layers** over one `Vec<WireView>`, and
  which layer a wire lands in is the only thing the split changes: `.fn-wires`
  (`z-index: 0`, under the flow's own viewport) takes everything at rest,
  `.fn-wires-lit` (`z-index: 2`) takes the wires a selection inked, and
  `.fn-wires-hot` (`z-index: 2`, `will-change: transform`) takes the settled
  hover's own copies. `WireLayer` draws both plates from one component with an
  `over` flag, so the two together are exactly the set one layer drew before.
  The hover layer stays what it was — its own compositor layer, no mount
  animation, never re-rendered by a pointer that is merely travelling.
- The tie geometry lives with the box, in `layout.rs`: `Placed::tie_side` picks
  the facing edge of the block's band (`TieSide::{Top, Under, Left, Right}` —
  `own` height from `Sizes::own`, the same number the seating reserved) and
  `Placed::tie_at` places the point along that edge from a 0..1 slot.
  `FnDrawing::build` then runs one **fan pass**: every (mark, side) pair
  collects the ends that tie there, sorts them by where their far ends stand
  (wire index breaking the tie, so one survey draws one chart) and hands each a
  slot. A `HashMap` walk is safe here because each pass writes one distinct
  wire end.
- The boundary rings are one `EdgeLayer` on the ground layer: two rects per
  shelving frame (a 10px transparent `pointer-events: stroke` hit and a 1.2px
  line that inks on hover), painted outermost first so a nested boundary wins
  the click.
- The **far edition's landmark register** is one `NameLayer` above the blocks
  (`z-index: 30`, `pointer-events: none`), one engraved `text` per frame
  standing on the ground — `model.seats` and `model.ring`. `NameView::measure`
  sizes it: `min(w × 0.055, the width the identifier needs, the room under the
  head row)`, capped at 54 and refused below 26, so the name always fits the box
  and an identifier is never broken or clipped. Only the ground is engraved —
  the seating nests ten deep and a name per frame would be ten names over one
  square of paper — and the block's own `.fm-nm` still draws at reading size at
  every zoom. `PAD` is shared with `layout` so the landmark stands on the same
  left edge as the shelf under it.
- The **far wire pressures** are the one place the `is-far` edition subtracts
  rather than adds. `stroke-width: 2px` now reaches only `.is-answers`,
  `.is-kin` and `.is-hot`; the resting families (`.is-quiet`, `.is-faint`) and
  anything a selection dimmed (`.is-dim`) drop to 0.06 at their engraved width.
  A contract at rest keeps opacity 0.9 (0.45 through the dash's own 0.5), being
  the one edge containment cannot draw.
- `FnState` carries the folds beside the three readings on the app shell, so
  stepping out to another rung and back keeps them, and the camera keeps one
  viewport per `order`.

## Not here

- **No fold by anything but a hand.** Zoom is one disclosure (below 0.45 the far
  edition retires the quoted rows and the head's secondary runs, quiets the
  resting wires and engraves the ground frames' names, back above 0.55), the
  visibility slider is the narrowing, the reviewer's own fold mark is the
  third — and no count, depth or budget ever folds anything.
- **No block grown to fit its box.** The landmark register is a mark of its own
  on a layer of its own; `.fm-nm` keeps the reading ramp at every zoom. A block
  sits in a box the call tree sized, so a name measured to that box would be a
  different size in every frame — which is why `--far-name` and the
  `far-name-fn` step were retired on 2026-08-26 and stay retired. A frame's
  *territory* is a different subject, and naming a territory is what an atlas
  does.
- **No fit-to-illegible first paint.** Where the whole sheet cannot be read at
  once the chart holds 0.34 and opens on the **start** of the ground — its
  top-left corner, where the heaviest entry frame sits, because the shelves read
  left to right and heaviest first. (Centring on the bounds of every entry point
  at once put the reader in the middle of the widest gap between two of them,
  looking at blank paper; fixed 2026-08-26 when the restored signatures made the
  sheet big enough for it to matter.) `f` still fits the whole sheet at whatever
  zoom that takes.
- **No per-call diff.** The structural diff reads the base edition
  syntactically, so it is exact about declarations (`A`, `M`, a flare frame, a
  `+` on an added parameter) and says nothing about a rewritten body. The
  limits fold states it.
- **No second colour.** Flare still means CHANGED and nothing else; a
  function's name takes the quotation palette's fn colour, which is token
  class inside a quotation and stops at the block's frame.
