# take `pkgs` as arg to allow injection of other nixpkgs instances, without flakes
{pkgs ? import ./pkgs.nix}: rec {
  checks = import ./internal/checks.nix {
    inherit (pkgs) lib;
    inherit (packages) ndg-builder;
    inherit (pkgs.stdenv.hostPlatform) system;
    nixpkgs = pkgs.path;
  };
  formatter = import ./internal/formatter.nix pkgs;
  packages = import ./internal/packages.nix pkgs;
  shell = import ./internal/shell.nix pkgs;
}
