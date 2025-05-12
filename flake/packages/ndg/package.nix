{
  lib,
  rustPlatform,
  installShellFiles,
  stdenv,
}: let
  fs = lib.fileset;
in
  rustPlatform.buildRustPackage (finalAttrs: {
    pname = "ndg";
    version = "0.1.0";

    nativeBuildInputs = [installShellFiles];

    src = fs.toSource {
      root = ../../..;
      fileset = fs.unions [
        (fs.fileFilter (file: builtins.any file.hasExt ["rs"]) ../../../src)
        ../../../Cargo.lock
        ../../../Cargo.toml
        ../../../templates
      ];
    };

    cargoLock.lockFile = "${finalAttrs.src}/Cargo.lock";
    useFetchCargoVendor = true;
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
      mainProgram = "ndg";
    };
  })
