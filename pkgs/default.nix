{
  perSystem = {pkgs, ...}: let
    inherit (pkgs) callPackage;
  in {
    packages = {
      builder = callPackage ./builder.nix {};
      stylesheet = callPackage ./stylesheet.nix {};
    };
  };
}
