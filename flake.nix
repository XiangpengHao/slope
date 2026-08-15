{
  description = "rust-viewer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        llvmPackages = pkgs.llvmPackages_latest;
        # Keep this exact nightly aligned with both Rust install steps in CI.
        rustToolchain = pkgs.rust-bin.nightly."2026-05-17".default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "llvm-tools-preview"
          ];
          # Dioxus web builds compile the client to wasm.
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.pkg-config
            pkgs.cargo-fuzz
            llvmPackages.llvm
            pkgs.cargo-binutils

            # Dioxus toolchain. `dx` normally downloads wasm-bindgen/wasm-opt
            # itself, but those prebuilt binaries don't run on NixOS, so we
            # pin them here and point `dx` at them below.
            pkgs.dioxus-cli
            # Must match the pinned wasm-bindgen crate in Cargo.toml exactly.
            pkgs.wasm-bindgen-cli_0_2_126
            pkgs.binaryen

            # Standalone Tailwind v4 CLI; `dx serve` autodetects and drives it.
            pkgs.tailwindcss_4
          ];

          ASAN_SYMBOLIZER_PATH = "${llvmPackages.llvm}/bin/llvm-symbolizer";

          # Prebuilt binaries `dx` fetches don't run on NixOS; make it resolve
          # tailwindcss/wasm-bindgen/wasm-opt from PATH instead.
          NO_DOWNLOADS = "1";
        };
      }
    );
}
