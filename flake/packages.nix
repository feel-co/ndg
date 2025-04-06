{inputs, ...}: {
  imports = [inputs.flake-parts.flakeModules.easyOverlay];
  perSystem = {
    self',
    config,
    pkgs,
    lib,
    ...
  }: let
    inherit (lib.attrsets) recursiveUpdate;
    inherit (lib.filesystem) packagesFromDirectoryRecursive;
    inherit (lib.customisation) callPackageWith;
  in {
    # Collect and call packages from the `packages` directory. Then put them
    # neatly into an overlay so that I can consume them inside my own `pkgs`
    # instance with ease. This is the top-level alternative to using the
    # module system, a.k.a, `nixpkgs.overlays`.
    overlayAttrs = config.packages;
    packages = packagesFromDirectoryRecursive {
      callPackage = callPackageWith (recursiveUpdate pkgs {
        inherit (self'.packages) ndg;
      });

      directory = ./packages;
    };
  };
}
