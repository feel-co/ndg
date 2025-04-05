{
  perSystem = {
    self',
    pkgs,
    ...
  }: {
    packages = {
      ndg = pkgs.callPackage ./packages/ndg.nix {};
      default = self'.packages.ndg;
    };
  };
}
