{
  lib,
  rustPlatform,
  installShellFiles,
  versionCheckHook,
  stdenv,
  rustc,
}: let
  fs = lib.fileset;
  s = ../../..;

  cargoTOML = lib.importTOML (s + /Cargo.toml);
  inherit (rustc) llvmPackages;
in
  rustPlatform.buildRustPackage (finalAttrs: {
    pname = "ndg";
    version = cargoTOML.workspace.package.version;

    src = fs.toSource {
      root = s;
      fileset = fs.unions [
        (s + /.config)
        (s + /.cargo)

        (s + /ndg)
        (s + /ndg-commonmark)
        (s + /crates)

        (s + /Cargo.lock)
        (s + /Cargo.toml)
      ];
    };

    cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
    cargoBuildFlags = ["-p" "ndg" "-p" "xtask"];
    enableParallelBuilding = true;

    nativeBuildInputs = [
      installShellFiles

      # Link with lld for faster and more optimized results. As of Rust 1.90 this
      # is the default when lld is available in the system. This is not a problem
      # for reasonable distros that package up-to-date toolchains, such as NixOS.
      # Let's try it, users can always opt out anyway.
      llvmPackages.lld
      llvmPackages.clang
    ];

    env = {
      # lld is the default on Rust 1.90+, but we don't stand to lose anything from
      # being more explicit. If anything, this makes errors clearer when lld is not
      # available, instead of silently falling back.
      #
      # We use Clang + lld as the Rust linker. This doesn't actually affect the
      # codegen part, but *can* yield:
      #  - significantly faster link times for large crate graphs
      #  - slightly smaller binaries in some cases
      # and makes our entire toolchain LLVM-based, which never hurts.
      RUSTFLAGS = "-C linker=clang -C link-arg=-fuse-ld=lld";

      # Path to libclang for crates that require it (e.g. bindgen). We don't use
      # bindgen yet for anything, but we *might* for my nix-bindings in the near
      # future if we want to add, e.g., Nix eval.
      LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

      # Attempt to optimize the various Treesitter crates pulled by NDG to power
      # syntax highlighting for codeblocks. Since those are data-heavy crates our
      # options are limited, but we should still try. Unfortunately Cargo does
      # not have a good way (e.g., build.rs doesn't help) that lets us propagate
      # C/PP flags to builds so it has to be done on the packaging level.
      CFLAGS = "-O2 -ffunction-sections -fdata-sections";
    };

    nativeInstallCheckInputs = [versionCheckHook];
    doInstallCheck = true;
    versionCheckProgram = "${placeholder "out"}/bin/${finalAttrs.meta.mainProgram}";
    versionCheckProgramArg = "--version";

    # Besides the install check, we have a bunch of tests to run. Nextest is
    # the fastest way of running those since it's significantly faster than
    # `cargo test`, and has a nicer UI with CI-friendly characteristics.
    useNextest = true;
    cargoTestFlags = ["--workspace" "--verbose"];

    postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
      # Install required files with the 'dist' task
      $out/bin/xtask dist

      # Install the generated completions/ and man/ artifacts
      installManPage ./man/*
      installShellCompletion --cmd ${finalAttrs.meta.mainProgram} ./completions/*

      # Avoid populating PATH with an 'xtask' cmd
      rm $out/bin/xtask
    '';

    meta = {
      description = "NDG: not a docs generator";
      changelog = "https://github.com/feel-co/ndg/blob/${finalAttrs.version}/CHANGELOG.md";
      homepage = "https://github.com/feel-co/ndg";
      license = lib.licenses.mpl20;
      maintainers = [lib.maintainers.NotAShelf];
      mainProgram = "ndg";
    };
  })
