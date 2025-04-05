{
  lib,
  rustPlatform,
  installShellFiles,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "ndg";
  version = "0.1.0";

  nativeBuildInputs = [installShellFiles];

  # TODO: add a filter here
  src = ../.;
  cargoLock.lockFile = finalAttrs.src + /Cargo.lock;
  useFetchCargoVendor = true;
  enableParallelBuilding = true;

  postInstall = ''
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
