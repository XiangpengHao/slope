---
name: rust-viewer
description: A Rust workspace read as a flow chart you open one hop at a time — white cards on a paper-white pane, hue spent only on which way a dependency runs.
colors:
  canvas: "#f3f5f9"
  dot: "#cbd2dd"
  surface: "#ffffff"
  sunken: "#f6f8fa"
  raised: "#eef1f6"
  line: "#e4e8ee"
  line-strong: "#7f8c9f"
  line-hover: "#4b5666"
  ink: "#14171d"
  ink-muted: "#5b6472"
  ink-faint: "#667085"
  ink-invert: "#f7f8fa"
  ink-invert-muted: "#a8b0bd"
  ink-raised: "#2c313b"
  ink-disabled: "#c4cad4"
  wire: "#78859a"
  wire-dim: "#c3cbd6"
  inbound: "#bf3d10"
  inbound-lit: "#e2551f"
  outbound: "#1d4ed8"
  flag: "#b0201d"
  flag-field: "#fdeceb"
  select: "#d7e2fb"
  scroll: "#d3d9e3"
  scroll-lit: "#b8c0cd"
typography:
  title:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "17px"
    fontWeight: 600
    letterSpacing: "-0.01em"
  headline:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    letterSpacing: "-0.01em"
  card:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "-0.005em"
  body:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "10.5px"
    fontWeight: 650
    letterSpacing: "0.06em"
  mono:
    fontFamily: "ui-monospace, \"SF Mono\", SFMono-Regular, Menlo, Consolas, \"Liberation Mono\", monospace"
    fontSize: "11.5px"
    fontWeight: 400
    fontFeature: "tnum"
  micro:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "10px"
    fontWeight: 650
    lineHeight: 1.4
    letterSpacing: "0.02em"
    fontFeature: "tnum"
rounded:
  chip: "5px"
  node: "8px"
  panel: "10px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
components:
  node:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.node}"
    padding: "0 12px"
    width: "190px"
    height: "48px"
  node-workspace:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ink-invert}"
    rounded: "{rounded.node}"
    padding: "0 12px"
    width: "190px"
    height: "48px"
  port:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.pill}"
    padding: "0 4px"
    height: "18px"
    typography: "{typography.micro}"
  port-open-in:
    backgroundColor: "{colors.inbound}"
    textColor: "{colors.ink-invert}"
    rounded: "{rounded.pill}"
  port-open-out:
    backgroundColor: "{colors.outbound}"
    textColor: "{colors.ink-invert}"
    rounded: "{rounded.pill}"
  plate:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.panel}"
  ctrl:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    size: "30px"
  ctrl-hover:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.ink}"
  ctrl-disabled:
    backgroundColor: "transparent"
    textColor: "{colors.ink-disabled}"
    size: "30px"
  action:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
  chip-flag:
    backgroundColor: "{colors.flag-field}"
    textColor: "{colors.flag}"
    rounded: "{rounded.chip}"
    padding: "1px 5px"
    typography: "{typography.micro}"
  chip-quiet:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.chip}"
    padding: "1px 5px"
    typography: "{typography.micro}"
  chip-workspace:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.ink-invert}"
    rounded: "{rounded.chip}"
    padding: "1px 5px"
    typography: "{typography.micro}"
  input-search:
    backgroundColor: "{colors.sunken}"
    textColor: "{colors.ink}"
    rounded: "7px"
    padding: "7px 8px 7px 32px"
  input-search-focus:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
  lens-tab-active:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.chip}"
    padding: "4px 10px"
  lens-tab:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.chip}"
    padding: "4px 10px"
  panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    width: "358px"
  list-row:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.chip}"
    padding: "5px 6px"
---

# Design System: rust-viewer

## Overview

**Creative North Star: "The Opened Diagram"**

rust-viewer draws a Rust workspace as a flow chart that is opened rather than
displayed. Cards are crates, wires are dependencies, and the two numbered ports
on the sides of every card say exactly how much is still folded behind it. The
whole graph — 346 cards and 1175 wires — is refused on purpose: drawn at once it
is a texture, not a diagram. What is on the pane is what the reader asked for,
and the count on a port is the promise that nothing was quietly dropped.

The world is paper-white and the cards float a single value step above it. That
one step is the entire depth system: a white card on a near-white pane is told
apart by its edge and its shadow, so the edge carries real weight
(`line-strong`) and the shadow is a small honest offset rather than a glow.
Colour is rationed to a single fact — which way a dependency runs — and
everything else in the picture is grey. A crate this workspace builds is the
only card that inverts, because "you wrote this" is the one fact that changes
how every other row is read.

The register is a working canvas, not a dashboard: the category standard played
straight, at full fidelity, in the idiom the user pinned (React Flow's own
canvas, tldraw/Figma feel). **Confirmed refusals:** the whole-graph picture, a
selection that re-lays-out the world under the reader, and a canvas that drops
its grid at the widest view — exactly when orientation is scarcest.

**Key Characteristics:**
- Paper-white pane, white cards, one value step of separation
- Hue reserved for direction; every other variable is grey
- Exactly one card inverts: the crates this workspace builds
- Folded, never hidden — a port always carries its whole count
- The camera is the only authored motion, and it is refusable
- System type stack; mono for every version, path, signature and keycap

## Colors

A near-monochrome system with two directional hues and one alarm: the graph is
grey until you hold something, and then only the wires attached to what you hold
take colour.

### Primary
- **Dependency Rust** (`inbound`, lit `inbound-lit`): everything that runs *into*
  the crate you hold — what depends on it. Always enters from the left: the left
  port when open, the inbound wires, the "Depended on by" rule and total in the
  record, the last step of the "why it's here" chain, and the held mark on the
  minimap.
- **Dependency Blue** (`outbound`): everything that runs *out*
  of it — what it depends on. Always leaves to the right: the right port when
  open, the outbound wires and their arrowheads, the "Depends on" rule and total.

### Secondary
- **Duplicate Red** (`flag`) on **Duplicate Field** (`flag-field`): a crate name
  that resolves at more than one version. The only alarm in the system, and it is
  spent on nothing else — the `DUP` chip, the top bar's duplicate tally, and the
  duplicates count in the key.

### Neutral
- **Pane** (`canvas`): the ground the graph floats on. It is a ground and nothing
  else — it is never painted back over the graph to clear a path for something.
- **Lattice Dot** (`dot`): the dot grid painted on the pane.
- **Card** (`surface`): every card, every plate, the panel and the top bar.
- **Sunken** (`sunken`) / **Raised** (`raised`): the recessed field behind the
  finder, the lens group and every hovered row; and the quiet chip field and
  meter track.
- **Chrome Hairline** (`line`): every division inside the chrome — panel section
  rules, the top bar's underline, control dividers, plate borders.
- **Object Edge** (`line-strong`), **Object Edge Hover** (`line-hover`): the
  border of something you can pick up. A card and a port carry this; chrome never
  does. This is the heavier of the two hairline weights and it exists because a
  white card on a near-white ground is told apart by its edge and nothing else.
- **Wire** (`wire`) / **Wire Dim** (`wire-dim`): a dependency at rest, and a
  dependency attached to nothing you are holding while something is held.
- **Ink** (`ink`), **Ink Muted** (`ink-muted`), **Ink Faint** (`ink-faint`): the
  three lettering values — statements, secondary rows, and the small print
  (versions, paths, counts, section labels).
- **Inverted Ink** (`ink-invert`), **Inverted Ink Muted** (`ink-invert-muted`),
  **Inverted Chip Field** (`ink-raised`): lettering and chips on an ink-filled
  card — and the numeral on a port once the port fills with its direction.
- **Ink Disabled** (`ink-disabled`): the lettering of a control with nothing
  behind it. Reserved for `:disabled` and spent nowhere else — it says "this does
  nothing", never "this is quiet", which is `ink-muted`'s job. It is spent on the
  zoom controls, which disable themselves at the magnification limits rather than
  staying live and refusing.
- **Selection** (`select`), **Scrollbar** (`scroll`, `scroll-lit`): the surfaces
  nobody draws and everybody sees, named as tokens rather than left to the
  browser.

### Named Rules

**The Direction Rule.** Hue is spent on direction and nothing else. Rust means
"this depends on the crate you hold", blue means "the crate you hold depends on
this", and no third graph variable — weight, depth, ownership, fan-in — is ever
allowed to take a hue. Everything that is not direction is said in grey, in ink,
or in a number. There is exactly one borrowed use of the direction blue, and it
is named: the `:focus-visible` ring, which is the one mark that has to read on
the paper card and the ink one alike. Nothing else in the chrome is blue for
being interactive.

**The Ink Action Rule.** An action written as a word — "Draw this route", "Draw
this chain", "Show all N", "Show fewer" — is set in `ink` at 11.5px/650 with a
`line-strong` underline offset 3px, and the underline goes to `ink` on hover. The
underline is what makes it an action; the colour is what keeps it out of the
graph's vocabulary. A word that turns blue is claiming to be a direction.

**The One Alarm Rule.** `flag` on `flag-field` means one thing: this crate name
resolves at more than one version. It never marks an error, a warning, a missing
value or a busy crate. An alarm that fires for two reasons stops being an alarm.

**The Rationed Hue Rule.** Hue is scarce enough that a graph at rest carries
none of it. Selection lights the two directions and dims the rest of the mesh;
nothing is coloured to be decorated.

**Recorded constraints.** Resting wires sit at 3.4:1 against the pane — above the
3:1 floor for a graphical object, and deliberately quiet, because the mesh is
context rather than content. The lit wires are at 5.4:1 (rust) and 6.7:1 (blue)
against the card, and 5.0:1 / 6.1:1 against the pane. `wire-dim` is at 1.5:1 and
is legible only as a texture, which is what "dim" is for; it never carries a
fact on its own. `inbound-lit`, the route's own colour, is at 3.46:1 against the
pane: clear of the 3:1 floor for a graphical object, which is exactly what it is
— a 2.25px walking wire and its arrowhead — and it is never set as text, where
it would fail.

The focus ring is the single named exception to the Direction Rule, and it wears
two shapes: the global 2px `outbound` outline at 2px offset, and the border the
finder substitutes for it when it suppresses the native outline. Both are the
same mark. The reach meter in the call lens is blue for a different reason and is
not an exception: it measures how much a function reaches — outbound reach — so
its hue is direction and its *length*, not its colour, carries the quantity. The
two drawn wires in the chrome (the product's mark, and the wire walked while the
server resolves) are wires, and a wire running out is blue wherever it appears.

## Typography

**UI Font:** the system UI stack (`ui-sans-serif, system-ui, -apple-system,
"Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`)
**Mono Font:** the system mono stack (`ui-monospace, "SF Mono", SFMono-Regular,
Menlo, Consolas, "Liberation Mono", monospace`)

**Character:** the reader alt-tabbed from an editor, and this is that editor's UI
face next door. The pairing is deliberately unbranded: the voice of the product
is in the drawing, not in the letterforms, and the two faces do exactly one job
each — the sans states, the mono quotes the machine.

**The stated trade-off.** The named craft bar for this build (React Flow's own
site, tldraw, Figma) self-hosts a display face; this does not. A local tool must
render its first frame without a CDN, so the type is a system stack by
commitment and the build accepts that it reads a shade more like a developer
tool and a shade less like a designed product than its references. This is a
choice, not an oversight, and it is the one place the build knowingly sits below
its craft bar.

### Hierarchy
- **Title** (600, 17px, -0.01em): the held crate's or function's name at the top
  of the record. One per screen.
- **Headline** (600, 15px, -0.01em): the key panel's own heading, failure-state
  headings, and — set tabular — the single big count at the head of each record
  section.
- **Card** (600, 13px, -0.005em, 1.25): the crate or function name on a card.
- **Body** (400, 13px, 1.5–1.625): record rows, key prose, list entries.
- **Label** (650, 10.5px, 0.06em, uppercase, `ink-faint`): every section heading
  in the chrome. Small, tracked open, never shouted.
- **Mono** (400, 10.5–12px, tabular): every version, filesystem path, qualified
  signature, "why it's here" chain, duplicate version list, and keycap.
- **Micro** (650, 10px, 0.02em, tabular): chips and port counts.

### Named Rules

**The Machine-String Rule.** Anything the machine produced and the reader may
need to copy or compare character by character is mono: versions, paths,
qualified names, chains, keycaps. Anything the product wrote is sans. There is no
third case.

**The Tabular Rule.** Every quantity compared down a column carries
`font-variant-numeric: tabular-nums` — port counts, versions, dependency counts,
the zoom readout, the search field itself.

**The No-Webfont Rule.** No `@font-face`, no font CDN, no display face. If a face
is wanted later it must be self-hosted and must not block the first frame.

## Layout

**The frame.** A full-height column that never scrolls: a 48px top bar (one row,
fixed, never wraps — workspace mark, name, path, lens tabs, tally, finder), then
the lens body. The lens body is the pane plus one docked panel: stacked on a
phone (panel under the pane, capped at 45vh, scrolling on its own) and side by
side from `lg` up (panel 358px, docked right, bordered left). The pane never
loses its height to a long record.

**The pane is a separate crate.** Everything on the graph — the ground, the
cards, the ports, the wires, the controls and the map — is drawn by
`dioxus-flow`, which carries its own stylesheet written against `--flow-*`
custom properties. This product supplies exactly one block of CSS mapping the
palette below onto those properties. That is the whole contract: the pane is
themed here, never restyled here.

**The layout.** Columns are the lens's own answer, compacted onto a 280-unit
pitch; rows are worked out here and re-worked every time the reader opens or
folds a card. A wire that spans more than one column is given a lane in each one
it crosses, so it routes around the cards there rather than over them; ordering
is median sweeps plus a transpose that swaps adjacent pairs while that removes
crossings; and coordinates are granted in rank order, lanes before cards and the
busiest cards first, which is what makes a long wire come out as a straight run.
A column opens by exactly what it holds — there is no row pitch, because a lane
costs 7 units and a card costs 68.

**The pane.** One transformed layer holds every card and wire, so pan and zoom
write a single CSS transform rather than touching a hundred elements. The dot
lattice is a background image on the pane itself, re-offset by the same camera.
Overlays sit in the corners the category has always kept them: view controls
bottom-left (zoom out · zoom-percent reset · zoom in · frame everything, 30px
each in one plate divided by hairlines), and the minimap bottom-right, 148×94,
shown from `lg` up only.

**World geometry.** Cards are a fixed 190×48 world units in every lens — uniform
objects are what let a column be read as a list. Columns are 280 units apart
(wide enough for a wire to leave horizontally, curve, and arrive horizontally);
rows are 68 units apart. A node's column is handed in by the lens and is
longest-path rank from the workspace, so everything a card points at is strictly
to its right, at every zoom, without exception.

**The lattice ladder.** The dot step climbs 20 → 40 → 80 → 160 → 320 world units
as the camera pulls back, always choosing the first step whose on-screen gap
clears 14px. The ground never smears into a wash and never disappears.

**Spacing rhythm.** 4 / 8 / 12 / 16px. Panel sections are 16px horizontal, 14px
vertical, separated by a 1px chrome hairline rather than by a gap — which is what
keeps a record dense enough to hold a header, a chain and two counted lists
without scrolling into uselessness. The key panel is the one looser surface, at
16px padding and 24px between sections.

**Breakpoints.** `sm` 640px (tally appears; finder becomes a 148px field),
`md` 768px (second tally figure), `lg` 1024px (panel docks right; minimap
appears; finder reaches 240px), `xl` 1280px (workspace path appears in the top
bar). Below `sm` the finder collapses to a 36px icon and takes the whole bar when
focused, rather than squeezing the workspace's name to nothing.

**The camera's three moves.** *Frame everything* (the control bottom-left),
*frame a card and what it is attached to* (opening a port, or holding a card
already on the pane), and *frame a chain end to end* (a card asked for by name,
arriving with the route that put it there). The third is requested by ids alone —
no focus card, because a route has no centre — and it is the one move allowed to
shrink to the widest floor, since a chain is one sentence and is read whole or
not at all.

**Recorded constraints.** Framing has floors, and they are the honest kind: below
a floor the frame overflows and the reader pans. A first reading stops shrinking
at 90% (55% on a narrow viewport, where two or three cards of context beat one
card at full size); framing a card and its neighbours stops at 75%; "frame
everything" and framing a chain both stop at 30% — so on a large graph that
control genuinely does not show everything, and the minimap is what says the rest
is there. Framing never magnifies past 100%, so a short chain still lands at life
size. Because a column is longest-path rank, the first viewport of
a real workspace reads as a **chain** — one card per column stepping right —
rather than as the fan the same graph would make under shortest-path ranking.
That is the column law being honest about depth, and it is the cost of the law.

## Elevation & Depth

Depth is offset plus blur — never a halo, never a glow, never a hard offset. Four
shadows exist and there is no fifth: three steps on a card and one for anything
floating over the pane, all in the same cool near-black (`rgba(16,24,40,·)`), all
small enough that a hundred cards on a pane still read as a flat drawing with
things sitting on it. Chrome that needs to sit up borrows from the same four
rather than inventing a value — the active lens tab wears the card's own rest
shadow, which is what makes it read as the same kind of object.

Tonal layering does the rest: `canvas` → `surface` is one value step, and that
step plus the `line-strong` edge is what makes a card an object rather than a box
drawn on the ground.

### Shadow Vocabulary
- **Rest** (`box-shadow: 0 1px 2px rgba(16,24,40,0.07), 0 1px 1px rgba(16,24,40,0.04)`):
  every card, always — and the active lens tab, which is the one piece of chrome
  that borrows it.
- **Lift** (`box-shadow: 0 3px 8px rgba(16,24,40,0.10), 0 1px 2px rgba(16,24,40,0.06)`):
  a card or a port under the pointer, and a card standing on a walked route.
- **Held** (`box-shadow: 0 0 0 2px <surface>, 0 0 0 4px <ink>, 0 8px 20px rgba(16,24,40,0.14), 0 2px 4px rgba(16,24,40,0.08)`):
  the one card the reader is holding. A white gap then an ink ring, then the
  deepest shadow — the gap is what lets one ring read on both the paper card and
  the ink one.
- **Plate** (`box-shadow: 0 1px 2px rgba(16,24,40,0.05), 0 10px 28px rgba(16,24,40,0.07)`):
  anything floating over the pane — the view controls, the minimap, the finder's
  results, a failure transcript.

### Named Rules

**The Offset-And-Blur Rule.** Depth is an offset plus a blur. A ring is a state,
never an elevation — ink at 2px behind a white gap for the card you are holding,
`inbound` at 1px for a card standing on the route — and a state that needs to say
more says it with a ring, not with a new shadow. Nothing in this system glows,
and nothing casts a hard un-blurred shadow.

**The Three-Step Rule.** A card has exactly three elevations — rest, under the
pointer, held — and they are the same three in every lens. A fourth would mean
the ladder had stopped saying anything. Chrome adds one shadow to the vocabulary
and no more: the plate.

## Shapes

Softly rounded rectangles throughout, at four steps: **8px** on a card (the
signature corner, echoed at 4px in the key's swatches and on the minimap's
marks), **10px** on a floating plate, **5px** on a chip and on every hoverable
row, and a **full pill** on ports, the direction rules in the record, and the
reach meter. Two ad-hoc radii exist and should be treated as the same family
rather than as new steps: 7px on the finder field and the lens group, 3px on the
inline chain buttons.

Borders are 1px and come in exactly two weights: `line` for chrome divisions and
`line-strong` for the edge of an object you can pick up.

Wires are cubic beziers between side handles at the vertical middle of a card.
They leave and enter **horizontally**, always, so an edge announces its direction
in its first few pixels rather than in the arrowhead at the far end. Control
distance is half the horizontal gap while the edge runs forward, and falls back
to a square-root bulge when it runs backwards, which keeps a recursive call
legible as a loop instead of a line drawn straight through both cards.
Arrowheads are 9×9 user-space markers drawn in their wire's own hue, every one of
them — the outbound head briefly sat a step lighter than its wire, which is the
kind of drift a recorded system exists to catch.

## Components

### The Card (signature component)
- **Character:** a uniform object. Nothing about a crate — how heavy, how deep,
  how many depend on it — changes its size, shape or colour.
- **Shape:** 190×48 world units, 8px corners, 1px `line-strong` border, rest
  shadow, 12px horizontal padding, 8px internal gap.
- **Contents, in three registers:** name (card type), subtitle (mono, 10.5px,
  `ink-faint`: version, or the crate and module a function lives in), and an
  optional chip.
- **Workspace:** filled with `ink`, lettering `ink-invert`, subtitle
  `ink-invert-muted`, chips on `ink-raised`. The only card that inverts.
- **Hover:** border to `line-hover`, shadow to lift, over 120ms.
- **Held:** the held shadow — white gap, ink ring, deep shadow — plus
  `aria-current="true"`.
- **On a route:** border to `inbound`, plus a 1px `inbound` ring and the lift
  shadow — unless it is the held card, which keeps its own ring. Which cards are
  steps is now the whole of how a route that crosses a column is read, so the
  mark is a firm one.
- **Arriving:** fades up from 0 opacity and 1.5px blur over 180ms.
- **Position:** a transform, so a placement change is composited rather than
  re-laid-out — and it lands on one frame rather than gliding. A re-tidy changes
  how many segments a wire has, and a path whose command count changes cannot be
  transitioned, so gliding the cards left the wires hanging up to 133 units off
  their own cards for about a fifth of a second. Measured, then removed.

### Ports
- **Character:** a real control, not decoration. It is how the next hop opens,
  and its number is how many are behind it.
- **Shape:** 18px tall, 18px minimum wide, pill, 1px `line-strong`, white field,
  `ink-muted` numeral at 10px/650 tabular. Straddles the card's edge — left port
  centred on the left edge, right port on the right.
- **Left is what depends on this card; right is what it depends on.** Never the
  reverse, in any lens.
- **Hover / open:** the port fills with its own direction — `inbound` on the
  left, `outbound` on the right — with `ink-invert` lettering, the same value
  that letters an ink-filled card. Hover also lifts.
- **The count is always there**, open or closed.

### Wires
- **Rest:** 1.25px `wire`.
- **Inbound / outbound:** 1.75px in the direction's hue, painted *over* the
  resting mesh.
- **Dim:** `wire-dim`, painted under everything, for edges attached to nothing
  held while something is held.
- **Route:** 2.25px `inbound-lit`, dashed 6/5, walking at 900ms linear, painted
  in the same single edge layer as every other wire — over the lit ones, under
  the cards. No halo and no second overlay: a halo drawn over the graph blanked
  the port counts the route crossed, and a route is not worth a number. A chain
  that crosses a column is held together by the ringed cards it steps on
  instead.

### Plates (view controls, minimap, finder results, transcripts)
- **Corner:** 10px. **Background:** `surface`. **Border:** 1px `line`.
  **Shadow:** plate. **Padding:** 4px on the minimap, 12px on a transcript,
  none on the control row (its buttons supply their own).
- Controls inside a plate are divided by 1px `line`, never by a gap.

### Buttons
- **View control:** 30×30, no field at rest, `ink-muted` glyph at 1.5px stroke;
  `sunken` field and `ink` glyph on hover; `ink-disabled` and no field when
  disabled.
- **Zoom readout:** the same row, 52px minimum, 11px/600 tabular, and it is a
  button — clicking it returns to 100%.
- **Row button** (record lists, duplicates, finder results): full-width, no field
  at rest, `sunken` on hover, 5px corners, baseline-aligned columns of name ·
  chip · version · count.
- **Text action** (`Draw this route`, `Draw this chain`, `Show all N`, `Show
  fewer`): 11.5px/650 `ink`, no field, permanently underlined in `line-strong` at
  3px offset, the underline going to `ink` on hover. One class, every text action
  in the product, in the chrome's own voice rather than in a direction's.

### Chips
- **Shape:** 5px corners, 1px 5px padding, 10px/650/0.02em, 1.4 line-height.
- **Flag:** `flag` on `flag-field` — `DUP`, `26 DUPLICATED`, `ALSO AT ANOTHER
  VERSION`. Never anything else.
- **Quiet:** `ink-muted` on `raised` — a fact worth carrying without raising the
  voice (a trait name on a function card).
- **Workspace:** `ink-invert` on `ink`.
- A chip that appears on every card is a column, not a chip.

### Inputs
- **Finder:** `sunken` field, 1px `line`, 7px corners, 12.5px, magnifier glyph
  inset left, placeholder in `ink-faint`. `/` focuses it from anywhere.
- **Focus:** field to `surface`, border to `outbound`, native outline suppressed
  in favour of that border — the same focus ring in the shape the field wants,
  not a second use of the hue. Results drop below in a plate, 320px wide, capped
  at the viewport.
- **Empty:** the finder says what it did not match rather than silently guessing.

### Navigation
- **Lens tabs:** a `sunken` group at 7px corners with 3px of inset padding; the
  active lens is a `surface` tab at 5px corners carrying the card's rest shadow,
  12px/600 ink; inactive is `ink-muted` going to `ink` on hover.
- Unbuilt lenses are not rendered at all.

### The Panel
- **Background:** `surface`. **Border:** 1px `line` on the side that meets the
  pane. **Width:** 358px from `lg`; full width and ≤45vh below it. Scrolls on its
  own, with `overscroll-contain`.
- **Sections:** 16px/14px padding, divided by a 1px `line` rule. Every record
  carries the same fields in the same order, so a reader who has learned one
  lens has learned the other.
- **Direction rules:** each counted section is headed by a 16×2px pill in its
  direction's hue, its label, and its total set 15px/600 tabular in the same hue.
- **When nothing is held**, the panel is the key: what the marks mean, drawn with
  the same elements the pane draws, and how to move, with keycaps in mono on a
  `sunken` field.

### Browser Surfaces
- `color-scheme: light` — one rendition, declared, so form controls, the caret
  and native scrollbars read the design rather than the OS.
- Selection is `select` under `ink`. Scrollbars are 11px, thumb `scroll` (→
  `scroll-lit` on hover) inset by a 3px transparent border and clipped to
  content, fully rounded, on a transparent track.
- The focus ring is a 2px `outbound` outline at 2px offset, everywhere. It is the
  one borrowed use of a direction hue in the chrome, and it is borrowed on
  purpose: it is the single mark that must read on the paper card and on the
  ink-filled one alike.

## Do's and Don'ts

### Do:
- **Do** spend hue on direction alone: rust for what depends on the crate you
  hold, blue for what it depends on. The focus ring is the single named
  exception, and it stays the only one.
- **Do** write an action as a word in ink with a `line-strong` underline. An
  action is told from prose by its underline, never by taking a direction's
  colour.
- **Do** invert exactly one card — a crate this workspace builds — and say every
  other fact in ink, in a chip, or in a number.
- **Do** keep the count on a port whether it is open or closed. Nothing is
  hidden, only folded.
- **Do** let selection change ink and the camera and nothing else. The one
  sanctioned exception is asking for a card that is not on the pane, which
  arrives with the chain that put it there.
- **Do** give framing a floor and let the frame overflow below it — a graph
  shrunk past reading is a picture of a graph.
- **Do** anchor every zoom on the point being aimed at, hold magnification as an
  exponent of 1.2 so a notch out and back is exactly reversible, and interpolate
  magnification geometrically during a flight.
- **Do** honour `prefers-reduced-motion`: the camera jumps to the destination,
  the arriving card stops fading up, the route holds still — and the route stays
  lit, because lighting is what carries the answer. Listen for the preference
  *changing*, too; it applies on the next frame, not at the next resize.
- **Do** draw a route in the one edge layer, and mark the cards it steps on. A
  route that needs a second layer over the graph is a route that hides the graph.
- **Do** draw the key's marks exactly as the pane draws them. A key whose marks
  do not appear on the pane in that form is a key to a different picture.
- **Do** name a colour, a radius or a shadow as a token before using it; a hex
  typed into one rule belongs to no design system.

### Don't:
- **Don't** give a third graph variable a hue. Weight, depth, ownership and
  fan-in are grey; a meter may wear a direction's hue only when the quantity it
  measures *is* that direction, and its length still carries the number.
- **Don't** widen the focus ring's exception. No new blue link, blue label or
  blue affordance in the chrome — an action is a word in ink.
- **Don't** lay a halo, a scrim or a second overlay over the graph to make one
  mark legible. It blanks whatever it crosses, and what it crosses is also an
  answer.
- **Don't** spend the alarm colour on anything but a name that resolves at more
  than one version.
- **Don't** add a fourth card elevation, a fifth shadow, a glow, or a hard
  un-blurred shadow. Chrome that needs depth borrows one of the four.
- **Don't** re-lay-out the pane on selection, at any zoom, for any reason.
- **Don't** introduce a webfont, a font CDN, or a display face that blocks the
  first frame.
- **Don't** let the pane lose its height to a long record; the panel scrolls on
  its own.
- **Don't** drop the dot lattice at the widest view — climb the ladder instead.
- **Don't** let a wire leave or arrive at an angle. Every edge leaves and enters
  horizontally.
- **Don't** advertise a lens that is not built.
