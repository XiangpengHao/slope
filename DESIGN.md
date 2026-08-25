---
name: Slope — Workspace Atlas
description: A cargo workspace charted as an engraved star atlas — ink on paper, where only change takes color, and every declaration is quoted from the source.
colors:
  paper: "#f6f4ed"
  ink: "#23303c"
  ink-soft: "#5a6574"
  ink-line: "#949dab"
  ink-faint: "#b3bac4"
  flare: "#a54c06"
  tok-kw: "#1c4f8b"
  tok-sum: "#6b3fa0"
  tok-type: "#0f6a6a"
  tok-num: "#8a2f7a"
  tok-punct: "#4b5563"
  tok-fn: "#6b3fa0"
  tok-str: "#9c2f4a"
  tok-doc: "#2f6b45"
  tok-comment: "#5f7060"
typography:
  chart-title:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "19px"
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: "0.18em"
  plate-title:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "17px"
    fontWeight: 400
    letterSpacing: "0.18em"
  chart-heading:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "11px"
    fontWeight: 400
    letterSpacing: "0.22em"
  chart-heading-lg:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "13px"
    fontWeight: 600
    letterSpacing: "0.26em"
  chart-caption:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "12px"
    fontWeight: 400
  data-focal:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace"
    fontSize: "15px"
    fontWeight: 600
  data-name:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace"
    fontSize: "11.5px"
    fontWeight: 500
  data-body:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace"
    fontSize: "10.5px"
    fontWeight: 400
    lineHeight: 1.6
  data-state:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace"
    fontSize: "9.5px"
    fontWeight: 400
  data-meta:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace"
    fontSize: "8.5px"
    fontWeight: 400
rounded:
  none: "0px"
spacing:
  hairline-gap: "6px"
  row: "8px"
  block: "12px"
  plate-pad-x: "16px"
  chrome-inset: "12px"
components:
  plate:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "12px 16px"
  search-input:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    typography: "{typography.data-body}"
    rounded: "{rounded.none}"
    padding: "6px 12px"
  crate-row:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.data-name}"
    padding: "2px 4px"
  segment-toggle-active:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.data-body}"
    padding: "2px 6px"
---

# Design System: Slope — Workspace Atlas

## Overview

**Creative North Star: "The Engraved Star Atlas"**

Slope draws a cargo workspace the way a nineteenth-century plate draws the sky: engraving ink on warm paper, hairline constellation lines, a cartouche for the title block — and every mark carrying its own words, so the plate needs no manual beside it. It deliberately refuses the glowing graph-dashboard with sidebar filters. The whole page is one material — ink at varying pressure on one sheet of paper — and the chart rests as a monochrome engraving until change appears.

The atlas has two altitudes of that one plate: the dependency chart (`/dep`, crates as stars on rings of hops) and the data chart (`/data`, the workspace's state — every struct, enum, union and static, whatever its visibility — tiered into roots and the blocks nested inside them, inside module frames, with holding and body-dependence edges between them). The altitude ladder — a `dependencies · data` line in every cartouche, the current rung engraved solid, the other a link — is the only navigation between them. Both share the plate, the words-at-the-mark discipline, and the URL-per-focus rule; the data altitude adds module frames, its two edge families, the nested state block and the root's widened left edge, without adding a second material.

Two more altitudes stood between those two and were removed on 2026-08-24, both by user decision. The **surface chart** (`/surface`) drew every contract the code publishes — types with their method bands, free functions, traits, consts and aliases — as blocks in the same module frames, with interface coupling solid and body dependence dashed; it read the same types the data chart reads, one rung apart. The **code map** (`/code`) drew the workspace's files as ruled blocks seated inside nested directory frames, with resolved reference ties between them and a definition plate that quoted an item's own source, syntax-highlighted, every resolved name in it a link. Neither grammar died with them: the module frames, the ownership seating, the quoted rows, the two inks and the fold machinery are the data chart's now, and the token palette still colours a data block's quoted rows. That definition plate came back on 2026-08-24 (user) as the data chart's **quotation plate** — no map, no three tracks, one row of one sheet quoted beside the sheet that names it — which is the whole of the code map that a reviewer actually asked for. This record keeps their rules wherever they still describe what ships, and says so where they do not.

The one thing the drawing never does is paraphrase the code. Where the workspace's source, cargo's output, or the VCS's status already answers a question, the interface quotes it: a field is its own declared row; a kind is its rust keyword; a location is `path:line`; a changed file is `M`. The atlas draws what has no textual form and quotes everything that does.

Density is instrument-grade: small mono type, tight tracking on the few spaced-caps headings, thin rules. The interface furniture (cartouche, search, toggles, focus panel) floats over a full-bleed chart as double-ruled paper plates; nothing is a "card" in the app sense. Motion is nearly absent — the authored moments are the loading constellation drawing itself in and the 400ms camera/mark glide; both stand down under `prefers-reduced-motion`.

The build is light-theme only by user decision; a dark "field edition" of the same plate is deferred, not implied. The designed surface is the desktop; narrow layouts ship and stay usable but are explicitly descoped (see Layout).

**Key Characteristics:**
- One material: engraving ink on paper; no gradients, no glass, no glow.
- Color is state everywhere except inside a quoted row: the page is monochrome except the flare amber, which means CHANGED (and its blast halo) and nothing else, ever. Inside a quotation, color is token class, and it stops at the block's frame.
- Every state is named in words, and where a state has a name in rust, cargo, or the VCS, that name *is* the words: `pub(crate)`, `dev-dependencies`, `M`, `3 files changed`, `+ 4 private`.
- A count is drawn only where something is hidden. A fold counts; a container whose contents are on the paper does not restate them.
- Every focus is a URL (`/dep`, `/dep/crate/:name`, `/dep/ring/:hop`, `/data`, `/data/mark/:..path?item=`, `/data/mod/:..module`; `/` redirects to `/dep`, the rung a review starts on); the back button retraces the review trail.
- Square corners everywhere; depth by ruling, not by shadow.
- Two altitudes, one grammar: the dependency chart draws edges only for the selection; the data chart says plain same-module ownership as nesting, draws only what nesting cannot say, and thins its body dependences to the reading its own toggle names. Neither ever draws the whole graph at once.

## Colors

An ink-pressure ramp on one paper ground, one reserved signal color, and one token palette that lives only inside a quotation.

### Primary
- **Flare Amber** (#a54c06): the one color on the chart. It means CHANGED — flare rays on a changed star, the fading blast-radius halo on downstream stars, manifest-event edges and their labels (added / removed / version bumps), the `M` on a changed file at either altitude, the structural diff's ink on the data chart — the `A`/`M`/`D` letters and the diff-touched block's own frame, the `+`/`−` row marks, added and removed edges with their words — and the counts that state a change (`3 files changed`, `7 added · 13 removed`). It appears nowhere else: never decoration, never emphasis, never brand.

### Neutral
- **Paper** (#f6f4ed): the page ground, plate backgrounds, code-pane ground, the fill of external-crate open circles, and the halo stroke behind any text engraved on the chart. The browser chrome the app doesn't draw (selection, caret, scrollbar) is tinted to stay on the plate.
- **Engraving Ink** (#23303c): body text, solid star fills, quoted source, plate borders, focus rings, selected underlines, and the selection's "uses" ties. The darkest pressure; there is no black.
- **Soft Ink** (#5a6574): secondary text — the keyword in front of a name, external crate names, counts, edge counts, attributes inside a quotation, ghost (removed) outlines, muted labels.
- **Hairline Ink** (#949dab): constellation lines and reference ties, ring guides, file-block and directory frames, the left rule of an in-place quotation, thin rules inside plates, hover underlines, minimap marks. Never used for body text.
- **Faint Ink** (#b3bac4): ghost outlines only; the lightest pressure the plate allows.

### Quotation tokens
Only inside a quotation, and only as token class: a data mark's quoted rows (2026-08-19, user-confirmed — those rows are sliced source, so they carry the palette and it stops at the block's frame; the palette was the removed code map's definition plate's first), and the quotation plate's whole quoted item (2026-08-24). Each clears 4.5:1 on paper, and none sits in the flare's hue, so amber keeps its exclusive meaning inside a quotation.

- **Keyword** (#1c4f8b) — every rust keyword.
- **Type / lifetime** (#0f6a6a) — uppercase-initial names, `'a`.
- **Sum type's name** (#6b3fa0) — an enum's own name in a block header; see the Kind-Color Rule.
- **Number** (#8a2f7a) — integer and float literals.
- **Punctuation** (#4b5563). Attributes take soft ink whole, `#[derive(...)]` included: an attribute reads as one unit.

Four more classes exist only where a whole item is quoted, because only a body says them — **fn / macro name** (#6b3fa0), **string** (#9c2f4a), **doc comment** (#2f6b45), **comment** (#5f7060). They left with the code map on 2026-08-24 and returned the same day with the quotation plate; a declared row on a block still says none of them.

### Named Rules
**The Kind-Color Rule.** Inside a data block — the one place at any altitude where a type's kind is drawn rather than only written — a struct's or union's name is type-teal, and an enum's is the palette's purple (#6b3fa0). It is a second reading of what the keyword in front of the name already says, taken because at chart zoom the name is the run that carries. Nothing else on the chart takes a kind color, and no kind color leaves a block.

**The Flare Monopoly Rule.** #a54c06 exists only as CHANGED and its consequences (blast halo, manifest events, the structural diff's letters, row marks, event edges, and diff-touched frames). No other element may take it. A quoted row carries an editor's token palette; no highlight color may leak outside a block's frame, and amber keeps its exclusive CHANGED meaning everywhere. Remove every changed crate and the chart must read as a pure monochrome engraving.

**The Words-Not-Color Rule.** Every state the chart can draw is also written out. Where the state has a name in rust, cargo, or the VCS, that name is the words — lowercase, mono, spelled the way the tool spells it: `pub`, `pub(crate)`, `(dev)`, `dev-dependencies`, `M`, `3 files changed`, `2 hops downstream`, `+ 4 private`. Invented uppercase abbreviations (EXT, DEV, GATE, `12 L`) are retired. Color reinforces; it never carries meaning alone.

**The Ink-Pressure Rule.** Grays are not arbitrary: ink → ink-soft → ink-line → ink-faint is a pressure ramp with fixed roles (text → secondary text → lines → ghosts). Do not invent intermediate grays; mixes are expressed as `color-mix(in srgb, var(--color-ink) N%, transparent)` over paper.

## Typography

**Chart Font (furniture):** EB Garamond (with Georgia, "Times New Roman", serif) — 400, 400 italic, 600; self-hosted woff2.
**Data Font:** JetBrains Mono (with ui-monospace, SFMono-Regular, monospace) — 400, 500; self-hosted woff2.

**Character:** The serif is the engraver's lettering — spaced small caps for plate titles and section headings, and nothing else. The mono is the surveyor's data hand, and it holds every fact: crate and file names, directory names, item rows, keywords, versions, counts, state words, locators, and quoted source. The two voices never swap jobs.

### Hierarchy
- **Chart Title** (Garamond 400, 19px, uppercase, 0.18em tracking): the cartouche's workspace name; error-plate titles at 17px.
- **Chart Heading** (Garamond 400/600, 11–13px, uppercase, 0.18–0.26em tracking): plate section headings — "Changes (n)", "Used by (n)", "Depends on (n)", "Uses (n)", "Implements (n)", the toggle caption.
- **Data Name** (Mono 500, 10.5–11.5px): crate, file, and item names on the chart and in lists; focal names at 15px semibold in panels.
- **Data Body** (Mono 400, 10–10.5px, relaxed leading): facts, counts, the survey fold's notes, the diff line, locators, item rows.
- **Data State** (Mono 400, 8.5–9.5px): keywords in front of a name, counts, `M`, `held by 4 types`, block meta at 8.5px. Lowercase, untracked; tracking is reserved for the few uppercase runs that remain (the altitude ladder, breadcrumb links, and "show all n" actions at 0.1–0.14em).

The whole ramp, so nothing drifts off it: 8.5, 9, 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13, 15, 17, 19px.

### Named Rules
**The Two-Voices Rule.** EB Garamond spaced caps for chart furniture — plate titles and section headings; JetBrains Mono for everything that is data. Garamond never carries a count, a state, a reference row, a locator, or a sentence about the data. Pub/private is weight and pressure, not a new voice: pub names are ink at 500, private names are soft ink at 400.

**The Spaced-Caps Rule.** Uppercase always carries letterspacing (0.1em minimum), and only furniture may be uppercase at all. Rust keywords, cargo table names, file paths, directory names, crate names, and VCS status letters are never uppercased and never tracked — they are written the way the tool writes them. Untracked uppercase does not exist on this plate.

**The Paper-Halo Rule.** Any text engraved directly on the chart (star labels, ring captions, module labels and names, edge labels and their counts) wears a paper halo — `paint-order: stroke` with a 3–5px paper stroke, or the stacked paper text-shadow — so words stay legible where lines cross. Text inside a plate never needs one.

## Layout

The chart is the page: a full-bleed `h-dvh w-dvw` canvas with `overflow: hidden`, dressed as the plate (paper ground, faint ink dot grid at 16% ink). Furniture floats over it in a fixed choreography — cartouche top-left, search and toggles top-right, zoom controls bottom-right, focus panel right (desktop) — each layer `pointer-events-none` with interactive plates opted back in.

- **Scope:** the desktop is the designed surface; mobile was descoped by explicit user decision. Narrow layouts (< 640px) ship and remain usable — chrome stacks vertically, panels dock to the bottom at max 44dvh — but they are a serviceable fallback, not a designed composition. Do not derive system rules from the narrow layout.
- **Chrome insets:** 12px page margin (`p-3`). Fit-view reserves the furniture's space so the chart centers in what remains — dependency altitude desktop: 52px top, 284px left, 20px bottom, 20px right rising to 330px when a panel is open; data altitude desktop: 56px top, 284px left, 24px bottom, 24px right rising to 330px when a sheet is open. The dependency fit leaves a 12% breathing margin (fit × 0.88); the data fit leaves 6% (fit × 0.94), because its blocks are rectangles that already carry their own padding.
- **Legibility floor:** fit-view never zooms below a floor — dependency chart 0.7 focused / 0.22 overview; data chart 0.22. Past the floor the chart holds readable scale and pans, seating the focal point mid-frame.
- **Camera discipline:** the camera moves only for a focus, the first paint of a fresh session, or an explicit refit (`f`). Deselecting never moves the camera, and a remount is a return: both charts keep their pan and zoom in session state, so coming back to a chart (by any path, back button included) restores the camera exactly as the reader left it (2026-08-19). Every move is one 400ms glide (`cubic-bezier(0.65, 0, 0.35, 1)`), 0ms under reduced motion; marks, ring guides, and re-inked edges travel on the same beat so plate and camera move together. One amendment (2026-08-21, user decision): selecting a mark the glass cannot show — off the viewport, or below reading zoom — glides the camera to it, because a selection the reader cannot see is not a focus; a selection already legible moves nothing. It began on the data chart the day a row's held type became a link to a block that can stand anywhere on the paper.
- **Disclosure:** the data chart folds by attention, never by a mark budget: a module folds because the reviewer folded it, and every fold leaves a counted row on the boundary it stood in. Layout is pure and deterministic — the same survey always draws the same chart.
- **The selection sheet:** a 288px plate in the right column, `max-h-full` with its own scroll, so the chart keeps the rest of the glass. On a phone it lays across the bottom at `max-h-[44dvh]`. (The removed code map's definition plate was a whole scrolling page of three tracks instead; nothing on either chart takes the glass that way now.)
- **The quotation plate** (2026-08-24, user): a second plate immediately left of the sheet (`right-[19.5rem]`), top-aligned with it, capped at the room between the cartouche and the sheet (`min(46rem, 100vw − 37rem)` — about a hundred columns of rust on a desktop) and at the glass's height, its quoted source scrolling inside it both ways. It never covers either piece of furniture, and it opens only from a sheet row. On a phone it lays over the sheet, which is one back-step behind it.
- **Spacing rhythm:** tight and even — 4/6/8/12px steps inside plates; 16px horizontal plate padding; 17px block row height; hairline rules (`border-t` in ink-line) divide sections instead of extra whitespace.

## Elevation & Depth

Effectively flat: depth is drawn, not cast. Plates sit on the paper via their double ruling; the only shadow is a single faint contact shadow under floating plates, and it never grows on hover or state change. Layering is z-index choreography over the chart, not simulated altitude — the ground layer (ring guides, directory frames) under the ties, the ties under the blocks and stars, the floating furniture over all of it.

### Shadow Vocabulary
- **Plate contact** (`0 1px 4px color-mix(in srgb, var(--color-ink) 14%, transparent)`): the sole drop shadow, applied to every floating plate and the re-ruled flow overlays (controls, minimap). Nothing else casts — the map's file blocks and directory frames are ruled, not raised.

### Named Rules
**The Drawn-Depth Rule.** Depth comes from ruling — the double-line frame — not from shadow ramps. There is no shadow scale; the one contact shadow is a constant, not a state.

## Shapes

Square corners everywhere: `border-radius: 0` on plates, inputs, buttons, and the third-party flow overlays (explicitly re-ruled to 0). The signature silhouette is the **double-ruled plate**: a 1px ink border with an inner hairline drawn as inset box-shadows (`inset 0 0 0 2.5px paper, inset 0 0 0 3.25px 45% ink`) so it survives on replaced elements like inputs.

Geometry is vocabulary, and it is spent only where words cannot go: circles belong to the star grammar (crates on the dependency chart), ruled rectangles to the data chart's blocks and module frames. Every container rectangle is ruled, never rounded. Item kind is not a shape — it is a keyword (see below).

### Named Rules
**The One-Plate Rule.** Every panel, popover, input, and overlay is the same plate: paper ground, ink frame, inner hairline, square corners. There is no second container style.

**The Keyword-Is-Kind Rule.** An item's kind is its lowercase rust keyword, set in mono in front of its name — `fn parse`, `pub struct Trail`, `pub(crate) mod tree`. Kind is a word, not a shape and not a color — with the single exception the Kind-Color Rule names, a data block's own name, where the color is a second reading of the keyword standing right in front of it. A private item writes no visibility, exactly as rust writes none. This replaces the retired Shape-Is-Kind rule and the item-glyph set it described: a learned shape vocabulary asked the reader to memorize what rust already spells out.

## Components

### Plates (Title Blocks, selection panels, toggles, error plates)
- **Corner Style:** square (0px).
- **Background:** paper (#f6f4ed); **Border:** 1px ink + inner hairline (see Shapes); **Shadow:** plate contact only.
- **Internal Padding:** 16px horizontal, 8–12px vertical; sections divided by ink-line hairline rules.
- A foldable section carries the class `fold` (the title block's changes, the cartouche's survey fold). Its summary shows a typographic `–`/`+` marker in ink-soft — no chevron glyph.
- Every cartouche carries the altitude ladder (`dependencies · code · data` in 9.5px tracked mono caps): the current rung engraved solid ink with an underline, the others soft-ink links. Under it the cartouche states its own facts in mono — the census, the diff line (`diff main @ 1a2b3c4 → working copy`), what changed, and which modules it landed in.
- **The Census-Not-Inventory Rule** (2026-08-21, distill). A cartouche states the census of what the chart draws — `98 structs · 35 enums · 1 static`, in rust's own words — and the diff. It does not recite the model's bookkeeping: `55 roots · 63 nested · 16 standing` and `209 body dependences · 127 at rest` were four invented terms defined only in legend prose, and no reviewer decides anything on them. Two count lines is the ceiling. Where a number *is* the chart's own reading, the paper draws it (a root wears the ink edge) and the mark's own hover words teach it — the cartouche does not tally it. This is the same rule the open directory frame already keeps one altitude down: a fact whose subject is on the paper is not restated in the corner.
- A selection panel opens with the breadcrumb ("← whole chart"), then names the selection. It caps at 44dvh on a phone and at the column height on a desktop, scrolling its lists inside. Long reference lists chunk at 8 rows with a typographic "show all n" action.

### The Star Mark (signature, dependency altitude)
The unit of the dependency chart, reused by chart, search results, and list rows. Radius encodes magnitude — `4 + √dependents × 1.3`, capped at 11px (the shared `star_radius`). Grammar: workspace member = solid ink disc with a 0.6px orbit ring; external crate = open paper circle with 1.3px ink stroke; changed = flare-filled with eight amber rays; downstream = amber halo ring whose opacity fades with hop distance (0.85 − 0.22/hop, floor 0.3); removed = dashed ink-soft outline only; focal = double ink ring with four compass ticks. Node labels: mono 500 name with a paper halo and an underline that appears in ink-line on hover and ink when selected; state words beneath in 9.5px mono — `3 files changed`, `2 hops downstream`, `removed`.

### The Data Mark (signature, data altitude)
One block per shape the workspace keeps — struct, enum, union — and per static, as narrow as the visibility reading admits (2026-08-25, user): the cartouche's `visibility` slider runs `pub` → `pub(crate)` → `pub(super)` → `all`, and a declaration written narrower than the stop has no block at all — no `+ n private` row either, just `n off` at the slider's foot. Each mark is a hairline frame around a quotation. The header: the keyword and visibility in keyword-blue, the name at 700 — type-teal for a product type, the palette's purple for a sum type — and the structural diff's letter in git's own alphabet (`A` added since the base, `M` declaration changed, `D` removed; a file-level change alone marks nothing). The rows are the declaration's clauses, quoted as written: a struct's or union's fields in declaration order, an enum's variants with payloads and discriminants (`File(String, String)`), a static's declared type. Each row is colored by token class, with the one run that names a workspace mark in bold; a plain type name is from outside the workspace, has no mark, and so has no line drawn to it. Each row also wears its own `pub`, `pub(crate)`, `pub(super)` or `pub(in path)` in keyword-blue where it declares one (2026-08-24, user — a block's header says what the *type* publishes, and a reader deciding what may touch this state needs the field's own answer), written as the source writes it and never as a wider rung's keyword. The visibility reading never touches rows: a block is a quotation, and a quotation missing its private fields is a misquotation. There is no method band: a block here is state only. What a type promises and what it offers are read on the selection sheet, not on the paper (amended 2026-08-24, user).

**The bold run is also the way to it** (2026-08-24, user). Where the chart draws the block that run names, the run is a link: click `ItemKind` in `kind: ItemKind` and the chart selects `ItemKind`. It is the same focus the block's own click is — the sheet lifts, the blast radius inks, and the camera glides only where the chosen block was not already legible — so following a type costs one click instead of a search across the paper by eye. The affordance is the block name's own, an underline while the pointer rests on the run and nothing at all at rest: a row whose types were underlined all the time would read as decoration rather than as a declaration. A run with no block to reach stays bold text and nothing more — a type inside a hand-folded module, and a run naming the block it is written in, which is where the reader already stands. This holds wherever a row is quoted: fields, variants, and a static's declared type.

**A block is bracketed the way rust brackets the declaration it quotes** (2026-08-24, user). The head opens with `{` and a line of its own closes it — that closing brace is what makes a long block's end findable, and it is what tells a reader that the blocks nested below it are state this one owns, not more of its own declaration. A static has no body to bracket, so its head ends in the `:` that introduces the line under it. A declaration with no rows at all brackets nothing: inventing `{}` would be guessing at a unit struct. The braces are punctuation ink, like every other bracket inside a quotation, and the far edition drops them with the rest of the rows. **The rows stand in from the edge** the way rust indents a body, and the closing brace does not — which is what makes the block's end findable at a glance. The indent is 12px, not rust's four spaces: at 10px it reads as an indent and costs a block twelve pixels of width instead of twenty-four. The diff's row marker lives in that gutter, so a `+` row and the untouched row under it start their text on the same column; before the indent the marker pushed its own row nine pixels right and a woven block read as ragged.

**No row caps** (revised 2026-08-20, user decision). A block draws its whole declaration — every field, every variant — and the layout is handed that height. A declaration read eight rows deep is a declaration half read, and a reader who has to select a block to finish its shape is reading the chart twice, so `+ 4 more fields` and the windowing that produced it are gone. Selecting a block opens nothing, because nothing was closed. `held by n types` is the one counted line left at the foot, and it is the chart's own fold — ink it will not draw — not a row the block is holding back.

The ground is **modules, not directories**: one frame per workspace crate, and inside it one frame per module nested exactly as rust's modules nest, each labeled in rust's own words with the last segment alone on the border because the paper's own nesting says the rest (revised 2026-08-20, user decision — one flat level read as a lie about the code, since `views::data` and `views::codemap` are not one module). Prose that names a frame away from the paper — the cartouche's insight line, a sheet's fold row — spells the whole path (`views::data`), because three modules answer to `mod map`. Within a frame, types seat under their one heaviest same-frame `Owns` holder, ownership depth as layers, so an owns edge is usually a short line between neighbours; a type never seats outside its own module, so cross-frame ownership stays drawn ink instead of being arranged away.

The chart's one organizing move is the **tier** (2026-08-21, user decision). Top-level data is a **root** — a static, or a type no other workspace type keeps in a field (`Owns` or `Shares`; a borrow is a view, not a hold) — and every root wears a 2.5px ink left edge, the static's own mark widened to every block a chain of holding begins at. Secondary data is drawn **inside** the block of the type that owns it hardest: a hairline rule closes the rows and the owned blocks shelve on the clear paper under it, recursively, so reading the tier is reading the paper's own nesting. Plain same-module ownership is therefore never a line — the containment is the edge. What stays drawn ink, solid with the wrapper's word, is exactly what nesting cannot say: sharing (a handle has no single container, so shared state stands beside its holders), borrowing, second holders, cross-module ownership, a ring of mutual owners, and the diff's added and removed relations. A type held by more than three types is vocabulary: it stands, its fan-in folds to `held by n types` on its own foot, and hover or selection inks the lines back in. That left edge is the last of the moves left in the plate vocabulary: dashed is a ghost, flare is diff ink, a left edge is a root, line weight is the wires'.

**Exactly two inks, and one direction rule: the arrowhead rests on the dependent.** *Solid* is holding — the dependent's own declaration keeps the tail — and draws at one pressure whatever the wrapper is, because the wrapper writes its own word on the line (`Arc`, `&`, `dyn`, `implements`) and no word at all is plain ownership. Dash used to encode which wrapper it was; dash is spoken for now. *Dashed and lighter* (`5 3` at 0.45) is implementation coupling: the dependent's *body* leans on the tail — a call, a name written inside a function, its ends climbed to the type that declares them — carrying its count, lighter because a rewrite can take it back without anyone else noticing. The two families draw to opposite sides of a block and never merge. The `references` toggle thins the dashed family to two per mark at rest, labeling the heaviest dozen; a folded wire is never cut — it inks back in on hover of either end, and stays inked for as long as either end is selected.

Nothing folds by count (revised 2026-08-21, distill). Every mark is drawn, however many there are. The retired 200-mark budget (`MARK_BUDGET`) folded each frame's quietest into a `+ n more items` row: it hid marks by a number nobody set, reflowed the whole chart when the threshold moved, invented a row that attracted edges, and left marks a URL could still point at. The folds that remain are the ones a reader asks for — a module folded by hand — and the ones the chart draws a count for in their place.

What has no block here is counted, never cut: `named by n signatures` (free fns, method rows, consts, aliases whose declared surface names the type) and `used by n bodies` (references from code with no mark of its own) live in the block's hover words, and the sheet lists both as rows (revised 2026-08-21, critique — stamped on every resting block they were texture, not signal, and they were never elided ink). The used-by half became real rows on 2026-08-23 (user): an undrawn body is still a nameable item, so `Used by` and `Uses` name every free function, trait and const alongside the drawn types — keyword, count, and a link to its definition, since the chart cannot re-centre on a mark it does not draw. One relation is one ranking: the merged list reads heaviest-first whatever kind the far end is, because the sheet shows eight rows before `show all` and those eight must be the heaviest, not the drawn ones. The sentence that used to count the rest is gone. The one count on a block's foot is `held by n types`, the vocabulary fan-in the chart folded; locators likewise live in the hover words and on the sheet, not on the resting paper. A reading's recede acts on a block's *own paint* — its frame and text — never on its box, because a receded holder can have a lit block nested inside it.

The **structural diff** (2026-08-19) is drawn in this same grammar, against the base edition of every changed file (read syntactically, matched by kind and name — the cartouche's survey fold states the method). A diff-touched block wears the flare on its own frame and its letter beside its name; what `M` reads is the declaration's own text, because a rewritten body moves no row this chart draws. A removed mark is a **ghost**: the same block dashed, rows quoted as the base wrote them, locator `…:113 (base)`, seated in the frame its path names. An added row wears a flare `+`; a dropped one is quoted from the base, struck, and seated where it stood. A relation the base could not have drawn takes flare with `added` on the line; one only the base had is re-drawn flare-dashed (`9 5`) with `removed`. Diff marks and edges never fold, and while the diff has anything to say the untouched marks rest at 0.62 pressure (hover restores). A clean diff draws none of this, and the chart reads as the monochrome engraving the Flare Monopoly promises.

**The far edition** (2026-08-21, user decision). Zoom is the fold: below reading zoom (0.45, back above 0.55) the chart swaps to a projection of names, tiers and wires — each block draws its keyword and name alone at a size its box can carry, the root's edge and the chosen block's ring hold near-constant screen width, wire words retire while the lines thicken to stay lines, and module names engrave across their own frames in soft ink the way an atlas names a region. The blocks keep their boxes, so nothing moves and every wire lands where it landed; descend — or select, which glides to reading zoom — and the rows return. Nothing is cut, only deferred to the zoom that can carry it. A selected module boundary bundles its crossing ink: one line per far module and direction, its count on the line, in place of the hairball; the boundary's inside keeps its own lines, and hovering a block inks that block's own wires. The data chart also keeps the search plate top-right, as every altitude does (`/` focuses it).

### Chart lines (dependency altitude)
Hairline ink-line strokes (1.1px), arrows pointing the way change travels (dependency → dependent). Kind is dash grammar: normal solid, dev `6 4`, build `2 3`, and the focus panel's rows name the cargo tables that own them (`(dev)`, `(build)`, spelled the way `cargo tree` spells them). Manifest events take flare: solid 1.4px for added/bumped, dashed `9 5` at 0.65 opacity for removed, with mono edge labels in flare (`added`, `removed`, `1.0 → 1.2`). Edges draw only for the selection.

### The Self-Stating Chart (legends retired 2026-08-24, user decision)
The "Reading this chart" plates are gone. A legend was a manual standing beside an instrument, and most of its lines restated what the paper or common sense already said (`top-level data`, `secondary data`, a block is a file). The teaching moved onto the marks themselves, where the reader's question actually lands:

- **The rings caption their own hops.** `1 hop`, `2 hops`, `4+ hops` engraved above each guide in the data voice with a paper halo (`.ring-caption`), the way an atlas captions a parallel; the caption is positioned by transform so it glides on the same 400ms beat as the guide when the outer band expands, goes full ink with a hovered or selected ring, and is never a pointer target — the hairline under it stays the control.
- **Hover words carry the rest.** Every mark's `title` is its own sentence, in the plate's plain vocabulary: a star says its name, hop, state, `used by n crates`, and the one gesture no mark can show (`ctrl-click adds it to the selection`; the focal star says `click again to deselect`); a uses edge says `DataModel uses Frame · 12 references`, so a drawn count always has a findable subject; a block's fold row says where its folded items went; a root data block says `a root — no type holds it`; a diff letter says its git meaning. Progressive disclosure at the mark, never a panel beside it.
- **Keys are taught where they act.** `/` rides every search placeholder, `f` is the zoom controls' fit button, `n · p walk` sits beside the changes list those keys walk, and `esc` / `←` `→` are the browser's own grammar and need no teaching.
- **The survey's limits** rest behind one fold at the data cartouche's foot — "what the survey cannot read", a soft-ink summary over the survey's own words (the unresolved census, then `walk_notes` for the holding ink and `CodeGraph::notes` for the dashed uses ink). No chrome paraphrases them.

**The No-Manual Rule.** No surface may exist whose job is to explain another surface. A mark that needs a sentence carries the sentence in its own hover words; a state is written beside the mark that has it; a gesture is taught at the control it acts on. What cannot be taught in place is a defect in the mark, not a gap for a legend to fill. (This generalizes the retired No-Second-Voice rule; the Line-Budget and Key-Only-For-Drawn-Marks rules retired with the plates that needed them.)

### Inputs / Search
- **Style:** the plate itself as an input — square, paper, ink frame; mono 11px; ink-soft placeholder carrying its own shortcut hint ("find a datum…   /"); native search-cancel button hidden.
- **Focus:** the app-wide focus ring — 1.5px solid ink outline, 2px offset; no glow.
- **Results:** a plate popover of rows, hover/active `bg-ink/5`; arrows move, Enter selects, Escape clears. The data search runs over marks — this altitude's places are types and statics: a row is its keyword and name with `src/analyze/manifest.rs:67` right-aligned in soft ink. Prefix matches rank first, then by how much of the workspace leans on the hit. The dependency search rows carry the star mark, the crate name, and `v1.0.229` for externals.

### Buttons / Rows / Toggles
No filled button exists. Actions are typographic: mono links with underline-offset (ink-soft rising to ink on hover), full-width list rows whose hover is a 5% ink wash, and the two segmented toggles (`dependencies`: depends on / used by / reverse deps; `references`: uses / used by / both) — small mono segments where the active segment wears a 1px ink border, never a fill. Module labels and fold marks are bare buttons: the mark itself is the control, with no button chrome.

### Navigation
Navigation is the chart, at two altitudes joined by the cartouche's altitude ladder and by one descent link in the dep chart's focus panel: a workspace member offers "its data ↓" to its own frame on the data chart (`/data/mod/:name`). The link works because both altitudes now key a crate on the **cargo package name**, and only because of that. Until 2026-08-24 the panel offered "its files ↓" to the code map's crate sheet, and it pushed a crate name the survey had never heard of: cargo names the package (`slope-cli`) while rust-analyzer names the target it resolved (`slope`, this workspace's renamed bin), so the sheet answered `No crate named "slope-cli" in this survey.` and nobody noticed. The survey now labels every file with the member whose directory owns it on disk, so the two altitudes say the same word for the same crate — and a descent link that silently selects nothing, which is worse than the ladder, is not something this build can quietly ship again.


At the **dependency altitude** (`/dep`, where `/` lands), the crate under review sits at the center and every ring outward is one dependency hop; stars never move. Clicking a star pushes `/dep/crate/:name` and draws that crate's edges; clicking the selected star (or back / Esc) returns to `/dep`. Ctrl/cmd/shift-click builds a multi-selection (`/dep/crate/a+b`), clicking a ring line selects the whole ring (`/dep/ring/:hop`), and the dependencies toggle draws one reading of the selection's edges.

Every focus at either altitude is a URL — there is no navbar or sidebar; the URL, the back button, and the panel's breadcrumb are the trail, and `←` / `→` step back and forward along it from the keyboard, on either chart.

At the **data altitude** (`/data`), every shape the workspace keeps seats as a block inside its module's frame, roots at module level and held state nested inside its holder. Clicking a mark selects it: the selection is a URL (`/data/mark/:..path?item=`) and a *reading*, never a quotation — the block was already quoted whole. The chosen block wears the app's focus ring, everything a change to it could reach keeps full ink with its wires, folded ones inking back in, and every stranger recedes to a lighter pressure over the same 400ms beat. Every uses edge touching the selection inks in and *stays* inked, and the block at the far end of each reads a step behind the blast radius rather than receding with the strangers — the two families still never merge, because a body is not a shape change, so a uses neighbour is drawn beside the radius and never counted in it. Hover is the passing reading; selection is the one that survives the cursor leaving, which is what following an edge to its other end requires. Nothing moves, and the camera holds still for any selection the glass can already show.

A **module boundary is selectable too**: clicking a frame's border, or the label chipped onto it, selects the module (`/data/mod/:..module` — the crate, then the module path). Everything inside the boundary keeps full ink whatever module inside it was written in, everything one hop across the line reads a step behind — both families at once, because what crosses a module boundary is what a reader came to the boundary to read — and every other module recedes, its frame with it. The frames the chosen one sits inside never recede; they are the paper it stands on. Two hops would be the whole chart again. There is no sheet: a module is a place on the paper, and the paper is already saying it. The mark at the border's other end — `−` while the module is drawn, `+` once it is folded — takes the whole module off the paper and leaves one counted row (`+ 21 items`) inside the boundary that stood there, however deep the nesting. A fold is a **re-layout**, not a reading — the chart is drawn again around what is left — which is why selecting and folding are two marks on the border and never one gesture; the fold is view state kept per session, the selection is the URL.

Two readings ride the cartouche: `references` (uses / used by / both) and `visibility` — a four-stop slider, `pub` / `pub(crate)` / `pub(super)` / `all`, with `n off` at its foot (2026-08-25, user). `references` reads **against an anchor** (fixed 2026-08-25, user): the same hairline is one type's use and another type's users, so direction can select edges only once the chart knows which mark the reader has in hand. The anchor is whatever is in focus — the **selection** (blocks and hairlines light in the chosen direction together), **hover** the same way round, and on the resting plate the **diff**, where `uses` rests what the changed declarations lean on and `used by` rests whose code leans on them. With neither a selection nor a diff, all three readings draw every reference, because a reading with nothing in focus has no direction to take. The first build had no anchor and thinned by a per-mark quota of two instead, so a reader moving the switch saw nothing move. The visibility slider reads the keyword each declaration writes, not what a chain of private modules leaves reachable; that caveat is the label's hover words rather than three words of plate, since the scale underneath already says which alphabet it reads. A type whose every holder the reading leaves off does not read as a root — it stands, and its tier sentence says which reading hid the holder. Naming a declaration in the search widens the reading to the stop that draws it, so a search never lands on a sheet that declines to show what was asked for; a selection URL kept from a wider reading opens a sheet naming the rung it is written at, with `draw it`. A selection sheet on the right column names the mark and gives its relations **two headings, not six** (2026-08-25, user): `Used by`, then `Uses`, then what the type itself offers — `Implements` and `Methods` (2026-08-24, user). `Held by`, `In the contract of`, `In the API of`, `Holds`, `Used by` and `Uses` were six names for two directions and read as six unrelated questions; being held, being named in a contract, being named in an API and being used by a body are four kinds of one fact. So the heading says the direction and the row's own word says the kind — `owns`, the wrapper's word (`Vec`, `Arc`, `dyn`), `owns · off` for a holder the visibility reading left off the paper (the row quotes that holder's own source, because a holder with no block is still a holder), `signature` for a free declaration whose signature names it, `API` for a type whose method rows do, and `12 refs` for a body. One row per end, however many ways that end reaches: a neighbour that both names the type in its API and leans on it from its bodies is `API · 13 refs` on one row, not the same name engraved twice. Inside a heading the order is the strength of the claim — structure first, then signatures, then bodies heaviest first. `Implements` is one row per **hand-written** trait impl, wherever in the workspace it is written, quoting the trait as the header writes it (`From<Option<ast::Visibility>>`), naming where the contract is written when the workspace declares it, and taking the flare with `added` / `removed` when this epoch made or broke the promise; a derive is not here, because a derive stands in the type's own source. `Methods` is one row per method — its keyword, its name, its `A` when the epoch added it, the contract it answers where a trait asked for it, and — in the row's hover words, since 256 pixels of mono is a name and not a signature — its signature as written followed by the file and line the impl block is in. The type's own methods read first, then the ones a contract asked for, gathered under their promise. Neither list is on the paper: the sheet is a list. Under them the blast radius in words (`a shape change here reaches 9 more types upstream.`). An empty `Used by` is now the whole verdict a reviewer deletes code on, in one sentence — `nothing in the workspace reaches it.` — where four bespoke sentences used to split it by which of the six headings was empty. A row the chart draws a block for is a link that re-centres the chart on it. A row it draws no block for — a trait, a method, a free function — opens as a **quotation** instead (2026-08-24, user): the item's own source, lexed into the same token palette a block's rows carry, on a plate that stands to the sheet's left with a gutter counting from the item's first line in the real file, while the row that asked for it keeps a 2px ink edge and its ink so the plate is never loose from the row. A method is quoted *inside* its `impl` or `trait` block — header, method at its own indent, closing brace — because the span of an associated item holds none of its header and `fn edge_style(self, …)` alone is neither rust nor placeable (2026-08-25, user); the lines between two quoted blocks are not carried, and the gutter marks the jump with `⋮`, never with a count of them. The reviewer's next move after reading such a row was always to go and read the code, and it was the one move the chart made them leave for. The quotation is a URL like every other focus (`peek=<file>@<label>` beside the selection's own `item=`), so the back button closes it and Escape steps out of the quotation before it steps out of the selection; `close ×` says so in words. Every resolved name inside the quotation is a link — to that datum's block where the chart draws one, to its own quotation where it does not — so reading the code is the same move as reading the chart. The row's own hover words still name the file and line, and the header's `src/api.rs:67` locator still states where the selected mark itself is written. Escape, bare paper, or the selected block clicked again deselects; `f` refits, and `←` / `→` retrace history as everywhere. The survey is fetched once behind the altitude's own gate, so stepping out to the crates and back never re-runs rust-analyzer.

**The Selection's-Ink Rule.** On the dependency chart, edges are drawn only for the current selection; the resting chart is stars on an engraved ground, never a hairball. The data chart does draw its edges at rest, because coupling between types is the thing that altitude exists to show — but it draws a *reading* of them, not all of them. Its cartouche's `references` toggle names the reading (`uses` / `used by` / `both`), and under either anchored reading a block rests only its two heaviest edges and gives up the rest to a hover. Drawing every aggregated edge was the same hairball this rule forbids one altitude up: a hundred curves and a scatter of counts attachable to none of them.

**The Kept-Ground Rule.** Focus changes the focused thing, not its neighbors, and deselecting never moves the camera. The reader's mental map is never repaid with a re-layout, and the one gesture that does re-lay the paper — folding a module — says so by being its own mark on the boundary, never the selection's side effect.

**The Drawn-Fold Rule.** Whatever is elided says so in words and stays reachable: a folded module leaves `+ 21 items` on the boundary it stood in, a standing mark's folded fan-in writes `held by n types`, an undrawn end is counted as `named by n signatures` / `used by n bodies` and then listed by name on the sheet, the edge reading gives the folded edges back on hover of either end, and lists chunk with "show all n". Nothing is silently cut. The converse binds too: a count is drawn *only* where something is hidden, so a container whose contents are on the paper states no tally.

**The Quoted-Source Rule.** Where the workspace's source, cargo's output, or the VCS's status answers the question, the interface quotes it rather than re-encoding it: a declared row is its own source text, a visibility is `pub` or `pub(crate)`, a kind is `fn` or `struct`, a cargo table is `dev-dependencies`, a location is `path:line`, a change is `M`. The atlas draws what has no textual form and quotes everything that does. A reconstructed signature, an alphabetized derive list, a paraphrase of a doc comment — each is a lossy copy of something the file already says, and none of them ships.

**The Plain-Words Rule.** Every user-facing string is simple tech English: plain, direct, no metaphor and no marketing register. States are counts and nouns — `37 references in 6 files`, not "×37 references arrive from 6 containers"; `No references.`, not "a door no one has opened"; `loading…`, not "reading the item's body…". A thing is called what the reviewer's tools call it: a file is a file, a directory is a directory, a diff is a diff.

## Do's and Don'ts

### Do:
- **Do** draw every new surface as a plate: paper ground, 1px ink border, inner hairline, square corners, 16px side padding.
- **Do** write every state and every elision in words, and reuse the vocabulary rust, cargo, and the VCS already use: `pub`, `pub(crate)`, `fn`, `struct`, `enum`, `trait`, `impl`, `mod`, `(dev)`, `(build)`, `dev-dependencies`, `M`, `added`, `removed`, `1.0 → 1.2`, `3 files changed`, `2 hops downstream`, `254 lines`, `+ 4 private`, `src/api.rs:10`.
- **Do** make every focusable view a URL, and every focus treatment the 1.5px ink outline at 2px offset.
- **Do** derive new marks from the shared grammar (circle = crate, ruled rectangle = file or directory; fill = membership, ring = focus, dash = absence, rays/halo = change), and give every new mark its own hover words — a mark that needs a manual is not finished.
- **Do** quote the source when the question is "what is this?" — with a line-number gutter counting from the item's real first line, no wrapping, selectable text, and a counted fold on anything cut.
- **Do** give any text engraved on the chart a paper halo.
- **Do** keep the data altitude's two edge families apart: solid is holding (the declaration keeps the other end), dashed and lighter is implementation coupling (a body leans on it). The arrowhead rests on the dependent in both, and a body dependence is never counted in a shape change's blast radius.
- **Do** honor `prefers-reduced-motion`: any authored motion must have a static equivalent that carries the same information.

### Don't:
- **Don't** let #a54c06 mean anything but CHANGED and its blast radius — no amber accents, hovers, or branding.
- **Don't** introduce a second color outside a quoted row, a gradient, a glow, or a shadow beyond the single plate contact shadow. Inside a quotation, use the documented token palette and nothing else.
- **Don't** round a corner. 0px everywhere, including third-party overlays (re-rule them as plates).
- **Don't** use hairline ink (#949dab) or faint ink (#b3bac4) for text; they are for lines and ghosts. Line numbers and counts are text — they stay soft ink.
- **Don't** move the camera except for a focus, the first paint, or an explicit refit; deselecting never moves it.
- **Don't** encode meaning in color where words can carry it, outside a quoted row and the one exception the Kind-Color Rule names. Inside one, color carries token class and nothing else. Item kind is a keyword; pub/private is the `pub` keyword and ink pressure.
- **Don't** uppercase or track a keyword, a path, a directory, a crate name, or a VCS status letter. They are written the way the tool writes them.
- **Don't** ship a dark theme surface; the dark "field edition" is deferred by explicit user decision and must be designed as its own plate, not inverted.
- **Don't** merge the two edge families into one reading, or re-use dash on the solid family to encode which wrapper a walk met — the wrapper writes its own word on the line (`Arc`, `&`, `dyn`), and dash is spoken for.
- **Don't** cap a data block's rows. A block draws its whole declaration (revised 2026-08-20, user decision); the only counted line at its foot is the chart's own fold, `held by n types`.
- **Don't** add glyph icon sets; the only pictures on the page are the mark grammar (stars, blocks, frames) and hand-drawn SVG line samples.
