# The function viewer (`/fn`), built 2026-08-25

The third rung of the ladder: `dependencies · data · functions`. The crates
say what the workspace depends on, the data chart says what it keeps, and this
one says **what it does**.

It is the data chart's dual, and it is built as one. A block there quotes a
struct's fields; a block here quotes a function's signature, because a
function's parameters are its fields, of the other half of the language.
Selecting a datum there lists what holds it; selecting a function here lists
what calls it, what it calls, and every type it touches — and each of those
types is a link down to `/data`, which is the rung that draws types.

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
reading admits. A block is its declaration, quoted:

```
pub fn survey(
  dir: &std::path::Path
) -> Result<CodeIndex, String>
```

The head opens the parameter list, the parameters are the rows, and the line
that closes it carries the return — the way rust writes it. A method's rows
carry its receiver as written (`&self`, `&mut self`), which declares no type
and so takes the whole row. Nothing is reconstructed: every row is the
source's own text, from the same walk that quotes a struct's fields
(`analyze/data.rs` now fills a method's *own* mark as well as the row it
files on the type's contract).

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

**The resting reading is the way in.** For every mark, the chart draws the one
call that put it at its depth: one wire per mark, which is the whole of "what
runs from where". Every other resolved call stays in the set, folded, and inks
back on hover or selection of either end. A contract wire never folds — it is
what makes the tree honest about a `dyn` call. Nothing else earns a place at
rest: drawing all the resolved calls would be exactly the hairball the
Selection's-Ink rule forbids one rung up.

The diff earns no place there either, and was tried: unfolding every wire that
merely touched a changed declaration washed a sixty-declaration change's whole
sheet amber. It was also untrue — the survey reads the base edition
syntactically, so a changed *call* is not a thing it can see. The diff's ink on
this chart is the block's own frame and its letter, and nothing else.

## The section (one of three built, chosen 2026-08-25 by the user)

The sheet is a **section**. Bands of call depth run its full width, captioned
at the left margin the way the dependency chart's rings caption their hops, and
prisms cross every band — one per frame the grouping asks for. A mark sits at
the crossing of its depth and its frame, so both readings come off the paper at
once.

Three seatings were built and read on this workspace before one was kept:

- **strata** (kept) — the most legible of the three fitted or not, and the diff
  reads off it immediately: every added declaration sat in one column.
- **mechanism** (cut) — module frames nested the way rust's modules do, parts
  in depth rows inside them, the depth engraved once per row. The most
  continuous with `/data`, but the frames won the eye: it read *whose code*
  before *what runs from where*, and the packing left about a quarter of the
  sheet empty.
- **strips** (cut) — one row per road: an entry point at the left margin, its
  calls as stations running right, one global column per band. On real data it
  came out a twenty-thousand-unit ribbon — twenty-four roads, the busy ones
  stacking a hundred marks at one depth — and never fit the glass. What it
  wanted survives as a selection on the section.

## Grouping (2026-08-25, user)

Two thirds of what runs is a method, and a method's first fact is whose it is.
So what a prism *is* is a reading of its own, on the cartouche as `group`:

- **module** (the default) — one prism per module, the ground `/data` frames on.
- **type** — a prism per type: a method sits with the type its impl names, and
  with the trait clause it answers; a free declaration stays on the module's
  own prism, because nothing owns it. On this workspace that is ~90 prisms and
  a sparse grid — a reading to switch to, not to open on.
- **file** — a prism per file, which is the grouping a reviewer reading a diff
  already has in hand. A file that already gave its module its name draws no
  prism of its own: `src/load.rs` is `mod load`, and a frame around a frame
  saying the same word says nothing.

The grouping nests inside the module path rather than replacing it, so a prism
is always `module` or `module::Type` or `module::file.rs` and never loses where
rust reads the code.

## Readings and focuses

- **visibility** — the same four-stop slider the data chart slides, reading the
  same keyword each declaration writes. `n off` at its foot; naming a
  declaration in the search widens the reading to the stop that draws it.
- **calls** — `calls · callers · both`, against an anchor: the selection, the
  block under the cursor, or, on the resting plate, the declarations the epoch
  touched. A direction with nothing in focus has no direction to take.
- Every focus is a URL: `/fn`, `/fn/mark/:..path?item=` (with `peek=` for a
  quoted row), `/fn/mod/:..module` (one prism's boundary — the crate, the
  module path, and the group where the reading draws one), and `/fn/depth/:band`
  (one whole band). Back retraces the trail; `←`/`→`, `f`, `/` and Escape
  behave as at every altitude.
- A **band** reads differently from a mark or a boundary, and has to: a stratum
  holds sixty marks, so "touches the selection" would be most of the sheet.
  A band lights its own marks and the **way in** to each of them — one resting
  wire per lit block, which reads as the descent into that depth — and folds
  nothing back. A mark or a boundary does give its folded calls back, because a
  frame is small enough that doing so stays a reading.
- A selection lights its **blast radius** — every caller a rewrite could reach
  — and reads its direct neighbours in the chosen direction a step behind,
  because what a function calls does not change when the function does. The
  sheet says it in words: `a rewrite here reaches 9 functions upstream.`

## What the sheet says

Two relation headings and one descent, in the data sheet's own discipline:
`Called by` (with `answers` for the contract, which is the one caller a call
graph cannot see), `Calls`, then `Data touched` — every workspace type the
signature names (`signature`) or the body uses (`12 refs`), each row linking
to that type's block on `/data`, or quoting its source where `/data` draws no
block for it either. An empty `Called by` is the whole verdict in one
sentence: `nothing in the workspace calls it.`

## Not here

- **No hand-folding.** Zoom is the fold (the far edition swaps to names and
  wires below 0.45 and back above 0.55), the visibility slider is the
  narrowing, and no count ever folds anything.
- **No fit-to-illegible first paint.** A sheet this size fits the glass at
  about a fifth of full size, where even the far edition's names are dust. The
  chart opens at 0.34 instead, centred on the entry band — the top of the
  running order, which is what this altitude is about. `f` still fits the whole
  sheet at whatever zoom that takes.
- **No per-call diff.** The structural diff reads the base edition
  syntactically, so it is exact about declarations (`A`, `M`, a flare frame, a
  `+` on an added parameter) and says nothing about a rewritten body. The
  limits fold states it.
- **No second colour.** Flare still means CHANGED and nothing else; a
  function's name takes the quotation palette's fn colour, which is token
  class inside a quotation and stops at the block's frame.
