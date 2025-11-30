{
  lib,
  rustPlatform,
  installShellFiles,
  versionCheckHook,
  stdenv,
}: let
  fs = lib.fileset;
  s = ../../..;

  cargoTOML = lib.importTOML (s + /Cargo.toml);
in
  rustPlatform.buildRustPackage (finalAttrs: {
    pname = "ndg";
    version = cargoTOML.workspace.package.version;

    nativeBuildInputs = [installShellFiles];

    src = fs.toSource {
      root = s;
      fileset = fs.unions [
        (s + /.config)
        (s + /.cargo)
        (s + /ndg)
        (s + /ndg-commonmark)
        (s + /xtask)
        (s + /Cargo.lock)
        (s + /Cargo.toml)
      ];
    };

    cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
    cargoBuildFlags = ["-p" "ndg" "-p" "xtask"];
    enableParallelBuilding = true;

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
      description = "not a docs generator";
      homepage = "https://github.com/feel-co/ndg";
      license = lib.licenses.mpl20;
      maintainers = with lib.maintainers; [NotAShelf];
      mainProgram = "ndg";
    };
  })
