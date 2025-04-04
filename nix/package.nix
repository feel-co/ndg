{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "ndg";
  version = "0.1.0";

  # TODO: add a filter here
  src = ../.;
  cargoLock.lockFile = finalAttrs.src + /Cargo.lock;
  enableParallelBuilding = true;

  meta = {
    description = "ndg - not a docs generator";
    homepage = "https://github.com/feel-co/ndg";
    license = lib.licenses.mpl20;
    mainProgram = "ndg";
  };
})
