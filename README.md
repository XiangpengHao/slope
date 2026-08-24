# slope

[![CI](https://github.com/XiangpengHao/slope/actions/workflows/ci.yml/badge.svg)](https://github.com/XiangpengHao/slope/actions/workflows/ci.yml)

Slope is a tool to review slop.

## Install

**Nix (recommended)**

```bash
nix run github:XiangpengHao/slope
```

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

## Develop

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
