pkgs: let
  inherit (pkgs.lib.filesystem) packagesFromDirectoryRecursive;

  # Collect and call packages from the `packages` directory.
  packages =
    packagesFromDirectoryRecursive {
      callPackage = pkgs.newScope packages;
      directory = ../packages;
    }
    // {
      default = packages.ndg;
      nrd = packages.ndg.override {nrdCompat = true;};
      nixos-render-docs = packages.nrd;
    };
in
  packages
