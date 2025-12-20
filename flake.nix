{
  description = "Particle simulation with Rust and Bevy";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Detect if we're on macOS
        isDarwin = pkgs.stdenv.isDarwin;

        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };

        # Platform-specific dependencies
        linuxInputs = with pkgs; [
          udev
          alsa-lib
          vulkan-loader
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon
          wayland
        ];

        darwinInputs = with pkgs; [
          darwin.apple_sdk.frameworks.Metal
          darwin.apple_sdk.frameworks.AppKit
          darwin.apple_sdk.frameworks.CoreGraphics
          darwin.apple_sdk.frameworks.CoreServices
          darwin.apple_sdk.frameworks.Foundation
        ];

        buildInputs =
          if isDarwin
          then darwinInputs
          else linuxInputs;

        nativeBuildInputs = with pkgs; [
          pkg-config
          clang
          lld
        ];
      in {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          packages = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-flamegraph
            bun
          ];

          # Environment variables for Bevy (Linux only)
          LD_LIBRARY_PATH =
            if isDarwin
            then ""
            else pkgs.lib.makeLibraryPath buildInputs;

          shellHook = ''
            echo "Rust + Bevy + Motion Canvas development environment"
            echo "Rust version: $(rustc --version)"
            echo "Bun version: $(bun --version)"
          '';
        };
      }
    );
}
