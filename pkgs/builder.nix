{
  lib,
  # build dependencies
  runCommandLocal,
  pandoc,
  # options
  configMD, # TODO: find a suitable default that we can type-check
  title ? "My Option Documentation",
  templatePath ? ./assets/default-template.html,
  styleSheet ? ./assets/default-styles.scss,
  syntax ? ./assets/default-syntax.json, # TODO: pass syntax file to the pandoc builder
  ...
}: let
  inherit (lib.strings) optionalString;
in
  runCommandLocal "generate-option-docs" {nativeBuildInputs = [pandoc];} (
    ''
      # convert to pandoc markdown instead of using commonmark directly,
      # as the former automatically generates heading ids and TOC links.
      pandoc \
        --from commonmark \
        --to markdown \
        ${configMD} |


      # convert pandoc markdown to html using our own template and css files
      # where available. --sandbox is passed for extra security.
      pandoc \
       --sandbox \
       --from markdown \
       --to html \
       --metadata title="${title}" \
       --toc \
       --standalone \
    ''
    + optionalString (templatePath != null) ''--template ${templatePath} \''
    + optionalString (styleSheet != null) ''--css=${styleSheet} \''
    + "-o $out"
  )
