pkgs:
pkgs.mkShell {
  name = "ndg-devshell";
  packages = [
    pkgs.taplo # TOML formatter

    # Build tool
    pkgs.cargo
    pkgs.rustc
    pkgs.lld # linker

    pkgs.clippy # lints
    pkgs.lldb # debugger
    pkgs.rust-analyzer-unwrapped # LSP
    (pkgs.rustfmt.override {asNightly = true;}) # formatter

    pkgs.cargo-binutils
    pkgs.cargo-nextest
    pkgs.cargo-machete
    pkgs.cargo-llvm-cov
    pkgs.cargo-deny
  ];

  env = {
    RUST_BACKTRACE = "1";

    RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

    # Allow Cargo to use lld properly
    RUSTFLAGS = "-C link-arg=-fuse-ld=lld";

    # 'cargo llvm-cov' reads these environment variables to find these
    # binaries, which are needed to run the tests.
    LLVM_COV = "${pkgs.llvm}/bin/llvm-cov";
    LLVM_PROFDATA = "${pkgs.llvm}/bin/llvm-profdata";
  };
}
