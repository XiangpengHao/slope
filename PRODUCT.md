# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The author: a Rust systems developer who uses LLM coding agents on large cargo
workspaces and needs to review the changes those agents make. Built for this
personal workflow first, with the confirmed intent to open-source later — no
decisions may hard-wire personal paths, machines, or habits in a way that
blocks a public release.

## Product Purpose

slope is a code reviewer for massive LLM-generated changes. The thesis: a
human cannot read every line of a large agent-written change — it costs too
much time and cognitive load. slope sits above the raw Rust implementation
and lets the reviewer navigate between altitudes — from high level (crate
dependencies) down to detail (function call graph) — so they gain confidence
by checking code structure without reading every line.

Mechanically, it is a CLI pointed at a cargo workspace: it analyzes the
workspace and opens a browser window that visualizes it. Success means the
reviewer can judge a change — where it landed and what it affects — at the
zoom level the question demands.

Sharpened 2026-08-19: slope is a **diff tool** first. Its claim over an
ordinary diff: it keeps the important thing visible and helps the reviewer
understand what changed, what the consequences are, and whether the change
landed in the right place. The dep view and the code view are instruments in
service of that diff story, with the code view the most important surface.

## Positioning

Line-by-line diff review does not scale to agent-sized changes, and existing
dependency tooling (`cargo tree`, graphviz dumps) prints whole graphs and
leaves the reader to cope. slope's claim: structural review at the right
altitude replaces exhaustive line reading. It takes the full resolved graph —
thousands of nodes and edges — and manages the reviewer's cognitive load,
centered on the blast radius of a change rather than on the graph as an
artifact.

## Operating Context

- Runs locally: `slope <path-to-workspace>` analyzes and opens the browser.
- Used after an LLM agent session, when the user reviews what the agent did.
- Target workspaces are large real-world Rust projects; graphs at full
  resolution reach thousands of nodes.
- Near-term focus (confirmed 2026-08-19): the diff story. Two target flows:
  (1) diff the uncommitted changes — working copy vs trunk base — so a
  developer reviews code before commit (build this now); (2) diff two
  branches, e.g. a PR in CI (later, explicitly out of scope for now). The
  dependency viewer and code viewer exist (built 2026-08-18/19) but neither
  yet tells a real diff story beyond file-level change marks.

## Capabilities and Constraints

- **Graph scope:** the full resolved graph — workspace members, direct
  externals, and transitive external crates. Taming this scale is the central
  design problem, not an edge case.
- **Core job:** blast radius of a change. The viewer is diff-aware from the
  start: it must show which crates a change touched and what depends on them,
  not only static structure.
- **Stack (fixed by existing code):** Dioxus 0.7 fullstack web app (one binary
  serves the backend and the wasm frontend), Tailwind v4, Nix flake dev
  environment, `dx serve` for development.
- **Graph rendering:** [dioxus-flow](https://github.com/XiangpengHao/dioxus-flow)
  is the committed graph-viewer library; consult its docs carefully before use.
  Not yet added to Cargo.toml.
- **Repo rules (from AGENTS.md):** never edit AGENTS.md; iterate `./spec` as
  features are added or behavior changes; write docs and copy in simple tech
  English.

## Brand Commitments

- Name: **slope** — "slop" with an e. It keeps the wry nod to reviewing LLM
  slop, and names the core move: the gradient the reviewer walks between
  altitudes. Published on crates.io as `slope-cli` because the bare name is
  taken; the binary is `slope`.
- Voice: simple tech English — plain, direct, no marketing register.

## Evidence on Hand

- No users, testimonials, benchmarks, or case studies exist; nothing may be
  fabricated.
- Real data comes from `cargo metadata` of actual workspaces; demos should use
  a genuine workspace, not invented crate names.

## Product Principles

1. **Cognitive load is the enemy.** Never present the whole resolved graph at
   once; every view starts from a focal point and discloses progressively.
2. **Review is the job.** Each view should help answer "what did this change
   affect?" — structure display serves that question, it is not the goal.
3. **Confidence by altitude, not by lines.** The reviewer descends from
   dependencies toward call graphs only where a question demands it; every
   feature should make descending (and climbing back) cheap.
4. **Truthful to cargo.** Show what cargo actually resolves; any simplification
   is labeled as one, never passed off as the real graph.
5. **Personal-first, public-ready.** Optimize for the author's workflow, but
   keep every decision compatible with open-sourcing.
