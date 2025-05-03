{
  description = "Streamlined documentation generation for NixOS modules";

  inputs = {
    flake-compat.url = "github:edolstra/flake-compat";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = inputs.nixpkgs.lib.systems.flakeExposed;
      imports = [
        ./flake/packages.nix
        ./flake/checks.nix
      ];

      perSystem = {
        self',
        pkgs,
        ...
      }: {
        formatter = pkgs.alejandra;

        devShells.default = pkgs.mkShell {
          name = "ndg";
          packages = [
            self'.formatter

            # Poor man's Rust environment
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            (pkgs.rustfmt.override {asNightly = true;})
            pkgs.lldb
          ];

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };

        # TODO: add more checks, ideally machine tests
        checks = self'.packages;
      };
    };
}
