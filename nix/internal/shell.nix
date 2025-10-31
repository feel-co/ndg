{
  rustPkgs,
  taplo,
  clippy,
  lldb,
  llvm,
  rust-analyzer,
  rustfmt,
  cargo-nextest,
  cargo-machete,
  cargo-llvm-cov,
}:
rustPkgs.workspaceShell {
  name = "ndg-devshell";
  packages = [
    taplo # TOML formatter

    clippy # lints
    lldb # debugger
    rust-analyzer # LSP
    (rustfmt.override {asNightly = true;}) # formatter

    cargo-nextest
    cargo-machete
    cargo-llvm-cov
  ];

  env = {
    # 'cargo llvm-cov' reads these environment variables to find these
    # binaries, which are needed to run the tests.
    LLVM_COV = "${llvm}/bin/llvm-cov";
    LLVM_PROFDATA = "${llvm}/bin/llvm-profdata";
  };
}
