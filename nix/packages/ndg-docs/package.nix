{
  pkgs,
  ndg,
}:
pkgs.writeShellApplication {
  name = "ndg-docs";
  runtimeInputs = [
    ndg
    pkgs.cargo
    pkgs.rustc
    pkgs.clang
    pkgs.llvmPackages.lld
  ];

  text = ''
    set -euo pipefail

    if [ ! -f .github/ndg.toml ]; then
      echo "error: run from repository root (missing .github/ndg.toml)" >&2
      exit 1
    fi

    # Stage docs into a temp dir so the repo stays unchanged.
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT
    mkdir -p "$tmpdir/docs"
    cp -R ./ndg/docs/. "$tmpdir/docs/"
    cp -f ./ndg/README.md "$tmpdir/docs/index.md"

    extra_flags=()
    if rustc -V | grep -q "nightly"; then
      extra_flags+=("-Zunstable-options" "-Zrustdoc-scrape-examples")
    fi

    cargo doc --workspace --exclude xtask --no-deps --document-private-items "''${extra_flags[@]}"

    ndg --config-file .github/ndg.toml html --input-dir "$tmpdir/docs"

    echo "Docs generated: ./build (NDG HTML), ./target/doc (Rustdoc)"
  '';
}
