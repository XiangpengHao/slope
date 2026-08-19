# Code structure viewer — design brief and behavior

Confirmed via /impeccable craft, 2026-08-18. **Resurveyed 2026-08-19**: the
streets-and-dots plan with its cutaway list was rejected and replaced by
nested territories plus a focus plate. The second altitude of the review
ladder: crates → **files → items**.

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
  the ladder: item → file → whole map.

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
   private helpers call another module is still coupled to it. In lists, the
   lifted references show as one italic row: "private items — folded, lifted
   here", with its own count.
3. **Cartographic generalization.** The ambient map engraves only landmarks
   whose interest clears the altitude's bar: interest = item-level fan-in +
   visibility (`pub` 2, `pub(crate)` 1) + 2 if the epoch touched the file. The
   bar is the whole map's (a mark budget, ~210 rows) with a per-block cap of 7,
   and every block still names its loudest item so no block goes mute.
   Everything else folds into a counted line — "+ 27 private folded", "+ 14
   quieter pub", "+ 9 quieter pub · 5 private folded". Counts only: a fold that
   does not count what it hides is a lie by omission, but the sentence saying
   where a private item's references went is the legend's, said once, not
   repeated by fifteen blocks.
4. **Ambient coupling, aggregated.** The map always draws territory-to-
   territory ties: curved hairlines, width by count, the arrowhead resting on
   the **user** (the way change travels). No global item-level spaghetti,
   ever. The heaviest dozen ties carry their `×n` at rest and draw at full
   weight; lighter ties draw at half and keep their count until the reader
   hovers either end, and hovering a block brings its own ties up to full ink.
5. **Selection becomes a focus plate.** Center plate = the selection
   unfolded: fields with their types, enum variants with their payloads,
   methods (wherever their impls are written), a signature block for
   functions, derive and trait names as badges, and a last line in the chart's
   italic voice. Left column USED BY, right column USES — each grouped by
   container (file path header + group total `×n`), groups sorted by weight,
   rows by count, rows capped at 3 with an expandable "+ n more · ×m folded".
   Every row re-centers the plate. Breadcrumb: whole map ▸ directory ▸ file ▸
   item.
6. **Semantic containment.** Methods attach to their self type, resolved
   through the impl's self ty def — including when the impl lives in another
   file. Impl blocks are attribution, not geometry: all impls of a type merge
   into the type, and the impl block itself holds no ground. Trait membership
   is a lens (a badge on the type), never nesting.

## The map

- **Districts**: one per open directory, faint ink tint, engraved uppercase
  name on the border with its file and item counts beside it. The label folds
  the district; the crate name ("CRATE slopify") on the shallowest directory
  holding all of a crate's files climbs to the crate sheet. The root district
  carries the workspace.
- The label band (name, counts, crate tag) is measured per segment on its own
  face and tracking — these are tracked caps, and tracking is most of their
  width — and each segment is placed clear of the last, with the frame widened
  to hold the whole band. A frame with no room drops the counts entirely: a
  truncated name names nothing, and the halo of an overlapping segment eats
  the name it overlaps.
- **Blocks**: one per visible file. Header = name (carmine `▎` if the epoch
  touched it) + lines and item count. Then landmark rows: kind glyph (■
  struct, ◆ enum, ● fn, △ trait, and the rest of the vocabulary), engraved in
  three size tiers by fan-in, each row a link to that item's focus plate.
  Then the fold's own words.
- **Gates**: a folded directory becomes one bordered row — "▸ views/ folded ·
  12 files · 255 items" — and every tie into anything inside it lands there.
  Clicking opens it. Disclosure is budgeted: the first paint keeps the visible
  mark count under ~320 by folding the deepest directories.
- Blocks are measured before they are placed (name width, row widths, wrapped
  fold text), so a plate and its box agree to the pixel and no block stands on
  its neighbor. Districts pack their children into shelves aimed at a
  landscape sheet: files first in name order, then subdirectories.

## The focus plate

- Replaces the map for `/code/file/...`; the map's cartouche and legend go
  with it, search stays.
- Both directions always show, so the REFS toggle is gone (and with it the
  `RefDir` state).
- Same-file references come from the file detail and are grouped by the same
  rules as the rest of the workspace.
- A type's methods are listed even when their impl is written elsewhere; that
  file's detail is fetched so the row can name where it lives.
- Private members are counted into "+ n private members folded into this
  plate — their outside references lift here".

## Routes

- `/code` — the whole map.
- `/code/crate/:name` — a crate's district emphasized on the map; the sheet
  lists both directions of its boundary references and links up to the crate's
  dependency focus (`/crate/:name`).
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
  `pub(crate)` never reads as `pub`), its fields or variants, a function's
  signature without its body, and its derive names. A trait's items inherit
  the trait's visibility; an impl's items keep what they declare, so a trait
  impl's methods fold into their type.
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
  name, URL label, kind, vis, line, parent, item-level `fan_in`, traits), and
  every cross-file reference at item precision as an `ItemEdge` (from_file,
  from, to_file, to, count). That is everything the client needs to lift edges
  at any fold state without fetching item detail.
- `file_detail(id)` still ships one file's items with their bodies (fields,
  variants, signatures, derives), its same-file `item_refs`, and its
  `refs_out` / `refs_in` at item precision. The focus plate uses it for bodies
  and same-file references; the item-level cross-file rows now come from
  `item_edges`.
- Honesty: unresolved names are counted and written on the legend, never
  guessed. Derive-macro output is not counted, and the derived traits are
  listed on the type's plate instead.
- The survey is computed once per server run (tokio OnceCell) and cached.
- Quiet by default: the `ra_ap_*` crates trace every query at INFO, so `main`
  appends `ra_ap=warn,salsa=warn,chalk=warn` to `RUST_LOG`.

## States

- Loading: its own constellation moment — "Surveying the code…", honest about
  the first survey taking a while.
- Failure: plain-words error plate with retry; the dependency atlas keeps
  working without the code survey.
- A clean epoch draws no carmine: the `▎` tick and the cartouche's "n files
  touched this epoch" only appear when the diff has something to say.

## Scope note

Desktop only (user decision, 2026-08-18): mobile "isn't covered, we don't care
mobile usage". Narrow-viewport layouts exist and stay merely usable (stacked
furniture, single-column focus plate, lowered zoom floor), but they get no
design or review effort.

## Open decisions

- Item-level epoch marks: files carry `changed`, items do not (that needs diff
  hunks against item ranges).
- Splitting an aggregate tie into per-block strands on hover — the design
  allows it, the map does not do it yet.
- Search covers files, not items.
- The dependency trail (breadcrumb) does not span altitudes yet.
