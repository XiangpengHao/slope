# slope

[![CI](https://github.com/XiangpengHao/slope/actions/workflows/ci.yml/badge.svg)](https://github.com/XiangpengHao/slope/actions/workflows/ci.yml)

Structural code review for large LLM-written changes to a cargo workspace.

A human cannot read every line of a big agent-written change — it costs too
much time and attention. slope works above the raw Rust: it analyzes a cargo
workspace and draws it as a chart in the browser, so you can judge *where a
change landed and what it affects* without reading every line.

The name is "slop" with an e: the gradient you walk between altitudes.

## Altitudes

One plate, three altitudes, joined by the `dependencies · code · data` line in
every cartouche:

| Route | Altitude | What it draws |
| --- | --- | --- |
| `/` | dependencies | crates as stars on rings of hops from the root |
| `/code` | code | files as blocks in nested directory frames, with reference ties |
| `/surface` | surface | every contract the code publishes — types, traits, functions, statics, consts, aliases — as blocks in module frames, with interface and body-dependence edges |

Changed things take amber; everything else stays monochrome ink. Every
selection is a URL, so the back button retraces the review trail.

## Install

slope is a Dioxus fullstack app: the browser client is compiled to wasm by
`dx`, not by cargo alone. Use one of the first two paths.

**Nix (recommended)**

```bash
nix run github:XiangpengHao/slope
```

**Prebuilt binary** — download the tarball for your platform from the
[latest release](https://github.com/XiangpengHao/slope/releases/latest),
unpack it, and run `./slope`.

**From crates.io** — `cargo install slope-cli` compiles and installs, but
cargo alone cannot produce the wasm client or the `public/` directory the
server reads, so that binary serves nothing. The crates.io package is source
distribution; install via nix or a release binary.

## Use

slope serves the chart for one workspace. Point it at that workspace with
`SLOPE_WORKSPACE`, or start it with the workspace as the working directory:

```bash
SLOPE_WORKSPACE=/path/to/workspace slope
```

Then open the URL it prints.

| Variable | Default | Meaning |
| --- | --- | --- |
| `SLOPE_WORKSPACE` | working directory | the cargo workspace to analyze |
| `SLOPE_BASE` | `main`/`master` merge-base | revision to diff the working copy against |
| `RUST_LOG` | `info,slope=debug` | log filter; rust-analyzer's crates are pinned to `warn` |

slope detects git or jujutsu and diffs the working copy — including uncommitted
changes — against the base revision. Diffing two branches (a PR in CI) is not
built yet.

## Develop

The repo is flake-only; the dev shell pins the Rust nightly, `dx`,
`wasm-bindgen-cli`, and the Tailwind CLI to matching versions.

```bash
nix develop
```

Then, with direnv, `direnv allow` once and the shell loads on `cd`.

```bash
SLOPE_WORKSPACE=/path/to/workspace dx serve
```

`dx serve` builds the wasm client with the `web` feature and the server with
`server`, and drives the Tailwind watcher. Workspace analysis — `cargo
metadata`, the VCS diff, and the rust-analyzer code survey — is server-only and
never compiled into the client.

Design and intent live in [DESIGN.md](DESIGN.md), [PRODUCT.md](PRODUCT.md), and
[spec/](spec/).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.
