{inputs, ...}: {
  imports = [inputs.flake-parts.flakeModules.easyOverlay];

  perSystem = {
    final,
    lib,
    pkgs,
    self',
    system,
    ...
  }: let
    packages = {
      ndg-builder = final.callPackage ./builder.nix {};
      ndg-stylesheet = final.callPackage ./stylesheet.nix {};
    };
  in {
    checks = {
      default = self'.checks.nixos;
      nixos = self'.packages.ndg-builder.override {
        embedResources = true; # bundle stylesheet into HTML
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

    devShells.default = final.mkShell {
      packages = [pkgs.pandoc pkgs.sassc self'.formatter];
    };

    overlayAttrs = packages;

    packages = packages // {default = packages.ndg-builder;};
  };
}
