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
   clauses; the bold run of a row is the mark its edge lands on.
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
  inside it, one frame per top-level module (`mod analyze`, `mod api`,
  `mod views`), labeled in rust's own words. Crate-root items sit in the
  crate's own frame. One level of module frames only — a deeper module path
  stays in the mark's locator (`codemap/model.rs:278`).
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
  by the door and by the budget, and never roots.
- **A contract's plate is a different plate.** A function's block stands on a
  wash of ink (11% over paper) under a 2.5px ink rule across its top — the
  gate mark the static wears down its left, turned to face the paper. State is
  raised paper; contracts are recessed. It is the one pair of moves left in
  the vocabulary: dashed is a ghost, flare is diff ink, a left edge is a
  static, line weight is the wires'.
- **Clicking a mark selects it.** Selection is a URL
  (`/surface/mark/:..path?item=`) and a reading: the block opens to every row
  it quoted a count for, everything a change to it could reach keeps full ink
  with its wires, and the rest recedes. Nothing moves; the camera holds still.

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
mark it did reach — references from marks the door or the budget folded, or
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
  removed thing has no visibility left to fold on.
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

- `ItemMark` carries the quotation: `field_rows` (fields, or a function's
  parameters), `variants`, `ty` (a static's or const's declared type, an
  alias's target, a function's return), and `method_rows: Vec<MethodRow>` —
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
- **Caps**: eight fields and eight variants at rest (`FIELD_CAP`), five
  method rows (`METHOD_CAP` — a signature is a wide row and the shape is what
  the block is for), the rest counted on the foot (`+ 4 more fields`,
  `+ 2 more methods`); selecting the block draws them all. The resting width
  already fits the widest folded row, so nothing reflows on opening.
  `held by n types` · `named by n signatures` is the chart's own fold and
  survives opening.
- **Wires**: quadratic hairlines bowed toward open paper, the two families to
  opposite sides. Solid at one pressure with the wrapper's word; dashed and
  lighter with the count, thinned by the references toggle (two per mark at
  rest, folded ones ink in on hover, heaviest dozen labeled).
- **Budget**: the first paint aims to stay under `MARK_BUDGET` (200) drawn
  contracts. Past it, each frame folds its quietest into `+ n more`; statics
  and diff-touched marks never fold. Ghosts are drawn on top of the budget.
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
- Legend: the two inks with drawn samples, the contract plate and the trait
  block, the static's edge and the one-liners, the two name colors, the row
  and fan-in folds, the diff's key, then the honesty notes (the wrapper
  table, holes, what stays off the chart, macros, unresolved names).
- Routes: `/surface` and `/surface/mark/:..path?item=`. Escape deselects; `f`
  refits; `←`/`→` retrace history. The camera survives the round trip.

## Honesty notes the chart states out loud

- A reference whose other end has no block — folded by the door or the
  budget, or an item with no mark (a const inside an impl, a macro) — is
  counted on the mark it reaches, not drawn.
- A macro declares surface this chart cannot read: what `macro_rules!` or a
  derive writes is not surveyed as rows. A type's derives stand on its
  definition plate, one altitude up.
- Type parameters, trait bounds and `impl Trait` are holes.
- Names the survey could not resolve are counted and named as unresolved.

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
- `ItemMark::fan_in` stays cross-file only, and the budget's `interest` ranks
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
