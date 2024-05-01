{
  lib,
  # build dependencies
  runCommandLocal,
  pandoc,
  nixosOptionsDoc,
  ndg-stylesheet,
  # options
  rawModules ? [
    {
      options.hello = lib.mkOption {
        default = "world";
        defaultText = lib.literalMD ''
          ```nix
          # comment
          a: lib.hasSuffix "test" a
          ```
        '';
        description = "Example option.";
        type = lib.types.str;
      };
    }
  ],
  specialArgs ? {},
  evaluatedModules ?
    lib.evalModules {
      modules = rawModules;
      inherit specialArgs;
    },
  title ? "My Option Documentation",
  sandboxing ? true,
  templatePath ? ./assets/default-template.html,
  styleSheetPath ? ./assets/default-styles.scss,
  codeThemePath ? ./assets/default-syntax.theme,
  optionsDocArgs ? {},
} @ args:
assert args ? specialArgs -> args ? rawModules;
assert args ? evaluatedModules -> !(args ? rawModules); let
  inherit (lib.strings) optionalString;

  configMD =
    (nixosOptionsDoc (
      (removeAttrs optionsDocArgs ["options"])
      // {inherit (evaluatedModules) options;}
    ))
    .optionsCommonMark;
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
       --from markdown \
       --to html \
       --metadata title="${title}" \
       --toc \
       --standalone \
    ''
    + optionalString sandboxing ''--sandbox \''
    + optionalString (templatePath != null) ''--template ${templatePath} \''
    + optionalString (styleSheetPath != null) ''--css ${ndg-stylesheet.override {inherit styleSheetPath;}} \''
    + optionalString (codeThemePath != null) ''--highlight-style ${codeThemePath} \''
    + "-o $out"
  )
