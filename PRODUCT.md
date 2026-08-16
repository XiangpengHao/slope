# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Rust developers inspecting a workspace they own and can change. They have the
source checked out on disk, they are already in an editor and terminal, and they
open rust-viewer to answer a structural question about their own code that the
editor does not answer well.

## Product Purpose

rust-viewer makes the structure of a Rust workspace legible. It runs locally,
reads the project from a filesystem path, and presents that project through
views that answer questions about how its parts relate.

The dependency graph is the first view, not the product. For each crate it shows
both directions: what that crate depends on, and what depends on it. Further
lenses over the same workspace — function flow analysis, trait relationships,
and others — are planned; their specifics are undecided (see Capabilities and
Constraints).

Success is the user leaving with their question answered, not with a picture
they still have to trace by hand.

## Positioning

A local, multi-lens viewer for a workspace the user owns. It reads source from
disk rather than a registry, so it can answer questions that manifest metadata
alone cannot — and it treats dependencies as one lens among several rather than
as the whole subject. This is what separates it from `cargo tree` and from
registry-browsing tools: the graph is an entry point into a codebase the user
has the authority to change.

## Operating Context

- Runs locally, alongside the code it inspects. The user alt-tabs to it from an
  editor or terminal, mid-task.
- The server side reads a Cargo manifest (`Cargo.toml`, and the lockfile where
  resolution requires it) from a path on the local filesystem. There is no
  upload flow and no registry lookup.
- Single-crate Dioxus fullstack app: the client is compiled to wasm and the
  server runs in the same crate, so filesystem access lives behind server
  functions.

## Capabilities and Constraints

**Confirmed**

- Reads a Cargo manifest from a local filesystem path and builds a dependency
  graph from it.
- First view is the dependency graph viewer. For every crate it shows both
  outgoing dependencies and incoming reverse-dependencies.
- Three jobs carry equal weight and all three are in scope: tracing why a crate
  is present (reverse-dependency chains), auditing weight and risk (duplicate
  versions, tree size, bloat), and exploring/navigating the graph.
- Source-level analysis covers **workspace crates only**. Dependencies appear as
  graph nodes and metadata; their internals stay opaque.
- Stack is fixed by the existing codebase: Rust (edition 2024), Dioxus 0.7
  fullstack with the router, Tailwind CSS v4, wasm client. `dx serve` drives the
  build and the Tailwind watcher. Nix flake provides the toolchain.
- `spec/` is the living feature spec and is updated as features land
  (per `AGENTS.md`).

**Open — do not resolve by assumption**

- The engine producing semantic data for the later lenses is undecided
  (rust-analyzer/rustc internals vs. syn-based parsing vs. something else). The
  dependency view is expected to ship on Cargo metadata without settling this.
- The later lenses themselves — function flow analysis, trait relationships, and
  any others — are committed in direction but unspecified in behavior.
- Whether more than one workspace can be open at a time is not established.

## Brand Commitments

Name: **rust-viewer**. No other identity, voice, or asset commitments have been
established.

## Evidence on Hand

- `spec/spec.md` — the current feature spec.
- `AGENTS.md` — build and repo conventions.
- `assets/favicon.ico` — from the Dioxus scaffold; not a chosen mark.
- The current UI is the unmodified Dioxus starter (an `Echo` server-function
  demo). It is scaffold, not incumbent design, and carries no product truth.

No real user data, benchmarks, testimonials, customers, pricing, or deployment
story exists. Future work must not fabricate any of these.

## Product Principles

1. **The graph is a lens, not the product.** Every view is one lens onto one
   workspace. Structure the app so function flow and trait relationships can
   join as peers, not as tabs bolted onto the dependency view.
2. **Both directions are first class.** "What does this depend on" and "what
   depends on this" carry equal weight. The reverse direction is the one
   existing tooling makes hardest, so it must not read as secondary.
3. **Local and owned changes what we can assume.** The user has the source and
   the authority to change it. Answer at the depth filesystem access allows
   rather than at the depth a registry API would have forced.
4. **Answer the question, not just draw the picture.** Users arrive with a
   specific question — why is this here, what breaks if I drop it, what is
   heavy. A rendering that still requires manual tracing has not answered it.
5. **Real graphs are large.** A working Rust project resolves to hundreds of
   nodes. Legibility at that scale is a product requirement, not a later
   polish pass.
