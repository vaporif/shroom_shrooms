{
  description = "shroom_shrooms - a Bevy game";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    devshells.url = "github:vaporif/nix-devshells";
    devshells.inputs.nixpkgs.follows = "nixpkgs";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    devshells,
    fenix,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [fenix.overlays.default];
      };

      rust = pkgs.fenix.stable;

      rustStable = pkgs.fenix.combine [
        (rust.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rustfmt"
          "rust-analyzer"
          "rust-src"
        ])
      ];

      rustNightly = pkgs.fenix.combine [
        (pkgs.fenix.latest.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rustfmt"
          "rust-analyzer"
          "rust-src"
        ])
      ];

      darwinDeps = with pkgs;
        pkgs.lib.optionals stdenv.hostPlatform.isDarwin [
          apple-sdk
        ];

      linuxDeps = with pkgs;
        pkgs.lib.optionals stdenv.hostPlatform.isLinux [
          udev
          alsa-lib
          vulkan-loader

          # wayland
          libxkbcommon
          wayland

          # x11
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];

      commonShellHook =
        ''
          export RUST_LOG=info
          export RUST_SRC_PATH="${rust.rust-src}/lib/rustlib/src/rust/library"
          export PATH=$HOME/.cargo/bin:$PATH
        ''
        + pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath linuxDeps}:$LD_LIBRARY_PATH"
        '';

      commonPackages = with pkgs; [
        just
        taplo
        bacon
        cargo-nextest
        cargo-watch
      ];

      mkDevShell = toolchain:
        pkgs.mkShell {
          nativeBuildInputs = with pkgs;
            [
              toolchain
              pkg-config
            ]
            ++ commonPackages;

          buildInputs = darwinDeps ++ linuxDeps;

          shellHook = commonShellHook;
        };
    in {
      formatter = pkgs.alejandra;

      devShells.default = mkDevShell rustStable;

      devShells.nightly = (mkDevShell rustNightly).overrideAttrs (old: {
        shellHook =
          old.shellHook
          + ''
            export CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift
            export RUSTFLAGS="-Zshare-generics=y $RUSTFLAGS"
          '';
      });
    });
}
