{
  perSystem = {
    self',
    pkgs,
    ...
  }: {
    formatter = pkgs.alejandra;

    devShells.default = pkgs.mkShell {
      name = "ndg";
      packages = [
        self'.formatter
        pkgs.taplo # TOML formatter

        # Poor man's Rust environment
        pkgs.cargo
        pkgs.rustc

        pkgs.clippy # lints
        pkgs.lldb # debugger
        pkgs.rust-analyzer # LSP
        (pkgs.rustfmt.override {asNightly = true;}) # formatter

        pkgs.cargo-nextest
      ];

      env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
    };
  };
}
