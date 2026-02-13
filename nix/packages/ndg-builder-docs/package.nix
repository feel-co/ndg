{
  pkgs,
  ndg-builder,
}:
let
  stagedDocs = pkgs.runCommandLocal "ndg-docs-src" {} ''
    mkdir -p "$out"
    cp -R ${./../../../ndg/docs}/. "$out/"
    cp -f ${./../../../ndg/README.md} "$out/index.md"
  '';
in
let
  docs = ndg-builder.override {
    title = "NDG Documentation";
    description = "NDG documentation (ndg-builder)";
    inputDir = stagedDocs;
    rawModules = [];
    optionsDepth = 2;
    generateSearch = true;
    highlightCode = true;
  };
in
pkgs.runCommandLocal "ndg-builder-docs" {} ''
  mkdir -p "$out/bin"
  ln -s ${docs} "$out/docs"

  cat > "$out/bin/ndg-builder-docs" <<'EOF'
  #!/usr/bin/env sh
  set -eu

  out_dir="./build"
  while [ "$#" -gt 0 ]; do
    case "$1" in
      -o|--out-dir)
        out_dir="$2"
        shift 2
        ;;
      --print-path)
        echo "${docs}"
        exit 0
        ;;
      *)
        echo "usage: ndg-builder-docs [--out-dir PATH] [--print-path]" >&2
        exit 2
        ;;
    esac
  done

  mkdir -p "$out_dir"
  cp -R "${docs}/." "$out_dir/"
  echo "Docs copied to: $out_dir"
  EOF

  chmod +x "$out/bin/ndg-builder-docs"
''
