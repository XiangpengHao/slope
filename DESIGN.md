---
name: Slopify — Workspace Atlas
description: A cargo workspace charted as an engraved star atlas — ink on paper, where only change takes color.
colors:
  paper: "#f6f4ed"
  ink: "#23303c"
  ink-soft: "#5a6574"
  ink-line: "#949dab"
  ink-faint: "#b3bac4"
  flare: "#a54c06"
typography:
  chart-title:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "19px"
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: "0.18em"
  chart-heading:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "11px"
    fontWeight: 400
    letterSpacing: "0.22em"
  chart-caption:
    fontFamily: "EB Garamond, Georgia, Times New Roman, serif"
    fontSize: "12px"
    fontWeight: 400
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
    letterSpacing: "0.12em"
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

The atlas now has two altitudes of that one plate: the dependency chart (`/`, crates as stars on rings of hops) and the code chart (`/code`, files as stars seated on the directory structure). The altitude ladder — a spaced-caps `DEPENDENCIES · CODE` line in every cartouche, the current rung engraved solid, the other a link — is the only navigation between them. Both altitudes share the plate, the star grammar, the legend discipline, and the URL-per-focus rule; the code altitude adds its own mark vocabulary (directory gates, item glyphs, reference chords, the cutaway) without adding a second material.

Density is instrument-grade: small mono type, tight tracking on spaced caps, thin rules. The interface furniture (cartouche, legend, search, toggles, focus panel) floats over a full-bleed chart as double-ruled paper plates; nothing is a "card" in the app sense. Motion is nearly absent — the authored moments are the loading constellation drawing itself in, the 400ms camera/mark glide, and the cutaway unfolding; all stand down under `prefers-reduced-motion`.

The build is light-theme only by user decision; a dark "field edition" of the same plate is deferred, not implied. The designed surface is the desktop; narrow layouts ship and stay usable but are explicitly descoped (see Layout).

**Key Characteristics:**
- One material: engraving ink on paper; no gradients, no glass, no glow.
- Color is state: the page is monochrome except the flare amber, which means CHANGED (and its blast halo) and nothing else, ever. An altitude that draws no change draws zero amber.
- Every state is named in words (CHANGED, AFFECTED, REMOVED, ADDED, GATE counts, honesty notes), never signaled by color alone.
- Every focus is a URL (`/`, `/crate/:name`, `/code`, `/code/crate/:name`, `/code/file/:..path?item=`); the back button retraces the review trail.
- Square corners everywhere; depth by ruling, not by shadow.
- Two altitudes, one grammar: edges and chords draw only for the selection, on both charts.

## Colors

An ink-pressure ramp on one paper ground, plus a single reserved signal color.

### Primary
- **Flare Amber** (#a54c06): the one color on the page. It means CHANGED — flare rays on a changed star, the fading blast-radius halo on affected stars, manifest-event edges and their labels (ADDED / REMOVED / version bumps), and CHANGED/AFFECTED state text. It appears nowhere else: never decoration, never emphasis, never brand. The code altitude currently contains zero amber — no diff is drawn there yet, and its cartouche says so in words.

### Neutral
- **Paper** (#f6f4ed): the page ground, plate backgrounds, the fill of external-crate open circles and open directory squares, and the halo stroke behind any text engraved on the chart. The browser chrome the app doesn't draw (selection, caret, scrollbar) is tinted to stay on the plate.
- **Engraving Ink** (#23303c): body text, solid star fills (workspace members, files, closed gates), plate borders, focus rings, selected underlines, and the selection's "uses" chords. The darkest pressure; there is no black.
- **Soft Ink** (#5a6574): secondary text — captions, external crate names, private item names, gate counts, chord ×n counts, line numbers, ghost (REMOVED) outlines, muted labels.
- **Hairline Ink** (#949dab): constellation lines and reference chords in the "used by" role, ring guides, street lines and stubs, thin rules inside plates, hover underlines, minimap marks. Never used for body text.
- **Faint Ink** (#b3bac4): ghost outlines only; the lightest pressure the plate allows.

### Named Rules
**The Flare Monopoly Rule.** #a54c06 exists only as CHANGED and its consequences (blast halo, manifest events). No other element may take it, and no other color may enter the page. Remove every changed crate and the chart must read as a pure monochrome engraving.

**The Zero-Amber Altitude Rule.** Color-is-state cuts both ways: a surface that does not yet draw change carries no amber at all, and states the absence in words ("changes are not yet drawn at this altitude — structure only"). Never borrow the flare to decorate a chart that has no diff to show.

**The Words-Not-Color Rule.** Every state the chart can draw is also written out in spaced-caps mono ("CHANGED · 3 FILES", "AFFECTED · 1 HOP", "REMOVED", "12 FILES" on a gate). Color reinforces; it never carries meaning alone.

**The Ink-Pressure Rule.** Grays are not arbitrary: ink → ink-soft → ink-line → ink-faint is a pressure ramp with fixed roles (text → secondary text → lines → ghosts). Do not invent intermediate grays; mixes are expressed as `color-mix(in srgb, var(--color-ink) N%, transparent)` over paper.

## Typography

**Chart Font (furniture):** EB Garamond (with Georgia, "Times New Roman", serif) — 400, 400 italic, 600; self-hosted woff2.
**Data Font:** JetBrains Mono (with ui-monospace, SFMono-Regular, monospace) — 400, 500; self-hosted woff2.

**Character:** The serif is the engraver's lettering — spaced small caps for titles, section headings, and engraved place-names (the crate district's `CRATE name` caption); italic for captions. The mono is the surveyor's data hand — every crate name, file name, street name, version, count, and state word. The two voices never swap jobs: if it's chart furniture, it's Garamond; if it's data, it's mono.

### Hierarchy
- **Chart Title** (Garamond 400, 19px, uppercase, 0.18em tracking): the cartouche's workspace name; also error-plate titles at 17px.
- **Chart Heading** (Garamond 400, 10–12px, uppercase, 0.18–0.22em tracking): plate section headings ("Reading this map", "Used by (n)", "Uses (n)", the toggle caption "Refs") and the district place-name at 10px/0.2em.
- **Chart Caption** (Garamond 400 italic, 12px, ink-soft): subtitle lines under titles; the loading message at 16px italic.
- **Data Name** (Mono 500, 10.5–11.5px): crate, file, and street names on the chart and in lists; focal names at 15px semibold (600) in panels.
- **Data Body** (Mono 400, 10–10.5px, relaxed leading): facts, counts, legend prose, epoch line, cutaway rows at 10px.
- **Data State** (Mono 400, 9–9.5px, 0.08–0.12em tracking, UPPERCASE): state words, edge labels, kind words (FN, STRUCT, PUB, PRIVATE, DEV, BUILD, EXT), gate counts, "+ n more" lines.

### Named Rules
**The Two-Voices Rule.** EB Garamond spaced caps for chart furniture; JetBrains Mono for crate and code data. No third face, no system UI font, anywhere. Pub/private is weight and pressure, not a new voice: pub names are ink at 500, private names are soft ink at 400.

**The Spaced-Caps Rule.** Uppercase always carries letterspacing (0.08em minimum for mono state text, 0.18–0.22em for serif furniture). Untracked uppercase does not exist on this plate.

**The Paper-Halo Rule.** Any text engraved directly on the chart (star labels, street names, hub and district captions, chord counts, edge labels) wears a paper halo — `paint-order: stroke` with a 3px paper stroke, or the stacked paper text-shadow — so words stay legible where lines cross. Text inside a plate never needs one.

## Layout

The chart is the page: a full-bleed `h-dvh w-dvw` canvas with `overflow: hidden`, dressed as the plate (paper ground, faint ink dot grid at 16% ink). Furniture floats over it in a fixed choreography — cartouche top-left (legend beneath it at the code altitude, bottom-left at the dependency altitude), search and toggles top-right, zoom controls bottom-right, focus panel right (desktop) — each layer `pointer-events-none` with interactive plates opted back in.

- **Scope:** the desktop is the designed surface; mobile was descoped by explicit user decision. Narrow layouts (< 640px) ship and remain usable — chrome stacks vertically, the legend starts folded, panels dock to the bottom at max 44dvh — but they are a serviceable fallback, not a designed composition. Do not derive system rules from the narrow layout.
- **Chrome insets:** 12px page margin (`p-3`). Fit-view reserves the furniture's space so the chart centers in what remains — dependency altitude desktop: 128px top, 270px left, +330px right when focused; code altitude desktop: 56px top, 284px left, 20px bottom, 330px right when focused. Fit leaves a 12% breathing margin (fit × 0.88).
- **Legibility floor:** fit-view never zooms below a floor — 0.75 on the dependency chart; 0.7 focused / 0.18 overview on the code chart's desktop, 0.42 focused on a narrow viewport (legibility yields to visibility there; the reader can zoom). Past the floor the chart holds readable scale and pans, seating the focal point mid-frame; a floored cutaway pins its header row to the top of the free band so the file's name always identifies the plate.
- **Camera discipline:** the camera moves only for a focus, the first paint, or an explicit refit (`f`). Deselecting never moves the camera. Every move is one 400ms glide (`cubic-bezier(0.65, 0, 0.35, 1)`), 0ms under reduced motion; marks, ring guides, and re-inked edges travel on the same beat so plate and camera move together.
- **Disclosure:** the code chart budgets roughly 320 marks; the deepest directories fold to gates by default, selection into folded ground opens the gates above it, and folding back is one click. Layout is pure and deterministic — the same tree always draws the same map.
- **Spacing rhythm:** tight and even — 4/6/8/12px steps inside plates; 16px horizontal plate padding; 16px cutaway row height; hairline rules (`border-t` in ink-line) divide sections instead of extra whitespace.

## Elevation & Depth

Effectively flat: depth is drawn, not cast. Plates sit on the paper via their double ruling; the only shadow is a single faint contact shadow under floating plates, and it never grows on hover or state change. Layering is z-index choreography over the chart, not simulated altitude — the ground layer (ring guides, streets) under the chords, the chords under the marks, the open cutaway above every sibling mark.

### Shadow Vocabulary
- **Plate contact** (`0 1px 4px color-mix(in srgb, var(--color-ink) 14%, transparent)`): the sole drop shadow, applied to every floating plate — including the cutaway — and the re-ruled flow overlays (controls, minimap). Nothing else casts.

### Named Rules
**The Drawn-Depth Rule.** Depth comes from ruling — the double-line frame — not from shadow ramps. There is no shadow scale; the one contact shadow is a constant, not a state.

## Shapes

Square corners everywhere: `border-radius: 0` on plates, inputs, buttons, and the third-party flow overlays (explicitly re-ruled to 0). The signature silhouette is the **double-ruled plate**: a 1px ink border with an inner hairline drawn as inset box-shadows (`inset 0 0 0 2.5px paper, inset 0 0 0 3.25px 45% ink`) so it survives on replaced elements like inputs.

Geometry is vocabulary: circles belong to the star grammar (crates and files), ruled squares to directory junctions, and the small item glyphs to code items — meaning always by shape and fill, never by color. Every container rectangle is ruled, never rounded.

### Named Rules
**The One-Plate Rule.** Every panel, popover, input, and overlay is the same plate: paper ground, ink frame, inner hairline, square corners. There is no second container style.

**The Shape-Is-Kind Rule.** An item's kind is its glyph's geometry, drawn in ink only: fn = solid disc, struct = solid square, enum = solid diamond, union = open diamond with a dot, trait = open square, type alias = open diamond, const/static = bar, macro = asterisk, module = open square with a dot, impl = bracket. Solid means a value-bearing definition, open means an abstraction. Never reach for a color or an icon font to say what a shape can say.

## Components

### Plates (Title Blocks, Legends, selection panels, toggles, error plates)
- **Corner Style:** square (0px).
- **Background:** paper (#f6f4ed); **Border:** 1px ink + inner hairline (see Shapes); **Shadow:** plate contact only.
- **Internal Padding:** 16px horizontal, 8–12px vertical; sections divided by ink-line hairline rules.
- A foldable section carries the class `fold`, whether it is the plate itself (the Legend) or a section inside one. Its summary shows a typographic `–`/`+` marker in ink-soft — no chevron glyph.
- Every cartouche carries the altitude ladder (`DEPENDENCIES · CODE` in 9.5px tracked mono caps): the current rung engraved solid ink with an underline, the other rung a soft-ink link. The code cartouche also carries the survey's facts and its honesty line about undrawn changes.
- A selection panel opens with the breadcrumb ("← whole chart" / "← whole map"), then names the selection. It caps at 44dvh on a phone and at the column height on a desktop, scrolling its lists inside. Long reference lists chunk at 8 rows with a typographic "show all n" action.

### The Star Mark (signature, dependency altitude)
The unit of the whole system, reused by chart, legend, search results, and list rows. Radius encodes magnitude — `4 + √dependents × 1.3`, capped at 11px (the shared `star_radius`). Grammar: workspace member = solid ink disc with a 0.6px orbit ring; external crate = open paper circle with 1.3px ink stroke; CHANGED = flare-filled with eight amber rays; AFFECTED = amber halo ring whose opacity fades with hop distance (0.85 − 0.22/hop, floor 0.3); REMOVED = dashed ink-soft outline only; focal = double ink ring with four compass ticks. Node labels: mono 500 name with a paper halo and an underline that appears in ink-line on hover and ink when selected; state words beneath in 9.5px tracked caps.

### The Code Marks (signature, code altitude)
The same grammar re-read for files. A **file** is a solid ink star sized by how many files reference it, via the same shared `star_radius`; the focal file wears the double ring and compass ticks. A **directory** is a ruled square junction: paper with a center dot when open, solid ink when it is a closed **GATE** holding its subtree — and a gate always writes its contents in words beside its name ("12 FILES", 9px tracked caps in ink-soft). The workspace root gets a double rule and never folds. A crate's directory hangs its engraved place-name beneath the mark ("CRATE name", Garamond 10px, 0.2em caps, paper halo), a link to the district. The plan names every lot at rest — its geometry budgets for the labels — and seats each name east of its mark.

### Item Glyphs
The tiny ink-only shape vocabulary for code items (see The Shape-Is-Kind Rule), rendered by one shared component in the cutaway, the panels, and the legend, so key and chart cannot drift.

### Reference Chords (code altitude)
Drawn only for the selection — a file, an item, or a crate boundary — never for the resting chart. Quadratic curves bowed toward open paper (12% of length, capped at 56px), trimmed at mark rims. Role is pressure and direction: **ink 1.25px, arrow in** = the selection uses this (definition → selection); **hairline 1.1px, arrow out** = this uses the selection. Arrowheads always point the way change travels — into the file that uses the definition. Repeated references carry a ×n count in 9px ink-soft mono with a paper halo, seated away from the cutaway's text. A file folded inside a gate still shows its chords, ending at the gate that stands in for it. Chords ink in over 400ms (`edge-ink`, opacity only), none under reduced motion.

### The Cutaway (signature, code altitude)
The selected file unfolds **in place** into a plate (268px wide) growing from its own mark's corner (`transform-origin: 10px 10px`, 300ms scale+fade, none under reduced motion) — neighbors keep their ground beneath it, and the open plate paints above every sibling mark. Header: file name (underlined, a link that folds it back), its directory and line count in 9px ink-soft, then a fact rule ("n fns · n types · n traits" in tracked caps). Body: the file's items in source order as 16px rows — glyph, name (ink 500 if pub, soft ink if private), line number right-aligned; impl headers draw as section rules (hairline top border) and indent their methods. The list chunks at 24 rows with "+ n more — the panel lists all" in tracked caps; selecting a hidden item unfolds the whole file. The selected item row takes a 7% ink wash and a 2px inset ink bar.

### The Ground (the plan)
The code chart's engraved ground layer, under chords and marks: a town plan — every open directory a horizontal street (1px ink-line), lettered on its own line in mono 500 with a paper halo (name with trailing slash, seated below the line in the stub gap); files sit as lots above the spine, subdirectories branch below on dotted stubs (0.8px, `1.5 3` dash); a street's rule terminates at its last lot or stub. Rings were dealt as a second projection of the same tree and retired: one ground means the reviewer's map never rearranges itself.

### Chart lines (dependency altitude)
Hairline ink-line strokes (1.1px), arrows pointing the way change travels (dependency → dependent). Kind is dash grammar: normal solid, dev `6 4`, build `2 3`. Manifest events take flare: solid 1.4px for added/bumped, dashed `9 5` at 0.65 opacity for removed, with spaced-caps mono edge labels in flare. Like the code chords, edges draw only for the selection.

### Legends
One per altitude, both plates with a `fold` summary in Garamond tracked caps. Content reads in fixed order: the key first (every mark, size ramp, gate, chord role, item glyph — rendered by the same components as the chart), then the gestures, then the survey's own honesty notes in ink-soft (unresolved-name counts, undrawn changes). When the key runs past the plate's height it scrolls under an **always-visible** thin ink scrollbar (`.legend-scroll`, 6px, ink-line thumb on a 7% ink track): the fold is drawn, never hidden.

### Inputs / Search
- **Style:** the plate itself as an input — square, paper, ink frame; mono 11px; ink-soft placeholder carrying its own shortcut hint ("find a file…   /"); native search-cancel button hidden.
- **Focus:** the app-wide focus ring — 1.5px solid ink outline, 2px offset; no glow.
- **Results:** a plate popover of rows (star mark + mono name + soft-ink directory or `EXT` tag), hover/active `bg-ink/5`; arrows move, Enter selects, Escape clears.

### Buttons / Rows / Toggles
No filled button exists. Actions are typographic: mono spaced-caps links with underline-offset (ink-soft rising to ink on hover), full-width list rows whose hover is a 5% ink wash, and segmented toggles (EDGES, REFS) sharing one grammar — small tracked-caps mono segments where the active segment wears a 1px ink border, never a fill, under a Garamond tracked-caps caption. Directory marks are bare buttons: the mark itself is the control, with no button chrome.

### Navigation
Navigation is the chart, at two altitudes joined by the cartouche's altitude ladder and by cross-links in the panels (the dep chart's focus panel offers "its files ↓"; the code chart's crate district panel offers "its dependencies ↑").

At the **dependency altitude** (`/`), the crate under review sits at the center and every ring outward is one dependency hop; stars never move. Clicking a star pushes `/crate/:name` and draws that crate's edges; clicking the selected star (or back / Esc) returns to `/`. Ctrl/cmd/shift-click builds a multi-selection (`/crate/a+b`), clicking a ring line selects the whole ring (`/ring/:hop`), and the EDGES toggle draws one reading of the selection's edges.

At the **code altitude** (`/code`), files seat on the directory structure as lots along their directory's street. Clicking a file pushes `/code/file/:..path` and draws its chords; clicking it again cuts it away; clicking an item row selects `?item=Type::name`; clicking a directory folds or opens it (a view state, not a URL); the crate district's name pushes `/code/crate/:name`. Escape steps up one focus level, `/` finds a file, `f` refits. Every focus on both charts is a URL — there is no navbar or sidebar; the URL, the back button, and the panel's breadcrumb are the trail.

**The Selection's-Ink Rule.** On both altitudes, relationship lines (edges, chords) are drawn only for the current selection. The resting chart is marks on an engraved ground — never a hairball.

**The Kept-Ground Rule.** Focus changes the focused thing, not its neighbors: the cutaway unfolds in place while every other mark keeps its seat, and deselecting never moves the camera. The reader's mental map is never repaid with a re-layout: folding a gate moves the ground it holds and nothing else.

**The Drawn-Fold Rule.** Whatever is elided says so in words and stays reachable: gates write their file counts, the cutaway writes "+ n more — the panel lists all", lists chunk with "show all n", and the legend scrolls under an always-visible ink scrollbar. Nothing is silently cut.

## Do's and Don'ts

### Do:
- **Do** draw every new surface as a plate: paper ground, 1px ink border, inner hairline, square corners, 16px side padding.
- **Do** write every state and every elision in words in 9–9.5px tracked mono caps, and reuse the exact shipped vocabulary (CHANGED, AFFECTED, REMOVED, ADDED, DEV, BUILD, EXT, GATE, "N FILES", PUB, PRIVATE).
- **Do** make every focusable view a URL, and every focus treatment the 1.5px ink outline at 2px offset.
- **Do** derive new marks from the shared grammar (circle = star/file, ruled square = directory, glyph shape = item kind; fill = membership/presence, ring = focus, dash = absence, rays/halo = change) and render legend samples with the same components as the chart.
- **Do** draw relationship lines only for the selection, with arrowheads pointing the way change travels.
- **Do** give any text engraved on the chart a paper halo.
- **Do** honor `prefers-reduced-motion`: any authored motion must have a static equivalent that carries the same information.

### Don't:
- **Don't** let #a54c06 mean anything but CHANGED and its blast radius — no amber accents, hovers, or branding; a chart with no diff drawn carries zero amber and says so.
- **Don't** introduce a second color, a gradient, a glow, or a shadow beyond the single plate contact shadow.
- **Don't** round a corner. 0px everywhere, including third-party overlays (re-rule them as plates).
- **Don't** use hairline ink (#949dab) or faint ink (#b3bac4) for text; they are for lines and ghosts. Line numbers and counts are text — they stay soft ink.
- **Don't** move the camera except for a focus, the first paint, or an explicit refit; deselecting never moves it.
- **Don't** encode meaning in color where shape or words can carry it — item kind is glyph geometry, pub/private is ink pressure.
- **Don't** ship a dark theme surface; the dark "field edition" is deferred by explicit user decision and must be designed as its own plate, not inverted.
- **Don't** add glyph icon sets; the only pictures on the page are the mark grammar (stars, junctions, item glyphs) and hand-drawn SVG line samples.
