---
name: Slopify — Workspace Atlas
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

# Design System: Slopify — Workspace Atlas

## Overview

**Creative North Star: "The Engraved Star Atlas"**

Slopify draws a cargo workspace the way a nineteenth-century plate draws the sky: engraving ink on warm paper, hairline constellation lines, a cartouche for the title block, and a legend that names every mark in words. It deliberately refuses the glowing graph-dashboard with sidebar filters. The whole page is one material — ink at varying pressure on one sheet of paper — and the chart rests as a monochrome engraving until change appears.

The atlas has three altitudes of that one plate: the dependency chart (`/`, crates as stars on rings of hops), the code chart (`/code`, files as blocks seated inside nested directory frames), and the data chart (`/data`, types as blocks seated inside module frames, with holding edges between them). The altitude ladder — a `dependencies · code · data` line in every cartouche, the current rung engraved solid, the others links — is the only navigation between them. All three share the plate, the legend discipline, and the URL-per-focus rule; the code altitude adds directory frames, file blocks, reference ties, and the definition plate, and the data altitude adds type marks, holding edges, and the static's root mark, without adding a second material.

The one thing the drawing never does is paraphrase the code. Where the workspace's source, cargo's output, or the VCS's status already answers a question, the interface quotes it: an item's definition is its own source text, syntax-highlighted, on a plate; a kind is its rust keyword; a location is `path:line`; a changed file is `M`. The atlas draws what has no textual form and quotes everything that does.

Density is instrument-grade: small mono type, tight tracking on the few spaced-caps headings, thin rules. The interface furniture (cartouche, legend, search, toggles, focus panel) floats over a full-bleed chart as double-ruled paper plates; nothing is a "card" in the app sense. Motion is nearly absent — the authored moments are the loading constellation drawing itself in and the 400ms camera/mark glide; both stand down under `prefers-reduced-motion`.

The build is light-theme only by user decision; a dark "field edition" of the same plate is deferred, not implied. The designed surface is the desktop; narrow layouts ship and stay usable but are explicitly descoped (see Layout).

**Key Characteristics:**
- One material: engraving ink on paper; no gradients, no glass, no glow.
- Color is state everywhere except inside a code pane: the page is monochrome except the flare amber, which means CHANGED (and its blast halo) and nothing else, ever. Inside a code pane, color is token class, and it stops at the pane's frame.
- Every state is named in words, and where a state has a name in rust, cargo, or the VCS, that name *is* the words: `pub(crate)`, `dev-dependencies`, `M`, `3 files changed`, `+ 4 private`.
- A count is drawn only where something is hidden. A fold counts; a container whose contents are on the paper does not restate them.
- Every focus is a URL (`/`, `/crate/:name`, `/code`, `/code/crate/:name`, `/code/file/:..path?item=`); the back button retraces the review trail.
- Square corners everywhere; depth by ruling, not by shadow.
- Three altitudes, one grammar: the dependency chart draws edges only for the selection; the code and data charts draw ties for the reading their shared toggle names (the data chart's holding structure rests whole, its hubs folded to counts). None ever draws the whole graph at once.

## Colors

An ink-pressure ramp on one paper ground, one reserved signal color, and one token palette that lives only inside a code pane.

### Primary
- **Flare Amber** (#a54c06): the one color on the chart. It means CHANGED — flare rays on a changed star, the fading blast-radius halo on downstream stars, manifest-event edges and their labels (added / removed / version bumps), the `M` on a changed file at either altitude, and the counts that state a change (`3 files changed`). It appears nowhere else: never decoration, never emphasis, never brand.

### Neutral
- **Paper** (#f6f4ed): the page ground, plate backgrounds, code-pane ground, the fill of external-crate open circles, and the halo stroke behind any text engraved on the chart. The browser chrome the app doesn't draw (selection, caret, scrollbar) is tinted to stay on the plate.
- **Engraving Ink** (#23303c): body text, solid star fills, quoted source, plate borders, focus rings, selected underlines, and the selection's "uses" ties. The darkest pressure; there is no black.
- **Soft Ink** (#5a6574): secondary text — the keyword in front of a name, external crate names, counts, gate counts, tie counts, code-pane line numbers, attributes inside a code pane, ghost (removed) outlines, muted labels.
- **Hairline Ink** (#949dab): constellation lines and reference ties, ring guides, file-block and directory frames, the left rule of an in-place quotation, thin rules inside plates, hover underlines, minimap marks. Never used for body text.
- **Faint Ink** (#b3bac4): ghost outlines only; the lightest pressure the plate allows.

### Code pane tokens
Only inside a code pane, and only as token class. Each clears 4.5:1 on paper, and none sits in the flare's hue, so amber keeps its exclusive meaning even on a pane's own gutter.

- **Keyword** (#1c4f8b) — every rust keyword.
- **Fn / macro name** (#6b3fa0) — the name in a `fn` declaration, and any macro.
- **Type / lifetime** (#0f6a6a) — uppercase-initial names, `'a`.
- **String** (#9c2f4a) — string, char, and byte literals.
- **Number** (#8a2f7a) — integer and float literals.
- **Doc comment** (#2f6b45) — `///`, `//!`, `/** */`.
- **Comment** (#5f7060) — every other comment.
- **Punctuation** (#4b5563). Attributes take soft ink whole, `#[derive(...)]` included: an attribute reads as one unit.

### Named Rules
**The Flare Monopoly Rule.** #a54c06 exists only as CHANGED and its consequences (blast halo, manifest events, the `M` marker). No other element may take it. A code pane is a quotation and carries an editor's token palette; no highlight color may leak outside a pane, and amber keeps its exclusive CHANGED meaning everywhere, pane gutters included. Remove every changed crate and the chart must read as a pure monochrome engraving.

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
- **Camera discipline:** the camera moves only for a focus, the first paint, or an explicit refit (`f`). Deselecting never moves the camera. Every move is one 400ms glide (`cubic-bezier(0.65, 0, 0.35, 1)`), 0ms under reduced motion; marks, ring guides, and re-inked edges travel on the same beat so plate and camera move together.
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

**The Keyword-Is-Kind Rule.** An item's kind is its lowercase rust keyword, set in mono in front of its name — `fn parse`, `pub struct Trail`, `pub(crate) mod tree`. Kind is a word, not a shape and not a color. A private item writes no visibility, exactly as rust writes none. This replaces the retired Shape-Is-Kind rule and the item-glyph set it described: a learned shape vocabulary asked the reader to memorize what rust already spells out.

## Components

### Plates (Title Blocks, Legends, selection panels, toggles, error plates)
- **Corner Style:** square (0px).
- **Background:** paper (#f6f4ed); **Border:** 1px ink + inner hairline (see Shapes); **Shadow:** plate contact only.
- **Internal Padding:** 16px horizontal, 8–12px vertical; sections divided by ink-line hairline rules.
- A foldable section carries the class `fold`, whether it is the plate itself (the Legend) or a section inside one. Its summary shows a typographic `–`/`+` marker in ink-soft — no chevron glyph.
- Every cartouche carries the altitude ladder (`dependencies · code` in 9.5px tracked mono caps): the current rung engraved solid ink with an underline, the other rung a soft-ink link. Under it the cartouche states its own facts in mono — counts, the diff line (`diff main @ 1a2b3c4 → working copy`), and how many files changed.
- A selection panel opens with the breadcrumb ("← whole chart" / "← whole map"), then names the selection. It caps at 44dvh on a phone and at the column height on a desktop, scrolling its lists inside. Long reference lists chunk at 8 rows with a typographic "show all n" action.

### The Star Mark (signature, dependency altitude)
The unit of the dependency chart, reused by chart, legend, search results, and list rows. Radius encodes magnitude — `4 + √dependents × 1.3`, capped at 11px (the shared `star_radius`). Grammar: workspace member = solid ink disc with a 0.6px orbit ring; external crate = open paper circle with 1.3px ink stroke; changed = flare-filled with eight amber rays; downstream = amber halo ring whose opacity fades with hop distance (0.85 − 0.22/hop, floor 0.3); removed = dashed ink-soft outline only; focal = double ink ring with four compass ticks. Node labels: mono 500 name with a paper halo and an underline that appears in ink-line on hover and ink when selected; state words beneath in 9.5px mono — `3 files changed`, `2 hops downstream`, `removed`.

### The Block (signature, code altitude)
The code map draws no stars. A **file** is a ruled block: a paper rectangle with a 1px hairline frame, measured before it is placed so its plate and its box agree to the pixel. Header: the file name in mono 700 at 10.5px and an amber `M` beside it when the file changed since the diff base — and nothing else. It once carried `254 lines · 31 items` too, and because the header was a flex row with the count fixed, the count won the width and the *name* was what got clipped: `chrome.rs…`, `map.rs…`, `ego.rs…`. The name is the one thing the block exists to state, so it is now the only thing that can take the header's width. Body: its loudest items as rows, each written as rust — the keyword and visibility in soft ink (`pub fn`), the name in ink — in three weight tiers by fan-in (700 / 500 / 400), every row a link to that item's definition plate. Foot: the fold's own words in rust's own vocabulary, `+ 9 pub · 5 private`, above a dotted rule. A block whose items all folded away draws one rule, not two four pixels apart.

A **gate** is a folded directory standing in for its whole subtree: one block-shaped button with a 2.5px ink left edge, `▸ views/` on the first line and `12 files · 255 items` on the second. Clicking it opens the directory.

### The Ground (nested directory frames)
The code map's ground layer, under the ties and blocks: one bordered frame per open directory, filled at 2.6% ink so nesting reads as depth without a second material. Nesting means one thing — belongs to. The frame's label band sits on its top border and holds two segments, each measured on its own face: the directory as it is on disk (`▾ views/`, mono 500 at 12px, clicking folds it), then the crate whose sources live under it (`slopify`, 10px soft ink, a link to the crate sheet), drawn only where the survey has more than one crate to tell apart. An open frame states no counts: its files are on the paper to be counted, and `18 files · 380 items` on the root, again on `src/`, and again in the cartouche was one fact drawn three times in eighty vertical pixels. The root frame carries the workspace name and never folds. Every segment wears a paper halo.

The layout packs children into shelves aimed at a landscape sheet — files in name order first, then subdirectories — and is a pure function of (tree, disclosure, measured sizes): the same workspace always draws the same map.

### Reference Ties (code altitude)
Every reference between two territories, summed. Quadratic curves bowed toward open paper (16% of length, capped at 52px), width `0.55 + count × 0.13` capped at 2.8px, the arrowhead resting on the **user** — the way change travels.

Which of them rest on the paper is a **reading**, set by the cartouche's `references` toggle — `uses` (default) / `used by` / `both`. Drawing every aggregated tie at rest was a hairball: about a hundred curves over eighteen files, with counts floating in mid-paper that no reader could attach to a particular curve. So each reading anchors on the territories themselves — under `uses` a block draws the two heaviest files it leans on, under `used by` the two heaviest that lean on it, `both` draws every tie unthinned. A folded tie is never cut: it stays in the set and inks in, with its count, the moment the reader hovers either of its ends. Of the *resting* ties the heaviest dozen carry their count in 9.5px soft-ink mono with a paper halo and draw at full weight; the rest draw at half opacity and keep their count until hovered. A file folded inside a gate still shows its ties, ending at the gate that stands in for it. Ties ink in over 400ms (`edge-ink`, opacity only), none under reduced motion. Both world layers sit under the flow viewport, so a tie draws over the district tints and under the blocks — never across a file's own rows.

### The Definition Plate (signature, code altitude)
Selecting an item replaces the map with the definition plate: three tracks on the paper — `Used by` on the left, the plate in the middle, `Uses` on the right — under a breadcrumb (whole map ▸ directory ▸ file ▸ item).

The center plate is a quotation, not a description — and the quotation *is* the plate. There is exactly one frame on the center track: the double-ruled plate is the code pane itself, because a hairline frame twenty pixels inside another frame is a card in a card, not depth. It opens with the locator, `src/api.rs:10` in mono, with an amber `M` when the file changed, set on the page's bare paper. Under it, the item's **own source text**, sliced from the file the survey read: doc comment, attributes, signature, and body, exactly as written, syntax-highlighted by token class (see Colors), on a paper ground inside the plate's own double ruling. A line-number gutter counts from the item's true first line, right-aligned in 10.5px soft ink, `position: sticky` so it holds while the code scrolls under it, and `user-select: none` so a copy takes the code and not the numbers. The code sets at 12px/1.55, never wraps, and scrolls horizontally inside its own frame; the text itself is selectable. A method quoted out of its impl block is given back its own indent and then dedented with its body, so it starts at the plate's left edge instead of four spaces in. A long body is folded at roughly 60 lines past its docs and signature, and the fold counts what it holds back: `+ 42 more lines`. Types, traits, consts, and statics are never cut.

Below the quotation sits what the source cannot show, because it is written somewhere else: the type's associated items, grouped under the impl header they are written under (`impl Vis`, `impl Clone for Vis`, in mono 500), each row `pub fn weight` with `src/api.rs:165` right-aligned in soft ink, and each row a link. A hand-written trait impl with no items of its own is still listed — it is still code someone wrote. Private members are counted, never named: `+ 3 private`.

Selecting a whole **file** gives the same plate with an outline instead of a quotation — a file has no single definition to quote, so it keeps no frame either, and the outline sets on the page's paper under the facts line. The locator is the path, the facts line reads `373 lines · 24 items · crate slopify`, and each row is `pub enum DepKind`, its reference count, and its line in the plate's own locator idiom — `87 refs` then `:29`, set apart so the two never read as one run of digits.

The two reference columns are grouped by the file the references are written in: a header naming the file with its total (`37 refs`) over a hairline rule, then rows of keyword + name + count, capped at 3 with `+2 more (5 refs)`. A group is a section of one list, not a plate — nine boxed groups down a column drew nine frames to say what one rule each says. Every row names something; a file's private items are the group's own counted fold, `+ 96 private`, in the same grammar the map's blocks use. (A row named `private items` names nothing, and it appeared in eight of nine groups, usually as the loudest row in each — the column's dominant content was the row that said the least.) An empty column says `No references.` / `No outgoing references.` Every row re-centers the plate on itself.

### The Data Mark (signature, data altitude)
One block per visible struct or enum: hairline frame, header of keyword + name (amber `M` when its file changed), holding fields quoted exactly as written with the held type's name at full ink, an enum's variants in soft ink, counted folds (`+ 4 plain fields`, `+ 2 more variants`, `held by 6 types`) above a locator in soft ink. A static wears the gate's 2.5px ink left edge — a root, state no type holds, drawn whatever its visibility — and quotes its declared type whole. The ground is the crate and its top-level module frames (`mod views`), tinted like districts, labeled in rust's own words. Two wire families share the paper: **holds** (structure, at rest — Owns solid, Shares dashed with the wrapper's own word on the line, Borrows dotted with `&`, Dyn dashed with `dyn`, the arrowhead on the holder) and the **reference reading** (the code map's tie grammar at half ink, arrowhead on the user, set by the same `references` toggle). Privacy and the mark budget fold types to counted rows that receive the folded types' edges; a hub past three holders folds its fan-in to `held by n types`, and hovering either end inks a folded wire back in. Nothing at this altitude takes a second color.

### Chart lines (dependency altitude)
Hairline ink-line strokes (1.1px), arrows pointing the way change travels (dependency → dependent). Kind is dash grammar: normal solid, dev `6 4`, build `2 3`, and the legend names the cargo tables that own them (`dev-dependencies — dashed · build-dependencies — dotted`). Manifest events take flare: solid 1.4px for added/bumped, dashed `9 5` at 0.65 opacity for removed, with mono edge labels in flare (`added`, `removed`, `1.0 → 1.2`). Like the code ties, edges draw only for the selection.

### Legends
One per altitude, both plates with a `fold` summary in Garamond tracked caps. Content reads in fixed order: the key first (every mark and line the chart can draw, plus the `M` marker, rendered by the same components as the chart), then the gestures, then the survey's own honesty notes in ink-soft (unresolved-name counts, what the survey does not resolve). The code legend carries no glyph key: item kind is a keyword and needs none.

A legend is a key, not a manual. It names only what the drawing cannot state for itself: it does not tell a reader who is looking at blocks inside frames that "a block is one file; the frame around it is its directory", and it does not list the gestures a first click teaches. What it must carry is every fold's terms and how to reach what is folded — the tie reading's per-block bar and the hover that undoes it, what a block's last line counts, where a private item's references went — plus the keys nothing on screen advertises (`/`, `f`, `esc`, `←` `→`). At the code altitude the legend sits directly beneath the cartouche, one stack, rather than at the far end of the column with several hundred pixels of empty paper between them. When the key runs past the plate's height it scrolls under an **always-visible** thin ink scrollbar (`.legend-scroll`, 6px, ink-line thumb on a 7% ink track): the fold is drawn, never hidden.

### Inputs / Search
- **Style:** the plate itself as an input — square, paper, ink frame; mono 11px; ink-soft placeholder carrying its own shortcut hint ("find a file or item…   /"); native search-cancel button hidden.
- **Focus:** the app-wide focus ring — 1.5px solid ink outline, 2px offset; no glow.
- **Results:** a plate popover of rows, hover/active `bg-ink/5`; arrows move, Enter selects, Escape clears. The code search runs over files **and** items in one list: a file row is its name, an amber `M` if changed, and its directory right-aligned; an item row is its keyword and name with `src/analyze/manifest.rs:67` right-aligned in soft ink. Prefix matches rank first, files before items, then by how much of the workspace leans on the hit. The dependency search rows carry the star mark, the crate name, and `v1.0.229` for externals.

### Buttons / Rows / Toggles
No filled button exists. Actions are typographic: mono links with underline-offset (ink-soft rising to ink on hover), full-width list rows whose hover is a 5% ink wash, and the one segmented toggle (`dependencies`: depends on / used by / reverse deps) — small mono segments where the active segment wears a 1px ink border, never a fill. Directory labels and gates are bare buttons: the mark itself is the control, with no button chrome.

### Navigation
Navigation is the chart, at three altitudes joined by the cartouche's altitude ladder and by cross-links in the panels (the dep chart's focus panel offers "its files ↓"; the code chart's crate sheet offers "its dependencies ↑").

At the **data altitude** (`/data`), structs, enums, and statics seat as blocks inside their crate's top-level module frames. Clicking a type opens its definition plate on the code altitude — the data chart adds no second plate; hovering a block inks every edge that touches it, folded ones included; `f` refits, and `←` / `→` retrace history as everywhere. The `/code` and `/data` routes share one survey fetch, so stepping between those two rungs never re-runs rust-analyzer.

At the **dependency altitude** (`/`), the crate under review sits at the center and every ring outward is one dependency hop; stars never move. Clicking a star pushes `/crate/:name` and draws that crate's edges; clicking the selected star (or back / Esc) returns to `/`. Ctrl/cmd/shift-click builds a multi-selection (`/crate/a+b`), clicking a ring line selects the whole ring (`/ring/:hop`), and the dependencies toggle draws one reading of the selection's edges.

At the **code altitude** (`/code`), files seat as blocks inside their directory's frame. Clicking a file pushes `/code/file/:..path` and opens its definition plate; clicking an item row selects `?item=Type::name` and quotes that item's source; clicking a directory label or gate folds or opens it (a view state, not a URL); a frame's crate name pushes `/code/crate/:name`. On a plate, an outline or impl-member row expands its definition in place instead of going one level deeper (view state, not a URL; the row folds it again; middle-click or the pane's footer opens that item's own plate). Escape folds open quotations, then steps up one focus level — item → file → whole map — `/` finds a file or item, `f` refits. Inside a quotation, every resolved reference's name is a link to its definition. Every focus on both charts is a URL — there is no navbar or sidebar; the URL, the back button, and the panel's breadcrumb are the trail, and `←` / `→` step back and forward along it from the keyboard, on either chart.

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
- **Do** honor `prefers-reduced-motion`: any authored motion must have a static equivalent that carries the same information.

### Don't:
- **Don't** let #a54c06 mean anything but CHANGED and its blast radius — no amber accents, hovers, or branding.
- **Don't** introduce a second color outside a code pane, a gradient, a glow, or a shadow beyond the single plate contact shadow. Inside a code pane, use the documented token palette and nothing else.
- **Don't** round a corner. 0px everywhere, including third-party overlays (re-rule them as plates).
- **Don't** use hairline ink (#949dab) or faint ink (#b3bac4) for text; they are for lines and ghosts. Line numbers and counts are text — they stay soft ink.
- **Don't** move the camera except for a focus, the first paint, or an explicit refit; deselecting never moves it.
- **Don't** encode meaning in color where words can carry it, outside a code pane. Inside one, color carries token class and nothing else. Item kind is a keyword; pub/private is the `pub` keyword and ink pressure.
- **Don't** uppercase or track a keyword, a path, a directory, a crate name, or a VCS status letter. They are written the way the tool writes them.
- **Don't** ship a dark theme surface; the dark "field edition" is deferred by explicit user decision and must be designed as its own plate, not inverted.
- **Don't** add glyph icon sets; the only pictures on the page are the mark grammar (stars, blocks, frames) and hand-drawn SVG line samples.
