{
  lib,
  rustPlatform,
  installShellFiles,
  versionCheckHook,
  stdenv,
}: let
  fs = lib.fileset;
  s = ../../..;

  cargoTOML = builtins.fromTOML (builtins.readFile (s + /Cargo.toml));
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
    enableParallelBuilding = true;
    useNextest = true;

    # xtask doesn't support passing --target
    # but nix hooks expect the folder structure from when it's set
    env.CARGO_BUILD_TARGET = stdenv.hostPlatform.rust.cargoShortTarget;

    nativeInstallCheckInputs = [versionCheckHook];
    versionCheckProgram = "${placeholder "out"}/bin/${finalAttrs.meta.mainProgram}";
    versionCheckProgramArg = "--version";
    doInstallCheck = true;

    postInstall =
      lib.optionalString
      (stdenv.hostPlatform.canExecute stdenv.targetPlatform) ''
        # Install required files with the 'dist' task
        $out/bin/xtask dist

        for dir in completions man; do
          mkdir -p "$out/share/$dir"
          cp -rf "dist/$dir" "$out/share/"
        done

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
