# Code structure viewer — design brief and behavior

Confirmed via /impeccable craft, 2026-08-18. Built the same day. The second
altitude of the review ladder: crates → **files → items**. No diff at this
altitude yet — structure only; change-awareness is the next rung.

## Job and audience

The same reviewer as the dependency atlas, one zoom level down. The question
this altitude answers: "what is in this code, and what does a change here
touch?" — at file precision first, item precision on demand. Visitor mode:
**Operate**.

## Decisions (user-confirmed)

- **Engine: rust-analyzer as a library** (`ra_ap_*` crates, server-only).
  References are resolved semantically — types, traits, and method calls the
  way rustc sees them — not by text matching. The weight is accepted; the
  survey runs once and is cached.
- **Scope: the whole workspace's files at once**, reflecting the directory
  structure (the shape the reviewer already knows). Workspace code only:
  references into dependencies are dropped at this altitude.
- **Gesture: focus + unfold in place.** Clicking a file selects it (a URL)
  and draws its references; the selected file **cuts away** — unfolds in
  place into a plate listing its items while every neighbor keeps its
  ground. Clicking an item draws that item's references. Back retraces.
- **Projection: the plan, and only the plan.** Both dealt projections were
  built and tried side by side; rings retired the same day (user decision,
  2026-08-18) and the PROJECTION toggle went with it. The chart draws one
  ground, so the map never rearranges itself under the reviewer.

## The map

- Files are the stars at this altitude: solid ink marks sized by how many
  other files reference them. Directories are ruled square junctions — paper
  when open, solid ink when folded shut into a **gate** that carries its
  count in words ("12 FILES"). Clicking a directory mark folds or opens it.
- **The plan** — an engraved town plan growing downward: every directory a
  street with its name lettered on the line, files as lots in rows above
  their street, subdirectories branching below. Every lot carries its name;
  a street's own name is lettered on its line, so open directory marks stay
  quiet and only gates label themselves. Street layout is a pure function of
  the tree: name-ordered, collision-free, deterministic.
- Crate districts: the shallowest directory holding all of a crate's files
  carries the engraved crate name ("CRATE slopify"); it links to the
  crate's district selection.
- Disclosure is budgeted: the first paint keeps the visible mark count
  under ~320 by folding the deepest directories into gates (stated in
  words); selecting into folded ground opens the gates above it.
- **References draw only for the selection** — the same discipline as the
  dependency rings. The REFS toggle picks the reading: USES (ink, arrow in)
  or USED BY (hairline, arrow out). Arrows point the way change travels —
  into the file that uses the definition. `×n` counts repeated references.
- The cutaway plate lists items in source order with a drawn glyph
  vocabulary (fn disc, struct square, enum diamond, trait open square, type
  open diamond, const bar, macro asterisk, module gate; impl headers as
  section rules). Public items are ink, private are quiet; line numbers
  ride on the right. Past ~24 rows the plate defers to the panel.
- Selecting an item (`?item=Type::name`) draws item-level chords: what this
  item uses / what uses it, across files. Chasing a reference in the panel
  descends into the other file with the target item selected — this is the
  function-call-graph navigation, one URL per hop.

## Routes

- `/code` — the whole map.
- `/code/crate/:name` — a crate's district selected; the map draws every
  reference crossing its boundary; the panel lists both directions and
  links back up to the crate's dependency focus (`/crate/:name`).
- `/code/file/:..path` — one file selected and cut away; its file panel
  lists refs both ways and every item.
- `/code/file/:..path?item=X` — one item selected inside the cutaway.
- The altitude ladder: both title blocks carry DEPENDENCIES · CODE; a
  member crate's dependency focus panel carries "its files ↓", a crate
  district panel carries "its dependencies ↑".

## Backend

- `src/analyze/code.rs`: `ra_ap_load_cargo::load_workspace_at` (build
  scripts on, sysroot proc-macro server, worker threads capped at a
  quarter of the machine — the survey is a guest). All Semantics work runs
  under `attach_db` (the new trait solver reads the db from a
  thread-local).
- Items: every file's syntax tree walked for fns, types, traits, consts,
  statics, macros, inline modules, and impl blocks (impl headers become
  sections; their assoc fns are items).
- References: paths (outermost only), method calls, and field accesses,
  resolved via Semantics; macro calls are expanded (three levels deep) and
  their references attributed to the call site. Enum-variant references
  chart as references to the enum; `Self` as the impl's type. References
  from an item to itself are dropped.
- Honesty: unresolved names are counted and written on the legend, never
  guessed. Derive-macro output is not counted, and the legend says so.
- The survey is computed once per server run (tokio OnceCell) and cached;
  `code_graph()` ships files + file-level edges eagerly, `file_detail(id)`
  ships one file's items and item-level references on unfold.
- Quiet by default: the `ra_ap_*` crates trace every query at INFO, so
  `main` appends `ra_ap=warn,salsa=warn,chalk=warn` to `RUST_LOG` — their
  warnings still surface, and a longer directive (`RUST_LOG=ra_ap_hir_ty=info`)
  still turns one of them back on.

## States

- Loading: its own constellation moment — "Surveying the code…", honest
  about the first survey taking a while.
- Failure: plain-words error plate with retry; the dependency atlas keeps
  working without the code survey.
- The map with a clean epoch is identical to the map with changes: this
  altitude does not draw the diff yet (stated in the cartouche).

## Scope note

Desktop only (user decision, 2026-08-18): mobile "isn't covered, we don't
care mobile usage". Narrow-viewport layouts exist and stay merely usable
(stacked furniture, bottom-docked panel, lowered zoom floor), but they get
no design or review effort.

## Open decisions

- Diff at this altitude: changed files flare, item-level blast radius.
- Search does not yet cover items, only files.
- The dependency trail (breadcrumb) does not span altitudes yet.
