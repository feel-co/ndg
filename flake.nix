{
  description = "Streamlined documentation generation for NixOS modules";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [./pkgs];
      systems = inputs.nixpkgs.lib.systems.flakeExposed;
      perSystem = {pkgs, ...}: {
        formatter = pkgs.alejandra;
      };
    };
}
