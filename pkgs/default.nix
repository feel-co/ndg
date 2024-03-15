{
  perSystem = {pkgs, ...}: let
    inherit (pkgs) callPackage mkShell;
  in {
    devShells.default = mkShell {
      packages = [pkgs.pandoc pkgs.sassc];
    };

    packages = {
      builder = callPackage ./builder.nix {};
      stylesheet = callPackage ./stylesheet.nix {};
    };
  };
}
