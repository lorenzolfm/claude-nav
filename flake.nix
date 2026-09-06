{
  description = "How Claude Code agents read, and how you get to one";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs @ {
    flake-parts,
    nixpkgs,
    rust-overlay,
    crane,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      # Linux only, like claude-ps, and for the same reason: this program runs claude-ps, and
      # claude-ps reads /proc. The jump then speaks to zellij and to a Wayland compositor. A
      # darwin build would compile and answer nothing.
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = {
        system,
        pkgs,
        ...
      }: let
        craneLib = (crane.mkLib pkgs).overrideToolchain (p:
          p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        claude-nav = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;

            pname = "claude-nav";

            doCheck = false;

            # Nothing is pinned into this package on purpose, and the list is longer than it
            # looks: `claude-ps`, `zellij`, `ss`, `hyprctl` and a terminal all come from PATH.
            #
            # `claude-ps` has its own release cadence and must stay upgradeable without a rebuild
            # here. The other three are worse than merely inconvenient to pin: the jump speaks to
            # a *running* zellij server and a *running* compositor, and a second build of either
            # from a different store path could answer incorrectly. The program must reach the
            # ones the person is looking at, which is whatever their PATH resolves to.
            #
            # A consumer that runs this from a systemd user unit must therefore set the unit's
            # PATH. NixOS gives a user unit coreutils, findutils, grep, sed and systemd, and none
            # of the five above.
            meta = {
              description = "How Claude Code agents read, and how you get to one";
              homepage = "https://github.com/lorenzolfm/claude-nav";
              license = pkgs.lib.licenses.mit;
              mainProgram = "claude-nav";
              platforms = pkgs.lib.platforms.linux;
            };
          });

        # One derivation for each gate, so that CI builds them in parallel and a lint failure
        # does not stop a build of the crate.
        gates = {
          inherit claude-nav;

          claude-nav-clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          claude-nav-test = craneLib.cargoNextest (commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });

          claude-nav-fmt = craneLib.cargoFmt {inherit (commonArgs) src;};
        };
      in {
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        checks = gates;

        packages =
          gates
          // {
            default = claude-nav;
          };

        apps.default = {
          type = "app";
          program = "${pkgs.lib.getExe claude-nav}";
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [cargo-nextest];

          shellHook = ''
            echo "  Rust: $(rustc --version)"
          '';
        };

        formatter = pkgs.alejandra;
      };
    };
}
