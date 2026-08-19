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

The diff story this altitude tells: which types a change added or touched,
and whether the change moved state across a sharing boundary. The cartouche
states it in one line ("every changed type sits in views::codemap").

## Decisions (user-confirmed)

- **Types are the first-class citizens.** Structs and enums are the marks;
  files, functions, and traits are secondary. A mark's home is written as a
  locator (`codemap/model.rs:278`), not as geometry.
- **Ground = modules, not directories.** One frame per workspace crate; inside
  it, one frame per top-level module (`mod analyze`, `mod api`, `mod views`),
  labeled with rust's own words. Crate-root items sit in the crate's own
  frame. One level of module frames only — deeper module paths stay in the
  locator.
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
- **Clicking a type goes to its definition plate** (`/code/file/..?item=`) —
  the plate already quotes the source and lists references; the data chart
  adds no second plate in v1.

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
  reaches no workspace type counts as one **plain field**.
- Type aliases resolve through HIR (a `TrailStep` field walks as
  `Option<String>` — plain). Generic parameters on the holder are holes and
  count as plain. Nothing is guessed; the legend carries the wrapper table
  and the rule.

Field rows on a mark quote the source: the field name and its declared type
exactly as written (`details: Vec<FileDetail>`), never a reconstruction.

## Wire model

Extends `CodeGraph` (src/api.rs); the survey already carries every type as an
`ItemMark`.

- `HoldKind` — `Owns` / `Shares` / `Borrows` / `Dyn`.
- `HoldEdge { from: u32, to: u32, kind: HoldKind, via: String,
  fields: Vec<(String, String)> }` — `from` and `to` are `ItemMark` ids;
  `fields` holds each holding field as written: (name, declared type).
  A static's edge uses the static's mark as `from`.
- `ItemMark` gains: `plain_fields: u32` (structs: fields whose walk found no
  workspace type), `variants: Vec<String>` (enums: variant names as written),
  `ty: String` (statics: the declared type as written; empty otherwise).
- `CodeGraph` gains `holds: Vec<HoldEdge>`.
- Reference ties at type precision are computed on the client from the
  existing `item_edges`: each endpoint climbs `parent` to its outermost mark;
  ties where both ends land on a drawn struct/enum are kept, counts summed,
  self-ties dropped. References from free functions and trait items are not
  on this chart; the legend says so.

## The map

- **Marks**: one bordered block per visible pub struct/enum (hairline frame,
  paper ground, square corners). Header: the keyword and visibility in soft
  ink, the name in ink at 700, an amber `M` when the defining file changed.
  Body: the holding fields quoted as written (field name soft, the run that
  names the held type in ink 500); enums list their variant names in soft ink.
  Foot: `+ n plain fields` where any are hidden, then the locator in 8.5px soft
  ink. The whole block is the link to the type's definition plate — a field row
  names a type, it does not go anywhere the block does not.
- **Statics**: same block with a 2.5px ink left edge; `static NAME` header,
  the declared type quoted beneath, locator.
- **Frames**: crate frames and top-level module frames, 2.6% ink tint,
  label band on the border (`mod views` mono 500; the crate name where
  more than one crate exists). Each frame's counted fold row collects its
  private types.
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
  deterministic, blocks measured before placement, frames packed toward a
  landscape sheet.

## Chrome

- Cartouche (top-left): workspace name, the altitude ladder
  `dependencies · code · data` (all three cartouches gain the third rung),
  facts (`54 structs · 18 enums · 11 roots`), the diff line, the amber
  changed counts, and the insight line naming the top-level modules that
  hold changed types.
- The `references` toggle rides the cartouche (uses / used by / both).
- Legend beneath the cartouche: the three edge families with drawn samples,
  the static mark, `held by n` folds, `M`, the plain-field fold, then the
  honesty notes (wrapper table, what the walk does not chart, unresolved
  counts).
- Route: `/data`. Escape climbs to `/data` from nothing (no sub-focus in
  v1); `f` refits; `←`/`→` retrace history as everywhere.
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

- A data-side selection sheet (held by / holds / lifecycle per type) — v1
  links to the code plate instead.
- Lifecycle bands (born / mutated / read / consumed from fn signatures).
- Item-level diff marks (shape diff of a type against the base revision).
- Search on `/data` (the code search exists; jumping to data marks does not).
- Whether traits deserve resting marks or appear only as `dyn` targets.
