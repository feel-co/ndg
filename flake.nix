{
  description = ''
    Fast, customizable and convenient documentation generator for the Nix ecosystem
  '';

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
        ./flake/shell.nix
      ];
    };
}
