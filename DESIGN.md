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
  edge-toggle-active:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.data-body}"
    padding: "2px 6px"
---

# Design System: Slopify — Workspace Atlas

## Overview

**Creative North Star: "The Engraved Star Atlas"**

Slopify draws a cargo workspace the way a nineteenth-century plate draws the sky: engraving ink on warm paper, hairline constellation lines, a cartouche for the title block, and a legend that names every mark in words. It deliberately refuses the glowing graph-dashboard with sidebar filters. The whole page is one material — ink at varying pressure on one sheet of paper — and the chart rests as a monochrome engraving until change appears.

Density is instrument-grade: small mono type, tight tracking on spaced caps, thin rules. The interface furniture (cartouche, legend, search, focus panel) floats over a full-bleed chart as double-ruled paper plates; nothing is a "card" in the app sense. Motion is nearly absent — the single authored moment is the loading constellation drawing itself in, and it stands down under `prefers-reduced-motion`.

The build is light-theme only by user decision; a dark "field edition" of the same plate is deferred, not implied.

**Key Characteristics:**
- One material: engraving ink on paper; no gradients, no glass, no glow.
- Color is state: the page is monochrome except the flare amber, which means CHANGED (and its blast halo) and nothing else, ever.
- Every state is named in words (CHANGED, AFFECTED, REMOVED, ADDED), never signaled by color alone.
- Every focus is a URL (`/`, `/crate/:name`); the back button retraces the review trail.
- Square corners everywhere; depth by ruling, not by shadow.

## Colors

An ink-pressure ramp on one paper ground, plus a single reserved signal color.

### Primary
- **Flare Amber** (#a54c06): the one color on the page. It means CHANGED — flare rays on a changed star, the fading blast-radius halo on affected stars, manifest-event edges and their labels (ADDED / REMOVED / version bumps), and CHANGED/AFFECTED state text. It appears nowhere else: never decoration, never emphasis, never brand.

### Neutral
- **Paper** (#f6f4ed): the page ground, plate backgrounds, and the fill of external-crate open circles. The browser chrome the app doesn't draw (selection, caret, scrollbar) is tinted to stay on the plate.
- **Engraving Ink** (#23303c): body text, solid star fills for workspace members, plate borders, focus rings, selected underlines. The darkest pressure; there is no black.
- **Soft Ink** (#5a6574): secondary text — captions, external crate names, ghost (REMOVED) outlines, muted labels.
- **Hairline Ink** (#949dab): constellation lines (normal/dev/build dependency edges), thin rules inside plates, hover underlines, minimap marks. Never used for body text.
- **Faint Ink** (#b3bac4): ghost outlines only; the lightest pressure the plate allows.

### Named Rules
**The Flare Monopoly Rule.** #a54c06 exists only as CHANGED and its consequences (blast halo, manifest events). No other element may take it, and no other color may enter the page. Remove every changed crate and the chart must read as a pure monochrome engraving.

**The Words-Not-Color Rule.** Every state the chart can draw is also written out in spaced-caps mono ("CHANGED · 3 FILES", "AFFECTED · 1 HOP", "REMOVED"). Color reinforces; it never carries meaning alone.

**The Ink-Pressure Rule.** Grays are not arbitrary: ink → ink-soft → ink-line → ink-faint is a pressure ramp with fixed roles (text → secondary text → lines → ghosts). Do not invent intermediate grays; mixes are expressed as `color-mix(in srgb, var(--color-ink) N%, transparent)` over paper.

## Typography

**Chart Font (furniture):** EB Garamond (with Georgia, "Times New Roman", serif) — 400, 400 italic, 600; self-hosted woff2.
**Data Font:** JetBrains Mono (with ui-monospace, SFMono-Regular, monospace) — 400, 500; self-hosted woff2.

**Character:** The serif is the engraver's lettering — spaced small caps for titles and section headings, italic for captions. The mono is the surveyor's data hand — every crate name, version, count, and state word. The two voices never swap jobs: if it's chart furniture, it's Garamond; if it's data, it's mono.

### Hierarchy
- **Chart Title** (Garamond 400, 19px, uppercase, 0.18em tracking): the cartouche's workspace name; also error-plate titles at 17px.
- **Chart Heading** (Garamond 400, 11–12px, uppercase, 0.22em tracking): plate section headings — "Reading this chart", "Depends on (n)", "Used by (n)".
- **Chart Caption** (Garamond 400 italic, 12px, ink-soft): subtitle lines under titles; the loading message at 16px italic.
- **Data Name** (Mono 500, 11–11.5px): crate names on the chart and in lists; focal crate name at 15px semibold (600).
- **Data Body** (Mono 400, 10–10.5px, relaxed leading): facts, counts, legend prose, epoch line.
- **Data State** (Mono 400, 9–9.5px, 0.08–0.12em tracking, UPPERCASE): state words, edge labels, kind tags (DEV, BUILD, EXT).

### Named Rules
**The Two-Voices Rule.** EB Garamond spaced caps for chart furniture; JetBrains Mono for crate data. No third face, no system UI font, anywhere.

**The Spaced-Caps Rule.** Uppercase always carries letterspacing (0.08em minimum for mono state text, 0.18–0.22em for serif furniture). Untracked uppercase does not exist on this plate.

## Layout

The chart is the page: a full-bleed `h-dvh w-dvw` canvas with `overflow: hidden`, dressed as the plate (paper ground, faint ink dot grid at 16% ink). Furniture floats over it in a fixed choreography — cartouche top-left, search top-right, legend bottom-left, zoom controls and minimap bottom-right, focus panel right (desktop) — each layer `pointer-events-none` with interactive plates opted back in.

- **Chrome insets:** 12px page margin (`p-3`); fit-view reserves the furniture's space (desktop: 128px top, 270px left; +330px right when focused) so the chart centers in what remains.
- **Legibility floor:** fit-view never zooms below 0.75; past the floor the chart holds readable scale and pans, keeping the focal star seated at 42% of the safe width in a focused desktop view.
- **Responsive:** below 40rem/640px the chart lays out top-to-bottom instead of left-to-right, the cartouche and search stack vertically, the legend collapses (closed `<details>`), the minimap hides, and the focus panel docks to the bottom at max 44% height.
- **Spacing rhythm:** tight and even — 4/6/8/12px steps inside plates; 16px horizontal plate padding; hairline rules (`border-t` in ink-line) divide sections instead of extra whitespace.

## Elevation & Depth

Effectively flat: depth is drawn, not cast. Plates sit on the paper via their double ruling; the only shadow is a single faint contact shadow under floating plates, and it never grows on hover or state change. Layering is z-index choreography over the chart, not simulated altitude.

### Shadow Vocabulary
- **Plate contact** (`0 1px 4px color-mix(in srgb, var(--color-ink) 14%, transparent)`): the sole drop shadow, applied to every floating plate and the re-ruled flow overlays (controls, minimap). Nothing else casts.

### Named Rules
**The Drawn-Depth Rule.** Depth comes from ruling — the double-line frame — not from shadow ramps. There is no shadow scale; the one contact shadow is a constant, not a state.

## Shapes

Square corners everywhere: `border-radius: 0` on plates, inputs, buttons, and the third-party flow overlays (explicitly re-ruled to 0). The signature silhouette is the **double-ruled plate**: a 1px ink border with an inner hairline drawn as inset box-shadows (`inset 0 0 0 2.5px paper, inset 0 0 0 3.25px 45% ink`) so it survives on replaced elements like inputs. Circles belong exclusively to the star grammar; every rectangle is ruled, never rounded.

### Named Rules
**The One-Plate Rule.** Every panel, popover, input, and overlay is the same plate: paper ground, ink frame, inner hairline, square corners. There is no second container style.

## Components

### Plates (Title Block, Legend, selection panels, error plate)
- **Corner Style:** square (0px).
- **Background:** paper (#f6f4ed); **Border:** 1px ink + inner hairline (see Shapes); **Shadow:** plate contact only.
- **Internal Padding:** 16px horizontal, 8–12px vertical; sections divided by ink-line hairline rules.
- A foldable section carries the class `fold`, whether it is the plate itself (the Legend) or a section inside one (the Title Block's changes list). Its summary shows a typographic `–`/`+` marker in ink-soft — no chevron glyph. Legend samples are rendered by the same `StarMark`/`LineSample` code as the chart, so key and chart cannot drift.
- The Title Block is one plate carrying two sections divided by a hairline rule: the workspace's identity and epoch, then the foldable list of what changed. Each fact is stated once — the changed count in the section heading, the affected count in its footer.
- A selection panel opens with the breadcrumb, then names the selection. It caps at 44dvh on a phone and at the column height on a desktop, scrolling its lists inside.

### The Star Mark (signature)
The unit of the whole system, reused by chart, legend, search results, and list rows. Radius encodes magnitude — `4 + √dependents × 1.3`, capped at 11px. Grammar: workspace member = solid ink disc with a 0.6px orbit ring; external crate = open paper circle with 1.3px ink stroke; CHANGED = flare-filled with eight amber rays; AFFECTED = amber halo ring whose opacity fades with hop distance (0.85 − 0.22/hop, floor 0.3); REMOVED = dashed ink-soft outline only; focal = double ink ring with four compass ticks. Node labels: mono 500 name (ink for members, ink-soft for externals) with an underline that appears in ink-line on hover and ink when selected; state words beneath in 9.5px tracked caps.

### Chart lines
Hairline ink-line strokes (1.1px), arrows pointing the way change travels (dependency → dependent). Kind is dash grammar: normal solid, dev `6 4`, build `2 3`. Manifest events take flare: solid 1.4px for added/bumped, dashed `9 5` at 0.65 opacity for removed, with spaced-caps mono edge labels in flare.

### Inputs / Search
- **Style:** the plate itself as an input — square, paper, ink frame; mono 11px; ink-soft placeholder; native search-cancel button hidden.
- **Focus:** the app-wide focus ring — 1.5px solid ink outline, 2px offset; no glow.
- **Results:** a plate popover of rows (star mark + mono name + `EXT` tag), hover `bg-ink/5`; Enter focuses the top hit, Escape clears.

### Buttons / Rows
No filled button exists. Actions are typographic: mono spaced-caps links with underline-offset (e.g. "← WHOLE CHART", ink-soft rising to ink on hover), full-width list rows whose hover is a 5% ink wash, and the EDGES toggle where the active segment wears a 1px ink border, not a fill.

### Navigation
Navigation is the chart (reworked 2026-08-18 to dependency rings — see `spec/dependency-viewer.md`): the crate under review sits at the center and every ring outward is one dependency hop; stars never move. Clicking a star selects it and pushes `/crate/:name` — the chart draws that crate's edges; clicking the selected star (or back / Esc) returns to `/`. Ctrl / cmd / shift-click toggles stars in a multi-selection (`/crate/a+b`), clicking a ring line selects the whole ring (`/ring/:hop`), and the EDGES toggle (depends on / used by / path to root) draws one reading of the selection's edges — the last lights every route from the root down to the selection; every edge's arrowhead points the way change travels. The EDGES toggle rides inside the selection's own panel, because it has nothing to act on without one. Search, changes, and list rows are real links to the same routes. There is no navbar or sidebar — the URL, the back button, and the panel's breadcrumb (`← whole chart → a → b`, every step a link) are the trail.

## Do's and Don'ts

### Do:
- **Do** draw every new surface as a plate: paper ground, 1px ink border, inner hairline, square corners, 16px side padding.
- **Do** write every state in words in 9–9.5px tracked mono caps, and reuse the exact shipped vocabulary (CHANGED, AFFECTED, REMOVED, ADDED, DEV, BUILD, EXT).
- **Do** make every focusable view a URL, and every focus treatment the 1.5px ink outline at 2px offset.
- **Do** derive new marks from the star grammar (fill = membership, ring = focus, dash = absence, rays/halo = change) and render legend samples with the same component as the chart.
- **Do** honor `prefers-reduced-motion`: any authored motion must have a static equivalent that carries the same information.

### Don't:
- **Don't** let #a54c06 mean anything but CHANGED and its blast radius — no amber accents, hovers, or branding.
- **Don't** introduce a second color, a gradient, a glow, or a shadow beyond the single plate contact shadow.
- **Don't** round a corner. 0px everywhere, including third-party overlays (re-rule them as plates).
- **Don't** use hairline ink (#949dab) or faint ink (#b3bac4) for text; they are for lines and ghosts.
- **Don't** ship a dark theme surface; the dark "field edition" is deferred by explicit user decision and must be designed as its own plate, not inverted.
- **Don't** add glyph icon sets; the only pictures on the page are the star grammar and hand-drawn SVG line samples.
