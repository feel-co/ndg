{
  description = "Fast, customizable and convenient documentation generator for the Nix ecosystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
  }: let
    inherit (nixpkgs) lib;

    systems = ["x86_64-linux" "aarch64-linux"];
    forEachSystem = lib.genAttrs systems;
    pkgsFor = system: nixpkgs.legacyPackages.${system};
  in {
    packages = forEachSystem (system:
      import ./nix/internal/packages.nix rec {
        craneLib = crane.mkLib pkgs;
        pkgs = pkgsFor system;
      });

    checks = forEachSystem (system:
      import ./nix/internal/checks.nix {
        inherit lib nixpkgs system;
        inherit (self.packages.${system}) ndg-builder;
      });

    devShells = forEachSystem (system: {
      default = import ./nix/internal/shell.nix rec {
        craneLib = crane.mkLib pkgs;
        pkgs = pkgsFor system;
      };
    });

    formatter =
      forEachSystem (system:
        import ./nix/internal/formatter.nix (pkgsFor system));
  };
}
