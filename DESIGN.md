---
name: rust-viewer
description: A Rust workspace read as a bare two-layer board — uniform pads, routed copper, one world the camera moves through.
colors:
  mask: "#0a1e14"
  mask-deep: "#06140d"
  mask-raised: "#10291b"
  substrate: "#163a26"
  edge: "#1e4530"
  legend: "#e4e7de"
  legend-soft: "#8ca396"
  copper: "#2a6647"
  pad: "#d9a93c"
  pad-lit: "#f0c860"
  drill: "#05100a"
  incoming: "#f0c24a"
  outgoing: "#5ab0e8"
  flag: "#f0805a"
  flag-field: "#4a1c0e"
typography:
  title:
    fontFamily: "Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 600
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "10px"
    fontWeight: 600
    letterSpacing: "0.09em"
    fontStretch: "75%"
  mono:
    fontFamily: "\"JetBrains Mono\", ui-monospace, monospace"
    fontSize: "12px"
    fontWeight: 400
rounded:
  none: "0px"
spacing:
  xs: "4px"
  sm: "6px"
  md: "12px"
  lg: "16px"
components:
  panel:
    backgroundColor: "{colors.mask-deep}"
    textColor: "{colors.legend}"
    rounded: "{rounded.none}"
    padding: "12px 16px"
  control:
    backgroundColor: "{colors.mask-deep}"
    textColor: "{colors.legend}"
    rounded: "{rounded.none}"
    size: "44px"
  control-hover:
    backgroundColor: "{colors.mask-raised}"
    textColor: "{colors.legend}"
  input-search:
    backgroundColor: "{colors.mask}"
    textColor: "{colors.legend}"
    rounded: "{rounded.none}"
    padding: "6px 8px 6px 28px"
  badge-flag:
    backgroundColor: "{colors.flag-field}"
    textColor: "{colors.flag}"
    rounded: "{rounded.none}"
    padding: "2px 6px"
---

# Design System: rust-viewer

## Overview

**Creative North Star: "The Backplane"**

rust-viewer draws a Rust workspace as a bare two-layer board. Every crate is an
identical gold pad on a lattice; every dependency is a length of routed copper
between two pads. Nothing about a crate — how many things need it, how deep it
sits, whether you wrote it — changes the pad. What a crate *is* lives entirely in
the copper running into and out of it, because that is the only thing a
dependency actually is.

The board is fabricated once and never moves. Holding a crate flies the camera
and changes which copper is lit; it does not re-seat a single pad. That
constraint is the system's spine: a world that rebuilds itself under you is a
world you cannot learn, and every rule below exists to keep the board learnable
at 346 pads and 1174 traces.

The register is a fabrication document, not a dashboard. Density is a material
here, not a problem — a real backplane is dense, and thinning most of the copper
to noise (the previous design drew 68% of dependencies at 18% alpha) is what
made the workspace unreadable. **Confirmed anti-references:** the force-directed
hairball, the Beck-lineage transit map this replaced, and the generic
dark-canvas graph explorer with rounded-rect nodes and bezier edges.

**Key Characteristics:**
- Uniform objects; all meaning in the wiring and the arrangement
- Committed palette — solder mask owns the entire ground
- Direction is the only variable that earns hue
- Right angles everywhere; zero border radius in the chrome
- Two renditions: the board (dark) and the fabrication drawing (light)

## Colors

A committed palette: the solder mask owns the whole ground, and gold and blue are
spent on exactly one variable between them.

### Primary
- **ENIG Gold** (`#d9a93c`, lit `#f0c860`): every pad, without exception. The
  plated finish of a bare board, and the only colour that means "a crate is
  here."

### Secondary
- **Aniline Gold** (`#f0c24a`): copper that runs *into* the held crate — what
  depends on it. Always enters from the left.
- **Layer Blue** (`#5ab0e8`): copper that runs *out of* the held crate — what it
  depends on. Always leaves to the right.

### Tertiary
- **Fab-Note Coral** (`#f0805a`) on **Scorched Field** (`#4a1c0e`): a crate name
  resolving at more than one version. The only alarm register in the system.

### Neutral
- **Solder Mask** (`#0a1e14`): the board itself.
- **Surround** (`#06140d`): what the board is cut out of — the darker ground
  outside the routed edge, and the field for every chrome panel.
- **Raised Mask** (`#10291b`): hover and active states on chrome.
- **Etched Copper** (`#2a6647`): every dependency at rest. One value step above
  the mask in the same hue, so 1174 traces read as a fabricated surface rather
  than as ink spilled over a chart.
- **Substrate** (`#163a26`): the drill lattice, drawn only at the closest tier.
- **Routed Edge** (`#1e4530`): the board outline, column rules, and every hairline
  in the chrome.
- **Silkscreen** (`#e4e7de`) / **Silkscreen Soft** (`#8ca396`): all lettering.

### Named Rules

**The Direction Rule.** Hue is spent on direction and nothing else. Gold means
"depends on this," blue means "this depends on it," and no third variable is ever
allowed to take a hue. Twelve hues encoding which dependency subtree reached a
crate first is the mistake this rule exists to prevent — it spent the strongest
channel available on a fact nobody decides anything on.

**The Uniform Pad Rule.** No crate property may change a pad's size, shape, or
colour. Weight, depth, ownership, and health are said in silkscreen beside the
pad — a component outline for a workspace crate, a coral flag for a duplicate —
never by deforming the object.

**The Tinted Neutral Rule.** Secondary text is tinted from the mask's own hue
(`#8ca396`), never neutral grey. Grey on a green ground reads as dirt.

**The Density Rule.** Every dependency is drawn, at weight, always. Fading
relationships to make a picture calmer deletes the subject.

## Typography

**Display / UI Font:** Archivo (with `ui-sans-serif, system-ui, sans-serif`)
**Label Font:** Archivo at 75% width (`font-stretch: condensed`)
**Mono Font:** JetBrains Mono (with `ui-monospace, monospace`)

**Character:** Archivo is an American-gothic workhorse with a real width axis, so
the silkscreen legend and the document chrome are one family at two widths rather
than two families fighting. JetBrains Mono carries the code register — versions,
paths, reference designators — and it is the face the reader already has open in
the editor they alt-tabbed from.

### Hierarchy
- **Title** (600, 17px, -0.01em): the held crate's name in the record header.
- **Headline** (600, 15px): workspace identity in the title block; failure-state
  headings.
- **Body** (400, 13px, 1.5): record rows, key-plate prose, list entries.
- **Label / Designator** (600, 10px, 0.09em, 75% width, uppercase): every section
  heading in the chrome, and the reference designator. This is the silkscreen
  voice.
- **Mono** (400, 11–12px, tabular): versions, filesystem paths, designators, the
  why-it's-here chain, and every keycap.
- **Board legend** (500, 11–12px, 75% width): crate names drawn on the canvas, in
  Archivo condensed to match the chrome's label voice exactly.

### Named Rules

**The Silkscreen Rule.** Labels are condensed (75%), uppercase, and tracked open
at 0.09em. Chrome sets this with `font-stretch`; the canvas sets it with the
`condensed` keyword in the font shorthand — Chrome applies it and then drops it
from the property when read back, so it must be verified by measuring text width,
never by reading `ctx.font`.

**The Tabular Rule.** Every quantity compared down a column — versions, counts,
column indices — carries `font-variant-numeric: tabular-nums`.

## Layout

The board is a strict lattice computed server-side, once: pads sit on exact
multiples of a 36-unit pitch, columns are 330 units apart, and routing channels
take a 2-unit sub-pitch. Column index is longest-path distance from the workspace,
which makes the x axis a law rather than a layout convenience — everything a crate
depends on has a strictly greater column, at every zoom, without exception.

The chrome is a fabrication document wrapped around it: a title-block header
(56px, one row, never wraps), the board, and a record panel docked right at 23rem
on `lg` and above, dropping under the board at `max-h-[42%]` below that. A depth
ruler (20px) is pinned to the canvas's top edge and a position register to its
bottom-left; neither ever scales with the zoom.

Spacing rhythm is 4 / 6 / 12 / 16px. Sections are separated by a 1px `edge`
hairline plus 12px of padding rather than by large gaps, which is what keeps the
record dense enough to hold 27 rows without scrolling into uselessness.

## Elevation & Depth

Depth is tonal, not shadowed. Three mask values (`surround` → `mask` →
`mask-raised`) carry every surface relationship in the chrome, and the board's own
depth comes from value separation between mask, copper, pad, and silkscreen —
four clear steps, which is the hierarchy the palette exists to produce.

Exactly two shadows exist, both on the canvas, both attached to the detent — the
moment a pad comes proud of the board under the pointer. Nothing at rest casts a
shadow.

### Shadow Vocabulary
- **Proud pad** (`0 3px 9px rgba(0,0,0,0.55)`): the pad lifting off the board.
- **Readout plate** (`0 4px 14px rgba(0,0,0,0.45)`): the plate naming it.

### Named Rules

**The Flat-At-Rest Rule.** Surfaces are flat until they are being touched. A
shadow in this system means "this object has physically lifted," so decorative
elevation would make the one real affordance unreadable.

## Shapes

Zero radius, everywhere in the chrome. Panels, badges, inputs, buttons, and
dropdowns are all square-cornered rectangles bounded by 1px `edge` hairlines.
Boards do not have rounded corners and fabrication drawings do not have soft
edges; a radius anywhere in the chrome would be the first thing that reads as
generic web UI.

The two curved forms in the system are both literal board geometry: the routed
board outline (radiused corners, ~72 units, with a mounting hole inset at each)
and the pads themselves (annular rings with a drill hole, at three radii by tier:
2.6 / 4.2 / 6.4px, capped against the on-screen lattice gap so a dense column
never packs into a solid bar of gold).

Copper is octilinear: every trace segment sits at 0, 45, or 90 degrees. A right
angle in copper is an etchant trap on a real board, and the chamfer is also what
keeps a run traceable by eye where a bezier tangle is not.

## Components

### Buttons
- **Shape:** square (0 radius), 1px `edge` border where they sit in a group.
- **Primary (zoom / fit controls):** `mask-deep` field, `legend` glyph, 44×44px
  on touch and 32×32px from `md` up.
- **Hover:** field lifts to `mask-raised`. No transition on colour; the state is
  binary because the control is.
- **Row buttons (record lists, why-path steps):** no field at rest, `mask-raised`
  on hover, full-width hit area.

### Badges
- **Fab note** (`26 FLAGGED`, `DUP`): `flag-field` ground, `flag` text, 10px
  semibold, square. The only saturated alarm in the system; never used for
  anything that is not a duplicate version.

### Panels
- **Corner:** square. **Background:** `mask-deep`. **Border:** 1px `edge` on the
  side that meets the board. **Shadow:** none. **Padding:** 12px 16px.
- Sections inside a panel are divided by a 1px `edge` hairline, never by a gap
  alone.

### Inputs
- **Style:** 1px `edge` border, `mask` field, square, 13px Archivo.
- **Focus:** border shifts to `pad`, plus a 2px `pad-lit` outline at 2px offset —
  the same registration-mark logic the canvas uses for the held pad.

### Navigation
- The lens tab carries a 2px `pad` underline when active. Unbuilt lenses are not
  rendered at all; a permanently disabled tab spends credibility on first run for
  nothing.

### The board (signature component)
- **Pads:** identical gold annuli with drill holes, three radii by zoom tier.
- **Copper:** all of it, at rest in `copper`; gold and blue only relative to a
  held pad, with a fourth state for related-but-beyond-the-lit-depth (44% alpha)
  and a fifth for unrelated (20%).
- **Fiducial:** the held pad takes a registration mark — a ring plus four ticks —
  in `legend`. This is the only "you are here" the board draws.
- **Detent:** the pad under the pointer lifts 2px, gains 1.6px of radius, casts
  the proud-pad shadow, and names itself on a plate. Read, but not yet held.
- **Zoom tiers:** Board (<0.34) names only what the workspace routes through;
  Component (<0.92) fills in the legend and resolves pads; Pad (≥0.92) draws drill
  holes, the substrate lattice, and every crate name.

## Do's and Don'ts

### Do:
- **Do** compute geometry once, server-side, and treat the board as immutable on
  the client. Selection changes ink and camera, never position.
- **Do** draw every dependency at weight. The `every_dependency_is_routed` test
  asserts it, because this is the rule the previous design broke.
- **Do** read canvas colours from the CSS custom properties at draw time, and
  re-read them every frame — a system theme change marks nothing dirty, and the
  board would otherwise sit in board ink under drawing-paper chrome.
- **Do** keep both renditions honest: dark is the bare board, light is the
  fabrication drawing. Copper must carry the same visual weight on paper
  (`#5d7259`) that it does on the mask.
- **Do** anchor every zoom on the point being aimed at — the pointer, or a
  pinch's midpoint — never on the viewport centre.
- **Do** hold zoom as a float so a pinch lands between notches, while keeping a
  wheel notch at exactly ±1.0 so discrete zoom stays reversible.
- **Do** attach the wheel listener natively with `passive: false`. A trackpad
  pinch arrives as `ctrl`+wheel, which is also the browser's page-zoom binding;
  without `preventDefault()` the whole page scales instead of the board.
- **Do** say counts in buckets that partition exactly. "27 directly · 37 further
  out" must add to the headline number; longest-path hop levels may light the
  board but may never be used to count, because a crate can be both an immediate
  dependent and reachable by a longer route.

### Don't:
- **Don't** let any crate property change a pad's size, shape, or colour.
- **Don't** spend hue on anything but direction.
- **Don't** introduce a corner radius anywhere in the chrome.
- **Don't** add a shadow to anything at rest.
- **Don't** re-lay-out the board on selection, at any zoom, for any reason.
- **Don't** use neutral grey for secondary text; tint it from the mask's hue.
- **Don't** name a crate in a label that would run off an edge — half a crate
  name reads as a different crate, so the legend is skipped instead of clipped.
