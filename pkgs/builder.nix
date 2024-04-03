{
  lib,
  # build dependencies
  runCommandLocal,
  pandoc,
  nixosOptionsDoc,
  # options
  modules ? [
    {
      options.hello = lib.mkOption {
        default = "world";
        description = "Example option.";
        type = lib.types.str;
      };
    }
  ],
  title ? "My Option Documentation",
  templatePath ? ./assets/default-template.html,
  styleSheetPath ? ./assets/default-styles.scss,
  codeThemePath ? ./assets/default-syntax.theme,
  ...
}: let
  inherit (lib.modules) evalModules;
  inherit (lib.strings) optionalString;

  configMD = (nixosOptionsDoc {inherit (evalModules {inherit modules;}) options;}).optionsCommonMark;
in
  runCommandLocal "generate-option-docs.html" {nativeBuildInputs = [pandoc];} (
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
    + optionalString (styleSheetPath != null) ''--css ${styleSheetPath} \''
    + optionalString (codeThemePath != null) ''--highlight-style ${codeThemePath} \''
    + "-o $out"
  )
