{
  description = "Fast, customizable and convenient documentation generator for the Nix ecosystem";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    inherit (nixpkgs) lib;

    systems = ["x86_64-linux" "aarch64-linux"];
    forEachSystem = lib.genAttrs systems;
    pkgsFor = system: nixpkgs.legacyPackages.${system};
  in {
    packages =
      forEachSystem (system:
        import ./nix/internal/packages.nix (pkgsFor system));

    checks = forEachSystem (system:
      import ./nix/internal/checks.nix {
        inherit lib nixpkgs system;
        inherit (self.packages.${system}) ndg-builder;
      });

    devShells = forEachSystem (system: {
      default = import ./nix/internal/shell.nix (pkgsFor system);
    });

    formatter =
      forEachSystem (system:
        import ./nix/internal/formatter.nix (pkgsFor system));
  };
}
