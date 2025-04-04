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
  enableParallelBuilding = true;

  postInstall = ''
    # Generate manpages and completion files
    $out/bin/ndg generate --output-dir ./dist

    # Install shell completions
    installShellCompletion --bash --name ndg.bash ./dist/completions/ndg.bash
    installShellCompletion --fish --name ndg.fish ./dist/completions/ndg.fish
    installShellCompletion --zsh --name _ndg ./dist/completions/_ndg

    # Install manpages
    installManPage ./dist/man/ndg.1
  '';

  meta = {
    description = "ndg - not a docs generator";
    homepage = "https://github.com/feel-co/ndg";
    license = lib.licenses.mpl20;
    mainProgram = "ndg";
  };
})
