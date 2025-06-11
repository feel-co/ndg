{inputs, ...}: {
  perSystem = {
    lib,
    self',
    system,
    ...
  }: {
    checks = {
      default = self'.checks.nixos // self'.packages;
      nixos = self'.packages.ndg-builder.override {
        evaluatedModules = inputs.nixpkgs.lib.nixosSystem {
          modules = [
            ({modulesPath, ...}: {
              imports = ["${modulesPath}/profiles/minimal.nix"];

              boot.loader.grub.enable = false;
              fileSystems."/".device = "nodev";
              nixpkgs.hostPlatform = system;
              system.stateVersion = "24.11";
            })
          ];
        };
      };
    };
  };
}
