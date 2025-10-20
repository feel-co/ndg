{
  lib,
  nixpkgs,
  system ? builtins.currentSystem,
  ndg-builder,
}: {
  nixos = ndg-builder.override {
    evaluatedModules = import "${nixpkgs}/nixos/lib/eval-config.nix" {
      inherit lib system;
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
}
