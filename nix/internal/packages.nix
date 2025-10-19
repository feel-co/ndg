pkgs: let
  inherit (pkgs) lib;
  inherit (lib.attrsets) recursiveUpdate;
  inherit (lib.filesystem) packagesFromDirectoryRecursive;
  inherit (lib.customisation) callPackageWith;

  # Collect and call packages from the `packages` directory.
  packages =
    packagesFromDirectoryRecursive {
      callPackage = callPackageWith (recursiveUpdate pkgs {
        inherit (packages) ndg;
      });
      directory = ../packages;
    }
    // {
      default = packages.ndg;
    };
in
  packages
