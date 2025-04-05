{
  description = "Streamlined documentation generation for NixOS modules";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    flake-compat.url = "github:edolstra/flake-compat";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = inputs.nixpkgs.lib.systems.flakeExposed;

      imports = [
        ./flake/shell.nix
        ./flake/packages.nix
      ];

      perSystem = {
        self',
        pkgs,
        ...
      }: {
        formatter = pkgs.alejandra;

        # TODO: add more checks, ideally machine tests
        checks = self'.packages;
      };
    };
}
