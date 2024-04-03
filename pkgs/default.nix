{
  perSystem = {
    lib,
    pkgs,
    ...
  }: let
    inherit (lib.customisation) callPackageWith;
    inherit (pkgs) mkShell;

    callPackage = callPackageWith (pkgs // packages);

    packages = {
      ndg-builder = callPackage ./builder.nix {};
      ndg-stylesheet = callPackage ./stylesheet.nix {};
    };
  in {
    devShells.default = mkShell {
      packages = [pkgs.pandoc pkgs.sassc];
    };

    packages = packages // {default = packages.ndg-builder;};
  };
}
