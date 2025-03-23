{
  description = "Streamlined documentation generation for NixOS modules";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = inputs.nixpkgs.lib.systems.flakeExposed;
      perSystem = {
        self',
        pkgs,
        ...
      }: {
        formatter = pkgs.alejandra;

        devShells.default = pkgs.mkShell {
          name = "ndg";
          strictDeps = true;
          packages = [
            self'.formatter

            # Poor man's Rust environment
            pkgs.cargo
            pkgs.rustc
            (pkgs.rustfmt.override {asNightly = true;})
            pkgs.clippy
          ];

          env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      };
    };
}
