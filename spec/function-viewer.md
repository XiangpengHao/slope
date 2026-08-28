# The function viewer (`/fn`), built 2026-08-25, reseated 2026-08-27

The third rung of the ladder: `dependencies · data · functions`. The crates
say what the workspace depends on, the data chart says what it keeps, and this
one says **what it does**.

It is the data chart's dual, and since 2026-08-27 it is built on the same
ground. The data chart draws the workspace's household — a crate frame, module
frames nested the way rust's modules nest — and seats a type in the module that
declares it. This one draws the same household and seats a **function** the way
rust owns one: a method belongs to the type or trait whose impl block declares
it, a free declaration belongs to its module. So containment means *written-in*
on both charts, and the two are one grammar with two duals — `/data` draws what
a type keeps, `/fn` draws what it does.

## The question it answers

*What runs from where?* (user decision, 2026-08-25.) The **household** is how it
answers it (user decision, 2026-08-27: *"enhance the concept of ownership…
functions belong to struct (if member method) or mod. we should see similar
boxes like the data view — outer crate, then mod, then zoom into struct or enum
container. and we use lines to indicate caller and callee. when visibility is
pub, we should clearly see the public methods of each mod."*).

- An **entry point** is a declaration nothing in the workspace calls: `main`, a
  server function the client reaches through generated code, a component the
  router mounts, a method answering a foreign trait's contract. It wears the
  root's 2.5px ink left edge — the same mark the data chart's roots wear,
  because it is the same fact one rung down: nothing above it starts it. It is
  a **fact about the code, and no longer a place**: the household seats it
  wherever it is written.
- Every other mark is as many calls in as the shortest way something that starts
  reaches it, said in its hover words and on its sheet (`3 calls from an entry
  point · runs 41 by the way in`) and selectable as a band (`/fn/depth/:band`).
- A mark nothing reaches is **in a call ring**, said in words. Not dead — the
  survey cannot see every caller — but nothing on this paper starts it. The
  cartouche counts them, because "can I delete this" is a question a reviewer
  brings.

## The ground: the household

Exactly the ground the data chart draws, and drawn by the same packer.

- **A crate frame** per workspace crate, labelled with its cargo package name
  where the survey has more than one to tell apart.
- **A module frame** per module, nested as rust nests them, the last path
  segment alone on the border (`mod func` inside `mod views`) because the
  paper's own nesting says the rest. Border click selects (`/fn/mod/:..module`),
  the mark at the border's other end folds.
- **An owner container** per type or trait that declares methods, standing in
  the module the *type* is declared in and gathering **every impl block that
  type has, wherever in the workspace they are written**. Labelled in rust's own
  words: `impl FnModel`, `trait Chart`. A container can only stand in one place,
  and the type's own module is the place the data chart draws that type in, so
  that is where it stands. A method whose own source lives elsewhere still says
  so — its sheet's module line is its own file's module.
- **Free declarations** sit on the module's own shelf, outside every container.
- Inside a container and on a module's shelf the order is **declaration order as
  the source writes it** — (file, line). Nothing is alphabetized and nothing is
  ranked; the line number is the honest order, and it is the same order the
  Quoted-Source Rule keeps everywhere else.
- **A room with nothing admitted draws no husk.** The visibility reading takes
  declarations off the paper, and a module or a container the reading emptied is
  simply not built — the same rule the data chart's frames keep.

Three marks on a room's border and they are three gestures, never one: the
**border** selects, the **label** selects too (a container's owner name descends
to that type's block on `/data` instead), and the `–` / `+` at the border's
other end **folds** what is inside.

## Marks

Every `fn`, method, trait method clause and `macro_rules!` the visibility
reading admits. A block is a **head row and the signature quoted under it**:

```
pub fn  build(   A                    ← the head
    graph: &CodeGraph                 ← the quotation
    reading: &FnReading
) -> FnModel
```

The head row, run by run: the keyword as rust writes it; the name in the fn
purple; the bracket the signature opens with; and the diff's letter. Under it,
the signature as rust writes it — receiver, one row per parameter (`name` in
ink, type in type-teal, indented 12px), then the return on the closing line. A
block is measured to its own longest line and clamped to 152–300px, the same
span the data chart clamps a block to; a longer line ellipsizes and its hover
words carry the whole of it. Nothing is reconstructed: every row is the source's
own text.

**The `Owner::` prefix is off the head** (2026-08-27). It stood there while the
ground was the call tree, because nothing else on the paper said whose method a
block was; the container says it now, and saying it twice cost every method row
the width of its own type's name. The qualified label (`FnModel::build`) still
rides the search rows, the sheet, the fold keys and every URL. **The module word
is off the head** for the same reason: a block's module is the frame it is drawn
in.

The sheet quotes the signature too, where there is room for the whole of it, and
`read it` / `enter` there opens the declaration's own **body** on the quotation
plate.

## Two families of ink, and every call is a wire

- **Calls** — solid. At this altitude a body *is* the declaration: a struct's
  fields are its shape and a function's calls are its shape.
- **Contracts** — dashed and lighter, labelled `answers`: a trait's own method
  clause and the methods that answer it. A call graph on its own lies about a
  trait-heavy workspace, because a `dyn` call lands on the clause and the code
  that runs is somewhere else. This family carries reachability across that gap,
  so a method answering a *workspace* trait is not an entry point. One answering
  a foreign trait (`Display`, `Iterator`) still is, and the limits fold says so.

Both rest on the dependent — the caller, the answering method — the way change
travels, as at every altitude.

**Every call is a wire** (2026-08-27). The shelved section drew the way-in call
as containment and spent ink only on the rest; the household's containment says
*whose code this is*, which no call can stand for, so a call not drawn is a call
not said.

**The resting reading is the corridor.** With nothing in focus, calls between
two different module frames bundle into **one line per ordered module pair**,
border to border, with the count it carries riding it and its own hover words
(`views::func calls graph::data · 87 calls · select graph::data`); clicking a
corridor selects the module at its far end, so a count on this chart always has
a subject a reader can reach. Calls *inside* one module rest as their own short
lines. Drawing all fifteen hundred cross-module calls at once is the hairball
this system forbids one rung up; drawing none of them would be a chart that says
nothing about how the modules talk.

**An anchor unbundles.** A selection draws that anchor's own wires in the
direction the `wires` reading names and takes the corridors off the paper — the
reader asked about one box, and a bundle answers about a whole module. Hovering
a mark inks every wire it has, both ways round, in its own layer. With a diff
and no selection the corridors **keep standing** — they are the shape — and the
individual lines thin to the ones the diff anchors: a workspace under review is
nearly always dirty, so a rule that took the corridors away whenever the diff
had something to say would be a resting reading almost nobody ever saw.

**A contract never bundles and is never counted into one.** It is drawn whatever
the reading rests, so a corridor that counted it would be counting a line
already on the paper.

**Resting ink goes under the blocks; ink the reader asked for goes over them**
(2026-08-27, user: *"don't you think the lines crossing over the boxes doesn't
look the best? they are too pronounced"*). The resting families and the
strangers a reading pushed back are drawn **beneath** the blocks: they keep the
gutters on the ground, and the paper of a head row and its quotation covers them
where they pass behind a block. Beneath the blocks they rest a step up — 0.34
for a wire the reading rests, 0.18 for one it merely admits, 0.24 for one a
selection dimmed. The hover reading and the selection's lit wires stay **above**,
at 0.85.

**A wire ties on the edge of a block's band, and ends that share an edge fan
across it.** The band is the head row plus the quotation under it. The tie is on
the side of that band facing the far end: the **top** where the far end is above
it, the band's **foot** where it is below, otherwise the **near side**. Along the
top and the foot the tie stays within the first 150 units of the head, beside the
name. The head row's own centre was the tie until 2026-08-27, which is what drove
the lines through the boxes: a wire had to cross half a head row of quoted source
to reach its end point, and a head six calls reach took six arrowheads on one
pixel. Ends sharing one edge now spread across it, ordered by where their far
ends stand so the fan spreads rather than braids.

The diff earns no place there either, and was tried: unfolding every wire that
merely touched a changed declaration washed a sixty-declaration change's whole
sheet amber. It was also untrue — the survey reads the base edition
syntactically, so a changed *call* is not a thing it can see. The diff's ink on
this chart is the block's own frame and its letter, and nothing else.

## Folds (2026-08-26, carried over 2026-08-27)

The sheet is bigger than one glass, and that is what folds are for. The two
things a hand folds are the two rooms: a **module frame** and an **owner
container**. A mark is a leaf and folds nothing. Three rules carry it, and they
are the same three the data chart's folds keep — recorded once in DESIGN.md as
the **Hand-Fold Rule**:

- **Its own mark, its own gesture.** Every room wears `–` / `+` at the
  right-most end of its border, where the data chart puts its own. The mark
  **rides the glass**: `FN_SLIDE_JS` writes `--cam-r` (the world x under the
  viewport's right edge) on the chart root once per animation frame, each room
  carries its own `--own-r` and `--own-w`, and the mark translates by the
  difference, clamped so it never leaves its room. No component reads the
  camera, so a pan costs no re-render. `z` does it from the keyboard on the room
  the selection is in; shift- or alt-clicking a module's mark takes every room
  under it.
- **A fold is an elision, not a re-layout** (2026-08-27, user: *"when a thing is
  folded, try not to re-layout? because it just disrupts the visual anchor."*).
  Folding takes a room's contents off the *drawing*; the layout still measures
  and still places every one of them, so the room keeps its whole footprint and
  no sibling, ancestor or wire moves by a pixel. It is deterministic, it is
  per-session state (kept like the camera, never in the URL), and **the camera
  does not move for it** — fold is not focus. Measured live: folding
  `views::data` takes 125 blocks off the paper and every one of the 305 that
  remain keeps its exact on-screen box, camera included.
- **The compact packing is a reading's job, not a fold's.** The packer is allowed
  to skip a fold only where the paper is being laid again anyway: a `visibility`
  change, or the session's first build. `FnReading` carries two sets for it —
  `folds`, and the `packed` subset the packer skipped — and `FnState::repack` is
  the only thing that widens the second one. Opening a fold the packer *did* skip
  has to give its contents room, so that one re-lays and leaves both sets at once
  (`FnState::fold`).
- **Nothing is silently cut.** A folded room keeps its border, its label and its
  fold mark, and writes `+ 41 inside` under the label. Every wire whose far end
  the fold hides **re-anchors to the folded room's border** rather than
  disappearing, with two calls that gather onto one line gathering their counts
  too. A selection, a search hit, a sheet row or an arrow step landing inside a
  fold **opens the way in first**, outermost fold first. And where a fold hides
  part of a lit chain, its border carries full ink, because recede acts on paint
  and never on the box.

Nothing folds itself, and nothing folds by a count — no depth, no budget, no
item cap. The retired surface chart's `MARK_BUDGET` is the standing example of
what this rule exists to prevent.

## Readings and focuses

- **visibility** — the same four-stop slider the data chart slides, reading the
  same keyword each declaration writes. `n off` at its foot; naming a
  declaration in the search widens the reading to the stop that draws it. This
  is the **API lens**: at `pub(crate)` each module frame reads as exactly its
  crate-public surface — the free functions it publishes and, in each container,
  the methods that type publishes — and the rooms the stop emptied are gone
  rather than standing as husks.
- **wires** — `calls · callers · both`: **which way round** the paper reads its
  calls. It is the data chart's `references` reading one rung down, and it is
  read the same way: direction only means something against an anchor, and the
  anchor is whatever is in focus — the **selection** where there is one, else the
  **diff** on the resting plate (`calls` draws what the changed declarations run,
  `callers` whose code runs them), else nothing, in which case the corridors and
  the intra-module lines rest whichever stop is on. `both` is a *direction* and
  never an amount: both ways round whatever is in focus. It drew the whole
  family under a diff until 2026-08-27, which is the exact word-about-the-drawing
  the stops were renamed away from, and with the corridors standing it was also
  the hairball they exist to prevent.
- Every focus is a URL: `/fn`, `/fn/mark/:..path?item=` (with `peek=` for a
  quoted row), `/fn/impl/:..path?owner=` (one owner container), `/fn/mod/:..module`
  (one module boundary), and `/fn/depth/:band` (one whole band). Back retraces
  the trail; `f`, `/` and Escape behave as at every altitude. A fold is the one
  review state that is *not* a URL.
- A **mark selection** lights the mark, inks its own wires in the chosen
  direction and keeps them inked, reads the block at the far end of each a step
  behind, and recedes every stranger to 0.32. The recede acts on a block's paint
  and never on its box — a receded room can hold a lit block.
- An **owner-container selection** is the room read as what it is. Clicking its
  border selects every method written on that type or trait; inside keeps full
  ink, everything one call across the line reads a step behind, and every wire
  crossing the border inks and stays inked in the direction the reading names.
  Its sheet **lists** rather than counts: `Methods` in declaration order (keyword,
  visibility, name, diff letter, locator, each a link), then `Called from
  outside` and `Calls out`, heaviest first, eight rows before `show all n`.
- A **module boundary** is the dual of the data chart's, and reads the same way:
  everything written inside keeps full ink, everything one call across the line
  reads a step behind, every crossing wire inks and stays inked, and every other
  room recedes. **There is no sheet** — a module is a place on the paper, and the
  paper is already saying it, which is the rule one rung up.
- A **band** reads differently, and has to: a stratum holds sixty marks, so
  "touches the selection" would be most of the sheet. A band lights every mark at
  its depth and folds no wire back.
- The **keys**: `↓` into the picked mark's **heaviest callee**, `↑` to its
  heaviest caller — a glide across the paper now, because the household seats a
  callee wherever its own code is written, and the glide only happens where the
  target is not already legible. `z` folds the room the selection is in, `enter`
  reads the source, `f` fits the sheet, `/` finds, and **double-clicking a
  room's border** fills the glass with that room.
- **`←`/`→` are the trail**, whatever is selected (2026-08-27, user: *"left and
  right should mean navigate backwards and forward."*). This chart answers the
  two keys itself — the shell's own arrow listener stands down on `/fn` — so only
  one of them acts on a press. `enter` stands down while the page's own focus is
  on a control.
- The keys line also names the gestures no key carries: clicking a room's
  **border** takes the whole room, double-clicking it fills the glass with it,
  clicking an owner's **name** goes down to its data, and **shift-clicking** a
  module's fold mark folds all the way down.

## What the sheet says

The declaration's own words first — the keyword run and name, the impl section
**with the owner's name as a descent link** (to its block on `/data`, or to its
own quotation where that chart draws none), the module as a link that lights
everything written in it, the locator with `read it` beside it, and the
**signature quoted as rust writes it** — then two relation headings and one
descent: `Called by` (with `answers` for the contract, which is the one caller a
call graph cannot see), `Calls`, then `Data touched` — every workspace type the
signature names (`signature`) or the body uses (`12 refs`), each row linking to
that type's block on `/data`. An empty `Called by` is the whole verdict in one
sentence: `nothing in the workspace calls it.`

**The whole declaration is one step away** (2026-08-26, user). `read it`, the
quoted signature block, and `enter` all open the selected mark's own source on
the quotation plate — `peek=<file>@<label>` naming the selection itself, so back
closes it and Escape steps out of the quotation before the selection.

## The far edition

Below 0.45 zoom (back above 0.55) the quoted rows and the brackets retire, the
entry edge widens to 8px and the focus ring to 5px at 6px offset, and the
**landmark register** takes over: each room's own name engraved across its
territory on a layer above the blocks — the modules first, then the heaviest
owner containers — measured to the box, never guessed, on the far ramp between
26 and 54px, each one a step smaller than the room it stands in and clear of its
ink. A room whose box cannot carry the floor gets no landmark and keeps its
reading-size label. A room's fold mark and its counted words take the far ramp
too, because the one thing a fold may never do is go quiet about what it is
holding.

**The individual resting lines retire and the corridors stay.** Thickening every
wire was the far edition's one loud mistake: the sheet rests several hundred
diagonals, and at far zoom they became the heaviest ink on a paper whose blocks
were three pixels of type. What answers the question at that zoom is *which
module calls which*, which is exactly what a corridor says — so a corridor takes
the far ground pressure and its count reads at 22px, contracts keep their
pressure, and the individual resting lines go. Nothing is cut: pan in past 0.55
and every one of them is back at its resting pressure.

## Superseded

Each of these shipped or was prototyped, and each is recorded because the
reasons are the ones that keep coming back.

- **the shelved call ground** (superseded 2026-08-27 by user decision) — the
  ground was the **call tree**: every declaration seated inside the frame of the
  caller that reached it first, its callees shelved in wrapped rows inside its
  frame, the entry points on the ground, a ring strip below them, and ink spent
  only on the calls that seating could not say. It was honest about *what runs
  from where* and it read well at a mark. What it cost was the question a
  reviewer actually opens the chart with — *whose code is this* — which the sheet
  answered only as an `Owner::` prefix, an `order` reading and a word on a head.
  The user asked for ownership to be the ground instead, and for the two charts
  to draw the same rooms. Everything it invented that was not about the call tree
  survives it: the head row and its quotation, the fold machinery and its
  in-place elision, the edge ties and the fan, the two wire altitudes, the
  landmark register, the `wires` reading, and the visibility slider.
- **the `order` reading** (retired with it) — `weight · module · owner`, the
  order the callees on one shelf were seated in. The household has one order and
  the source writes it, so the control had nothing left to say. Its `owner` stop
  was the answer to "show me a type's methods together" that cost no second
  ground; the container is that answer now, and it is a box.
- **`/fn/tree/:..path?item=`** (retired with it) — one frame's whole boundary
  read as a subtree, because containment was the call. A box is a room now, so
  the boundary focus is `/fn/impl/:..path?owner=` and `/fn/mod/:..module`.
- **the ring strip** (retired with it) — what no entry point reached stood below
  the ground under one caption, because the call tree had nowhere else to put it.
  The household has somewhere: a ring's declarations are drawn in the rooms that
  declare them, dashed, and `/fn/depth/:band` still selects the band from the
  cartouche's own count.
- **strata** (retired 2026-08-26) — the band × prism **section**: full-width
  bands of call depth crossed by one prism per module, type or file, as the
  `group` reading asked. It read both facts at once and the diff came straight
  off it, but the ground it drew was the module tree, not the call tree: the
  question that altitude then asked was answered by wires rather than by the
  paper, and a sheet of four-row blocks never fitted one glass.
- **mechanism** (cut 2026-08-25) — module frames nested the way rust's modules
  do, parts in depth rows inside them. It read *whose code* before *what runs
  from where*, and the packing left about a quarter of the sheet empty. The
  household is its rehabilitation: the reading it lost to was retired in turn,
  and what it got wrong was the *inside* of a frame — depth rows rather than
  ownership — not the frames.
- **strips** (cut 2026-08-25) — one row per road, stations running right. On
  real data a twenty-thousand-unit ribbon that never fit the glass.
- **icicle** (prototype, rejected 2026-08-26) — the call tree as a sparse icicle:
  one column per depth, a block's children stacked in the column to its right.
  Honest about the tree and unreadable at this size.
- **the loose seating** (prototype, rejected 2026-08-26) — containment-as-call
  with the signature still quoted on every block. A few dozen declarations filled
  the glass, and for one day the answer was the head row alone. The real answer,
  the same day: keep the quotation and give the reviewer a **fold**.

## Implementation notes

- `src/views/func/{model,layout,map,chrome,mod,quote}.rs`.
- `FnModel::build` reads the marks, the two families, the tier, and then the
  household (`House::read`): the frames from the module of every drawn mark's
  **owner** (or its own file, for a free declaration), the containers from
  `ItemMark::parent` — the survey's own semantic container, resolved through the
  impl's self type even when the impl sits in another file — and then the folds
  over both.
- `Spot` is the one address the whole chart speaks: `Mark(id)`, `Owner(id)`,
  `Frame(id)`. Wires tie to spots rather than to marks, so a fold re-anchors ink
  by mapping a hidden spot to the room that stands for it (`FnModel::shown`).
- `FnLayout` packs the household with the data chart's own `skyline`, so two
  charts of one workspace never disagree about how a shelf fills. A fold by hand
  is measured and placed in full; only `model.packed` — the folds the packer was
  allowed to skip — is seated as its own label and counted row.
- `FnDrawing::build` measures **every** mark for that reason and pushes a node
  only for the ones a fold has not hidden. The corridors are built after the
  individual wires, from the module frame each end stands in, and carry the
  summed reference count.
- The `wires` reading never re-reads the survey, so the direction is applied
  where the ink is: `FnWires::draws` in `FnKin::mark` / `FnKin::boundary` for a
  selection anchor, and `WireView::{def_dirty, user_dirty}` for the resting
  plate. `drawn_under` puts the two together. `kin` is a memo over `(selection,
  wires)`.
- `FnModel::reveal(spot)` is the fold path a focus opens, outermost first; a
  room's own border is on the paper whatever its fold says, so nothing ever asks
  a room to open itself. The chart runs it in an effect keyed on the *selection*
  alone.
- The wires are drawn by **three world layers** over one `Vec<WireView>`:
  `.fn-wires` (`z-index: 0`, under the flow's own viewport) takes everything at
  rest, `.fn-wires-lit` (`z-index: 2`) the wires a selection inked, and
  `.fn-wires-hot` (`z-index: 2`, `will-change: transform`) the settled hover's
  own copies.
- The rooms are two SVG groups on the ground layer (`FrameLayer`, then
  `OwnerLayer`, so a container's border wins the click inside a module's), each
  with a wide transparent hit stroke over its own rectangle.
- `FnState` carries the folds beside the two readings on the app shell, so
  stepping out to another rung and back keeps them, and `FnCamera` keeps one
  viewport — the three cameras the `order` reading needed went with it.

## Not here

- **No fold by anything but a hand.** Zoom is one disclosure, the visibility
  slider is the narrowing, the reviewer's own fold mark is the third — and no
  count, depth or budget ever folds anything.
- **No block grown to fit its box.** The landmark register is a mark of its own
  on a layer of its own; `.fm-nm` keeps the reading ramp at every zoom.
- **No second ground.** The call tree is a reading of the wires now, not a place:
  `runs 41 by the way in` is a sentence, and `/fn/depth/:band` is a lighting.
- **No per-call diff.** The structural diff reads the base edition syntactically,
  so it is exact about declarations (`A`, `M`, a flare frame, a `+` on an added
  parameter) and says nothing about a rewritten body.
- **No second colour.** Flare still means CHANGED and nothing else. A function's
  name takes the quotation palette's fn colour; an owner's name on its container
  takes the kind colour the data chart inks that kind with — type-teal for a
  struct or union, the palette's purple for an enum — and a **trait's name stays
  ink**, because on this chart the purple already means *a function's name* and
  `/data` draws no block for a trait to agree with.
