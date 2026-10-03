{
  pkgs,
  lib,
  ndg-builder,
}: let
  fs = lib.fileset;
  src = ../../..;
  docsSrc = fs.toSource {
    root = src;
    fileset = fs.unions [
      (src + /ndg/docs)
      (src + /ndg-commonmark/docs)
      (src + /ndg/README.md)
    ];
  };
  docs =
    (ndg-builder.override {
      title = "NDG Documentation";
      description = "NDG documentation (ndg-builder)";
      inputDir = "docs";
      rawModules = [];
      optionsDepth = 2;
      generateSearch = true;
      highlightCode = true;
      buildZim = true;
      zimId = "ndg-builder";
      creator = "feel-co.org";
      publisher = "feel-co.org";
      zimIllustration = ./logo.png;
      source = "https://ndg.feel-co.org/";
      extraConfig.anchor.on_duplicate = "deduplicate";
    }).overrideAttrs (previous: {
      src = docsSrc;
      buildCommand = ''
        mkdir docs
        cp -R "$src"/{ndg,ndg-commonmark}/docs/. docs/
        cp -f "$src/ndg/README.md" docs/index.md

        ${previous.buildCommand}
      '';
    });
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
