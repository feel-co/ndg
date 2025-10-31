{
  description = "Fast, customizable and convenient documentation generator for the Nix ecosystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    cargo2nix,
    rust-overlay,
  }: let
    inherit (nixpkgs) lib;

    systems = ["x86_64-linux" "aarch64-linux"];
    forEachSystem = lib.genAttrs systems;
    pkgsFor = system:
      import nixpkgs {
        inherit system;
        overlays = [
          (import (cargo2nix + /overlay) rust-overlay.overlays.default).default
        ];
      };

    cargoTOML = fromTOML (builtins.readFile ./Cargo.toml);
    rustPkgsFor = pkgs:
      pkgs.rustBuilder.makePackageSet {
        rustVersion = cargoTOML.workspace.package.rust-version;
        packageFun = import ./Cargo.nix;
      };
  in {
    packages = forEachSystem (system:
      import ./nix/internal/packages.nix rec {
        rustPkgs = rustPkgsFor pkgs;
        pkgs = pkgsFor system;
      });

    checks = forEachSystem (system:
      import ./nix/internal/checks.nix {
        inherit lib nixpkgs system;
        inherit (self.packages.${system}) ndg-builder;
      });

    devShells = forEachSystem (system: let
      pkgs = pkgsFor system;
    in {
      default = pkgs.callPackage ./nix/internal/shell.nix {
        rustPkgs = rustPkgsFor pkgs;
      };
    });

    formatter =
      forEachSystem (system:
        import ./nix/internal/formatter.nix (pkgsFor system));
  };
}
