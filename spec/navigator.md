# The navigator — design brief and behavior

The fourth rung of the review ladder, and the first one with no chart on it.
Drawn from a user-approved HTML prototype (`prototypes/navigator.html`,
2026-08-21) and ported over the live `CodeGraph`: the navigator needs no new
analysis, only a different question.

## Job and audience

The same reviewer, asking the two questions a map cannot answer: **"what
changed, and what does it reach?"** and **"what is this, and what would a
change here break?"** The other altitudes draw the workspace and let the
reader find their way around it. This one refuses to draw the workspace at
all. Visitor mode: **Operate**.

## The thesis, in four rules

1. **Position encodes relation, never location.** No frames, no 2D map, no
   camera — nothing here pans, zooms, or fits. The focused mark stands in the
   middle, quoted whole. **Left** is what it depends on: first what its own
   surface names, then what only its body reaches. **Right** is what depends on
   it, in three bands kept strictly apart — *held by*, *named in the signatures
   of*, *read by the bodies of*. **Far right** is the reach beyond one hop,
   layer by layer. A module is a small label on a block, never a place on the
   paper.
2. **The screen is always the answer to a question, never the graph.** The
   opening screen is the diff agenda: every touched contract in the centre with
   its own delta note, the coupling the change took on and gave back on the
   left, and everything it reaches that itself did not change on the right,
   nearest layer first.
3. **Navigation is refocusing, and the trail is the map.** Every focus is a URL
   (`/navigator/mark/:..path?item=`, the same selection idiom `/surface` uses),
   so the browser's own back and forward retrace the review and a deep link
   opens on the answer. The trail bar records the path; a chip jumps back
   without dropping the way forward. `/` opens type-ahead over every mark;
   Escape asks the opening question again.
4. **Pins accumulate the review.** Each focus can be pinned to the board, and
   between each consecutive pair of pins the board threads the shortest path
   through the whole graph — both edge families, either direction, because the
   question is how two marks connect at all. Intermediate hops are dotted
   *via* chips; a disconnected pair says "no path". That subgraph is the
   artifact a review produces.

## Chrome

A bar that never scrolls away: the brand, the altitude ladder (the navigator is
its fourth rung, and a rung with no way off it is a trap), the trail, and the
search box. Under it, one line saying how to read the page. At the foot, the
board, and only while something is pinned. Nothing else — no legend, no
cartouche, no toggles: a toggle is a thing the reader has to know about before
the page will answer them.

## What is a mark

Every item the survey found, and every ghost the base had. No door, no budget,
no count-based fold: a question about one mark is answered with the whole mark,
and a page leaves out only what the question did not ask. Methods are marks
here — they are rows on the surface chart, but a reviewer can ask about one.

## The two families, and the words on them

The same two the surface chart runs. **Solid** is interface coupling, one pair
per (dependent, tail) with every holds edge folded into it — so a pair that
lost one edge and gained another reads as **rewritten**, not as a removal
beside an addition. **Dashed** is implementation coupling, the resolved
references at mark precision, summed. One word rides each wire, and the diff
speaks first: `rewritten` / `added` / `removed`, then `implements`, then the
wrapper the walk met (`Arc`, `Signal`, `&`), then the row that wrote it, then
`owns`.

## The harness

Wires are drawn from the layout's own numbers, not from measured DOM: every
block is measured and placed in rust first, the way the other charts do it, so
the harness and the blocks agree without asking the browser anything. Per band:
one trunk from its own port on the focus block's edge, a rail beside the band's
plates, one short branch per plate with the word at the plate end. The
arrowhead rests on the dependent — the focus's own port on the left, each
plate's branch on the right. On the agenda the wires are stubs beside the
left-column plates, because there is no one block to run to.

## Empty bands say why

A band with nothing in it prints one sentence in the serif voice rather than
nothing at all: *a root — state no type holds* for a static, *nothing holds it
— it enters through the signatures that name it*, *no interface names it — only
bodies reach it*, *nothing reaches it at all*.

## Open decisions

- Hovering a plate does not light its wire (the prototype's `is-hot`); the
  plate answers the look on its own.
- No FLIP: a refocus draws the new page rather than travelling the plates that
  survive it.
- Narrow viewports are serviceable, not composed — the page has a measured
  width and scrolls sideways under it.
- The board's threads are recomputed per render; a session with many pins will
  want them cached.
- Pins live in the session, not the URL — a review cannot be handed over yet.
