{inputs, ...}: {
  perSystem = {
    lib,
    pkgs,
    self',
    system,
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
    checks = {
      nixos = self'.packages.ndg-builder.override {
        evaluatedModules = inputs.nixpkgs.lib.nixosSystem {
          modules = [
            ({modulesPath, ...}: {
              imports = ["${modulesPath}/profiles/minimal.nix"];

              boot.loader.grub.enable = false;
              fileSystems."/".device = "nodev";
              nixpkgs.hostPlatform = system;
              system.stateVersion = "24.05";
            })
          ];
        };
      };
    };

    devShells.default = mkShell {
      packages = [pkgs.pandoc pkgs.sassc self'.formatter];
    };

    packages = packages // {default = packages.ndg-builder;};
  };
}
