{
  description = "The 5th Kingdom - a Bevy game";

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
          "rustc-codegen-cranelift"
        ])
      ];

      # bevy_lint is built from TheBevyFlock/bevy_cli at the lint-v0.6.0 tag.
      # It links rustc-private crates and therefore must be compiled with
      # (and at runtime resolved against) the exact nightly pinned by upstream's
      # rust-toolchain.toml (currently nightly-2026-01-22).
      bevyLintRev = "lint-v0.6.0";
      bevyLintNightlyDate = "2026-01-22";

      bevyLintSrc = pkgs.fetchFromGitHub {
        owner = "TheBevyFlock";
        repo = "bevy_cli";
        rev = bevyLintRev;
        sha256 = "sha256-Swj7j/A7Mgd2ufSADZdGMXOLbmvpdHGJfQVFCaWX9yg=";
      };

      bevyLintNightly = pkgs.fenix.toolchainOf {
        channel = "nightly";
        date = bevyLintNightlyDate;
        sha256 = "sha256-5XAIyRQMcynTWJvX5VkqErB0H4Oyg0AjeSefOyKSt7g=";
      };

      bevyLintToolchain = pkgs.fenix.combine [
        bevyLintNightly.cargo
        bevyLintNightly.rustc
        bevyLintNightly.rust-src
        bevyLintNightly.rustc-dev
        bevyLintNightly.llvm-tools-preview
      ];

      bevyLintRustPlatform = pkgs.makeRustPlatform {
        cargo = bevyLintToolchain;
        rustc = bevyLintToolchain;
      };

      bevy_lint = bevyLintRustPlatform.buildRustPackage {
        pname = "bevy_lint";
        version = "0.6.0";
        src = bevyLintSrc;
        cargoLock.lockFile = "${bevyLintSrc}/Cargo.lock";
        doCheck = false;
        cargoBuildFlags = ["-p" "bevy_lint"];

        nativeBuildInputs = [pkgs.makeBinaryWrapper];

        # rustc_driver links against zlib; on darwin libiconv is also needed.
        buildInputs =
          [pkgs.zlib]
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [pkgs.libiconv];

        # bevy_lint dynamically loads librustc_driver from the nightly toolchain
        # at runtime, and the cargo it spawns must use the matching rustc
        # (otherwise dep crates compile under one rustc and the workspace
        # driver under another, producing E0514 "incompatible version of rustc").
        postInstall = ''
          for bin in $out/bin/bevy_lint $out/bin/bevy_lint_driver; do
            if [ -f "$bin" ]; then
              wrapProgram "$bin" \
                --set BEVY_LINT_SYSROOT ${bevyLintToolchain} \
                --set RUSTC ${bevyLintToolchain}/bin/rustc \
                --prefix PATH : ${bevyLintToolchain}/bin
            fi
          done
        '';
      };

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
        lld
        bevy_lint
        tracy_0_13
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

      # Cranelift for fast builds WITHOUT dynamic linking (cargo run)
      # On macOS, cranelift can't handle __mod_init_func sections
      # that bevy/dynamic_linking uses, so don't combine the two.
      devShells.nightly = (mkDevShell rustNightly).overrideAttrs (old: {
        shellHook =
          old.shellHook
          + (
            if pkgs.stdenv.hostPlatform.isDarwin
            then ''
              # macOS: use LLVM backend (cranelift breaks bevy dynamic linking).
              # Rustflags (share-generics, lld) live in .cargo/config.toml.
              # Run with: cargo run --features dev
            ''
            else ''
              # Linux: cranelift works fine with dynamic linking.
              # Rustflags (share-generics, lld) live in .cargo/config.toml.
              export CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift
            ''
          );
      });
    });
}
