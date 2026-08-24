# The surface chart — design brief and behavior

The third altitude of the review ladder: crates → files/items → **the
contracts the code publishes**. Drawn from an approved HTML prototype
(2026-08-19) as the *data* chart, and redesigned from first principles as the
*surface* chart (2026-08-20, user-approved) when the data reading proved to be
the wrong thesis.

## Job and audience

The same reviewer, one rung further down, asking: **"what does this code
promise, and what would have to change with it?"** In rust that answer is
statically readable, because the promise is written in the types:
`Arc<Mutex<T>>` says shared state in the signature, `&'a T` says view, a
`static` says state no type holds. The chart draws exactly that and quotes the
rest. Visitor mode: **Operate**.

The diff story it tells is the **surface diff**: the chart is drawn against
the base revision in its own grammar — added contracts flare, removed ones
stand as ghosts, appeared and disappeared edges go with them, and change kinds
are written in the tools' own words.

## The thesis, in five rules

1. **Every item that crosses a door is a contract, and every contract is a
   mark.** Structs, enums, unions, traits, free functions, statics, free
   consts, free type aliases. Methods and associated items are *not* marks —
   they are rows of the block that declares them, which is the volume
   decision that keeps a chart of contracts from becoming a chart of names.
2. **A block is a header plus rows, and every row is a clause.** A struct's
   fields, an enum's variants, a trait's declared methods and associated
   items, a function's parameters and return, a static's or const's declared
   type, an alias's target — and, under a rule of its own, the methods a type
   publishes. Everything is quoted as written; nothing is reconstructed.
3. **Every row that names a workspace mark files an edge under that row's
   name.** One machinery for fields, parameters, method rows and trait
   clauses; the bold run of a row is the mark its edge lands on — and the way
   to it: the run is a link that selects that mark (2026-08-24, user).
4. **One direction rule: the arrowhead rests on the dependent.** A change at
   the tail travels to the head. True of holds, signatures, method rows,
   implements, and body references alike.
5. **Exactly two inks.**
   - **Solid = interface coupling.** The dependent's own published surface
     names the tail. The wrapper the walk met writes its own word on the line
     (`Arc`, `&`, `dyn`, `aliases`, `implements`); no word at all is plain
     ownership. Dash used to encode which wrapper it was — dash is spoken for
     now.
   - **Dashed = implementation coupling.** The dependent's *body* leans on the
     tail: calls and references, one family with counts, lighter, because a
     rewrite can take it back without anyone else noticing.

`Doors` is the view's definition: it draws exactly the surface that crosses
the chosen door, rows included. Default `pub(crate)`.

## Decisions (user-confirmed)

- **Ground = modules, not directories.** One frame per workspace crate;
  inside it, one frame per module, nested exactly as rust's modules nest —
  `mod views` holds `mod codemap` and `mod surface`, each labeled in rust's own
  words, the last segment alone on the border because the paper's own nesting
  says the rest (revised 2026-08-20, user decision: one flat level read as a
  lie about the code — `views::surface` and `views::codemap` are not one
  module). A frame's path is the directory chain under the crate's source root:
  `src/views/surface/map.rs` and `src/views/surface/mod.rs` both frame in
  `views::surface`, `src/views/atlas.rs` frames in `views` beside them, a file
  directly under the root is the module it declares (`src/api.rs` → `mod api`),
  and crate-root items (`main.rs`, `lib.rs`) sit in the crate's own frame. A
  module between two others is drawn even when its own files declare nothing —
  the module below it has to sit somewhere. Prose that names a frame away from
  the paper — the cartouche's insight line, a sheet's fold row — spells the
  whole path (`views::surface`), because three modules answer to
  `mod surface`.
- **Within a frame, the ownership forest.** Types seat under their one
  heaviest same-frame `Owns` holder, ownership depth as layers, so an owns
  edge is usually a short line between neighbours. A type never seats outside
  its own module, so cross-frame ownership stays drawn ink and module
  coupling stays visible instead of being arranged away.
- **Contracts are never seated and never seat.** A function, trait, const or
  alias is a leaf at both ends: nothing holds one, and naming a type is not
  containment — a method that hands a `Nut` back does not keep one. They fall
  in *beside* what they are most about (a trait beside the same-frame type
  that implements it most, everything else beside its heaviest signature
  target), and the ones about nothing in their own frame read after the
  shapes, in a band.
- **Statics are roots.** A `static` is drawn at every door (2.5px ink left
  edge, the gate's own mark): it is state no type holds. A const and an alias
  are the same one line *without* the ink edge — ordinary contracts that fold
  by the door, and never roots.
- **A contract's plate is a different plate.** A function's block stands on a
  wash of ink (11% over paper) under a 2.5px ink rule across its top — the
  gate mark the static wears down its left, turned to face the paper. State is
  raised paper; contracts are recessed. It is the one pair of moves left in
  the vocabulary: dashed is a ghost, flare is diff ink, a left edge is a
  static, line weight is the wires'.
- **Clicking a mark selects it.** Selection is a URL
  (`/surface/mark/:..path?item=`) and a reading: everything a change to it
  could reach keeps full ink with its wires, and the rest recedes. The block
  itself was already quoted whole — selection is the reading, never the
  quotation. Nothing moves, and the camera holds still for any selection the
  glass can already show; one it cannot — off the viewport, or below reading
  zoom — it glides to, which is the data chart's rule (2026-08-21) reaching
  this altitude on 2026-08-24, when a row's held type became a link to a block
  that can stand anywhere on the paper.
- **Selection is also how a neighbourhood is read.** Every uses edge touching
  the selected mark inks in and stays inked — folded ones included — and the
  block at the far end of each reads a step behind the blast radius rather
  than receding with the strangers. The two families still never merge: a
  body is not a shape change, so a uses neighbour is drawn beside the radius
  and never counted in it. Hover is the passing reading; selection is the one
  that survives the cursor leaving, which is what following an edge to the
  other end requires.
- **A module boundary is selectable too.** Clicking a frame's border — or the
  label chipped onto it — selects the module (`/surface/mod/:..module`, the
  crate then the module path). Every contract inside the boundary keeps full
  ink, whatever module inside it was written in; everything one hop across the
  line reads a step behind, both families at once, because what crosses a
  module boundary is what a reader came to the boundary to read; every other
  module recedes, its frame with it. The frames the chosen one sits inside
  never recede — they are the paper it stands on. Two hops would be the whole
  chart again. There is no sheet: a module is a place on the paper, and the
  paper is already saying it.
- **A module can be folded to one row.** The mark at the border's other end —
  `−` while the module is drawn, `+` once it is folded — takes the whole
  module off the paper and leaves one counted row (`+ 21 items`) inside
  the boundary that stood there. Everything inside folds with it, however deep
  the nesting: the modules nested in it earn no frame of their own, their
  private items join the same count, and every
  holds edge that touched a contract inside lands on the row, exactly as the
  visibility and budget folds already land. Folding the module above a folded
  one swallows it; unfolding the outer one hands the inner fold back. A fold is
  a **re-layout**, not a reading — the chart is drawn again around what is left
  — which is why selecting and folding are two marks on the border and never
  one gesture. The fold is view state, kept per session; the selection is the
  URL.

## What the survey walks

Per contract, the declared types are walked semantically (rust-analyzer's
resolution, the same one the code altitude's references use):

- References strip to **Borrows**, via `&` / `&mut`.
- `Arc`, `Rc`, `Weak` → **Shares**; dioxus `Signal`, `GlobalSignal`,
  `ReadSignal`, `Memo`, `Resource` → **Shares** (a signal is Copy-shared
  runtime storage).
- `dyn Trait` → **Dyn**, landing on the trait's own mark.
- Everything else with generic arguments — `Vec`, `Option`, `Box`, `HashMap`,
  `Result`, `OnceCell`, `Mutex`, tuples, arrays, any external type — is
  transparent: the walk recurses. Interior mutability without a shared handle
  is still Owns.
- The edge's `via` is the strongest wrapper met on the path (Shares > Dyn >
  Borrows > plain).
- Type aliases resolve through HIR. **Type parameters, trait bounds and
  `impl Trait` are holes**: the row quotes them, the walk reads nothing
  through them.

What each kind contributes:

| Contract | Rows | The `ty` line | Edges |
| --- | --- | --- | --- |
| struct, union | every field, whatever its visibility | — | per field |
| enum | every variant as written | — | per variant payload |
| trait | every declared method signature, associated type and const | — | per row |
| type block's band | the methods that clear the door | — | per method row, `from_method` |
| free fn | its parameters | `-> R` | per parameter, return under the fn's name |
| static, const | — | the declared type | under its own name |
| type alias | — | the target | under its own name, word `aliases` |

A method's signature files under the **method's** name with the *type* as the
dependent, and those edges are kept apart from field edges everywhere: they
seat nothing, they count in `named by n signatures` rather than `held by n
types`, and the sheet lists them under `Its API names` / `In the API of`. A
pair reached both ways draws both lines.

`impl Trait for Type` is resolved through the impl's own trait and self type,
never off the header string, and drawn solid from the trait to the type. An
impl of a foreign trait, or for a foreign type, has no second end and stays a
string on the definition plate.

**A quotation never waits on inference.** A function an attribute macro
rewrites (`#[server]`, `#[component]`) resolves to the expansion or to nothing
at all; its rows are still quoted from the reader's own file, and where the
resolved signature does not line up with the written one, the written types
are resolved in the file's scope instead. (Found by rendering the finished
chart: before this, every `#[server]` fn read as `M` with its whole parameter
list struck.)

## The uses family

Every reference the survey resolved at item precision — across files and
inside one — with each end climbing its containment chain to the mark that
draws it, so a method's call is its type's and a default trait method's is its
trait's. A pair is kept when both ends land on a drawn mark; counts sum. Where
the survey resolved a call to a *method*, the tie remembers which row, so the
sheet can say `4 calls · build` rather than only naming the block.

Same-file references reach the wire as `CodeGraph::local_refs`
(`MarkRef { from, to, count }`), beside the cross-file `item_edges`: which
file a call was written in says nothing about whether one contract leans on
another. On this workspace that is 670 more pairs beside 1035 cross-file ones.

What cannot land is counted, never cut: `unseen_users` / `unseen_uses` on the
mark it did reach — references from marks the door folded, or
from items with no mark of their own. The sheet says the count, so a quiet
contract reads as quiet rather than dead.

## The surface diff

- **Base edition, read syntactically** (`src/analyze/basediff.rs`). For each
  changed file, `file_at_base` fetches the text as the base wrote it and
  `ra_ap_syntax` parses it — no name resolution, no second rust-analyzer run.
  Declarations match by kind and name (inline-module paths included); a
  removed relation's target is matched by name; ambiguous names are counted,
  never guessed. A full semantic survey of the base would slot in behind the
  same wire model; this is the cheap edition by user decision.
- **Letters are git's own**: `A` added, `M` declaration changed, `D` removed.
  A file-level change never marks a contract — the letter is the
  declaration's own delta.
- **What `M` reads.** A shape's whole declaration text; a *function's*
  signature only, and a *trait's* band only, because those blocks quote the
  promise and a rewritten body is the code altitude's news.
- **Ghosts.** A removed contract is drawn from the base edition — dashed
  frame, rows quoted as the base wrote them, locator `…:113 (base)` — and
  seats in the frame its path names. Ghosts are drawn at every door: a
  removed thing has no visibility left to fold on. The one fold that does take
  a ghost is a module the reviewer folded by hand — that fold is a boundary,
  and it holds for everything inside it.
- **Rows.** An added row wears the diff's `+` in flare; a dropped one is
  quoted from the base, struck, and seated where it stood — fields,
  variants, and method bands alike, the band weaving through the door's own
  folding.
- **Edges.** An edge the base could not have drawn takes flare with `added`
  on the line; one only the base had is re-drawn flare-dashed with `removed`.
  An `impl Trait for Type` added or dropped this epoch is diff ink — a type
  taking on a contract is what a reviewer came for.
- **The recede.** While the diff has anything to say, untouched marks rest at
  a lighter pressure (hover restores); diff-touched marks never fold.

## Wire model

Extends `CodeGraph` (src/api.rs):

- `ItemMark` carries the quotation: `field_rows: Vec<DeclRow>` — fields, or a
  function's parameters, as `{ name, ty, vis }` (2026-08-24: each row's *own*
  visibility, because a `pub(crate)` struct can publish some of its state and
  keep the rest, and both charts draw the keyword in front of the name; a
  parameter declares nothing and is always private) — `variants`, `ty` (a
  static's or const's declared type, an alias's target, a function's return),
  and `method_rows: Vec<MethodRow>` —
  `{ name, sig, vis, via_trait, mark }`, every method of the type whatever its
  visibility, because which rows are drawn is a door and a door is the
  client's. `via_trait` is what lets a `pub`-less trait-impl method read as
  published; `mark` is the method's own id, so a call resolved to it can be
  filed under its row.
- `HoldEdge { from, to, kind, via, fields, from_method, event }` — the solid
  family, aggregated per (from, to, kind, via, from_method, event).
  `HoldKind` is `Owns` / `Shares` / `Borrows` / `Dyn` / `Implements`.
- `ImplEdge { trait_mark, ty, header, event }` — workspace trait, workspace
  type, resolved semantically.
- `MarkRef { from, to, count }` in `local_refs` — same-file references at mark
  precision.
- The diff rides along: `Delta` per item, `fields_added` / `fields_removed`,
  `variants_*`, `methods_*`, `HoldEvent` on edges, and `GhostMark` (with its
  own `method_rows`) for what the base had and the working copy dropped.

## The chart

- **Marks**: bordered blocks, hairline frame, square corners. Header: the
  keyword and visibility in keyword-blue, the name at 700 — type-teal for a
  product type, purple for a sum type and for a function. An amber `A`, `M`
  or `D` where the diff has something to say about the declaration.
- **Rows**: quoted as written, colored by token class the way a definition
  plate colors its source, with the one run naming the mark the row reaches
  in bold. A plain type name is from outside the workspace: no mark, no line.
  **The bold run is a link** (2026-08-24, user): click the type's name inside
  the row and the chart selects that type's block — the same focus the block's
  own click is, gliding only where the block is not already legible. It
  underlines while the pointer is on it and carries nothing at rest. A run
  with no block to reach — folded behind a door or a hand-folded module, or
  the block's own name — is bold text and nothing more.
  Each field row carries the visibility it declares for itself, in front of
  its name (2026-08-24): a `pub(crate)` struct can publish some of its state
  and keep the rest, and only the row can say which.
- **Brackets** (2026-08-24, user decision). A block is bracketed the way rust
  brackets the declaration it quotes. `{` on the head and `}` on a line of its
  own for a shape with rows; `(` against the name and `)` carrying the return
  for a signature with parameters — `()` on the head and the return below when
  it has none; `:` (an alias, `=`) on the head of a static, const or alias,
  whose one line below is its declared type; nothing at all where there are no
  rows, since inventing `{}` would guess at a unit struct. A shape's `}` closes
  before its method band — that band is an `impl`, and an `impl` stands outside
  the shape — while a trait's closes after it, the band being the body, and a
  trait's band opens on no rule of its own. Punctuation ink; the far edition
  drops it with the rows. Rows are indented 12px inside the brackets and the
  closing brace is not, so a block's end is findable; the diff's `+`/`−`
  marker sits in that gutter, which aligns a woven block's text column across
  marked and untouched rows alike.
- **No row caps** (revised 2026-08-20, user decision). A block draws its whole
  declaration: every field, every variant, every method row of the band, every
  parameter of a signature. A declaration read eight rows deep is a declaration
  half read, and a reader who has to select a block to finish its shape is
  reading the chart twice — so `+ 4 more fields`, `+ 2 more methods` and
  `+ 3 more params` are gone, with the windowing that produced them. A block is
  as tall as what it promises, and the layout is handed that height. Selecting
  a block opens nothing, because nothing was closed.
  `held by n types` · `named by n signatures` is the one counted line left at
  the foot, and it is the chart's own fold — ink it will not draw — not a row
  the block is holding back.
- **Wires**: quadratic hairlines bowed toward open paper, the two families to
  opposite sides. Solid at one pressure with the wrapper's word; dashed and
  lighter with the count, thinned by the references toggle (two per mark at
  rest, folded ones ink in on hover and for as long as either end is
  selected, heaviest dozen labeled).
- **No budget fold** (2026-08-21). Every contract that clears the door is
  drawn, however many there are. The old `MARK_BUDGET` (200) folded each
  frame's quietest into a `+ n more` row: it hid marks by a number nobody set,
  reflowed the whole chart when the threshold moved, invented a row that
  attracted edges, and left contracts a URL could still point at. What folds
  here is what a reader asks for — the visibility door, and a module folded by
  hand.
- Layout is a pure function of (marks, edges, measured sizes): blocks are
  measured before placement, trees are tidied one layer per ownership depth,
  trees shelve toward a landscape frame, frames pack toward a landscape sheet.
  The same survey always draws the same chart.

## Chrome

- Cartouche (top-left): workspace name; `57 structs · 20 enums · 0 traits`;
  `88 fns · 16 consts · 1 alias · 5 roots`; `38 method rows · 413 body
  dependences drawn`; the altitude ladder `dependencies · code · surface`;
  the diff line, the amber counts (`7 added · 13 removed · 13 changed`), and
  the insight line naming the modules the diff landed in.
- Two toggles ride the cartouche: `references` (uses / used by / both) and
  `visibility` (pub / pub(crate) / private) — the second is what the view
  *is*, and it moves the rows as well as the marks.
- Selection sheet (right): the header and locator, the diff's own quoted
  rows, then — kept strictly apart — `Held by` (fields), `In the contract of`
  (free functions), `In the API of` (another type's method row),
  `Implements` / `Implemented by`, `Holds`, `Its API names`, and the dashed
  family both ways round as `Used by` / `Uses` with `3 calls` or `12
  references` and the clause named. Under them the residue line, and the
  blast radius in words (`a shape change here reaches 9 more types and 2 more
  signatures upstream.`). The empty `Held by` is a four-way truth: a static is
  `a root`; a mark only signatures name enters through them; a mark only
  bodies reach says so; and a mark nothing reaches at all says exactly that —
  the verdict a reviewer deletes code on.
- Legend (cut to a key 2026-08-21, distill — about eleven hundred words to
  ~230): a key strip first (interface, implements, uses, each sample beside
  the one word it names; then the static's edge and the signature plate), one
  paragraph of grammar per family, the quotation rule and the counted folds
  (`held by 6 · named by 2`, `+ 5 private items`), the diff's key *only while
  the diff has something to say*, the gestures on a two-column grid — and the
  survey's own limits behind a nested `what the survey cannot read` fold,
  printed from `walk_notes` + `notes` in the survey's words rather than
  paraphrased in six paragraphs of prose. What else came off: the references
  and visibility toggle paragraphs (they restated six button titles), and the
  clause announcing that no row waits behind a count.
- Routes: `/surface`, `/surface/mark/:..path?item=`, and
  `/surface/mod/:..module`. Escape deselects; `f` refits; `←`/`→` retrace
  history. The camera survives the round trip — a legible selection never
  moves it, and neither does a fold.

## Honesty notes the chart states out loud

- A reference whose other end has no block — folded by the door or the
  door, or an item with no mark (a const inside an impl, a macro) — is
  counted on the mark it reaches, not drawn.
- A macro declares surface this chart cannot read: what `macro_rules!` or a
  derive writes is not surveyed as rows. A type's derives stand on its
  definition plate, one altitude up.
- Type parameters, trait bounds and `impl Trait` are holes.
- Names the survey could not resolve are counted and named as unresolved.
- A folded module's contracts are off the chart, and the cartouche's counts are
  what the chart draws: they fall with the fold, and the folded boundary's own
  row is where the missing contracts are counted. Its uses edges are not drawn
  — the dashed family runs between drawn marks, so a body leaning on a folded
  module is counted on the mark at the other end, the way any unlanded
  reference is. Its holds edges are drawn, and land on the row.

## Open decisions

- A method band is diffed against the impls in the type's own file only.
  Moving an impl block between files reads as the whole band added and
  removed. A trait's band has no such limit — its clauses are written inside
  its own declaration.
- A full semantic survey of the base revision (exact base edges) behind the
  same wire model — the syntactic edition is the committed first step, not
  the last word.
- A function's or const's declared type changing draws no `removed` edge,
  only the `M`.
- `ItemMark::fan_in` stays cross-file only, and the sheet's rows rank
  on it; same-file references could sharpen that ranking now that they are
  surveyed.
- Lifecycle on the selection sheet (born / mutated / read / consumed) — the
  signatures are on the chart, but the sheet reads them as coupling, not as a
  lifecycle.
- The code altitude's pass of the same diff grammar: `A`/`D`/`M`/`R` on file
  blocks, and the untouched-callers split on the definition plate.
- Renames and moves as drawn traces (a matched removed+added pair is already
  read as a move, silently; it draws nothing yet).
- Search on `/surface` (the code search exists; jumping to a contract does
  not).
- Ghosts are drawn at every door, private ones included — a removed private
  helper still takes a block.
