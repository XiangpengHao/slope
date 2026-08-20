# Code structure viewer — design brief and behavior

Confirmed via /impeccable craft, 2026-08-18. **Resurveyed 2026-08-19**: the
streets-and-dots plan with its cutaway list was rejected and replaced by
nested territories plus a focus plate. **Reworked 2026-08-19** (audit): the
focus plate now quotes the item's own source instead of describing it, and
every invented label was replaced by the word rust, cargo, or the VCS already
uses. **Distilled 2026-08-19**: a count is drawn only where something is
hidden, the ambient tie layer became a reading with a per-block bar, and the
legend stopped restating what the drawing already says. The second altitude of
the review ladder: crates → **files → items**.

## Job and audience

The same reviewer as the dependency atlas, one zoom level down. The question
this altitude answers: "what is in this code, and what does a change here
touch?" — territory and coupling first, item precision on demand. Visitor
mode: **Operate**.

## Decisions (user-confirmed)

- **Engine: rust-analyzer as a library** (`ra_ap_*` crates, server-only, pinned
  at 0.0.348). References are resolved semantically — types, traits, and method
  calls the way rustc sees them — not by text matching. The weight is accepted;
  the survey runs once and is cached.
- **Scope: the whole workspace's files at once**, reflecting the directory
  structure (the shape the reviewer already knows). Workspace code only:
  references into dependencies are dropped at this altitude.
- **Projection: nested territories.** Directories are bordered districts,
  files are blocks inside them, items are rows inside a block. Nesting means
  one thing: belongs to. The layout is a pure function of (tree, disclosure,
  measured block sizes) — deterministic, no physics, same workspace same map.
  The streets-and-dots plan (2026-08-18) and the rings before it are retired.
- **Gesture: focus replaces the map.** Clicking a file or an item is a URL,
  and that URL draws a focus plate instead of the map. The map is for
  territory and coupling; the plate is for precision. Back / Escape climbs
  the ladder: item → file → whole map. One refinement (2026-08-19): on a
  focus plate, an item row is a leaf — its own focus would only quote code —
  so clicking it expands the definition in place instead of going one level
  deeper. That holds for every item kind (fn, struct, enum, trait, …), on
  the file outline and the impl sections alike; the item's own plate, with
  Used by / Uses, stays one step away (the pane's footer link, or
  middle-click). On the map the default stands: clicking anything goes to
  its focus.
- **Arrow keys retrace the review.** Every focus is a URL, so `←` / `→` are
  the browser's back and forward, globally, on every route (typing fields
  keep their caret keys).
- **The camera survives the round trip** (2026-08-19). The map's pan and zoom
  are session state, like the fold toggles: opening a focus plate and coming
  back — by any path, back button included — restores the camera exactly as
  the reviewer left it. The map frames itself only on a fresh session; `f`
  refits on demand. The same rule holds on `/data`.

## The six mechanisms

1. **One container model.** Crate → directory/module → file → type →
   method/field, one tree, every node with a fold state. References are
   recorded between leaf items but always *rendered* between the lowest
   visible containers, counts summed (`×n`). Fold a district and every edge
   into everything inside it gathers onto its gate; open it and they
   redistribute.
2. **Privacy is a permanent fold.** Private items never draw as marks
   anywhere. Their cross-container references lift — to their type if the type
   is visible, else to their file — and stay counted, because a file whose
   private helpers call another module is still coupled to it. In lists they are
   the group's counted fold, `+ 96 private` (distilled 2026-08-19): a row named
   `private items` names nothing, and it appeared in eight of nine groups on a
   plate, usually as the loudest row in each.
3. **Cartographic generalization.** The ambient map engraves only landmarks
   whose interest clears the altitude's bar: interest = item-level fan-in +
   visibility (`pub` 2, `pub(crate)` 1) + 2 if the diff touched the file. The
   bar is the whole map's (a mark budget, ~210 rows) with a per-block cap of 7,
   and every block still names its loudest item so no block goes mute.
   Everything else folds into a counted line — "+ 27 private", "+ 14 pub",
   "+ 9 pub · 5 private". Counts only, in rust's own word: a fold that does not
   count what it hides is a lie by omission, but the sentence saying what "pub"
   means here, and where a private item's references went, is the legend's, said
   once, not repeated by fifteen blocks.
   **Count only what is hidden** (distilled 2026-08-19): a fold counts, and
   nothing else does. A block's header carries its name and its `M`, never a
   `254 lines · 31 items` that competed with the name for width and won — the
   name was the half that got clipped. An open district frame names itself and
   stops: its files are on the paper to be counted. A folded gate still counts,
   because a gate is where the elision is. Line and item totals live in one
   place each — the cartouche for the workspace, the crate sheet's file list per
   file, the focus plate's facts line for the file in focus.
4. **Ambient coupling, as a reading** (reworked 2026-08-19, user decision).
   Ties are territory-to-territory: curved hairlines, width by count, the
   arrowhead resting on the **user** (the way change travels), aggregated — no
   global item-level spaghetti, ever. But *every* aggregated tie at rest was a
   hairball: ~100 curves over 18 files, counts floating unattributable to any
   one of them, which is the whole-graph-at-once the product's first principle
   forbids. So the map carries a **references** toggle — `uses` (default) /
   `used by` / `both` — and it is a reading, not a filter. Direction alone
   cannot thin an unanchored map, because `ties()` already holds one tie per
   ordered (definition, user) pair: every tie is somebody's use and somebody
   else's users, so a global direction filter is the identity. Each mode
   therefore anchors on the territories themselves — under `uses` a block draws
   the two heaviest files it leans on, under `used by` the two heaviest that
   lean on it, and `both` draws every tie unthinned. Nothing is cut: a folded
   tie stays in the set and inks in, with its count, when the reader hovers
   either of its ends, and the legend says so. The heaviest dozen *resting*
   ties carry their count on the paper; the rest keep it until hovered. A
   selected crate is already an anchor, so its boundary set draws whole in the
   reading's direction — a tie is the crate's when its file block belongs to the
   crate or its gate stands inside the crate's district. The toggle rides on the
   cartouche, because it acts on the whole plate; the crate sheet no longer
   carries it, and a crate with no boundary references says so in one line
   instead of two empty headings over a control with nothing to act on.
5. **Selection becomes a definition plate.** Center plate = the item's own
   source text, sliced from the file the survey read and lexed on the server:
   doc comment, attributes, signature, and body, exactly as written, with a
   line-number gutter counting from the item's real first line. Under it, the
   type's associated items grouped by the impl header they are written under
   (`impl Vis`), each row `pub fn weight` with `src/api.rs:165`. Left column
   `Used by`, right column `Uses` — each grouped by the file the reference is
   written in (path header + `37 refs`), groups sorted by weight, rows by
   count, rows capped at 3 with an expandable `+2 more (5 refs)`. Every row
   re-centers the plate. Breadcrumb: whole map ▸ directory ▸ file ▸ item.
6. **Semantic containment.** Methods attach to their self type, resolved
   through the impl's self ty def — including when the impl lives in another
   file. Impl blocks are attribution, not geometry: all impls of a type merge
   into the type, and the impl block itself holds no ground. A hand-written
   trait impl is listed on the type's plate under its own header, never as
   nesting; a derive is not listed at all, because it stands in the source
   the plate already quotes.

## The map

- **Directory frames**: one per open directory, faint ink tint, its name on
  the border exactly as it is on disk (`▾ views/`, mono, lowercase) and nothing
  else. The label folds the directory; the crate name (`slopify`, mono) on the
  shallowest directory holding all of a crate's files climbs to the crate sheet,
  and is drawn only where the survey has more than one crate to tell apart. The
  root frame carries the workspace.
- The label band (name, crate tag) is measured per segment on its own face, and
  the crate is placed clear of the name with the frame widened to hold both.
- **Blocks**: one per visible file. Header = name (amber `M` if the file
  changed since the diff base). Then landmark rows,
  each written as rust — `pub fn parse`, `struct Trail`, `pub(crate) mod
  tree` — the keyword and visibility in soft ink, the name in ink, engraved in
  three weight tiers by fan-in, each row a link to that item's definition
  plate. Then the fold's own words.
- **Gates**: a folded directory becomes one bordered row — "▸ views/" over
  "12 files · 255 items", the one place on the ground that counts, because it is
  the one place that hides something — and every tie into anything inside it
  lands there.
  Clicking opens it. Disclosure is budgeted: the first paint keeps the visible
  mark count under ~320 by folding the deepest directories.
- Blocks are measured before they are placed (name width, row widths, wrapped
  fold text), so a plate and its box agree to the pixel and no block stands on
  its neighbor. Districts pack their children into shelves aimed at a
  landscape sheet: files first in name order, then subdirectories.

## The definition plate

- Replaces the map for `/code/file/...`; the map's cartouche and legend go
  with it, search stays.
- An item focus quotes the item. `item_source(file, item)` returns the item's
  own source text, lexed into per-line coloured runs; the plate renders it at
  12px mono with a sticky, non-selectable line-number gutter, no wrapping,
  horizontal scroll inside the pane, and selectable code. A method quoted out
  of an impl block is given back its own indent and then dedented with its
  body. Nothing is cut (2026-08-19; the ~60-line body cut is retired): the
  pane caps at 70vh and scrolls, so a long body is whole and the plate keeps
  its shape.
- The quotation itself navigates. Every resolved reference's name token in
  the quoted source is a link to its definition — the same jump the Uses
  column offers, made where the reader is already looking. The link keeps its
  token's colour under a quiet underline; unresolved names and references
  into dependencies are plain text, as honest as the chart.
- The locator is `src/api.rs:10`, with an amber `M` when the file changed.
- Below the quotation: the type's associated items, grouped under the impl
  header they are written under, wherever in the workspace that is (that
  file's detail is fetched so the row can carry `path:line`). A hand-written
  trait impl with no items of its own is still listed.
- A whole-file focus has no single definition to quote, so it keeps an
  outline instead: `pub enum DepKind` · `12 refs` · line number, per item.
- **Rows expand in place** (2026-08-19): clicking an outline row or an impl
  section's member quotes that definition right under the row — same pane,
  capped at 40vh — instead of going one level deeper. Its own plate stays a
  footer link (`focus · used by / uses`) or a middle-click away. Clicking
  the row again folds it; Escape folds all open quotations before it climbs.
  Expansion is view state, not a URL.
- Both directions always show, so the plate's REFS toggle is gone. (`RefDir`
  returned 2026-08-19 as the map's reading toggle; the plate's columns are not
  affected by it.)
- Same-file references come from the file detail and are grouped by the same
  rules as the rest of the workspace.
- Private members are counted, never named: `+ 3 private`.
- **The columns' rows name things** (distilled 2026-08-19). References that lift
  to a file's private items used to take a row called `private items` in every
  group — eight of nine groups on a plate, and usually the loudest row in each,
  so the column's dominant content named nothing. They are now the group's own
  counted fold, `+ 96 private`, in the same grammar the map's blocks use.
- A reference group is a section of one list — a header, a rule, its rows — not
  a plate. Nine boxed groups down a column drew nine frames to say what one rule
  each says, and nested a card inside the page's own paper. Both columns set
  their type left: the column's side of the plate carries the direction, so a
  heading never floats away from the rows it names and a path is read the way a
  path is read.
- The centre track holds exactly one frame, and it is the quotation's: the
  double-ruled plate *is* the code pane, so a quotation is not a frame inside a
  frame 20px away from it. A whole-file focus, which has no quotation, keeps its
  outline on the page's bare paper under the facts line. The page runs to 1460px
  with 240px columns, so the quotation gets the width instead of the furniture.
- An outline row's two numbers read as two: `87 refs` then `:29`, the plate's own
  `path:line` idiom completed by the row, never `87 refs 29` running together as
  one column of digits.

## Routes

- `/code` — the whole map.
- `/code/crate/:name` — a crate's district emphasized on the map, only its
  own ties drawn (filtered by the sheet's uses / used by / both toggle); the
  sheet lists both directions of its boundary references and links up to the
  crate's dependency focus (`/crate/:name`).
- `/code/file/:..path` — one file's focus plate.
- `/code/file/:..path?item=X` — one item's focus plate (`X` is
  `Type::method` for anything inside a section).
- The altitude ladder: both title blocks carry DEPENDENCIES · CODE; a member
  crate's dependency focus panel carries "its files ↓", a crate district sheet
  carries "its dependencies ↑".

## Backend and wire model

- `src/analyze/code.rs`: `ra_ap_load_cargo::load_workspace_at` (build scripts
  on, sysroot proc-macro server, worker threads capped at a quarter of the
  machine — the survey is a guest). All Semantics work runs under `attach_db`
  (the new trait solver reads the db from a thread-local).
- Items: every file's syntax tree walked for fns, types, traits, consts,
  statics, macros, inline modules, and impl blocks. Each item carries a `Vis`
  (`Pub` / `Crate` / `Private`, classified from the ast visibility so
  `pub(crate)` never reads as `pub`) and the byte range of its own source,
  doc comment and attributes included. A trait's items inherit the trait's
  visibility; an impl's items keep what they declare, so a trait impl's
  methods fold into their type. There is no reconstructed signature and no
  extracted field list: the plate quotes the source instead.
- Containment: every impl block's self type is resolved semantically and the
  items under it get a `parent` link to that type's mark, cross-file included.
  A trait declaration owns the items inside it. Inline modules are not
  containers at this altitude — their items keep the module path in their name
  and stay on the file's shelf.
- References: paths (outermost only), method calls, and field accesses,
  resolved via Semantics; macro calls are expanded (three levels deep) and
  their references attributed to the call site. Enum-variant references chart
  as references to the enum; `Self` as the impl's type. A reference written
  inside an impl block belongs to the type that impl names, which is how
  `impl Trait for Type` becomes a type → trait tie. References from an item to
  itself are dropped.
- `code_graph()` ships eagerly: `files` (with `changed` from the VCS diff),
  file-level `refs`, every chartable item as an `ItemMark` (file, local id,
  name, URL label, kind, vis, line, parent, item-level `fan_in`, and the
  hand-written trait impls written for it anywhere in the workspace, as their
  headers are written — derives are not in that list; they stand in the
  type's own source), and
  every cross-file reference at item precision as an `ItemEdge` (from_file,
  from, to_file, to, count). That is everything the client needs to lift edges
  at any fold state without fetching item detail.
- `file_detail(id)` ships one file's items (name, section, kind, line range,
  vis, mark, byte range), its same-file `item_refs`, and its `refs_out` /
  `refs_in` at item precision. The plate uses it for the file outline, the
  impl headers, and same-file references; the item-level cross-file rows come
  from `item_edges`.
- `item_source(file, item)` slices one item's own text out of the per-file
  source the survey kept (`CodeIndex::sources`, server-only, never on the
  wire), dedents it, and lexes it with `ra_ap_syntax` into per-line runs
  tagged with a token class (keyword, doc, comment, string, number, lifetime,
  attribute, type, fn, macro, ident, punctuation, space). No new dependency,
  nothing on the wasm client. Fetched per (file, item) and cached in the
  shell's session state (a context on the layout) beside the file details.
- Runs carry their targets: the scan keeps every resolved reference's
  name-token byte range per file (`CodeIndex::ref_spans` — method and field
  names, a path's last segment; references inside macro expansions map back
  through `original_range_opt` and are kept only when they land in the file
  being scanned). `item_source` translates the spans inside the quoted range
  into display coordinates and marks the matching run with an index into a
  deduped `SrcLink` table (target file path + item URL label; empty label =
  the whole file). A run is `SrcRun { text, tok, link }`; nothing is split —
  a name token lexes as exactly one run.
- Honesty: unresolved names are counted and written on the legend, never
  guessed. Derive-macro output is not counted; a type's derives stand in its
  own source, on its plate.
- The survey is computed once per server run (tokio OnceCell) and cached.
- Quiet by default: the `ra_ap_*` crates trace every query at INFO, so `main`
  appends `ra_ap=warn,salsa=warn,chalk=warn` to `RUST_LOG`.

## States

- Loading: its own constellation moment, headed by what is actually running —
  "rust-analyzer is reading every source file and resolving references" —
  and honest about the first survey taking a while. The definition plate says
  `loading…` while an item's source is in flight.
- Failure: plain-words error plate with retry; the dependency atlas keeps
  working without the code survey.
- A clean diff draws no amber: the `M` marker and the cartouche's "n files
  changed" only appear when the diff has something to say.

## Scope note

Desktop only (user decision, 2026-08-18): mobile "isn't covered, we don't care
mobile usage". Narrow-viewport layouts exist and stay merely usable (stacked
furniture, single-column focus plate, lowered zoom floor), but they get no
design or review effort.

## Open decisions

- Item-level diff marks: files carry `changed`, items do not (that needs diff
  hunks against item ranges), so the plate's `M` states that the *file*
  changed, not the item.
- Splitting an aggregate tie into per-block strands on hover — the design
  allows it, the map does not do it yet.
- The impl section lists `pub fn weight`, not the full signature: the
  signature would have to be quoted per method, which is one request each.
- The dependency trail (breadcrumb) does not span altitudes yet.
