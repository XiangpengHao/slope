---
name: Slope — Workspace Atlas
description: A cargo workspace charted as an engraved star atlas — ink on paper, where only change takes color, and every definition is quoted from the source.
colors:
  paper: "#f6f4ed"
  ink: "#23303c"
  ink-soft: "#5a6574"
  ink-line: "#949dab"
  ink-faint: "#b3bac4"
  flare: "#a54c06"
  tok-kw: "#1c4f8b"
  tok-fn: "#6b3fa0"
  tok-sum: "#6b3fa0"
  tok-type: "#0f6a6a"
  tok-str: "#9c2f4a"
  tok-num: "#8a2f7a"
  tok-doc: "#2f6b45"
  tok-comment: "#5f7060"
  tok-punct: "#4b5563"
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
  code-line:
    fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.55
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

Slope draws a cargo workspace the way a nineteenth-century plate draws the sky: engraving ink on warm paper, hairline constellation lines, a cartouche for the title block, and a legend that names every mark in words. It deliberately refuses the glowing graph-dashboard with sidebar filters. The whole page is one material — ink at varying pressure on one sheet of paper — and the chart rests as a monochrome engraving until change appears.

The atlas has three altitudes of that one plate: the dependency chart (`/dep`, crates as stars on rings of hops), the code chart (`/code`, files as blocks seated inside nested directory frames), and the data chart (`/data`, the workspace's state — every struct, enum, union and static, whatever its visibility — tiered into roots and the blocks nested inside them, inside module frames, with holding and body-dependence edges between them). The altitude ladder — a `dependencies · code · data` line in every cartouche, the current rung engraved solid, the others links — is the only navigation between them. All three share the plate, the legend discipline, and the URL-per-focus rule; the code altitude adds directory frames, file blocks, reference ties, and the definition plate, and the data altitude adds module frames, its two edge families, the nested state block and the root's widened left edge, without adding a second material.

A fourth altitude stood between the last two until 2026-08-24: the surface chart (`/surface`), which drew every contract the code publishes — types with their method bands, free functions, traits, consts and aliases — as blocks in the same module frames, with interface coupling solid and body dependence dashed. The user removed it: it read the same types the data chart reads, one rung apart, and the questions it answered were the definition plate's. Its grammar did not die with it — the module frames, the ownership seating, the quoted rows, the two inks and the fold machinery are the data chart's now, and this record keeps its rules wherever they still describe what ships.

The one thing the drawing never does is paraphrase the code. Where the workspace's source, cargo's output, or the VCS's status already answers a question, the interface quotes it: an item's definition is its own source text, syntax-highlighted, on a plate; a kind is its rust keyword; a location is `path:line`; a changed file is `M`. The atlas draws what has no textual form and quotes everything that does.

Density is instrument-grade: small mono type, tight tracking on the few spaced-caps headings, thin rules. The interface furniture (cartouche, legend, search, toggles, focus panel) floats over a full-bleed chart as double-ruled paper plates; nothing is a "card" in the app sense. Motion is nearly absent — the authored moments are the loading constellation drawing itself in and the 400ms camera/mark glide; both stand down under `prefers-reduced-motion`.

The build is light-theme only by user decision; a dark "field edition" of the same plate is deferred, not implied. The designed surface is the desktop; narrow layouts ship and stay usable but are explicitly descoped (see Layout).

**Key Characteristics:**
- One material: engraving ink on paper; no gradients, no glass, no glow.
- Color is state everywhere except inside a code pane: the page is monochrome except the flare amber, which means CHANGED (and its blast halo) and nothing else, ever. Inside a code pane, color is token class, and it stops at the pane's frame.
- Every state is named in words, and where a state has a name in rust, cargo, or the VCS, that name *is* the words: `pub(crate)`, `dev-dependencies`, `M`, `3 files changed`, `+ 4 private`.
- A count is drawn only where something is hidden. A fold counts; a container whose contents are on the paper does not restate them.
- Every focus is a URL (`/dep`, `/dep/crate/:name`, `/dep/ring/:hop`, `/code`, `/code/crate/:name`, `/code/file/:..path?item=`, `/data`, `/data/mark/:..path?item=`, `/data/mod/:..module`; `/` redirects to `/dep`, the rung a review starts on); the back button retraces the review trail.
- Square corners everywhere; depth by ruling, not by shadow.
- Three altitudes, one grammar: the dependency chart draws edges only for the selection; the code and data charts draw ties for the reading their shared toggle names (the data chart says plain same-module ownership as nesting, draws only what nesting cannot say, and thins its body dependences to the reading). None ever draws the whole graph at once.

## Colors

An ink-pressure ramp on one paper ground, one reserved signal color, and one token palette that lives only inside a code pane.

### Primary
- **Flare Amber** (#a54c06): the one color on the chart. It means CHANGED — flare rays on a changed star, the fading blast-radius halo on downstream stars, manifest-event edges and their labels (added / removed / version bumps), the `M` on a changed file at either altitude, the structural diff's ink on the data chart — the `A`/`M`/`D` letters and the diff-touched block's own frame, the `+`/`−` row marks, added and removed edges with their words — and the counts that state a change (`3 files changed`, `7 added · 13 removed`). It appears nowhere else: never decoration, never emphasis, never brand.

### Neutral
- **Paper** (#f6f4ed): the page ground, plate backgrounds, code-pane ground, the fill of external-crate open circles, and the halo stroke behind any text engraved on the chart. The browser chrome the app doesn't draw (selection, caret, scrollbar) is tinted to stay on the plate.
- **Engraving Ink** (#23303c): body text, solid star fills, quoted source, plate borders, focus rings, selected underlines, and the selection's "uses" ties. The darkest pressure; there is no black.
- **Soft Ink** (#5a6574): secondary text — the keyword in front of a name, external crate names, counts, gate counts, tie counts, code-pane line numbers, attributes inside a code pane, ghost (removed) outlines, muted labels.
- **Hairline Ink** (#949dab): constellation lines and reference ties, ring guides, file-block and directory frames, the left rule of an in-place quotation, thin rules inside plates, hover underlines, minimap marks. Never used for body text.
- **Faint Ink** (#b3bac4): ghost outlines only; the lightest pressure the plate allows.

### Code pane tokens
Only inside a quotation, and only as token class: the definition plate's code pane, and a data mark's quoted rows (2026-08-19, user-confirmed — those rows are sliced source, so they carry the pane's palette and it stops at the block's frame). Each clears 4.5:1 on paper, and none sits in the flare's hue, so amber keeps its exclusive meaning even on a pane's own gutter.

- **Keyword** (#1c4f8b) — every rust keyword.
- **Fn / macro name** (#6b3fa0) — the name in a `fn` declaration, and any macro.
- **Type / lifetime** (#0f6a6a) — uppercase-initial names, `'a`.
- **String** (#9c2f4a) — string, char, and byte literals.
- **Number** (#8a2f7a) — integer and float literals.
- **Doc comment** (#2f6b45) — `///`, `//!`, `/** */`.
- **Comment** (#5f7060) — every other comment.
- **Punctuation** (#4b5563). Attributes take soft ink whole, `#[derive(...)]` included: an attribute reads as one unit.

### Named Rules
**The Kind-Color Rule.** Inside a data block — the one place at any altitude where a type's kind is drawn rather than only written — a struct's or union's name is type-teal, and an enum's is the palette's purple (#6b3fa0). It is a second reading of what the keyword in front of the name already says, taken because at chart zoom the name is the run that carries. Nothing else on the chart takes a kind color, and no kind color leaves a block.

**The Flare Monopoly Rule.** #a54c06 exists only as CHANGED and its consequences (blast halo, manifest events, the structural diff's letters, row marks, event edges, and diff-touched frames). No other element may take it. A code pane is a quotation and carries an editor's token palette; no highlight color may leak outside a pane, and amber keeps its exclusive CHANGED meaning everywhere, pane gutters included. Remove every changed crate and the chart must read as a pure monochrome engraving.

**The Words-Not-Color Rule.** Every state the chart can draw is also written out. Where the state has a name in rust, cargo, or the VCS, that name is the words — lowercase, mono, spelled the way the tool spells it: `pub`, `pub(crate)`, `(dev)`, `dev-dependencies`, `M`, `3 files changed`, `2 hops downstream`, `+ 4 private`. Invented uppercase abbreviations (EXT, DEV, GATE, `12 L`) are retired. Color reinforces; it never carries meaning alone.

**The Ink-Pressure Rule.** Grays are not arbitrary: ink → ink-soft → ink-line → ink-faint is a pressure ramp with fixed roles (text → secondary text → lines → ghosts). Do not invent intermediate grays; mixes are expressed as `color-mix(in srgb, var(--color-ink) N%, transparent)` over paper.

## Typography

**Chart Font (furniture):** EB Garamond (with Georgia, "Times New Roman", serif) — 400, 400 italic, 600; self-hosted woff2.
**Data Font:** JetBrains Mono (with ui-monospace, SFMono-Regular, monospace) — 400, 500; self-hosted woff2.

**Character:** The serif is the engraver's lettering — spaced small caps for plate titles and section headings, and nothing else. The mono is the surveyor's data hand, and it holds every fact: crate and file names, directory names, item rows, keywords, versions, counts, state words, locators, and quoted source. The two voices never swap jobs.

### Hierarchy
- **Chart Title** (Garamond 400, 19px, uppercase, 0.18em tracking): the cartouche's workspace name; error-plate titles at 17px.
- **Chart Heading** (Garamond 400/600, 11–13px, uppercase, 0.18–0.26em tracking): plate section headings — "Reading this map", "Used by (n)", "Depends on (n)", the focus plate's "Used by" / "Uses" columns, the toggle caption.
- **Code Line** (Mono 400, 12px, 1.55 leading): quoted source inside a code pane; its gutter sets at 10.5px in soft ink.
- **Data Name** (Mono 500, 10.5–11.5px): crate, file, and item names on the chart and in lists; focal names at 15px semibold in panels.
- **Data Body** (Mono 400, 10–10.5px, relaxed leading): facts, counts, legend prose, the diff line, locators, item rows.
- **Data State** (Mono 400, 8.5–9.5px): keywords in front of a name, counts, `M`, `+ 4 private`, `+ 12 more lines`, gate and block meta at 8.5px. Lowercase, untracked; tracking is reserved for the few uppercase runs that remain (the altitude ladder, breadcrumb links, and "show all n" actions at 0.1–0.14em).

The whole ramp, so nothing drifts off it: 8.5, 9, 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13, 15, 17, 19px.

### Named Rules
**The Two-Voices Rule.** EB Garamond spaced caps for chart furniture — plate titles and section headings; JetBrains Mono for everything that is data. Garamond never carries a count, a state, a reference row, a locator, or a sentence about the data. Pub/private is weight and pressure, not a new voice: pub names are ink at 500, private names are soft ink at 400.

**The Spaced-Caps Rule.** Uppercase always carries letterspacing (0.1em minimum), and only furniture may be uppercase at all. Rust keywords, cargo table names, file paths, directory names, crate names, and VCS status letters are never uppercased and never tracked — they are written the way the tool writes them. Untracked uppercase does not exist on this plate.

**The Paper-Halo Rule.** Any text engraved directly on the chart (star labels, directory names, crate names, tie counts, edge labels) wears a paper halo — `paint-order: stroke` with a 3–5px paper stroke, or the stacked paper text-shadow — so words stay legible where lines cross. Text inside a plate never needs one.

## Layout

The chart is the page: a full-bleed `h-dvh w-dvw` canvas with `overflow: hidden`, dressed as the plate (paper ground, faint ink dot grid at 16% ink). Furniture floats over it in a fixed choreography — cartouche top-left (legend beneath it at the code altitude, bottom-left at the dependency altitude), search and toggles top-right, zoom controls bottom-right, focus panel right (desktop) — each layer `pointer-events-none` with interactive plates opted back in.

- **Scope:** the desktop is the designed surface; mobile was descoped by explicit user decision. Narrow layouts (< 640px) ship and remain usable — chrome stacks vertically, the legend starts folded, panels dock to the bottom at max 44dvh — but they are a serviceable fallback, not a designed composition. Do not derive system rules from the narrow layout.
- **Chrome insets:** 12px page margin (`p-3`). Fit-view reserves the furniture's space so the chart centers in what remains — dependency altitude desktop: 52px top, 284px left, 20px bottom, 20px right rising to 330px when a panel is open; code altitude desktop: 56px top, 284px left, 24px bottom, 24px right rising to 330px when a crate sheet is open. The dependency fit leaves a 12% breathing margin (fit × 0.88); the code fit leaves 6% (fit × 0.94), because its blocks are rectangles that already carry their own padding.
- **Legibility floor:** fit-view never zooms below a floor — dependency chart 0.7 focused / 0.22 overview; code map 0.22. Past the floor the chart holds readable scale and pans, seating the focal point mid-frame.
- **Camera discipline:** the camera moves only for a focus, the first paint of a fresh session, or an explicit refit (`f`). Deselecting never moves the camera, and a remount is a return: the code and data charts keep their pan and zoom in session state, so coming back from a definition plate (by any path, back button included) restores the camera exactly as the reader left it (2026-08-19). Every move is one 400ms glide (`cubic-bezier(0.65, 0, 0.35, 1)`), 0ms under reduced motion; marks, ring guides, and re-inked edges travel on the same beat so plate and camera move together. One amendment (2026-08-21, user decision): selecting a mark the glass cannot show — off the viewport, or below reading zoom — glides the camera to it, because a selection the reader cannot see is not a focus; a selection already legible moves nothing. It began on the data chart the day a row's held type became a link to a block that can stand anywhere on the paper.
- **Disclosure:** the code chart budgets roughly 320 marks; the deepest directories fold to gates by default, selection into folded ground opens the gates above it, and folding back is one click. Layout is pure and deterministic — the same tree always draws the same map.
- **The definition plate:** the focus plate is its own scrolling page, not an overlay — a 1360px column holding three tracks (264px / free / 264px) with 40px gutters, so the center track has real width for code. The code pane scrolls horizontally on its own and the page never does.
- **Spacing rhythm:** tight and even — 4/6/8/12px steps inside plates; 16px horizontal plate padding; 17px block row height; hairline rules (`border-t` in ink-line) divide sections instead of extra whitespace.

## Elevation & Depth

Effectively flat: depth is drawn, not cast. Plates sit on the paper via their double ruling; the only shadow is a single faint contact shadow under floating plates, and it never grows on hover or state change. Layering is z-index choreography over the chart, not simulated altitude — the ground layer (ring guides, directory frames) under the ties, the ties under the blocks and stars, the floating furniture over all of it.

### Shadow Vocabulary
- **Plate contact** (`0 1px 4px color-mix(in srgb, var(--color-ink) 14%, transparent)`): the sole drop shadow, applied to every floating plate and the re-ruled flow overlays (controls, minimap). Nothing else casts — the map's file blocks and directory frames are ruled, not raised.

### Named Rules
**The Drawn-Depth Rule.** Depth comes from ruling — the double-line frame — not from shadow ramps. There is no shadow scale; the one contact shadow is a constant, not a state.

## Shapes

Square corners everywhere: `border-radius: 0` on plates, inputs, buttons, and the third-party flow overlays (explicitly re-ruled to 0). The signature silhouette is the **double-ruled plate**: a 1px ink border with an inner hairline drawn as inset box-shadows (`inset 0 0 0 2.5px paper, inset 0 0 0 3.25px 45% ink`) so it survives on replaced elements like inputs.

Geometry is vocabulary, and it is spent only where words cannot go: circles belong to the star grammar (crates on the dependency chart), ruled rectangles to the code map's files and directories. Every container rectangle is ruled, never rounded. Item kind is not a shape — it is a keyword (see below).

### Named Rules
**The One-Plate Rule.** Every panel, popover, input, and overlay is the same plate: paper ground, ink frame, inner hairline, square corners. There is no second container style.

**The Keyword-Is-Kind Rule.** An item's kind is its lowercase rust keyword, set in mono in front of its name — `fn parse`, `pub struct Trail`, `pub(crate) mod tree`. Kind is a word, not a shape and not a color — with the single exception the Kind-Color Rule names, a data block's own name, where the color is a second reading of the keyword standing right in front of it. A private item writes no visibility, exactly as rust writes none. This replaces the retired Shape-Is-Kind rule and the item-glyph set it described: a learned shape vocabulary asked the reader to memorize what rust already spells out.

## Components

### Plates (Title Blocks, Legends, selection panels, toggles, error plates)
- **Corner Style:** square (0px).
- **Background:** paper (#f6f4ed); **Border:** 1px ink + inner hairline (see Shapes); **Shadow:** plate contact only.
- **Internal Padding:** 16px horizontal, 8–12px vertical; sections divided by ink-line hairline rules.
- A foldable section carries the class `fold`, whether it is the plate itself (the Legend) or a section inside one. Its summary shows a typographic `–`/`+` marker in ink-soft — no chevron glyph.
- Every cartouche carries the altitude ladder (`dependencies · code · data` in 9.5px tracked mono caps): the current rung engraved solid ink with an underline, the others soft-ink links. Under it the cartouche states its own facts in mono — the census, the diff line (`diff main @ 1a2b3c4 → working copy`), what changed, and which modules it landed in.
- **The Census-Not-Inventory Rule** (2026-08-21, distill). A cartouche states the census of what the chart draws — `98 structs · 35 enums · 1 static`, in rust's own words — and the diff. It does not recite the model's bookkeeping: `55 roots · 63 nested · 16 standing` and `209 body dependences · 127 at rest` were four invented terms defined only in legend prose, and no reviewer decides anything on them. Two count lines is the ceiling. Where a number *is* the chart's own reading, the paper draws it (a root wears the ink edge) and the legend teaches the word — the cartouche does not tally it. This is the same rule the open directory frame already keeps one altitude down: a fact whose subject is on the paper is not restated in the corner.
- A selection panel opens with the breadcrumb ("← whole chart" / "← whole map"), then names the selection. It caps at 44dvh on a phone and at the column height on a desktop, scrolling its lists inside. Long reference lists chunk at 8 rows with a typographic "show all n" action.

### The Star Mark (signature, dependency altitude)
The unit of the dependency chart, reused by chart, legend, search results, and list rows. Radius encodes magnitude — `4 + √dependents × 1.3`, capped at 11px (the shared `star_radius`). Grammar: workspace member = solid ink disc with a 0.6px orbit ring; external crate = open paper circle with 1.3px ink stroke; changed = flare-filled with eight amber rays; downstream = amber halo ring whose opacity fades with hop distance (0.85 − 0.22/hop, floor 0.3); removed = dashed ink-soft outline only; focal = double ink ring with four compass ticks. Node labels: mono 500 name with a paper halo and an underline that appears in ink-line on hover and ink when selected; state words beneath in 9.5px mono — `3 files changed`, `2 hops downstream`, `removed`.

### The Block (signature, code altitude)
The code map draws no stars. A **file** is a ruled block: a paper rectangle with a 1px hairline frame, measured before it is placed so its plate and its box agree to the pixel. Header: the file name in mono 700 at 10.5px and an amber `M` beside it when the file changed since the diff base — and nothing else. It once carried `254 lines · 31 items` too, and because the header was a flex row with the count fixed, the count won the width and the *name* was what got clipped: `chrome.rs…`, `map.rs…`, `ego.rs…`. The name is the one thing the block exists to state, so it is now the only thing that can take the header's width. Body: its loudest items as rows, each written as rust — the keyword and visibility in soft ink (`pub fn`), the name in ink — in three weight tiers by fan-in (700 / 500 / 400), every row a link to that item's definition plate. Foot: the fold's own words in rust's own vocabulary, `+ 9 pub · 5 private`, above a dotted rule. A block whose items all folded away draws one rule, not two four pixels apart.

A **gate** is a folded directory standing in for its whole subtree: one block-shaped button with a 2.5px ink left edge, `▸ views/` on the first line and `12 files · 255 items` on the second. Clicking it opens the directory.

### The Ground (nested directory frames)
The code map's ground layer, under the ties and blocks: one bordered frame per open directory, filled at 2.6% ink so nesting reads as depth without a second material. Nesting means one thing — belongs to. The frame's label band sits on its top border and holds two segments, each measured on its own face: the directory as it is on disk (`▾ views/`, mono 500 at 12px, clicking folds it), then the crate whose sources live under it (`slope`, 10px soft ink, a link to the crate sheet), drawn only where the survey has more than one crate to tell apart. An open frame states no counts: its files are on the paper to be counted, and `18 files · 380 items` on the root, again on `src/`, and again in the cartouche was one fact drawn three times in eighty vertical pixels. The root frame carries the workspace name and never folds. Every segment wears a paper halo.

The layout packs children into shelves aimed at a landscape sheet — files in name order first, then subdirectories — and is a pure function of (tree, disclosure, measured sizes): the same workspace always draws the same map.

### Reference Ties (code altitude)
Every reference between two territories, summed. Quadratic curves bowed toward open paper (16% of length, capped at 52px), width `0.55 + count × 0.13` capped at 2.8px, the arrowhead resting on the **user** — the way change travels.

Which of them rest on the paper is a **reading**, set by the cartouche's `references` toggle — `uses` (default) / `used by` / `both`. Drawing every aggregated tie at rest was a hairball: about a hundred curves over eighteen files, with counts floating in mid-paper that no reader could attach to a particular curve. So each reading anchors on the territories themselves — under `uses` a block draws the two heaviest files it leans on, under `used by` the two heaviest that lean on it, `both` draws every tie unthinned. A folded tie is never cut: it stays in the set and inks in, with its count, the moment the reader hovers either of its ends. Of the *resting* ties the heaviest dozen carry their count in 9.5px soft-ink mono with a paper halo and draw at full weight; the rest draw at half opacity and keep their count until hovered. A file folded inside a gate still shows its ties, ending at the gate that stands in for it. Ties ink in over 400ms (`edge-ink`, opacity only), none under reduced motion. Both world layers sit under the flow viewport, so a tie draws over the district tints and under the blocks — never across a file's own rows.

### The Definition Plate (signature, code altitude)
Selecting an item replaces the map with the definition plate: three tracks on the paper — `Used by` on the left, the plate in the middle, `Uses` on the right — under a breadcrumb (whole map ▸ directory ▸ file ▸ item).

The center plate is a quotation, not a description — and the quotation *is* the plate. There is exactly one frame on the center track: the double-ruled plate is the code pane itself, because a hairline frame twenty pixels inside another frame is a card in a card, not depth. It opens with the locator, `src/api.rs:10` in mono, with an amber `M` when the file changed, set on the page's bare paper. Under it, the item's **own source text**, sliced from the file the survey read: doc comment, attributes, signature, and body, exactly as written, syntax-highlighted by token class (see Colors), on a paper ground inside the plate's own double ruling. A line-number gutter counts from the item's true first line, right-aligned in 10.5px soft ink, `position: sticky` so it holds while the code scrolls under it, and `user-select: none` so a copy takes the code and not the numbers. The code sets at 12px/1.55, never wraps, and scrolls horizontally inside its own frame; the text itself is selectable. A method quoted out of its impl block is given back its own indent and then dedented with its body, so it starts at the plate's left edge instead of four spaces in. A long body is folded at roughly 60 lines past its docs and signature, and the fold counts what it holds back: `+ 42 more lines`. Types, traits, consts, and statics are never cut.

Below the quotation sits what the source cannot show, because it is written somewhere else: the type's associated items, grouped under the impl header they are written under (`impl Vis`, `impl Clone for Vis`, in mono 500), each row `pub fn weight` with `src/api.rs:165` right-aligned in soft ink, and each row a link. A hand-written trait impl with no items of its own is still listed — it is still code someone wrote. Private members are counted, never named: `+ 3 private`.

Selecting a whole **file** gives the same plate with an outline instead of a quotation — a file has no single definition to quote, so it keeps no frame either, and the outline sets on the page's paper under the facts line. The locator is the path, the facts line reads `373 lines · 24 items · crate slope`, and each row is `pub enum DepKind`, its reference count, and its line in the plate's own locator idiom — `87 refs` then `:29`, set apart so the two never read as one run of digits.

The two reference columns are grouped by the file the references are written in: a header naming the file with its total (`37 refs`) over a hairline rule, then rows of keyword + name + count, capped at 3 with `+2 more (5 refs)`. A group is a section of one list, not a plate — nine boxed groups down a column drew nine frames to say what one rule each says. Every row names something; a file's private items are the group's own counted fold, `+ 96 private`, in the same grammar the map's blocks use. (A row named `private items` names nothing, and it appeared in eight of nine groups, usually as the loudest row in each — the column's dominant content was the row that said the least.) An empty column says `No references.` / `No outgoing references.` Every row re-centers the plate on itself.

### The Data Mark (signature, data altitude)
One block per shape the workspace keeps — struct, enum, union — and per static, whatever its visibility: state does not fold at a door, so the data chart has no doors toggle and no `+ n private` row. Each mark is a hairline frame around a quotation. The header: the keyword and visibility in keyword-blue, the name at 700 — type-teal for a product type, the palette's purple for a sum type — and the structural diff's letter in git's own alphabet (`A` added since the base, `M` declaration changed, `D` removed; a file-level change alone marks nothing). The rows are the declaration's clauses, quoted as written: a struct's or union's fields in declaration order, an enum's variants with payloads and discriminants (`File(String, String)`), a static's declared type. Each row is colored by token class the way the definition plate colors its source, with the one run that names a workspace mark in bold; a plain type name is from outside the workspace, has no mark, and so has no line drawn to it. Each row also wears its own `pub` or `pub(crate)` in keyword-blue where it declares one (2026-08-24, user — a block's header says what the *type* publishes, and a reader deciding what may touch this state needs the field's own answer). There is no method band: a block here is state only, and what a type promises is read on its definition plate, one rung up.

**The bold run is also the way to it** (2026-08-24, user). Where the chart draws the block that run names, the run is a link: click `ItemKind` in `kind: ItemKind` and the chart selects `ItemKind`. It is the same focus the block's own click is — the sheet lifts, the blast radius inks, and the camera glides only where the chosen block was not already legible — so following a type costs one click instead of a search across the paper by eye. The affordance is the block name's own, an underline while the pointer rests on the run and nothing at all at rest: a row whose types were underlined all the time would read as decoration rather than as a declaration. A run with no block to reach stays bold text and nothing more — a type inside a hand-folded module, and a run naming the block it is written in, which is where the reader already stands. This holds wherever a row is quoted: fields, variants, and a static's declared type.

**A block is bracketed the way rust brackets the declaration it quotes** (2026-08-24, user). The head opens with `{` and a line of its own closes it — that closing brace is what makes a long block's end findable, and it is what tells a reader that the blocks nested below it are state this one owns, not more of its own declaration. A static has no body to bracket, so its head ends in the `:` that introduces the line under it. A declaration with no rows at all brackets nothing: inventing `{}` would be guessing at a unit struct. The braces are punctuation ink, like every other bracket inside a quotation, and the far edition drops them with the rest of the rows. **The rows stand in from the edge** the way rust indents a body, and the closing brace does not — which is what makes the block's end findable at a glance. The indent is 12px, not rust's four spaces: at 10px it reads as an indent and costs a block twelve pixels of width instead of twenty-four. The diff's row marker lives in that gutter, so a `+` row and the untouched row under it start their text on the same column; before the indent the marker pushed its own row nine pixels right and a woven block read as ragged.

**No row caps** (revised 2026-08-20, user decision). A block draws its whole declaration — every field, every variant — and the layout is handed that height. A declaration read eight rows deep is a declaration half read, and a reader who has to select a block to finish its shape is reading the chart twice, so `+ 4 more fields` and the windowing that produced it are gone. Selecting a block opens nothing, because nothing was closed. `held by n types` is the one counted line left at the foot, and it is the chart's own fold — ink it will not draw — not a row the block is holding back.

The ground is **modules, not directories**: one frame per workspace crate, and inside it one frame per module nested exactly as rust's modules nest, each labeled in rust's own words with the last segment alone on the border because the paper's own nesting says the rest (revised 2026-08-20, user decision — one flat level read as a lie about the code, since `views::data` and `views::codemap` are not one module). Prose that names a frame away from the paper — the cartouche's insight line, a sheet's fold row — spells the whole path (`views::data`), because three modules answer to `mod map`. Within a frame, types seat under their one heaviest same-frame `Owns` holder, ownership depth as layers, so an owns edge is usually a short line between neighbours; a type never seats outside its own module, so cross-frame ownership stays drawn ink instead of being arranged away.

The chart's one organizing move is the **tier** (2026-08-21, user decision). Top-level data is a **root** — a static, or a type no other workspace type keeps in a field (`Owns` or `Shares`; a borrow is a view, not a hold) — and every root wears the gate's 2.5px ink left edge, the static's own mark widened to every block a chain of holding begins at. Secondary data is drawn **inside** the block of the type that owns it hardest: a hairline rule closes the rows and the owned blocks shelve on the clear paper under it, recursively, so reading the tier is reading the paper's own nesting. Plain same-module ownership is therefore never a line — the containment is the edge. What stays drawn ink, solid with the wrapper's word, is exactly what nesting cannot say: sharing (a handle has no single container, so shared state stands beside its holders), borrowing, second holders, cross-module ownership, a ring of mutual owners, and the diff's added and removed relations. A type held by more than three types is vocabulary: it stands, its fan-in folds to `held by n types` on its own foot, and hover or selection inks the lines back in. That left edge is the last pair of moves left in the plate vocabulary: dashed is a ghost, flare is diff ink, a left edge is a root, line weight is the wires'.

**Exactly two inks, and one direction rule: the arrowhead rests on the dependent.** *Solid* is holding — the dependent's own declaration keeps the tail — and draws at one pressure whatever the wrapper is, because the wrapper writes its own word on the line (`Arc`, `&`, `dyn`, `implements`) and no word at all is plain ownership. Dash used to encode which wrapper it was; dash is spoken for now. *Dashed and lighter* (`5 3` at 0.45) is implementation coupling: the dependent's *body* leans on the tail — a call, a name written inside a function, its ends climbed to the type that declares them — carrying its count, lighter because a rewrite can take it back without anyone else noticing. The two families draw to opposite sides of a block and never merge. The `references` toggle thins the dashed family to two per mark at rest, labeling the heaviest dozen; a folded wire is never cut — it inks back in on hover of either end, and stays inked for as long as either end is selected.

Nothing folds by count (revised 2026-08-21, distill). Every mark is drawn, however many there are. The retired 200-mark budget (`MARK_BUDGET`) folded each frame's quietest into a `+ n more items` row: it hid marks by a number nobody set, reflowed the whole chart when the threshold moved, invented a row that attracted edges, and left marks a URL could still point at. The folds that remain are the ones a reader asks for — a module folded by hand — and the ones the chart draws a count for in their place.

What has no block here is counted, never cut: `named by n signatures` (free fns, method rows, consts, aliases whose declared surface names the type) and `used by n bodies` (references from code with no mark of its own) live in the block's hover words, and the sheet lists both as rows (revised 2026-08-21, critique — stamped on every resting block they were texture, not signal, and they were never elided ink). The used-by half became real rows on 2026-08-23 (user): an undrawn body is still a nameable item, so `Used by` and `Uses` name every free function, trait and const alongside the drawn types — keyword, count, and a link to its definition, since the chart cannot re-centre on a mark it does not draw. One relation is one ranking: the merged list reads heaviest-first whatever kind the far end is, because the sheet shows eight rows before `show all` and those eight must be the heaviest, not the drawn ones. The sentence that used to count the rest is gone. The one count on a block's foot is `held by n types`, the vocabulary fan-in the chart folded; locators likewise live in the hover words and on the sheet, not on the resting paper. A reading's recede acts on a block's *own paint* — its frame and text — never on its box, because a receded holder can have a lit block nested inside it.

The **structural diff** (2026-08-19) is drawn in this same grammar, against the base edition of every changed file (read syntactically, matched by kind and name — the legend states the method). A diff-touched block wears the flare on its own frame and its letter beside its name; what `M` reads is the declaration's own text, because a rewritten body is the code altitude's news. A removed mark is a **ghost**: the same block dashed, rows quoted as the base wrote them, locator `…:113 (base)`, seated in the frame its path names. An added row wears a flare `+`; a dropped one is quoted from the base, struck, and seated where it stood. A relation the base could not have drawn takes flare with `added` on the line; one only the base had is re-drawn flare-dashed (`9 5`) with `removed`. Diff marks and edges never fold, and while the diff has anything to say the untouched marks rest at 0.62 pressure (hover restores). A clean diff draws none of this, and the chart reads as the monochrome engraving the Flare Monopoly promises.

**The far edition** (2026-08-21, user decision). Zoom is the fold: below reading zoom (0.45, back above 0.55) the chart swaps to a projection of names, tiers and wires — each block draws its keyword and name alone at a size its box can carry, the root's edge and the chosen block's ring hold near-constant screen width, wire words retire while the lines thicken to stay lines, and module names engrave across their own frames in soft ink the way an atlas names a region. The blocks keep their boxes, so nothing moves and every wire lands where it landed; descend — or select, which glides to reading zoom — and the rows return. Nothing is cut, only deferred to the zoom that can carry it. A selected module boundary bundles its crossing ink: one line per far module and direction, its count on the line, in place of the hairball; the boundary's inside keeps its own lines, and hovering a block inks that block's own wires. The data chart also keeps the search plate top-right, as every altitude does (`/` focuses it).

### Chart lines (dependency altitude)
Hairline ink-line strokes (1.1px), arrows pointing the way change travels (dependency → dependent). Kind is dash grammar: normal solid, dev `6 4`, build `2 3`, and the legend names the cargo tables that own them (`dev-dependencies — dashed · build-dependencies — dotted`). Manifest events take flare: solid 1.4px for added/bumped, dashed `9 5` at 0.65 opacity for removed, with mono edge labels in flare (`added`, `removed`, `1.0 → 1.2`). Like the code ties, edges draw only for the selection.

### Legends
One per altitude, each a plate with a `fold` summary in Garamond tracked caps. All three read in one order, and it is a **key strip, not prose**: the drawn samples first, each on its own line beside the one word it names, then a single paragraph carrying the grammar for that whole family, then the counted folds, then the diff's key, then the gestures on a two-column grid, and last the survey's limits behind a nested fold. The code legend carries no glyph key: item kind is a keyword and needs none.

A legend is a key, not a manual — and after the 2026-08-21 distill it is held to that by three rules.

**The Line-Budget Rule.** The legend's budget is *lines*, not words. At 224px of plate in 10px mono a line holds about twenty-eight characters, so every sentence costs four or five lines and four sentences run a plate off the page. Write a sample and one word; put the grammar in one paragraph per family, not one per mark. The retired surface chart's legend went from about eleven hundred words to a key on this rule alone; the chart is gone and the rule it proved is not.

**The No-Second-Voice Rule.** A legend states nothing that is already stated somewhere the reader is looking. Not a control that carries its own label and tooltip — the references and visibility toggles each had a paragraph here restating their button titles word for word. Not what the selection sheet says the moment a mark is picked. Not the absence of a fold ("no row waits behind a count" describes nothing on the paper). And never a paraphrase of the survey: the survey's limits are two lists on the wire (`CodeGraph::notes` for references, `walk_notes` for the holds walk), each altitude prints the ones for the ink it actually draws, and no legend writes prose of its own about them.

**The Key-Only-For-Drawn-Marks Rule.** The converse of the Drawn-Fold Rule. A key for a mark the chart is not drawing is the same dead weight as a count for nothing hidden, so the diff's key — the letters, the row marks, the flare frame — appears only while the diff has something to say. A clean diff draws none of it and neither does the plate.

What a legend must still carry: every fold's terms and how to reach what is folded (the tie reading's per-block bar and the hover that undoes it, what a block's last line counts, where a private item's references went), and the keys nothing on screen advertises (`/`, `f`, `esc`, `←` `→`).

The legend sits directly beneath the cartouche, one stack, and **takes the column's remainder** — no altitude hand-sets a height, because a longer diff line then breaks it. Mechanically: the column is a `flex-col` between `top-3` and `bottom-3`, the plate is `flex min-h-0 flex-col` (shrink-only, so a short key still hugs its own content instead of stretching to the page edge), and `.legend-plate::details-content` is the flex track — without that rule the fold's own content box is unbounded and the plate runs off the page however short the key gets. When the key does run past the plate it scrolls under an **always-visible** thin ink scrollbar (`.legend-scroll`, 6px, ink-line thumb on a 7% ink track): the fold is drawn, never hidden.

### Inputs / Search
- **Style:** the plate itself as an input — square, paper, ink frame; mono 11px; ink-soft placeholder carrying its own shortcut hint ("find a file or item…   /"); native search-cancel button hidden.
- **Focus:** the app-wide focus ring — 1.5px solid ink outline, 2px offset; no glow.
- **Results:** a plate popover of rows, hover/active `bg-ink/5`; arrows move, Enter selects, Escape clears. The code search runs over files **and** items in one list: a file row is its name, an amber `M` if changed, and its directory right-aligned; an item row is its keyword and name with `src/analyze/manifest.rs:67` right-aligned in soft ink. Prefix matches rank first, files before items, then by how much of the workspace leans on the hit. The dependency search rows carry the star mark, the crate name, and `v1.0.229` for externals.

### Buttons / Rows / Toggles
No filled button exists. Actions are typographic: mono links with underline-offset (ink-soft rising to ink on hover), full-width list rows whose hover is a 5% ink wash, and the one segmented toggle (`dependencies`: depends on / used by / reverse deps) — small mono segments where the active segment wears a 1px ink border, never a fill. Directory labels and gates are bare buttons: the mark itself is the control, with no button chrome.

### Navigation
Navigation is the chart, at three altitudes joined by the cartouche's altitude ladder and by cross-links in the panels (the dep chart's focus panel offers "its files ↓"; the code chart's crate sheet offers "its dependencies ↑").


At the **dependency altitude** (`/dep`, where `/` lands), the crate under review sits at the center and every ring outward is one dependency hop; stars never move. Clicking a star pushes `/dep/crate/:name` and draws that crate's edges; clicking the selected star (or back / Esc) returns to `/dep`. Ctrl/cmd/shift-click builds a multi-selection (`/dep/crate/a+b`), clicking a ring line selects the whole ring (`/dep/ring/:hop`), and the dependencies toggle draws one reading of the selection's edges.

At the **code altitude** (`/code`), files seat as blocks inside their directory's frame. Clicking a file pushes `/code/file/:..path` and opens its definition plate; clicking an item row selects `?item=Type::name` and quotes that item's source; clicking a directory label or gate folds or opens it (a view state, not a URL); a frame's crate name pushes `/code/crate/:name`. On a plate, an outline or impl-member row expands its definition in place instead of going one level deeper (view state, not a URL; the row folds it again; middle-click or the pane's footer opens that item's own plate). Escape folds open quotations, then steps up one focus level — item → file → whole map — `/` finds a file or item, `f` refits. Inside a quotation, every resolved reference's name is a link to its definition. Every focus at every altitude is a URL — there is no navbar or sidebar; the URL, the back button, and the panel's breadcrumb are the trail, and `←` / `→` step back and forward along it from the keyboard, on any chart.

At the **data altitude** (`/data`), every shape the workspace keeps seats as a block inside its module's frame, roots at module level and held state nested inside its holder. Clicking a mark selects it: the selection is a URL (`/data/mark/:..path?item=`) and a *reading*, never a quotation — the block was already quoted whole. The chosen block wears the app's focus ring, everything a change to it could reach keeps full ink with its wires, folded ones inking back in, and every stranger recedes to a lighter pressure over the same 400ms beat. Every uses edge touching the selection inks in and *stays* inked, and the block at the far end of each reads a step behind the blast radius rather than receding with the strangers — the two families still never merge, because a body is not a shape change, so a uses neighbour is drawn beside the radius and never counted in it. Hover is the passing reading; selection is the one that survives the cursor leaving, which is what following an edge to its other end requires. Nothing moves, and the camera holds still for any selection the glass can already show.

A **module boundary is selectable too**: clicking a frame's border, or the label chipped onto it, selects the module (`/data/mod/:..module` — the crate, then the module path). Everything inside the boundary keeps full ink whatever module inside it was written in, everything one hop across the line reads a step behind — both families at once, because what crosses a module boundary is what a reader came to the boundary to read — and every other module recedes, its frame with it. The frames the chosen one sits inside never recede; they are the paper it stands on. Two hops would be the whole chart again. There is no sheet: a module is a place on the paper, and the paper is already saying it. The mark at the border's other end — `−` while the module is drawn, `+` once it is folded — takes the whole module off the paper and leaves one counted row (`+ 21 items`) inside the boundary that stood there, however deep the nesting. A fold is a **re-layout**, not a reading — the chart is drawn again around what is left — which is why selecting and folding are two marks on the border and never one gesture; the fold is view state kept per session, the selection is the URL.

One toggle rides the cartouche: `references` (uses / used by / both), the same reading the code map is set to. There is no visibility toggle — state does not fold at a door. A selection sheet on the right column names the mark and keeps its relations strictly apart: `Held by` (nesting first, then the drawn relations), `In the contract of` (free functions naming it, each a link to its definition), `In the API of` (another type's method row), `Holds`, and the dashed family both ways round as `Used by` / `Uses` with `3 calls` or `12 references`. Under them the blast radius in words (`a shape change here reaches 9 more types upstream.`). An empty `Held by` is a four-way truth: a static is `a root`; a mark only signatures name enters through them; a mark only bodies reach says so; and a mark nothing reaches at all says exactly that — the verdict a reviewer deletes code on. The sheet carries the one explicit step further: `open its definition →` to the code plate, which stays the only place source is quoted whole. Escape, bare paper, or the selected block clicked again deselects; `f` refits, and `←` / `→` retrace history as everywhere. The `/code` and `/data` routes share one survey fetch, so stepping between those two rungs never re-runs rust-analyzer.

**The Selection's-Ink Rule.** On the dependency chart, edges are drawn only for the current selection; the resting chart is stars on an engraved ground, never a hairball. The code map does draw ties at rest, because coupling between files is the thing that altitude exists to show — but it draws a *reading* of them, not all of them. Its cartouche's `references` toggle names the reading (`uses` / `used by` / `both`), and under either anchored reading a block rests only its two heaviest ties, states the count on the heaviest dozen, and gives up the rest to a hover. Drawing every aggregated tie was the same hairball this rule forbids one altitude up: a hundred curves and a scatter of counts attachable to none of them.

**The Kept-Ground Rule.** Focus changes the focused thing, not its neighbors, and deselecting never moves the camera. The reader's mental map is never repaid with a re-layout: folding a gate moves the ground it holds and nothing else.

**The Drawn-Fold Rule.** Whatever is elided says so in words and stays reachable: gates write their file counts, blocks write `+ 9 pub · 5 private`, a reference group writes `+ 96 private`, a folded function body writes `+ 42 more lines`, the tie reading states its per-block bar on the legend and gives the folded ties back on hover, lists chunk with "show all n" and `+2 more (5 refs)`, and the legend scrolls under an always-visible ink scrollbar. Nothing is silently cut. The converse binds too: a count is drawn *only* where something is hidden, so a container whose contents are on the paper states no tally.

**The Quoted-Source Rule.** Where the workspace's source, cargo's output, or the VCS's status answers the question, the interface quotes it rather than re-encoding it: a definition is its own source text, a visibility is `pub` or `pub(crate)`, a kind is `fn` or `struct`, a cargo table is `dev-dependencies`, a location is `path:line`, a change is `M`. The atlas draws what has no textual form and quotes everything that does. A reconstructed signature, an alphabetized derive list, a paraphrase of a doc comment — each is a lossy copy of something the file already says, and none of them ships.

**The Plain-Words Rule.** Every user-facing string is simple tech English: plain, direct, no metaphor and no marketing register. States are counts and nouns — `37 references in 6 files`, not "×37 references arrive from 6 containers"; `No references.`, not "a door no one has opened"; `loading…`, not "reading the item's body…". A thing is called what the reviewer's tools call it: a file is a file, a directory is a directory, a diff is a diff.

## Do's and Don'ts

### Do:
- **Do** draw every new surface as a plate: paper ground, 1px ink border, inner hairline, square corners, 16px side padding.
- **Do** write every state and every elision in words, and reuse the vocabulary rust, cargo, and the VCS already use: `pub`, `pub(crate)`, `fn`, `struct`, `enum`, `trait`, `impl`, `mod`, `(dev)`, `(build)`, `dev-dependencies`, `M`, `added`, `removed`, `1.0 → 1.2`, `3 files changed`, `2 hops downstream`, `254 lines`, `+ 4 private`, `src/api.rs:10`.
- **Do** make every focusable view a URL, and every focus treatment the 1.5px ink outline at 2px offset.
- **Do** derive new marks from the shared grammar (circle = crate, ruled rectangle = file or directory; fill = membership, ring = focus, dash = absence, rays/halo = change) and render legend samples with the same components as the chart.
- **Do** quote the source when the question is "what is this?" — with a line-number gutter counting from the item's real first line, no wrapping, selectable text, and a counted fold on anything cut.
- **Do** give any text engraved on the chart a paper halo.
- **Do** keep the data altitude's two edge families apart: solid is holding (the declaration keeps the other end), dashed and lighter is implementation coupling (a body leans on it). The arrowhead rests on the dependent in both, and a body dependence is never counted in a shape change's blast radius.
- **Do** honor `prefers-reduced-motion`: any authored motion must have a static equivalent that carries the same information.

### Don't:
- **Don't** let #a54c06 mean anything but CHANGED and its blast radius — no amber accents, hovers, or branding.
- **Don't** introduce a second color outside a code pane, a gradient, a glow, or a shadow beyond the single plate contact shadow. Inside a code pane, use the documented token palette and nothing else.
- **Don't** round a corner. 0px everywhere, including third-party overlays (re-rule them as plates).
- **Don't** use hairline ink (#949dab) or faint ink (#b3bac4) for text; they are for lines and ghosts. Line numbers and counts are text — they stay soft ink.
- **Don't** move the camera except for a focus, the first paint, or an explicit refit; deselecting never moves it.
- **Don't** encode meaning in color where words can carry it, outside a code pane and the one exception the Kind-Color Rule names. Inside one, color carries token class and nothing else. Item kind is a keyword; pub/private is the `pub` keyword and ink pressure.
- **Don't** uppercase or track a keyword, a path, a directory, a crate name, or a VCS status letter. They are written the way the tool writes them.
- **Don't** ship a dark theme surface; the dark "field edition" is deferred by explicit user decision and must be designed as its own plate, not inverted.
- **Don't** merge the two edge families into one reading, or re-use dash on the solid family to encode which wrapper a walk met — the wrapper writes its own word on the line (`Arc`, `&`, `dyn`), and dash is spoken for.
- **Don't** cap a data block's rows. A block draws its whole declaration (revised 2026-08-20, user decision); the only counted line at its foot is the chart's own fold, `held by n types`.
- **Don't** add glyph icon sets; the only pictures on the page are the mark grammar (stars, blocks, frames) and hand-drawn SVG line samples.
