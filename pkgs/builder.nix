{
  lib,
  # build dependencies
  runCommandLocal,
  pandoc,
  nixosOptionsDoc,
  # options
  configMD ?
    (nixosOptionsDoc
      {
        inherit
          (lib.evalModules {
            modules = [
              {
                options.hello = lib.mkOption {
                  default = "world";
                  description = "Example option.";
                  type = lib.types.str;
                };
              }
            ];
          })
          options
          ;
      })
    .optionsCommonMark,
  title ? "My Option Documentation",
  templatePath ? ./assets/default-template.html,
  styleSheet ? ./assets/default-styles.scss,
  codeThemePath ? ./assets/default-syntax.json,
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
    + optionalString (codeThemePath != null) ''--highlight-style ${codeThemePath} \''
    + "-o $out"
  )
