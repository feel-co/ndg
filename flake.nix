{
  description = "Streamlined documentation generation for NixOS modules";

  inputs = {
    systems.url = "github:nix-systems/default";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [./pkgs];
      systems = import inputs.systems;
      perSystem = {pkgs, ...}: {
        formatter = pkgs.alejandra;
      };
    };
}
