{
  lib,
  rustPlatform,
  installShellFiles,
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
        (s + /ndg)
        (s + /ndg-commonmark)
        (s + /Cargo.lock)
        (s + /Cargo.toml)
      ];
    };

    cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
    enableParallelBuilding = true;

    postInstall =
      lib.optionalString
      (stdenv.hostPlatform.canExecute stdenv.targetPlatform) ''
        $out/bin/ndg generate
        installShellCompletion dist/completions/{ndg.bash,ndg.fish,_ndg}
        installManPage dist/man/ndg.1
      '';

    meta = {
      description = "not a docs generator";
      homepage = "https://github.com/feel-co/ndg";
      license = lib.licenses.mpl20;
      maintainers = with lib.maintainers; [NotAShelf];
      mainProgram = "ndg";
    };
  })
