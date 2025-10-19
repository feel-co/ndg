{
  description = "Fast, customizable and convenient documentation generator for the Nix ecosystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";
    flake-compat.url = "github:edolstra/flake-compat";
  };

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
    inherit (nixpkgs) lib;

    systems = ["x86_64-linux" "aarch64-linux"];
    forEachSystem = lib.genAttrs systems;
    pkgsFor = system: nixpkgs.legacyPackages.${system};
  in {
    packages = forEachSystem (system: let
      pkgs = pkgsFor system;
      inherit (lib.attrsets) recursiveUpdate;
      inherit (lib.filesystem) packagesFromDirectoryRecursive;
      inherit (lib.customisation) callPackageWith;

      # Collect and call packages from the `packages` directory.
      packages =
        packagesFromDirectoryRecursive {
          callPackage = callPackageWith (recursiveUpdate pkgs {
            inherit (packages) ndg;
          });
          directory = ./nix/packages;
        }
        // {
          default = packages.ndg;
        };
    in
      packages);

    checks = forEachSystem (system: {
      nixos = self.packages.${system}.ndg-builder.override {
        evaluatedModules = nixpkgs.lib.nixosSystem {
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
    });

    devShells = forEachSystem (system: let
      pkgs = pkgsFor system;
    in {
      default = pkgs.mkShell {
        name = "ndg";
        packages = [
          pkgs.taplo # TOML formatter

          # Build tool
          pkgs.cargo
          pkgs.rustc

          pkgs.clippy # lints
          pkgs.lldb # debugger
          pkgs.rust-analyzer # LSP
          (pkgs.rustfmt.override {asNightly = true;}) # formatter

          pkgs.cargo-nextest
          pkgs.cargo-machete
          pkgs.cargo-llvm-cov
        ];

        env = {
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          # 'cargo llvm-cov' reads these environment variables to find these
          # binaries, which are needed to run the tests.
          LLVM_COV = "${pkgs.llvm}/bin/llvm-cov";
          LLVM_PROFDATA = "${pkgs.llvm}/bin/llvm-profdata";
        };
      };
    });

    formatter = forEachSystem (system: let
      pkgs = pkgsFor system;
    in
      pkgs.writeShellApplication {
        name = "nix3-fmt-wrapper";

        runtimeInputs = [
          pkgs.alejandra
          pkgs.fd
        ];

        text = ''
          fd "$@" -t f -e nix -x alejandra -q '{}'
        '';
      });
  };
}
