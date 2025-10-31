# take `pkgs` as arg to allow injection of other nixpkgs instances, without flakes
{
  cargo2nix ? import ./internal/flake-parse.nix "cargo2nix",
  rust-overlay ? import (import ./internal/flake-parse.nix "rust-overlay"),
  _pkgs ? import (import ./internal/flake-parse.nix "nixpkgs") {},
}: let
  pkgs = _pkgs.extend (import (cargo2nix + /overlay) rust-overlay).default;

  cargoTOML = fromTOML (builtins.readFile ./Cargo.toml);
  rustPkgs = pkgs.rustBuilder.makePackageSet {
    rustVersion = cargoTOML.workspace.package.rust-version;
    packageFun = import ./Cargo.nix;
  };
in rec {
  checks = import ./internal/checks.nix {
    inherit (pkgs) lib;
    inherit (packages) ndg-builder;
    inherit (pkgs.stdenv.hostPlatform) system;
    nixpkgs = pkgs.path;
  };
  formatter = import ./internal/formatter.nix pkgs;
  packages = import ./internal/packages.nix {inherit rustPkgs pkgs;};
  shell = pkgs.callPackage ./internal/shell.nix {inherit rustPkgs;};
}
