# Data chart — design brief and behavior

Confirmed via /impeccable, 2026-08-19, from an approved HTML prototype drawn
against this workspace's real types. The third altitude of the review ladder:
crates → files/items → **types**. It answers the question the other two
cannot: what state exists, what shape it has, and who can reach it.

## Job and audience

The same reviewer, asking the data question: "what is the data model, and
what is the boundary of data sharing?" In rust the answer is statically
readable — ownership is in the types. `Arc<Mutex<T>>` says shared mutable
state in the signature; `&'a T` says view; a `static` says state no type
holds. The chart draws exactly that. Visitor mode: **Operate**.

The diff story this altitude tells (built 2026-08-19, from an approved
prototype): the **structural diff**. The chart is drawn against the base
revision in its own grammar — added marks flare, removed marks stand as
ghosts, appeared and disappeared edges go with them, and change kinds are
written in the tools' own words. The cartouche states it in one line ("the
diff lands in api and views").

## Decisions (user-confirmed)

- **Types are the first-class citizens.** Structs and enums are the marks;
  files, functions, and traits are secondary. A mark's home is written as a
  locator (`codemap/model.rs:278`), not as geometry.
- **Ground = modules, not directories.** One frame per workspace crate; inside
  it, one frame per top-level module (`mod analyze`, `mod api`, `mod views`),
  labeled with rust's own words. Crate-root items sit in the crate's own
  frame. One level of module frames only — deeper module paths stay in the
  locator.
- **Within a frame, the ownership forest** (user decision 2026-08-19, from a
  two-structure comp against this workspace's real graph). Marks seat as
  trees: every type under its one heaviest same-frame `Owns` holder, ownership
  depth as layers, so an owns edge is usually a short line between neighbors.
  Statics, unheld types, high-fan-in types, and types owned only from other
  modules are the frame's roots. A type never seats outside its own module —
  the frame stays belongs-to — so cross-frame ownership stays drawn ink, and
  module coupling at data precision is visible instead of arranged away.
  The other dealt structure, physical containment (blocks inside blocks), was
  declined at rest; if it returns it returns as a focus gesture.
- **Edges = holding, plus a reference reading.** Two families:
  1. **Holds** (structure, always drawn): `from` has a field whose type walk
     reaches `to`. Kind and wrapper are written, never invented (see below).
  2. **References** (a reading, like the code map's): the existing item-level
     reference edges, lifted to types, drawn per the cartouche's
     `references` toggle — `uses` / `used by` / `both`, anchored per type
     (two heaviest at rest), folded ties ink in on hover.
- **The arrowhead rests on the holder / the user** — the way a shape change
  travels, consistent with both other altitudes.
- **Statics are roots.** A `static` is drawn (2.5px ink left edge, the gate's
  own mark) regardless of visibility: it is state no type holds, and the
  session's whole story is the URL plus these. Its declared type is quoted
  as written.
- **Privacy folds types, not statics.** Private structs/enums never draw as
  marks; each frame counts them (`+ 5 private types`) and any holds edge
  touching one lands on that counted fold row, the way ties land on gates.
- **Clicking a type selects it** (revised 2026-08-19, user-confirmed after
  review; it replaced the earlier click-to-plate decision). Selection is a URL
  (`/data/type/:..path?item=`) and a reading: the selected block wears the
  app's focus ring and opens to every field and variant it quoted a count for
  (added 2026-08-19), everything a shape change to it could reach — its
  transitive holders, walked holder-ward over the holds edges — keeps full
  ink with its wires (folded ones ink back in), what it directly holds keeps
  ink one hop down, ties touching the selection keep their own, and every
  other mark and wire recedes to a lighter pressure. Nothing moves and the
  camera holds still. A selection sheet (right column) names the selection,
  lists who holds it and what it holds — each row re-centers the selection —
  states the blast radius in words (`a shape change here reaches 9 more types
  upstream.`), and carries the one step further: `open its definition →` to
  the code plate. Escape, bare paper, or clicking the selected block again
  deselects; the definition plate itself is never duplicated.

## The walk (how holds edges are computed)

Per struct/enum (and per static), every field's (or variant payload's)
semantic type is walked:

- References strip to **Borrows**, via `&` / `&mut`.
- `Arc`, `Rc` → **Shares**, via the wrapper's own word. `Weak` → Shares,
  via `Weak`. Dioxus `Signal`, `GlobalSignal`, `ReadSignal`, `Memo`,
  `Resource` → Shares (a signal is Copy-shared runtime storage).
- `dyn Trait` → **Dyn**, via `dyn`, targeting the trait's mark.
- Everything else with generic arguments — `Vec`, `Option`, `Box`, `HashMap`,
  `Result`, `OnceCell`, `Mutex`, tuples, arrays, and any external type —
  is transparent: the walk recurses into the arguments. Interior mutability
  without `Arc`/`Rc` is still Owns: sharing needs a shared handle.
- The edge's `via` is the strongest wrapper met on the path (Shares > Dyn >
  Borrows > plain); its kind follows the same order.
- A workspace struct/enum/trait reached by the walk becomes an edge target;
  external types are wrappers and atoms, never marks. A field whose walk
  reaches no workspace type draws no edge — its row is still quoted.
- Type aliases resolve through HIR (a `TrailStep` field walks as
  `Option<String>`). Generic parameters on the holder are holes: the walk
  reads nothing through them. Nothing is guessed; the legend carries the
  wrapper table and the rule.

Every field on a mark is quoted from the source in declaration order: the
field name and its declared type exactly as written (`details:
Vec<FileDetail>`), never a reconstruction (revised 2026-08-19; plain fields
were counted before, and the reviewer asked for the fields themselves).

## The structural diff (built 2026-08-19)

One rule at every altitude, landed here first: the diff is drawn in the
map's own grammar, at the map's own granularity.

- **Base edition, read syntactically** (`src/analyze/basediff.rs`). For each
  changed file, `file_at_base` fetches the text as the base wrote it and
  `ra_ap_syntax` parses it — no name resolution, no second rust-analyzer
  run. Declarations match by kind and name (inline-module paths included);
  a removed relation's target is matched by name against the types the
  chart knows, ambiguous names are counted, and the legend states the
  method. A full semantic survey of the base would slot in behind the same
  wire model; this is the cheap edition by user decision ("syntactic first,
  keep the full survey in mind").
- **Letters are git's own**: `A` added since the base, `M` declaration
  changed, `D` removed. A file-level change no longer marks a type: the
  letter is the declaration's own delta, so an untouched type in a touched
  file stays quiet. A diff-touched block wears the flare on its own frame.
- **Ghosts.** A removed type or static is drawn from the base edition —
  dashed frame, rows quoted as the base wrote them, locator
  `src/views/shell.rs:113 (base)` — and seats in the frame its path names.
  Its sheet says "removed since the base"; its definition link is replaced
  by "its definition left the working copy."
- **Rows.** An added field or variant wears the diff's `+` in flare; a
  dropped one is quoted from the base, struck, and seated where it stood
  (a same-named field whose type changed is both, the diff's own idiom for
  a changed line). The resting row window stretches to the last diff row,
  so diff rows never hide behind a fold.
- **Edges.** A holding edge the base could not have drawn (either end
  added, or every drawing field added) takes flare with `added` on the
  line, after the wrapper's word. An edge only the base had is re-drawn
  flare-dashed with `removed` — from the base's dropped rows and the
  ghosts' own, by name. A pair that still holds through another field
  draws no removed edge. Diff edges never fold and never lose their flare
  to hover or selection ink.
- **The recede.** While the diff has anything to say, untouched marks and
  fold rows rest at a lighter pressure (hover restores); the diff-touched
  never fold, whatever the budget. A clean diff draws none of this.
- **Words.** The cartouche: `7 added · 13 removed · 13 changed` (no noun —
  statics count too) and "the diff lands in api and views". The sheet
  quotes the selection's own change rows (`+ delta: Delta`,
  `− changed: bool` struck), marks hold rows with their far end's letter
  and the edge's event word, and a ghost's blast line reads "the removal
  reaches n more types upstream."

## Wire model

Extends `CodeGraph` (src/api.rs); the survey already carries every type as an
`ItemMark`.

- `HoldKind` — `Owns` / `Shares` / `Borrows` / `Dyn`.
- `HoldEdge { from: u32, to: u32, kind: HoldKind, via: String,
  fields: Vec<(String, String)> }` — `from` and `to` are `ItemMark` ids;
  `fields` holds each holding field as written: (name, declared type).
  A static's edge uses the static's mark as `from`.
- `ItemMark` gains: `field_rows: Vec<(String, String)>` (structs and unions:
  every field as written, in declaration order — name, declared type),
  `variants: Vec<String>` (enums: variants as written, payloads and
  discriminants included), `ty: String` (statics: the declared type as
  written; empty otherwise).
- The structural diff (2026-08-19): `ItemMark` gains `delta: Delta`
  (`Same` / `Added` / `Changed`), `fields_added` / `variants_added`
  (indexes into the quoted rows) and `fields_removed` / `variants_removed`
  (base rows with the working-copy index they seat before). `HoldEdge`
  gains `event: Option<HoldEvent>` (`Added` / `Removed`); removed edges are
  synthesized from the base rows. `CodeGraph` gains
  `ghosts: Vec<GhostMark>` — removed types and statics quoted from the
  base, their ids continuing after `items` so edges can land on them.
- `CodeGraph` gains `holds: Vec<HoldEdge>`.
- Reference ties at type precision are computed on the client from the
  existing `item_edges`: each endpoint climbs `parent` to its outermost mark;
  ties where both ends land on a drawn struct/enum are kept, counts summed,
  self-ties dropped. References from free functions and trait items are not
  on this chart; the legend says so.

## The map

- **Marks**: one bordered block per visible pub struct/enum (hairline frame,
  paper ground, square corners). Header: the keyword and visibility in
  keyword-blue, the name at 700 — type-teal for a struct or union, purple for
  an enum (decided 2026-08-19, user-confirmed), so a product type and a sum
  type read apart at chart zoom, where the keyword is the small run and the
  name is the one that carries. An amber `A`, `M`, or `D` where the
  structural diff has something to say about the declaration itself.
  Body: every field quoted as written and every variant quoted whole
  (payloads and discriminants included), colored by token class the way a
  definition plate colors its source, with the one run naming a held
  workspace type bold. A plain type name is from outside the workspace: it has
  no mark, so no line is drawn to it — which is the whole reason the bold run
  exists. Foot: the counted folds (`+ 4 more fields`, `+ 2 more variants`,
  `held by 6 types`), then the locator in 8.5px soft ink. The block is the
  link to its own selection; its definition plate is one step further, on the
  selection sheet.
- **Eight rows per list at rest, all of them when selected** (decided
  2026-08-19, user-confirmed). A resting block quotes eight fields and eight
  variants and counts the rest; selecting it draws every one, and the plate
  grows down past the box the layout gave it, over neighbours that are
  receding anyway. The box the edges land on stays the resting one, so the
  chart still does not move, and the resting width already fits the widest
  folded row so no line reflows on opening. `held by n types` is the chart's
  own fold, not the block's, and survives opening.
- **Statics**: same block with a 2.5px ink left edge; `static NAME` header,
  the declared type quoted beneath with the workspace type it reaches bold,
  locator. `GlobalSignal<Trail>` bolds `Trail` and draws the line into the
  static; `GlobalSignal<Option<Viewport>>` bolds nothing, because `Viewport`
  comes from a dependency and a dependency has no mark to point at.
- **Frames**: crate frames and top-level module frames, 2.6% ink tint,
  label band on the border (`mod views` mono 500; the crate name where
  more than one crate exists). Each frame's counted fold row collects its
  private types.
- **Seating order inside a frame**: statics first (the frame's root
  register), then trees by subtree size, then the high-fan-in leaves
  (`held by n types`, never seated under a parent and never parents
  themselves), then the counted fold rows. The primary holder is the
  heaviest same-frame `Owns` holder by field count, ties broken by survey
  order; a cycle keeps the earlier seat and draws the closing edge; a type
  owned solely by private code seats under the frame's private fold row.
- **Holds edges**: quadratic hairlines bowed toward open paper, arrowhead on
  the holder. Owns solid; Shares dashed with the wrapper word on the line
  (paper halo); Borrows dotted with `&`; Dyn dashed with `dyn`.
- **Reference ties**: the code map's tie grammar — width by count, half
  opacity at rest, heaviest dozen labeled, anchored two-per-type by the
  reading, folded ties ink in on hover of either end.
- **Budget**: the first paint stays under ~200 blocks. Past the budget each
  frame folds its quietest types (interest = holds degree + type-level ref
  fan-in + 2 if changed) into a counted row: `+ 23 more types`. A fold
  counts; an open frame states no tally.
- Layout is a pure function of (marks, edges, measured sizes) —
  deterministic, blocks measured before placement. Each frame lays its
  forest as tidy trees: children in a row under their parent, the parent
  centered over them, layers by ownership depth; trees then shelve toward a
  landscape frame, and frames pack toward a landscape sheet as before.

## Chrome

- Cartouche (top-left): workspace name, the altitude ladder
  `dependencies · code · data` (all three cartouches gain the third rung),
  facts (`54 structs · 18 enums · 11 roots`), the diff line, the amber
  structural-diff counts (`7 added · 13 removed · 13 changed`), and the
  insight line naming the top-level modules the diff landed in.
- The `references` toggle rides the cartouche (uses / used by / both).
- Legend beneath the cartouche: the three edge families with drawn samples,
  the static mark, the two name colors (`Wire` · `HoldKind`), `held by n`
  folds, the structural diff's key (`A`/`M`/`D`, `+`/`−` rows, added and
  removed wire samples, the recede), the row fold and what selecting the
  block gives back, then the
  honesty notes (wrapper table, that a plain type name is external and draws
  no line, what the walk does not chart, unresolved counts).
- Routes: `/data` (the whole chart) and `/data/type/:..path?item=` (one
  selection). Escape deselects to `/data`; `f` refits (reserving the sheet's
  column while one is open); `←`/`→` retrace history as everywhere.
- The camera survives the round trip (2026-08-19): pan and zoom are session
  state, so leaving for a definition plate (or another altitude) and coming
  back restores the camera exactly as the reviewer left it. The chart frames
  itself only on a fresh session.
- Loading: the shared survey's constellation moment (same index as `/code`).
  Failure: plain-words plate; the other altitudes keep working.

## States

- A clean diff draws no amber anywhere.
- An enum held by more than three drawn types folds its incoming holds edges
  to a counted line on its own mark (`held by 6 types`); hovering either end
  inks them in. Only a drawn mark folds its fan-in: a counted fold row has no
  foot to state a second count on, so every edge landing on one stays drawn.
- A `dyn` hold lands on a trait, and v1 draws no trait marks, so those edges
  are not on the chart. The legend counts them where there are any.

## Open decisions

- Lifecycle on the selection sheet (born / mutated / read / consumed from fn
  signatures) — the sheet exists (held by / holds); lifecycle does not.
- A full semantic survey of the base revision (exact base edges) behind the
  same wire model — the syntactic edition is the committed first step, not
  the last word.
- The code altitude's pass of the same grammar: `A`/`D`/`M`/`R` on file
  blocks, added/removed/signature-changed/body-changed on item rows, and
  the untouched-callers split on the definition plate.
- Renames and moves as drawn traces (a matched removed+added pair is
  already read as a move, silently; it draws nothing yet).
- Search on `/data` (the code search exists; jumping to data marks does not).
- Whether traits deserve resting marks or appear only as `dyn` targets.
