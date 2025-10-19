pkgs:
pkgs.mkShell {
  name = "ndg-devshell";
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
}
