{
  description = "Particle simulation with Rust and Bevy";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Bevy dependencies for Linux
        buildInputs = with pkgs; [
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

        nativeBuildInputs = with pkgs; [
          pkg-config
          clang
          lld
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          packages = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-flamegraph
          ];

          # Environment variables for Bevy
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

          shellHook = ''
            echo "Rust + Bevy development environment"
            echo "Rust version: $(rustc --version)"
          '';
        };
      }
    );
}
