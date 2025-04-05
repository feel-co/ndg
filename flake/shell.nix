{
  perSystem = {
    self',
    pkgs,
    ...
  }: {
    devShells.default = pkgs.mkShell {
      name = "ndg";
      packages = [
        self'.formatter

        # Poor man's Rust environment
        pkgs.cargo
        pkgs.rustc
        pkgs.clippy
        (pkgs.rustfmt.override {asNightly = true;})
      ];

      env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
    };
  };
}
