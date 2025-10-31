# take `pkgs` as arg to allow injection of other nixpkgs instances, without flakes
{
  craneLib ? import (import ./internal/flake-parse.nix "crane") {inherit pkgs;},
  pkgs ? import (import ./internal/flake-parse.nix "nixpkgs") {},
}: rec {
  checks = import ./internal/checks.nix {
    inherit (pkgs) lib;
    inherit (packages) ndg-builder;
    inherit (pkgs.stdenv.hostPlatform) system;
    nixpkgs = pkgs.path;
  };
  formatter = import ./internal/formatter.nix pkgs;
  packages = import ./internal/packages.nix {inherit craneLib pkgs;};
  shell = import ./internal/shell.nix {inherit craneLib pkgs;};
}
