{
  craneLib,
  pkgs,
}: let
  inherit (pkgs.lib.filesystem) packagesFromDirectoryRecursive;

  # Collect and call packages from the `packages` directory.
  packages =
    packagesFromDirectoryRecursive {
      callPackage = pkgs.newScope (packages // {inherit craneLib;});
      directory = ../packages;
    }
    // {
      default = packages.ndg;
    };
in
  packages
